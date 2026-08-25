//! Module root for `genomes`: re-exports the [`Genome`] trait, both
//! representations, and the context, mutation and configuration types each of
//! them needs, so callers name them from `crate::genomes` rather than from the
//! module each is declared in.

use rand::Rng;

// ADD A GENOME STEP 3 — declare your module, then add your types to the
// re-exports below: your genome and its helpers on a line of their own, your
// context and mutation kind onto the end of the existing `genome::` line.
//
//     pub mod my_genome;
//     pub use my_genome::{MyGenome, MyOperators};
//     pub use genome::{..., MyContext, MyMutation};

pub mod edge_edit;
pub mod genome;
pub mod sda;

pub use edge_edit::{EdgeEditGenome, EdgeEditOperationWeights, EdgeEditOperators};
pub use genome::{EdgeEditContext, EdgeEditMutation, Genome, SdaContext, SdaMutation};
pub use sda::{SdaDimensions, SdaGenome};

/// Draw two distinct cut points in `0..=shared_length` and return them in
/// ascending order, for a two-point crossover that swaps the half-open
/// segment `[start, end)`.
///
/// Callers must ensure `shared_length >= 1`; below that there is no pair of
/// distinct points to draw and this loops forever.
fn two_distinct_cut_points<R: Rng + ?Sized>(shared_length: usize, rng: &mut R) -> (usize, usize) {
    loop {
        let a = rng.random_range(0..=shared_length);
        let b = rng.random_range(0..=shared_length);
        if a != b {
            return (a.min(b), a.max(b));
        }
    }
}
