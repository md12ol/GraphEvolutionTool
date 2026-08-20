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

/// Recombination operator, chosen once per run.
///
/// An enum rather than a trait, and for the same reason as [`Selection`] below:
/// a second operator is one extra variant plus one match arm, selectable by
/// name from `config.toml` with no Rust at the call site.
///
/// # Why this is engine-level and shared, while the mutation operator is not
///
/// Both shipped representations recombine the *same* way — two-point, over
/// whatever their linear unit is, drawing cut points from one shared helper —
/// so `TwoPoint` is a truthful name for both and there is one enum for the
/// run. Mutation is the opposite: what one mutation *does* differs completely
/// between representations, so the mutation operator is selected per genome,
/// under `[genome]`, and its variants live beside the representation they
/// belong to. See `crate::genomes::EdgeEditMutation` and
/// `crate::genomes::SdaMutation`.
///
/// The practical consequence is that a variant added here is offered for
/// *every* representation. One that only some genomes can honour has to be
/// rejected by `Config`'s validation, since nothing in the type system pairs
/// this enum with the selected genome.
///
/// # Adding an operator
///
/// 1. **This enum** — the variant, plus any parameters it reads from the file.
/// 2. **[`Crossover::recombine`]** — the arm that performs it. The compiler
///    finds this one for you: the match is exhaustive.
/// 3. **`config::CrossoverConfig`** — the variant a user names under
///    `[crossover]`, and any constraint it needs in
///    `Config::validate_crossover`, which is also where an operator no genome
///    can honour is refused.
/// 4. **`dispatch::crossover`** — the arm mapping that config variant onto this
///    one.
/// 5. **`py_config::PyCrossoverConfig`** — optional, and only buys a Python
///    caller the ability to name it. Skipped, the operator still runs from TOML
///    and from Rust.
/// 6. **`config.example.toml`** — also optional, and also the step people skip
///    and then wonder why nobody uses the operator: that file is what a new
///    user copies from.
///
/// A `Genome::crossover` that cannot express the operator is the case this
/// enum does not cover. `Genome::crossover` takes no context, deliberately, so
/// an operator needing per-representation behaviour adds a trait method rather
/// than a variant here.
///
/// **Every step above is marked at its own site in the code.** Search the
/// repo for `ADD A CROSSOVER STEP 3`, or any other number:
///
/// ```text
/// git grep -n "ADD A CROSSOVER STEP"    # all six, in one list
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crossover {
    /// Two-point: swap one contiguous band between the parents, leaving
    /// everything outside it untouched on both sides. What the band is made of
    /// is the representation's own business — genes for edge-edit, states for
    /// SDA — and each decides how much shared structure it needs before
    /// crossing at all.
    TwoPoint,
    // ADD A CROSSOVER STEP 1 — a variant here, plus any parameters it reads
    // from the file:
    //
    //     MyCrossover { some_param: f64 },
    //
    // Then the arm performing it, in `Crossover::recombine` below — search
    // `ADD A CROSSOVER STEP 2` for it.
}

impl Crossover {
    /// Recombine one pair in place, both children kept.
    ///
    /// The caller has already rolled `crossover_rate` and decided this pair
    /// breeds; this only chooses *how*.
    pub fn recombine<G, R>(&self, first: &mut G, second: &mut G, rng: &mut R)
    where
        G: Genome,
        R: Rng + ?Sized,
    {
        match self {
            // Two-point is what `Genome::crossover` already means for every
            // representation, so this arm is the trait call itself rather than
            // an implementation. A second variant would not be — it would
            // dispatch to a second trait method, which is the point at which
            // `Genome` grows one.
            Crossover::TwoPoint => first.crossover(second, rng),
            // ADD A CROSSOVER STEP 2 — the arm performing your variant. The
            // step after this one is `config::CrossoverConfig` — search
            // `ADD A CROSSOVER STEP 3` for it.
        }
    }
}

