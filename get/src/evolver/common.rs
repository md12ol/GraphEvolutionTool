//! GA mechanics shared by every evolution strategy.
//!
//! These helpers keep selection, population setup, evaluation and logging in
//! one place, so no strategy re-implements them.

use std::cmp::Ordering;

use rand::Rng;
use rayon::prelude::*;

use super::{GenerationStats, SharedEvolutionContext};
use crate::fitness::Fitness;
use crate::genomes::Genome;
use crate::graph::Graph;

/// Recombination operator, chosen once per run, for every representation.
///
/// A variant here is therefore offered to all of them, and nothing in the type
/// system pairs this enum with the selected genome — so one only some genomes
/// can honour must be refused in `Config::validate_crossover`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crossover {
    /// Swap one contiguous band between the parents. What the band is made of
    /// is the representation's own business — genes for edge-edit, states for
    /// SDA.
    TwoPoint,
    // ADD A CROSSOVER STEP 1 — a variant here, plus any parameters it reads
    // from the file:
    //
    //     MyCrossover { some_param: f64 },
    //
    // The chain does not currently reach a working operator. `recombine` can
    // only call trait methods, so a second variant also needs a method on
    // `Genome` and an implementation in every representation — neither of which
    // is a step, and neither compiles from the six that are.
}

impl Crossover {
    /// Recombine one pair in place, both children kept.
    pub fn recombine<G, R>(&self, first: &mut G, second: &mut G, rng: &mut R)
    where
        G: Genome,
        R: Rng + ?Sized,
    {
        match self {
            // The arm is the trait call itself: two-point is what
            // `Genome::crossover` already means for every representation. A
            // second variant cannot reuse it, so it is the point at which
            // `Genome` grows a second method and every representation
            // implements it.
            Crossover::TwoPoint => first.crossover(second, rng),
            // ADD A CROSSOVER STEP 2 — the arm performing your variant:
            //
            //     Crossover::MyCrossover { some_param } => first.my_crossover(second, *some_param, rng),
        }
    }
}

/// Parent-selection strategy: who breeds, within the scope an event drew.
///
/// None of this is enforced, and breaking it fails silently:
///
/// - **Pick only from `scope`** — reaching past it breaks the guarantee that a
///   strategy's best individual is never among those replaced.
/// - **Sample with replacement** — rejecting a repeat draw quietly lowers
///   selection pressure.
/// - **Compare through `rank`**, never a `Direction` of your own.
/// - **All randomness comes from `rng`**, never a thread RNG or the clock, or
///   two runs at one seed stop agreeing.
pub enum Selection {
    /// The fittest members of the scope, best first. Consumes no randomness.
    Best,
    /// Sample `tournament_size` members of the scope per pick, keep the best.
    Tournament { tournament_size: usize },
    // ADD A SELECTION STEP 1 — a variant here, plus any parameters the scheme
    // reads out of the file:
    //
    //     Roulette { pressure: f64 },
}

/// Order two individuals, better first, ties broken by lower index.
///
/// The index tie-break makes a tournament's outcome depend on which indices
/// were drawn, not the order the RNG produced them.
pub(super) fn rank(fitnesses: &[f64], a: usize, b: usize) -> Ordering {
    fitnesses[a].total_cmp(&fitnesses[b]).then(a.cmp(&b))
}

/// Index of the best individual in `fitnesses`, ties broken by lower index.
///
/// Panics if `fitnesses` is empty: both callers construct a non-empty
/// population and never shrink it, so an empty slice is a bug, not an input.
pub(super) fn best_index(fitnesses: &[f64]) -> usize {
    assert!(
        !fitnesses.is_empty(),
        "cannot pick a best of no individuals"
    );

    let mut best = 0;
    for candidate in 1..fitnesses.len() {
        if rank(fitnesses, candidate, best) == Ordering::Less {
            best = candidate;
        }
    }
    best
}

