use rayon::prelude::*;

use crate::graph::Graph;

/// Whether an objective is better when its value is smaller or larger.
///
/// The engine only ever *minimizes*. An objective declares its natural
/// direction here and returns its natural value from [`Fitness::evaluate`];
/// [`Direction::orient`] converts that into the internal cost the engine
/// compares, exactly once, in [`crate::evolver::common::evaluate`].
///
/// Declaring the direction rather than making each objective pre-negate its own
/// output is deliberate. If `evaluate` returned an already-negated value *and*
/// the trait declared `Maximize`, the two could silently disagree — and a run
/// that optimizes backwards is indistinguishable from a run that simply is not
/// converging. Here there is only one place the sign is applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// Smaller values are better — a cost or error. The default.
    Minimize,
    /// Larger values are better.
    Maximize,
}

impl Direction {
    /// Convert between an objective's natural value and the engine's internal
    /// cost. Negation is its own inverse, so this maps both ways: pass a raw
    /// fitness to get a cost, pass a cost to get the fitness back for logging
    /// and for reporting to Python.
    ///
    /// Does not check for `NaN` — use [`Direction::to_cost`] on the way in,
    /// which does. This stays unchecked so the reverse conversion, on values
    /// the engine already validated, costs nothing.
    pub fn orient(self, value: f64) -> f64 {
        match self {
            Direction::Minimize => value,
            Direction::Maximize => -value,
        }
    }

    /// Convert a freshly computed objective value into the engine's internal
    /// cost, **rejecting `NaN`**.
    ///
    /// This is the one gate every fitness value passes through on its way into
    /// the engine, and the only enforcement of the trait's no-`NaN` contract.
    ///
    /// # Panics
    ///
    /// If `value` is `NaN`. That is deliberate, and preferable to both
    /// alternatives: silently ordering `NaN` as worst would hide a bug in the
    /// objective, and letting it through would be catastrophic under
    /// [`Direction::Maximize`], where `-NaN` sorts below `-inf` and wins every
    /// tournament it enters. A panic names the problem at the moment it occurs,
    /// rather than yielding a converged-looking run built on one poisoned
    /// genome.
    pub fn to_cost(self, value: f64) -> f64 {
        assert!(
            !value.is_nan(),
            "fitness function returned NaN, which the Fitness contract forbids. \
             Check for division by a possibly-zero count, 0.0/0.0, or inf - inf \
             in the objective's arithmetic.",
        );
        self.orient(value)
    }
}

/// An objective the genetic algorithm optimizes over expressed graphs.
///
/// [`Fitness::evaluate`] returns the objective's **natural** value — whatever
/// units make sense to whoever wrote it. [`Fitness::direction`] says whether
/// bigger or smaller is better. The engine minimizes internally and converts
/// once via [`Direction::orient`], so logs and the value handed back to Python
/// are always in the objective's own units and sign.
///
/// # Implementors must not return `NaN`
///
/// This is enforced, not merely documented: every value entering the engine
/// passes through [`Direction::to_cost`], which panics on `NaN`.
///
/// The enforcement is there because the failure it prevents is silent. The
/// engine orders individuals with `f64::total_cmp`, under which `NaN` sorts
/// beyond `+inf` — worst under minimization, which is harmless. But
/// [`Direction::Maximize`] negates, and `-NaN` sorts *below* `-inf`, making it
/// the **best** individual in every tournament it enters. One `NaN` from a
/// maximizing objective would fill the population with whatever genome produced
/// it and leave a run that looks converged.
///
/// Guard the arithmetic that can produce one: division by a possibly-zero
/// count, `0.0 / 0.0`, `inf - inf`.
///
/// The `Send + Sync` bound lets [`Fitness::evaluate_population`] score a whole
/// generation across rayon worker threads.
pub trait Fitness: Send + Sync {
    /// Score a single expressed graph, in the objective's natural units.
    ///
    /// Must not return `NaN`; the engine panics if it does. See the trait
    /// documentation.
    fn evaluate(&self, graph: &Graph) -> f64;

    /// Whether larger or smaller scores are better.
    ///
    /// Defaults to [`Direction::Minimize`], so an objective that is naturally a
    /// cost or error needs to say nothing.
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
        todo!("run the SIR epidemic to completion over `graph` and return a cost")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimize_leaves_a_score_untouched() {
        assert_eq!(Direction::Minimize.orient(2.5), 2.5);
        assert_eq!(Direction::Minimize.orient(-2.5), -2.5);
    }

    #[test]
    fn maximize_flips_a_score_so_the_engine_can_minimize_it() {
        assert_eq!(Direction::Maximize.orient(2.5), -2.5);
        // Bigger natural values must become smaller costs.
        assert!(Direction::Maximize.orient(9.0) < Direction::Maximize.orient(1.0));
    }

    #[test]
    fn orient_is_its_own_inverse_so_reporting_round_trips() {
        // The engine stores costs but logs and reports natural values, so the
        // same call has to convert both ways.
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
    fn to_cost_passes_ordinary_values_through_like_orient() {
        assert_eq!(Direction::Minimize.to_cost(2.5), 2.5);
        assert_eq!(Direction::Maximize.to_cost(2.5), -2.5);
        // Infinities are extreme but well-ordered, so they are allowed through.
        assert_eq!(Direction::Minimize.to_cost(f64::INFINITY), f64::INFINITY);
        assert_eq!(
            Direction::Maximize.to_cost(f64::INFINITY),
            f64::NEG_INFINITY
        );
    }

    #[test]
    #[should_panic(expected = "returned NaN")]
    fn to_cost_rejects_nan_when_minimizing() {
        Direction::Minimize.to_cost(f64::NAN);
    }

    #[test]
    #[should_panic(expected = "returned NaN")]
    fn to_cost_rejects_nan_when_maximizing() {
        Direction::Maximize.to_cost(f64::NAN);
    }

    /// Records *why* `to_cost` asserts instead of trusting the ordering: under
    /// `Maximize` a `NaN` that got through would sort **best**, not worst. If
    /// this test ever fails, the assertion could be reconsidered.
    #[test]
    fn an_unchecked_negated_nan_would_have_sorted_best() {
        let poisoned = Direction::Maximize.orient(f64::NAN);
        assert!(poisoned.is_nan());

        let mut costs = vec![poisoned, -100.0, 0.0, 100.0];
        costs.sort_by(|a, b| a.total_cmp(b));

        // Under minimization the first element is "best" — and it is the NaN.
        assert!(
            costs[0].is_nan(),
            "negated NaN should sort to the best position, got {costs:?}",
        );
    }
}
