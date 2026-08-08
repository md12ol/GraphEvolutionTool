//! Python-facing builders for the `config.toml` schema (spec §8).
//!
//! These mirror [`crate::config`]'s types one field for one field. The user
//! fills them in Python, they serialize to TOML, and that TOML is what
//! [`crate::config::Config::from_toml_str`] parses — so there is exactly one
//! parser and one validator, and the Python path cannot accept a config the
//! hand-written TOML path rejects.
//!
//! # Why these are a separate mirror rather than `#[pyclass]` on `config`'s own types
//!
//! Not a stylistic choice — the two attribute sets are mutually exclusive on
//! the fitness enum, measured on pyo3 0.27.2 and serde 1.0.228:
//!
//! - pyo3 rejects a **unit variant** in a complex enum ("not yet supported in a
//!   complex enum; change to an empty tuple variant instead"), so
//!   [`crate::config::FitnessConfig::Python`] would have to become `Python()`.
//! - serde then rejects *that*: `#[serde(tag = "type")] cannot be used with
//!   tuple variants`. And the tag is what deserializes `type = "python"` for
//!   the hand-written TOML path.
//!
//! So annotating `config`'s enum directly would break the TOML front end to
//! serve the Python one. The mirror also confines every pyo3 attribute to this
//! one file, which matters with two owners editing the crate at once.
//!
//! The cost is drift: a field added to [`crate::config`] and not here is
//! invisible from Python. That is what the round-trip tests below guard — they
//! build a mirror, serialize it, parse it back as a real
//! [`crate::config::Config`], and compare field for field.

use std::path::PathBuf;

use pyo3::prelude::*;

/// Epidemic sampling parameters, shared by the three SIR objectives.
///
/// Mirrors [`crate::config::SirParams`], which is `#[serde(flatten)]`ed into
/// its variant — so this becomes keys of `[fitness]` directly, not a
/// sub-table.
///
/// Unrelated to [`crate::fitness::PyFitness`], despite both starting `Py`: that
/// one adapts a registered Python *callable* to the `Fitness` trait, this one
/// is configuration.
#[pyclass(name = "SirParams")]
#[derive(Debug, Clone)]
pub struct PySirParams {
    /// Per-edge transmission probability per timestep.
    #[pyo3(get, set)]
    pub infection_rate: f64,
    /// Pinned patient zero; left unset, a fresh node is drawn per epidemic.
    #[pyo3(get, set)]
    pub patient_zero: Option<usize>,
    /// Outbreaks averaged per evaluation.
    #[pyo3(get, set)]
    pub num_epidemics: usize,
    /// Outbreaks shorter than this are re-rolled; 1 disables the re-roll.
    #[pyo3(get, set)]
    pub min_epidemic_length: usize,
    /// Attempts before keeping whatever came out.
    #[pyo3(get, set)]
    pub max_epidemic_retries: usize,
}

#[pymethods]
impl PySirParams {
    /// The two defaults are the legacy C++ constants `mepl` and `rse`, so an
    /// omitted pair reproduces historical behaviour (spec §5.2) — the same
    /// values `config`'s serde defaults supply.
    #[new]
    #[pyo3(signature = (
        infection_rate,
        num_epidemics,
        patient_zero = None,
        min_epidemic_length = 3,
        max_epidemic_retries = 5,
    ))]
    fn new(
        infection_rate: f64,
        num_epidemics: usize,
        patient_zero: Option<usize>,
        min_epidemic_length: usize,
        max_epidemic_retries: usize,
    ) -> Self {
        Self {
            infection_rate,
            patient_zero,
            num_epidemics,
            min_epidemic_length,
            max_epidemic_retries,
        }
    }
}

/// Relative probability of each edge-edit operation.
///
/// Mirrors [`crate::genomes::EdgeEditOperationWeights`]. Every field defaults
/// to 1.0, giving all nine operations equal probability; a weight of 0.0
/// disables its operation outright.
#[pyclass(name = "OperationWeights")]
#[derive(Debug, Clone)]
pub struct PyOperationWeights {
    #[pyo3(get, set)]
    pub toggle: f64,
    #[pyo3(get, set)]
    pub hop: f64,
    #[pyo3(get, set)]
    pub add: f64,
    #[pyo3(get, set)]
    pub delete: f64,
    #[pyo3(get, set)]
    pub swap: f64,
    #[pyo3(get, set)]
    pub local_toggle: f64,
    #[pyo3(get, set)]
    pub local_add: f64,
    #[pyo3(get, set)]
    pub local_delete: f64,
    #[pyo3(get, set)]
    pub null: f64,
}

#[pymethods]
impl PyOperationWeights {
    #[new]
    #[pyo3(signature = (
        toggle = 1.0,
        hop = 1.0,
        add = 1.0,
        delete = 1.0,
        swap = 1.0,
        local_toggle = 1.0,
        local_add = 1.0,
        local_delete = 1.0,
        null = 1.0,
    ))]
    #[allow(clippy::too_many_arguments)] // nine weights is the schema, not a smell
    fn new(
        toggle: f64,
        hop: f64,
        add: f64,
        delete: f64,
        swap: f64,
        local_toggle: f64,
        local_add: f64,
        local_delete: f64,
        null: f64,
    ) -> Self {
        Self {
            toggle,
            hop,
            add,
            delete,
            swap,
            local_toggle,
            local_add,
            local_delete,
            null,
        }
    }
}

impl Default for PyOperationWeights {
    fn default() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0)
    }
}

