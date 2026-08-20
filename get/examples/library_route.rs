//! Driving GET as a Rust library, with your own objective and no config file.
//!
//! Run it with:
//!
//! ```text
//! cargo run -p get --example library_route
//! ```
//!
//! This is the route for someone who wants a native objective, or who is
//! embedding GET in their own package. It uses nothing but the public API:
//! the `Fitness` trait, a genome, a context, and an evolver. There is no
//! `Config`, no TOML, and no Python anywhere in it.
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
//! optimize.

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
            if graph.degree(node) == self.target_degree {
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

fn main() {
    // Everything random in the run comes from this one generator, so the whole
    // example reproduces from SEED alone.
    let mut rng = ChaCha8Rng::seed_from_u64(SEED);

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
    let outcome = evolver.run(&fitness, SEED);

    // 5. Reading the result. Everything inside the engine is lower-is-better,
    //    whatever the objective computed, so convert once on the way out —
    //    otherwise a maximizing objective reports its scores negated.
    let best = outcome.direction.orient(outcome.best_fitness_engine);
    println!(
        "target degree {} — {best} of {NUM_NODES} nodes reached it",
        fitness.target_degree,
    );
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
}
