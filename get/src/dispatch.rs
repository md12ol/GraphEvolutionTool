//! Config → concrete types: the layer spec §1 and §8 call "dispatch".
//!
//! Runtime choice is resolved here and nowhere else. The objective is erased to
//! `Box<dyn Fitness>` first, then strategy × genome is matched — which is what
//! keeps dispatch at 4 arms instead of 16 (§8). The starting population is built
//! here too, because `Genome` has no uniform random constructor and a generic
//! evolver therefore cannot mint one (§6).
//!
//! # Why this is its own module, and specifically not `evolver/common.rs`
//!
//! This layer is the **only** place that knows both sides. It reads
//! [`crate::config`] and returns `PyResult`, so it depends on the config schema
//! *and* on pyo3; the engine below it — `evolver/`, `genomes/`, `sir`, `graph` —
//! deliberately depends on neither, and `sir.rs` says so about its own params
//! type. Putting these functions in `evolver/common.rs` would drag both
//! dependencies into the engine core, and `common.rs` is genome-*agnostic*
//! generics over `G: Genome` where these name `EdgeEditGenome` and `SdaGenome`
//! concretely. Keeping it separate is what lets the engine be tested without a
//! config or a Python interpreter. See `decisions.md` 2026-08-11.
//!
//! `lib.rs` keeps the `#[pyclass]` surface — the constructors,
//! `set_fitness_function`, and `run`, which calls into here.

use std::sync::Arc;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rayon::ThreadPoolBuilder;
use rayon::prelude::*;

use crate::GraphEvolver;
use crate::config::{
    self, Config, CrossoverConfig, EdgeEditGenomeConfig, EdgeEditMutationConfig, EvolutionConfig,
    FitnessConfig, GenomeConfig, ReplacementConfig, ScopeConfig, SdaGenomeConfig,
    SdaMutationConfig, SelectionConfig,
};
use crate::evolver::common::{Crossover, Selection};
use crate::evolver::replacement::Replacement;
use crate::evolver::scope::Scope;
use crate::evolver::{
    EvolutionOutcome, Evolver, GenerationStats, GenerationalContext, GenerationalEvolver,
    SharedEvolutionContext, SteadyStateContext, SteadyStateEvolver,
};
use crate::fitness::{EpiLength, EpiProfMatch, EpiSpread, Fitness, StructMatch};
use crate::genomes::edge_edit::IDENTITY_GENE;
use crate::genomes::{
    EdgeEditContext, EdgeEditGenome, EdgeEditMutation, EdgeEditOperators, Genome, SdaContext,
    SdaDimensions, SdaGenome, SdaMutation,
};
use crate::graph::Graph;
use crate::graph_io;
use crate::sir::{self, SirSampleParams};
use crate::stats::{HistogramAxes, PerFamily, ReferenceStatistics};

/// One run's result, with the genome type erased.
///
/// `#[pyclass]` cannot carry a type parameter but [`EvolutionOutcome<G>`] does,
/// so every dispatch arm erases `G` before returning (§8). What survives is what
/// a caller can use without knowing the representation.
///
/// **Everything in here has already been converted out of engine orientation.**
/// That is the difference between this type and [`EvolutionOutcome`], which is
/// engine-oriented throughout and says so in its field names. This is the far
/// side of the boundary [`erase`] draws, and no field here needs converting
/// again.
pub(crate) struct ErasedOutcome {
    /// Best fitness in the objective's **own units and sign**, not engine
    /// orientation.
    ///
    /// The engine compares in oriented values throughout, so a maximizing
    /// objective's numbers are negative in there (§5.1). Converting once here is
    /// what keeps that an engine-internal detail: `Direction::orient` is its own
    /// inverse, so the same call that oriented the value on the way in undoes it
    /// on the way out.
    pub best_fitness: f64,
    /// The best individual's expressed network as `(u, v, multiplicity)`.
    pub best_edges: Vec<(usize, usize, u32)>,
    /// How many nodes that network has.
    ///
    /// Carried rather than left to be counted from `best_edges`, because it
    /// cannot be: a node with no edges appears nowhere in an edge list, and an
    /// evolved graph acquires isolated nodes routinely. Every writer states it,
    /// so a run's output loads back through `graph_io` unedited.
    pub num_nodes: usize,
    /// The winning genome's `Genome::print()` string.
    ///
    /// The entry point is not generic over the genome, so this is the only way
    /// it can record *which* individual won.
    pub best_genome_repr: String,
    /// The convergence log, one row per logged iteration.
    ///
    /// Reuses the engine's [`GenerationStats`] row, but the two fitness columns
    /// have been oriented by [`erase`] — see this struct's own note above.
    pub history: Vec<GenerationStats>,
}

/// The node index a reference file may not exceed, when the run's own
/// `network_size` is smaller.
///
/// `load_edge_folder` takes one node count for a whole folder and uses it to
/// reject out-of-range indices. Reference graphs come from real data and differ
/// in size, so there is no single right value; this is a sanity bound that
/// still catches a file indexed the wrong way (a global TUDataset index, say)
/// while admitting any plausible reference graph.
pub(crate) const MAX_REFERENCE_NODES: usize = 100_000;

/// The parts of dispatch that need the evolver itself, not just its config.
///
/// `objective` reads the registered callable as well as `[fitness]`, and
/// `python_fitness` is entirely about that registration — so both take `&self`.
/// The three pure config→engine mappings below are free functions instead, which
/// makes them testable from a bare [`Config`] with no evolver to construct.
impl GraphEvolver {
    /// Build the objective for one run, erased to `Box<dyn Fitness>`.
    ///
    /// **Step 1 of the two-step dispatch** (§1, §8, GitHub #26). The objective
    /// is erased *before* any strategy or genome is chosen, which is what keeps
    /// dispatch at 2 strategies × 2 genomes = 4 arms instead of 16: nothing
    /// downstream knows which objective it holds. Adding a fifth objective is
    /// one arm here and touches nothing else.
    ///
    /// The asymmetry is not an oversight. `Fitness` erases cleanly — no generic
    /// methods, no `Self` in argument position, and `Send + Sync` through its
    /// supertrait, so rayon is unaffected. `Genome` cannot, for four
    /// independent reasons — `mutate`/`crossover` are generic over the RNG,
    /// `crossover` takes `&mut Self`, `Clone` requires `Sized`, and `Context` is
    /// an associated type differing per representation — so that axis stays a
    /// match.
    ///
    /// **Call this once per run, never once per evolver.** Every SIR objective
    /// owns an `EpidemicScorer` holding a per-run counter, and two replicates
    /// sharing one would let thread scheduling decide which run saw which seed
    /// — reproducibility goes with it (§8.1). Taking `run_seed` by argument
    /// rather than reading a field is what makes that misuse awkward.
    ///
    /// # Errors
    ///
    /// `ValueError` if the config selected Python and no callable was
    /// registered, or if `epi_prof_match`'s target profile is unusable.
    /// `Config::validate` already rejects an empty or non-finite profile, so
    /// that second case is a backstop for a `Config` built in Rust without
    /// going through validation — not a path a Python caller can reach.
    /// # Part of the chain that adds an objective
    ///
    /// This match is the step after adding the `FitnessConfig` variant in
    /// `crate::config`, and it is the one that erases the choice: everything
    /// downstream of here sees one `Box<dyn Fitness>` and not a variant, which
    /// is what keeps a new objective to a single arm rather than one arm per
    /// strategy and genome combination. `crate::fitness`'s module doc has the
    /// whole chain.
    ///
    /// `py` is the GIL token to report a `struct_match` reference folder's
    /// `LoadWarning`s through, when one is held — `None` on the Rust-native
    /// route (`GraphEvolver::run_from_toml`, spec §5.3 route 4), which has no
    /// Python interpreter to raise a `UserWarning` on, so those warnings go to
    /// stderr instead. See `struct_match_reference`.
    pub(crate) fn objective(
        &self,
        run_seed: u64,
        py: Option<Python<'_>>,
    ) -> PyResult<Box<dyn Fitness>> {
        match &self.config.fitness {
            FitnessConfig::EpiSpread { sir } => {
                Ok(Box::new(EpiSpread::new(sir_sample_params(sir), run_seed)))
            }
            FitnessConfig::EpiLength { sir } => {
                Ok(Box::new(EpiLength::new(sir_sample_params(sir), run_seed)))
            }
            FitnessConfig::EpiProfMatch {
                sir,
                target_profile,
            } => {
                // Cloned because the objective owns its target and the config
                // outlives it — a run must not be able to mutate the profile it
                // is being scored against.
                let objective =
                    EpiProfMatch::new(sir_sample_params(sir), run_seed, target_profile.clone())
                        .map_err(PyValueError::new_err)?;
                Ok(Box::new(objective))
            }
            FitnessConfig::StructMatch {
                degree_gamma,
                clustering_gamma,
                spectral_gamma,
                degree_weight,
                clustering_weight,
                spectral_weight,
                density_weight,
                ..
            } => {
                // Shared, not rebuilt: this arm runs once per replicate, and
                // the reduced reference set is immutable. See
                // `GraphEvolver::struct_match_reference`.
                let reference = self.struct_match_reference(py)?;

                let objective = StructMatch::new(
                    reference,
                    PerFamily {
                        degree: *degree_gamma,
                        clustering: *clustering_gamma,
                        spectral: *spectral_gamma,
                    },
                    PerFamily {
                        degree: *degree_weight,
                        clustering: *clustering_weight,
                        spectral: *spectral_weight,
                    },
                    *density_weight,
                )
                .map_err(PyValueError::new_err)?;

                Ok(Box::new(objective))
            }
            // The one arm that is not built from config alone: the callable
            // arrived through a setter, so `python_fitness` owns the "nothing
            // registered" error and this stays one call.
            FitnessConfig::Python => self.python_fitness(),
        }
    }

    /// `struct_match`'s reduced reference set, loaded and reduced at most once.
    ///
    /// # Why the folder is read here rather than in `Config::validate`
    ///
    /// `validate` does no I/O deliberately, so every failure that needs the
    /// filesystem surfaces here instead: a missing or unreadable folder, a
    /// file that does not parse, and an empty reference set — which task A
    /// made a hard error because scoring against nothing returns 0.0, and 0.0
    /// is a *perfect* score in this objective. A mistyped folder would
    /// otherwise make every candidate ideal and the convergence log read like
    /// a solved problem.
    ///
    /// # The degree axis is taken from the reference set
    ///
    /// Clustering and the normalized spectrum have natural bounds; degree does
    /// not, so its axis needs a top. Deriving it from the reference graphs
    /// means it cannot be set too low — which would squash every reference
    /// histogram into the last bin and silently retire the whole family.
    /// A candidate above the top lands in the last bin, which is the honest
    /// reading: more connected than anything in the reference set.
    ///
    /// # Warnings
    ///
    /// Reported the same way `GraphEvolver::load_reference_graphs` reports
    /// them for a base-graph reference set: a repeated edge, a zero-weight
    /// edge, and a file with no edges, each naming the file it came from.
    /// Only the caller whose load wins the `OnceLock` race below reports —
    /// a losing caller's graphs are discarded in favour of the winner's, so
    /// its warnings would describe a reference set nobody is using.
    fn struct_match_reference(&self, py: Option<Python<'_>>) -> PyResult<Arc<ReferenceStatistics>> {
        if let Some(cached) = self.struct_match_reference.get() {
            return Ok(Arc::clone(cached));
        }

        let FitnessConfig::StructMatch {
            reference_folder,
            degree_bins,
            clustering_bins,
            spectral_bins,
            ..
        } = &self.config.fitness
        else {
            return Err(PyValueError::new_err(
                "struct_match_reference is only for a \"struct_match\" objective",
            ));
        };

        // `min_node_index` is whatever a loader already established for this
        // run, and 0 when none has: reference files are the caller's data and
        // share the run's indexing convention.
        let min_node_index = self.min_node_index.unwrap_or(0);

        // The loader wants one node count for the whole folder, and reference
        // graphs differ in size. This is an upper bound that still catches a
        // wild index; each graph's real size comes from `EdgeFile::to_graph`.
        // `load_reference_graphs` computes the same bound, so what a run reads
        // and what a caller can inspect are the same set of files.
        let index_cap = self.config.network_size.max(MAX_REFERENCE_NODES);

        let loaded = graph_io::load_edge_folder(
            std::path::Path::new(reference_folder.as_str()),
            index_cap,
            1,
            min_node_index,
        )
        .map_err(|error| {
            PyValueError::new_err(format!(
                "[fitness] reference_folder {reference_folder:?} could not be read: {error}"
            ))
        })?;

        let mut graphs = Vec::with_capacity(loaded.len());
        for file in &loaded {
            graphs.push(file.to_graph(1));
        }

        let mut max_degree = 0;
        for graph in &graphs {
            for node in 0..graph.num_nodes {
                let degree = graph.degree(node);
                if degree > max_degree {
                    max_degree = degree;
                }
            }
        }

        let axes = HistogramAxes {
            max_degree,
            degree_bins: *degree_bins,
            clustering_bins: *clustering_bins,
            spectral_bins: *spectral_bins,
        };

        let statistics = ReferenceStatistics::from_graphs(&graphs, axes).map_err(|error| {
            PyValueError::new_err(format!(
                "[fitness] reference_folder {reference_folder:?} did not yield a usable \
                 reference set: {error}"
            ))
        })?;

        let shared = Arc::new(statistics);

        // If another caller won the race, take theirs, so every replicate in a
        // run shares exactly one reduced reference set.
        match self.struct_match_reference.set(Arc::clone(&shared)) {
            Ok(()) => {
                for file in &loaded {
                    crate::emit_load_warnings_maybe(py, &file.source, &file.warnings)?;
                }
                Ok(shared)
            }
            Err(_) => {
                Ok(Arc::clone(self.struct_match_reference.get().expect(
                    "set failed only because a value is already present",
                )))
            }
        }
    }

