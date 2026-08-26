//! The objectives the GA optimizes, and the sign rule that makes their scores
//! comparable.

use std::slice;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use pyo3::prelude::*;
use rayon::prelude::*;

use crate::graph::Graph;
use crate::sir::{Epidemic, SirSampleParams, simulate_epidemics};
use crate::stats::{PerFamily, ReferenceStatistics};

/// Whether an objective wants its value small or large.
///
/// Every fitness number is in one of two forms, and mixing them up is the bug
/// this type exists to prevent:
///
/// - **as-measured** — what the fitness function returned. 28 nodes infected is
///   `28.0`, and bigger is better.
/// - **lower-is-better** — the same value through [`Direction::orient`]: that
///   28 becomes `-28.0`. The engine compares in these throughout, and the
///   boundary converts back.
///
/// The objective never negates its own output: the value and the declared
/// direction would then disagree, and a run optimizing backwards looks exactly
/// like one that is simply not converging.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// Smaller is better, as for an error or a distance. The default.
    Minimize,
    /// Larger is better.
    Maximize,
}

impl Direction {
    /// Convert between the two forms. The same call converts back.
    ///
    /// Panics on `NaN`: under [`Direction::Maximize`] it becomes `-NaN`, which
    /// sorts below `-inf`, so it would win every tournament it entered and
    /// leave a run that looks converged. An `assert!`, so it survives a release
    /// build.
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
/// [`Fitness::evaluate`] returns an **as-measured** score and
/// [`Fitness::direction`] says which way is better; the engine converts to
/// lower-is-better exactly once, so logs and results stay as-measured.
///
/// **Implement these, never call them.** Only `common::express_and_score` does,
/// and it is what converts and what rejects `NaN`; a direct call compiles,
/// skips both, and returns plausible numbers.
pub trait Fitness: Send + Sync {
    /// Score one graph, **as-measured** — never converted. Must not return
    /// `NaN`.
    fn evaluate(&self, graph: &Graph) -> f64;

    /// Which way is better. Defaults to [`Direction::Minimize`], so an error
    /// or distance objective says nothing.
    fn direction(&self) -> Direction {
        Direction::Minimize
    }

    /// Score a batch of graphs — whatever set the evolver scores together, from
    /// a whole population down to one steady-state pair.
    ///
    /// The default runs [`Fitness::evaluate`] on each graph across rayon.
    /// **A stochastic objective must override it**, or each graph draws its own
    /// randomness and scores stop being comparable within the batch; a Python
    /// one must too, to take the GIL once per batch rather than once per graph.
    fn evaluate_batch(&self, graphs: &[Graph]) -> Vec<f64> {
        graphs
            .par_iter()
            .map(|graph| self.evaluate(graph))
            .collect()
    }
}

/// A boxed objective is an objective.
///
/// The config layer erases its fitness variant to one `Box<dyn Fitness>`, and a
/// `Box` holding a `Fitness` is not itself one until this says so. It has to
/// live beside the trait — the orphan rule rejects it anywhere else.
///
/// **Every method is forwarded, including the two with defaults**, and both
/// omissions compile. Without `evaluate_batch` the box inherits the fan-out
/// default, which deadlocks a Python objective and re-seeds an epidemic one per
/// graph; without `direction` it reports [`Direction::Minimize`] whatever it
/// holds, running every maximizing search backwards.
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

/// Runs the epidemics that every SIR objective scores.
///
/// The epidemic is the expensive part and every SIR objective wants the same
/// one, so this runs the batch and each objective supplies only a reading.
///
/// **One scorer per run.** The batch counter is per-run state; two replicates
/// sharing a scorer would let thread scheduling decide which run saw which
/// seed, and reproducibility goes with it.
pub struct EpidemicScorer {
    params: SirSampleParams,
    run_seed: u64,
    batches_scored: AtomicU64,
}

impl EpidemicScorer {
    /// Build a scorer for one run.
    ///
    /// `run_seed` is this run's share of the master seed handed to
    /// `GraphEvolver::run`; `[fitness]` has no seed of its own.
    pub fn new(params: SirSampleParams, run_seed: u64) -> Self {
        Self {
            params,
            run_seed,
            batches_scored: AtomicU64::new(0),
        }
    }

    /// The seed for the next batch of graphs, advancing the counter so that
    /// every call returns a different one.
    ///
    /// **Call this once per batch, then give that one seed to every graph in
    /// the batch.** One seed across the batch, because those graphs are
    /// compared with each other: if each drew its own, a graph could rank first
    /// for having been handed a milder outbreak. A new seed for the next batch,
    /// because reusing one forever would breed a population good at that
    /// outbreak rather than good at the disease.

