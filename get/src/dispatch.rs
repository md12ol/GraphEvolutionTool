//! Config → concrete types: the layer spec §1 and §8 call "dispatch".
//!
//! Runtime choice is resolved here and nowhere else. The objective is erased to
//! `Box<dyn Fitness>` first, then strategy × genome is matched — which is what
//! keeps dispatch at 4 arms instead of 16 (§8). The starting population is built
//! here too, because `Genome` has no uniform random constructor and a generic
//! evolver therefore cannot mint one (§6).
//!
//! # Why this is its own module, and specifically not `evolver/common.rs`
//!
//! This layer is the **only** place that knows both sides. It reads
//! [`crate::config`] and returns `PyResult`, so it depends on the config schema
//! *and* on pyo3; the engine below it — `evolver/`, `genomes/`, `sir`, `graph` —
//! deliberately depends on neither, and `sir.rs` says so about its own params
//! type. Putting these functions in `evolver/common.rs` would drag both
//! dependencies into the engine core, and `common.rs` is genome-*agnostic*
//! generics over `G: Genome` where these name `EdgeEditGenome` and `SdaGenome`
//! concretely. Keeping it separate is what lets the engine be tested without a
//! config or a Python interpreter. See `decisions.md` 2026-08-11.
//!
//! `lib.rs` keeps the `#[pyclass]` surface — the constructors,
//! `set_fitness_function`, and `run`, which calls into here.

use std::sync::Arc;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::GraphEvolver;
use crate::config::{self, Config, EvolutionConfig, FitnessConfig, GenomeConfig, SelectionConfig};
use crate::evolver::common::Selection;
use crate::evolver::{
    EvolutionOutcome, Evolver, GenerationStats, GenerationalContext, GenerationalEvolver,
    SharedEvolutionContext, SteadyStateContext, SteadyStateEvolver,
};
use crate::fitness::{EpiLength, EpiProfMatch, EpiSpread, Fitness};
use crate::genomes::edge_edit::EdgeEditOperators;
use crate::genomes::{
    EdgeEditContext, EdgeEditGenome, EdgeEditOperationWeights, Genome, SdaContext, SdaGenome,
};
use crate::graph::Graph;
use crate::sir::{self, SirSampleParams};

/// One run's result, with the genome type erased.
///
/// `#[pyclass]` cannot carry a type parameter but [`EvolutionOutcome<G>`] does,
/// so every dispatch arm erases `G` before returning (§8). What survives is what
/// a caller can use without knowing the representation.
///
/// **Everything in here has already been converted out of engine orientation.**
/// That is the difference between this type and [`EvolutionOutcome`], which is
/// engine-oriented throughout and says so in its field names. This is the far
/// side of the boundary [`erase`] draws, and no field here needs converting
/// again.
pub(crate) struct ErasedOutcome {
    /// Best fitness in the objective's **own units and sign**, not engine
    /// orientation.
    ///
    /// The engine compares in oriented values throughout, so a maximizing
    /// objective's numbers are negative in there (§5.1). Converting once here is
    /// what keeps that an engine-internal detail: `Direction::orient` is its own
    /// inverse, so the same call that oriented the value on the way in undoes it
    /// on the way out.
    pub best_fitness: f64,
    /// The best individual's expressed network as `(u, v, multiplicity)`.
    pub best_edges: Vec<(usize, usize, u32)>,
    /// The winning genome's `Genome::print()` string.
    ///
    /// This is what [`Genome::print`] exists for (§8): a non-generic entry point
    /// can record *which* individual won without knowing its representation.
    pub best_genome_repr: String,
    /// The convergence log, one row per logged iteration (§6.4).
    ///
    /// Reuses the engine's [`GenerationStats`] row, but the two fitness columns
    /// have been oriented by [`erase`] — see this struct's own note above.
    pub history: Vec<GenerationStats>,
}