    /// The objective for one run, when the config selected Python.
    ///
    /// This is the seam the dispatch in **#26** calls: it turns the registered
    /// callable into the erased `Box<dyn Fitness>` that §8 hands the evolver,
    /// so the `python` arm of that match is one call rather than a second place
    /// that knows how registration works.
    ///
    /// **A fresh instance per call, not a shared one.** Replicate runs each
    /// need their own objective (§8.1), so the erasing step is re-run per
    /// replicate rather than cloning one box. The Python callable itself is
    /// shared by refcount — see [`PyFitness::clone_ref`] — which is right,
    /// because it is the *scorer* state that must stay per-run and this has
    /// none.
    ///
    /// The other three variants are not built here — they need
    /// `config::SirParams` mapped onto [`crate::sir::SirSampleParams`] plus the
    /// run seed. [`GraphEvolver::objective`] is that match, and this is its
    /// `python` arm.
    ///
    /// # Errors
    ///
    /// `ValueError` if no callable has been registered — the case spec §8 and
    /// issue #19 both call out, since a run that reached scoring with nothing
    /// registered would otherwise panic deep inside the engine. Also if the
    /// config did not select Python, which is a caller mistake rather than a
    /// user one, but is reported rather than asserted so it cannot become a
    /// panic in a release build.
    pub(crate) fn python_fitness(&self) -> PyResult<Box<dyn Fitness>> {
        if !matches!(self.config.fitness, FitnessConfig::Python) {
            return Err(PyValueError::new_err(format!(
                "python_fitness is only for a \"python\" objective, but [fitness] type \
                 is \"{}\"",
                self.config.fitness.type_name(),
            )));
        }

        match self.fitness_function {
            Some(ref registered) => Ok(Box::new(registered.clone_ref())),
            None => Err(PyValueError::new_err(
                "[fitness] type is \"python\" but no fitness function has been \
                 registered; call set_fitness_function(callable, direction) before run()",
            )),
        }
    }
}

// ADD A GENOME STEP 5 — a start builder beside this one and `sda_start`.
//
//     pub(crate) fn my_genome_start<R: Rng + ?Sized>(
//         config: &Config,
//         mine: &MyGenomeConfig,
//         rng: &mut R,
//     ) -> PyResult<(MyContext, Vec<MyGenome>)> {
//         // Validate the dimensions once, here — not per individual.
//         let mut population = Vec::with_capacity(config.population_size);
//         for _ in 0..config.population_size {
//             population.push(MyGenome::random(mine.some_dimension, rng));
//         }
//         let context = MyContext { num_nodes: config.network_size, .. };
//         Ok((context, population))
//     }
//
// Exactly `population_size` individuals, a context `express` can index without
// panicking, and rejection rather than clamping for caller data that disagrees
// with the config. The doc below says why each of those matters.

/// The edge-edit starting population and the context it expresses against.
///
/// # Part of the chain that adds a representation
///
/// This is step 5 of seven, and the one with real obligations rather than
/// wiring: a start builder owes the engine a population of exactly
/// `population_size`, a context `express` can use, and rejection rather than
/// clamping for caller data that disagrees with the config.
/// [`crate::genomes::genome`]'s module doc states all three and has the other
/// six steps; a new representation adds a function beside this one and
/// [`sda_start`].
///
/// **Why the dispatch layer builds the population at all.** `Evolver::new`
/// takes a ready-made `Vec<G>` because `Genome` has no uniform random
/// constructor: this one needs a gene length and an operation mix, the SDA
/// one needs three dimensions and is fallible. A generic evolver cannot call
/// either, so the knowledge lives here, where genome-specific detail already
/// is — and a bad dimension surfaces as a config error at startup instead of
/// an `.expect()` mid-run inside a generic.
///
/// The operation mix is built **once** and shared across the population
/// behind the `Arc`: `EdgeEditOperators::new` compiles the weights into a
/// `WeightedIndex` sampler, and doing that per individual would rebuild the
/// same table `population_size` times.
///
/// `base_graph` is what the edit script is applied to: `Some` seeds the run
/// from a caller-supplied graph — raw data, or a previous run's best edges —
/// and `None` starts from an empty one. It is cloned rather than borrowed
/// because the context owns its graph for the whole run, while the caller
/// keeps theirs for any further runs.
///
/// An empty base is the case worth knowing about, because **five of the nine
/// opcodes are inert on an empty graph** — `Swap`, `Hop` and the three
/// `Local*` all need existing structure to walk, so early generations do
/// nothing until `Add`/`Toggle` have built something. Self-correcting, and
/// stated here so it is not read as a defect.
///
/// # Errors
///
/// `ValueError` if the operation weights are unusable — all zero, negative,
/// or non-finite. `Config::validate` already rejects those, so this is a
/// backstop for a `Config` assembled in Rust without validation.
pub(crate) fn edge_edit_start<R: Rng + ?Sized>(
    config: &Config,
    edge_edit: &EdgeEditGenomeConfig,
    base_graph: Option<&Graph>,
    rng: &mut R,
) -> PyResult<(EdgeEditContext, Vec<EdgeEditGenome>)> {
    let operators =
        EdgeEditOperators::new(edge_edit.operation_weights).map_err(PyValueError::new_err)?;

    let mut population = Vec::with_capacity(config.population_size);
    for _ in 0..config.population_size {
        population.push(EdgeEditGenome::random_with_operators(
            edge_edit.gene_length,
            Arc::clone(&operators),
            rng,
        ));
    }

    // A seeded run keeps one individual that edits nothing, so generation 0
    // contains the graph the caller supplied and not only random departures
    // from it. Every gene is opcode 8, `Null`, which expression skips — so this
    // genome expresses to exactly the base graph.
    //
    // Unconditional, with no config flag: without it a seeded run can return
    // something worse than its own input, if nothing in a random generation 0
    // happens to beat it. What that buys is a soft floor rather than a hard one
    // — elites are rescored every generation, so a stochastic objective can
    // still evict this individual on a bad draw.
    if base_graph.is_some() && !population.is_empty() {
        population[0] = EdgeEditGenome::new_with_operators(
            vec![IDENTITY_GENE; edge_edit.gene_length],
            Arc::clone(&operators),
        );
    }

    // Unset means empty, which is the default an unseeded run gets.
    let starting_graph = match base_graph {
        Some(graph) => graph.clone(),
        None => Graph::new(config.network_size, config.max_edge_multiplicity),
    };

    let context = EdgeEditContext {
        base_graph: starting_graph,
        mutation: edge_edit_mutation(&edge_edit.mutation),
    };
    Ok((context, population))
}

/// The SDA starting population and the context it expresses against.
///
/// The second start builder — step 5 of the chain that adds a representation,
/// alongside [`edge_edit_start`], where that step's obligations are stated.
///
/// **`num_chars` is derived, never configured** (§3.2): the alphabet is
/// `max_edge_multiplicity + 1`, so every character the automaton can emit is
/// a legal edge weight and none is silently clamped away by
/// `Graph::set_edge`. The same cap goes into the context, so the genome and
/// the graph it expresses against agree by construction rather than by the
/// caller remembering to pass the same number twice.
///
/// # Errors
///
/// `ValueError` if `init_state >= num_states`, or if the dimensions do not
/// fit `SdaGenome`'s storage types (`num_states` up to 65536, the derived
/// `num_chars` up to 256, `max_resp_len` at least 1).
///
/// Both are backstops rather than the primary guard — `Config::validate`
/// rejects the `init_state` case (`config.rs`, `validate_genome`) and both
/// front ends validate before constructing. Kept because the alternative is
/// worse than redundant: `SdaGenome::run` indexes its response table with
/// `init_state`, so an out-of-range value **panics during expression**,
/// which crosses the FFI as an opaque `PanicException` (§7). Reporting beats
/// asserting for anything that can reach a release build.
pub(crate) fn sda_start<R: Rng + ?Sized>(
    config: &Config,
    sda: &SdaGenomeConfig,
    rng: &mut R,
) -> PyResult<(SdaContext, Vec<SdaGenome>)> {
    if sda.init_state >= sda.num_states {
        return Err(PyValueError::new_err(format!(
            "init_state ({}) must be less than num_states ({}); \
             SdaGenome::run indexes its response table with it",
            sda.init_state, sda.num_states,
        )));
    }

    let cap = config.max_edge_multiplicity;
    // Validate once, here, rather than on every individual: the three
    // dimensions are the same for the whole population, so a failure can only
    // be a startup failure.
    let dimensions =
        SdaDimensions::from_edge_multiplicity_cap(sda.num_states, cap, sda.max_resp_len)
            .map_err(PyValueError::new_err)?;

    let mut population = Vec::with_capacity(config.population_size);
    for _ in 0..config.population_size {
        population.push(SdaGenome::random_with_dimensions(&dimensions, rng));
    }

    let context = SdaContext {
        num_nodes: config.network_size,
        init_state: sda.init_state,
        max_edge_multiplicity: cap,
        init_char_mutation_rate: sda.init_char_mutation_rate,
        transition_vs_response_rate: sda.transition_vs_response_rate,
        mutation: sda_mutation(&sda.mutation),
    };
    Ok((context, population))
}

/// The per-run seed list for a replicate request: one master seed in, `n_runs`
/// seeds out, in run order.
///
/// The master seeds a generator whose output stream *is* the seed list — run
/// `i` takes draw `i`. That buys one property deliberately: **a run's seed does
/// not depend on how many runs were asked for.** Extending an experiment from 30
/// replicates to 50 reproduces the first 30 exactly, so replicates already
/// collected are never invalidated by asking for more.
///
/// `master + i` or `hash(master, i)` would give the same property. `master ^ i`
/// would not — nearby masters collide across run indices, so master 4 run 1 and
/// master 5 run 0 are the same run.
///
/// Each seed returned is then a whole run's `seed` argument, drawn from
/// independent generator state rather than shared with any other replicate.
pub(crate) fn replicate_seeds(master: u64, n_runs: usize) -> Vec<u64> {
    let mut rng = ChaCha8Rng::seed_from_u64(master);
    let mut seeds = Vec::with_capacity(n_runs);
    for _ in 0..n_runs {
        seeds.push(rng.random::<u64>());
    }
    seeds
}

/// Run one evolution and hand back its result with the genome type erased.
///
/// # Part of the chain that adds a representation
///
/// The match below is step 6 of seven: one arm, selecting the representation
/// and calling its step-5 start builder. Steps 4, 5 and 6 are one change split
/// across two files — a `GenomeConfig` variant nothing constructs is dead code,
/// and an arm for a variant that does not exist will not compile.
/// [`crate::genomes::genome`]'s module doc has all seven.
///
/// **Step 2 of the dispatch** (§1, §8). The objective has already been erased to
/// `Box<dyn Fitness>`, so only strategy × genome is left — and this is arranged
/// as genome outside, strategy inside [`run_strategy`], which is why there are
/// two arms here and two there rather than four copies of the same body. Adding
/// a third genome is one arm here; a third strategy is one arm there.
///
/// **All randomness derives from `seed`.** The population is drawn first, then
/// the evolver's own seed is drawn from the same stream — never `seed` itself,
/// which the evolver would use to re-seed its own ChaCha8 and thereby replay the
/// exact draws that just built the population.
///
/// `base_graph` reaches only the edge-edit arm. The SDA genome generates its
/// graph rather than editing one, so there is nothing for a base to seed —
/// which is why `set_base_graph` refuses an SDA-configured evolver outright
/// instead of accepting a value that dies here.
///
/// # Errors
///
/// `ValueError` for any dimension the genome constructors reject. `Config::validate`
/// has already run at construction, so these are backstops rather than the first
/// line of defence — see [`sda_start`].
pub(crate) fn evolve<F: Fitness>(
    config: &Config,
    fitness: &F,
    base_graph: Option<&Graph>,
    seed: u64,
) -> PyResult<ErasedOutcome> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let scope = scope(&config.scope);
    let selection = selection(&config.selection);

    // Genome outside, strategy inside: `Genome` cannot be a trait object, so the
    // concrete type has to be settled before an evolver can be named at all.
    match &config.genome {
        GenomeConfig::EdgeEdit(edge_edit) => {
            let (genome_context, population) =
                edge_edit_start(config, edge_edit, base_graph, &mut rng)?;
            Ok(run_strategy(
                config,
                genome_context,
                population,
                scope,
                selection,
                fitness,
                rng.random::<u64>(),
            ))
        }
        GenomeConfig::Sda(sda) => {
            let (genome_context, population) = sda_start(config, sda, &mut rng)?;
            Ok(run_strategy(
                config,
                genome_context,
                population,
                scope,
                selection,
                fitness,
                rng.random::<u64>(),
            ))
        } // ADD A GENOME STEP 6 — one arm, calling your step-5 start builder:
          //
          //     GenomeConfig::MyGenome(mine) => {
          //         let (genome_context, population) = my_genome_start(config, mine, &mut rng)?;
          //         Ok(run_strategy(
          //             config,
          //             genome_context,
          //             population,
          //             scope,
          //             selection,
          //             fitness,
          //             rng.random::<u64>(),
          //         ))
          //     }
          //
          // Copy the arm above verbatim and change the builder it calls. The
          // match is exhaustive, so omitting this will not compile — which is
          // the whole reason steps 4, 5 and 6 cannot be half-done.
    }
}

