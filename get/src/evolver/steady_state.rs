//! Steady-state evolution: each mating event breeds two children inside one
//! drawn scope and replaces two members of that same scope, so most of the
//! population persists between events.

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use super::common::{best_index, breed_pair, express_and_score, generation_stats};
use super::scope::Scope;
use super::{
    EvolutionOutcome, Evolver, GenerationStats, SharedEvolutionContext, SteadyStateContext,
};
use crate::fitness::{Direction, Fitness};
use crate::genomes::Genome;

/// How many parents one mating event breeds from.
///
/// Two, because `Genome::crossover` recombines a pair. Not configurable: a
/// third parent has nowhere to enter the trait.
pub const PARENTS_PER_EVENT: usize = 2;

/// How many individuals one mating event overwrites.
///
/// Two, because recombination yields two children and steady-state keeps both —
/// discarding one would waste half of every crossover.
pub const REPLACED_PER_EVENT: usize = 2;

/// Smallest scope keeping the parents and the replaced disjoint.
///
/// Derived from the two counts above rather than written down: at three the
/// scope's best still survives but the second parent is also replaced, and at
/// two both parents are overwritten by their own children, so the scope's best
/// is not carried forward and the strategy stops being self-elitist.
///
/// `Config::validate_evolution_and_selection` reads this so the config error and
/// the construction backstop cannot drift apart.
pub const MIN_SCOPE_SIZE: usize = PARENTS_PER_EVENT + REPLACED_PER_EVENT;

/// Evolves a population one mate-and-replace event at a time for a fixed number
/// of mating events.
pub struct SteadyStateEvolver<G: Genome> {
    shared: SharedEvolutionContext<G>,
    context: SteadyStateContext,
    population: Vec<G>,
    history: Vec<GenerationStats>,
    /// Reused across events so a 100,000-event run allocates one scope, not one
    /// per event.
    scope_buffer: Vec<usize>,
}

impl<G: Genome> SteadyStateEvolver<G> {
    /// Perform one mating event.
    ///
    /// Draws one scope of distinct individuals, breeds two parents from it into
    /// two children, and overwrites two of its members. The scope's best is
    /// never among the replaced, so the population's best is never discarded and
    /// no explicit elitism is needed.
    ///
    /// Breeding goes through [`breed_pair`](super::common::breed_pair), which
    /// owns the crossover roll and both mutation rolls, so the two strategies
    /// cannot disagree about what they mean or draw them in a different order.
    ///
    /// Replacement is unconditional: a child takes its slot even if it scores
    /// worse than what it displaces.
    ///
    /// # Only the two children are scored, and an unreplaced score is frozen
    ///
    /// Nothing else in the population is re-evaluated, so an individual keeps
    /// the number it was born with until something overwrites it — which over
    /// 100,000 events on a population of 100 can be thousands of events. Under
    /// a stochastic objective that number is one sample, and the error is
    /// self-reinforcing: a lucky score makes an individual likelier to be
    /// selected *and* less likely to be replaced.
    ///
    /// **This is intended for now, and it is a cost decision.** Rescoring the
    /// whole population per event is the `O(population)` work that drawing a
    /// scope exists to avoid, and against a Python-backed objective it is one
    /// FFI hop per individual per event. The wanted change is the narrow one —
    /// rescore the two parents this event selected — and it is deliberately not
    /// in this release, because it alters the trajectory of every seeded run
    /// that already exists. Generational takes the opposite trade and rescores
    /// its elites every generation; the difference is real and neither side of
    /// it is an oversight.
    fn mating_event<F, R>(&mut self, fitness: &F, fitnesses: &mut [f64], rng: &mut R)
    where
        F: Fitness,
        R: Rng + ?Sized,
    {
        self.shared
            .scope
            .draw_into(self.population.len(), &mut self.scope_buffer, rng);

        let parents =
            self.shared
                .selection
                .pick(&self.scope_buffer, fitnesses, PARENTS_PER_EVENT, rng);

        let mut first = self.population[parents[0]].clone();
        let mut second = self.population[parents[1]].clone();

        breed_pair(&mut first, &mut second, &self.shared, rng);

        // Scoring both children in one batch rather than individually halves the
        // FFI hops a Python-backed objective pays per event.
        let children = [first, second];
        let (_, scores) = express_and_score(&children, &self.shared.genome_context, fitness);

        // Same scope as the parents: that is what keeps the scope's best out of
        // the replaced set whatever scheme picked them.
        let worst =
            self.context
                .replacement
                .pick(&self.scope_buffer, fitnesses, REPLACED_PER_EVENT, rng);

        let mut children = children.into_iter();
        let mut scores = scores.into_iter();
        for &slot in &worst {
            self.population[slot] = children.next().expect("exactly two children");
            fitnesses[slot] = scores.next().expect("exactly two scores");
        }
    }

