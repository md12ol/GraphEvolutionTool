#!/usr/bin/env python3
"""Write the three base graphs the example bundle feeds to edge editing.

    python3 tools/make_base_graphs.py get-examples

One optional argument, the directory to write into; it defaults to the
working directory. Writes `base_graph_ring.csv`, `base_graph_empty.csv` and
`base_graph_powerlaw.csv`, overwriting them.

The three exist to be swapped for each other in one config's `[genome]
base_graph` and compared, so they share a node count and differ only in
structure: a regular lattice, nothing at all, and a heavy-tailed hub graph.

Standard library only, like everything else in `tools/`. Regenerating is
byte-stable — the ring is deterministic by construction and the power-law
graph is seeded — so a rerun that changes a file means this script changed.
"""

import os
import random
import sys

NUM_NODES = 100

RING_NEIGHBOURS = 2
"""Each node joins its +-1 and +-2 neighbours, giving a degree of 4.

A plain cycle leaves every node at degree 2, where edge-edit's delete and
rewire operations have almost nothing to work on.
"""

POWERLAW_ATTACHMENTS = 2
POWERLAW_TRIANGLE_PROB = 0.4
POWERLAW_SEED = 42


def write_edge_file(path, num_nodes, edges):
    """Write an edge list GET can load, with the mandatory node-count header.

    Indices are written 0-based, which is what `[genome] base_graph` reads
    and what GET writes back out; a file numbered from 1 loads only through
    `set_base_graph_from_file`, which takes a `min_node_index`.
    """
    with open(path, "w", newline="\n") as handle:
        handle.write(f"# nodes = {num_nodes}\n")
        for start, end in edges:
            handle.write(f"{start},{end},1\n")


def ring_edges(num_nodes, neighbours):
    """Every node joined to its +-1 .. +-`neighbours` neighbours, as a cycle."""
    edges = set()
    for node in range(num_nodes):
        for offset in range(1, neighbours + 1):
            other = (node + offset) % num_nodes
            if node != other:
                edges.add((min(node, other), max(node, other)))
    return sorted(edges)


def powerlaw_cluster_edges(num_nodes, attachments, triangle_prob, seed):
    """Grow a graph by preferential attachment that also closes triangles.

    Each new node takes `attachments` edges to existing nodes drawn from a
    list in which a node appears once per edge it already has, so the draw
    is proportional to degree and the degree distribution comes out
    heavy-tailed. After each such edge, with probability `triangle_prob`,
    it also joins one of that target's neighbours — without this the graph
    is a hub-and-spoke tree with no clustering to speak of.
    """
    rng = random.Random(seed)
    edges = set()
    neighbours = [set() for _ in range(num_nodes)]

    # A node appears once per edge it has, so `choice` is degree-weighted.
    draw_pool = []

    def join(start, end):
        if start == end or end in neighbours[start]:
            return False
        neighbours[start].add(end)
        neighbours[end].add(start)
        edges.add((min(start, end), max(start, end)))
        draw_pool.append(start)
        draw_pool.append(end)
        return True

    # Seed the pool with a path, so the first draw has somewhere to land.
    for node in range(1, attachments):
        join(node - 1, node)
    if not draw_pool:
        draw_pool.append(0)

    for new_node in range(attachments, num_nodes):
        targets = set()
        while len(targets) < attachments:
            targets.add(rng.choice(draw_pool))
        for target in targets:
            if neighbours[target] and rng.random() < triangle_prob:
                join(new_node, rng.choice(sorted(neighbours[target])))
            join(new_node, target)

    return sorted(edges)


def main():
    out_dir = sys.argv[1] if len(sys.argv) > 1 else "."
    os.makedirs(out_dir, exist_ok=True)

    graphs = {
        "base_graph_ring.csv": ring_edges(NUM_NODES, RING_NEIGHBOURS),
        "base_graph_empty.csv": [],
        "base_graph_powerlaw.csv": powerlaw_cluster_edges(
            NUM_NODES,
            POWERLAW_ATTACHMENTS,
            POWERLAW_TRIANGLE_PROB,
            POWERLAW_SEED,
        ),
    }
    for name, edges in graphs.items():
        path = os.path.join(out_dir, name)
        write_edge_file(path, NUM_NODES, edges)
        print(f"{path}: {NUM_NODES} nodes, {len(edges)} edges")


if __name__ == "__main__":
    main()
