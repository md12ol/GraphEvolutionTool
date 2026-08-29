//! Evolve graphs with a genetic algorithm, from Rust or from Python.
//!
//! A run is described entirely by a `config.toml` — population and network
//! size, the evolution strategy, the genome representation, and the objective
//! to optimize — and three entry points read that same document, so a run
//! means the same thing whichever one drives it:
//!
//! - [`run_from_toml`] and [`run_many_from_toml`], for a Rust caller, with no
//!   Python interpreter involved.
//! - The `get-run` binary, which is those two behind a command line.
//! - [`GraphEvolver`], which is also the `get` Python extension module: built
//!   from a config file, or from config objects assembled in Python.
//!
//! A fourth route bypasses config entirely: the engine types in [`evolver`],
//! [`genomes`], [`fitness`] and [`graph`], for a caller driving their own loop
//! with a native objective and no `Config`, no TOML, and no Python involved —
//! see `examples/library_route.rs`.
//!
//! Fitness reaches a caller **as-measured** — the units and sign the objective
//! returned. The lower-is-better form the engine compares on is internal and
//! never leaves it.

// Denied here rather than in CI, so a doc link into a private item fails on the
// machine that wrote it.
#![deny(rustdoc::private_intra_doc_links, rustdoc::redundant_explicit_links)]

mod config;
mod dispatch;
pub mod evolver;
pub mod fitness;
pub mod genomes;
pub mod graph;
pub mod graph_io;
mod py_config;
pub mod py_result;
pub mod sir;
pub mod stats;

use crate::config::{Config, ConfigError, FitnessConfig, GenomeConfig};
use crate::evolver::GenerationStats;
use crate::fitness::{Direction, PyFitness};
use crate::graph::Graph;
use crate::graph_io::{LoadWarning, SourcedEdge, canonicalize};
use crate::py_config::{
    PyConfig, PyCrossoverConfig, PyEdgeEditMutationConfig, PyEvolutionConfig, PyFitnessConfig,
    PyGenomeConfig, PyOperationWeights, PyReplacementConfig, PyScopeConfig, PySdaMutationConfig,
    PySelectionConfig, PySirParams, config_error_to_py,
};
use crate::py_result::{PyGenerationStats, PyRunResult};
use crate::stats::ReferenceStatistics;
use pyo3::exceptions::{PyUserWarning, PyValueError};
use pyo3::prelude::*;
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// One graph as a folder load hands it back: the file it was read from, its
/// node count, and its edges as `(u, v, multiplicity)`.
type NamedGraph = (String, usize, Vec<(usize, usize, u32)>);

/// Raise every warning a load produced through Python's `warnings` machinery.
///
/// `stacklevel` is 2, so a message points at the line that called the setter
/// rather than at this function.
fn emit_load_warnings(py: Python<'_>, source: &str, warnings: &[LoadWarning]) -> PyResult<()> {
    let category = py.get_type::<PyUserWarning>();

    for warning in warnings {
        let text = format!("{source}: {warning}");
        let message = CString::new(text).map_err(|_| {
            PyValueError::new_err("a load warning could not be converted for Python")
        })?;

        PyErr::warn(py, &category, &message, 2)?;
    }

    Ok(())
}

/// Raise every warning a load produced, on either route.
///
/// `Some(py)` raises a `UserWarning`; `None` — the Rust route, which never
/// acquires the GIL — prints the same text to stderr, the only sink it has.
pub(crate) fn emit_load_warnings_maybe(
    py: Option<Python<'_>>,
    source: &str,
    warnings: &[LoadWarning],
) -> PyResult<()> {
    match py {
        Some(py) => emit_load_warnings(py, source, warnings),
        None => {
            for warning in warnings {
                eprintln!("{source}: {warning}");
            }
            Ok(())
        }
    }
}

/// Load the base graph a config names, resolved against the config file itself.
///
/// `Ok(None)` when the genome is not edge-edit, or names no base graph. The
/// path is relative to the config file's directory rather than the working
/// directory, so a folder holding a config and the graph it names runs from
/// wherever it is unpacked; an absolute path is taken as given.
///
/// The two routes that reach this both load a `Config` *from a path*, which is
/// what makes the key resolvable at all. A config assembled in Python has no
/// directory to resolve against, so it has no such key and uses
/// [`GraphEvolver::set_base_graph_from_file`] — which is also the way in for a
/// file numbered from anything but 0, since this route has no
/// `min_node_index` to declare one.
fn config_base_graph(
    py: Option<Python<'_>>,
    config: &Config,
    config_path: &Path,
) -> PyResult<Option<Graph>> {
    let GenomeConfig::EdgeEdit(edge_edit) = &config.genome else {
        return Ok(None);
    };
    let Some(named) = &edge_edit.base_graph else {
        return Ok(None);
    };

    let named = Path::new(named);
    let path = match config_path.parent() {
        Some(directory) if named.is_relative() => directory.join(named),
        _ => named.to_path_buf(),
    };

    let loaded =
        graph_io::load_edge_file(&path, config.network_size, config.max_edge_multiplicity, 0)
            .map_err(|err| PyValueError::new_err(err.to_string()))?;

    emit_load_warnings_maybe(py, &loaded.source, &loaded.warnings)?;

    // The same disagreement `set_base_graph_from_file` rejects: the loader
    // refuses a header above `network_size`, so below it is what lands here,
    // and a file the writer believes is 200 nodes but which says 180 would
    // otherwise load as 200 padded with isolated nodes nobody asked for.
    if loaded.num_nodes != config.network_size {
        return Err(PyValueError::new_err(format!(
            "`{}` states `# nodes = {}` but network_size is {}; the graph an \
             edit script is applied to must be the size the run evolves",
            loaded.source, loaded.num_nodes, config.network_size,
        )));
    }

    let mut graph = Graph::new(config.network_size, config.max_edge_multiplicity);
    graph.set_edges(&loaded.edges);
    Ok(Some(graph))
}

/// Python-facing entry point to the graph-evolution engine.
///
/// Constructed from a `config.toml` path; `run` dispatches on the configured
/// evolution strategy, genome representation and fitness objective.
///
/// **It holds no results.** A run's state lives in the `RunResult` that `run`
/// returns, so one evolver drives repeated runs with nothing stale from the
/// previous one hanging off it.
#[pyclass(module = "get")]
pub struct GraphEvolver {
    config: Config,
    /// The objective registered by `set_fitness_function`, set only when
    /// `[fitness] type = "python"`.
    fitness_function: Option<PyFitness>,
    /// The graph an edge-edit script is applied to. `None` means every run
    /// starts from an empty graph, which is the default.
    base_graph: Option<Graph>,
    /// Where the caller's own node numbering starts, declared by whichever
    /// loader ran first. One per run, shared by every loader: two files
    /// disagreeing about where counting starts would silently mix two graphs.
    min_node_index: Option<i64>,
    /// The TOML document `config` was parsed from — the run's provenance
    /// record, written alongside its results.
    config_toml: String,
    /// `struct_match`'s reduced reference set, built at most once per evolver.
    ///
    /// Shared across replicates where the objective itself is not: these are
    /// immutable, and rebuilding them per replicate would re-read the folder and
    /// re-run an eigendecomposition of every reference graph.
    struct_match_reference: OnceLock<Arc<ReferenceStatistics>>,
}

#[pymethods]
impl GraphEvolver {
    /// Load configuration from a `config.toml` file.
    #[new]
    pub fn new(py: Python<'_>, config_path: String) -> PyResult<Self> {
        // Read here rather than through `Config::from_path`, which does not hand
        // back the raw text the provenance record needs.
        let text = std::fs::read_to_string(&config_path).map_err(|err| {
            PyValueError::new_err(format!("failed to load config: {}", ConfigError::Io(err)))
        })?;
        let config = Config::from_toml_str(&text)
            .map_err(|err| PyValueError::new_err(format!("failed to load config: {err}")))?;
        config
            .validate()
            .map_err(|err| PyValueError::new_err(format!("failed to load config: {err}")))?;
        // Loaded here rather than at `run`, so a bad path is an error where the
        // config was named. A later `set_base_graph` call overwrites it: the
        // setter is the more specific instruction and it arrives second.
        let base_graph = config_base_graph(Some(py), &config, Path::new(&config_path))?;
        Ok(Self {
            config,
            fitness_function: None,
            base_graph,
            min_node_index: None,
            struct_match_reference: OnceLock::new(),
            config_toml: text,
        })
    }

