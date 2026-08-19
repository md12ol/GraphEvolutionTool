//! The objectives the GA optimizes, and the sign rule that makes them
//! comparable.
//!
//! # Adding your own objective
//!
//! An objective touches up to five files, in six steps — two of the files want
//! more than one edit, and `dispatch.rs` appears twice, once for the code and
//! once for the test. How many are yours depends on which way you are using
//! GET, so start by working out which reader you are:
//!
//! - **You depend on this crate from your own program.** Step 1 is the only
//!   one available to you, and it is enough on its own: write your type,
//!   implement [`Fitness`] for it, and pass it to `Evolver::run`. Steps 2–6
//!   are unreachable rather than skipped: `config`, `dispatch` and `py_config`
//!   are private modules, so nothing outside this crate can add a config
//!   variant or a dispatch arm, and the example file and the tests live in the
//!   GET repository rather than yours. Your objective reaches the evolver by
//!   being handed over directly, and is never named in a config file.
//! - **You are editing your own copy of GET.** All six steps are yours. What that
//!   buys, and what the first reader structurally cannot have, is an objective
//!   selectable by name from `config.toml` and runnable by the `get-run`
//!   binary, with no Rust written at the call site.
//!
//! The steps, in the order you would walk them:
//!
//! 1. **This file** — implement [`Fitness`] for your type. [`Fitness::evaluate`]
//!    is the only required method. Add [`Fitness::direction`] if bigger is
//!    better, and override [`Fitness::evaluate_batch`] if the default is wrong
//!    for you — see "When overriding `evaluate_batch` is required" below,
//!    because that case is about correctness rather than speed.
//! 2. **`config.rs`** — three edits, not one. Add a variant to
//!    `FitnessConfig` holding whatever the objective reads out of the file;
//!    add its arm to `FitnessConfig::type_name`, which is the string error
//!    messages name the objective by; and add validation for any parameter
//!    worth constraining. Only the first is what a user selects under
//!    `[fitness]`. The `type_name` arm cannot be forgotten — the match is
//!    exhaustive, so omitting it fails to compile.
//! 3. **`dispatch.rs`** — add the matching arm to `objective()`, which turns
//!    that variant into a `Box<dyn Fitness>`. Steps 2 and 3 are one change
//!    split across two files: a variant nothing constructs is dead code, and
//!    an arm for a variant that does not exist will not compile.
//! 4. **`py_config.rs`** — add the Python-side constructor, if the objective
//!    should be reachable from Python. Leave it out and everything else still
//!    works; a Python caller simply has no way to name the objective. If step
//!    2's validation raises a new field name, that name also needs a Python
//!    attribute path here, or the error a Python caller sees will name a TOML
//!    field they never wrote. The test
//!    `every_validation_field_maps_to_a_python_attribute` is what catches a
//!    missing one.
//! 5. **`config.example.toml`** — add an example block if the objective ships.
//!    The example file is what a user copies from, so an objective missing
//!    from it is one most people never find.
//! 6. **The dispatch tests** — assert the new variant erases to a box that
//!    reports its own [`Direction`]. Worth writing because the failure it
//!    catches is silent: an objective whose direction is lost runs the search
//!    backwards and looks merely unconverged.
//!
//! # What `Direction` costs you if you get it wrong
//!
//! [`Fitness::evaluate`] returns your score in your own units, and
//! [`Fitness::direction`] is what tells the engine whether large or small
//! wins. The engine compares in one convention throughout and converts back at
//! the boundary, so you never negate your own output — see [`Direction`],
//! which explains both forms and why the objective does not do the flipping.
//!
//! The default is [`Direction::Minimize`], which means an objective that
//! should be maximized and does not say so is not rejected anywhere. It runs,
//! it logs, and it optimizes for the worst graph it can find.
//!
//! # When overriding `evaluate_batch` is required
//!
//! The default scores each graph independently across rayon threads, which is
//! correct for an objective that is a pure function of one graph. Two cases
//! are not merely slower under it — they are wrong:
//!
//! - **The objective is stochastic.** The default draws fresh randomness per
//!   graph, so two graphs in one batch are scored against different samples
//!   and their scores stop being comparable to each other. Share one sample
//!   across the batch instead; [`EpidemicScorer::mean_batch`] is how the
//!   epidemic objectives do it.
//! - **The objective calls Python.** Taking the GIL inside a rayon closure
//!   deadlocks rather than running slowly, so a Python objective must batch
//!   its crossing of the boundary. `PyFitness` exists for this.
//!
//! # What an objective must not do
//!
//! Nothing that makes two runs at the same seed disagree. Every source of
//! randomness an objective uses has to derive from the seed it was built with,
//! never from the system clock, an address, thread scheduling, or iteration
//! order over a hash map. Reproducing a run from its seed is the only way a
//! result can be checked afterwards, and a run that quietly stopped being
//! reproducible looks exactly like one that is fine.
//!
//! # If it is epidemic-based
//!
//! Build it on [`EpidemicScorer`] rather than calling the simulator yourself.
//! It owns the seeding, and wrong seeding still gives numbers that look fine.
//! Copy the shape [`EpiSpread`] uses: both `evaluate` and `evaluate_batch`
//! hand [`EpidemicScorer::mean_batch`] a closure saying what to read off one
//! epidemic. Write the same reading in both — the test
//! `both_entry_points_use_the_same_reading` fails if they disagree.

use std::slice;
use std::sync::atomic::{AtomicU64, Ordering};

use pyo3::prelude::*;
use rayon::prelude::*;

use crate::graph::Graph;
use crate::sir::{Epidemic, SirSampleParams, simulate_epidemics};

/// Whether an objective wants its value small or large.
///
/// Every fitness number in the engine is in one of two forms, and mixing them
/// up is the bug this type exists to prevent:
///
/// - **original** — what the fitness function returned, in its own units. 28
///   nodes infected is `28.0`, and bigger is better.
/// - **oriented** — the original after [`Direction::orient`], which negates it
///   when the objective maximizes and leaves it alone when the objective
///   minimizes, so that smaller is always better. That same 28 becomes
///   `-28.0`.
///
/// The engine only ever compares, so it works in oriented values throughout;
/// logs and results are turned back into originals at the boundary (§5.1),
/// which is what the sheet calls engine orientation.
///
/// The objective does not negate its own output, because then the value and
/// the declared direction could disagree — and a run optimizing backwards
/// looks exactly like a run that is simply not converging.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// Smaller is better, as for an error or a distance. The default.
    Minimize,
    /// Larger is better.
    Maximize,
}

