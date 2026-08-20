//! Driving the SDA genome under the steady-state strategy, and building the
//! two objectives that need more than a parameter block.
//!
//! Run it with:
//!
//! ```text
//! cargo run -p get --example sda_steady_state
//! ```
//!
//! The last of four programs covering every genome × evolver combination from
//! outside the crate — see `edge_edit_generational.rs` for the grid.
//!
//! This one closes the fourth cell (SDA under steady-state; `library_route.rs`
//! covers SDA under generational) and exercises the two shipped objectives the
//! other programs do not:
//!
//! - **`EpiProfMatch`** drives the run. It takes a target profile — how many
//!   nodes are newly infected at each timestep — supplied inline here, where a
//!   config file would name it under `[fitness]`. It is *minimized*: the score
//!   is an error against the target, so lower is better and the engine needs no
//!   conversion. Reading the result still goes through `direction.orient`,
//!   which is its own inverse and so is correct either way — writing it
//!   unconditionally is what keeps a program correct when its objective
//!   changes.
//! - **`StructMatch`** is built at the end and used to score the winner once,
//!   rather than to drive a second run. It needs the most assembly of any
//!   objective: a reference set reduced to [`ReferenceStatistics`] on shared
//!   [`HistogramAxes`], plus a gamma and a weight per statistic family. The
//!   reference set here is three hand-built graphs, because the point is that
//!   a library caller can assemble one at all — not that these are good
//!   reference data.
//!
//! `StructMatch` is also where the config route and the library route differ
//! most visibly. From a config file the reference set is a folder of edge
//! files, read and reduced by a private module; from here it is whatever
//! `&[Graph]` you can produce, which is strictly more general — the graphs
//! need never have been on disk.

use std::sync::Arc;

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use get::evolver::common::{Crossover, Selection};
use get::evolver::{Evolver, SharedEvolutionContext, SteadyStateContext, SteadyStateEvolver};
use get::fitness::{EpiProfMatch, Fitness, StructMatch};
use get::genomes::{Genome, SdaContext, SdaGenome, SdaMutation};
use get::graph::Graph;
use get::sir::{SirParams, SirSampleParams};
use get::stats::{HistogramAxes, PerFamily, ReferenceStatistics};

const NUM_NODES: usize = 24;
// StructMatch compares simple graphs, so the alphabet is 0..=1 and every
// character the automaton emits is a legal edge weight.
const MAX_EDGE_MULTIPLICITY: u32 = 1;
const POPULATION_SIZE: usize = 40;
const NUM_STATES: usize = 8;
const MAX_RESP_LEN: usize = 3;
const TOURNAMENT_SIZE: usize = 6;
const SEED: u64 = 20260820;

/// A ring with every `step`-th chord added, as reference material.
///
/// Three of these at different chord spacings give the reference set some
/// spread in degree and clustering, which is what the statistics compare.
fn chorded_ring(num_nodes: usize, step: usize) -> Graph {
    let mut graph = Graph::new(num_nodes, MAX_EDGE_MULTIPLICITY);
    for node in 0..num_nodes {
        graph.set_edge(node, (node + 1) % num_nodes, 1);
        if node % step == 0 {
            graph.set_edge(node, (node + step) % num_nodes, 1);
        }
    }
    graph
}

/// The highest degree anywhere in the set — the top of the degree axis.
///
/// Taken from the data rather than guessed: degrees above the axis all land in
/// the last bin, so an axis that is too short flattens exactly the differences
/// the objective is meant to see.
fn highest_degree(graphs: &[Graph]) -> usize {
    let mut highest = 0;
    for graph in graphs {
        for node in 0..graph.num_nodes {
            let degree = graph.degree(node);
            if degree > highest {
                highest = degree;
            }
        }
    }
    highest
}

