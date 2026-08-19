//! Test doubles shared by the two evolvers' test modules.
//!
//! Both strategies need the same genomes, the same objectives and the same
//! summary helpers to test against, and keeping two copies meant the two files
//! could drift apart without either being wrong. This module is `cfg(test)`, so
//! nothing here reaches the lib target.

use rand::Rng;

use crate::fitness::{Direction, Fitness};
use crate::genomes::Genome;
use crate::graph::Graph;

/// A genome whose single value drives both its identity and its fitness, so
/// a test can say exactly which individual ended up in which slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Val(pub(crate) usize);

impl Genome for Val {
    type Context = ();

    fn express(&self, _context: &Self::Context) -> Graph {
        Graph::new(self.0 + 1, 1)
    }

    /// Swap, so a crossover that happened is visible in the result.
    fn crossover<R: Rng + ?Sized>(&mut self, other: &mut Self, _rng: &mut R) {
        std::mem::swap(&mut self.0, &mut other.0);
    }

    /// A large, unmistakable jump, and no test value is near it: one mutation
    /// adds 100, so a child's value carries both its parent (modulo 100) and
    /// its mutation count.
    fn mutate<R: Rng + ?Sized>(&mut self, _context: &Self::Context, _rng: &mut R) {
        self.0 += 100;
    }

    fn print(&self) -> String {
        format!("Val({})", self.0)
    }
}

/// Like `Val`, but mutation drifts up or down using the RNG, so evolution
/// can actually improve a population and a run test is not vacuous.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Walk(pub(crate) usize);

impl Genome for Walk {
    type Context = ();

    fn express(&self, _context: &Self::Context) -> Graph {
        Graph::new(self.0 + 1, 1)
    }

    fn crossover<R: Rng + ?Sized>(&mut self, other: &mut Self, _rng: &mut R) {
        std::mem::swap(&mut self.0, &mut other.0);
    }

    fn mutate<R: Rng + ?Sized>(&mut self, _context: &Self::Context, rng: &mut R) {
        if rng.random_bool(0.5) {
            self.0 = self.0.saturating_sub(1);
        } else {
            self.0 += 1;
        }
    }

    fn print(&self) -> String {
        format!("Walk({})", self.0)
    }
}

/// Fitness is the node count, which both genomes' `express` sets from their
/// value, so a lower value is a fitter individual.
pub(crate) struct NodeCount;

impl Fitness for NodeCount {
    fn evaluate(&self, graph: &Graph) -> f64 {
        graph.num_nodes as f64
    }
}

/// The same score under `Maximize`, so a test can tell an engine-oriented
/// outcome from a converted one — under `NodeCount` the two are identical,
/// because orienting a minimizing objective is the identity.
pub(crate) struct MostNodes;

impl Fitness for MostNodes {
    fn evaluate(&self, graph: &Graph) -> f64 {
        graph.num_nodes as f64
    }

    fn direction(&self) -> Direction {
        Direction::Maximize
    }
}

/// The best (lowest) fitness among a population's scores.
pub(crate) fn best_of(fitnesses: &[f64]) -> f64 {
    let mut best = fitnesses[0];
    for &f in &fitnesses[1..] {
        if f < best {
            best = f;
        }
    }
    best
}

/// The mean fitness across a population's scores.
pub(crate) fn mean_of(fitnesses: &[f64]) -> f64 {
    let mut sum = 0.0;
    for &f in fitnesses {
        sum += f;
    }
    sum / fitnesses.len() as f64
}
