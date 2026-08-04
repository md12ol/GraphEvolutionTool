//! Objectives the genetic algorithm optimizes, and the direction gate.
//!
//! # Adding your own objective
//!
//! 1. Implement [`Fitness`] on your type — [`Fitness::evaluate`] is the only
//!    required method; add [`Fitness::direction`] if bigger is better.
//! 2. Add a variant to `FitnessConfig` in [`crate::config`].
//! 3. Add the matching arm in `GraphEvolver::run`.
//!
//! The three SIR objectives below are worked examples. If yours is
//! epidemic-based, build it on [`EpidemicScorer`] rather than calling the
//! simulator directly — it owns the seeding, which is easy to get subtly wrong
//! and gives plausible-looking numbers when you do.

use rayon::prelude::*;

use crate::graph::Graph;
use crate::sir::{SirBatchParams, SirRun, batch_epidemics};

/// Whether an objective is better when its value is smaller or larger.
///
/// The engine always minimizes. An objective returns its own value from
/// [`Fitness::evaluate`] and declares its direction here; [`Direction::orient`]
/// puts that value into the order the engine works in.
///
/// Declaring the direction rather than making each objective negate its own
/// output is deliberate. If `evaluate` returned an already-negated value *and*
/// the trait declared `Maximize`, the two could silently disagree — and a run
/// that optimizes backwards is indistinguishable from one that simply is not
/// converging.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// Smaller is better, as for an error or a distance. The default.
    Minimize,
    /// Larger is better.
    Maximize,
}

impl Direction {
    /// Convert between an objective's own value and the order the engine works
    /// in, where lower is always better.
    ///
    /// Negation is its own inverse, so one function maps both ways: in, to
    /// compare individuals; out again, to log and report in the objective's
    /// units and sign.
    ///
    /// # Panics
    ///
    /// If `value` is `NaN`. That is deliberate. Ordering `NaN` as merely worst
    /// would hide a bug in the objective, and letting it through is dangerous
    /// under [`Direction::Maximize`]: `-NaN` sorts below `-inf`, so it would win
    /// every tournament it entered and leave a run that looks converged.
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

/// An objective the genetic algorithm optimizes over expressed graphs.
///
/// [`Fitness::evaluate`] returns a score in whatever units suit the objective;
/// [`Fitness::direction`] says whether bigger or smaller is better. The engine
/// minimizes, converting once via [`Direction::orient`], so logs and the value
/// handed back to Python stay in the objective's own units and sign.
///
/// # Implementors must not return `NaN`
///
/// Enforced, not merely documented: every value entering the engine passes
/// through [`Direction::orient`], which panics on `NaN`. Guard the arithmetic
/// that produces one — division by a possibly-zero count, `0.0 / 0.0`,
/// `inf - inf`.
///
/// The `Send + Sync` bound lets [`Fitness::evaluate_population`] score a whole
/// generation across rayon worker threads.
///
/// # These methods are implemented here and called by the engine in exactly one place
///
/// **Nothing in the engine calls [`Fitness::evaluate`] or
/// [`Fitness::evaluate_population`] directly.** Both are reached only through
/// `common::express_and_score`, which is the sole path from a population to a set
/// of fitnesses. Implement them; do not call them.
///
/// A direct call compiles, returns plausible numbers, and is wrong in two ways
/// that never announce themselves — it skips the [`Direction`] conversion, so
/// under [`Direction::Maximize`] every comparison runs backwards, and it skips
/// the `NaN` rejection that [`Direction::orient`] performs. Spec §5.1.
pub trait Fitness: Send + Sync {
    /// Score a single expressed graph, in the objective's own units.
    ///
    /// Must not return `NaN`; the engine panics if it does.
    ///
    /// Implemented by an objective, called by `common::express_and_score` — see
    /// the note on the trait. The engine does not call this.
    fn evaluate(&self, graph: &Graph) -> f64;

    /// Whether larger or smaller scores are better.
    ///
    /// Defaults to [`Direction::Minimize`], so an error or distance objective
    /// needs to say nothing.
    fn direction(&self) -> Direction {
        Direction::Minimize
    }