/// Parent-selection strategy: who breeds, within the scope an event drew.
///
/// An enum rather than a trait, so a new mechanism is one variant plus one
/// match arm, selectable by name from `config.toml` with no Rust at the call
/// site. **This is the one extension point unreachable from outside the crate**
/// — `Fitness`, `Genome` and `Evolver` are traits a depending program
/// implements, but a variant cannot be added to another crate's enum.
///
/// # The contract every scheme keeps
///
/// None is enforced by the signature, and breaking any changes every evolver's
/// behaviour at once rather than failing anywhere visible.
///
/// - **Pick only from `scope`** — reaching past it breaks the guarantee that a
///   strategy's best individual is never among those replaced.
/// - **Sample with replacement.** A caller wanting distinct parents enforces
///   that itself; quietly de-duplicating removes the pressure that comes from a
///   strong individual being drawn twice.
/// - **Fitnesses arrive oriented**, lower is better, so compare them directly
///   through `rank` and never consult a `Direction`. Re-checking it looks
///   defensive and silently inverts every maximizing objective.
/// - **All randomness comes from `rng`**, never a thread RNG or the clock, or
///   two runs at one seed stop agreeing with nothing to report it.
///
/// How a scheme picks is its own business — pressure, and whether it reads
/// fitness values or only their order.
///
/// # Adding a scheme
///
/// 1. **This enum** — the variant and its parameters.
/// 2. **`Selection::pick`** — the arm; the match is exhaustive.
/// 3. **`config::SelectionConfig`** — what a user names under `[selection]`,
///    plus any parameter constraint in `validate_evolution_and_selection`.
/// 4. **`dispatch::selection`** — config variant onto this one. Nothing here
///    decides a scope: `[scope]` is its own block, with its own chain.
/// 5. **`py_config::PySelectionConfig`** — optional; buys a Python caller the
///    ability to name it.
/// 6. **`config.example.toml`** — optional, and the step people skip and then
///    wonder why nobody uses the scheme.
///
/// No step here concerns locality or replacement, which is the point: a scheme
/// works with every strategy because it answers only this one question. The
/// other two axes are [`super::scope::Scope`] and
/// [`super::replacement::Replacement`].
///
/// **Every step above is marked at its own site in the code.** Search the repo
/// for `ADD A SELECTION STEP 3`, or any other number:
///
/// ```text
/// git grep -n "ADD A SELECTION STEP"    # all six, in one list
/// ```
pub enum Selection {
    /// The fittest members of the scope, best first. Consumes no randomness.
    ///
    /// Truncation over a randomly drawn subset is what "tournament selection"
    /// decomposes into once the draw is its own step, which is how steady-state
    /// is expressed.
    Best,
    /// Sample `tournament_size` members of the scope per pick, keep the best.
    Tournament { tournament_size: usize },
    // ADD A SELECTION STEP 1 — a variant here, plus any parameters the scheme
    // reads out of the file:
    //
    //     Roulette { pressure: f64 },
    //
    // The variant name becomes `type = "roulette"` under `[selection]`, via the
    // `rename_all` on `config::SelectionConfig`. Then the arm performing it, in
    // `Selection::pick` below — search `ADD A SELECTION STEP 2` for it.
}

/// Order two individuals, better first, ties broken by lower index.
///
/// Fitnesses are already oriented so lower is better — every comparison in this
/// module rests on that, and none of them consults a `Direction`. Orientation
/// happens once, before a score reaches selection at all; a scheme that checks
/// the direction again inverts it for maximizing objectives. The index
/// tie-break makes
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
    /// Choose `count` parents from `scope`, returning indices into the
    /// population.
    ///
    /// Indices rather than clones: the caller knows whether it wants a copy or
    /// the slot number, and steady-state wants both.
    pub(super) fn pick<R>(
        &self,
        scope: &[usize],
        fitnesses: &[f64],
        count: usize,
        rng: &mut R,
    ) -> Vec<usize>
    where
        // `?Sized` lets callers pass a trait-object RNG (e.g. `&mut dyn RngCore`),
        // not just a concrete sized type — same reason on every `R: Rng` bound below.
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
              //
              // Pick only from `scope`, compare through `rank`, and take every
              // random value from `rng` — the contract above says why each
              // matters. The step after this is the config variant a user names —
              // search `ADD A SELECTION STEP 3` for it.
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
