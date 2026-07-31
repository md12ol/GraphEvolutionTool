//! GA mechanics shared by every evolution strategy.
//!
//! These helpers keep selection, population setup, evaluation, and logging in
//! one place so [`super::generational`] and [`super::steady_state`] don't each
//! re-implement them.

use std::cmp::Ordering;

use rand::Rng;

use super::GenerationStats;
use crate::fitness::Fitness;
use crate::genomes::Genome;
use crate::graph::Graph;

/// Parent-selection strategy.
///
/// An enum so a new mechanism (roulette-wheel, truncation, rank, ...) is one
/// extra variant plus one match arm, and maps directly onto a `config.toml`
/// field.
pub enum Selection {
    /// Sample `tournament_size` individuals per pick and keep the best.
    Tournament { tournament_size: usize },
}

/// Order two individuals: lower cost first, ties broken by lower index.
///
/// The index tie-break makes a tournament's outcome depend only on which
/// indices were drawn, not the order the RNG produced them. `total_cmp` is used
/// simply because sorting needs a total order; `NaN` cannot reach here, as
/// `Direction::to_cost` rejects it on the way in.
fn rank(fitnesses: &[f64], a: usize, b: usize) -> Ordering {
    fitnesses[a].total_cmp(&fitnesses[b]).then(a.cmp(&b))
}

impl Selection {
    /// Select `count` parents, sampling **with** replacement — the same
    /// individual may be returned more than once. Callers needing distinct
    /// individuals must enforce that themselves, since `select` cannot know
    /// whether its output is a mating pair or an unrelated batch.
    pub fn select<G, R>(
        &self,
        population: &[G],
        fitnesses: &[f64],
        count: usize,
        rng: &mut R,
    ) -> Vec<G>
    where
        G: Genome,
        R: Rng + ?Sized,
    {
        assert_eq!(
            population.len(),
            fitnesses.len(),
            "every individual needs exactly one fitness",
        );
        assert!(
            !population.is_empty(),
            "cannot select from an empty population",
        );

        match self {
            Selection::Tournament { tournament_size } => {
                assert!(*tournament_size > 0, "tournament_size must be at least 1");

                (0..count)
                    .map(|_| {
                        population[Self::tournament_winner(fitnesses, *tournament_size, rng)]
                            .clone()
                    })
                    .collect()
            }
        }
    }

    /// Draw one tournament of **distinct** individuals, best first.
    ///
    /// Feeds tournament-local replacement: the front of the result are parents,
    /// the back are the individuals they displace. Distinctness is required —
    /// "the worst two members" means nothing over a multiset.
    pub fn tournament_indices<R>(&self, fitnesses: &[f64], rng: &mut R) -> Vec<usize>
    where
        R: Rng + ?Sized,
    {
        match self {
            Selection::Tournament { tournament_size } => {
                assert!(*tournament_size > 0, "tournament_size must be at least 1");
                assert!(
                    *tournament_size <= fitnesses.len(),
                    "tournament_size {} exceeds population size {}",
                    tournament_size,
                    fitnesses.len(),
                );

                // Rejection sampling. `tournament_size` is small, so the linear
                // membership scan beats a hash set, and this avoids the
                // O(population) buffer a shuffle would need on every event.
                let mut entrants = Vec::with_capacity(*tournament_size);
                while entrants.len() < *tournament_size {
                    let candidate = rng.random_range(0..fitnesses.len());
                    if !entrants.contains(&candidate) {
                        entrants.push(candidate);
                    }
                }

                entrants.sort_by(|&a, &b| rank(fitnesses, a, b));
                entrants
            }
        }
    }

    /// Draw `tournament_size` individuals **with** replacement, best wins.
    fn tournament_winner<R>(fitnesses: &[f64], tournament_size: usize, rng: &mut R) -> usize
    where
        R: Rng + ?Sized,
    {
        (0..tournament_size)
            .map(|_| rng.random_range(0..fitnesses.len()))
            .min_by(|&a, &b| rank(fitnesses, a, b))
            .expect("tournament_size is at least 1")
    }
}

/// Express every genome against the shared context and score the whole batch,
/// returning the expressed graphs alongside their fitnesses. Index `i` of both
/// vectors refers to `population[i]`.
///
/// Defers to [`Fitness::evaluate_population`] so native objectives parallelize
/// over rayon and Python-backed ones batch across the FFI boundary.
///
/// The graphs are returned rather than dropped because scoring has to build them
/// anyway: handing them back costs nothing, and it saves the caller re-expressing
/// the winner to fill [`super::EvolutionOutcome::best_graph`]. Callers that only
/// need scores can ignore the first element and let it drop.
///
/// Deliberately says nothing about which fitness is *best* — the
/// lower-is-better convention lives with the caller, so this stays a plain
/// express-and-score pass.
pub fn evaluate<G, F>(population: &[G], context: &G::Context, fitness: &F) -> (Vec<Graph>, Vec<f64>)
where
    G: Genome,
    F: Fitness,
{
    let _ = (population, context, fitness);
    todo!("express each genome, score the batch, and return both")
}