fn main() {
    let mut rng = ChaCha8Rng::seed_from_u64(SEED);

    // 1. A starting population of random automata. The alphabet is derived as
    //    the multiplicity cap plus one, never chosen, so every character is a
    //    legal edge weight and none is clamped away at expression.
    let mut population = Vec::with_capacity(POPULATION_SIZE);
    for _ in 0..POPULATION_SIZE {
        let genome = SdaGenome::random_with_edge_multiplicity_cap(
            NUM_STATES,
            MAX_EDGE_MULTIPLICITY,
            MAX_RESP_LEN,
            &mut rng,
        )
        .expect("SDA dimensions are within the genome's storage limits");
        population.push(genome);
    }

    // 2. How an automaton becomes a graph, and which mutation the run applies.
    //    The two rates shape that mutation; `mutation` selects it.
    let genome_context = SdaContext {
        num_nodes: NUM_NODES,
        init_state: 0,
        max_edge_multiplicity: MAX_EDGE_MULTIPLICITY,
        init_char_mutation_rate: 0.1,
        transition_vs_response_rate: 0.5,
        mutation: SdaMutation::RedrawOne,
    };

    let shared = SharedEvolutionContext {
        genome_context,
        crossover_rate: 0.8,
        mutation_rate: 0.5,
        max_mutations: 2,
        selection: Selection::Tournament {
            tournament_size: TOURNAMENT_SIZE,
        },
        crossover: Crossover::TwoPoint,
    };

    // 3. The driving objective. The target is the shape of epidemic being
    //    asked for: a slow start, a peak around the third timestep, a long
    //    tail. A config file would carry this as `target_profile`.
    let target_profile = vec![1.0, 3.0, 8.0, 6.0, 4.0, 2.0, 1.0];
    let fitness = EpiProfMatch::new(
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
        target_profile.clone(),
    )
    .expect("the target profile is non-empty and finite");

    let strategy = SteadyStateContext {
        num_mating_events: POPULATION_SIZE * 30,
    };

    let mut evolver = SteadyStateEvolver::new(shared, strategy, population);
    let outcome = evolver.run(&fitness, SEED);

    let best = outcome.direction.orient(outcome.best_fitness_engine);
    println!("sda + steady-state, objective epi_prof_match");
    println!(
        "  best RMSE against a {}-step target: {best:.3} (lower is better)",
        target_profile.len(),
    );
    println!(
        "  best graph has {} edges over {NUM_NODES} nodes",
        outcome.best_graph.get_edge_list().len(),
    );

    let first = outcome.direction.orient(outcome.history[0].best_fitness);
    let last_row = &outcome.history[outcome.history.len() - 1];
    println!(
        "  best-of-population went {first:.3} -> {:.3} over {} mating events",
        outcome.direction.orient(last_row.best_fitness),
        last_row.iteration,
    );

    // 4. StructMatch, assembled from scratch and used to score the winner.
    //    Nothing here came from a file: the reference set is three graphs built
    //    in this program, which is the general case a config folder is one
    //    instance of.
    let reference_graphs = vec![
        chorded_ring(NUM_NODES, 3),
        chorded_ring(NUM_NODES, 4),
        chorded_ring(NUM_NODES, 6),
    ];
    let axes = HistogramAxes {
        max_degree: highest_degree(&reference_graphs),
        degree_bins: 8,
        clustering_bins: 8,
        spectral_bins: 8,
    };
    let reference = ReferenceStatistics::from_graphs(&reference_graphs, axes)
        .expect("three non-empty graphs and non-zero bin counts");

    let struct_match = StructMatch::new(
        // Shared behind an Arc because replicates each need their own
        // objective but can share one reduced reference set.
        Arc::new(reference),
        // One gamma per family: the RBF kernel's width for that statistic.
        PerFamily {
            degree: 1.0,
            clustering: 1.0,
            spectral: 1.0,
        },
        // And one weight per family, deciding how much each contributes.
        PerFamily {
            degree: 1.0,
            clustering: 1.0,
            spectral: 0.5,
        },
        // How hard to penalise a candidate whose edge density is unlike the
        // reference set's.
        0.25,
    )
    .expect("gammas are positive and not every weight is zero");

    // `evaluate` scores one graph in the objective's own units. StructMatch is
    // a distance, so this is minimized and cannot go below zero.
    let distance = struct_match.evaluate(&outcome.best_graph);
    println!("  the same graph, scored against a 3-graph reference set:");
    println!("    struct_match distance {distance:.4} (lower is better)");

    let repr = outcome.best_genome.print();
    let head = repr.lines().next().unwrap_or_default();
    println!("  best genome starts: {head}");
}
