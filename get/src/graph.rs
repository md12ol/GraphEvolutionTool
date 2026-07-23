#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Graph {
    pub num_nodes: usize,
    pub max_edge_multiplicity: u32,
    /// Symmetric adjacency matrix whose entries are edge weights.
    ///
    /// A zero entry means that no edge exists. A value greater than one
    /// represents parallel edges between the same pair of vertices.
    adjacency: Vec<Vec<u32>>,
}

impl Graph {
    /// Create an empty graph with `num_nodes` nodes, no edges, and edge
    /// weights capped at `max_edge_multiplicity`. Pass `1` for a simple
    /// (unweighted) graph.
    pub fn new(num_nodes: usize, max_edge_multiplicity: u32) -> Self {
        Self {
            num_nodes,
            max_edge_multiplicity,
            adjacency: vec![vec![0; num_nodes]; num_nodes],
        }
    }

    /// Return true if at least one edge exists between `u` and `v`.
    pub fn has_edge(&self, u: usize, v: usize) -> bool {
        self.weight(u, v) != 0
    }

    /// Return the edge multiplicity, or zero for invalid vertices.
    pub fn weight(&self, u: usize, v: usize) -> u32 {
        if u >= self.num_nodes || v >= self.num_nodes {
            return 0;
        }
        self.adjacency[u][v]
    }

    /// Set the multiplicity of an undirected edge directly.
    ///
    /// A zero weight clears the edge. Values above
    /// this graph's configured cap are clamped to that cap. Self-loops and
    /// invalid vertices are ignored.
    pub fn set_edge(&mut self, u: usize, v: usize, weight: u32) {
        if u >= self.num_nodes || v >= self.num_nodes || u == v {
            return;
        }

        let weight = weight.min(self.max_edge_multiplicity);
        self.adjacency[u][v] = weight;
        self.adjacency[v][u] = weight;
    }

    /// Add one parallel edge, saturating at this graph's configured cap.
    /// Returns `true` if the multiplicity actually increased, or `false` if
    /// the pair was invalid, a self-loop, or already at the cap.
    pub fn add_edge(&mut self, u: usize, v: usize) -> bool {
        let current = self.weight(u, v);
        let next = current.saturating_add(1).min(self.max_edge_multiplicity);
        self.set_edge(u, v, next);
        self.weight(u, v) != current
    }

    /// Remove one parallel edge, if present.
    pub fn remove_edge(&mut self, u: usize, v: usize) {
        let current = self.weight(u, v);
        if current > 0 {
            self.set_edge(u, v, current - 1);
        }
    }

    /// Remove all parallel edges between `u` and `v`.
    pub fn clear_edge(&mut self, u: usize, v: usize) {
        self.set_edge(u, v, 0);
    }

    /// Set every weighted edge in `edges`.
    pub fn set_edges(&mut self, edges: &[(usize, usize, u32)]) {
        for &(u, v, weight) in edges {
            self.set_edge(u, v, weight);
        }
    }

    /// Return each undirected edge once as `(u, v, weight)`.
    pub fn get_edge_list(&self) -> Vec<(usize, usize, u32)> {
        let mut edges = Vec::new();
        for u in 0..self.num_nodes {
            for v in (u + 1)..self.num_nodes {
                let weight = self.adjacency[u][v];
                if weight != 0 {
                    edges.push((u, v, weight));
                }
            }
        }
        edges
    }

    /// Return the distinct neighbor at `index`, wrapping modulo the number of
    /// distinct neighbors.
    pub fn get_neighbor_at_index(&self, node: usize, index: usize) -> Option<usize> {
        if node >= self.num_nodes {
            return None;
        }

        let degree = self.degree(node);
        if degree == 0 {
            return None;
        }

        let target = index % degree;
        let mut seen = 0;
        for neighbor in 0..self.num_nodes {
            if self.adjacency[node][neighbor] > 0 {
                if seen == target {
                    return Some(neighbor);
                }
                seen += 1;
            }
        }
        None
    }

    /// Return the number of distinct nodes connected to `node`.
    pub fn degree(&self, node: usize) -> usize {
        if node >= self.num_nodes {
            return 0;
        }
        self.adjacency[node]
            .iter()
            .filter(|&&weight| weight > 0)
            .count()
    }

    /// Return the total number of incident edge copies, counting parallel
    /// edges separately.
    pub fn total_edge_multiplicity(&self, node: usize) -> usize {
        if node >= self.num_nodes {
            return 0;
        }
        self.adjacency[node]
            .iter()
            .map(|&weight| weight as usize)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::Graph;

    #[test]
    fn add_and_remove_change_one_copy() {
        let mut graph = Graph::new(3, 5);

        graph.add_edge(0, 1);
        graph.add_edge(0, 1);
        assert_eq!(graph.weight(0, 1), 2);
        assert_eq!(graph.weight(1, 0), 2);

        graph.remove_edge(0, 1);
        assert_eq!(graph.weight(0, 1), 1);

        graph.remove_edge(0, 1);
        graph.remove_edge(0, 1);
        assert_eq!(graph.weight(0, 1), 0);
    }

    #[test]
    fn multiplicity_is_capped_at_five() {
        let mut graph = Graph::new(2, 5);

        graph.set_edge(0, 1, 12);

        assert_eq!(graph.weight(0, 1), 5);
        assert_eq!(graph.weight(1, 0), 5);

        graph.add_edge(0, 1);
        assert_eq!(graph.weight(0, 1), 5);

        graph.remove_edge(0, 1);
        assert_eq!(graph.weight(0, 1), 4);
        graph.add_edge(0, 1);
        assert_eq!(graph.weight(0, 1), 5);
    }

    #[test]
    fn unweighted_graph_keeps_multiplicity_in_zero_or_one() {
        let mut graph = Graph::new(2, 1);

        graph.set_edge(0, 1, 5);
        assert_eq!(graph.weight(0, 1), 1);
        assert_eq!(graph.max_edge_multiplicity, 1);

        graph.add_edge(0, 1);
        assert_eq!(graph.weight(0, 1), 1);

        graph.remove_edge(0, 1);
        assert_eq!(graph.weight(0, 1), 0);
    }

    #[test]
    fn explicit_multiplicity_cap_is_enforced() {
        let mut graph = Graph::new(2, 3);
        graph.set_edge(0, 1, 5);

        assert_eq!(graph.max_edge_multiplicity, 3);
        assert_eq!(graph.weight(0, 1), 3);
    }

    #[test]
    fn degree_and_neighbor_selection_use_distinct_neighbors() {
        let mut graph = Graph::new(4, 5);
        graph.set_edge(0, 1, 2);
        graph.set_edge(0, 3, 1);

        assert_eq!(graph.degree(0), 2);
        assert_eq!(graph.total_edge_multiplicity(0), 3);
        assert_eq!(graph.get_neighbor_at_index(0, 0), Some(1));
        assert_eq!(graph.get_neighbor_at_index(0, 1), Some(3));
        assert_eq!(graph.get_neighbor_at_index(0, 2), Some(1));
    }

    #[test]
    fn set_edges_sets_multiplicity_and_preserves_symmetry() {
        let mut graph = Graph::new(3, 5);
        graph.set_edges(&[(0, 1, 4), (1, 2, 2)]);

        assert_eq!(graph.get_edge_list(), vec![(0, 1, 4), (1, 2, 2)]);
        assert_eq!(graph.weight(1, 0), 4);
        assert_eq!(graph.weight(2, 1), 2);
    }
}
