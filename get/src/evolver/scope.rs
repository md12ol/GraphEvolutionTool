//! Which slice of the population one breeding event may touch.

use rand::Rng;

/// The candidates a single breeding event draws from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Every individual is a candidate. Consumes no randomness.
    Global,
    /// `size` distinct individuals, drawn uniformly without replacement.
    RandomSubset { size: usize },
    // ADD A SCOPE STEP 1 — a variant here, plus whatever it needs to locate a
    // slice:
    //
    //     Neighbourhood { radius: usize },
}

impl Scope {
    /// Fill `out` with the distinct indices this event may touch, unordered.
    /// `out` is cleared first, so any prior contents are discarded.
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

                while out.len() < *size {
                    let candidate = rng.random_range(0..population_len);
                    if !out.contains(&candidate) {
                        out.push(candidate);
                    }
                }
            } // ADD A SCOPE STEP 2 — the arm filling `out` with the indices your
              // scope covers. Push each index once.
              //
              //     Scope::Neighbourhood { radius } => { /* push indices around a random centre */ }
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
