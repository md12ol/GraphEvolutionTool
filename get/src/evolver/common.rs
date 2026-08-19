//! GA mechanics shared by every evolution strategy.
//!
//! These helpers keep selection, population setup, evaluation, and logging in
//! one place so [`super::generational`] and [`super::steady_state`] don't each
//! re-implement them.

use std::cmp::Ordering;

use rand::Rng;
use rayon::prelude::*;

use super::{GenerationStats, SharedEvolutionContext};
use crate::fitness::Fitness;
use crate::genomes::Genome;
use crate::graph::Graph;

/// Parent-selection strategy.
///
/// An enum rather than a trait, so a new mechanism (roulette-wheel, truncation,
/// rank, ...) is one extra variant plus its match arms, and maps directly onto a
/// `config.toml` field.
///
/// # Adding your own scheme
///
/// **This is the one extension point that cannot be reached from outside the
/// crate.** The others — `Fitness`, `Genome`, `Evolver` — are traits, so a
/// program that depends on GET implements them for its own types. A variant
/// cannot be added to an enum from another crate, so a depending program's only
/// schemes are the ones GET ships. Adding one means editing your own copy of
/// GET, and what that buys is a scheme selectable by name from `config.toml`
/// and runnable by the `get-run` binary, with no Rust at the call site. A
/// reader expecting the trait pattern to hold here will go looking for a
/// `Selection` trait that does not exist.
///
/// A scheme touches five files, in seven steps — `common.rs` and `config.rs`
/// each want more than one edit. In the order you would walk them:
///
/// 1. **This enum** — add the variant, carrying whatever parameters the scheme
///    reads out of the file.
/// 2. **`select`** — add its arm. This is the whole of parent selection and the
///    only method every scheme must answer. Read its contract first: it governs
///    replacement, orientation and where randomness may come from, and none of
///    the three is enforced by the signature.
/// 3. **`tournament_indices`** — add an arm **only if the scheme is to be usable
///    with steady-state**. It is deliberately not part of the general contract.
///    It draws one ranked set of *distinct* individuals, from which
///    [`super::steady_state`] takes both its parents and the individuals they
///    replace — a replacement policy rather than a selection one, and one that
///    only means something over a set. A scheme whose own theory has no such
///    draw (roulette-wheel samples with replacement, and a distinct-sample
///    variant of it is a different scheme wearing the name) supplies no arm, and
///    step 5 rejects the pairing at config time.
/// 4. **`steady_state.rs`** — `SteadyStateEvolver::new` matches on the variant
///    directly, to assert its tournament floor before a run starts rather than
///    at the first mating event. A scheme that reached step 3 needs the
///    equivalent guarantee stated here; one that did not still needs the arm, to
///    reject the combination.
/// 5. **`config.rs`** — three edits. Add a `SelectionConfig` variant holding
///    what the scheme reads out of the file; add any constraint on its own
///    parameters; and extend `validate_evolution_and_selection`, whose
///    `let SelectionConfig::Tournament { .. } = self.selection;` is irrefutable
///    only while there is one variant and becomes a real match with the second.
///    That function is where a scheme is paired with a strategy, so it is where
///    a scheme with no step 3 is rejected against steady-state — a config error
///    naming the pair, rather than a run that quietly is not the scheme asked
///    for.
/// 6. **`dispatch.rs`** — add the arm to `selection()`, which turns the config
///    variant into this enum. Steps 5 and 6 are one change split across two
///    files: a variant nothing constructs is dead code, and an arm for a variant
///    that does not exist will not compile.
/// 7. **`config.example.toml`** — add the `[selection]` block if the scheme
///    ships. The example file is what a user copies from, so a scheme missing
///    from it is one most people never find.
///
/// Steps 1, 2, 5 and 6 fail to compile if forgotten; the matches are exhaustive.
/// Steps 3, 4 and 7 do not, and are the ones to check by hand.
pub enum Selection {
    /// Sample `tournament_size` individuals per pick and keep the best.
    Tournament { tournament_size: usize },
}

/// Order two individuals, better first, ties broken by lower index.
///
/// Fitnesses are already oriented so lower is better. The index tie-break makes
/// a tournament's outcome depend only on which indices were drawn, not the
/// order the RNG produced them. `total_cmp` is used simply because sorting
/// needs a total order; `Direction::orient` rejects `NaN` before it gets here.
pub(super) fn rank(fitnesses: &[f64], a: usize, b: usize) -> Ordering {
    // .then() only falls through to the index comparison when the fitness
    // comparison was Equal — this is the tie-break, not a second sort key.
    fitnesses[a].total_cmp(&fitnesses[b]).then(a.cmp(&b))
}

