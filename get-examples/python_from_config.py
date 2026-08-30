r"""Run one of the example configurations, with every parameter read from a TOML file.

Work from inside this folder, and install GET into a virtual environment first.
On Linux or macOS:

    python3 -m venv .venv
    source .venv/bin/activate
    pip install graph-evolution-tool

On Windows, in PowerShell. Nothing is activated: a stock machine refuses to run
`Activate.ps1`, so every command names the environment's own interpreter, and
that is the spelling used throughout the rest of this file:

    py -m venv .venv
    .venv\Scripts\python.exe -m pip install graph-evolution-tool

Then run any of the configurations beside this file:

    python python_from_config.py 01_edge_edit_generational.toml
    python python_from_config.py 02_sda_steady_state.toml 42

or, on Windows:

    .venv\Scripts\python.exe python_from_config.py 01_edge_edit_generational.toml
    .venv\Scripts\python.exe python_from_config.py 02_sda_steady_state.toml 42

The optional second argument is the master seed. Run `01` through `04` as they
are. `05` is the exercise: uncomment the one block in `register_objective` below
before you run it, or it stops with a message asking for an objective.

Results are written to `output/example_N/`, N counting up from whatever is
already there, so one run never overwrites another. Each example folder keeps a
copy of the configuration that produced it, so the folder says what it is
without opening anything inside it, and holds one `run_M/` per replicate. Each
replicate writes its convergence log and the winning network as an edge list GET
can read back — so one run's result can be the next run's base graph.

Every parameter lives in the TOML, the base graph included — `03` names its
starting network with a `base_graph` key, resolved beside the configuration
file rather than beside whatever directory you ran from.
"""

import os
import shutil
import sys

import get

N_RUNS = 10
"""How many replicates to draw from the master seed.

Each replicate gets its own derived seed, so they are independent samples of
the same configuration and the master seed still reproduces the whole set. Ten
rather than two because an example folder is one distribution when it is
plotted, and a box drawn from two points describes nothing.
"""

DEFAULT_SEED = 7

HERE = os.path.dirname(os.path.abspath(__file__))
OUTPUT_ROOT = os.path.join(HERE, "output")


def register_objective(evolver):
    """Give `05` the objective its `[fitness] type = "python"` selects.

    Uncomment every line of the block below before running `05`. Nothing else
    in this file, and nothing in the configuration, needs to change.

    The objective counts how many nodes have exactly `TARGET_DEGREE`, and asks
    GET to maximize that count. Both the target and the direction live here
    rather than in the TOML: a registered callable can read whatever it likes,
    so there is nothing for the configuration to describe, and nothing can
    infer whether your function wants its value large or small.

    A callable is handed the whole population at once — a list of
    `(num_nodes, edges)`, each edge a `(start, end, weight)` triple — and
    returns one score per graph, in the same order.

    Degree here sums edge weights, so a doubled edge counts twice. It makes no
    difference to `05`, which caps multiplicity at 1, but it is what the word
    means everywhere else in GET.
    """
    # TARGET_DEGREE = 4
    #
    # def count_nodes_at_target_degree(batch):
    #     scores = []
    #     for num_nodes, edges in batch:
    #         degree = [0] * num_nodes
    #         for start, end, weight in edges:
    #             degree[start] += weight
    #             degree[end] += weight
    #         matching = 0
    #         for node_degree in degree:
    #             if node_degree == TARGET_DEGREE:
    #                 matching += 1
    #         scores.append(float(matching))
    #     return scores
    #
    # evolver.set_fitness_function(count_nodes_at_target_degree, "maximize")


def next_example_directory():
    """The `output/example_N/` this invocation writes to, created.

    N is one past the highest that already exists, so a second run of the same
    configuration lands beside the first instead of on top of it.
    """
    os.makedirs(OUTPUT_ROOT, exist_ok=True)

    prefix = "example_"
    highest = 0
    for name in os.listdir(OUTPUT_ROOT):
        suffix = name[len(prefix):]
        if name.startswith(prefix) and suffix.isdigit():
            number = int(suffix)
            if number > highest:
                highest = number

    directory = os.path.join(OUTPUT_ROOT, f"{prefix}{highest + 1}")
    os.makedirs(directory, exist_ok=True)
    return directory


def run_directory(example_directory, run_index):
    """Where one replicate's files go, created if needed.

    `run_index` is zero-based, because `(seed, run_index)` is the pair that
    reproduces a replicate. The directory is numbered from one, matching what
    the run prints, and zero-padded to the width of `N_RUNS` so that ten or more
    replicates still sort in order in a shell, a file browser, or a glob.
    """
    directory = example_directory
    if N_RUNS > 1:
        width = len(str(N_RUNS))
        directory = os.path.join(directory, f"run_{run_index + 1:0{width}d}")
    os.makedirs(directory, exist_ok=True)
    return directory


def usage():
    """Print how to call this, and which configurations are available."""
    print("usage: python python_from_config.py <config.toml> [seed]")
    print()
    print("Configurations beside this file:")
    for name in sorted(os.listdir(HERE)):
        if name.endswith(".toml"):
            print(f"    {name}")


def main():
    if len(sys.argv) < 2:
        usage()
        return 1

    config_path = sys.argv[1]
    seed = int(sys.argv[2]) if len(sys.argv) > 2 else DEFAULT_SEED

    print(f"config = {config_path}, seed = {seed}, runs = {N_RUNS}")

    evolver = get.GraphEvolver(config_path)
    register_objective(evolver)

    results = evolver.run(seed=seed, n_runs=N_RUNS)

    # Claimed after the run, so a run that fails leaves no empty directory
    # behind and does not consume a number.
    example_directory = next_example_directory()
    shutil.copy2(config_path, os.path.join(example_directory, os.path.basename(config_path)))

    for run_index, result in enumerate(results):
        if N_RUNS > 1:
            print(
                f"\n=== run {run_index + 1} of {N_RUNS} "
                f"(reproduce with seed={seed}, run_index={run_index}) ==="
            )

        print(f"best_fitness = {result.best_fitness}")
        print(f"nodes        = {result.num_nodes}")
        print(f"edges        = {len(result.best_edges)}")

        first = result.history[0].best_fitness
        last = result.history[-1]
        print(
            f"best-of-run went {first:.3f} -> {last.best_fitness:.3f} "
            f"over {last.iteration} iterations"
        )

        directory = run_directory(example_directory, run_index)
        result.save_logs(os.path.join(directory, "run_log.csv"))
        result.save_results(os.path.join(directory, "best_individual.txt"))
        print(f"wrote {os.path.relpath(directory, HERE)}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
