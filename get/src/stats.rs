//! Size-invariant structural statistics for a graph.
//!
//! Every statistic here reduces a graph to a **normalized histogram on a fixed
//! axis**: the bin edges are the same for every graph, whatever its node count.
//! That is what lets one candidate be compared against reference graphs of
//! many different sizes — the size stops entering the comparison at all,
//! because each graph contributes a probability distribution over the same
//! bins.
//!
//! Binning each graph over its own observed range would break exactly that.
//! Two histograms built on different axes can still be compared arithmetically,
//! and nothing about the result looks wrong: the numbers stay finite and the
//! score still moves. Bin *k* simply means a different thing on each side.
//!
//! All statistics here are **unweighted**. An edge is present or absent; a
//! multiplicity above one counts once. `Graph::degree` already counts distinct
//! neighbours, so this needs no special handling for degree.

use crate::graph::Graph;

/// Bin a value into `num_bins` bins spanning `[0.0, max]`, clamping anything at
/// or above `max` into the last bin.
///
/// Shared by every statistic so that no two of them can disagree about how a
/// value on a bin boundary is placed, or about what happens at the top of the
/// range.
fn bin_index(value: f64, max: f64, num_bins: usize) -> usize {
    if num_bins == 0 {
        return 0;
    }
    if !max.is_finite() || max <= 0.0 {
        return 0;
    }
    if !value.is_finite() || value <= 0.0 {
        return 0;
    }

    let scaled = value / max * num_bins as f64;
    if scaled >= num_bins as f64 {
        return num_bins - 1;
    }
    scaled as usize
}

/// Turn per-node counts into a probability distribution summing to 1.0.
///
/// A histogram with no observations is left as all zeros rather than being
/// given a uniform distribution: no observations is not the same claim as
/// "every bin equally likely", and the callers that can produce one are
/// documented where they occur.
fn normalize(hist: &mut [f64]) {
    let mut total = 0.0;
    for count in hist.iter() {
        total += *count;
    }
    if total > 0.0 {
        for count in hist.iter_mut() {
            *count /= total;
        }
    }
}

/// Degree histogram over the fixed absolute axis `[0, max_degree]`.
///
/// `max_degree` is supplied by the caller rather than taken from the graph, so
/// that a candidate and every reference graph land on one shared axis.
///
/// The axis is **absolute degree, not degree / (n - 1)**. Reference graphs here
/// have their degree bounded by a chemical valence, so an absolute degree is
/// already comparable across sizes and is the quantity that carries the
/// structure. Dividing by `n - 1` would instead call a degree-5 node in a
/// 12-node graph the same as a degree-20 node in a 45-node graph.
///
/// A degree above `max_degree` lands in the top bin rather than being dropped,
/// so the distribution always sums to 1.0 for a graph with at least one node.
pub fn degree_histogram(graph: &Graph, max_degree: usize, num_bins: usize) -> Vec<f64> {
    let mut hist = vec![0.0; num_bins];
    if num_bins == 0 || graph.num_nodes == 0 {
        return hist;
    }

    for node in 0..graph.num_nodes {
        let degree = graph.degree(node) as f64;
        // Degree 0 is a real observation — an isolated node — so it belongs in
        // bin 0 rather than being skipped.
        let index = bin_index(degree, max_degree as f64, num_bins);
        hist[index] += 1.0;
    }

    normalize(&mut hist);
    hist
}

/// The distinct neighbours of `node`, in increasing order.
///
/// `Graph` stores a dense weight matrix and does not expose it, so this walks
/// the row. That is the same cost the matrix representation already imposes on
/// every other traversal in the crate.
fn neighbours(graph: &Graph, node: usize) -> Vec<usize> {
    let mut result = Vec::new();
    for other in 0..graph.num_nodes {
        if other != node && graph.has_edge(node, other) {
            result.push(other);
        }
    }
    result
}

/// Local clustering coefficient of every node, histogrammed over the fixed
/// axis `[0, 1]`.
///
/// This family needs no size-invariance work: a clustering coefficient is
/// already a ratio in `[0, 1]`, whatever the node count. It is on a fixed axis
/// for the same reason as the others, not to repair anything.
///
/// A node with fewer than two neighbours has no pair of neighbours that could
/// be connected, so its coefficient is 0 — matching NetworkX. That is a real
/// observation and lands in bin 0 rather than being skipped, which is why a
/// graph too small to hold a triangle still produces a distribution summing to
/// 1.0 rather than a vector of zeros.
pub fn clustering_histogram(graph: &Graph, num_bins: usize) -> Vec<f64> {
    let mut hist = vec![0.0; num_bins];
    if num_bins == 0 || graph.num_nodes == 0 {
        return hist;
    }

    for node in 0..graph.num_nodes {
        let neighbours = neighbours(graph, node);
        let degree = neighbours.len();

        let coefficient = if degree < 2 {
            0.0
        } else {
            // Count the neighbour pairs that are themselves connected. Each
            // unordered pair is visited once, so no halving is needed beyond
            // the usual 2 * links / (k * (k - 1)).
            let mut links = 0;
            for i in 0..degree {
                for j in (i + 1)..degree {
                    if graph.has_edge(neighbours[i], neighbours[j]) {
                        links += 1;
                    }
                }
            }
            let possible = degree * (degree - 1);
            2.0 * links as f64 / possible as f64
        };

        let index = bin_index(coefficient, 1.0, num_bins);
        hist[index] += 1.0;
    }

    normalize(&mut hist);
    hist
}

