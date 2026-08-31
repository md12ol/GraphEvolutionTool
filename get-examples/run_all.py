r"""Run every example configuration beside this file, then plot each result.

    python run_all.py

On Windows, `.venv\Scripts\python.exe run_all.py`, as everywhere else in this
bundle.

It runs `python_from_config.py` once per `NN_*.toml`, in name order, and then
`analyze_output.py` on each `output/example_N/` the run produced. Nothing here
does anything you could not do by hand; it exists so that "run the whole bundle
and look at the pictures" is one command rather than ten.

`05` is the exercise, and shipping it already solved would remove the exercise.
So this script does not edit `python_from_config.py`: it writes a copy beside it
with the commented block uncommented, runs that, and deletes it. The copy sits
in the same folder so that the output and base-graph paths, which resolve
relative to the script, land where they would have anyway. If you have already
uncommented the block yourself, the copy is your file verbatim, so what runs is
what you wrote either way.

The plots need matplotlib, networkx and scipy, which the rest of the bundle does
not. They are checked before the first run rather than after the last, so that a
missing import is not found at the end of every run instead of before the first:

    pip install matplotlib networkx scipy
"""

import argparse
import glob
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
OUTPUT_ROOT = os.path.join(HERE, "output")

RUNNER = os.path.join(HERE, "python_from_config.py")
ANALYSER = os.path.join(HERE, "analyze_output.py")

# The generated copy of the runner that has `05`'s objective uncommented. The
# leading dot keeps it out of the way, and it is deleted after the run whether
# or not the run succeeded.
PATCHED_RUNNER = os.path.join(HERE, ".run_all_with_objective.py")

PLOT_PACKAGES = ["matplotlib", "networkx", "scipy"]

# `analyze_output.py` imports `graph_to_png.py` to read the edge-file format.
# The download ships a copy beside it, but the repository keeps exactly one, in
# `tools/`, so a checkout has nothing to import and the network drawing fails
# with an error about unpacking the whole bundle. Where that is the situation,
# `tools/` goes on the child's import path. In the download the file is present
# and this finds nothing to add.
REPO_TOOLS = os.path.join(os.path.dirname(HERE), "tools")

# The first line of the commented-out block in `register_objective`, and the
# last. Matching both, rather than assuming the block's extent, means a block
# that has been edited into a different shape is noticed instead of half
# uncommented.
BLOCK_FIRST = "TARGET_DEGREE"
BLOCK_LAST = "evolver.set_fitness_function"


def configs():
    """Every `.toml` beside this file, in name order.

    Any of them, not only the numbered ones: a configuration someone dropped in
    beside the shipped five is a configuration, and the numbers are what makes
    the order meaningful rather than what makes a file eligible.
    """
    found = []
    for name in sorted(os.listdir(HERE)):
        if name.endswith(".toml"):
            found.append(name)
    return found


def needs_uncommenting(config_name):
    """Whether this configuration selects a Python objective that is not registered.

    Only `05` does, and asking the file rather than its number means a sixth
    example with the same shape is handled without editing this script.
    """
    with open(os.path.join(HERE, config_name), encoding="utf-8") as handle:
        text = handle.read()
    return re.search(r'^\s*type\s*=\s*"python"', text, re.MULTILINE) is not None


def uncomment_objective(text):
    """The runner's source with `register_objective`'s block uncommented.

    Returns the text unchanged if the block is already live, and raises if the
    block cannot be found in either state, which means the file has been edited
    into a shape this does not understand and guessing would be worse than
    stopping.
    """
    lines = text.splitlines(keepends=True)

    live = False
    start = None
    end = None
    for index, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith("#"):
            body = stripped.lstrip("#").strip()
            if body.startswith(BLOCK_FIRST) and start is None:
                start = index
            if body.startswith(BLOCK_LAST):
                end = index
        elif stripped.startswith(BLOCK_LAST):
            live = True

    if live:
        return text

    if start is None or end is None or end < start:
        raise ValueError(
            f"{os.path.basename(RUNNER)} has no commented objective block to uncomment.\n"
            f"    expected a commented `{BLOCK_FIRST}` line followed by a commented\n"
            f"    `{BLOCK_LAST}` line inside register_objective()"
        )

    for index in range(start, end + 1):
        line = lines[index]
        indent = line[: len(line) - len(line.lstrip())]
        rest = line.strip()[1:]
        # A comment marking a blank line carries nothing after the `#`; one
        # marking code carries a single separating space that is not part of it.
        if rest.startswith(" "):
            rest = rest[1:]
        lines[index] = f"{indent}{rest}\n" if rest else "\n"

    return "".join(lines)


def write_patched_runner():
    """`PATCHED_RUNNER`, written from the real runner with `05`'s block live."""
    with open(RUNNER, encoding="utf-8") as handle:
        text = handle.read()
    with open(PATCHED_RUNNER, "w", encoding="utf-8") as handle:
        handle.write(uncomment_objective(text))
    return PATCHED_RUNNER