/// How many replicates may execute at once.
///
/// Never more than the caller allowed, never more than there are runs — extra
/// threads past the replicate count have nothing to do — and never zero, which
/// `ThreadPoolBuilder` reads as "pick a default" rather than "run nothing".
fn effective_concurrency(max_cores: Option<usize>, n_runs: usize) -> usize {
    let available = match max_cores {
        Some(cap) => cap,
        // `available_parallelism` fails on some constrained targets; one thread
        // is the honest fallback, since the alternative is guessing high.
        None => std::thread::available_parallelism().map_or(1, |n| n.get()),
    };
    available.min(n_runs).max(1)
}

/// Run one replicate per entry in `seeds`, and hand back their outcomes **in
/// run order**.
///
/// `objectives` is parallel to `seeds` — one objective per run, built by the
/// caller before this is entered. They are not built here because each needs a
/// fresh per-run instance (an SIR objective owns an epidemic counter, and
/// sharing one across concurrent runs lets thread scheduling decide which run
/// sees which epidemic seed), and because building a Python objective touches
/// the interpreter, which must not happen inside a rayon worker.
///
/// **The engine picks parallel or sequential; the caller does not.** A native
/// Rust objective scores without the interpreter, so replicates are independent
/// and concurrency is nearly free. Under `fitness = "python"` every scoring call
/// re-acquires the GIL, so `n` concurrent runs are `n` threads contending for
/// one lock — slower than sequential *and* contended. Exposing that as a setting
/// would only create a way to choose wrong.
///
/// **The pool is built here, per call, never configured globally.** Rayon's
/// global pool can be configured once per process, and this crate is a Python
/// extension module imported once per session — a global configuration would
/// make `max_cores` a property of whichever `run` happened first, and the second
/// call with a different cap would fail outright.
///
/// # Errors
///
/// `ValueError` if the thread pool cannot be built, or from any replicate that
/// fails — the first such error is returned and the rest are abandoned.
pub(crate) fn run_replicates(
    config: &Config,
    objectives: &[Box<dyn Fitness>],
    base_graph: Option<&Graph>,
    seeds: &[u64],
    max_cores: Option<usize>,
) -> PyResult<Vec<ErasedOutcome>> {
    debug_assert_eq!(objectives.len(), seeds.len(), "one objective per run");

    // Nothing to run, and in particular no pool to size: `effective_concurrency`
    // would clamp to 1 and build a pool for zero work.
    if seeds.is_empty() {
        return Ok(Vec::new());
    }

    if matches!(config.fitness, FitnessConfig::Python) {
        let mut outcomes = Vec::with_capacity(seeds.len());
        for (objective, &seed) in objectives.iter().zip(seeds) {
            outcomes.push(evolve(config, objective, base_graph, seed)?);
        }
        return Ok(outcomes);
    }

    let pool = ThreadPoolBuilder::new()
        .num_threads(effective_concurrency(max_cores, seeds.len()))
        .build()
        .map_err(|err| PyValueError::new_err(format!("could not build the thread pool: {err}")))?;

    // `zip` over two slices is an *indexed* parallel iterator, so `collect`
    // rebuilds the vector by position rather than by completion. That is what
    // makes the result order the run order: the same master seed has to give the
    // same output ordering on every machine, whatever order the runs finish in.
    pool.install(|| {
        seeds
            .par_iter()
            .zip(objectives.par_iter())
            .map(|(&seed, objective)| evolve(config, objective, base_graph, seed))
            .collect::<PyResult<Vec<ErasedOutcome>>>()
    })
}

/// Pick the evolution strategy and run it, for a genome type already settled.
///
/// Generic over `G`, so both genomes share this body instead of the strategy
/// match being written once per genome. That is the whole reason dispatch is
/// 2 + 2 arms rather than 2 × 2.
///
/// Step 4 of the chain that adds a strategy (`crate::evolver::Evolver`'s doc
/// has all seven): this match is where a `config::EvolutionConfig` variant
/// becomes a running evolver — build the strategy's `TypeContext` from the
/// variant's fields, construct it, and call `run`. `erase`, right below, is
/// the step after this one, and for most strategies it is not a step at all.
fn run_strategy<G: Genome, F: Fitness>(
    config: &Config,
    genome_context: G::Context,
    population: Vec<G>,
    scope: Scope,
    selection: Selection,
    fitness: &F,
    seed: u64,
) -> ErasedOutcome {
    let shared = SharedEvolutionContext {
        genome_context,
        crossover_rate: config.crossover_rate,
        mutation_rate: config.mutation_rate,
        max_mutations: config.max_mutations,
        selection,
        scope,
        crossover: crossover(&config.crossover),
    };

    match &config.evolution {
        EvolutionConfig::Generational {
            num_generations,
            elite_count,
        } => {
            let type_context = GenerationalContext {
                num_generations: *num_generations,
                elite_count: *elite_count,
            };
            let mut evolver = GenerationalEvolver::new(shared, type_context, population);
            erase(evolver.run(fitness, seed))
        }
        EvolutionConfig::SteadyState {
            num_mating_events,
            replacement: replacement_config,
        } => {
            let type_context = SteadyStateContext {
                num_mating_events: *num_mating_events,
                replacement: replacement(replacement_config),
            };
            let mut evolver = SteadyStateEvolver::new(shared, type_context, population);
            erase(evolver.run(fitness, seed))
        } // ADD A STRATEGY STEP 4 — one arm, building your `TypeContext` and
          // calling your evolver's `new` and `run`:
          //
          //     EvolutionConfig::MyStrategy { num_my_events } => {
          //         let type_context = MyStrategyContext {
          //             num_my_events: *num_my_events,
          //             // If your strategy displaces individuals, map its own
          //             // policy here as steady-state does:
          //             //     replacement: replacement(replacement_config),
          //         };
          //         let mut evolver = MyStrategyEvolver::new(shared, type_context, population);
          //         erase(evolver.run(fitness, seed))
          //     }
          //
          // `shared` already carries the scope and the selection scheme, built
          // once above this match, so a strategy reads them without asking for
          // them. Only what is *yours* goes in the `TypeContext`.
          //
          // `erase`, right below, is the step after this one — for most
          // strategies it is not a step at all; search `ADD A STRATEGY STEP 5`.
    }
}

/// Drop the genome type, converting every fitness back to the objective's units.
///
/// One generic function rather than the same lines in each arm. The conversion
/// is the part that matters: everything inside the engine is lower-is-better,
/// and this is the boundary where that stops being true.
/// `Direction::orient` is its own inverse, so it is also what undoes itself.
///
/// **The history needs converting too, row by row** — and only its two fitness
/// columns. `std_dev` and `ci_95` are left exactly as the engine computed them,
/// because a spread is identical under negation. Orienting either would be a
/// silent defect: the number stays positive, so nothing looks wrong.
///
/// **The last step of the chain that adds a strategy is usually nothing, and
/// this function is why.** It is generic over `G` alone — no match on
/// strategy inside it — so a new strategy that returns a normal
/// `EvolutionOutcome<G>` needs no edit here at all. It only becomes a step if
/// a strategy's outcome needs handling this conversion does not already give
/// it, which none of the shipped strategies do.
// ADD A STRATEGY STEP 5 — usually nothing, and that is the point: this
// function is generic over `G` alone, with no match on strategy inside it.
// Only touch it if your strategy's `EvolutionOutcome` needs converting in a
// way the loop above does not already cover.
fn erase<G: Genome>(outcome: EvolutionOutcome<G>) -> ErasedOutcome {
    let direction = outcome.direction;

    let mut history = Vec::with_capacity(outcome.history.len());
    for row in outcome.history {
        history.push(GenerationStats {
            iteration: row.iteration,
            best_fitness: direction.orient(row.best_fitness),
            mean_fitness: direction.orient(row.mean_fitness),
            std_dev: row.std_dev,
            ci_95: row.ci_95,
        });
    }

    ErasedOutcome {
        best_fitness: direction.orient(outcome.best_fitness_engine),
        best_edges: outcome.best_graph.get_edge_list(),
        num_nodes: outcome.best_graph.num_nodes,
        best_genome_repr: outcome.best_genome.print(),
        history,
    }
}

/// Turn the `[crossover]` block into the operator the engine runs.
///
/// Step 4 of the six on [`Crossover`], and the counterpart to `selection`
/// below. A variant added to `config::CrossoverConfig` and not here does not
/// compile, which is the whole reason the mapping is a match rather than a
/// blanket conversion.
fn crossover(config: &CrossoverConfig) -> Crossover {
    match config {
        CrossoverConfig::TwoPoint => Crossover::TwoPoint,
        // ADD A CROSSOVER STEP 4 — the arm mapping your `CrossoverConfig`
        // variant onto the matching `Crossover` one. The step after this one
        // is optional — search `ADD A CROSSOVER STEP 5`.
    }
}

/// Turn `[genome] mutation` into the operator an edge-edit run applies.
///
/// One per representation, unlike `crossover` above, because the operators are
/// per representation — `EdgeEditMutation` says why.
fn edge_edit_mutation(config: &EdgeEditMutationConfig) -> EdgeEditMutation {
    match config {
        EdgeEditMutationConfig::RerollGene => EdgeEditMutation::RerollGene,
        // ADD A MUTATION STEP 3 (for EdgeEdit) — the arm mapping your `EdgeEditMutationConfig`
        // variant onto the matching `EdgeEditMutation` one. The step after
        // this one is optional — search `ADD A MUTATION STEP 4 (for EdgeEdit)`.
    }
}

/// Turn `[genome] mutation` into the operator an SDA run applies.
fn sda_mutation(config: &SdaMutationConfig) -> SdaMutation {
    match config {
        SdaMutationConfig::RedrawOne => SdaMutation::RedrawOne,
        // ADD A MUTATION STEP 3 (for SDA) — the arm mapping your `SdaMutationConfig`
        // variant onto the matching `SdaMutation` one. The step after this
        // one is optional — search `ADD A MUTATION STEP 4 (for SDA)`.
    }
}

/// Map the `[scope]` block onto the slice each breeding event draws from.
///
/// Independent of `[selection]`: the scope's size is its own parameter, so a
/// scheme with no tournament can still say how large a scope it wants. This arm
/// is the last step a new `Scope` variant needs — `crate::evolver::scope::Scope`
/// walks all three.
fn scope(config: &ScopeConfig) -> Scope {
    match config {
        ScopeConfig::Global => Scope::Global,
        ScopeConfig::RandomSubset { size } => Scope::RandomSubset { size: *size },
        // ADD A SCOPE STEP 4 — the arm turning your config variant into the
        // engine one:
        //
        //     ScopeConfig::Neighbourhood { radius } => {
        //         Scope::Neighbourhood { radius: *radius }
        //     }
        //
        // Steps 3 and 4 are one change split across two files: a config variant
        // nothing constructs is dead, and an arm for a variant that does not
        // exist will not compile. The Python mirror is next, and optional —
        // search `ADD A SCOPE STEP 5` for it.
    }
}

/// Map `[evolution] replacement` onto the engine's own replacement policy.
///
/// Steady-state's, like `elite_count` is generational's — which is why it is
/// read from the strategy's own table rather than a block of its own.
fn replacement(config: &ReplacementConfig) -> Replacement {
    match config {
        ReplacementConfig::Worst => Replacement::Worst,
        // ADD A REPLACEMENT STEP 3 (for SteadyState, second half) — the arm turning your config
        // variant into the engine one:
        //
        //     ReplacementConfig::Random => Replacement::Random,
        //
        // This and the config variant are one change split across two files.
    }
}

/// Map the `[selection]` block onto the engine's own selection scheme.
///
/// Kept a function rather than inlined so a second scheme is one arm here and
/// touches neither evolver.
fn selection(config: &SelectionConfig) -> Selection {
    match config {
        SelectionConfig::Best => Selection::Best,
        SelectionConfig::Tournament { tournament_size } => Selection::Tournament {
            tournament_size: *tournament_size,
        },
        // ADD A SELECTION STEP 4 — the arm turning your config variant into the
        // engine one:
        //
        //     SelectionConfig::Roulette { pressure } => {
        //         Selection::Roulette { pressure: *pressure }
        //     }
        //
        // Nothing here decides a scope: that is `[scope]`'s own block, and the
        // reason this function no longer needs to know which strategy is
        // running. Steps 3 and 4 are one change split across two files. The
        // Python mirror is next, and optional — search
        // `ADD A SELECTION STEP 5` for it.
    }
}