/// Build the symmetric normalized Laplacian, `L = I - D^(-1/2) A D^(-1/2)`.
///
/// Unweighted: a present edge contributes 1 whatever its multiplicity.
///
/// **Isolated nodes are the hazard this function exists to contain.** The
/// candidate can strip a node's last edge mid-run, and `D^(-1/2)` is undefined
/// at degree zero, so the natural expression yields `inf * 0 = NaN`. A `NaN`
/// reaching the fitness value crashes the run rather than scoring it badly.
///
/// Taking `1/sqrt(0) = 0` — as NetworkX does — leaves an isolated node's row
/// and column entirely zero, so it contributes one zero eigenvalue. That is
/// consistent rather than a patch: an isolated node *is* a connected
/// component, and the multiplicity of eigenvalue 0 is exactly the number of
/// connected components.
fn normalized_laplacian(graph: &Graph) -> Vec<Vec<f64>> {
    let n = graph.num_nodes;
    let mut inverse_sqrt_degree = Vec::with_capacity(n);
    for node in 0..n {
        let degree = graph.degree(node);
        if degree > 0 {
            inverse_sqrt_degree.push(1.0 / (degree as f64).sqrt());
        } else {
            // Degree zero deliberately contributes 0.0 here, not infinity.
            inverse_sqrt_degree.push(0.0);
        }
    }

    let mut laplacian = vec![vec![0.0; n]; n];
    for u in 0..n {
        // An isolated node keeps an all-zero row, including the diagonal: it
        // has no self-similarity to subtract from.
        if graph.degree(u) > 0 {
            laplacian[u][u] = 1.0;
        }
        for v in 0..n {
            if u != v && graph.has_edge(u, v) {
                laplacian[u][v] -= inverse_sqrt_degree[u] * inverse_sqrt_degree[v];
            }
        }
    }
    laplacian
}

/// All eigenvalues of a real symmetric matrix, ascending, by cyclic Jacobi
/// rotation.
///
/// Jacobi is chosen over a general eigensolver because the input is always
/// small (one row per node) and always symmetric, which is the case Jacobi is
/// exact and unconditionally convergent for. Each rotation zeroes one
/// off-diagonal pair and preserves symmetry, so the matrix converges to a
/// diagonal one whose entries are the eigenvalues.
// The rotation below touches two specific rows and two specific columns of a
// square matrix, so indices are what the algorithm is written in terms of.
// Rewriting it as iterator chains would obscure it, and this crate prefers the
// explicit loop where the two disagree.
#[allow(clippy::needless_range_loop)]
fn symmetric_eigenvalues(mut matrix: Vec<Vec<f64>>) -> Vec<f64> {
    let n = matrix.len();
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![matrix[0][0]];
    }

    // Generous ceilings: a sweep costs O(n^2) rotations and convergence is
    // quadratic once the off-diagonal mass is small, so this is a guard
    // against a pathological input rather than the expected exit.
    const MAX_SWEEPS: usize = 100;
    const TOLERANCE: f64 = 1e-12;

    for _ in 0..MAX_SWEEPS {
        // Total off-diagonal magnitude: the quantity each rotation reduces.
        let mut off_diagonal = 0.0;
        for p in 0..n {
            for q in (p + 1)..n {
                off_diagonal += matrix[p][q] * matrix[p][q];
            }
        }
        if off_diagonal <= TOLERANCE {
            break;
        }

        for p in 0..n {
            for q in (p + 1)..n {
                if matrix[p][q].abs() <= TOLERANCE {
                    continue;
                }

                // The rotation angle that sends matrix[p][q] to exactly zero.
                let theta = (matrix[q][q] - matrix[p][p]) / (2.0 * matrix[p][q]);
                let sign = if theta >= 0.0 { 1.0 } else { -1.0 };
                let t = sign / (theta.abs() + (theta * theta + 1.0).sqrt());
                let cos = 1.0 / (t * t + 1.0).sqrt();
                let sin = t * cos;

                // Apply the rotation to rows p and q, then to columns p and q,
                // keeping the matrix symmetric throughout.
                for k in 0..n {
                    let a_kp = matrix[k][p];
                    let a_kq = matrix[k][q];
                    matrix[k][p] = cos * a_kp - sin * a_kq;
                    matrix[k][q] = sin * a_kp + cos * a_kq;
                }
                for k in 0..n {
                    let a_pk = matrix[p][k];
                    let a_qk = matrix[q][k];
                    matrix[p][k] = cos * a_pk - sin * a_qk;
                    matrix[q][k] = sin * a_pk + cos * a_qk;
                }
            }
        }
    }

    let mut eigenvalues = Vec::with_capacity(n);
    for (i, row) in matrix.iter().enumerate() {
        eigenvalues.push(row[i]);
    }
    eigenvalues.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    eigenvalues
}

/// Spectrum of the normalized Laplacian, histogrammed over the fixed axis
/// `[0, 2]`.
///
/// The axis is fixed at `[0, 2]` because that is where the normalized
/// Laplacian's eigenvalues lie for **any** graph of any size — the
/// size-invariance the other two families have natively.
///
/// This deliberately replaces the more usual "first *k* eigenvalues of the
/// combinatorial Laplacian". An *n*-node graph has *n* eigenvalues, so a
/// fixed-length prefix compares different fractions of two spectra, and the
/// prefix is dominated by the zeros — one per connected component — rather
/// than by anything structural. A histogram uses the whole spectrum and stays
/// the same length whatever *n* is.
///
/// Bin 0 carries every eigenvalue below `2 / num_bins`, so it *contains* the
/// exact zeros — one per connected component — without being only them. A
/// connected graph whose spectrum crowds near zero puts several eigenvalues
/// there as well: a 12-node path on 8 bins contributes three, not one. So a
/// heavy bin 0 tracks fragmentation rather than counting it, and the component
/// count is the zero multiplicity alone, which this histogram does not isolate.
/// Read it as a signal, never as `components / n`.
pub fn spectral_histogram(graph: &Graph, num_bins: usize) -> Vec<f64> {
    let mut hist = vec![0.0; num_bins];
    if num_bins == 0 || graph.num_nodes == 0 {
        return hist;
    }

    let eigenvalues = symmetric_eigenvalues(normalized_laplacian(graph));
    for value in &eigenvalues {
        // Rounding can push an eigenvalue a hair outside [0, 2]; clamping
        // keeps every one of them an observation instead of silently dropping
        // the ones at the ends, which are the structurally meaningful ones.
        let clamped = value.clamp(0.0, 2.0);
        let index = bin_index(clamped, 2.0, num_bins);
        hist[index] += 1.0;
    }

    normalize(&mut hist);
    hist
}

