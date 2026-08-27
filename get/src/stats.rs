//! Size-invariant structural statistics for a graph.
//!
//! Every statistic reduces a graph to a normalized histogram on a **fixed**
//! axis: the same bin edges whatever the node count, which is what lets graphs
//! of different sizes be compared. Bin per graph instead and bin *k* means a
//! different thing on each side, with every number still plausible.
//!
//! All statistics here are **unweighted**: an edge is present or absent, and a
//! multiplicity above one counts once.

use crate::graph::Graph;

/// Bin a value into `num_bins` bins spanning `[0.0, max]`, clamping anything at
/// or above `max` into the last bin.
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
/// A histogram with no observations is left as all zeros, not made uniform.
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
/// `max_degree` comes from the caller, not from the graph: a candidate and every
/// reference graph it is compared against must be given the same one.
///
/// A degree above `max_degree` lands in the top bin rather than being dropped,
/// so the distribution always sums to 1.0 for a graph with at least one node.
pub fn degree_histogram(graph: &Graph, max_degree: usize, num_bins: usize) -> Vec<f64> {
    let mut hist = vec![0.0; num_bins];
    if num_bins == 0 || graph.num_nodes == 0 {
        return hist;
    }

    for node in 0..graph.num_nodes {
        let degree = graph.neighbor_count(node) as f64;
        let index = bin_index(degree, max_degree as f64, num_bins);
        hist[index] += 1.0;
    }

    normalize(&mut hist);
    hist
}

