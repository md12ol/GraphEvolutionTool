#!/usr/bin/env python3
"""Convert a TUDataset collection into a folder GET's `load_edge_folder` reads.

Run once per dataset. Standard library only -- no `get`, no torch_geometric.

TUDataset ships one global edge file plus a node-to-graph map:

    DS_A.txt                 "u, v" per line, BOTH directions, ids 1-based and
                             numbered across the entire dataset
    DS_graph_indicator.txt   line i names the graph that global node i is in

GET wants the opposite shape: one file per graph, a `# nodes = N` header, then
`u,v,weight` per line, each undirected edge once, node ids 0-based and local to
that graph.

Three things here are silent if wrong, so each is checked rather than assumed:

  * The delimiter is a COMMA. GET's parser splits on ',' and rejects anything
    else as a bad column count -- a whitespace-separated file fails on line 1.
  * An undirected edge is written ONCE, as u < v. GET warns per collapsed
    duplicate but still loads, so emitting both directions produces a
    silently-wrong reference set.
  * A graph's node count comes from the indicator file, never from the highest
    edge index, and is written as the `# nodes = N` header GET requires. A
    trailing isolated node has no edges at all, so nothing in the rows below
    the header could carry it.
"""

import argparse
import os
import sys


class ConversionError(Exception):
    """Input that cannot be converted without guessing at what was meant."""


def find_dataset_files(dataset_dir):
    """Locate `DS_A.txt` and `DS_graph_indicator.txt` under a dataset directory.

    TUDataset downloads land as either `<root>/<DS>/<DS>_A.txt` or, via
    torch_geometric, `<root>/<DS>/raw/<DS>_A.txt`. Both are accepted, and the
    dataset's own name prefix is read off whichever file is found rather than
    being asked for on the command line.
    """
    candidates = [dataset_dir, os.path.join(dataset_dir, "raw")]

    for directory in candidates:
        if not os.path.isdir(directory):
            continue

        for entry in sorted(os.listdir(directory)):
            if not entry.endswith("_A.txt"):
                continue

            name = entry[: -len("_A.txt")]
            edges = os.path.join(directory, entry)
            indicator = os.path.join(directory, name + "_graph_indicator.txt")

            if not os.path.isfile(indicator):
                raise ConversionError(
                    "found {} but no {} beside it; the indicator file is what "
                    "gives each graph its node count".format(edges, indicator)
                )

            return name, edges, indicator

    raise ConversionError(
        "no `*_A.txt` in {} or {}/raw".format(dataset_dir, dataset_dir)
    )


def read_indicator(path):
    """Map each global node id to its graph, and each graph to its nodes.

    Returns `(node_graph, graph_ids, graph_nodes)`. Node ids are 1-based, as
    the file writes them: line i describes node i.

    `graph_ids` is in first-appearance order and `graph_nodes[g]` is ascending,
    so the local numbering below is a function of the file alone.
    """
    node_graph = {}
    graph_ids = []
    graph_nodes = {}

    with open(path, "r") as handle:
        for index, raw_line in enumerate(handle):
            line = raw_line.strip()
            if not line:
                continue

            node = index + 1
            try:
                graph = int(line)
            except ValueError:
                raise ConversionError(
                    "{}, line {}: {!r} is not a graph id".format(path, node, line)
                )

            node_graph[node] = graph
            if graph not in graph_nodes:
                graph_nodes[graph] = []
                graph_ids.append(graph)
            graph_nodes[graph].append(node)

    if not graph_ids:
        raise ConversionError("{} is empty; there are no graphs to convert".format(path))

    return node_graph, graph_ids, graph_nodes


def build_local_numbering(graph_nodes):
    """Global node id -> 0-based index within its own graph.

    Built from the indicator file rather than re-derived from edge structure,
    so a node with no edges still gets a number.
    """
    local = {}
    for nodes in graph_nodes.values():
        for position, node in enumerate(sorted(nodes)):
            local[node] = position
    return local


def read_edges(path, node_graph, local_index):
    """Collect each graph's undirected edges, in local 0-based numbering.

    Returns `(edges_by_graph, self_loops, duplicate_rows)`. `edges_by_graph[g]`
    is a list of `(u, v)` with u < v, first-appearance order, no repeats --
    `DS_A.txt` carries both directions, and the second one is dropped here
    rather than left for GET's loader to warn about.
    """
    edges_by_graph = {}
    seen_by_graph = {}
    self_loops = 0
    duplicate_rows = 0

    with open(path, "r") as handle:
        for index, raw_line in enumerate(handle):
            line = raw_line.strip()
            if not line:
                continue

            fields = line.split(",")
            if len(fields) != 2:
                raise ConversionError(
                    "{}, line {}: expected `u, v` but found {} comma-separated "
                    "fields".format(path, index + 1, len(fields))
                )

            try:
                source = int(fields[0].strip())
                target = int(fields[1].strip())
            except ValueError:
                raise ConversionError(
                    "{}, line {}: {!r} is not a pair of node ids".format(
                        path, index + 1, line
                    )
                )

            # A self-loop would make GET reject the whole file, taking a
            # perfectly good graph with it, so it is dropped and counted.
            if source == target:
                self_loops += 1
                continue

            if source not in node_graph:
                raise ConversionError(
                    "{}, line {}: node {} is not in the indicator file".format(
                        path, index + 1, source
                    )
                )
            if target not in node_graph:
                raise ConversionError(
                    "{}, line {}: node {} is not in the indicator file".format(
                        path, index + 1, target
                    )
                )

            graph = node_graph[source]
            if node_graph[target] != graph:
                raise ConversionError(
                    "{}, line {}: edge ({}, {}) spans graphs {} and {}".format(
                        path, index + 1, source, target, graph, node_graph[target]
                    )
                )

            low = min(local_index[source], local_index[target])
            high = max(local_index[source], local_index[target])

            if graph not in edges_by_graph:
                edges_by_graph[graph] = []
                seen_by_graph[graph] = set()

            if (low, high) in seen_by_graph[graph]:
                duplicate_rows += 1
                continue

            seen_by_graph[graph].add((low, high))
            edges_by_graph[graph].append((low, high))

    return edges_by_graph, self_loops, duplicate_rows