/// Fraction of the possible edges that are present, `2m / (n(n-1))`.
///
/// A density rather than an edge count, because an edge count is not
/// comparable across graphs of different sizes: the source penalises distance
/// from an average edge count taken over reference graphs of many sizes, which
/// makes the target wrong for any candidate whose node count differs from the
/// reference mean. A density is unchanged when a graph and its target scale
/// together.
///
/// Unweighted, like everything else here: multiplicity does not add edges.
pub fn density(graph: &Graph) -> f64 {
    let n = graph.num_nodes;
    if n < 2 {
        return 0.0;
    }

    let mut edges = 0.0;
    for u in 0..n {
        for v in (u + 1)..n {
            if graph.has_edge(u, v) {
                edges += 1.0;
            }
        }
    }

    let possible = n as f64 * (n as f64 - 1.0) / 2.0;
    edges / possible
}

/// Why a reference set could not be turned into statistics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatsError {
    /// No reference graphs were supplied.
    ///
    /// This is an error and not a score. The source returns a *perfect* score
    /// for an empty reference set, so a mistyped or empty reference folder
    /// makes every candidate ideal, the population converges immediately, and
    /// the run looks healthy while measuring nothing.
    EmptyReferenceSet,
    /// A family was configured with zero bins, which leaves nothing to compare.
    ZeroBins { family: &'static str },
}

impl std::fmt::Display for StatsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StatsError::EmptyReferenceSet => write!(
                f,
                "the reference set is empty, so there is nothing to compare a candidate against"
            ),
            StatsError::ZeroBins { family } => {
                write!(f, "the {family} statistic was configured with zero bins")
            }
        }
    }
}

impl std::error::Error for StatsError {}

/// The shared axes every graph is binned onto — the candidate and every
/// reference graph alike, so the two can never bin differently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistogramAxes {
    /// Top of the degree axis. Degrees above it land in the last bin.
    pub max_degree: usize,
    pub degree_bins: usize,
    pub clustering_bins: usize,
    pub spectral_bins: usize,
}

/// A per-family value: the three statistics each take their own.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PerFamily {
    pub degree: f64,
    pub clustering: f64,
    pub spectral: f64,
}

/// One statistic family's view of the reference set.
#[derive(Debug, Clone)]
struct FamilyReference {
    /// One standardized histogram per reference graph.
    standardized: Vec<Vec<f64>>,
    /// Per-bin mean across the reference set, used to standardize a candidate.
    mean: Vec<f64>,
    /// Per-bin standard deviation, already guarded against zero.
    deviation: Vec<f64>,
}

/// Added to every per-bin standard deviation before dividing by it.
///
/// A bin that is identical across every reference graph has deviation exactly
/// zero, and dividing by it would put a `NaN` into the fitness value — which
/// crashes a run rather than scoring it badly. The guard also carries a
/// meaning worth keeping: a candidate that differs on a bin the reference set
/// is unanimous about is divided by a very small number, so it is pushed far
/// away, which is the right verdict.
const DEVIATION_FLOOR: f64 = 1e-6;

impl FamilyReference {
    /// Standardize each bin against the reference set's own mean and deviation
    /// for that bin.
    ///
    /// This is what makes the score weight *what the reference set agrees
    /// about*: a bin the reference graphs vary widely on contributes less to
    /// the distance than one they all agree on. An unstandardized kernel would
    /// treat every bin alike.
    fn build(histograms: Vec<Vec<f64>>) -> Self {
        let count = histograms.len() as f64;
        let bins = histograms[0].len();

        let mut mean = vec![0.0; bins];
        for histogram in &histograms {
            for bin in 0..bins {
                mean[bin] += histogram[bin];
            }
        }
        for value in mean.iter_mut() {
            *value /= count;
        }

        let mut deviation = vec![0.0; bins];
        for histogram in &histograms {
            for bin in 0..bins {
                let difference = histogram[bin] - mean[bin];
                deviation[bin] += difference * difference;
            }
        }
        for value in deviation.iter_mut() {
            *value = (*value / count).sqrt() + DEVIATION_FLOOR;
        }

        let mut standardized = Vec::with_capacity(histograms.len());
        for histogram in &histograms {
            standardized.push(standardize(histogram, &mean, &deviation));
        }

        Self {
            standardized,
            mean,
            deviation,
        }
    }

    /// `1 - mean RBF similarity` between a candidate histogram and every
    /// reference histogram.
    ///
    /// Deliberately **not** MMD, and not named as such. MMD is a two-sample
    /// statistic; this compares one candidate against a fixed reference
    /// distribution, so the within-reference term is a constant and the
    /// candidate's self-similarity is 1. Both quantities are strictly
    /// decreasing in the mean similarity, so candidates rank identically —
    /// the simplification is sound, but the name would claim something else.
    fn error(&self, histogram: &[f64], gamma: f64) -> f64 {
        let candidate = standardize(histogram, &self.mean, &self.deviation);

        let mut total_similarity = 0.0;
        for reference in &self.standardized {
            let mut squared_distance = 0.0;
            for bin in 0..candidate.len() {
                let difference = candidate[bin] - reference[bin];
                squared_distance += difference * difference;
            }
            total_similarity += (-gamma * squared_distance).exp();
        }

        1.0 - total_similarity / self.standardized.len() as f64
    }
}

