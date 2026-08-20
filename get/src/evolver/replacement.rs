//! Who a breeding event's children overwrite.

use super::common::rank;

/// Which members of a scope are replaced by the children bred within it.
///
/// The counterpart to [`super::common::Selection`]: that answers who breeds,
/// this who dies, both inside the slice [`super::scope::Scope`] drew. Keeping
/// them apart is what stops a scheme from having to supply a replacement draw
/// it has no theory for.
///
/// **Consumes no randomness** — every policy reads the scope's fitnesses and
/// nothing else, so a replacement choice cannot shift a seeded run's stream.
///
/// # Adding a policy
///
/// 1. **This enum** — the variant and its parameters.
/// 2. **`Replacement::pick`** — the arm; the match is exhaustive.
/// 3. **`dispatch::run_strategy`** — only if a user should be able to choose
///    it. There is no `[replacement]` block today, so a policy reachable only
///    from Rust stops at step 2.
///
/// A policy needing anything the engine does not record per individual — an
/// age, a lineage, a distance — needs that recorded first, which is a wider
/// change than a variant.
///
/// **Every step is marked at its own site.** Search the repo for
/// `ADD A REPLACEMENT STEP 2`, or any other number:
///
/// ```text
/// git grep -n "ADD A REPLACEMENT STEP"    # all three, in one list
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Replacement {
    /// The scope's least fit members, worst first. Never the scope's best,
    /// which is what makes a strategy using it self-elitist whatever scheme
    /// chose the parents.
    Worst,
    // ADD A REPLACEMENT STEP 1 — a variant here, plus any parameters:
    //
    //     Random,
    //
    // Note what a policy gives up: `Worst` is what makes a strategy using it
    // self-elitist, and one that can overwrite the scope's best removes that
    // guarantee. Say so at the variant rather than leaving it to be discovered.
    // Then the arm choosing victims — search `ADD A REPLACEMENT STEP 2`.
}

impl Replacement {
    /// The `count` members of `scope` to overwrite, in the order children take
    /// their slots. Indices into the population, not into `scope`.
    pub(super) fn pick(&self, scope: &[usize], fitnesses: &[f64], count: usize) -> Vec<usize> {
        assert!(
            count <= scope.len(),
            "cannot replace {} of a scope of {}",
            count,
            scope.len(),
        );

        match self {
            Replacement::Worst => {
                let mut ranked = scope.to_vec();
                ranked.sort_by(|&a, &b| rank(fitnesses, a, b));

                let mut victims = Vec::with_capacity(count);
                for offset in 1..=count {
                    victims.push(ranked[ranked.len() - offset]);
                }
                victims
            } // ADD A REPLACEMENT STEP 2 — the arm naming who the children
              // overwrite:
              //
              //     Replacement::Random => {
              //         // needs an `rng` parameter; `pick` takes none today
              //         // precisely because no shipped policy draws.
              //     }
              //
              // Return indices into the *population*, not into `scope`, and in
              // the order children should take the slots. Adding a policy that
              // consumes randomness changes this signature and shifts every
              // seeded run's RNG stream — a real cost, not a formality. To make
              // it selectable from a config file, search
              // `ADD A REPLACEMENT STEP 3`.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_worst_are_returned_worst_first() {
        // Scope deliberately unordered, so returning it as-given fails.
        let fitnesses = [5.0, 1.0, 9.0, 3.0, 7.0];
        let scope = [2, 0, 4, 1, 3];

        assert_eq!(Replacement::Worst.pick(&scope, &fitnesses, 2), vec![2, 4]);
    }

    #[test]
    fn only_the_scope_is_at_risk() {
        // Index 0 is globally worst but outside the scope, so it must survive.
        let fitnesses = [99.0, 1.0, 5.0, 3.0];
        let scope = [1, 2, 3];

        assert_eq!(Replacement::Worst.pick(&scope, &fitnesses, 1), vec![2]);
    }

    #[test]
    fn ties_are_broken_by_lower_index_so_a_draw_is_reproducible() {
        // Two individuals at 4.0: the higher index is the worse of the pair,
        // matching the ordering selection uses everywhere else.
        let fitnesses = [4.0, 4.0, 1.0];
        let scope = [0, 1, 2];

        assert_eq!(Replacement::Worst.pick(&scope, &fitnesses, 1), vec![1]);
    }

    #[test]
    fn a_nan_fitness_sorts_to_the_replaced_end() {
        // A poisoned slot must be the first thing overwritten, never left to
        // breed. `rank` uses `total_cmp`, which puts NaN above every real
        // number, so it lands at the replaceable end rather than winning.
        let fitnesses = [4.0, 7.0, f64::NAN, 1.0, 9.0];
        let scope = [0, 1, 2, 3, 4];

        assert_eq!(Replacement::Worst.pick(&scope, &fitnesses, 2), vec![2, 4]);
    }

    #[test]
    #[should_panic(expected = "cannot replace 4 of a scope of 2")]
    fn replacing_more_than_the_scope_holds_is_a_bug() {
        Replacement::Worst.pick(&[0, 1], &[1.0, 2.0], 4);
    }
}
