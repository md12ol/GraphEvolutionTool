use rayon::prelude::*;

use crate::graph::Graph;

/// An objective the genetic algorithm optimizes over expressed graphs.
///
/// Convention: **lower is better** — a returned value is treated as a cost or
/// error, matching the reference implementation's MMD scoring. Objectives that
/// are naturally maximized (for example, "maximize epidemic spread") should
/// return a negated or inverted value so the engine's minimization still holds.
///
/// The `Send + Sync` bound lets [`Fitness::evaluate_population`] score a whole
/// generation across rayon worker threads.
pub trait Fitness: Send + Sync {
    /// Score a single expressed graph.
    fn evaluate(&self, graph: &Graph) -> f64;

    /// Score an entire generation of expressed graphs.
    ///
    /// The default fans [`Fitness::evaluate`] out across rayon, which is ideal
    /// for native Rust objectives. A Python-backed adapter overrides this to
    /// acquire the GIL once per generation and vectorize the whole batch,
    /// instead of paying the FFI/GIL cost once per individual.
    fn evaluate_population(&self, graphs: &[Graph]) -> Vec<f64> {
        graphs.par_iter().map(|graph| self.evaluate(graph)).collect()
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
