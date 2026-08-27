//! Python-facing builders for the `config.toml` schema.
//!
//! Each type here mirrors one in [`crate::config`], field for field. The user
//! fills them in from Python, they serialize to TOML, and
//! [`crate::config::Config::from_toml_str`] parses that TOML — so there is one
//! parser and one validator, and the Python path cannot accept a config the
//! hand-written TOML path rejects.
//!
//! The mirror is forced, not chosen: `#[pyclass]` on [`crate::config`]'s own
//! enums would break the TOML front end, because pyo3 rejects a unit variant in
//! a complex enum and serde rejects the tuple variant it would become. It is
//! also why several variants here carry an empty struct body.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use toml::Value;
use toml::map::Map;

/// Epidemic sampling parameters, shared by the epidemic objectives.
///
/// Nothing is range-checked here; every field is checked when the config is
/// handed to an evolver.
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
/// Every weight defaults to 1.0, so the operations are equally likely; 0.0
/// disables an operation outright.
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
    #[allow(clippy::too_many_arguments)] // one argument per weight is the schema, not a smell
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
#[pyclass(name = "EvolutionConfig")]
#[derive(Debug, Clone)]
pub enum PyEvolutionConfig {
    // `elite_count`'s default of 1 has to match `config`'s `default_elite_count`;
    // nothing checks that the two agree.
    #[pyo3(constructor = (num_generations, elite_count = 1))]
    Generational {
        num_generations: usize,
        /// Best individuals carried forward each generation.
        elite_count: usize,
    },
    #[pyo3(constructor = (num_mating_events, replacement = None))]
    SteadyState {
        num_mating_events: usize,
        /// Which members of the scope a mating event's children overwrite;
        /// `None` means the least fit.
        replacement: Option<PyReplacementConfig>,
    },
    // ADD A STRATEGY STEP 6 — the Python-side variant, if the strategy should be
    // selectable from Python, plus its arm in `to_toml_value` below writing
    // `type = "my_strategy"` and the fields under `[evolution]`. Optional: without
    // it the strategy still runs from a TOML config and from Rust.
    //
    //     #[pyo3(constructor = (num_my_events))]
    //     MyStrategy { num_my_events: usize },
}

/// Which members of a scope a mating event's children overwrite.
#[pyclass(name = "ReplacementConfig")]
#[derive(Debug, Clone)]
pub enum PyReplacementConfig {
    #[pyo3(constructor = ())]
    Worst {},
    #[pyo3(constructor = ())]
    Random {},
    // ADD A REPLACEMENT STEP 3 (for SteadyState, Python half) — the Python-side
    // variant plus its arm in `to_toml_value` below. Optional: without it the
    // policy still works from a TOML config and from Rust.
    //
    //     #[pyo3(constructor = (size))]
    //     Tournament { size: usize },
}

/// The slice of the population one breeding event draws from.
#[pyclass(name = "ScopeConfig")]
#[derive(Debug, Clone)]
pub enum PyScopeConfig {
    #[pyo3(constructor = ())]
    Global {},
    #[pyo3(constructor = (size))]
    RandomSubset { size: usize },
    // ADD A SCOPE STEP 5 — the Python-side variant, plus its arm in `to_toml_value`
    // below writing the `[scope]` table, and a field step 3 validates by name also
    // needs a path in `python_attribute_path`. Optional: without it the scope still
    // works from a TOML config and from Rust.
    //
    //     #[pyo3(constructor = (radius))]
    //     Neighbourhood { radius: usize },
}

/// Parent-selection strategy.
#[pyclass(name = "SelectionConfig")]
#[derive(Debug, Clone)]
pub enum PySelectionConfig {
    #[pyo3(constructor = ())]
    Best {},
    #[pyo3(constructor = (tournament_size))]
    Tournament { tournament_size: usize },
    // ADD A SELECTION STEP 5 — the Python-side variant, plus its arm in
    // `to_toml_value` below writing the `[selection]` table, and a field step 3
    // validates by name also needs a path in `python_attribute_path`. Optional:
    // without it the scheme still works from a TOML config and from Rust.
    //
    //     #[pyo3(constructor = (pressure))]
    //     Roulette { pressure: f64 },
}

