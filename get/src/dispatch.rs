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
use rayon::ThreadPoolBuilder;
use rayon::prelude::*;

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
    /// The entry point is not generic over the genome, so this is the only way
    /// it can record *which* individual won.
    pub best_genome_repr: String,
    /// The convergence log, one row per logged iteration.
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
/// `base_graph` is what the edit script is applied to: `Some` seeds the run
/// from a caller-supplied graph — raw data, or a previous run's best edges —
/// and `None` starts from an empty one. It is cloned rather than borrowed
/// because the context owns its graph for the whole run, while the caller
/// keeps theirs for any further runs.
///
/// An empty base is the case worth knowing about, because **five of the nine
/// opcodes are inert on an empty graph** — `Swap`, `Hop` and the three
/// `Local*` all need existing structure to walk, so early generations do
/// nothing until `Add`/`Toggle` have built something. Self-correcting, and
/// stated here so it is not read as a defect.
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
    base_graph: Option<&Graph>,
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

    // Unset means empty, which is the default an unseeded run gets.
    let starting_graph = match base_graph {
        Some(graph) => graph.clone(),
        None => Graph::new(config.network_size, config.max_edge_multiplicity),
    };

    let context = EdgeEditContext {
        base_graph: starting_graph,
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

/// The per-run seed list for a replicate request: one master seed in, `n_runs`
/// seeds out, in run order.
///
/// The master seeds a generator whose output stream *is* the seed list — run
/// `i` takes draw `i`. That buys one property deliberately: **a run's seed does
/// not depend on how many runs were asked for.** Extending an experiment from 30
/// replicates to 50 reproduces the first 30 exactly, so replicates already
/// collected are never invalidated by asking for more.
///
/// `master + i` or `hash(master, i)` would give the same property. `master ^ i`
/// would not — nearby masters collide across run indices, so master 4 run 1 and
/// master 5 run 0 are the same run.
///
/// Each seed returned is then a whole run's `seed` argument, drawn from
/// independent generator state rather than shared with any other replicate.
pub(crate) fn replicate_seeds(master: u64, n_runs: usize) -> Vec<u64> {
    let mut rng = ChaCha8Rng::seed_from_u64(master);
    let mut seeds = Vec::with_capacity(n_runs);
    for _ in 0..n_runs {
        seeds.push(rng.random::<u64>());
    }
    seeds
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
/// `base_graph` reaches only the edge-edit arm. The SDA genome generates its
/// graph rather than editing one, so there is nothing for a base to seed —
/// which is why `set_base_graph` refuses an SDA-configured evolver outright
/// instead of accepting a value that dies here.
///
/// # Errors
///
/// `ValueError` for any dimension the genome constructors reject. `Config::validate`
/// has already run at construction, so these are backstops rather than the first
/// line of defence — see [`sda_start`].
pub(crate) fn evolve<F: Fitness>(
    config: &Config,
    fitness: &F,
    base_graph: Option<&Graph>,
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
            let (genome_context, population) = edge_edit_start(
                config,
                *gene_length,
                *operation_weights,
                base_graph,
                &mut rng,
            )?;
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

/// How many replicates may execute at once.
///
/// Never more than the caller allowed, never more than there are runs — extra
/// threads past the replicate count have nothing to do — and never zero, which
/// `ThreadPoolBuilder` reads as "pick a default" rather than "run nothing".
fn effective_concurrency(max_cores: Option<usize>, n_runs: usize) -> usize {
    let available = match max_cores {
        Some(cap) => cap,
        // `available_parallelism` fails on some constrained targets; one thread
        // is the honest fallback, since the alternative is guessing high.
        None => std::thread::available_parallelism().map_or(1, |n| n.get()),
    };
    available.min(n_runs).max(1)
}

/// Run one replicate per entry in `seeds`, and hand back their outcomes **in
/// run order**.
///
/// `objectives` is parallel to `seeds` — one objective per run, built by the
/// caller before this is entered. They are not built here because each needs a
/// fresh per-run instance (an SIR objective owns an epidemic counter, and
/// sharing one across concurrent runs lets thread scheduling decide which run
/// sees which epidemic seed), and because building a Python objective touches
/// the interpreter, which must not happen inside a rayon worker.
///
/// **The engine picks parallel or sequential; the caller does not.** A native
/// Rust objective scores without the interpreter, so replicates are independent
/// and concurrency is nearly free. Under `fitness = "python"` every scoring call
/// re-acquires the GIL, so `n` concurrent runs are `n` threads contending for
/// one lock — slower than sequential *and* contended. Exposing that as a setting
/// would only create a way to choose wrong.
///
/// **The pool is built here, per call, never configured globally.** Rayon's
/// global pool can be configured once per process, and this crate is a Python
/// extension module imported once per session — a global configuration would
/// make `max_cores` a property of whichever `run` happened first, and the second
/// call with a different cap would fail outright.
///
/// # Errors
///
/// `ValueError` if the thread pool cannot be built, or from any replicate that
/// fails — the first such error is returned and the rest are abandoned.
pub(crate) fn run_replicates(
    config: &Config,
    objectives: &[Box<dyn Fitness>],
    base_graph: Option<&Graph>,
    seeds: &[u64],
    max_cores: Option<usize>,
) -> PyResult<Vec<ErasedOutcome>> {
    debug_assert_eq!(objectives.len(), seeds.len(), "one objective per run");

    // Nothing to run, and in particular no pool to size: `effective_concurrency`
    // would clamp to 1 and build a pool for zero work.
    if seeds.is_empty() {
        return Ok(Vec::new());
    }

    if matches!(config.fitness, FitnessConfig::Python) {
        let mut outcomes = Vec::with_capacity(seeds.len());
        for (objective, &seed) in objectives.iter().zip(seeds) {
            outcomes.push(evolve(config, objective, base_graph, seed)?);
        }
        return Ok(outcomes);
    }

    let pool = ThreadPoolBuilder::new()
        .num_threads(effective_concurrency(max_cores, seeds.len()))
        .build()
        .map_err(|err| PyValueError::new_err(format!("could not build the thread pool: {err}")))?;

    // `zip` over two slices is an *indexed* parallel iterator, so `collect`
    // rebuilds the vector by position rather than by completion. That is what
    // makes the result order the run order: the same master seed has to give the
    // same output ordering on every machine, whatever order the runs finish in.
    pool.install(|| {
        seeds
            .par_iter()
            .zip(objectives.par_iter())
            .map(|(&seed, objective)| evolve(config, objective, base_graph, seed))
            .collect::<PyResult<Vec<ErasedOutcome>>>()
    })
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
/// and this is the boundary where that stops being true.
/// `Direction::orient` is its own inverse, so it is also what undoes itself.
///
/// **The history needs converting too, row by row** — and only its two fitness
/// columns. `std_dev` and `ci_95` are left exactly as the engine computed them,
/// because a spread is identical under negation. Orienting either would be a
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
            ci_95: row.ci_95,
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
            base_graph: None,
            config_toml: String::new(),
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

    /// A config whose `[genome]` block is `genome_block` and whose edge cap is
    /// `cap`, everything else fixed. The cap is a parameter because the
    /// cap-narrowing check only bites when a base graph was built under a wider
    /// one than the run it is fed into.
    fn config_with_genome_and_cap(genome_block: &str, cap: u32) -> Config {
        let text = format!(
            "population_size = 4\n\
             network_size = 8\n\
             max_edge_multiplicity = {cap}\n\
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
        evolver_with_genome_and_cap(genome_block, 3)
    }

    fn evolver_with_genome_and_cap(genome_block: &str, cap: u32) -> GraphEvolver {
        GraphEvolver {
            config: config_with_genome_and_cap(genome_block, cap),
            fitness_function: None,
            base_graph: None,
            config_toml: String::new(),
        }
    }

    fn test_rng() -> ChaCha8Rng {
        ChaCha8Rng::seed_from_u64(11)
    }

    #[test]
    fn the_edge_edit_start_sizes_the_population_and_the_empty_base_graph() {
        let evolver = evolver_with_genome("[genome]\ntype = \"edge_edit\"\ngene_length = 16\n");
        let weights = EdgeEditOperationWeights::default();

        let (context, population) =
            edge_edit_start(&evolver.config, 16, weights, None, &mut test_rng())
                .expect("default weights are usable");

        assert_eq!(population.len(), 4, "one individual per population_size");
        for genome in &population {
            assert_eq!(genome.genes.len(), 16, "each genome gets gene_length genes");
        }

        // No base graph passed, so the context gets an empty one sized from
        // config — the default an unseeded run starts from.
        assert_eq!(context.base_graph.num_nodes, 8);
        assert_eq!(
            context.base_graph.get_edge_list().len(),
            0,
            "no edges to start",
        );
    }

    #[test]
    fn a_set_base_graph_is_what_the_edge_edit_population_expresses_against() {
        let mut evolver = evolver_with_genome("[genome]\ntype = \"edge_edit\"\ngene_length = 16\n");
        let seeded = vec![(0, 1, 2), (3, 4, 1)];
        evolver
            .set_base_graph(8, seeded.clone())
            .expect("a graph matching the config is accepted");

        let (context, _) = edge_edit_start(
            &evolver.config,
            16,
            EdgeEditOperationWeights::default(),
            evolver.base_graph.as_ref(),
            &mut test_rng(),
        )
        .expect("default weights are usable");

        assert_eq!(
            context.base_graph.get_edge_list(),
            seeded,
            "the population expresses against what was set, not an empty graph",
        );
    }

    #[test]
    fn a_base_graph_whose_node_count_disagrees_with_the_config_is_rejected() {
        let mut evolver = evolver_with_genome("[genome]\ntype = \"edge_edit\"\ngene_length = 16\n");

        let err = evolver
            .set_base_graph(9, vec![(0, 1, 1)])
            .expect_err("9 nodes against a network_size of 8 must be rejected");

        let message = err.to_string();
        assert!(message.contains('9'), "names the graph's size: {message}");
        assert!(message.contains('8'), "names network_size: {message}");
        assert!(evolver.base_graph.is_none(), "nothing stored on rejection");
    }

    #[test]
    fn a_base_graph_edge_above_the_configs_cap_is_rejected_rather_than_clamped() {
        // `Graph::set_edge` clamps an over-cap weight instead of refusing it, so
        // a graph built under a wider cap would otherwise be narrowed silently
        // and evolved against — the caller never seeing a different graph from
        // the one they handed in.
        let mut evolver =
            evolver_with_genome_and_cap("[genome]\ntype = \"edge_edit\"\ngene_length = 16\n", 1);

        let err = evolver
            .set_base_graph(8, vec![(0, 1, 1), (2, 3, 3)])
            .expect_err("multiplicity 3 against a cap of 1 must be rejected");

        let message = err.to_string();
        assert!(
            message.contains("(2, 3)"),
            "names the offending edge: {message}",
        );
        assert!(
            message.contains("multiplicity 3"),
            "names the offending value, not just the edge: {message}",
        );
        assert!(
            message.contains('1'),
            "names the cap it exceeded: {message}"
        );
        assert!(evolver.base_graph.is_none(), "nothing stored on rejection");
    }

    #[test]
    fn a_base_graph_edge_naming_a_node_outside_the_network_is_rejected() {
        // The node-count check compares one number against another and never
        // looks at the edges, so a caller taking `num_nodes` from their config
        // rather than their data passes it while every out-of-range edge is
        // dropped by `Graph::set_edge` without a word.
        let mut evolver = evolver_with_genome("[genome]\ntype = \"edge_edit\"\ngene_length = 16\n");

        let err = evolver
            .set_base_graph(8, vec![(0, 1, 1), (2, 9, 1)])
            .expect_err("node 9 in an 8-node network must be rejected");

        let message = err.to_string();
        assert!(
            message.contains("(2, 9)"),
            "names the offending edge: {message}",
        );
        assert!(
            message.contains('8'),
            "names the range it fell outside: {message}",
        );
        assert!(evolver.base_graph.is_none(), "nothing stored on rejection");
    }

    #[test]
    fn a_base_graph_self_loop_is_rejected_rather_than_dropped() {
        // This graph has no representation for a self-loop, and one in caller
        // data almost always means the indices are wrong — 1-indexed edge lists
        // being the common case, which also lands every survivor on the wrong
        // vertex. Reported rather than absorbed.
        let mut evolver = evolver_with_genome("[genome]\ntype = \"edge_edit\"\ngene_length = 16\n");

        let err = evolver
            .set_base_graph(8, vec![(0, 1, 1), (3, 3, 1)])
            .expect_err("a self-loop must be rejected");

        let message = err.to_string();
        assert!(
            message.contains("(3, 3)"),
            "names the offending edge: {message}",
        );
        assert!(
            message.contains("self-loop"),
            "says what is wrong with it: {message}",
        );
        assert!(evolver.base_graph.is_none(), "nothing stored on rejection");
    }

    #[test]
    fn a_base_graph_is_rejected_on_an_sda_configured_evolver() {
        // The SDA genome generates its graph rather than editing one, so a
        // stored base would never be read. Accepting it looks, from Python,
        // exactly like having seeded the run.
        let mut evolver =
            evolver_with_genome("[genome]\ntype = \"sda\"\nnum_states = 5\nmax_resp_len = 3\n");

        let err = evolver
            .set_base_graph(8, vec![(0, 1, 1)])
            .expect_err("an SDA run has no base graph to seed");

        let message = err.to_string();
        assert!(
            message.contains("edge_edit"),
            "points at what would work: {message}",
        );
        assert!(evolver.base_graph.is_none(), "nothing stored on rejection");
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
            .run(1, 1, None)
            .expect_err("init_state 9 with num_states 4 must be rejected");

        let message = err.to_string();
        assert!(message.contains("init_state"), "names the field: {message}");
        assert!(message.contains('4'), "names num_states: {message}");
    }

    #[test]
    fn asking_for_more_replicates_does_not_move_the_earlier_ones() {
        // The property the stream exists for: run `i`'s seed is a function of
        // the master and `i` alone, never of how many runs were requested. A
        // user who collects 30 replicates and later wants 50 keeps the 30.
        let thirty = replicate_seeds(20260813, 30);
        let fifty = replicate_seeds(20260813, 50);

        assert_eq!(thirty.len(), 30, "one seed per requested run");
        assert_eq!(fifty.len(), 50, "one seed per requested run");
        assert_eq!(
            fifty[..30],
            thirty[..],
            "the first 30 of a 50-run request must be the 30-run request, exactly",
        );
    }

    #[test]
    fn replicate_seeds_do_not_collide_across_nearby_masters() {
        // `master ^ i` is the anti-pattern this rules out: under it, master 4
        // run 1 and master 5 run 0 are the same run. Two adjacent masters must
        // share no seed at any index.
        let first = replicate_seeds(4, 8);
        let second = replicate_seeds(5, 8);

        for (index, seed) in first.iter().enumerate() {
            assert!(
                !second.contains(seed),
                "seed {seed} from master 4 (run {index}) also appears under master 5",
            );
        }
    }

    #[test]
    fn each_replicate_gets_a_distinct_seed() {
        // Two replicates sharing a seed would be the same run twice, which is
        // not a replicate — it reports as agreement between independent runs.
        let seeds = replicate_seeds(7, 32);

        for (index, seed) in seeds.iter().enumerate() {
            assert!(
                !seeds[index + 1..].contains(seed),
                "seed at run {index} repeats later in the same request",
            );
        }
    }

    #[test]
    fn a_zero_run_request_yields_no_seeds() {
        // The degenerate end of `min(max_cores, n_runs)`: nothing to run, and
        // nothing drawn, rather than one seed drawn and discarded.
        assert!(replicate_seeds(1, 0).is_empty());
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
            None,
            &mut ChaCha8Rng::seed_from_u64(5),
        )
        .expect("first build");
        let (_, second) = edge_edit_start(
            &evolver.config,
            12,
            weights,
            None,
            &mut ChaCha8Rng::seed_from_u64(5),
        )
        .expect("second build");
        let (_, different) = edge_edit_start(
            &evolver.config,
            12,
            weights,
            None,
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
            None,
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
        runnable_with_fitness(
            evolution_block,
            genome_block,
            "[fitness]\ntype = \"epi_spread\"\ninfection_rate = 0.3\nnum_epidemics = 2\n",
        )
    }

    /// The same, with the `[fitness]` block chosen — the replicate tests need a
    /// `python` one to reach the sequential arm, which no SIR block can.
    fn runnable_with_fitness(
        evolution_block: &str,
        genome_block: &str,
        fitness_block: &str,
    ) -> Config {
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
             {fitness_block}"
        );
        let config = Config::from_toml_str(&text).expect("the runnable config parses");
        config.validate().expect("the runnable config validates");
        config
    }

    /// A runnable `edge_edit` config whose expression step is a guaranteed
    /// no-op: `mutation_rate` and `crossover_rate` both zero, and
    /// `operation_weights` gives `null` the only non-zero weight. A population
    /// expressed against this never edits the base graph it started from, so
    /// a seeded run's output can be checked by exact equality rather than
    /// overlap — the same recipe as
    /// `a_set_base_graph_is_what_the_edge_edit_population_expresses_against`,
    /// lifted to the `run_replicates` level.
    fn no_op_runnable(evolution_block: &str) -> Config {
        let fitness_block = "[fitness]\ntype = \"epi_spread\"\ninfection_rate = 0.3\n\
             num_epidemics = 2\n";
        let text = format!(
            "population_size = 6\n\
             network_size = 8\n\
             max_edge_multiplicity = 2\n\
             crossover_rate = 0.0\n\
             mutation_rate = 0.0\n\
             \n\
             {evolution_block}\n\
             [selection]\n\
             type = \"tournament\"\n\
             tournament_size = 4\n\
             \n\
             [genome]\n\
             type = \"edge_edit\"\n\
             gene_length = 12\n\
             \n\
             [genome.operation_weights]\n\
             null = 1.0\n\
             toggle = 0.0\n\
             hop = 0.0\n\
             add = 0.0\n\
             delete = 0.0\n\
             swap = 0.0\n\
             local_toggle = 0.0\n\
             local_add = 0.0\n\
             local_delete = 0.0\n\
             \n\
             {fitness_block}"
        );
        let config = Config::from_toml_str(&text).expect("the no-op config parses");
        config.validate().expect("the no-op config validates");
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

            let outcome = evolve(&config, &objective, None, 1)
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

    /// One objective per seed, which is what `run_replicates` requires — a
    /// fresh instance each, never one shared, because an SIR objective owns a
    /// per-run epidemic counter.
    fn objectives_for(config: &Config, seeds: &[u64]) -> Vec<Box<dyn Fitness>> {
        let mut objectives: Vec<Box<dyn Fitness>> = Vec::with_capacity(seeds.len());
        for &seed in seeds {
            objectives.push(Box::new(EpiSpread::new(
                sir_sample_params(sir_of(config)),
                seed,
            )));
        }
        objectives
    }

    #[test]
    fn replicates_come_back_in_run_order_not_completion_order() {
        // The ordering guarantee the whole feature rests on: the same master
        // seed must give the same output ordering on every machine, however the
        // runs interleave. Checked by running the same seeds concurrently and
        // sequentially and requiring the two sequences to match element for
        // element — a completion-ordered collect would diverge under load.
        let config = runnable(GENERATIONAL, EDGE_EDIT);
        let seeds = replicate_seeds(20260813, 6);

        let concurrent = run_replicates(
            &config,
            &objectives_for(&config, &seeds),
            None,
            &seeds,
            Some(4),
        )
        .expect("concurrent replicates complete");
        let serial = run_replicates(
            &config,
            &objectives_for(&config, &seeds),
            None,
            &seeds,
            Some(1),
        )
        .expect("sequential replicates complete");

        assert_eq!(concurrent.len(), 6, "one outcome per seed");
        for (index, (parallel, sequential)) in concurrent.iter().zip(&serial).enumerate() {
            assert_eq!(
                parallel.best_fitness, sequential.best_fitness,
                "run {index} differs between max_cores=4 and max_cores=1",
            );
            assert_eq!(
                parallel.best_edges, sequential.best_edges,
                "run {index}'s graph differs between max_cores=4 and max_cores=1",
            );
        }
    }

    #[test]
    fn a_python_objective_runs_its_replicates_through_the_sequential_arm() {
        // The python arm end to end, with a real registered callable — not a
        // native config standing in for one. A timing assertion would prove
        // nothing here, so what this checks is that the branch completes and
        // scores through the callable at all: under the parallel arm these
        // would be n threads contending for one GIL.
        Python::attach(|py| {
            let mut evolver = GraphEvolver {
                config: runnable_with_fitness(GENERATIONAL, EDGE_EDIT, PYTHON_FITNESS),
                fitness_function: None,
                base_graph: None,
                config_toml: String::new(),
            };
            let callable = py
                .eval(
                    c"lambda batch: [float(len(edges)) for (n, edges) in batch]",
                    None,
                    None,
                )
                .expect("the lambda compiles");
            evolver
                .set_fitness_function(&callable, "maximize")
                .expect("a python config accepts a callable");

            assert!(
                matches!(evolver.config.fitness, FitnessConfig::Python),
                "fixture sanity: this must be the python arm, not a native one",
            );

            let seeds = replicate_seeds(11, 3);
            let mut objectives = Vec::with_capacity(seeds.len());
            for &seed in &seeds {
                objectives.push(evolver.objective(seed).expect("objective per run"));
            }

            // `max_cores` is deliberately generous: under the python arm it is
            // moot, and passing it proves the cap does not drag the config onto
            // the parallel path.
            let outcomes = run_replicates(&evolver.config, &objectives, None, &seeds, Some(8))
                .expect("python replicates complete sequentially");

            assert_eq!(outcomes.len(), 3, "one outcome per seed");
            for (index, outcome) in outcomes.iter().enumerate() {
                assert!(
                    outcome.best_fitness.is_finite(),
                    "run {index} scored through the callable",
                );
            }
        });
    }

    #[test]
    fn a_native_objective_runs_with_the_core_cap_unset() {
        // The other half of the gate: unset `max_cores` means all available,
        // which must still complete rather than building a zero-width pool.
        let config = runnable(GENERATIONAL, EDGE_EDIT);
        let seeds = replicate_seeds(11, 3);

        let outcomes = run_replicates(
            &config,
            &objectives_for(&config, &seeds),
            None,
            &seeds,
            None,
        )
        .expect("replicates complete with max_cores unset");

        assert_eq!(outcomes.len(), 3, "one outcome per seed, cap unset");
    }

    #[test]
    fn a_zero_replicate_request_yields_no_outcomes() {
        // Named for what is actually asserted. The early return in
        // `run_replicates` also avoids constructing a pool for zero work —
        // rayon reads `num_threads(0)` as "pick a default", so an unguarded
        // empty request would build a full-width pool to do nothing — but that
        // is not observable from here and this test does not claim it.
        let config = runnable(GENERATIONAL, EDGE_EDIT);

        let outcomes = run_replicates(&config, &[], None, &[], Some(8))
            .expect("an empty request is not an error");

        assert!(outcomes.is_empty(), "no seeds means no outcomes");
    }

    #[test]
    fn concurrency_is_capped_by_the_run_count_and_the_caller() {
        // `min(max_cores, n_runs)`, and never zero. Threads beyond the replicate
        // count have nothing to do, and zero means "default" to rayon.
        assert_eq!(effective_concurrency(Some(8), 3), 3, "capped by run count");
        assert_eq!(
            effective_concurrency(Some(2), 30),
            2,
            "capped by the caller"
        );
        assert_eq!(effective_concurrency(Some(1), 30), 1, "fully sequential");
        assert_eq!(effective_concurrency(Some(0), 4), 1, "never zero threads");
        assert_eq!(effective_concurrency(None, 1), 1, "unset, but one run");
        assert!(
            effective_concurrency(None, 64) >= 1,
            "unset means all available, whatever this machine has",
        );
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

        let outcome = evolve(&config, &objective, None, 6).expect("run completes");

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
            // Same reasoning as std_dev: ci_95 is a spread, so erase must leave
            // it alone rather than orient it.
            assert!(
                row.ci_95 >= 0.0,
                "iteration {}: ci_95 is never negative, got {}",
                row.iteration,
                row.ci_95,
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
            evolve(&config, &objective, None, seed).expect("run completes")
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

        let a = evolve(&generational, &first, None, 2).expect("generational runs");
        let b = evolve(&steady, &second, None, 2).expect("steady-state runs");

        assert_ne!(
            a.best_edges, b.best_edges,
            "the two strategies should not be producing identical runs",
        );
    }

    #[test]
    fn run_returns_a_complete_result_object() {
        let config_toml = "population_size = 6\n\
             network_size = 8\n\
             max_edge_multiplicity = 2\n\
             crossover_rate = 0.8\n\
             mutation_rate = 0.5\n\
             \n\
             [evolution]\n\
             type = \"generational\"\n\
             num_generations = 3\n\
             \n\
             [selection]\n\
             type = \"tournament\"\n\
             tournament_size = 4\n\
             \n\
             [genome]\n\
             type = \"edge_edit\"\n\
             gene_length = 12\n\
             \n\
             [fitness]\n\
             type = \"epi_spread\"\n\
             infection_rate = 0.3\n\
             num_epidemics = 2\n"
            .to_string();
        // Through the Python entry point, so this also exercises the GIL release.
        let mut evolver = GraphEvolver {
            config: Config::from_toml_str(&config_toml).expect("the fixture parses"),
            fitness_function: None,
            base_graph: None,
            config_toml: config_toml.clone(),
        };

        let [result] = <[_; 1]>::try_from(
            evolver
                .run(8, 1, None)
                .expect("a full config run completes"),
        )
        .expect("one run returns exactly one result");

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

        // Task 4's verify-by: seed, run_index and the config TOML reach the
        // result and the TOML round-trips.
        assert_eq!(result.seed, 8, "the seed run was called with");
        assert_eq!(result.run_index, 0, "hard 0 until replicates land (#20)");
        Config::from_toml_str(&result.config_toml).expect("the provenance TOML round-trips");
        assert_eq!(result.config_toml, config_toml);
    }

    /// An evolver on the small runnable config, for the replicate tests.
    fn replicate_evolver() -> GraphEvolver {
        GraphEvolver {
            config: runnable(GENERATIONAL, EDGE_EDIT),
            fitness_function: None,
            base_graph: None,
            config_toml: String::new(),
        }
    }

    #[test]
    fn the_core_cap_changes_the_speed_and_never_the_answer() {
        // The issue's own verify-by, and the strongest isolation check available
        // at this level: if replicates shared an objective, its per-run epidemic
        // counter would be advanced by whichever run got there first, so four
        // concurrent replicates would not reproduce four sequential ones. Equal
        // results across the two caps is that cross-talk not happening.
        let mut evolver = replicate_evolver();

        let sequential = evolver
            .run(20260813, 4, Some(1))
            .expect("four replicates, one at a time");
        let concurrent = evolver
            .run(20260813, 4, Some(8))
            .expect("four replicates, up to eight at a time");

        assert_eq!(sequential.len(), 4, "one result per requested run");
        assert_eq!(concurrent.len(), 4, "one result per requested run");
        for (index, (serial, parallel)) in sequential.iter().zip(&concurrent).enumerate() {
            assert_eq!(
                serial.best_fitness, parallel.best_fitness,
                "run {index}'s fitness depends on the core cap",
            );
            assert_eq!(
                serial.best_edges, parallel.best_edges,
                "run {index}'s graph depends on the core cap",
            );
            assert_eq!(
                serial.best_genome_repr, parallel.best_genome_repr,
                "run {index}'s genome depends on the core cap",
            );
            assert_eq!(serial.run_index, index, "results arrive in run order");
        }
    }

    #[test]
    fn extending_a_request_reproduces_the_replicates_already_collected() {
        // The other half of the issue's verify-by, through the public call
        // rather than the seed helper: a user who has collected three
        // replicates and wants five keeps the three they had.
        let mut evolver = replicate_evolver();

        let three = evolver.run(99, 3, Some(2)).expect("three replicates");
        let five = evolver.run(99, 5, Some(2)).expect("five replicates");

        assert_eq!(five.len(), 5, "the larger request runs all five");
        for (index, (small, large)) in three.iter().zip(&five).enumerate() {
            assert_eq!(
                small.best_fitness, large.best_fitness,
                "run {index} moved when more runs were requested",
            );
            assert_eq!(
                small.best_edges, large.best_edges,
                "run {index}'s graph moved when more runs were requested",
            );
        }
    }

    #[test]
    fn replicates_are_different_runs_rather_than_the_same_run_repeated() {
        // Guards the failure every reproducibility test above would wave
        // through: if each replicate were handed the master seed instead of its
        // own draw, all n results would be identical and every "same seed, same
        // answer" assertion would still pass.
        let mut evolver = replicate_evolver();

        let results = evolver.run(7, 4, Some(2)).expect("four replicates");

        let first = &results[0];
        let all_identical = results
            .iter()
            .all(|r| r.best_edges == first.best_edges && r.best_fitness == first.best_fitness);
        assert!(
            !all_identical,
            "all four replicates produced the same run — each is not getting its own seed",
        );
    }

    #[test]
    fn every_replicate_carries_the_master_seed_and_its_own_index() {
        // `(seed, run_index)` is the pair that reproduces a replicate, and it is
        // what `save_logs` stamps on every row so concatenated replicate logs
        // can be separated again.
        let mut evolver = replicate_evolver();

        let results = evolver.run(4242, 3, None).expect("three replicates");

        for (index, result) in results.iter().enumerate() {
            assert_eq!(result.seed, 4242, "the master seed, not the per-run draw");
            assert_eq!(result.run_index, index, "0-based position in the request");
        }
    }

    #[test]
    fn two_runs_on_one_evolver_do_not_leak_state() {
        // The reason `run` returns a value instead of caching one: an evolver
        // that held the previous run's result would hand the next run the wrong
        // numbers, and nothing about them would look wrong.
        let mut evolver = GraphEvolver {
            config: runnable(GENERATIONAL, EDGE_EDIT),
            fitness_function: None,
            base_graph: None,
            config_toml: String::new(),
        };

        let [first] = <[_; 1]>::try_from(evolver.run(4, 1, None).expect("first run"))
            .expect("one run returns exactly one result");
        let [second] = <[_; 1]>::try_from(evolver.run(5, 1, None).expect("second run"))
            .expect("one run returns exactly one result");
        let [first_again] =
            <[_; 1]>::try_from(evolver.run(4, 1, None).expect("first run, repeated"))
                .expect("one run returns exactly one result");

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
    fn save_logs_writes_one_row_per_logged_iteration_plus_a_header() {
        // Task 5's verify-by: row count is `num_generations + 1` under
        // generational and `num_mating_events / population_size + 1` under
        // steady-state. `GENERATIONAL` and `STEADY_STATE` are `runnable`'s
        // fixtures: population_size 6, num_generations 3, num_mating_events 12
        // — 4 and 3 rows respectively.
        for (evolution, expected_rows) in [(GENERATIONAL, 4), (STEADY_STATE, 3)] {
            let mut evolver = GraphEvolver {
                config: runnable(evolution, EDGE_EDIT),
                fitness_function: None,
                base_graph: None,
                config_toml: String::new(),
            };
            let [result] = <[_; 1]>::try_from(
                evolver
                    .run(3, 1, None)
                    .expect("a full config run completes"),
            )
            .expect("one run returns exactly one result");
            assert_eq!(result.history.len(), expected_rows);

            let path = std::env::temp_dir().join(format!(
                "get_save_logs_test_{}_{expected_rows}.csv",
                std::process::id(),
            ));
            result
                .save_logs(path.to_str().expect("temp path is valid UTF-8"))
                .expect("save_logs writes successfully");

            let contents = std::fs::read_to_string(&path).expect("the file was written");
            std::fs::remove_file(&path).expect("temp file cleans up");

            let lines: Vec<&str> = contents.lines().collect();
            assert_eq!(
                lines[0],
                "iteration,best_fitness,mean_fitness,std_dev,ci_95,seed,run_index",
            );
            assert_eq!(
                lines.len(),
                expected_rows + 1,
                "header plus one row per logged iteration",
            );
            for line in &lines[1..] {
                let fields: Vec<&str> = line.split(',').collect();
                assert_eq!(fields.len(), 7);
                assert_eq!(fields[5], "3", "seed column carries the run's seed");
                assert_eq!(fields[6], "0", "run_index column is the hard 0");
            }
        }
    }

    #[test]
    fn save_results_writes_the_best_individual_and_a_reparseable_config() {
        // Task 6's verify-by: both files exist, and the derived TOML path
        // parses back through the same `Config::from_toml_str` the run itself
        // used — that round-trip is the whole point of the provenance record.
        let config_toml = "population_size = 6\n\
             network_size = 8\n\
             max_edge_multiplicity = 2\n\
             crossover_rate = 0.8\n\
             mutation_rate = 0.5\n\
             \n\
             [evolution]\n\
             type = \"generational\"\n\
             num_generations = 3\n\
             \n\
             [selection]\n\
             type = \"tournament\"\n\
             tournament_size = 4\n\
             \n\
             [genome]\n\
             type = \"edge_edit\"\n\
             gene_length = 12\n\
             \n\
             [fitness]\n\
             type = \"epi_spread\"\n\
             infection_rate = 0.3\n\
             num_epidemics = 2\n"
            .to_string();
        let mut evolver = GraphEvolver {
            config: Config::from_toml_str(&config_toml).expect("the fixture parses"),
            fitness_function: None,
            base_graph: None,
            config_toml: config_toml.clone(),
        };
        let [result] = <[_; 1]>::try_from(
            evolver
                .run(7, 1, None)
                .expect("a full config run completes"),
        )
        .expect("one run returns exactly one result");

        let path =
            std::env::temp_dir().join(format!("get_save_results_test_{}", std::process::id()));
        let path_str = path.to_str().expect("temp path is valid UTF-8");
        result
            .save_results(path_str)
            .expect("save_results writes successfully");

        let contents = std::fs::read_to_string(&path).expect("the results file was written");
        std::fs::remove_file(&path).expect("results temp file cleans up");
        assert!(contents.contains(&format!("best_fitness = {}", result.best_fitness)));
        assert!(contents.contains(&result.best_genome_repr));
        for &(u, v, weight) in &result.best_edges {
            assert!(contents.contains(&format!("{u},{v},{weight}")));
        }

        let toml_path = format!("{path_str}.toml");
        let toml_contents = std::fs::read_to_string(&toml_path).expect("the TOML file was written");
        std::fs::remove_file(&toml_path).expect("TOML temp file cleans up");
        assert_eq!(toml_contents, config_toml);
        Config::from_toml_str(&toml_contents).expect("the provenance TOML round-trips");
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
            evolve(&config, &objective, None, 3)
                .expect("runs")
                .best_fitness
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
