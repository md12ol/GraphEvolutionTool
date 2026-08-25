//! Who a breeding event's children overwrite. Steady-state only — generational
//! rebuilds its whole population and never asks.

use super::common::rank;
use rand::Rng;

/// Which members of a scope are replaced by the children bred within it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Replacement {
    /// The scope's least fit members, worst first. Self-elitist: the scope's
    /// best is never overwritten, whatever scheme chose the parents.
    Worst,
    /// Distinct members of the scope, drawn uniformly.
    Random,
    // ADD A REPLACEMENT STEP 1 (for SteadyState) — a variant here, plus any parameters:
    //
    //     Tournament { size: usize },
}

impl Replacement {
    /// The `count` members of `scope` to overwrite, in the order children take
    /// their slots. Indices into the population, not into `scope`.
    pub(super) fn pick<R>(
        &self,
        scope: &[usize],
        fitnesses: &[f64],
        count: usize,
        rng: &mut R,
    ) -> Vec<usize>
    where
        R: Rng + ?Sized,
    {
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
            }
            Replacement::Random => {
                // Scope entries are distinct; with repeats this would not terminate.
                let mut victims = Vec::with_capacity(count);
                while victims.len() < count {
                    let candidate = scope[rng.random_range(0..scope.len())];
                    if !victims.contains(&candidate) {
                        victims.push(candidate);
                    }
                }
                victims
            } // ADD A REPLACEMENT STEP 2 (for SteadyState) — the arm naming who the children
              // overwrite, as indices into the population:
              //
              //     Replacement::Tournament { size } => { /* ... */ }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    fn rng() -> ChaCha8Rng {
        ChaCha8Rng::seed_from_u64(7)
    }

    #[test]
    fn the_worst_are_returned_worst_first() {
        // Scope deliberately unordered, so returning it as-given fails.
        let fitnesses = [5.0, 1.0, 9.0, 3.0, 7.0];
        let scope = [2, 0, 4, 1, 3];

        assert_eq!(
            Replacement::Worst.pick(&scope, &fitnesses, 2, &mut rng()),
            vec![2, 4]
        );
    }

    #[test]
    fn only_the_scope_is_at_risk() {
        // Index 0 is globally worst but outside the scope, so it must survive.
        let fitnesses = [99.0, 1.0, 5.0, 3.0];
        let scope = [1, 2, 3];

        assert_eq!(
            Replacement::Worst.pick(&scope, &fitnesses, 1, &mut rng()),
            vec![2]
        );
    }

    #[test]
    fn ties_are_broken_by_lower_index_so_a_draw_is_reproducible() {
        // Two individuals at 4.0: the higher index is the worse of the pair,
        // matching the ordering selection uses everywhere else.
        let fitnesses = [4.0, 4.0, 1.0];
        let scope = [0, 1, 2];

        assert_eq!(
            Replacement::Worst.pick(&scope, &fitnesses, 1, &mut rng()),
            vec![1]
        );
    }

    #[test]
    fn a_nan_fitness_sorts_to_the_replaced_end() {
        // A poisoned slot must be the first thing overwritten, never left to
        // breed. `rank` uses `total_cmp`, which puts NaN above every real
        // number, so it lands at the replaceable end rather than winning.
        let fitnesses = [4.0, 7.0, f64::NAN, 1.0, 9.0];
        let scope = [0, 1, 2, 3, 4];

        assert_eq!(
            Replacement::Worst.pick(&scope, &fitnesses, 2, &mut rng()),
            vec![2, 4]
        );
    }

    #[test]
    #[should_panic(expected = "cannot replace 4 of a scope of 2")]
    fn replacing_more_than_the_scope_holds_is_a_bug() {
        Replacement::Worst.pick(&[0, 1], &[1.0, 2.0], 4, &mut rng());
    }

    #[test]
    fn random_returns_distinct_members_of_the_scope() {
        // Index 0 is outside the scope, so no draw may ever return it, and the
        // two victims must differ or one child overwrites the other.
        let fitnesses = [9.0, 1.0, 5.0, 3.0, 7.0];
        let scope = [1, 2, 3, 4];

        let mut rng = rng();
        for _ in 0..200 {
            let victims = Replacement::Random.pick(&scope, &fitnesses, 2, &mut rng);
            assert_eq!(victims.len(), 2);
            assert_ne!(victims[0], victims[1]);
            for victim in &victims {
                assert!(scope.contains(victim), "{victim} is outside the scope");
            }
        }
    }

    #[test]
    fn random_can_overwrite_the_scopes_best() {
        // The property that separates it from `Worst`: over enough draws the
        // fittest member is picked, so self-elitism is genuinely given up.
        let fitnesses = [1.0, 2.0, 99.0];
        let scope = [0, 1, 2];

        let mut rng = rng();
        let mut best_was_replaced = false;
        for _ in 0..200 {
            if Replacement::Random.pick(&scope, &fitnesses, 1, &mut rng)[0] == 2 {
                best_was_replaced = true;
            }
        }
        assert!(best_was_replaced, "the scope's best was never drawn");
    }

    #[test]
    fn random_is_reproducible_from_a_seed() {
        let fitnesses = [1.0, 2.0, 3.0, 4.0];
        let scope = [0, 1, 2, 3];

        let first = Replacement::Random.pick(&scope, &fitnesses, 2, &mut rng());
        let second = Replacement::Random.pick(&scope, &fitnesses, 2, &mut rng());
        assert_eq!(first, second);
    }
}
