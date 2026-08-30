r"""Turn finished runs into plots: a boxplot, convergence curves, and the winner drawn.

Point it at one or more of the `output/example_N/` folders that
`python_from_config.py` writes, and it reads every replicate underneath them:

    python analyze_output.py output/example_1
    python analyze_output.py output/example_1 output/example_2
    python analyze_output.py output/example_*

On Windows, write `.venv\Scripts\python.exe` in place of `python` throughout
this file, and `.venv\Scripts\python.exe -m pip` in place of `pip`. Nothing
else changes — the last form above still works, because PowerShell does not
expand `output/example_*` itself, so it arrives here as a literal pattern and is
expanded below rather than by the shell.

With no `--out`, each generated file lands beside the folders it was drawn
from, named after the set that produced it, so analysing two different
comparisons into the same `output/` does not overwrite either.

**This script is the only file in the bundle that needs anything installed.**
Everything else here runs on a bare interpreter with GET. This one is a
workshop tool rather than part of the package, so it may depend on what the
wheel does not:

    pip install matplotlib networkx scipy

matplotlib draws the two plots; networkx lays the winning network out, and its
Kamada-Kawai layout is what needs scipy. Reading and summarising work with none
of them installed, and each error says which one is missing.

**No installation at all?** `graph_to_png.py`, beside this file, turns any one
result into a PNG using nothing but the standard library:

    python graph_to_png.py output/example_1/run_01/best_individual.txt

It is the fallback rather than the main path. Its layout starts from a circle,
which makes it reproducible without a seed but means a ring-shaped picture can
be the starting condition rather than the graph, and it shows neither node
degree nor edge multiplicity — the two things this script colours for.
"""

import argparse
import csv
import glob
import hashlib
import math
import os
import sys

CONFIG_SUFFIX = ".toml"
LOG_NAME = "run_log.csv"
BEST_INDIVIDUAL = "best_individual.txt"

# Beyond this many folders a filename listing them all stops being a filename,
# so the set is identified by a digest of every name instead. The digest is of
# the whole sorted set, so two different comparisons never collide.
MAX_NAME_PARTS = 4
NAME_DIGEST_LENGTH = 8

# Boxes of one config sit together; this is the empty slot between groups.
GROUP_GAP = 1

# Where isolated nodes are ringed, in the coordinates Kamada-Kawai returns for
# the connected part — outside it, so they read as detached rather than central.
STRAY_RADIUS = 1.35

# Up to this multiplicity the edge scale is a legend with one swatch per value;
# above it, a colourbar. A multiplicity is a count of parallel copies, so a run
# capped at 5 or fewer produces a handful of distinct integers, and naming each
# one reads faster than a continuous bar the eye has to measure a colour against.
# Past that the swatches stop fitting and a bar is the honest instrument.
MAX_LEGEND_MULTIPLICITY = 5

# Which part of a colour ramp is used. Both ends of `plasma` are extreme enough
# to lose against white — dark navy below, pale yellow above — and an edge is one
# pixel wide, so the whole edge scale is taken from the middle of the range. The
# swatches and the colourbar sample the same trimmed ramp, so a legend and a bar
# drawn from the same data agree.
RAMP_SPAN = (0.15, 0.85)

# How finely the trimmed ramp is resampled. Enough that a colourbar over it is
# smooth rather than banded.
RAMP_SAMPLES = 64

# Over a wider span than this a colourbar keeps matplotlib's own ticks. They are
# already integers by then, and one per value would be an unreadable stack.
MAX_INTEGER_TICKS = 12

NO_CONFIG = "(no config copied in)"

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


def pyplot():
    """matplotlib's pyplot, with the message this script owes a reader without it.

    Imported here rather than at the top so that reading and summarising work on
    an interpreter that has only GET installed — the plots are the one part of
    the bundle with a dependency, and it should fail where it is used.
    """
    try:
        import matplotlib
    except ImportError:
        raise ValueError(
            "the plots need matplotlib, which is not installed.\n"
            "    pip install matplotlib\n"
            "Reading and summarising work without it."
        ) from None

    # Chosen before pyplot is imported: without it matplotlib looks for a
    # display and fails on a headless machine, which is where a batch of runs
    # is most likely to be analysed.
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    return plt


