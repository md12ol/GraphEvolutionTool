pub mod config;
pub mod evolver;
pub mod fitness;
pub mod genomes;
pub mod graph;
pub mod py_config;
pub mod sir;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::config::{Config, FitnessConfig};
use crate::fitness::{Direction, EpiLength, EpiProfMatch, EpiSpread, Fitness, PyFitness};
use crate::py_config::{
    PyConfig, PyEvolutionConfig, PyFitnessConfig, PyGenomeConfig, PyOperationWeights,
    PySelectionConfig, PySirParams, config_error_to_py,
};
use crate::sir::SirSampleParams;

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
    ///
    /// # For whoever implements the dispatch (#26)
    ///
    /// Two things #19 left in place for this method, both easy to miss because
    /// neither fails loudly:
    ///
    /// **Get the Python objective from [`GraphEvolver::python_fitness`]**, don't
    /// reach for `self.fitness_function` directly. It returns the erased
    /// `Box<dyn Fitness>` this method needs, a fresh instance per call — which
    /// is what replicate runs require, since each needs its own objective
    /// (§8.1) — and it turns "no callable registered" into a `ValueError`
    /// naming `set_fitness_function`, rather than a panic from deep inside
    /// scoring. It carries a temporary `#[allow(dead_code)]` only because this
    /// method is its only non-test caller; delete that attribute once this
    /// calls it (`hotfixes.md`).
    ///
    /// **Release the GIL around the run itself** — wrap the evolve loop in
    /// `Python::attach(|py| py.detach(|| ...))` (pyo3's older name for it is
    /// `allow_threads`). Everything the engine does between scoring calls is
    /// pure Rust, and [`crate::fitness::PyFitness`] re-acquires the GIL per
    /// batch on its own, so holding it across the whole run buys nothing and
    /// blocks every other Python thread in the process for the duration. Under
    /// a native Rust objective it also serializes rayon against any Python
    /// caller. The failure mode is a run that works and is inexplicably slow, or
    /// a host application that freezes while a run is in progress — neither of
    /// which points back here.
    ///
    /// Spec §8 has the surrounding argument; `.claude/reference/pyo3-maturin.md`
    /// §2 has the measured deadlock that motivates the GIL discipline.
    fn run(&mut self, seed: u64) -> PyResult<Vec<(usize, usize, u32)>> {
        // Step 1 of two (§8): erase the objective before any strategy or genome
        // is chosen. Built here rather than inside the dispatch match so it
        // happens exactly once per run, per §8.1's per-run-instance rule.
        //
        // Deliberately fallible-first: a config selecting Python with no
        // callable registered is reported before any population is built, which
        // is cheaper than discovering it at the first scoring call.
        let fitness = self.objective(seed)?;

        let _ = (&fitness, &self.config, &mut self.best_fitness);
        todo!(
            "step 2: match (evolution, genome), build the population and contexts, \
             run the evolver against `fitness`, cache best_fitness, and return the \
             best graph's edge list"
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
    /// Build the objective for one run, erased to `Box<dyn Fitness>`.
    ///
    /// **Step 1 of the two-step dispatch** (§1, §8, GitHub #26). The objective
    /// is erased *before* any strategy or genome is chosen, which is what keeps
    /// dispatch at 2 strategies × 2 genomes = 4 arms instead of 16: nothing
    /// downstream knows which objective it holds. Adding a fifth objective is
    /// one arm here and touches nothing else.
    ///
    /// The asymmetry is not an oversight. `Fitness` erases cleanly — no generic
    /// methods, no `Self` in argument position, and `Send + Sync` through its
    /// supertrait, so rayon is unaffected. `Genome` cannot, for four
    /// independent reasons (see `GraphEvolver::run`), so that axis stays a
    /// match.
    ///
    /// **Call this once per run, never once per evolver.** Every SIR objective
    /// owns an `EpidemicScorer` holding a per-run counter, and two replicates
    /// sharing one would let thread scheduling decide which run saw which seed
    /// — reproducibility goes with it (§8.1). Taking `run_seed` by argument
    /// rather than reading a field is what makes that misuse awkward.
    ///
    /// # Errors
    ///
    /// `ValueError` if the config selected Python and no callable was
    /// registered, or if `epi_prof_match`'s target profile is unusable.
    /// `Config::validate` already rejects an empty or non-finite profile, so
    /// that second case is a backstop for a `Config` built in Rust without
    /// going through validation — not a path a Python caller can reach.
    fn objective(&self, run_seed: u64) -> PyResult<Box<dyn Fitness>> {
        match &self.config.fitness {
            FitnessConfig::EpiSpread { sir } => {
                Ok(Box::new(EpiSpread::new(sir_sample_params(sir), run_seed)))
            }
            FitnessConfig::EpiLength { sir } => {
                Ok(Box::new(EpiLength::new(sir_sample_params(sir), run_seed)))
            }
            FitnessConfig::EpiProfMatch {
                sir,
                target_profile,
            } => {
                // Cloned because the objective owns its target and the config
                // outlives it — a run must not be able to mutate the profile it
                // is being scored against.
                let objective =
                    EpiProfMatch::new(sir_sample_params(sir), run_seed, target_profile.clone())
                        .map_err(PyValueError::new_err)?;
                Ok(Box::new(objective))
            }
            // The one arm that is not built from config alone: the callable
            // arrived through a setter, so `python_fitness` owns the "nothing
            // registered" error and this stays one call.
            FitnessConfig::Python => self.python_fitness(),
        }
    }

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
    /// The other three variants are not built here — they need
    /// `config::SirParams` mapped onto [`crate::sir::SirSampleParams`] plus the
    /// run seed. [`GraphEvolver::objective`] is that match, and this is its
    /// `python` arm.
    ///
    /// # Errors
    ///
    /// `ValueError` if no callable has been registered — the case spec §8 and
    /// issue #19 both call out, since a run that reached scoring with nothing
    /// registered would otherwise panic deep inside the engine. Also if the
    /// config did not select Python, which is a caller mistake rather than a
    /// user one, but is reported rather than asserted so it cannot become a
    /// panic in a release build.
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

/// Map the `[fitness]` block onto the simulator's own sampling parameters.
///
/// Two types with overlapping names and neither is redundant:
/// [`crate::config::SirParams`] is the deserializable config block, and
/// [`SirSampleParams`] is what the simulator takes — deliberately independent
/// of the config schema, so `sir.rs` does not depend on `[fitness]`'s spelling.
/// This function is the seam, and it lives in the dispatch layer because that
/// is where `config.rs`'s module doc says config becomes engine types.
///
/// Note the nesting changes shape: the config block is flat, while
/// [`SirSampleParams`] separates the epidemic's own two parameters into a nested
/// [`sir::SirParams`] from the batch settings around them.
///
/// **No seed is mapped.** One master seed reaches `run` and every objective
/// derives from it (§7, §8.1), so a seed in the config would be a second,
/// competing source — `Config::from_toml_str` rejects a stray `seed` key
/// outright rather than ignoring it.
fn sir_sample_params(params: &config::SirParams) -> SirSampleParams {
    SirSampleParams {
        epidemic: sir::SirParams {
            infection_rate: params.infection_rate,
            patient_zero: params.patient_zero,
        },
        num_epidemics: params.num_epidemics,
        min_epidemic_length: params.min_epidemic_length,
        max_epidemic_retries: params.max_epidemic_retries,
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
            best_fitness: None,
            fitness_function: None,
        }
    }

    const PYTHON_FITNESS: &str = "[fitness]\ntype = \"python\"\n";
    const SIR_FITNESS: &str =
        "[fitness]\ntype = \"epi_spread\"\ninfection_rate = 0.05\nnum_epidemics = 30\n";

    /// The `[fitness]` block for `objective`'s remaining two SIR arms.
    fn sir_block(type_name: &str, extra: &str) -> String {
        format!(
            "[fitness]\ntype = \"{type_name}\"\ninfection_rate = 0.05\n\
             num_epidemics = 30\n{extra}"
        )
    }

    #[test]
    fn each_objective_erases_to_a_box_carrying_its_own_direction() {
        // The failure this exists for is silent. `Fitness::direction` has a
        // default of `Minimize`, so a boxed objective whose direction is not
        // forwarded reports "minimize" whatever it holds — and both maximizing
        // objectives then run the search backwards while merely looking
        // unconverged (§5.1). Nothing panics and no number looks wrong.
        let cases = [
            (sir_block("epi_spread", ""), Direction::Maximize),
            (sir_block("epi_length", ""), Direction::Maximize),
            (
                sir_block("epi_prof_match", "target_profile = [1, 3, 7, 2]\n"),
                Direction::Minimize,
            ),
        ];

        for (block, expected) in cases {
            let evolver = evolver_with(&block);
            let objective = evolver
                .objective(7)
                .unwrap_or_else(|err| panic!("{block} should build an objective: {err}"));

            assert_eq!(
                objective.direction(),
                expected,
                "wrong direction erased for: {block}",
            );
        }
    }

    #[test]
    fn the_sir_block_reaches_the_simulator_field_for_field() {
        // `config::SirParams` and `sir::SirSampleParams` are two types with
        // overlapping names and a different shape — the config block is flat,
        // the simulator's nests the epidemic's own parameters. A field mapped to
        // the wrong place still compiles when the types agree, so this checks
        // each one rather than trusting the mapping to be obvious.
        let config = config_with(&sir_block(
            "epi_spread",
            "patient_zero = 4\nmin_epidemic_length = 2\nmax_epidemic_retries = 9\n",
        ));

        let block = match &config.fitness {
            FitnessConfig::EpiSpread { sir } => sir,
            other => panic!("expected epi_spread, got {other:?}"),
        };
        let mapped = sir_sample_params(block);

        assert_eq!(mapped.epidemic.infection_rate, 0.05);
        assert_eq!(mapped.epidemic.patient_zero, Some(4));
        assert_eq!(mapped.num_epidemics, 30);
        assert_eq!(mapped.min_epidemic_length, 2);
        assert_eq!(mapped.max_epidemic_retries, 9);
    }

    #[test]
    fn an_omitted_patient_zero_stays_unpinned_through_the_mapping() {
        // `None` means "draw a fresh node per epidemic" (§5.2). Defaulting it to
        // node 0 instead would seed every outbreak from the same vertex and
        // quietly change what the objective measures.
        let config = config_with(SIR_FITNESS);
        let block = match &config.fitness {
            FitnessConfig::EpiSpread { sir } => sir,
            other => panic!("expected epi_spread, got {other:?}"),
        };

        assert_eq!(sir_sample_params(block).epidemic.patient_zero, None);
    }

    #[test]
    fn each_call_builds_a_fresh_sir_objective() {
        // §8.1: replicates must not share an objective, because every SIR
        // objective owns an `EpidemicScorer` whose counter is per-run state.
        // Sharing one lets thread scheduling decide which run sees which seed.
        let evolver = evolver_with(SIR_FITNESS);

        let first = evolver.objective(1).expect("first objective");
        let second = evolver.objective(1).expect("second objective");

        // Same seed, so the two agree — what matters is that both are live at
        // once, which a shared or moved-out instance could not manage.
        let graph = Graph::new(6, 1);
        assert_eq!(first.evaluate(&graph), second.evaluate(&graph));
    }

    #[test]
    fn a_python_objective_with_no_callable_fails_before_anything_is_built() {
        // `run` calls `objective` first precisely so this is reported before a
        // population exists, rather than at the first scoring call.
        let evolver = evolver_with(PYTHON_FITNESS);

        let err = evolver
            .objective(1)
            .map(|_| ())
            .expect_err("a python config with no callable cannot build an objective");

        assert!(
            err.to_string().contains("set_fitness_function"),
            "says what to do about it: {err}",
        );
    }

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
        assert!(evolver.best_fitness.is_none());
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
