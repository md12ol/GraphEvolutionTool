//! The objectives the GA optimizes, and the sign rule that makes them
//! comparable.
//!
//! # Adding your own objective
//!
//! 1. Implement [`Fitness`] — [`Fitness::evaluate`] is the only required
//!    method; add [`Fitness::direction`] if bigger is better.
//! 2. Add a variant to `FitnessConfig` in [`crate::config`].
//! 3. Add the matching arm in `GraphEvolver::run`.
//!
//! If it is epidemic-based, build it on [`EpidemicScorer`] rather than calling
//! the simulator yourself. It owns the seeding, and wrong seeding still gives
//! numbers that look fine. Copy the shape [`EpiSpread`] uses: both `evaluate`
//! and `evaluate_population` hand [`EpidemicScorer::mean_batch`] a closure
//! saying what to read off one epidemic. Write the same reading in both — the
//! test `both_entry_points_use_the_same_reading` fails if they disagree.

use std::slice;
use std::sync::atomic::{AtomicU64, Ordering};

use rayon::prelude::*;

use crate::graph::Graph;
use crate::sir::{SirRun, SirSampleParams, simulate_epidemics};

/// Whether an objective wants its value small or large.
///
/// Every fitness number in the engine is in one of two forms, and mixing them
/// up is the bug this type exists to prevent:
///
/// - **original** — what the fitness function returned, in its own units. 28
///   nodes infected is `28.0`, and bigger is better.
/// - **oriented** — the original after [`Direction::orient`], which negates it
///   when the objective maximizes and leaves it alone when the objective
///   minimizes, so that smaller is always better. That same 28 becomes
///   `-28.0`.
///
/// The engine only ever compares, so it works in oriented values throughout;
/// logs and results are turned back into originals at the boundary (§5.1),
/// which is what the sheet calls engine orientation.
///
/// The objective does not negate its own output, because then the value and
/// the declared direction could disagree — and a run optimizing backwards
/// looks exactly like a run that is simply not converging.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// Smaller is better, as for an error or a distance. The default.
    Minimize,
    /// Larger is better.
    Maximize,
}

impl Direction {
    /// Orient an original, so that smaller always wins.
    ///
    /// Under [`Direction::Minimize`] the two are the same number; under
    /// [`Direction::Maximize`] the oriented value is the negated original, so
    /// the largest original becomes the smallest oriented value.
    ///
    /// Negation is its own inverse, so this one function converts both ways:
    /// an original in to compare, an oriented value in to report.
    ///
    /// # Panics
    ///
    /// On `NaN`. Under [`Direction::Maximize`] it becomes `-NaN`, which sorts
    /// below `-inf` — so it would win every tournament it entered and leave a
    /// run that looks converged. Rust's `assert!` survives a release build, so
    /// this check always runs.
    pub fn orient(self, value: f64) -> f64 {
        assert!(
            !value.is_nan(),
            "fitness function returned NaN, which the Fitness contract forbids. \
             Check for division by a possibly-zero count, 0.0/0.0, or inf - inf \
             in the objective's arithmetic.",
        );
        match self {
            Direction::Minimize => value,
            Direction::Maximize => -value,
        }
    }
}

/// An objective the GA optimizes over expressed graphs.
///
/// [`Fitness::evaluate`] returns the **original** score, in the objective's
/// own units; [`Fitness::direction`] says which way is better. The engine
/// orients it exactly once, so logs and results keep the original units and
/// sign (§5.1). See [`Direction`] for both terms.
///
/// `Send + Sync` lets [`Fitness::evaluate_population`] score across rayon
/// threads.
///
/// # Implement these; never call them
///
/// Only `common::express_and_score` calls them. A direct call compiles and
/// returns plausible numbers, but hands the engine an original where an
/// oriented value belongs — so under [`Direction::Maximize`] every comparison
/// runs backwards, and nothing says so. It skips the `NaN` check too.
///
/// # Never return `NaN`
///
/// [`Direction::orient`] panics on it. Watch for division by a count that can
/// be zero, `0.0 / 0.0`, and `inf - inf`.
pub trait Fitness: Send + Sync {
    /// Score one graph: the **original**, in the objective's own units, never
    /// an oriented value. Must not return `NaN`.
    fn evaluate(&self, graph: &Graph) -> f64;

