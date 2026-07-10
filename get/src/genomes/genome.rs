use rand::Rng;
use crate::graph::Graph;

/// The variation-operator interface every genome representation implements.
///
/// `Clone + Send + Sync` lets populations copy individuals and evolve across threads.
pub trait Genome: Clone + Send + Sync {
    /// Express a genome as a graph. The graph is the phenotype of the genome.
    fn express(&self) -> Graph;

    /// Recombine two parents in place, producing two children.
    ///
    /// `self` and `other` are the parents on entry and the children on return
    /// (recombined in place). Generic over the RNG so the engine controls the
    /// reproducible random stream.
    fn crossover<R: Rng + ?Sized>(&mut self, other: &mut Self, rng: &mut R);

    /// Mutate a genome in place.
    /// 
    /// `self` is the genome on entry and the mutated genome on return (mutated in place).
    fn mutate<R: Rng + ?Sized>(&mut self, rng: &mut R);

    /// Create a copy of the genome.
    fn copy(&self) -> Self;

    /// Print a genome in a human-readable format.
    fn print(&self);
}