impl Selection {
    /// Choose `count` parents from `scope`, returning indices into the
    /// population.
    ///
    pub(super) fn pick<R>(
        &self,
        scope: &[usize],
        fitnesses: &[f64],
        count: usize,
        rng: &mut R,
    ) -> Vec<usize>
    where
        R: Rng + ?Sized,
    {
        assert!(!scope.is_empty(), "cannot select from an empty scope");
        for &index in scope {
            assert!(
                index < fitnesses.len(),
                "scope names individual {} but only {} were scored",
                index,
                fitnesses.len(),
            );
        }

        match self {
            Selection::Best => {
                assert!(
                    count <= scope.len(),
                    "cannot take {} best of a scope of {}",
                    count,
                    scope.len(),
                );

                let mut ranked = scope.to_vec();
                ranked.sort_by(|&a, &b| rank(fitnesses, a, b));
                ranked.truncate(count);
                ranked
            }
            Selection::Tournament { tournament_size } => {
                assert!(*tournament_size > 0, "tournament_size must be at least 1");

                let mut parents = Vec::with_capacity(count);
                for _ in 0..count {
                    parents.push(Self::tournament_winner(
                        scope,
                        fitnesses,
                        *tournament_size,
                        rng,
                    ));
                }
                parents
            } // ADD A SELECTION STEP 2 — the arm choosing parents your way:
              //
              //     Selection::Roulette { pressure } => {
              //         let mut parents = Vec::with_capacity(count);
              //         for _ in 0..count {
              //             parents.push(spin(scope, fitnesses, *pressure, rng));
              //         }
              //         parents
              //     }
        }
    }

    /// Draw `tournament_size` members of `scope` **with** replacement, best wins.
    fn tournament_winner<R>(
        scope: &[usize],
        fitnesses: &[f64],
        tournament_size: usize,
        rng: &mut R,
    ) -> usize
    where
        R: Rng + ?Sized,
    {
        let mut winner = scope[rng.random_range(0..scope.len())];
        for _ in 1..tournament_size {
            let candidate = scope[rng.random_range(0..scope.len())];
            if rank(fitnesses, candidate, winner) == Ordering::Less {
                winner = candidate;
            }
        }
        winner
    }
}

/// Roll `mutation_rate` for whether this child mutates, then `max_mutations`
/// for how many times, drawn from `1..=max_mutations`. Both rolls are the
/// engine's, never the genome's.
///
/// The count roll is unconditional once the rate roll passes, and
/// `random_range(1..=1)` consumes RNG state even though it has one outcome:
/// skipping it at 1 would make the RNG stream depend on a config value.
///
/// Panics if `max_mutations` is zero: `1..=0` is an empty range. A backstop —
/// the config layer rejects it first, but the evolvers are constructible
/// directly.
pub fn mutate_child<G, R>(
    child: &mut G,
    context: &G::Context,
    mutation_rate: f64,
    max_mutations: usize,
    rng: &mut R,
) where
    G: Genome,
    R: Rng + ?Sized,
{
    assert!(
        max_mutations >= 1,
        "max_mutations must be at least 1, got 0: a child that mutates takes \
         between 1 and max_mutations mutations, which is an empty range at 0",
    );

    if !rng.random_bool(mutation_rate) {
        return;
    }

    let count = rng.random_range(1..=max_mutations);
    for _ in 0..count {
        child.mutate(context, rng);
    }
}

/// Recombine a selected pair and mutate both children, in the fixed order every
/// strategy uses.
///
/// One crossover roll for the pair, then one [`mutate_child`] call per child.
/// The order is the contract: a seeded run reproduces only if every strategy
/// draws from the RNG in the same sequence, which is why both strategies breed
/// through here.
///
/// `first` and `second` are the parents, mutated in place into the children,
/// and neither is scored here.
pub fn breed_pair<G, R>(
    first: &mut G,
    second: &mut G,
    shared: &SharedEvolutionContext<G>,
    rng: &mut R,
) where
    G: Genome,
    R: Rng + ?Sized,
{
    if rng.random_bool(shared.crossover_rate) {
        shared.crossover.recombine(first, second, rng);
    }
    mutate_child(
        first,
        &shared.genome_context,
        shared.mutation_rate,
        shared.max_mutations,
        rng,
    );
    mutate_child(
        second,
        &shared.genome_context,
        shared.mutation_rate,
        shared.max_mutations,
        rng,
    );
}