def comparison_slug(examples):
    """A filename fragment naming exactly this set of folders.

    Sorted, so the same comparison requested in a different order writes the
    same file rather than a second copy, and distinct, so analysing
    `{1, 2}` and then `{1, 3}` into one directory leaves four files rather than
    silently overwriting two.
    """
    names = sorted(example.name for example in examples)
    if len(names) <= MAX_NAME_PARTS:
        return "+".join(names)

    digest = hashlib.sha256("\0".join(names).encode("utf-8")).hexdigest()
    return f"{names[0]}+{len(names) - 1}-more-{digest[:NAME_DIGEST_LENGTH]}"


def output_path(examples, out_dir, kind):
    """Where one generated plot goes, with its directory created."""
    if out_dir is None:
        parents = []
        for example in examples:
            parents.append(os.path.dirname(os.path.abspath(example.directory)))
        out_dir = os.path.commonpath(parents) if len(parents) > 1 else parents[0]

    os.makedirs(out_dir, exist_ok=True)
    return os.path.join(out_dir, f"{kind}__{comparison_slug(examples)}.png")


def group_by_config(examples):
    """`[(config, [example, ...]), ...]`, in the order the configs first appear.

    Two folders produced by the same configuration are replicated experiments
    rather than different ones, and the plots put them side by side so that
    reads as what it is.
    """
    order = []
    groups = {}
    for example in examples:
        key = example.config or NO_CONFIG
        if key not in groups:
            groups[key] = []
            order.append(key)
        groups[key].append(example)

    grouped = []
    for key in order:
        grouped.append((key, groups[key]))
    return grouped


def draw_boxplot(examples, out_dir):
    """One box per example folder, folders of the same config grouped together.

    Returns the path written, or None when there is nothing worth drawing: a
    boxplot of a single folder says nothing its final fitness does not.

    The y-axis is shared, and two configs optimising different objectives do
    not share a scale — the colour and the legend name the config for exactly
    that reason, so a reader can see when they are looking at two units rather
    than one.
    """
    if len(examples) < 2:
        print("only one folder given, so no boxplot: a single box compares nothing")
        return None

    plt = pyplot()
    grouped = group_by_config(examples)
    colours = plt.get_cmap("tab10").colors

    data = []
    positions = []
    labels = []
    box_colours = []
    handles = []
    position = 0
    for index, (config, members) in enumerate(grouped):
        colour = colours[index % len(colours)]
        for example in members:
            data.append(example.final_fitnesses)
            positions.append(position)
            labels.append(example.name)
            box_colours.append(colour)
            position += 1
        position += GROUP_GAP
        handles.append((config, colour))

    figure, axes = plt.subplots(figsize=(max(6, len(data) * 1.4), 5))
    drawn = axes.boxplot(data, positions=positions, widths=0.6, patch_artist=True)

    for box, colour in zip(drawn["boxes"], box_colours):
        box.set_facecolor(colour)
        box.set_alpha(0.65)

    axes.set_xticks(positions)
    axes.set_xticklabels(labels, rotation=45, ha="right")
    axes.set_ylabel("best fitness at the end of the run")
    axes.set_title("Final fitness by example folder")
    axes.yaxis.grid(True, alpha=0.3)

    patches = []
    for config, colour in handles:
        patches.append(plt.Rectangle((0, 0), 1, 1, facecolor=colour, alpha=0.65))
    axes.legend(patches, [config for config, _ in handles], title="configuration", fontsize="small")

    path = output_path(examples, out_dir, "boxplot")
    figure.tight_layout()
    figure.savefig(path, dpi=140)
    plt.close(figure)
    return path


