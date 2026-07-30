//! Steady-state evolution: each mating event produces a child that replaces the
//! worst individual, so most of the population persists between events.

use rand::Rng;

use super::{
    Evolver, EvolutionOutcome, GenerationStats, SharedEvolutionContext, SteadyStateContext,
};
use crate::fitness::Fitness;
use crate::genomes::Genome;

/// Evolves a population one mate-and-replace event at a time for a fixed number
/// of mating events.
pub struct SteadyStateEvolver<G: Genome> {
    shared: SharedEvolutionContext<G>,
    context: SteadyStateContext,
    population: Vec<G>,
    history: Vec<GenerationStats>,
}

impl<G: Genome> SteadyStateEvolver<G> {
    /// Perform one mating event: select parents, recombine them by
    /// `crossover_rate`, mutate the child by `mutation_rate`, and replace the
    /// worst current individual. The population's best is never discarded, so
    /// no explicit elitism is needed.
    fn mating_event<F, R>(&mut self, fitness: &F, fitnesses: &mut [f64], rng: &mut R)
    where
        F: Fitness,
        R: Rng + ?Sized,
    {
        let _ = (fitness, fitnesses, rng);
        todo!("select parents, breed a child, and replace the worst individual")
    }
}

impl<G: Genome> Evolver<G> for SteadyStateEvolver<G> {
    type TypeContext = SteadyStateContext;

    fn new(
        shared: SharedEvolutionContext<G>,
        type_context: Self::TypeContext,
        population: Vec<G>,
    ) -> Self {
        Self {
            shared,
            context: type_context,
            population,
            history: Vec::new(),
        }
    }

    fn run<F: Fitness>(&mut self, fitness: &F, seed: u64) -> EvolutionOutcome<G> {
        let _ = (fitness, seed);
        todo!("seed the RNG and run `num_mating_events` mating events")
    }
}
