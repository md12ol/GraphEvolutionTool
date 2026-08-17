"""Check that the Python builder still agrees with `config.example.toml`.

The shipped example exists three times: `config.example.toml`, the
`the_shipped_example()` builder in `examples/config_builder.py`, and the
`example_mirror()` replica in `get/src/lib.rs`. The Rust replica is checked
against the TOML by a unit test; this script is what checks the Python one,
which needs the built extension module and so runs from CI's wheel job rather
than from `cargo test`.

Run it against an installed wheel:

    python tests/python/check_shipped_example.py

Exits 0 when the two agree, 1 naming the offending field when they do not.
"""

import contextlib
import importlib.util
import io
import sys
import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
EXAMPLE_TOML = REPO_ROOT / "config.example.toml"
CONFIG_BUILDER = REPO_ROOT / "examples" / "config_builder.py"

# Keys `to_toml()` renders that the shipped TOML leaves out, because Rust fills
# them from a default and the file documents them in a commented line instead.
# The set is closed and the values are the ones those comments claim: a new
# defaulted field, or a changed default, fails here until the commented block in
# `config.example.toml` is updated too. Nothing else checks that prose.
DOCUMENTED_DEFAULTS = {
    ("fitness", "min_epidemic_length"): 3,
    ("fitness", "max_epidemic_retries"): 5,
}


def load_config_builder():
    """Import `examples/config_builder.py` rather than copying it a fourth time."""
    spec = importlib.util.spec_from_file_location("config_builder", CONFIG_BUILDER)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def flatten(table, prefix=()):
    """Flatten a parsed TOML table into {(section, ..., key): value}."""
    flat = {}
    for key, value in table.items():
        path = prefix + (key,)
        if isinstance(value, dict):
            flat.update(flatten(value, path))
        else:
            flat[path] = value
    return flat


def compare(from_file, from_python):
    """Return a list of complaints, empty when the two agree."""
    problems = []

    for path, expected in from_file.items():
        name = ".".join(path)
        if path not in from_python:
            problems.append(f"{name}: set to {expected!r} in the TOML, absent from the builder")
        elif from_python[path] != expected:
            problems.append(
                f"{name}: {expected!r} in the TOML, {from_python[path]!r} from the builder"
            )

    for path, rendered in from_python.items():
        if path in from_file:
            continue
        name = ".".join(path)
        if path not in DOCUMENTED_DEFAULTS:
            problems.append(
                f"{name}: rendered as {rendered!r} by the builder, and the TOML never mentions it"
            )
        elif DOCUMENTED_DEFAULTS[path] != rendered:
            problems.append(
                f"{name}: rendered as {rendered!r}, but config.example.toml's commented line "
                f"documents the default as {DOCUMENTED_DEFAULTS[path]!r}"
            )

    return problems


def check_the_shipped_example(builder):
    config = builder.the_shipped_example()
    from_python = flatten(tomllib.loads(config.to_toml()))
    from_file = flatten(tomllib.loads(EXAMPLE_TOML.read_text()))

    problems = compare(from_file, from_python)
    if problems:
        print("the_shipped_example() and config.example.toml disagree:", file=sys.stderr)
        for problem in problems:
            print(f"  {problem}", file=sys.stderr)
        return False

    print(f"the_shipped_example() matches config.example.toml ({len(from_file)} keys)")
    return True


def check_every_builder_still_runs(builder):
    """Run the example script as shipped, so the other builders are exercised too.

    Its output is swallowed - this is a check, not a demonstration - but any
    exception from a builder propagates and fails the run.
    """
    try:
        with contextlib.redirect_stdout(io.StringIO()):
            builder.main()
    except Exception as err:
        print(f"examples/config_builder.py failed: {err!r}", file=sys.stderr)
        return False

    print("examples/config_builder.py runs to completion")
    return True


def main():
    builder = load_config_builder()
    checks = [
        check_the_shipped_example(builder),
        check_every_builder_still_runs(builder),
    ]
    return 0 if all(checks) else 1


if __name__ == "__main__":
    sys.exit(main())
