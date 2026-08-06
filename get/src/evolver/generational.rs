//! Generational evolution: the whole population is replaced each generation,
//! carrying a configurable number of elites forward unchanged.

use rand::Rng;

use super::{
    EvolutionOutcome, Evolver, GenerationStats, GenerationalContext, SharedEvolutionContext,
};
use crate::fitness::Fitness;
use crate::genomes::Genome;

/// Evolves a population one full generation at a time for a fixed number of
/// generations.
pub struct GenerationalEvolver<G: Genome> {
    shared: SharedEvolutionContext<G>,
    context: GenerationalContext,
    population: Vec<G>,
    history: Vec<GenerationStats>,
}

impl<G: Genome> GenerationalEvolver<G> {
    /// Produce the next generation: copy `context.elite_count` elites forward,
    /// then fill the rest by selecting parents, recombining them with
    /// probability `crossover_rate`, and mutating each child through
    /// [`mutate_child`](super::common::mutate_child).
    ///
    /// That helper owns **both** mutation rolls — `mutation_rate` and
    /// `max_mutations` — so this strategy and steady-state cannot disagree about
    /// what they mean. Neither roll is made here.
    fn advance_generation<F, R>(&mut self, fitness: &F, fitnesses: &[f64], rng: &mut R)
    where
        F: Fitness,
        R: Rng + ?Sized,
    {
        let _ = (fitness, fitnesses, rng, self.context.elite_count);
        todo!("carry elites, select parents, recombine and mutate the rest")
    }
}

impl<G: Genome> Evolver<G> for GenerationalEvolver<G> {
    type TypeContext = GenerationalContext;

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
        todo!("seed the RNG and evolve the population for `num_generations`")
    }
}