    /// Build from configuration assembled in Python, rather than from a file.
    ///
    /// Accepts exactly what a `config.toml` would, and reports a rejection in
    /// the same words. Every block is its own object — `EvolutionConfig`,
    /// `ScopeConfig`, `SelectionConfig`, `GenomeConfig`, `FitnessConfig` — and
    /// all of them are required.
    ///
    /// A field too large for a TOML integer is rejected here, which a
    /// `config.toml` writer never meets.
    #[staticmethod]
    fn from_config(config: &PyConfig) -> PyResult<Self> {
        let text = config.to_toml()?;

        let parsed = Config::from_toml_str(&text).map_err(|err| {
            PyValueError::new_err(format!(
                "the generated config did not parse: {err}. This is a bug in GET, not in \
                 your configuration — please report it with the document below.\n---\n{text}"
            ))
        })?;

        // Rewritten into the Python attribute path that produced the field, for
        // a caller who never saw a TOML document.
        parsed.validate().map_err(|err| config_error_to_py(&err))?;

        Ok(Self {
            config: parsed,
            fitness_function: None,
            base_graph: None,
            min_node_index: None,
            struct_match_reference: OnceLock::new(),
            config_toml: text,
        })
    }

    /// Register a Python callable as the objective, with the direction it is
    /// meant to be optimized in.
    ///
    /// `config.toml` only *selects* Python — `[fitness] type = "python"`. The
    /// callable itself arrives here, and so does its direction: nothing can
    /// infer whether a user's function wants its value large or small.
    ///
    /// The callable takes the **whole batch** and returns one float per graph,
    /// in the same order:
    ///
    /// A batch element is `(num_nodes, edges)`, and `direction` is
    /// `"minimize"` or `"maximize"`.
    ///
    /// Registering a callable when the config did not select Python is a
    /// `ValueError`, not a silent no-op.
    fn set_fitness_function(
        &mut self,
        callable: &Bound<'_, PyAny>,
        direction: &str,
    ) -> PyResult<()> {
        if !matches!(self.config.fitness, FitnessConfig::Python) {
            return Err(PyValueError::new_err(format!(
                "[fitness] type is \"{}\", so a registered callable would never be used; \
                 set type = \"python\" in the config to use a Python objective",
                self.config.fitness.type_name(),
            )));
        }

        if !callable.is_callable() {
            return Err(PyValueError::new_err(format!(
                "fitness function: must be callable, got {}",
                callable.get_type().name()?,
            )));
        }

        let direction = if direction.eq_ignore_ascii_case("minimize") {
            Direction::Minimize
        } else if direction.eq_ignore_ascii_case("maximize") {
            Direction::Maximize
        } else {
            return Err(PyValueError::new_err(format!(
                "direction: must be \"minimize\" or \"maximize\", got \"{direction}\"",
            )));
        };

        self.fitness_function = Some(PyFitness::new(callable.clone().unbind(), direction));
        Ok(())
    }

    /// Seed an edge-edit run from a graph the caller already has.
    ///
    /// `edges` is `(u, v, multiplicity)` — the same shape `run` hands back as
    /// `best_edges`, so one run's output feeds the next without reshaping.
    ///
    /// **One run has one numbering.** `min_node_index` is where the caller's
    /// own numbering starts — pass `1` for 1-indexed edges. Every loader on this
    /// evolver must declare the same one, and it is what results are shifted
    /// back into, so they return in the numbering the data arrived in.
    ///
    /// **Left unset, every run starts from an empty graph.** Several edit
    /// opcodes need existing structure to walk, so early generations do little
    /// until `Add` and `Toggle` have built some. That is self-correcting.
    ///
    /// A rejected edge raises `ValueError` naming the index as the caller wrote
    /// it, not as it would be after shifting. A pair given more than once is a
    /// `UserWarning` and the **last** occurrence wins, compared canonically, so
    /// `(2, 5)` and `(5, 2)` are one undirected edge.
    #[pyo3(signature = (num_nodes, edges, min_node_index = 0))]
    fn set_base_graph(
        &mut self,
        py: Python<'_>,
        num_nodes: usize,
        edges: Vec<(usize, usize, u32)>,
        min_node_index: i64,
    ) -> PyResult<()> {
        self.check_min_node_index(min_node_index)?;

        if num_nodes != self.config.network_size {
            return Err(PyValueError::new_err(format!(
                "base graph has {} nodes but [evolution] network_size is {}; \
                 the graph an edit script is applied to must be the size the run evolves",
                num_nodes, self.config.network_size,
            )));
        }

        if !matches!(self.config.genome, GenomeConfig::EdgeEdit(_)) {
            return Err(PyValueError::new_err(
                "[genome] type is \"sda\", which generates its graph rather than editing one, \
                 so a base graph would never be read; set type = \"edge_edit\" to seed a run",
            ));
        }

        // Checked before anything is built: `Graph::set_edge` absorbs all three
        // failures rather than reporting them, so a graph built first would
        // already have lost the offending edge. Bounds are in the caller's own
        // numbering, so a message quotes the indices they wrote.
        let lowest = min_node_index;
        let highest = min_node_index + num_nodes as i64 - 1;

        for &(u, v, multiplicity) in &edges {
            let (u_given, v_given) = (u as i64, v as i64);
            if u_given < lowest || u_given > highest || v_given < lowest || v_given > highest {
                return Err(PyValueError::new_err(format!(
                    "edge ({u}, {v}) names a node outside {lowest}..={highest}; the node count \
                     matching network_size does not make the edges in range, and an \
                     out-of-range edge would be dropped without a word",
                )));
            }

            if u == v {
                return Err(PyValueError::new_err(format!(
                    "edge ({u}, {v}) is a self-loop, which this graph has no representation \
                     for; a self-loop in caller data usually means the indices are wrong, so \
                     it is reported rather than dropped",
                )));
            }

            if multiplicity > self.config.max_edge_multiplicity {
                return Err(PyValueError::new_err(format!(
                    "edge ({u}, {v}) has multiplicity {multiplicity}, above this config's \
                     max_edge_multiplicity of {}; lower it and resubmit rather than having \
                     it silently clamped",
                    self.config.max_edge_multiplicity,
                )));
            }
        }

        // Repeats are collapsed before building: `set_edge` writes each edge in
        // turn, so a list holding a pair twice silently kept whichever came last.
        let mut sourced = Vec::with_capacity(edges.len());
        for &(u, v, multiplicity) in &edges {
            sourced.push(SourcedEdge {
                // The range check above is what keeps these casts non-negative:
                // every index is at least `lowest`.
                u: (u as i64 - min_node_index) as usize,
                v: (v as i64 - min_node_index) as usize,
                weight: multiplicity,
                line: None,
            });
        }

        let (deduplicated, warnings) = canonicalize(&sourced, 0);
        emit_load_warnings(py, "base graph", &warnings)?;

        let mut graph = Graph::new(self.config.network_size, self.config.max_edge_multiplicity);
        graph.set_edges(&deduplicated);
        self.base_graph = Some(graph);
        self.commit_min_node_index(min_node_index);
        Ok(())
    }

    /// Seed an edge-edit run from an edge-list file, rather than from a list
    /// built in Python.
    ///
    /// One edge per line, `start,end,weight`, comma-delimited, any line ending.
    /// `min_node_index` is where the caller's own node numbering starts — pass
    /// `1` for 1-indexed data, which is the common case in graph files.
    /// 1-indexed in is 1-indexed out.
    ///
    /// A `# nodes = N` header is mandatory and must agree with `network_size`;
    /// a file with no header is rejected rather than assumed to match it.
    ///
    /// **Nothing is stored unless the whole file survives**, and a rejection
    /// names the line it came from. A repeated edge, a zero-weight edge and an
    /// empty file are each a `UserWarning` rather than an error.
    #[pyo3(signature = (path, min_node_index = 0))]
    fn set_base_graph_from_file(
        &mut self,
        py: Python<'_>,
        path: String,
        min_node_index: i64,
    ) -> PyResult<()> {
        if !matches!(self.config.genome, GenomeConfig::EdgeEdit(_)) {
            return Err(PyValueError::new_err(
                "[genome] type is \"sda\", which generates its graph rather than editing one, \
                 so a base graph would never be read; set type = \"edge_edit\" to seed a run",
            ));
        }

        self.check_min_node_index(min_node_index)?;

        let loaded = graph_io::load_edge_file(
            std::path::Path::new(&path),
            self.config.network_size,
            self.config.max_edge_multiplicity,
            min_node_index,
        )
        .map_err(|err| PyValueError::new_err(err.to_string()))?;

        emit_load_warnings(py, &loaded.source, &loaded.warnings)?;

        if loaded.num_nodes != self.config.network_size {
            return Err(PyValueError::new_err(format!(
                "`{}` states `# nodes = {}` but [evolution] network_size is {}; \
                 the graph an edit script is applied to must be the size the run evolves",
                loaded.source, loaded.num_nodes, self.config.network_size,
            )));
        }

        let mut graph = Graph::new(self.config.network_size, self.config.max_edge_multiplicity);
        graph.set_edges(&loaded.edges);
        self.base_graph = Some(graph);
        self.commit_min_node_index(min_node_index);
        Ok(())
    }