/// Z-score a histogram bin by bin against a supplied mean and deviation.
fn standardize(histogram: &[f64], mean: &[f64], deviation: &[f64]) -> Vec<f64> {
    let mut result = Vec::with_capacity(histogram.len());
    for bin in 0..histogram.len() {
        result.push((histogram[bin] - mean[bin]) / deviation[bin]);
    }
    result
}

/// A reference set reduced to the statistics a candidate is scored against.
///
/// Built once per run: the reference graphs never change, so their histograms
/// and per-bin moments are computed here rather than on every evaluation.
#[derive(Debug, Clone)]
pub struct ReferenceStatistics {
    degree: FamilyReference,
    clustering: FamilyReference,
    spectral: FamilyReference,
    mean_density: f64,
    axes: HistogramAxes,
}

impl ReferenceStatistics {
    /// Reduce a reference set to its statistics, on the supplied shared axes.
    pub fn from_graphs(graphs: &[Graph], axes: HistogramAxes) -> Result<Self, StatsError> {
        if graphs.is_empty() {
            return Err(StatsError::EmptyReferenceSet);
        }
        if axes.degree_bins == 0 {
            return Err(StatsError::ZeroBins { family: "degree" });
        }
        if axes.clustering_bins == 0 {
            return Err(StatsError::ZeroBins {
                family: "clustering",
            });
        }
        if axes.spectral_bins == 0 {
            return Err(StatsError::ZeroBins { family: "spectral" });
        }

        let mut degree = Vec::with_capacity(graphs.len());
        let mut clustering = Vec::with_capacity(graphs.len());
        let mut spectral = Vec::with_capacity(graphs.len());
        let mut density_total = 0.0;

        for graph in graphs {
            degree.push(degree_histogram(graph, axes.max_degree, axes.degree_bins));
            clustering.push(clustering_histogram(graph, axes.clustering_bins));
            spectral.push(spectral_histogram(graph, axes.spectral_bins));
            density_total += density(graph);
        }

        Ok(Self {
            degree: FamilyReference::build(degree),
            clustering: FamilyReference::build(clustering),
            spectral: FamilyReference::build(spectral),
            mean_density: density_total / graphs.len() as f64,
            axes,
        })
    }

    /// The axes this reference set was built on. A candidate must be binned on
    /// the same ones, which is why they are carried here rather than passed
    /// again at scoring time.
    pub fn axes(&self) -> &HistogramAxes {
        &self.axes
    }

    /// Mean density across the reference set — the density penalty's target.
    pub fn mean_density(&self) -> f64 {
        self.mean_density
    }

    /// How far a candidate is from this reference set: a weighted sum of the
    /// three family errors and the density penalty. Zero is a perfect match.
    ///
    /// `gammas` are the RBF bandwidths, one per family. They are per-family and
    /// small on purpose: a gamma that is too large collapses
    /// `exp(-gamma * d^2)` to zero for every candidate, so the whole population
    /// scores ~1.0, there is no gradient to climb, and evolution stalls while
    /// appearing to run normally.
    ///
    /// The result is always finite. Every route to a `NaN` is closed —
    /// the deviation floor, the isolated-node rule in the Laplacian, and the
    /// empty reference set being rejected at construction — because a `NaN`
    /// fitness aborts a run instead of scoring a bad candidate.
    pub fn error(&self, candidate: &Graph, gammas: PerFamily, weights: PerFamily) -> f64 {
        let degree_histogram_of_candidate =
            degree_histogram(candidate, self.axes.max_degree, self.axes.degree_bins);
        let clustering_of_candidate = clustering_histogram(candidate, self.axes.clustering_bins);
        let spectral_of_candidate = spectral_histogram(candidate, self.axes.spectral_bins);

        let degree_error = self
            .degree
            .error(&degree_histogram_of_candidate, gammas.degree);
        let clustering_error = self
            .clustering
            .error(&clustering_of_candidate, gammas.clustering);
        let spectral_error = self.spectral.error(&spectral_of_candidate, gammas.spectral);

        weights.degree * degree_error
            + weights.clustering * clustering_error
            + weights.spectral * spectral_error
    }

    /// Absolute distance between a candidate's density and the reference mean.
    ///
    /// Kept separate from `error` so its weight can be set independently, and
    /// so a caller can report the two halves apart when diagnosing a run.
    pub fn density_penalty(&self, candidate: &Graph) -> f64 {
        (density(candidate) - self.mean_density).abs()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        HistogramAxes, PerFamily, ReferenceStatistics, StatsError, bin_index, clustering_histogram,
        degree_histogram, density, normalize, normalized_laplacian, spectral_histogram,
        symmetric_eigenvalues,
    };
    use crate::graph::Graph;

    /// Sum of a histogram, for the "is a probability distribution" checks.
    fn total(hist: &[f64]) -> f64 {
        let mut sum = 0.0;
        for value in hist {
            sum += *value;
        }
        sum
    }

    /// A path 0 - 1 - 2: degrees 1, 2, 1.
    fn path_of_three() -> Graph {
        let mut graph = Graph::new(3, 1);
        graph.add_edge(0, 1);
        graph.add_edge(1, 2);
        graph
    }

