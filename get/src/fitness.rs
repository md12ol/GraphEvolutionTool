//! Objectives the genetic algorithm optimizes, and the direction gate.
//!
//! # Adding your own objective
//!
//! Three steps, and the three SIR objectives below are worked examples of all
//! of them:
//!
//! 1. Implement [`Fitness`] on your own type — one required method,
//!    [`Fitness::evaluate`], plus [`Fitness::direction`] if bigger is better.
//! 2. Add a variant to `FitnessConfig` in [`crate::config`], so a run can name
//!    it in `config.toml`.
//! 3. Add the matching arm to the objective match in `GraphEvolver::run`, which
//!    boxes it as `Box<dyn Fitness>` before the evolver is instantiated.
//!
//! **If your objective is epidemic-based, do not re-implement the sampling.**
//! Call [`crate::sir::batch_epidemics`], which owns the short-epidemic re-roll
//! and the position-indexed seeding that keeps common random numbers intact.
//! Both fail *silently* when reimplemented slightly wrong — a broken CRN scheme
//! produces plausible numbers and a run that selects on dice rather than
//! structure. [`EpidemicScorer`] wraps it with the batch mean, so a new reading
//! of the same epidemics is a few lines.

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
pub trait Fitness: Send + Sync {
    /// Score a single expressed graph, in the objective's own units.
    ///
    /// Must not return `NaN`; the engine panics if it does.
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
    fn evaluate_population(&self, graphs: &[Graph]) -> Vec<f64> {
        graphs
            .par_iter()
            .map(|graph| self.evaluate(graph))
            .collect()
    }
}

/// The epidemic sampling every SIR objective shares.
///
/// The expensive part of an evaluation is the epidemic, and all three
/// objectives want the same one, so this owns *running* the batch and each
/// objective supplies only the reading (spec §5.2). A fourth objective — peak
/// height, time to peak, attack rate within a component — is then a closure
/// over [`EpidemicScorer::mean`] and nothing else.
///
/// The model itself lives in [`crate::sir`], which owns the description of it.
/// Do not restate it here; two copies will drift.
pub struct EpidemicScorer {
    params: SirBatchParams,
    run_seed: u64,
}

impl EpidemicScorer {
    /// Build a scorer for one run's epidemics.
    ///
    /// `run_seed` is derived from the single master seed passed to `run`, never
    /// configured separately — a fitness seed of its own would leave replicate
    /// runs at different evolution seeds facing *identical* epidemic draws,
    /// which is exactly what replicates exist to vary (§5.2).
    pub fn new(params: SirBatchParams, run_seed: u64) -> Self {
        Self { params, run_seed }
    }

    /// The seed every graph in the current batch is simulated from.
    ///
    /// **Stub — issue #18 replaces this.** The design is a run seed plus an
    /// atomic evaluation counter, incremented once per batch, so every graph in
    /// one batch faces the same dice and the next batch faces different ones
    /// (§5.2). Only the second half is missing: within a batch this is already
    /// correct, because it does not vary with the graph. What it does not yet
    /// do is *change between batches*, so a run currently optimizes against one
    /// frozen sample of the disease. Tracked in `hotfixes.md`.
    fn batch_seed(&self) -> u64 {
        self.run_seed
    }

    /// One evaluation's epidemics over `graph`.
    pub fn runs(&self, graph: &Graph) -> Vec<SirRun> {
        batch_epidemics(graph, &self.params, self.batch_seed())
    }

    /// Mean of `read` across this evaluation's epidemics.
    ///
    /// Averaging is not a tuning nicety: a single SIR draw is noisy enough that
    /// selection would chase the dice instead of graph structure.
    ///
    /// Cannot divide by zero — [`crate::sir::batch_epidemics`] rejects a batch
    /// of no epidemics, which is what keeps the `NaN` the [`Fitness`] contract
    /// forbids out of the arithmetic.
    pub fn mean(&self, graph: &Graph, read: impl Fn(&SirRun) -> f64) -> f64 {
        let runs = self.runs(graph);
        runs.iter().map(read).sum::<f64>() / runs.len() as f64
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

/// Timesteps to burn out, averaged over the evaluation's epidemics.
/// **Maximized.**
///
/// `length` includes the final burnout step in which the last infectious node
/// recovers without transmitting, per the §5.2 amendment of 2026-08-04 — so a
/// lone patient zero reads 1, not 0.
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

/// RMSE between the epidemic profile and a user-supplied target. **Minimized.**
///
/// The target is a vector of newly-infected counts. Note that a run's profile
/// has `profile[0] == 1` for patient zero and carries a **terminating zero**
/// (§5.2, amended 2026-08-04), so a target captured from older output will not
/// line up element for element.
pub struct EpiProfMatch {
    scorer: EpidemicScorer,
    target: Vec<f64>,
}

impl EpiProfMatch {
    /// Build the objective from its sampling parameters and a target profile.
    ///
    /// # Errors
    ///
    /// If `target` is empty — it is the divisor of every RMSE, so an empty one
    /// yields the `NaN` the [`Fitness`] contract forbids — or if any element is
    /// not finite, which would propagate into every score.
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
    /// **The target fixes the comparison, not the run** (spec §5.2, matching
    /// `legacy/main.cpp:545-553`): iterate the target's indices, treat a missing
    /// run value as `0` because the epidemic had ended and nobody was newly
    /// infected that day, ignore any surplus where the run outlasts the target,
    /// and divide by the target's length always.
    ///
    /// **The asymmetry is deliberate and worth knowing when reading a score.** A
    /// run that burns out early is penalised by the entire remaining target; a
    /// run that outlasts it is not penalised for the overshoot at all. So this
    /// objective rewards *matching or exceeding* the target's tail, not matching
    /// it exactly. Inherited from the C++ for comparability — see
    /// `decisions.md` 2026-08-04 18:13 for the alternatives that were rejected.
    fn rmse(&self, run: &SirRun) -> f64 {
        let total: f64 = self
            .target
            .iter()
            .enumerate()
            .map(|(step, wanted)| {
                let actual = run.profile.get(step).copied().unwrap_or(0) as f64;
                (actual - wanted).powi(2)
            })
            .sum();
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

    /// Sampling that makes every epidemic identical and certain: rate 1.0 from a
    /// pinned patient zero, so the batch mean is the single deterministic
    /// reading and no test depends on the seed.
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

    /// A run that burns out early is penalised by the **whole** remaining
    /// target — the missing days count as zero newly infected, not as absent.
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

    /// The deliberate asymmetry: overshoot is free. This objective rewards
    /// matching *or exceeding* the target's tail — see `rmse`'s doc comment.
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

    /// Averaging is the objective's job, not the simulator's, so it is worth
    /// pinning that more epidemics do not change a deterministic reading.
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
