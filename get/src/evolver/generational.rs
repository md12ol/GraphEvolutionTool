//! Generational evolution: the whole population is replaced each generation,
//! carrying a configurable number of elites forward unchanged.

use std::cmp::Ordering;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use super::common::{express_and_score, generation_stats, mutate_child, rank};
use super::{
    EvolutionOutcome, Evolver, GenerationStats, GenerationalContext, SharedEvolutionContext,
};
use crate::fitness::{Direction, Fitness};
use crate::genomes::Genome;
use crate::graph::Graph;

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
    ///
    /// Takes `fitnesses` for the population it is replacing and does no scoring:
    /// the whole next generation is scored in one batch by [`Evolver::run`].
    ///
    /// **Odd slot count.** Crossover yields two children, but
    /// `population_size - elite_count` may be odd. The last pair contributes one
    /// child and the other is discarded; only the final pair is ever affected.
    /// Spec §6.2.
    fn advance_generation<R>(&mut self, fitnesses: &[f64], rng: &mut R)
    where
        R: Rng + ?Sized,
    {
        let population_size = self.population.len();
        let mut next = Vec::with_capacity(population_size);

        // Elites first, copied forward unchanged. Ranking uses the same
        // comparator selection does, so "best" means one thing engine-wide.
        // They are not exempt from being rescored next generation — see `run`.
        if self.context.elite_count > 0 {
            let mut by_rank: Vec<usize> = (0..population_size).collect();
            by_rank.sort_by(|&a, &b| rank(fitnesses, a, b));
            for &slot in &by_rank[..self.context.elite_count] {
                next.push(self.population[slot].clone());
            }
        }

        // Then breed the rest. Selection samples with replacement (spec §6.1),
        // so a pair can be one individual and its own clone — crossover then
        // does nothing and only mutation moves the child.
        while next.len() < population_size {
            let mut pair = self
                .shared
                .selection
                .select(&self.population, fitnesses, 2, rng);
            let mut second = pair.pop().expect("select returned two parents");
            let mut first = pair.pop().expect("select returned two parents");

            // One crossover roll for the pair, then the mutation rolls per
            // child, in the same fixed order steady-state uses so the two
            // strategies consume their RNG the same way.
            if rng.random_bool(self.shared.crossover_rate) {
                first.crossover(&mut second, rng);
            }
            mutate_child(
                &mut first,
                self.shared.mutation_rate,
                self.shared.max_mutations,
                rng,
            );
            mutate_child(
                &mut second,
                self.shared.mutation_rate,
                self.shared.max_mutations,
                rng,
            );

            // Both children are always bred, so a pass costs the same RNG
            // whether or not its second child is kept. On an odd fill count the
            // final pass discards `second`.
            next.push(first);
            if next.len() < population_size {
                next.push(second);
            }
        }

        self.population = next;
    }

    /// Package the best individual and the accumulated history into an outcome.
    ///
    /// Moves the winner's graph out of the ones the final scoring pass built
    /// rather than re-expressing it — generational scores every individual every
    /// generation, so the graph already exists. Spec §6.2.
    ///
    /// The winner is the best of the **final** population, which is the same
    /// individual the last history row reports, and the same rule steady-state's
    /// `outcome` uses.
    ///
    /// `direction` is stored, not applied: the outcome leaves here in engine
    /// orientation and the boundary converts it once. Spec §5.1.
    fn outcome(
        &mut self,
        mut graphs: Vec<Graph>,
        fitnesses: &[f64],
        direction: Direction,
    ) -> EvolutionOutcome<G> {
        // Non-empty: `new` asserts elite_count is smaller than the population,
        // and generational replaces individuals without ever removing one.
        let mut best = 0;
        for candidate in 1..fitnesses.len() {
            if rank(fitnesses, candidate, best) == Ordering::Less {
                best = candidate;
            }
        }

        EvolutionOutcome {
            best_genome: self.population[best].clone(),
            // swap_remove moves the winner out without cloning; the rest of the
            // graphs are dropped here anyway.
            best_graph: graphs.swap_remove(best),
            best_fitness_engine: fitnesses[best],
            direction,
            // mem::take moves history out and leaves an empty Vec in its place,
            // so `self` stays valid without cloning the whole log.
            history: std::mem::take(&mut self.history),
        }
    }
}

impl<G: Genome> Evolver<G> for GenerationalEvolver<G> {
    type TypeContext = GenerationalContext;

    fn new(
        shared: SharedEvolutionContext<G>,
        type_context: Self::TypeContext,
        population: Vec<G>,
    ) -> Self {
        // Backstop. The config layer should reject this before we get here, but
        // the evolver is constructible directly (tests, embedding), so it checks
        // rather than trusting its caller. The failure it catches is the silent
        // kind: elites fill every slot, nothing breeds, and the run is a fixed
        // point that reads as a broken fitness function.
        //
        // There is deliberately no matching `population.len() >= tournament_size`
        // assert as steady-state has — generational samples with replacement
        // (spec §6.1), so an oversized tournament still draws fine.
        assert!(
            type_context.elite_count < population.len(),
            "elite_count {} must be smaller than the population of {}: every \
             slot would be filled by an elite and nothing would breed",
            type_context.elite_count,
            population.len(),
        );

        Self {
            shared,
            context: type_context,
            population,
            history: Vec::new(),
        }
    }

    /// Score, log, advance — `num_generations` times, then report the best.
    ///
    /// The initial population is scored and logged as **generation 0**, matching
    /// steady-state's iteration-0 row, so the two strategies' histories share an
    /// axis and a zero-generation run still says where it began. So
    /// `history.len() == num_generations + 1`. Spec §6.2, §6.4.
    ///
    /// **Every individual is rescored every generation, elites included.** They
    /// are copied forward unchanged, not exempted from scoring: under a
    /// stochastic objective an elite's number moves while its genome does not,
    /// and that is correct — the new number is a fresh sample of the same
    /// individual, where keeping the old one would let a lucky draw persist.
    /// Spec §6.2, §5.2.
    fn run<F: Fitness>(&mut self, fitness: &F, seed: u64) -> EvolutionOutcome<G> {
        // ChaCha8 rather than StdRng: StdRng's algorithm is allowed to change
        // between `rand` releases, which would silently break the reproducibility
        // this `seed` argument exists to provide.
        let mut rng = ChaCha8Rng::seed_from_u64(seed);

        // express_and_score is the engine's only scoring entry — it is what
        // orients the fitnesses and rejects NaN. Calling `Fitness::evaluate`
        // here instead would bypass both, silently.
        let (mut graphs, mut fitnesses) =
            express_and_score(&self.population, &self.shared.genome_context, fitness);

        self.history.clear();
        self.history.push(generation_stats(0, &fitnesses));

        for generation in 1..=self.context.num_generations {
            self.advance_generation(&fitnesses, &mut rng);

            let scored = express_and_score(&self.population, &self.shared.genome_context, fitness);
            graphs = scored.0;
            fitnesses = scored.1;

            self.history.push(generation_stats(generation, &fitnesses));
        }

        self.outcome(graphs, &fitnesses, fitness.direction())
    }
}