    /// Score an entire generation of expressed graphs.
    ///
    /// The default fans [`Fitness::evaluate`] out across rayon, which is ideal
    /// for native Rust objectives. A Python-backed adapter overrides this to
    /// acquire the GIL once per generation and vectorize the whole batch,
    /// instead of paying the FFI/GIL cost once per individual.
    ///
    /// Implemented or overridden by an objective, called by
    /// `common::express_and_score` — see the note on the trait. The engine does
    /// not call this. Note the scores it returns are in the objective's own
    /// units and are **not** yet oriented; `express_and_score` converts them.
    fn evaluate_population(&self, graphs: &[Graph]) -> Vec<f64> {
        graphs
            .par_iter()
            .map(|graph| self.evaluate(graph))
            .collect()
    }
}

/// Runs the epidemics that every SIR objective scores.
///
/// One epidemic is the expensive part and all three objectives want the same
/// one, so this runs the batch and each objective supplies only the reading
/// (spec §5.2). A new epidemic-based objective needs nothing but a reading —
/// see [`EpiSpread`] for the smallest possible example.
pub struct EpidemicScorer {
    params: SirBatchParams,
    run_seed: u64,
}

impl EpidemicScorer {
    /// Build a scorer for one run's epidemics.
    ///
    /// `run_seed` comes from the single master seed passed to `run`; there is
    /// deliberately no separate fitness seed (§5.2).
    pub fn new(params: SirBatchParams, run_seed: u64) -> Self {
        Self { params, run_seed }
    }

    /// The seed every graph in the current batch is simulated from.
    ///
    /// **Stub — issue #18 replaces this method.** It should be the run seed
    /// plus a counter that ticks once per batch, so each batch draws fresh
    /// dice. Sharing one seed *within* a batch is already right; what is
    /// missing is changing it *between* batches, so a run currently optimizes
    /// against one frozen sample of the disease. See `hotfixes.md`.
    fn batch_seed(&self) -> u64 {
        self.run_seed
    }

    /// One evaluation's epidemics over `graph`.
    pub fn runs(&self, graph: &Graph) -> Vec<SirRun> {
        batch_epidemics(graph, &self.params, self.batch_seed())
    }

    /// Average `read` across this evaluation's epidemics.
    ///
    /// Averaging matters: a single SIR draw is noisy enough that selection
    /// would chase the dice instead of the graph. The division is safe because
    /// [`batch_epidemics`] rejects an empty batch.
    pub fn mean(&self, graph: &Graph, read: impl Fn(&SirRun) -> f64) -> f64 {
        let runs = self.runs(graph);

        let mut total = 0.0;
        for run in &runs {
            total += read(run);
        }
        total / runs.len() as f64
    }
}

/// Total ever-infected, averaged over the evaluation's epidemics. **Maximized.**
pub struct EpiSpread {
    scorer: EpidemicScorer,
}

impl EpiSpread {
    /// Build the objective from its epidemic sampling parameters.
    pub fn new(params: SirBatchParams, run_seed: u64) -> Self {
        Self {
            scorer: EpidemicScorer::new(params, run_seed),
        }
    }
}

impl Fitness for EpiSpread {
    fn evaluate(&self, graph: &Graph) -> f64 {
        self.scorer.mean(graph, |run| run.spread as f64)
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
    pub fn new(params: SirBatchParams, run_seed: u64) -> Self {
        Self {
            scorer: EpidemicScorer::new(params, run_seed),
        }
    }
}

impl Fitness for EpiLength {
    fn evaluate(&self, graph: &Graph) -> f64 {
        self.scorer.mean(graph, |run| run.length as f64)
    }

