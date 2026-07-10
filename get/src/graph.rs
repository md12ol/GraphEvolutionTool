#[derive(Clone)]
pub struct Graph {
    pub num_nodes: usize,
    /// Weighted adjacency matrix: `adjacency[u][v]` is the weight of edge `u—v`,
    /// or 0 if there is no edge. Kept symmetric (`[u][v] == [v][u]`) since the
    /// graph is undirected.
    pub adjacency: Vec<Vec<u32>>
}

impl Graph {
    /// Create an empty graph with `num_nodes` nodes and no edges.
    pub fn new(num_nodes: usize) -> Self {
        Graph {
            num_nodes,
            adjacency: vec![vec![0; num_nodes]; num_nodes]
        }
    }

    /// Return true if an edge (nonzero weight) exists between `u` and `v`.
    pub fn has_edge(&self, u: usize, v: usize) -> bool {
        return self.weight(u, v) != 0;
    }

    /// Return the weight of edge `u—v`, or 0 if there is no edge
    /// (or either endpoint is out of range).
    pub fn weight(&self, u: usize, v: usize) -> u32 {
        if u >= self.num_nodes || v >= self.num_nodes {
            return 0;
        }
        return self.adjacency[u][v];
    }

    /// Add every weighted edge in `edges` to the graph.
    pub fn set_edges(&mut self, edges: &Vec<(usize, usize, u32)>) {
        for &(u, v, w) in edges {
            self.add_edge(u, v, w);
        }
    }

    /// Set the undirected edge between `u` and `v` to `weight`.
    ///
    /// A `weight` of 0 clears the edge. No-op if either endpoint is out of range
    /// or `u == v` (self-loops are not allowed).
    pub fn add_edge(&mut self, u: usize, v: usize, weight: u32) {
        if u >= self.num_nodes || v >= self.num_nodes || u == v {
            return;
        }
        self.adjacency[u][v] = weight;
        self.adjacency[v][u] = weight; // undirected: keep the matrix symmetric
    }

    /// Remove the undirected edge between `u` and `v`, if present.
    pub fn remove_edge(&mut self, u: usize, v: usize) {
        if u >= self.num_nodes || v >= self.num_nodes {
            return;
        }
        self.adjacency[u][v] = 0;
        self.adjacency[v][u] = 0;
    }

    /// Return each undirected edge once, as `(u, v, weight)` tuples with `u < v`.
    pub fn get_edge_list(&self) -> Vec<(usize, usize, u32)> {
        let mut edges = Vec::new();
        for u in 0..self.num_nodes {
            // start at u + 1 so each undirected edge is emitted exactly once
            for v in (u + 1)..self.num_nodes {
                let w = self.adjacency[u][v];
                if w != 0 {
                    edges.push((u, v, w));
                }
            }
        }
        return edges;
    }

    /// Return the neighbor of `node` at `index`, wrapping modulo its degree.
    ///
    /// Returns `None` if `node` is out of range or has no neighbors.
    pub fn get_neighbor_at_index(&self, node: usize, index: usize) -> Option<usize> {
        if node >= self.num_nodes {
            return None;
        }
        let neighbors: Vec<usize> = (0..self.num_nodes)
            .filter(|&v| self.adjacency[node][v] != 0)
            .collect();
        if neighbors.is_empty() {
            return None;
        } else {
            return Some(neighbors[index % neighbors.len()]);
        }
    }

    /// Return the number of neighbors of `node` (0 if out of range).
    pub fn degree(&self, node: usize) -> usize {
        if node >= self.num_nodes {
            return 0;
        }

        let mut count = 0;
        for &w in self.adjacency[node].iter() {
            if w != 0 {
                count += 1;
            }
        }
        return count;
    }
}
