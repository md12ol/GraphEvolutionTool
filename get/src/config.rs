//! Deserializable mirror of `config.toml`.
//!
//! The engine's own context types cannot be deserialized into directly — they
//! are generic over the genome and carry a non-deserializable
//! `Genome::Context` — so parsing lands in these plain structs and `dispatch`
//! maps them onto the engine's types.

use serde::Deserialize;

use crate::evolver::steady_state::{MIN_SCOPE_SIZE, PARENTS_PER_EVENT, REPLACED_PER_EVENT};
use crate::genomes::EdgeEditOperationWeights;

/// Everything the genetic algorithm needs for a run.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub evolution: EvolutionConfig,
    pub population_size: usize,
    /// Nodes in every expressed graph, whatever the representation.
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
    #[serde(default = "default_max_mutations")]
    pub max_mutations: usize,
    /// Which slice of the population one breeding event draws from.
    pub scope: ScopeConfig,
    /// Parent-selection strategy, applied within that scope.
    pub selection: SelectionConfig,
    /// Recombination operator; omitted, two-point.
    #[serde(default)]
    pub crossover: CrossoverConfig,
    pub genome: GenomeConfig,
    pub fitness: FitnessConfig,
}

/// Evolution strategy and its strategy-specific settings.
#[derive(Debug, Deserialize)]
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
        /// Which members of the scope a mating event's children overwrite.
        #[serde(default)]
        replacement: ReplacementConfig,
    },
    // ADD A STRATEGY STEP 2 — a variant here, carrying your stopping condition
    // and any axis that is yours rather than every strategy's. What every
    // strategy shares is already on `Config`. The variant name becomes
    // `type = "my_strategy"` under `[evolution]`, via the `rename_all` above.
    //
    //     MyStrategy {
    //         num_my_events: usize,
    //     },
}

/// The slice of the population one breeding event draws from. Maps onto
/// [`crate::evolver::scope::Scope`].
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScopeConfig {
    /// Every individual is a candidate.
    Global,
    /// `size` distinct individuals, drawn fresh for each breeding event.
    RandomSubset { size: usize },
    // ADD A SCOPE STEP 3 — the variant a user names under `[scope]`, mirroring
    // the one added to `Scope`. Give it parameters of its own rather than
    // reading another block's.
    //
    //     Neighbourhood { radius: usize },
}

/// Which members of a scope a mating event's children overwrite. Maps onto
/// [`crate::evolver::replacement::Replacement`].
#[derive(Debug, Default, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReplacementConfig {
    /// The least fit of the scope. What makes steady-state self-elitist: the
    /// scope's best is never among those overwritten.
    #[default]
    Worst,
    /// Distinct members of the scope, drawn uniformly. Not self-elitist.
    Random,
    // ADD A REPLACEMENT STEP 3 (for SteadyState) — the variant a user names
    // under `[evolution] replacement`, mirroring the one added to
    // `Replacement`. Say at the variant what the policy gives up.
    //
    //     Tournament { size: usize },
}

/// Parent-selection strategy. Maps onto [`crate::evolver::common::Selection`].
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SelectionConfig {
    /// The fittest of the scope.
    Best,
    /// A tournament of `tournament_size`, sampled with replacement from the
    /// scope. `tournament_size` sizes the tournament and nothing else — the
    /// scope has its own `size` under `[scope]`.
    Tournament { tournament_size: usize },
    // ADD A SELECTION STEP 3 — the variant a user names under `[selection]`,
    // mirroring the one added to `Selection`. Constrain its own parameters in
    // `validate_evolution_and_selection`; there is no scheme-by-strategy check
    // to extend, since every scheme works with every strategy.
    //
    //     Roulette { pressure: f64 },
}

/// Recombination operator. Maps onto [`crate::evolver::common::Crossover`].
///
/// `[crossover]` is a shared section, chosen independently of `[genome]`, so an
/// operator only some representations can honour is rejected by
/// [`Config::validate_crossover`] rather than by the type system.
#[derive(Debug, Default, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CrossoverConfig {
    /// Swap one contiguous band between the parents.
    #[default]
    TwoPoint,
    // ADD A CROSSOVER STEP 3 — a variant here, matching the one added to
    // `Crossover`. Pair it with each representation in
    // `Config::validate_crossover`.
    //
    //     MyCrossover { some_param: f64 },
}

/// Genome representation and the dimensions used to build random individuals.
///
/// A variant carries only what a *random* individual is built from, plus
/// whatever the representation's own mutation needs — not run-level settings
/// like `network_size` or `max_edge_multiplicity`, which are top-level and
/// reach the genome through its context.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GenomeConfig {
    EdgeEdit(EdgeEditGenomeConfig),
    Sda(SdaGenomeConfig),
    // ADD A GENOME STEP 4 — a variant here, plus the struct it carries. The
    // variant name becomes `type = "my_genome"` under `[genome]`, via the
    // `rename_all` above.
    //
    //     MyGenome(MyGenomeConfig),
    //
    //     #[derive(Debug, Deserialize)]
    //     #[serde(deny_unknown_fields)]
    //     pub struct MyGenomeConfig {
    //         pub some_dimension: usize,
    //     }
}

