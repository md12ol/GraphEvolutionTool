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
    /// `ValueError` if a field is too large for a TOML integer — see
    /// [`integer`].
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
                target_profile_path,
            } => {
                table.insert(
                    "type".to_string(),
                    Value::String("epi_prof_match".to_string()),
                );
                sir.flatten_into(&mut table)?;
                // `display()` rather than `to_string_lossy()`: a path that is
                // not valid UTF-8 cannot be written into a TOML string at all,
                // and lossy replacement would silently point at a file that
                // does not exist. Non-UTF-8 paths are out of scope until a
                // config carries one.
                table.insert(
                    "target_profile_path".to_string(),
                    Value::String(target_profile_path.display().to_string()),
                );
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
        Config, EvolutionConfig, FitnessConfig, GenomeConfig, SelectionConfig, SirParams,
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
            GenomeConfig::EdgeEdit {
                gene_length,
                operation_weights,
            } => {
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
            GenomeConfig::Sda {
                num_states,
                max_resp_len,
                init_state,
            } => {
                assert_eq!(num_states, 12);
                assert_eq!(max_resp_len, 4);
                assert_eq!(init_state, 3);
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
            target_profile_path: PathBuf::from("Profiles/Profile3.dat"),
        };
        match round_trip(&config).fitness {
            FitnessConfig::EpiProfMatch {
                sir,
                target_profile_path,
            } => {
                assert_sir(&sir, 0.05, 30, Some(7), 1, 9);
                assert_eq!(target_profile_path, PathBuf::from("Profiles/Profile3.dat"));
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
            GenomeConfig::EdgeEdit {
                operation_weights, ..
            } => {
                assert_eq!(operation_weights.null, 0.0);
                assert_eq!(operation_weights.swap, 2.0);
                // Untouched fields keep the 1.0 default.
                assert_eq!(operation_weights.toggle, 1.0);
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
