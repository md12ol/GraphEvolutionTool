//! Deserializable mirror of `config.toml`.
//!
//! These are mostly flat, owned structs rather than the engine's own context
//! types: [`crate::evolver::SharedEvolutionContext`] is generic over the genome
//! and carries a non-deserializable `Genome::Context`, so configuration is
//! parsed into this plain shape and then mapped onto concrete engine types by
//! the dispatch layer in `dispatch.rs`.
//!
//! Plain data types that carry no such baggage are deserialized directly rather
//! than mirrored — [`EdgeEditOperationWeights`] is nine `f64`s with a `Default`,
//! and duplicating it here would buy nothing but a conversion to maintain.
//!
//! [`crate::py_config`] mirrors these types field-for-field for the Python
//! front end, and cannot be collapsed into them — pyo3 and serde disagree
//! about one variant of [`FitnessConfig`]. See that module's header for the
//! mechanism.

use std::path::Path;

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
        /// The target profile itself, inline in the config (spec §8).
        ///
        /// Compared verbatim. Neither C++ loading convention is reproduced —
        /// no patient-zero element is prepended and nothing is rescaled by
        /// `verts / 128` — so this is the profile the run is scored against,
        /// at the size of the network being built.
        target_profile: Vec<f64>,
    },
    /// A Python callable registered before the run. Its direction is declared
    /// at registration, not here (spec §7).
    Python,
}

impl FitnessConfig {
    /// The `type` string this variant is written as in `config.toml`.
    ///
    /// For error messages that have to name the configured objective back to
    /// the user in the words they typed. `Debug` would print the variant's
    /// fields too, which is not what a message about the *choice* wants.
    pub fn type_name(&self) -> &'static str {
        match self {
            FitnessConfig::EpiSpread { .. } => "epi_spread",
            FitnessConfig::EpiLength { .. } => "epi_length",
            FitnessConfig::EpiProfMatch { .. } => "epi_prof_match",
            FitnessConfig::Python => "python",
        }
    }
}

/// Epidemic sampling parameters, shared by the three SIR objectives.
///
/// **Not the same type as [`crate::sir::SirParams`], despite the name.** That
/// one is the simulator's own two-field struct (`infection_rate`,
/// `patient_zero`) and is deliberately independent of the config schema; this
/// one is the deserializable `[fitness]` block and also carries the batch
/// settings. This type maps onto [`crate::sir::SirSampleParams`], not onto its
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
    /// The config parsed, but broke one of the constraints in spec §7.
    ///
    /// The field and the constraint are kept apart rather than pre-formatted
    /// into one string: the Python front end has to build its own exception
    /// message, and tests assert on `field` instead of matching prose that
    /// would break every time the wording changes.
    Validation {
        /// The offending field, spelled as it appears in the TOML.
        field: &'static str,
        /// What that field must satisfy, phrased for the user.
        constraint: String,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(error) => write!(f, "could not read the config file: {error}"),
            ConfigError::Toml(error) => write!(f, "could not parse the config: {error}"),
            ConfigError::Validation { field, constraint } => {
                write!(f, "invalid config: `{field}` {constraint}")
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::Io(error) => Some(error),
            ConfigError::Toml(error) => Some(error),
            // Nothing underlies a validation failure — it is our own check, not
            // a wrapped error from another library.
            ConfigError::Validation { .. } => None,
        }
    }
}

/// Build a [`ConfigError::Validation`] for `field`.
///
/// A free function rather than an inline struct literal so the twelve checks in
/// [`Config::validate`] each read as one line.
fn invalid(field: &'static str, constraint: impl Into<String>) -> ConfigError {
    ConfigError::Validation {
        field,
        constraint: constraint.into(),
    }
}