/// The parts of dispatch that need the evolver itself, not just its config.
///
/// `objective` reads the registered callable as well as `[fitness]`, and
/// `python_fitness` is entirely about that registration — so both take `&self`.
/// The three pure config→engine mappings below are free functions instead, which
/// makes them testable from a bare [`Config`] with no evolver to construct.
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
    /// independent reasons — `mutate`/`crossover` are generic over the RNG,
    /// `crossover` takes `&mut Self`, `Clone` requires `Sized`, and `Context` is
    /// an associated type differing per representation — so that axis stays a
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
    pub(crate) fn objective(&self, run_seed: u64) -> PyResult<Box<dyn Fitness>> {
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

/// The edge-edit starting population and the context it expresses against.
///
/// **Why the dispatch layer builds the population at all.** `Evolver::new`
/// takes a ready-made `Vec<G>` because `Genome` has no uniform random
/// constructor: this one needs a gene length and an operation mix, the SDA
/// one needs three dimensions and is fallible. A generic evolver cannot call
/// either, so the knowledge lives here, where genome-specific detail already
/// is — and a bad dimension surfaces as a config error at startup instead of
/// an `.expect()` mid-run inside a generic.
///
/// The operation mix is built **once** and shared across the population
/// behind the `Arc`: `EdgeEditOperators::new` compiles the weights into a
/// `WeightedIndex` sampler, and doing that per individual would rebuild the
/// same table `population_size` times.
///
/// The base graph is empty here. `set_base_graph` (GitHub #28) is what will
/// let a previous run's output seed it; until then every edge-edit run starts
/// from nothing, which matters because **five of the nine opcodes are inert
/// on an empty graph** — `Swap`, `Hop` and the three `Local*` all need
/// existing structure to walk, so early generations do nothing until
/// `Add`/`Toggle` have built something. Self-correcting, and stated here so
/// it is not read as a defect.
///
/// # Errors
///
/// `ValueError` if the operation weights are unusable — all zero, negative,
/// or non-finite. `Config::validate` already rejects those, so this is a
/// backstop for a `Config` assembled in Rust without validation.
pub(crate) fn edge_edit_start<R: Rng + ?Sized>(
    config: &Config,
    gene_length: usize,
    weights: EdgeEditOperationWeights,
    rng: &mut R,
) -> PyResult<(EdgeEditContext, Vec<EdgeEditGenome>)> {
    let operators = EdgeEditOperators::new(weights).map_err(PyValueError::new_err)?;

    let mut population = Vec::with_capacity(config.population_size);
    for _ in 0..config.population_size {
        population.push(EdgeEditGenome::random_with_operators(
            gene_length,
            Arc::clone(&operators),
            rng,
        ));
    }

    let context = EdgeEditContext {
        base_graph: Graph::new(config.network_size, config.max_edge_multiplicity),
    };
    Ok((context, population))
}

/// The SDA starting population and the context it expresses against.
///
/// **`num_chars` is derived, never configured** (§3.2): the alphabet is
/// `max_edge_multiplicity + 1`, so every character the automaton can emit is
/// a legal edge weight and none is silently clamped away by
/// `Graph::set_edge`. The same cap goes into the context, so the genome and
/// the graph it expresses against agree by construction rather than by the
/// caller remembering to pass the same number twice.
///
/// # Errors
///
/// `ValueError` if `init_state >= num_states`, or if the dimensions do not
/// fit `SdaGenome`'s storage types (`num_states` up to 65536, the derived
/// `num_chars` up to 256, `max_resp_len` at least 1).
///
/// Both are backstops rather than the primary guard — `Config::validate`
/// rejects the `init_state` case (`config.rs`, `validate_genome`) and both
/// front ends validate before constructing. Kept because the alternative is
/// worse than redundant: `SdaGenome::run` indexes its response table with
/// `init_state`, so an out-of-range value **panics during expression**,
/// which crosses the FFI as an opaque `PanicException` (§7). Reporting beats
/// asserting for anything that can reach a release build.
pub(crate) fn sda_start<R: Rng + ?Sized>(
    config: &Config,
    num_states: usize,
    max_resp_len: usize,
    init_state: usize,
    rng: &mut R,
) -> PyResult<(SdaContext, Vec<SdaGenome>)> {
    if init_state >= num_states {
        return Err(PyValueError::new_err(format!(
            "init_state ({init_state}) must be less than num_states ({num_states}); \
             SdaGenome::run indexes its response table with it",
        )));
    }

    let cap = config.max_edge_multiplicity;
    let mut population = Vec::with_capacity(config.population_size);
    for _ in 0..config.population_size {
        let genome =
            SdaGenome::random_with_edge_multiplicity_cap(num_states, cap, max_resp_len, rng)
                .map_err(PyValueError::new_err)?;
        population.push(genome);
    }

    let context = SdaContext {
        num_nodes: config.network_size,
        init_state,
        max_edge_multiplicity: cap,
    };
    Ok((context, population))
}

/// Run one evolution and hand back its result with the genome type erased.
///
/// **Step 2 of the dispatch** (§1, §8). The objective has already been erased to
/// `Box<dyn Fitness>`, so only strategy × genome is left — and this is arranged
/// as genome outside, strategy inside [`run_strategy`], which is why there are
/// two arms here and two there rather than four copies of the same body. Adding
/// a third genome is one arm here; a third strategy is one arm there.
///
/// **All randomness derives from `seed`.** The population is drawn first, then
/// the evolver's own seed is drawn from the same stream — never `seed` itself,
/// which the evolver would use to re-seed its own ChaCha8 and thereby replay the
/// exact draws that just built the population.
///
/// # Errors
///
/// `ValueError` for any dimension the genome constructors reject. `Config::validate`
/// has already run at construction, so these are backstops rather than the first
/// line of defence — see [`sda_start`].
pub(crate) fn evolve<F: Fitness>(
    config: &Config,
    fitness: &F,
    seed: u64,
) -> PyResult<ErasedOutcome> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let selection = selection(&config.selection);

    // Genome outside, strategy inside: `Genome` cannot be a trait object, so the
    // concrete type has to be settled before an evolver can be named at all.
    match &config.genome {
        GenomeConfig::EdgeEdit {
            gene_length,
            operation_weights,
        } => {
            let (genome_context, population) =
                edge_edit_start(config, *gene_length, *operation_weights, &mut rng)?;
            Ok(run_strategy(
                config,
                genome_context,
                population,
                selection,
                fitness,
                rng.random::<u64>(),
            ))
        }
        GenomeConfig::Sda {
            num_states,
            max_resp_len,
            init_state,
        } => {
            let (genome_context, population) =
                sda_start(config, *num_states, *max_resp_len, *init_state, &mut rng)?;
            Ok(run_strategy(
                config,
                genome_context,
                population,
                selection,
                fitness,
                rng.random::<u64>(),
            ))
        }
    }
}

