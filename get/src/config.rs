//! Deserializable mirror of `config.toml`.
//!
//! These are mostly flat, owned structs rather than the engine's own context
//! types: [`crate::evolver::SharedEvolutionContext`] is generic over the genome
//! and carries a non-deserializable `Genome::Context`, so configuration is
//! parsed into this plain shape and then mapped onto concrete engine types by
//! the dispatch layer in `lib.rs`.
//!
//! Plain data types that carry no such baggage are deserialized directly rather
//! than mirrored — [`EdgeEditOperationWeights`] is nine `f64`s with a `Default`,
//! and duplicating it here would buy nothing but a conversion to maintain.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::genomes::EdgeEditOperationWeights;

/// Everything the genetic algorithm needs for a run.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Which evolution strategy to run, and its strategy-specific settings.
    pub evolution: EvolutionConfig,
    /// Number of individuals in the population.
    pub population_size: usize,
    /// Number of nodes in every expressed graph.
    pub network_size: usize,
    /// Edge-weight cap; defaults to 1 (unweighted).
    #[serde(default = "default_max_edge_multiplicity")]
    pub max_edge_multiplicity: u32,
    /// Probability that a selected pair is recombined.
    pub crossover_rate: f64,
    /// Probability that a child is mutated at all.
    pub mutation_rate: f64,
    /// How many mutations a mutating child takes, drawn uniformly from
    /// `1..=max_mutations`. Defaults to 1.
    ///
    /// Adjacent to `mutation_rate` because the two are one conceptual knob:
    /// whether a child mutates, then how many mutations it takes.
    #[serde(default = "default_max_mutations")]
    pub max_mutations: usize,
    /// Parent-selection strategy.
    pub selection: SelectionConfig,
    /// Genome representation and its dimensions.
    pub genome: GenomeConfig,
    /// Fitness objective.
    pub fitness: FitnessConfig,
}

/// Evolution strategy and its strategy-specific settings.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EvolutionConfig {
    Generational {
        num_generations: usize,
        /// Best individuals carried forward each generation; defaults to 1.
        #[serde(default = "default_elite_count")]
        elite_count: usize,
    },
    SteadyState {
        num_mating_events: usize,
    },
}

/// Parent-selection strategy. Maps onto [`crate::evolver::common::Selection`].
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SelectionConfig {
    Tournament { tournament_size: usize },
}

/// Genome representation and the dimensions used to build random individuals.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GenomeConfig {
    EdgeEdit {
        gene_length: usize,
        /// Relative probability of each edit operation. Omitted entirely, or
        /// omitted field by field, every operation defaults to a weight of 1.0.
        #[serde(default)]
        operation_weights: EdgeEditOperationWeights,
    },
    Sda {
        num_states: usize,
        /// No `num_chars` here: the alphabet is derived as
        /// `max_edge_multiplicity + 1` so every character is a legal edge
        /// weight (spec §3.2). Whatever maps this onto `SdaContext` derives it.
        max_resp_len: usize,
        /// State the automaton starts in, before consuming `init_char`'s first
        /// transition; defaults to 0.
        ///
        /// Must be `< num_states`. This is a precondition, not just a default:
        /// `SdaGenome::run` indexes its response table with this value, so an
        /// out-of-range `init_state` panics during expression. Whatever maps
        /// this onto `SdaContext` is responsible for rejecting it at startup.
        #[serde(default)]
        init_state: usize,
    },
}

/// Fitness objective and its parameters.
///
/// The three epidemic objectives read the same simulation differently (spec
/// §5.2), so they share one parameter block rather than triplicating it — see
/// [`SirParams`]. `epi_prof_match` is the only one that adds anything.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FitnessConfig {
    /// Total ever-infected. Maximized.
    EpiSpread {
        #[serde(flatten)]
        sir: SirParams,
    },
    /// Timesteps to burn out. Maximized.
    EpiLength {
        #[serde(flatten)]
        sir: SirParams,
    },
    /// RMSE against a target profile. Minimized.
    EpiProfMatch {
        #[serde(flatten)]
        sir: SirParams,
        /// File holding the target profile.
        ///
        /// Stored, never opened here: parsing a config must stay pure
        /// deserialization (spec §7), so reading and validating this file
        /// belongs to whatever builds the objective. The file's format is not
        /// settled — see `collab.md` item 24.
        target_profile_path: PathBuf,
    },
    /// A Python callable registered before the run. Its direction is declared
    /// at registration, not here (spec §7).
    Python,
}