/// Reject the two `[fitness]` keys serde discards in silence, reading the raw
/// text: a leftover `seed`, and a `target_profile` under the wrong objective.
///
/// Serde cannot do either. [`SirParams`] is `#[serde(flatten)]`ed into
/// [`FitnessConfig`], and a flattened field deserializes through a buffered
/// content map, so `deny_unknown_fields` never fires — an unrecognized key is
/// discarded without a word. Both keys here are dangerous for the same reason:
///
/// - `seed` — a config written before the seed moved out of `[fitness]` still
///   parses, and runs under a different seeding model than its author believes,
///   since the master seed now comes from the `run` call (spec §7).
/// - `target_profile` — spec §8 requires it be "rejected as a contradiction if
///   supplied for any other objective". Left alone, someone who switches
///   objective and forgets the profile behind believes they are matching a
///   curve while the run maximizes spread.
///
/// **Deliberately two keys by name, not a general unknown-key sweep.** That
/// would hand-roll what serde does everywhere else and start rejecting keys as
/// the schema grows — the narrowness is a recorded choice (`collab.md` #25),
/// pinned by `an_unknown_fitness_key_outside_the_two_named_ones_is_still_ignored`.
///
/// This is the TOML path only — the Python front end has no text to inspect,
/// and `target_profile` exists there only on the `EpiProfMatch` variant, so the
/// contradiction cannot be expressed. It has to happen here rather than in
/// [`Config::validate`], which sees an already-parsed config with the key gone.
fn reject_stray_fitness_keys(text: &str) -> Result<(), ConfigError> {
    // Parsed loosely, as plain TOML values, so this sees keys the `Config`
    // schema throws away. Text that isn't valid TOML at all is left for the
    // real parse below to report properly.
    let document: toml::Value = match toml::from_str(text) {
        Ok(value) => value,
        Err(_) => return Ok(()),
    };

    let Some(fitness) = document.get("fitness") else {
        return Ok(());
    };

    if fitness.get("seed").is_some() {
        return Err(invalid(
            "seed",
            "is not a config field. One master seed is supplied to the `run` call and \
             everything derives from it (spec §7), so remove `seed` from `[fitness]`",
        ));
    }

    // Three conditions, all of which must hold: a profile was supplied, the
    // objective is present and reads as a string, and it is not the one
    // objective that owns a profile. A missing, misspelled or non-string
    // `type` falls through to the real parse, which names that problem far
    // better than a message about the profile would.
    let profile_supplied = fitness.get("target_profile").is_some();

    if profile_supplied
        && let Some(objective) = fitness.get("type").and_then(|value| value.as_str())
        && objective != "epi_prof_match"
    {
        return Err(invalid(
            "target_profile",
            format!(
                "belongs to `epi_prof_match` alone (spec §8), but the objective here is \
                 `{objective}`, which never reads a profile. Either set \
                 `type = \"epi_prof_match\"` or remove `target_profile`"
            ),
        ));
    }
    Ok(())
}

impl Config {
    /// Parse a [`Config`] from TOML text, **without** checking spec §7's
    /// constraints.
    ///
    /// Rejects a stray `[fitness] seed`, and a `target_profile` supplied under
    /// an objective that is not `epi_prof_match`. Both are parse-time concerns
    /// because neither key survives deserialization — see
    /// `reject_stray_fitness_keys`. Everything else is [`Config::validate`],
    /// kept separate so a test can build a config that deliberately breaks one
    /// constraint.
    pub fn from_toml_str(text: &str) -> Result<Self, ConfigError> {
        reject_stray_fitness_keys(text)?;
        toml::from_str(text).map_err(ConfigError::Toml)
    }