/// Everything the edge-edit genome takes from `[genome]`.
///
/// `deny_unknown_fields`: every key here is required or defaulted, so an
/// unrecognized one is a typo, and ignoring it silently runs with parameters
/// its author never chose.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeEditGenomeConfig {
    pub gene_length: usize,
    /// Relative probability of each edit operation. Omitted entirely, or
    /// omitted field by field, every operation defaults to a weight of 1.0.
    #[serde(default)]
    pub operation_weights: EdgeEditOperationWeights,
    /// Which mutation this representation applies.
    #[serde(default)]
    pub mutation: EdgeEditMutationConfig,
    /// A file the run starts from, instead of the empty graph.
    ///
    /// Resolved relative to the config file's own directory, so a folder
    /// holding a config and the graph it names runs from anywhere. The file
    /// needs a `# nodes = N` header stating the count, and that count must be
    /// `network_size`.
    ///
    /// Read as 0-indexed, and there is no key to say otherwise: this route
    /// writes its output 0-indexed too, so a graph it produced reloads
    /// unchanged. A file numbered from 1 goes through
    /// `GraphEvolver::set_base_graph_from_file`, which takes a
    /// `min_node_index`.
    ///
    /// Nested under `[genome]` because it is edge-edit's alone — SDA generates
    /// its graph rather than editing one, and naming this under an SDA
    /// `[genome]` is refused by `deny_unknown_fields` above rather than by any
    /// check of ours.
    #[serde(default)]
    pub base_graph: Option<String>,
}

/// Which mutation an edge-edit genome applies. Mirrors
/// [`crate::genomes::EdgeEditMutation`], mapped in `dispatch::edge_edit_mutation`.
#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EdgeEditMutationConfig {
    /// Reroll one gene from the operation mix.
    #[default]
    RerollGene,
    // ADD A MUTATION STEP 3 (for EdgeEdit) — a variant here, matching the one
    // added to `EdgeEditMutation`.
    //
    //     MyMutation { some_param: f64 },
}

/// Everything the sda genome takes from `[genome]`.
///
/// No `num_chars`: the alphabet is derived as `max_edge_multiplicity + 1`, so
/// every character is a legal edge weight.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SdaGenomeConfig {
    pub num_states: usize,
    pub max_resp_len: usize,
    /// State the automaton starts in; defaults to 0. Must be `< num_states`,
    /// or expression panics indexing the response table with it.
    #[serde(default)]
    pub init_state: usize,
    /// Chance a mutation redraws the initial character rather than touching
    /// the transition table.
    #[serde(default = "default_init_char_mutation_rate")]
    pub init_char_mutation_rate: f64,
    /// Applies only when the initial character was not redrawn: the chance of
    /// redrawing a transition's target state rather than its response.
    #[serde(default = "default_transition_vs_response_rate")]
    pub transition_vs_response_rate: f64,
    /// Which mutation this representation applies. The rates above shape it;
    /// this selects it.
    #[serde(default)]
    pub mutation: SdaMutationConfig,
}

/// Which mutation an SDA genome applies. Mirrors
/// [`crate::genomes::SdaMutation`], mapped in `dispatch::sda_mutation`.
#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SdaMutationConfig {
    /// Redraw the initial character, a transition target or a response,
    /// chosen by the rates above.
    #[default]
    RedrawOne,
    // ADD A MUTATION STEP 3 (for SDA) — a variant here, matching the one added
    // to `SdaMutation`.
    //
    //     MyMutation { some_param: f64 },
}

/// Fitness objective and its parameters.
///
/// The epidemic objectives read the same simulation differently, so they share
/// one parameter block rather than triplicating it — see [`SirParams`].
#[derive(Debug, Deserialize)]
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
        /// The target profile, inline in the config and compared verbatim:
        /// no element is prepended and nothing is rescaled, so it must already
        /// be at the size of the network being built.
        target_profile: Vec<f64>,
    },
    /// How closely a graph's structure matches a set of reference graphs.
    ///
    /// Minimized: zero is a perfect match. Unweighted by construction, so it
    /// requires `max_edge_multiplicity = 1`.
    ///
    /// The degree axis has no top in the config, deliberately: it is taken from
    /// the reference set. A top set too low squashes every reference graph into
    /// the last bin, so their histograms come out identical and the degree
    /// family contributes nothing to any score, with nothing reporting it.
    StructMatch {
        /// Folder of reference graphs, one edge-list file per graph. Not
        /// opened until the objective is built, so an unreadable or empty
        /// folder is reported then rather than at parse.
        reference_folder: String,
        /// Bins per statistic family. More bins resolve finer differences and
        /// need more reference graphs to fill them.
        #[serde(default = "default_struct_bins")]
        degree_bins: usize,
        #[serde(default = "default_struct_bins")]
        clustering_bins: usize,
        #[serde(default = "default_struct_bins")]
        spectral_bins: usize,
        /// RBF bandwidths, one per family. **Too large is the dangerous
        /// direction**: `exp(-gamma * d^2)` collapses to zero for every
        /// candidate, so the population scores alike and evolution stalls while
        /// appearing to run normally.
        #[serde(default = "default_struct_gamma")]
        degree_gamma: f64,
        #[serde(default = "default_struct_gamma")]
        clustering_gamma: f64,
        #[serde(default = "default_struct_gamma")]
        spectral_gamma: f64,
        /// How much each family counts. Zero switches a family off; they
        /// cannot all be zero, which would score every candidate identically.
        /// A weight is also only as live as the reference set makes it — a set
        /// whose graphs all have clustering coefficient 0 leaves
        /// `clustering_weight` set and the family it weights inert.
        #[serde(default = "default_struct_weight")]
        degree_weight: f64,
        #[serde(default = "default_struct_weight")]
        clustering_weight: f64,
        #[serde(default = "default_struct_weight")]
        spectral_weight: f64,
        /// How much the distance from the reference set's mean density counts.
        /// Zero switches the penalty off.
        #[serde(default = "default_struct_weight")]
        density_weight: f64,
    },
    // ADD AN OBJECTIVE STEP 2 — a variant here, carrying whatever the
    // objective reads out of the file.
    //
    //     MyObjective { threshold: f64 },
    /// A Python callable registered before the run. Its direction is declared
    /// at registration, not here.
    Python,
}