/// Summarize a scored population into one evolution-log row.
pub fn generation_stats(iteration: usize, fitnesses: &[f64]) -> GenerationStats {
    let _ = (iteration, fitnesses);
    todo!("compute best, mean, and standard deviation of `fitnesses`")
}

#[cfg(test)]
mod tests {
    use super::*;

    use rand::SeedableRng;
    use rand::rngs::StdRng;

    /// A genome that is just its index, so a winner reports its own slot.
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct IndexGenome(usize);

    impl Genome for IndexGenome {
        type Context = ();

        fn express(&self, _context: &Self::Context) -> Graph {
            Graph::new(1, 1)
        }

        fn crossover<R: Rng + ?Sized>(&mut self, _other: &mut Self, _rng: &mut R) {}

        fn mutate<R: Rng + ?Sized>(&mut self, _rng: &mut R) {}

        fn print(&self) -> String {
            format!("IndexGenome({})", self.0)
        }
    }

    fn population(size: usize) -> Vec<IndexGenome> {
        (0..size).map(IndexGenome).collect()
    }

    /// What a tournament should pick, written independently of `rank`.
    fn expected_winner(fitnesses: &[f64], entrants: &[usize]) -> usize {
        let mut winner = entrants[0];
        for &entrant in &entrants[1..] {
            let better = fitnesses[entrant] < fitnesses[winner];
            let tied = fitnesses[entrant] == fitnesses[winner];
            if better || (tied && entrant < winner) {
                winner = entrant;
            }
        }
        winner
    }

    #[test]
    fn a_size_one_tournament_is_uniform_random_sampling() {
        let selection = Selection::Tournament { tournament_size: 1 };
        let population = population(8);
        let fitnesses = vec![0.0; 8];

        let mut rng = StdRng::seed_from_u64(3);
        let selected = selection.select(&population, &fitnesses, 20, &mut rng);

        // Nothing to compare, so winners are the raw index stream.
        let mut mirror = StdRng::seed_from_u64(3);
        let expected: Vec<_> = (0..20)
            .map(|_| IndexGenome(mirror.random_range(0..8)))
            .collect();

        assert_eq!(selected, expected);
    }

    #[test]
    fn each_winner_is_the_lowest_fitness_entrant_of_its_own_tournament() {
        let tournament_size = 4;
        let selection = Selection::Tournament { tournament_size };
        let population = population(10);
        // Non-monotonic, with a tie at slots 2 and 7 to exercise the tie-break.
        let fitnesses = vec![5.0, 9.0, 1.0, 4.0, 7.0, 3.0, 8.0, 1.0, 6.0, 2.0];

        let mut rng = StdRng::seed_from_u64(41);
        let selected = selection.select(&population, &fitnesses, 50, &mut rng);

        let mut mirror = StdRng::seed_from_u64(41);
        for winner in selected {
            let entrants: Vec<usize> = (0..tournament_size)
                .map(|_| mirror.random_range(0..10))
                .collect();
            assert_eq!(winner, IndexGenome(expected_winner(&fitnesses, &entrants)));
        }
    }

    #[test]
    fn a_tie_is_broken_toward_the_lower_index() {
        // All equally fit, so the lowest drawn index must always win.
        let selection = Selection::Tournament { tournament_size: 5 };
        let population = population(6);
        let fitnesses = vec![2.5; 6];

        let mut rng = StdRng::seed_from_u64(17);
        let selected = selection.select(&population, &fitnesses, 40, &mut rng);

        let mut mirror = StdRng::seed_from_u64(17);
        for winner in selected {
            let lowest_drawn = (0..5).map(|_| mirror.random_range(0..6)).min().unwrap();
            assert_eq!(winner, IndexGenome(lowest_drawn));
        }
    }

    #[test]
    fn a_larger_tournament_applies_more_selection_pressure() {
        // Fitness equals the index, so individual 0 is best and 19 is worst.
        let population = population(20);
        let fitnesses: Vec<f64> = (0..20).map(|i| i as f64).collect();

        let mean_selected = |tournament_size: usize| -> f64 {
            let selection = Selection::Tournament { tournament_size };
            let mut rng = StdRng::seed_from_u64(97);
            let picks = selection.select(&population, &fitnesses, 2_000, &mut rng);
            picks.iter().map(|g| g.0 as f64).sum::<f64>() / picks.len() as f64
        };

        let uniform = mean_selected(1);
        let mild = mean_selected(2);
        let strong = mean_selected(8);

        // Size 1 is uniform: expect roughly the population mean of 9.5.
        assert!(
            (uniform - 9.5).abs() < 1.0,
            "size-1 tournament should be ~uniform, got mean index {uniform}",
        );
        // Catches an inverted comparison, which the other tests would not.
        assert!(
            strong < mild && mild < uniform,
            "pressure should increase with tournament size: {strong} < {mild} < {uniform}",
        );
    }

