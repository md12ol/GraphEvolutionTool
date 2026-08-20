// Doc links are load-bearing: a public doc that links a private item renders a
// dead link, and nothing failed when that count grew from 2 to 10 unnoticed.
// Denying it here rather than in CI means it fails on the machine that wrote it.
#![deny(rustdoc::private_intra_doc_links, rustdoc::redundant_explicit_links)]

// Crate-internal: nothing outside can use a parsed `Config`, because the only
// thing that consumes one is `dispatch`, which is private. A caller who wants
// to run from a TOML file goes through `GraphEvolver` or `run_from_toml`.
mod config;
// Crate-internal: the config → concrete-type layer `run` dispatches through.
// Not `pub`, because it is machinery rather than API — the Rust route uses the
// engine types directly (spec §5.3).
mod dispatch;
pub mod evolver;
pub mod fitness;
pub mod genomes;
pub mod graph;
pub mod graph_io;
// Crate-internal: the Python config builder. pyo3 needs these types nameable
// from the crate root to register them, not publicly reachable from Rust.
mod py_config;
pub mod py_result;
pub mod sir;
// Structural graph statistics. Public for the same reason as `sir`: it is a
// domain computation objectives are built on, and a caller who writes their own
// fitness function needs the same measurements the built-in ones use.
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

/// One graph as a folder load hands it back: the file it was read from, and its
/// edges as `(u, v, multiplicity)`.
type NamedGraph = (String, usize, Vec<(usize, usize, u32)>);

/// Raise every warning a load produced through Python's `warnings` machinery.
///
/// `source` names what produced them — a file path, or `base graph` for a
/// setter call — since a folder of reference graphs can warn about several files
/// in one call and the user needs to know which.
///
/// Warnings go here rather than to stdout so a caller can silence, capture or
/// promote them to errors with `warnings.simplefilter`, which is where a Python
/// user already looks. `stacklevel` is 2, so the message points at the line that
/// called the setter rather than at this function.
fn emit_load_warnings(py: Python<'_>, source: &str, warnings: &[LoadWarning]) -> PyResult<()> {
    let category = py.get_type::<PyUserWarning>();

    for warning in warnings {
        let text = format!("{source}: {warning}");
        // Python's C API takes a NUL-terminated string, and text we formatted
        // ourselves cannot contain an interior NUL — but the conversion is
        // fallible, and dropping a warning silently is the failure this whole
        // path exists to avoid, so it is reported rather than skipped.
        let message = CString::new(text).map_err(|_| {
            PyValueError::new_err("a load warning could not be converted for Python")
        })?;

        PyErr::warn(py, &category, &message, 2)?;
    }

    Ok(())
}

/// Raise every warning a load produced, on whichever route has no GIL to
/// raise a `UserWarning` on.
///
/// `struct_match`'s reference-set loader (`dispatch::struct_match_reference`)
/// is reachable from `GraphEvolver::run_from_toml` (spec §5.3 route 4), which
/// never acquires the GIL, as well as from the ordinary Python-driven `run`.
/// `emit_load_warnings` needs a `Python<'_>` token either way, so this is the
/// one seam both routes call through: `Some(py)` uses it, `None` prints the
/// same `{source}: {warning}` text to stderr — the only sink route 4 has,
/// since there is no `warnings.simplefilter` to route it through there.
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

