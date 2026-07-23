use rand::Rng;

use crate::graph::{EdgeMultiplicityCap, Graph};

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
    /// The state the automaton starts in before consuming `init_char`'s
    /// first transition. Fixed run configuration, not evolved genome data
    /// (unlike `init_char`, `init_state` is never touched by
    /// [`Genome::mutate`]/[`Genome::crossover`]), so it lives here rather
    /// than on `SdaGenome`.
    pub init_state: usize,
    edge_multiplicity_cap: EdgeMultiplicityCap,
}

impl SdaContext {
    /// Construct an SDA context using GET's default multigraph cap of five.
    pub fn new(num_nodes: usize, init_state: usize) -> Self {
        Self {
            num_nodes,
            init_state,
            edge_multiplicity_cap: EdgeMultiplicityCap::DEFAULT,
        }
    }

    /// Construct an SDA context whose expressed graphs are unweighted.
    pub fn unweighted(num_nodes: usize, init_state: usize) -> Self {
        Self {
            num_nodes,
            init_state,
            edge_multiplicity_cap: EdgeMultiplicityCap::UNWEIGHTED,
        }
    }

    /// Construct an SDA context with an explicit graph multiplicity cap.
    pub fn with_max_edge_multiplicity(
        num_nodes: usize,
        init_state: usize,
        max_edge_multiplicity: u32,
    ) -> Result<Self, &'static str> {
        Ok(Self {
            num_nodes,
            init_state,
            edge_multiplicity_cap: EdgeMultiplicityCap::new(max_edge_multiplicity)?,
        })
    }

    /// Return the multiplicity cap applied to graphs expressed by this context.
    pub fn max_edge_multiplicity(&self) -> u32 {
        self.edge_multiplicity_cap.get()
    }

    pub(crate) fn edge_multiplicity_cap(&self) -> EdgeMultiplicityCap {
        self.edge_multiplicity_cap
    }
}
