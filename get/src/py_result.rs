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

use std::fs::File;
use std::io::Write;

use pyo3::exceptions::PyIOError;
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
    /// The seed `run` was called with.
    ///
    /// Run-level rather than per-row: it lives here once and `save_logs`
    /// stamps it onto every CSV row it writes, rather than every in-memory
    /// row carrying its own copy of a value that never varies within a run.
    #[pyo3(get)]
    pub seed: u64,
    /// Which replicate this is, `0`-based.
    ///
    /// A hard `0` until GitHub #20 gives `run` more than one replicate to
    /// number — reserved now so the CSV schema does not change under users
    /// once it does.
    #[pyo3(get)]
    pub run_index: usize,
    /// The TOML document this run's config was parsed from — the provenance
    /// record `save_results` writes alongside the best individual, so the run
    /// can be reproduced verbatim.
    #[pyo3(get)]
    pub config_toml: String,
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

    /// Write the convergence log to `filename` as CSV.
    ///
    /// Header, then one row per logged iteration: §6.4's five columns, then
    /// `seed` and `run_index` last — both are run-level and so identical on
    /// every row, which is what lets several runs' logs be concatenated and
    /// still be separable.
    pub fn save_logs(&self, filename: &str) -> PyResult<()> {
        let mut file = File::create(filename)
            .map_err(|err| PyIOError::new_err(format!("could not create {filename}: {err}")))?;

        writeln!(
            file,
            "iteration,best_fitness,mean_fitness,std_dev,ci_95,seed,run_index"
        )
        .map_err(|err| PyIOError::new_err(format!("could not write to {filename}: {err}")))?;

        for row in &self.history {
            writeln!(
                file,
                "{},{},{},{},{},{},{}",
                row.iteration,
                row.best_fitness,
                row.mean_fitness,
                row.std_dev,
                row.ci_95,
                self.seed,
                self.run_index,
            )
            .map_err(|err| PyIOError::new_err(format!("could not write to {filename}: {err}")))?;
        }

        Ok(())
    }

    /// Write the best individual to `filename`, and the run's config TOML
    /// alongside it at `{filename}.toml` — the provenance record §8 promises,
    /// derived rather than a second argument so callers cannot forget it.
    ///
    /// Three sections: the best fitness, the winning genome's
    /// `Genome::print()` string, and its expressed network as a weighted edge
    /// list — §6.4's "best individual".
    pub fn save_results(&self, filename: &str) -> PyResult<()> {
        let mut file = File::create(filename)
            .map_err(|err| PyIOError::new_err(format!("could not create {filename}: {err}")))?;

        writeln!(file, "best_fitness = {}", self.best_fitness)
            .map_err(|err| PyIOError::new_err(format!("could not write to {filename}: {err}")))?;
        writeln!(file, "genome = {}", self.best_genome_repr)
            .map_err(|err| PyIOError::new_err(format!("could not write to {filename}: {err}")))?;
        writeln!(file, "\nedges (u,v,multiplicity):")
            .map_err(|err| PyIOError::new_err(format!("could not write to {filename}: {err}")))?;
        for &(u, v, weight) in &self.best_edges {
            writeln!(file, "{u},{v},{weight}").map_err(|err| {
                PyIOError::new_err(format!("could not write to {filename}: {err}"))
            })?;
        }

        let config_path = format!("{filename}.toml");
        std::fs::write(&config_path, &self.config_toml).map_err(|err| {
            PyIOError::new_err(format!("could not write to {config_path}: {err}"))
        })?;

        Ok(())
    }
}

impl PyRunResult {
    /// Wrap an erased outcome for the trip out to Python.
    ///
    /// No conversion happens here — `dispatch::erase` has already done it, and
    /// doing it twice would put a maximizing objective's numbers back into
    /// engine orientation while every one of them still looked plausible.
    ///
    /// `seed`, `run_index` and `config_toml` are run-level rather than part of
    /// `ErasedOutcome`: dispatch knows nothing of the config document or which
    /// replicate it's building, so those three arrive from the caller instead.
    pub(crate) fn from_erased(
        outcome: ErasedOutcome,
        seed: u64,
        run_index: usize,
        config_toml: String,
    ) -> Self {
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
            seed,
            run_index,
            config_toml,
        }
    }
}