/// Python-facing entry point to the graph-evolution engine.
///
/// Constructed from a `config.toml` path; [`GraphEvolver::run`] dispatches on
/// the configured evolution strategy, genome representation, and fitness
/// objective, then returns everything that run produced.
///
/// **It holds no results.** A run's state lives in the
/// [`PyRunResult`] `run` returns, never on the
/// evolver — so one evolver is reusable across repeated runs with nothing stale
/// from the previous one hanging off it.
#[pyclass]
pub struct GraphEvolver {
    config: Config,
    /// The objective registered by [`GraphEvolver::set_fitness_function`], set
    /// only when `[fitness] type = "python"`.
    ///
    /// Runtime configuration that cannot live in a config file arrives through
    /// a setter instead (§8): the config *selects* Python, this *is* the
    /// callable.
    fitness_function: Option<PyFitness>,
    /// The graph an edge-edit script is applied to, set by
    /// [`GraphEvolver::set_base_graph`].
    ///
    /// A setter rather than a config field for the same reason as the objective
    /// above: this is either data the caller brought or the output of a previous
    /// run, and neither belongs in a `config.toml`. `None` means every run starts
    /// from an empty graph, which is the default.
    ///
    /// Unused by SDA, which generates its graph from scratch rather than editing
    /// one — the setter rejects a call on an SDA-configured evolver instead of
    /// storing something nothing will read.
    base_graph: Option<Graph>,
    /// The index the caller's own data starts at, set by whichever file loader
    /// was called first and `None` until one is.
    ///
    /// One value per run, shared by every loader: node 4 means the same node
    /// whichever file it was read from, and two files disagreeing about where
    /// counting starts would silently mix two graphs. Everything is shifted to
    /// 0 on the way in, so nothing inside the engine ever sees another
    /// indexing; only the evolved graph `run` hands back is shifted here.
    min_node_index: Option<i64>,
    /// The TOML document `config` was parsed from — the run's provenance
    /// record, written alongside its results.
    config_toml: String,
    /// `struct_match`'s reduced reference set, built at most once per evolver.
    ///
    /// # Why this is cached when the objective itself is not
    ///
    /// `objective()` is called once per replicate, and §8.1 requires each
    /// replicate its *own* objective because an `EpidemicScorer` holds a
    /// per-run counter that must not be shared. `StructMatch` has no such
    /// state — it is a pure function of one graph — so what must be fresh is
    /// the objective, not the reference statistics behind it. Those are
    /// immutable, and rebuilding them per replicate would re-read the folder
    /// and re-run an eigendecomposition of every reference graph, `n_runs`
    /// times, in a phase that logs nothing.
    ///
    /// Empty for every other objective, and until the first run.
    struct_match_reference: OnceLock<Arc<ReferenceStatistics>>,
}

#[pymethods]
impl GraphEvolver {
    /// Load configuration from a `config.toml` file.
    #[new]
    pub fn new(config_path: String) -> PyResult<Self> {
        // Read separately from `Config::from_toml_str`, rather than through
        // `Config::from_path`, because the raw text is the provenance record
        // and `from_path` does not hand it back.
        let text = std::fs::read_to_string(&config_path).map_err(|err| {
            PyValueError::new_err(format!("failed to load config: {}", ConfigError::Io(err)))
        })?;
        let config = Config::from_toml_str(&text)
            // `{err}`, not `{err:?}`: `ConfigError`'s `Display` names the
            // offending field and its constraint, which is what spec §7 says a
            // bad config must reach the user as.
            .map_err(|err| PyValueError::new_err(format!("failed to load config: {err}")))?;
        config
            .validate()
            .map_err(|err| PyValueError::new_err(format!("failed to load config: {err}")))?;
        Ok(Self {
            config,
            fitness_function: None,
            base_graph: None,
            min_node_index: None,
            struct_match_reference: OnceLock::new(),
            config_toml: text,
        })
    }