    fn direction(&self) -> Direction {
        Direction::Maximize
    }
}

/// RMSE between the epidemic profile and a target profile. **Minimized.**
///
/// The target is a vector of newly-infected counts, one per timestep. A run's
/// profile starts with patient zero and ends with a terminating zero (§5.2), so
/// a target captured from older output will not line up element for element.
pub struct EpiProfMatch {
    scorer: EpidemicScorer,
    target: Vec<f64>,
}

impl EpiProfMatch {
    /// Build the objective from its sampling parameters and a target profile.
    ///
    /// # Errors
    ///
    /// If `target` is empty or holds a non-finite value. Either would put a
    /// `NaN` into every score, which the [`Fitness`] contract forbids.
    pub fn new(
        params: SirBatchParams,
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

    /// RMSE of one epidemic against the target.
    ///
    /// **The target sets the comparison, not the run** (§5.2, matching
    /// `legacy/main.cpp:545-553`). The consequence is asymmetric and worth
    /// knowing when reading a score: a run that ends early is penalised for the
    /// whole remaining target, while a run that outlasts the target is not
    /// penalised at all. So this rewards *matching or exceeding* the target's
    /// tail, not matching it exactly. See `decisions.md` 2026-08-04 18:13.
    fn rmse(&self, run: &SirRun) -> f64 {
        let mut total = 0.0;

        for (step, wanted) in self.target.iter().enumerate() {
            // Past the end of the run, nobody was newly infected: the epidemic
            // had already finished. Any surplus run beyond the target is never
            // visited, so overshoot costs nothing.
            let actual = run.profile.get(step).copied().unwrap_or(0) as f64;
            total += (actual - wanted).powi(2);
        }

        (total / self.target.len() as f64).sqrt()
    }
}

impl Fitness for EpiProfMatch {
    fn evaluate(&self, graph: &Graph) -> f64 {
        self.scorer.mean(graph, |run| self.rmse(run))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sir::SirParams;

    /// A path `0 - 1 - ... - (n-1)`, every edge at multiplicity 1.
    fn path_graph(num_nodes: usize) -> Graph {
        let mut graph = Graph::new(num_nodes, 1);
        for node in 0..num_nodes.saturating_sub(1) {
            graph.set_edge(node, node + 1, 1);
        }
        graph
    }

    /// Rate 1.0 from a pinned patient zero, so every epidemic is identical and
    /// no test depends on the seed.
    fn certain_batch(num_epidemics: usize) -> SirBatchParams {
        SirBatchParams {
            epidemic: SirParams {
                infection_rate: 1.0,
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
        let run = SirRun {
            length: 3,
            spread: 3,
            profile: vec![1, 1, 1, 0],
        };

        assert_eq!(objective.rmse(&run), 0.0);
        assert_eq!(objective.direction(), Direction::Minimize);
    }

    /// The missing days count as zero newly infected, not as absent.
    #[test]
    fn a_run_shorter_than_the_target_is_penalised_for_the_remainder() {
        let objective = profile_match(vec![1.0, 2.0, 3.0, 4.0]);
        let run = SirRun {
            length: 1,
            spread: 3,
            profile: vec![1, 2],
        };

        // Squared error 0 + 0 + 9 + 16 = 25, over 4 steps, square-rooted.
        assert_eq!(objective.rmse(&run), 2.5);
    }

    /// The deliberate asymmetry: overshoot is free — see `rmse`.
    #[test]
    fn a_run_longer_than_the_target_is_not_penalised_for_the_surplus() {
        let objective = profile_match(vec![1.0, 2.0]);
        let short_run = SirRun {
            length: 1,
            spread: 3,
            profile: vec![1, 2],
        };
        let long_run = SirRun {
            length: 3,
            spread: 17,
            profile: vec![1, 2, 5, 9, 0],
        };

        assert_eq!(objective.rmse(&short_run), 0.0);
        assert_eq!(
            objective.rmse(&long_run),
            objective.rmse(&short_run),
            "the surplus beyond the target is ignored entirely",
        );
    }

    #[test]
    fn the_divisor_is_the_target_length_not_the_overlap() {
        // One matching step out of four. Were the divisor the overlap (2), the
        // score would be sqrt(9/2); it must be sqrt(9/4).
        let objective = profile_match(vec![1.0, 3.0, 0.0, 0.0]);
        let run = SirRun {
            length: 1,
            spread: 1,
            profile: vec![1, 0],
        };

        assert_eq!(objective.rmse(&run), 1.5);
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
    fn minimize_leaves_a_score_untouched() {
        assert_eq!(Direction::Minimize.orient(2.5), 2.5);
        assert_eq!(Direction::Minimize.orient(-2.5), -2.5);
        assert_eq!(Direction::Minimize.orient(f64::INFINITY), f64::INFINITY);
    }

    #[test]
    fn maximize_flips_a_score_so_the_engine_can_minimize_it() {
        assert_eq!(Direction::Maximize.orient(2.5), -2.5);
        assert_eq!(Direction::Maximize.orient(f64::INFINITY), f64::NEG_INFINITY);
        // A better score must come out lower.
        assert!(Direction::Maximize.orient(9.0) < Direction::Maximize.orient(1.0));
    }

    #[test]
    fn orient_is_its_own_inverse_so_reporting_round_trips() {
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
        scores.sort_by(|a, b| a.total_cmp(b));
        assert!(
            scores[0].is_nan(),
            "negated NaN should sort first, got {scores:?}",
        );
    }
}
