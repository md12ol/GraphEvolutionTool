//! Which slice of the population one breeding event may touch.

use rand::Rng;

/// The candidates a single breeding event draws from.
///
/// The "where" of an event, separate from who breeds within it
/// ([`super::common::Selection`]) and who is replaced
/// ([`super::replacement::Replacement`]). Splitting it out is what lets any
/// scheme work with any strategy: a scheme picks from whatever slice it is
/// handed and needs no opinion about locality.
///
/// It is also what makes steady-state self-elitist — parents and replacements
/// come from the *same* slice, so the slice's best is never overwritten, and
/// that holds whichever scheme picked the parents.
///
/// A new variant is one arm in `Scope::draw_into`, plus
/// `dispatch::scope_and_selection` to reach it from a config file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Every individual is a candidate. Consumes no randomness.
    Global,
    /// `size` distinct individuals, drawn uniformly without replacement.
    /// Distinctness is what lets a replacement policy name "the worst two
    /// members", which means nothing over a multiset.
    RandomSubset { size: usize },
}

impl Scope {
    /// Fill `out` with the indices this event may touch, unordered.
    ///
    /// Writes into the caller's buffer rather than returning a new `Vec`:
    /// generational draws a scope per breeding pair, and an allocation per pair
    /// would be a cost this abstraction has no reason to add. Anything needing
    /// them ranked sorts its own copy, which for a small subset beats sorting a
    /// global scope nothing was going to read in order.
    pub(super) fn draw_into<R>(&self, population_len: usize, out: &mut Vec<usize>, rng: &mut R)
    where
        R: Rng + ?Sized,
    {
        assert!(
            population_len > 0,
            "cannot draw a scope from an empty population"
        );
        out.clear();

        match self {
            Scope::Global => {
                for index in 0..population_len {
                    out.push(index);
                }
            }
            Scope::RandomSubset { size } => {
                assert!(*size > 0, "a scope of no individuals cannot breed");
                assert!(
                    *size <= population_len,
                    "scope of {} exceeds population size {}",
                    size,
                    population_len,
                );

                // Rejection sampling. `size` is small, so the linear membership
                // scan beats a hash set, and this avoids the O(population)
                // buffer a shuffle would need on every event.
                while out.len() < *size {
                    let candidate = rng.random_range(0..population_len);
                    if !out.contains(&candidate) {
                        out.push(candidate);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn a_global_scope_is_every_index_and_consumes_no_randomness() {
        let mut rng = StdRng::seed_from_u64(7);
        let before = rng.random::<u64>();

        let mut out = Vec::new();
        Scope::Global.draw_into(5, &mut out, &mut rng);
        assert_eq!(out, vec![0, 1, 2, 3, 4]);

        // Same draw as if the scope had never run: it must not touch the stream.
        let mut fresh = StdRng::seed_from_u64(7);
        assert_eq!(before, fresh.random::<u64>());
        let mut second = Vec::new();
        Scope::Global.draw_into(5, &mut second, &mut fresh);
        assert_eq!(fresh.random::<u64>(), rng.random::<u64>());
    }

    #[test]
    fn a_random_subset_is_distinct_and_the_size_asked_for() {
        let mut rng = StdRng::seed_from_u64(11);
        let mut out = Vec::new();

        for _ in 0..50 {
            Scope::RandomSubset { size: 4 }.draw_into(10, &mut out, &mut rng);
            assert_eq!(out.len(), 4);

            let mut seen = out.clone();
            seen.sort_unstable();
            seen.dedup();
            assert_eq!(seen.len(), 4, "entrants must be distinct: {out:?}");
        }
    }

    #[test]
    fn the_buffer_is_reused_rather_than_appended_to() {
        let mut rng = StdRng::seed_from_u64(3);
        let mut out = vec![99, 98, 97];

        Scope::RandomSubset { size: 2 }.draw_into(6, &mut out, &mut rng);
        assert_eq!(out.len(), 2, "stale entries must not survive: {out:?}");
    }

    #[test]
    #[should_panic(expected = "scope of 8 exceeds population size 5")]
    fn a_subset_larger_than_the_population_cannot_be_drawn() {
        let mut rng = StdRng::seed_from_u64(1);
        let mut out = Vec::new();
        Scope::RandomSubset { size: 8 }.draw_into(5, &mut out, &mut rng);
    }
}