    /// Read a folder of graphs, one file per graph, and hand them back.
    ///
    /// The bulk counterpart to `set_base_graph_from_file`, for reference data an
    /// objective matches against. Each file is one edge per line,
    /// `start,end,weight`, and every file in the folder shares this run's node
    /// numbering.
    ///
    /// **A reference graph may be larger than the network being evolved**, and
    /// usually is — the only ceiling is a sanity bound against a file indexed
    /// the wrong way.
    ///
    /// Each graph comes back as `(name, num_nodes, edges)`, **sorted by file
    /// name**, because a reference set is consumed positionally and filesystem
    /// order would let a run's numbers depend on how its data was written to
    /// disk. The node count is stated rather than derived: an isolated node
    /// appears in no edge, so each file declares its own in a `# nodes = N`
    /// header. Sub-directories are skipped; every other file is read.
    ///
    /// **This call declares the run's numbering** as the base-graph setters do.
    /// The reference graphs themselves come back exactly as supplied.
    ///
    /// A rejection names the file and the line; the warnings are the base-graph
    /// loader's, each naming the file it came from.
    #[pyo3(signature = (folder, min_node_index = 0))]
    fn load_reference_graphs(
        &mut self,
        py: Python<'_>,
        folder: String,
        min_node_index: i64,
    ) -> PyResult<Vec<NamedGraph>> {
        self.check_min_node_index(min_node_index)?;

        // An upper bound rather than the network size, since reference graphs
        // may be larger; the objective applies the same one.
        let index_cap = self.config.network_size.max(dispatch::MAX_REFERENCE_NODES);

        let loaded = graph_io::load_edge_folder(
            std::path::Path::new(&folder),
            index_cap,
            self.config.max_edge_multiplicity,
            min_node_index,
        )
        .map_err(|err| PyValueError::new_err(err.to_string()))?;

        let mut graphs = Vec::with_capacity(loaded.len());
        for file in loaded {
            emit_load_warnings(py, &file.source, &file.warnings)?;
            graphs.push((file.source, file.num_nodes, file.edges));
        }

        self.commit_min_node_index(min_node_index);
        Ok(graphs)
    }

    /// Evolve a population `n_runs` times and return what every run produced.
    ///
    /// **Always a list, one `RunResult` per replicate, in run order** — even at
    /// the default `n_runs = 1`, so the return type does not change shape with
    /// an argument.
    ///
    /// **One master seed, not `n` of them**, so a run's seed does not depend on
    /// how many were requested: asking for 50 reproduces the first 30 of a
    /// 30-run request exactly.
    ///
    /// **Whether replicates run concurrently is the engine's call, not yours.**
    /// A native Rust objective runs them in parallel; `fitness = "python"` runs
    /// them one at a time, since concurrent runs would contend for a single GIL.
    /// `max_cores` caps the concurrency; unset means all available.
    ///
    /// A replicate that fails abandons the remaining runs rather than returning
    /// half-complete.
    ///
    /// # Memory: the sizes multiply, they do not add
    ///
    /// The whole population is materialized before scoring, and a graph is a
    /// **dense** matrix however sparse it actually is, so peak memory is roughly
    /// `network_size² × 4 bytes × population_size × min(max_cores, replicates)`.
    /// Raising any one of them scales the whole product.
    #[pyo3(signature = (seed, n_runs = 1, max_cores = None))]
    pub fn run(
        &mut self,
        py: Python<'_>,
        seed: u64,
        n_runs: usize,
        max_cores: Option<usize>,
    ) -> PyResult<Vec<PyRunResult>> {
        if n_runs == 0 {
            return Err(PyValueError::new_err(
                "n_runs must be at least 1; asking for zero runs returns nothing and is \
                 more likely a mistake than an intent",
            ));
        }

        if max_cores == Some(0) {
            return Err(PyValueError::new_err(
                "max_cores must be at least 1 if given; leave it unset for all available \
                 cores, or pass 1 to run replicates one at a time",
            ));
        }

        let seeds = dispatch::replicate_seeds(seed, n_runs);

        // One objective per run, never one shared: every SIR objective owns an
        // epidemic counter, and sharing it across concurrent replicates would
        // let thread scheduling decide which run saw which epidemic seed.
        // Built before the GIL is released, because the Python arm reads the
        // registered callable and that must not happen on a rayon worker.
        let mut objectives = Vec::with_capacity(n_runs);
        for &run_seed in &seeds {
            objectives.push(self.objective(run_seed, Some(py))?);
        }

        // The GIL is released for the run: everything between scoring calls is
        // pure Rust, and `PyFitness` re-acquires it per batch. Holding it would
        // deadlock — a rayon worker calling back into Python needs the GIL, and
        // cannot get it while this thread holds it and waits on that worker.
        let outcomes = Python::attach(|py| {
            py.detach(|| {
                dispatch::run_replicates(
                    &self.config,
                    &objectives,
                    self.base_graph.as_ref(),
                    &seeds,
                    max_cores,
                )
            })
        })?;

        // `seed` is the master, not the per-run draw: the pair that reproduces a
        // replicate is `(master, run_index)`. Handing back the derived seed
        // would make the stream draw from *it*, producing a different run.
        let mut results = Vec::with_capacity(outcomes.len());
        for (run_index, mut outcome) in outcomes.into_iter().enumerate() {
            // The one place a node index goes back to the caller's numbering.
            shift_out(&mut outcome.best_edges, self.min_node_index);

            results.push(PyRunResult::from_erased(
                outcome,
                seed,
                run_index,
                self.config_toml.clone(),
            ));
        }
        Ok(results)
    }
}

impl GraphEvolver {
    /// Reject a numbering that disagrees with one already in force, recording
    /// nothing either way.
    ///
    /// Separate from `commit_min_node_index` so a load that fails leaves the
    /// evolver as it found it: a numbering recorded by a call that then failed
    /// would shift every later run's output, with no base graph loaded.
    fn check_min_node_index(&self, min_node_index: i64) -> PyResult<()> {
        match self.min_node_index {
            Some(existing) if existing != min_node_index => Err(PyValueError::new_err(format!(
                "this evolver already reads node indices as starting at {existing}, and this \
                 call says {min_node_index}; one run has one numbering, so two files that \
                 disagree about where counting starts would be mixed together silently"
            ))),
            _ => Ok(()),
        }
    }

    /// Record the numbering this run reads node indices in.
    ///
    /// Call only once every fallible step of a load has succeeded.
    fn commit_min_node_index(&mut self, min_node_index: i64) {
        self.min_node_index = Some(min_node_index);
    }
}

/// Put an evolved edge list back into the caller's own numbering.
fn shift_out(edges: &mut [(usize, usize, u32)], min_node_index: Option<i64>) {
    let shift = match min_node_index {
        Some(0) | None => return,
        Some(shift) => shift,
    };

    for edge in edges.iter_mut() {
        // Every index here came from the engine, so it is within
        // `0..network_size` and shifting it back lands where it came from.
        edge.0 = (edge.0 as i64 + shift) as usize;
        edge.1 = (edge.1 as i64 + shift) as usize;
    }
}

/// What a Rust-native run hands back — the [`PyRunResult`] fields, without the
/// pyo3 wrapper. See [`run_from_toml`].
pub struct RunSummary {
    /// Best fitness found, **as-measured**: the units and sign the objective
    /// returned, not the lower-is-better form the engine compares.
    pub best_fitness: f64,
    /// The best individual's expressed network, as `(u, v, multiplicity)`.
    pub best_edges: Vec<(usize, usize, u32)>,
    /// How many nodes that network has, isolated ones included. Stated rather
    /// than counted from `best_edges`, which cannot see an isolated node.
    pub num_nodes: usize,
    /// The best individual's genome, via `Genome::print`.
    pub best_genome_repr: String,
    /// The convergence log, one row per logged iteration.
    pub history: Vec<GenerationStats>,
    /// The master seed the run was made with. Private: `save_logs` stamps it
    /// onto every row, which is the only way it is meant to be read.
    seed: u64,
    /// Which replicate this is, `0`-based. Private for the same reason as
    /// `seed`.
    run_index: usize,
    /// The TOML document this run's config was parsed from. Private:
    /// `save_results` writes it beside the results as `config.toml`.
    config_toml: String,
}

impl RunSummary {
    /// Write the convergence log to `filename` as CSV.
    ///
    /// The same columns as the Python route's `save_logs`, `seed` and
    /// `run_index` included, so logs from either front end concatenate into one
    /// file and stay separable.
    pub fn save_logs(&self, filename: &str) -> std::io::Result<()> {
        use std::io::Write;

        let mut file = std::fs::File::create(filename)?;
        writeln!(
            file,
            "iteration,best_fitness,mean_fitness,std_dev,ci_95,seed,run_index"
        )?;
        for row in &self.history {
            writeln!(
                file,
                "{},{},{},{},{},{},{}",
                row.iteration,
                row.best_fitness,
                row.mean_fitness,
                row.std_dev,
                row.ci_95,
                self.seed,
                self.run_index,
            )?;
        }
        Ok(())
    }