impl Direction {
    /// Orient an original, so that smaller always wins.
    ///
    /// Under [`Direction::Minimize`] the two are the same number; under
    /// [`Direction::Maximize`] the oriented value is the negated original, so
    /// the largest original becomes the smallest oriented value.
    ///
    /// Negation is its own inverse, so this one function converts both ways:
    /// an original in to compare, an oriented value in to report.
    ///
    /// # Panics
    ///
    /// On `NaN`. Under [`Direction::Maximize`] it becomes `-NaN`, which sorts
    /// below `-inf` — so it would win every tournament it entered and leave a
    /// run that looks converged. Rust's `assert!` survives a release build, so
    /// this check always runs.
    pub fn orient(self, value: f64) -> f64 {
        assert!(
            !value.is_nan(),
            "fitness function returned NaN, which the Fitness contract forbids. \
             Check for division by a possibly-zero count, 0.0/0.0, or inf - inf \
             in the objective's arithmetic.",
        );
        match self {
            Direction::Minimize => value,
            Direction::Maximize => -value,
        }
    }
}

/// An objective the GA optimizes over expressed graphs.
///
/// [`Fitness::evaluate`] returns the **original** score, in the objective's
/// own units; [`Fitness::direction`] says which way is better. The engine
/// orients it exactly once, so logs and results keep the original units and
/// sign (§5.1). See [`Direction`] for both terms.
///
/// `Send + Sync` lets [`Fitness::evaluate_batch`] score across rayon
/// threads.
///
/// # Implement these; never call them
///
/// Only `common::express_and_score` calls them. A direct call compiles and
/// returns plausible numbers, but hands the engine an original where an
/// oriented value belongs — so under [`Direction::Maximize`] every comparison
/// runs backwards, and nothing says so. It skips the `NaN` check too.
///
/// # Never return `NaN`
///
/// [`Direction::orient`] panics on it. Watch for division by a count that can
/// be zero, `0.0 / 0.0`, and `inf - inf`.
pub trait Fitness: Send + Sync {
    /// Score one graph: the **original**, in the objective's own units, never
    /// an oriented value. Must not return `NaN`.
    fn evaluate(&self, graph: &Graph) -> f64;

    /// Which way is better. Defaults to [`Direction::Minimize`], so an error
    /// or distance objective says nothing.
    fn direction(&self) -> Direction {
        Direction::Minimize
    }

    /// Score a **batch of graphs** — whatever set the evolver scores together.
    /// These come back as originals too; the caller converts them.
    ///
    /// The batch is not always a generation. Generational hands over the whole
    /// population each cycle; steady-state hands over just the two new
    /// children per mating event, and its starting population once (§6.3). All
    /// three are batches.
    ///
    /// The default runs [`Fitness::evaluate`] on each graph across rayon,
    /// which suits a Rust objective. A Python one overrides this to take the
    /// GIL once per batch instead of once per graph.
    ///
    /// **A stochastic objective must override it as well** — the default would
    /// draw a fresh seed per graph, so scores would no longer be comparable
    /// inside the batch. See [`EpidemicScorer::mean_batch`].
    fn evaluate_batch(&self, graphs: &[Graph]) -> Vec<f64> {
        graphs
            .par_iter()
            .map(|graph| self.evaluate(graph))
            .collect()
    }
}

/// A boxed objective is an objective.
///
/// # Why this exists
///
/// The config layer erases its fitness variant to one `Box<dyn Fitness>` before
/// instantiating anything, so that adding an objective is one match arm rather
/// than one arm per strategy × genome combination (§1, §8). But `Evolver::run<F>`
/// requires `F: Fitness`, and a `Box` holding a `Fitness` is not itself one
/// until this says so. It has to live here beside the trait — the orphan rule
/// rejects it anywhere else.
///
/// # Every method is forwarded, including the two with defaults
///
/// This is the whole point, and both omissions **compile**:
///
/// - **`evaluate_batch`** — without it the box inherits the trait's
///   default, which fans out over rayon and calls `evaluate` per graph. For a
///   Python objective that means one GIL acquisition per individual from inside
///   a rayon closure, which is what `PyFitness`'s batching exists to prevent —
///   and which **deadlocks** rather than merely running slowly (measured
///   2026-08-07; see `PyFitness`). For an epidemic objective it also re-seeds
///   per graph, so scores stop being comparable within a batch.
/// - **`direction`** — without it the box reports [`Direction::Minimize`]
///   whatever it holds, so every maximizing objective runs the search backwards
///   while looking merely unconverged.
///
/// Neither failure produces a compiler error, a panic, or a wrong-looking
/// number, which is why `a_boxed_objective_forwards_every_method` exists.
impl Fitness for Box<dyn Fitness> {
    fn evaluate(&self, graph: &Graph) -> f64 {
        (**self).evaluate(graph)
    }

    fn direction(&self) -> Direction {
        (**self).direction()
    }

    fn evaluate_batch(&self, graphs: &[Graph]) -> Vec<f64> {
        (**self).evaluate_batch(graphs)
    }
}

/// Runs the epidemics that every SIR objective scores (§5.2).
///
/// The epidemic is the expensive part and all three objectives want the same
/// one, so this runs the batch and each objective supplies only a reading —
/// see [`EpiSpread`] for the smallest example.
///
/// The nesting, widest first (§5.2, §8.1): an **experiment** is many **runs**
/// at one set of parameters; a run scores many **batches of graphs**; each
/// batch averages many **epidemics** per graph. One scorer covers one run.
///
/// A batch is whatever the evolver scores in one call, and its size is not
/// fixed: the whole population for a generational cycle or for either
/// evolver's starting population, but only the **two new children** for a
/// steady-state mating event (§6.3). Nothing here needs to know which — it
/// seeds whatever arrives.
///
/// **One scorer per run.** The counter below is per-run state; two replicates
/// sharing a scorer would let thread scheduling pick which run saw which seed,
/// and reproducibility goes with it (§8.1).
pub struct EpidemicScorer {
    params: SirSampleParams,
    run_seed: u64,
    /// Batches scored so far — see [`EpidemicScorer::next_batch_seed`].
    batches_scored: AtomicU64,
}

impl EpidemicScorer {
    /// Build a scorer for one run.
    ///
    /// `run_seed` is this run's share of the master seed handed to
    /// `GraphEvolver::run`; `[fitness]` has no seed of its own (§5.2).
    pub fn new(params: SirSampleParams, run_seed: u64) -> Self {
        Self {
            params,
            run_seed,
            batches_scored: AtomicU64::new(0),
        }
    }

