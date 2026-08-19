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
//!
//! # What this looks like from Python
//!
//! ```python
//! config = get.Config(
//!     population_size=200,
//!     network_size=100,
//!     crossover_rate=0.9,
//!     mutation_rate=0.2,
//!     evolution=get.EvolutionConfig.Generational(num_generations=500),
//!     selection=get.SelectionConfig.Tournament(tournament_size=5),
//!     genome=get.GenomeConfig.EdgeEdit(gene_length=256),
//!     fitness=get.FitnessConfig.EpiSpread(
//!         sir=get.SirParams(infection_rate=0.05, num_epidemics=30)
//!     ),
//! )
//! evolver = get.GraphEvolver.from_config(config)
//! ```
//!
//! Worked examples for every objective, both genomes and both evolution
//! strategies live in `examples/config_builder.py`, which is runnable and is
//! the place to add a new one — not this comment, which cannot be executed and
//! so cannot be caught going stale.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use toml::Value;
use toml::map::Map;

/// Epidemic sampling parameters, shared by the three SIR objectives.
///
/// Mirrors [`crate::config::SirParams`], which is `#[serde(flatten)]`ed into
/// its variant — so this becomes keys of `[fitness]` directly, not a
/// sub-table.
///
/// Unrelated to [`crate::fitness::PyFitness`], despite both starting `Py`: that
/// one adapts a registered Python *callable* to the `Fitness` trait, this one
/// is configuration.
///
/// None of these five fields is validated here — `PySirParams` carries no
/// range checks of its own. All of them are checked later by
/// `Config::validate_fitness`, once this has round-tripped through TOML,
/// the same deferral `PyGenomeConfig::Sda::init_state` uses.
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
    pub fn new(
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
    pub toggle: f64,
    pub hop: f64,
    pub add: f64,
    pub delete: f64,
    pub swap: f64,
    pub local_toggle: f64,
    pub local_add: f64,
    pub local_delete: f64,
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
    pub fn new(
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
    /// `elite_count`'s default of 1 carries the same value as `config`'s
    /// `default_elite_count`; there's no shared constant, so a change to one
    /// side needs the other updated by hand.
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
///
/// This is step 7 of the eight a new scheme touches, and the only one nothing
/// checks: [`crate::evolver::common::Selection`] lists them all. A variant
/// missing here compiles and tests clean — the Rust side is complete and Python
/// simply cannot name the scheme.
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
    /// `operation_weights` is `Option`-wrapped because it's a whole nested
    /// type with its own defaults, not a scalar a `#[pyo3(constructor)]`
    /// default can hand back directly — contrast `Sda::init_state` below,
    /// where the default *is* a plain `usize` and needs no `Option`.
    #[pyo3(constructor = (gene_length, operation_weights = None))]
    EdgeEdit {
        gene_length: usize,
        /// Omitted entirely, every operation defaults to a weight of 1.0.
        operation_weights: Option<PyOperationWeights>,
    },
    /// No `num_chars`: the alphabet is derived as `max_edge_multiplicity + 1`,
    /// so every character is a legal edge weight (spec §3.2, GitHub #6).
    #[pyo3(constructor = (
        num_states,
        max_resp_len,
        init_state = 0,
        init_char_mutation_rate = None,
        transition_vs_response_rate = None,
    ))]
    Sda {
        num_states: usize,
        max_resp_len: usize,
        /// Defaults to 0, matching `config`'s `#[serde(default)]` (a bare
        /// `usize` defaults to 0, so there's no named default fn to track
        /// here the way `elite_count` has one). Must be `< num_states`;
        /// checked by `Config::validate`, since an out-of-range value panics
        /// during expression.
        init_state: usize,
        /// Chance a mutation redraws the initial character instead of
        /// touching the transition table. `Option`-wrapped so an unset value
        /// is left out of the TOML entirely and `config`'s own default
        /// supplies it — writing a number here would mean the default lived
        /// in two places.
        init_char_mutation_rate: Option<f64>,
        /// Chance of redrawing a transition's target rather than its
        /// response, once the initial character was not chosen.
        /// `Option`-wrapped for the same reason.
        transition_vs_response_rate: Option<f64>,
    },
}