/// Which evolution strategy to run, and its strategy-specific settings.
///
/// Mirrors [`crate::config::EvolutionConfig`].
#[pyclass(name = "EvolutionConfig")]
#[derive(Debug, Clone)]
pub enum PyEvolutionConfig {
    #[pyo3(constructor = (num_generations, elite_count = 1))]
    Generational {
        num_generations: usize,
        /// Best individuals carried forward each generation.
        elite_count: usize,
    },
    #[pyo3(constructor = (num_mating_events))]
    SteadyState { num_mating_events: usize },
}

/// Parent-selection strategy.
///
/// Mirrors [`crate::config::SelectionConfig`]. One variant today, and an enum
/// rather than a bare integer so adding a second scheme does not change the
/// shape of the Python API.
#[pyclass(name = "SelectionConfig")]
#[derive(Debug, Clone)]
pub enum PySelectionConfig {
    #[pyo3(constructor = (tournament_size))]
    Tournament { tournament_size: usize },
}

/// Genome representation and the dimensions used to build random individuals.
///
/// Mirrors [`crate::config::GenomeConfig`].
#[pyclass(name = "GenomeConfig")]
#[derive(Debug, Clone)]
pub enum PyGenomeConfig {
    #[pyo3(constructor = (gene_length, operation_weights = None))]
    EdgeEdit {
        gene_length: usize,
        /// Omitted entirely, every operation defaults to a weight of 1.0.
        operation_weights: Option<PyOperationWeights>,
    },
    /// No `num_chars`: the alphabet is derived as `max_edge_multiplicity + 1`,
    /// so every character is a legal edge weight (spec §3.2, GitHub #6).
    #[pyo3(constructor = (num_states, max_resp_len, init_state = 0))]
    Sda {
        num_states: usize,
        max_resp_len: usize,
        /// Must be `< num_states`; checked by `Config::validate`, since an
        /// out-of-range value panics during expression.
        init_state: usize,
    },
}

/// Fitness objective and its parameters.
///
/// Mirrors [`crate::config::FitnessConfig`]. The three epidemic objectives
/// read the same simulation differently (spec §5.2), so they share one
/// [`PySirParams`] block rather than triplicating it.
#[pyclass(name = "FitnessConfig")]
#[derive(Debug, Clone)]
pub enum PyFitnessConfig {
    /// Total ever-infected. Maximized.
    #[pyo3(constructor = (sir))]
    EpiSpread { sir: PySirParams },
    /// Timesteps to burn out. Maximized.
    #[pyo3(constructor = (sir))]
    EpiLength { sir: PySirParams },
    /// RMSE against a target profile. Minimized.
    #[pyo3(constructor = (sir, target_profile_path))]
    EpiProfMatch {
        sir: PySirParams,
        /// File holding the target profile. Stored, never opened here —
        /// validating a config stays pure (spec §7).
        target_profile_path: PathBuf,
    },
    /// A Python callable registered before the run, via
    /// `GraphEvolver.set_fitness_function`. Its direction is declared at
    /// registration, not here (spec §7).
    ///
    /// An empty tuple variant rather than a unit one because pyo3 rejects unit
    /// variants in a complex enum — see this module's header.
    #[pyo3(constructor = ())]
    Python(),
}

/// Everything the genetic algorithm needs for a run.
///
/// Mirrors [`crate::config::Config`]. Serializing this is what produces the
/// TOML the Rust side parses; it is deliberately not a second parser of that
/// format (spec §8).
#[pyclass(name = "Config")]
#[derive(Debug, Clone)]
pub struct PyConfig {
    /// Which evolution strategy to run.
    #[pyo3(get, set)]
    pub evolution: PyEvolutionConfig,
    /// Number of individuals in the population.
    #[pyo3(get, set)]
    pub population_size: usize,
    /// Number of nodes in every expressed graph.
    #[pyo3(get, set)]
    pub network_size: usize,
    /// Edge-weight cap; 1 is unweighted.
    #[pyo3(get, set)]
    pub max_edge_multiplicity: u32,
    /// Probability that a selected pair is recombined.
    #[pyo3(get, set)]
    pub crossover_rate: f64,
    /// Probability that a child is mutated at all.
    #[pyo3(get, set)]
    pub mutation_rate: f64,
    /// How many mutations a mutating child takes, drawn uniformly from
    /// `1..=max_mutations`.
    #[pyo3(get, set)]
    pub max_mutations: usize,
    /// Parent-selection strategy.
    #[pyo3(get, set)]
    pub selection: PySelectionConfig,
    /// Genome representation and its dimensions.
    #[pyo3(get, set)]
    pub genome: PyGenomeConfig,
    /// Fitness objective.
    #[pyo3(get, set)]
    pub fitness: PyFitnessConfig,
}

#[pymethods]
impl PyConfig {
    /// The two defaulted arguments carry the same defaults as `config`'s serde
    /// attributes: an unweighted graph, and one mutation per mutating child.
    ///
    /// No `seed`: one master seed is supplied to the `run` call and everything
    /// derives from it (spec §7, §8.1).
    #[new]
    #[pyo3(signature = (
        evolution,
        population_size,
        network_size,
        crossover_rate,
        mutation_rate,
        selection,
        genome,
        fitness,
        max_edge_multiplicity = 1,
        max_mutations = 1,
    ))]
    #[allow(clippy::too_many_arguments)] // ten fields is the schema, not a smell
    fn new(
        evolution: PyEvolutionConfig,
        population_size: usize,
        network_size: usize,
        crossover_rate: f64,
        mutation_rate: f64,
        selection: PySelectionConfig,
        genome: PyGenomeConfig,
        fitness: PyFitnessConfig,
        max_edge_multiplicity: u32,
        max_mutations: usize,
    ) -> Self {
        Self {
            evolution,
            population_size,
            network_size,
            max_edge_multiplicity,
            crossover_rate,
            mutation_rate,
            max_mutations,
            selection,
            genome,
            fitness,
        }
    }
}