/// Epidemic sampling parameters, shared by the three SIR objectives.
///
/// **Not the same type as [`crate::sir::SirParams`], despite the name.** That
/// one is the simulator's own two-field struct (`infection_rate`,
/// `patient_zero`) and is deliberately independent of the config schema; this
/// one is the deserializable `[fitness]` block and also carries the batch
/// settings. This type maps onto [`crate::sir::SirBatchParams`], not onto its
/// namesake.
///
/// **No seed appears here.** One master seed is supplied to the Python `run`
/// call and everything derives from it (spec §7).
#[derive(Debug, Clone, Deserialize)]
pub struct SirParams {
    /// Per-edge transmission probability per timestep.
    pub infection_rate: f64,
    /// Pinned patient zero; omitted draws a fresh node per epidemic.
    #[serde(default)]
    pub patient_zero: Option<usize>,
    /// Outbreaks averaged per evaluation.
    pub num_epidemics: usize,
    /// Outbreaks shorter than this are re-rolled; 1 disables the re-roll.
    /// Defaults to the C++ `mepl`.
    #[serde(default = "default_min_epidemic_length")]
    pub min_epidemic_length: usize,
    /// Attempts before keeping whatever came out. Defaults to the C++ `rse`.
    #[serde(default = "default_max_epidemic_retries")]
    pub max_epidemic_retries: usize,
}

fn default_max_edge_multiplicity() -> u32 {
    1
}

fn default_elite_count() -> usize {
    1
}

fn default_max_mutations() -> usize {
    1
}

// The two below are the legacy C++ constants `mepl` and `rse`, so an omitted
// pair reproduces historical behaviour (spec §5.2). Both need an explicit
// function rather than `#[serde(default)]`, which yields 0 on a `usize` — zero
// retries would mean no epidemic runs at all.

fn default_min_epidemic_length() -> usize {
    3
}

fn default_max_epidemic_retries() -> usize {
    5
}

/// Failure while loading a [`Config`].
#[derive(Debug)]
pub enum ConfigError {
    /// The config file could not be read.
    Io(std::io::Error),
    /// The config text was not valid TOML for a [`Config`].
    Toml(toml::de::Error),
}

impl Config {
    /// Parse a [`Config`] from TOML text.
    pub fn from_toml_str(text: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(text)
    }

    /// Read and parse a [`Config`] from a TOML file on disk.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let _ = path.as_ref();
        todo!("read the file, then delegate to `from_toml_str`")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal valid config; `genome_extra` is appended to the `[genome]`
    /// table so each test can vary only the operation-weight block.
    fn config_text(genome_extra: &str) -> String {
        format!(
            r#"
population_size = 200
network_size    = 100
crossover_rate  = 0.9
mutation_rate   = 0.2

[evolution]
type            = "generational"
num_generations = 500

[selection]
type            = "tournament"
tournament_size = 5

[genome]
type        = "edge_edit"
gene_length = 256
{genome_extra}

[fitness]
type           = "epi_spread"
infection_rate = 0.05
num_epidemics  = 30
"#
        )
    }

    fn edge_edit_weights(text: &str) -> EdgeEditOperationWeights {
        match Config::from_toml_str(text)
            .expect("config should parse")
            .genome
        {
            GenomeConfig::EdgeEdit {
                operation_weights, ..
            } => operation_weights,
            other => panic!("expected an edge-edit genome, got {other:?}"),
        }
    }

    #[test]
    fn an_omitted_operation_weight_table_defaults_every_operation() {
        assert_eq!(
            edge_edit_weights(&config_text("")),
            EdgeEditOperationWeights::default()
        );
    }

    #[test]
    fn a_partial_operation_weight_table_leaves_unlisted_operations_at_one() {
        let weights = edge_edit_weights(&config_text(
            "\n[genome.operation_weights]\ntoggle = 2.5\nnull = 0.0\n",
        ));

        assert_eq!(weights.toggle, 2.5);
        assert_eq!(weights.null, 0.0);
        assert_eq!(
            EdgeEditOperationWeights {
                toggle: 1.0,
                null: 1.0,
                ..weights
            },
            EdgeEditOperationWeights::default(),
            "operations absent from the table should keep the default weight"
        );
    }

