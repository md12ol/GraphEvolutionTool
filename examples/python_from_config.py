"""Python route 2 of 2: one evolution, with every parameter read from a TOML file.

Run it after installing the extension module:

    maturin develop        # or: pip install .
    python examples/python_from_config.py [config.toml] [seed]

This is the route for someone who wants the run's parameters to be a document
rather than a program — one that can be archived beside the results, diffed
against last week's, or handed to somebody who does not read Python. The file
it defaults to is `config.example.toml` at the repository root.

Its counterpart, `python_inline.py`, writes the same parameters into the
program itself. Between them they are the two Python routes; `config_builder.py`
beside them builds configurations without running anything.

**A base graph is not a config key**, on this route or any other — it is data,
not configuration. Supply one with `set_base_graph_from_file` as shown below,
and note that the numbering you declare there is the numbering results come
back in.
"""

import os
import sys
import warnings

import get

from _output_layout import run_output_dir, utc_stamp

# --- What you would change ---------------------------------------------------

# Where results go. Each run lands in OUTPUT_DIR/<timestamp>-<seed>/, and each
# replicate in a run_<index>/ of its own inside that.
OUTPUT_DIR = "./output"

# The default document, relative to this file, so the example runs from
# anywhere. Override it with the first command-line argument.
DEFAULT_CONFIG = os.path.join(os.path.dirname(__file__), "..", "config.example.toml")

# How many replicates to draw from the master seed. A replicate is reproduced by
# re-running with the same master seed and reading the same run_index.
N_RUNS = 2

# Set to a path to start every individual from a graph you supply, rather than
# from an empty one. Any file will do as long as it carries a `# nodes = N`
# header. `examples/base_graph.csv` is a small ring, and it is **1-indexed** —
# which is why the second constant is here at all: declare where your own
# numbering starts, and results come back in it.
BASE_GRAPH = None  # e.g. os.path.join(os.path.dirname(__file__), "base_graph.csv")
BASE_GRAPH_MIN_NODE_INDEX = 1


def main():
    config_path = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_CONFIG
    seed = int(sys.argv[2]) if len(sys.argv) > 2 else 7

    print(f"config = {config_path}, seed = {seed}, runs = {N_RUNS}")

    # A bad document is reported here, by name and constraint, before anything
    # is built — the same message the Rust route would print for the same file.
    evolver = get.GraphEvolver(config_path)

    if BASE_GRAPH is not None:
        # Load problems that do not stop the run arrive as UserWarnings — a
        # repeated edge, a zero-weight edge, an empty file. Turning them into
        # errors is a one-liner if you would rather not run past one.
        with warnings.catch_warnings():
            warnings.simplefilter("always")
            evolver.set_base_graph_from_file(
                BASE_GRAPH,
                min_node_index=BASE_GRAPH_MIN_NODE_INDEX,
            )

    # One stamp for the whole invocation, taken before anything runs.
    stamp = utc_stamp()

    results = evolver.run(seed=seed, n_runs=N_RUNS)

    for run_index, result in enumerate(results):
        if N_RUNS > 1:
            print(f"\n=== run_index {run_index}, of {N_RUNS} ===")

        print(f"best_fitness = {result.best_fitness}")
        print(f"edges        = {len(result.best_edges)}")

        first = result.history[0].best_fitness
        last = result.history[-1]
        print(
            f"best-of-run went {first:.3f} -> {last.best_fitness:.3f} "
            f"over {last.iteration} iterations"
        )

        directory = run_output_dir(OUTPUT_DIR, stamp, seed, run_index, N_RUNS)

        # The winner is written as an edge list GET can read back, header and
        # all, so one run's result is the next run's base graph. It comes back
        # in whatever numbering the loader above declared.
        result.save_logs(f"{directory}/run_log.csv")
        result.save_results(f"{directory}/best_individual.txt")
        print(f"wrote {directory}")


if __name__ == "__main__":
    main()
