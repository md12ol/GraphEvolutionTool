//! GA mechanics shared by every evolution strategy.
//!
//! These helpers keep selection, population setup, evaluation, and logging in
//! one place so [`super::generational`] and [`super::steady_state`] don't each
//! re-implement them.

use rand::Rng;

use super::GenerationStats;
use crate::fitness::Fitness;
use crate::genomes::Genome;
use crate::graph::Graph;

/// Parent-selection strategy.
///
/// Kept as an enum so a new mechanism (roulette-wheel, truncation, rank, ...)
/// is a single extra variant plus one match arm in [`Selection::select`]; the
/// evolvers just hold a `Selection` and are unaffected. An enum also stays
/// monomorphized, avoiding `dyn`-dispatch friction with the generic `select`,
/// and maps directly onto a `config.toml` field.
pub enum Selection {
    /// Sample `tournament_size` individuals at random per pick and keep the
    /// best (lowest fitness).
    Tournament { tournament_size: usize },
}

impl Selection {
    /// Select `count` parents from the scored population. `count` lets a single
    /// selection round yield more than one individual (e.g. a pair of parents).
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
        match self {
            Selection::Tournament { tournament_size } => {
                let _ = (population, fitnesses, count, tournament_size, rng);
                todo!("run `count` fitness tournaments and clone the winners")
            }
        }
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