    /// Which way is better. Defaults to [`Direction::Minimize`], so an error
    /// or distance objective says nothing.
    fn direction(&self) -> Direction {
        Direction::Minimize
    }

    /// Score a **batch of graphs** — whatever set the evolver scores together.
    /// These come back as originals too; the caller converts them.
    ///
    /// The batch is not always a generation. Generational hands over the whole
    /// population each cycle; steady-state hands over just the two new
    /// children per mating event, and its starting population once (§6.3). All
    /// three are batches.
    ///
    /// The default runs [`Fitness::evaluate`] on each graph across rayon,
    /// which suits a Rust objective. A Python one overrides this to take the
    /// GIL once per batch instead of once per graph.
    ///
    /// **A stochastic objective must override it as well** — the default would
    /// draw a fresh seed per graph, so scores would no longer be comparable
    /// inside the batch. See [`EpidemicScorer::mean_batch`].
    fn evaluate_population(&self, graphs: &[Graph]) -> Vec<f64> {
        graphs
            .par_iter()
            .map(|graph| self.evaluate(graph))
            .collect()
    }
}

/// Runs the epidemics that every SIR objective scores (§5.2).
///
/// The epidemic is the expensive part and all three objectives want the same
/// one, so this runs the batch and each objective supplies only a reading —
/// see [`EpiSpread`] for the smallest example.
///
/// The nesting, widest first (§5.2, §8.1): an **experiment** is many **runs**
/// at one set of parameters; a run scores many **batches of graphs**; each
/// batch averages many **epidemics** per graph. One scorer covers one run.
///
/// A batch is whatever the evolver scores in one call, and its size is not
/// fixed: the whole population for a generational cycle or for either
/// evolver's starting population, but only the **two new children** for a
/// steady-state mating event (§6.3). Nothing here needs to know which — it
/// seeds whatever arrives.
///
/// **One scorer per run.** The counter below is per-run state; two replicates
/// sharing a scorer would let thread scheduling pick which run saw which seed,
/// and reproducibility goes with it (§8.1).
pub struct EpidemicScorer {
    params: SirSampleParams,
    run_seed: u64,
    /// Batches scored so far — see [`EpidemicScorer::next_batch_seed`].
    batches_scored: AtomicU64,
}

impl EpidemicScorer {
    /// Build a scorer for one run.
    ///
    /// `run_seed` is this run's share of the master seed handed to
    /// `GraphEvolver::run`; `[fitness]` has no seed of its own (§5.2).
    pub fn new(params: SirSampleParams, run_seed: u64) -> Self {
        Self {
            params,
            run_seed,
            batches_scored: AtomicU64::new(0),
        }
    }

    /// The seed for the next batch of graphs. Every call returns a different
    /// one, because it advances the counter.
    ///
    /// A seed fixes every random choice the epidemic simulator makes. Same
    /// seed, same epidemics. Different seed, different epidemics.
    ///
    /// **Call this once per batch, then give that one seed to every graph in
    /// the batch.** The batch size depends on the evolver, and nothing here
    /// changes with it:
    ///
    /// ```text
    /// generational   batch 1   seed A   all 200 of the population
    ///                batch 2   seed B   all 200 of the next generation
    ///
    /// steady-state   batch 1   seed A   the 200 starting graphs
    ///                batch 2   seed B   the 2 children of one mating event
    ///                batch 3   seed C   the 2 children of the next event
    /// ```
    ///
    /// *One seed across the batch*, because those graphs are compared with
    /// each other. If each drew its own, a graph could rank first for having
    /// been handed a milder outbreak.
    ///
    /// *A new seed for the next batch*, because reusing A forever would breed
    /// a population good at outbreak A rather than good at the disease.
    ///
    /// Both properties together are what §5.2 calls common random numbers.
    ///
    /// Steady-state pays a known cost here, accepted in §5.2: its two children
    /// are scored under a newer seed than the population they are compared
    /// against, and a graph that drew an easy outbreak keeps that score until
    /// something replaces it.
    ///
    /// The counter is an atomic for a duller reason than it looks: `evaluate`
    /// only gets `&self`, so a plain `+= 1` will not compile, and `Cell` is
    /// not `Sync`, which [`Fitness`] requires. Nothing here is actually
    /// contended — [`EpidemicScorer::mean_batch`] calls this once on its own
    /// thread before rayon fans out, batches are scored one after another, and
    /// each replicate owns its own scorer (§8.1). `Relaxed` is enough because
    /// no other data rides along with the count.
    pub fn next_batch_seed(&self) -> u64 {
        let counter = self.batches_scored.fetch_add(1, Ordering::Relaxed);
        mix_seed(self.run_seed, counter)
    }

