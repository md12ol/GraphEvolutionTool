pub mod config;
pub mod evolver;
pub mod fitness;
pub mod genomes;
pub mod graph;
pub mod sir;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::config::Config;

/// Python-facing entry point to the graph-evolution engine.
///
/// Constructed from a `config.toml` path; [`GraphEvolver::run`] dispatches on
/// the configured evolution strategy, genome representation, and fitness
/// objective, then returns the best graph found.
#[pyclass]
pub struct GraphEvolver {
    config: Config,
    best_fitness: Option<f64>,
}

#[pymethods]
impl GraphEvolver {
    /// Load configuration from a `config.toml` file.
    #[new]
    fn new(config_path: String) -> PyResult<Self> {
        let config = Config::from_path(&config_path)
            .map_err(|err| PyValueError::new_err(format!("failed to load config: {err:?}")))?;
        Ok(Self {
            config,
            best_fitness: None,
        })
    }

    /// Evolve a population and return the best graph as a weighted edge list
    /// `(u, v, multiplicity)`.
    fn run(&mut self, seed: u64) -> PyResult<Vec<(usize, usize, u32)>> {
        let _ = (seed, &self.config, &mut self.best_fitness);
        todo!(
            "dispatch on config (evolution x genome x fitness), run the evolver, \
             cache best_fitness, and return the best graph's edge list"
        )
    }

    /// Best fitness found so far, or infinity before any run completes.
    fn best_fitness(&self) -> f64 {
        self.best_fitness.unwrap_or(f64::INFINITY)
    }

    /// Write the per-iteration evolution log to `filename` as CSV.
    fn save_logs(&self, filename: &str) -> PyResult<()> {
        let _ = filename;
        todo!("write the run history to `filename`")
    }

    /// Write the best individual and its graph to `filename`.
    fn save_results(&self, filename: &str) -> PyResult<()> {
        let _ = filename;
        todo!("write the best genome and edge list to `filename`")
    }
}

#[pymodule]
fn get(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<GraphEvolver>()?;
    Ok(())
}
