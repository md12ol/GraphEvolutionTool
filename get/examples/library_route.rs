//! Driving GET as a Rust library, with your own objective and no config file.
//!
//! Run it with:
//!
//! ```text
//! cargo run -p graph-evolution-tool --example library_route
//! ```
//!
//! This is the route for someone who wants a native objective, or who is
//! embedding GET in their own package. It uses nothing but the public API:
//! the `Fitness` trait, a genome, a context, and an evolver. There is no
//! `Config`, no TOML, and no Python anywhere in it.
//!
//! **Nothing here writes a file until you say so.** An evolver hands back an
//! `EvolutionOutcome` and stops; the config-driven routes write three files
//! because their front end chose to. `OUTPUT_DIR` below is this program making
//! the same choice, and `write_results` is the whole of what it takes — the
//! same layout and the same two file formats `get-run` produces, so the
//! winner it writes loads straight back in as another run's base graph.
//!
//! **The config-driven routes cannot be reached from here, and that is
//! deliberate.** Turning a config document into concrete types is the job of a
//! private module, so a library caller assembles the population and contexts
//! itself, as below. What you gain is that your objective never has to be a
//! config variant — it is just a type you own, holding whatever data you like.
//!
//! The objective here is not one GET ships: reward a graph for how many of its
//! nodes sit at exactly a target degree, which pushes evolution toward a
//! regular graph. It is deliberately cheap to read rather than interesting to
//! optimize. **It is also the only one of the four programs that writes its own
//! objective** — the others build the ones GET ships, which is the case this is
//! a foil for.
//!
//! # One of four, and where the rest are
//!
//! This program covers SDA under the generational strategy. Between them the
//! four cover every genome × evolver combination from outside the crate, which
//! is what makes them a proof rather than a demonstration: narrowing anything
//! the library route needs breaks one of these and nothing else, because
//! nothing inside the crate goes through this door.
//!
//! | | `GenerationalEvolver` | `SteadyStateEvolver` |
//! |---|---|---|
//! | `SdaGenome` | this program | `sda_steady_state.rs` |
//! | `EdgeEditGenome` | `edge_edit_generational.rs` | `edge_edit_steady_state.rs` |
//!
//! `edge_edit_generational.rs` also carries the audit of which `config.toml`
//! settings have a route-3 equivalent and which deliberately do not.

use std::fs::File;
use std::io::Write;

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use get::evolver::common::{Crossover, Selection};
use get::evolver::scope::Scope;
use get::evolver::{Evolver, GenerationalContext, GenerationalEvolver, SharedEvolutionContext};
use get::fitness::{Direction, Fitness};
// `Genome` is imported for its `print` method at the end — a trait's methods
// are only callable where the trait is in scope.
use get::genomes::{Genome, SdaContext, SdaGenome, SdaMutation};
use get::graph::Graph;

/// How many nodes sit at exactly `target_degree`. Larger is better.
struct Regularity {
    target_degree: usize,
}

impl Fitness for Regularity {
    /// Score the **original**, in your own units. The engine converts it; do
    /// not pre-negate for a maximizing objective.
    fn evaluate(&self, graph: &Graph) -> f64 {
        let mut at_target = 0;
        for node in 0..NUM_NODES {
            if graph.neighbor_count(node) == self.target_degree {
                at_target += 1;
            }
        }
        at_target as f64
    }

    /// Without this the trait defaults to `Minimize`, and the run would
    /// silently optimize for the *fewest* nodes at the target degree. Nothing
    /// warns about it, so a maximizing objective must say so here.
    fn direction(&self) -> Direction {
        Direction::Maximize
    }

    // `evaluate_batch` is left at its default, which runs `evaluate` across
    // rayon. Override it only if scoring is stochastic — the default would
    // draw a fresh sample per graph, and scores inside one batch would stop
    // being comparable — or if a batch can be scored more cheaply at once.
}

const NUM_NODES: usize = 30;
const MAX_EDGE_MULTIPLICITY: u32 = 1;
const POPULATION_SIZE: usize = 60;
const SEED: u64 = 20260817;

/// **Where this program writes its results. Change it to anywhere you like.**
///
/// Each run lands in `OUTPUT_DIR/<timestamp>-<seed>/`, and each replicate below
/// gets a `run_<index>/` of its own inside that — the same layout `get-run`
/// produces, through the same `get::run_output_dir`, so output from the two
/// routes can be compared without translating a path.
const OUTPUT_DIR: &str = "./output";

/// How many replicates to run from `SEED`.
///
/// A library caller drives its own loop; there is no `n_runs` argument to a
/// `run` call here, because the evolver runs once and hands back one outcome.
/// Each replicate gets its own seed, derived from `SEED` so that a replicate's
/// numbers do not change when you ask for more of them.
const N_RUNS: usize = 2;