    /// The seed for the next batch of graphs. Every call returns a different
    /// one, because it advances the counter.
    ///
    /// A seed fixes every random choice the epidemic simulator makes. Same
    /// seed, same epidemics. Different seed, different epidemics.
    ///
    /// **Call this once per batch, then give that one seed to every graph in
    /// the batch.** The batch size depends on the evolver, and nothing here
    /// changes with it:
    ///
    /// ```text
    /// generational   batch 1   seed A   all 200 of the population
    ///                batch 2   seed B   all 200 of the next generation
    ///
    /// steady-state   batch 1   seed A   the 200 starting graphs
    ///                batch 2   seed B   the 2 children of one mating event
    ///                batch 3   seed C   the 2 children of the next event
    /// ```
    ///
    /// *One seed across the batch*, because those graphs are compared with
    /// each other. If each drew its own, a graph could rank first for having
    /// been handed a milder outbreak.
    ///
    /// *A new seed for the next batch*, because reusing A forever would breed
    /// a population good at outbreak A rather than good at the disease.
    ///
    /// Both properties together are what §5.2 calls common random numbers.
    ///
    /// Steady-state pays a known cost here, accepted in §5.2: its two children
    /// are scored under a newer seed than the population they are compared
    /// against, and a graph that drew an easy outbreak keeps that score until
    /// something replaces it.
    ///
    /// The counter is an atomic for a duller reason than it looks: `evaluate`
    /// only gets `&self`, so a plain `+= 1` will not compile, and `Cell` is
    /// not `Sync`, which [`Fitness`] requires. Nothing here is actually
    /// contended — [`EpidemicScorer::mean_batch`] calls this once on its own
    /// thread before rayon fans out, batches are scored one after another, and
    /// each replicate owns its own scorer (§8.1). `Relaxed` is enough because
    /// no other data rides along with the count.
    pub(crate) fn next_batch_seed(&self) -> u64 {
        let counter = self.batches_scored.fetch_add(1, Ordering::Relaxed);
        mix_seed(self.run_seed, counter)
    }

    /// Score a whole batch of graphs — **one seed for every graph, one tick**.
    ///
    /// This is the only way to score, and the method that delivers common
    /// random numbers. It is why each objective overrides
    /// [`Fitness::evaluate_batch`] rather than letting the default score
    /// the graphs one at a time (§5.2). It does not care whether the batch is
    /// a generation, a starting population or two steady-state children — a
    /// single graph is a batch of one.
    ///
    /// `read` turns one epidemic into one number, which is what keeps each
    /// objective to a single line. Averaging matters: a single epidemic is
    /// noisy enough that selection would chase the dice instead of the graph.
    /// The division is safe — [`simulate_epidemics`] rejects an empty batch.
    ///
    /// `+ Sync` on `read` lets rayon call it from several threads at once.
    pub fn mean_batch(&self, graphs: &[Graph], read: impl Fn(&Epidemic) -> f64 + Sync) -> Vec<f64> {
        // Taken once, here, and handed to every graph below. Taking it inside
        // the loop would give each graph its own dice — see next_batch_seed.
        let seed = self.next_batch_seed();

        graphs
            .par_iter()
            .map(|graph| {
                let epidemics = simulate_epidemics(graph, &self.params, seed);

                let mut total = 0.0;
                for epidemic in &epidemics {
                    total += read(epidemic);
                }
                total / epidemics.len() as f64
            })
            .collect()
    }
}