impl FitnessConfig {
    /// The `type` string this variant is written as in `config.toml`, for
    /// error messages that name the objective back in the words the user typed.
    pub fn type_name(&self) -> &'static str {
        match self {
            FitnessConfig::EpiSpread { .. } => "epi_spread",
            FitnessConfig::EpiLength { .. } => "epi_length",
            FitnessConfig::EpiProfMatch { .. } => "epi_prof_match",
            FitnessConfig::StructMatch { .. } => "struct_match",
            // ADD AN OBJECTIVE STEP 2 — the name a user writes under
            // `[fitness] type`. Derived from nothing: this string is the whole
            // mapping, and it is what error messages print.
            //
            //     FitnessConfig::MyObjective { .. } => "my_objective",
            FitnessConfig::Python => "python",
        }
    }
}

/// Epidemic sampling parameters, shared by the SIR objectives.
///
/// **Not the same type as [`crate::sir::SirParams`], despite the name**, and it
/// maps onto [`crate::sir::SirSampleParams`] rather than onto its namesake.
///
/// No seed appears here: one master seed is supplied to the `run` call and
/// everything derives from it.
#[derive(Debug, Deserialize)]
pub struct SirParams {
    /// Per-edge transmission probability per timestep.
    pub infection_rate: f64,
    /// Pinned patient zero; omitted draws a fresh node per epidemic.
    #[serde(default)]
    pub patient_zero: Option<usize>,
    /// Outbreaks averaged per evaluation.
    pub num_epidemics: usize,
    /// Outbreaks shorter than this are re-rolled; 1 disables the re-roll.
    #[serde(default = "default_min_epidemic_length")]
    pub min_epidemic_length: usize,
    /// Attempts before keeping whatever came out.
    #[serde(default = "default_max_epidemic_retries")]
    pub max_epidemic_retries: usize,
}

fn default_max_edge_multiplicity() -> u32 {
    1
}

fn default_struct_bins() -> usize {
    50
}

fn default_struct_gamma() -> f64 {
    1.0
}

fn default_struct_weight() -> f64 {
    1.0
}

fn default_init_char_mutation_rate() -> f64 {
    crate::genomes::sda::DEFAULT_INIT_CHAR_MUTATION_RATE
}

fn default_transition_vs_response_rate() -> f64 {
    crate::genomes::sda::DEFAULT_TRANSITION_VS_RESPONSE_RATE
}

fn default_elite_count() -> usize {
    1
}

fn default_max_mutations() -> usize {
    1
}

// Both need an explicit function: `#[serde(default)]` yields 0 on a `usize`,
// and zero retries means no epidemic is ever run.

fn default_min_epidemic_length() -> usize {
    3
}

fn default_max_epidemic_retries() -> usize {
    5
}

/// Failure while loading a [`Config`].
///
/// Not mirrored for Python: [`crate::py_config`]'s `config_error_to_py`
/// translates a value of this type into a `PyErr` instead, remapping
/// `Validation`'s `field` through `python_attribute_path`.
#[derive(Debug)]
pub enum ConfigError {
    /// The config file could not be read.
    Io(std::io::Error),
    /// The config text was not valid TOML for a [`Config`].
    Toml(toml::de::Error),
    /// The config parsed, but broke one of [`Config::validate`]'s constraints.
    ///
    /// The field and the constraint are kept apart rather than pre-formatted:
    /// the Python front end builds its own message from them.
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
            ConfigError::Validation { .. } => None,
        }
    }
}

/// Build a [`ConfigError::Validation`] for `field`.
fn invalid(field: &'static str, constraint: impl Into<String>) -> ConfigError {
    ConfigError::Validation {
        field,
        constraint: constraint.into(),
    }
}

/// Reject the two `[fitness]` keys serde discards in silence, reading the raw
/// text: a leftover `seed`, and a `target_profile` under the wrong objective.
///
/// **Serde cannot do either, so do not replace this with
/// `deny_unknown_fields`.** [`SirParams`] is `#[serde(flatten)]`ed into
/// [`FitnessConfig`], and a flattened field deserializes through a buffered
/// content map, so that attribute never fires and an unrecognized key is
/// dropped without a word. Both keys are silent wrong answers rather than
/// errors: a stale `seed` runs under a seeding model its author does not
/// expect, and a `target_profile` left behind after switching objective looks
/// like curve-matching while the run maximizes spread.
///
/// **Two keys by name, deliberately, not a general unknown-key sweep** —
/// pinned by `an_unknown_fitness_key_outside_the_two_named_ones_is_still_ignored`.
///
/// TOML only: the Python front end has no text to inspect, and it cannot
/// express the contradiction in the first place.
fn reject_stray_fitness_keys(text: &str) -> Result<(), ConfigError> {
    // Parsed loosely, as plain TOML values, so this sees keys the `Config`
    // schema throws away. Text that isn't valid TOML at all is left for the
    // real parse to report properly.
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
             everything derives from it, so remove `seed` from `[fitness]`",
        ));
    }

    // A missing, misspelled or non-string `type` falls through to the real
    // parse, which names that problem far better than a message about the
    // profile would.
    let profile_supplied = fitness.get("target_profile").is_some();

    if profile_supplied
        && let Some(objective) = fitness.get("type").and_then(|value| value.as_str())
        && objective != "epi_prof_match"
    {
        return Err(invalid(
            "target_profile",
            format!(
                "belongs to `epi_prof_match` alone, but the objective here is \
                 `{objective}`, which never reads a profile. Either set \
                 `type = \"epi_prof_match\"` or remove `target_profile`"
            ),
        ));
    }
    Ok(())
}

impl Config {
    /// Parse a [`Config`] from TOML text, **without** running
    /// [`Config::validate`], which is kept separate so a test can build a
    /// config that deliberately breaks one constraint.
    ///
    /// Rejects a stray `[fitness] seed` and a misplaced `target_profile` here
    /// rather than there, because neither key survives deserialization.
    pub fn from_toml_str(text: &str) -> Result<Self, ConfigError> {
        reject_stray_fitness_keys(text)?;
        toml::from_str(text).map_err(ConfigError::Toml)
    }

