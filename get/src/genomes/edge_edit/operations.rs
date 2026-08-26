//! What each edge-edit opcode does to the graph.

use crate::graph::Graph;

/// The operations encoded by an edge-edit gene.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GraphOperation {
    Toggle,
    Hop,
    Add,
    Delete,
    Swap,
    LocalToggle,
    LocalAdd,
    LocalDelete,
    Null,
}

impl GraphOperation {
    /// These numbers are gene data, not an ordering: renumbering one silently
    /// redecodes every existing genome, saved result and `IDENTITY_GENE`.
    pub(super) fn from_opcode(opcode: u8) -> Option<Self> {
        match opcode {
            0 => Some(Self::Toggle),
            1 => Some(Self::Hop),
            2 => Some(Self::Add),
            3 => Some(Self::Delete),
            4 => Some(Self::Swap),
            5 => Some(Self::LocalToggle),
            6 => Some(Self::LocalAdd),
            7 => Some(Self::LocalDelete),
            8 => Some(Self::Null),
            _ => None,
        }
    }

    pub(super) fn apply(self, graph: &mut Graph, v1: usize, v2: usize, v3: usize, v4: usize) {
        match self {
            Self::Toggle => Self::toggle(graph, v1, v2, v3),
            Self::Hop => Self::hop(graph, v1, v2, v3),
            Self::Add => Self::add(graph, v1, v2),
            Self::Delete => Self::delete(graph, v1, v2),
            Self::Swap => Self::swap(graph, v1, v2, v3, v4),
            Self::LocalToggle => Self::local_toggle(graph, v1, v2, v3, v4),
            Self::LocalAdd => Self::local_add(graph, v1, v2, v3),
            Self::LocalDelete => Self::local_delete(graph, v1, v2, v3),
            Self::Null => {}
        }
    }

    fn valid_pair(graph: &Graph, u: usize, v: usize) -> bool {
        u < graph.num_nodes && v < graph.num_nodes && u != v
    }

    fn toggle(graph: &mut Graph, u: usize, v: usize, direction: usize) {
        if !Self::valid_pair(graph, u, v) {
            return;
        }

        match graph.weight(u, v) {
            0 => _ = graph.add_edge(u, v),
            weight if weight == graph.max_edge_multiplicity => graph.remove_edge(u, v),
            _ if direction.is_multiple_of(2) => graph.remove_edge(u, v),
            _ => _ = graph.add_edge(u, v),
        }
    }

    fn add(graph: &mut Graph, u: usize, v: usize) {
        if Self::valid_pair(graph, u, v) {
            graph.add_edge(u, v);
        }
    }

    fn delete(graph: &mut Graph, u: usize, v: usize) {
        if Self::valid_pair(graph, u, v) {
            graph.remove_edge(u, v);
        }
    }

    fn two_hop_endpoint(
        graph: &Graph,
        start: usize,
        first_neighbor_index: usize,
        second_neighbor_index: usize,
    ) -> Option<(usize, usize)> {
        let first_neighbor = graph.get_neighbor_at_index(start, first_neighbor_index)?;
        if graph.neighbor_count(first_neighbor) < 2 {
            return None;
        }
        let endpoint = graph.get_neighbor_at_index(first_neighbor, second_neighbor_index)?;
        if endpoint == start {
            return None;
        }
        Some((first_neighbor, endpoint))
    }

    fn local_toggle(
        graph: &mut Graph,
        start: usize,
        first_neighbor_index: usize,
        second_neighbor_index: usize,
        direction: usize,
    ) {
        let Some((_, endpoint)) =
            Self::two_hop_endpoint(graph, start, first_neighbor_index, second_neighbor_index)
        else {
            return;
        };
        Self::toggle(graph, start, endpoint, direction);
    }

    fn local_add(
        graph: &mut Graph,
        start: usize,
        first_neighbor_index: usize,
        second_neighbor_index: usize,
    ) {
        let Some((_, endpoint)) =
            Self::two_hop_endpoint(graph, start, first_neighbor_index, second_neighbor_index)
        else {
            return;
        };
        Self::add(graph, start, endpoint);
    }

    fn local_delete(
        graph: &mut Graph,
        start: usize,
        first_neighbor_index: usize,
        second_neighbor_index: usize,
    ) {
        let Some((_, endpoint)) =
            Self::two_hop_endpoint(graph, start, first_neighbor_index, second_neighbor_index)
        else {
            return;
        };
        Self::delete(graph, start, endpoint);
    }