/// Index of the best individual in `fitnesses`, ties broken by lower index.
///
/// The same ordering as [`rank`], applied across the whole slice. Both evolvers
/// need this to package their outcome, so it lives here rather than being
/// written twice.
///
/// Panics if `fitnesses` is empty. Both callers construct a non-empty
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
    /// Select `count` parents, sampling **with** replacement — the same
    /// individual may be returned more than once. Callers needing distinct
    /// individuals must enforce that themselves, since `select` cannot know
    /// whether its output is a mating pair or an unrelated batch.
    pub(super) fn select<G, R>(
        &self,
        population: &[G],
        fitnesses: &[f64],
        count: usize,
        rng: &mut R,
    ) -> Vec<G>
    where
        G: Genome,
        // `?Sized` lets callers pass a trait-object RNG (e.g. `&mut dyn RngCore`),
        // not just a concrete sized type — same reason on every `R: Rng` bound below.
        R: Rng + ?Sized,
    {
        assert_eq!(
            population.len(),
            fitnesses.len(),
            "every individual needs exactly one fitness",
        );
        assert!(
            !population.is_empty(),
            "cannot select from an empty population",
        );

        match self {
            Selection::Tournament { tournament_size } => {
                assert!(*tournament_size > 0, "tournament_size must be at least 1");

                (0..count)
                    .map(|_| {
                        population[Self::tournament_winner(fitnesses, *tournament_size, rng)]
                            .clone()
                    })
                    .collect()
            }
        }
    }

    /// Draw one tournament of **distinct** individuals, best first.
    ///
    /// Feeds tournament-local replacement: the front of the result are parents,
    /// the back are the individuals they displace. Distinctness is required —
    /// "the worst two members" means nothing over a multiset.
    pub(super) fn tournament_indices<R>(&self, fitnesses: &[f64], rng: &mut R) -> Vec<usize>
    where
        R: Rng + ?Sized,
    {
        match self {
            Selection::Tournament { tournament_size } => {
                assert!(*tournament_size > 0, "tournament_size must be at least 1");
                assert!(
                    *tournament_size <= fitnesses.len(),
                    "tournament_size {} exceeds population size {}",
                    tournament_size,
                    fitnesses.len(),
                );

                // Rejection sampling. `tournament_size` is small, so the linear
                // membership scan beats a hash set, and this avoids the
                // O(population) buffer a shuffle would need on every event.
                let mut entrants = Vec::with_capacity(*tournament_size);
                while entrants.len() < *tournament_size {
                    let candidate = rng.random_range(0..fitnesses.len());
                    if !entrants.contains(&candidate) {
                        entrants.push(candidate);
                    }
                }

                entrants.sort_by(|&a, &b| rank(fitnesses, a, b));
                entrants
            }
        }
    }

    /// Draw `tournament_size` individuals **with** replacement, best wins.
    fn tournament_winner<R>(fitnesses: &[f64], tournament_size: usize, rng: &mut R) -> usize
    where
        R: Rng + ?Sized,
    {
        let mut winner = rng.random_range(0..fitnesses.len());
        for _ in 1..tournament_size {
            let candidate = rng.random_range(0..fitnesses.len());
            if rank(fitnesses, candidate, winner) == Ordering::Less {
                winner = candidate;
            }
        }
        winner
    }
}