/// Map the `[fitness]` block onto the simulator's own sampling parameters.
///
/// Two types with overlapping names and neither is redundant:
/// [`crate::config::SirParams`] is the deserializable config block, and
/// [`SirSampleParams`] is what the simulator takes — deliberately independent
/// of the config schema, so `sir.rs` does not depend on `[fitness]`'s spelling.
/// This function is the seam, and it lives in the dispatch layer because that
/// is where `config.rs`'s module doc says config becomes engine types.
///
/// Note the nesting changes shape: the config block is flat, while
/// [`SirSampleParams`] separates the epidemic's own two parameters into a nested
/// [`sir::SirParams`] from the batch settings around them.
///
/// **No seed is mapped.** One master seed reaches `run` and every objective
/// derives from it (§7, §8.1), so a seed in the config would be a second,
/// competing source — `Config::from_toml_str` rejects a stray `seed` key
/// outright rather than ignoring it.
fn sir_sample_params(params: &config::SirParams) -> SirSampleParams {
    SirSampleParams {
        epidemic: sir::SirParams {
            infection_rate: params.infection_rate,
            patient_zero: params.patient_zero,
        },
        num_epidemics: params.num_epidemics,
        min_epidemic_length: params.min_epidemic_length,
        max_epidemic_retries: params.max_epidemic_retries,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    use crate::config::FitnessConfig;
    use crate::fitness::Direction;
    // Only the tests express a genome directly; a run goes through the evolver.
    use crate::genomes::Genome;

    // Fixtures are deliberately local rather than shared with `lib.rs`'s test
    // module. Test helpers cannot be imported across sibling `#[cfg(test)]`
    // modules without giving them a home in the lib target, which GitHub #56
    // calls out as a decision to make on purpose rather than by default.
    const SIR_FITNESS: &str =
        "[fitness]\ntype = \"epi_spread\"\ninfection_rate = 0.05\nnum_epidemics = 30\n";
    const PYTHON_FITNESS: &str = "[fitness]\ntype = \"python\"\n";

    /// A config whose `[fitness]` block is exactly `fitness_block`.
    fn config_with(fitness_block: &str) -> Config {
        let text = format!(
            "population_size = 10\n\
             network_size = 8\n\
             max_edge_multiplicity = 1\n\
             crossover_rate = 0.8\n\
             mutation_rate = 0.2\n\
             \n\
             [evolution]\n\
             type = \"generational\"\n\
             num_generations = 5\n\
             \n\
             [scope]\n\
             type = \"global\"\n\
             \n\
             [selection]\n\
             type = \"tournament\"\n\
             tournament_size = 4\n\
             \n\
             [genome]\n\
             type = \"edge_edit\"\n\
             gene_length = 16\n\
             \n\
             {fitness_block}"
        );
        Config::from_toml_str(&text).expect("the test config parses")
    }

    fn evolver_with(fitness_block: &str) -> GraphEvolver {
        GraphEvolver {
            config: config_with(fitness_block),
            fitness_function: None,
            base_graph: None,
            min_node_index: None,
            struct_match_reference: Default::default(),
            config_toml: String::new(),
        }
    }

    /// Write a reference set into a fresh temporary folder and hand back the
    /// `[fitness]` block naming it.
    ///
    /// Three triangles and a four-cycle: the triangles give the clustering
    /// family something non-zero to work with, which a set of rings or paths
    /// alone would not.
    fn struct_match_block(name: &str) -> String {
        let folder = std::env::temp_dir().join(format!("get_struct_match_{name}"));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("temp reference folder");

        // Each file states its own size, as the loader requires. The two
        // triangles are three nodes and the four-cycle is four, so the counts
        // differ across the set — which is the normal case for real reference
        // data and the reason nothing here is checked against `network_size`.
        let files = [
            ("a.csv", "# nodes = 3\n0,1,1\n1,2,1\n0,2,1\n"),
            ("b.csv", "# nodes = 3\n0,1,1\n1,2,1\n0,2,1\n"),
            ("c.csv", "# nodes = 4\n0,1,1\n1,2,1\n2,3,1\n0,3,1\n"),
        ];
        for (file_name, text) in files {
            std::fs::write(folder.join(file_name), text).expect("temp reference file");
        }

        format!(
            "[fitness]\ntype = \"struct_match\"\nreference_folder = {:?}\n\
             degree_bins = 6\nclustering_bins = 4\nspectral_bins = 4\n",
            folder.display().to_string()
        )
    }

    /// The `[fitness]` block for `objective`'s remaining two SIR arms.
    fn sir_block(type_name: &str, extra: &str) -> String {
        format!(
            "[fitness]\ntype = \"{type_name}\"\ninfection_rate = 0.05\n\
             num_epidemics = 30\n{extra}"
        )
    }

    #[test]
    fn each_objective_erases_to_a_box_carrying_its_own_direction() {
        // Last step of the chain that adds an objective: a new
        // `FitnessConfig` variant gets a case here. `crate::fitness`'s module
        // doc walks all six steps.
        //
        // The failure this exists for is silent. `Fitness::direction` has a
        // default of `Minimize`, so a boxed objective whose direction is not
        // forwarded reports "minimize" whatever it holds — and both maximizing
        // objectives then run the search backwards while merely looking
        // unconverged (§5.1). Nothing panics and no number looks wrong.
        let cases = [
            (sir_block("epi_spread", ""), Direction::Maximize),
            (sir_block("epi_length", ""), Direction::Maximize),
            (
                sir_block("epi_prof_match", "target_profile = [1, 3, 7, 2]\n"),
                Direction::Minimize,
            ),
            (struct_match_block("direction"), Direction::Minimize),
        ];

        for (block, expected) in cases {
            let evolver = evolver_with(&block);
            let objective = evolver
                .objective(7, None)
                .unwrap_or_else(|err| panic!("{block} should build an objective: {err}"));

            assert_eq!(
                objective.direction(),
                expected,
                "wrong direction erased for: {block}",
            );
        }
    }

    #[test]
    fn struct_match_replicates_share_one_reduced_reference_set() {
        // `objective()` runs once per replicate (`lib.rs`), and rebuilding the
        // reference set each time would re-read the folder and re-run an
        // eigendecomposition of every reference graph, n_runs times, in a
        // phase that logs nothing. The objects must differ; what is behind
        // them must not.
        let evolver = evolver_with(&struct_match_block("shared"));

        let first = evolver
            .objective(1, None)
            .expect("first replicate's objective");
        let second = evolver
            .objective(2, None)
            .expect("second replicate's objective");

        // Same reference set means the same score for the same graph.
        let mut candidate = Graph::new(3, 1);
        candidate.set_edges(&[(0, 1, 1), (1, 2, 1), (0, 2, 1)]);
        assert_eq!(first.evaluate(&candidate), second.evaluate(&candidate));

        let cached = evolver
            .struct_match_reference
            .get()
            .expect("the first build populates the cache");
        // Both objectives are still alive, so the cache plus the two of them
        // is three handles on ONE allocation. Had each replicate rebuilt its
        // own, every count here would be 1.
        assert_eq!(
            Arc::strong_count(cached),
            3,
            "the cache and both replicates' objectives should share one reference set"
        );
    }

    #[test]
    fn struct_match_takes_its_degree_axis_from_the_reference_set() {
        // Not a config field: a top set too low squashes every reference
        // histogram into the last bin, retiring the whole degree family with
        // nothing reporting it. The fixture's densest graph is a triangle, so
        // the top is 2.
        let evolver = evolver_with(&struct_match_block("axis"));

        evolver.objective(1, None).expect("an objective");

        let axes = evolver
            .struct_match_reference
            .get()
            .expect("built above")
            .axes();
        assert_eq!(axes.max_degree, 2, "the reference set's highest degree");
        assert_eq!(axes.degree_bins, 6, "and the configured bin count");
    }

    #[test]
    fn struct_match_reports_a_reference_folder_it_cannot_read() {
        // `Config::validate` does no I/O, so this is the layer that catches a
        // mistyped path -- and it must catch it, because scoring against an
        // absent reference set would otherwise be indistinguishable from a
        // run that is going well.
        let block = "[fitness]\ntype = \"struct_match\"\n\
                     reference_folder = \"no_such_reference_folder_anywhere\"\n";
        let evolver = evolver_with(block);

        let text = match evolver.objective(1, None) {
            Err(error) => error.to_string(),
            Ok(_) => panic!("a missing folder must not build an objective"),
        };
        assert!(
            text.contains("no_such_reference_folder_anywhere"),
            "the error should name the folder, got: {text}"
        );
    }

    #[test]
    fn struct_match_rejects_an_empty_reference_folder() {
        // An empty reference set scores every candidate 0.0, and 0.0 is a
        // *perfect* score here: the population would converge immediately and
        // the log would read like a solved problem. Task A made it a hard
        // error; this is the layer that surfaces it.
        let folder = std::env::temp_dir().join("get_struct_match_empty");
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("temp folder");

        let block = format!(
            "[fitness]\ntype = \"struct_match\"\nreference_folder = {:?}\n",
            folder.display().to_string()
        );
        let evolver = evolver_with(&block);

        assert!(
            evolver.objective(1, None).is_err(),
            "an empty reference set must not build an objective"
        );
    }

    /// Capture every `UserWarning` a closure raises, the same way
    /// `lib.rs`'s `warnings_from` does for the base-graph and
    /// `load_reference_graphs` tests — duplicated locally because that one
    /// is private to `lib.rs`'s own test module.
    fn warnings_from(py: Python<'_>, body: impl FnOnce()) -> Vec<String> {
        let scope = pyo3::types::PyDict::new(py);

        py.run(
            c"import warnings\n\
              recorder = warnings.catch_warnings(record=True)\n\
              caught = recorder.__enter__()\n\
              warnings.simplefilter('always')",
            None,
            Some(&scope),
        )
        .expect("the recorder starts");

        body();

        py.run(
            c"recorder.__exit__(None, None, None)\n\
              messages = [str(entry.message) for entry in caught]",
            None,
            Some(&scope),
        )
        .expect("the recorder stops");

        scope
            .get_item("messages")
            .expect("reading the collected messages")
            .expect("the recorder left messages behind")
            .extract()
            .expect("they are strings")
    }

    /// A reference folder carrying all three `LoadWarning`s `graph_io`
    /// produces, plus a clean file so the reduced set is non-empty and the
    /// objective actually builds. Hands back the `[fitness]` block naming it.
    fn struct_match_block_with_warnings(name: &str) -> String {
        let folder = std::env::temp_dir().join(format!("get_struct_match_warn_{name}"));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("temp reference folder");

        let files = [
            // Repeated edge: (0, 1) appears twice. `struct_match_reference`
            // hardcodes `max_edge_multiplicity = 1`, so both weights stay 1 —
            // the duplicate is what's under test, not the weight cap.
            ("a_duplicate.csv", "# nodes = 3\n0,1,1\n0,1,1\n1,2,1\n"),
            // Zero-weight edge alongside a real one.
            ("b_zero_weight.csv", "# nodes = 3\n0,1,0\n1,2,1\n"),
            // No edges at all.
            ("c_empty.csv", "# nodes = 2\n"),
            ("d_clean.csv", "# nodes = 3\n0,1,1\n1,2,1\n0,2,1\n"),
        ];
        for (file_name, text) in files {
            std::fs::write(folder.join(file_name), text).expect("temp reference file");
        }

        format!(
            "[fitness]\ntype = \"struct_match\"\nreference_folder = {:?}\n\
             degree_bins = 4\nclustering_bins = 2\nspectral_bins = 2\n",
            folder.display().to_string()
        )
    }

    #[test]
    fn struct_match_reference_warns_through_python_for_every_load_warning() {
        // GitHub #145: `struct_match_reference` used to call `to_graph`
        // straight after `load_edge_folder` and never look at `.warnings`,
        // so a reference folder built with, say, both directions of every
        // undirected edge loaded clean through this path and loud through
        // `load_reference_graphs` — the same kind of folder, two different
        // front ends, one of them silent.
        Python::attach(|py| {
            let evolver = evolver_with(&struct_match_block_with_warnings("python"));

            let messages = warnings_from(py, || {
                evolver
                    .objective(1, Some(py))
                    .expect("a folder with warnings still yields a usable reference set");
            });

            assert!(
                messages
                    .iter()
                    .any(|m| m.contains("appears more than once")),
                "no duplicate-edge warning in {messages:?}"
            );
            assert!(
                messages.iter().any(|m| m.contains("has weight 0")),
                "no zero-weight warning in {messages:?}"
            );
            assert!(
                messages.iter().any(|m| m.contains("holds no edges")),
                "no empty-file warning in {messages:?}"
            );
            // Each warning names the file it came from, not just the folder,
            // so the four rows above must produce at least four messages —
            // three files with something to say and the folder itself never
            // collapsing them into one.
            assert!(
                messages.iter().any(|m| m.contains("a_duplicate.csv")),
                "{messages:?}"
            );
            assert!(
                messages.iter().any(|m| m.contains("b_zero_weight.csv")),
                "{messages:?}"
            );
            assert!(
                messages.iter().any(|m| m.contains("c_empty.csv")),
                "{messages:?}"
            );
        });
    }

    #[test]
    fn struct_match_reference_only_the_oncelock_winner_warns() {
        // The race `struct_match_reference` documents: a second call after
        // the reference set is already cached must not re-read the folder or
        // re-emit its warnings, because nothing about the cached call is
        // using a fresh load.
        Python::attach(|py| {
            let evolver = evolver_with(&struct_match_block_with_warnings("cached"));

            let messages = warnings_from(py, || {
                evolver
                    .objective(1, Some(py))
                    .expect("first call loads and warns");
                evolver
                    .objective(2, Some(py))
                    .expect("second call reuses the cached reference set");
            });

            let duplicate_edge_warnings = messages
                .iter()
                .filter(|m| m.contains("appears more than once"))
                .count();
            assert_eq!(
                duplicate_edge_warnings, 1,
                "the cached call must not re-warn: {messages:?}"
            );
        });
    }

    #[test]
    fn the_sir_block_reaches_the_simulator_field_for_field() {
        // `config::SirParams` and `sir::SirSampleParams` are two types with
        // overlapping names and a different shape — the config block is flat,
        // the simulator's nests the epidemic's own parameters. A field mapped to
        // the wrong place still compiles when the types agree, so this checks
        // each one rather than trusting the mapping to be obvious.
        let config = config_with(&sir_block(
            "epi_spread",
            "patient_zero = 4\nmin_epidemic_length = 2\nmax_epidemic_retries = 9\n",
        ));

        let block = match &config.fitness {
            FitnessConfig::EpiSpread { sir } => sir,
            other => panic!("expected epi_spread, got {other:?}"),
        };
        let mapped = sir_sample_params(block);

        assert_eq!(mapped.epidemic.infection_rate, 0.05);
        assert_eq!(mapped.epidemic.patient_zero, Some(4));
        assert_eq!(mapped.num_epidemics, 30);
        assert_eq!(mapped.min_epidemic_length, 2);
        assert_eq!(mapped.max_epidemic_retries, 9);
    }

    #[test]
    fn an_omitted_patient_zero_stays_unpinned_through_the_mapping() {
        // `None` means "draw a fresh node per epidemic" (§5.2). Defaulting it to
        // node 0 instead would seed every outbreak from the same vertex and
        // quietly change what the objective measures.
        let config = config_with(SIR_FITNESS);
        let block = match &config.fitness {
            FitnessConfig::EpiSpread { sir } => sir,
            other => panic!("expected epi_spread, got {other:?}"),
        };

        assert_eq!(sir_sample_params(block).epidemic.patient_zero, None);
    }

    #[test]
    fn each_call_builds_a_fresh_sir_objective() {
        // §8.1: replicates must not share an objective, because every SIR
        // objective owns an `EpidemicScorer` whose counter is per-run state.
        // Sharing one lets thread scheduling decide which run sees which seed.
        let evolver = evolver_with(SIR_FITNESS);

        let first = evolver.objective(1, None).expect("first objective");
        let second = evolver.objective(1, None).expect("second objective");

        // Same seed, so the two agree — what matters is that both are live at
        // once, which a shared or moved-out instance could not manage.
        let graph = Graph::new(6, 1);
        assert_eq!(first.evaluate(&graph), second.evaluate(&graph));
    }

    /// A config whose `[genome]` block is `genome_block` and whose edge cap is
    /// `cap`, everything else fixed. The cap is a parameter because the
    /// cap-narrowing check only bites when a base graph was built under a wider
    /// one than the run it is fed into.
    fn config_with_genome_and_cap(genome_block: &str, cap: u32) -> Config {
        let text = format!(
            "population_size = 4\n\
             network_size = 8\n\
             max_edge_multiplicity = {cap}\n\
             crossover_rate = 0.8\n\
             mutation_rate = 0.2\n\
             \n\
             [evolution]\n\
             type = \"generational\"\n\
             num_generations = 5\n\
             \n\
             [scope]\n\
             type = \"global\"\n\
             \n\
             [selection]\n\
             type = \"tournament\"\n\
             tournament_size = 4\n\
             \n\
             {genome_block}\n\
             {SIR_FITNESS}"
        );
        Config::from_toml_str(&text).expect("the test config parses")
    }

    fn evolver_with_genome(genome_block: &str) -> GraphEvolver {
        evolver_with_genome_and_cap(genome_block, 3)
    }

    fn evolver_with_genome_and_cap(genome_block: &str, cap: u32) -> GraphEvolver {
        GraphEvolver {
            config: config_with_genome_and_cap(genome_block, cap),
            fitness_function: None,
            base_graph: None,
            min_node_index: None,
            struct_match_reference: Default::default(),
            config_toml: String::new(),
        }
    }

    fn test_rng() -> ChaCha8Rng {
        ChaCha8Rng::seed_from_u64(11)
    }

    /// The `[genome]` block of a config built by `evolver_with_genome`, for the
    /// tests that call `edge_edit_start` directly. Reading it back out of the
    /// config is what stops the test restating `gene_length` beside the TOML
    /// that already sets it, and disagreeing with it.
    fn edge_edit_config(config: &Config) -> &EdgeEditGenomeConfig {
        match &config.genome {
            GenomeConfig::EdgeEdit(edge_edit) => edge_edit,
            other => panic!("expected an edge-edit genome, got {other:?}"),
        }
    }

    #[test]
    fn the_edge_edit_start_sizes_the_population_and_the_empty_base_graph() {
        let evolver = evolver_with_genome("[genome]\ntype = \"edge_edit\"\ngene_length = 16\n");

        let (context, population) = edge_edit_start(
            &evolver.config,
            edge_edit_config(&evolver.config),
            None,
            &mut test_rng(),
        )
        .expect("default weights are usable");

        assert_eq!(population.len(), 4, "one individual per population_size");
        for genome in &population {
            assert_eq!(genome.genes.len(), 16, "each genome gets gene_length genes");
        }

        // No base graph passed, so the context gets an empty one sized from
        // config — the default an unseeded run starts from.
        assert_eq!(context.base_graph.num_nodes, 8);
        assert_eq!(
            context.base_graph.get_edge_list().len(),
            0,
            "no edges to start",
        );
    }

    #[test]
    fn a_set_base_graph_is_what_the_edge_edit_population_expresses_against() {
        let mut evolver = evolver_with_genome("[genome]\ntype = \"edge_edit\"\ngene_length = 16\n");
        let seeded = vec![(0, 1, 2), (3, 4, 1)];
        Python::attach(|py| evolver.set_base_graph(py, 8, seeded.clone(), 0))
            .expect("a graph matching the config is accepted");

        let (context, _) = edge_edit_start(
            &evolver.config,
            edge_edit_config(&evolver.config),
            evolver.base_graph.as_ref(),
            &mut test_rng(),
        )
        .expect("default weights are usable");

        assert_eq!(
            context.base_graph.get_edge_list(),
            seeded,
            "the population expresses against what was set, not an empty graph",
        );
    }

    #[test]
    fn a_seeded_population_keeps_one_individual_that_expresses_the_base_graph() {
        // Without this, generation 0 holds nothing near the graph the caller
        // supplied, and a run can return something worse than its own input.
        let mut evolver = evolver_with_genome("[genome]\ntype = \"edge_edit\"\ngene_length = 16\n");
        let seeded = vec![(0, 1, 1), (3, 4, 1)];
        Python::attach(|py| evolver.set_base_graph(py, 8, seeded.clone(), 0))
            .expect("a graph matching the config is accepted");

        let (context, population) = edge_edit_start(
            &evolver.config,
            edge_edit_config(&evolver.config),
            evolver.base_graph.as_ref(),
            &mut test_rng(),
        )
        .expect("default weights are usable");

        // Expressed, not just inspected: the guarantee is about the graph this
        // individual produces, not about the bytes in its genes.
        let expressed = population[0].express(&context);
        assert_eq!(expressed.get_edge_list(), seeded);
        assert_eq!(
            population.len(),
            evolver.config.population_size,
            "the identity replaces a slot rather than adding one",
        );
    }

    #[test]
    fn an_unseeded_population_gets_no_identity_individual() {
        // The identity is only a floor when there is something to hold up: with
        // no base graph it would just be an empty graph taking a slot.
        let evolver = evolver_with_genome("[genome]\ntype = \"edge_edit\"\ngene_length = 16\n");

        let (_, population) = edge_edit_start(
            &evolver.config,
            edge_edit_config(&evolver.config),
            None,
            &mut test_rng(),
        )
        .expect("default weights are usable");

        let all_null = population[0]
            .genes
            .iter()
            .all(|gene| *gene == IDENTITY_GENE);
        assert!(!all_null, "an unseeded run has no identity individual");
    }

    #[test]
    fn a_base_graph_whose_node_count_disagrees_with_the_config_is_rejected() {
        let mut evolver = evolver_with_genome("[genome]\ntype = \"edge_edit\"\ngene_length = 16\n");

        let err = Python::attach(|py| evolver.set_base_graph(py, 9, vec![(0, 1, 1)], 0))
            .expect_err("9 nodes against a network_size of 8 must be rejected");

        let message = err.to_string();
        assert!(message.contains('9'), "names the graph's size: {message}");
        assert!(message.contains('8'), "names network_size: {message}");
        assert!(evolver.base_graph.is_none(), "nothing stored on rejection");
    }

    #[test]
    fn a_base_graph_edge_above_the_configs_cap_is_rejected_rather_than_clamped() {
        // `Graph::set_edge` clamps an over-cap weight instead of refusing it, so
        // a graph built under a wider cap would otherwise be narrowed silently
        // and evolved against — the caller never seeing a different graph from
        // the one they handed in.
        let mut evolver =
            evolver_with_genome_and_cap("[genome]\ntype = \"edge_edit\"\ngene_length = 16\n", 1);

        let err = Python::attach(|py| evolver.set_base_graph(py, 8, vec![(0, 1, 1), (2, 3, 3)], 0))
            .expect_err("multiplicity 3 against a cap of 1 must be rejected");

        let message = err.to_string();
        assert!(
            message.contains("(2, 3)"),
            "names the offending edge: {message}",
        );
        assert!(
            message.contains("multiplicity 3"),
            "names the offending value, not just the edge: {message}",
        );
        assert!(
            message.contains('1'),
            "names the cap it exceeded: {message}"
        );
        assert!(evolver.base_graph.is_none(), "nothing stored on rejection");
    }

    #[test]
    fn a_base_graph_edge_naming_a_node_outside_the_network_is_rejected() {
        // The node-count check compares one number against another and never
        // looks at the edges, so a caller taking `num_nodes` from their config
        // rather than their data passes it while every out-of-range edge is
        // dropped by `Graph::set_edge` without a word.
        let mut evolver = evolver_with_genome("[genome]\ntype = \"edge_edit\"\ngene_length = 16\n");

        let err = Python::attach(|py| evolver.set_base_graph(py, 8, vec![(0, 1, 1), (2, 9, 1)], 0))
            .expect_err("node 9 in an 8-node network must be rejected");

        let message = err.to_string();
        assert!(
            message.contains("(2, 9)"),
            "names the offending edge: {message}",
        );
        // The range is inclusive and stated in the caller's own numbering, the
        // same way the file loader states it. Asserting the whole range rather
        // than one digit of it: `contains('8')` passed on the old exclusive
        // `0..8` wording by accident, and would pass again on any message that
        // merely mentioned the network size.
        assert!(
            message.contains("0..=7"),
            "names the range it fell outside: {message}",
        );
        assert!(evolver.base_graph.is_none(), "nothing stored on rejection");
    }

    #[test]
    fn a_base_graph_self_loop_is_rejected_rather_than_dropped() {
        // This graph has no representation for a self-loop, and one in caller
        // data almost always means the indices are wrong — 1-indexed edge lists
        // being the common case, which also lands every survivor on the wrong
        // vertex. Reported rather than absorbed.
        let mut evolver = evolver_with_genome("[genome]\ntype = \"edge_edit\"\ngene_length = 16\n");

        let err = Python::attach(|py| evolver.set_base_graph(py, 8, vec![(0, 1, 1), (3, 3, 1)], 0))
            .expect_err("a self-loop must be rejected");

        let message = err.to_string();
        assert!(
            message.contains("(3, 3)"),
            "names the offending edge: {message}",
        );
        assert!(
            message.contains("self-loop"),
            "says what is wrong with it: {message}",
        );
        assert!(evolver.base_graph.is_none(), "nothing stored on rejection");
    }

    #[test]
    fn a_base_graph_is_rejected_on_an_sda_configured_evolver() {
        // The SDA genome generates its graph rather than editing one, so a
        // stored base would never be read. Accepting it looks, from Python,
        // exactly like having seeded the run.
        let mut evolver =
            evolver_with_genome("[genome]\ntype = \"sda\"\nnum_states = 5\nmax_resp_len = 3\n");

        let err = Python::attach(|py| evolver.set_base_graph(py, 8, vec![(0, 1, 1)], 0))
            .expect_err("an SDA run has no base graph to seed");

        let message = err.to_string();
        assert!(
            message.contains("edge_edit"),
            "points at what would work: {message}",
        );
        assert!(evolver.base_graph.is_none(), "nothing stored on rejection");
    }

    #[test]
    fn the_sda_start_derives_its_alphabet_from_the_edge_multiplicity_cap() {
        // §3.2: `num_chars` is never configured, it is `max_edge_multiplicity + 1`
        // — the cap is 3 in this fixture, so the alphabet is 4. Configuring it
        // separately would let the automaton emit a character that
        // `Graph::set_edge` then clamps, losing structure with nothing reported.
        let evolver = evolver_with_genome(
            "[genome]\ntype = \"sda\"\nnum_states = 6\nmax_resp_len = 3\ninit_state = 2\n",
        );

        let GenomeConfig::Sda(sda) = &evolver.config.genome else {
            panic!("the fixture above configures an sda genome");
        };

        let (context, population) =
            sda_start(&evolver.config, sda, &mut test_rng()).expect("valid SDA dimensions");

        assert_eq!(population.len(), 4);
        assert_eq!(context.num_nodes, 8);
        assert_eq!(context.init_state, 2);
        assert_eq!(
            context.max_edge_multiplicity, 3,
            "the context carries the same cap the alphabet was derived from",
        );

        // Every genome must be expressible against that context — this is what
        // would fail if the two disagreed about the cap.
        for genome in &population {
            let graph = genome.express(&context);
            assert_eq!(graph.num_nodes, 8);
        }
    }

    #[test]
    fn an_out_of_range_init_state_is_reported_rather_than_panicking() {
        // `SdaGenome::run` indexes its response table with `init_state`, so this
        // panics during expression if it gets through — and a panic crossing the
        // FFI reaches the user as an opaque `PanicException` (§7). `run` is the
        // path that matters, so the check is exercised through it.
        let mut evolver = evolver_with_genome(
            "[genome]\ntype = \"sda\"\nnum_states = 4\nmax_resp_len = 3\ninit_state = 9\n",
        );

        let err = Python::attach(|py| evolver.run(py, 1, 1, None))
            .expect_err("init_state 9 with num_states 4 must be rejected");

        let message = err.to_string();
        assert!(message.contains("init_state"), "names the field: {message}");
        assert!(message.contains('4'), "names num_states: {message}");
    }

    #[test]
    fn asking_for_more_replicates_does_not_move_the_earlier_ones() {
        // The property the stream exists for: run `i`'s seed is a function of
        // the master and `i` alone, never of how many runs were requested. A
        // user who collects 30 replicates and later wants 50 keeps the 30.
        let thirty = replicate_seeds(20260813, 30);
        let fifty = replicate_seeds(20260813, 50);

        assert_eq!(thirty.len(), 30, "one seed per requested run");
        assert_eq!(fifty.len(), 50, "one seed per requested run");
        assert_eq!(
            fifty[..30],
            thirty[..],
            "the first 30 of a 50-run request must be the 30-run request, exactly",
        );
    }

    #[test]
    fn replicate_seeds_do_not_collide_across_nearby_masters() {
        // `master ^ i` is the anti-pattern this rules out: under it, master 4
        // run 1 and master 5 run 0 are the same run. Two adjacent masters must
        // share no seed at any index.
        let first = replicate_seeds(4, 8);
        let second = replicate_seeds(5, 8);

        for (index, seed) in first.iter().enumerate() {
            assert!(
                !second.contains(seed),
                "seed {seed} from master 4 (run {index}) also appears under master 5",
            );
        }
    }

    #[test]
    fn each_replicate_gets_a_distinct_seed() {
        // Two replicates sharing a seed would be the same run twice, which is
        // not a replicate — it reports as agreement between independent runs.
        let seeds = replicate_seeds(7, 32);

        for (index, seed) in seeds.iter().enumerate() {
            assert!(
                !seeds[index + 1..].contains(seed),
                "seed at run {index} repeats later in the same request",
            );
        }
    }

    #[test]
    fn a_zero_run_request_yields_no_seeds() {
        // The degenerate end of `min(max_cores, n_runs)`: nothing to run, and
        // nothing drawn, rather than one seed drawn and discarded.
        assert!(replicate_seeds(1, 0).is_empty());
    }

    #[test]
    fn the_same_seed_builds_the_same_starting_population() {
        // The whole point of `run(seed)`: one master seed reproduces everything.
        let evolver = evolver_with_genome("[genome]\ntype = \"edge_edit\"\ngene_length = 12\n");
        let genome = edge_edit_config(&evolver.config);

        let (_, first) = edge_edit_start(
            &evolver.config,
            genome,
            None,
            &mut ChaCha8Rng::seed_from_u64(5),
        )
        .expect("first build");
        let (_, second) = edge_edit_start(
            &evolver.config,
            genome,
            None,
            &mut ChaCha8Rng::seed_from_u64(5),
        )
        .expect("second build");
        let (_, different) = edge_edit_start(
            &evolver.config,
            genome,
            None,
            &mut ChaCha8Rng::seed_from_u64(6),
        )
        .expect("third build");

        for (a, b) in first.iter().zip(second.iter()) {
            assert_eq!(a.genes, b.genes, "same seed, same population");
        }
        assert_ne!(
            first[0].genes, different[0].genes,
            "a different seed must give a different population",
        );
    }

    #[test]
    fn population_construction_does_not_replay_the_evolvers_stream() {
        // `run` draws the population and then draws the evolver's seed from the
        // same generator. Handing the evolver the master `seed` instead would
        // have it replay the exact values that just built the population —
        // correlating initial genomes with the first mutations, which looks like
        // nothing at all in the output.
        let mut rng = ChaCha8Rng::seed_from_u64(3);
        let evolver = evolver_with_genome("[genome]\ntype = \"edge_edit\"\ngene_length = 8\n");

        let (_, population) = edge_edit_start(
            &evolver.config,
            edge_edit_config(&evolver.config),
            None,
            &mut rng,
        )
        .expect("build");
        let evolution_seed = rng.random::<u64>();

        // The evolver's seed must not be the master seed, and must not be a
        // value the population already consumed.
        assert_ne!(evolution_seed, 3, "not the master seed");
        let mut fresh = ChaCha8Rng::seed_from_u64(3);
        let first_draw = fresh.random::<u64>();
        assert_ne!(
            evolution_seed, first_draw,
            "the population consumed the stream ahead of the evolver's seed",
        );
        assert_eq!(population.len(), 4);
    }

    #[test]
    fn a_python_objective_with_no_callable_fails_before_anything_is_built() {
        // `run` calls `objective` first precisely so this is reported before a
        // population exists, rather than at the first scoring call.
        let evolver = evolver_with(PYTHON_FITNESS);

        let err = evolver
            .objective(1, None)
            .map(|_| ())
            .expect_err("a python config with no callable cannot build an objective");

        assert!(
            err.to_string().contains("set_fitness_function"),
            "says what to do about it: {err}",
        );
    }

    /// A whole runnable config: `evolution_block` × `genome_block`, small enough
    /// that four end-to-end runs stay fast. Two epidemics per evaluation, not the
    /// realistic thirty — this asserts the wiring, not the search quality.
    ///
    /// `tournament_size = 4` is not arbitrary: `Config::validate` requires at
    /// least 4 under steady-state (`config.rs`, `validate_evolution_and_selection`),
    /// which needs two parents *and* two individuals to replace. Generational has
    /// no such floor, but one fixture serving both keeps the four combinations
    /// comparable.
    fn runnable(evolution_block: &str, genome_block: &str) -> Config {
        runnable_with_fitness(
            evolution_block,
            genome_block,
            "[fitness]\ntype = \"epi_spread\"\ninfection_rate = 0.3\nnum_epidemics = 2\n",
        )
    }

    /// The same, with the `[fitness]` block chosen — the replicate tests need a
    /// `python` one to reach the sequential arm, which no SIR block can.
    fn runnable_with_fitness(
        evolution_block: &str,
        genome_block: &str,
        fitness_block: &str,
    ) -> Config {
        let text = format!(
            "population_size = 6\n\
             network_size = 8\n\
             max_edge_multiplicity = 2\n\
             crossover_rate = 0.8\n\
             mutation_rate = 0.5\n\
             \n\
             {evolution_block}\n\
             [scope]\n\
             type = \"global\"\n\
             \n\
             [selection]\n\
             type = \"tournament\"\n\
             tournament_size = 4\n\
             \n\
             {genome_block}\n\
             {fitness_block}"
        );
        let config = Config::from_toml_str(&text).expect("the runnable config parses");
        config.validate().expect("the runnable config validates");
        config
    }

    /// A runnable `edge_edit` config whose expression step is a guaranteed
    /// no-op: `mutation_rate` and `crossover_rate` both zero, and
    /// `operation_weights` gives `null` the only non-zero weight. A population
    /// expressed against this never edits the base graph it started from, so
    /// a seeded run's output can be checked by exact equality rather than
    /// overlap — the same recipe as
    /// `a_set_base_graph_is_what_the_edge_edit_population_expresses_against`,
    /// lifted to the `run_replicates` level.
    fn no_op_runnable(evolution_block: &str) -> Config {
        let fitness_block = "[fitness]\ntype = \"epi_spread\"\ninfection_rate = 0.3\n\
             num_epidemics = 2\n";
        let text = format!(
            "population_size = 6\n\
             network_size = 8\n\
             max_edge_multiplicity = 2\n\
             crossover_rate = 0.0\n\
             mutation_rate = 0.0\n\
             \n\
             {evolution_block}\n\
             [scope]\n\
             type = \"global\"\n\
             \n\
             [selection]\n\
             type = \"tournament\"\n\
             tournament_size = 4\n\
             \n\
             [genome]\n\
             type = \"edge_edit\"\n\
             gene_length = 12\n\
             \n\
             [genome.operation_weights]\n\
             null = 1.0\n\
             toggle = 0.0\n\
             hop = 0.0\n\
             add = 0.0\n\
             delete = 0.0\n\
             swap = 0.0\n\
             local_toggle = 0.0\n\
             local_add = 0.0\n\
             local_delete = 0.0\n\
             \n\
             {fitness_block}"
        );
        let config = Config::from_toml_str(&text).expect("the no-op config parses");
        config.validate().expect("the no-op config validates");
        config
    }

    const GENERATIONAL: &str = "[evolution]\ntype = \"generational\"\nnum_generations = 3\n";
    const STEADY_STATE: &str = "[evolution]\ntype = \"steady_state\"\nnum_mating_events = 12\n";
    const EDGE_EDIT: &str = "[genome]\ntype = \"edge_edit\"\ngene_length = 12\n";
    const SDA: &str = "[genome]\ntype = \"sda\"\nnum_states = 5\nmax_resp_len = 3\n";

    #[test]
    fn every_strategy_by_genome_combination_runs_end_to_end() {
        // #26's verify-by: all four arms of the dispatch complete a real run.
        // Before this, three of the four had never been executed at all — the
        // types line up whether or not the arms are wired to the right evolver.
        let combinations = [
            ("generational × edge_edit", GENERATIONAL, EDGE_EDIT),
            ("generational × sda", GENERATIONAL, SDA),
            ("steady_state × edge_edit", STEADY_STATE, EDGE_EDIT),
            ("steady_state × sda", STEADY_STATE, SDA),
        ];

        for (name, evolution, genome) in combinations {
            let config = runnable(evolution, genome);
            let objective: Box<dyn Fitness> =
                Box::new(EpiSpread::new(sir_sample_params(sir_of(&config)), 1));

            let outcome = evolve(&config, &objective, None, 1)
                .unwrap_or_else(|err| panic!("{name} should complete: {err}"));

            // `epi_spread` counts ever-infected and patient zero always counts,
            // so any real run scores at least 1 — and it is **maximized**, so a
            // negative value here would mean the engine's orientation leaked out
            // rather than being undone at this boundary.
            assert!(
                outcome.best_fitness >= 1.0,
                "{name}: fitness should be a positive spread in the objective's own \
                 units, got {}",
                outcome.best_fitness,
            );
            assert!(
                outcome.best_fitness.is_finite(),
                "{name}: fitness must be finite",
            );

            // Every edge must be inside the configured network and within the cap.
            for &(u, v, weight) in &outcome.best_edges {
                assert!(
                    u < 8 && v < 8,
                    "{name}: edge ({u},{v}) outside network_size 8"
                );
                assert!(
                    (1..=2).contains(&weight),
                    "{name}: weight {weight} outside 1..=max_edge_multiplicity",
                );
            }
        }
    }

    /// One objective per seed, which is what `run_replicates` requires — a
    /// fresh instance each, never one shared, because an SIR objective owns a
    /// per-run epidemic counter.
    fn objectives_for(config: &Config, seeds: &[u64]) -> Vec<Box<dyn Fitness>> {
        let mut objectives: Vec<Box<dyn Fitness>> = Vec::with_capacity(seeds.len());
        for &seed in seeds {
            objectives.push(Box::new(EpiSpread::new(
                sir_sample_params(sir_of(config)),
                seed,
            )));
        }
        objectives
    }

    #[test]
    fn replicates_come_back_in_run_order_not_completion_order() {
        // The ordering guarantee the whole feature rests on: the same master
        // seed must give the same output ordering on every machine, however the
        // runs interleave. Checked by running the same seeds concurrently and
        // sequentially and requiring the two sequences to match element for
        // element — a completion-ordered collect would diverge under load.
        let config = runnable(GENERATIONAL, EDGE_EDIT);
        let seeds = replicate_seeds(20260813, 6);

        let concurrent = run_replicates(
            &config,
            &objectives_for(&config, &seeds),
            None,
            &seeds,
            Some(4),
        )
        .expect("concurrent replicates complete");
        let serial = run_replicates(
            &config,
            &objectives_for(&config, &seeds),
            None,
            &seeds,
            Some(1),
        )
        .expect("sequential replicates complete");

        assert_eq!(concurrent.len(), 6, "one outcome per seed");
        for (index, (parallel, sequential)) in concurrent.iter().zip(&serial).enumerate() {
            assert_eq!(
                parallel.best_fitness, sequential.best_fitness,
                "run {index} differs between max_cores=4 and max_cores=1",
            );
            assert_eq!(
                parallel.best_edges, sequential.best_edges,
                "run {index}'s graph differs between max_cores=4 and max_cores=1",
            );
        }
    }

    /// Every replicate must carry the seeded base graph forward exactly,
    /// unedited, under a given `max_cores`. Shared by the rayon-arm and
    /// sequential-arm tests below so the two only differ in the cap.
    fn assert_seeded_base_graph_reaches_every_replicate(max_cores: Option<usize>) {
        let config = no_op_runnable(GENERATIONAL);
        let seeded = vec![(0, 1, 2), (3, 4, 1)];
        let base_graph = {
            let mut graph = Graph::new(8, 2);
            graph.set_edges(&seeded);
            graph
        };
        let seeds = replicate_seeds(20260816, 4);

        let outcomes = run_replicates(
            &config,
            &objectives_for(&config, &seeds),
            Some(&base_graph),
            &seeds,
            max_cores,
        )
        .expect("seeded replicates complete");

        assert_eq!(outcomes.len(), 4, "one outcome per seed");
        for (index, outcome) in outcomes.iter().enumerate() {
            assert_eq!(
                outcome.best_edges, seeded,
                "replicate {index} does not carry the seeded base graph forward \
                 exactly, under max_cores={max_cores:?}",
            );
        }
    }

    #[test]
    fn a_seeded_base_graph_reaches_every_replicate_on_the_rayon_arm() {
        // #84: #72 (`set_base_graph`) and #83 (replicate runs) were each
        // tested against the other's absence. This is their intersection —
        // a seeded run through `n_runs > 1` on the concurrent arm, which
        // `evolve` reaches once per `seeds.par_iter()` entry.
        assert_seeded_base_graph_reaches_every_replicate(Some(4));
    }

    #[test]
    fn a_seeded_base_graph_reaches_every_replicate_on_the_sequential_arm() {
        // The other half of #84: the same seeded config, forced onto the
        // sequential loop at the top of `run_replicates` by pinning
        // `max_cores` to 1 rather than by a python fitness type, so this
        // exercises the same `for` loop
        // `a_python_objective_runs_its_replicates_through_the_sequential_arm`
        // reaches, but with a native objective and a base graph to check.
        assert_seeded_base_graph_reaches_every_replicate(Some(1));
    }

    #[test]
    fn an_unseeded_replicate_run_expresses_zero_edges_control() {
        // Control for the two tests above: without a base graph, the same
        // no-op config expresses nothing. If this failed, the exact-equality
        // assertions above would be vacuous — passing because the no-op
        // config never grows any edges at all, seeded or not.
        let config = no_op_runnable(GENERATIONAL);
        let seeds = replicate_seeds(20260816, 4);

        for max_cores in [Some(4), Some(1)] {
            let outcomes = run_replicates(
                &config,
                &objectives_for(&config, &seeds),
                None,
                &seeds,
                max_cores,
            )
            .expect("unseeded replicates complete");

            for (index, outcome) in outcomes.iter().enumerate() {
                assert!(
                    outcome.best_edges.is_empty(),
                    "replicate {index} should express no edges without a base \
                     graph, under max_cores={max_cores:?}, got {:?}",
                    outcome.best_edges,
                );
            }
        }
    }

    #[test]
    fn a_python_objective_runs_its_replicates_through_the_sequential_arm() {
        // The python arm end to end, with a real registered callable — not a
        // native config standing in for one. A timing assertion would prove
        // nothing here, so what this checks is that the branch completes and
        // scores through the callable at all: under the parallel arm these
        // would be n threads contending for one GIL.
        Python::attach(|py| {
            let mut evolver = GraphEvolver {
                config: runnable_with_fitness(GENERATIONAL, EDGE_EDIT, PYTHON_FITNESS),
                fitness_function: None,
                base_graph: None,
                min_node_index: None,
                struct_match_reference: Default::default(),
                config_toml: String::new(),
            };
            let callable = py
                .eval(
                    c"lambda batch: [float(len(edges)) for (n, edges) in batch]",
                    None,
                    None,
                )
                .expect("the lambda compiles");
            evolver
                .set_fitness_function(&callable, "maximize")
                .expect("a python config accepts a callable");

            assert!(
                matches!(evolver.config.fitness, FitnessConfig::Python),
                "fixture sanity: this must be the python arm, not a native one",
            );

            let seeds = replicate_seeds(11, 3);
            let mut objectives = Vec::with_capacity(seeds.len());
            for &seed in &seeds {
                objectives.push(
                    evolver
                        .objective(seed, Some(py))
                        .expect("objective per run"),
                );
            }

            // `max_cores` is deliberately generous: under the python arm it is
            // moot, and passing it proves the cap does not drag the config onto
            // the parallel path.
            let outcomes = run_replicates(&evolver.config, &objectives, None, &seeds, Some(8))
                .expect("python replicates complete sequentially");

            assert_eq!(outcomes.len(), 3, "one outcome per seed");
            for (index, outcome) in outcomes.iter().enumerate() {
                assert!(
                    outcome.best_fitness.is_finite(),
                    "run {index} scored through the callable",
                );
            }
        });
    }

    #[test]
    fn a_native_objective_runs_with_the_core_cap_unset() {
        // The other half of the gate: unset `max_cores` means all available,
        // which must still complete rather than building a zero-width pool.
        let config = runnable(GENERATIONAL, EDGE_EDIT);
        let seeds = replicate_seeds(11, 3);

        let outcomes = run_replicates(
            &config,
            &objectives_for(&config, &seeds),
            None,
            &seeds,
            None,
        )
        .expect("replicates complete with max_cores unset");

        assert_eq!(outcomes.len(), 3, "one outcome per seed, cap unset");
    }

    #[test]
    fn a_zero_replicate_request_yields_no_outcomes() {
        // Named for what is actually asserted. The early return in
        // `run_replicates` also avoids constructing a pool for zero work —
        // rayon reads `num_threads(0)` as "pick a default", so an unguarded
        // empty request would build a full-width pool to do nothing — but that
        // is not observable from here and this test does not claim it.
        let config = runnable(GENERATIONAL, EDGE_EDIT);

        let outcomes = run_replicates(&config, &[], None, &[], Some(8))
            .expect("an empty request is not an error");

        assert!(outcomes.is_empty(), "no seeds means no outcomes");
    }

    #[test]
    fn concurrency_is_capped_by_the_run_count_and_the_caller() {
        // `min(max_cores, n_runs)`, and never zero. Threads beyond the replicate
        // count have nothing to do, and zero means "default" to rayon.
        assert_eq!(effective_concurrency(Some(8), 3), 3, "capped by run count");
        assert_eq!(
            effective_concurrency(Some(2), 30),
            2,
            "capped by the caller"
        );
        assert_eq!(effective_concurrency(Some(1), 30), 1, "fully sequential");
        assert_eq!(effective_concurrency(Some(0), 4), 1, "never zero threads");
        assert_eq!(effective_concurrency(None, 1), 1, "unset, but one run");
        assert!(
            effective_concurrency(None, 64) >= 1,
            "unset means all available, whatever this machine has",
        );
    }

    #[test]
    fn the_erased_history_comes_out_in_the_objectives_own_units() {
        // `epi_spread` is maximized, so every row is negative inside the engine
        // and must be positive here. The failure this catches is silent in the
        // worst way: an unconverted log plots upside down while every value in
        // it still looks like a plausible epidemic size.
        let config = runnable(GENERATIONAL, EDGE_EDIT);
        let objective: Box<dyn Fitness> =
            Box::new(EpiSpread::new(sir_sample_params(sir_of(&config)), 6));

        let outcome = evolve(&config, &objective, None, 6).expect("run completes");

        // Row 0 is the starting population, then one per generation.
        assert_eq!(outcome.history.len(), 4, "num_generations + 1 rows");

        for row in &outcome.history {
            assert!(
                row.best_fitness >= 1.0,
                "iteration {}: best fitness should be a positive spread, got {}",
                row.iteration,
                row.best_fitness,
            );
            assert!(
                row.mean_fitness >= 1.0,
                "iteration {}: mean fitness should be a positive spread, got {}",
                row.iteration,
                row.mean_fitness,
            );
            // Left alone by `erase` on purpose — a spread is identical under
            // negation, so orienting it would make it negative here.
            assert!(
                row.std_dev >= 0.0,
                "iteration {}: a deviation is never negative, got {}",
                row.iteration,
                row.std_dev,
            );
            // Same reasoning as std_dev: ci_95 is a spread, so erase must leave
            // it alone rather than orient it.
            assert!(
                row.ci_95 >= 0.0,
                "iteration {}: ci_95 is never negative, got {}",
                row.iteration,
                row.ci_95,
            );
        }

        // The **last** row must agree with the headline number, because both are
        // read from the same final scoring pass (`generational.rs:208`) — so this
        // pins that the log and `best_fitness` went through the same conversion.
        //
        // Deliberately not the best row of the whole log: under a stochastic
        // objective an earlier generation can out-score the final one, and the
        // reported best is the best of the *final* population, not best-ever.
        let last = &outcome.history[outcome.history.len() - 1];
        assert_eq!(
            last.best_fitness, outcome.best_fitness,
            "the final log row and the reported best fitness must be the same number",
        );

        assert!(
            !outcome.best_genome_repr.is_empty(),
            "the winning genome's printed form must survive erasure",
        );
    }

    /// The `[fitness]` block of a config known to be `epi_spread`.
    fn sir_of(config: &Config) -> &config::SirParams {
        match &config.fitness {
            FitnessConfig::EpiSpread { sir } => sir,
            other => panic!("expected epi_spread, got {other:?}"),
        }
    }

    #[test]
    fn one_seed_reproduces_a_whole_run_and_another_changes_it() {
        // The guarantee the single `seed` argument exists to give (§7, §8.1).
        // It covers the population, the evolution and the epidemics at once,
        // since all three derive from it.
        let config = runnable(GENERATIONAL, EDGE_EDIT);
        let run = |seed: u64| {
            let objective: Box<dyn Fitness> =
                Box::new(EpiSpread::new(sir_sample_params(sir_of(&config)), seed));
            evolve(&config, &objective, None, seed).expect("run completes")
        };

        let first = run(4);
        let again = run(4);
        let other = run(5);

        assert_eq!(
            first.best_fitness, again.best_fitness,
            "same seed, same score"
        );
        assert_eq!(
            first.best_edges, again.best_edges,
            "same seed, same network"
        );
        assert!(
            other.best_fitness != first.best_fitness || other.best_edges != first.best_edges,
            "a different seed should not reproduce the same run",
        );
    }

    #[test]
    fn two_struct_match_replicates_at_one_seed_agree() {
        // The second test #99 asks every new objective for. It is worth having
        // here specifically because this objective reads from disk: a loader
        // that let filesystem order into the reference set -- which
        // `load_edge_folder` sorts precisely to prevent -- would give two
        // replicates different targets, and the run would still look fine.
        let evolver = evolver_with(&struct_match_block("replicates"));
        let run = |seed: u64| {
            let objective = evolver
                .objective(seed, None)
                .expect("an objective per replicate");
            evolve(&evolver.config, &objective, None, seed).expect("run completes")
        };

        let first = run(4);
        let again = run(4);
        let other = run(5);

        assert_eq!(
            first.best_fitness, again.best_fitness,
            "same seed, same score"
        );
        assert_eq!(
            first.best_edges, again.best_edges,
            "same seed, same network"
        );
        assert!(
            first.best_fitness.is_finite(),
            "a structural score must never be non-finite"
        );
        assert!(
            other.best_fitness != first.best_fitness || other.best_edges != first.best_edges,
            "a different seed should not reproduce the same run"
        );
    }

    #[test]
    fn steady_state_and_generational_do_not_produce_the_same_run() {
        // Guards against both arms of `run_strategy` reaching the same evolver —
        // which would compile, run, and return plausible numbers.
        let generational = runnable(GENERATIONAL, EDGE_EDIT);
        let steady = runnable(STEADY_STATE, EDGE_EDIT);

        let first: Box<dyn Fitness> =
            Box::new(EpiSpread::new(sir_sample_params(sir_of(&generational)), 2));
        let second: Box<dyn Fitness> =
            Box::new(EpiSpread::new(sir_sample_params(sir_of(&steady)), 2));

        let a = evolve(&generational, &first, None, 2).expect("generational runs");
        let b = evolve(&steady, &second, None, 2).expect("steady-state runs");

        assert_ne!(
            a.best_edges, b.best_edges,
            "the two strategies should not be producing identical runs",
        );
    }

    #[test]
    fn run_returns_a_complete_result_object() {
        let config_toml = "population_size = 6\n\
             network_size = 8\n\
             max_edge_multiplicity = 2\n\
             crossover_rate = 0.8\n\
             mutation_rate = 0.5\n\
             \n\
             [evolution]\n\
             type = \"generational\"\n\
             num_generations = 3\n\
             \n\
             [scope]\n\
             type = \"global\"\n\
             \n\
             [selection]\n\
             type = \"tournament\"\n\
             tournament_size = 4\n\
             \n\
             [genome]\n\
             type = \"edge_edit\"\n\
             gene_length = 12\n\
             \n\
             [fitness]\n\
             type = \"epi_spread\"\n\
             infection_rate = 0.3\n\
             num_epidemics = 2\n"
            .to_string();
        // Through the Python entry point, so this also exercises the GIL release.
        let mut evolver = GraphEvolver {
            config: Config::from_toml_str(&config_toml).expect("the fixture parses"),
            fitness_function: None,
            base_graph: None,
            min_node_index: None,
            struct_match_reference: Default::default(),
            config_toml: config_toml.clone(),
        };

        let [result] = <[_; 1]>::try_from(
            Python::attach(|py| evolver.run(py, 8, 1, None)).expect("a full config run completes"),
        )
        .expect("one run returns exactly one result");

        assert!(
            result.best_fitness >= 1.0,
            "in the objective's own units, got {}",
            result.best_fitness,
        );
        assert!(
            !result.best_genome_repr.is_empty(),
            "the winning genome's printed form comes back",
        );
        assert_eq!(result.history.len(), 4, "num_generations + 1 log rows");
        for &(u, v, _) in &result.best_edges {
            assert!(u < 8 && v < 8);
        }

        // Task 4's verify-by: seed, run_index and the config TOML reach the
        // result and the TOML round-trips.
        assert_eq!(result.seed, 8, "the seed run was called with");
        assert_eq!(result.run_index, 0, "hard 0 until replicates land (#20)");
        Config::from_toml_str(&result.config_toml).expect("the provenance TOML round-trips");
        assert_eq!(result.config_toml, config_toml);
    }

    /// An evolver on the small runnable config, for the replicate tests.
    fn replicate_evolver() -> GraphEvolver {
        GraphEvolver {
            config: runnable(GENERATIONAL, EDGE_EDIT),
            fitness_function: None,
            base_graph: None,
            min_node_index: None,
            struct_match_reference: Default::default(),
            config_toml: String::new(),
        }
    }

    #[test]
    fn the_core_cap_changes_the_speed_and_never_the_answer() {
        // The issue's own verify-by, and the strongest isolation check available
        // at this level: if replicates shared an objective, its per-run epidemic
        // counter would be advanced by whichever run got there first, so four
        // concurrent replicates would not reproduce four sequential ones. Equal
        // results across the two caps is that cross-talk not happening.
        let mut evolver = replicate_evolver();

        let sequential = Python::attach(|py| evolver.run(py, 20260813, 4, Some(1)))
            .expect("four replicates, one at a time");
        let concurrent = Python::attach(|py| evolver.run(py, 20260813, 4, Some(8)))
            .expect("four replicates, up to eight at a time");

        assert_eq!(sequential.len(), 4, "one result per requested run");
        assert_eq!(concurrent.len(), 4, "one result per requested run");
        for (index, (serial, parallel)) in sequential.iter().zip(&concurrent).enumerate() {
            assert_eq!(
                serial.best_fitness, parallel.best_fitness,
                "run {index}'s fitness depends on the core cap",
            );
            assert_eq!(
                serial.best_edges, parallel.best_edges,
                "run {index}'s graph depends on the core cap",
            );
            assert_eq!(
                serial.best_genome_repr, parallel.best_genome_repr,
                "run {index}'s genome depends on the core cap",
            );
            assert_eq!(serial.run_index, index, "results arrive in run order");
        }
    }

    #[test]
    fn extending_a_request_reproduces_the_replicates_already_collected() {
        // The other half of the issue's verify-by, through the public call
        // rather than the seed helper: a user who has collected three
        // replicates and wants five keeps the three they had.
        let mut evolver = replicate_evolver();

        let three = Python::attach(|py| evolver.run(py, 99, 3, Some(2))).expect("three replicates");
        let five = Python::attach(|py| evolver.run(py, 99, 5, Some(2))).expect("five replicates");

        assert_eq!(five.len(), 5, "the larger request runs all five");
        for (index, (small, large)) in three.iter().zip(&five).enumerate() {
            assert_eq!(
                small.best_fitness, large.best_fitness,
                "run {index} moved when more runs were requested",
            );
            assert_eq!(
                small.best_edges, large.best_edges,
                "run {index}'s graph moved when more runs were requested",
            );
        }
    }

    #[test]
    fn replicates_are_different_runs_rather_than_the_same_run_repeated() {
        // Guards the failure every reproducibility test above would wave
        // through: if each replicate were handed the master seed instead of its
        // own draw, all n results would be identical and every "same seed, same
        // answer" assertion would still pass.
        let mut evolver = replicate_evolver();

        let results = Python::attach(|py| evolver.run(py, 7, 4, Some(2))).expect("four replicates");

        let first = &results[0];
        let all_identical = results
            .iter()
            .all(|r| r.best_edges == first.best_edges && r.best_fitness == first.best_fitness);
        assert!(
            !all_identical,
            "all four replicates produced the same run — each is not getting its own seed",
        );
    }

    #[test]
    fn every_replicate_carries_the_master_seed_and_its_own_index() {
        // `(seed, run_index)` is the pair that reproduces a replicate, and it is
        // what `save_logs` stamps on every row so concatenated replicate logs
        // can be separated again.
        let mut evolver = replicate_evolver();

        let results =
            Python::attach(|py| evolver.run(py, 4242, 3, None)).expect("three replicates");

        for (index, result) in results.iter().enumerate() {
            assert_eq!(result.seed, 4242, "the master seed, not the per-run draw");
            assert_eq!(result.run_index, index, "0-based position in the request");
        }
    }

    #[test]
    fn two_runs_on_one_evolver_do_not_leak_state() {
        // The reason `run` returns a value instead of caching one: an evolver
        // that held the previous run's result would hand the next run the wrong
        // numbers, and nothing about them would look wrong.
        let mut evolver = GraphEvolver {
            config: runnable(GENERATIONAL, EDGE_EDIT),
            fitness_function: None,
            base_graph: None,
            min_node_index: None,
            struct_match_reference: Default::default(),
            config_toml: String::new(),
        };

        let [first] = <[_; 1]>::try_from(
            Python::attach(|py| evolver.run(py, 4, 1, None)).expect("first run"),
        )
        .expect("one run returns exactly one result");
        let [second] = <[_; 1]>::try_from(
            Python::attach(|py| evolver.run(py, 5, 1, None)).expect("second run"),
        )
        .expect("one run returns exactly one result");
        let [first_again] = <[_; 1]>::try_from(
            Python::attach(|py| evolver.run(py, 4, 1, None)).expect("first run, repeated"),
        )
        .expect("one run returns exactly one result");

        // Seed 4 reproduces exactly, after seed 5 has run through the same
        // evolver — so nothing the second run did survived into the third.
        assert_eq!(first.best_fitness, first_again.best_fitness);
        assert_eq!(first.best_edges, first_again.best_edges);
        assert_eq!(first.best_genome_repr, first_again.best_genome_repr);
        assert_eq!(first.history.len(), first_again.history.len());

        // And the middle run is its own result rather than a copy of the first.
        assert!(
            second.best_fitness != first.best_fitness || second.best_edges != first.best_edges,
            "a different seed should not reproduce the same run",
        );
    }

    #[test]
    fn save_logs_writes_one_row_per_logged_iteration_plus_a_header() {
        // Task 5's verify-by: row count is `num_generations + 1` under
        // generational and `num_mating_events / population_size + 1` under
        // steady-state. `GENERATIONAL` and `STEADY_STATE` are `runnable`'s
        // fixtures: population_size 6, num_generations 3, num_mating_events 12
        // — 4 and 3 rows respectively.
        for (evolution, expected_rows) in [(GENERATIONAL, 4), (STEADY_STATE, 3)] {
            let mut evolver = GraphEvolver {
                config: runnable(evolution, EDGE_EDIT),
                fitness_function: None,
                base_graph: None,
                min_node_index: None,
                struct_match_reference: Default::default(),
                config_toml: String::new(),
            };
            let [result] = <[_; 1]>::try_from(
                Python::attach(|py| evolver.run(py, 3, 1, None))
                    .expect("a full config run completes"),
            )
            .expect("one run returns exactly one result");
            assert_eq!(result.history.len(), expected_rows);

            let path = std::env::temp_dir().join(format!(
                "get_save_logs_test_{}_{expected_rows}.csv",
                std::process::id(),
            ));
            result
                .save_logs(path.to_str().expect("temp path is valid UTF-8"))
                .expect("save_logs writes successfully");

            let contents = std::fs::read_to_string(&path).expect("the file was written");
            std::fs::remove_file(&path).expect("temp file cleans up");

            let lines: Vec<&str> = contents.lines().collect();
            assert_eq!(
                lines[0],
                "iteration,best_fitness,mean_fitness,std_dev,ci_95,seed,run_index",
            );
            assert_eq!(
                lines.len(),
                expected_rows + 1,
                "header plus one row per logged iteration",
            );
            for line in &lines[1..] {
                let fields: Vec<&str> = line.split(',').collect();
                assert_eq!(fields.len(), 7);
                assert_eq!(fields[5], "3", "seed column carries the run's seed");
                assert_eq!(fields[6], "0", "run_index column is the hard 0");
            }
        }
    }

    #[test]
    fn save_results_writes_the_best_individual_and_a_reparseable_config() {
        // Task 6's verify-by: both files exist, and the derived TOML path
        // parses back through the same `Config::from_toml_str` the run itself
        // used — that round-trip is the whole point of the provenance record.
        let config_toml = "population_size = 6\n\
             network_size = 8\n\
             max_edge_multiplicity = 2\n\
             crossover_rate = 0.8\n\
             mutation_rate = 0.5\n\
             \n\
             [evolution]\n\
             type = \"generational\"\n\
             num_generations = 3\n\
             \n\
             [scope]\n\
             type = \"global\"\n\
             \n\
             [selection]\n\
             type = \"tournament\"\n\
             tournament_size = 4\n\
             \n\
             [genome]\n\
             type = \"edge_edit\"\n\
             gene_length = 12\n\
             \n\
             [fitness]\n\
             type = \"epi_spread\"\n\
             infection_rate = 0.3\n\
             num_epidemics = 2\n"
            .to_string();
        let mut evolver = GraphEvolver {
            config: Config::from_toml_str(&config_toml).expect("the fixture parses"),
            fitness_function: None,
            base_graph: None,
            min_node_index: None,
            struct_match_reference: Default::default(),
            config_toml: config_toml.clone(),
        };
        let [result] = <[_; 1]>::try_from(
            Python::attach(|py| evolver.run(py, 7, 1, None)).expect("a full config run completes"),
        )
        .expect("one run returns exactly one result");

        let path =
            std::env::temp_dir().join(format!("get_save_results_test_{}", std::process::id()));
        let path_str = path.to_str().expect("temp path is valid UTF-8");
        result
            .save_results(path_str)
            .expect("save_results writes successfully");

        let contents = std::fs::read_to_string(&path).expect("the results file was written");
        std::fs::remove_file(&path).expect("results temp file cleans up");
        assert!(contents.contains(&format!("best_fitness = {}", result.best_fitness)));
        assert!(contents.contains(&result.best_genome_repr));
        for &(u, v, weight) in &result.best_edges {
            assert!(contents.contains(&format!("{u},{v},{weight}")));
        }

        let toml_path = format!("{path_str}.toml");
        let toml_contents = std::fs::read_to_string(&toml_path).expect("the TOML file was written");
        std::fs::remove_file(&toml_path).expect("TOML temp file cleans up");
        assert_eq!(toml_contents, config_toml);
        Config::from_toml_str(&toml_contents).expect("the provenance TOML round-trips");
    }

    #[test]
    fn a_maximizing_objective_actually_climbs_through_the_dispatch() {
        // The failure this catches is the loudest-consequence, quietest-symptom
        // one in the whole layer: if the direction is lost anywhere between the
        // config and the evolver, the search *minimizes* ever-infected and every
        // number it reports still looks like a plausible spread.
        //
        // `infection_rate` is high here on purpose. At the shipped example's 0.05
        // an outbreak on a sparse graph dies immediately whatever the topology,
        // so every individual scores about the same and there is no gradient to
        // climb — measured 2026-08-11, see `issues.md`. This test needs a signal,
        // not a realistic parameter.
        let evolved = "[evolution]\ntype = \"generational\"\nnum_generations = 25\n";
        let unevolved = "[evolution]\ntype = \"generational\"\nnum_generations = 0\n";

        let spread_after = |evolution: &str| {
            let text = format!(
                "population_size = 20\n\
                 network_size = 20\n\
                 max_edge_multiplicity = 1\n\
                 crossover_rate = 0.9\n\
                 mutation_rate = 0.3\n\
                 \n\
                 {evolution}\n\
                 [scope]\n\
                 type = \"global\"\n\
                 \n\
                 [selection]\n\
                 type = \"tournament\"\n\
                 tournament_size = 4\n\
                 \n\
                 [genome]\n\
                 type = \"edge_edit\"\n\
                 gene_length = 64\n\
                 \n\
                 [fitness]\n\
                 type = \"epi_spread\"\n\
                 infection_rate = 0.6\n\
                 num_epidemics = 5\n"
            );
            let config = Config::from_toml_str(&text).expect("parses");
            config.validate().expect("validates");
            let objective: Box<dyn Fitness> =
                Box::new(EpiSpread::new(sir_sample_params(sir_of(&config)), 3));
            evolve(&config, &objective, None, 3)
                .expect("runs")
                .best_fitness
        };

        let start = spread_after(unevolved);
        let end = spread_after(evolved);

        assert!(
            end > start,
            "25 generations of maximizing ever-infected should beat 0 generations, \
             got {end} against {start} — if this reversed, the objective's direction \
             is being lost between the config and the evolver",
        );
    }
}
