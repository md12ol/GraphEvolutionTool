//! Driving the edge-edit genome as a Rust library, against a shipped objective.
//!
//! Run it with:
//!
//! ```text
//! cargo run -p get --example edge_edit_generational
//! ```
//!
//! One of four programs covering every genome × evolver combination from
//! outside the crate. The others are `library_route.rs` (SDA + generational,
//! and a caller's own objective), `edge_edit_steady_state.rs` and
//! `sda_steady_state.rs`. Together they are the proof that the library route
//! actually reaches what it promises: narrowing any of it would break these
//! programs and nothing else, because nothing inside the crate calls the
//! library route's front door.
//!
//! # Every `config.toml` setting, and its route-3 equivalent
//!
//! The direction most likely to turn up a miss, because the config layer is a
//! private module: anything reachable only *through* a parsed config document
//! is, by construction, unreachable from here. Kept beside a program that
//! compiles so it cannot quietly stop being true.
//!
//! Shared settings, the top level of the document:
//!
//! | `config.toml` | route 3 |
//! |---|---|
//! | `population_size` | the length of the `Vec<G>` handed to `Evolver::new` — no context field carries it, because the population is the authority on its own size |
//! | `network_size` | `SdaContext::num_nodes`, or for edge-edit the `num_nodes` of the base graph you build |
//! | `max_edge_multiplicity` | `SdaContext::max_edge_multiplicity`, or `Graph::new`'s second argument |
//! | `crossover_rate` | `SharedEvolutionContext::crossover_rate` |
//! | `mutation_rate` | `SharedEvolutionContext::mutation_rate` |
//! | `max_mutations` | `SharedEvolutionContext::max_mutations` |
//!
//! The four tagged blocks:
//!
//! | `config.toml` | route 3 |
//! |---|---|
//! | `[evolution] type` | the evolver you construct: `GenerationalEvolver` or `SteadyStateEvolver` |
//! | `[evolution] num_generations`, `elite_count` | `GenerationalContext`'s two fields |
//! | `[evolution] num_mating_events` | `SteadyStateContext`'s one field |
//! | `[selection] type`, `tournament_size` | `Selection::Tournament { tournament_size }` |
//! | `[crossover] type` | `SharedEvolutionContext::crossover`, a `Crossover` variant |
//! | `[genome] type` | the genome type you populate the `Vec` with |
//! | `[genome] gene_length` | the length passed to `EdgeEditGenome::random_with_operators`, or the length of the `Vec<u64>` passed to `new_with_operators` |
//! | `[genome] operation_weights` | `EdgeEditOperationWeights`, compiled by `EdgeEditOperators::new` |
//! | `[genome] mutation` | `EdgeEditContext::mutation` or `SdaContext::mutation` |
//! | `[genome] num_states`, `max_resp_len` | arguments to `SdaGenome::random_with_edge_multiplicity_cap` |
//! | `[genome] init_state` | `SdaContext::init_state` |
//! | `[genome] init_char_mutation_rate`, `transition_vs_response_rate` | the matching `SdaContext` fields |
//!
//! Three things differ rather than map, and all three are deliberate:
//!
//! - **`[fitness]` has no route-3 equivalent at all, by design.** `Evolver::run`
//!   is generic over the objective, so a library caller hands it a concrete
//!   `F: Fitness` and never names a config variant. That is the whole reason a
//!   user-supplied objective is kept out of the config schema: nothing
//!   user-supplied is ever deserialized, so there is nothing new to validate.
//!   All four shipped objectives are constructible directly — `EpiSpread` and
//!   `EpiLength` here and in `edge_edit_steady_state.rs`, `EpiProfMatch` in
//!   `sda_steady_state.rs`, and `StructMatch` through
//!   `ReferenceStatistics::from_graphs`, which no shipped example runs.
//! - **`struct_match`'s `reference_folder` is a config-only convenience, and
//!   route 3 has something more general.** From a document the folder is read
//!   and reduced by a private module; from here you hand
//!   `ReferenceStatistics::from_graphs` any `&[Graph]`, which need never have
//!   been on disk. Nothing is lost, only spelled differently. **No shipped
//!   example uses it**, because it is the one objective that cannot be
//!   demonstrated by running the program: it needs reference data the reader
//!   does not have.
//! - **The base graph is not a config key on either route.** From a config file
//!   it arrives through the evolver's own setter, and here it is the
//!   `EdgeEditContext::base_graph` built below.
//!
//! Nothing else in the schema is unreachable from here.
//!
//! Two things here that `library_route.rs` does not show:
//!
//! - **Edge-edit needs the most assembly of any genome.** The operation mix is
//!   validated once into an [`EdgeEditOperators`] the whole population shares,
//!   and the base graph the edit script runs against is supplied by you.
//! - **A chosen starting population, not only a random one.** `genes` is
//!   readable, and `new_with_operators` is what feeds a gene sequence back —
//!   a recorded edit script, a previous run's winner, or the do-nothing
//!   individual built below.
//!
//! The objective is `EpiSpread`, one GET ships, constructed by hand rather
//! than named in a config file. A library caller assembles its sampling
//! parameters directly; there is no TOML anywhere in this program.

use std::sync::Arc;

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use get::evolver::common::{Crossover, Selection};
use get::evolver::scope::Scope;
use get::evolver::{Evolver, GenerationalContext, GenerationalEvolver, SharedEvolutionContext};
use get::fitness::EpiSpread;
use get::genomes::edge_edit::IDENTITY_GENE;
// `Genome` is imported for its `print` method at the end — a trait's methods
// are only callable where the trait is in scope.
use get::genomes::{
    EdgeEditContext, EdgeEditGenome, EdgeEditMutation, EdgeEditOperationWeights, EdgeEditOperators,
    Genome,
};
use get::graph::Graph;
use get::sir::{SirParams, SirSampleParams};