/// Fitness objective and its parameters.
///
/// Mirrors [`crate::config::FitnessConfig`]. The three epidemic objectives
/// read the same simulation differently (spec §5.2), so they share one
/// [`PySirParams`] block rather than triplicating it.
///
/// # Part of the chain that adds an objective
///
/// This is the optional step: it is what lets a Python caller name the
/// objective. Skipping it costs nothing anywhere else — the objective still
/// runs from a TOML config and from Rust — so it is only needed if Python
/// should be able to select it. The steps before it are the `FitnessConfig`
/// variant and its `dispatch` arm; `crate::fitness`'s module doc has all six.
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
    #[pyo3(constructor = (sir, target_profile))]
    EpiProfMatch {
        sir: PySirParams,
        /// The target profile itself, as a Python list of numbers (spec §8).
        /// Compared verbatim — see [`crate::config::FitnessConfig`].
        target_profile: Vec<f64>,
    },
    /// How closely a graph's structure matches a set of reference graphs.
    /// Minimized; requires `max_edge_multiplicity = 1`.
    ///
    /// Every parameter but the folder has a default, matching the TOML side —
    /// see [`crate::config::FitnessConfig`] for what each one does and for the
    /// two ways a reference set can retire a family without saying so.
    #[pyo3(constructor = (
        reference_folder,
        degree_bins = 50,
        clustering_bins = 50,
        spectral_bins = 50,
        degree_gamma = 1.0,
        clustering_gamma = 1.0,
        spectral_gamma = 1.0,
        degree_weight = 1.0,
        clustering_weight = 1.0,
        spectral_weight = 1.0,
        density_weight = 1.0,
    ))]
    StructMatch {
        reference_folder: String,
        degree_bins: usize,
        clustering_bins: usize,
        spectral_bins: usize,
        degree_gamma: f64,
        clustering_gamma: f64,
        spectral_gamma: f64,
        degree_weight: f64,
        clustering_weight: f64,
        spectral_weight: f64,
        density_weight: f64,
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
    pub fn new(
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

    /// Render this config as the TOML document the Rust side parses.
    ///
    /// Spec §8 calls this out as provenance: the generated text *is* the record
    /// of what was run, so writing it beside the results re-runs verbatim.
    ///
    /// Also the seam whatever builds a [`crate::config::Config`] from this
    /// calls, so the document a user inspects for provenance is byte-for-byte
    /// the one that was parsed.
    ///
    /// # Errors
    ///
    /// `ValueError` if a field is too large for a TOML integer, whose maximum
    /// is `i64::MAX`.
    pub fn to_toml(&self) -> PyResult<String> {
        let document = Value::Table(self.to_toml_table()?);
        toml::to_string(&document).map_err(|err| {
            PyValueError::new_err(format!("could not render the config as TOML: {err}"))
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "Config(population_size={}, network_size={}, max_edge_multiplicity={})",
            self.population_size, self.network_size, self.max_edge_multiplicity,
        )
    }
}

/// The Python attribute path a validation field name corresponds to.
///
/// [`crate::config::Config::validate`] names the offending field as it appears
/// in the TOML document, which is the right answer for the file front end and a
/// poor one here: a user who wrote `SirParams(num_epidemics=0)` is told
/// `num_epidemics` and left to work out which of the objects they assembled
/// owns it. Spec §8 calls this out — errors referencing a document the user
/// never wrote need mapping back to the attribute that produced them.
///
/// Returns `None` for a field with no Python equivalent, and the caller then
/// falls back to the unmapped name rather than inventing a path. Today the only
/// such field is `seed`, which `reject_stray_fitness_keys` raises against raw TOML
/// text and which this front end cannot produce — [`PyConfig`] has no seed to
/// write (spec §7: one master seed, supplied to `run`).
///
/// Every name here is checked against `config.rs`'s actual `invalid(...)` calls
/// by `every_validation_field_maps_to_a_python_attribute` below, so a check
/// added there without a mapping here fails the suite rather than silently
/// degrading to a bare field name.
fn python_attribute_path(field: &str) -> Option<&'static str> {
    match field {
        // Directly on the config object.
        "population_size" => Some("config.population_size"),
        "max_edge_multiplicity" => Some("config.max_edge_multiplicity"),
        "crossover_rate" => Some("config.crossover_rate"),
        "mutation_rate" => Some("config.mutation_rate"),
        "max_mutations" => Some("config.max_mutations"),
        // On the strategy and selection objects.
        "elite_count" => Some("config.evolution.elite_count"),
        "tournament_size" => Some("config.selection.tournament_size"),
        // On the genome object; each belongs to one variant.
        "operation_weights" => Some("config.genome.operation_weights"),
        "init_state" => Some("config.genome.init_state"),
        // Raised from a loop in `validate_genome` rather than as a literal at
        // the call site — the second of the two shapes the scraper below reads.
        "init_char_mutation_rate" => Some("config.genome.init_char_mutation_rate"),
        "transition_vs_response_rate" => Some("config.genome.transition_vs_response_rate"),
        // On the SIR block, which every epidemic objective reaches the same
        // way even though it flattens into `[fitness]` in the document.
        "infection_rate" => Some("config.fitness.sir.infection_rate"),
        "num_epidemics" => Some("config.fitness.sir.num_epidemics"),
        "min_epidemic_length" => Some("config.fitness.sir.min_epidemic_length"),
        "max_epidemic_retries" => Some("config.fitness.sir.max_epidemic_retries"),
        "patient_zero" => Some("config.fitness.sir.patient_zero"),
        // On the objective itself rather than the shared SIR block — only
        // `epi_prof_match` has one.
        "target_profile" => Some("config.fitness.target_profile"),
        // `struct_match`'s own block. All are raised from loops in
        // `validate_struct_match` rather than as literals at the call site.
        "reference_folder" => Some("config.fitness.reference_folder"),
        "degree_bins" => Some("config.fitness.degree_bins"),
        "clustering_bins" => Some("config.fitness.clustering_bins"),
        "spectral_bins" => Some("config.fitness.spectral_bins"),
        "degree_gamma" => Some("config.fitness.degree_gamma"),
        "clustering_gamma" => Some("config.fitness.clustering_gamma"),
        "spectral_gamma" => Some("config.fitness.spectral_gamma"),
        "degree_weight" => Some("config.fitness.degree_weight"),
        "clustering_weight" => Some("config.fitness.clustering_weight"),
        "spectral_weight" => Some("config.fitness.spectral_weight"),
        "density_weight" => Some("config.fitness.density_weight"),
        _ => None,
    }
}

/// Report a [`crate::config::ConfigError`] to Python, naming the attribute that
/// caused it wherever one can be identified.
pub fn config_error_to_py(error: &crate::config::ConfigError) -> PyErr {
    if let crate::config::ConfigError::Validation { field, constraint } = error
        && let Some(path) = python_attribute_path(field)
    {
        return PyValueError::new_err(format!("invalid config: `{path}` {constraint}"));
    }

    // Anything else already reads correctly without a document to point at.
    PyValueError::new_err(error.to_string())
}

/// A `usize` as a TOML integer.
///
/// TOML integers are `i64`, so this is not infallible on a 64-bit `usize`, and
/// the gap is reachable from Python: `population_size=2**63` converts into
/// `usize` happily and only fails here. Reported against the field rather than
/// wrapping to a negative number, which would be a silently wrong config.
fn integer(field: &str, value: usize) -> PyResult<Value> {
    match i64::try_from(value) {
        Ok(number) => Ok(Value::Integer(number)),
        Err(_) => Err(PyValueError::new_err(format!(
            "{field}: {value} is too large for a TOML integer (the maximum is {})",
            i64::MAX,
        ))),
    }
}

impl PySirParams {
    /// Write this block's keys *into* `table`.
    ///
    /// Not a table of its own: [`crate::config::SirParams`] is
    /// `#[serde(flatten)]`ed into its variant, so these are keys of
    /// `[fitness]` directly.
    fn flatten_into(&self, table: &mut Map<String, Value>) -> PyResult<()> {
        table.insert(
            "infection_rate".to_string(),
            Value::Float(self.infection_rate),
        );
        table.insert(
            "num_epidemics".to_string(),
            integer("num_epidemics", self.num_epidemics)?,
        );
        table.insert(
            "min_epidemic_length".to_string(),
            integer("min_epidemic_length", self.min_epidemic_length)?,
        );
        table.insert(
            "max_epidemic_retries".to_string(),
            integer("max_epidemic_retries", self.max_epidemic_retries)?,
        );
        // Omitted when unset, matching `#[serde(default)]` on the Rust side —
        // emitting a null would not parse, and emitting a sentinel would pin
        // patient zero to a node the user never chose.
        if let Some(node) = self.patient_zero {
            table.insert("patient_zero".to_string(), integer("patient_zero", node)?);
        }
        Ok(())
    }
}

impl PyOperationWeights {
    fn to_toml_table(&self) -> Map<String, Value> {
        let mut table = Map::new();
        table.insert("toggle".to_string(), Value::Float(self.toggle));
        table.insert("hop".to_string(), Value::Float(self.hop));
        table.insert("add".to_string(), Value::Float(self.add));
        table.insert("delete".to_string(), Value::Float(self.delete));
        table.insert("swap".to_string(), Value::Float(self.swap));
        table.insert("local_toggle".to_string(), Value::Float(self.local_toggle));
        table.insert("local_add".to_string(), Value::Float(self.local_add));
        table.insert("local_delete".to_string(), Value::Float(self.local_delete));
        table.insert("null".to_string(), Value::Float(self.null));
        table
    }
}

impl PyEvolutionConfig {
    fn to_toml_value(&self) -> PyResult<Value> {
        let mut table = Map::new();
        match self {
            PyEvolutionConfig::Generational {
                num_generations,
                elite_count,
            } => {
                table.insert(
                    "type".to_string(),
                    Value::String("generational".to_string()),
                );
                table.insert(
                    "num_generations".to_string(),
                    integer("num_generations", *num_generations)?,
                );
                table.insert(
                    "elite_count".to_string(),
                    integer("elite_count", *elite_count)?,
                );
            }
            PyEvolutionConfig::SteadyState { num_mating_events } => {
                table.insert(
                    "type".to_string(),
                    Value::String("steady_state".to_string()),
                );
                table.insert(
                    "num_mating_events".to_string(),
                    integer("num_mating_events", *num_mating_events)?,
                );
            }
        }
        Ok(Value::Table(table))
    }
}

impl PySelectionConfig {
    fn to_toml_value(&self) -> PyResult<Value> {
        let mut table = Map::new();
        match self {
            PySelectionConfig::Tournament { tournament_size } => {
                table.insert("type".to_string(), Value::String("tournament".to_string()));
                table.insert(
                    "tournament_size".to_string(),
                    integer("tournament_size", *tournament_size)?,
                );
            }
        }
        Ok(Value::Table(table))
    }
}

impl PyGenomeConfig {
    fn to_toml_value(&self) -> PyResult<Value> {
        let mut table = Map::new();
        match self {
            PyGenomeConfig::EdgeEdit {
                gene_length,
                operation_weights,
            } => {
                table.insert("type".to_string(), Value::String("edge_edit".to_string()));
                table.insert(
                    "gene_length".to_string(),
                    integer("gene_length", *gene_length)?,
                );
                // Left out entirely when unset, so serde's `Default` supplies
                // all nine weights rather than this writing them out.
                if let Some(weights) = operation_weights {
                    table.insert(
                        "operation_weights".to_string(),
                        Value::Table(weights.to_toml_table()),
                    );
                }
            }
            PyGenomeConfig::Sda {
                num_states,
                max_resp_len,
                init_state,
                init_char_mutation_rate,
                transition_vs_response_rate,
            } => {
                table.insert("type".to_string(), Value::String("sda".to_string()));
                table.insert(
                    "num_states".to_string(),
                    integer("num_states", *num_states)?,
                );
                table.insert(
                    "max_resp_len".to_string(),
                    integer("max_resp_len", *max_resp_len)?,
                );
                table.insert(
                    "init_state".to_string(),
                    integer("init_state", *init_state)?,
                );
                // Left out when unset, so `config`'s serde default supplies
                // the rate rather than this writing a second copy of it.
                if let Some(rate) = init_char_mutation_rate {
                    table.insert("init_char_mutation_rate".to_string(), Value::Float(*rate));
                }
                if let Some(rate) = transition_vs_response_rate {
                    table.insert(
                        "transition_vs_response_rate".to_string(),
                        Value::Float(*rate),
                    );
                }
            }
        }
        Ok(Value::Table(table))
    }
}

impl PyFitnessConfig {
    fn to_toml_value(&self) -> PyResult<Value> {
        let mut table = Map::new();
        match self {
            PyFitnessConfig::EpiSpread { sir } => {
                table.insert("type".to_string(), Value::String("epi_spread".to_string()));
                sir.flatten_into(&mut table)?;
            }
            PyFitnessConfig::EpiLength { sir } => {
                table.insert("type".to_string(), Value::String("epi_length".to_string()));
                sir.flatten_into(&mut table)?;
            }
            PyFitnessConfig::EpiProfMatch {
                sir,
                target_profile,
            } => {
                table.insert(
                    "type".to_string(),
                    Value::String("epi_prof_match".to_string()),
                );
                sir.flatten_into(&mut table)?;
                let mut profile = Vec::with_capacity(target_profile.len());
                for value in target_profile {
                    profile.push(Value::Float(*value));
                }
                table.insert("target_profile".to_string(), Value::Array(profile));
            }
            PyFitnessConfig::StructMatch {
                reference_folder,
                degree_bins,
                clustering_bins,
                spectral_bins,
                degree_gamma,
                clustering_gamma,
                spectral_gamma,
                degree_weight,
                clustering_weight,
                spectral_weight,
                density_weight,
            } => {
                table.insert(
                    "type".to_string(),
                    Value::String("struct_match".to_string()),
                );
                table.insert(
                    "reference_folder".to_string(),
                    Value::String(reference_folder.clone()),
                );
                for (key, bins) in [
                    ("degree_bins", *degree_bins),
                    ("clustering_bins", *clustering_bins),
                    ("spectral_bins", *spectral_bins),
                ] {
                    table.insert(key.to_string(), integer(key, bins)?);
                }
                for (key, value) in [
                    ("degree_gamma", *degree_gamma),
                    ("clustering_gamma", *clustering_gamma),
                    ("spectral_gamma", *spectral_gamma),
                    ("degree_weight", *degree_weight),
                    ("clustering_weight", *clustering_weight),
                    ("spectral_weight", *spectral_weight),
                    ("density_weight", *density_weight),
                ] {
                    table.insert(key.to_string(), Value::Float(value));
                }
            }
            PyFitnessConfig::Python() => {
                table.insert("type".to_string(), Value::String("python".to_string()));
            }
        }
        Ok(Value::Table(table))
    }
}

/// Rust-only, so outside the `#[pymethods]` block: builds a `toml` type that
/// has no Python representation.
impl PyConfig {
    fn to_toml_table(&self) -> PyResult<Map<String, Value>> {
        let mut table = Map::new();
        table.insert(
            "population_size".to_string(),
            integer("population_size", self.population_size)?,
        );
        table.insert(
            "network_size".to_string(),
            integer("network_size", self.network_size)?,
        );
        // A `u32` always fits an `i64`, so this one needs no check.
        table.insert(
            "max_edge_multiplicity".to_string(),
            Value::Integer(i64::from(self.max_edge_multiplicity)),
        );
        table.insert(
            "crossover_rate".to_string(),
            Value::Float(self.crossover_rate),
        );
        table.insert(
            "mutation_rate".to_string(),
            Value::Float(self.mutation_rate),
        );
        table.insert(
            "max_mutations".to_string(),
            integer("max_mutations", self.max_mutations)?,
        );
        table.insert("evolution".to_string(), self.evolution.to_toml_value()?);
        table.insert("selection".to_string(), self.selection.to_toml_value()?);
        table.insert("genome".to_string(), self.genome.to_toml_value()?);
        table.insert("fitness".to_string(), self.fitness.to_toml_value()?);
        Ok(table)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::config::{
        Config, EdgeEditGenomeConfig, EvolutionConfig, FitnessConfig, GenomeConfig,
        SdaGenomeConfig, SelectionConfig, SirParams,
    };
    use crate::genomes::EdgeEditOperationWeights;

    /// A fully specified config, with nothing left to a default.
    ///
    /// Built through the real constructors rather than struct literals, so the
    /// tests exercise the argument order and defaults Python sees.
    fn mirror() -> PyConfig {
        PyConfig::new(
            PyEvolutionConfig::Generational {
                num_generations: 500,
                elite_count: 2,
            },
            200,
            100,
            0.9,
            0.2,
            PySelectionConfig::Tournament { tournament_size: 5 },
            PyGenomeConfig::EdgeEdit {
                gene_length: 256,
                operation_weights: None,
            },
            PyFitnessConfig::EpiSpread {
                sir: PySirParams::new(0.05, 30, None, 3, 5),
            },
            1,
            1,
        )
    }

    /// Parse a rendered mirror back through the real front end.
    ///
    /// This is the whole point of the mirror: the Python path produces text,
    /// and `Config::from_toml_str` is the only thing that ever interprets it.
    fn round_trip(config: &PyConfig) -> Config {
        let text = config.to_toml().expect("the mirror renders as TOML");
        Config::from_toml_str(&text)
            .unwrap_or_else(|err| panic!("the rendered TOML should parse, but: {err}\n---\n{text}"))
    }

    #[test]
    fn every_top_level_field_survives_the_round_trip() {
        // Destructured exhaustively, with no `..`: adding a field to `Config`
        // and not to the mirror then fails to COMPILE here, which is the drift
        // alarm this module's header promises.
        let Config {
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
        } = round_trip(&mirror());

        assert_eq!(population_size, 200);
        assert_eq!(network_size, 100);
        assert_eq!(max_edge_multiplicity, 1);
        assert_eq!(crossover_rate, 0.9);
        assert_eq!(mutation_rate, 0.2);
        assert_eq!(max_mutations, 1);

        match evolution {
            EvolutionConfig::Generational {
                num_generations,
                elite_count,
            } => {
                assert_eq!(num_generations, 500);
                assert_eq!(elite_count, 2);
            }
            other => panic!("expected generational, got {other:?}"),
        }

        match selection {
            SelectionConfig::Tournament { tournament_size } => assert_eq!(tournament_size, 5),
        }

        match genome {
            // Destructured exhaustively, no `..`: a field added to
            // `EdgeEditGenomeConfig` and forgotten here fails to compile, which
            // is the drift guard `traps.md` describes for the config mirror.
            GenomeConfig::EdgeEdit(EdgeEditGenomeConfig {
                gene_length,
                operation_weights,
            }) => {
                assert_eq!(gene_length, 256);
                // Omitted from the document, so serde's default supplies it.
                assert_eq!(operation_weights, EdgeEditOperationWeights::default());
            }
            other => panic!("expected edge_edit, got {other:?}"),
        }

        match fitness {
            FitnessConfig::EpiSpread { sir } => assert_sir(&sir, 0.05, 30, None, 3, 5),
            other => panic!("expected epi_spread, got {other:?}"),
        }
    }

    /// Every field of a parsed `SirParams`, destructured for the same reason as
    /// above.
    fn assert_sir(
        sir: &SirParams,
        infection_rate: f64,
        num_epidemics: usize,
        patient_zero: Option<usize>,
        min_epidemic_length: usize,
        max_epidemic_retries: usize,
    ) {
        let SirParams {
            infection_rate: parsed_rate,
            patient_zero: parsed_zero,
            num_epidemics: parsed_epidemics,
            min_epidemic_length: parsed_min,
            max_epidemic_retries: parsed_retries,
        } = sir;

        assert_eq!(*parsed_rate, infection_rate);
        assert_eq!(*parsed_zero, patient_zero);
        assert_eq!(*parsed_epidemics, num_epidemics);
        assert_eq!(*parsed_min, min_epidemic_length);
        assert_eq!(*parsed_retries, max_epidemic_retries);
    }

    #[test]
    fn the_steady_state_and_sda_variants_survive_the_round_trip() {
        let mut config = mirror();
        config.evolution = PyEvolutionConfig::SteadyState {
            num_mating_events: 100_000,
        };
        config.genome = PyGenomeConfig::Sda {
            num_states: 12,
            max_resp_len: 4,
            init_state: 3,
            init_char_mutation_rate: Some(0.1),
            transition_vs_response_rate: Some(0.25),
        };
        config.max_edge_multiplicity = 5;

        let parsed = round_trip(&config);

        assert_eq!(parsed.max_edge_multiplicity, 5);
        match parsed.evolution {
            EvolutionConfig::SteadyState { num_mating_events } => {
                assert_eq!(num_mating_events, 100_000);
            }
            other => panic!("expected steady_state, got {other:?}"),
        }
        match parsed.genome {
            // Destructured exhaustively, no `..`: a field added to
            // `SdaGenomeConfig` and forgotten here fails to compile, which is
            // the drift guard `traps.md` describes for the config mirror.
            GenomeConfig::Sda(SdaGenomeConfig {
                num_states,
                max_resp_len,
                init_state,
                init_char_mutation_rate,
                transition_vs_response_rate,
            }) => {
                assert_eq!(num_states, 12);
                assert_eq!(max_resp_len, 4);
                assert_eq!(init_state, 3);
                assert_eq!(init_char_mutation_rate, 0.1);
                assert_eq!(transition_vs_response_rate, 0.25);
            }
            other => panic!("expected sda, got {other:?}"),
        }
    }

    #[test]
    fn all_four_fitness_variants_survive_the_round_trip() {
        let sir = PySirParams::new(0.05, 30, Some(7), 1, 9);

        let mut config = mirror();
        config.fitness = PyFitnessConfig::EpiLength { sir: sir.clone() };
        match round_trip(&config).fitness {
            FitnessConfig::EpiLength { sir } => assert_sir(&sir, 0.05, 30, Some(7), 1, 9),
            other => panic!("expected epi_length, got {other:?}"),
        }

        config.fitness = PyFitnessConfig::EpiProfMatch {
            sir: sir.clone(),
            target_profile: vec![0.0, 2.5, 7.0, 1.25],
        };
        match round_trip(&config).fitness {
            FitnessConfig::EpiProfMatch {
                sir,
                target_profile,
            } => {
                assert_sir(&sir, 0.05, 30, Some(7), 1, 9);
                assert_eq!(target_profile, vec![0.0, 2.5, 7.0, 1.25]);
            }
            other => panic!("expected epi_prof_match, got {other:?}"),
        }

        // The variant that forced this module to exist: pyo3 needs the empty
        // tuple, serde needs the unit variant, and the text is what bridges
        // them.
        config.fitness = PyFitnessConfig::Python();
        match round_trip(&config).fitness {
            FitnessConfig::Python => {}
            other => panic!("expected python, got {other:?}"),
        }
    }

    #[test]
    fn operation_weights_are_written_only_when_set() {
        let mut config = mirror();
        config.genome = PyGenomeConfig::EdgeEdit {
            gene_length: 256,
            operation_weights: Some(PyOperationWeights {
                null: 0.0,
                swap: 2.0,
                ..PyOperationWeights::default()
            }),
        };

        let text = config.to_toml().expect("renders");
        assert!(
            text.contains("[genome.operation_weights]"),
            "the weights should reach the document as a sub-table:\n{text}"
        );

        match round_trip(&config).genome {
            GenomeConfig::EdgeEdit(edge_edit) => {
                assert_eq!(edge_edit.operation_weights.null, 0.0);
                assert_eq!(edge_edit.operation_weights.swap, 2.0);
                // Untouched fields keep the 1.0 default.
                assert_eq!(edge_edit.operation_weights.toggle, 1.0);
            }
            other => panic!("expected edge_edit, got {other:?}"),
        }
    }

    #[test]
    fn an_unset_patient_zero_is_omitted_rather_than_written_as_a_null() {
        let text = mirror().to_toml().expect("renders");

        assert!(
            !text.contains("patient_zero"),
            "an unset patient zero should not reach the document at all:\n{text}"
        );
    }

    #[test]
    fn the_rendered_document_validates() {
        // `from_toml_str` only parses; the front ends validate separately, and
        // a mirror built from sane values must pass that too.
        round_trip(&mirror())
            .validate()
            .expect("a sane mirror should produce a valid config");
    }

    /// The string literal starting at `index`, ignoring leading whitespace.
    ///
    /// Returns `None` when anything else sits there — an identifier, or another
    /// call — which is how a caller tells a literal field name from a variable
    /// holding one.
    fn literal_at(source: &str, index: usize) -> Option<String> {
        let rest = &source[index..];
        let open = rest.find('"')?;
        if !rest[..open].trim().is_empty() {
            return None;
        }
        let after_open = &rest[open + 1..];
        let close = after_open.find('"')?;
        Some(after_open[..close].to_string())
    }

    /// The `[...]` list of a `for (field, value) in [...]` loop whose `for`
    /// keyword sits at `index`, or `None` if what follows is not that shape.
    ///
    /// The list has to follow the loop pattern immediately: searching the whole
    /// file for the next `in [` would silently scrape an unrelated loop.
    fn field_loop_list(source: &str, index: usize) -> Option<&str> {
        let close_pattern = source[index..].find(')')?;
        let mut start = index + close_pattern + 1;
        while start < source.len() && source.as_bytes()[start].is_ascii_whitespace() {
            start += 1;
        }
        let rest = source.get(start..)?.strip_prefix("in [")?;
        let contents = source.len() - rest.len();
        Some(&source[contents..closing_bracket(source, contents)?])
    }

    /// The index of the `]` closing a list whose contents start at `start`.
    ///
    /// Depth-counted rather than searched for: the tuples inside can hold
    /// brackets of their own, and a `]` inside a string literal must not be
    /// taken for the end of the list.
    fn closing_bracket(source: &str, start: usize) -> Option<usize> {
        let bytes = source.as_bytes();
        let mut depth = 1;
        let mut index = start;
        while index < bytes.len() {
            match bytes[index] {
                b'"' => {
                    index += 1;
                    while index < bytes.len() && bytes[index] != b'"' {
                        if bytes[index] == b'\\' {
                            index += 1;
                        }
                        index += 1;
                    }
                }
                b'[' => depth += 1,
                b']' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(index);
                    }
                }
                _ => {}
            }
            index += 1;
        }
        None
    }

    /// Every field name `config.rs` can raise, scraped from its own source.
    ///
    /// Reading the source rather than maintaining a second list by hand: a list
    /// is exactly the thing that goes stale, and the failure it would hide is
    /// silent — an unmapped field degrades to a bare name rather than erroring.
    ///
    /// Two shapes, because `config.rs` raises in two ways: the direct
    /// `invalid("<field>", ...)` call, and a loop over a list of
    /// `("<field>", value)` pairs whose body raises `invalid(field, ...)`. Both
    /// have to be read — a field visible in neither is one the sweep below
    /// stops checking without saying so. A *third* shape is not handled here but
    /// by `every_invalid_call_names_its_field_scrapably`, which fails rather than
    /// letting this function silently cover less than it claims.
    fn validation_fields_in_config_rs() -> Vec<String> {
        let source = include_str!("config.rs");
        let mut fields = Vec::new();

        // Shape 1: `invalid("<field>", ...)`, sometimes wrapped onto the
        // following line by rustfmt, so this takes the first string literal
        // after each call rather than assuming it is on the same line.
        for (index, _) in source.match_indices("invalid(") {
            if let Some(field) = literal_at(source, index + "invalid(".len())
                && !fields.contains(&field)
            {
                fields.push(field);
            }
        }

        // Shape 2: `for (field, value) in [("<field>", value), ...]`. Every name
        // is the head of a tuple in that list, so this takes the first literal
        // after each `(` inside it. A `(` in one of the value expressions could
        // add a name that is not a field — harmless, since the only cost is an
        // attribute path nobody needs, and the sweep stays loud rather than
        // silently dropping a real one.
        for (index, _) in source.match_indices("for (field,") {
            let Some(list) = field_loop_list(source, index) else {
                // Not the shape this reads. Left to
                // `every_field_loop_is_a_shape_the_scraper_reads`, which fails
                // rather than letting the skip go unnoticed here.
                continue;
            };
            for (paren, _) in list.match_indices('(') {
                if let Some(field) = literal_at(list, paren + 1)
                    && !fields.contains(&field)
                {
                    fields.push(field);
                }
            }
        }

        fields
    }

    #[test]
    fn the_scraper_finds_the_checks_it_is_supposed_to() {
        // Guards the test below: a scraper that silently matched nothing would
        // make it pass while checking no fields at all.
        let fields = validation_fields_in_config_rs();

        assert!(
            fields.len() >= 10,
            "expected the scraper to find every validation field, found {}: {fields:?}",
            fields.len()
        );
        for expected in [
            // Raised as literals at the call site.
            "max_edge_multiplicity",
            "init_state",
            "patient_zero",
            // Raised through a loop variable, one from each of the two
            // functions that use that shape.
            "init_char_mutation_rate",
            "degree_bins",
        ] {
            assert!(
                fields.iter().any(|field| field == expected),
                "the scraper missed `{expected}`, so it is not reading config.rs correctly: \
                 {fields:?}"
            );
        }
    }

    #[test]
    fn every_invalid_call_names_its_field_scrapably() {
        // `validation_fields_in_config_rs` reads two shapes: a literal at the
        // call site, and the `for (field, value) in [...]` loop. A raise fitting
        // neither is invisible to it, and the sweep below then passes whether or
        // not that field is mapped — a guard that has quietly stopped guarding.
        // Fail loudly on a third shape instead.
        let source = include_str!("config.rs");

        let mut unreadable = Vec::new();
        for (index, _) in source.match_indices("invalid(") {
            let after = index + "invalid(".len();
            if literal_at(source, after).is_some() {
                continue;
            }
            // Not a literal, so it is a variable. `field` is the only name the
            // loop shape binds, and so the only one the scraper can resolve.
            let mut argument = String::new();
            for character in source[after..].trim_start().chars() {
                if !character.is_alphanumeric() && character != '_' {
                    break;
                }
                argument.push(character);
            }
            if argument != "field" {
                unreadable.push(argument);
            }
        }

        assert!(
            unreadable.is_empty(),
            "these `invalid(...)` calls name their field with something the scraper cannot read: \
             {unreadable:?}. Either pass a string literal, or bind the name as `field` in a \
             `for (field, value) in [...]` loop, or teach `validation_fields_in_config_rs` the new \
             shape. Left alone, `every_validation_field_maps_to_a_python_attribute` silently stops \
             checking that field."
        );
    }

    #[test]
    fn every_field_loop_is_a_shape_the_scraper_reads() {
        // The other half of the guard above. That one checks the raise site
        // names `field`; this checks the loop binding it is a list of literal
        // pairs the scraper can actually read. A loop whose pattern or list is
        // written some other way — a nested binding, a named constant in place
        // of the list — is skipped in silence otherwise, and the two together
        // are what make "named `field`" mean "scraped".
        let source = include_str!("config.rs");

        let mut unreadable = Vec::new();
        for (index, _) in source.match_indices("for (field,") {
            let readable = match field_loop_list(source, index) {
                Some(list) => list
                    .match_indices('(')
                    .any(|(paren, _)| literal_at(list, paren + 1).is_some()),
                None => false,
            };
            if !readable {
                // The line number is what locates it; the source text of a
                // wrapped loop would run to the end of the file.
                unreadable.push(source[..index].lines().count() + 1);
            }
        }

        assert!(
            unreadable.is_empty(),
            "the `for (field, ...)` loops at these lines of config.rs are not a list of \
             (\"<field>\", value) pairs, so `validation_fields_in_config_rs` skips them and the \
             fields they raise go unchecked: {unreadable:?}"
        );
    }

    #[test]
    fn every_validation_field_maps_to_a_python_attribute() {
        // `seed` is raised by `reject_stray_fitness_keys` against raw TOML text and
        // is unreachable from this front end, which has no seed to write. It is
        // exempt by name so that adding any OTHER unmapped field still fails.
        const NOT_REACHABLE_FROM_PYTHON: [&str; 1] = ["seed"];

        let mut unmapped = Vec::new();
        for field in validation_fields_in_config_rs() {
            if NOT_REACHABLE_FROM_PYTHON.contains(&field.as_str()) {
                continue;
            }
            if python_attribute_path(&field).is_none() {
                unmapped.push(field);
            }
        }

        assert!(
            unmapped.is_empty(),
            "these validation fields have no Python attribute path, so a user would be told a \
             bare field name: {unmapped:?}. Add them to `python_attribute_path`."
        );
    }

    #[test]
    fn a_nested_field_is_reported_by_its_full_python_path() {
        let mut config = mirror();
        config.fitness = PyFitnessConfig::EpiSpread {
            sir: PySirParams::new(0.05, 0, None, 3, 5),
        };

        let parsed = Config::from_toml_str(&config.to_toml().expect("renders")).expect("parses");
        let message =
            config_error_to_py(&parsed.validate().expect_err("zero epidemics is invalid"))
                .to_string();

        assert!(
            message.contains("config.fitness.sir.num_epidemics"),
            "the error should name the Python attribute path, got: {message}"
        );
        // The constraint itself must survive the rewrite.
        assert!(
            message.contains("must be at least 1"),
            "the constraint should be kept intact, got: {message}"
        );
    }

    #[test]
    fn an_error_with_no_python_equivalent_keeps_its_original_wording() {
        // A parse failure has no attribute to point at, and its message is
        // already the useful one.
        let error = Config::from_toml_str("this is not toml at all")
            .expect_err("that is not a valid document");

        let message = config_error_to_py(&error).to_string();

        assert!(
            message.contains("could not parse the config"),
            "a parse error should pass through unchanged, got: {message}"
        );
    }

    #[test]
    fn a_field_too_large_for_a_toml_integer_is_reported_not_wrapped() {
        let mut config = mirror();
        config.population_size = usize::MAX;

        let message = config
            .to_toml()
            .expect_err("usize::MAX does not fit a TOML integer")
            .to_string();

        assert!(
            message.contains("population_size"),
            "the error should name the offending field, got: {message}"
        );
    }
}