def example_directories():
    """The set of `output/example_N/` folders that exist right now."""
    found = set()
    for path in glob.glob(os.path.join(OUTPUT_ROOT, "example_*")):
        if os.path.isdir(path):
            found.add(path)
    return found


def check_plot_packages():
    """Raise unless every package the plots need is importable."""
    missing = []
    for name in PLOT_PACKAGES:
        try:
            __import__(name)
        except ImportError:
            missing.append(name)

    if missing:
        raise ValueError(
            f"the plots need {', '.join(missing)}, which {'is' if len(missing) == 1 else 'are'}"
            " not installed.\n"
            f"    pip install {' '.join(PLOT_PACKAGES)}\n"
            "Pass --no-plots to run the configurations without analysing them."
        )


def child_environment():
    """The environment the child scripts run in.

    Identical to this process's, except that a checkout gets `tools/` on
    `PYTHONPATH` so `graph_to_png.py` is importable. See `REPO_TOOLS`.
    """
    environment = dict(os.environ)
    if os.path.isfile(os.path.join(HERE, "graph_to_png.py")):
        return environment
    if not os.path.isfile(os.path.join(REPO_TOOLS, "graph_to_png.py")):
        return environment

    existing = environment.get("PYTHONPATH")
    environment["PYTHONPATH"] = REPO_TOOLS + os.pathsep + existing if existing else REPO_TOOLS
    return environment


def run(command, label):
    """Run one subprocess, streaming its output, and raise if it fails."""
    # Flushed, or these headers land after the child's output: this process
    # writes to a buffered pipe and the child writes to the terminal directly.
    print(f"\n=== {label} ===", flush=True)
    print("    " + " ".join(command), flush=True)
    completed = subprocess.run(command, cwd=HERE, env=child_environment(), check=False)
    if completed.returncode != 0:
        raise ValueError(f"{label} failed with exit code {completed.returncode}")


def run_one(config_name, seed):
    """Run one configuration, and return the `output/example_N/` it created.

    The folder is identified by diffing the output directory rather than by
    parsing what the run printed, because the runner claims its number after the
    evolution finishes and nothing guarantees this is the only process running.
    """
    before = example_directories()

    runner = write_patched_runner() if needs_uncommenting(config_name) else RUNNER

    command = [sys.executable, runner, config_name]
    if seed is not None:
        command.append(str(seed))
    run(command, f"running {config_name}")

    created = example_directories() - before
    if not created:
        raise ValueError(f"{config_name} produced no new output/example_N folder")
    if len(created) > 1:
        names = ", ".join(sorted(os.path.basename(path) for path in created))
        raise ValueError(f"{config_name} produced more than one output folder: {names}")
    return created.pop()


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--seed", type=int, help="master seed, passed to every run")
    parser.add_argument(
        "--only",
        action="append",
        metavar="CONFIG",
        help="run just this configuration; repeatable (default: all of them)",
    )
    parser.add_argument(
        "--no-plots",
        action="store_true",
        help="run the configurations but do not call analyze_output.py",
    )
    parser.add_argument("--out", help="where generated plots go (default: beside output/example_N)")
    args = parser.parse_args()

    wanted = args.only if args.only else configs()
    if not wanted:
        print("error: no .toml configurations beside this script", file=sys.stderr)
        return 1

    try:
        if not args.no_plots:
            check_plot_packages()

        produced = []
        for config_name in wanted:
            directory = run_one(config_name, args.seed)
            produced.append((config_name, directory))

        if not args.no_plots:
            for config_name, directory in produced:
                command = [sys.executable, ANALYSER, os.path.relpath(directory, HERE)]
                if args.out:
                    command.extend(["--out", args.out])
                run(command, f"plotting {config_name}")

            # One more pass over every folder at once, for the two figures that
            # compare folders: the boxplot, which needs more than one folder and
            # is skipped by every pass above, and the combined convergence grid.
            # Not the networks. Each of those is named after the single folder it
            # shows rather than after the set it was asked for, so drawing them
            # here would rewrite the five files the passes above just wrote, with
            # the same bytes.
            if len(produced) > 1:
                command = [sys.executable, ANALYSER, "--no-networks"]
                for _, directory in produced:
                    command.append(os.path.relpath(directory, HERE))
                if args.out:
                    command.extend(["--out", args.out])
                run(command, "plotting all folders together")
    except (ValueError, OSError) as error:
        print(f"\nerror: {error}", file=sys.stderr)
        return 1
    finally:
        if os.path.exists(PATCHED_RUNNER):
            os.remove(PATCHED_RUNNER)

    print(f"\n=== done: {len(produced)} configuration(s) ===")
    for config_name, directory in produced:
        print(f"    {config_name} -> {os.path.relpath(directory, HERE)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
