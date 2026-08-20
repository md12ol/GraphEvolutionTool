//! The genetic-algorithm engine that drives genomes toward a fitness target.
//!
//! [`Evolver`] is the shared interface; [`generational`] and [`steady_state`]
//! provide the two evolution strategies. Both are generic over the [`Genome`]
//! representation, so the same engine drives edge-edit and SDA genomes alike.

pub mod common;
pub mod generational;
pub mod steady_state;

// Test doubles both strategies' test modules share. Compiled out of the lib
// target entirely, so nothing here ships.
#[cfg(test)]
pub(crate) mod test_support;

// Re-exported so callers write evolver::GenerationalEvolver rather than
// reaching into the generational submodule directly.
pub use generational::GenerationalEvolver;
pub use steady_state::SteadyStateEvolver;

use crate::fitness::{Direction, Fitness};
use crate::genomes::Genome;
use crate::graph::Graph;

use common::Selection;

/// Run-level configuration shared by every evolution strategy.
///
/// Field names follow the planning document's "Shared Evolution Context", minus
/// everything that is already knowable from somewhere else. Each omission is
/// deliberate, and for the same reason: a second copy of a value can drift out
/// of step with the original, and nothing would report the disagreement.
///
/// - **Population size** is the length of the population handed to
///   [`Evolver::new`]; evolvers read `self.population.len()`.
/// - **Network size** and **edge-weight cap** belong to `genome_context` —
///   `SdaContext` states them directly, `EdgeEditContext` through its
///   `base_graph`. Read them from there, or from an expressed [`Graph`].
///
/// `config.toml` still carries `population_size`, `network_size`, and
/// `max_edge_multiplicity`; the dispatch layer is what turns them into a sized
/// population and a genome context, and is the one place they are read.
pub struct SharedEvolutionContext<G: Genome> {
    /// Genome-specific expression configuration (e.g. `EdgeEditContext`,
    /// `SdaContext`) supplied by the associated [`Genome::Context`] type.
    ///
    /// `G::Context` is not a static member of a class named `G` — it's the
    /// concrete type that `G`'s [`Genome`] implementation supplies for
    /// `Genome`'s associated `Context` type (Rust's per-implementation "type
    /// member").
    ///
    /// Also the authority on graph size and edge-weight cap — see above.
    pub genome_context: G::Context,
    /// Probability that a selected pair is recombined.
    pub crossover_rate: f64,
    /// Probability that a child is mutated at all.
    ///
    /// One half of a single conceptual knob, with `max_mutations` — whether a
    /// child mutates, then how many mutations it takes. Both are rolled by
    /// [`common::mutate_child`], never by the genome.
    pub mutation_rate: f64,
    /// Upper bound on how many mutations a mutating child takes, drawn uniformly
    /// from `1..=max_mutations`. Defaults to 1 in `config.toml`.
    ///
    /// Shared across representations by count, not by strength: one edge-edit
    /// gene of 256 is a far smaller perturbation than one SDA transition of 24.
    pub max_mutations: usize,
    /// Parent-selection strategy used by both evolution strategies.
    pub selection: Selection,
}

/// Extra configuration specific to the generational strategy.
pub struct GenerationalContext {
    /// Number of generations to evolve.
    pub num_generations: usize,
    /// Number of best individuals copied unchanged into each next generation.
    /// Configured via `config.toml`; defaults to 1.
    pub elite_count: usize,
}

/// Extra configuration specific to the steady-state strategy.
pub struct SteadyStateContext {
    /// Number of mate-and-replace events to perform.
    pub num_mating_events: usize,
}

/// A single row of the evolution log.
///
/// `iteration` counts generations for the generational strategy and mating
/// events for the steady-state strategy.
///
/// `best_fitness` and `mean_fitness` are in **engine orientation** — lower is
/// better — like everything else inside the engine. The boundary converts them
/// when it writes the log, and leaves `std_dev` and `ci_95` alone because a
/// spread is identical under negation. Spec §5.1, §6.4.
pub struct GenerationStats {
    pub iteration: usize,
    pub best_fitness: f64,
    pub mean_fitness: f64,
    pub std_dev: f64,
    /// Half-width of the 95% confidence interval on `mean_fitness`, using the
    /// *sample* deviation (divides by `n - 1`) rather than `std_dev`'s
    /// population deviation (divides by `n`) — the population is being used to
    /// estimate a distribution here, not fully described by it. Zero when
    /// `n == 1`, never `NaN`.
    pub ci_95: f64,
}

