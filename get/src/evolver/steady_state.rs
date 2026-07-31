//! Steady-state evolution: each mating event breeds two children inside one
//! tournament and replaces that tournament's two worst members, so most of the
//! population persists between events.

use rand::Rng;

use super::common::{Selection, evaluate};
use super::{
    EvolutionOutcome, Evolver, GenerationStats, SharedEvolutionContext, SteadyStateContext,
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
    /// Smallest tournament that keeps the two parents and the two individuals
    /// they replace disjoint.
    ///
    /// Three would still preserve the tournament's best, but the second parent
    /// would also be one of the replaced. Two breaks the guarantee outright:
    /// both parents are replaced by their own children, so the tournament's
    /// best is not carried forward and the strategy stops being self-elitist.
    const MIN_TOURNAMENT_SIZE: usize = 4;

    /// Perform one mating event.
    ///
    /// Draws a single tournament of distinct individuals, breeds its two best
    /// into two children, and overwrites its two worst. Because the tournament's
    /// best is never among the replaced, the population's best individual is
    /// never discarded and no explicit elitism is needed.
    ///
    /// Replacement is unconditional: a child takes its slot even if it scores
    /// worse than the individual it displaces.
    fn mating_event<F, R>(&mut self, fitness: &F, fitnesses: &mut [f64], rng: &mut R)
    where
        F: Fitness,
        R: Rng + ?Sized,
    {
        let tournament = self.shared.selection.tournament_indices(fitnesses, rng);

        let mut first = self.population[tournament[0]].clone();
        let mut second = self.population[tournament[1]].clone();

        // One roll for the pair, then one per child, in a fixed order so a
        // seeded run reproduces exactly.
        if rng.random_bool(self.shared.crossover_rate) {
            first.crossover(&mut second, rng);
        }
        for child in [&mut first, &mut second] {
            if rng.random_bool(self.shared.mutation_rate) {
                child.mutate(rng);
            }
        }

        // Scoring both children in one batch rather than individually halves the
        // FFI hops a Python-backed objective pays per event.
        let children = [first, second];
        let (_, scores) = evaluate(&children, &self.shared.genome_context, fitness);

        let worst = [
            tournament[tournament.len() - 1],
            tournament[tournament.len() - 2],
        ];
        for (slot, (child, score)) in worst.into_iter().zip(children.into_iter().zip(scores)) {
            self.population[slot] = child;
            fitnesses[slot] = score;
        }
    }
}

impl<G: Genome> Evolver<G> for SteadyStateEvolver<G> {
    type TypeContext = SteadyStateContext;