    /// Score a whole batch of graphs — **one seed for every graph, one tick**.
    ///
    /// This is the only way to score, and the method that delivers common
    /// random numbers. It is why each objective overrides
    /// [`Fitness::evaluate_population`] rather than letting the default score
    /// the graphs one at a time (§5.2). It does not care whether the batch is
    /// a generation, a starting population or two steady-state children — a
    /// single graph is a batch of one.
    ///
    /// `read` turns one epidemic into one number, which is what keeps each
    /// objective to a single line. Averaging matters: a single epidemic is
    /// noisy enough that selection would chase the dice instead of the graph.
    /// The division is safe — [`simulate_epidemics`] rejects an empty batch.
    ///
    /// `+ Sync` on `read` lets rayon call it from several threads at once.
    pub fn mean_batch(&self, graphs: &[Graph], read: impl Fn(&SirRun) -> f64 + Sync) -> Vec<f64> {
        // Taken once, here, and handed to every graph below. Taking it inside
        // the loop would give each graph its own dice — see next_batch_seed.
        let seed = self.next_batch_seed();

        graphs
            .par_iter()
            .map(|graph| {
                let epidemics = simulate_epidemics(graph, &self.params, seed);

                let mut total = 0.0;
                for epidemic in &epidemics {
                    total += read(epidemic);
                }
                total / epidemics.len() as f64
            })
            .collect()
    }
}