    /// Run every mating event, logging one row per "generation equivalent" —
    /// every `population_size` events. Row 0 is already in place, seeded by
    /// [`Evolver::run`] from the starting population.
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
        // Non-zero: `new` asserts the population is at least MIN_SCOPE_SIZE,
        // and steady-state only ever replaces individuals, never removes them.
        let log_interval = self.population.len();

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
    /// whole run to save a single expression at the end. Generational does the
    /// opposite — it `swap_remove`s the winner from the graphs its final scoring
    /// pass already built — because it scores everyone every generation and so
    /// has them to hand.
    ///
    /// `direction` is stored, not applied: the outcome leaves here in engine
    /// orientation and the boundary converts it once. Spec §5.1.
    fn outcome(&mut self, fitnesses: &[f64], direction: Direction) -> EvolutionOutcome<G> {
        // Non-empty: the population is built at construction and every mating
        // event replaces two members rather than removing any.
        let best = best_index(fitnesses);
        let best_genome = self.population[best].clone();

        EvolutionOutcome {
            best_graph: best_genome.express(&self.shared.genome_context),
            best_fitness_engine: fitnesses[best],
            direction,
            best_genome,
            // mem::take moves history out and leaves an empty Vec in its
            // place, so `self` stays valid without cloning the whole log.
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
        // Backstop. The config layer rejects these first, but the evolver is
        // constructible directly (tests, embedding), and checking here rather
        // than at the first mating event avoids a mid-run failure. A global
        // scope needs neither check: it is every individual, so it is large
        // enough whenever the population is.
        if let Scope::RandomSubset { size } = shared.scope {
            assert!(
                size >= MIN_SCOPE_SIZE,
                "steady-state needs a scope of at least {}, got {}: two parents \
                 and the two individuals they replace must be distinct",
                MIN_SCOPE_SIZE,
                size,
            );
            assert!(
                population.len() >= size,
                "population of {} is smaller than the scope of {}: a scope of \
                 distinct individuals cannot be drawn",
                population.len(),
                size,
            );
        }

        Self {
            shared,
            context: type_context,
            population,
            history: Vec::new(),
            scope_buffer: Vec::new(),
        }
    }

    fn run<F: Fitness>(&mut self, fitness: &F, seed: u64) -> EvolutionOutcome<G> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);

        // The graphs are dropped rather than kept: steady-state re-expresses the
        // winner in `outcome`, so holding a `Graph` per individual for the whole
        // run would buy one expression at the end.
        let (_, mut fitnesses) =
            express_and_score(&self.population, &self.shared.genome_context, fitness);

        // Row 0 is seeded here, beside the scoring it summarizes, for the same
        // reason and in the same place as generational's.
        self.history.clear();
        self.history.push(generation_stats(0, &fitnesses));

        self.evolve(fitness, &mut fitnesses, &mut rng);
        self.outcome(&fitnesses, fitness.direction())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use crate::evolver::common::{Crossover, Selection, express_and_score, rank};
    use crate::evolver::replacement::Replacement;
    use crate::evolver::test_support::{MostNodes, NodeCount, Val, Walk, best_of, mean_of};

    /// Steady-state's scope size. Larger than generational's tournament,
    /// because the two parents and the two individuals they replace must be
    /// distinct.
    const SCOPE_SIZE: usize = 5;

    fn scope() -> Scope {
        Scope::RandomSubset { size: SCOPE_SIZE }
    }

    /// Population of `size` individuals with distinct values, and their scores.
    fn val_evolver(
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
            selection: Selection::Best,
            scope: scope(),
            crossover: Crossover::TwoPoint,
        };
        let context = SteadyStateContext {
            num_mating_events: 0,
            replacement: Replacement::Worst,
        };

