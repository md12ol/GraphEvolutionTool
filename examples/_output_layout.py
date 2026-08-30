"""Where a run's output files go — shared by the two Python route examples.

Every route GET ships lays its output out the same way, so results from a
Python run and a `get-run` run can be compared without translating a path:

    <root>/<YYYYmmdd-HHMMSS>-<seed>/            config.toml, shared by every replicate
    <root>/<YYYYmmdd-HHMMSS>-<seed>/run_<n>/    one directory per replicate, 1-based

The stamp is UTC, so directories from two machines sort into the order the runs
actually happened, and it is taken **once per invocation** — reading the clock
per replicate would scatter one run's output across two directories the moment
it crossed a second boundary.

This is a helper, not part of GET's API. It lives beside the examples because
both of them need it and neither is the right place to own it.
"""

import os
from datetime import datetime, timezone


def utc_stamp():
    """`YYYYmmdd-HHMMSS` in UTC. Take it once, then pass it around."""
    return datetime.now(timezone.utc).strftime("%Y%m%d-%H%M%S")


def experiment_output_dir(root, stamp, seed):
    """The folder one invocation's replicates all land inside, created if needed.

    Where anything belonging to the invocation rather than to a single
    replicate goes — the config document, which every replicate shares because
    they were all produced by it.
    """
    directory = os.path.join(root, f"{stamp}-{seed}")
    os.makedirs(directory, exist_ok=True)
    return directory


def run_output_dir(root, stamp, seed, run_index, n_runs):
    """The directory one replicate's files belong in, created if needed.

    `run_index` stays zero-based even though the folder name does not —
    `(seed, run_index)` is the pair that reproduces a replicate, and
    renumbering it would invite asking GET for the wrong one. The folder
    itself counts from one and is zero-padded to the width of `n_runs`, so ten
    replicates sort as `run_01`..`run_10` in a shell or a file browser, the
    same as `get-run` produces.
    """
    directory = experiment_output_dir(root, stamp, seed)
    if n_runs > 1:
        width = len(str(n_runs))
        directory = os.path.join(directory, f"run_{run_index + 1:0{width}d}")
    os.makedirs(directory, exist_ok=True)
    return directory