/// Recombination operator.
#[pyclass(name = "CrossoverConfig")]
#[derive(Debug, Clone)]
pub enum PyCrossoverConfig {
    #[pyo3(constructor = ())]
    TwoPoint {},
    // ADD A CROSSOVER STEP 5 — the Python-side variant, if the operator should be
    // selectable from Python, plus its arm in `to_toml_value` below. Optional:
    // without it the operator still runs from a TOML config and from Rust.
    //
    //     #[pyo3(constructor = (some_param))]
    //     MyCrossover { some_param: f64 },
}

impl Default for PyCrossoverConfig {
    fn default() -> Self {
        PyCrossoverConfig::TwoPoint {}
    }
}

/// Genome representation and the dimensions used to build random individuals.
#[pyclass(name = "GenomeConfig")]
#[derive(Debug, Clone)]
pub enum PyGenomeConfig {
    #[pyo3(constructor = (gene_length, operation_weights = None, mutation = None))]
    EdgeEdit {
        gene_length: usize,
        /// Omitted, every operation gets a weight of 1.0.
        operation_weights: Option<PyOperationWeights>,
        /// Which mutation the run applies; omitted, the default one.
        mutation: Option<PyEdgeEditMutationConfig>,
    },
    /// No `num_chars`: the alphabet is `max_edge_multiplicity + 1`, so every
    /// character is a legal edge weight.
    #[pyo3(constructor = (
        num_states,
        max_resp_len,
        init_state = 0,
        init_char_mutation_rate = None,
        transition_vs_response_rate = None,
        mutation = None,
    ))]
    Sda {
        num_states: usize,
        max_resp_len: usize,
        /// Must be less than `num_states`, or expression panics.
        init_state: usize,
        /// Chance a mutation redraws the initial character instead of touching
        /// the transition table; omitted, the default rate.
        init_char_mutation_rate: Option<f64>,
        /// Chance of redrawing a transition's target rather than its response,
        /// once the initial character was not chosen.
        transition_vs_response_rate: Option<f64>,
        /// Which mutation the run applies; omitted, the default one.
        mutation: Option<PySdaMutationConfig>,
    },
    // ADD A GENOME STEP 7 — the Python-side variant, if the representation should
    // be selectable from Python, plus its arm in `to_toml_value` below writing
    // `type = "my_genome"` and the fields under `[genome]`. Optional: without it
    // the representation still runs from a TOML config and from Rust.
    //
    //     #[pyo3(constructor = (some_dimension))]
    //     MyGenome { some_dimension: usize },
}

/// Fitness objective and its parameters.
///
/// The epidemic objectives read one simulation differently, so they share a
/// single [`PySirParams`] block.
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
        /// The profile the run is scored against, compared verbatim — nothing
        /// is prepended to it and nothing is rescaled.
        target_profile: Vec<f64>,
    },
    /// How closely a graph's structure matches a set of reference graphs.
    /// Minimized; requires `max_edge_multiplicity = 1`.
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
    // ADD AN OBJECTIVE STEP 4 — the Python-side variant, if the objective should
    // be selectable from Python, plus a path in `python_attribute_path` below for
    // every field step 2 validates by name. Optional: without it the objective
    // still runs from a TOML config and from Rust.
    //
    //     #[pyo3(constructor = (threshold))]
    //     MyObjective { threshold: f64 },
    /// A Python callable registered before the run, via
    /// `GraphEvolver.set_fitness_function`. Whether it is maximized or minimized
    /// is declared at registration, not here.
    #[pyo3(constructor = ())]
    Python(),
}

/// Everything the genetic algorithm needs for a run.
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
    /// Which slice of the population one breeding event draws from.
    #[pyo3(get, set)]
    pub scope: PyScopeConfig,
    /// Parent-selection strategy, applied within that scope.
    #[pyo3(get, set)]
    pub selection: PySelectionConfig,
    /// Recombination operator; two-point when unset.
    #[pyo3(get, set)]
    pub crossover: PyCrossoverConfig,
    /// Genome representation and its dimensions.
    #[pyo3(get, set)]
    pub genome: PyGenomeConfig,
    /// Fitness objective.
    #[pyo3(get, set)]
    pub fitness: PyFitnessConfig,
}