def convert(dataset_dir, output_dir, manifest_path=None, quiet=False):
    """Convert one dataset. Returns the manifest rows, one per graph."""
    name, edge_path, indicator_path = find_dataset_files(dataset_dir)

    node_graph, graph_ids, graph_nodes = read_indicator(indicator_path)
    local_index = build_local_numbering(graph_nodes)
    edges_by_graph, self_loops, duplicate_rows = read_edges(
        edge_path, node_graph, local_index
    )

    if not os.path.isdir(output_dir):
        os.makedirs(output_dir)

    # Zero-padded: `load_edge_folder` sorts by file name and a reference set is
    # consumed positionally, so `graph_10` must not sort before `graph_2`.
    width = len(str(len(graph_ids)))

    rows = []

    for position, graph in enumerate(graph_ids):
        num_nodes = len(graph_nodes[graph])
        edges = edges_by_graph.get(graph, [])

        # The header first, because it is the one thing the rows below cannot
        # say: a node with no edges appears in none of them. A graph with no
        # edges at all is a header and nothing else, which is a legal file --
        # every node in it is isolated, which is what the source says. One node
        # is legal too, unlike under the sentinel this replaced.
        filename = "graph_{}.txt".format(str(position + 1).zfill(width))
        with open(os.path.join(output_dir, filename), "w") as handle:
            handle.write("# nodes = {}\n".format(num_nodes))
            for low, high in edges:
                handle.write("{},{},1\n".format(low, high))

        rows.append(
            {
                "file": filename,
                "graph_id": graph,
                "num_nodes": num_nodes,
                "num_edges": len(edges),
            }
        )

    if manifest_path is None:
        # Beside the folder, never inside it: `load_edge_folder` reads every
        # regular file in the folder and would reject a manifest as an edge file.
        manifest_path = os.path.join(
            os.path.dirname(os.path.abspath(output_dir)),
            os.path.basename(os.path.abspath(output_dir)) + "_manifest.csv",
        )

    with open(manifest_path, "w") as handle:
        handle.write("file,graph_id,num_nodes,num_edges\n")
        for row in rows:
            handle.write(
                "{},{},{},{}\n".format(
                    row["file"],
                    row["graph_id"],
                    row["num_nodes"],
                    row["num_edges"],
                )
            )

    if not quiet:
        report(name, rows, manifest_path, self_loops, duplicate_rows)

    return rows


def report(name, rows, manifest_path, self_loops, duplicate_rows):
    """Print what was converted, and everything the caller has to know about."""
    largest = 0
    for row in rows:
        if row["num_nodes"] > largest:
            largest = row["num_nodes"]

    print("{}: {} graphs".format(name, len(rows)))
    print("manifest: {}".format(manifest_path))
    print("largest graph: {} nodes -- set network_size to at least this".format(largest))

    if duplicate_rows:
        # Every undirected edge appears twice in `DS_A.txt` by design, so this
        # is about half the source rows on a healthy dataset. Named as the
        # reverse directions they are, so a normal conversion does not read
        # like a warning.
        print(
            "collapsed {} reverse directions and repeats (DS_A.txt stores both "
            "directions, so about half of every dataset)".format(duplicate_rows)
        )
    if self_loops:
        print(
            "DROPPED {} self-loops; GET rejects a file containing one, so the "
            "graphs holding them would not have loaded at all".format(self_loops)
        )


def main():
    parser = argparse.ArgumentParser(
        description="Convert a TUDataset collection into a GET reference folder."
    )
    parser.add_argument(
        "dataset_dir",
        help="directory holding DS_A.txt and DS_graph_indicator.txt, or a "
        "parent with them in raw/",
    )
    parser.add_argument("output_dir", help="folder to write one file per graph into")
    parser.add_argument(
        "--manifest",
        default=None,
        help="where to write the manifest (default: beside output_dir). It must "
        "not be inside output_dir -- GET reads every file in there as edges.",
    )
    parser.add_argument(
        "--quiet", action="store_true", help="suppress the summary report"
    )
    args = parser.parse_args()

    try:
        convert(args.dataset_dir, args.output_dir, args.manifest, args.quiet)
    except ConversionError as error:
        sys.stderr.write("error: {}\n".format(error))
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
