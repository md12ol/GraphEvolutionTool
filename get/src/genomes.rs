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
