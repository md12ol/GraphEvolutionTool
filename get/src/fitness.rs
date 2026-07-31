use rayon::prelude::*;

use crate::graph::Graph;

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

/// Native fitness driven by an epidemic simulation over the expressed graph.
///
/// The model is SIR with a one-timestep infectious period. When a susceptible
/// node is infected during a step, it spends the *following* step in the
/// infected state — that is when it can transmit to each of its still-
/// susceptible neighbors, with probability [`SirFitness::infection_rate`] per
/// edge — and it then moves to recovered/removed and never infects again. So
/// each node is infectious for exactly one step, one step after it is infected.
/// A single node seeds the outbreak, which runs until no infected nodes remain.
/// Quantities such as epidemic length or total infected are measured from that
/// completed run.
pub struct SirFitness {
    /// Per-contact probability of transmission along an edge in one timestep.
    pub infection_rate: f64,
    /// Which node seeds the outbreak; `None` selects one at random.
    pub patient_zero: Option<usize>,
    /// Seed for the stochastic simulation, so scoring is reproducible.
    pub seed: u64,
}

impl SirFitness {
    /// Build an SIR fitness from its simulation parameters.
    pub fn new(infection_rate: f64, patient_zero: Option<usize>, seed: u64) -> Self {
        let _ = (infection_rate, patient_zero, seed);
        todo!("store SIR simulation parameters")
    }
}

impl Fitness for SirFitness {
    fn evaluate(&self, graph: &Graph) -> f64 {
        let _ = graph;
        todo!("run the SIR epidemic to completion over `graph` and score it")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
