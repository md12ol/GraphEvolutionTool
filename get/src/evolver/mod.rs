//! The genetic-algorithm engine that drives genomes toward a fitness target.
//!
//! [`Evolver`] is the shared interface; [`generational`] and [`steady_state`]
//! are the two strategies.

pub mod common;
pub mod generational;
pub mod replacement;
pub mod scope;
pub mod steady_state;

#[cfg(test)]
pub(crate) mod test_support;

pub use generational::GenerationalEvolver;
pub use steady_state::SteadyStateEvolver;

// ADD A STRATEGY STEP 1 — a new module beside these two, `evolver/<name>.rs`,
// plus a `pub mod` line and a re-export above:
//
//     pub mod my_strategy;
//
//     pub use my_strategy::MyStrategyEvolver;

use crate::fitness::{Direction, Fitness};
use crate::genomes::Genome;
use crate::graph::Graph;

use common::{Crossover, Selection};
use replacement::Replacement;
use scope::Scope;

/// Run-level configuration shared by every evolution strategy.
///
/// Population size is not a field: it is the length of the population handed to
/// [`Evolver::new`].
pub struct SharedEvolutionContext<G: Genome> {
    /// Genome-specific expression configuration (e.g. `EdgeEditContext`,
    /// `SdaContext`), and the authority on graph size and edge-weight cap.
    pub genome_context: G::Context,
    /// Probability that a selected pair is recombined.
    pub crossover_rate: f64,
    /// Probability that a child is mutated at all.
    pub mutation_rate: f64,
    /// Upper bound on how many mutations a mutating child takes, drawn uniformly
    /// from `1..=max_mutations`.
    pub max_mutations: usize,
    /// Parent-selection strategy used by both evolution strategies.
    pub selection: Selection,
    /// The slice of the population one breeding event draws from.
    pub scope: Scope,
    /// Recombination operator, applied to every pair that passes the
    /// `crossover_rate` roll.
    pub crossover: Crossover,
}

/// Extra configuration specific to the generational strategy.
pub struct GenerationalContext {
    /// Number of generations to evolve.
    pub num_generations: usize,
    /// Number of best individuals copied unchanged into each next generation.
    pub elite_count: usize,
}

/// Extra configuration specific to the steady-state strategy.
pub struct SteadyStateContext {
    /// Number of mate-and-replace events to perform.
    pub num_mating_events: usize,
    /// Which members of the scope a mating event's children overwrite.
    pub replacement: Replacement,
}

/// A single row of the evolution log.
///
/// `iteration` counts generations for the generational strategy and mating
/// events for the steady-state strategy.
///
/// `best_fitness` and `mean_fitness` are in **engine orientation** — lower is
/// better. The boundary converts them when it writes the log, and leaves
/// `std_dev` and `ci_95` alone because a spread is identical under negation.
pub struct GenerationStats {
    pub iteration: usize,
    pub best_fitness: f64,
    pub mean_fitness: f64,
    pub std_dev: f64,
    /// Half-width of the 95% confidence interval on `mean_fitness`, using the
    /// *sample* deviation (divides by `n - 1`) rather than `std_dev`'s
    /// population deviation (divides by `n`). Zero when `n == 1`, never `NaN`.
    pub ci_95: f64,
}

/// The result of an evolution run.
///
/// Carries the best genome together with its expressed [`Graph`], so callers
/// can use the final network without re-expressing it.
pub struct EvolutionOutcome<G: Genome> {
    pub best_genome: G,
    pub best_graph: Graph,
    /// Best fitness in **engine orientation** (lower is better). Convert with
    /// `direction.orient(..)` to get the objective's own units.
    pub best_fitness_engine: f64,
    /// The objective's direction, so the boundary can convert on the way out.
    pub direction: Direction,
    /// One row per logged iteration, engine-oriented like `best_fitness_engine`.
    pub history: Vec<GenerationStats>,
}

/// A genetic-algorithm evolution strategy over genome type `G`.
///
/// Implementors pair the [`SharedEvolutionContext`] with their own
/// [`Evolver::TypeContext`] (generations or mating events) and drive a
/// population against a [`Fitness`] objective.
///
/// A strategy reaches [`common`] for parent selection, the two mutation rolls,
/// expression and scoring, and the per-iteration log row. Calling a [`Fitness`]
/// objective, a [`Selection`] scheme or [`Genome::mutate`] directly instead
/// makes this strategy disagree with the others in a way nothing reports — a
/// reader only finds it by diffing implementations.
pub trait Evolver<G: Genome> {
    /// Strategy-specific configuration ([`GenerationalContext`] or
    /// [`SteadyStateContext`]).
    type TypeContext;

    /// Build an evolver from the shared and strategy-specific contexts and a
    /// ready-made starting population.
    fn new(
        shared: SharedEvolutionContext<G>,
        type_context: Self::TypeContext,
        population: Vec<G>,
    ) -> Self
    where
        Self: Sized;

    /// Evolve the population against `fitness` and return the best genome and
    /// its expressed graph.
    ///
    /// Every draw an implementation makes comes from an RNG seeded from `seed`
    /// and from nowhere else — not the clock, not an address, not thread
    /// scheduling, not iteration order over a hash map — or two runs at the same
    /// seed disagree. Seed a `ChaCha8Rng`, not a `StdRng`, whose algorithm may
    /// change between `rand` releases.
    fn run<F: Fitness>(&mut self, fitness: &F, seed: u64) -> EvolutionOutcome<G>;
}