    pub(crate) fn next_batch_seed(&self) -> u64 {
        let counter = self.batches_scored.fetch_add(1, Ordering::Relaxed);
        mix_seed(self.run_seed, counter)
    }

    /// Score a whole batch of graphs — one seed for every graph, one tick of
    /// the counter. Every epidemic objective routes through here, which is what
    /// gives a batch common random numbers.
    ///
    /// `read` turns one epidemic into one number. Averaging matters: a single
    /// epidemic is noisy enough that selection would chase the dice instead of
    /// the graph. The division is safe — [`simulate_epidemics`] rejects an
    /// empty batch.
    pub fn mean_batch(&self, graphs: &[Graph], read: impl Fn(&Epidemic) -> f64 + Sync) -> Vec<f64> {
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
/// **Not `run_seed ^ counter`**: neighbouring run seeds would collide across
/// batch numbers, so two replicates would replay each other's epidemics one
/// batch apart.
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
/// 0.
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
/// The target is newly-infected counts, one per timestep. An epidemic's profile
/// starts with patient zero and ends with a terminating zero, so a target
/// captured from older output will not line up element for element.
pub struct EpiProfMatch {
    scorer: EpidemicScorer,
    target: Vec<f64>,
}

impl EpiProfMatch {
    /// Errors if `target` is empty or holds a non-finite value: either would put
    /// a `NaN` into every score, which [`Fitness`] forbids.
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
    /// **The target sets the comparison, not the epidemic**, so the scoring is
    /// asymmetric: an epidemic that ends early is penalised for the whole
    /// remaining target, while one that outlasts it is not penalised at all.
    /// This rewards matching *or exceeding* the tail.
    fn rmse(&self, epidemic: &Epidemic) -> f64 {
        let mut total = 0.0;

        for (step, wanted) in self.target.iter().enumerate() {
            // Past the end of the epidemic nobody was newly infected, so a
            // missing step counts as zero.
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

/// How closely a graph's structure matches a reference set of real graphs.
///
/// Three size-invariant statistics — degree, clustering and the normalized
/// Laplacian spectrum — are reduced to histograms on axes shared with the
/// reference set, and compared by an RBF kernel. Zero is a perfect match, so
/// this minimizes and says nothing about direction.
///
/// **A reference set can make a whole family inert, silently.** Rings and paths
/// have clustering coefficient 0 at every node. A reference set
/// drawn only from those leaves `clustering_weight` live while the family it
/// weights contributes nothing to any candidate's score, and nothing reports
/// it. Give the weights a reference set that varies in the statistic weighted.
pub struct StructMatch {
    reference: Arc<ReferenceStatistics>,
    gammas: PerFamily,
    weights: PerFamily,
    density_weight: f64,
}

impl StructMatch {
    /// Errors unless every gamma is finite and positive, every weight finite and
    /// non-negative, and at least one weight non-zero: each reaches `evaluate`
    /// as a multiplier, and all-zero weights score every candidate identically,
    /// so the search runs with no gradient while looking healthy.
    pub fn new(
        reference: Arc<ReferenceStatistics>,
        gammas: PerFamily,
        weights: PerFamily,
        density_weight: f64,
    ) -> Result<Self, &'static str> {
        let gamma_values = [gammas.degree, gammas.clustering, gammas.spectral];
        for gamma in gamma_values {
            if !gamma.is_finite() || gamma <= 0.0 {
                return Err("struct_match gammas must be finite and greater than zero");
            }
        }

        let weight_values = [weights.degree, weights.clustering, weights.spectral];
        let mut weight_total = 0.0;
        for weight in weight_values {
            if !weight.is_finite() || weight < 0.0 {
                return Err("struct_match weights must be finite and non-negative");
            }
            weight_total += weight;
        }
        if weight_total == 0.0 {
            return Err("struct_match weights must not all be zero");
        }

        if !density_weight.is_finite() || density_weight < 0.0 {
            return Err("struct_match density_weight must be finite and non-negative");
        }

        Ok(Self {
            reference,
            gammas,
            weights,
            density_weight,
        })
    }
}

impl Fitness for StructMatch {
    fn evaluate(&self, graph: &Graph) -> f64 {
        let kernel = self.reference.error(graph, self.gammas, self.weights);
        let penalty = self.reference.density_penalty(graph);

        kernel + self.density_weight * penalty
    }
}

// ADD AN OBJECTIVE STEP 1 — implement the trait for your own type, here beside
// the shipped objectives. Validate the objective's own inputs in its
// constructor and make it fallible if any are worth checking: the config
// layer's validation does not run when GET is used as a library.
//
//     impl Fitness for MyObjective {
//         fn evaluate(&self, graph: &Graph) -> f64 { ... }
//         fn direction(&self) -> Direction { Direction::Maximize }
//
//         fn evaluate_batch(&self, graphs: &[Graph]) -> Vec<f64> { ... }  // if stochastic
//     }

/// A user's Python callable, used as an objective. Registered at runtime
/// through `GraphEvolver::set_fitness_function`, with its [`Direction`] — that
/// cannot be inferred from a function.
///
/// **Calling it per graph deadlocks** rather than merely running slowly: the
/// trait's default `evaluate_batch` fans out over rayon, and each worker takes
/// the GIL while the calling thread holds it and blocks on rayon to finish.
/// Nothing fails — the run hangs with no message. So Python is never called
/// inside a rayon closure: expression fans out into a `Vec<Graph>` first, and
/// the single batched call happens here.
///
/// [`Fitness`]'s methods return `f64` with no `Result` path, so a callable that
/// raises, returns the wrong type, returns the wrong number of scores, or
/// returns `NaN` panics, naming what was expected and which item was at fault.
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
    /// Replicate runs need one objective each. The Python object is shared
    /// rather than copied — `clone_ref` bumps its refcount — which is correct
    /// here: it is per-run *scorer* state that must not be shared, and this has
    /// none.
    pub(crate) fn clone_ref(&self) -> Self {
        Python::attach(|py| Self {
            callable: self.callable.clone_ref(py),
            direction: self.direction,
        })
    }

    /// The one call into Python, which both trait methods route through.
    ///
    /// Inherent rather than having [`Fitness::evaluate`] call
    /// [`Fitness::evaluate_batch`]: that pairing is a latent stack overflow,
    /// because the trait's default `evaluate_batch` calls `evaluate`, so
    /// removing the override below turns the two into infinite recursion instead
    /// of a compile error.
    fn score_batch(&self, graphs: &[Graph]) -> Vec<f64> {
        // An empty batch never reaches the user's function.
        if graphs.is_empty() {
            return Vec::new();
        }

        // Built before the GIL is taken: holding it across plain Rust work
        // blocks every other Python thread for no reason.
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
            // number and cannot say which graph produced it.
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

    /// A star: node 0 joined to every other, and nothing else joined.
    ///
    /// The fixture that separates `length` from `spread`, which `path_graph`
    /// cannot: at rate 1.0 every leaf is infected in the same step, so a
    /// 6-node star is 6 nodes wide and 2 timesteps long. A path of 6 is 6 and
    /// 6, and so scores the same whichever field an objective reads.
    fn star_graph(num_nodes: usize) -> Graph {
        let mut graph = Graph::new(num_nodes, 1);
        for leaf in 1..num_nodes {
            graph.set_edge(0, leaf, 1);
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
        // The path cannot tell spread from length — it is 6 of both. The star
        // is 6 wide and 2 long, so only a reading of `spread` scores it 6.
        assert_eq!(
            objective.evaluate(&star_graph(6)),
            6.0,
            "all six nodes are infected, in two timesteps",
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
        // Both readings give 6 on the path, and 1 on an isolated node, so
        // neither case below can tell them apart. The star can: every leaf
        // falls in the same step, making it 6 nodes wide and 2 steps long.
        assert_eq!(
            objective.evaluate(&star_graph(6)),
            2.0,
            "one step to infect every leaf, plus the burnout step",
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
        // The star, not the path: a path scores 6 under either reading, so the
        // two entry points would agree here even if they read different fields.
        let graph = star_graph(6);

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

    /// The axes the struct_match tests share. Small bin counts keep the
    /// hand-checking tractable; the values themselves are not special.
    fn struct_match_axes() -> crate::stats::HistogramAxes {
        crate::stats::HistogramAxes {
            max_degree: 4,
            degree_bins: 5,
            clustering_bins: 4,
            spectral_bins: 4,
        }
    }

    /// A triangle — every node degree 2, clustering 1.0.
    fn triangle() -> Graph {
        let mut graph = Graph::new(3, 1);
        graph.set_edges(&[(0, 1, 1), (1, 2, 1), (0, 2, 1)]);
        graph
    }

    fn uniform(value: f64) -> PerFamily {
        PerFamily {
            degree: value,
            clustering: value,
            spectral: value,
        }
    }

    fn struct_match_over(reference: &[Graph]) -> StructMatch {
        let statistics = Arc::new(
            ReferenceStatistics::from_graphs(reference, struct_match_axes())
                .expect("a non-empty reference set on valid axes"),
        );
        StructMatch::new(statistics, uniform(1.0), uniform(1.0), 1.0)
            .expect("finite positive gammas and weights")
    }

    #[test]
    fn struct_match_minimizes_because_its_score_is_an_error() {
        // The default is Minimize, so this passes whether or not `direction`
        // is implemented. That is exactly why it is asserted rather than
        // assumed: if someone later adds a `direction` returning Maximize, the
        // search runs backwards and looks merely unconverged.
        let objective = struct_match_over(&[triangle()]);

        assert_eq!(objective.direction(), Direction::Minimize);
    }

    #[test]
    fn struct_match_scores_a_copy_of_its_reference_at_zero() {
        let objective = struct_match_over(&[triangle()]);

        let score = objective.evaluate(&triangle());

        // Identical histograms, identical density: both halves vanish.
        assert!(
            score.abs() < 1e-9,
            "a candidate identical to a single-graph reference should score ~0, got {score}"
        );
    }

    #[test]
    fn struct_match_scores_a_different_graph_worse_than_a_matching_one() {
        let objective = struct_match_over(&[triangle()]);

        let matching = objective.evaluate(&triangle());
        // Three isolated nodes: no edges, so no clustering, no density, and a
        // spectrum of three zeros rather than the triangle's.
        let different = objective.evaluate(&Graph::new(3, 1));

        assert!(
            different > matching,
            "an unrelated graph must score worse: {different} vs {matching}"
        );
        assert!(different.is_finite(), "score must never be non-finite");
    }

    #[test]
    fn struct_match_never_returns_a_non_finite_score() {
        // Direction::orient panics on NaN, so this is the guard that keeps a
        // bad candidate from aborting the whole run. The empty graph and a
        // graph with an isolated node are the two shapes that reach the
        // division-by-zero paths in the Laplacian.
        let objective = struct_match_over(&[triangle()]);

        let mut one_isolated = Graph::new(4, 1);
        one_isolated.set_edges(&[(0, 1, 1), (1, 2, 1), (0, 2, 1)]);

        for candidate in [Graph::new(0, 1), Graph::new(3, 1), one_isolated, triangle()] {
            let score = objective.evaluate(&candidate);
            assert!(
                score.is_finite(),
                "every candidate must score finitely, got {score} on a {}-node graph",
                candidate.num_nodes
            );
        }
    }

    #[test]
    fn struct_match_rejects_the_inputs_that_would_poison_every_score() {
        let axes = struct_match_axes();
        let statistics = || {
            Arc::new(
                ReferenceStatistics::from_graphs(&[triangle()], axes.clone())
                    .expect("a non-empty reference set"),
            )
        };

        // Each of these reaches `evaluate` as a multiplier, so a bad one is a
        // non-finite score on every candidate rather than a visible failure.
        assert!(StructMatch::new(statistics(), uniform(0.0), uniform(1.0), 1.0).is_err());
        assert!(StructMatch::new(statistics(), uniform(f64::NAN), uniform(1.0), 1.0).is_err());
        assert!(StructMatch::new(statistics(), uniform(-1.0), uniform(1.0), 1.0).is_err());
        assert!(StructMatch::new(statistics(), uniform(1.0), uniform(-1.0), 1.0).is_err());
        assert!(StructMatch::new(statistics(), uniform(1.0), uniform(f64::NAN), 1.0).is_err());
        assert!(StructMatch::new(statistics(), uniform(1.0), uniform(1.0), -1.0).is_err());
        assert!(StructMatch::new(statistics(), uniform(1.0), uniform(1.0), f64::INFINITY).is_err());

        // All-zero weights score every candidate identically: no gradient, and
        // a run that looks healthy while searching nothing.
        assert!(StructMatch::new(statistics(), uniform(1.0), uniform(0.0), 1.0).is_err());

        // A zero density weight is legitimate — it turns the penalty off.
        assert!(StructMatch::new(statistics(), uniform(1.0), uniform(1.0), 0.0).is_ok());
    }
}
