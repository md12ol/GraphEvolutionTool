//! The genome representations the GA evolves, and where a new one has to be
//! wired in.
//!
//! A representation is whatever an individual *is*: [`crate::genomes::sda`]
//! evolves an automaton that generates a graph from nothing, and
//! [`crate::genomes::edge_edit`] evolves a script of edits applied to a graph
//! you already have. Both reach the engine through [`Genome`] below, and the
//! engine never sees anything else about them.
//!
//! # Adding your own representation
//!
//! A representation touches **seven steps across five files**. `dispatch.rs`
//! appears twice, once to build the starting population and once to select the
//! representation, and they are separate steps because they fail differently:
//! the first reports a bad dimension, the second only picks.
//!
//! How many of the seven are yours depends on which way you are using GET, so
//! work out which reader you are first:
//!
//! - **You depend on this crate from your own program.** Steps 1 and 2 are the
//!   only ones available to you, and together they are enough: write your
//!   type, implement [`Genome`] for it, and build the population and the
//!   [`Genome::Context`] by hand before handing both to the evolver. Steps 3-7
//!   are unreachable rather than skipped — `config`, `dispatch` and
//!   `py_config` are private modules, so nothing outside this crate can add a
//!   config variant, a start builder or a dispatch arm, and the example file
//!   lives in the GET repository rather than yours. `examples/library_route.rs`
//!   walks this end to end.
//! - **You are editing your own copy of GET.** All seven steps are yours. What
//!   that buys, and what the first reader structurally cannot have, is a
//!   representation selectable by name under `[genome]` in `config.toml` and
//!   runnable by the `get-run` binary with no Rust at the call site. Step 5 is
//!   the one that makes that true: a start builder is what turns a config
//!   document into a sized population and a context.
//!
//! **Every step below is marked at its own site in the code.** Search the repo
//! for `ADD A GENOME STEP 3` — or any other number — and you land on the exact
//! place that step is made, next to a worked example of what to add there:
//!
//! ```text
//! git grep -n "ADD A GENOME STEP"      # all seven, in one list
//! ```
//!
//! The steps, in the order you would walk them:
//!
//! 1. **This file** — implement [`Genome`] for your type: the
//!    [`Genome::Context`] associated type, [`Genome::express`],
//!    [`Genome::crossover`], [`Genome::mutate`] and [`Genome::print`]. Every
//!    one is required; there are no defaults to inherit. `crossover` and
//!    `mutate` carry a contract about mutation *count* that the engine depends
//!    on — it is stated on [`Genome::mutate`], and is the one part of this
//!    trait a representation cannot reinterpret locally.
//! 2. **Your context type**, declared beside the representation or here next to
//!    [`EdgeEditContext`] and [`SdaContext`]. See "What `Context` is, and what
//!    it is not" below — deciding what goes on the context rather than on the
//!    genome is a design call this trait makes you take deliberately, and it is
//!    easy to get wrong in a way nothing reports.
//! 3. **`genomes/mod.rs`** — declare the module and re-export the type and its
//!    context, so callers name them from `crate::genomes` rather than from the
//!    private path.
//! 4. **`config.rs`** — add a `GenomeConfig` variant carrying the dimensions
//!    random individuals are built from, and validate them in
//!    `Config::validate_genome`. A dimension that would panic during expression
//!    belongs here, at load, rather than mid-run: `init_state` is checked
//!    against `num_states` for exactly that reason.
//! 5. **`dispatch.rs`, the start builder** — a function beside `edge_edit_start`
//!    and `sda_start` that turns that config variant into a population and a
//!    context. See "What a start builder owes the engine" below; this is the
//!    step with real obligations rather than a line of wiring.
//! 6. **`dispatch.rs`, the `evolve` match** — one arm selecting your
//!    representation. Steps 4, 5 and 6 are one change split across two files: a
//!    variant nothing constructs is dead code, and an arm for a variant that
//!    does not exist will not compile. The match is genome outside, strategy
//!    inside, so a third representation is one arm there and nothing at all in
//!    the evolvers.
//! 7. **`py_config.rs` and `config.example.toml`** — the Python-side
//!    constructor, if the representation should be reachable from Python, and
//!    an example block if it ships. Both are optional and neither costs
//!    anything elsewhere if left out: a caller simply has no way to name the
//!    representation from Python, and most people never find one that is
//!    missing from the example file. If step 4's validation raises a new field
//!    name, that name also needs a Python attribute path in `py_config.rs`, or
//!    the error a Python caller sees will name a TOML field they never wrote.
//!
//! # What `Context` is, and what it is not
//!
//! [`Genome::Context`] is **run configuration** — built once, before generation
//! 0, and read but never written for the rest of the run. It is not evolved
//! state, and nothing in the trait can write to it: [`Genome::express`] and
//! [`Genome::mutate`] both take `&Self::Context`.
//!
//! It comes from the start builder in step 5, which is the only place holding
//! both the parsed config and the freedom to fail. [`EdgeEditContext`] carries
//! the base graph an edit script is applied to; [`SdaContext`] carries the node
//! count, the edge-weight cap and the two mutation probabilities.
//!
//! [`SdaContext::init_state`] is the worked example of the split, and the
//! reason it is worth taking deliberately. It looks like genome data — it is
//! the automaton's starting state — and an earlier shape could have put it on
//! `SdaGenome` beside `init_char`, which *is* evolved. It sits on the context
//! because [`Genome::mutate`] and [`Genome::crossover`] never touch it: it is
//! the same for every individual for the whole run. So the test is not "does
//! this describe the individual" but **"can variation change it"**. Anything
//! variation cannot change is configuration, and putting it on the genome
//! instead costs one copy per individual and invites a mutation that has no
//! business existing.
//!
//! # Why `Send + Sync` is on both the trait and the context
//!
//! `evolver::common::express_and_score` expresses a whole population in
//! parallel over rayon: it `par_iter`s the batch and hands every worker thread
//! the *same* `&Self::Context`. That needs `Genome: Send + Sync` for
//! individuals to cross thread boundaries, and `Context: Send + Sync` for one
//! reference to be shared across them.
//!
//! Both bounds are load-bearing, and the compiler error if either is missing
//! does not say why — it surfaces inside rayon's `ParallelIterator` machinery,
//! several layers away from anything you wrote. A representation or context
//! holding an `Rc`, a `RefCell` or a raw pointer fails there rather than at its
//! own definition.
//!
//! # What a start builder owes the engine
//!
//! Step 5 has three obligations, and no type enforces any of them:
//!
//! - **A population of exactly `config.population_size` individuals.** The
//!   evolvers size their working buffers from what they are handed, so a short
//!   population is a quietly smaller search rather than an error.
//! - **A context `express` can actually use.** Anything `express` indexes with
//!   has to be range-checked here, because the alternative is a panic mid-run
//!   inside a generic, which crosses the Python boundary as an opaque
//!   `PanicException`. Reporting beats asserting for anything that can reach a
//!   release build.
//! - **Rejection rather than clamping** when caller-supplied data disagrees
//!   with the config. A base graph with the wrong node count, or one carrying
//!   an edge above `max_edge_multiplicity`, is refused outright — clamping it
//!   would run the search against a graph the caller never handed over and
//!   never gets to see.
//!
//! Validate once, in the builder, rather than per individual: the dimensions
//! are the same for the whole population, so a failure there can only ever be a
//! startup failure.

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
//
// All five items are required. Read `mutate`'s contract below before writing
// it — exactly one mutation per call is what the engine's `max_mutations`
// depends on.

