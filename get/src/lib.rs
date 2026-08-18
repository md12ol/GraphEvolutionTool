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
// Crate-internal: the Python config builder. pyo3 needs these types nameable
// from the crate root to register them, not publicly reachable from Rust.
mod py_config;
pub mod py_result;
pub mod sir;

use crate::config::{Config, ConfigError, FitnessConfig, GenomeConfig};
use crate::evolver::GenerationStats;
use crate::fitness::{Direction, PyFitness};
use crate::graph::Graph;
use crate::py_config::{
    PyConfig, PyEvolutionConfig, PyFitnessConfig, PyGenomeConfig, PyOperationWeights,
    PySelectionConfig, PySirParams, config_error_to_py,
};
use crate::py_result::{PyGenerationStats, PyRunResult};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

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
    /// The TOML document `config` was parsed from — the run's provenance
    /// record, written alongside its results.
    config_toml: String,
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
    /// if the config selected the SDA genome, or if any edge names a node
    /// outside `0..num_nodes`, is a self-loop, or carries a multiplicity above
    /// the config's `max_edge_multiplicity`.
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
    fn set_base_graph(
        &mut self,
        num_nodes: usize,
        edges: Vec<(usize, usize, u32)>,
    ) -> PyResult<()> {
        if num_nodes != self.config.network_size {
            return Err(PyValueError::new_err(format!(
                "base graph has {} nodes but [evolution] network_size is {}; \
                 the graph an edit script is applied to must be the size the run evolves",
                num_nodes, self.config.network_size,
            )));
        }

        if !matches!(self.config.genome, GenomeConfig::EdgeEdit { .. }) {
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
        for &(u, v, multiplicity) in &edges {
            if u >= num_nodes || v >= num_nodes {
                return Err(PyValueError::new_err(format!(
                    "edge ({u}, {v}) names a node outside 0..{num_nodes}; the node count \
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

        let mut graph = Graph::new(self.config.network_size, self.config.max_edge_multiplicity);
        graph.set_edges(&edges);
        self.base_graph = Some(graph);
        Ok(())
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
            objectives.push(self.objective(run_seed)?);
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
        for (run_index, outcome) in outcomes.into_iter().enumerate() {
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

/// What a Rust-native run hands back — the [`PyRunResult`] fields, without the
/// pyo3 wrapper. See [`run_from_toml`].
pub struct RunSummary {
    /// Best fitness found, in the objective's own units and sign — see
    /// [`PyRunResult::best_fitness`].
    pub best_fitness: f64,
    /// The best individual's expressed network, as `(u, v, multiplicity)`.
    pub best_edges: Vec<(usize, usize, u32)>,
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
        writeln!(file, "best_fitness = {}", self.best_fitness)?;
        writeln!(file, "genome = {}", self.best_genome_repr)?;
        writeln!(file, "\nedges (u,v,multiplicity):")?;
        for &(u, v, weight) in &self.best_edges {
            writeln!(file, "{u},{v},{weight}")?;
        }

        std::fs::write(format!("{filename}.toml"), &self.config_toml)?;
        Ok(())
    }
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
        config_toml: text.clone(),
    };

    let fitness = evolver.objective(seed).map_err(|err| err.to_string())?;
    let outcome = dispatch::evolve(&evolver.config, &fitness, evolver.base_graph.as_ref(), seed)
        .map_err(|err| err.to_string())?;

    Ok(RunSummary {
        best_fitness: outcome.best_fitness,
        best_edges: outcome.best_edges,
        best_genome_repr: outcome.best_genome_repr,
        history: outcome.history,
        seed,
        run_index: 0,
        config_toml: text,
    })
}

#[pymodule]
fn get(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<GraphEvolver>()?;
    // The config builders (spec §8). Registered under their unprefixed names —
    // the `Py` prefix is a Rust-side disambiguator, not part of the API.
    m.add_class::<PyConfig>()?;
    m.add_class::<PyEvolutionConfig>()?;
    m.add_class::<PySelectionConfig>()?;
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
            config_toml: String::new(),
        }
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
            PySelectionConfig::Tournament { tournament_size: 5 },
            PyGenomeConfig::EdgeEdit {
                gene_length: 256,
                operation_weights: None,
            },
            PyFitnessConfig::EpiSpread {
                sir: PySirParams::new(0.5, 30, None, 3, 5),
            },
            1,
            1,
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
