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
/// # Adding a variant
///
/// 1. **This enum** — the variant and its parameters.
/// 2. **`Scope::draw_into`** — the arm filling the buffer; the match is
///    exhaustive, so the compiler finds it.
/// 3. **`config::ScopeConfig`** — what a user names under `[scope]`, plus any
///    constraint on its own parameters in `Config::validate_scope`.
/// 4. **`dispatch::scope`** — the arm mapping that config variant onto this
///    one. Steps 3 and 4 are one change split across two files.
/// 5. **`py_config::PyScopeConfig`** — optional; buys a Python caller the
///    ability to name it.
/// 6. **`config.example.toml`** — optional, and the step people skip.
///
/// A variant's parameters are its own. `size` belongs to `[scope]` and nothing
/// else reads it — steady-state used to take its scope size from
/// `[selection]`'s `tournament_size`, which left a scheme with no tournament
/// unable to say how large a scope it wanted.
///
/// **Every step is marked at its own site.** Search the repo for
/// `ADD A SCOPE STEP 3`, or any other number:
///
/// ```text
/// git grep -n "ADD A SCOPE STEP"    # all six, in one list
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Every individual is a candidate. Consumes no randomness.
    Global,
    /// `size` distinct individuals, drawn uniformly without replacement.
    /// Distinctness is what lets a replacement policy name "the worst two
    /// members", which means nothing over a multiset.
    RandomSubset { size: usize },
    // ADD A SCOPE STEP 1 — a variant here, plus whatever it needs to locate a
    // slice:
    //
    //     Neighbourhood { radius: usize },
    //
    // A grid neighbourhood is the obvious one: it makes every existing scheme
    // and policy into a cellular GA without either of them changing. Give it
    // its own parameters rather than reading another block's — that coupling is
    // what this enum was split out to end. Then the arm drawing it, in
    // `draw_into` below — search `ADD A SCOPE STEP 2`.
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
            } // ADD A SCOPE STEP 2 — the arm filling `out` with the indices your
              // scope covers:
              //
              //     Scope::Neighbourhood { radius } => {
              //         let centre = rng.random_range(0..population_len);
              //         for offset in 0..=(radius * 2) {
              //             out.push((centre + offset) % population_len);
              //         }
              //     }
              //
              // Clear `out` first (done above, once, for every arm), leave the
              // indices unordered, and keep them distinct if any replacement
              // policy is to name "the worst two". If it should be selectable from
              // a config file, search `ADD A SCOPE STEP 3`; if Rust-only, you are
              // finished here.
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
