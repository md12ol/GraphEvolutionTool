pub mod config;
pub mod evolver;
pub mod fitness;
pub mod genomes;
pub mod graph;
pub mod sir;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::config::{Config, FitnessConfig};
use crate::fitness::{Direction, Fitness, PyFitness};

/// Python-facing entry point to the graph-evolution engine.
///
/// Constructed from a `config.toml` path; [`GraphEvolver::run`] dispatches on
/// the configured evolution strategy, genome representation, and fitness
/// objective, then returns the best graph found.
#[pyclass]
pub struct GraphEvolver {
    config: Config,
    best_fitness: Option<f64>,
    /// The objective registered by [`GraphEvolver::set_fitness_function`], set
    /// only when `[fitness] type = "python"`.
    ///
    /// Runtime configuration that cannot live in a config file arrives through
    /// a setter instead (§8): the config *selects* Python, this *is* the
    /// callable.
    fitness_function: Option<PyFitness>,
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
            best_fitness: None,
            fitness_function: None,
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

    /// Evolve a population and return the best graph as a weighted edge list
    /// `(u, v, multiplicity)`.
    fn run(&mut self, seed: u64) -> PyResult<Vec<(usize, usize, u32)>> {
        let _ = (seed, &self.config, &mut self.best_fitness);
        todo!(
            "dispatch on config (evolution x genome x fitness), run the evolver, \
             cache best_fitness, and return the best graph's edge list"
        )
    }

    /// Best fitness found so far, or infinity before any run completes.
    fn best_fitness(&self) -> f64 {
        self.best_fitness.unwrap_or(f64::INFINITY)
    }

    /// Write the per-iteration evolution log to `filename` as CSV.
    fn save_logs(&self, filename: &str) -> PyResult<()> {
        let _ = filename;
        todo!("write the run history to `filename`")
    }

    /// Write the best individual and its graph to `filename`.
    fn save_results(&self, filename: &str) -> PyResult<()> {
        let _ = filename;
        todo!("write the best genome and edge list to `filename`")
    }
}

/// Rust-only, so deliberately outside the `#[pymethods]` block above: these
/// take and return engine types that have no Python representation.
impl GraphEvolver {
    /// The objective for one run, when the config selected Python.
    ///
    /// This is the seam the dispatch in **#26** calls: it turns the registered
    /// callable into the erased `Box<dyn Fitness>` that §8 hands the evolver,
    /// so the `python` arm of that match is one call rather than a second place
    /// that knows how registration works.
    ///
    /// **A fresh instance per call, not a shared one.** Replicate runs each
    /// need their own objective (§8.1), so the erasing step is re-run per
    /// replicate rather than cloning one box. The Python callable itself is
    /// shared by refcount — see [`PyFitness::clone_ref`] — which is right,
    /// because it is the *scorer* state that must stay per-run and this has
    /// none.
    ///
    /// The other three variants are not built here: they need
    /// `config::SirParams` mapped onto [`crate::sir::SirSampleParams`] plus the
    /// run seed, which is #26's to write.
    ///
    /// # Errors
    ///
    /// `ValueError` if no callable has been registered — the case spec §8 and
    /// issue #19 both call out, since a run that reached scoring with nothing
    /// registered would otherwise panic deep inside the engine. Also if the
    /// config did not select Python, which is a caller mistake rather than a
    /// user one, but is reported rather than asserted so it cannot become a
    /// panic in a release build.
    // TEMPORARY — remove when #26 lands. This is the seam #26's dispatch calls,
    // so until that match exists nothing in non-test code calls it and
    // `dead_code` fires, which would break the `-D warnings` gate that #25 just
    // made usable. Its tests do exercise it. Recorded in `hotfixes.md`.
    #[allow(dead_code)]
    pub(crate) fn python_fitness(&self) -> PyResult<Box<dyn Fitness>> {
        if !matches!(self.config.fitness, FitnessConfig::Python) {
            return Err(PyValueError::new_err(format!(
                "python_fitness is only for a \"python\" objective, but [fitness] type \
                 is \"{}\"",
                self.config.fitness.type_name(),
            )));
        }

        match self.fitness_function {
            Some(ref registered) => Ok(Box::new(registered.clone_ref())),
            None => Err(PyValueError::new_err(
                "[fitness] type is \"python\" but no fitness function has been \
                 registered; call set_fitness_function(callable, direction) before run()",
            )),
        }
    }
}

#[pymodule]
fn get(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<GraphEvolver>()?;
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
            best_fitness: None,
            fitness_function: None,
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
                objective.evaluate_population(&[Graph::new(3, 1), Graph::new(7, 1)]),
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
}