        (
            SteadyStateEvolver::new(shared, context, population),
            fitnesses,
        )
    }

    /// A `Walk` evolver whose population starts at `20..20 + size`.
    fn walk_evolver(size: usize, events: usize) -> SteadyStateEvolver<Walk> {
        let shared = SharedEvolutionContext {
            genome_context: (),
            crossover_rate: 0.7,
            mutation_rate: 0.7,
            max_mutations: 1,
            selection: Selection::Best,
            scope: scope(),
            crossover: Crossover::TwoPoint,
        };
        let context = SteadyStateContext {
            num_mating_events: events,
            replacement: Replacement::Worst,
        };
        let population = (0..size).map(|i| Walk(i + 20)).collect();
        SteadyStateEvolver::new(shared, context, population)
    }

    /// The tournament `mating_event` will draw: it is the first thing to consume
    /// the RNG, so an identically seeded mirror reproduces it exactly.
    ///
    /// Steady-state-only, and deliberately: generational draws no tournament whose
    /// membership a test needs to know.
    fn tournament_for(fitnesses: &[f64], seed: u64) -> Vec<usize> {
        let mut mirror = StdRng::seed_from_u64(seed);
        let mut drawn = Vec::new();
        scope().draw_into(fitnesses.len(), &mut drawn, &mut mirror);
        drawn.sort_by(|&a, &b| rank(fitnesses, a, b));
        drawn
    }

    #[test]
    fn a_mating_event_replaces_the_tournaments_two_worst_and_nothing_else() {
        let seed = 12;
        let (mut evolver, mut fitnesses) = val_evolver(10, 0.0, 0.0);
        let before = evolver.population.clone();
        let tournament = tournament_for(&fitnesses, seed);

        let mut rng = StdRng::seed_from_u64(seed);
        evolver.mating_event(&NodeCount, &mut fitnesses, &mut rng);

        let replaced = [tournament[SCOPE_SIZE - 1], tournament[SCOPE_SIZE - 2]];
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
        let (mut evolver, mut fitnesses) = val_evolver(12, 1.0, 1.0);
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
        let (mut evolver, mut fitnesses) = val_evolver(9, 0.7, 0.7);
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
        let (mut evolver, mut fitnesses) = val_evolver(10, 0.9, 0.9);
        let mut rng = StdRng::seed_from_u64(4);
        let mut best = best_of(&fitnesses);

        for event in 0..50 {
            evolver.mating_event(&NodeCount, &mut fitnesses, &mut rng);
            let now = best_of(&fitnesses);
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
        let (mut evolver, mut fitnesses) = val_evolver(10, 0.0, 1.0);
        let before = evolver.population.clone();
        let tournament = tournament_for(&fitnesses, seed);

        let mut rng = StdRng::seed_from_u64(seed);
        evolver.mating_event(&NodeCount, &mut fitnesses, &mut rng);

        // No crossover, mutation always: each child is its parent plus 100.
        let replaced = [tournament[SCOPE_SIZE - 1], tournament[SCOPE_SIZE - 2]];
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
    #[should_panic(expected = "scope of at least 4")]
    fn a_scope_too_small_to_separate_roles_is_rejected_at_construction() {
        let shared = SharedEvolutionContext {
            genome_context: (),
            crossover_rate: 0.5,
            mutation_rate: 0.5,
            max_mutations: 1,
            selection: Selection::Best,
            scope: Scope::RandomSubset { size: 3 },
            crossover: Crossover::TwoPoint,
        };
        let context = SteadyStateContext {
            num_mating_events: 0,
            replacement: Replacement::Worst,
        };
        SteadyStateEvolver::new(shared, context, (0..10).map(Val).collect());
    }

    #[test]
    #[should_panic(expected = "is smaller than the scope")]
    fn a_population_smaller_than_the_scope_is_rejected_at_construction() {
        let shared = SharedEvolutionContext {
            genome_context: (),
            crossover_rate: 0.5,
            mutation_rate: 0.5,
            max_mutations: 1,
            selection: Selection::Best,
            scope: scope(),
            crossover: Crossover::TwoPoint,
        };
        let context = SteadyStateContext {
            num_mating_events: 0,
            replacement: Replacement::Worst,
        };
        SteadyStateEvolver::new(shared, context, (0..3).map(Val).collect());
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
        let best = best_of(&finals);
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

    /// Pins a whole seeded run, slot by slot.
    ///
    /// This is a regression oracle, not a behaviour test: it exists so that a
    /// refactor which reorders what the RNG is asked for — or how many times —
    /// fails loudly instead of quietly producing a different search. Nothing
    /// here asserts the numbers are *good*, only that they are what a run at
    /// this seed has always produced.
    #[test]
    fn a_seeded_run_reproduces_slot_for_slot() {
        let mut evolver = walk_evolver(8, 40);
        let outcome = evolver.run(&NodeCount, 20_260_820);

        let mut values = Vec::with_capacity(evolver.population.len());
        for individual in &evolver.population {
            values.push(individual.0);
        }
        assert_eq!(
            values,
            vec![11, 10, 11, 11, 11, 11, 11, 10],
            "final population"
        );

        let mut log = Vec::with_capacity(outcome.history.len());
        for row in &outcome.history {
            log.push((row.iteration, row.best_fitness, row.mean_fitness));
        }
        assert_eq!(
            log,
            vec![
                (0, 21.0, 24.5),
                (8, 20.0, 21.25),
                (16, 19.0, 19.5),
                (24, 16.0, 18.0),
                (32, 12.0, 13.625),
                (40, 11.0, 11.75)
            ],
            "history"
        );

        assert_eq!(outcome.best_fitness_engine, 11.0, "best fitness");
    }
}