def average_curve(runs):
    """`(iterations, mean_best_fitness, contributing)` averaged across `runs`.

    Averaged **by iteration number and over whatever runs reached it**, not by
    row index and not over a common prefix. Three properties follow, and each
    matters for a real set of replicates:

    - Runs are matched on the evolver's own `iteration` value, so a run logged
      at a different cadence still lines up with the others.
    - A run that stopped early contributes to every iteration it reached and to
      none after, so one short outlier neither truncates the average to its own
      length nor drags it down past its end.
    - No value is interpolated or invented; an iteration nobody logged is not
      in the result at all.

    `contributing` is how many runs stood behind each point, which is what the
    caller marks on the plot so a thinly-supported tail is visible as one.
    """
    totals = {}
    counts = {}
    for run in runs:
        for iteration, value in zip(run.iterations, run.best_fitness):
            totals[iteration] = totals.get(iteration, 0.0) + value
            counts[iteration] = counts.get(iteration, 0) + 1

    iterations = sorted(totals)
    means = []
    contributing = []
    for iteration in iterations:
        means.append(totals[iteration] / counts[iteration])
        contributing.append(counts[iteration])
    return iterations, means, contributing


def first_thinning(iterations, contributing):
    """The first iteration backed by fewer runs than the fullest point, or None."""
    if not contributing:
        return None

    fullest = max(contributing)
    for iteration, count in zip(iterations, contributing):
        if count < fullest:
            return iteration
    return None


def draw_convergence(examples, out_dir):
    """Every replicate in grey, one coloured average per folder, one panel per config.

    Two decisions shape this plot, and both come from the same fact: folders
    produced by different configurations optimise different objectives.

    The average is **per folder**, never one line for the whole comparison —
    averaging two objectives together gives a number in no unit at all. With one
    folder given, that collapses to exactly one average over its replicates.

    Each configuration gets its **own panel**, rather than a shared pair of
    axes. Iteration counts differ by orders of magnitude between configurations
    — 150 against 20000 among the shipped examples — so a shared x-axis crushes
    the shorter run into the left edge and hides the convergence it was drawn to
    show. Panels keep every curve readable at its own scale, which is the whole
    point of plotting it.
    """
    plt = pyplot()
    colours = plt.get_cmap("tab10").colors
    grouped = group_by_config(examples)

    figure, panels = plt.subplots(
        1,
        len(grouped),
        figsize=(max(7.0, 6.0 * len(grouped)), 5.2),
        squeeze=False,
    )

    folder_index = 0
    for panel, (config, members) in zip(panels[0], grouped):
        grey_label = "individual runs"
        for example in members:
            for run in example.runs:
                panel.plot(
                    run.iterations,
                    run.best_fitness,
                    color="0.75",
                    linewidth=0.8,
                    zorder=1,
                    label=grey_label,
                )
                grey_label = None

        for example in members:
            iterations, means, contributing = average_curve(example.runs)
            colour = colours[folder_index % len(colours)]
            folder_index += 1
            panel.plot(
                iterations,
                means,
                color=colour,
                linewidth=2.2,
                zorder=3,
                label=f"{example.name}: average of {len(example.runs)}",
            )

            thinning = first_thinning(iterations, contributing)
            if thinning is not None:
                panel.axvline(
                    thinning,
                    color=colour,
                    linestyle=":",
                    linewidth=1.2,
                    zorder=2,
                    label=f"{example.name}: fewer runs beyond here",
                )

        panel.set_xlabel("iteration")
        panel.set_title(config, fontsize="medium")
        panel.grid(True, alpha=0.3)
        panel.legend(fontsize="small")

    panels[0][0].set_ylabel("best fitness")
    figure.suptitle("Convergence: every run, and the average per folder")

    path = output_path(examples, out_dir, "convergence")
    figure.tight_layout()
    figure.savefig(path, dpi=140)
    plt.close(figure)
    return path


def best_individual_path(run):
    """The winning network's edge file for one replicate, or None if absent."""
    candidate = os.path.join(run.directory, BEST_INDIVIDUAL)
    return candidate if os.path.isfile(candidate) else None


