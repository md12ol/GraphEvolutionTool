//! What a run hands back to Python: [`PyRunResult`] and its log rows.
//!
//! # Why these are a separate mirror rather than `#[pyclass]` on the engine's own types
//!
//! Two reasons, and each on its own would be enough.
//! [`crate::evolver::EvolutionOutcome`] is generic over the genome, and a
//! `#[pyclass]` cannot carry a type parameter. And
//! [`crate::evolver::GenerationStats`] holds engine-oriented numbers —
//! lower-is-better, whatever the objective actually computes — which are not the
//! numbers a user should ever see.
//!
//! So the dispatch layer erases the genome and converts the orientation, and this
//! module is what that erased result becomes on the way out.
//!
//! **Everything here is in the objective's own units.** Nothing in this module
//! converts anything; [`crate::dispatch`]'s `erase` is the one place that
//! happens.

use pyo3::prelude::*;

use crate::dispatch::ErasedOutcome;

/// One row of the convergence log.
///
/// `iteration` counts generations under the generational strategy and mating
/// events under steady-state.
///
/// Frozen, and every field read-only: this is a record of something that already
/// happened, so there is nothing a caller could correctly change.
#[pyclass(name = "GenerationStats", frozen, get_all)]
#[derive(Debug, Clone)]
pub struct PyGenerationStats {
    /// Generation number, or mating-event number.
    pub iteration: usize,
    /// Best fitness in the population at this iteration.
    pub best_fitness: f64,
    /// Population mean fitness at this iteration.
    pub mean_fitness: f64,
    /// **Population** standard deviation — divides by `n`, not `n - 1`, because
    /// these are all the individuals there are rather than a sample of some
    /// larger group. A population of one therefore has a deviation of zero.
    ///
    /// Unconverted, and correctly so: a spread is identical under negation, so
    /// this reads the same whichever direction the objective runs in.
    pub std_dev: f64,
    /// Half-width of the 95% confidence interval on `mean_fitness`, using the
    /// **sample** deviation (divides by `n - 1`) rather than `std_dev`'s
    /// population deviation — estimating the mean's uncertainty is a
    /// sample-deviation question even though `std_dev` beside it is not. Zero
    /// when the population has one individual, never `NaN`. Unconverted, like
    /// `std_dev`, for the same reason.
    pub ci_95: f64,
}

#[pymethods]
impl PyGenerationStats {
    fn __repr__(&self) -> String {
        format!(
            "GenerationStats(iteration={}, best_fitness={}, mean_fitness={}, std_dev={}, ci_95={})",
            self.iteration, self.best_fitness, self.mean_fitness, self.std_dev, self.ci_95,
        )
    }
}

/// Everything one run produced.
///
/// Returned by [`crate::GraphEvolver::run`]. There is deliberately no accessor on
/// the evolver for reading any of this: the run's state lives here, so the
/// evolver holds nothing stale from a previous run and is reusable across
/// repeated runs.
#[pyclass(name = "RunResult", frozen)]
#[derive(Debug)]
pub struct PyRunResult {
    /// Best fitness found, in the **objective's own units and sign**.
    #[pyo3(get)]
    pub best_fitness: f64,
    /// The best individual's expressed network, as `(u, v, multiplicity)`.
    #[pyo3(get)]
    pub best_edges: Vec<(usize, usize, u32)>,
    /// The best individual's genome, via `Genome::print`.
    ///
    /// This is the record of *which* individual won, in a form the entry point
    /// can carry without knowing the representation — which it cannot, since it
    /// is not generic over the genome.
    #[pyo3(get)]
    pub best_genome_repr: String,
    /// The convergence log, one row per logged iteration.
    ///
    /// Each access builds a fresh list of new `GenerationStats` objects, so bind
    /// it once (`rows = result.history`) rather than re-reading it in a loop.
    #[pyo3(get)]
    pub history: Vec<PyGenerationStats>,
}

#[pymethods]
impl PyRunResult {
    fn __repr__(&self) -> String {
        format!(
            "RunResult(best_fitness={}, {} edges, {} log rows)",
            self.best_fitness,
            self.best_edges.len(),
            self.history.len(),
        )
    }
}

impl PyRunResult {
    /// Wrap an erased outcome for the trip out to Python.
    ///
    /// No conversion happens here — `dispatch::erase` has already done it, and
    /// doing it twice would put a maximizing objective's numbers back into
    /// engine orientation while every one of them still looked plausible.
    pub(crate) fn from_erased(outcome: ErasedOutcome) -> Self {
        let mut history = Vec::with_capacity(outcome.history.len());
        for row in outcome.history {
            history.push(PyGenerationStats {
                iteration: row.iteration,
                best_fitness: row.best_fitness,
                mean_fitness: row.mean_fitness,
                std_dev: row.std_dev,
                ci_95: row.ci_95,
            });
        }

        Self {
            best_fitness: outcome.best_fitness,
            best_edges: outcome.best_edges,
            best_genome_repr: outcome.best_genome_repr,
            history,
        }
    }
}