/// Pick the evolution strategy and run it, for a genome type already settled.
///
/// Generic over `G`, so both genomes share this body instead of the strategy
/// match being written once per genome. That is the whole reason dispatch is
/// 2 + 2 arms rather than 2 × 2.
fn run_strategy<G: Genome, F: Fitness>(
    config: &Config,
    genome_context: G::Context,
    population: Vec<G>,
    selection: Selection,
    fitness: &F,
    seed: u64,
) -> ErasedOutcome {
    let shared = SharedEvolutionContext {
        genome_context,
        crossover_rate: config.crossover_rate,
        mutation_rate: config.mutation_rate,
        max_mutations: config.max_mutations,
        selection,
    };

    match &config.evolution {
        EvolutionConfig::Generational {
            num_generations,
            elite_count,
        } => {
            let type_context = GenerationalContext {
                num_generations: *num_generations,
                elite_count: *elite_count,
            };
            let mut evolver = GenerationalEvolver::new(shared, type_context, population);
            erase(evolver.run(fitness, seed))
        }
        EvolutionConfig::SteadyState { num_mating_events } => {
            let type_context = SteadyStateContext {
                num_mating_events: *num_mating_events,
            };
            let mut evolver = SteadyStateEvolver::new(shared, type_context, population);
            erase(evolver.run(fitness, seed))
        }
    }
}