def infer_direction(example):
    """Whether a larger fitness is better in `example`, judged from its runs.

    GET does not tell Python which way an objective points — `RunResult` has no
    orientation, and a callable registered through `set_fitness_function` gets
    its direction at registration, where nothing downstream can see it. So the
    direction is read from the runs themselves: evolution improves, so whichever
    way the population's best fitness travelled from first iteration to last is
    the direction that counts as better.

    **One folder at a time, never pooled.** The objective is a property of the
    configuration, so two folders can point opposite ways — `epi_prof_match`
    minimizes an RMSE while `epi_spread` maximizes a node count — and the
    shipped examples do exactly that. Judging them together lets whichever
    objective has the larger numbers decide for the other, which silently draws
    the *worst* replicate of the folder that lost the vote.

    Every replicate of this one folder is counted, which is what makes it safe
    on a single unlucky run. Pass `--maximize` or `--minimize` to override it —
    an objective already near its optimum at iteration zero can drift the wrong
    way and fool this.
    """
    first = 0.0
    last = 0.0
    counted = 0
    for run in example.runs:
        first += run.best_fitness[0]
        last += run.best_fitness[-1]
        counted += 1

    if counted == 0:
        return True
    return last >= first


def networkx():
    """The `networkx` module, with the message this script owes a reader without it."""
    try:
        import networkx
    except ImportError:
        raise ValueError(
            "drawing the winning network needs networkx and scipy:\n"
            "    pip install networkx scipy\n"
            "For a picture with no installation at all, run the graph file through "
            "graph_to_png.py, which ships beside this script."
        ) from None
    return networkx


def read_edge_file(path):
    """`(num_nodes, edges)` from a GET edge file.

    Parsed by `graph_to_png.py`, which ships beside this script and already
    reads the format `save_results` writes — including the `# nodes = N` header,
    without which a node that has no edges is invisible and the count comes up
    short by exactly the nodes hardest to notice.
    """
    try:
        import graph_to_png
    except ImportError:
        raise ValueError(
            "graph_to_png.py is not beside this script, and it is what reads the "
            "edge-file format. Unpack the whole bundle rather than one file."
        ) from None
    return graph_to_png.read_graph(path)


def best_run_of(example, maximize):
    """The replicate with the best fitness that also wrote a network to draw."""
    best_run = None
    best_value = None
    for run in example.runs:
        if best_individual_path(run) is None:
            continue
        value = run.final_fitness
        if best_value is None or (value > best_value if maximize else value < best_value):
            best_value = value
            best_run = run
    return best_run, best_value


def network_positions(graph, nx):
    """Node positions: Kamada-Kawai over the connected part, strays on a ring.

    Isolated nodes are placed rather than solved for. Kamada-Kawai works from
    shortest-path distances, which are undefined between disconnected nodes, and
    it settles them in a tight knot at the **centre** — where a reader reads
    "central, therefore important" about the one kind of node that is connected
    to nothing. Putting them on a ring outside the graph says what they are.
    """
    core = []
    strays = []
    for node in graph.nodes:
        if graph.degree(node) > 0:
            core.append(node)
        else:
            strays.append(node)

    if len(core) > 2:
        positions = nx.kamada_kawai_layout(graph.subgraph(core))
    elif core:
        positions = nx.circular_layout(graph.subgraph(core))
    else:
        positions = {}

    for index, node in enumerate(strays):
        angle = 2 * math.pi * index / max(len(strays), 1)
        positions[node] = (
            STRAY_RADIUS * math.cos(angle),
            STRAY_RADIUS * math.sin(angle),
        )
    return positions, core, strays


def trimmed_ramp(name, plt):
    """`name`, restricted to `RAMP_SPAN` and rebuilt as a colormap of its own.

    Returned as a colormap rather than a list of colours so that the same ramp
    can colour a set of swatches and a continuous colourbar. Both paths are used
    below, and a reader comparing two graphs should not find the same
    multiplicity in two different colours.
    """
    from matplotlib.colors import LinearSegmentedColormap

    ramp = plt.get_cmap(name)
    low, high = RAMP_SPAN
    samples = []
    for index in range(RAMP_SAMPLES):
        samples.append(ramp(low + (high - low) * index / (RAMP_SAMPLES - 1)))
    return LinearSegmentedColormap.from_list(f"{name}-trimmed", samples)