#[pymethods]
impl PyConfig {
    /// No `seed` here: one master seed is supplied to the run, and every draw
    /// derives from it.
    #[new]
    #[pyo3(signature = (
        evolution,
        population_size,
        network_size,
        crossover_rate,
        mutation_rate,
        scope,
        selection,
        genome,
        fitness,
        max_edge_multiplicity = 1,
        max_mutations = 1,
        crossover = None,
    ))]
    #[allow(clippy::too_many_arguments)] // one argument per field is the schema, not a smell
    pub fn new(
        evolution: PyEvolutionConfig,
        population_size: usize,
        network_size: usize,
        crossover_rate: f64,
        mutation_rate: f64,
        scope: PyScopeConfig,
        selection: PySelectionConfig,
        genome: PyGenomeConfig,
        fitness: PyFitnessConfig,
        max_edge_multiplicity: u32,
        max_mutations: usize,
        crossover: Option<PyCrossoverConfig>,
    ) -> Self {
        Self {
            evolution,
            population_size,
            network_size,
            max_edge_multiplicity,
            crossover_rate,
            mutation_rate,
            max_mutations,
            scope,
            selection,
            crossover: crossover.unwrap_or_default(),
            genome,
            fitness,
        }
    }

    /// Render this config as the TOML document GET parses — the record of what
    /// was run, byte for byte.
    ///
    /// `ValueError` if a field is too large for a TOML integer, `2**63 - 1`.
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
/// in the TOML document, which a Python caller never wrote; this maps it back to
/// the attribute they set. `None` means there is no Python equivalent and the
/// caller falls back to the unmapped name.
///
/// `every_validation_field_maps_to_a_python_attribute` checks every name here
/// against `config.rs`'s own `invalid(...)` calls, so a validation added there
/// without a mapping fails the suite rather than degrading to a bare field name.
fn python_attribute_path(field: &str) -> Option<&'static str> {
    match field {
        "population_size" => Some("config.population_size"),
        "max_edge_multiplicity" => Some("config.max_edge_multiplicity"),
        "crossover_rate" => Some("config.crossover_rate"),
        "mutation_rate" => Some("config.mutation_rate"),
        "max_mutations" => Some("config.max_mutations"),
        "elite_count" => Some("config.evolution.elite_count"),
        "tournament_size" => Some("config.selection.tournament_size"),
        "size" => Some("config.scope.size"),
        "operation_weights" => Some("config.genome.operation_weights"),
        "init_state" => Some("config.genome.init_state"),
        "init_char_mutation_rate" => Some("config.genome.init_char_mutation_rate"),
        "transition_vs_response_rate" => Some("config.genome.transition_vs_response_rate"),
        // ADD A GENOME STEP 7 — one line per field step 4's validation names, or a
        // Python caller gets an error naming a TOML field they never wrote.
        //
        //     "some_dimension" => Some("config.genome.some_dimension"),
        // The SIR keys flatten into `[fitness]` in the document, but every
        // epidemic objective reaches them through the same `sir` attribute.
        "infection_rate" => Some("config.fitness.sir.infection_rate"),
        "num_epidemics" => Some("config.fitness.sir.num_epidemics"),
        "min_epidemic_length" => Some("config.fitness.sir.min_epidemic_length"),
        "max_epidemic_retries" => Some("config.fitness.sir.max_epidemic_retries"),
        "patient_zero" => Some("config.fitness.sir.patient_zero"),
        "target_profile" => Some("config.fitness.target_profile"),
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
        // ADD AN OBJECTIVE STEP 4 — one line per field step 2's validation names,
        // mapping the TOML name to the Python attribute path.
        //
        //     "threshold" => Some("config.fitness.threshold"),
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

    PyValueError::new_err(error.to_string())
}

/// A `usize` as a TOML integer.
///
/// TOML integers are `i64`, and the gap is reachable from Python:
/// `population_size=2**63` converts into `usize` happily and fails only here.
/// Rejected rather than wrapped to a negative number, which would be a silently
/// wrong config.
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
    /// `#[serde(flatten)]`ed into its variant, so these become keys of
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
        // Omitted when unset: a null would not parse, and a sentinel would pin
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
            PyEvolutionConfig::SteadyState {
                num_mating_events,
                replacement,
            } => {
                table.insert(
                    "type".to_string(),
                    Value::String("steady_state".to_string()),
                );
                table.insert(
                    "num_mating_events".to_string(),
                    integer("num_mating_events", *num_mating_events)?,
                );
                // Omitted when unset, so the parser's own default applies rather
                // than this recording a choice the caller never made.
                if let Some(replacement) = replacement {
                    table.insert("replacement".to_string(), replacement.to_toml_value()?);
                }
            } // ADD A STRATEGY STEP 6 — the matching arm for your variant:
              //
              //     PyEvolutionConfig::MyStrategy { num_my_events } => {
              //         table.insert("type".to_string(), Value::String("my_strategy".to_string()));
              //         table.insert("num_my_events".to_string(), integer("num_my_events", *num_my_events)?);
              //     }
        }
        Ok(Value::Table(table))
    }
}

