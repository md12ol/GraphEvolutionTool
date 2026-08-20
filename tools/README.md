# `tools/` — one-shot scripts that sit outside the crate

Nothing here is part of GET. These are standard-library Python scripts, run by
hand, that prepare data for it or read what it produced. They are not a package,
not an entry point in `pyproject.toml`, and not built into the wheel.

## `tudataset_to_get.py`

Converts a [TUDataset](https://chrsmrrs.github.io/datasets/) collection into a
folder of per-graph edge files that `load_edge_folder` reads — the reference set
`struct_match` matches against.

```bash
python3 tools/tudataset_to_get.py ~/datasets/MUTAG out/mutag_reference
```

It prints the largest graph's node count, which is the floor for `network_size`.

### The four things it has to get right

Each is silent if wrong — the run completes and the numbers are wrong.

1. **Renumber global to local, 0-based**, from `DS_graph_indicator.txt` rather
   than from edge structure, so a node with no edges still gets a number.
2. **Write each undirected edge once**, as `u < v`. `DS_A.txt` carries both
   directions; GET warns per collapsed duplicate but still loads, so emitting
   both produces a wrong reference set that looks fine.
3. **Write three comma-separated fields**, `u,v,1`. GET's parser splits on `,`
   and rejects anything else as a bad column count. *GitHub #137 describes the
   format as whitespace-separated; it is not, and the parser is what runs.*
4. **Take the node count from the indicator file** and write it as the
   `# nodes = N` header. A graph whose last nodes have no edges is invisible to
   edge-based inference, which is the one thing a TUDataset-aware converter
   knows that GET's loader structurally cannot.

### The `# nodes = N` header

Every file GET reads states its own node count on a `#` comment line, and the
loader refuses a file without one. That is the whole reason a converter can be
faithful here: a graph with a trailing isolated node loads at its real size
rather than one node short, and short is not cosmetic — all three reference
histograms count isolated nodes as real observations and normalize over the node
count, so a lost node shifts every distribution the evolver optimizes toward.

The header is checked against the file's own edges (a count below them is an
error, not a truncation) and against the count the run allows, so a typo is
rejected rather than silently sizing a graph nobody can index.

**This replaced a zero-weight sentinel row**, `0,<n-1>,0`, which carried the
count on warning-level behaviour and could not express a one-node graph at all —
a sentinel needs two endpoints, and `u == v` is a self-loop. Both of those go
away with the header: a one-node graph is a header and no rows.

### What it drops, and what it refuses

**Self-loops are dropped and counted.** GET rejects an entire file for one
self-loop, which would take a usable graph with it.

**These are errors, naming the line** — each means the input is corrupt, and
guessing would be worse than stopping: an edge naming a node the indicator file
does not have; an edge spanning two graphs; a row that is not a pair of ids.

**Labels and attributes are ignored.** `DS_node_labels.txt`,
`DS_edge_labels.txt`, `DS_graph_labels.txt` and the attribute files have no
representation in GET's edge format.

### The manifest

Written **beside** the output folder, never inside it — `load_edge_folder` reads
every regular file in the folder and would reject a manifest as a bad edge file.
It records `file,graph_id,num_nodes,num_edges` per graph — a human-readable
record of the conversion, not something GET reads: the node count each graph
loads at is in the file itself, as the header.

### Tests

```bash
python3 -m unittest discover -s tools -v
```

The conversion tests are stdlib-only. The round-trip tests need the built `get`
module and skip with a message without it — this lives outside the crate, so
`cargo test` does not build it.

`testdata/tiny_tu/` is a hand-written four-graph fixture covering the ordinary
case, a trailing isolated node, a graph with no edges, and a graph following an
empty one. Real datasets do not reliably contain the isolated-node case —
MUTAG has none in 188 graphs — which is why it is constructed.

### Verified against a real dataset

MUTAG (2026-08-19): 188 graphs, 3371 nodes, 3721 edges converted and loaded back
through `get` with zero node-count or edge-count mismatches, matching the
dataset's own file counts and its published graph count.
