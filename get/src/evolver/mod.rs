//! The genetic-algorithm engine that drives genomes toward a fitness target.
//!
//! [`Evolver`] is the shared interface; [`generational`] and [`steady_state`]
//! provide the two evolution strategies. Both are generic over the [`Genome`]
//! representation, so the same engine drives edge-edit and SDA genomes alike.

pub mod common;
pub mod generational;
pub mod steady_state;

pub use generational::GenerationalEvolver;
pub use steady_state::SteadyStateEvolver;

use crate::fitness::Fitness;
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
pub struct GenerationStats {
    pub iteration: usize,
    pub best_fitness: f64,
    pub mean_fitness: f64,
    pub std_dev: f64,
}

/// The result of an evolution run.
///
/// Carries the best genome together with its expressed [`Graph`], so callers
/// can inspect the genome and use the final network without re-expressing it.
pub struct EvolutionOutcome<G: Genome> {
    pub best_genome: G,
    pub best_graph: Graph,
    pub best_fitness: f64,
    pub history: Vec<GenerationStats>,
}

/// A genetic-algorithm evolution strategy over genome type `G`.
///
/// Implementors pair the [`SharedEvolutionContext`] with their own
/// [`Evolver::TypeContext`] (generations or mating events) and drive a
/// population against a [`Fitness`] objective.
pub trait Evolver<G: Genome> {
    /// Strategy-specific configuration ([`GenerationalContext`] or
    /// [`SteadyStateContext`]).
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
    where
        Self: Sized;

    /// Evolve the population against `fitness`, seeding all randomness from
    /// `seed` for reproducibility, and return the best genome and its
    /// expressed graph.
    fn run<F: Fitness>(&mut self, fitness: &F, seed: u64) -> EvolutionOutcome<G>;
}