/// Drop the genome type, converting every fitness back to the objective's units.
///
/// One generic function rather than the same lines in each arm. The conversion
/// is the part that matters: everything inside the engine is lower-is-better,
/// and this is the boundary where that stops being true (§5.1).
/// `Direction::orient` is its own inverse, so it is also what undoes itself.
///
/// **The history needs converting too, row by row** — and only its two fitness
/// columns. `std_dev` is left exactly as the engine computed it, because a
/// spread is identical under negation (§6.4). Orienting it as well would be a
/// silent defect: the number stays positive, so nothing looks wrong.
fn erase<G: Genome>(outcome: EvolutionOutcome<G>) -> ErasedOutcome {
    let direction = outcome.direction;

    let mut history = Vec::with_capacity(outcome.history.len());
    for row in outcome.history {
        history.push(GenerationStats {
            iteration: row.iteration,
            best_fitness: direction.orient(row.best_fitness),
            mean_fitness: direction.orient(row.mean_fitness),
            std_dev: row.std_dev,
        });
    }

    ErasedOutcome {
        best_fitness: direction.orient(outcome.best_fitness_engine),
        best_edges: outcome.best_graph.get_edge_list(),
        best_genome_repr: outcome.best_genome.print(),
        history,
    }
}

/// Map the `[selection]` block onto the engine's own selection strategy.
///
/// One variant each today. Kept as a function rather than inlined so a second
/// selection strategy is one arm here and touches neither evolver.
fn selection(config: &SelectionConfig) -> Selection {
    match config {
        SelectionConfig::Tournament { tournament_size } => Selection::Tournament {
            tournament_size: *tournament_size,
        },
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

#[cfg(test)]
mod tests {
    use super::*;

    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    use crate::config::FitnessConfig;
    use crate::fitness::Direction;
    // Only the tests express a genome directly; a run goes through the evolver.
    use crate::genomes::Genome;

    // Fixtures are deliberately local rather than shared with `lib.rs`'s test
    // module. Test helpers cannot be imported across sibling `#[cfg(test)]`
    // modules without giving them a home in the lib target, which GitHub #56
    // calls out as a decision to make on purpose rather than by default.
    const SIR_FITNESS: &str =
        "[fitness]\ntype = \"epi_spread\"\ninfection_rate = 0.05\nnum_epidemics = 30\n";
    const PYTHON_FITNESS: &str = "[fitness]\ntype = \"python\"\n";

    /// A config whose `[fitness]` block is exactly `fitness_block`.
    fn config_with(fitness_block: &str) -> Config {
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
        }
    }

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

    /// A config whose `[genome]` block is `genome_block`, everything else fixed.
    fn config_with_genome(genome_block: &str) -> Config {
        let text = format!(
            "population_size = 4\n\
             network_size = 8\n\
             max_edge_multiplicity = 3\n\
             crossover_rate = 0.8\n\
             mutation_rate = 0.2\n\
             \n\
             [evolution]\n\
             type = \"generational\"\n\
             num_generations = 5\n\
             \n\
             [selection]\n\
             type = \"tournament\"\n\
             tournament_size = 4\n\
             \n\
             {genome_block}\n\
             {SIR_FITNESS}"
        );
        Config::from_toml_str(&text).expect("the test config parses")
    }

    fn evolver_with_genome(genome_block: &str) -> GraphEvolver {
        GraphEvolver {
            config: config_with_genome(genome_block),
            fitness_function: None,
        }
    }

    fn test_rng() -> ChaCha8Rng {
        ChaCha8Rng::seed_from_u64(11)
    }

    #[test]
    fn the_edge_edit_start_sizes_the_population_and_the_empty_base_graph() {
        let evolver = evolver_with_genome("[genome]\ntype = \"edge_edit\"\ngene_length = 16\n");
        let weights = EdgeEditOperationWeights::default();

        let (context, population) = edge_edit_start(&evolver.config, 16, weights, &mut test_rng())
            .expect("default weights are usable");

        assert_eq!(population.len(), 4, "one individual per population_size");
        for genome in &population {
            assert_eq!(genome.genes.len(), 16, "each genome gets gene_length genes");
        }

        // The base graph comes from config, and is empty until `set_base_graph`
        // (GitHub #28) exists to seed it.
        assert_eq!(context.base_graph.num_nodes, 8);
        assert_eq!(
            context.base_graph.get_edge_list().len(),
            0,
            "no edges to start",
        );
    }

    #[test]
    fn the_sda_start_derives_its_alphabet_from_the_edge_multiplicity_cap() {
        // §3.2: `num_chars` is never configured, it is `max_edge_multiplicity + 1`
        // — the cap is 3 in this fixture, so the alphabet is 4. Configuring it
        // separately would let the automaton emit a character that
        // `Graph::set_edge` then clamps, losing structure with nothing reported.
        let evolver = evolver_with_genome(
            "[genome]\ntype = \"sda\"\nnum_states = 6\nmax_resp_len = 3\ninit_state = 2\n",
        );

        let (context, population) =
            sda_start(&evolver.config, 6, 3, 2, &mut test_rng()).expect("valid SDA dimensions");

        assert_eq!(population.len(), 4);
        assert_eq!(context.num_nodes, 8);
        assert_eq!(context.init_state, 2);
        assert_eq!(
            context.max_edge_multiplicity, 3,
            "the context carries the same cap the alphabet was derived from",
        );

        // Every genome must be expressible against that context — this is what
        // would fail if the two disagreed about the cap.
        for genome in &population {
            let graph = genome.express(&context);
            assert_eq!(graph.num_nodes, 8);
        }
    }

    #[test]
    fn an_out_of_range_init_state_is_reported_rather_than_panicking() {
        // `SdaGenome::run` indexes its response table with `init_state`, so this
        // panics during expression if it gets through — and a panic crossing the
        // FFI reaches the user as an opaque `PanicException` (§7). `run` is the
        // path that matters, so the check is exercised through it.
        let mut evolver = evolver_with_genome(
            "[genome]\ntype = \"sda\"\nnum_states = 4\nmax_resp_len = 3\ninit_state = 9\n",
        );

        let err = evolver
            .run(1)
            .expect_err("init_state 9 with num_states 4 must be rejected");

        let message = err.to_string();
        assert!(message.contains("init_state"), "names the field: {message}");
        assert!(message.contains('4'), "names num_states: {message}");
    }

    #[test]
    fn the_same_seed_builds_the_same_starting_population() {
        // The whole point of `run(seed)`: one master seed reproduces everything.
        let evolver = evolver_with_genome("[genome]\ntype = \"edge_edit\"\ngene_length = 12\n");
        let weights = EdgeEditOperationWeights::default();

        let (_, first) = edge_edit_start(
            &evolver.config,
            12,
            weights,
            &mut ChaCha8Rng::seed_from_u64(5),
        )
        .expect("first build");
        let (_, second) = edge_edit_start(
            &evolver.config,
            12,
            weights,
            &mut ChaCha8Rng::seed_from_u64(5),
        )
        .expect("second build");
        let (_, different) = edge_edit_start(
            &evolver.config,
            12,
            weights,
            &mut ChaCha8Rng::seed_from_u64(6),
        )
        .expect("third build");

        for (a, b) in first.iter().zip(second.iter()) {
            assert_eq!(a.genes, b.genes, "same seed, same population");
        }
        assert_ne!(
            first[0].genes, different[0].genes,
            "a different seed must give a different population",
        );
    }

    #[test]
    fn population_construction_does_not_replay_the_evolvers_stream() {
        // `run` draws the population and then draws the evolver's seed from the
        // same generator. Handing the evolver the master `seed` instead would
        // have it replay the exact values that just built the population —
        // correlating initial genomes with the first mutations, which looks like
        // nothing at all in the output.
        let mut rng = ChaCha8Rng::seed_from_u64(3);
        let evolver = evolver_with_genome("[genome]\ntype = \"edge_edit\"\ngene_length = 8\n");

        let (_, population) = edge_edit_start(
            &evolver.config,
            8,
            EdgeEditOperationWeights::default(),
            &mut rng,
        )
        .expect("build");
        let evolution_seed = rng.random::<u64>();

        // The evolver's seed must not be the master seed, and must not be a
        // value the population already consumed.
        assert_ne!(evolution_seed, 3, "not the master seed");
        let mut fresh = ChaCha8Rng::seed_from_u64(3);
        let first_draw = fresh.random::<u64>();
        assert_ne!(
            evolution_seed, first_draw,
            "the population consumed the stream ahead of the evolver's seed",
        );
        assert_eq!(population.len(), 4);
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

    /// A whole runnable config: `evolution_block` × `genome_block`, small enough
    /// that four end-to-end runs stay fast. Two epidemics per evaluation, not the
    /// realistic thirty — this asserts the wiring, not the search quality.
    ///
    /// `tournament_size = 4` is not arbitrary: `Config::validate` requires at
    /// least 4 under steady-state (`config.rs`, `validate_evolution_and_selection`),
    /// which needs two parents *and* two individuals to replace. Generational has
    /// no such floor, but one fixture serving both keeps the four combinations
    /// comparable.
    fn runnable(evolution_block: &str, genome_block: &str) -> Config {
        let text = format!(
            "population_size = 6\n\
             network_size = 8\n\
             max_edge_multiplicity = 2\n\
             crossover_rate = 0.8\n\
             mutation_rate = 0.5\n\
             \n\
             {evolution_block}\n\
             [selection]\n\
             type = \"tournament\"\n\
             tournament_size = 4\n\
             \n\
             {genome_block}\n\
             [fitness]\n\
             type = \"epi_spread\"\n\
             infection_rate = 0.3\n\
             num_epidemics = 2\n"
        );
        let config = Config::from_toml_str(&text).expect("the runnable config parses");
        config.validate().expect("the runnable config validates");
        config
    }

    const GENERATIONAL: &str = "[evolution]\ntype = \"generational\"\nnum_generations = 3\n";
    const STEADY_STATE: &str = "[evolution]\ntype = \"steady_state\"\nnum_mating_events = 12\n";
    const EDGE_EDIT: &str = "[genome]\ntype = \"edge_edit\"\ngene_length = 12\n";
    const SDA: &str = "[genome]\ntype = \"sda\"\nnum_states = 5\nmax_resp_len = 3\n";

    #[test]
    fn every_strategy_by_genome_combination_runs_end_to_end() {
        // #26's verify-by: all four arms of the dispatch complete a real run.
        // Before this, three of the four had never been executed at all — the
        // types line up whether or not the arms are wired to the right evolver.
        let combinations = [
            ("generational × edge_edit", GENERATIONAL, EDGE_EDIT),
            ("generational × sda", GENERATIONAL, SDA),
            ("steady_state × edge_edit", STEADY_STATE, EDGE_EDIT),
            ("steady_state × sda", STEADY_STATE, SDA),
        ];

        for (name, evolution, genome) in combinations {
            let config = runnable(evolution, genome);
            let objective: Box<dyn Fitness> =
                Box::new(EpiSpread::new(sir_sample_params(sir_of(&config)), 1));

            let outcome = evolve(&config, &objective, 1)
                .unwrap_or_else(|err| panic!("{name} should complete: {err}"));

            // `epi_spread` counts ever-infected and patient zero always counts,
            // so any real run scores at least 1 — and it is **maximized**, so a
            // negative value here would mean the engine's orientation leaked out
            // rather than being undone at this boundary.
            assert!(
                outcome.best_fitness >= 1.0,
                "{name}: fitness should be a positive spread in the objective's own \
                 units, got {}",
                outcome.best_fitness,
            );
            assert!(
                outcome.best_fitness.is_finite(),
                "{name}: fitness must be finite",
            );

            // Every edge must be inside the configured network and within the cap.
            for &(u, v, weight) in &outcome.best_edges {
                assert!(
                    u < 8 && v < 8,
                    "{name}: edge ({u},{v}) outside network_size 8"
                );
                assert!(
                    (1..=2).contains(&weight),
                    "{name}: weight {weight} outside 1..=max_edge_multiplicity",
                );
            }
        }
    }

    #[test]
    fn the_erased_history_comes_out_in_the_objectives_own_units() {
        // `epi_spread` is maximized, so every row is negative inside the engine
        // and must be positive here. The failure this catches is silent in the
        // worst way: an unconverted log plots upside down while every value in
        // it still looks like a plausible epidemic size.
        let config = runnable(GENERATIONAL, EDGE_EDIT);
        let objective: Box<dyn Fitness> =
            Box::new(EpiSpread::new(sir_sample_params(sir_of(&config)), 6));

        let outcome = evolve(&config, &objective, 6).expect("run completes");

        // Row 0 is the starting population, then one per generation.
        assert_eq!(outcome.history.len(), 4, "num_generations + 1 rows");

        for row in &outcome.history {
            assert!(
                row.best_fitness >= 1.0,
                "iteration {}: best fitness should be a positive spread, got {}",
                row.iteration,
                row.best_fitness,
            );
            assert!(
                row.mean_fitness >= 1.0,
                "iteration {}: mean fitness should be a positive spread, got {}",
                row.iteration,
                row.mean_fitness,
            );
            // Left alone by `erase` on purpose — a spread is identical under
            // negation, so orienting it would make it negative here.
            assert!(
                row.std_dev >= 0.0,
                "iteration {}: a deviation is never negative, got {}",
                row.iteration,
                row.std_dev,
            );
        }

        // The **last** row must agree with the headline number, because both are
        // read from the same final scoring pass (`generational.rs:208`) — so this
        // pins that the log and `best_fitness` went through the same conversion.
        //
        // Deliberately not the best row of the whole log: under a stochastic
        // objective an earlier generation can out-score the final one, and the
        // reported best is the best of the *final* population, not best-ever.
        let last = &outcome.history[outcome.history.len() - 1];
        assert_eq!(
            last.best_fitness, outcome.best_fitness,
            "the final log row and the reported best fitness must be the same number",
        );

        assert!(
            !outcome.best_genome_repr.is_empty(),
            "the winning genome's printed form must survive erasure",
        );
    }

    /// The `[fitness]` block of a config known to be `epi_spread`.
    fn sir_of(config: &Config) -> &config::SirParams {
        match &config.fitness {
            FitnessConfig::EpiSpread { sir } => sir,
            other => panic!("expected epi_spread, got {other:?}"),
        }
    }

    #[test]
    fn one_seed_reproduces_a_whole_run_and_another_changes_it() {
        // The guarantee the single `seed` argument exists to give (§7, §8.1).
        // It covers the population, the evolution and the epidemics at once,
        // since all three derive from it.
        let config = runnable(GENERATIONAL, EDGE_EDIT);
        let run = |seed: u64| {
            let objective: Box<dyn Fitness> =
                Box::new(EpiSpread::new(sir_sample_params(sir_of(&config)), seed));
            evolve(&config, &objective, seed).expect("run completes")
        };

        let first = run(4);
        let again = run(4);
        let other = run(5);

        assert_eq!(
            first.best_fitness, again.best_fitness,
            "same seed, same score"
        );
        assert_eq!(
            first.best_edges, again.best_edges,
            "same seed, same network"
        );
        assert!(
            other.best_fitness != first.best_fitness || other.best_edges != first.best_edges,
            "a different seed should not reproduce the same run",
        );
    }

    #[test]
    fn steady_state_and_generational_do_not_produce_the_same_run() {
        // Guards against both arms of `run_strategy` reaching the same evolver —
        // which would compile, run, and return plausible numbers.
        let generational = runnable(GENERATIONAL, EDGE_EDIT);
        let steady = runnable(STEADY_STATE, EDGE_EDIT);

        let first: Box<dyn Fitness> =
            Box::new(EpiSpread::new(sir_sample_params(sir_of(&generational)), 2));
        let second: Box<dyn Fitness> =
            Box::new(EpiSpread::new(sir_sample_params(sir_of(&steady)), 2));

        let a = evolve(&generational, &first, 2).expect("generational runs");
        let b = evolve(&steady, &second, 2).expect("steady-state runs");

        assert_ne!(
            a.best_edges, b.best_edges,
            "the two strategies should not be producing identical runs",
        );
    }

    #[test]
    fn run_returns_a_complete_result_object() {
        // Through the Python entry point, so this also exercises the GIL release.
        let mut evolver = GraphEvolver {
            config: runnable(GENERATIONAL, EDGE_EDIT),
            fitness_function: None,
        };

        let result = evolver.run(8).expect("a full config run completes");

        assert!(
            result.best_fitness >= 1.0,
            "in the objective's own units, got {}",
            result.best_fitness,
        );
        assert!(
            !result.best_genome_repr.is_empty(),
            "the winning genome's printed form comes back",
        );
        assert_eq!(result.history.len(), 4, "num_generations + 1 log rows");
        for &(u, v, _) in &result.best_edges {
            assert!(u < 8 && v < 8);
        }
    }

    #[test]
    fn two_runs_on_one_evolver_do_not_leak_state() {
        // The reason #27 returns a value instead of caching one: an evolver that
        // held the previous run's result would hand a replicate the wrong
        // numbers, and nothing about them would look wrong.
        let mut evolver = GraphEvolver {
            config: runnable(GENERATIONAL, EDGE_EDIT),
            fitness_function: None,
        };

        let first = evolver.run(4).expect("first run");
        let second = evolver.run(5).expect("second run");
        let first_again = evolver.run(4).expect("first run, repeated");

        // Seed 4 reproduces exactly, after seed 5 has run through the same
        // evolver — so nothing the second run did survived into the third.
        assert_eq!(first.best_fitness, first_again.best_fitness);
        assert_eq!(first.best_edges, first_again.best_edges);
        assert_eq!(first.best_genome_repr, first_again.best_genome_repr);
        assert_eq!(first.history.len(), first_again.history.len());

        // And the middle run is its own result rather than a copy of the first.
        assert!(
            second.best_fitness != first.best_fitness || second.best_edges != first.best_edges,
            "a different seed should not reproduce the same run",
        );
    }

    #[test]
    fn a_maximizing_objective_actually_climbs_through_the_dispatch() {
        // The failure this catches is the loudest-consequence, quietest-symptom
        // one in the whole layer: if the direction is lost anywhere between the
        // config and the evolver, the search *minimizes* ever-infected and every
        // number it reports still looks like a plausible spread.
        //
        // `infection_rate` is high here on purpose. At the shipped example's 0.05
        // an outbreak on a sparse graph dies immediately whatever the topology,
        // so every individual scores about the same and there is no gradient to
        // climb — measured 2026-08-11, see `issues.md`. This test needs a signal,
        // not a realistic parameter.
        let evolved = "[evolution]\ntype = \"generational\"\nnum_generations = 25\n";
        let unevolved = "[evolution]\ntype = \"generational\"\nnum_generations = 0\n";

        let spread_after = |evolution: &str| {
            let text = format!(
                "population_size = 20\n\
                 network_size = 20\n\
                 max_edge_multiplicity = 1\n\
                 crossover_rate = 0.9\n\
                 mutation_rate = 0.3\n\
                 \n\
                 {evolution}\n\
                 [selection]\n\
                 type = \"tournament\"\n\
                 tournament_size = 4\n\
                 \n\
                 [genome]\n\
                 type = \"edge_edit\"\n\
                 gene_length = 64\n\
                 \n\
                 [fitness]\n\
                 type = \"epi_spread\"\n\
                 infection_rate = 0.6\n\
                 num_epidemics = 5\n"
            );
            let config = Config::from_toml_str(&text).expect("parses");
            config.validate().expect("validates");
            let objective: Box<dyn Fitness> =
                Box::new(EpiSpread::new(sir_sample_params(sir_of(&config)), 3));
            evolve(&config, &objective, 3).expect("runs").best_fitness
        };

        let start = spread_after(unevolved);
        let end = spread_after(evolved);

        assert!(
            end > start,
            "25 generations of maximizing ever-infected should beat 0 generations, \
             got {end} against {start} — if this reversed, the objective's direction \
             is being lost between the config and the evolver",
        );
    }
}
