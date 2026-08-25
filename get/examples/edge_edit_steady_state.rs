//! Driving the edge-edit genome under the steady-state strategy, as a library.
//!
//! Run it with:
//!
//! ```text
//! cargo run -p graph-evolution-tool --example edge_edit_steady_state
//! ```
//!
//! One of four programs covering every genome × evolver combination from
//! outside the crate — see `library_route.rs` for the grid, and
//! `edge_edit_generational.rs` for both the edge-edit assembly this program
//! does not re-explain and the audit of which `config.toml` settings have a
//! route-3 equivalent.
//!
//! **This is the program that proves [`SteadyStateContext`] is reachable.**
//! `Evolver::new` takes the strategy's own `TypeContext`, so a caller who
//! cannot name that type cannot build the evolver at all — and until this
//! example existed, every construction of it was inside the crate. That made
//! it look, to any pass counting external callers, exactly like a type nobody
//! needs. It is not: it is structurally load-bearing for this route, and
//! narrowing it would break nothing inside the crate and every library caller
//! outside it.
//!
//! Two things steady-state does differently, and both bite at construction:
//!
//! - **`tournament_size` must be at least 4**, and the population at least as
//!   large. `SteadyStateEvolver::new` asserts both. Four is the smallest
//!   tournament that keeps the two parents and the two individuals they
//!   replace disjoint; below it the strategy stops carrying its best forward.
//! - **No `elite_count`.** Elitism is not configured because it is structural:
//!   a tournament's best is never among the two it replaces, so the
//!   population's best individual is never discarded.
//!
//! The objective is `EpiLength`, a second shipped one, again built by hand.

use std::sync::Arc;

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use get::evolver::common::{Crossover, Selection};
use get::evolver::replacement::Replacement;
use get::evolver::scope::Scope;
use get::evolver::{Evolver, SharedEvolutionContext, SteadyStateContext, SteadyStateEvolver};
use get::fitness::EpiLength;
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
const SCOPE_SIZE: usize = 6;
const SEED: u64 = 20260820;

/// A path: nodes joined in a line, with no edge closing it into a ring.
///
/// Connected, so an epidemic started anywhere can reach the rest, but with the
/// longest possible shortest-path structure for its size — which is what
/// `EpiLength` rewards, so the run starts somewhere the objective already
/// likes and has to work to keep it.
fn path(num_nodes: usize) -> Graph {
    let mut graph = Graph::new(num_nodes, MAX_EDGE_MULTIPLICITY);
    for node in 0..num_nodes - 1 {
        graph.set_edge(node, node + 1, 1);
    }
    graph
}

fn main() {
    let mut rng = ChaCha8Rng::seed_from_u64(SEED);

    let operators = EdgeEditOperators::new(EdgeEditOperationWeights::default())
        .expect("the default mix weights every operation equally");

    let mut population = Vec::with_capacity(POPULATION_SIZE);
    for _ in 0..POPULATION_SIZE {
        population.push(EdgeEditGenome::random_with_operators(
            GENE_LENGTH,
            Arc::clone(&operators),
            &mut rng,
        ));
    }

    let genome_context = EdgeEditContext {
        base_graph: path(NUM_NODES),
        mutation: EdgeEditMutation::RerollGene,
    };

    let shared = SharedEvolutionContext {
        genome_context,
        crossover_rate: 0.8,
        mutation_rate: 0.5,
        max_mutations: 2,
        // The slice each mating event draws from. At least 4, and no larger
        // than the population — `SteadyStateEvolver` asserts both, at
        // construction rather than at the first mating event.
        scope: Scope::RandomSubset { size: SCOPE_SIZE },
        // The two fittest of that slice breed. Steady-state's pressure comes
        // from the scope being small, not from a draw within it.
        selection: Selection::Best,
        crossover: Crossover::TwoPoint,
    };

    let fitness = EpiLength::new(
        SirSampleParams {
            epidemic: SirParams {
                infection_rate: 0.3,
                patient_zero: None,
            },
            num_epidemics: 12,
            min_epidemic_length: 1,
            max_epidemic_retries: 1,
        },
        SEED,
    );

    // The strategy's own configuration: one field, and the type a library
    // caller must be able to name to build the evolver at all.
    //
    // Mating events are not generations. Each one breeds a single pair and
    // replaces two individuals, so the whole population turns over roughly
    // every `POPULATION_SIZE` events — which is also the logging interval, so
    // the history below has one row per that many events plus a row 0.
    let strategy = SteadyStateContext {
        num_mating_events: POPULATION_SIZE * 30,
        // The two least fit of the same scope are overwritten. Because the
        // scope's best is never among them, the population's best individual
        // is never discarded and no explicit elitism is needed.
        replacement: Replacement::Worst,
    };

    let mut evolver = SteadyStateEvolver::new(shared, strategy, population);
    let outcome = evolver.run(&fitness, SEED);

    let best = outcome.direction.orient(outcome.best_fitness_engine);
    println!("edge-edit + steady-state, objective epi_length");
    println!("  best mean burnout time: {best:.2} timesteps");
    println!(
        "  best graph has {} edges (base path had {})",
        outcome.best_graph.get_edge_list().len(),
        NUM_NODES - 1,
    );

    let first = outcome.direction.orient(outcome.history[0].best_fitness);
    let last_row = &outcome.history[outcome.history.len() - 1];
    println!(
        "  best-of-population went {first:.2} -> {:.2} over {} mating events, logged in {} rows",
        outcome.direction.orient(last_row.best_fitness),
        last_row.iteration,
        outcome.history.len(),
    );

    let repr = outcome.best_genome.print();
    let head: String = repr.chars().take(60).collect();
    println!("  best genome: {head}...");
}