/// Turn a run seed and a batch number into that batch's seed.
///
/// SplitMix64: step a large odd constant `counter` times, then scramble. Every
/// pair gives a different, well-spread `u64`, which is all this needs — the
/// result seeds a real generator and is never used as randomness itself.
/// (`wrapping_*` lets the arithmetic overflow and wrap instead of panicking.)
///
/// **Not `run_seed ^ counter`** (§8.1): neighbouring run seeds would collide
/// across batch numbers, so two replicates would replay each other's epidemics
/// one batch apart. See `decisions.md` 2026-08-06.
fn mix_seed(run_seed: u64, counter: u64) -> u64 {
    let mut z = run_seed.wrapping_add(counter.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Total ever-infected, averaged over the batch's epidemics. **Maximized.**
pub struct EpiSpread {
    scorer: EpidemicScorer,
}

impl EpiSpread {
    /// Build the objective from its epidemic sampling parameters.
    pub fn new(params: SirSampleParams, run_seed: u64) -> Self {
        Self {
            scorer: EpidemicScorer::new(params, run_seed),
        }
    }
}

impl Fitness for EpiSpread {
    fn evaluate(&self, graph: &Graph) -> f64 {
        self.scorer
            .mean_batch(slice::from_ref(graph), |epidemic| epidemic.spread as f64)[0]
    }

    fn evaluate_population(&self, graphs: &[Graph]) -> Vec<f64> {
        self.scorer
            .mean_batch(graphs, |epidemic| epidemic.spread as f64)
    }

    fn direction(&self) -> Direction {
        Direction::Maximize
    }
}

/// Timesteps to burn out, averaged over the epidemics. **Maximized.**
///
/// `length` counts the final burnout step, so a lone patient zero reads 1, not
/// 0 (§5.2).
pub struct EpiLength {
    scorer: EpidemicScorer,
}

impl EpiLength {
    /// Build the objective from its epidemic sampling parameters.
    pub fn new(params: SirSampleParams, run_seed: u64) -> Self {
        Self {
            scorer: EpidemicScorer::new(params, run_seed),
        }
    }
}

impl Fitness for EpiLength {
    fn evaluate(&self, graph: &Graph) -> f64 {
        self.scorer
            .mean_batch(slice::from_ref(graph), |epidemic| epidemic.length as f64)[0]
    }

    fn evaluate_population(&self, graphs: &[Graph]) -> Vec<f64> {
        self.scorer
            .mean_batch(graphs, |epidemic| epidemic.length as f64)
    }

    fn direction(&self) -> Direction {
        Direction::Maximize
    }
}

/// RMSE between the epidemic profile and a target profile. **Minimized.**
///
/// The target is newly-infected counts, one per timestep. An epidemic's
/// profile starts with patient zero and ends with a terminating zero (§5.2),
/// so a target captured from older output will not line up element for element.
pub struct EpiProfMatch {
    scorer: EpidemicScorer,
    target: Vec<f64>,
}

impl EpiProfMatch {
    /// Build the objective from its sampling parameters and a target profile.
    ///
    /// # Errors
    ///
    /// If `target` is empty or holds a non-finite value — either would put a
    /// `NaN` into every score, which [`Fitness`] forbids. (`&'static str` is a
    /// fixed string literal, used here as a lightweight error type.)
    pub fn new(
        params: SirSampleParams,
        run_seed: u64,
        target: Vec<f64>,
    ) -> Result<Self, &'static str> {
        if target.is_empty() {
            return Err("epi_prof_match target profile must not be empty");
        }
        if !target.iter().all(|value| value.is_finite()) {
            return Err("epi_prof_match target profile must be finite");
        }
        Ok(Self {
            scorer: EpidemicScorer::new(params, run_seed),
            target,
        })
    }

    /// RMSE of one epidemic against the target — this objective's reading.
    ///
    /// A method rather than an inline closure only because it is too long to
    /// read twice; the other two objectives inline theirs.
    ///
    /// **The target sets the comparison, not the epidemic** (§5.2, matching
    /// `legacy/main.cpp:545-553`), so the scoring is asymmetric: an epidemic
    /// that ends early is penalised for the whole remaining target, while one
    /// that outlasts the target is not penalised at all. This rewards matching
    /// *or exceeding* the tail. See `decisions.md` 2026-08-04 18:13.
    fn rmse(&self, epidemic: &SirRun) -> f64 {
        let mut total = 0.0;

        for (step, wanted) in self.target.iter().enumerate() {
            // Past the end of the epidemic nobody was newly infected, so a
            // missing step counts as zero. `.get` returns None instead of
            // panicking there, and `.unwrap_or(0)` supplies that zero.
            let actual = epidemic.profile.get(step).copied().unwrap_or(0) as f64;
            total += (actual - wanted).powi(2);
        }

        (total / self.target.len() as f64).sqrt()
    }
}

impl Fitness for EpiProfMatch {
    fn evaluate(&self, graph: &Graph) -> f64 {
        self.scorer
            .mean_batch(slice::from_ref(graph), |epidemic| self.rmse(epidemic))[0]
    }

    fn evaluate_population(&self, graphs: &[Graph]) -> Vec<f64> {
        self.scorer
            .mean_batch(graphs, |epidemic| self.rmse(epidemic))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sir::SirParams;

    /// A path `0 - 1 - ... - (n-1)`, every edge at multiplicity 1.
    fn path_graph(num_nodes: usize) -> Graph {
        let mut graph = Graph::new(num_nodes, 1);
        // saturating_sub: clamps at 0 instead of underflowing when num_nodes
        // is 0 or 1 (usize can't go negative).
        for node in 0..num_nodes.saturating_sub(1) {
            graph.set_edge(node, node + 1, 1);
        }
        graph
    }

    /// Rate 1.0 from a pinned patient zero, so every epidemic is identical and
    /// no test depends on the seed.
    fn certain_batch(num_epidemics: usize) -> SirSampleParams {
        SirSampleParams {
            epidemic: SirParams {
                infection_rate: 1.0,
                patient_zero: Some(0),
            },
            num_epidemics,
            min_epidemic_length: 1,
            max_epidemic_retries: 1,
        }
    }

    /// A rate whose epidemics genuinely vary with the seed. The seeding tests
    /// need that — under `certain_batch` every epidemic is identical, so they
    /// would pass no matter how the seeding worked.
    ///
    /// 0.15 on `complete_graph(12)` is picked from measurement, not taste:
    /// higher and every epidemic reaches all 12 nodes, lower and the average
    /// over `num_epidemics` keeps landing on the same value.
    fn chancy_batch(num_epidemics: usize) -> SirSampleParams {
        SirSampleParams {
            epidemic: SirParams {
                infection_rate: 0.15,
                patient_zero: Some(0),
            },
            num_epidemics,
            min_epidemic_length: 1,
            max_epidemic_retries: 1,
        }
    }

    fn profile_match(target: Vec<f64>) -> EpiProfMatch {
        EpiProfMatch::new(certain_batch(1), 0, target).expect("valid target")
    }

    #[test]
    fn epi_spread_reads_total_ever_infected() {
        let objective = EpiSpread::new(certain_batch(3), 2026);

        assert_eq!(
            objective.evaluate(&path_graph(6)),
            6.0,
            "every node of the path is reached at rate 1.0",
        );
        assert_eq!(objective.direction(), Direction::Maximize);
    }

    #[test]
    fn epi_length_reads_timesteps_including_the_burnout_step() {
        let objective = EpiLength::new(certain_batch(3), 2026);

        assert_eq!(
            objective.evaluate(&path_graph(6)),
            6.0,
            "one step per edge, plus the burnout step (spec 5.2)",
        );
        assert_eq!(
            objective.evaluate(&Graph::new(4, 1)),
            1.0,
            "a lone patient zero still occupies the burnout step",
        );
        assert_eq!(objective.direction(), Direction::Maximize);
    }

    #[test]
    fn epi_prof_match_minimizes_and_scores_an_exact_match_at_zero() {
        let objective = profile_match(vec![1.0, 1.0, 1.0, 0.0]);
        let epidemic = SirRun {
            length: 3,
            spread: 3,
            profile: vec![1, 1, 1, 0],
        };

        assert_eq!(objective.rmse(&epidemic), 0.0);
        assert_eq!(objective.direction(), Direction::Minimize);
    }

    /// The missing steps count as zero newly infected, not as absent.
    #[test]
    fn an_epidemic_shorter_than_the_target_is_penalised_for_the_remainder() {
        let objective = profile_match(vec![1.0, 2.0, 3.0, 4.0]);
        let epidemic = SirRun {
            length: 1,
            spread: 3,
            profile: vec![1, 2],
        };

        // Squared error 0 + 0 + 9 + 16 = 25, over 4 steps, square-rooted.
        assert_eq!(objective.rmse(&epidemic), 2.5);
    }

    /// The deliberate asymmetry: overshoot is free — see `rmse`.
    #[test]
    fn an_epidemic_longer_than_the_target_is_not_penalised_for_the_surplus() {
        let objective = profile_match(vec![1.0, 2.0]);
        let short = SirRun {
            length: 1,
            spread: 3,
            profile: vec![1, 2],
        };
        let long = SirRun {
            length: 3,
            spread: 17,
            profile: vec![1, 2, 5, 9, 0],
        };

        assert_eq!(objective.rmse(&short), 0.0);
        assert_eq!(
            objective.rmse(&long),
            objective.rmse(&short),
            "the surplus beyond the target is ignored entirely",
        );
    }

    #[test]
    fn the_divisor_is_the_target_length_not_the_overlap() {
        // One matching step out of four. Were the divisor the overlap (2), the
        // score would be sqrt(9/2); it must be sqrt(9/4).
        let objective = profile_match(vec![1.0, 3.0, 0.0, 0.0]);
        let epidemic = SirRun {
            length: 1,
            spread: 1,
            profile: vec![1, 0],
        };

        assert_eq!(objective.rmse(&epidemic), 1.5);
    }

    #[test]
    fn an_unusable_target_profile_is_rejected_at_construction() {
        assert!(
            EpiProfMatch::new(certain_batch(1), 0, Vec::new()).is_err(),
            "an empty target is the divisor of every RMSE, so it yields NaN",
        );
        assert!(EpiProfMatch::new(certain_batch(1), 0, vec![1.0, f64::NAN]).is_err());
        assert!(EpiProfMatch::new(certain_batch(1), 0, vec![1.0, f64::INFINITY]).is_err());
        assert!(EpiProfMatch::new(certain_batch(1), 0, vec![1.0, 2.0]).is_ok());
    }

    /// More epidemics must not change a deterministic reading.
    #[test]
    fn the_batch_mean_averages_the_epidemics() {
        let graph = path_graph(6);

        for num_epidemics in [1, 2, 7] {
            assert_eq!(
                EpiSpread::new(certain_batch(num_epidemics), 5).evaluate(&graph),
                6.0,
                "{num_epidemics} identical epidemics average to the same reading",
            );
        }
    }

    #[test]
    fn minimizing_leaves_the_oriented_value_equal_to_the_original() {
        assert_eq!(Direction::Minimize.orient(2.5), 2.5);
        assert_eq!(Direction::Minimize.orient(-2.5), -2.5);
        assert_eq!(Direction::Minimize.orient(f64::INFINITY), f64::INFINITY);
    }

    #[test]
    fn maximizing_makes_the_oriented_value_the_negated_original() {
        assert_eq!(Direction::Maximize.orient(2.5), -2.5);
        assert_eq!(Direction::Maximize.orient(f64::INFINITY), f64::NEG_INFINITY);
        // A better original must give a lower oriented value.
        assert!(Direction::Maximize.orient(9.0) < Direction::Maximize.orient(1.0));
    }

    #[test]
    fn orienting_an_oriented_value_gives_back_the_original() {
        for direction in [Direction::Minimize, Direction::Maximize] {
            for value in [0.0, 1.5, -7.25, f64::MAX, f64::INFINITY] {
                assert_eq!(direction.orient(direction.orient(value)), value);
            }
        }
    }

    #[test]
    fn the_default_direction_is_minimize() {
        struct Bare;
        impl Fitness for Bare {
            fn evaluate(&self, _graph: &Graph) -> f64 {
                0.0
            }
        }
        assert_eq!(Bare.direction(), Direction::Minimize);
    }

    #[test]
    #[should_panic(expected = "returned NaN")]
    fn orient_rejects_nan_when_minimizing() {
        Direction::Minimize.orient(f64::NAN);
    }

    #[test]
    #[should_panic(expected = "returned NaN")]
    fn orient_rejects_nan_when_maximizing() {
        Direction::Maximize.orient(f64::NAN);
    }

    /// Records why `orient` asserts rather than trusting the ordering: a `NaN`
    /// that got through under `Maximize` would sort **best**, not worst.
    #[test]
    fn an_unchecked_negated_nan_would_have_sorted_best() {
        let mut scores = vec![-f64::NAN, -100.0, 0.0, 100.0];
        // total_cmp: a total ordering for floats, unlike plain < where NaN is
        // unordered — needed so NaN actually sorts (first, here) instead of
        // being silently skipped.
        scores.sort_by(|a, b| a.total_cmp(b));
        assert!(
            scores[0].is_nan(),
            "negated NaN should sort first, got {scores:?}",
        );
    }

    /// Every pair of nodes joined, at multiplicity 1.
    fn complete_graph(num_nodes: usize) -> Graph {
        let mut graph = Graph::new(num_nodes, 1);
        for from in 0..num_nodes {
            for to in (from + 1)..num_nodes {
                graph.set_edge(from, to, 1);
            }
        }
        graph
    }

    /// A batch of `count` identical graphs.
    ///
    /// Complete rather than a path: at rate 0.5 a path's spread barely varies,
    /// and averaging over the epidemics quantizes two different batches onto
    /// the same score often enough to make a difference test useless.
    fn identical_batch(count: usize) -> Vec<Graph> {
        let mut graphs = Vec::with_capacity(count);
        for _ in 0..count {
            graphs.push(complete_graph(12));
        }
        graphs
    }

    #[test]
    fn one_batch_ticks_the_counter_once_however_many_graphs_it_holds() {
        let objective = EpiSpread::new(chancy_batch(3), 2026);

        objective.evaluate_population(&identical_batch(6));

        assert_eq!(
            objective.scorer.batches_scored.load(Ordering::Relaxed),
            1,
            "the counter must advance per batch, not per graph",
        );
    }

    #[test]
    fn scoring_one_graph_ticks_the_counter_once_like_any_other_batch() {
        let objective = EpiSpread::new(chancy_batch(3), 2026);

        objective.evaluate(&complete_graph(12));

        assert_eq!(
            objective.scorer.batches_scored.load(Ordering::Relaxed),
            1,
            "a single graph is a batch of one, not a special case",
        );
    }

    /// `evaluate` and `evaluate_population` must read the same thing off an
    /// epidemic. Each objective writes that reading twice, once per entry
    /// point, so this is what catches the two drifting apart.
    #[test]
    fn both_entry_points_use_the_same_reading() {
        let graph = path_graph(6);

        // certain_batch is deterministic, so the two differing seeds cannot
        // account for any difference in the scores.
        let spread = EpiSpread::new(certain_batch(2), 7);
        assert_eq!(
            spread.evaluate(&graph),
            spread.evaluate_population(slice::from_ref(&graph))[0],
        );

        let length = EpiLength::new(certain_batch(2), 7);
        assert_eq!(
            length.evaluate(&graph),
            length.evaluate_population(slice::from_ref(&graph))[0],
        );

        let profile = profile_match(vec![1.0, 1.0, 1.0]);
        assert_eq!(
            profile.evaluate(&graph),
            profile.evaluate_population(slice::from_ref(&graph))[0],
        );
    }

    #[test]
    fn every_graph_in_one_batch_faces_the_same_epidemics() {
        let objective = EpiSpread::new(chancy_batch(3), 2026);

        let scores = objective.evaluate_population(&identical_batch(6));

        for (i, score) in scores.iter().enumerate() {
            assert_eq!(
                *score, scores[0],
                "graph {i} of the batch drew different dice from graph 0",
            );
        }
    }

    #[test]
    fn consecutive_batches_face_different_epidemics() {
        let objective = EpiSpread::new(chancy_batch(3), 2026);
        let population = identical_batch(6);

        let first = objective.evaluate_population(&population);
        let second = objective.evaluate_population(&population);

        assert_ne!(
            first, second,
            "the dice never changed, so the run would optimize against one \
             frozen sample of the disease",
        );
    }

    /// The first `count` batch seeds a fresh scorer at `run_seed` hands out.
    fn first_batch_seeds(run_seed: u64, count: usize) -> Vec<u64> {
        let scorer = EpidemicScorer::new(certain_batch(1), run_seed);

        let mut seeds = Vec::with_capacity(count);
        for _ in 0..count {
            seeds.push(scorer.next_batch_seed());
        }
        seeds
    }

    #[test]
    fn one_run_seed_always_produces_the_same_batch_seed_sequence() {
        assert_eq!(
            first_batch_seeds(2026, 4),
            first_batch_seeds(2026, 4),
            "the same run seed must reproduce a run exactly",
        );
    }

    #[test]
    fn consecutive_batches_get_different_seeds() {
        let seeds = first_batch_seeds(2026, 4);

        for (i, seed) in seeds.iter().enumerate() {
            for (j, other) in seeds.iter().enumerate().skip(i + 1) {
                assert_ne!(
                    seed, other,
                    "batches {i} and {j} share a seed, so the run would \
                     optimize against one frozen sample of the disease",
                );
            }
        }
    }

    /// The property that rules out `run_seed ^ counter`: under xor, run seed
    /// `n`'s batch 1 is run seed `n + 1`'s batch 0, so two replicates replay
    /// each other's epidemics one batch out of step.
    #[test]
    fn neighbouring_run_seeds_share_no_batch_seed() {
        let mine = first_batch_seeds(2026, 4);
        let neighbour = first_batch_seeds(2027, 4);

        for (i, seed) in mine.iter().enumerate() {
            assert!(
                !neighbour.contains(seed),
                "batch {i} of run seed 2026 also appears in run seed 2027",
            );
        }
    }
}