const NUM_NODES: usize = 24;
const MAX_EDGE_MULTIPLICITY: u32 = 1;
const POPULATION_SIZE: usize = 40;
const GENE_LENGTH: usize = 32;
const SEED: u64 = 20260820;

/// A ring: every node joined to its two neighbours.
///
/// Something for the edit script to start from, and connected, so an epidemic
/// can actually spread from any node it starts at.
fn ring(num_nodes: usize) -> Graph {
    let mut graph = Graph::new(num_nodes, MAX_EDGE_MULTIPLICITY);
    for node in 0..num_nodes {
        graph.set_edge(node, (node + 1) % num_nodes, 1);
    }
    graph
}

fn main() {
    // Everything random in the run comes from this one generator, so the whole
    // example reproduces from SEED alone.
    let mut rng = ChaCha8Rng::seed_from_u64(SEED);

    // 1. The operation mix, validated once and shared by the whole population.
    //    Every weight defaults to 1.0; naming them is how a library caller sets
    //    what `[genome.operation_weights]` sets from a config file. A weight of
    //    0.0 disables its operation outright — `null` is off here, so every
    //    gene does something.
    let weights = EdgeEditOperationWeights {
        null: 0.0,
        ..EdgeEditOperationWeights::default()
    };
    let operators = EdgeEditOperators::new(weights).expect("at least one weight is positive");

    // 2. A starting population. Random individuals, then one *chosen* one.
    let mut population = Vec::with_capacity(POPULATION_SIZE);
    for _ in 0..POPULATION_SIZE {
        population.push(EdgeEditGenome::random_with_operators(
            GENE_LENGTH,
            Arc::clone(&operators),
            &mut rng,
        ));
    }

    // The do-nothing individual: opcode 8 is `Null`, which expression skips, so
    // this genome expresses to exactly the base graph. Keeping one means
    // generation 0 contains the graph you supplied rather than only random
    // departures from it, so the run cannot return something worse than its own
    // input without having been beaten fairly.
    population[0] = EdgeEditGenome::new_with_operators(
        vec![IDENTITY_GENE; GENE_LENGTH],
        Arc::clone(&operators),
    );

    // Genes are readable and writable, which is what makes a chosen population
    // possible at all: read a script off one genome, change it, hand it back.
    // Here the second individual is the first's script with its opening edit
    // replaced by a real one, so it starts one edit away from the base graph.
    let mut seeded_genes = population[0].genes.clone();
    seeded_genes[0] = population[1].genes[0];
    population[1] = EdgeEditGenome::new_with_operators(seeded_genes, Arc::clone(&operators));

    // 3. How a genome becomes a graph: the base graph its edits apply to, and
    //    which mutation the run uses. Run configuration, not evolved state —
    //    `mutate` and `crossover` never see the base graph.
    let genome_context = EdgeEditContext {
        base_graph: ring(NUM_NODES),
        mutation: EdgeEditMutation::RerollGene,
    };

    // 4. What the engine owns: the two variation dice rolls, how a pair
    //    recombines, which slice of the population an event draws from, and
    //    parent selection. A strategy never rolls these itself.
    let shared = SharedEvolutionContext {
        genome_context,
        crossover_rate: 0.8,
        mutation_rate: 0.5,
        max_mutations: 2,
        // Generational breeds from the whole population, so the tournament is
        // what applies the pressure.
        scope: Scope::Global,
        selection: Selection::Tournament { tournament_size: 7 },
        crossover: Crossover::TwoPoint,
    };

    // 5. The objective, built by hand. `run_seed` fixes which epidemics every
    //    graph in a batch is scored against, so two graphs differ by their
    //    structure rather than by their dice.
    let fitness = EpiSpread::new(
        SirSampleParams {
            epidemic: SirParams {
                infection_rate: 0.3,
                // None draws a fresh patient zero per epidemic.
                patient_zero: None,
            },
            num_epidemics: 12,
            // 1 disables the short-outbreak re-roll; every epidemic has
            // length >= 1, so nothing is ever re-rolled.
            min_epidemic_length: 1,
            max_epidemic_retries: 1,
        },
        SEED,
    );

    // 6. The strategy's own configuration, and the run.
    let strategy = GenerationalContext {
        num_generations: 40,
        elite_count: 1,
    };

    let mut evolver = GenerationalEvolver::new(shared, strategy, population);
    let outcome = evolver.run(&fitness, SEED);

    // 7. Reading the result. Everything inside the engine is lower-is-better,
    //    whatever the objective computed, so convert once on the way out —
    //    otherwise a maximizing objective like this one reports its scores
    //    negated.
    let best = outcome.direction.orient(outcome.best_fitness_engine);
    println!("edge-edit + generational, objective epi_spread");
    println!("  best mean ever-infected: {best:.2} of {NUM_NODES} nodes");
    println!(
        "  best graph has {} edges (base ring had {})",
        outcome.best_graph.get_edge_list().len(),
        NUM_NODES,
    );

    let first = outcome.direction.orient(outcome.history[0].best_fitness);
    let last_row = &outcome.history[outcome.history.len() - 1];
    println!(
        "  best-of-generation went {first:.2} -> {:.2} over {} generations",
        outcome.direction.orient(last_row.best_fitness),
        last_row.iteration,
    );

    // The genome is here too, not just the graph it expressed to — so a run's
    // winner can be fed back into a later one through `new_with_operators`.
    let repr = outcome.best_genome.print();
    let head: String = repr.chars().take(60).collect();
    println!("  best genome: {head}...");
}