    fn hop(
        graph: &mut Graph,
        start: usize,
        first_neighbor_index: usize,
        second_neighbor_index: usize,
    ) {
        let Some((first_neighbor, endpoint)) =
            Self::two_hop_endpoint(graph, start, first_neighbor_index, second_neighbor_index)
        else {
            return;
        };

        // `add_edge` fails at the multiplicity cap; removing the old edge
        // anyway would lose an edge copy.
        if !graph.add_edge(start, endpoint) {
            return;
        }
        graph.remove_edge(start, first_neighbor);
    }

    fn swap(
        graph: &mut Graph,
        first_vertex: usize,
        second_vertex: usize,
        first_neighbor_index: usize,
        second_neighbor_index: usize,
    ) {
        if graph.num_nodes < 4
            || first_vertex >= graph.num_nodes
            || second_vertex >= graph.num_nodes
            || first_vertex == second_vertex
            || graph.neighbor_count(first_vertex) <= 2
            || graph.neighbor_count(second_vertex) <= 2
            || graph.has_edge(first_vertex, second_vertex)
        {
            return;
        }

        let Some(first_neighbor) = graph.get_neighbor_at_index(first_vertex, first_neighbor_index)
        else {
            return;
        };
        let Some(second_neighbor) =
            graph.get_neighbor_at_index(second_vertex, second_neighbor_index)
        else {
            return;
        };

        // `has_edge(first_neighbor, second_neighbor)` below is an anti-clustering
        // guard, not a would-be edge: the swap never creates that pair.
        let mut quartet = [first_vertex, first_neighbor, second_vertex, second_neighbor];
        quartet.sort_unstable();
        if quartet.windows(2).any(|pair| pair[0] == pair[1])
            || graph.has_edge(first_vertex, second_neighbor)
            || graph.has_edge(second_vertex, first_neighbor)
            || graph.has_edge(first_neighbor, second_neighbor)
        {
            return;
        }

        graph.remove_edge(first_vertex, first_neighbor);
        graph.remove_edge(second_vertex, second_neighbor);
        graph.add_edge(first_vertex, second_neighbor);
        graph.add_edge(second_vertex, first_neighbor);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph_with_edges(num_nodes: usize, edges: &[(usize, usize, u32)]) -> Graph {
        let mut graph = Graph::new(num_nodes, 5);
        graph.set_edges(edges);
        graph
    }

    fn unweighted_graph_with_edges(num_nodes: usize, edges: &[(usize, usize, u32)]) -> Graph {
        let mut graph = Graph::new(num_nodes, 1);
        graph.set_edges(edges);
        graph
    }

    fn total_edge_multiplicity(graph: &Graph) -> u32 {
        graph
            .get_edge_list()
            .iter()
            .map(|(_, _, weight)| *weight)
            .sum()
    }

    #[test]
    fn direct_operations_change_one_edge_copy_and_reject_self_loops() {
        let mut graph = graph_with_edges(3, &[(0, 1, 3)]);

        GraphOperation::Add.apply(&mut graph, 0, 1, 0, 0);
        assert_eq!(graph.weight(0, 1), 4);

        GraphOperation::Delete.apply(&mut graph, 0, 1, 0, 0);
        assert_eq!(graph.weight(0, 1), 3);

        GraphOperation::Toggle.apply(&mut graph, 0, 1, 0, 0);
        assert_eq!(graph.weight(0, 1), 2);
        GraphOperation::Toggle.apply(&mut graph, 0, 2, 0, 0);
        assert_eq!(graph.weight(0, 2), 1);

        let before = graph.clone();
        GraphOperation::Add.apply(&mut graph, 1, 1, 0, 0);
        GraphOperation::Delete.apply(&mut graph, 3, 0, 0, 0);
        assert_eq!(graph, before);
    }

    #[test]
    fn direct_operations_preserve_unweighted_graph_semantics() {
        let mut graph = unweighted_graph_with_edges(3, &[(0, 1, 1)]);

        GraphOperation::Add.apply(&mut graph, 0, 1, 0, 0);
        assert_eq!(graph.weight(0, 1), 1);

        GraphOperation::Toggle.apply(&mut graph, 0, 1, 1, 0);
        assert_eq!(graph.weight(0, 1), 0);

        GraphOperation::Toggle.apply(&mut graph, 0, 2, 0, 0);
        assert_eq!(graph.weight(0, 2), 1);
        assert!(
            graph
                .get_edge_list()
                .iter()
                .all(|(_, _, weight)| *weight == 1)
        );
    }

    #[test]
    fn toggle_uses_direction_between_forced_boundary_moves() {
        let mut graph = graph_with_edges(2, &[(0, 1, 2)]);

        GraphOperation::Toggle.apply(&mut graph, 0, 1, 1, 0);
        assert_eq!(graph.weight(0, 1), 3);

        GraphOperation::Toggle.apply(&mut graph, 0, 1, 0, 0);
        assert_eq!(graph.weight(0, 1), 2);

        graph.set_edge(0, 1, 5);
        GraphOperation::Toggle.apply(&mut graph, 0, 1, 1, 0);
        assert_eq!(graph.weight(0, 1), 4);

        graph.set_edge(0, 1, 0);
        GraphOperation::Toggle.apply(&mut graph, 0, 1, 0, 0);
        assert_eq!(graph.weight(0, 1), 1);

        // `direction` is a decoded gene field, not a flag, and only its parity
        // is read. Exercised at 0 and 1 alone, any rule agreeing with parity on
        // those two passes — so the values above pin nothing about parity, and
        // these do.
        graph.set_edge(0, 1, 2);
        GraphOperation::Toggle.apply(&mut graph, 0, 1, 2, 0);
        assert_eq!(graph.weight(0, 1), 1, "2 is even, so it removes");

        GraphOperation::Toggle.apply(&mut graph, 0, 1, 7, 0);
        assert_eq!(graph.weight(0, 1), 2, "7 is odd, so it adds");

        GraphOperation::Toggle.apply(&mut graph, 0, 1, 12, 0);
        assert_eq!(graph.weight(0, 1), 1, "12 is even, so it removes");
    }

    #[test]
    fn swap_declines_between_two_vertices_that_are_already_adjacent() {
        // Swapping across an existing edge is what the guard exists to stop:
        // 0 and 1 are joined, and rewiring their neighbours around them would
        // move edges without the pair of disjoint edges a swap is defined on.
        let graph = graph_with_edges(6, &[(0, 1, 1), (0, 2, 1), (0, 3, 1), (1, 4, 1), (1, 5, 1)]);
        let before = graph.get_edge_list();

        // Both have degree 3, so nothing else in the guard declines for them,
        // and indices 1 and 1 name neighbours 2 and 4 — a quartet that is
        // otherwise perfectly swappable.
        for direction in 0..4 {
            let mut candidate = graph.clone();
            GraphOperation::Swap.apply(&mut candidate, 0, 1, 1, direction);
            assert_eq!(candidate.get_edge_list(), before, "direction {direction}");
        }
    }

    #[test]
    fn local_operations_follow_a_two_hop_path() {
        let mut graph = graph_with_edges(4, &[(0, 1, 1), (1, 2, 1)]);

        GraphOperation::LocalAdd.apply(&mut graph, 0, 0, 1, 0);
        GraphOperation::LocalAdd.apply(&mut graph, 0, 0, 1, 0);
        assert_eq!(graph.weight(0, 2), 2);

        GraphOperation::LocalToggle.apply(&mut graph, 0, 0, 1, 1);
        assert_eq!(graph.weight(0, 2), 3);

        GraphOperation::LocalToggle.apply(&mut graph, 0, 0, 1, 0);
        assert_eq!(graph.weight(0, 2), 2);

        GraphOperation::LocalDelete.apply(&mut graph, 0, 0, 1, 0);
        GraphOperation::LocalDelete.apply(&mut graph, 0, 0, 1, 0);
        assert_eq!(graph.weight(0, 2), 0);
    }

    #[test]
    fn local_operations_are_noops_without_a_two_hop_endpoint() {
        let mut graph = graph_with_edges(3, &[(0, 1, 1)]);
        let before = graph.clone();

        GraphOperation::LocalAdd.apply(&mut graph, 0, 0, 0, 0);
        GraphOperation::LocalDelete.apply(&mut graph, 2, 0, 0, 0);
        GraphOperation::LocalToggle.apply(&mut graph, 0, 0, 0, 0);

        assert_eq!(graph, before);
    }

    #[test]
    fn hop_moves_one_copy_and_can_increase_target_multiplicity() {
        let mut graph = graph_with_edges(3, &[(0, 1, 2), (1, 2, 1), (0, 2, 2)]);

        GraphOperation::Hop.apply(&mut graph, 0, 0, 1, 0);

        assert_eq!(graph.weight(0, 1), 1);
        assert_eq!(graph.weight(0, 2), 3);
    }

    #[test]
    fn hop_respects_an_unweighted_graph_cap() {
        let mut graph = unweighted_graph_with_edges(3, &[(0, 1, 1), (1, 2, 1)]);

        GraphOperation::Hop.apply(&mut graph, 0, 0, 1, 0);

        assert_eq!(graph.weight(0, 1), 0);
        assert_eq!(graph.weight(0, 2), 1);
        assert!(
            graph
                .get_edge_list()
                .iter()
                .all(|(_, _, weight)| *weight == 1)
        );

        let mut saturated = unweighted_graph_with_edges(3, &[(0, 1, 1), (1, 2, 1), (0, 2, 1)]);
        let before = saturated.clone();
        GraphOperation::Hop.apply(&mut saturated, 0, 0, 1, 0);
        assert_eq!(saturated, before);
    }

    #[test]
    fn hop_is_a_noop_when_target_multiplicity_is_saturated() {
        let mut graph = graph_with_edges(3, &[(0, 1, 2), (1, 2, 1), (0, 2, 5)]);
        let before = graph.clone();

        GraphOperation::Hop.apply(&mut graph, 0, 0, 1, 0);

        assert_eq!(graph, before);
    }

    #[test]
    fn swap_moves_one_copy_and_preserves_total_multiplicity() {
        let mut graph = graph_with_edges(
            6,
            &[
                (0, 1, 2),
                (0, 4, 1),
                (0, 5, 1),
                (2, 3, 1),
                (2, 4, 1),
                (2, 5, 1),
            ],
        );
        let before_total = total_edge_multiplicity(&graph);

        GraphOperation::Swap.apply(&mut graph, 0, 2, 0, 0);

        assert_eq!(graph.weight(0, 1), 1);
        assert_eq!(graph.weight(2, 3), 0);
        assert_eq!(graph.weight(0, 3), 1);
        assert_eq!(graph.weight(2, 1), 1);
        assert_eq!(total_edge_multiplicity(&graph), before_total);
    }

    #[test]
    fn swap_rejects_low_degree_and_conflicting_quartets() {
        let mut low_degree = graph_with_edges(5, &[(0, 1, 1), (0, 4, 1), (2, 3, 1), (2, 4, 1)]);
        let before = low_degree.clone();
        GraphOperation::Swap.apply(&mut low_degree, 0, 2, 0, 0);
        assert_eq!(low_degree, before);

        let mut conflict = graph_with_edges(
            6,
            &[
                (0, 1, 1),
                (0, 4, 1),
                (0, 5, 1),
                (2, 3, 1),
                (2, 4, 1),
                (2, 5, 1),
                (0, 3, 1),
            ],
        );
        let before = conflict.clone();
        GraphOperation::Swap.apply(&mut conflict, 0, 2, 0, 0);
        assert_eq!(conflict, before);
    }

    #[test]
    fn swap_rejects_when_the_two_neighbors_are_already_adjacent() {
        // Same quartet as the conflict case above — 0 and 2 swap their
        // neighbors 1 and 3 — except the edge that already exists is (1, 3),
        // the pair the swap never touches. Both would-be edges (0, 3) and
        // (2, 1) are absent, so only the anti-clustering guard can reject it.
        let mut clustered = graph_with_edges(
            6,
            &[
                (0, 1, 1),
                (0, 4, 1),
                (0, 5, 1),
                (2, 3, 1),
                (2, 4, 1),
                (2, 5, 1),
                (1, 3, 1),
            ],
        );
        assert_eq!(clustered.weight(0, 3), 0);
        assert_eq!(clustered.weight(2, 1), 0);

        let before = clustered.clone();
        GraphOperation::Swap.apply(&mut clustered, 0, 2, 0, 0);

        assert_eq!(clustered, before);
    }

    #[test]
    fn null_and_unknown_opcodes_are_noops() {
        let mut graph = graph_with_edges(2, &[(0, 1, 1)]);
        let before = graph.clone();

        GraphOperation::Null.apply(&mut graph, 0, 1, 0, 0);

        assert_eq!(graph, before);
        assert_eq!(GraphOperation::from_opcode(9), None);
        assert_eq!(GraphOperation::from_opcode(15), None);
    }
}