    fn new(
        shared: SharedEvolutionContext<G>,
        type_context: Self::TypeContext,
        population: Vec<G>,
    ) -> Self {
        // Backstop. The config layer should reject these before we get here, but
        // the evolver is constructible directly (tests, embedding), so it checks
        // rather than trusting its caller. Both checks belong at construction:
        // `tournament_indices` would catch the second one too, but not until the
        // first mating event, which is exactly the mid-run failure this avoids.
        match shared.selection {
            Selection::Tournament { tournament_size } => {
                assert!(
                    tournament_size >= Self::MIN_TOURNAMENT_SIZE,
                    "steady-state needs tournament_size >= {}, got {}: two parents \
                     and the two individuals they replace must be distinct",
                    Self::MIN_TOURNAMENT_SIZE,
                    tournament_size,
                );
                assert!(
                    population.len() >= tournament_size,
                    "population of {} is smaller than tournament_size {}: a \
                     tournament of distinct individuals cannot be drawn",
                    population.len(),
                    tournament_size,
                );
            }
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use crate::evolver::common::evaluate;
    use crate::graph::Graph;

    /// A genome whose single value drives both its identity and its fitness, so
    /// a test can say exactly which individual ended up in which slot.
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Val(usize);

    impl Genome for Val {
        type Context = ();

        fn express(&self, _context: &Self::Context) -> Graph {
            Graph::new(self.0 + 1, 1)
        }

        /// Swap, so a crossover that happened is visible in the result.
        fn crossover<R: Rng + ?Sized>(&mut self, other: &mut Self, _rng: &mut R) {
            std::mem::swap(&mut self.0, &mut other.0);
        }

        /// A large, unmistakable jump — no test value is near it.
        fn mutate<R: Rng + ?Sized>(&mut self, _rng: &mut R) {
            self.0 += 100;
        }

        fn print(&self) -> String {
            format!("Val({})", self.0)
        }
    }

    /// Fitness is the node count, which `Val::express` sets from its value, so
    /// a lower value is a fitter individual.
    struct NodeCount;

    impl Fitness for NodeCount {
        fn evaluate(&self, graph: &Graph) -> f64 {
            graph.num_nodes as f64
        }
    }

    const TOURNAMENT_SIZE: usize = 5;

    fn selection() -> Selection {
        Selection::Tournament {
            tournament_size: TOURNAMENT_SIZE,
        }
    }

    /// Population of `size` individuals with distinct values, and their scores.
    fn evolver(
        size: usize,
        crossover_rate: f64,
        mutation_rate: f64,
    ) -> (SteadyStateEvolver<Val>, Vec<f64>) {
        let population: Vec<Val> = (0..size).map(Val).collect();
        let (_, fitnesses) = evaluate(&population, &(), &NodeCount);

        let shared = SharedEvolutionContext {
            genome_context: (),
            crossover_rate,
            mutation_rate,
            selection: selection(),
        };
        let context = SteadyStateContext {
            num_mating_events: 0,
        };

        (
            SteadyStateEvolver::new(shared, context, population),
            fitnesses,
        )
    }

    /// The tournament `mating_event` will draw: it is the first thing to consume
    /// the RNG, so an identically seeded mirror reproduces it exactly.
    fn tournament_for(fitnesses: &[f64], seed: u64) -> Vec<usize> {
        let mut mirror = StdRng::seed_from_u64(seed);
        selection().tournament_indices(fitnesses, &mut mirror)
    }

    #[test]
    fn a_mating_event_replaces_the_tournaments_two_worst_and_nothing_else() {
        let seed = 12;
        let (mut evolver, mut fitnesses) = evolver(10, 0.0, 0.0);
        let before = evolver.population.clone();
        let tournament = tournament_for(&fitnesses, seed);

        let mut rng = StdRng::seed_from_u64(seed);
        evolver.mating_event(&NodeCount, &mut fitnesses, &mut rng);

        let replaced = [
            tournament[TOURNAMENT_SIZE - 1],
            tournament[TOURNAMENT_SIZE - 2],
        ];
        for slot in 0..before.len() {
            if !replaced.contains(&slot) {
                assert_eq!(
                    evolver.population[slot], before[slot],
                    "slot {slot} changed but was not one of the replaced {replaced:?}",
                );
            }
        }

        // With both rates at zero the children are exact copies of the parents.
        assert_eq!(evolver.population[replaced[0]], before[tournament[0]]);
        assert_eq!(evolver.population[replaced[1]], before[tournament[1]]);
    }

    #[test]
    fn the_tournaments_best_is_never_replaced() {
        // Both rates at 1.0 so every child differs from its parent. At lower
        // rates a child can be an exact clone, and writing it back into the
        // best slot would be invisible — the test would pass vacuously.
        let seed = 7;
        let (mut evolver, mut fitnesses) = evolver(12, 1.0, 1.0);
        let before = evolver.population.clone();
        let tournament = tournament_for(&fitnesses, seed);

        let mut rng = StdRng::seed_from_u64(seed);
        evolver.mating_event(&NodeCount, &mut fitnesses, &mut rng);

        assert_eq!(
            evolver.population[tournament[0]], before[tournament[0]],
            "the tournament's best individual was overwritten",
        );
    }

    #[test]
    fn fitnesses_still_describe_the_population_after_an_event() {
        let (mut evolver, mut fitnesses) = evolver(9, 0.7, 0.7);
        let mut rng = StdRng::seed_from_u64(99);

        for _ in 0..25 {
            evolver.mating_event(&NodeCount, &mut fitnesses, &mut rng);

            let (_, recomputed) = evaluate(&evolver.population, &(), &NodeCount);
            assert_eq!(
                fitnesses, recomputed,
                "the fitness array drifted out of step with the population",
            );
        }
    }

    #[test]
    fn the_best_individual_never_gets_worse() {
        let (mut evolver, mut fitnesses) = evolver(10, 0.9, 0.9);
        let mut rng = StdRng::seed_from_u64(4);
        let mut best = fitnesses.iter().copied().reduce(f64::min).unwrap();

        for event in 0..50 {
            evolver.mating_event(&NodeCount, &mut fitnesses, &mut rng);
            let now = fitnesses.iter().copied().reduce(f64::min).unwrap();
            assert!(
                now <= best,
                "best worsened from {best} to {now} at event {event}"
            );
            best = now;
        }
    }

    #[test]
    fn mutation_is_applied_to_the_children() {
        let seed = 3;
        let (mut evolver, mut fitnesses) = evolver(10, 0.0, 1.0);
        let before = evolver.population.clone();
        let tournament = tournament_for(&fitnesses, seed);

        let mut rng = StdRng::seed_from_u64(seed);
        evolver.mating_event(&NodeCount, &mut fitnesses, &mut rng);

        // No crossover, mutation always: each child is its parent plus 100.
        let replaced = [
            tournament[TOURNAMENT_SIZE - 1],
            tournament[TOURNAMENT_SIZE - 2],
        ];
        assert_eq!(
            evolver.population[replaced[0]].0,
            before[tournament[0]].0 + 100
        );
        assert_eq!(
            evolver.population[replaced[1]].0,
            before[tournament[1]].0 + 100
        );
    }

    #[test]
    #[should_panic(expected = "tournament_size >= 4")]
    fn a_tournament_too_small_to_separate_roles_is_rejected_at_construction() {
        let shared = SharedEvolutionContext {
            genome_context: (),
            crossover_rate: 0.5,
            mutation_rate: 0.5,
            selection: Selection::Tournament { tournament_size: 3 },
        };
        let context = SteadyStateContext {
            num_mating_events: 0,
        };
        SteadyStateEvolver::new(shared, context, (0..10).map(Val).collect());
    }

    #[test]
    #[should_panic(expected = "is smaller than tournament_size")]
    fn a_population_smaller_than_the_tournament_is_rejected_at_construction() {
        let shared = SharedEvolutionContext {
            genome_context: (),
            crossover_rate: 0.5,
            mutation_rate: 0.5,
            selection: selection(),
        };
        let context = SteadyStateContext {
            num_mating_events: 0,
        };
        SteadyStateEvolver::new(shared, context, (0..3).map(Val).collect());
    }
}
