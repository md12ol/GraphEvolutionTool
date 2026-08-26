//! The [`Genome`] trait every representation implements, and the run-level
//! context and mutation-kind types each representation pairs with it.

use rand::Rng;

use crate::graph::Graph;

// ADD A GENOME STEP 1 — implement this trait for your own type.
//
//     impl Genome for MyGenome {
//         type Context = MyContext;
//
//         fn express(&self, context: &Self::Context) -> Graph { ... }
//         fn crossover<R: Rng + ?Sized>(&mut self, other: &mut Self, rng: &mut R) { ... }
//         fn mutate<R: Rng + ?Sized>(&mut self, context: &Self::Context, rng: &mut R) { ... }
//         fn print(&self) -> String { ... }
//     }

/// The variation-operator interface implemented by every genome representation.
pub trait Genome: Clone + Send + Sync {
    /// Run-level configuration required to express this genome: the same for
    /// every individual, for the whole run.
    type Context: Send + Sync;

    /// Express this genome as a graph using shared run-level configuration.
    fn express(&self, context: &Self::Context) -> Graph;

    /// Recombine two parents in place, leaving the resulting children in
    /// `self` and `other`.
    ///
    /// **Both parents must still be valid for the representation when this
    /// returns** — nothing checks it.
    ///
    /// All randomness must come from `rng`. A draw taken from anywhere else
    /// makes two replicate runs at the same seed disagree.
    fn crossover<R: Rng + ?Sized>(&mut self, other: &mut Self, rng: &mut R);

    /// Apply **exactly one** mutation to this genome, in place. Rolling your own
    /// count makes the run's `max_mutations` meaningless, and nothing reports it.
    ///
    /// A genome with nothing to mutate leaves itself unchanged rather than
    /// panicking.
    fn mutate<R: Rng + ?Sized>(&mut self, context: &Self::Context, rng: &mut R);

    /// Return a human-readable description of the genome.
    fn print(&self) -> String;
}

// ADD A GENOME STEP 2 — declare your context type here, beside the other two.
// Make the struct and every field `pub`, and keep it to run configuration: if
// variation can change it, it belongs on the genome instead.
//
//     #[derive(Clone, Debug)]
//     pub struct MyContext {
//         pub num_nodes: usize,
//         pub some_mutation_rate: f64,
//     }

/// Configuration used when an edge-edit genome modifies an initial graph.
#[derive(Clone, Debug)]
pub struct EdgeEditContext {
    pub base_graph: Graph,
    pub mutation: EdgeEditMutation,
}

/// Configuration used when an SDA genome generates a graph from scratch, and
/// the probabilities that shape how one mutates.
#[derive(Clone, Debug, PartialEq)]
pub struct SdaContext {
    pub num_nodes: usize,
    /// The state the automaton starts in before consuming `init_char`'s first
    /// transition. Must be below the genome's state count — expression indexes
    /// the transition table with it, and panics if it is out of range.
    pub init_state: usize,
    /// Pass `1` for unweighted graphs.
    pub max_edge_multiplicity: u32,
    /// Chance that a mutation redraws the initial character rather than
    /// touching the transition table at all.
    pub init_char_mutation_rate: f64,
    /// Given that the initial character was *not* chosen, the chance of
    /// redrawing a transition's target state; the remainder redraws that
    /// transition's response instead.
    pub transition_vs_response_rate: f64,
    /// Which mutation this run applies; the two rates above shape it.
    pub mutation: SdaMutation,
}

/// Which mutation an edge-edit genome performs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EdgeEditMutation {
    /// Reroll one gene, its opcode drawn from the operation mix.
    #[default]
    RerollGene,
    // ADD A MUTATION STEP 1 (for EdgeEdit) — a variant here, plus any parameters it reads:
    //
    //     MyMutation { some_param: f64 },
}

/// Which mutation an SDA genome performs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SdaMutation {
    /// Redraw exactly one of: the initial character, one transition's target
    /// state, or that transition's response — chosen by the two rates on
    /// [`SdaContext`].
    #[default]
    RedrawOne,
    // ADD A MUTATION STEP 1 (for SDA) — a variant here, plus any parameters it reads:
    //
    //     MyMutation { some_param: f64 },
}
