pub mod config;
// Crate-internal: the config → concrete-type layer `run` dispatches through.
// Not `pub`, because it is machinery rather than API — the Rust route uses the
// engine types directly (spec §5.3).
mod dispatch;
pub mod evolver;
pub mod fitness;
pub mod genomes;
pub mod graph;
pub mod py_config;
pub mod py_result;
pub mod sir;

use crate::config::{Config, FitnessConfig, GenomeConfig};
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
/// [`PyRunResult`](crate::py_result::PyRunResult) `run` returns, never on the
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
}

#[pymethods]
impl GraphEvolver {
    /// Load configuration from a `config.toml` file.
    #[new]
    fn new(config_path: String) -> PyResult<Self> {
        let config = Config::from_path(&config_path)
            // `{err}`, not `{err:?}`: `ConfigError`'s `Display` names the
            // offending field and its constraint, which is what spec §7 says a
            // bad config must reach the user as.
            .map_err(|err| PyValueError::new_err(format!("failed to load config: {err}")))?;
        Ok(Self {
            config,
            fitness_function: None,
            base_graph: None,
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
    ///         sir=get.SirParams(infection_rate=0.05, num_epidemics=30)
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
    /// Endpoints outside `0..num_nodes` and self-loops are dropped rather than
    /// rejected, which is [`Graph::set_edge`]'s behaviour and not re-litigated
    /// here.
    ///
    /// # Errors
    ///
    /// `ValueError` if `num_nodes` disagrees with the config's `network_size`,
    /// if the config selected the SDA genome, or if any edge's multiplicity
    /// exceeds the config's `max_edge_multiplicity`.
    ///
    /// The SDA case is a rejection rather than a no-op because an SDA run
    /// generates its graph from scratch instead of editing one: storing a base
    /// graph nothing would ever read is indistinguishable, from Python, from
    /// having seeded the run successfully.
    ///
    /// The cap case rejects rather than clamping, unlike [`Graph::set_edge`].
    /// Handing a graph built under a wider cap into a narrower one is the
    /// stacking trap: clamping would silently evolve against a different graph
    /// from the one passed in, and nothing downstream of here reads a warning.
    /// Lower the weights and resubmit.
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

        // Checked before anything is built, because `Graph::set_edge` clamps an
        // over-cap weight instead of refusing it — so a graph constructed first
        // and validated after would already have lost the offending value.
        for &(u, v, multiplicity) in &edges {
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

    /// Evolve a population and return everything the run produced.
    ///
    /// The returned [`PyRunResult`] carries the best fitness in the objective's
    /// own units, the best individual's edge list as `(u, v, multiplicity)`, its
    /// genome's printed form, and the convergence log:
    ///
    /// ```python
    /// result = evolver.run(seed=1)
    /// print(result.best_fitness, len(result.best_edges))
    /// for row in result.history:
    ///     print(row.iteration, row.best_fitness, row.std_dev)
    /// ```
    ///
    /// Nothing is cached on the evolver, so a second `run` cannot be confused
    /// with the first and the same evolver drives repeated runs safely.
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
    /// Required by spec §8.1 and agreed at the joint meeting of 2026-08-04; the
    /// `max_cores` and replicate-count parameters it refers to arrive with
    /// GitHub #20, which is what makes the last column reachable.
    fn run(&mut self, seed: u64) -> PyResult<PyRunResult> {
        // Step 1 of two (§8): erase the objective before any strategy or genome
        // is chosen. Built here rather than inside the dispatch match so it
        // happens exactly once per run, per §8.1's per-run-instance rule.
        //
        // Deliberately fallible-first: a config selecting Python with no
        // callable registered is reported before any population is built, which
        // is cheaper than discovering it at the first scoring call.
        let fitness = self.objective(seed)?;

        // The GIL is held on entry to any `#[pymethods]` function, and holding it
        // across the run buys nothing: everything between scoring calls is pure
        // Rust, and `PyFitness` re-acquires it per batch on its own. Keeping it
        // would block every other Python thread for the whole run and serialize
        // rayon against any Python caller — a run that works and is inexplicably
        // slow, or a host application that freezes, neither pointing back here.
        // `.claude/reference/pyo3-maturin.md` §2 has the measured deadlock.
        let outcome =
            Python::attach(|py| py.detach(|| dispatch::evolve(&self.config, &fitness, seed)))?;

        // Nothing is stored. `dispatch::erase` has already converted every
        // number out of engine orientation, so this only re-homes the erased
        // outcome onto the Python-visible type.
        Ok(PyRunResult::from_erased(outcome))
    }

    /// Write the per-iteration evolution log to `filename` as CSV.
    ///
    /// **These two take `&self` and the evolver holds nothing to write** — the
    /// log and the best individual live on the value `run` returns, so both need
    /// re-homing onto `PyRunResult` before they can be implemented.
    fn save_logs(&self, filename: &str) -> PyResult<()> {
        let _ = filename;
        todo!("write the run history to `filename`; it belongs on PyRunResult")
    }

    /// Write the best individual and its graph to `filename`. See `save_logs`.
    fn save_results(&self, filename: &str) -> PyResult<()> {
        let _ = filename;
        todo!("write the best genome and edge list to `filename`")
    }
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

    use crate::config::{EvolutionConfig, GenomeConfig, SelectionConfig};
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

    /// The Python builder equivalent of `config.example.toml`.
    ///
    /// Kept in step with that file by
    /// `the_python_builder_and_config_example_toml_agree` below — the point of
    /// spec §8's single-parser design is that these two cannot diverge, so a
    /// change to one that is not made to the other should fail here.
    fn example_mirror() -> PyConfig {
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
                sir: PySirParams::new(0.05, 30, None, 3, 5),
            },
            1,
            1,
        )
    }

    #[test]
    fn the_python_builder_and_config_example_toml_agree() {
        // The shipped example, read at compile time so the test cannot pass by
        // finding a stale copy on disk.
        let from_file = Config::from_toml_str(include_str!("../../config.example.toml"))
            .expect("the shipped example parses");
        let from_python =
            Config::from_toml_str(&example_mirror().to_toml().expect("the mirror renders"))
                .expect("the rendered mirror parses");

        // Destructured with no `..` for the same reason as `py_config`'s tests:
        // a field added to `Config` must break this, not slip past it.
        let Config {
            evolution,
            population_size,
            network_size,
            max_edge_multiplicity,
            crossover_rate,
            mutation_rate,
            max_mutations,
            selection,
            genome,
            fitness,
        } = from_file;

        assert_eq!(population_size, from_python.population_size);
        assert_eq!(network_size, from_python.network_size);
        assert_eq!(max_edge_multiplicity, from_python.max_edge_multiplicity);
        assert_eq!(crossover_rate, from_python.crossover_rate);
        assert_eq!(mutation_rate, from_python.mutation_rate);
        assert_eq!(max_mutations, from_python.max_mutations);

        match (evolution, from_python.evolution) {
            (
                EvolutionConfig::Generational {
                    num_generations: file_generations,
                    elite_count: file_elites,
                },
                EvolutionConfig::Generational {
                    num_generations: python_generations,
                    elite_count: python_elites,
                },
            ) => {
                assert_eq!(file_generations, python_generations);
                assert_eq!(file_elites, python_elites);
            }
            (file, python) => panic!("evolution differs: {file:?} vs {python:?}"),
        }

        match (selection, from_python.selection) {
            (
                SelectionConfig::Tournament {
                    tournament_size: file_size,
                },
                SelectionConfig::Tournament {
                    tournament_size: python_size,
                },
            ) => assert_eq!(file_size, python_size),
        }

        match (genome, from_python.genome) {
            (
                GenomeConfig::EdgeEdit {
                    gene_length: file_length,
                    operation_weights: file_weights,
                },
                GenomeConfig::EdgeEdit {
                    gene_length: python_length,
                    operation_weights: python_weights,
                },
            ) => {
                assert_eq!(file_length, python_length);
                assert_eq!(file_weights, python_weights);
            }
            (file, python) => panic!("genome differs: {file:?} vs {python:?}"),
        }

        match (fitness, from_python.fitness) {
            (
                FitnessConfig::EpiSpread { sir: file_sir },
                FitnessConfig::EpiSpread { sir: python_sir },
            ) => {
                assert_eq!(file_sir.infection_rate, python_sir.infection_rate);
                assert_eq!(file_sir.patient_zero, python_sir.patient_zero);
                assert_eq!(file_sir.num_epidemics, python_sir.num_epidemics);
                assert_eq!(file_sir.min_epidemic_length, python_sir.min_epidemic_length);
                assert_eq!(
                    file_sir.max_epidemic_retries,
                    python_sir.max_epidemic_retries
                );
            }
            (file, python) => panic!("fitness differs: {file:?} vs {python:?}"),
        }
    }

    #[test]
    fn from_config_builds_an_evolver_and_keeps_the_parsed_config() {
        let evolver = GraphEvolver::from_config(&example_mirror())
            .expect("a config equivalent to the shipped example should build");

        assert_eq!(evolver.config.population_size, 200);
        assert!(evolver.fitness_function.is_none());
    }

    #[test]
    fn from_config_rejects_what_the_toml_front_end_would_reject() {
        // Zero clamps every edge weight to nothing under any genome, so the run
        // would look like a broken fitness function rather than a bad config
        // (spec §7, GitHub #6). The TOML path rejects it; so must this one.
        let mut config = example_mirror();
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
            let mut config = example_mirror();
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
