//! Generational evolution: the whole population is replaced each generation,
//! carrying a configurable number of elites forward unchanged.

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use super::common::{best_index, express_and_score, generation_stats, mutate_child, rank};
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
        let best = best_index(fitnesses);

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

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    use crate::evolver::common::Selection;
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

        /// A large, unmistakable jump: one mutation adds 100, so a child's value
        /// carries both its parent (modulo 100) and its mutation count.
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

    /// Fitness is the node count, which `express` sets from the genome's value,
    /// so a lower value is a fitter individual.
    struct NodeCount;

    impl Fitness for NodeCount {
        fn evaluate(&self, graph: &Graph) -> f64 {
            graph.num_nodes as f64
        }
    }

    /// The same score under `Maximize`, so a test can tell an engine-oriented
    /// outcome from a converted one.
    struct MostNodes;

    impl Fitness for MostNodes {
        fn evaluate(&self, graph: &Graph) -> f64 {
            graph.num_nodes as f64
        }

        fn direction(&self) -> Direction {
            Direction::Maximize
        }
    }

    /// A stochastic objective, in the only way a test can check deterministically:
    /// the same graph scores one higher on every second scoring pass. That is what
    /// makes "elites are rescored" observable — an unchanged genome whose recorded
    /// fitness still moves.
    struct Alternating {
        passes: AtomicUsize,
    }

    impl Fitness for Alternating {
        fn evaluate(&self, graph: &Graph) -> f64 {
            graph.num_nodes as f64
        }

        // Overridden rather than left to the default rayon fan-out: the counter
        // has to advance once per generation, not once per individual.
        fn evaluate_batch(&self, graphs: &[Graph]) -> Vec<f64> {
            let pass = self.passes.fetch_add(1, AtomicOrdering::SeqCst);
            let bump = (pass % 2) as f64;

            let mut scores = Vec::with_capacity(graphs.len());
            for graph in graphs {
                scores.push(graph.num_nodes as f64 + bump);
            }
            scores
        }
    }

    /// A `Val` evolver over the given starting values.
    fn val_evolver(
        values: &[usize],
        elite_count: usize,
        num_generations: usize,
        crossover_rate: f64,
        mutation_rate: f64,
        max_mutations: usize,
    ) -> GenerationalEvolver<Val> {
        let shared = SharedEvolutionContext {
            genome_context: (),
            crossover_rate,
            mutation_rate,
            max_mutations,
            selection: Selection::Tournament { tournament_size: 3 },
        };
        let context = GenerationalContext {
            num_generations,
            elite_count,
        };
        let population = values.iter().map(|&v| Val(v)).collect();
        GenerationalEvolver::new(shared, context, population)
    }

    /// A `Walk` evolver whose population starts at `20..20 + size`.
    fn walk_evolver(size: usize, num_generations: usize) -> GenerationalEvolver<Walk> {
        let shared = SharedEvolutionContext {
            genome_context: (),
            crossover_rate: 0.7,
            mutation_rate: 0.7,
            max_mutations: 1,
            selection: Selection::Tournament { tournament_size: 3 },
        };
        let context = GenerationalContext {
            num_generations,
            elite_count: 1,
        };
        let population = (0..size).map(|i| Walk(i + 20)).collect();
        GenerationalEvolver::new(shared, context, population)
    }

    /// The best (lowest) fitness among a population's scores.
    fn best_of(fitnesses: &[f64]) -> f64 {
        let mut best = fitnesses[0];
        for &f in &fitnesses[1..] {
            if f < best {
                best = f;
            }
        }
        best
    }

    /// The mean fitness across a population's scores.
    fn mean_of(fitnesses: &[f64]) -> f64 {
        let mut sum = 0.0;
        for &f in fitnesses {
            sum += f;
        }
        sum / fitnesses.len() as f64
    }

    #[test]
    #[should_panic(expected = "elite_count 8 must be smaller than the population of 8")]
    fn an_elite_count_that_fills_the_population_is_rejected_at_construction() {
        // Every slot an elite means nothing ever breeds: the run would be a
        // fixed point that reads as a broken fitness function.
        val_evolver(&[0, 1, 2, 3, 4, 5, 6, 7], 8, 10, 0.5, 0.5, 1);
    }

    #[test]
    fn the_elites_carried_forward_are_the_best_of_the_previous_generation() {
        // Deliberately unsorted, so carrying "the first two" rather than "the
        // best two" fails. Mutation is certain and adds 100, so every bred child
        // is unmistakably not an elite.
        let mut evolver = val_evolver(&[5, 3, 9, 1, 7, 2], 2, 1, 0.0, 1.0, 1);
        evolver.run(&NodeCount, 4);

        assert_eq!(evolver.population[0], Val(1), "best elite");
        assert_eq!(evolver.population[1], Val(2), "second-best elite");
        for (slot, individual) in evolver.population.iter().enumerate().skip(2) {
            assert!(
                individual.0 >= 100,
                "slot {slot} holds {individual:?}, which was never bred",
            );
        }
    }

    #[test]
    fn every_slot_is_filled_whether_the_fill_count_is_odd_or_even() {
        // Crossover yields two children, so an odd number of slots to fill means
        // the last pair contributes one child and the other is discarded.
        // elite_count 2 over 9 leaves 7 to fill; over 10 it leaves 8.
        for size in [9, 10] {
            let values: Vec<usize> = (0..size).collect();
            let mut evolver = val_evolver(&values, 2, 3, 0.5, 1.0, 1);
            evolver.run(&NodeCount, 11);

            assert_eq!(
                evolver.population.len(),
                size,
                "a population of {size} did not come back whole",
            );
        }
    }

    #[test]
    fn children_take_one_to_max_mutations_through_the_shared_helper() {
        // Val adds 100 per mutation and the starting values are below 100, so a
        // child's value splits cleanly: hundreds are the mutation count, units
        // are the parent it came from. Certain mutation, no crossover.
        let mut evolver = val_evolver(&[0, 1, 2, 3, 4], 1, 1, 0.0, 1.0, 3);
        evolver.run(&NodeCount, 8);

        for individual in evolver.population.iter().skip(1) {
            let mutations = individual.0 / 100;
            let parent = individual.0 % 100;

            assert!(
                (1..=3).contains(&mutations),
                "{individual:?} took {mutations} mutations, max_mutations was 3",
            );
            assert!(
                parent < 5,
                "{individual:?} came from no starting individual"
            );
        }
    }

    #[test]
    fn elites_survive_as_genomes_while_their_recorded_fitness_moves() {
        // The elite genomes are copied forward untouched, but they are rescored
        // with everyone else — so under a stochastic objective their numbers move
        // while they themselves do not. Spec §6.2.
        let objective = Alternating {
            passes: AtomicUsize::new(0),
        };
        let mut evolver = val_evolver(&[0, 1, 2, 3, 4, 5, 6, 7], 2, 1, 0.0, 1.0, 1);
        let outcome = evolver.run(&objective, 6);

        // Unchanged as genomes.
        assert_eq!(evolver.population[0], Val(0));
        assert_eq!(evolver.population[1], Val(1));

        // Val(0) is the best individual in both generations, and it is the same
        // genome throughout — but the second scoring pass adds one, so an
        // implementation that carried the old score forward would report 1.0.
        assert_eq!(outcome.history[0].best_fitness, 1.0);
        assert_eq!(outcome.history[1].best_fitness, 2.0);
        assert_eq!(outcome.best_genome, Val(0));
        assert_eq!(outcome.best_fitness_engine, 2.0);
    }

    #[test]
    fn the_starting_population_is_generation_zero_and_every_generation_is_logged() {
        for generations in [0, 1, 5, 20] {
            let mut evolver = walk_evolver(10, generations);
            let outcome = evolver.run(&NodeCount, 5);

            assert_eq!(
                outcome.history.len(),
                generations + 1,
                "{generations} generations",
            );
            for (row, i) in outcome.history.iter().zip(0..) {
                assert_eq!(row.iteration, i);
            }
            // Row 0 is the population before any breeding: Walk(20..30), so the
            // best expresses 21 nodes.
            assert_eq!(outcome.history[0].best_fitness, 21.0);
        }
    }

    #[test]
    fn a_run_of_zero_generations_returns_the_starting_population() {
        let mut evolver = walk_evolver(8, 0);
        let before = evolver.population.clone();
        let outcome = evolver.run(&NodeCount, 3);

        assert_eq!(evolver.population, before);
        assert_eq!(outcome.best_fitness_engine, 21.0);
        assert_eq!(outcome.history.len(), 1);
    }

    #[test]
    fn the_same_seed_reproduces_a_run_exactly() {
        let mut a = walk_evolver(12, 40);
        let mut b = walk_evolver(12, 40);

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
        let mut a = walk_evolver(12, 40);
        let mut b = walk_evolver(12, 40);

        let first = a.run(&NodeCount, 1);
        let second = b.run(&NodeCount, 999);

        let mut histories_match = true;
        for (x, y) in first.history.iter().zip(&second.history) {
            if x.mean_fitness != y.mean_fitness {
                histories_match = false;
                break;
            }
        }
        assert!(!histories_match, "two seeds produced identical histories");
    }

    #[test]
    fn a_run_actually_improves_the_population() {
        // The point of the whole thing. Without this, an `advance_generation`
        // that never breeds still satisfies most of the tests above.
        let mut evolver = walk_evolver(10, 60);
        let (_, before) = express_and_score(&evolver.population, &(), &NodeCount);
        let best_before = best_of(&before);
        let mean_before = mean_of(&before);

        let outcome = evolver.run(&NodeCount, 2024);

        let (_, after) = express_and_score(&evolver.population, &(), &NodeCount);
        let mean_after = mean_of(&after);

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
    fn the_logged_best_never_worsens_while_an_elite_is_carried() {
        // elite_count is 1 here and the objective is deterministic, so the best
        // individual is copied forward every generation and cannot be lost.
        let mut evolver = walk_evolver(10, 60);
        let outcome = evolver.run(&NodeCount, 77);

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
        let mut evolver = walk_evolver(10, 30);
        let outcome = evolver.run(&NodeCount, 31);

        // Nothing in the final population may beat the reported best.
        let (_, finals) = express_and_score(&evolver.population, &(), &NodeCount);
        assert_eq!(outcome.best_fitness_engine, best_of(&finals));

        // The graph must be the winner's expression, not a stale one from an
        // earlier generation — `outcome` moves it out of the final scoring pass.
        assert_eq!(outcome.best_graph, outcome.best_genome.express(&()));
        assert_eq!(outcome.best_graph.num_nodes, outcome.best_genome.0 + 1);
    }

    /// The outcome leaves the engine unconverted and carries the direction the
    /// boundary needs. Every other test here minimizes, where orientation is the
    /// identity and this is invisible.
    ///
    /// `Walk(20..=27)` is the starting population, so the best under `Maximize`
    /// is 28 nodes — engine-oriented to -28.0. Spec §5.1.
    #[test]
    fn the_outcome_stays_engine_oriented_and_carries_the_direction() {
        let mut evolver = walk_evolver(8, 0);
        let outcome = evolver.run(&MostNodes, 3);

        assert_eq!(outcome.best_fitness_engine, -28.0);
        assert_eq!(outcome.history[0].best_fitness, -28.0);

        assert_eq!(outcome.direction, Direction::Maximize);
        assert_eq!(outcome.direction.orient(outcome.best_fitness_engine), 28.0);
    }
}
