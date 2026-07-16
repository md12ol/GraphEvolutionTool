use rand::Rng;

use crate::graph::Graph;

/// The variation-operator interface implemented by every genome representation.
///
/// `Clone + Send + Sync` allows the GA to copy individuals and evaluate a
/// population across worker threads.
pub trait Genome: Clone + Send + Sync {
    /// Run-level configuration required to express this genome.
    type Context;

    /// Express this genome as a graph using shared run-level configuration.
    fn express(&self, context: &Self::Context) -> Graph;

    /// Recombine two parents in place, leaving the resulting children in
    /// `self` and `other`.
    fn crossover<R: Rng + ?Sized>(&mut self, other: &mut Self, rng: &mut R);

    /// Mutate this genome in place.
    fn mutate<R: Rng + ?Sized>(&mut self, rng: &mut R);

    fn copy(&self) -> Self;

    /// Return a human-readable description of the genome.
    fn print(&self) -> String;
}

/// Configuration used when an edge-edit genome modifies an initial graph.
#[derive(Clone, Debug)]
pub struct EdgeEditContext {
    pub base_graph: Graph,
}

/// Configuration used when an SDA genome generates a graph from scratch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SdaContext {
    pub num_nodes: usize,
    pub max_steps: usize,
}
