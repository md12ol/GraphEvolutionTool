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
//! converts anything; the dispatch layer's `erase` is the one place that
//! happens.

use std::fs::File;
use std::io::Write;

use pyo3::exceptions::PyIOError;
use pyo3::prelude::*;

use crate::dispatch::ErasedOutcome;

/// Turn a value into `#` comment lines, **every** line of it.
///
/// `Genome::print` is free to return several lines — `SdaGenome`'s is a whole
/// transition table — and a saved result is a loadable edge file, so a line
/// that escaped the `#` would reach the parser as a malformed row. Commenting
/// only the first line is exactly the bug this exists to prevent.
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
    ///
    /// This is the best of the **final** population, not the best ever seen —
    /// it matches `history`'s last row rather than the highest `best_fitness`
    /// across every row. Under a stochastic objective an earlier generation
    /// can score higher by a lucky draw that a later, better-adapted
    /// individual does not repeat; reporting that draw instead would credit
    /// noise rather than the search.
    #[pyo3(get)]
    pub best_fitness: f64,
    /// The best individual's expressed network, as `(u, v, multiplicity)`.
    #[pyo3(get)]
    pub best_edges: Vec<(usize, usize, u32)>,
    /// How many nodes that network has, isolated ones included.
    ///
    /// Not derivable from `best_edges` — an isolated node appears in no edge —
    /// which is the same reason a file has to state its size rather than have
    /// it inferred.
    #[pyo3(get)]
    pub num_nodes: usize,
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
    /// **The file is a loadable edge list**, not a report that happens to
    /// contain one: the fitness, the genome string and the node count are `#`
    /// comments, which `graph_io` skips, and the rows below them are the
    /// `u,v,weight` the loader reads. So a run's winner goes straight back in
    /// as the next run's base graph, or into a reference folder, with no
    /// editing step in between — and the `# nodes` line the loader requires is
    /// written from the graph rather than left for someone to count.
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

        // The provenance TOML is written beside it, under a derived name.
        let config = std::fs::read_to_string(folder.join("best.csv.toml"))
            .expect("the config should be written alongside");
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