    /// Write the best individual to `filename`, and the run's config TOML
    /// beside it as `config.toml`.
    pub fn save_results(&self, filename: &str) -> std::io::Result<()> {
        use std::io::Write;

        let mut file = std::fs::File::create(filename)?;
        write!(
            file,
            "{}",
            py_result::as_comment("best_fitness", &self.best_fitness.to_string())
        )?;
        write!(
            file,
            "{}",
            py_result::as_comment("genome", &self.best_genome_repr)
        )?;
        writeln!(file, "# nodes = {}", self.num_nodes)?;
        for &(u, v, weight) in &self.best_edges {
            writeln!(file, "{u},{v},{weight}")?;
        }

        // Beside the results file, not derived from its name: several replicates
        // written into one folder share the config that produced them, so one
        // `config.toml` per folder is the record.
        let config_path = Path::new(filename).with_file_name("config.toml");
        std::fs::write(config_path, &self.config_toml)?;
        Ok(())
    }
}

/// One seed per replicate, derived from a master seed.
///
/// A replicate's seed depends on the master and on its own index, never on how
/// many replicates were asked for. A library caller driving its own loop wants
/// these rather than seeds of its own invention, or its replicates will not line
/// up with the same master seed run through `get-run`.
pub fn replicate_seeds(master: u64, n_runs: usize) -> Vec<u64> {
    dispatch::replicate_seeds(master, n_runs)
}

/// `YYYYmmdd-HHMMSS` in UTC, for naming a run's output directory.
///
/// UTC rather than local time, so directories from two machines sort into the
/// order the runs actually happened.
pub fn utc_stamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);

    let days = (secs / 86_400) as i64;
    let time_of_day = secs % 86_400;

    // Shift the epoch to 0000-03-01, which puts the leap day at the end of the
    // year and makes the month arithmetic below a single linear formula.
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;

    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);

    format!(
        "{year:04}{month:02}{day:02}-{:02}{:02}{:02}",
        time_of_day / 3_600,
        (time_of_day % 3_600) / 60,
        time_of_day % 60,
    )
}

/// Where one replicate's files belong. Returns a path and creates nothing —
/// every caller runs `create_dir_all` on the result itself.
///
/// `None` for `out_dir` means the working directory under fixed names. It
/// ignores `run_index`, so it is only correct for a single replicate — `get-run`
/// rejects `--runs N` above 1 without `--out` for that reason.
///
/// `stamp` is passed in rather than read here: every replicate of one invocation
/// belongs in the same timestamped directory, and reading the clock per
/// replicate would scatter them the moment a run crossed a second boundary.
///
/// Run folders count from one and are zero-padded to the width of `n_runs`, so
/// ten replicates sort as `run_01` through `run_10` rather than `run_1`,
/// `run_10`, `run_2` — in a shell, a file browser, or a glob. `run_index`
/// itself stays zero-based: it is half of the `(seed, run_index)` pair that
/// reproduces a replicate, and renumbering it would invite asking for the
/// wrong one.
pub fn run_output_dir(
    out_dir: Option<&str>,
    stamp: &str,
    seed: u64,
    run_index: usize,
    n_runs: usize,
) -> PathBuf {
    let Some(root) = out_dir else {
        return PathBuf::from(".");
    };

    let mut path = Path::new(root).join(format!("{stamp}-{seed}"));
    if n_runs > 1 {
        let width = n_runs.to_string().len();
        path = path.join(format!("run_{:0width$}", run_index + 1, width = width));
    }
    path
}

/// Rust-native entry point: run a `config.toml` with no Python interpreter.
///
/// Follows the same steps a Python caller's [`GraphEvolver`] does — parse,
/// validate, erase the objective, dispatch — so the two differ only in front
/// end.
///
/// # Errors
///
/// The config's own parse or validate error, or `[fitness] type = "python"`,
/// which has no callable to call on this route.
pub fn run_from_toml(config_path: &str, seed: u64) -> Result<RunSummary, String> {
    let mut summaries = run_many_from_toml(config_path, seed, 1)?;
    Ok(summaries.remove(0))
}

/// The same run, `n_runs` times from one master seed.
///
/// Mirrors [`GraphEvolver::run`]'s replicate handling, so the Rust and Python
/// front ends produce the same numbers for the same `(seed, run_index)` pair —
/// the master seed and the replicate's index, which is what reproduces it.
///
/// Runs sequentially, unlike the Python path.
///
/// # Errors
///
/// The config's own parse or validate error, `n_runs` of zero, or
/// `[fitness] type = "python"` with nothing to call it.
pub fn run_many_from_toml(
    config_path: &str,
    seed: u64,
    n_runs: usize,
) -> Result<Vec<RunSummary>, String> {
    if n_runs == 0 {
        return Err(
            "n_runs must be at least 1; asking for zero runs returns nothing and is \
                    more likely a mistake than an intent"
                .to_string(),
        );
    }

    let text = std::fs::read_to_string(config_path)
        .map_err(|err| format!("failed to load config: {}", ConfigError::Io(err)))?;
    let config =
        Config::from_toml_str(&text).map_err(|err| format!("failed to load config: {err}"))?;
    config
        .validate()
        .map_err(|err| format!("failed to load config: {err}"))?;

    // `None` for the GIL: this route never acquires one, so a load warning is
    // printed to stderr rather than raised as a `UserWarning`.
    let base_graph =
        config_base_graph(None, &config, Path::new(config_path)).map_err(|err| err.to_string())?;

    let evolver = GraphEvolver {
        config,
        fitness_function: None,
        base_graph,
        min_node_index: None,
        struct_match_reference: OnceLock::new(),
        config_toml: text.clone(),
    };

    let seeds = dispatch::replicate_seeds(seed, n_runs);

    let mut summaries = Vec::with_capacity(n_runs);
    for (run_index, &run_seed) in seeds.iter().enumerate() {
        let fitness = evolver
            .objective(run_seed, None)
            .map_err(|err| err.to_string())?;
        let outcome = dispatch::evolve(
            &evolver.config,
            &fitness,
            evolver.base_graph.as_ref(),
            run_seed,
        )
        .map_err(|err| err.to_string())?;

        summaries.push(RunSummary {
            best_fitness: outcome.best_fitness,
            best_edges: outcome.best_edges,
            num_nodes: outcome.num_nodes,
            best_genome_repr: outcome.best_genome_repr,
            history: outcome.history,
            seed,
            run_index,
            config_toml: text.clone(),
        });
    }

    Ok(summaries)
}