    #[test]
    fn a_nan_fitness_never_wins_a_tournament() {
        let selection = Selection::Tournament { tournament_size: 3 };
        let population = population(4);
        // Under a naive `<` this would win outright; it may only win alone.
        let fitnesses = vec![10.0, f64::NAN, 20.0, 30.0];

        let mut rng = StdRng::seed_from_u64(5);
        let selected = selection.select(&population, &fitnesses, 500, &mut rng);

        let mut mirror = StdRng::seed_from_u64(5);
        for winner in selected {
            let entrants: Vec<usize> = (0..3).map(|_| mirror.random_range(0..4)).collect();
            if entrants.iter().any(|&e| e != 1) {
                assert_ne!(
                    winner,
                    IndexGenome(1),
                    "NaN won against a real fitness in {entrants:?}",
                );
            }
        }
    }

    #[test]
    fn a_tournament_of_the_whole_population_is_just_the_fitness_ordering() {
        // Tournament == population: every index is drawn once, so the result
        // cannot depend on the RNG. Ties at 1.0 and 3.0 pin the tie-break.
        let fitnesses = vec![3.0, 1.0, 8.0, 2.0, 1.0, 3.0];
        let selection = Selection::Tournament { tournament_size: 6 };

        for seed in 0..8 {
            let mut rng = StdRng::seed_from_u64(seed);
            let drawn = selection.tournament_indices(&fitnesses, &mut rng);
            assert_eq!(drawn, vec![1, 4, 3, 0, 5, 2], "seed {seed}");
        }
    }

    #[test]
    fn a_tournament_draws_distinct_individuals_ordered_best_first() {
        let fitnesses = vec![5.0, 9.0, 1.0, 4.0, 7.0, 3.0, 8.0, 1.0, 6.0, 2.0];
        let selection = Selection::Tournament { tournament_size: 4 };

        let mut rng = StdRng::seed_from_u64(23);
        for _ in 0..200 {
            let drawn = selection.tournament_indices(&fitnesses, &mut rng);

            assert_eq!(drawn.len(), 4);

            let mut unique = drawn.clone();
            unique.sort_unstable();
            unique.dedup();
            assert_eq!(unique.len(), 4, "tournament had duplicates: {drawn:?}");

            for pair in drawn.windows(2) {
                let (earlier, later) = (pair[0], pair[1]);
                assert!(
                    fitnesses[earlier] < fitnesses[later]
                        || (fitnesses[earlier] == fitnesses[later] && earlier < later),
                    "not best-first at {earlier} -> {later} in {drawn:?}",
                );
            }
        }
    }

    #[test]
    fn a_nan_fitness_sorts_to_the_replaceable_end_of_a_tournament() {
        // A poisoned slot must sort last, never into a parent position.
        let fitnesses = vec![4.0, 7.0, f64::NAN, 1.0, 9.0];
        let selection = Selection::Tournament { tournament_size: 5 };

        let mut rng = StdRng::seed_from_u64(2);
        let drawn = selection.tournament_indices(&fitnesses, &mut rng);

        assert_eq!(
            drawn,
            vec![3, 0, 1, 4, 2],
            "NaN should sort last, making it the first individual replaced",
        );
    }

    #[test]
    #[should_panic(expected = "exceeds population size")]
    fn a_tournament_larger_than_the_population_is_rejected() {
        let selection = Selection::Tournament { tournament_size: 7 };
        let mut rng = StdRng::seed_from_u64(1);
        selection.tournament_indices(&[0.0; 5], &mut rng);
    }

    #[test]
    #[should_panic(expected = "every individual needs exactly one fitness")]
    fn a_fitness_array_of_the_wrong_length_is_rejected() {
        let selection = Selection::Tournament { tournament_size: 2 };
        let mut rng = StdRng::seed_from_u64(1);
        selection.select(&population(4), &[0.0, 1.0], 1, &mut rng);
    }

    #[test]
    #[should_panic(expected = "cannot select from an empty population")]
    fn an_empty_population_is_rejected() {
        let selection = Selection::Tournament { tournament_size: 2 };
        let mut rng = StdRng::seed_from_u64(1);
        selection.select::<IndexGenome, _>(&[], &[], 1, &mut rng);
    }

    #[test]
    #[should_panic(expected = "tournament_size must be at least 1")]
    fn a_zero_sized_tournament_is_rejected() {
        let selection = Selection::Tournament { tournament_size: 0 };
        let mut rng = StdRng::seed_from_u64(1);
        selection.select(&population(4), &[0.0; 4], 1, &mut rng);
    }
}