/// The result of an evolution run.
///
/// Carries the best genome together with its expressed [`Graph`], so callers
/// can inspect the genome and use the final network without re-expressing it.
///
/// # The numbers in here are engine-oriented, and `direction` is how you undo it
///
/// `best_fitness_engine` and every row of `history` are lower-is-better,
/// whatever the objective actually computed — the engine converts once inward at
/// [`common::express_and_score`] and never again. This struct is the far edge of
/// that region, so it carries the [`Direction`] the run used and the boundary
/// converts once outward: `direction.orient(outcome.best_fitness_engine)` is the
/// value in the objective's own units.
///
/// The cost is stated plainly in spec §5.1: a Rust embedder reading this
/// directly gets engine-oriented numbers. The field name says so rather than
/// leaving it to be discovered.
pub struct EvolutionOutcome<G: Genome> {
    pub best_genome: G,
    pub best_graph: Graph,
    /// Best fitness in **engine orientation** (lower is better). Convert with
    /// `direction.orient(..)` to get the objective's own units.
    pub best_fitness_engine: f64,
    /// The objective's direction, so the boundary can convert on the way out.
    pub direction: Direction,
    pub history: Vec<GenerationStats>,
}

/// A genetic-algorithm evolution strategy over genome type `G`.
///
/// Implementors pair the [`SharedEvolutionContext`] with their own
/// [`Evolver::TypeContext`] (generations or mating events) and drive a
/// population against a [`Fitness`] objective. [`generational`] and
/// [`steady_state`] are the two shipped strategies.
///
/// # Adding your own strategy
///
/// Unlike a new objective or genome, a new strategy *is* the loop: there is no
/// engine underneath it left to keep doing the work, so it has to reach
/// [`common`] for everything a strategy is not allowed to reinvent — see "What
/// the engine owns" below. That makes this extension point a **route-4**
/// change in practice, whichever way you read the two routes below:
///
/// - **You depend on this crate from your own program (route 3).** `Evolver`
///   is a public trait, so you can `impl Evolver<G>` for your own type and
///   call [`Evolver::run`] on it directly — nothing here stops you. But
///   `config`, `dispatch` and `py_config` are private modules, so your
///   strategy can never be named from a config file or selected by the
///   `get-run` binary, and [`common`]'s helpers are the only load-bearing
///   reuse available to you from outside — you still have to write your own
///   parent-selection call, your own mutation, your own scoring loop, using
///   them. [`Evolver::new`], [`Evolver::run`] and [`EvolutionOutcome`] stay
///   public and usable from outside the crate regardless — that is what lets
///   a route-3 caller drive one of the *shipped* strategies, which is the
///   common case.
/// - **You are editing your own copy of GET (route 4).** All seven steps below
///   are yours, and what that buys is a strategy selectable by name from
///   `config.toml` and runnable by `get-run`, with no Rust at the call site.
///
/// The steps, in the order you would walk them:
///
/// 1. **A new module beside [`generational`] and [`steady_state`]** —
///    `evolver/<name>.rs` — implementing [`Evolver<G>`] for your type, plus
///    the `pub mod` and re-export lines in this file that make the other two
///    strategies reachable as `evolver::GenerationalEvolver` and
///    `evolver::SteadyStateEvolver`.
/// 2. **`config.rs`** — add a variant to `EvolutionConfig` carrying your
///    strategy's own stopping condition (generations, mating events, or
///    whatever yours uses).
/// 3. **`config.rs`** — add whatever constraint `validate_evolution_and_selection`
///    needs on the new variant, alongside the strategy-specific checks already
///    there for the shipped two. See that function's doc.
/// 4. **`dispatch.rs`** — add the arm of `run_strategy`'s match that builds
///    your `TypeContext` from the new `EvolutionConfig` variant and constructs
///    your evolver. See that function's doc.
/// 5. **`dispatch.rs`** — nothing, if your strategy returns a normal
///    `EvolutionOutcome<G>`. `erase` is generic over `G` alone, with no
///    strategy match inside it, precisely so this step is a non-step — see
///    its doc for why it still gets a mention here instead of being left off
///    the list entirely.
/// 6. **`py_config.rs`** — optional. Add a `PyEvolutionConfig` variant
///    mirroring the one from step 2, if the strategy should be selectable
///    from Python. Skipping it costs nothing anywhere else: the strategy
///    still runs from a TOML config and from Rust, it is simply not nameable
///    from the Python front end. See that type's doc.
/// 7. **`config.example.toml`** — add or extend the `[evolution]` block if
///    the strategy ships. The example file is what a user copies from, so a
///    strategy missing from it is one most people never find.
///
/// # What the engine owns, so a strategy must not re-implement it
///
/// [`common`] is where the pieces a strategy is not free to redo live:
/// parent selection (`Selection::select`, reached through
/// [`SharedEvolutionContext::selection`]), the two mutation dice rolls
/// (`common::mutate_child`, `common::breed_pair` — whether a child mutates,
/// then how many mutations it takes), parallel expression and scoring
/// (`common::express_and_score`, the one place fitness is read, exactly
/// once per individual per batch), and the per-iteration log row
/// (`common::generation_stats`). A strategy that calls the fitness objective,
/// the selection scheme, or `Genome::mutate` directly instead of going
/// through these has diverged from the other strategies in exactly the way
/// #56 exists to clean up in the two that already ship — divergence a reader
/// discovers by diffing implementations, not by anything failing loudly.
///
/// # `EvolutionOutcome` is built exactly once, at the end of `run`
///
/// A strategy constructs one [`EvolutionOutcome`] per call to
/// [`Evolver::run`], after the loop has finished — never partway through, and
/// never more than one. It carries the final population's best genome and its
/// expressed graph together (so a caller never has to re-express the winner
/// to inspect the network it produced), the best fitness and the run's
/// `Direction` in engine orientation (see [`EvolutionOutcome`]'s own doc for
/// what "engine orientation" costs a caller), and the accumulated
/// `Vec<GenerationStats>` history, one row per `generation_stats` call along
/// the way.
///
/// [`generational`] and [`steady_state`] build the winner's graph
/// differently — one takes it from a set of graphs its final scoring pass
/// already built, the other re-expresses the genome — and each strategy's own
/// `outcome()` method says why, rather than restating it here: see
/// `GenerationalEvolver::outcome`'s and `SteadyStateEvolver::outcome`'s docs.
///
/// # Determinism
///
/// Two replicate runs at the same `seed` must agree. Every draw a strategy
/// makes — which parents, whether and how much a child mutates, anything else
/// specific to the strategy itself — has to come from the RNG [`Evolver::run`]
/// seeds from its own `seed` argument, and nowhere else: not the system clock,
/// not an address, not thread scheduling, not iteration order over a hash map.
/// [`Evolver::run`]'s own doc covers the `ChaCha8Rng` requirement this rests
/// on.
pub trait Evolver<G: Genome> {
    /// Strategy-specific configuration ([`GenerationalContext`] or
    /// [`SteadyStateContext`]).
    ///
    /// An "associated type": each implementor of `Evolver` fixes this to one
    /// concrete type, rather than `Evolver` taking a second generic parameter
    /// like `Evolver<G, T>`.
    type TypeContext;

