"""Where a run's output files go — shared by the two Python route examples.

Every route GET ships lays its output out the same way, so results from a
Python run and a `get-run` run can be compared without translating a path:

    <root>/<YYYYmmdd-HHMMSS>-<seed>/            a single run's three files
    <root>/<YYYYmmdd-HHMMSS>-<seed>/run_<i>/    one directory per replicate

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


def run_output_dir(root, stamp, seed, run_index, n_runs):
    """The directory one replicate's files belong in, created if needed.

    `run_index` is zero-based, because `(seed, run_index)` is the pair that
    reproduces a replicate — numbering them from one here would invite someone
    to ask GET for the wrong one.
    """
    directory = os.path.join(root, f"{stamp}-{seed}")
    if n_runs > 1:
        directory = os.path.join(directory, f"run_{run_index}")
    os.makedirs(directory, exist_ok=True)
    return directory
