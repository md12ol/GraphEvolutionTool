# `tiny_tu` — hand-written TUDataset fixture

Four graphs, 15 nodes, 10 undirected edges. Each graph exercises one thing the
converter has to get right, and every one of them is silent if wrong.

| Graph | Global nodes | Nodes | Edges | What it is for |
|---|---|---|---|---|
| 1 | 1–4   | 4 | 4 | Ordinary. A 4-ring — checks renumbering and one-direction output |
| 2 | 5–8   | 4 | 3 | Triangle on 5,6,7 with node **8 isolated and trailing**. Edge-based inference sees 3 nodes; the indicator says 4 |
| 3 | 9–11  | 3 | 0 | No edges at all. The sentinel alone carries its size — every node isolated, which is what the source says |
| 4 | 12–15 | 4 | 3 | A path. Checks that a graph after the empty one is not knocked out of position |

`tiny_tu_A.txt` carries **both directions** of every edge, as TUDataset does —
20 lines for 10 edges — and is deliberately not fully sorted.

Counts, for `wc -l`: `tiny_tu_graph_indicator.txt` is 15 lines, `tiny_tu_A.txt`
is 20.