def ramp_swatches(ramp, values):
    """One colour per value, evenly spaced along `ramp`: `{value: rgba}`.

    Spacing is by position in the sorted list rather than by the values
    themselves, which keeps two adjacent swatches apart whether the values run
    1, 2, 3 or 1, 2, 9.

    The point of returning the mapping, rather than letting matplotlib colour
    the edges from a cmap, is that the legend and the edges are then coloured
    from the same dictionary and cannot drift apart.
    """
    ordered = sorted(set(values))

    swatches = {}
    for index, value in enumerate(ordered):
        # A single value has no span to sit in, so it takes the middle of the
        # ramp rather than dividing by zero to reach the bottom of it.
        if len(ordered) == 1:
            position = 0.5
        else:
            position = index / (len(ordered) - 1)
        swatches[value] = ramp(position)
    return swatches


def add_integer_bar(figure, axes, drawn, label, values):
    """A colourbar beside `axes`, ticked at the integers `values` can hold.

    Both scales this draws are counts — a degree and a multiplicity are both
    numbers of edge copies — so a tick at 1.4 labels a value nothing in the
    graph can be. Only forced over a narrow span: over a wide one matplotlib
    already picks integers, and one tick per value would be an unreadable stack.
    """
    bar = figure.colorbar(drawn, ax=axes, shrink=0.6, label=label)
    span = range(min(values), max(values) + 1)
    if len(span) <= MAX_INTEGER_TICKS:
        bar.set_ticks(list(span))
    return bar