/// Apply the engine's two mutation dice rolls to one child.
///
/// 1. `mutation_rate` — whether this child mutates at all.
/// 2. `max_mutations` — if it does, how many mutations it takes, drawn uniformly
///    from `1..=max_mutations`.
///
/// Both rolls live here, in one helper both evolution strategies call, so they
/// cannot drift apart on mutation semantics the way they did on selection
/// sampling. Neither roll belongs to the genome: [`Genome::mutate`] applies
/// exactly one mutation per call, and a representation that rolled its own count
/// would make `max_mutations` mean nothing for that representation with nothing
/// to report it.
///
/// The rolls happen in a fixed order — rate first, then count — so a seeded run
/// reproduces exactly.
///
/// **A seeded run does not reproduce what the pre-`max_mutations` engine
/// produced, even at `max_mutations = 1`.** The count roll is unconditional once
/// the rate roll passes, and `random_range(1..=1)` still consumes RNG state
/// despite having only one possible outcome (measured 2026-08-03). Skipping the
/// draw at 1 would restore the old sequence, and is deliberately not done: a
/// special case that changes the RNG stream based on a config value is a worse
/// thing to own than a one-time change in seeded output, which was accepted when
/// this was designed.
///
/// # Panics
///
/// If `max_mutations` is zero: `1..=0` is an empty range with no meaningful
/// draw. A backstop only — the config layer rejects it first, but the evolvers
/// are constructible directly, so this checks rather than trusting its caller.
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
/// The order is part of the contract, not an implementation detail: a seeded run
/// only reproduces if every strategy draws from the RNG in the same sequence, so
/// generational and steady-state both breed through here rather than each
/// spelling the same four calls out.
///
/// Takes the whole [`SharedEvolutionContext`] rather than its four relevant
/// fields, so a field added there cannot be forgotten at one of the two call
/// sites.
///
/// `first` and `second` are the parents, mutated in place into the children.
/// Neither is scored here — the caller decides when and in what batch.
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
        first.crossover(second, rng);
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