/// The variation-operator interface implemented by every genome representation.
///
/// `Clone + Send + Sync` allows the GA to copy individuals and evaluate a
/// population across worker threads.
pub trait Genome: Clone + Send + Sync {
    /// Run-level configuration required to express this genome.
    ///
    /// `Send + Sync` is required because `evolver::common::express_and_score`
    /// expresses a whole population in parallel over rayon, sharing one `&Self::Context`
    /// across worker threads. Without the bound that parallel expression does
    /// not compile.
    type Context: Send + Sync;

    /// Express this genome as a graph using shared run-level configuration.
    fn express(&self, context: &Self::Context) -> Graph;

    /// Recombine two parents in place, leaving the resulting children in
    /// `self` and `other`.
    ///
    /// **In place, and both children are kept.** Recombination inherently
    /// produces two children, so neither parent is preserved and neither child
    /// is discarded — an engine that kept only one would waste half of every
    /// crossover. The caller has already decided this pair recombines;
    /// `crossover_rate` is rolled once per pair by
    /// [`crate::evolver::common::breed_pair`], never here.
    ///
    /// **Both parents must still be valid for the representation when this
    /// returns.** Nothing checks it, and that is the whole hazard: an operator
    /// that can leave a genome `express` rejects breaks a run mid-flight, deep
    /// inside a generic, rather than at config time where a user could act on
    /// it. Whatever invariant the representation's own constructors maintain —
    /// a state index below `num_states`, a response length within
    /// `max_resp_len`, a gene the operation mix can decode — this has to
    /// maintain too, on *both* sides.
    ///
    /// **All randomness comes from `rng`.** A representation reaching for a
    /// thread RNG or a clock breaks replicate reproducibility, which is
    /// GET's whole seeding model: one master seed reaches the population, the
    /// evolution and the epidemics, and a single unseeded draw anywhere in the
    /// chain makes two runs at the same seed disagree.
    ///
    /// # An operator chooses how much shared structure it needs
    ///
    /// The two shipped operators are both two-point and they answer this
    /// differently, which is the point: [`crate::genomes::EdgeEditGenome`]
    /// declines to cross below two shared genes, because a one-gene exchange
    /// is not choosing anything, while [`crate::genomes::SdaGenome`] does cross
    /// at one shared state, because `init_char` travels with state 0 and so
    /// there is genuinely something more to exchange. Each states its own
    /// answer at its `crossover`. A third representation makes the same choice
    /// explicitly rather than inheriting either.
    ///
    /// `R: Rng + ?Sized` (rather than a plain `Rng` type) lets callers pass
    /// either a concrete RNG or a trait object like `&mut dyn RngCore`
    /// through the same parameter; `?Sized` opts out of Rust's default
    /// requirement that generic types have a compile-time-known size, which
    /// trait objects don't have. Same bound, same reason, on `mutate` below.
    fn crossover<R: Rng + ?Sized>(&mut self, other: &mut Self, rng: &mut R);

