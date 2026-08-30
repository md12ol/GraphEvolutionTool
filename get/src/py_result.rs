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
#[pyclass(name = "GenerationStats", module = "get", frozen, get_all)]
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
/// Returned by `GraphEvolver.run`. The evolver keeps none of it, so it is
/// reusable across runs and never reports a previous one's numbers.
#[pyclass(name = "RunResult", module = "get", frozen)]
#[derive(Debug)]
pub struct PyRunResult {
    /// Best of the **final** population, **as-measured** — the units and sign
    /// your objective returned. Matches `history`'s last row, which a
    /// stochastic objective may have scored worse than an earlier one.
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
    /// The TOML document this run's config was parsed from. `save_config`
    /// writes it into a folder, so the run can be reproduced.
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

    /// Write the best individual to `filename`.
    ///
    /// **The file is a loadable edge list**, which GET reads back unedited.
    ///
    /// The config that produced it is not written here — it belongs to the
    /// whole invocation rather than to one replicate, so `save_config` writes
    /// it once into the folder the replicates share.
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

        Ok(())
    }

    /// Write the run's config TOML into `directory` as `config.toml`.
    ///
    /// Called once per invocation rather than once per replicate: every
    /// replicate of one invocation was produced by the same document, and a
    /// copy beside each would be the same bytes N times.
    pub fn save_config(&self, directory: &str) -> PyResult<()> {
        let config_path = std::path::Path::new(directory).join("config.toml");
        std::fs::write(&config_path, &self.config_toml).map_err(|err| {
            PyIOError::new_err(format!(
                "could not write to {}: {err}",
                config_path.display()
            ))
        })
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

    /// A result file is a **loadable edge list**, not a report that contains
    /// one — that is what lets a run's winner go straight back in as the next
    /// run's base graph or into a reference folder with no editing step. Only
    /// `as_comment` was covered, so nothing checked the claim itself: the
    /// header, the `# nodes` line and the rows all have to survive the parser
    /// that will actually read them.
    #[test]
    fn a_saved_result_loads_back_through_the_edge_list_parser() {
        let result = super::PyRunResult {
            best_fitness: -12.5,
            // Node 4 is isolated: it appears in no edge, so only the header
            // can carry it, which is why num_nodes is stored rather than
            // inferred.
            best_edges: vec![(0, 1, 2), (1, 3, 1)],
            num_nodes: 5,
            best_genome_repr: "init_char: 0\n0 + 0 -> 1 [ 1 1 1 ]".to_string(),
            history: Vec::new(),
            seed: 7,
            run_index: 0,
            config_toml: "population_size = 10\n".to_string(),
        };

        let folder = std::env::temp_dir().join("get_py_result_roundtrip");
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("temp folder");
        let path = folder.join("best.csv");

        result
            .save_results(path.to_str().expect("utf-8 path"))
            .expect("the result should save");

        let loaded = crate::graph_io::load_edge_file(&path, 10, 2, 0)
            .expect("a saved result must be a file the loader accepts");

        assert_eq!(loaded.num_nodes, 5, "the isolated node must survive");
        assert_eq!(loaded.edges, vec![(0, 1, 2), (1, 3, 1)]);
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);

        // The provenance TOML is written by `save_config`, into a folder of
        // its own choosing rather than derived from the results file's name.
        result
            .save_config(folder.to_str().expect("temp path is valid UTF-8"))
            .expect("save_config writes successfully");
        let config = std::fs::read_to_string(folder.join("config.toml"))
            .expect("the config should be written into the folder");
        assert_eq!(config, "population_size = 10\n");

        std::fs::remove_dir_all(&folder).expect("cleanup");
    }

    /// The CSV header names seven columns and each row writes seven values.
    /// Nothing checked they agree, and a mismatch is silent — every consumer
    /// reads the file by column position.
    #[test]
    fn every_log_row_has_one_value_per_column_in_the_header() {
        let result = super::PyRunResult {
            best_fitness: 1.0,
            best_edges: Vec::new(),
            num_nodes: 2,
            best_genome_repr: String::new(),
            history: vec![
                super::PyGenerationStats {
                    iteration: 0,
                    best_fitness: 1.0,
                    mean_fitness: 2.0,
                    std_dev: 0.5,
                    ci_95: 0.25,
                },
                super::PyGenerationStats {
                    iteration: 10,
                    best_fitness: 0.5,
                    mean_fitness: 1.5,
                    std_dev: 0.25,
                    ci_95: 0.125,
                },
            ],
            seed: 99,
            run_index: 0,
            config_toml: String::new(),
        };

        let folder = std::env::temp_dir().join("get_py_result_logs");
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("temp folder");
        let path = folder.join("logs.csv");

        result
            .save_logs(path.to_str().expect("utf-8 path"))
            .expect("the logs should save");

        let text = std::fs::read_to_string(&path).expect("written");
        let mut lines = text.lines();
        let header = lines.next().expect("a header row");
        let columns = header.split(',').count();
        assert_eq!(columns, 7, "{header}");

        let mut rows = 0;
        for line in lines {
            assert_eq!(line.split(',').count(), columns, "row: {line}");
            rows += 1;
        }
        assert_eq!(rows, 2, "one row per logged iteration");

        // The run-level values are stamped onto every row rather than carried
        // per row in memory.
        assert!(text.lines().nth(1).expect("first row").ends_with("99,0"));

        std::fs::remove_dir_all(&folder).expect("cleanup");
    }
}