impl PyReplacementConfig {
    fn to_toml_value(&self) -> PyResult<Value> {
        let mut table = Map::new();
        match self {
            PyReplacementConfig::Worst {} => {
                table.insert("type".to_string(), Value::String("worst".to_string()));
            }
            PyReplacementConfig::Random {} => {
                table.insert("type".to_string(), Value::String("random".to_string()));
            } // ADD A REPLACEMENT STEP 3 (for SteadyState) — the matching arm:
              //
              //     PyReplacementConfig::Tournament { size } => {
              //         table.insert("type".to_string(), Value::String("tournament".to_string()));
              //         table.insert("size".to_string(), integer("size", *size)?);
              //     }
        }
        Ok(Value::Table(table))
    }
}

impl PyScopeConfig {
    fn to_toml_value(&self) -> PyResult<Value> {
        let mut table = Map::new();
        match self {
            PyScopeConfig::Global {} => {
                table.insert("type".to_string(), Value::String("global".to_string()));
            }
            PyScopeConfig::RandomSubset { size } => {
                table.insert(
                    "type".to_string(),
                    Value::String("random_subset".to_string()),
                );
                table.insert("size".to_string(), integer("size", *size)?);
            } // ADD A SCOPE STEP 5 — the matching arm for your variant:
              //
              //     PyScopeConfig::Neighbourhood { radius } => {
              //         table.insert("type".to_string(), Value::String("neighbourhood".to_string()));
              //         table.insert("radius".to_string(), integer("radius", *radius)?);
              //     }
        }
        Ok(Value::Table(table))
    }
}

impl PySelectionConfig {
    fn to_toml_value(&self) -> PyResult<Value> {
        let mut table = Map::new();
        match self {
            PySelectionConfig::Best {} => {
                table.insert("type".to_string(), Value::String("best".to_string()));
            }
            PySelectionConfig::Tournament { tournament_size } => {
                table.insert("type".to_string(), Value::String("tournament".to_string()));
                table.insert(
                    "tournament_size".to_string(),
                    integer("tournament_size", *tournament_size)?,
                );
            } // ADD A SELECTION STEP 5 — the matching arm for your variant:
              //
              //     PySelectionConfig::Roulette { pressure } => {
              //         table.insert("type".to_string(), Value::String("roulette".to_string()));
              //         table.insert("pressure".to_string(), integer("pressure", *pressure)?);
              //     }
        }
        Ok(Value::Table(table))
    }
}

impl PyCrossoverConfig {
    fn to_toml_value(&self) -> PyResult<Value> {
        let mut table = Map::new();
        match self {
            PyCrossoverConfig::TwoPoint {} => {
                table.insert("type".to_string(), Value::String("two_point".to_string()));
            } // ADD A CROSSOVER STEP 5 — the matching arm for your variant:
              //
              //     PyCrossoverConfig::Uniform { swap_rate } => {
              //         table.insert("type".to_string(), Value::String("uniform".to_string()));
              //         table.insert("swap_rate".to_string(), integer("swap_rate", *swap_rate)?);
              //     }
        }
        Ok(Value::Table(table))
    }
}

/// Which mutation an edge-edit genome applies.
#[pyclass(name = "EdgeEditMutationConfig")]
#[derive(Debug, Clone)]
pub enum PyEdgeEditMutationConfig {
    #[pyo3(constructor = ())]
    RerollGene {},
    // ADD A MUTATION STEP 4 (for EdgeEdit) — the Python-side variant, if the
    // operator should be selectable from Python, plus its arm in `to_toml_value`
    // below and an example line under `[genome]` in `config.example.toml`.
    // Optional: without it the operator still runs from a TOML config and from Rust.
    //
    //     #[pyo3(constructor = (some_param))]
    //     MyMutation { some_param: f64 },
}

impl Default for PyEdgeEditMutationConfig {
    fn default() -> Self {
        PyEdgeEditMutationConfig::RerollGene {}
    }
}