    /// Build from configuration assembled in Python, rather than from a file.
    ///
    /// The config object is rendered to TOML and parsed by exactly the same
    /// [`Config::from_toml_str`] and [`Config::validate`] the file path above
    /// goes through — Python is a *builder* for the config format, not a second
    /// parser of it (spec §8). So this cannot accept a configuration a
    /// `config.toml` would reject, and both front ends report the same
    /// constraint in the same words.
    ///
    /// ```python
    /// config = get.Config(
    ///     population_size=200,
    ///     network_size=100,
    ///     crossover_rate=0.9,
    ///     mutation_rate=0.2,
    ///     evolution=get.EvolutionConfig.Generational(num_generations=500),
    ///     selection=get.SelectionConfig.Tournament(tournament_size=5),
    ///     genome=get.GenomeConfig.EdgeEdit(gene_length=256),
    ///     fitness=get.FitnessConfig.EpiSpread(
    ///         sir=get.SirParams(infection_rate=0.5, num_epidemics=30)
    ///     ),
    /// )
    /// evolver = get.GraphEvolver.from_config(config)
    /// ```
    ///
    /// `config.to_toml()` returns the document this parsed, which is the run's
    /// provenance record: written beside the results, it re-runs verbatim.
    ///
    /// # Errors
    ///
    /// `ValueError` if the configuration breaks one of spec §7's constraints,
    /// or if a field is too large for a TOML integer.
    #[staticmethod]
    fn from_config(config: &PyConfig) -> PyResult<Self> {
        let text = config.to_toml()?;

        // Parsing its own rendering should not fail, so this reports the text
        // as well: a failure here is a defect in `py_config`'s emission rather
        // than anything the user did, and the document is what a bug report
        // needs.
        let parsed = Config::from_toml_str(&text).map_err(|err| {
            PyValueError::new_err(format!(
                "the generated config did not parse: {err}. This is a bug in GET, not in \
                 your configuration — please report it with the document below.\n---\n{text}"
            ))
        })?;

        // Reported through `config_error_to_py`, which rewrites the field name
        // `validate` uses — spelled as it appears in the TOML — into the Python
        // attribute path that produced it. The bare name is right for the file
        // front end and unhelpful here, where the user never saw a document
        // (spec §8).
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
    /// callable itself arrives here, and so does its direction, because nothing
    /// can infer whether a user's function wants its value large or small
    /// (§5, §8).
    ///
    /// The callable takes the **whole batch** and returns one float per graph,
    /// in the same order — see [`crate::fitness::PyFitness`] for the contract
    /// and why a per-graph callback is not an option:
    ///
    /// ```python
    /// evolver.set_fitness_function(
    ///     lambda batch: [score(n, edges) for (n, edges) in batch],
    ///     "maximize",
    /// )
    /// ```
    ///
    /// # Errors
    ///
    /// `ValueError` if `callable` is not callable, if `direction` is not
    /// `"minimize"` or `"maximize"`, or if the config did not select Python.
    /// That last one is the interesting case: accepting a callable the run
    /// would never consult is indistinguishable, from Python, from having
    /// registered it successfully.
    fn set_fitness_function(
        &mut self,
        callable: &Bound<'_, PyAny>,
        direction: &str,
    ) -> PyResult<()> {
        // Refused rather than ignored: silently accepting this leaves the user
        // watching an SIR objective's numbers and wondering why their own
        // function never runs.
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

        // Spelled out rather than taking a `Direction` enum across the boundary,
        // so the Python side needs no import to say which way is better.
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
    /// `best_edges`, so one run's output feeds the next without reshaping:
    ///
    /// ```python
    /// first = sda_evolver.run(seed=1)
    /// edge_evolver.set_base_graph(64, first.best_edges)
    /// second = edge_evolver.run(seed=2)
    /// ```
    ///
    /// **`min_node_index` is where the caller's own node numbering starts, and it
    /// is the same numbering the two file loaders take** — pass `1` for
    /// 1-indexed edges, and they shift to 0 here exactly as a file's would, so
    /// nobody has to renumber data by hand to use this setter. It defaults to
    /// `0`, which is what `best_edges` above is, so the round-trip in that
    /// example needs no argument.
    ///
    /// **One run has one numbering, and every entry point shares it.** This
    /// setter, [`set_base_graph_from_file`](GraphEvolver::set_base_graph_from_file)
    /// and [`load_reference_graphs`](GraphEvolver::load_reference_graphs) all
    /// declare the same `min_node_index`, and a second call that disagrees with
    /// the first is rejected rather than mixed in. That numbering is also what
    /// the evolved graph is shifted back into on the way out, so results return
    /// in the numbering the data arrived in. Supplying a 0-indexed graph here
    /// and then a 1-indexed reference set therefore raises, instead of quietly
    /// handing back indices that match neither.
    ///
    /// **Left unset, every run starts from an empty graph** — that is the
    /// default, and it is worth stating rather than leaving to be discovered:
    /// five of the nine edit opcodes are inert on an empty graph. `Swap`, `Hop`
    /// and the three `Local*` all need existing structure to walk, so early
    /// generations do nothing until `Add`/`Toggle` have built something. That
    /// is self-correcting and not a defect.
    ///
    /// # Errors
    ///
    /// `ValueError` if `num_nodes` disagrees with the config's `network_size`,
    /// if the config selected the SDA genome, if any edge names a node outside
    /// `min_node_index ..= min_node_index + num_nodes - 1`, is a self-loop, or
    /// carries a multiplicity above the config's `max_edge_multiplicity` — or if
    /// a loader on this evolver already declared a different `min_node_index`.
    /// An out-of-range message names the index as the caller wrote it, not as it
    /// would be after shifting.
    ///
    /// The SDA case is a rejection rather than a no-op because an SDA run
    /// generates its graph from scratch instead of editing one: storing a base
    /// graph nothing would ever read is indistinguishable, from Python, from
    /// having seeded the run successfully.
    ///
    /// **The three per-edge checks all reject what [`Graph::set_edge`] would
    /// absorb**, and that asymmetry is deliberate. `set_edge` returns early on
    /// a bad endpoint or a self-loop and clamps an over-cap weight, which is
    /// right for the engine — the edit opcodes decode vertex indices out of a
    /// random payload and are all no-ops when their preconditions fail, so a
    /// fallible `set_edge` would turn that into an error path in every one of
    /// them. Permissiveness that suits engine-generated indices is wrong for
    /// data a caller handed over.
    ///
    /// Each failure is one a caller cannot otherwise see. A node count equal to
    /// `network_size` does not make the *edges* in range: a caller who takes
    /// `num_nodes` from their config rather than their data passes the first
    /// check while every edge above the network size vanishes. A self-loop
    /// almost always means the indices are wrong — 1-indexed data is the common
    /// case, and it also lands every surviving edge on the wrong vertex. And a
    /// graph built under a wider cap would be narrowed silently, evolving
    /// against a different graph from the one passed in. Nothing downstream
    /// reads a warning, so all three raise.
    ///
    /// # Warnings
    ///
    /// A pair given more than once raises a `UserWarning` and the **last**
    /// occurrence wins. Comparison is canonical — `(2, 5)` and `(5, 2)` are the
    /// same undirected edge — so writing one both ways round is a repeat, not
    /// two edges. This is a warning rather than an error because the list still
    /// describes a graph and the caller may well have meant to overwrite; what
    /// it must not do is happen in silence, which is what applying the list in
    /// order used to do.
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

        // Every check runs before anything is built, because `Graph::set_edge`
        // absorbs all three failures rather than reporting them: it returns
        // early on a bad endpoint or a self-loop and clamps an over-cap weight.
        // A graph constructed first and validated after would already have lost
        // the offending edge, leaving nothing to report.
        // The caller's own numbering, so a message quotes the indices they wrote
        // rather than shifted ones they would not recognise. Matches how the
        // file loader validates the same thing.
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

        // Collapse repeats before building. `Graph::set_edge` writes each edge
        // in turn, so a list holding a pair twice silently kept whichever came
        // last — a real disagreement in the caller's data, reported by nothing.
        let mut sourced = Vec::with_capacity(edges.len());
        for &(u, v, multiplicity) in &edges {
            sourced.push(SourcedEdge {
                // Shifted to 0 here, once, the same as the file loader does on
                // the way in. The range check above is what makes this safe:
                // every index is at least `lowest`, so neither can go negative.
                u: (u as i64 - min_node_index) as usize,
                v: (v as i64 - min_node_index) as usize,
                weight: multiplicity,
                // No file behind these, so no line to point at.
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
    /// `1` for 1-indexed data, which is the common case in graph files. Every
    /// index is shifted to 0 here, and the evolved graph [`GraphEvolver::run`]
    /// hands back is shifted the same distance the other way, so a caller reads
    /// their results in the numbering they wrote.
    ///
    /// ```python
    /// evolver.set_base_graph_from_file("network.csv", min_node_index=1)
    /// best = evolver.run(seed=1)[0].best_edges   # also 1-indexed
    /// ```
    ///
    /// **There is no node-count argument, unlike
    /// [`set_base_graph`](GraphEvolver::set_base_graph)** — the file is checked
    /// against `network_size` directly, so the mistake that check existed to
    /// catch (a caller deriving the count from their config rather than their
    /// data) cannot be made here. A file that states its own size in a
    /// `# nodes = N` header is checked against `network_size` too, and a
    /// disagreement is an error: the base graph and the graphs a run evolves
    /// are the same size by definition. A file with no header is taken to be
    /// `network_size` nodes, which is what it has always been taken to be.
    ///
    /// # Errors
    ///
    /// `ValueError` if the config selected the SDA genome, if the file cannot be
    /// read, or if any row fails validation — a self-loop, a malformed or
    /// non-numeric row, a negative weight, a node outside
    /// `min_node_index ..= min_node_index + network_size - 1`, or a multiplicity
    /// above `max_edge_multiplicity`. Every message names the line it came from,
    /// and nothing is stored unless the whole file survives.
    ///
    /// Also `ValueError` if a previous loader on this evolver was given a
    /// different `min_node_index`. One run has one numbering: two files
    /// disagreeing about where counting starts would mix two graphs together
    /// with nothing to show for it.
    ///
    /// # Warnings
    ///
    /// A `UserWarning` for each repeated edge (canonical, so `2,5` and `5,2` are
    /// one edge, and the last occurrence wins), each zero-weight edge (kept as
    /// given, which is no edge at all), and for a file holding no edges.
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

        // The file states its own size, and disagreeing with the run is the one
        // mistake this path could not previously catch. The loader rejects a
        // header above `network_size` as it rejects an index above it; below it
        // is what lands here, and it is the interesting half — a file the caller
        // believes is 200 nodes but which says 180 would otherwise load as 200,
        // padded with isolated nodes nobody asked for. `set_base_graph` has
        // rejected exactly this disagreement since it took `num_nodes`; a file
        // could not say enough to be checked until the header existed.
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
    /// The bulk counterpart to
    /// [`set_base_graph_from_file`](GraphEvolver::set_base_graph_from_file), for
    /// reference data an objective matches against. Each file is one edge per
    /// line, `start,end,weight`, and every file in the folder shares this run's
    /// node numbering — `min_node_index` here means what it means there.
    ///
    /// **It returns the graphs rather than storing them**, and that is the whole
    /// point of the shape: nothing in the engine reads a reference set yet, and
    /// a setter that stored one would be keeping data no objective would ever
    /// look at. The reader is the caller until an objective needs one.
    ///
    /// ```python
    /// graphs = evolver.load_reference_graphs("references/", min_node_index=1)
    /// name, num_nodes, edges = graphs[0]
    /// ```
    ///
    /// **The node count is handed back beside the edges because it cannot be
    /// recovered from them** — an isolated node appears in no edge, and a
    /// reference graph with one is ordinary rather than exotic. Each file
    /// states its own count in a `# nodes = N` header, which is where this
    /// comes from.
    ///
    /// **Each graph comes back paired with the file it was read from, in sorted
    /// file-name order.** Directory order is not reproducible across machines
    /// and a reference set is consumed positionally, so leaving the order to the
    /// filesystem would let a run's numbers depend on how its data happened to
    /// be written to disk. Sub-directories are skipped; every other file is
    /// read, since an extension convention would silently drop data the caller
    /// meant to include.
    ///
    /// **The reference graphs themselves are never shifted back** — nothing hands
    /// one to the caller in their own numbering, because these are the numbers
    /// they supplied in the first place.
    ///
    /// **This call does, however, declare the run's numbering**, in common with
    /// the two base-graph entry points, and that numbering is what the evolved
    /// graph is shifted back into. So loading a 1-indexed reference set means
    /// `run` returns `best_edges` 1-indexed too. That is deliberate — one run has
    /// one numbering — and it is why a base graph supplied under a different
    /// numbering is rejected rather than mixed in.
    ///
    /// # Errors
    ///
    /// `ValueError` if the folder cannot be read, if any file in it fails
    /// validation — the message names the file and the line — or if a previous
    /// loader on this evolver was given a different `min_node_index`.
    ///
    /// # Warnings
    ///
    /// The same `UserWarning`s as the base-graph loader, each naming the file it
    /// came from: a repeated edge, a zero-weight edge, and a file with no edges.
    #[pyo3(signature = (folder, min_node_index = 0))]
    fn load_reference_graphs(
        &mut self,
        py: Python<'_>,
        folder: String,
        min_node_index: i64,
    ) -> PyResult<Vec<NamedGraph>> {
        self.check_min_node_index(min_node_index)?;

        let loaded = graph_io::load_edge_folder(
            std::path::Path::new(&folder),
            self.config.network_size,
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
    /// **Always a list, one [`PyRunResult`] per replicate, in run order** — even
    /// at the default `n_runs = 1`, so the return type does not change shape
    /// with an argument. Each result carries the best fitness in the objective's
    /// own units, the best individual's edge list as `(u, v, multiplicity)`, its
    /// genome's printed form, the convergence log, and the `(seed, run_index)`
    /// pair that identifies it — `seed` being the master you passed, so the two
    /// together are what reproduces that replicate:
    ///
    /// ```python
    /// results = evolver.run(seed=1, n_runs=30, max_cores=8)
    /// best = max(results, key=lambda r: r.best_fitness)
    /// print(best.best_fitness, best.run_index)
    ///
    /// single, = evolver.run(seed=1)          # one run still returns a list
    /// for row in single.history:
    ///     print(row.iteration, row.best_fitness, row.std_dev)
    /// ```
    ///
    /// **One master seed, not `n` of them.** `seed` seeds a generator whose
    /// output stream *is* the per-run seed list, so run `i` takes draw `i` and a
    /// run's seed does not depend on how many runs were requested. Asking for 50
    /// reproduces the first 30 of a 30-run request exactly, so extending an
    /// experiment never invalidates the replicates already collected.
    ///
    /// **Whether replicates run concurrently is the engine's call, not yours.**
    /// A native Rust objective runs them in parallel; `fitness = "python"` runs
    /// them one at a time, because `n` concurrent runs would be `n` threads
    /// contending for a single GIL — slower than sequential *and* contended.
    /// `max_cores` caps the concurrency: unset means all available, `1` is fully
    /// sequential, and there is no point exceeding `n_runs`.
    ///
    /// Nothing is cached on the evolver, so a second `run` cannot be confused
    /// with the first and the same evolver drives repeated runs safely.
    ///
    /// # Errors
    ///
    /// `ValueError` if `n_runs` is zero, if `max_cores` is given as zero, if the
    /// config selected Python and no callable was registered, or from the first
    /// replicate that fails — the remaining runs are abandoned rather than
    /// returned half-complete.
    ///
    /// # Memory: the three sizes multiply, they do not add
    ///
    /// Expression materializes the whole population as `Vec<Graph>` before
    /// scoring, and a `Graph` is a **dense** `network_size × network_size`
    /// matrix however sparse the graph actually is (spec §2). So peak memory is
    /// roughly:
    ///
    /// ```text
    /// network_size² × 4 bytes × population_size × min(max_cores, replicates)
    /// ```
    ///
    /// | `network_size` | one graph | population of 200 | × 8 concurrent replicates |
    /// |---|---|---|---|
    /// | 100 | 40 KB | 8 MB | 64 MB |
    /// | 500 | 1 MB | 200 MB | 1.6 GB |
    /// | 1000 | 4 MB | 800 MB | 6.4 GB |
    ///
    /// Treat those as a floor rather than an exact figure: the matrix is a
    /// `Vec<Vec<u32>>`, so each row carries its own allocation header on top of
    /// the `4 × network_size` bytes of weights. That overhead is well under 1%
    /// at these sizes and does not change the shape of the problem.
    ///
    /// The failure mode is unintuitive, which is why it is documented on the
    /// call rather than left to be derived: a configuration that ran fine, given
    /// a larger `max_cores` to exploit a bigger machine, multiplies peak memory
    /// by that same factor and can exhaust hardware that handled the smaller
    /// setting. A user cannot work this out from the Rust internals, and this
    /// package is the only surface they see.
    ///
    /// The last column is reachable from here: `min(max_cores, n_runs)` is the
    /// multiplier, so raising either raises peak memory by the same factor.
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

        // One master seed in, one seed per run out, in run order — so a run's
        // seed does not depend on how many were asked for.
        let seeds = dispatch::replicate_seeds(seed, n_runs);

        // Step 1 of two (§8): erase the objective before any strategy or genome
        // is chosen. **One instance per run, never one shared** — every SIR
        // objective owns an epidemic counter, and sharing it across concurrent
        // replicates would let thread scheduling decide which run saw which
        // epidemic seed, losing reproducibility exactly where it is hardest to
        // debug.
        //
        // Built here, before the GIL is released, because the python arm reads
        // the registered callable — that must not happen on a rayon worker.
        // Deliberately fallible-first too: a config selecting Python with no
        // callable registered is reported before any population is built.
        let mut objectives = Vec::with_capacity(n_runs);
        for &run_seed in &seeds {
            objectives.push(self.objective(run_seed, Some(py))?);
        }

        // The GIL is held on entry to any `#[pymethods]` function, and holding it
        // across the run buys nothing: everything between scoring calls is pure
        // Rust, and `PyFitness` re-acquires it per batch on its own. Keeping it
        // would block every other Python thread for the whole run and serialize
        // rayon against any Python caller — a run that works and is inexplicably
        // slow, or a host application that freezes, neither pointing back here.
        // The failure is a hard deadlock, not just slowness: a rayon worker that
        // calls back into Python needs the GIL, and it cannot get it while this
        // thread holds it and waits on that worker to finish.
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

        // Nothing is stored. `dispatch::erase` has already converted every
        // number out of engine orientation, so this only re-homes each erased
        // outcome onto the Python-visible type, keeping run order.
        //
        // **`seed` is the master, not the per-run draw.** The pair that
        // reproduces a replicate is `(master, run_index)` — call `run` with the
        // same master and take that index. The derived per-run seed cannot do
        // that: handing it back as `seed` would make the stream draw from *it*,
        // producing a different run, so recording it would look like provenance
        // while being unusable as provenance.
        let mut results = Vec::with_capacity(outcomes.len());
        for (run_index, mut outcome) in outcomes.into_iter().enumerate() {
            // The one place a node index goes back to the caller's numbering.
            // Only the evolved graph is shifted: everything else a loader read
            // is input, and shifting a reference graph nobody hands back would
            // be a second conversion with no reader.
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
    /// Checking and recording are two steps rather than one so that a load which
    /// fails leaves the evolver exactly as it found it. The check runs first, so
    /// a mismatch is still reported before the named file is opened; the
    /// recording waits until every fallible step has succeeded. A numbering
    /// recorded by a call that then failed would shift the node indices of every
    /// later run's output, with no base graph loaded and nothing to say so.
    ///
    /// Not a `#[pymethods]` function: it is internal bookkeeping, and everything
    /// in that block is exposed to Python.
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
    /// Call only once every fallible step of a load has succeeded — see
    /// [`GraphEvolver::check_min_node_index`] for why the two are separate.
    fn commit_min_node_index(&mut self, min_node_index: i64) {
        self.min_node_index = Some(min_node_index);
    }
}

/// Put an evolved edge list back into the caller's own numbering.
///
/// A no-op when no loader set one, which is every run whose data arrived
/// 0-indexed through a setter.
fn shift_out(edges: &mut [(usize, usize, u32)], min_node_index: Option<i64>) {
    let shift = match min_node_index {
        Some(0) | None => return,
        Some(shift) => shift,
    };

    for edge in edges.iter_mut() {
        // Every index here was produced by the engine, so it is within
        // `0..network_size` and shifting it back lands where it came from.
        edge.0 = (edge.0 as i64 + shift) as usize;
        edge.1 = (edge.1 as i64 + shift) as usize;
    }
}

/// What a Rust-native run hands back — the [`PyRunResult`] fields, without the
/// pyo3 wrapper. See [`run_from_toml`].
pub struct RunSummary {
    /// Best fitness found, in the objective's own units and sign — see
    /// [`PyRunResult::best_fitness`].
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
    /// The seed the run was made with. Private: `save_logs` stamps it onto
    /// every row, which is the only way it is meant to be read.
    seed: u64,
    /// Which replicate this is, `0`-based. Private, and a hard `0` until GitHub
    /// #20 — a reader would take a constant for something that varies. The
    /// field stays so #20 does not change the CSV's column set under anyone.
    run_index: usize,
    /// The TOML document this run's config was parsed from. Private:
    /// `save_results` writes it to `{filename}.toml`.
    config_toml: String,
}

impl RunSummary {
    /// Write the convergence log to `filename` as CSV.
    ///
    /// Same five columns, plus `seed` and `run_index`, as
    /// [`PyRunResult::save_logs`] — a log from this binary and one from
    /// Python are byte-for-byte the same shape.
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
    /// alongside it at `{filename}.toml` — mirrors
    /// [`PyRunResult::save_results`].
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

        std::fs::write(format!("{filename}.toml"), &self.config_toml)?;
        Ok(())
    }
}

/// One seed per replicate, derived from a master seed.
///
/// Re-exported from the private dispatch layer so every route derives them the
/// same way: a replicate's seed depends on the master and on its own index, and
/// never on how many replicates were asked for. That is what makes
/// `(master, run_index)` the pair that reproduces a run — the derived seed
/// cannot, since passing it back in would make the stream draw from *it*.
///
/// A library caller driving its own loop (see `examples/library_route.rs`)
/// wants this rather than a seed per run of its own invention, or its replicates
/// will not line up with the same master seed run through `get-run`.
pub fn replicate_seeds(master: u64, n_runs: usize) -> Vec<u64> {
    dispatch::replicate_seeds(master, n_runs)
}

/// `YYYYmmdd-HHMMSS` in UTC, for naming a run's output directory.
///
/// UTC rather than local time, so directories from two machines sort into the
/// order the runs actually happened. Converted here rather than through a date
/// crate: one directory name does not justify a dependency, and the arithmetic
/// below is the standard civil-from-days algorithm, exact for every date this
/// program will ever see.
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

/// Where one replicate's files belong, creating the directory if needed.
///
/// `None` for `out_dir` means the working directory under fixed names, which
/// is what `get-run` did before it took an output folder and what its route
/// check in CI still expects.
///
/// Public so every route lays its output out the same way. A library caller
/// writing its own files (see `examples/library_route.rs`) gets the same
/// `<root>/<stamp>-<seed>/run_<i>/` shape as `get-run` without copying the
/// rule, which is how the four routes stay comparable.
///
/// `stamp` is passed in rather than read here: every replicate of one
/// invocation belongs in the same timestamped directory, and reading the clock
/// per replicate would scatter them the moment a run crossed a second boundary.
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
        path = path.join(format!("run_{run_index}"));
    }
    path
}

/// Rust-native entry point: run a `config.toml` file with no Python
/// interpreter (spec §5.3's "Rust route", used by the `get-run` binary,
/// `src/bin/run.rs`).
///
/// Follows exactly the same steps [`GraphEvolver::new`] and
/// [`GraphEvolver::run`] do for a Python caller — parse, validate, erase the
/// objective, dispatch — so a run driven from here and one driven from
/// Python differ only in front end. `[fitness] type = "python"` is rejected,
/// the same way an un-registered Python run is: there is no callable for it
/// to call.
///
/// # Errors
///
/// The config's own parse/validate error, or `[fitness] type = "python"`
/// with nothing to call it.
pub fn run_from_toml(config_path: &str, seed: u64) -> Result<RunSummary, String> {
    let mut summaries = run_many_from_toml(config_path, seed, 1)?;
    Ok(summaries.remove(0))
}

/// The same run, `n_runs` times from one master seed.
///
/// Mirrors [`GraphEvolver::run`]'s replicate handling exactly, so the Rust and
/// Python front ends produce the same numbers for the same `(seed, run_index)`
/// pair: one master seed goes in, one seed per replicate comes out, and a
/// replicate's seed therefore does not depend on how many were asked for. Each
/// summary carries the **master** seed and its own index, which is the pair that
/// reproduces it — the derived seed would draw a different run if it were passed
/// back in, so recording it would look like provenance while being unusable as
/// provenance.
///
/// Runs sequentially. The Python path spreads replicates across rayon because a
/// caller there may be waiting on a notebook cell; a command-line run has
/// nothing to overlap with, and a sequential loop keeps the objective's
/// construction, which is fallible, on the calling thread.
///
/// # Errors
///
/// The config's own parse/validate error, or `[fitness] type = "python"` with
/// nothing to call it. `n_runs` of zero yields an empty vector rather than an
/// error — there is nothing wrong with asking for no runs, only with pretending
/// one happened.
pub fn run_many_from_toml(
    config_path: &str,
    seed: u64,
    n_runs: usize,
) -> Result<Vec<RunSummary>, String> {
    let text = std::fs::read_to_string(config_path)
        .map_err(|err| format!("failed to load config: {}", ConfigError::Io(err)))?;
    let config =
        Config::from_toml_str(&text).map_err(|err| format!("failed to load config: {err}"))?;
    config
        .validate()
        .map_err(|err| format!("failed to load config: {err}"))?;

    let evolver = GraphEvolver {
        config,
        fitness_function: None,
        base_graph: None,
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

#[pymodule]
fn get(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<GraphEvolver>()?;
    // The config builders (spec §8). Registered under their unprefixed names —
    // the `Py` prefix is a Rust-side disambiguator, not part of the API.
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
    // What `run` hands back. Registered so the types are importable and
    // `isinstance`-able, not because a user constructs one.
    m.add_class::<PyRunResult>()?;
    m.add_class::<PyGenerationStats>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::fitness::Fitness;
    use crate::graph::Graph;

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
        // The second half of #19's verify-by. Without this the run would reach
        // scoring with nothing to call and panic somewhere inside the engine,
        // where the message would name none of this.
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
        // Replicates need one instance each (§8.1), so the seam is re-run per
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
        // would look like a broken fitness function rather than a bad config
        // (spec §7, GitHub #6). The TOML path rejects it; so must this one.
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
}