    /// Apply **exactly one** mutation to this genome, in place.
    ///
    /// This is a contract, not a suggestion. A representation that rolls its own
    /// mutation count internally makes the engine's `max_mutations` meaningless
    /// for that representation, and nothing would report the disagreement — which
    /// is precisely how the two genomes here drifted apart before it was written
    /// down.
    ///
    /// **The engine owns both dice rolls**, in one shared helper
    /// ([`crate::evolver::common::mutate_child`]): `mutation_rate` decides whether
    /// a child mutates at all, then `max_mutations` decides how many times this
    /// method is called. Callers wanting more disruption call it repeatedly.
    ///
    /// What *one mutation* means belongs to the representation, and the
    /// magnitudes are deliberately not equalized — one gene of 256 is a far
    /// smaller perturbation than one transition of 24. A shared `max_mutations`
    /// buys equal mutation **count**, not equal **strength**.
    ///
    /// A genome with nothing to mutate (an empty gene list, a zero-state
    /// automaton) leaves itself unchanged rather than panicking.
    ///
    /// **All randomness comes from `rng`**, the same obligation `crossover`
    /// carries and for the same reason: one master seed reaches the
    /// population, the evolution and the epidemics, so a single draw taken
    /// from anywhere else makes two replicate runs at the same seed stop
    /// agreeing.
    ///
    /// `context` is the same run-level configuration `express` reads, passed
    /// so a representation can take its mutation probabilities from the run
    /// rather than from a private constant. A representation that keeps its
    /// mix elsewhere — edge-edit carries a prebuilt sampler on the genome —
    /// ignores the parameter.
    fn mutate<R: Rng + ?Sized>(&mut self, context: &Self::Context, rng: &mut R);

    /// Return a human-readable description of the genome.
    fn print(&self) -> String;
}

// ADD A GENOME STEP 2 — declare your context type, here or beside your
// representation.
//
//     #[derive(Clone, Debug)]
//     pub struct MyContext {
//         pub num_nodes: usize,
//         pub some_mutation_rate: f64,
//     }
//
// Run configuration only, never evolved state — the test is "can variation
// change it", and anything variation cannot change belongs here rather than on
// the genome. `SdaContext::init_state` below is the worked example.

