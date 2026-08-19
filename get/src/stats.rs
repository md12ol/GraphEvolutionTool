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

#[cfg(test)]
mod tests {
    use super::{bin_index, clustering_histogram, degree_histogram, normalize};
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
}