/// Express and score the whole batch through [`Fitness::evaluate_batch`], so a
/// Python-backed objective crosses the FFI boundary once. Index `i` of both
/// returned vectors refers to `batch[i]`.
///
/// Fitnesses come back **lower-is-better**, and this is the only place that
/// conversion happens.
///
/// **Nothing else calls [`Fitness::evaluate`] or [`Fitness::evaluate_batch`].**
/// A direct call stays as-measured, so under
/// [`crate::fitness::Direction::Maximize`] every later comparison runs
/// backwards, and it skips the `NaN` gate, where an unchecked `-NaN` sorts
/// below `-inf` and wins every tournament it enters. Both leave a run that
/// looks merely unconverged.
///
/// Panics if the objective returns `NaN` — see
/// [`crate::fitness::Direction::orient`].
pub fn express_and_score<G, F>(
    batch: &[G],
    context: &G::Context,
    fitness: &F,
) -> (Vec<Graph>, Vec<f64>)
where
    G: Genome,
    F: Fitness,
{
    let graphs: Vec<Graph> = batch.par_iter().map(|g| g.express(context)).collect();

    let direction = fitness.direction();
    let fitnesses = fitness
        .evaluate_batch(&graphs)
        .into_iter()
        .map(|score| direction.orient(score))
        .collect();

    (graphs, fitnesses)
}