def draw_best_networks(examples, out_dir, directions):
    """Render each folder's best replicate, one PNG per `example_N`.

    `directions` maps a folder's name to whether larger is better in it. Per
    folder rather than one flag for the whole comparison: see `infer_direction`.

    Node colour is degree and edge colour is multiplicity, because those are the
    two things a picture can show that a fitness number cannot: which nodes
    became hubs, and where the run spent its parallel edges.

    Both codes are given a key, and the two get different ones because they are
    different kinds of number. Degree is open-ended and takes a colourbar.
    Multiplicity is a small count — `max_edge_multiplicity` is usually 1 to 5 —
    so up to `MAX_LEGEND_MULTIPLICITY` it takes a legend naming each value, and
    only a graph that goes past that falls back to a bar. Nothing about the
    edges is drawn at all when every edge is a single copy: under
    `max_edge_multiplicity = 1` a scale saying so is furniture.
    """
    plt = pyplot()
    nx = networkx()
    # One ramp for every graph drawn in this call, so the same multiplicity is
    # the same colour across a comparison rather than only within one picture.
    ramp = trimmed_ramp("plasma", plt)

    written = []
    for example in examples:
        best_run, best_value = best_run_of(example, directions[example.name])
        if best_run is None:
            print(f"{example.name}: no {BEST_INDIVIDUAL} in any replicate, nothing to draw")
            continue

        num_nodes, edges = read_edge_file(best_individual_path(best_run))
        graph = nx.Graph()
        graph.add_nodes_from(range(num_nodes))
        for start, end, weight in edges:
            if start != end:
                graph.add_edge(start, end, weight=weight)

        positions, core, strays = network_positions(graph, nx)
        weights = []
        for start, end in graph.edges:
            weights.append(graph[start][end]["weight"])
        present = sorted(set(weights))
        multigraph = max(weights, default=1) > 1
        # A legend can name a handful of values; past that the swatches stop
        # fitting and the scale goes back to being a bar.
        by_swatch = multigraph and max(weights) <= MAX_LEGEND_MULTIPLICITY

        figure, axes = plt.subplots(figsize=(8.5, 8.5))

        swatches = ramp_swatches(ramp, present) if by_swatch else {}
        if by_swatch:
            edge_colour = [swatches[weight] for weight in weights]
        elif multigraph:
            edge_colour = weights
        else:
            edge_colour = "0.55"

        drawn_edges = nx.draw_networkx_edges(
            graph,
            positions,
            ax=axes,
            width=1.0,
            alpha=0.75,
            edge_color=edge_colour,
            # Only when the edges are handed raw multiplicities to scale. Given
            # colours already, a cmap would be ignored anyway.
            edge_cmap=ramp if multigraph and not by_swatch else None,
        )
        drawn_nodes = None
        if core:
            drawn_nodes = nx.draw_networkx_nodes(
                graph,
                positions,
                ax=axes,
                nodelist=core,
                node_size=60,
                node_color=[graph.degree(node) for node in core],
                cmap="viridis",
                # The bright end of `viridis` is a yellow that all but vanishes
                # on white, and it lands on exactly the hubs worth seeing.
                edgecolors="0.35",
                linewidths=0.4,
            )
        if strays:
            nx.draw_networkx_nodes(
                graph,
                positions,
                ax=axes,
                nodelist=strays,
                node_size=45,
                node_color="0.75",
                edgecolors="0.45",
                linewidths=0.6,
            )

        # Degree is genuinely open-ended — a hub in a 100-node graph can reach
        # any degree at all — so it takes a bar. Without one the node colours are
        # a code with no key, which defeats the point of colouring by degree.
        if drawn_nodes is not None:
            degrees = [graph.degree(node) for node in core]
            add_integer_bar(figure, axes, drawn_nodes, "node degree", degrees)

        # A multiplicity past the legend's reach gets a bar of its own rather
        # than a sentence describing one. Two bars stack on the same side, which
        # is cheaper than asking a reader to hold "dark to light" in their head
        # while looking at a colour.
        if multigraph and not by_swatch:
            add_integer_bar(figure, axes, drawn_edges, "edge multiplicity", weights)

        # What the two bars cannot say. Isolated nodes need their own entry
        # because grey-with-a-ring is not a point on the degree scale — it is
        # what a node off the scale looks like.
        keys = []
        for weight in present if by_swatch else []:
            keys.append(
                plt.Line2D(
                    [], [], color=swatches[weight], linewidth=2.0,
                    label=f"{weight} parallel edge" + ("s" if weight > 1 else ""),
                )
            )
        if strays:
            keys.append(
                plt.Line2D(
                    [], [], linestyle="none", marker="o", markersize=6,
                    markerfacecolor="0.75", markeredgecolor="0.45",
                    label="isolated (degree 0), on the outer ring",
                )
            )
        if keys:
            axes.legend(handles=keys, loc="upper left", fontsize="small", framealpha=0.9)

        stray_note = f", {len(strays)} isolated on the outer ring" if strays else ""
        axes.set_title(
            f"{example.name}: {best_run.name}, fitness {best_value:g}\n"
            f"{num_nodes} nodes, {graph.number_of_edges()} edges{stray_note}",
            fontsize="medium",
        )
        axes.set_axis_off()

        destination = output_path([example], out_dir, "best_network")
        figure.tight_layout()
        figure.savefig(destination, dpi=140)
        plt.close(figure)

        written.append(destination)
        print(
            f"{example.name}: best is {best_run.name} at {best_value:g} "
            f"({num_nodes} nodes, {graph.number_of_edges()} edges{stray_note})"
        )

    return written


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
    direction = parser.add_mutually_exclusive_group()
    direction.add_argument(
        "--maximize",
        dest="maximize",
        action="store_true",
        default=None,
        help="treat a larger fitness as better, instead of judging from the runs",
    )
    direction.add_argument(
        "--minimize",
        dest="maximize",
        action="store_false",
        help="treat a smaller fitness as better, instead of judging from the runs",
    )
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

    # One direction per folder. The flag, when given, is deliberately global:
    # it is the answer for a comparison of folders sharing an objective, and on
    # a mixed set there is no single right value to pass — which is why the
    # inferred case is the one that has to be per folder.
    directions = {}
    for example in examples:
        if args.maximize is None:
            directions[example.name] = infer_direction(example)
        else:
            directions[example.name] = args.maximize

    if args.maximize is None:
        for example in examples:
            judged = "larger is better" if directions[example.name] else "smaller is better"
            print(f"{example.name}: direction judged from its runs: {judged}")
        print("override either way with --maximize/--minimize")

    try:
        written = [draw_boxplot(examples, args.out), draw_convergence(examples, args.out)]
        written.extend(draw_best_networks(examples, args.out, directions))
    except ValueError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    for path in written:
        if path:
            print(f"wrote {path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
