use rand::Rng;

/// The variation-operator interface every genome representation implements.
///
/// This is the Rust equivalent of a "base class": engines are written against
/// `Genome` and never against a concrete representation, so `EdgeEditGenome`
/// and `SdaGenome` are interchangeable from the engine's point of view.
///
/// `Clone + Send + Sync` lets populations copy individuals and evolve across
/// threads.
pub trait Genome: Clone + Send + Sync {
    /// Recombine two parents in place, producing two children.
    ///
    /// `a` and `b` are the parents on entry and the children on return
    /// (in-place, matching the `setu.cpp` two-parent operators). Generic over
    /// the RNG so the engine controls the reproducible random stream.
    fn crossover<R: Rng + ?Sized>(a: &mut Self, b: &mut Self, rng: &mut R);
}