/// Express every genome against the shared context and score the whole batch,
/// returning the expressed graphs alongside their fitnesses. Index `i` of both
/// vectors refers to `batch[i]`.
///
/// The returned fitnesses are **oriented**: lower is better, whatever the
/// objective's own direction. This is the one place that conversion happens, so
/// everything downstream can compare without knowing the direction.
///
/// # This is the engine's sole scoring entry
///
/// **The engine never calls [`Fitness::evaluate`] or
/// [`Fitness::evaluate_batch`] directly.** Every path from a batch of genomes
/// to a set of fitnesses goes through here — generational scoring, steady-state child
/// scoring, the final outcome, all of it. Those two trait methods exist to be
/// *implemented* by an objective and *called by this function*.
///
/// The rule is worth stating because breaking it fails **silently**, twice over:
///
/// - **Orientation is bypassed.** A direct call returns the objective's own
///   units, so under [`crate::fitness::Direction::Maximize`] every comparison
///   runs backwards. A run optimizing away from the goal looks exactly like one
///   that is merely not converging.
/// - **The `NaN` gate is bypassed.** [`crate::fitness::Direction::orient`] is
///   what rejects `NaN`,
///   and under `Maximize` an unchecked `-NaN` sorts *below* `-inf` — so it wins
///   every tournament it enters and fills the population with whatever produced
///   it, leaving a run that looks converged.
///
/// Both doors are the same door by design. This is also why the alternative — a
/// direction-aware comparator — was rejected: it needs the direction at every
/// comparison site, and a missed one is invisible. Scoring the whole batch
/// in one place is what lets "exactly once" be *guaranteed* rather than
/// remembered. Spec §5.1.
///
/// Defers to [`Fitness::evaluate_batch`] so native objectives parallelize
/// over rayon and Python-backed ones batch across the FFI boundary.
///
/// The graphs are returned rather than dropped because scoring has to build them
/// anyway: handing them back costs nothing, and it saves the caller re-expressing
/// the winner to fill [`super::EvolutionOutcome::best_graph`]. Callers that only
/// need scores can ignore the first element and let it drop.
///
/// # Panics
///
/// If the objective returns `NaN` for any individual — see
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
    // Expression is parallel; `Genome::Context: Send + Sync` exists for this.
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
/// Every field is in **engine orientation** — lower is better — because that is
/// what the caller passes in and nothing here converts. Only the boundary
/// converts, once, on the way out of a run; see [`express_and_score`] for the
/// matching flip inward, and spec §5.1 for why there are exactly two.
///
/// This function used to take a [`crate::fitness::Direction`] and convert
/// `best` and `mean` back
/// into the objective's units, which meant `std_dev` had to be deliberately
/// *skipped* — deviation is unchanged by negation. That exception read as a
/// missed case and needed a test to defend it. With nothing converting here,
/// there is nothing to except.
///
/// `std_dev` is the population deviation (divides by `n`), not the sample
/// deviation: these are all the individuals there are, not a sample of a larger
/// group. A single individual therefore has a deviation of zero.
///
/// `ci_95` divides by `n - 1` instead, on purpose: it estimates the
/// uncertainty in `mean_fitness` as a statistic, which is a sample-deviation
/// question even though `std_dev` right beside it is a population-deviation
/// one. `n == 1` gives `0.0`, not a division by zero.
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
    /// of the first two assertions and fails the test. Spec §5.1.
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
        // erases the objective (§8), so the box has to reach the objective's own
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

    #[test]
    fn a_size_one_tournament_is_uniform_random_sampling() {
        let selection = Selection::Tournament { tournament_size: 1 };
        let population = population(8);
        let fitnesses = vec![0.0; 8];

        let mut rng = StdRng::seed_from_u64(3);
        let selected = selection.select(&population, &fitnesses, 20, &mut rng);

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
        let selected = selection.select(&population, &fitnesses, 50, &mut rng);

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
        let selected = selection.select(&population, &fitnesses, 40, &mut rng);

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
            let picks = selection.select(&population, &fitnesses, 2_000, &mut rng);
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
        let selected = selection.select(&population, &fitnesses, 500, &mut rng);

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
    fn a_tournament_of_the_whole_population_is_just_the_fitness_ordering() {
        // Tournament == population: every index is drawn once, so the result
        // cannot depend on the RNG. Ties at 1.0 and 3.0 pin the tie-break.
        let fitnesses = vec![3.0, 1.0, 8.0, 2.0, 1.0, 3.0];
        let selection = Selection::Tournament { tournament_size: 6 };

        for seed in 0..8 {
            let mut rng = StdRng::seed_from_u64(seed);
            let drawn = selection.tournament_indices(&fitnesses, &mut rng);
            assert_eq!(drawn, vec![1, 4, 3, 0, 5, 2], "seed {seed}");
        }
    }

    #[test]
    fn a_tournament_draws_distinct_individuals_ordered_best_first() {
        let fitnesses = vec![5.0, 9.0, 1.0, 4.0, 7.0, 3.0, 8.0, 1.0, 6.0, 2.0];
        let selection = Selection::Tournament { tournament_size: 4 };

        let mut rng = StdRng::seed_from_u64(23);
        for _ in 0..200 {
            let drawn = selection.tournament_indices(&fitnesses, &mut rng);

            assert_eq!(drawn.len(), 4);

            let mut unique = drawn.clone();
            unique.sort_unstable();
            unique.dedup();
            assert_eq!(unique.len(), 4, "tournament had duplicates: {drawn:?}");

            for pair in drawn.windows(2) {
                let (earlier, later) = (pair[0], pair[1]);
                assert!(
                    fitnesses[earlier] < fitnesses[later]
                        || (fitnesses[earlier] == fitnesses[later] && earlier < later),
                    "not best-first at {earlier} -> {later} in {drawn:?}",
                );
            }
        }
    }

    #[test]
    fn a_nan_fitness_sorts_to_the_replaceable_end_of_a_tournament() {
        // A poisoned slot must sort last, never into a parent position.
        let fitnesses = vec![4.0, 7.0, f64::NAN, 1.0, 9.0];
        let selection = Selection::Tournament { tournament_size: 5 };

        let mut rng = StdRng::seed_from_u64(2);
        let drawn = selection.tournament_indices(&fitnesses, &mut rng);

        assert_eq!(
            drawn,
            vec![3, 0, 1, 4, 2],
            "NaN should sort last, making it the first individual replaced",
        );
    }

    #[test]
    #[should_panic(expected = "exceeds population size")]
    fn a_tournament_larger_than_the_population_is_rejected() {
        let selection = Selection::Tournament { tournament_size: 7 };
        let mut rng = StdRng::seed_from_u64(1);
        selection.tournament_indices(&[0.0; 5], &mut rng);
    }

    #[test]
    #[should_panic(expected = "every individual needs exactly one fitness")]
    fn a_fitness_array_of_the_wrong_length_is_rejected() {
        let selection = Selection::Tournament { tournament_size: 2 };
        let mut rng = StdRng::seed_from_u64(1);
        selection.select(&population(4), &[0.0, 1.0], 1, &mut rng);
    }

    #[test]
    #[should_panic(expected = "cannot select from an empty population")]
    fn an_empty_population_is_rejected() {
        let selection = Selection::Tournament { tournament_size: 2 };
        let mut rng = StdRng::seed_from_u64(1);
        selection.select::<IndexGenome, _>(&[], &[], 1, &mut rng);
    }

    #[test]
    #[should_panic(expected = "tournament_size must be at least 1")]
    fn a_zero_sized_tournament_is_rejected() {
        let selection = Selection::Tournament { tournament_size: 0 };
        let mut rng = StdRng::seed_from_u64(1);
        selection.select(&population(4), &[0.0; 4], 1, &mut rng);
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

            // The contract this task exists to enforce: one call, one mutation.
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
