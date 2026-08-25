//! The genome representations, and the helpers both of them share.
//!
//! # Part of the chain that adds a representation
//!
//! This is step 3: declare the module below and re-export the type and its
//! context, so callers name them from `crate::genomes` rather than from the
//! private path. The step before it is implementing [`Genome`] itself, and
//! [`genome`]'s module doc has all seven.
//!
//! Anything two representations genuinely share lives here rather than in one
//! of them — `two_distinct_cut_points` is the case that exists, so that a
//! crossover segment means the same thing whichever genome drew it.

use rand::Rng;

// ADD A GENOME STEP 3 — declare the module and re-export the type and its
// context, so callers name them from `crate::genomes`.
//
//     pub mod my_genome;
//     pub use my_genome::MyGenome;
//     pub use genome::MyContext;      // if the context lives in `genome.rs`

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
/// Rejection sampling — redraw until the two differ — rather than drawing
/// `start` first and then `end` somewhere above it. Both reach the same set
/// of pairs, but not with the same weights: drawing sequentially gives
/// `P(start, end) = 1 / (L * (L - start))`, which depends on `start` and so
/// concentrates on short segments late in the genome, while redrawing until
/// the two differ gives every pair the same `2 / (L * (L + 1))`. Both genomes
/// share this so that a segment means the same thing in either
/// representation.
///
/// Callers must ensure `shared_length >= 1`; below that there is no pair of
/// distinct points to draw and this would not terminate.
fn two_distinct_cut_points<R: Rng + ?Sized>(shared_length: usize, rng: &mut R) -> (usize, usize) {
    loop {
        let a = rng.random_range(0..=shared_length);
        let b = rng.random_range(0..=shared_length);
        if a != b {
            return (a.min(b), a.max(b));
        }
    }
}