/// Summarize a scored population into one evolution-log row.
///
/// Every field is **lower-is-better**: that is what the caller passes in, and
/// nothing here converts.
///
/// `std_dev` divides by `n` — these are all the individuals there are, not a
/// sample. `ci_95` divides by `n - 1` on purpose: the uncertainty in
/// `mean_fitness` as a statistic is a sample question even though the deviation
/// beside it is not.
pub(super) fn generation_stats(iteration: usize, fitnesses: &[f64]) -> GenerationStats {
    assert!(
        !fitnesses.is_empty(),
        "cannot summarize an empty population",
    );

    let n = fitnesses.len() as f64;
    let mut best = fitnesses[0];
    for &f in &fitnesses[1..] {
        if f < best {
            best = f;
        }
    }
    let mut sum = 0.0;
    for &f in fitnesses {
        sum += f;
    }
    let mean = sum / n;

    let mut sum_sq_dev = 0.0;
    for &f in fitnesses {
        sum_sq_dev += (f - mean).powi(2);
    }
    let variance = sum_sq_dev / n;

    let ci_95 = if n > 1.0 {
        let sample_variance = sum_sq_dev / (n - 1.0);
        1.96 * sample_variance.sqrt() / n.sqrt()
    } else {
        0.0
    };

    GenerationStats {
        iteration,
        best_fitness: best,
        mean_fitness: mean,
        std_dev: variance.sqrt(),
        ci_95,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use crate::fitness::Direction;

    /// Scores a graph by its node count, which `IndexGenome` sets from its own
    /// index — so a fitness identifies the genome it came from.
    struct NodeCount(Direction);

    impl Fitness for NodeCount {
        fn evaluate(&self, graph: &Graph) -> f64 {
            graph.num_nodes as f64
        }

        fn direction(&self) -> Direction {
            self.0
        }
    }

    /// A genome that is just its index, so a winner reports its own slot.
    ///
    /// `mutations` counts how many mutations have been applied, and is a
    /// **separate field from `index` on purpose**. The two used to be one: the
    /// index was incremented by `mutate`, so the `mutate_child` tests could read
    /// the count directly — but then an individual that went through a mutation
    /// path stopped identifying the slot it came from, and a selection test that
    /// mutated would fail as though selection were broken. Kept apart, each test
    /// reads the field it means.
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct IndexGenome {
        index: usize,
        mutations: usize,
    }

    impl IndexGenome {
        fn new(index: usize) -> Self {
            Self {
                index,
                mutations: 0,
            }
        }
    }

    impl Genome for IndexGenome {
        type Context = ();

        // Node count encodes the index, so an expressed graph is traceable
        // back to the genome it came from. Mutations deliberately do not move
        // it — that is what keeps the slot identifiable.
        fn express(&self, _context: &Self::Context) -> Graph {
            Graph::new(self.index + 1, 1)
        }

        fn crossover<R: Rng + ?Sized>(&mut self, _other: &mut Self, _rng: &mut R) {}

        // One mutation is one increment of the counter, which is what makes the
        // count observable — see the type's doc comment.
        fn mutate<R: Rng + ?Sized>(&mut self, _context: &Self::Context, _rng: &mut R) {
            self.mutations += 1;
        }

        fn print(&self) -> String {
            format!("IndexGenome({}, {} mutations)", self.index, self.mutations)
        }
    }

    fn population(size: usize) -> Vec<IndexGenome> {
        (0..size).map(IndexGenome::new).collect()
    }

    /// The genomes a scheme picks over the whole population.
    ///
    /// Every scheme now picks within a scope and returns indices; these tests
    /// predate that and read better in terms of individuals, so this puts the
    /// global scope in and takes the genomes back out.
    fn selected_from_all(
        selection: &Selection,
        population: &[IndexGenome],
        fitnesses: &[f64],
        count: usize,
        rng: &mut impl Rng,
    ) -> Vec<IndexGenome> {
        let mut scope = Vec::with_capacity(population.len());
        for index in 0..population.len() {
            scope.push(index);
        }

        let mut chosen = Vec::with_capacity(count);
        for index in selection.pick(&scope, fitnesses, count, rng) {
            chosen.push(population[index].clone());
        }
        chosen
    }

    /// Two-point recombination is *exactly* what `Genome::crossover` already
    /// did, and this pins both halves of that: the same children, and the same
    /// number of RNG draws consumed getting there.
    ///
    /// The draw count is the half that would otherwise rot silently. A run is
    /// reproducible only if every strategy pulls from the seeded stream in the
    /// same sequence, so an operator layer that consumed one extra value would
    /// still produce valid children while changing every seeded result in the
    /// project — which is why the check is "the next draw agrees", not just
    /// "the genomes agree".
    #[test]
    fn two_point_recombine_is_exactly_the_genomes_own_crossover() {
        use std::sync::Arc;

        use crate::genomes::{EdgeEditGenome, EdgeEditOperationWeights, EdgeEditOperators};

        let operators = EdgeEditOperators::new(EdgeEditOperationWeights::default())
            .expect("an all-default operation mix is valid");

        // Two pairs built from one seed, so both paths start from identical
        // parents rather than merely similar ones.
        let mut build = StdRng::seed_from_u64(7);
        let first = EdgeEditGenome::random_with_operators(32, Arc::clone(&operators), &mut build);
        let second = EdgeEditGenome::random_with_operators(32, Arc::clone(&operators), &mut build);

        let (mut direct_a, mut direct_b) = (first.clone(), second.clone());
        let mut direct_rng = StdRng::seed_from_u64(99);
        direct_a.crossover(&mut direct_b, &mut direct_rng);

        let (mut routed_a, mut routed_b) = (first, second);
        let mut routed_rng = StdRng::seed_from_u64(99);
        Crossover::TwoPoint.recombine(&mut routed_a, &mut routed_b, &mut routed_rng);

        assert_eq!(
            direct_a.genes, routed_a.genes,
            "routing through Crossover changed the first child",
        );
        assert_eq!(
            direct_b.genes, routed_b.genes,
            "routing through Crossover changed the second child",
        );
        assert_eq!(
            direct_rng.random::<u64>(),
            routed_rng.random::<u64>(),
            "the two paths consumed different amounts of the RNG stream, so \
             every seeded run in the project would shift",
        );
    }

    /// What a tournament should pick, written independently of `rank`.
    fn expected_winner(fitnesses: &[f64], entrants: &[usize]) -> usize {
        let mut winner = entrants[0];
        for &entrant in &entrants[1..] {
            let better = fitnesses[entrant] < fitnesses[winner];
            let tied = fitnesses[entrant] == fitnesses[winner];
            if better || (tied && entrant < winner) {
                winner = entrant;
            }
        }
        winner
    }

    #[test]
    fn generation_stats_computes_best_mean_and_population_deviation() {
        // mean 5, deviations -3,-1,1,3 -> variance (9+1+1+9)/4 = 5
        let stats = generation_stats(7, &[2.0, 4.0, 6.0, 8.0]);

        assert_eq!(stats.iteration, 7);
        assert_eq!(stats.best_fitness, 2.0);
        assert_eq!(stats.mean_fitness, 5.0);
        assert!((stats.std_dev - 5.0_f64.sqrt()).abs() < 1e-12);

        // Same sum of squared deviations (20), but ci_95 divides by n - 1 = 3,
        // not n = 4: sample variance 20/3, half-width 1.96 * sqrt(20/3) / sqrt(4).
        let expected_ci_95 = 1.96 * (20.0_f64 / 3.0).sqrt() / 4.0_f64.sqrt();
        assert!((stats.ci_95 - expected_ci_95).abs() < 1e-12);
        // The two denominators must actually differ, or this test can't tell
        // ci_95 apart from std_dev by coincidence.
        assert!((stats.ci_95 - stats.std_dev).abs() > 1e-6);
    }

    #[test]
    fn a_single_individual_has_zero_deviation() {
        let stats = generation_stats(0, &[3.5]);
        assert_eq!(stats.best_fitness, 3.5);
        assert_eq!(stats.mean_fitness, 3.5);
        assert_eq!(stats.std_dev, 0.0);
        // n - 1 = 0 would divide by zero if computed the same way as std_dev;
        // this is the guard that it doesn't produce NaN instead of 0.0.
        assert_eq!(stats.ci_95, 0.0);
    }

    /// Guards the rule that nothing inside the engine converts. These are the
    /// oriented fitnesses a `Maximize` objective produces, and every field must
    /// come back still oriented — reinstating a conversion here flips the signs
    /// of the first two assertions and fails the test.
    #[test]
    fn generation_stats_stays_in_engine_orientation_under_maximize() {
        // What `express_and_score` hands over for natural scores 2, 4, 6, 8
        // under Maximize: negated, so the largest score is now the smallest.
        let oriented = [-2.0, -4.0, -6.0, -8.0];
        let stats = generation_stats(1, &oriented);

        // Engine orientation, not the objective's units: the natural best is
        // 8.0 and it stays -8.0 here. The boundary is what turns it back.
        assert_eq!(stats.best_fitness, -8.0);
        assert_eq!(stats.mean_fitness, -5.0);

        // Deviation is unchanged by negation, so it matches the minimizing case
        // — and now that is just true rather than a carve-out to defend. Same
        // for ci_95: it's a spread, not a location.
        let minimized = generation_stats(1, &[2.0, 4.0, 6.0, 8.0]);
        assert_eq!(stats.std_dev, minimized.std_dev);
        assert_eq!(stats.ci_95, minimized.ci_95);
        assert!(stats.std_dev > 0.0, "a negated deviation would be negative");
    }

    #[test]
    #[should_panic(expected = "cannot summarize an empty population")]
    fn generation_stats_rejects_an_empty_population() {
        generation_stats(0, &[]);
    }

    #[test]
    fn express_and_score_expresses_every_genome_and_keeps_the_order() {
        let population = population(5);
        let (graphs, fitnesses) =
            express_and_score(&population, &(), &NodeCount(Direction::Minimize));

        assert_eq!(graphs.len(), 5);
        assert_eq!(fitnesses.len(), 5);
        for (i, graph) in graphs.iter().enumerate() {
            // Would fail if graphs came back defaulted or reordered by rayon.
            assert_eq!(graph.num_nodes, i + 1, "graph {i} is not genome {i}'s");
            assert_eq!(fitnesses[i], (i + 1) as f64);
        }
    }

    #[test]
    fn express_and_score_orients_scores_so_lower_is_always_better() {
        let population = population(4);

        let (_, minimized) = express_and_score(&population, &(), &NodeCount(Direction::Minimize));
        let (_, maximized) = express_and_score(&population, &(), &NodeCount(Direction::Maximize));

        assert_eq!(minimized, vec![1.0, 2.0, 3.0, 4.0]);
        assert_eq!(maximized, vec![-1.0, -2.0, -3.0, -4.0]);

        // Under Maximize the biggest graph is best, so it must sort lowest.
        let best = maximized.iter().cloned().fold(f64::INFINITY, f64::min);
        assert_eq!(best, -4.0);
    }

    /// Scores by node count, but counts the batches it was handed rather than
    /// the graphs — the one number that distinguishes a forwarded batched call
    /// from the trait's per-graph default.
    struct CountingBatches {
        /// Shared, because the objective is moved into the box and the test
        /// still has to read the count back out.
        batches: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }

    impl Fitness for CountingBatches {
        fn evaluate(&self, graph: &Graph) -> f64 {
            graph.num_nodes as f64
        }

        fn direction(&self) -> Direction {
            Direction::Maximize
        }

        fn evaluate_batch(&self, graphs: &[Graph]) -> Vec<f64> {
            self.batches
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

            let mut scores = Vec::with_capacity(graphs.len());
            for graph in graphs {
                scores.push(graph.num_nodes as f64);
            }
            scores
        }
    }

    #[test]
    fn a_boxed_objective_keeps_its_batched_override_through_express_and_score() {
        // The engine scores through `Box<dyn Fitness>` once the config layer
        // erases the objective, so the box has to reach the objective's own
        // `evaluate_batch` — not the trait default, which would call Python
        // once per individual from inside a rayon closure. Tested here rather
        // than in `fitness.rs` because this is the path the engine actually
        // takes: one call, through the scoring gate.
        let batches = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let boxed: Box<dyn Fitness> = Box::new(CountingBatches {
            batches: std::sync::Arc::clone(&batches),
        });

        let (_, fitnesses) = express_and_score(&population(5), &(), &boxed);

        // Maximize, so the oriented values are negated node counts 1..=5. Both
        // halves matter: the values prove `direction` was forwarded, the count
        // proves `evaluate_batch` was.
        assert_eq!(fitnesses, vec![-1.0, -2.0, -3.0, -4.0, -5.0]);
        assert_eq!(
            batches.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "five graphs should have been scored in one batched call",
        );
    }

    #[test]
    fn express_and_score_of_an_empty_batch_yields_empty_vectors() {
        let (graphs, fitnesses) =
            express_and_score::<IndexGenome, _>(&[], &(), &NodeCount(Direction::Minimize));
        assert!(graphs.is_empty());
        assert!(fitnesses.is_empty());
    }

    #[test]
    #[should_panic(expected = "returned NaN")]
    fn express_and_score_rejects_an_objective_that_returns_nan() {
        struct Poisoned;
        impl Fitness for Poisoned {
            fn evaluate(&self, _graph: &Graph) -> f64 {
                f64::NAN
            }
        }
        express_and_score(&population(3), &(), &Poisoned);
    }

    /// `Selection::Best` in one place: what it picks, in what order, that a
    /// tie goes to the lower index, and that it touches no randomness.
    ///
    /// Measured against the mutation corpus, none of these assertions catches
    /// anything `steady_state`'s tests do not already catch — reversing the
    /// sort, adding an RNG draw and dropping `rank`'s tie-break all fail there
    /// too. They are kept because a failure here names `Selection::Best`
    /// rather than a whole steady-state run, and merged into one test because
    /// four separate ones bought four names for a single unit of localization.
    #[test]
    fn best_selection_picks_the_fittest_of_its_scope_without_drawing() {
        // Scope deliberately unordered and missing index 1, the globally
        // fittest: `Best` ranks what it was handed, not the population.
        let fitnesses = [5.0, 0.5, 9.0, 3.0, 7.0];
        let scope = [4, 2, 0, 3];
        let mut rng = StdRng::seed_from_u64(1);

        // Best first, so a caller pairing parents gets the two fittest in a
        // defined order rather than whichever the sort happened to leave.
        assert_eq!(
            Selection::Best.pick(&scope, &fitnesses, 2, &mut rng),
            vec![3, 0]
        );
        assert_eq!(
            Selection::Best.pick(&scope, &fitnesses, 4, &mut rng),
            vec![3, 0, 4, 2]
        );

        // A tie goes to the lower index, as everywhere else in the engine.
        let tied = [2.0, 2.0, 9.0];
        assert_eq!(
            Selection::Best.pick(&[2, 1, 0], &tied, 1, &mut rng),
            vec![0]
        );

        // Steady-state draws its scope and then selects; if `Best` touched the
        // stream, adding a scheme that does not would shift every seeded run.
        let mut untouched = StdRng::seed_from_u64(42);
        let mut used = StdRng::seed_from_u64(42);
        Selection::Best.pick(&[0, 1, 2], &tied, 2, &mut used);
        assert_eq!(used.random::<u64>(), untouched.random::<u64>());
    }

    #[test]
    fn the_reported_best_is_the_lowest_fitness_and_ties_go_to_the_lower_index() {
        // What both evolvers package as the run's winner. Nothing else here
        // exercises it, and a tie at the top is exactly where an argmin that
        // keeps the later of two equals reports the wrong individual.
        assert_eq!(best_index(&[5.0, 1.0, 9.0, 3.0]), 1);
        assert_eq!(best_index(&[7.0]), 0);
        assert_eq!(best_index(&[2.0, 2.0, 2.0]), 0, "a tie keeps the first");
        assert_eq!(best_index(&[9.0, 4.0, 4.0]), 1);
        // Engine orientation is lower-is-better, so the largest value never
        // wins however it is spelled.
        assert_eq!(best_index(&[f64::INFINITY, 0.0]), 1);
        // `orient` rejects NaN long before this, so the guard is defensive —
        // but a plain `<` comparison would silently report a poisoned slot as
        // the run's answer, because every comparison against NaN is false.
        // `total_cmp` sorts NaN above every real number instead, matching how
        // the tournament and the replacement policy already treat it.
        assert_eq!(best_index(&[f64::NAN, 5.0]), 1);
        assert_eq!(best_index(&[5.0, f64::NAN]), 0);
    }

    #[test]
    #[should_panic(expected = "cannot pick a best of no individuals")]
    fn the_best_of_an_empty_population_is_a_bug() {
        best_index(&[]);
    }

    #[test]
    fn a_size_one_tournament_is_uniform_random_sampling() {
        let selection = Selection::Tournament { tournament_size: 1 };
        let population = population(8);
        let fitnesses = vec![0.0; 8];

        let mut rng = StdRng::seed_from_u64(3);
        let selected = selected_from_all(&selection, &population, &fitnesses, 20, &mut rng);

        // Nothing to compare, so winners are the raw index stream.
        let mut mirror = StdRng::seed_from_u64(3);
        let expected: Vec<_> = (0..20)
            .map(|_| IndexGenome::new(mirror.random_range(0..8)))
            .collect();

        assert_eq!(selected, expected);
    }

    #[test]
    fn each_winner_is_the_lowest_fitness_entrant_of_its_own_tournament() {
        let tournament_size = 4;
        let selection = Selection::Tournament { tournament_size };
        let population = population(10);
        // Non-monotonic, with a tie at slots 2 and 7 to exercise the tie-break.
        let fitnesses = vec![5.0, 9.0, 1.0, 4.0, 7.0, 3.0, 8.0, 1.0, 6.0, 2.0];

        let mut rng = StdRng::seed_from_u64(41);
        let selected = selected_from_all(&selection, &population, &fitnesses, 50, &mut rng);

        let mut mirror = StdRng::seed_from_u64(41);
        for winner in selected {
            let entrants: Vec<usize> = (0..tournament_size)
                .map(|_| mirror.random_range(0..10))
                .collect();
            assert_eq!(
                winner,
                IndexGenome::new(expected_winner(&fitnesses, &entrants))
            );
        }
    }

    #[test]
    fn a_tie_is_broken_toward_the_lower_index() {
        // All equally fit, so the lowest drawn index must always win.
        let selection = Selection::Tournament { tournament_size: 5 };
        let population = population(6);
        let fitnesses = vec![2.5; 6];

        let mut rng = StdRng::seed_from_u64(17);
        let selected = selected_from_all(&selection, &population, &fitnesses, 40, &mut rng);

        let mut mirror = StdRng::seed_from_u64(17);
        for winner in selected {
            let lowest_drawn = (0..5).map(|_| mirror.random_range(0..6)).min().unwrap();
            assert_eq!(winner, IndexGenome::new(lowest_drawn));
        }
    }

    #[test]
    fn a_larger_tournament_applies_more_selection_pressure() {
        // Fitness equals the index, so individual 0 is best and 19 is worst.
        let population = population(20);
        let fitnesses: Vec<f64> = (0..20).map(|i| i as f64).collect();

        let mean_selected = |tournament_size: usize| -> f64 {
            let selection = Selection::Tournament { tournament_size };
            let mut rng = StdRng::seed_from_u64(97);
            let picks = selected_from_all(&selection, &population, &fitnesses, 2_000, &mut rng);
            picks.iter().map(|g| g.index as f64).sum::<f64>() / picks.len() as f64
        };

        let uniform = mean_selected(1);
        let mild = mean_selected(2);
        let strong = mean_selected(8);

        // Size 1 is uniform: expect roughly the population mean of 9.5.
        assert!(
            (uniform - 9.5).abs() < 1.0,
            "size-1 tournament should be ~uniform, got mean index {uniform}",
        );
        // Catches an inverted comparison, which the other tests would not.
        assert!(
            strong < mild && mild < uniform,
            "pressure should increase with tournament size: {strong} < {mild} < {uniform}",
        );
    }

    #[test]
    fn a_nan_fitness_never_wins_a_tournament() {
        let selection = Selection::Tournament { tournament_size: 3 };
        let population = population(4);
        // Under a naive `<` this would win outright; it may only win alone.
        let fitnesses = vec![10.0, f64::NAN, 20.0, 30.0];

        let mut rng = StdRng::seed_from_u64(5);
        let selected = selected_from_all(&selection, &population, &fitnesses, 500, &mut rng);

        let mut mirror = StdRng::seed_from_u64(5);
        for winner in selected {
            let entrants: Vec<usize> = (0..3).map(|_| mirror.random_range(0..4)).collect();
            if entrants.iter().any(|&e| e != 1) {
                assert_ne!(
                    winner,
                    IndexGenome::new(1),
                    "NaN won against a real fitness in {entrants:?}",
                );
            }
        }
    }

    #[test]
    #[should_panic(expected = "scope names individual 2 but only 2 were scored")]
    fn a_scope_reaching_past_the_scored_population_is_rejected() {
        let selection = Selection::Tournament { tournament_size: 2 };
        let mut rng = StdRng::seed_from_u64(1);
        selected_from_all(&selection, &population(4), &[0.0, 1.0], 1, &mut rng);
    }

    #[test]
    #[should_panic(expected = "cannot select from an empty scope")]
    fn an_empty_scope_is_rejected() {
        let selection = Selection::Tournament { tournament_size: 2 };
        let mut rng = StdRng::seed_from_u64(1);
        selection.pick(&[], &[], 1, &mut rng);
    }

    #[test]
    #[should_panic(expected = "tournament_size must be at least 1")]
    fn a_zero_sized_tournament_is_rejected() {
        let selection = Selection::Tournament { tournament_size: 0 };
        let mut rng = StdRng::seed_from_u64(1);
        selected_from_all(&selection, &population(4), &[0.0; 4], 1, &mut rng);
    }

    #[test]
    fn a_zero_mutation_rate_never_mutates() {
        let mut rng = StdRng::seed_from_u64(1);
        let mut child = IndexGenome::new(0);

        // Over enough trials that a rate misread as a per-mutation probability,
        // or an unconditional count roll, would show up.
        for _ in 0..1_000 {
            mutate_child(&mut child, &(), 0.0, 4, &mut rng);
        }

        assert_eq!(child.mutations, 0, "rate 0.0 must not mutate at all");
    }

    #[test]
    fn a_certain_mutation_at_max_one_applies_exactly_one() {
        let mut rng = StdRng::seed_from_u64(2);

        for _ in 0..100 {
            let mut child = IndexGenome::new(0);
            mutate_child(&mut child, &(), 1.0, 1, &mut rng);

            assert_eq!(child.mutations, 1, "max_mutations 1 must apply exactly one");
        }
    }

    #[test]
    fn the_mutation_count_covers_one_to_max_and_never_exceeds_it() {
        let mut rng = StdRng::seed_from_u64(3);
        let mut seen = [false; 5];

        for _ in 0..500 {
            let mut child = IndexGenome::new(0);
            mutate_child(&mut child, &(), 1.0, 4, &mut rng);

            assert!(
                (1..=4).contains(&child.mutations),
                "{} mutations applied, max_mutations was 4",
                child.mutations,
            );
            seen[child.mutations] = true;
        }

        // An inclusive range drawn uniformly: a `1..max` off-by-one would never
        // produce 4, and a zero lower bound would eventually produce 0.
        assert!(
            seen[1..].iter().all(|&drawn| drawn),
            "not every count in 1..=4 was drawn over 500 trials: {seen:?}",
        );
    }

    #[test]
    #[should_panic(expected = "max_mutations must be at least 1")]
    fn a_zero_max_mutations_is_rejected() {
        let mut rng = StdRng::seed_from_u64(4);
        mutate_child(&mut IndexGenome::new(0), &(), 1.0, 0, &mut rng);
    }
}