/// Evolve networks against an objective you choose.
///
/// Describe a run either way, and both go through the same validation:
///
/// - **Config objects.** Build `Config` from the typed pieces — `EvolutionConfig`,
///   `SelectionConfig`, `GenomeConfig`, `FitnessConfig` and the rest — then
///   `GraphEvolver.from_config(config)`.
/// - **A TOML file.** `GraphEvolver(path)`, the constructor. `config.to_toml()`
///   returns the document that was actually parsed, so a run reproduces verbatim.
///
/// `run()` returns a `RunResult`: the best graph found, its fitness **as the
/// objective measured it**, and a `GenerationStats` row per logged generation.
///
/// Full documentation, including every configuration key:
/// https://md12ol.github.io/GraphEvolutionTool/
#[pymodule]
fn get(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<GraphEvolver>()?;
    // The `Py` prefix is a Rust-side disambiguator: each class carries a `name`
    // attribute, and that unprefixed name is what Python sees.
    m.add_class::<PyConfig>()?;
    m.add_class::<PyEvolutionConfig>()?;
    m.add_class::<PyReplacementConfig>()?;
    m.add_class::<PyScopeConfig>()?;
    m.add_class::<PySelectionConfig>()?;
    m.add_class::<PyCrossoverConfig>()?;
    m.add_class::<PyEdgeEditMutationConfig>()?;
    m.add_class::<PySdaMutationConfig>()?;
    m.add_class::<PyGenomeConfig>()?;
    m.add_class::<PyFitnessConfig>()?;
    m.add_class::<PySirParams>()?;
    m.add_class::<PyOperationWeights>()?;
    m.add_class::<PyRunResult>()?;
    m.add_class::<PyGenerationStats>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::fitness::Fitness;
    use crate::graph::Graph;

    #[test]
    fn a_single_replicate_gets_no_run_folder() {
        let path = run_output_dir(Some("out"), "20260828-071233", 7, 0, 1);
        assert_eq!(path, PathBuf::from("out/20260828-071233-7"));
    }

    #[test]
    fn run_folders_count_from_one_while_run_index_stays_zero_based() {
        let first = run_output_dir(Some("out"), "20260828-071233", 7, 0, 2);
        assert_eq!(first, PathBuf::from("out/20260828-071233-7/run_1"));
    }

    #[test]
    fn run_folders_are_padded_to_the_width_of_n_runs_so_ten_replicates_sort() {
        let mut names = Vec::new();
        for run_index in 0..10 {
            let path = run_output_dir(Some("out"), "20260828-071233", 7, run_index, 10);
            names.push(path.file_name().unwrap().to_str().unwrap().to_string());
        }
        assert_eq!(names[0], "run_01");
        assert_eq!(names[9], "run_10");

        // The property the padding exists for: lexical order is run order.
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(sorted, names);
    }

    #[test]
    fn without_an_output_directory_every_replicate_shares_the_working_directory() {
        // Why `get-run` rejects `--runs N` above 1 without `--out`: the paths
        // collide rather than being distinguished by the run index.
        let first = run_output_dir(None, "20260828-071233", 7, 0, 5);
        let second = run_output_dir(None, "20260828-071233", 7, 1, 5);
        assert_eq!(first, PathBuf::from("."));
        assert_eq!(first, second);
    }

    /// A config whose `[fitness]` block is exactly `fitness_block`, with the
    /// rest of the document held fixed.
    ///
    /// Written out rather than reaching for `config.rs`'s fixtures, which are
    /// private to its own test module.
    fn config_with(fitness_block: &str) -> Config {
        // Top-level keys first: in TOML anything after a `[table]` header
        // belongs to that table, so the shared settings cannot follow one.
        let text = format!(
            "population_size = 10\n\
             network_size = 8\n\
             max_edge_multiplicity = 1\n\
             crossover_rate = 0.8\n\
             mutation_rate = 0.2\n\
             \n\
             [evolution]\n\
             type = \"generational\"\n\
             num_generations = 5\n\
             elite_count = 1\n\
             \n\
             [scope]\n\
             type = \"global\"\n\
             \n\
             [selection]\n\
             type = \"tournament\"\n\
             tournament_size = 4\n\
             \n\
             [genome]\n\
             type = \"edge_edit\"\n\
             gene_length = 16\n\
             \n\
             {fitness_block}"
        );
        Config::from_toml_str(&text).expect("the test config parses")
    }

    fn evolver_with(fitness_block: &str) -> GraphEvolver {
        GraphEvolver {
            config: config_with(fitness_block),
            fitness_function: None,
            base_graph: None,
            min_node_index: None,
            struct_match_reference: OnceLock::new(),
            config_toml: String::new(),
        }
    }

    /// `evolver_with`, with a wider edge cap so a test can tell two
    /// multiplicities apart. The cap is set after parsing rather than in the
    /// document because every other test wants the simple-graph default.
    fn evolver_with_cap(fitness_block: &str, max_edge_multiplicity: u32) -> GraphEvolver {
        let mut evolver = evolver_with(fitness_block);
        evolver.config.max_edge_multiplicity = max_edge_multiplicity;
        evolver
    }

    const PYTHON_FITNESS: &str = "[fitness]\ntype = \"python\"\n";
    const SIR_FITNESS: &str =
        "[fitness]\ntype = \"epi_spread\"\ninfection_rate = 0.05\nnum_epidemics = 30\n";
    /// `lambda batch: [float(n) for (n, edges) in batch]`, as a bound object.
    fn scoring_lambda(py: Python<'_>) -> Bound<'_, PyAny> {
        py.eval(
            c"lambda batch: [float(n) for (n, edges) in batch]",
            None,
            None,
        )
        .expect("the lambda compiles")
    }

    /// Run `body` with Python's warnings collected, and hand back their text.
    ///
    /// `catch_warnings(record=True)` is entered and exited by hand because the
    /// call under test is Rust, not Python, so it cannot sit inside a `with`
    /// block. `simplefilter("always")` defeats the once-per-location de-duping
    /// that would otherwise hide a warning a previous test already raised.
    fn warnings_from(py: Python<'_>, body: impl FnOnce()) -> Vec<String> {
        let scope = pyo3::types::PyDict::new(py);

        py.run(
            c"import warnings\n\
              recorder = warnings.catch_warnings(record=True)\n\
              caught = recorder.__enter__()\n\
              warnings.simplefilter('always')",
            None,
            Some(&scope),
        )
        .expect("the recorder starts");

        body();

        py.run(
            c"recorder.__exit__(None, None, None)\n\
              messages = [str(entry.message) for entry in caught]",
            None,
            Some(&scope),
        )
        .expect("the recorder stops");

        scope
            .get_item("messages")
            .expect("reading the collected messages")
            .expect("the recorder left messages behind")
            .extract()
            .expect("they are strings")
    }

    #[test]
    fn a_repeated_base_graph_edge_warns_and_the_last_one_wins() {
        // The bug this covers: the edge list used to be applied in order, so a
        // pair given twice kept whichever came last with nothing said about it.
        Python::attach(|py| {
            let mut evolver = evolver_with_cap(PYTHON_FITNESS, 3);

            let messages = warnings_from(py, || {
                evolver
                    .set_base_graph(py, 8, vec![(2, 5, 1), (0, 1, 2), (5, 2, 3)], 0)
                    .expect("a valid base graph");
            });

            assert!(
                messages
                    .iter()
                    .any(|message| message.contains("appears more than once")),
                "{messages:?}"
            );

            let graph = evolver.base_graph.expect("the graph was stored");
            // Canonical comparison: (5, 2) is the same edge as (2, 5), so the
            // later weight replaces the earlier one rather than adding a second.
            assert_eq!(graph.weight(2, 5), 3);
            assert_eq!(graph.get_edge_list().len(), 2);
        });
    }

    #[test]
    fn a_base_graph_with_no_repeats_says_nothing() {
        Python::attach(|py| {
            let mut evolver = evolver_with_cap(PYTHON_FITNESS, 3);

            let messages = warnings_from(py, || {
                evolver
                    .set_base_graph(py, 8, vec![(0, 1, 1), (2, 3, 2)], 0)
                    .expect("a valid base graph");
            });

            assert!(messages.is_empty(), "{messages:?}");
        });
    }

    /// An SDA genome block, parsed rather than built, so its defaulted fields
    /// come from the same place a user's config would get them.
    fn sda_genome() -> GenomeConfig {
        let text = "[genome]\ntype = \"sda\"\nnum_states = 5\nmax_resp_len = 3\n";
        let parsed: toml::Value = toml::from_str(text).expect("the genome block parses");
        parsed["genome"]
            .clone()
            .try_into()
            .expect("it is a genome config")
    }

    /// Write `text` to a uniquely named file under the temp directory,
    /// appending the `# nodes` header the loader requires.
    ///
    /// Eight, because that is the `network_size` every fixture config here
    /// uses, and a base-graph file that disagrees with the run is rejected.
    /// Appended rather than prepended so a fixture's rows keep their line
    /// numbers. [`file_holding_stating`] is for tests about that check itself.
    fn file_holding(name: &str, text: &str) -> std::path::PathBuf {
        file_holding_stating(name, 8, text)
    }

    /// [`file_holding`], for a file that states some other node count.
    fn file_holding_stating(name: &str, nodes: usize, text: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("get_lib_{name}.csv"));
        std::fs::write(&path, format!("{text}# nodes = {nodes}\n"))
            .expect("the fixture is written");
        path
    }

    /// A directory holding a config that names `base_graph.csv`, and the file
    /// itself, the way a downloaded folder of examples is laid out.
    ///
    /// Under a uniquely named directory so the *relative* path in the config
    /// can only resolve one way. That is the property under test: the working
    /// directory during `cargo test` is the crate root, where no such file
    /// exists, so a test that passes has resolved against the config.
    fn folder_holding(name: &str, config_body: &str, graph: Option<&str>) -> std::path::PathBuf {
        let directory = std::env::temp_dir().join(format!("get_base_graph_{name}"));
        std::fs::create_dir_all(&directory).expect("the fixture directory is made");
        let config = directory.join("config.toml");
        std::fs::write(&config, config_body).expect("the config is written");
        if let Some(text) = graph {
            std::fs::write(directory.join("base_graph.csv"), text).expect("the graph is written");
        }
        config
    }

    /// [`config_with`], with extra lines appended to its `[genome]` block.
    ///
    /// The block is the last thing `config_with` writes before the fitness
    /// block, so a `[genome]` key has to be spliced in rather than appended.
    fn config_text_with_genome_keys(extra: &str) -> String {
        let text = format!(
            "population_size = 10\nnetwork_size = 8\nmax_edge_multiplicity = 1\n\
             crossover_rate = 0.8\nmutation_rate = 0.2\n\n\
             [evolution]\ntype = \"generational\"\nnum_generations = 5\nelite_count = 1\n\n\
             [scope]\ntype = \"global\"\n\n\
             [selection]\ntype = \"tournament\"\ntournament_size = 4\n\n\
             [genome]\ntype = \"edge_edit\"\ngene_length = 16\n{extra}\n\n{PYTHON_FITNESS}"
        );
        text
    }

    #[test]
    fn a_config_naming_a_base_graph_loads_it_from_beside_the_config() {
        let config_path = folder_holding(
            "beside",
            &config_text_with_genome_keys("base_graph = \"base_graph.csv\""),
            Some("0,1,1\n1,2,1\n# nodes = 8\n"),
        );
        let config = Config::from_toml_str(
            &std::fs::read_to_string(&config_path).expect("the fixture reads"),
        )
        .expect("the fixture parses");

        let graph = config_base_graph(None, &config, &config_path)
            .expect("the file is there and agrees with the run")
            .expect("a base graph was named, so one comes back");

        // Read as 0-indexed, which is what this route promises and the only
        // numbering it can express.
        assert_eq!(graph.get_edge_list(), vec![(0, 1, 1), (1, 2, 1)]);
    }

    #[test]
    fn a_config_naming_no_base_graph_starts_from_the_empty_graph() {
        let config = config_with(PYTHON_FITNESS);
        let resolved = config_base_graph(None, &config, std::path::Path::new("config.toml"))
            .expect("naming nothing is not an error");
        assert!(resolved.is_none());
    }

    #[test]
    fn an_sda_config_cannot_name_a_base_graph() {
        // SDA generates its graph rather than editing one, so the key is not on
        // its config at all. `deny_unknown_fields` is what refuses it — no
        // check of ours runs, and this pins that the free rejection is real.
        let text = "[genome]\ntype = \"sda\"\nnum_states = 5\nmax_resp_len = 3\n\
                    base_graph = \"base_graph.csv\"\n";
        let parsed: toml::Value = toml::from_str(text).expect("it is still valid TOML");
        let genome: Result<GenomeConfig, _> = parsed["genome"].clone().try_into();
        let message = genome
            .expect_err("an SDA genome has no base_graph")
            .to_string();
        assert!(message.contains("base_graph"), "{message}");
    }

    #[test]
    fn a_base_graph_whose_header_disagrees_with_network_size_is_rejected() {
        let config_path = folder_holding(
            "disagrees",
            &config_text_with_genome_keys("base_graph = \"base_graph.csv\""),
            // The run is 8 nodes; a file saying 6 would otherwise load as 8,
            // padded with two isolated nodes nobody asked for.
            Some("0,1,1\n# nodes = 6\n"),
        );
        let config = Config::from_toml_str(
            &std::fs::read_to_string(&config_path).expect("the fixture reads"),
        )
        .expect("the fixture parses");

        Python::attach(|_| {
            let message = config_base_graph(None, &config, &config_path)
                .expect_err("6 is not 8")
                .to_string();
            assert!(message.contains("# nodes = 6"), "{message}");
            assert!(message.contains("network_size is 8"), "{message}");
        });
    }

    #[test]
    fn a_base_graph_the_config_names_but_the_folder_lacks_says_which_path() {
        let config_path = folder_holding(
            "missing",
            &config_text_with_genome_keys("base_graph = \"base_graph.csv\""),
            None,
        );
        let config = Config::from_toml_str(
            &std::fs::read_to_string(&config_path).expect("the fixture reads"),
        )
        .expect("the fixture parses");

        Python::attach(|_| {
            let message = config_base_graph(None, &config, &config_path)
                .expect_err("the file is not there")
                .to_string();
            // The resolved path, not the bare name the config wrote: a reader
            // needs to know where it was looked for.
            assert!(message.contains("get_base_graph_missing"), "{message}");
            assert!(message.contains("base_graph.csv"), "{message}");
        });
    }

    #[test]
    fn an_absolute_base_graph_path_is_taken_as_given() {
        // Written into one directory and named absolutely from a config in
        // another, so resolving against the config's folder would miss it.
        let elsewhere = folder_holding("absolute_graph", "unused", Some("0,1,1\n# nodes = 8\n"));
        let graph_path = elsewhere
            .parent()
            .expect("the fixture has a directory")
            .join("base_graph.csv");
        let config_path = folder_holding(
            "absolute_config",
            &config_text_with_genome_keys(&format!("base_graph = \"{}\"", graph_path.display())),
            None,
        );
        let config = Config::from_toml_str(
            &std::fs::read_to_string(&config_path).expect("the fixture reads"),
        )
        .expect("the fixture parses");

        let graph = config_base_graph(None, &config, &config_path)
            .expect("an absolute path needs no resolving")
            .expect("a base graph was named");
        assert_eq!(graph.get_edge_list(), vec![(0, 1, 1)]);
    }

    #[test]
    fn a_base_graph_file_is_shifted_to_zero_on_the_way_in() {
        Python::attach(|py| {
            let mut evolver = evolver_with(PYTHON_FITNESS);
            let path = file_holding("shift_in", "1,2,1\n2,3,1\n");

            evolver
                .set_base_graph_from_file(py, path.display().to_string(), 1)
                .expect("a 1-indexed file the config's size accepts");

            // The engine only ever sees 0-based indices, whatever the file said.
            let graph = evolver.base_graph.expect("the graph was stored");
            assert_eq!(graph.get_edge_list(), vec![(0, 1, 1), (1, 2, 1)]);

            std::fs::remove_file(&path).expect("cleanup");
        });
    }

    #[test]
    fn a_run_hands_results_back_in_the_callers_numbering() {
        Python::attach(|py| {
            let mut evolver = evolver_with(PYTHON_FITNESS);
            let path = file_holding("round_trip", "1,2,1\n2,3,1\n");

            evolver
                .set_base_graph_from_file(py, path.display().to_string(), 1)
                .expect("a 1-indexed file");

            // Maximizing edge count, so the best individual has edges to look at.
            let objective = py
                .eval(
                    c"lambda batch: [float(len(edges)) for (n, edges) in batch]",
                    None,
                    None,
                )
                .expect("the lambda compiles");
            evolver
                .set_fitness_function(&objective, "maximize")
                .expect("registering a callable on a python config");

            let results = evolver.run(py, 1, 1, Some(1)).expect("the run completes");
            let edges = &results[0].best_edges;

            assert!(!edges.is_empty(), "an edge-maximizing run found no edges");
            for &(u, v, _) in edges {
                // 1-indexed in, 1-indexed out: node 0 does not exist for this
                // caller, and node 8 — the top of an 8-node network — does.
                assert!((1..=8).contains(&u) && (1..=8).contains(&v), "({u}, {v})");
            }

            std::fs::remove_file(&path).expect("cleanup");
        });
    }

    #[test]
    fn two_loaders_disagreeing_about_where_counting_starts_are_rejected() {
        Python::attach(|py| {
            let mut evolver = evolver_with(PYTHON_FITNESS);
            let path = file_holding("disagree", "1,2,1\n");

            evolver
                .set_base_graph_from_file(py, path.display().to_string(), 1)
                .expect("the first load sets the numbering");

            let err = evolver
                .set_base_graph_from_file(py, path.display().to_string(), 0)
                .expect_err("a second numbering must be rejected");

            assert!(
                err.to_string().contains("one run has one numbering"),
                "{err}"
            );

            std::fs::remove_file(&path).expect("cleanup");
        });
    }

    #[test]
    fn a_rejected_row_names_its_file_and_line_through_the_setter() {
        Python::attach(|py| {
            let mut evolver = evolver_with(PYTHON_FITNESS);
            let path = file_holding("bad_row", "0,1,1\n3,3,1\n");

            let err = evolver
                .set_base_graph_from_file(py, path.display().to_string(), 0)
                .expect_err("a self-loop must be rejected");

            let message = err.to_string();
            assert!(message.contains("line 2"), "{message}");
            assert!(message.contains("self-loop"), "{message}");
            // Nothing is stored when the file is rejected.
            assert!(evolver.base_graph.is_none());

            std::fs::remove_file(&path).expect("cleanup");
        });
    }

    #[test]
    fn a_one_indexed_edge_list_is_shifted_the_same_way_a_file_would_be() {
        Python::attach(|py| {
            let mut evolver = evolver_with(PYTHON_FITNESS);

            // The use case: the caller's dataset numbers nodes from 1, and they
            // should not have to renumber it to hand it over.
            evolver
                .set_base_graph(py, 8, vec![(1, 2, 1), (3, 8, 1)], 1)
                .expect("1-indexed edges are accepted");

            // Stored 0-based, so the engine sees an ordinary graph.
            let graph = evolver.base_graph.as_ref().expect("a base graph is stored");
            assert_eq!(graph.weight(0, 1), 1);
            assert_eq!(graph.weight(2, 7), 1);
            assert_eq!(graph.weight(1, 2), 0, "indices were shifted, not reused");

            // And the run's numbering is now 1, so output goes back the way it came.
            assert_eq!(evolver.min_node_index, Some(1));
            let mut edges = vec![(0, 1, 1), (2, 7, 1)];
            shift_out(&mut edges, evolver.min_node_index);
            assert_eq!(edges, vec![(1, 2, 1), (3, 8, 1)]);
        });
    }

    #[test]
    fn an_edge_below_the_callers_first_index_is_named_as_the_caller_wrote_it() {
        Python::attach(|py| {
            let mut evolver = evolver_with(PYTHON_FITNESS);

            // 0 is out of range for 1-indexed data, and shifting it would
            // underflow, so the range check has to catch it first.
            let err = evolver
                .set_base_graph(py, 8, vec![(0, 1, 1)], 1)
                .expect_err("node 0 is outside 1..=8");

            let message = err.to_string();
            assert!(message.contains("(0, 1)"), "{message}");
            assert!(message.contains("1..=8"), "{message}");
            assert!(evolver.base_graph.is_none());
            assert_eq!(evolver.min_node_index, None, "a rejection records nothing");
        });
    }

    #[test]
    fn a_list_setter_and_a_loader_that_disagree_about_numbering_are_rejected() {
        Python::attach(|py| {
            let mut evolver = evolver_with(PYTHON_FITNESS);
            let folder = std::env::temp_dir().join("get_lib_mixed_numbering");
            let _ = std::fs::remove_dir_all(&folder);
            std::fs::create_dir_all(&folder).expect("temp folder");
            std::fs::write(folder.join("a.csv"), "# nodes = 3\n1,2,1\n")
                .expect("the fixture is written");

            // The case that used to pass silently and return edges numbered to
            // match neither input.
            evolver
                .set_base_graph(py, 8, vec![(0, 1, 1)], 0)
                .expect("a 0-indexed list declares numbering 0");

            let err = evolver
                .load_reference_graphs(py, folder.display().to_string(), 1)
                .expect_err("a 1-indexed reference set disagrees with it");
            assert!(
                err.to_string().contains("one run has one numbering"),
                "{err}"
            );

            // The numbering the base graph declared still stands, so output is
            // not shifted by the call that failed.
            assert_eq!(evolver.min_node_index, Some(0));

            std::fs::remove_dir_all(&folder).expect("cleanup");
        });
    }

    #[test]
    fn a_failed_load_leaves_the_numbering_untouched() {
        Python::attach(|py| {
            let mut evolver = evolver_with(PYTHON_FITNESS);

            // A path that does not exist, so the load fails after the numbering
            // has been checked but before anything is read.
            let err = evolver
                .set_base_graph_from_file(py, "no_such_file.csv".to_string(), 1)
                .expect_err("a missing file must be rejected");
            assert!(
                !err.to_string().contains("one run has one numbering"),
                "{err}"
            );

            // Nothing observable changed. The numbering is the one that bites:
            // `shift_out` reads it on every later run, so a value left behind
            // here renumbers output for a run with no base graph at all.
            assert_eq!(evolver.min_node_index, None);
            assert!(evolver.base_graph.is_none());

            let mut edges = vec![(0, 1, 1)];
            shift_out(&mut edges, evolver.min_node_index);
            assert_eq!(
                edges,
                vec![(0, 1, 1)],
                "a failed load must not shift output"
            );

            // And a later load is still free to choose any numbering, rather
            // than being rejected by the one the failed call would have pinned.
            let path = file_holding("after_failure", "0,1,1\n");
            evolver
                .set_base_graph_from_file(py, path.display().to_string(), 0)
                .expect("a numbering is still free to be chosen after a failed load");
            assert_eq!(evolver.min_node_index, Some(0));

            std::fs::remove_file(&path).expect("cleanup");
        });
    }

    #[test]
    fn an_sda_config_rejects_a_base_graph_file_as_it_does_a_base_graph() {
        Python::attach(|py| {
            let mut evolver = evolver_with(PYTHON_FITNESS);
            evolver.config.genome = sda_genome();

            let err = evolver
                .set_base_graph_from_file(py, "unread.csv".to_string(), 0)
                .expect_err("an SDA run has no base graph to seed");

            // Rejected before the file is opened: a missing file must not be
            // what this reports, or the real problem is hidden.
            assert!(err.to_string().contains("generates its graph"), "{err}");
        });
    }

    #[test]
    fn reference_graphs_come_back_named_and_in_sorted_order() {
        Python::attach(|py| {
            let folder = std::env::temp_dir().join("get_lib_references");
            let _ = std::fs::remove_dir_all(&folder);
            std::fs::create_dir_all(&folder).expect("temp folder");
            for (name, text) in [
                ("b.csv", "# nodes = 4\n2,3,1\n"),
                ("a.csv", "# nodes = 3\n1,2,1\n"),
            ] {
                std::fs::write(folder.join(name), text).expect("the fixture is written");
            }

            let mut evolver = evolver_with(PYTHON_FITNESS);
            let graphs = evolver
                .load_reference_graphs(py, folder.display().to_string(), 1)
                .expect("a folder of 1-indexed files");

            assert!(graphs[0].0.ends_with("a.csv"), "{}", graphs[0].0);
            assert!(graphs[1].0.ends_with("b.csv"), "{}", graphs[1].0);
            // Shifted on the way in like any other loaded graph, and nothing
            // shifts them back — a reference set is input only.
            assert_eq!(graphs[0].2, vec![(0, 1, 1)]);
            assert_eq!(graphs[1].2, vec![(1, 2, 1)]);
            // Each file's own header, not the run's `network_size`: a
            // reference set is real data and its graphs differ in size.
            assert_eq!((graphs[0].1, graphs[1].1), (3, 4));
            // Nothing is stored: the caller is the reader until an objective is.
            assert!(evolver.base_graph.is_none());

            std::fs::remove_dir_all(&folder).expect("cleanup");
        });
    }

    #[test]
    fn a_reference_folder_shares_the_base_graphs_numbering() {
        Python::attach(|py| {
            let folder = std::env::temp_dir().join("get_lib_ref_disagree");
            let _ = std::fs::remove_dir_all(&folder);
            std::fs::create_dir_all(&folder).expect("temp folder");
            std::fs::write(folder.join("a.csv"), "# nodes = 3\n1,2,1\n")
                .expect("the fixture is written");

            let mut evolver = evolver_with(PYTHON_FITNESS);
            let path = file_holding("ref_base", "1,2,1\n");
            evolver
                .set_base_graph_from_file(py, path.display().to_string(), 1)
                .expect("the base graph sets the numbering");

            let err = evolver
                .load_reference_graphs(py, folder.display().to_string(), 0)
                .expect_err("a second numbering must be rejected");

            assert!(
                err.to_string().contains("one run has one numbering"),
                "{err}"
            );

            std::fs::remove_file(&path).expect("cleanup");
            std::fs::remove_dir_all(&folder).expect("cleanup");
        });
    }

    /// The objective reads a reference folder under a sanity bound, not
    /// under `network_size`, and this call has to agree with it or a set a
    /// run scores against cannot be inspected from Python.
    #[test]
    fn a_reference_graph_larger_than_network_size_is_read() {
        Python::attach(|py| {
            let mut evolver = evolver_with(PYTHON_FITNESS);
            let folder = std::env::temp_dir().join("get_lib_big_reference");
            let _ = std::fs::remove_dir_all(&folder);
            std::fs::create_dir_all(&folder).expect("temp folder");

            // 50 against the fixtures' network_size of 8 — the ordinary case,
            // since reference data is matched on normalized distributions.
            std::fs::write(folder.join("a.csv"), "# nodes = 50\n0,49,1\n")
                .expect("the fixture is written");

            let graphs = evolver
                .load_reference_graphs(py, folder.display().to_string(), 0)
                .expect("a reference graph may be larger than the evolved network");

            assert_eq!(graphs.len(), 1);
            assert_eq!(graphs[0].1, 50, "the file's own node count comes back");

            std::fs::remove_dir_all(&folder).expect("cleanup");
        });
    }

    /// The other half of the sanity bound: raising the cap is not removing
    /// it. A file indexed the wrong way — a global dataset index, say — still
    /// fails.
    #[test]
    fn a_reference_graph_above_the_sanity_bound_is_still_rejected() {
        Python::attach(|py| {
            let mut evolver = evolver_with(PYTHON_FITNESS);
            let folder = std::env::temp_dir().join("get_lib_absurd_reference");
            let _ = std::fs::remove_dir_all(&folder);
            std::fs::create_dir_all(&folder).expect("temp folder");

            let nodes = dispatch::MAX_REFERENCE_NODES + 1;
            std::fs::write(folder.join("a.csv"), format!("# nodes = {nodes}\n0,1,1\n"))
                .expect("the fixture is written");

            let err = evolver
                .load_reference_graphs(py, folder.display().to_string(), 0)
                .expect_err("above the sanity bound is still an error");
            assert!(
                err.to_string().contains(&nodes.to_string()),
                "the message names the count it rejected: {err}"
            );

            std::fs::remove_dir_all(&folder).expect("cleanup");
        });
    }

    #[test]
    fn shift_out_is_a_no_op_without_a_loader() {
        let mut edges = vec![(0, 1, 1), (2, 3, 2)];

        shift_out(&mut edges, None);
        assert_eq!(edges, vec![(0, 1, 1), (2, 3, 2)]);

        shift_out(&mut edges, Some(0));
        assert_eq!(edges, vec![(0, 1, 1), (2, 3, 2)]);

        shift_out(&mut edges, Some(1));
        assert_eq!(edges, vec![(1, 2, 1), (3, 4, 2)]);
    }

    #[test]
    fn a_registered_callable_becomes_the_objective() {
        Python::attach(|py| {
            let mut evolver = evolver_with(PYTHON_FITNESS);

            evolver
                .set_fitness_function(&scoring_lambda(py), "maximize")
                .expect("registering a callable on a python config");

            let objective = evolver
                .fitness_function
                .as_ref()
                .expect("the callable was stored");

            // Reached through the objective, not just stored: this is the same
            // path a run would take.
            assert_eq!(objective.direction(), Direction::Maximize);
            assert_eq!(objective.evaluate(&Graph::new(6, 1)), 6.0);
        });
    }

    #[test]
    fn the_direction_string_is_case_insensitive_but_closed() {
        Python::attach(|py| {
            for (spelling, expected) in [
                ("minimize", Direction::Minimize),
                ("MINIMIZE", Direction::Minimize),
                ("Maximize", Direction::Maximize),
            ] {
                let mut evolver = evolver_with(PYTHON_FITNESS);
                evolver
                    .set_fitness_function(&scoring_lambda(py), spelling)
                    .unwrap_or_else(|_| panic!("{spelling} should be accepted"));

                assert_eq!(
                    evolver.fitness_function.as_ref().unwrap().direction(),
                    expected,
                );
            }
        });
    }

    #[test]
    fn an_unrecognized_direction_is_rejected_and_says_what_is_allowed() {
        Python::attach(|py| {
            let mut evolver = evolver_with(PYTHON_FITNESS);

            // British spelling: the most likely near-miss, and silently
            // defaulting it to Minimize would run a maximizing search backwards.
            let err = evolver
                .set_fitness_function(&scoring_lambda(py), "maximise")
                .expect_err("an unknown direction must be rejected");

            let message = err.to_string();
            assert!(
                message.contains("maximise"),
                "names what was given: {message}"
            );
            assert!(
                message.contains("\"maximize\""),
                "names what is allowed: {message}"
            );
            assert!(evolver.fitness_function.is_none(), "nothing was stored");
        });
    }

    #[test]
    fn a_non_callable_is_rejected_naming_its_type() {
        Python::attach(|py| {
            let mut evolver = evolver_with(PYTHON_FITNESS);
            let not_a_function = py
                .eval(c"'not a function'", None, None)
                .expect("a string evaluates");

            let err = evolver
                .set_fitness_function(&not_a_function, "minimize")
                .expect_err("a non-callable must be rejected");

            let message = err.to_string();
            assert!(message.contains("callable"), "{message}");
            assert!(message.contains("str"), "names the type given: {message}");
            assert!(evolver.fitness_function.is_none(), "nothing was stored");
        });
    }

    #[test]
    fn a_python_config_with_no_registered_callable_is_an_error_not_a_panic() {
        // Without this the run would reach scoring with nothing to call and
        // panic somewhere inside the engine, where the message would name
        // none of this.
        let evolver = evolver_with(PYTHON_FITNESS);

        // `map(|_| ())` because `Box<dyn Fitness>` is not `Debug`, which
        // `expect_err` would require of the Ok type.
        let err = evolver
            .python_fitness()
            .map(|_| ())
            .expect_err("a python config with no callable cannot produce an objective");

        let message = err.to_string();
        assert!(
            message.contains("set_fitness_function"),
            "says what to do about it: {message}",
        );
    }

    #[test]
    fn the_resolved_objective_scores_through_the_registered_callable() {
        Python::attach(|py| {
            let mut evolver = evolver_with(PYTHON_FITNESS);
            evolver
                .set_fitness_function(&scoring_lambda(py), "maximize")
                .expect("registering a callable");

            let objective = evolver.python_fitness().expect("an objective is available");

            // Through the box, which is how the evolver will hold it: proves
            // the erasure keeps both the callable and its direction.
            assert_eq!(objective.direction(), Direction::Maximize);
            assert_eq!(
                objective.evaluate_batch(&[Graph::new(3, 1), Graph::new(7, 1)]),
                vec![3.0, 7.0],
            );
        });
    }

    #[test]
    fn each_call_hands_back_its_own_objective() {
        // Replicates need one instance each, so the seam is re-run per
        // replicate rather than handing the same box to several runs.
        Python::attach(|py| {
            let mut evolver = evolver_with(PYTHON_FITNESS);
            evolver
                .set_fitness_function(&scoring_lambda(py), "minimize")
                .expect("registering a callable");

            let first = evolver.python_fitness().expect("first instance");
            let second = evolver.python_fitness().expect("second instance");

            // Both live and usable at once, which a moved-out registration
            // could not manage.
            assert_eq!(first.evaluate(&Graph::new(4, 1)), 4.0);
            assert_eq!(second.evaluate(&Graph::new(9, 1)), 9.0);
            assert_eq!(first.evaluate(&Graph::new(2, 1)), 2.0);
        });
    }

    #[test]
    fn resolving_a_non_python_objective_reports_rather_than_panics() {
        let evolver = evolver_with(SIR_FITNESS);

        let err = evolver
            .python_fitness()
            .map(|_| ())
            .expect_err("a non-python config has no python objective to resolve");

        assert!(
            err.to_string().contains("epi_spread"),
            "names the configured type: {err}",
        );
    }

    #[test]
    fn registering_against_a_non_python_config_is_rejected() {
        // The failure this prevents is silent: the run would score with
        // epi_spread while the user watched for their own function's numbers.
        Python::attach(|py| {
            let mut evolver = evolver_with(SIR_FITNESS);

            let err = evolver
                .set_fitness_function(&scoring_lambda(py), "minimize")
                .expect_err("registering against a non-python config must be rejected");

            let message = err.to_string();
            assert!(
                message.contains("epi_spread"),
                "names the configured type: {message}"
            );
            assert!(
                message.contains("python"),
                "says what to change it to: {message}"
            );
            assert!(evolver.fitness_function.is_none(), "nothing was stored");
        });
    }

    /// An ordinary valid config, for tests that need one and do not care which.
    ///
    /// Built through the real constructors, so the argument order Python sees is
    /// exercised too. Nothing depends on these particular values — `to_toml()`
    /// and its round trip are covered in `py_config`, against a fixture that
    /// leaves nothing to a default.
    fn a_valid_config() -> PyConfig {
        PyConfig::new(
            PyEvolutionConfig::Generational {
                num_generations: 500,
                elite_count: 1,
            },
            200,
            100,
            0.9,
            0.2,
            PyScopeConfig::Global {},
            PySelectionConfig::Tournament { tournament_size: 5 },
            PyGenomeConfig::EdgeEdit {
                gene_length: 256,
                operation_weights: None,
                mutation: None,
            },
            PyFitnessConfig::EpiSpread {
                sir: PySirParams::new(0.5, 30, None, 3, 5),
            },
            1,
            1,
            None,
        )
    }

    #[test]
    fn from_config_builds_an_evolver_and_keeps_the_parsed_config() {
        let evolver = GraphEvolver::from_config(&a_valid_config())
            .expect("an ordinary valid config should build");

        assert_eq!(evolver.config.population_size, 200);
        assert!(evolver.fitness_function.is_none());
    }

    #[test]
    fn from_config_rejects_what_the_toml_front_end_would_reject() {
        // Zero clamps every edge weight to nothing under any genome, so the run
        // would look like a broken fitness function rather than a bad config.
        // The TOML path rejects it; so must this one.
        let mut config = a_valid_config();
        config.max_edge_multiplicity = 0;

        let message = match GraphEvolver::from_config(&config) {
            Err(err) => err.to_string(),
            Ok(_) => panic!("a zero edge multiplicity should have been rejected"),
        };

        assert!(
            message.contains("config.max_edge_multiplicity"),
            "the error should name the Python attribute path, not the bare TOML field, \
             got: {message}"
        );
    }

    #[test]
    fn from_config_accepts_a_python_objective_and_set_fitness_function_then_works() {
        // The two halves of the Python front end meeting: a config built in
        // Python selecting a callable registered from Python.
        Python::attach(|py| {
            let mut config = a_valid_config();
            config.fitness = PyFitnessConfig::Python();

            let mut evolver =
                GraphEvolver::from_config(&config).expect("a python objective is a valid config");

            let callable = py
                .eval(
                    c"lambda batch: [float(len(edges)) for (n, edges) in batch]",
                    None,
                    None,
                )
                .expect("the lambda compiles");

            evolver
                .set_fitness_function(&callable, "maximize")
                .expect("registering against a python config succeeds");
            assert!(evolver.python_fitness().is_ok());
        });
    }
    /// Both front ends reject zero replicates, and the Rust one does so before
    /// touching the config file — the path here does not exist.
    #[test]
    fn run_many_from_toml_rejects_zero_replicates() {
        match run_many_from_toml("no/such/config.toml", 7, 0) {
            Ok(_) => panic!("zero replicates should be rejected"),
            Err(err) => assert!(err.contains("n_runs must be at least 1"), "got: {err}"),
        }
    }
}
