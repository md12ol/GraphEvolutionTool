//! Steady-state evolution: each mating event breeds two children inside one
//! tournament and replaces that tournament's two worst members, so most of the
//! population persists between events.

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use super::common::{Selection, express_and_score, generation_stats, mutate_child, rank};
use super::{
    EvolutionOutcome, Evolver, GenerationStats, SharedEvolutionContext, SteadyStateContext,
};
use crate::fitness::{Direction, Fitness};
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

        // One crossover roll for the pair, then the mutation rolls per child, in
        // a fixed order so a seeded run reproduces exactly. Mutation goes through
        // the shared helper so this strategy and generational cannot disagree
        // about what `mutation_rate` and `max_mutations` mean.
        if rng.random_bool(self.shared.crossover_rate) {
            first.crossover(&mut second, rng);
        }
        for child in [&mut first, &mut second] {
            mutate_child(
                child,
                self.shared.mutation_rate,
                self.shared.max_mutations,
                rng,
            );
        }

        // Scoring both children in one batch rather than individually halves the
        // FFI hops a Python-backed objective pays per event.
        let children = [first, second];
        let (_, scores) = express_and_score(&children, &self.shared.genome_context, fitness);

        let worst = [
            tournament[tournament.len() - 1],
            tournament[tournament.len() - 2],
        ];
        for (slot, (child, score)) in worst.into_iter().zip(children.into_iter().zip(scores)) {
            self.population[slot] = child;
            fitnesses[slot] = score;
        }
    }

    /// Run every mating event, recording the starting population as iteration 0
    /// and then one row per "generation equivalent" — every `population_size`
    /// events.
    ///
    /// The interval keeps a steady-state log comparable to a generational one
    /// and stops a 100,000-event run from producing a 100,000-row history. The
    /// row at iteration 0 is what makes a log self-contained: without it there
    /// is no way to see where a run started, and a run shorter than one interval
    /// would produce nothing at all.
    ///
    /// So `history.len()` is `num_mating_events / population_size + 1`.
    fn evolve<F, R>(&mut self, fitness: &F, fitnesses: &mut [f64], rng: &mut R)
    where
        F: Fitness,
        R: Rng + ?Sized,
    {
        // Non-zero: `new` asserts the population is at least MIN_TOURNAMENT_SIZE,
        // and steady-state only ever replaces individuals, never removes them.
        let log_interval = self.population.len();

        self.history.clear();
        self.history.push(generation_stats(0, fitnesses));

        for event in 1..=self.context.num_mating_events {
            self.mating_event(fitness, fitnesses, rng);

            if event % log_interval == 0 {
                self.history.push(generation_stats(event, fitnesses));
            }
        }
    }

    /// Package the best individual and the accumulated history into an outcome.
    ///
    /// Expresses the winner once here rather than tracking graphs through every
    /// event, which would mean keeping a `Graph` per individual alive for the
    /// whole run to save a single expression at the end.
    ///
    /// `direction` is stored, not applied: the outcome leaves here in engine
    /// orientation and the boundary converts it once. Spec §5.1.
    fn outcome(&mut self, fitnesses: &[f64], direction: Direction) -> EvolutionOutcome<G> {
        let best = (0..fitnesses.len())
            .min_by(|&a, &b| rank(fitnesses, a, b))
            .expect("population is non-empty, checked at construction");
        let best_genome = self.population[best].clone();

        EvolutionOutcome {
            best_graph: best_genome.express(&self.shared.genome_context),
            best_fitness_engine: fitnesses[best],
            direction,
            best_genome,
            history: std::mem::take(&mut self.history),
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
        // ChaCha8 rather than StdRng: StdRng's algorithm is allowed to change
        // between `rand` releases, which would silently break the reproducibility
        // this `seed` argument exists to provide.
        let mut rng = ChaCha8Rng::seed_from_u64(seed);

        let (_, mut fitnesses) =
            express_and_score(&self.population, &self.shared.genome_context, fitness);
        self.evolve(fitness, &mut fitnesses, &mut rng);
        self.outcome(&fitnesses, fitness.direction())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use crate::evolver::common::express_and_score;
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

    /// Like `Val`, but mutation drifts up or down using the RNG, so evolution
    /// can actually improve a population and a run test is not vacuous.
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Walk(usize);

    impl Genome for Walk {
        type Context = ();

        fn express(&self, _context: &Self::Context) -> Graph {
            Graph::new(self.0 + 1, 1)
        }

        fn crossover<R: Rng + ?Sized>(&mut self, other: &mut Self, _rng: &mut R) {
            std::mem::swap(&mut self.0, &mut other.0);
        }

        fn mutate<R: Rng + ?Sized>(&mut self, rng: &mut R) {
            if rng.random_bool(0.5) {
                self.0 = self.0.saturating_sub(1);
            } else {
                self.0 += 1;
            }
        }

        fn print(&self) -> String {
            format!("Walk({})", self.0)
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

    /// The same score under `Maximize`, so a test can tell an engine-oriented
    /// outcome from a converted one — under `NodeCount` the two are identical,
    /// because orienting a minimizing objective is the identity.
    struct MostNodes;

    impl Fitness for MostNodes {
        fn evaluate(&self, graph: &Graph) -> f64 {
            graph.num_nodes as f64
        }

        fn direction(&self) -> Direction {
            Direction::Maximize
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
        let (_, fitnesses) = express_and_score(&population, &(), &NodeCount);

        let shared = SharedEvolutionContext {
            genome_context: (),
            crossover_rate,
            mutation_rate,
            max_mutations: 1,
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
        for (slot, original) in before.iter().enumerate() {
            if !replaced.contains(&slot) {
                assert_eq!(
                    &evolver.population[slot], original,
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

            let (_, recomputed) = express_and_score(&evolver.population, &(), &NodeCount);
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
            max_mutations: 1,
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
            max_mutations: 1,
            selection: selection(),
        };
        let context = SteadyStateContext {
            num_mating_events: 0,
        };
        SteadyStateEvolver::new(shared, context, (0..3).map(Val).collect());
    }

    fn walk_evolver(size: usize, events: usize) -> SteadyStateEvolver<Walk> {
        let shared = SharedEvolutionContext {
            genome_context: (),
            crossover_rate: 0.7,
            mutation_rate: 0.7,
            max_mutations: 1,
            selection: selection(),
        };
        let context = SteadyStateContext {
            num_mating_events: events,
        };
        let population = (0..size).map(|i| Walk(i + 20)).collect();
        SteadyStateEvolver::new(shared, context, population)
    }

    #[test]
    fn the_same_seed_reproduces_a_run_exactly() {
        let mut a = walk_evolver(12, 200);
        let mut b = walk_evolver(12, 200);

        let first = a.run(&NodeCount, 2026);
        let second = b.run(&NodeCount, 2026);

        assert_eq!(first.best_genome, second.best_genome);
        assert_eq!(first.best_fitness_engine, second.best_fitness_engine);
        assert_eq!(first.best_graph, second.best_graph);
        assert_eq!(first.history.len(), second.history.len());
        for (x, y) in first.history.iter().zip(&second.history) {
            assert_eq!(x.iteration, y.iteration);
            assert_eq!(x.best_fitness, y.best_fitness);
            assert_eq!(x.mean_fitness, y.mean_fitness);
            assert_eq!(x.std_dev, y.std_dev);
        }
    }

    #[test]
    fn different_seeds_produce_different_runs() {
        let mut a = walk_evolver(12, 200);
        let mut b = walk_evolver(12, 200);

        let first = a.run(&NodeCount, 1);
        let second = b.run(&NodeCount, 999);

        let histories_match = first
            .history
            .iter()
            .zip(&second.history)
            .all(|(x, y)| x.mean_fitness == y.mean_fitness);
        assert!(!histories_match, "two seeds produced identical histories");
    }

    #[test]
    fn the_starting_population_is_row_zero_then_one_per_population_size_events() {
        let population_size = 10;
        for events in [0, 9, 10, 25, 60] {
            let mut evolver = walk_evolver(population_size, events);
            let outcome = evolver.run(&NodeCount, 5);

            assert_eq!(
                outcome.history.len(),
                events / population_size + 1,
                "{events} events over a population of {population_size}",
            );
            // Rows are stamped with the event number they summarize, starting
            // at 0 for the population before any breeding.
            for (row, i) in outcome.history.iter().zip(0..) {
                assert_eq!(row.iteration, i * population_size);
            }
        }
    }

    #[test]
    fn the_logged_best_never_worsens_across_a_run() {
        let mut evolver = walk_evolver(10, 500);
        let outcome = evolver.run(&NodeCount, 77);

        assert!(outcome.history.len() > 1, "need rows to compare");
        for pair in outcome.history.windows(2) {
            assert!(
                pair[1].best_fitness <= pair[0].best_fitness,
                "best worsened from {} to {}",
                pair[0].best_fitness,
                pair[1].best_fitness,
            );
        }
    }

    #[test]
    fn the_outcome_reports_the_actual_best_and_its_graph() {
        let mut evolver = walk_evolver(10, 300);
        let outcome = evolver.run(&NodeCount, 31);

        // Nothing in the final population may beat the reported best.
        let (_, finals) = express_and_score(&evolver.population, &(), &NodeCount);
        let best = finals.iter().copied().reduce(f64::min).unwrap();
        assert_eq!(outcome.best_fitness_engine, best);

        // The graph must be the winner's expression, not a stale or default one.
        assert_eq!(outcome.best_graph, outcome.best_genome.express(&()));
        assert_eq!(outcome.best_graph.num_nodes, outcome.best_genome.0 + 1);
    }

    #[test]
    fn a_run_actually_improves_the_population() {
        // The point of the whole thing. Without this, an `evolve` that never
        // breeds still satisfies almost every other test here.
        let mut evolver = walk_evolver(10, 500);
        let (_, before) = express_and_score(&evolver.population, &(), &NodeCount);
        let best_before = before.iter().copied().reduce(f64::min).unwrap();
        let mean_before = before.iter().sum::<f64>() / before.len() as f64;

        let outcome = evolver.run(&NodeCount, 2024);

        let (_, after) = express_and_score(&evolver.population, &(), &NodeCount);
        let mean_after = after.iter().sum::<f64>() / after.len() as f64;

        assert!(
            outcome.best_fitness_engine < best_before,
            "best did not improve: {best_before} -> {}",
            outcome.best_fitness_engine,
        );
        assert!(
            mean_after < mean_before,
            "population mean did not improve: {mean_before} -> {mean_after}",
        );
    }

    #[test]
    fn a_run_of_zero_events_returns_the_starting_population() {
        let mut evolver = walk_evolver(8, 0);
        let before = evolver.population.clone();
        let outcome = evolver.run(&NodeCount, 3);

        assert_eq!(evolver.population, before);
        // Walk(20) is the fittest starting individual; its graph has 21 nodes.
        assert_eq!(outcome.best_fitness_engine, 21.0);

        // Even with no events the starting state is logged, so a log is never
        // empty and always says where the run began.
        assert_eq!(outcome.history.len(), 1);
        assert_eq!(outcome.history[0].iteration, 0);
        assert_eq!(outcome.history[0].best_fitness, 21.0);
    }

    /// The outcome leaves the engine unconverted, and carries the direction the
    /// boundary needs to convert it. Every other test here uses a minimizing
    /// objective, where orientation is the identity and this is invisible.
    ///
    /// `Walk(20..=27)` is the starting population, so the best under `Maximize`
    /// is 28 nodes — engine-oriented to -28.0. Reinstating a conversion in
    /// `outcome` makes this 28.0 and fails. Spec §5.1.
    #[test]
    fn the_outcome_stays_engine_oriented_and_carries_the_direction() {
        let mut evolver = walk_evolver(8, 0);
        let outcome = evolver.run(&MostNodes, 3);

        assert_eq!(outcome.best_fitness_engine, -28.0);
        assert_eq!(outcome.history[0].best_fitness, -28.0);

        // The direction is what makes the value recoverable at the boundary.
        assert_eq!(outcome.direction, Direction::Maximize);
        assert_eq!(outcome.direction.orient(outcome.best_fitness_engine), 28.0);
    }
}