    /// Read, parse **and validate** a [`Config`] from a TOML file on disk.
    ///
    /// This is the TOML front end, so it calls [`Config::validate`] itself.
    /// Spec §7 requires both front ends to validate through that one function;
    /// the Python one calls it at its own boundary, having never touched serde.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let text = std::fs::read_to_string(path).map_err(ConfigError::Io)?;
        let config = Self::from_toml_str(&text)?;
        config.validate()?;
        Ok(config)
    }

    /// Check every constraint in spec §7.
    ///
    /// **Returns an error; never panics.** A bad config is a user mistake, not
    /// a bug, and a panic crossing the FFI reaches the user as an opaque
    /// `PanicException` they cannot act on. The `assert!`s inside the evolvers
    /// stay as backstops for direct Rust use, but a config-driven run must
    /// never reach one.
    ///
    /// Runs before anything is built — no population, no graph — so everything
    /// downstream may assume a valid config.
    ///
    /// One thing it deliberately does not do: it does not check the base
    /// graph, which belongs to `set_base_graph`.
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.validate_top_level()?;
        self.validate_evolution_and_selection()?;
        self.validate_genome()?;
        self.validate_fitness()?;
        Ok(())
    }

    /// Constraints on the fields that apply to every run.
    fn validate_top_level(&self) -> Result<(), ConfigError> {
        // Not an SDA-only concern despite motivating the alphabet in §3.2: a
        // cap of 0 clamps every edge to zero weight under *any* genome, and the
        // run then looks like a broken fitness function rather than a bad
        // config.
        if self.max_edge_multiplicity == 0 || self.max_edge_multiplicity > 255 {
            return Err(invalid(
                "max_edge_multiplicity",
                "must be between 1 and 255",
            ));
        }

        if self.max_mutations == 0 {
            return Err(invalid(
                "max_mutations",
                "must be at least 1, since a mutating child takes 1..=max_mutations mutations",
            ));
        }

        if !(0.0..=1.0).contains(&self.crossover_rate) {
            return Err(invalid("crossover_rate", "must be between 0.0 and 1.0"));
        }

        if !(0.0..=1.0).contains(&self.mutation_rate) {
            return Err(invalid("mutation_rate", "must be between 0.0 and 1.0"));
        }
        Ok(())
    }

    /// Constraints that read the evolution strategy and the selection scheme
    /// together. Both live here because two of the three are strategy-specific.
    fn validate_evolution_and_selection(&self) -> Result<(), ConfigError> {
        // Irrefutable today — one variant. If a second selection scheme is
        // added, this stops compiling, which is the right way to find out.
        let SelectionConfig::Tournament { tournament_size } = self.selection;

        if tournament_size > self.population_size {
            return Err(invalid(
                "population_size",
                format!("must be at least tournament_size ({tournament_size})"),
            ));
        }

        match self.evolution {
            EvolutionConfig::Generational { elite_count, .. } => {
                if elite_count >= self.population_size {
                    return Err(invalid(
                        "elite_count",
                        format!(
                            "must be less than population_size ({}); equal means nothing \
                             breeds and the run is a fixed point",
                            self.population_size
                        ),
                    ));
                }
            }
            EvolutionConfig::SteadyState { .. } => {
                // Steady-state only. Generational has no such floor, so this
                // cannot be a blanket check (spec §7).
                if tournament_size < 4 {
                    return Err(invalid(
                        "tournament_size",
                        "must be at least 4 for the steady-state evolver",
                    ));
                }
            }
        }
        Ok(())
    }

    /// Constraints on the genome and its dimensions.
    fn validate_genome(&self) -> Result<(), ConfigError> {
        match &self.genome {
            GenomeConfig::EdgeEdit {
                operation_weights, ..
            } => {
                // The weights already own their rules; map the message rather
                // than restating it here and letting the two drift.
                if let Err(constraint) = operation_weights.validate() {
                    return Err(invalid("operation_weights", constraint));
                }
            }
            GenomeConfig::Sda {
                num_states,
                init_state,
                ..
            } => {
                if init_state >= num_states {
                    return Err(invalid(
                        "init_state",
                        format!(
                            "must be less than num_states ({num_states}); SdaGenome::run \
                             indexes its response table with it"
                        ),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Constraints on the epidemic sampling parameters.
    fn validate_fitness(&self) -> Result<(), ConfigError> {
        // `python` carries no SIR block at all — its parameters belong to the
        // callable, which is registered from Python (spec §7).
        let sir = match &self.fitness {
            FitnessConfig::EpiSpread { sir } => sir,
            FitnessConfig::EpiLength { sir } => sir,
            FitnessConfig::EpiProfMatch { sir, .. } => sir,
            FitnessConfig::Python => return Ok(()),
        };

        if !(0.0..=1.0).contains(&sir.infection_rate) {
            return Err(invalid("infection_rate", "must be between 0.0 and 1.0"));
        }

        if sir.num_epidemics == 0 {
            return Err(invalid(
                "num_epidemics",
                "must be at least 1, since fitness is their average",
            ));
        }

        // 1 is legal and means "never re-roll": every epidemic has length >= 1
        // under the §5.2 convention, so 1 is how a user opts out of the
        // re-roll's deliberate bias. Only 0 is an error.
        if sir.min_epidemic_length == 0 {
            return Err(invalid(
                "min_epidemic_length",
                "must be at least 1; 1 disables the re-roll",
            ));
        }

        if sir.max_epidemic_retries == 0 {
            return Err(invalid(
                "max_epidemic_retries",
                "must be at least 1, or no epidemic is ever run",
            ));
        }

        // Only checked when pinned; an omitted `patient_zero` draws a fresh
        // node per epidemic and cannot be out of range.
        if let Some(patient_zero) = sir.patient_zero
            && patient_zero >= self.network_size
        {
            return Err(invalid(
                "patient_zero",
                format!(
                    "must be less than network_size ({}), since it is a node index",
                    self.network_size
                ),
            ));
        }

        // `epi_prof_match` only — the other two epidemic objectives have no
        // target, and `python` returned above. These two checks are the reason
        // the profile is an inline config value rather than a path: a file
        // could not be checked here without `validate` opening it (spec §8).
        if let FitnessConfig::EpiProfMatch { target_profile, .. } = &self.fitness {
            if target_profile.is_empty() {
                return Err(invalid(
                    "target_profile",
                    "must have at least one element; there is nothing to score against otherwise",
                ));
            }

            // NaN and both infinities. RMSE against any of them is NaN or
            // infinite for *every* individual, so the whole population scores
            // identically and selection stops discriminating — a run that
            // looks like it is working and is searching nothing.
            for (index, value) in target_profile.iter().enumerate() {
                if !value.is_finite() {
                    return Err(invalid(
                        "target_profile",
                        format!("element {index} is {value}; every element must be finite"),
                    ));
                }
            }
        }
        Ok(())
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
            "[fitness]\ntype = \"epi_prof_match\"\n{params}target_profile = [0.0, 2.5, 7.0, 1.5]\n"
        )));

        match (spread, length, matched) {
            (
                FitnessConfig::EpiSpread { sir: spread },
                FitnessConfig::EpiLength { sir: length },
                FitnessConfig::EpiProfMatch {
                    sir: matched,
                    target_profile,
                },
            ) => {
                assert_eq!(spread.infection_rate, 0.05);
                assert_eq!(length.num_epidemics, 30);
                assert_eq!(matched.infection_rate, 0.05);
                assert_eq!(target_profile, vec![0.0, 2.5, 7.0, 1.5]);
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
    fn a_whole_number_in_the_target_profile_may_be_written_without_a_decimal_point() {
        // Measured 2026-08-10, not assumed: `toml` widens an integer element
        // into the `f64` the field asks for, so a hand-written `[0, 2, 7]` is
        // accepted rather than rejected as a type error. Worth a test because
        // the opposite is the obvious guess, and the natural way to write a
        // profile by hand is without decimal points.
        match fitness_of(&fitness_config_text(
            "[fitness]\ntype = \"epi_prof_match\"\ninfection_rate = 0.05\nnum_epidemics = 30\n\
             target_profile = [0, 2, 7]\n",
        )) {
            FitnessConfig::EpiProfMatch { target_profile, .. } => {
                assert_eq!(target_profile, vec![0.0, 2.0, 7.0]);
            }
            other => panic!("expected epi_prof_match, got {other:?}"),
        }
    }

    #[test]
    fn epi_prof_match_without_a_target_profile_is_an_error() {
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
    fn a_stray_fitness_seed_is_rejected_by_name() {
        // Serde still cannot catch this, and that has not changed: it buffers
        // the content of a `#[serde(flatten)]` field, so `deny_unknown_fields`
        // never fires — measured 2026-08-05 with the attribute on `SirParams`
        // itself. The key is caught by reading the raw text instead, because
        // the migration hazard is silent: an old config keeping `seed = 42`
        // would otherwise get a different seeding model with no error.
        //
        // Supersedes #24's `an_unknown_fitness_key_is_ignored_rather_than_rejected`,
        // which pinned the gap this closes.
        let error = Config::from_toml_str(&fitness_config_text(
            "[fitness]\ntype = \"epi_spread\"\ninfection_rate = 0.05\n\
             num_epidemics = 30\nseed = 42\n",
        ))
        .expect_err("a leftover `seed` should be rejected");

        match error {
            ConfigError::Validation { field, .. } => assert_eq!(field, "seed"),
            other => panic!("expected a validation error naming `seed`, got {other:?}"),
        }
    }

    #[test]
    fn a_target_profile_under_any_other_objective_is_rejected_as_a_contradiction() {
        // Spec §8: the profile is required by `epi_prof_match` and "rejected as
        // a contradiction if supplied for any other objective". Caught in the
        // same raw-text pass as `seed`, and for the same reason — the flatten
        // swallows it, so the realistic mistake of switching objective and
        // leaving the profile behind would otherwise run as a spread
        // maximization while its author believes a curve is being matched.
        //
        // One loop rather than three near-identical tests: the three objectives
        // differ only in the string, and keeping them together is what stops a
        // fourth non-matching objective being added to the enum and to two of
        // these cases.
        let others = ["epi_spread", "epi_length", "python"];

        for objective in others {
            // The epidemic parameters are valid, so a rejection cannot be a
            // missing-field error wearing this test's name. `python` ignores
            // them.
            let text = fitness_config_text(&format!(
                "[fitness]\ntype = \"{objective}\"\ninfection_rate = 0.05\n\
                 num_epidemics = 30\ntarget_profile = [1.0, 3.0, 8.0]\n"
            ));

            match Config::from_toml_str(&text) {
                Err(ConfigError::Validation { field, .. }) => {
                    assert_eq!(field, "target_profile", "wrong field for {objective}");
                }
                other => panic!("expected `{objective}` to reject a profile, got {other:?}"),
            }
        }
    }

    #[test]
    fn epi_prof_match_with_a_target_profile_is_untouched_by_the_contradiction_check() {
        // The other side of the same clause: the objective that owns the
        // profile still parses AND validates. Both halves matter — the check
        // reads `type` out of loosely-parsed TOML, so getting that comparison
        // backwards would break the one configuration it must leave alone.
        let text = fitness_config_text(
            "[fitness]\ntype = \"epi_prof_match\"\ninfection_rate = 0.05\n\
             num_epidemics = 30\ntarget_profile = [1.0, 3.0, 8.0]\n",
        );

        let config = Config::from_toml_str(&text).expect("epi_prof_match should keep its profile");
        config.validate().expect("the fixture should be valid");

        match config.fitness {
            FitnessConfig::EpiProfMatch { target_profile, .. } => {
                assert_eq!(target_profile, vec![1.0, 3.0, 8.0]);
            }
            other => panic!("expected epi_prof_match, got {other:?}"),
        }
    }

    #[test]
    fn an_unknown_fitness_key_outside_the_two_named_ones_is_still_ignored() {
        // The raw-text check is deliberately narrow — `seed` and
        // `target_profile` by name, not a general unknown-key sweep, which
        // would hand-roll what serde does everywhere else and reject keys as
        // the schema grows. Pinned so the narrowness is a recorded choice
        // rather than an oversight.
        match fitness_of(&fitness_config_text(
            "[fitness]\ntype = \"epi_spread\"\ninfection_rate = 0.05\n\
             num_epidemics = 30\nnot_a_real_key = 42\n",
        )) {
            FitnessConfig::EpiSpread { sir } => assert_eq!(sir.num_epidemics, 30),
            other => panic!("expected epi_spread, got {other:?}"),
        }
    }

    // ---- `Config::validate` — spec §7's constraints -----------------------

    /// Parse without validating, so a test can break exactly one field of an
    /// otherwise-valid fixture and check that `validate` catches that field.
    fn valid_config() -> Config {
        Config::from_toml_str(&config_text("")).expect("the fixture should parse")
    }

    /// The field named by a validation failure — or a panic saying what
    /// happened instead. Tests assert on this rather than on message prose.
    fn validation_field(config: &Config) -> &'static str {
        match config.validate() {
            Err(ConfigError::Validation { field, .. }) => field,
            Err(other) => panic!("expected a validation error, got {other:?}"),
            Ok(()) => panic!("expected the config to be rejected, but it validated"),
        }
    }

    #[test]
    fn the_fixture_config_validates() {
        // Guards every test below: they all assume the fixture is valid before
        // they break one field.
        valid_config()
            .validate()
            .expect("the test fixture should be a valid config");
    }

    #[test]
    fn a_zero_edge_multiplicity_is_rejected() {
        let mut config = valid_config();
        config.max_edge_multiplicity = 0;

        assert_eq!(validation_field(&config), "max_edge_multiplicity");
    }

    #[test]
    fn an_edge_multiplicity_above_the_byte_range_is_rejected() {
        let mut config = valid_config();
        config.max_edge_multiplicity = 256;

        assert_eq!(validation_field(&config), "max_edge_multiplicity");
    }

    #[test]
    fn a_zero_max_mutations_is_rejected() {
        let mut config = valid_config();
        config.max_mutations = 0;

        assert_eq!(validation_field(&config), "max_mutations");
    }

    #[test]
    fn a_negative_crossover_rate_is_rejected() {
        let mut config = valid_config();
        config.crossover_rate = -0.1;

        assert_eq!(validation_field(&config), "crossover_rate");
    }

    #[test]
    fn a_crossover_rate_above_one_is_rejected() {
        let mut config = valid_config();
        config.crossover_rate = 1.1;

        assert_eq!(validation_field(&config), "crossover_rate");
    }

    #[test]
    fn a_negative_mutation_rate_is_rejected() {
        let mut config = valid_config();
        config.mutation_rate = -0.1;

        assert_eq!(validation_field(&config), "mutation_rate");
    }

    #[test]
    fn a_mutation_rate_above_one_is_rejected() {
        let mut config = valid_config();
        config.mutation_rate = 1.1;

        assert_eq!(validation_field(&config), "mutation_rate");
    }

    #[test]
    fn a_negative_infection_rate_is_rejected() {
        let mut config = valid_config();
        let FitnessConfig::EpiSpread { sir } = &mut config.fitness else {
            panic!("the fixture's fitness type should be epi_spread");
        };
        sir.infection_rate = -0.1;

        assert_eq!(validation_field(&config), "infection_rate");
    }

    #[test]
    fn an_infection_rate_above_one_is_rejected() {
        let mut config = valid_config();
        let FitnessConfig::EpiSpread { sir } = &mut config.fitness else {
            panic!("the fixture's fitness type should be epi_spread");
        };
        sir.infection_rate = 1.1;

        assert_eq!(validation_field(&config), "infection_rate");
    }

    #[test]
    fn a_population_smaller_than_the_tournament_is_rejected() {
        let mut config = valid_config();
        config.population_size = 3; // the fixture's tournament_size is 5

        assert_eq!(validation_field(&config), "population_size");
    }

    #[test]
    fn an_elite_count_equal_to_the_population_is_rejected() {
        // Equal, not merely greater: equal already means nothing breeds and the
        // run is a fixed point.
        let mut config = valid_config();
        config.evolution = EvolutionConfig::Generational {
            num_generations: 500,
            elite_count: config.population_size,
        };

        assert_eq!(validation_field(&config), "elite_count");
    }

    #[test]
    fn the_tournament_floor_of_four_applies_to_steady_state_only() {
        let mut config = valid_config();
        config.selection = SelectionConfig::Tournament { tournament_size: 3 };

        // Generational has no such floor, so this must pass.
        config
            .validate()
            .expect("generational imposes no tournament floor");

        config.evolution = EvolutionConfig::SteadyState {
            num_mating_events: 1000,
        };
        assert_eq!(validation_field(&config), "tournament_size");
    }

    #[test]
    fn an_init_state_outside_the_state_count_is_rejected() {
        let mut config = valid_config();
        config.genome = GenomeConfig::Sda {
            num_states: 12,
            max_resp_len: 4,
            init_state: 12, // one past the last state
        };

        assert_eq!(validation_field(&config), "init_state");
    }

    #[test]
    fn all_zero_operation_weights_are_rejected() {
        // Delegated to `EdgeEditOperationWeights::validate`; this checks the
        // delegation reports under a field name, not that the rule is restated.
        let config = Config::from_toml_str(&config_text(
            "\n[genome.operation_weights]\n\
             toggle = 0.0\nhop = 0.0\nadd = 0.0\ndelete = 0.0\nswap = 0.0\n\
             local_toggle = 0.0\nlocal_add = 0.0\nlocal_delete = 0.0\nnull = 0.0\n",
        ))
        .expect("all-zero weights should still parse");

        assert_eq!(validation_field(&config), "operation_weights");
    }

    /// Build a config whose only unusual part is its `[fitness]` table.
    fn config_with_fitness(fitness_block: &str) -> Config {
        Config::from_toml_str(&fitness_config_text(fitness_block))
            .expect("the fitness fixture should parse")
    }

    const SIR_BASE: &str =
        "[fitness]\ntype = \"epi_spread\"\ninfection_rate = 0.05\nnum_epidemics = 30\n";

    #[test]
    fn a_zero_num_epidemics_is_rejected() {
        let config = config_with_fitness(
            "[fitness]\ntype = \"epi_spread\"\ninfection_rate = 0.05\nnum_epidemics = 0\n",
        );

        assert_eq!(validation_field(&config), "num_epidemics");
    }

    #[test]
    fn a_min_epidemic_length_of_zero_is_rejected_but_one_is_legal() {
        let zero = config_with_fitness(&format!("{SIR_BASE}min_epidemic_length = 0\n"));
        assert_eq!(validation_field(&zero), "min_epidemic_length");

        // 1 means "never re-roll" — every epidemic has length >= 1 under the
        // §5.2 convention, so 1 is the only way to opt out of the re-roll's
        // deliberate bias. Pinned so nobody "corrects" the floor to 2.
        config_with_fitness(&format!("{SIR_BASE}min_epidemic_length = 1\n"))
            .validate()
            .expect("min_epidemic_length = 1 disables the re-roll and is legal");
    }

    #[test]
    fn a_zero_max_epidemic_retries_is_rejected() {
        let config = config_with_fitness(&format!("{SIR_BASE}max_epidemic_retries = 0\n"));

        assert_eq!(validation_field(&config), "max_epidemic_retries");
    }

    #[test]
    fn a_patient_zero_outside_the_network_is_rejected() {
        // The fixture's network_size is 100, so 100 is one past the last node.
        let config = config_with_fitness(&format!("{SIR_BASE}patient_zero = 100\n"));
        assert_eq!(validation_field(&config), "patient_zero");

        config_with_fitness(&format!("{SIR_BASE}patient_zero = 99\n"))
            .validate()
            .expect("the last node is a legal patient zero");
    }

    /// An otherwise-valid `epi_prof_match` config carrying the given profile,
    /// written as TOML so the array goes through deserialization first.
    fn config_with_profile(profile: &str) -> Config {
        config_with_fitness(&format!(
            "[fitness]\ntype = \"epi_prof_match\"\ninfection_rate = 0.05\nnum_epidemics = 30\n\
             target_profile = {profile}\n"
        ))
    }

    #[test]
    fn an_empty_target_profile_is_rejected() {
        assert_eq!(
            validation_field(&config_with_profile("[]")),
            "target_profile"
        );
    }

    #[test]
    fn a_non_finite_target_profile_element_is_rejected() {
        // All three of TOML's non-finite floats, each in second position so the
        // check is seen to scan past the first element.
        for profile in ["[1.0, nan]", "[1.0, inf]", "[1.0, -inf]"] {
            assert_eq!(
                validation_field(&config_with_profile(profile)),
                "target_profile",
                "{profile} should be rejected"
            );
        }

        config_with_profile("[0.0, 2.5, 7.0]")
            .validate()
            .expect("a finite profile is legal");
    }

    #[test]
    fn a_python_objective_skips_the_epidemic_checks() {
        // `python` carries no SIR block at all, so none of the checks above
        // have anything to read.
        config_with_fitness("[fitness]\ntype = \"python\"\n")
            .validate()
            .expect("a python objective has no SIR parameters to check");
    }

    #[test]
    fn a_validation_error_names_both_the_field_and_its_constraint() {
        // What the Python front end turns into an exception message.
        let mut config = valid_config();
        config.max_mutations = 0;
        let message = config
            .validate()
            .expect_err("max_mutations = 0 should be rejected")
            .to_string();

        assert!(
            message.contains("max_mutations") && message.contains("at least 1"),
            "the message should name the field and its constraint, got: {message}"
        );
    }

    // ---- `Config::from_path` ----------------------------------------------

    #[test]
    fn the_example_config_loads_and_validates_from_disk() {
        // The whole TOML front end, end to end: read, parse, validate.
        Config::from_path(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../config.example.toml"
        ))
        .expect("the shipped example config should load and validate");
    }

    #[test]
    fn a_missing_config_file_is_an_io_error_rather_than_a_panic() {
        match Config::from_path("no/such/directory/config.toml") {
            Err(ConfigError::Io(_)) => {}
            other => panic!("expected an Io error, got {other:?}"),
        }
    }

    #[test]
    fn a_config_file_that_breaks_a_constraint_fails_to_load() {
        // `from_path` validates, so an invalid file must not yield a `Config`.
        let dir = std::env::temp_dir().join("get_config_validate_test");
        std::fs::create_dir_all(&dir).expect("the temp dir should be creatable");
        let path = dir.join("bad_config.toml");
        std::fs::write(&path, format!("max_mutations = 0\n{}", config_text("")))
            .expect("the temp config should be writable");

        let error = Config::from_path(&path).expect_err("an invalid config should not load");
        std::fs::remove_file(&path).ok();

        match error {
            ConfigError::Validation { field, .. } => assert_eq!(field, "max_mutations"),
            other => panic!("expected a validation error, got {other:?}"),
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
