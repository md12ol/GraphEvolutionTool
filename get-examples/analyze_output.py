r"""Turn finished runs into plots: a boxplot, convergence curves, and the winner drawn.

Point it at one or more of the `output/example_N/` folders that
`python_from_config.py` writes, and it reads every replicate underneath them:

    python analyze_output.py output/example_1
    python analyze_output.py output/example_1 output/example_2
    python analyze_output.py output/example_*

With no `--out`, each generated file lands beside the folders it was drawn
from, named after the set that produced it, so analysing two different
comparisons into the same `output/` does not overwrite either.

**This script needs matplotlib, and it is the only file in the bundle that
needs anything.** Everything else here runs on a bare interpreter with GET
installed. This one is a workshop tool rather than part of the package, so it
may depend on things the wheel does not:

    pip install matplotlib

Reading and summarising work without it; only the plots require it, and the
error says so if it is missing.
"""

import argparse
import csv
import glob
import os
import sys

CONFIG_SUFFIX = ".toml"
LOG_NAME = "run_log.csv"

# The columns `RunResult.save_logs` writes. Read by name rather than by
# position, so a column added to the middle of that header does not silently
# shift what this reads.
ITERATION = "iteration"
BEST_FITNESS = "best_fitness"


class Run:
    """One replicate: its convergence history and where it came from."""

    def __init__(self, directory, iterations, best_fitness, seed, run_index):
        self.directory = directory
        self.iterations = iterations
        self.best_fitness = best_fitness
        self.seed = seed
        self.run_index = run_index

    @property
    def name(self):
        return os.path.basename(self.directory)

    @property
    def final_fitness(self):
        return self.best_fitness[-1]


class Example:
    """One `example_N/` folder: the config that produced it and its replicates."""

    def __init__(self, directory, config, runs):
        self.directory = directory
        self.config = config
        self.runs = runs

    @property
    def name(self):
        return os.path.basename(os.path.normpath(self.directory))

    @property
    def final_fitnesses(self):
        values = []
        for run in self.runs:
            values.append(run.final_fitness)
        return values


def read_log(path):
    """Return `(iterations, best_fitness, seed, run_index)` from one `run_log.csv`.

    `iteration` is the evolver's own iteration number, not the row index: the
    logging cadence is a configuration choice, so a 20000-iteration run logged
    every 100 has the same 200 rows as a 200-iteration run logged every 1.
    Anything comparing two runs has to compare these numbers, never positions.
    """
    iterations = []
    best_fitness = []
    seed = None
    run_index = None

    with open(path, newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        for row in reader:
            iterations.append(int(row[ITERATION]))
            best_fitness.append(float(row[BEST_FITNESS]))
            if seed is None:
                seed = row.get("seed")
                run_index = row.get("run_index")

    if not iterations:
        raise ValueError(f"{path} has a header but no rows")

    return iterations, best_fitness, seed, run_index


def find_config(directory):
    """The name of the config copied into an example folder, or None.

    `python_from_config.py` copies the configuration in beside the run folders
    so the folder says what it is. Two folders sharing a config name are two
    runs of the same experiment, which is what the boxplot groups on.
    """
    for entry in sorted(os.listdir(directory)):
        if entry.endswith(CONFIG_SUFFIX):
            return entry
    return None


def read_example(directory):
    """Read one `example_N/` folder into an `Example`.

    Handles both layouts `python_from_config.py` produces: `run_01/`, `run_02/`
    ... when there is more than one replicate, and the log written directly into
    the example folder when there is exactly one.
    """
    if not os.path.isdir(directory):
        raise ValueError(f"{directory} is not a directory")

    runs = []
    direct = os.path.join(directory, LOG_NAME)
    if os.path.isfile(direct):
        iterations, best, seed, index = read_log(direct)
        runs.append(Run(directory, iterations, best, seed, index))
    else:
        for entry in sorted(os.listdir(directory)):
            candidate = os.path.join(directory, entry, LOG_NAME)
            if os.path.isfile(candidate):
                iterations, best, seed, index = read_log(candidate)
                runs.append(Run(os.path.join(directory, entry), iterations, best, seed, index))

    if not runs:
        raise ValueError(f"{directory} holds no {LOG_NAME}")

    return Example(directory, find_config(directory), runs)


def expand(paths):
    """Every example folder named by the arguments, de-duplicated, in order.

    A shell normally expands `output/example_*` before this sees it, but an
    unexpanded pattern is passed through too — on Windows PowerShell it arrives
    here verbatim.
    """
    found = []
    for path in paths:
        matches = sorted(glob.glob(path)) if glob.has_magic(path) else [path]
        if not matches:
            raise ValueError(f"{path} matched nothing")
        for match in matches:
            normalised = os.path.normpath(match)
            if normalised not in found:
                found.append(normalised)
    return found


def summarise(examples):
    """Print what was read, so a comparison can be checked before it is drawn."""
    for example in examples:
        config = example.config or "no config copied in"
        print(f"{example.name}  ({config})  {len(example.runs)} replicate(s)")
        for run in example.runs:
            span = f"{run.iterations[0]}..{run.iterations[-1]}"
            print(
                f"    {run.name:<8} final best_fitness = {run.final_fitness:<10.4g} "
                f"iterations {span} over {len(run.iterations)} rows"
            )


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("folders", nargs="+", help="one or more output/example_N directories")
    parser.add_argument("--out", help="where generated files go (default: beside the folders)")
    args = parser.parse_args()

    try:
        directories = expand(args.folders)
        examples = []
        for directory in directories:
            examples.append(read_example(directory))
    except (ValueError, OSError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    summarise(examples)
    return 0


if __name__ == "__main__":
    sys.exit(main())
