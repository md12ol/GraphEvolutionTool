//! What a run hands back to Python: [`PyRunResult`] and its log rows.
//!
//! **Numbers here are already in the objective's units and sign.** They are
//! converted on the way in, so converting again would flip every result.

use std::fs::File;
use std::io::Write;

use pyo3::exceptions::PyIOError;
use pyo3::prelude::*;

use crate::dispatch::ErasedOutcome;

/// Turn a value into `#` comment lines, **every** line of it.
///
/// `Genome::print` may return many lines and the file is a loadable edge list,
/// so a line that escaped the `#` would reach the parser as a malformed row.
pub(crate) fn as_comment(label: &str, value: &str) -> String {
    let mut out = String::new();
    let mut first = true;
    for line in value.lines() {
        if first {
            out.push_str(&format!("# {label} = {line}\n"));
            first = false;
        } else {
            out.push_str(&format!("# {line}\n"));
        }
    }
    // An empty value still has to say the field was there.
    if first {
        out.push_str(&format!("# {label} =\n"));
    }
    out
}

/// One row of the convergence log.
///
/// `iteration` counts generations under the generational strategy and mating
/// events under steady-state.
#[pyclass(name = "GenerationStats", frozen, get_all)]
#[derive(Debug, Clone)]
pub struct PyGenerationStats {
    /// Generation number, or mating-event number.
    pub iteration: usize,
    /// Best fitness in the population at this iteration.
    pub best_fitness: f64,
    /// Population mean fitness at this iteration.
    pub mean_fitness: f64,
    /// **Population** standard deviation, dividing by `n`. Zero for a
    /// population of one.
    pub std_dev: f64,
    /// Half-width of the 95% confidence interval on `mean_fitness`, using the
    /// **sample** deviation, dividing by `n - 1` — not `std_dev` beside it.
    /// Zero for a population of one, never `NaN`.
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
/// Returned by [`crate::GraphEvolver::run`]. The evolver keeps none of it, so it
/// is reusable across runs and never reports a previous one's numbers.
#[pyclass(name = "RunResult", frozen)]
#[derive(Debug)]
pub struct PyRunResult {
    /// Best of the **final** population, in the **objective's own units and
    /// sign**. Matches `history`'s last row, which a stochastic objective may
    /// have scored worse than an earlier one.
    #[pyo3(get)]
    pub best_fitness: f64,
    /// The best individual's expressed network, as `(u, v, multiplicity)`.
    #[pyo3(get)]
    pub best_edges: Vec<(usize, usize, u32)>,
    /// How many nodes that network has, isolated ones included.
    #[pyo3(get)]
    pub num_nodes: usize,
    /// The best individual's genome, via `Genome::print`.
    #[pyo3(get)]
    pub best_genome_repr: String,
    /// The convergence log, one row per logged iteration.
    #[pyo3(get)]
    pub history: Vec<PyGenerationStats>,
    /// The seed `run` was called with.
    #[pyo3(get)]
    pub seed: u64,
    /// Which replicate this is, `0`-based. With `seed`, the pair that
    /// reproduces this exact run.
    #[pyo3(get)]
    pub run_index: usize,
    /// The TOML document this run's config was parsed from. `save_results`
    /// writes it beside the best individual, so the run can be reproduced.
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
    /// Every row carries `seed` and `run_index`, so logs from several runs
    /// concatenate into one file and stay separable.
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
    /// alongside it at `{filename}.toml`.
    ///
    /// **The file is a loadable edge list**, which GET reads back unedited.
    pub fn save_results(&self, filename: &str) -> PyResult<()> {
        let mut file = File::create(filename)
            .map_err(|err| PyIOError::new_err(format!("could not create {filename}: {err}")))?;

        write!(
            file,
            "{}",
            as_comment("best_fitness", &self.best_fitness.to_string())
        )
        .map_err(|err| PyIOError::new_err(format!("could not write to {filename}: {err}")))?;
        write!(file, "{}", as_comment("genome", &self.best_genome_repr))
            .map_err(|err| PyIOError::new_err(format!("could not write to {filename}: {err}")))?;
        writeln!(file, "# nodes = {}", self.num_nodes)
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
    /// Wrap a run's outcome, its genome type already dropped, for the trip out
    /// to Python.
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
            num_nodes: outcome.num_nodes,
            best_genome_repr: outcome.best_genome_repr,
            history,
            seed,
            run_index,
            config_toml,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::as_comment;

    /// `SdaGenome::print` returns a transition table, several lines of it. A
    /// saved result is a loadable edge file, so a line that escaped the `#`
    /// would reach the parser as a malformed row — which is exactly what
    /// commenting only the first line did.
    #[test]
    fn every_line_of_a_multi_line_value_is_commented() {
        let genome = "init_char: 0\n0 + 0 -> 1 [ 1 1 1 ]\n0 + 1 -> 1 [ 0 1 ]";

        let written = as_comment("genome", genome);

        assert_eq!(
            written,
            "# genome = init_char: 0\n# 0 + 0 -> 1 [ 1 1 1 ]\n# 0 + 1 -> 1 [ 0 1 ]\n"
        );
        for line in written.lines() {
            assert!(line.starts_with('#'), "escaped the comment: {line}");
        }
    }

    #[test]
    fn an_empty_value_still_names_its_field() {
        assert_eq!(as_comment("genome", ""), "# genome =\n");
    }
}