    /// Check every constraint on a parsed config.
    ///
    /// Runs before anything is built, so everything downstream may assume a
    /// valid config. It does not check the base graph, which belongs to
    /// `set_base_graph`.
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.validate_top_level()?;
        self.validate_scope()?;
        self.validate_evolution_and_selection()?;
        self.validate_crossover()?;
        self.validate_genome()?;
        self.validate_fitness()?;
        Ok(())
    }

    /// Constraints on the fields that apply to every run.
    fn validate_top_level(&self) -> Result<(), ConfigError> {
        // A cap of 0 clamps every edge to zero weight under any genome, and
        // the run then looks like a broken fitness function rather than a bad
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
    /// together.
    fn validate_evolution_and_selection(&self) -> Result<(), ConfigError> {
        // No scheme-by-strategy rejection to make: every scheme answers one
        // question over whatever slice it is handed.
        match self.selection {
            SelectionConfig::Best => {}
            SelectionConfig::Tournament { tournament_size } => {
                // No upper bound: a tournament samples the scope *with*
                // replacement, so one larger than the scope just draws some
                // individuals twice.
                if tournament_size == 0 {
                    return Err(invalid(
                        "tournament_size",
                        "must be at least 1: a tournament of nobody has no winner",
                    ));
                }
            }
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
                // Steady-state overwrites members of the scope it bred from,
                // so the scope must hold the parents and the replaced without
                // overlap. Generational displaces nobody, so this cannot be a
                // blanket check.
                let floor = MIN_SCOPE_SIZE;
                if let ScopeConfig::RandomSubset { size } = self.scope
                    && size < floor
                {
                    return Err(invalid(
                        "size",
                        format!(
                            "must be at least {floor} for the steady-state evolver: \
                             {} parents and the {} individuals they replace must be \
                             distinct",
                            PARENTS_PER_EVENT, REPLACED_PER_EVENT,
                        ),
                    ));
                }
            } // ADD A STRATEGY STEP 3 — the constraint arm for your variant.
              // Optional: a strategy with nothing to constrain adds no arm.
              //
              //     EvolutionConfig::MyStrategy { num_my_events, .. } => {
              //         if *num_my_events == 0 {
              //             return Err(invalid("num_my_events", "must be at least 1"));
              //         }
              //     }
        }
        Ok(())
    }

    /// Constraints on the scope, which are its own and not any scheme's.
    fn validate_scope(&self) -> Result<(), ConfigError> {
        match self.scope {
            ScopeConfig::Global => {}
            ScopeConfig::RandomSubset { size } => {
                if size == 0 {
                    return Err(invalid(
                        "size",
                        "must be at least 1: a scope of nobody has no parents to draw",
                    ));
                }
                if size > self.population_size {
                    return Err(invalid(
                        "size",
                        format!(
                            "must be at most population_size ({}): a scope of distinct \
                             individuals cannot be drawn from fewer",
                            self.population_size
                        ),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Pair the recombination operator with the selected representation.
    ///
    /// **Nothing to reject today.** The `match` is written out in full anyway:
    /// a second operator stops it compiling, which forces whoever adds one to
    /// state which representations can honour it rather than finding out
    /// during a run.
    fn validate_crossover(&self) -> Result<(), ConfigError> {
        match (&self.crossover, &self.genome) {
            (CrossoverConfig::TwoPoint, GenomeConfig::EdgeEdit(_)) => Ok(()),
            (CrossoverConfig::TwoPoint, GenomeConfig::Sda(_)) => Ok(()),
            // ADD A CROSSOVER STEP 3 — a pair per representation your operator
            // can honour, `Err(invalid(..))` for any it cannot.
            //
            //     (CrossoverConfig::MyCrossover { .. }, GenomeConfig::Sda(_)) => Ok(()),
        }
    }

    /// Constraints on the genome and its dimensions.
    fn validate_genome(&self) -> Result<(), ConfigError> {
        match &self.genome {
            GenomeConfig::EdgeEdit(edge_edit) => {
                if let Err(constraint) = edge_edit.operation_weights.validate() {
                    return Err(invalid("operation_weights", constraint));
                }
            }
            GenomeConfig::Sda(sda) => {
                if sda.init_state >= sda.num_states {
                    return Err(invalid(
                        "init_state",
                        format!(
                            "must be less than num_states ({}); SdaGenome::run \
                             indexes its response table with it",
                            sda.num_states
                        ),
                    ));
                }

                // Handed straight to `random_bool`, which panics outside
                // 0..=1.
                for (field, rate) in [
                    ("init_char_mutation_rate", sda.init_char_mutation_rate),
                    (
                        "transition_vs_response_rate",
                        sda.transition_vs_response_rate,
                    ),
                ] {
                    if !(0.0..=1.0).contains(&rate) {
                        return Err(invalid(field, "must be between 0.0 and 1.0"));
                    }
                }
            } // ADD A GENOME STEP 4 — the validation arm for your variant.
              // Check anything that would otherwise panic during expression,
              // and report it with `invalid` rather than asserting: a panic
              // here crosses the FFI as an opaque `PanicException`. A field
              // named in an `invalid` call also needs an attribute path in
              // `py_config`'s `python_attribute_path`.
              //
              //     GenomeConfig::MyGenome(mine) => {
              //         if mine.some_dimension == 0 {
              //             return Err(invalid("some_dimension", "must be at least 1"));
              //         }
              //     }
        }
        Ok(())
    }

    /// The `struct_match` half of [`Config::validate_fitness`].
    ///
    /// Touches no filesystem, so `reference_folder` is not opened here — an
    /// unreadable folder, an empty one, or files that do not parse are all
    /// reported when the objective is built.
    fn validate_struct_match(&self) -> Result<(), ConfigError> {
        let FitnessConfig::StructMatch {
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
        } = &self.fitness
        else {
            return Ok(());
        };

        // A top-level field, not one of this objective's own: the statistics
        // count neighbours rather than summing weights, so a multigraph would
        // be scored as though every parallel edge were one. Rejected rather
        // than silently flattened.
        if self.max_edge_multiplicity != 1 {
            return Err(invalid(
                "max_edge_multiplicity",
                format!(
                    "must be 1 for struct_match, which is unweighted by construction, \
                     but is {}",
                    self.max_edge_multiplicity
                ),
            ));
        }

        if reference_folder.trim().is_empty() {
            return Err(invalid(
                "reference_folder",
                "must name a folder of reference graphs",
            ));
        }

        // A zero-bin family divides by a zero bin width.
        for (field, bins) in [
            ("degree_bins", *degree_bins),
            ("clustering_bins", *clustering_bins),
            ("spectral_bins", *spectral_bins),
        ] {
            if bins == 0 {
                return Err(invalid(field, "must be at least 1"));
            }
        }

        // Every gamma and weight reaches `evaluate` as a multiplier, so a
        // non-finite one makes every score non-finite — and a NaN fitness
        // aborts the run rather than scoring a bad candidate.
        for (field, gamma) in [
            ("degree_gamma", *degree_gamma),
            ("clustering_gamma", *clustering_gamma),
            ("spectral_gamma", *spectral_gamma),
        ] {
            if !gamma.is_finite() || gamma <= 0.0 {
                return Err(invalid(
                    field,
                    "must be finite and greater than zero; the RBF kernel divides by it",
                ));
            }
        }

        let mut weight_total = 0.0;
        for (field, weight) in [
            ("degree_weight", *degree_weight),
            ("clustering_weight", *clustering_weight),
            ("spectral_weight", *spectral_weight),
        ] {
            if !weight.is_finite() || weight < 0.0 {
                return Err(invalid(field, "must be finite and non-negative"));
            }
            weight_total += weight;
        }

        // All zero scores every candidate identically: selection stops
        // discriminating and the run searches nothing while looking healthy.
        if weight_total == 0.0 {
            return Err(invalid(
                "degree_weight",
                "at least one of degree_weight, clustering_weight and spectral_weight \
                 must be greater than zero, or every graph scores the same",
            ));
        }

        if !density_weight.is_finite() || *density_weight < 0.0 {
            return Err(invalid(
                "density_weight",
                "must be finite and non-negative; 0 switches the penalty off",
            ));
        }

        Ok(())
    }

    /// Constraints on the epidemic sampling parameters.
    fn validate_fitness(&self) -> Result<(), ConfigError> {
        // `python` carries no SIR block: its parameters belong to the
        // callable, and are supplied when it is registered.
        let sir = match &self.fitness {
            FitnessConfig::EpiSpread { sir } => sir,
            FitnessConfig::EpiLength { sir } => sir,
            FitnessConfig::EpiProfMatch { sir, .. } => sir,
            FitnessConfig::StructMatch { .. } => return self.validate_struct_match(),
            // ADD AN OBJECTIVE STEP 2 — the validation arm, if any parameter
            // is worth constraining; return `Ok(())` if none is. Report a
            // failure with `invalid` rather than asserting: a panic here
            // crosses the FFI as an opaque `PanicException`. It may not touch
            // the filesystem, and it may constrain fields outside `[fitness]`,
            // the way `struct_match` requires a top-level
            // `max_edge_multiplicity` of 1.
            //
            //     FitnessConfig::MyObjective { threshold } => {
            //         if !threshold.is_finite() {
            //             return Err(invalid("threshold", "must be finite"));
            //         }
            //         return Ok(());
            //     }
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

        // 1 is legal and means "never re-roll": every epidemic has length >= 1,
        // so 1 is how a user opts out of the re-roll's deliberate bias.
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

        // These checks are why the profile is an inline config value rather
        // than a path: a file could not be checked without opening it.
        if let FitnessConfig::EpiProfMatch { target_profile, .. } = &self.fitness {
            if target_profile.is_empty() {
                return Err(invalid(
                    "target_profile",
                    "must have at least one element; there is nothing to score against otherwise",
                ));
            }

            // RMSE against a NaN or an infinity is NaN or infinite for *every*
            // individual, so the whole population scores identically and
            // selection stops discriminating — a run that looks like it is
            // working and is searching nothing.
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

[scope]
type = "global"

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
            GenomeConfig::EdgeEdit(edge_edit) => edge_edit.operation_weights,
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
    fn a_misspelled_genome_key_is_an_error_rather_than_a_silent_default() {
        // Same guarantee one level up from the operation-weight test below: a
        // key under `[genome]` that nothing reads means the run is not the one
        // the writer configured.
        let error = Config::from_toml_str(&config_text("gene_lenght = 128"))
            .expect_err("an unknown genome key should not parse");

        assert!(
            error.to_string().contains("gene_lenght"),
            "the error should name the offending key, got: {error}"
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
            GenomeConfig::Sda(sda) => assert_eq!(sda.init_state, 0),
            other => panic!("expected an sda genome, got {other:?}"),
        }
    }

    #[test]
    fn an_explicit_sda_init_state_round_trips() {
        match Config::from_toml_str(&sda_config_text("\ninit_state = 7"))
            .expect("sda config should parse")
            .genome
        {
            GenomeConfig::Sda(sda) => {
                assert_eq!(sda.init_state, 7);
                assert_eq!(sda.num_states, 12);
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
        // `toml` widens an integer element into the `f64` the field asks for,
        // so a hand-written `[0, 2, 7]` is accepted rather than rejected as a
        // type error. Worth a test because the opposite is the obvious
        // guess, and the natural way to write a profile by hand is without
        // decimal points.
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
        // Serde cannot catch this: `#[serde(flatten)]` buffers the field's
        // content, so `deny_unknown_fields` on `SirParams` never fires. The
        // key is caught by reading the raw text instead, because the
        // migration hazard is silent: an old config keeping `seed = 42`
        // would otherwise get a different seeding model with no error.
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
        // The profile is required by `epi_prof_match` and rejected as a
        // contradiction if supplied for any other objective. Caught in the
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

    /// The behaviour-preservation guarantee `[crossover]` was added under: a
    /// config written before the operator was selectable names none, and has
    /// to keep running exactly as it did.
    ///
    /// Asserted by parsing rather than by reading the `#[serde(default)]`
    /// attribute, because the attribute is only half of it — a `Default` impl
    /// pointing at a different variant would satisfy the attribute and break
    /// every existing config.
    #[test]
    fn a_config_naming_no_operator_gets_two_point() {
        // The fixture has no `[crossover]` table at all, which is exactly the
        // shape of every config written before this section existed.
        assert!(!config_text("").contains("[crossover]"));

        let config = valid_config();
        match config.crossover {
            CrossoverConfig::TwoPoint => {}
        }
        config
            .validate()
            .expect("a config naming no operator should still validate");
    }

    /// Naming the default explicitly parses to the same thing as leaving it
    /// out — so a user who writes the block to document their intent is not
    /// quietly selecting something else.
    #[test]
    fn naming_two_point_explicitly_matches_leaving_the_block_out() {
        let text = format!("{}\n[crossover]\ntype = \"two_point\"\n", config_text(""));
        let config = Config::from_toml_str(&text).expect("an explicit operator should parse");

        match config.crossover {
            CrossoverConfig::TwoPoint => {}
        }
        config.validate().expect("and should validate");
    }

    /// The same behaviour-preservation guarantee as `[crossover]`'s, but for
    /// the per-genome mutation operator: a config naming neither representation's
    /// mutation keeps running the one it always ran.
    #[test]
    fn a_config_naming_no_mutation_operator_gets_the_representations_default() {
        let edge_edit = valid_config();
        match &edge_edit.genome {
            GenomeConfig::EdgeEdit(cfg) => {
                assert_eq!(cfg.mutation, EdgeEditMutationConfig::RerollGene);
            }
            other => panic!("expected the fixture's edge_edit genome, got {other:?}"),
        }

        let sda =
            Config::from_toml_str(&sda_config_text("")).expect("the sda fixture should parse");
        match &sda.genome {
            GenomeConfig::Sda(cfg) => {
                assert_eq!(cfg.mutation, SdaMutationConfig::RedrawOne);
            }
            other => panic!("expected the fixture's sda genome, got {other:?}"),
        }
    }

    /// An operator GET does not ship is refused by name rather than ignored.
    ///
    /// The section is a tagged enum, so this is serde's rejection, not a check
    /// of ours — pinned because the alternative failure is silent: an unknown
    /// `type` that parsed would run the default operator while the file says
    /// otherwise.
    #[test]
    fn an_unknown_operator_is_rejected() {
        let text = format!("{}\n[crossover]\ntype = \"uniform\"\n", config_text(""));
        assert!(
            Config::from_toml_str(&text).is_err(),
            "an operator GET does not ship should not parse",
        );
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
    fn a_population_smaller_than_the_scope_is_rejected() {
        // The scope draws *distinct* individuals, so it cannot exceed the
        // population. It is the scope's `size` that is at fault here, not the
        // population, and the error says so.
        let mut config = valid_config();
        config.scope = ScopeConfig::RandomSubset { size: 8 };
        config.population_size = 3;

        assert_eq!(validation_field(&config), "size");
    }

    #[test]
    fn a_tournament_larger_than_the_population_is_fine() {
        // It samples the scope *with* replacement, so an oversized tournament
        // just draws some individuals twice. This used to be rejected, because
        // one number was doing both jobs.
        let mut config = valid_config();
        config.selection = SelectionConfig::Tournament {
            tournament_size: 500,
        };
        config.population_size = 10;

        config
            .validate()
            .expect("a tournament may exceed the population it samples");
    }

    #[test]
    fn a_steady_state_run_naming_no_replacement_gets_worst() {
        // Absent means self-elitist, which is the guarantee a run that never
        // thought about it should keep.
        let text = config_text("").replace(
            "type            = \"generational\"\nnum_generations = 500",
            "type              = \"steady_state\"\nnum_mating_events = 1000",
        );
        let config = Config::from_toml_str(&text).expect("steady-state config should parse");

        match config.evolution {
            EvolutionConfig::SteadyState { replacement, .. } => match replacement {
                ReplacementConfig::Worst => {}
                other => panic!("expected the worst default, got {other:?}"),
            },
            other => panic!("expected steady_state, got {other:?}"),
        }
    }

    #[test]
    fn the_scope_size_and_the_tournament_size_are_independent() {
        // The regression this whole split exists to prevent: one number sizing
        // both the scope and the tournament. Steady-state needs a scope of at
        // least four, and must not inherit that floor from a tournament it does
        // not draw.
        let mut config = valid_config();
        config.evolution = EvolutionConfig::SteadyState {
            num_mating_events: 1000,
            replacement: ReplacementConfig::Worst,
        };
        config.scope = ScopeConfig::RandomSubset { size: 6 };
        config.selection = SelectionConfig::Best;

        config
            .validate()
            .expect("a scope of six with no tournament at all is valid");
    }

    #[test]
    fn a_scope_of_nobody_is_rejected() {
        // The `size > population_size` half of this rule has its own test; the
        // zero end had none. Steady-state's floor of four hides it there, so
        // the case that actually reaches this branch is a subset scope under
        // generational, which has no floor of its own.
        let mut config = valid_config();
        config.evolution = EvolutionConfig::Generational {
            num_generations: 500,
            elite_count: 1,
        };
        config.scope = ScopeConfig::RandomSubset { size: 0 };

        assert_eq!(validation_field(&config), "size");
    }

    #[test]
    fn a_tournament_of_nobody_is_rejected() {
        // The other settings that retire the search each have their own test;
        // this one had none. Left to run, `Selection::pick` asserts on it deep
        // inside the first breeding event, so the user gets a panic partway
        // through a run instead of a named field before it starts.
        let mut config = valid_config();
        config.selection = SelectionConfig::Tournament { tournament_size: 0 };

        assert_eq!(validation_field(&config), "tournament_size");
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
    fn the_scope_floor_of_four_applies_to_steady_state_only() {
        // Four is two parents plus the two they replace, all distinct. It binds
        // the scope, not any scheme, so a scheme's own size is irrelevant here.
        let mut config = valid_config();
        config.scope = ScopeConfig::RandomSubset { size: 3 };

        // Generational displaces nobody, so it has no such floor.
        config
            .validate()
            .expect("generational imposes no scope floor");

        config.evolution = EvolutionConfig::SteadyState {
            num_mating_events: 1000,
            replacement: ReplacementConfig::Worst,
        };
        assert_eq!(validation_field(&config), "size");
    }

    #[test]
    fn an_init_state_outside_the_state_count_is_rejected() {
        let mut config = valid_config();
        config.genome = sda_genome(12, 12);

        assert_eq!(validation_field(&config), "init_state");
    }

    /// An sda genome block with valid mutation rates, so a test about some
    /// other field does not have to spell them out.
    fn sda_genome(num_states: usize, init_state: usize) -> GenomeConfig {
        GenomeConfig::Sda(SdaGenomeConfig {
            num_states,
            max_resp_len: 4,
            init_state,
            init_char_mutation_rate: default_init_char_mutation_rate(),
            transition_vs_response_rate: default_transition_vs_response_rate(),
            mutation: SdaMutationConfig::default(),
        })
    }

    #[test]
    fn a_mutation_rate_outside_zero_to_one_is_rejected() {
        for field in ["init_char_mutation_rate", "transition_vs_response_rate"] {
            for value in ["-0.1", "1.5", "nan"] {
                let mut config = valid_config();
                config.genome = sda_genome(12, 0);
                if let GenomeConfig::Sda(sda) = &mut config.genome {
                    let parsed: f64 = value.parse().expect("test literal parses");
                    if field == "init_char_mutation_rate" {
                        sda.init_char_mutation_rate = parsed;
                    } else {
                        sda.transition_vs_response_rate = parsed;
                    }
                }

                assert_eq!(
                    validation_field(&config),
                    field,
                    "{field} = {value} should be rejected"
                );
            }
        }
    }

    #[test]
    fn omitted_mutation_rates_fall_back_to_sda_s_own_defaults() {
        let config = Config::from_toml_str(&sda_config_text("")).expect("sda config parses");

        match config.genome {
            GenomeConfig::Sda(sda) => {
                assert_eq!(
                    sda.init_char_mutation_rate,
                    crate::genomes::sda::DEFAULT_INIT_CHAR_MUTATION_RATE
                );
                assert_eq!(
                    sda.transition_vs_response_rate,
                    crate::genomes::sda::DEFAULT_TRANSITION_VS_RESPONSE_RATE
                );
            }
            other => panic!("expected sda, got {other:?}"),
        }
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

        // 1 means "never re-roll" — every epidemic already has length >= 1,
        // so 1 is the only way to opt out of the re-roll's deliberate bias.
        // Pinned so nobody "corrects" the floor to 2.
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

    /// The sequence both front ends run: read the file, parse it, validate it.
    /// Spelled out here rather than behind a `Config::from_path` helper —
    /// there was one, and nothing could use it, because every real caller also
    /// needs the raw text as the run's provenance record and the helper
    /// discarded it.
    fn load_and_validate(path: &str) -> Result<Config, ConfigError> {
        let text = std::fs::read_to_string(path).map_err(ConfigError::Io)?;
        let config = Config::from_toml_str(&text)?;
        config.validate()?;
        Ok(config)
    }

    #[test]
    fn the_example_config_loads_and_validates_from_disk() {
        load_and_validate(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../config.example.toml"
        ))
        .expect("the shipped example config should load and validate");
    }

    #[test]
    fn a_missing_config_file_is_an_io_error_rather_than_a_panic() {
        match load_and_validate("no/such/directory/config.toml") {
            Err(ConfigError::Io(_)) => {}
            other => panic!("expected an Io error, got {other:?}"),
        }
    }

    #[test]
    fn a_config_file_that_breaks_a_constraint_fails_to_load() {
        let dir = std::env::temp_dir().join("get_config_validate_test");
        std::fs::create_dir_all(&dir).expect("the temp dir should be creatable");
        let path = dir.join("bad_config.toml");
        std::fs::write(&path, format!("max_mutations = 0\n{}", config_text("")))
            .expect("the temp config should be writable");

        let error = load_and_validate(path.to_str().unwrap())
            .expect_err("an invalid config should not load");
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

    /// The example's alternative `[genome]` blocks are commented out, so
    /// `the_example_config_parses` never reads them and a typo in one ships
    /// silently — a wrong key name, or explanatory prose left inside the region
    /// a user uncomments. This uncomments the sda block and parses it.
    #[test]
    fn the_examples_commented_sda_genome_block_parses() {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../config.example.toml"
        ))
        .expect("config.example.toml should be readable");

        // The block is the run of comment lines starting at `# [genome]`; the
        // header itself is dropped so what is left is a bare `type = "sda"`
        // table that deserializes straight into `GenomeConfig`.
        let mut block = String::new();
        let mut inside = false;
        for line in text.lines() {
            let stripped = line.strip_prefix('#').map(|rest| rest.trim_start());
            match stripped {
                Some("[genome]") => inside = true,
                // A second table header ends the block. Setup B is commented
                // out as a whole, `#` and all, so its blank separator lines are
                // still comments and the run does not end at one -- without
                // this the collector swallows [fitness] too.
                Some(content) if inside && content.starts_with('[') => break,
                Some(content) if inside => {
                    block.push_str(content);
                    block.push('\n');
                }
                // Any non-comment line ends the block, including the blank one
                // that follows it.
                _ if inside => break,
                _ => {}
            }
        }
        assert!(
            block.contains("type"),
            "no commented [genome] block found in the example"
        );

        let genome: GenomeConfig =
            toml::from_str(&block).expect("the commented sda block should parse");
        match genome {
            GenomeConfig::Sda(sda) => {
                assert_eq!(sda.num_states, 12);
                assert_eq!(
                    sda.init_char_mutation_rate,
                    crate::genomes::sda::DEFAULT_INIT_CHAR_MUTATION_RATE
                );
                assert_eq!(
                    sda.transition_vs_response_rate,
                    crate::genomes::sda::DEFAULT_TRANSITION_VS_RESPONSE_RATE
                );
            }
            other => panic!("expected the sda genome, got {other:?}"),
        }
    }

    /// A `struct_match` config, with `overrides` folded into `[fitness]`.
    fn struct_match_config(overrides: &str) -> Config {
        let text = format!(
            r#"
population_size = 20
network_size    = 30
crossover_rate  = 0.9
mutation_rate   = 0.2
max_edge_multiplicity = 1

[evolution]
type            = "generational"
num_generations = 5

[scope]
type = "global"

[selection]
type            = "tournament"
tournament_size = 3

[genome]
type        = "edge_edit"
gene_length = 32

[fitness]
type = "struct_match"
reference_folder = "reference"
{overrides}
"#
        );
        Config::from_toml_str(&text).expect("the struct_match fixture should parse")
    }

    #[test]
    fn a_struct_match_config_validates_and_names_itself() {
        let config = struct_match_config("");

        config
            .validate()
            .expect("the struct_match fixture should be valid");
        assert_eq!(config.fitness.type_name(), "struct_match");
    }

    #[test]
    fn struct_match_defaults_every_field_but_the_folder() {
        // The example block a user copies is one line plus the type, so the
        // defaults are what most runs actually use.
        let config = struct_match_config("");

        match &config.fitness {
            FitnessConfig::StructMatch {
                reference_folder,
                degree_bins,
                spectral_gamma,
                density_weight,
                ..
            } => {
                assert_eq!(reference_folder, "reference");
                assert_eq!(*degree_bins, 50);
                assert_eq!(*spectral_gamma, 1.0);
                assert_eq!(*density_weight, 1.0);
            }
            other => panic!("expected struct_match, got {other:?}"),
        }
    }

    #[test]
    fn struct_match_requires_an_unweighted_graph() {
        // The three statistics count neighbours rather than summing weights,
        // so a multigraph would be scored as though parallel edges were one.
        let mut config = struct_match_config("");
        config.max_edge_multiplicity = 3;

        assert_eq!(validation_field(&config), "max_edge_multiplicity");
    }

    #[test]
    fn struct_match_rejects_the_settings_that_would_retire_the_search() {
        // Each of these leaves a run that produces numbers and searches
        // nothing, which is the failure this objective keeps having to guard.
        let cases = [
            ("degree_bins = 0", "degree_bins"),
            ("clustering_bins = 0", "clustering_bins"),
            ("spectral_bins = 0", "spectral_bins"),
            ("degree_gamma = 0.0", "degree_gamma"),
            ("clustering_gamma = -1.0", "clustering_gamma"),
            ("spectral_gamma = nan", "spectral_gamma"),
            ("degree_weight = -0.5", "degree_weight"),
            ("density_weight = -1.0", "density_weight"),
        ];

        for (override_text, expected_field) in cases {
            let config = struct_match_config(override_text);
            assert_eq!(
                validation_field(&config),
                expected_field,
                "`{override_text}` should be rejected against {expected_field}"
            );
        }
    }

    #[test]
    fn struct_match_rejects_an_all_zero_set_of_weights() {
        // Every candidate then scores identically: selection stops
        // discriminating while the run still logs generations.
        let config = struct_match_config(
            "degree_weight = 0.0\nclustering_weight = 0.0\nspectral_weight = 0.0",
        );

        assert_eq!(validation_field(&config), "degree_weight");
    }

    #[test]
    fn struct_match_accepts_switching_one_family_off() {
        // Zeroing a single weight is legitimate — it is how a user drops a
        // family whose reference set cannot support it.
        struct_match_config("clustering_weight = 0.0")
            .validate()
            .expect("one zero weight among three is a valid choice");
    }

    #[test]
    fn struct_match_rejects_an_empty_reference_folder_name() {
        // The *name* is checked here; the folder's contents are not, because
        // `validate` does no I/O. Anything needing the filesystem — a missing
        // folder, an empty one, files that do not parse — belongs to dispatch.
        let text = r#"
population_size = 20
network_size    = 30
crossover_rate  = 0.9
mutation_rate   = 0.2
max_edge_multiplicity = 1

[evolution]
type            = "generational"
num_generations = 5

[scope]
type = "global"

[selection]
type            = "tournament"
tournament_size = 3

[genome]
type        = "edge_edit"
gene_length = 32

[fitness]
type = "struct_match"
reference_folder = "   "
"#;
        let config = Config::from_toml_str(text).expect("a blank folder name still parses");

        assert_eq!(validation_field(&config), "reference_folder");
    }
}