/// Local clustering coefficient of every node, histogrammed over the fixed
/// axis `[0, 1]`.
///
/// A node with fewer than two neighbours scores 0 and is counted rather than
/// skipped, so a graph too small to hold a triangle still produces a
/// distribution summing to 1.0 rather than a vector of zeros.
pub fn clustering_histogram(graph: &Graph, num_bins: usize) -> Vec<f64> {
    let mut hist = vec![0.0; num_bins];
    if num_bins == 0 || graph.num_nodes == 0 {
        return hist;
    }

    for node in 0..graph.num_nodes {
        let neighbours = graph.neighbors(node);
        let degree = neighbours.len();

        let coefficient = if degree < 2 {
            0.0
        } else {
            // Each unordered neighbour pair is counted once, which is what the
            // factor of 2 below corrects for.
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
/// Degree zero takes `1/sqrt(0) = 0`: mutation can strip a node's last edge
/// mid-run, and the natural expression then yields `inf * 0 = NaN`, which
/// crashes the run rather than scoring it badly.
fn normalized_laplacian(graph: &Graph) -> Vec<Vec<f64>> {
    let n = graph.num_nodes;
    let mut inverse_sqrt_degree = Vec::with_capacity(n);
    for node in 0..n {
        let degree = graph.neighbor_count(node);
        if degree > 0 {
            inverse_sqrt_degree.push(1.0 / (degree as f64).sqrt());
        } else {
            inverse_sqrt_degree.push(0.0);
        }
    }

    let mut laplacian = vec![vec![0.0; n]; n];
    for u in 0..n {
        // An isolated node keeps an all-zero row, the diagonal included.
        if graph.neighbor_count(u) > 0 {
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
/// rotation. A matrix that is not symmetric gives meaningless results.
// The rotation below works on two specific rows and two specific columns, so
// indices are what the algorithm is written in terms of.
#[allow(clippy::needless_range_loop)]
fn symmetric_eigenvalues(mut matrix: Vec<Vec<f64>>) -> Vec<f64> {
    let n = matrix.len();
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![matrix[0][0]];
    }

    // Exhausting the sweeps returns the unconverged diagonal; the ceiling is a
    // guard against a pathological input, not the expected exit.
    const MAX_SWEEPS: usize = 100;
    const TOLERANCE: f64 = 1e-12;

    for _ in 0..MAX_SWEEPS {
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

                // `t` is the tangent of the rotation that sends `matrix[p][q]`
                // to exactly zero.
                let theta = (matrix[q][q] - matrix[p][p]) / (2.0 * matrix[p][q]);
                let sign = if theta >= 0.0 { 1.0 } else { -1.0 };
                let t = sign / (theta.abs() + (theta * theta + 1.0).sqrt());
                let cos = 1.0 / (t * t + 1.0).sqrt();
                let sin = t * cos;

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
/// `[0, 2]`, which is where its eigenvalues lie for any graph of any size.
///
/// Bin 0 holds every eigenvalue below `2 / num_bins`, so it contains the exact
/// zeros — one per connected component — without being only them: a connected
/// 12-node path on 8 bins contributes three. A heavy bin 0 is a signal of
/// fragmentation, never a component count.
pub fn spectral_histogram(graph: &Graph, num_bins: usize) -> Vec<f64> {
    let mut hist = vec![0.0; num_bins];
    if num_bins == 0 || graph.num_nodes == 0 {
        return hist;
    }

    let eigenvalues = symmetric_eigenvalues(normalized_laplacian(graph));
    for value in &eigenvalues {
        // Rounding can leave an eigenvalue a hair outside [0, 2].
        let clamped = value.clamp(0.0, 2.0);
        let index = bin_index(clamped, 2.0, num_bins);
        hist[index] += 1.0;
    }

    normalize(&mut hist);
    hist
}

/// Fraction of the possible edges that are present, `2m / (n(n-1))`. Zero for a
/// graph with fewer than two nodes.
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
    /// An error rather than a score, because scoring against nothing makes
    /// every candidate ideal: the population converges at once and the run
    /// looks healthy while measuring nothing.
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

/// The bin edges every graph is binned onto, a candidate and every reference
/// graph alike.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistogramAxes {
    /// Top of the degree axis. Degrees above it land in the last bin.
    pub max_degree: usize,
    pub degree_bins: usize,
    pub clustering_bins: usize,
    pub spectral_bins: usize,
}

/// One value per statistic family.
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
/// A bin identical across every reference graph has deviation exactly zero, and
/// dividing by it would put a `NaN` into the fitness value, crashing the run.
const DEVIATION_FLOOR: f64 = 1e-6;

impl FamilyReference {
    /// Standardize each bin against the reference set's own mean and deviation
    /// for that bin, so a bin the reference graphs agree on weighs more than
    /// one they vary widely on.
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
    /// Not MMD: against a fixed reference set the within-reference term is a
    /// constant and the candidate's self-similarity is 1, so dropping both
    /// leaves the ranking unchanged.
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

/// Check the RBF bandwidths before they reach `exp`.
fn assert_gammas(gammas: PerFamily) {
    let named = [
        ("degree", gammas.degree),
        ("clustering", gammas.clustering),
        ("spectral", gammas.spectral),
    ];
    for (family, gamma) in named {
        assert!(
            gamma.is_finite() && gamma > 0.0,
            "the {family} gamma must be finite and greater than zero, got {gamma}: \
             exp(-gamma * d^2) is a similarity in (0, 1] only for a positive gamma, \
             and a negative one sends the score to infinity",
        );
    }
}

/// Check the per-family weights before they multiply a family's error. Zero is
/// allowed and means "ignore this family".
fn assert_weights(weights: PerFamily) {
    let named = [
        ("degree", weights.degree),
        ("clustering", weights.clustering),
        ("spectral", weights.spectral),
    ];
    for (family, weight) in named {
        assert!(
            weight.is_finite() && weight >= 0.0,
            "the {family} weight must be finite and not negative, got {weight}: \
             a negative weight inverts the objective for that family, and a \
             non-finite one carries straight into the score",
        );
    }
}

/// A reference set reduced to the statistics a candidate is scored against.
///
/// Built once per run and reused for every evaluation.
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

    /// The axes this reference set was built on; a candidate must be binned on
    /// the same ones.
    pub fn axes(&self) -> &HistogramAxes {
        &self.axes
    }

    /// Mean density across the reference set — the density penalty's target.
    pub fn mean_density(&self) -> f64 {
        self.mean_density
    }

    /// How far a candidate is from this reference set: a weighted sum of the
    /// family errors, zero for a perfect match. [`Self::density_penalty`] is
    /// not part of it.
    ///
    /// `gammas` are the RBF bandwidths, one per family, and must be kept small:
    /// too large a gamma collapses `exp(-gamma * d^2)` to zero for every
    /// candidate, so the whole population scores ~1.0, there is no gradient to
    /// climb, and evolution stalls while appearing to run normally.
    ///
    /// Panics unless every gamma is finite and positive and every weight finite
    /// and non-negative: nothing else checks them on this path, and unchecked
    /// they return a plausible score rather than an error.
    pub fn error(&self, candidate: &Graph, gammas: PerFamily, weights: PerFamily) -> f64 {
        assert_gammas(gammas);
        assert_weights(weights);

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
    /// Not included in `error`: a caller weights and reports the two separately.
    pub fn density_penalty(&self, candidate: &Graph) -> f64 {
        (density(candidate) - self.mean_density).abs()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FamilyReference, HistogramAxes, PerFamily, ReferenceStatistics, StatsError, bin_index,
        clustering_histogram, degree_histogram, density, normalize, normalized_laplacian,
        spectral_histogram, symmetric_eigenvalues,
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

    /// The gamma and weight preconditions, which nothing but these asserts
    /// enforces on the route-3 path.
    ///
    /// `config` validates them for a config-file run, but this module is public
    /// so that a caller can score graphs without one — and a bad value there
    /// used to come back as a finite-looking score rather than a failure. The
    /// candidate must differ from the reference or the squared distance is zero
    /// and every gamma gives the same answer, which is what made this hard to
    /// see: the passing tests above all score near-identical graphs.
    fn stats_for_precondition_tests() -> ReferenceStatistics {
        ReferenceStatistics::from_graphs(&[ring(8)], axes()).unwrap()
    }

    #[test]
    #[should_panic(expected = "the degree gamma must be finite and greater than zero")]
    fn a_negative_gamma_panics_rather_than_returning_infinity() {
        let stats = stats_for_precondition_tests();
        let bad = PerFamily {
            degree: -1.0,
            clustering: 0.01,
            spectral: 0.01,
        };

        stats.error(&path_of_three(), bad, equal_weights());
    }

    #[test]
    #[should_panic(expected = "the clustering gamma must be finite and greater than zero")]
    fn a_nan_gamma_panics() {
        let stats = stats_for_precondition_tests();
        let bad = PerFamily {
            degree: 0.01,
            clustering: f64::NAN,
            spectral: 0.01,
        };

        stats.error(&path_of_three(), bad, equal_weights());
    }

    #[test]
    #[should_panic(expected = "the spectral gamma must be finite and greater than zero")]
    fn a_zero_gamma_panics_because_every_candidate_would_score_alike() {
        let stats = stats_for_precondition_tests();
        let bad = PerFamily {
            degree: 0.01,
            clustering: 0.01,
            spectral: 0.0,
        };

        stats.error(&path_of_three(), bad, equal_weights());
    }

    #[test]
    #[should_panic(expected = "the degree weight must be finite and not negative")]
    fn a_negative_weight_panics_rather_than_inverting_the_objective() {
        let stats = stats_for_precondition_tests();
        let bad = PerFamily {
            degree: -1.0,
            clustering: 1.0,
            spectral: 1.0,
        };

        stats.error(&path_of_three(), gammas(), bad);
    }

    #[test]
    #[should_panic(expected = "the spectral weight must be finite and not negative")]
    fn an_infinite_weight_panics() {
        let stats = stats_for_precondition_tests();
        let bad = PerFamily {
            degree: 1.0,
            clustering: 1.0,
            spectral: f64::INFINITY,
        };

        stats.error(&path_of_three(), gammas(), bad);
    }

    /// A zero weight is allowed — it means "ignore this family", which is a
    /// real configuration and must not be swept up by the negative check.
    #[test]
    fn a_zero_weight_is_allowed_and_drops_that_family() {
        let stats = stats_for_precondition_tests();
        let only_degree = PerFamily {
            degree: 1.0,
            clustering: 0.0,
            spectral: 0.0,
        };

        let error = stats.error(&path_of_three(), gammas(), only_degree);
        assert!(error.is_finite(), "got {error}");
    }

    /// The finiteness promise, over the whole legal input range rather than one
    /// convenient point: extreme-but-valid gammas and weights against a
    /// candidate as unlike the reference as possible.
    #[test]
    fn the_score_stays_finite_across_the_legal_gamma_and_weight_range() {
        let stats = stats_for_precondition_tests();

        let mut complete = Graph::new(9, 1);
        for u in 0..9 {
            for v in (u + 1)..9 {
                complete.add_edge(u, v);
            }
        }

        let candidates = [path_of_three(), ring(3), complete, Graph::new(4, 1)];
        let extremes = [1e-9, 1.0, 1e9];

        for candidate in &candidates {
            for gamma in extremes {
                for weight in extremes {
                    let g = PerFamily {
                        degree: gamma,
                        clustering: gamma,
                        spectral: gamma,
                    };
                    let w = PerFamily {
                        degree: weight,
                        clustering: weight,
                        spectral: weight,
                    };

                    let error = stats.error(candidate, g, w);
                    assert!(
                        error.is_finite(),
                        "gamma {gamma}, weight {weight} gave {error}"
                    );
                }
            }
        }
    }

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

    /// Standardizing against the reference set's own spread is the whole
    /// reason this score weights *what the reference set agrees about*, and
    /// dropping it leaves a kernel that treats every bin alike. Nothing else
    /// here separates the two: a matching candidate scores zero either way, a
    /// dissimilar one scores worse either way, and the deviation floor is
    /// reached either way. What only standardizing gets right is which of two
    /// candidates is worse when they miss in different bins.
    #[test]
    fn a_bin_the_reference_set_agrees_on_outweighs_one_it_varies_on() {
        // Bin 0 is unanimous at 0.5; bin 1 ranges over 0.0..0.6, mean 0.3,
        // deviation sqrt(0.05) ~ 0.2236.
        let family = FamilyReference::build(vec![
            vec![0.5, 0.0],
            vec![0.5, 0.2],
            vec![0.5, 0.4],
            vec![0.5, 0.6],
        ]);

        // Misses the unanimous bin by 0.01 and sits exactly on the mean of the
        // contested one.
        let off_in_the_agreed_bin = family.error(&[0.51, 0.3], 1.0);
        // Misses the contested bin by 0.30 — thirty times as far in raw terms
        // — and sits exactly on the unanimous one.
        let off_in_the_varying_bin = family.error(&[0.5, 0.6], 1.0);

        assert!(
            off_in_the_agreed_bin > off_in_the_varying_bin,
            "0.01 off a bin the references agree on ({off_in_the_agreed_bin}) must \
             cost more than 0.30 off a bin they disagree on ({off_in_the_varying_bin})"
        );

        // Against a deviation of 1e-6 the first candidate is thousands of
        // standard deviations out, so every kernel term underflows to zero.
        assert!(
            (off_in_the_agreed_bin - 1.0).abs() < 1e-9,
            "{off_in_the_agreed_bin}"
        );
        // Hand-computed: standardized 1.3416 against -1.3416, -0.4472, 0.4472
        // and 1.3416 gives squared distances 7.2, 3.2, 0.8 and 0, so the mean
        // similarity is 0.3727.
        assert!(
            (off_in_the_varying_bin - 0.6273).abs() < 1e-3,
            "{off_in_the_varying_bin}"
        );
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

    /// A distance, so being *below* the reference mean costs exactly what
    /// being the same amount above it does. The objective is minimized, so an
    /// unsigned difference here would not merely mis-rank a sparse candidate:
    /// it would pay one, without limit, for every edge it failed to have.
    #[test]
    fn the_density_penalty_is_a_distance_in_both_directions() {
        // A 6-node ring is 6 of the 15 possible edges, so the reference mean
        // density is 0.4.
        let reference = vec![ring(6), ring(6)];
        let stats = ReferenceStatistics::from_graphs(&reference, axes()).unwrap();
        assert!((stats.mean_density() - 0.4).abs() < 1e-12);

        // 3 of 15 edges: density 0.2, which is 0.2 below the mean.
        let mut sparser = Graph::new(6, 1);
        for (u, v) in [(0, 1), (2, 3), (4, 5)] {
            sparser.set_edge(u, v, 1);
        }

        // 9 of 15 edges: density 0.6, the same 0.2 above it.
        let mut denser = Graph::new(6, 1);
        for (u, v) in [
            (0, 1),
            (0, 2),
            (0, 3),
            (1, 2),
            (1, 3),
            (2, 3),
            (0, 4),
            (1, 4),
            (2, 4),
        ] {
            denser.set_edge(u, v, 1);
        }

        assert!(
            (density(&sparser) - 0.2).abs() < 1e-12,
            "{}",
            density(&sparser)
        );
        assert!(
            (density(&denser) - 0.6).abs() < 1e-12,
            "{}",
            density(&denser)
        );

        assert!((stats.density_penalty(&sparser) - 0.2).abs() < 1e-12);
        assert!((stats.density_penalty(&denser) - 0.2).abs() < 1e-12);
    }
}