impl PyEdgeEditMutationConfig {
    fn to_toml_value(&self) -> Value {
        let mut table = Map::new();
        match self {
            PyEdgeEditMutationConfig::RerollGene {} => {
                table.insert("type".to_string(), Value::String("reroll_gene".to_string()));
            } // ADD A MUTATION STEP 4 (for EdgeEdit) — the matching arm:
              //
              //     PyEdgeEditMutationConfig::MyMutation {} => {
              //         table.insert("type".to_string(), Value::String("my_mutation".to_string()));
              //     }
        }
        Value::Table(table)
    }
}

/// Which mutation an SDA genome applies.
#[pyclass(name = "SdaMutationConfig")]
#[derive(Debug, Clone)]
pub enum PySdaMutationConfig {
    #[pyo3(constructor = ())]
    RedrawOne {},
    // ADD A MUTATION STEP 4 (for SDA) — the Python-side variant, if the operator
    // should be selectable from Python, plus its arm in `to_toml_value` below and
    // an example line under `[genome]` in `config.example.toml`. Optional: without
    // it the operator still runs from a TOML config and from Rust.
    //
    //     #[pyo3(constructor = (some_param))]
    //     MyMutation { some_param: f64 },
}

impl Default for PySdaMutationConfig {
    fn default() -> Self {
        PySdaMutationConfig::RedrawOne {}
    }
}

impl PySdaMutationConfig {
    fn to_toml_value(&self) -> Value {
        let mut table = Map::new();
        match self {
            PySdaMutationConfig::RedrawOne {} => {
                table.insert("type".to_string(), Value::String("redraw_one".to_string()));
            } // ADD A MUTATION STEP 4 (for SDA) — the matching arm:
              //
              //     PySdaMutationConfig::MyMutation {} => {
              //         table.insert("type".to_string(), Value::String("my_mutation".to_string()));
              //     }
        }
        Value::Table(table)
    }
}