/// Turn a run seed and a batch number into that batch's seed.
///
/// SplitMix64: step a large odd constant `counter` times, then scramble. Every
/// pair gives a different, well-spread `u64`, which is all this needs — the
/// result seeds a real generator and is never used as randomness itself.
/// (`wrapping_*` lets the arithmetic overflow and wrap instead of panicking.)
///
/// **Not `run_seed ^ counter`** (§8.1): neighbouring run seeds would collide
/// across batch numbers, so two replicates would replay each other's epidemics
/// one batch apart. See `decisions.md` 2026-08-06.
fn mix_seed(run_seed: u64, counter: u64) -> u64 {
    let mut z = run_seed.wrapping_add(counter.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Total ever-infected, averaged over the batch's epidemics. **Maximized.**
pub struct EpiSpread {
    scorer: EpidemicScorer,
}

impl EpiSpread {
    /// Build the objective from its epidemic sampling parameters.
    pub fn new(params: SirSampleParams, run_seed: u64) -> Self {
        Self {
            scorer: EpidemicScorer::new(params, run_seed),
        }
    }
}

impl Fitness for EpiSpread {
    fn evaluate(&self, graph: &Graph) -> f64 {
        self.scorer
            .mean_batch(slice::from_ref(graph), |epidemic| epidemic.spread as f64)[0]
    }

    fn evaluate_batch(&self, graphs: &[Graph]) -> Vec<f64> {
        self.scorer
            .mean_batch(graphs, |epidemic| epidemic.spread as f64)
    }

    fn direction(&self) -> Direction {
        Direction::Maximize
    }
}

/// Timesteps to burn out, averaged over the epidemics. **Maximized.**
///
/// `length` counts the final burnout step, so a lone patient zero reads 1, not
/// 0 (§5.2).
pub struct EpiLength {
    scorer: EpidemicScorer,
}

impl EpiLength {
    /// Build the objective from its epidemic sampling parameters.
    pub fn new(params: SirSampleParams, run_seed: u64) -> Self {
        Self {
            scorer: EpidemicScorer::new(params, run_seed),
        }
    }
}

impl Fitness for EpiLength {
    fn evaluate(&self, graph: &Graph) -> f64 {
        self.scorer
            .mean_batch(slice::from_ref(graph), |epidemic| epidemic.length as f64)[0]
    }

    fn evaluate_batch(&self, graphs: &[Graph]) -> Vec<f64> {
        self.scorer
            .mean_batch(graphs, |epidemic| epidemic.length as f64)
    }

    fn direction(&self) -> Direction {
        Direction::Maximize
    }
}

/// RMSE between the epidemic profile and a target profile. **Minimized.**
///
/// The target is newly-infected counts, one per timestep. An epidemic's
/// profile starts with patient zero and ends with a terminating zero (§5.2),
/// so a target captured from older output will not line up element for element.
pub struct EpiProfMatch {
    scorer: EpidemicScorer,
    target: Vec<f64>,
}

impl EpiProfMatch {
    /// Build the objective from its sampling parameters and a target profile.
    ///
    /// # Errors
    ///
    /// If `target` is empty or holds a non-finite value — either would put a
    /// `NaN` into every score, which [`Fitness`] forbids. (`&'static str` is a
    /// fixed string literal, used here as a lightweight error type.)
    pub fn new(
        params: SirSampleParams,
        run_seed: u64,
        target: Vec<f64>,
    ) -> Result<Self, &'static str> {
        if target.is_empty() {
            return Err("epi_prof_match target profile must not be empty");
        }
        if !target.iter().all(|value| value.is_finite()) {
            return Err("epi_prof_match target profile must be finite");
        }
        Ok(Self {
            scorer: EpidemicScorer::new(params, run_seed),
            target,
        })
    }

    /// RMSE of one epidemic against the target — this objective's reading.
    ///
    /// A method rather than an inline closure only because it is too long to
    /// read twice; the other two objectives inline theirs.
    ///
    /// **The target sets the comparison, not the epidemic** (§5.2, matching
    /// `legacy/main.cpp:545-553`), so the scoring is asymmetric: an epidemic
    /// that ends early is penalised for the whole remaining target, while one
    /// that outlasts the target is not penalised at all. This rewards matching
    /// *or exceeding* the tail. See `decisions.md` 2026-08-04 18:13.
    fn rmse(&self, epidemic: &Epidemic) -> f64 {
        let mut total = 0.0;

        for (step, wanted) in self.target.iter().enumerate() {
            // Past the end of the epidemic nobody was newly infected, so a
            // missing step counts as zero. `.get` returns None instead of
            // panicking there, and `.unwrap_or(0)` supplies that zero.
            let actual = epidemic.profile.get(step).copied().unwrap_or(0) as f64;
            total += (actual - wanted).powi(2);
        }

        (total / self.target.len() as f64).sqrt()
    }
}

impl Fitness for EpiProfMatch {
    fn evaluate(&self, graph: &Graph) -> f64 {
        self.scorer
            .mean_batch(slice::from_ref(graph), |epidemic| self.rmse(epidemic))[0]
    }

    fn evaluate_batch(&self, graphs: &[Graph]) -> Vec<f64> {
        self.scorer
            .mean_batch(graphs, |epidemic| self.rmse(epidemic))
    }
}

/// A user's Python callable, used as an objective (§5, §8).
///
/// `config.toml` only *selects* Python — `[fitness] type = "python"`. The
/// callable itself is registered at runtime through
/// `GraphEvolver::set_fitness_function`, together with its [`Direction`],
/// because nothing can infer whether a user's function wants its value large
/// or small.
///
/// # The callable takes a whole batch, and that is not negotiable
///
/// One call receives the entire batch and returns one float per graph, in the
/// same order — the shape of pymoo's `Problem._evaluate`, which this audience
/// already knows:
///
/// ```python
/// def fitness(batch):   # batch: list[(num_nodes, [(u, v, weight), ...])]
///     return [score(n, edges) for (n, edges) in batch]
/// ```
///
/// A per-graph callback would serialize every call behind the GIL, losing all
/// rayon parallelism and paying lock contention on top of Python being slower
/// at the same arithmetic — together, potentially hundreds of times slower
/// wall-clock than native Rust. Batched, only the speed of the user's own code
/// remains.
///
/// **And "slower" understates it: the per-graph arrangement deadlocks.**
/// Measured 2026-08-07 by deleting the `evaluate_batch` override below and
/// running the batching test. The trait's default then fans out over rayon and
/// each worker tries to take the GIL, while the calling thread is holding it and
/// blocking on rayon to finish. The suite hung until it was killed — it did not
/// fail, which is worse, because a hang carries no message saying why.
///
/// Two rules keep that from happening, and both are structural rather than
/// advisory:
///
/// - **Python is never called from inside a rayon closure.** Expression fans
///   out across threads into a `Vec<Graph>` first; only then does the single
///   batched call happen, here.
/// - **The Rust-heavy part of a run should release the GIL**, this adapter
///   re-acquiring it per batch — the caller's job, at the entry point.
///
/// # Panics
///
/// Like every objective, this one has no `Result` path — [`Fitness`]'s methods
/// return `f64`. So a callable that raises, returns the wrong type, returns the
/// wrong number of scores, or returns `NaN` panics, each naming what was
/// expected and which item was at fault. That is the same posture
/// [`Direction::orient`] already takes on `NaN`.
pub(crate) struct PyFitness {
    callable: Py<PyAny>,
    direction: Direction,
}

impl PyFitness {
    /// Wrap a registered callable and the direction declared alongside it.
    pub(crate) fn new(callable: Py<PyAny>, direction: Direction) -> Self {
        Self {
            callable,
            direction,
        }
    }

    /// A second handle on the same callable.
    ///
    /// Replicate runs need one objective instance each (§8.1), and this is how
    /// the dispatch layer produces them. The Python object is shared rather
    /// than copied — `clone_ref` bumps its refcount — which is correct: the
    /// user's function is stateless as far as the engine is concerned, and it
    /// is the *scorer* state that must not be shared, of which this has none.
    pub(crate) fn clone_ref(&self) -> Self {
        Python::attach(|py| Self {
            callable: self.callable.clone_ref(py),
            direction: self.direction,
        })
    }

    /// The one call into Python, which both trait methods route through.
    ///
    /// Inherent rather than having [`Fitness::evaluate`] call
    /// [`Fitness::evaluate_batch`] directly: that arrangement is a latent
    /// stack overflow, because the trait's **default** `evaluate_batch`
    /// calls `evaluate`, so removing the override below turns the pair into
    /// infinite recursion instead of a compile error. Routing both through here
    /// has no cycle to fall into (`collab.md` #33).
    fn score_batch(&self, graphs: &[Graph]) -> Vec<f64> {
        // Nothing to score, so nothing to hand Python. Skipping the call also
        // spares a user's function from having to handle an empty batch.
        if graphs.is_empty() {
            return Vec::new();
        }

        // Built before the GIL is taken: this is plain Rust work, and holding
        // the GIL across it would block every other Python thread for no reason.
        let mut batch = Vec::with_capacity(graphs.len());
        for graph in graphs {
            batch.push((graph.num_nodes, graph.get_edge_list()));
        }

        Python::attach(|py| {
            let returned = self
                .callable
                .call1(py, (batch,))
                .unwrap_or_else(|err| panic!("the registered Python objective raised: {err}"));

            let scores: Vec<f64> = returned.extract(py).unwrap_or_else(|err| {
                panic!(
                    "the registered Python objective must return one float per graph, \
                     as a sequence: {err}"
                )
            });

            assert_eq!(
                scores.len(),
                graphs.len(),
                "the registered Python objective returned {} scores for a batch of {} \
                 graphs; it must return exactly one per graph, in the same order",
                scores.len(),
                graphs.len(),
            );

            // Caught here rather than at `Direction::orient`, which sees a lone
            // number and cannot say which graph produced it. A user debugging
            // their own function needs the index.
            for (index, &score) in scores.iter().enumerate() {
                assert!(
                    !score.is_nan(),
                    "the registered Python objective returned NaN for batch item {index}. \
                     Check for division by a possibly-zero count, 0.0/0.0, or inf - inf",
                );
            }

            scores
        })
    }
}

impl Fitness for PyFitness {
    /// One graph is a batch of one, so there is a single path into Python.
    fn evaluate(&self, graph: &Graph) -> f64 {
        self.score_batch(slice::from_ref(graph))[0]
    }

    fn direction(&self) -> Direction {
        self.direction
    }

    fn evaluate_batch(&self, graphs: &[Graph]) -> Vec<f64> {
        self.score_batch(graphs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sir::SirParams;

    /// An objective that records the size of every batch handed to it, and
    /// reports a non-default direction. Both are what a box must not lose.
    struct Instrumented {
        batch_sizes: std::sync::Arc<std::sync::Mutex<Vec<usize>>>,
    }

    impl Fitness for Instrumented {
        fn evaluate(&self, graph: &Graph) -> f64 {
            graph.num_nodes as f64
        }

        // Deliberately not Minimize: the box reports Minimize if `direction`
        // is left unforwarded, so a default here would hide that.
        fn direction(&self) -> Direction {
            Direction::Maximize
        }

        fn evaluate_batch(&self, graphs: &[Graph]) -> Vec<f64> {
            self.batch_sizes
                .lock()
                .expect("no test panics while holding this")
                .push(graphs.len());

            let mut scores = Vec::with_capacity(graphs.len());
            for graph in graphs {
                scores.push(graph.num_nodes as f64);
            }
            scores
        }
    }

    #[test]
    fn a_boxed_objective_forwards_every_method_including_the_defaulted_ones() {
        // The erasure in §8 hands the evolver a Box<dyn Fitness>. If the
        // forwarding impl omits either defaulted method it still compiles, and
        // both failures are silent: `direction` reports Minimize and runs the
        // search backwards, `evaluate_batch` reverts to the per-graph
        // rayon fan-out that deadlocks a Python objective.
        let batch_sizes = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let boxed: Box<dyn Fitness> = Box::new(Instrumented {
            batch_sizes: std::sync::Arc::clone(&batch_sizes),
        });

        assert_eq!(
            boxed.direction(),
            Direction::Maximize,
            "the box reported its own default instead of the objective's direction",
        );

        let graphs = [path_graph(3), path_graph(5), path_graph(8)];
        let scores = boxed.evaluate_batch(&graphs);

        assert_eq!(scores, vec![3.0, 5.0, 8.0]);
        assert_eq!(
            *batch_sizes.lock().unwrap(),
            vec![3],
            "the box fell back to the trait's per-graph default instead of \
             forwarding to the objective's own batched override",
        );

        // And the required method, for completeness.
        assert_eq!(boxed.evaluate(&path_graph(4)), 4.0);
    }

    /// Compile `source` and return the callable named `name` from it.
    ///
    /// The module keeps whatever state the source defines, so a test objective
    /// can record how it was called and the test can read that back.
    fn python_module(py: Python<'_>, source: &std::ffi::CStr) -> Py<PyAny> {
        use pyo3::types::PyModule;

        PyModule::from_code(py, source, c"objective.py", c"objective")
            .expect("the test objective compiles")
            .into_any()
            .unbind()
    }

    /// A `PyFitness` over `source`'s `fitness` function, plus a handle on the
    /// module so the test can read back what the call recorded.
    fn py_objective(
        py: Python<'_>,
        source: &std::ffi::CStr,
        direction: Direction,
    ) -> (PyFitness, Py<PyAny>) {
        use pyo3::types::PyAnyMethods;

        let module = python_module(py, source);
        let callable = module
            .bind(py)
            .getattr("fitness")
            .expect("the test objective defines `fitness`")
            .unbind();

        (PyFitness::new(callable, direction), module)
    }

    /// Scores each graph by its node count, and records the size of every batch
    /// it was handed — which is what distinguishes one call per batch from one
    /// call per graph.
    const COUNTING_OBJECTIVE: &std::ffi::CStr = c"
batch_sizes = []

def fitness(batch):
    batch_sizes.append(len(batch))
    return [float(num_nodes) for (num_nodes, edges) in batch]
";

    /// The batch sizes `COUNTING_OBJECTIVE` has seen so far.
    fn batch_sizes(py: Python<'_>, module: &Py<PyAny>) -> Vec<usize> {
        use pyo3::types::PyAnyMethods;

        module
            .bind(py)
            .getattr("batch_sizes")
            .expect("the objective records batch sizes")
            .extract()
            .expect("batch sizes are integers")
    }

    #[test]
    fn a_python_objective_is_called_once_per_batch_not_once_per_graph() {
        // The whole reason the contract is batched: a per-graph callback would
        // serialize every call behind the GIL and lose all rayon parallelism.
        Python::attach(|py| {
            let (objective, module) = py_objective(py, COUNTING_OBJECTIVE, Direction::Minimize);
            let graphs = [path_graph(3), path_graph(5), path_graph(8)];

            let scores = objective.evaluate_batch(&graphs);

            assert_eq!(scores, vec![3.0, 5.0, 8.0], "scores, in population order");
            assert_eq!(
                batch_sizes(py, &module),
                vec![3],
                "one call carrying all three graphs, not three calls",
            );
        });
    }

    #[test]
    fn scoring_one_graph_is_a_batch_of_one() {
        Python::attach(|py| {
            let (objective, module) = py_objective(py, COUNTING_OBJECTIVE, Direction::Minimize);

            let score = objective.evaluate(&path_graph(4));

            assert_eq!(score, 4.0);
            assert_eq!(batch_sizes(py, &module), vec![1]);
        });
    }

    #[test]
    fn an_empty_batch_never_reaches_python() {
        // A user's function should not have to handle a batch of nothing.
        Python::attach(|py| {
            let (objective, module) = py_objective(py, COUNTING_OBJECTIVE, Direction::Minimize);

            assert!(objective.evaluate_batch(&[]).is_empty());
            assert!(
                batch_sizes(py, &module).is_empty(),
                "an empty batch should not have called Python at all",
            );
        });
    }

    #[test]
    fn the_edges_handed_to_python_are_the_graph_that_was_expressed() {
        // Scores by edge weight, so a wrong or empty edge list changes the
        // answer rather than passing silently.
        const SUMS_WEIGHTS: &std::ffi::CStr = c"
def fitness(batch):
    return [float(sum(w for (u, v, w) in edges)) for (num_nodes, edges) in batch]
";
        Python::attach(|py| {
            let (objective, _module) = py_objective(py, SUMS_WEIGHTS, Direction::Minimize);

            let mut graph = Graph::new(4, 3);
            graph.set_edge(0, 1, 2);
            graph.set_edge(1, 2, 3);

            assert_eq!(objective.evaluate(&graph), 5.0);
        });
    }

    #[test]
    fn the_registered_direction_is_what_the_objective_reports() {
        // Nothing can infer this from the callable, so it is registered
        // alongside it — and getting it wrong runs the search backwards (§5).
        Python::attach(|py| {
            let (minimizing, _m) = py_objective(py, COUNTING_OBJECTIVE, Direction::Minimize);
            let (maximizing, _n) = py_objective(py, COUNTING_OBJECTIVE, Direction::Maximize);

            assert_eq!(minimizing.direction(), Direction::Minimize);
            assert_eq!(maximizing.direction(), Direction::Maximize);
        });
    }

    #[test]
    fn a_second_handle_scores_identically_to_the_first() {
        // Replicates need an objective instance each (§8.1); this is how the
        // dispatch layer will make them.
        Python::attach(|py| {
            let (objective, module) = py_objective(py, COUNTING_OBJECTIVE, Direction::Maximize);
            let second = objective.clone_ref();

            assert_eq!(second.direction(), Direction::Maximize);
            assert_eq!(second.evaluate(&path_graph(6)), 6.0);
            assert_eq!(objective.evaluate(&path_graph(6)), 6.0);
            assert_eq!(batch_sizes(py, &module), vec![1, 1], "both handles ran");
        });
    }

    #[test]
    #[should_panic(expected = "raised")]
    fn a_raising_callable_panics_rather_than_returning_a_wrong_number() {
        const RAISES: &std::ffi::CStr = c"
def fitness(batch):
    raise ValueError('no good')
";
        Python::attach(|py| {
            let (objective, _module) = py_objective(py, RAISES, Direction::Minimize);
            objective.evaluate(&path_graph(3));
        });
    }

    #[test]
    #[should_panic(expected = "must return one float per graph")]
    fn a_callable_returning_the_wrong_type_panics() {
        const RETURNS_A_STRING: &std::ffi::CStr = c"
def fitness(batch):
    return 'not a list of floats'
";
        Python::attach(|py| {
            let (objective, _module) = py_objective(py, RETURNS_A_STRING, Direction::Minimize);
            objective.evaluate(&path_graph(3));
        });
    }

    #[test]
    #[should_panic(expected = "returned 1 scores for a batch of 2")]
    fn a_callable_returning_too_few_scores_panics() {
        // Silently mismatched lengths would misalign every score with its
        // graph, which no later stage could detect.
        const RETURNS_ONE: &std::ffi::CStr = c"
def fitness(batch):
    return [1.0]
";
        Python::attach(|py| {
            let (objective, _module) = py_objective(py, RETURNS_ONE, Direction::Minimize);
            objective.evaluate_batch(&[path_graph(3), path_graph(4)]);
        });
    }

    #[test]
    #[should_panic(expected = "returned NaN for batch item 1")]
    fn a_callable_returning_nan_panics_naming_the_item() {
        const RETURNS_NAN: &std::ffi::CStr = c"
def fitness(batch):
    return [1.0, float('nan')]
";
        Python::attach(|py| {
            let (objective, _module) = py_objective(py, RETURNS_NAN, Direction::Minimize);
            objective.evaluate_batch(&[path_graph(3), path_graph(4)]);
        });
    }

    /// The test harness can reach a live Python interpreter.
    ///
    /// Not a test of this crate's logic — a guard on `get/Cargo.toml`. Moving
    /// `extension-module` back into `[dependencies]` leaves the Python symbols
    /// for an interpreter to supply at load time, and `cargo test` has none, so
    /// the whole suite stops **linking** — an error with no obvious connection
    /// to whatever was being changed. This fails first and says why.
    #[test]
    fn the_test_harness_can_call_a_live_python_interpreter() {
        use pyo3::types::PyAnyMethods;

        pyo3::Python::attach(|py| {
            let two: i64 = py
                .eval(c"1 + 1", None, None)
                .expect("evaluating `1 + 1` in the embedded interpreter")
                .extract()
                .expect("`1 + 1` is an integer");
            assert_eq!(two, 2);
        });
    }

    /// A path `0 - 1 - ... - (n-1)`, every edge at multiplicity 1.
    fn path_graph(num_nodes: usize) -> Graph {
        let mut graph = Graph::new(num_nodes, 1);
        // saturating_sub: clamps at 0 instead of underflowing when num_nodes
        // is 0 or 1 (usize can't go negative).
        for node in 0..num_nodes.saturating_sub(1) {
            graph.set_edge(node, node + 1, 1);
        }
        graph
    }

    /// Rate 1.0 from a pinned patient zero, so every epidemic is identical and
    /// no test depends on the seed.
    fn certain_batch(num_epidemics: usize) -> SirSampleParams {
        SirSampleParams {
            epidemic: SirParams {
                infection_rate: 1.0,
                patient_zero: Some(0),
            },
            num_epidemics,
            min_epidemic_length: 1,
            max_epidemic_retries: 1,
        }
    }

    /// A rate whose epidemics genuinely vary with the seed. The seeding tests
    /// need that — under `certain_batch` every epidemic is identical, so they
    /// would pass no matter how the seeding worked.
    ///
    /// 0.15 on `complete_graph(12)` is picked from measurement, not taste:
    /// higher and every epidemic reaches all 12 nodes, lower and the average
    /// over `num_epidemics` keeps landing on the same value.
    fn chancy_batch(num_epidemics: usize) -> SirSampleParams {
        SirSampleParams {
            epidemic: SirParams {
                infection_rate: 0.15,
                patient_zero: Some(0),
            },
            num_epidemics,
            min_epidemic_length: 1,
            max_epidemic_retries: 1,
        }
    }

    fn profile_match(target: Vec<f64>) -> EpiProfMatch {
        EpiProfMatch::new(certain_batch(1), 0, target).expect("valid target")
    }

    #[test]
    fn epi_spread_reads_total_ever_infected() {
        let objective = EpiSpread::new(certain_batch(3), 2026);

        assert_eq!(
            objective.evaluate(&path_graph(6)),
            6.0,
            "every node of the path is reached at rate 1.0",
        );
        assert_eq!(objective.direction(), Direction::Maximize);
    }

    #[test]
    fn epi_length_reads_timesteps_including_the_burnout_step() {
        let objective = EpiLength::new(certain_batch(3), 2026);

        assert_eq!(
            objective.evaluate(&path_graph(6)),
            6.0,
            "one step per edge, plus the burnout step (spec 5.2)",
        );
        assert_eq!(
            objective.evaluate(&Graph::new(4, 1)),
            1.0,
            "a lone patient zero still occupies the burnout step",
        );
        assert_eq!(objective.direction(), Direction::Maximize);
    }

    #[test]
    fn epi_prof_match_minimizes_and_scores_an_exact_match_at_zero() {
        let objective = profile_match(vec![1.0, 1.0, 1.0, 0.0]);
        let epidemic = Epidemic {
            length: 3,
            spread: 3,
            profile: vec![1, 1, 1, 0],
        };

        assert_eq!(objective.rmse(&epidemic), 0.0);
        assert_eq!(objective.direction(), Direction::Minimize);
    }

    /// The missing steps count as zero newly infected, not as absent.
    #[test]
    fn an_epidemic_shorter_than_the_target_is_penalised_for_the_remainder() {
        let objective = profile_match(vec![1.0, 2.0, 3.0, 4.0]);
        let epidemic = Epidemic {
            length: 1,
            spread: 3,
            profile: vec![1, 2],
        };

        // Squared error 0 + 0 + 9 + 16 = 25, over 4 steps, square-rooted.
        assert_eq!(objective.rmse(&epidemic), 2.5);
    }

    /// The deliberate asymmetry: overshoot is free — see `rmse`.
    #[test]
    fn an_epidemic_longer_than_the_target_is_not_penalised_for_the_surplus() {
        let objective = profile_match(vec![1.0, 2.0]);
        let short = Epidemic {
            length: 1,
            spread: 3,
            profile: vec![1, 2],
        };
        let long = Epidemic {
            length: 3,
            spread: 17,
            profile: vec![1, 2, 5, 9, 0],
        };

        assert_eq!(objective.rmse(&short), 0.0);
        assert_eq!(
            objective.rmse(&long),
            objective.rmse(&short),
            "the surplus beyond the target is ignored entirely",
        );
    }

    #[test]
    fn the_divisor_is_the_target_length_not_the_overlap() {
        // One matching step out of four. Were the divisor the overlap (2), the
        // score would be sqrt(9/2); it must be sqrt(9/4).
        let objective = profile_match(vec![1.0, 3.0, 0.0, 0.0]);
        let epidemic = Epidemic {
            length: 1,
            spread: 1,
            profile: vec![1, 0],
        };

        assert_eq!(objective.rmse(&epidemic), 1.5);
    }

    #[test]
    fn an_unusable_target_profile_is_rejected_at_construction() {
        assert!(
            EpiProfMatch::new(certain_batch(1), 0, Vec::new()).is_err(),
            "an empty target is the divisor of every RMSE, so it yields NaN",
        );
        assert!(EpiProfMatch::new(certain_batch(1), 0, vec![1.0, f64::NAN]).is_err());
        assert!(EpiProfMatch::new(certain_batch(1), 0, vec![1.0, f64::INFINITY]).is_err());
        assert!(EpiProfMatch::new(certain_batch(1), 0, vec![1.0, 2.0]).is_ok());
    }

    /// More epidemics must not change a deterministic reading.
    #[test]
    fn the_batch_mean_averages_the_epidemics() {
        let graph = path_graph(6);

        for num_epidemics in [1, 2, 7] {
            assert_eq!(
                EpiSpread::new(certain_batch(num_epidemics), 5).evaluate(&graph),
                6.0,
                "{num_epidemics} identical epidemics average to the same reading",
            );
        }
    }

    #[test]
    fn minimizing_leaves_the_oriented_value_equal_to_the_original() {
        assert_eq!(Direction::Minimize.orient(2.5), 2.5);
        assert_eq!(Direction::Minimize.orient(-2.5), -2.5);
        assert_eq!(Direction::Minimize.orient(f64::INFINITY), f64::INFINITY);
    }

    #[test]
    fn maximizing_makes_the_oriented_value_the_negated_original() {
        assert_eq!(Direction::Maximize.orient(2.5), -2.5);
        assert_eq!(Direction::Maximize.orient(f64::INFINITY), f64::NEG_INFINITY);
        // A better original must give a lower oriented value.
        assert!(Direction::Maximize.orient(9.0) < Direction::Maximize.orient(1.0));
    }

    #[test]
    fn orienting_an_oriented_value_gives_back_the_original() {
        for direction in [Direction::Minimize, Direction::Maximize] {
            for value in [0.0, 1.5, -7.25, f64::MAX, f64::INFINITY] {
                assert_eq!(direction.orient(direction.orient(value)), value);
            }
        }
    }

    #[test]
    fn the_default_direction_is_minimize() {
        struct Bare;
        impl Fitness for Bare {
            fn evaluate(&self, _graph: &Graph) -> f64 {
                0.0
            }
        }
        assert_eq!(Bare.direction(), Direction::Minimize);
    }

    #[test]
    #[should_panic(expected = "returned NaN")]
    fn orient_rejects_nan_when_minimizing() {
        Direction::Minimize.orient(f64::NAN);
    }

    #[test]
    #[should_panic(expected = "returned NaN")]
    fn orient_rejects_nan_when_maximizing() {
        Direction::Maximize.orient(f64::NAN);
    }

    /// Records why `orient` asserts rather than trusting the ordering: a `NaN`
    /// that got through under `Maximize` would sort **best**, not worst.
    #[test]
    fn an_unchecked_negated_nan_would_have_sorted_best() {
        let mut scores = vec![-f64::NAN, -100.0, 0.0, 100.0];
        // total_cmp: a total ordering for floats, unlike plain < where NaN is
        // unordered — needed so NaN actually sorts (first, here) instead of
        // being silently skipped.
        scores.sort_by(|a, b| a.total_cmp(b));
        assert!(
            scores[0].is_nan(),
            "negated NaN should sort first, got {scores:?}",
        );
    }

    /// Every pair of nodes joined, at multiplicity 1.
    fn complete_graph(num_nodes: usize) -> Graph {
        let mut graph = Graph::new(num_nodes, 1);
        for from in 0..num_nodes {
            for to in (from + 1)..num_nodes {
                graph.set_edge(from, to, 1);
            }
        }
        graph
    }

    /// A batch of `count` identical graphs.
    ///
    /// Complete rather than a path: at rate 0.5 a path's spread barely varies,
    /// and averaging over the epidemics quantizes two different batches onto
    /// the same score often enough to make a difference test useless.
    fn identical_batch(count: usize) -> Vec<Graph> {
        let mut graphs = Vec::with_capacity(count);
        for _ in 0..count {
            graphs.push(complete_graph(12));
        }
        graphs
    }

    #[test]
    fn one_batch_ticks_the_counter_once_however_many_graphs_it_holds() {
        let objective = EpiSpread::new(chancy_batch(3), 2026);

        objective.evaluate_batch(&identical_batch(6));

        assert_eq!(
            objective.scorer.batches_scored.load(Ordering::Relaxed),
            1,
            "the counter must advance per batch, not per graph",
        );
    }

    #[test]
    fn scoring_one_graph_ticks_the_counter_once_like_any_other_batch() {
        let objective = EpiSpread::new(chancy_batch(3), 2026);

        objective.evaluate(&complete_graph(12));

        assert_eq!(
            objective.scorer.batches_scored.load(Ordering::Relaxed),
            1,
            "a single graph is a batch of one, not a special case",
        );
    }

    /// `evaluate` and `evaluate_batch` must read the same thing off an
    /// epidemic. Each objective writes that reading twice, once per entry
    /// point, so this is what catches the two drifting apart.
    #[test]
    fn both_entry_points_use_the_same_reading() {
        let graph = path_graph(6);

        // certain_batch is deterministic, so the two differing seeds cannot
        // account for any difference in the scores.
        let spread = EpiSpread::new(certain_batch(2), 7);
        assert_eq!(
            spread.evaluate(&graph),
            spread.evaluate_batch(slice::from_ref(&graph))[0],
        );

        let length = EpiLength::new(certain_batch(2), 7);
        assert_eq!(
            length.evaluate(&graph),
            length.evaluate_batch(slice::from_ref(&graph))[0],
        );

        let profile = profile_match(vec![1.0, 1.0, 1.0]);
        assert_eq!(
            profile.evaluate(&graph),
            profile.evaluate_batch(slice::from_ref(&graph))[0],
        );
    }

    #[test]
    fn every_graph_in_one_batch_faces_the_same_epidemics() {
        let objective = EpiSpread::new(chancy_batch(3), 2026);

        let scores = objective.evaluate_batch(&identical_batch(6));

        for (i, score) in scores.iter().enumerate() {
            assert_eq!(
                *score, scores[0],
                "graph {i} of the batch drew different dice from graph 0",
            );
        }
    }

    #[test]
    fn consecutive_batches_face_different_epidemics() {
        let objective = EpiSpread::new(chancy_batch(3), 2026);
        let population = identical_batch(6);

        let first = objective.evaluate_batch(&population);
        let second = objective.evaluate_batch(&population);

        assert_ne!(
            first, second,
            "the dice never changed, so the run would optimize against one \
             frozen sample of the disease",
        );
    }

    /// Issue #18's own verification: one seed reproduces a whole run.
    ///
    /// Two objectives built identically score the same batches in the same
    /// order, and must agree score for score — not just on the first batch,
    /// which a frozen seed would also pass, but across a sequence long enough
    /// that the counter has advanced several times.
    #[test]
    fn the_same_run_seed_replays_every_batch_of_a_run() {
        let population = identical_batch(4);

        let first_run = EpiSpread::new(chancy_batch(3), 4242);
        let second_run = EpiSpread::new(chancy_batch(3), 4242);

        for batch in 0..5 {
            assert_eq!(
                first_run.evaluate_batch(&population),
                second_run.evaluate_batch(&population),
                "batch {batch} differed between two runs at the same seed",
            );
        }

        // And a different run seed must not replay it, or replicates would be
        // copies of each other rather than independent samples (§8.1). Both
        // sides are fresh, so this compares first batch against first batch.
        let this_seed = EpiSpread::new(chancy_batch(3), 4242);
        let other_seed = EpiSpread::new(chancy_batch(3), 4243);
        assert_ne!(
            this_seed.evaluate_batch(&population),
            other_seed.evaluate_batch(&population),
        );
    }

    /// The first `count` batch seeds a fresh scorer at `run_seed` hands out.
    fn first_batch_seeds(run_seed: u64, count: usize) -> Vec<u64> {
        let scorer = EpidemicScorer::new(certain_batch(1), run_seed);

        let mut seeds = Vec::with_capacity(count);
        for _ in 0..count {
            seeds.push(scorer.next_batch_seed());
        }
        seeds
    }

    #[test]
    fn one_run_seed_always_produces_the_same_batch_seed_sequence() {
        assert_eq!(
            first_batch_seeds(2026, 4),
            first_batch_seeds(2026, 4),
            "the same run seed must reproduce a run exactly",
        );
    }

    #[test]
    fn consecutive_batches_get_different_seeds() {
        let seeds = first_batch_seeds(2026, 4);

        for (i, seed) in seeds.iter().enumerate() {
            for (j, other) in seeds.iter().enumerate().skip(i + 1) {
                assert_ne!(
                    seed, other,
                    "batches {i} and {j} share a seed, so the run would \
                     optimize against one frozen sample of the disease",
                );
            }
        }
    }

    /// The property that rules out `run_seed ^ counter`: under xor, run seed
    /// `n`'s batch 1 is run seed `n + 1`'s batch 0, so two replicates replay
    /// each other's epidemics one batch out of step.
    #[test]
    fn neighbouring_run_seeds_share_no_batch_seed() {
        let mine = first_batch_seeds(2026, 4);
        let neighbour = first_batch_seeds(2027, 4);

        for (i, seed) in mine.iter().enumerate() {
            assert!(
                !neighbour.contains(seed),
                "batch {i} of run seed 2026 also appears in run seed 2027",
            );
        }
    }
}