/// Write one replicate's convergence log and winner into `directory`.
///
/// GET writes these files for you on the config-driven routes; a library caller
/// has an `EvolutionOutcome` and decides for itself what to keep, which is what
/// this function is. The formats are the ones GET reads back: the log is the
/// same seven columns `save_logs` emits, and the edge list carries
/// `# nodes = N`, so the winner here is a loadable base graph for the next run.
///
/// Every fitness written out is oriented first. Inside the engine lower is
/// always better, whatever the objective computed, so writing the raw numbers
/// would record a maximizing run's scores negated.
fn write_results(
    directory: &std::path::Path,
    outcome: &get::evolver::EvolutionOutcome<SdaGenome>,
    seed: u64,
    run_index: usize,
) -> std::io::Result<()> {
    std::fs::create_dir_all(directory)?;

    let mut log = File::create(directory.join("run_log.csv"))?;
    writeln!(
        log,
        "iteration,best_fitness,mean_fitness,std_dev,ci_95,seed,run_index"
    )?;
    for row in &outcome.history {
        writeln!(
            log,
            "{},{},{},{},{},{seed},{run_index}",
            row.iteration,
            outcome.direction.orient(row.best_fitness),
            outcome.direction.orient(row.mean_fitness),
            row.std_dev,
            row.ci_95,
        )?;
    }

    let mut best = File::create(directory.join("best_individual.txt"))?;
    writeln!(
        best,
        "# best_fitness = {}",
        outcome.direction.orient(outcome.best_fitness_engine)
    )?;
    writeln!(best, "# nodes = {NUM_NODES}")?;
    for (u, v, weight) in outcome.best_graph.get_edge_list() {
        writeln!(best, "{u},{v},{weight}")?;
    }

    Ok(())
}

/// One replicate, from its own derived seed.
///
/// Everything the run needs is built here rather than once outside the loop:
/// the population is consumed by the evolver, and the contexts are cheap, so
/// sharing them across replicates would buy nothing and make it easy to leak
/// state from one run into the next.
fn run_once(seed: u64) -> get::evolver::EvolutionOutcome<SdaGenome> {
    // Everything random in this replicate comes from this one generator, so it
    // reproduces from its seed alone.
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    // 1. A starting population. The library caller sizes it; no context field
    //    carries the population size, because the population itself is the
    //    authority on it.
    let mut population = Vec::with_capacity(POPULATION_SIZE);
    for _ in 0..POPULATION_SIZE {
        let genome = SdaGenome::random_with_edge_multiplicity_cap(
            8,                     // num_states
            MAX_EDGE_MULTIPLICITY, // alphabet is this + 1, so every character is a legal weight
            4,                     // max_resp_len
            &mut rng,
        )
        .expect("SDA dimensions are within the genome's storage limits");
        population.push(genome);
    }

    // 2. How a genome becomes a graph. This is run configuration, not evolved
    //    state — `mutate` and `crossover` never see it.
    let genome_context = SdaContext {
        num_nodes: NUM_NODES,
        init_state: 0,
        max_edge_multiplicity: MAX_EDGE_MULTIPLICITY,
        init_char_mutation_rate: 0.1,
        transition_vs_response_rate: 0.5,
        // Which mutation to apply; the two rates above shape it. SDA ships
        // exactly one, so this is the only choice today.
        mutation: SdaMutation::RedrawOne,
    };

    // 3. What the engine owns: the two variation dice rolls and parent
    //    selection. A strategy never rolls these itself.
    let shared = SharedEvolutionContext {
        genome_context,
        crossover_rate: 0.8,
        mutation_rate: 0.9,
        max_mutations: 3,
        selection: Selection::Tournament { tournament_size: 7 },
        // Which slice of the population one breeding event draws from.
        // Generational breeds from all of it; steady-state uses a small random
        // subset, which is what keeps its best individual safe.
        scope: Scope::Global,
        // How a pair recombines, separately from `crossover_rate`'s decision
        // of whether it does. Two-point is the only operator GET ships.
        crossover: Crossover::TwoPoint,
    };

    // 4. The strategy's own configuration, and the run.
    let strategy = GenerationalContext {
        num_generations: 200,
        elite_count: 1,
    };
    let fitness = Regularity { target_degree: 4 };

    let mut evolver = GenerationalEvolver::new(shared, strategy, population);
    evolver.run(&fitness, seed)
}

fn main() {
    // One stamp for the whole invocation, so every replicate lands under the
    // same directory however long the runs take.
    let stamp = get::utc_stamp();

    // One seed per replicate, derived the way every other route derives them.
    let seeds = get::replicate_seeds(SEED, N_RUNS);

    for (run_index, &run_seed) in seeds.iter().enumerate() {
        let outcome = run_once(run_seed);

        if N_RUNS > 1 {
            println!("\n=== run_index {run_index}, of {N_RUNS} ===");
        }

        // 5. Reading the result. Everything inside the engine is lower-is-better,
        //    whatever the objective computed, so convert once on the way out —
        //    otherwise a maximizing objective reports its scores negated.
        let best = outcome.direction.orient(outcome.best_fitness_engine);
        println!("target degree 4 — {best} of {NUM_NODES} nodes reached it");
        println!(
            "best graph has {} edges",
            outcome.best_graph.get_edge_list().len()
        );

        let first = outcome.direction.orient(outcome.history[0].best_fitness);
        let last_row = &outcome.history[outcome.history.len() - 1];
        println!(
            "best-of-generation went {first} -> {} over {} generations",
            outcome.direction.orient(last_row.best_fitness),
            last_row.iteration,
        );

        // The genome is here too, not just the graph it expressed to.
        let repr = outcome.best_genome.print();
        let head: String = repr.chars().take(60).collect();
        println!("best genome: {head}...");

        // 6. Persisting it. Nothing above wrote a file — that is the library
        //    route's default, and this is the caller deciding otherwise.
        let directory = get::run_output_dir(Some(OUTPUT_DIR), &stamp, SEED, run_index, N_RUNS);
        match write_results(&directory, &outcome, SEED, run_index) {
            Ok(()) => println!("wrote {}", directory.display()),
            Err(err) => eprintln!("warning: could not write {}: {err}", directory.display()),
        }
    }
}