    #[test]
    fn a_full_operation_weight_table_round_trips() {
        let weights = edge_edit_weights(&config_text(
            "\n[genome.operation_weights]\n\
             toggle = 1.0\nhop = 2.0\nadd = 3.0\ndelete = 4.0\nswap = 5.0\n\
             local_toggle = 6.0\nlocal_add = 7.0\nlocal_delete = 8.0\nnull = 9.0\n",
        ));

        assert_eq!(
            weights,
            EdgeEditOperationWeights {
                toggle: 1.0,
                hop: 2.0,
                add: 3.0,
                delete: 4.0,
                swap: 5.0,
                local_toggle: 6.0,
                local_add: 7.0,
                local_delete: 8.0,
                null: 9.0,
            }
        );
    }

    #[test]
    fn a_misspelled_operation_name_is_an_error_rather_than_a_silent_default() {
        let error =
            Config::from_toml_str(&config_text("\n[genome.operation_weights]\ntogle = 2.0\n"))
                .expect_err("an unknown operation name should not parse");

        assert!(
            error.to_string().contains("togle"),
            "the error should name the offending key, got: {error}"
        );
    }

    /// Swap the whole `[genome]` table for an SDA one, varying only what the
    /// test cares about.
    fn sda_config_text(genome_extra: &str) -> String {
        config_text("").replace(
            "type        = \"edge_edit\"\ngene_length = 256",
            &format!("type = \"sda\"\nnum_states = 12\nmax_resp_len = 4{genome_extra}"),
        )
    }

    #[test]
    fn an_omitted_sda_init_state_defaults_to_zero() {
        match Config::from_toml_str(&sda_config_text(""))
            .expect("sda config should parse")
            .genome
        {
            GenomeConfig::Sda { init_state, .. } => assert_eq!(init_state, 0),
            other => panic!("expected an sda genome, got {other:?}"),
        }
    }

    #[test]
    fn an_explicit_sda_init_state_round_trips() {
        match Config::from_toml_str(&sda_config_text("\ninit_state = 7"))
            .expect("sda config should parse")
            .genome
        {
            GenomeConfig::Sda {
                init_state,
                num_states,
                ..
            } => {
                assert_eq!(init_state, 7);
                assert_eq!(num_states, 12);
            }
            other => panic!("expected an sda genome, got {other:?}"),
        }
    }

    #[test]
    fn an_omitted_max_mutations_defaults_to_one() {
        let config = Config::from_toml_str(&config_text("")).expect("config should parse");

        // The default has to be 1, not 0: `mutate_child` asserts on a zero, and
        // an omitted field must give the pre-`max_mutations` behaviour of one
        // mutation per mutating child.
        assert_eq!(config.max_mutations, 1);
    }

    #[test]
    fn an_explicit_max_mutations_round_trips() {
        // Prepended, so it lands in the top-level table ahead of `[evolution]`.
        let config = Config::from_toml_str(&format!("max_mutations = 4\n{}", config_text("")))
            .expect("config should parse");

        assert_eq!(config.max_mutations, 4);
    }

    /// Swap the whole `[fitness]` table for another one.
    fn fitness_config_text(fitness_block: &str) -> String {
        let text = config_text("");
        let start = text
            .find("[fitness]")
            .expect("the fixture should have a fitness table");
        format!("{}{fitness_block}", &text[..start])
    }

    fn fitness_of(text: &str) -> FitnessConfig {
        Config::from_toml_str(text)
            .expect("config should parse")
            .fitness
    }

    #[test]
    fn each_epidemic_objective_parses_and_shares_one_parameter_block() {
        // The point of the flatten: the same flat TOML keys serve all three.
        let params = "infection_rate = 0.05\nnum_epidemics = 30\n";

        let spread = fitness_of(&fitness_config_text(&format!(
            "[fitness]\ntype = \"epi_spread\"\n{params}"
        )));
        let length = fitness_of(&fitness_config_text(&format!(
            "[fitness]\ntype = \"epi_length\"\n{params}"
        )));
        let matched = fitness_of(&fitness_config_text(&format!(
            "[fitness]\ntype = \"epi_prof_match\"\n{params}target_profile_path = \"p/Profile3.dat\"\n"
        )));

        match (spread, length, matched) {
            (
                FitnessConfig::EpiSpread { sir: spread },
                FitnessConfig::EpiLength { sir: length },
                FitnessConfig::EpiProfMatch {
                    sir: matched,
                    target_profile_path,
                },
            ) => {
                assert_eq!(spread.infection_rate, 0.05);
                assert_eq!(length.num_epidemics, 30);
                assert_eq!(matched.infection_rate, 0.05);
                assert_eq!(target_profile_path, PathBuf::from("p/Profile3.dat"));
            }
            other => panic!("expected the three epidemic objectives, got {other:?}"),
        }
    }