impl PyGenomeConfig {
    fn to_toml_value(&self) -> PyResult<Value> {
        let mut table = Map::new();
        match self {
            PyGenomeConfig::EdgeEdit {
                gene_length,
                operation_weights,
                mutation,
            } => {
                table.insert("type".to_string(), Value::String("edge_edit".to_string()));
                table.insert(
                    "gene_length".to_string(),
                    integer("gene_length", *gene_length)?,
                );
                // Left out when unset, so the parser's own defaults supply the
                // weights rather than this writing a second copy of them.
                if let Some(weights) = operation_weights {
                    table.insert(
                        "operation_weights".to_string(),
                        Value::Table(weights.to_toml_table()),
                    );
                }
                if let Some(mutation) = mutation {
                    table.insert("mutation".to_string(), mutation.to_toml_value());
                }
            }
            PyGenomeConfig::Sda {
                num_states,
                max_resp_len,
                init_state,
                init_char_mutation_rate,
                transition_vs_response_rate,
                mutation,
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
                // Left out when unset, so the parser's own defaults supply these
                // rather than this writing a second copy of them.
                if let Some(rate) = init_char_mutation_rate {
                    table.insert("init_char_mutation_rate".to_string(), Value::Float(*rate));
                }
                if let Some(rate) = transition_vs_response_rate {
                    table.insert(
                        "transition_vs_response_rate".to_string(),
                        Value::Float(*rate),
                    );
                }
                if let Some(mutation) = mutation {
                    table.insert("mutation".to_string(), mutation.to_toml_value());
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

/// A second impl block: the TOML value it builds has no Python equivalent.
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
        table.insert("scope".to_string(), self.scope.to_toml_value()?);
        table.insert("selection".to_string(), self.selection.to_toml_value()?);
        table.insert("crossover".to_string(), self.crossover.to_toml_value()?);
        table.insert("genome".to_string(), self.genome.to_toml_value()?);
        table.insert("fitness".to_string(), self.fitness.to_toml_value()?);
        Ok(table)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::config::{
        Config, CrossoverConfig, EdgeEditGenomeConfig, EdgeEditMutationConfig, EvolutionConfig,
        FitnessConfig, GenomeConfig, ReplacementConfig, ScopeConfig, SdaGenomeConfig,
        SdaMutationConfig, SelectionConfig, SirParams,
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
            PyScopeConfig::RandomSubset { size: 7 },
            PySelectionConfig::Tournament { tournament_size: 5 },
            PyGenomeConfig::EdgeEdit {
                gene_length: 256,
                operation_weights: None,
                mutation: Some(PyEdgeEditMutationConfig::RerollGene {}),
            },
            PyFitnessConfig::EpiSpread {
                sir: PySirParams::new(0.05, 30, None, 3, 5),
            },
            1,
            1,
            Some(PyCrossoverConfig::TwoPoint {}),
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
            scope,
            selection,
            crossover,
            genome,
            fitness,
        } = round_trip(&mirror());

        assert_eq!(population_size, 200);
        assert_eq!(network_size, 100);
        assert_eq!(max_edge_multiplicity, 1);
        assert_eq!(crossover_rate, 0.9);
        assert_eq!(mutation_rate, 0.2);
        assert_eq!(max_mutations, 1);

        // Named explicitly by the fixture, which leaves nothing to a default,
        // so this checks the mirror renders the operator rather than that the
        // default fills it in — `a_config_naming_no_operator_gets_two_point`
        // in `config` covers the other half.
        match crossover {
            CrossoverConfig::TwoPoint => {}
        }

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

        // Distinct numbers on purpose: 7 for the scope, 5 for the tournament.
        // Equal values would let the two fields be swapped without any test
        // noticing, which is the exact confusion this block was split to end.
        match scope {
            ScopeConfig::RandomSubset { size } => assert_eq!(size, 7),
            other => panic!("expected a random subset, got {other:?}"),
        }

        match selection {
            SelectionConfig::Tournament { tournament_size } => assert_eq!(tournament_size, 5),
            other => panic!("expected a tournament, got {other:?}"),
        }

        match genome {
            // Destructured exhaustively, no `..`: a field added to
            // `EdgeEditGenomeConfig` and forgotten here fails to compile,
            // which is this module's drift guard for the config mirror.
            GenomeConfig::EdgeEdit(EdgeEditGenomeConfig {
                gene_length,
                operation_weights,
                mutation,
            }) => {
                assert_eq!(gene_length, 256);
                // Omitted from the document, so serde's default supplies it.
                assert_eq!(operation_weights, EdgeEditOperationWeights::default());
                // Likewise the mutation operator: `mirror()` names it
                // explicitly (it leaves nothing to a default), so this pins
                // that the named choice round-trips through TOML rather than
                // only being spelled on the struct.
                // `a_config_naming_no_mutation_operator_gets_the_representations_default`
                // covers the defaulting half.
                assert_eq!(mutation, EdgeEditMutationConfig::RerollGene);
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
            replacement: Some(PyReplacementConfig::Worst {}),
        };
        config.genome = PyGenomeConfig::Sda {
            num_states: 12,
            max_resp_len: 4,
            init_state: 3,
            init_char_mutation_rate: Some(0.1),
            transition_vs_response_rate: Some(0.25),
            mutation: None,
        };
        config.max_edge_multiplicity = 5;

        let parsed = round_trip(&config);

        assert_eq!(parsed.max_edge_multiplicity, 5);
        match parsed.evolution {
            EvolutionConfig::SteadyState {
                num_mating_events,
                replacement,
            } => {
                assert_eq!(num_mating_events, 100_000);
                // Named explicitly by the fixture, so this checks the mirror
                // renders it rather than that the default filled it in.
                match replacement {
                    ReplacementConfig::Worst => {}
                    other => panic!("expected worst, got {other:?}"),
                }
            }
            other => panic!("expected steady_state, got {other:?}"),
        }
        match parsed.genome {
            // Destructured exhaustively, no `..`: a field added to
            // `SdaGenomeConfig` and forgotten here fails to compile, which is
            // this module's drift guard for the config mirror.
            GenomeConfig::Sda(SdaGenomeConfig {
                num_states,
                max_resp_len,
                init_state,
                init_char_mutation_rate,
                transition_vs_response_rate,
                mutation,
            }) => {
                assert_eq!(mutation, SdaMutationConfig::RedrawOne);
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
            mutation: None,
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

    /// `config.rs`'s source with whole-line `//` comments blanked out.
    ///
    /// Everything below reads `config.rs` as text, with no notion of Rust
    /// syntax, so without this a raise site written *inside* a comment counts
    /// as a real one — and `config.rs` deliberately carries worked examples in
    /// comments showing what a new genome's validation arm looks like. The
    /// failure is loud but misleading: the sweep demands a Python attribute
    /// path for a field name nobody ever wrote. The same blindness points the
    /// other way too, and that half is silent — a real check commented out
    /// while debugging still reads as present.
    ///
    /// Only lines whose **first non-whitespace is `//`** are blanked, so a
    /// `//` inside a string literal cannot take its line with it. That leaves a
    /// trailing comment on a line of code in place, which is deliberate: the
    /// shapes scanned for are calls, and a raise site hiding after `//` at the
    /// end of a live line is not a thing `config.rs` does.
    ///
    /// Blanked to spaces rather than removed, so every byte offset is
    /// unchanged. `literal_at` and `field_loop_list` both scan forward from an
    /// index into this string; deleting lines would shift those and report the
    /// wrong line numbers in the guards' failure messages.
    ///
    /// One space **per byte**, not per character. These comments contain
    /// em-dashes and other multi-byte text, and one space per `char` would
    /// shorten the string — quietly reintroducing the offset drift this exists
    /// to avoid.
    fn config_rs_without_comments() -> String {
        blank_comment_lines(include_str!("config.rs"))
    }

    /// The blanking itself, over any source text — separate from the reader
    /// above so it can be tested against a fixture rather than against
    /// `config.rs`, whose contents move.
    fn blank_comment_lines(source: &str) -> String {
        let mut out = String::with_capacity(source.len());
        for line in source.split_inclusive('\n') {
            if line.trim_start().starts_with("//") {
                for character in line.chars() {
                    if character == '\n' {
                        out.push('\n');
                    } else {
                        for _ in 0..character.len_utf8() {
                            out.push(' ');
                        }
                    }
                }
            } else {
                out.push_str(line);
            }
        }
        out
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
        let source = &config_rs_without_comments();
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
        let source = &config_rs_without_comments();

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
        let source = &config_rs_without_comments();

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
    fn a_raise_site_written_inside_a_comment_is_not_scraped() {
        // The third blind spot, after the two guards above: everything here
        // reads `config.rs` as text, so without `config_rs_without_comments` a
        // worked example in a comment is indistinguishable from a live call.
        // `config.rs` carries exactly such an example — a genome step-4 marker
        // — and the symptom is the sweep below demanding a Python attribute for
        // a field name nobody wrote.
        //
        // Asserted against a fixture rather than `config.rs` itself, so the test
        // still means something after someone edits that marker away.
        let source = "\
fn validate(&self) -> Result<(), ConfigError> {
    // Example for a new variant:
    //     return Err(invalid(\"commented_out_field\", \"must be at least 1\"));
    if self.real == 0 {
        return Err(invalid(\"real_field\", \"must be at least 1\"));
    }
    Ok(())
}
";
        let mut fields = Vec::new();
        for (index, _) in blank_comment_lines(source).match_indices("invalid(") {
            if let Some(field) = literal_at(&blank_comment_lines(source), index + "invalid(".len())
            {
                fields.push(field);
            }
        }

        assert_eq!(
            fields,
            vec!["real_field".to_string()],
            "a raise site inside a `//` comment must not be scraped, and a live one must be"
        );
    }

    #[test]
    fn blanking_a_comment_preserves_every_byte_offset() {
        // `literal_at` and `field_loop_list` scan forward from byte indices into
        // the blanked string and the guards report line numbers from it, so the
        // blanking has to be length-preserving. One space per *char* would not
        // be: these comments contain em-dashes, which are three bytes.
        let source = "let x = 1;\n    // an em-dash — three bytes\nlet y = 2;\n";
        let blanked = blank_comment_lines(source);

        assert_eq!(blanked.len(), source.len(), "byte length must not change");
        assert_eq!(blanked.lines().count(), source.lines().count());
        // The code either side survives; only the comment line is blanked.
        assert!(blanked.contains("let x = 1;"));
        assert!(blanked.contains("let y = 2;"));
        assert!(!blanked.contains("em-dash"));
    }

    #[test]
    fn a_double_slash_inside_a_string_literal_does_not_blank_its_line() {
        // Only a *leading* `//` counts. Blanking on any occurrence would take
        // out live code whose string happens to contain one.
        let source = "let url = \"http://example.com\";\n";
        assert_eq!(blank_comment_lines(source), source);
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
