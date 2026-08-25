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

#[cfg(test)]
mod tests {
    use super::two_distinct_cut_points;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    /// Every pair of cut points has to be equally likely, and the obvious
    /// simplification is not: drawing `start` first and `end` somewhere above
    /// it reaches the same pairs but weights them `1 / (L * (L - start))`,
    /// piling short segments up at the end of the genome. Neither genome's
    /// crossover test can see the difference — both check *which* segment
    /// moved and never how often — so this is the only thing standing between
    /// the two samplers.
    #[test]
    fn every_pair_of_cut_points_is_drawn_equally_often() {
        const LENGTH: usize = 4;
        const DRAWS: usize = 20_000;
        // Pairs (start, end) with start < end over 0..=LENGTH: 10 of them.
        const PAIRS: usize = (LENGTH + 1) * LENGTH / 2;

        // Flat, indexed by `start * width + end`, so the counts can be read
        // back by a pair of plain loops without indexing a slice by the loop
        // variable itself.
        let width = LENGTH + 1;
        let mut counts = vec![0usize; width * width];
        let mut rng = StdRng::seed_from_u64(2026);

        for _ in 0..DRAWS {
            let (start, end) = two_distinct_cut_points(LENGTH, &mut rng);
            assert!(
                start < end,
                "({start}, {end}) is not distinct and ascending"
            );
            assert!(end <= LENGTH, "({start}, {end}) leaves the shared prefix");
            counts[start * width + end] += 1;
        }

        // 2 / (L * (L + 1)) each, so a tenth of the draws at L = 4.
        let expected = DRAWS / PAIRS;
        for start in 0..=LENGTH {
            for end in (start + 1)..=LENGTH {
                let seen = counts[start * width + end];
                assert!(
                    seen.abs_diff(expected) < expected / 5,
                    "({start}, {end}) drawn {seen} times, expected about {expected}"
                );
            }
        }
    }
}