    /// Build an evolver from the shared and strategy-specific contexts and a
    /// ready-made starting population.
    ///
    /// The caller supplies `population` because genome constructors differ per
    /// representation (and some are fallible), so a generic evolver cannot
    /// build a `G` itself. Building it in the config-dispatch layer keeps that
    /// knowledge where it already lives and surfaces invalid dimensions at
    /// startup rather than mid-run.
    fn new(
        shared: SharedEvolutionContext<G>,
        type_context: Self::TypeContext,
        population: Vec<G>,
    ) -> Self
    // `Self: Sized` opts only this method out of dynamic-dispatch support, so
    // `Evolver` can still be used as `dyn Evolver<G>` elsewhere — a
    // constructor can't be called through a trait object anyway.
    where
        Self: Sized;

    /// Evolve the population against `fitness`, seeding all randomness from
    /// `seed` for reproducibility, and return the best genome and its
    /// expressed graph.
    ///
    /// Implementations seed a `ChaCha8Rng`, not a `StdRng`: `StdRng`'s algorithm
    /// is allowed to change between `rand` releases, which would silently break
    /// the reproducibility this `seed` argument exists to provide. Stated here
    /// rather than in each strategy because it binds all of them.
    fn run<F: Fitness>(&mut self, fitness: &F, seed: u64) -> EvolutionOutcome<G>;
}