    #[test]
    fn bin_index_places_zero_in_the_first_bin_and_the_top_value_in_the_last() {
        assert_eq!(bin_index(0.0, 4.0, 4), 0);
        assert_eq!(bin_index(4.0, 4.0, 4), 3);
        // Above the top of the axis is clamped, never dropped or out of range.
        assert_eq!(bin_index(9.0, 4.0, 4), 3);
    }

    #[test]
    fn bin_index_is_defined_on_degenerate_axes() {
        assert_eq!(bin_index(1.0, 0.0, 4), 0);
        assert_eq!(bin_index(1.0, 4.0, 0), 0);
        assert_eq!(bin_index(f64::NAN, 4.0, 4), 0);
    }

    #[test]
    fn normalize_leaves_an_all_zero_histogram_alone() {
        let mut hist = vec![0.0; 3];
        normalize(&mut hist);
        assert_eq!(hist, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn degree_histogram_counts_every_node_once() {
        let graph = path_of_three();
        let hist = degree_histogram(&graph, 4, 5);

        // Degrees are 1, 2, 1 — so two thirds at degree 1, one third at 2.
        assert!((total(&hist) - 1.0).abs() < 1e-12);
        assert!((hist[1] - 2.0 / 3.0).abs() < 1e-12);
        assert!((hist[2] - 1.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn an_isolated_node_is_an_observation_in_bin_zero() {
        let mut graph = Graph::new(3, 1);
        graph.add_edge(0, 1);
        // Node 2 has no edges at all.
        let hist = degree_histogram(&graph, 4, 5);

        assert!((hist[0] - 1.0 / 3.0).abs() < 1e-12);
        assert!((total(&hist) - 1.0).abs() < 1e-12);
    }

    /// The property the whole module exists for: two graphs of different sizes
    /// and different maximum degrees produce histograms on the same axis, so
    /// bin *k* means the same thing in both.
    #[test]
    fn two_graphs_of_different_size_share_one_axis() {
        let small = path_of_three();

        // A star on 6 nodes: centre has degree 5, every leaf degree 1.
        let mut large = Graph::new(6, 1);
        for leaf in 1..6 {
            large.add_edge(0, leaf);
        }

        let small_hist = degree_histogram(&small, 5, 6);
        let large_hist = degree_histogram(&large, 5, 6);

        assert_eq!(small_hist.len(), large_hist.len());
        // The star's centre is the only degree-5 node, and it lands in the bin
        // for degree 5 — which the 3-node graph leaves empty rather than
        // rescaling its own maximum onto.
        assert!((large_hist[5] - 1.0 / 6.0).abs() < 1e-12);
        assert_eq!(small_hist[5], 0.0);
        assert!((total(&small_hist) - 1.0).abs() < 1e-12);
        assert!((total(&large_hist) - 1.0).abs() < 1e-12);
    }

    /// Multiplicity must not reach the degree axis: a doubled edge is one
    /// neighbour, so this graph has to bin identically to the simple path.
    #[test]
    fn edge_multiplicity_does_not_change_the_degree_histogram() {
        let mut weighted = Graph::new(3, 5);
        weighted.set_edge(0, 1, 4);
        weighted.set_edge(1, 2, 3);

        assert_eq!(
            degree_histogram(&weighted, 4, 5),
            degree_histogram(&path_of_three(), 4, 5)
        );
    }

    #[test]
    fn an_empty_graph_produces_an_all_zero_histogram_rather_than_a_panic() {
        let graph = Graph::new(0, 1);
        let hist = degree_histogram(&graph, 4, 5);

        assert_eq!(hist.len(), 5);
        assert_eq!(total(&hist), 0.0);
    }

    /// A triangle: every node's two neighbours are joined, so every
    /// coefficient is exactly 1.0.
    #[test]
    fn a_triangle_puts_every_node_at_clustering_one() {
        let mut graph = Graph::new(3, 1);
        graph.add_edge(0, 1);
        graph.add_edge(1, 2);
        graph.add_edge(0, 2);

        let hist = clustering_histogram(&graph, 4);

        // 1.0 is the top of the axis, so it lands in the last bin.
        assert!((hist[3] - 1.0).abs() < 1e-12);
        assert!((total(&hist) - 1.0).abs() < 1e-12);
    }

    /// The path 0 - 1 - 2: the centre has two neighbours that are not joined,
    /// and each end has only one neighbour. Every coefficient is 0.0.
    #[test]
    fn a_path_of_three_puts_every_node_at_clustering_zero() {
        let hist = clustering_histogram(&path_of_three(), 4);

        assert!((hist[0] - 1.0).abs() < 1e-12);
        assert!((total(&hist) - 1.0).abs() < 1e-12);
    }

    /// A graph too small to contain a triangle still yields a distribution
    /// summing to 1.0, rather than the all-zero vector the source returns for
    /// fewer than three nodes.
    #[test]
    fn a_two_node_graph_is_still_a_distribution() {
        let mut graph = Graph::new(2, 1);
        graph.add_edge(0, 1);

        let hist = clustering_histogram(&graph, 4);

        assert!((total(&hist) - 1.0).abs() < 1e-12);
        assert!((hist[0] - 1.0).abs() < 1e-12);
    }

    /// One node of the four sits in a triangle-free corner, so the histogram
    /// has to carry two different coefficients rather than collapsing.
    #[test]
    fn clustering_separates_nodes_with_different_coefficients() {
        // Triangle 0-1-2, plus a pendant 3 hanging off node 0.
        let mut graph = Graph::new(4, 1);
        graph.add_edge(0, 1);
        graph.add_edge(1, 2);
        graph.add_edge(0, 2);
        graph.add_edge(0, 3);

        let hist = clustering_histogram(&graph, 4);

        // Node 0 has neighbours 1, 2, 3 with one link among them: 2*1/(3*2) = 1/3.
        // Nodes 1 and 2 are at 1.0; node 3 has one neighbour, so 0.0.
        assert!((hist[0] - 0.25).abs() < 1e-12); // node 3
        assert!((hist[1] - 0.25).abs() < 1e-12); // node 0, at 1/3
        assert!((hist[3] - 0.5).abs() < 1e-12); // nodes 1 and 2
        assert!((total(&hist) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn edge_multiplicity_does_not_change_the_clustering_histogram() {
        let mut weighted = Graph::new(3, 5);
        weighted.set_edge(0, 1, 4);
        weighted.set_edge(1, 2, 2);

        assert_eq!(
            clustering_histogram(&weighted, 4),
            clustering_histogram(&path_of_three(), 4)
        );
    }

    /// Count how many eigenvalues are zero, which is the connected-component
    /// count for the normalized Laplacian.
    fn zero_eigenvalue_count(graph: &Graph) -> usize {
        let values = symmetric_eigenvalues(normalized_laplacian(graph));
        let mut count = 0;
        for value in &values {
            if value.abs() < 1e-9 {
                count += 1;
            }
        }
        count
    }

    /// The bound the fixed axis depends on: every eigenvalue of the normalized
    /// Laplacian lies in [0, 2], whatever the graph.
    #[test]
    fn every_eigenvalue_lies_in_zero_to_two() {
        let mut complete = Graph::new(7, 1);
        for u in 0..7 {
            for v in (u + 1)..7 {
                complete.add_edge(u, v);
            }
        }

        let mut star = Graph::new(9, 1);
        for leaf in 1..9 {
            star.add_edge(0, leaf);
        }

        for graph in [complete, star, path_of_three()] {
            for value in symmetric_eigenvalues(normalized_laplacian(&graph)) {
                assert!(value.is_finite(), "eigenvalue was not finite");
                assert!(
                    (-1e-9..=2.0 + 1e-9).contains(&value),
                    "eigenvalue {value} outside [0, 2]"
                );
            }
        }
    }

    /// A complete graph K_n has eigenvalue 0 once and n/(n-1) with multiplicity
    /// n-1 — a spectrum known in closed form, so this pins the solver itself
    /// rather than just its self-consistency.
    #[test]
    fn a_complete_graph_matches_its_closed_form_spectrum() {
        let n = 5;
        let mut complete = Graph::new(n, 1);
        for u in 0..n {
            for v in (u + 1)..n {
                complete.add_edge(u, v);
            }
        }

        let values = symmetric_eigenvalues(normalized_laplacian(&complete));
        let expected = n as f64 / (n as f64 - 1.0);

        assert!(
            values[0].abs() < 1e-9,
            "smallest should be 0, was {}",
            values[0]
        );
        for value in &values[1..] {
            assert!(
                (value - expected).abs() < 1e-9,
                "expected {expected}, got {value}"
            );
        }
    }

    /// An isolated node must not produce a NaN: 1/sqrt(0) is taken as 0, so the
    /// node contributes one zero eigenvalue instead of an undefined one.
    #[test]
    fn an_isolated_node_scores_without_nan() {
        let mut graph = Graph::new(4, 1);
        graph.add_edge(0, 1);
        graph.add_edge(1, 2);
        // Node 3 is isolated.

        let hist = spectral_histogram(&graph, 10);
        for value in &hist {
            assert!(value.is_finite(), "histogram carried a non-finite value");
        }
        assert!((total(&hist) - 1.0).abs() < 1e-12);

        // Two components: the path 0-1-2, and the isolated node 3.
        assert_eq!(zero_eigenvalue_count(&graph), 2);
    }

    /// Three disjoint components, and the zero-eigenvalue multiplicity has to
    /// equal the component count exactly.
    #[test]
    fn three_components_give_three_zero_eigenvalues_and_no_nan() {
        let mut graph = Graph::new(9, 1);
        graph.add_edge(0, 1);
        graph.add_edge(1, 2);
        graph.add_edge(3, 4);
        graph.add_edge(4, 5);
        graph.add_edge(6, 7);
        graph.add_edge(7, 8);

        let hist = spectral_histogram(&graph, 10);
        for value in &hist {
            assert!(value.is_finite());
        }
        assert!((total(&hist) - 1.0).abs() < 1e-12);
        assert_eq!(zero_eigenvalue_count(&graph), 3);

        // Bin 0 holds 3/9 here, but only because it happens to catch nothing
        // besides the zeros: a 3-node path's other eigenvalues are 1 and 2, and
        // the first bin ends at 0.2. That coincidence is not the general rule —
        // see `bin_zero_is_a_range_not_the_component_count`.
        assert!((hist[0] - 3.0 / 9.0).abs() < 1e-12);
    }

    /// Bin 0 is a range, and reading it as the component count is wrong the
    /// moment a connected graph has eigenvalues near zero.
    ///
    /// A 12-node path is **one** component, so `components / n` would be 1/12.
    /// Bin 0 holds 3/12, because 0.0405 and 0.1587 also fall below the first
    /// boundary at 2/8 = 0.25. The three cases above all pin graphs whose
    /// nonzero eigenvalues sit far from zero, so none of them can catch this.
    #[test]
    fn bin_zero_is_a_range_not_the_component_count() {
        let mut path = Graph::new(12, 1);
        for i in 0..11 {
            path.add_edge(i, i + 1);
        }

        assert_eq!(zero_eigenvalue_count(&path), 1);

        let hist = spectral_histogram(&path, 8);
        assert!(
            (hist[0] - 3.0 / 12.0).abs() < 1e-12,
            "bin 0 was {}, expected 3/12 — three eigenvalues below 0.25",
            hist[0]
        );
    }

    /// A graph with no edges at all is every node isolated: n components, so
    /// every eigenvalue is zero and nothing is undefined.
    #[test]
    fn a_graph_with_no_edges_is_all_zero_eigenvalues() {
        let graph = Graph::new(5, 1);

        assert_eq!(zero_eigenvalue_count(&graph), 5);
        let hist = spectral_histogram(&graph, 10);
        assert!((hist[0] - 1.0).abs() < 1e-12);
        assert!((total(&hist) - 1.0).abs() < 1e-12);
    }

    /// The whole point of the fixed axis, checked on the spectral family: two
    /// graphs with different node counts produce histograms of the same length
    /// on the same axis.
    #[test]
    fn spectra_of_different_sized_graphs_share_one_axis() {
        let small = path_of_three();
        let mut large = Graph::new(12, 1);
        for i in 0..11 {
            large.add_edge(i, i + 1);
        }

        let small_hist = spectral_histogram(&small, 8);
        let large_hist = spectral_histogram(&large, 8);

        assert_eq!(small_hist.len(), large_hist.len());
        assert!((total(&small_hist) - 1.0).abs() < 1e-12);
        assert!((total(&large_hist) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn edge_multiplicity_does_not_change_the_spectral_histogram() {
        let mut weighted = Graph::new(3, 5);
        weighted.set_edge(0, 1, 5);
        weighted.set_edge(1, 2, 2);

        assert_eq!(
            spectral_histogram(&weighted, 8),
            spectral_histogram(&path_of_three(), 8)
        );
    }

    fn axes() -> HistogramAxes {
        HistogramAxes {
            max_degree: 6,
            degree_bins: 7,
            clustering_bins: 5,
            spectral_bins: 8,
        }
    }

    fn gammas() -> PerFamily {
        PerFamily {
            degree: 0.01,
            clustering: 0.01,
            spectral: 0.01,
        }
    }

    fn equal_weights() -> PerFamily {
        PerFamily {
            degree: 1.0,
            clustering: 1.0,
            spectral: 1.0,
        }
    }

    fn ring(n: usize) -> Graph {
        let mut graph = Graph::new(n, 1);
        for i in 0..n {
            graph.add_edge(i, (i + 1) % n);
        }
        graph
    }

    // --- density ---

    #[test]
    fn density_is_the_fraction_of_possible_edges_present() {
        // A triangle is complete on 3 nodes.
        let mut triangle = Graph::new(3, 1);
        triangle.add_edge(0, 1);
        triangle.add_edge(1, 2);
        triangle.add_edge(0, 2);
        assert!((density(&triangle) - 1.0).abs() < 1e-12);

        // The path 0-1-2 has 2 of 3 possible edges.
        assert!((density(&path_of_three()) - 2.0 / 3.0).abs() < 1e-12);

        assert_eq!(density(&Graph::new(5, 1)), 0.0);
    }

    /// The property the density penalty exists for: an absolute edge count is
    /// not comparable across sizes, but a density is unchanged when a graph
    /// and its target scale together.
    #[test]
    fn density_is_unchanged_when_a_graph_scales() {
        // Complete graphs of very different sizes both have density 1.0, while
        // their edge counts are 10 and 105.
        let mut small = Graph::new(5, 1);
        for u in 0..5 {
            for v in (u + 1)..5 {
                small.add_edge(u, v);
            }
        }
        let mut large = Graph::new(15, 1);
        for u in 0..15 {
            for v in (u + 1)..15 {
                large.add_edge(u, v);
            }
        }

        assert!((density(&small) - density(&large)).abs() < 1e-12);
    }

    #[test]
    fn edge_multiplicity_does_not_change_density() {
        let mut weighted = Graph::new(3, 5);
        weighted.set_edge(0, 1, 5);
        weighted.set_edge(1, 2, 3);

        assert!((density(&weighted) - density(&path_of_three())).abs() < 1e-12);
    }

    // --- reference set construction ---

    /// An empty reference set is an error, not a perfect score. The source
    /// returns 0.0 here, which makes a mistyped reference folder look like a
    /// solved problem.
    #[test]
    fn an_empty_reference_set_is_an_error_not_a_perfect_score() {
        let result = ReferenceStatistics::from_graphs(&[], axes());
        assert_eq!(result.unwrap_err(), StatsError::EmptyReferenceSet);
    }

    #[test]
    fn a_family_with_zero_bins_is_an_error() {
        let mut broken = axes();
        broken.spectral_bins = 0;

        let result = ReferenceStatistics::from_graphs(&[ring(6)], broken);
        assert_eq!(
            result.unwrap_err(),
            StatsError::ZeroBins { family: "spectral" }
        );
    }

    #[test]
    fn the_reference_mean_density_is_the_mean_of_its_graphs() {
        let reference = vec![path_of_three(), ring(3)];
        let stats = ReferenceStatistics::from_graphs(&reference, axes()).unwrap();

        // 2/3 and 1.0.
        let expected = (2.0 / 3.0 + 1.0) / 2.0;
        assert!((stats.mean_density() - expected).abs() < 1e-12);
    }

    // --- the kernel score ---

    /// A candidate identical to a single-graph reference set scores ~0.
    #[test]
    fn a_candidate_matching_its_only_reference_scores_about_zero() {
        let reference = vec![ring(8)];
        let stats = ReferenceStatistics::from_graphs(&reference, axes()).unwrap();

        let error = stats.error(&ring(8), gammas(), equal_weights());

        assert!(error.is_finite());
        assert!(error.abs() < 1e-9, "expected ~0, got {error}");
    }

    /// A structurally very different candidate scores worse than a matching
    /// one — the ordering the objective depends on.
    #[test]
    fn a_dissimilar_candidate_scores_worse_than_a_matching_one() {
        let reference = vec![ring(10), ring(12), ring(14)];
        let stats = ReferenceStatistics::from_graphs(&reference, axes()).unwrap();

        let matching = stats.error(&ring(11), gammas(), equal_weights());

        // A complete graph is about as unlike a ring as a graph gets.
        let mut complete = Graph::new(11, 1);
        for u in 0..11 {
            for v in (u + 1)..11 {
                complete.add_edge(u, v);
            }
        }
        let dissimilar = stats.error(&complete, gammas(), equal_weights());

        assert!(matching.is_finite() && dissimilar.is_finite());
        assert!(
            dissimilar > matching,
            "a complete graph scored {dissimilar}, a ring scored {matching}"
        );
    }

    /// Every route to a NaN is closed. These are the inputs that would find
    /// one: a graph with no edges, a complete graph, a single node, and a
    /// candidate carrying isolated nodes.
    #[test]
    fn no_candidate_produces_a_non_finite_score() {
        let reference = vec![ring(8), path_of_three(), ring(5)];
        let stats = ReferenceStatistics::from_graphs(&reference, axes()).unwrap();

        let mut complete = Graph::new(9, 1);
        for u in 0..9 {
            for v in (u + 1)..9 {
                complete.add_edge(u, v);
            }
        }

        let mut partly_isolated = Graph::new(7, 1);
        partly_isolated.add_edge(0, 1);
        partly_isolated.add_edge(1, 2);
        // Nodes 3..7 isolated.

        let candidates = vec![
            Graph::new(0, 1),
            Graph::new(1, 1),
            Graph::new(9, 1),
            complete,
            partly_isolated,
            ring(4),
        ];

        for candidate in &candidates {
            let error = stats.error(candidate, gammas(), equal_weights());
            let penalty = stats.density_penalty(candidate);
            assert!(
                error.is_finite(),
                "error was not finite for a {}-node candidate",
                candidate.num_nodes
            );
            assert!(
                penalty.is_finite(),
                "penalty was not finite for a {}-node candidate",
                candidate.num_nodes
            );
        }
    }

    /// A bin every reference graph agrees on has deviation zero. Without the
    /// floor that is a division by zero, and the NaN reaches the fitness value.
    #[test]
    fn a_reference_set_of_identical_graphs_does_not_divide_by_zero() {
        let reference = vec![ring(6), ring(6), ring(6)];
        let stats = ReferenceStatistics::from_graphs(&reference, axes()).unwrap();

        let same = stats.error(&ring(6), gammas(), equal_weights());
        let different = stats.error(&path_of_three(), gammas(), equal_weights());

        assert!(same.is_finite(), "identical candidate gave {same}");
        assert!(
            different.is_finite(),
            "different candidate gave {different}"
        );
        assert!(same.abs() < 1e-9);
    }

    /// Weights combine the families linearly: doubling every weight doubles
    /// the score, and zeroing every weight removes it entirely. Together these
    /// pin that no family is being added in twice or dropped.
    #[test]
    fn family_weights_combine_linearly() {
        let reference = vec![ring(9), ring(7)];
        let stats = ReferenceStatistics::from_graphs(&reference, axes()).unwrap();
        let candidate = path_of_three();

        let single = stats.error(&candidate, gammas(), equal_weights());
        let doubled = stats.error(
            &candidate,
            gammas(),
            PerFamily {
                degree: 2.0,
                clustering: 2.0,
                spectral: 2.0,
            },
        );
        let none = stats.error(
            &candidate,
            gammas(),
            PerFamily {
                degree: 0.0,
                clustering: 0.0,
                spectral: 0.0,
            },
        );

        assert!(single > 0.0, "a path should not match a ring reference");
        assert!((doubled - 2.0 * single).abs() < 1e-12);
        assert_eq!(none, 0.0);
    }

    /// Zeroing one family's weight must change the score, or that family was
    /// never reaching it.
    ///
    /// The candidate is deliberately a complete graph rather than a path: a
    /// ring and a path both have clustering 0 at every node, so a reference set
    /// and candidate drawn only from those would leave the clustering family
    /// with nothing to say and this test would pass for the wrong reason.
    #[test]
    fn each_family_actually_contributes_to_the_score() {
        let reference = vec![ring(9), ring(7)];
        let stats = ReferenceStatistics::from_graphs(&reference, axes()).unwrap();

        let mut candidate = Graph::new(6, 1);
        for u in 0..6 {
            for v in (u + 1)..6 {
                candidate.add_edge(u, v);
            }
        }

        let all = stats.error(&candidate, gammas(), equal_weights());

        let families = [
            (
                "degree",
                PerFamily {
                    degree: 0.0,
                    clustering: 1.0,
                    spectral: 1.0,
                },
            ),
            (
                "clustering",
                PerFamily {
                    degree: 1.0,
                    clustering: 0.0,
                    spectral: 1.0,
                },
            ),
            (
                "spectral",
                PerFamily {
                    degree: 1.0,
                    clustering: 1.0,
                    spectral: 0.0,
                },
            ),
        ];

        for (name, weights) in families {
            let without = stats.error(&candidate, gammas(), weights);
            assert!(
                (all - without).abs() > 1e-12,
                "dropping the {name} family did not change the score"
            );
        }
    }
    #[test]
    fn the_density_penalty_is_zero_when_the_candidate_matches_the_reference_mean() {
        let reference = vec![ring(6), ring(6)];
        let stats = ReferenceStatistics::from_graphs(&reference, axes()).unwrap();

        assert!(stats.density_penalty(&ring(6)).abs() < 1e-12);
    }
}