    #[test]
    fn a_python_fitness_block_parses() {
        // `config.example.toml` documented this block long before the enum
        // could parse it.
        match fitness_of(&fitness_config_text("[fitness]\ntype = \"python\"\n")) {
            FitnessConfig::Python => {}
            other => panic!("expected a python objective, got {other:?}"),
        }
    }

    #[test]
    fn omitted_retry_settings_default_to_the_cpp_constants() {
        // Both need explicit default fns: `#[serde(default)]` on a `usize`
        // gives 0, and zero retries would run no epidemic at all.
        match fitness_of(&config_text("")) {
            FitnessConfig::EpiSpread { sir } => {
                assert_eq!(sir.min_epidemic_length, 3);
                assert_eq!(sir.max_epidemic_retries, 5);
                assert_eq!(sir.patient_zero, None);
            }
            other => panic!("expected epi_spread, got {other:?}"),
        }
    }

    #[test]
    fn explicit_retry_settings_round_trip() {
        match fitness_of(&fitness_config_text(
            "[fitness]\ntype = \"epi_length\"\ninfection_rate = 0.1\nnum_epidemics = 2\n\
             min_epidemic_length = 1\nmax_epidemic_retries = 9\npatient_zero = 4\n",
        )) {
            FitnessConfig::EpiLength { sir } => {
                assert_eq!(sir.min_epidemic_length, 1);
                assert_eq!(sir.max_epidemic_retries, 9);
                assert_eq!(sir.patient_zero, Some(4));
            }
            other => panic!("expected epi_length, got {other:?}"),
        }
    }

    #[test]
    fn epi_prof_match_without_a_target_profile_path_is_an_error() {
        Config::from_toml_str(&fitness_config_text(
            "[fitness]\ntype = \"epi_prof_match\"\ninfection_rate = 0.05\nnum_epidemics = 30\n",
        ))
        .expect_err("epi_prof_match should require a target profile");
    }

    #[test]
    fn a_missing_num_epidemics_is_an_error_rather_than_a_silent_default() {
        // Deliberately required: silently averaging some default number of
        // epidemics would change what every fitness value means.
        Config::from_toml_str(&fitness_config_text(
            "[fitness]\ntype = \"epi_spread\"\ninfection_rate = 0.05\n",
        ))
        .expect_err("num_epidemics should be required");
    }

    #[test]
    fn an_unknown_fitness_key_is_ignored_rather_than_rejected() {
        // Pinned deliberately, because it is a gap rather than a design choice.
        // Issue #24 asked for a stray `seed` to be *rejected* as an unknown
        // key. It cannot be: serde buffers the content of a `#[serde(flatten)]`
        // field, so `deny_unknown_fields` never fires — measured 2026-08-05,
        // including with the attribute on `SirParams` itself. Spec §7 requires
        // the flatten, so the flatten wins and this is the behaviour.
        //
        // The migration hazard is real — an old config keeping `seed = 42` gets
        // a different seeding model with no error — so catching it belongs to
        // `Config::validate` (#23), which sees the raw text. See `collab.md` 25.
        match fitness_of(&fitness_config_text(
            "[fitness]\ntype = \"epi_spread\"\ninfection_rate = 0.05\n\
             num_epidemics = 30\nseed = 42\n",
        )) {
            FitnessConfig::EpiSpread { sir } => assert_eq!(sir.num_epidemics, 30),
            other => panic!("expected epi_spread, got {other:?}"),
        }
    }

    #[test]
    fn the_example_config_parses() {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../config.example.toml"
        ))
        .expect("config.example.toml should be readable");

        Config::from_toml_str(&text).expect("the shipped example config should parse");
    }
}