/// Which mutation an edge-edit genome performs.
///
/// **Per representation, not per run** — and that is the difference from
/// [`crate::evolver::common::Crossover`], which is shared. Both genomes
/// recombine the same way, so one enum describes the run; but what *one
/// mutation* means differs completely between them, so each owns its own set
/// and a config naming SDA's mutation under an edge-edit `[genome]` does not
/// parse at all. That is the whole reason this is nested rather than
/// top-level: the mismatch a shared enum would need validation to reject
/// cannot be written down here.
///
/// # Adding one
///
/// Applies the same way to [`SdaMutation`] below, for whichever
/// representation you are extending — and this chain **exists twice**, once
/// per genome, so every marker names which half it belongs to. Follow only the
/// half you are extending: a step from the other one is a different enum in a
/// different file, and applying it does nothing for your representation.
/// **Every step is marked at its own site** — search the repo for
/// `ADD A MUTATION STEP 3 (for EdgeEdit)`, or any other number:
///
/// ```text
/// git grep -nE "ADD A MUTATION STEP . \(for EdgeEdit\)"   # this genome's four
/// git grep -n  "ADD A MUTATION STEP"                     # both genomes, all eight
/// ```
///
/// 1. **This enum** — the variant, plus any parameters it reads.
/// 2. **[`crate::genomes::EdgeEditGenome::mutate`]** — the arm performing it.
///    The compiler finds it: the match is exhaustive.
/// 3. **`config::EdgeEditMutationConfig`**, and its arm in
///    `dispatch::edge_edit_mutation` that maps the choice onto this operator.
/// 4. **`py_config`'s mirror** and **`config.example.toml`** — both optional,
///    both the steps that decide whether anyone ever finds the operator.
///
/// Keep the **exactly one mutation** contract on [`Genome::mutate`]: a variant
/// applying several makes the engine's `max_mutations` meaningless for this
/// representation, and nothing reports the disagreement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EdgeEditMutation {
    /// Reroll one gene, its opcode drawn from the operation mix. What
    /// edge-edit did before the operator was selectable.
    #[default]
    RerollGene,
    // ADD A MUTATION STEP 1 (for EdgeEdit) — a variant here, plus any parameters it reads:
    //
    //     MyMutation { some_param: f64 },
    //
    // Then the arm performing it, in `EdgeEditGenome::mutate` — search
    // `ADD A MUTATION STEP 2 (for EdgeEdit)` for it.
}

/// Which mutation an SDA genome performs. Per representation, for the reason
/// [`EdgeEditMutation`] gives.
///
/// The rates that *shape* a mutation stay on [`SdaContext`] rather than moving
/// into a variant here. They predate this enum and are read by the one
/// operator below; folding them in would rename two live config keys to buy
/// nothing while a single operator ships.
///
/// # Adding one
///
/// The four steps [`EdgeEditMutation`] lists, in this genome's copy of the
/// chain. Take the `(for SDA)` markers and none of the `(for EdgeEdit)` ones:
///
/// ```text
/// git grep -nE "ADD A MUTATION STEP . \(for SDA\)"    # this genome's four
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SdaMutation {
    /// Redraw exactly one of: the initial character, one transition's target
    /// state, or that transition's response — chosen by the two rates on
    /// [`SdaContext`]. What SDA did before the operator was selectable.
    #[default]
    RedrawOne,
    // ADD A MUTATION STEP 1 (for SDA) — a variant here, plus any parameters it reads:
    //
    //     MyMutation { some_param: f64 },
    //
    // Then the arm performing it, in `SdaGenome::mutate` — search
    // `ADD A MUTATION STEP 2 (for SDA)` for it.
}

/// Configuration used when an edge-edit genome modifies an initial graph.
#[derive(Clone, Debug)]
pub struct EdgeEditContext {
    pub base_graph: Graph,
    /// Which mutation this run applies. Reaches [`Genome::mutate`] the same
    /// way every other run-level setting does — the genome does not store it,
    /// because variation cannot change it.
    pub mutation: EdgeEditMutation,
}

/// Configuration used when an SDA genome generates a graph from scratch, and
/// the probabilities that shape how one mutates.
///
/// `Eq` is deliberately not derived: the two mutation rates are `f64`, which
/// only implements `PartialEq`.
#[derive(Clone, Debug, PartialEq)]
pub struct SdaContext {
    pub num_nodes: usize,
    /// The state the automaton starts in before consuming `init_char`'s
    /// first transition. Fixed run configuration, not evolved genome data
    /// (unlike `init_char`, `init_state` is never touched by
    /// [`Genome::mutate`]/[`Genome::crossover`]), so it lives here rather
    /// than on `SdaGenome`.
    pub init_state: usize,
    /// Pass `1` for unweighted graphs.
    pub max_edge_multiplicity: u32,
    /// Chance that a mutation redraws the initial character rather than
    /// touching the transition table at all.
    pub init_char_mutation_rate: f64,
    /// Given that the initial character was *not* chosen, the chance of
    /// redrawing a transition's target state; the remainder redraws that
    /// transition's response instead. `0.5` mutates the two equally often.
    pub transition_vs_response_rate: f64,
    /// Which mutation this run applies. The two rates above shape it; this
    /// selects it.
    pub mutation: SdaMutation,
}
