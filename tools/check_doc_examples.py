#!/usr/bin/env python3
"""Run the code the documentation site prints, rather than reading it.

    python3 tools/check_doc_examples.py

`check_refs.py` verifies where a block points — the line it cites, the signature
it displays, the counts its comment claims. It says nothing about whether the
code in the block works. This does the other half: Python blocks are type-checked
against the installed package, and TOML blocks are fed to the real config loader.

Both halves found real defects the day they were first run by hand: two pages
claimed `save_results` wrote the config, and the two headline configurations used
an infection rate at which nothing evolves.

**Skipped rather than failed** when a prerequisite is missing, because this must
not turn a documentation edit into a red build on a machine with no toolchain:

- Python blocks need `mypy` and the built `get` module;
- TOML blocks need a `get-run` binary built with the `cli` feature;
- the config builder needs `node` and puppeteer, which pa11y installs.

Rust blocks are covered only through their `use get::…` lines, which are compiled
against the crate. The bodies are not: most are extension examples of types that do
not exist yet, which is what `data-example` marks. Bash is out of scope entirely —
a shell block would have to be executed to be checked, and most of them install
things or clone repositories.
"""

import glob
import html
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
os.chdir(REPO)

BLOCK = re.compile(
    r'<pre><code(?:\s+class="language-([a-z]+)")?( data-example)?>(.*?)</code></pre>', re.S)

# Generated from `get-examples/`, and `build_bundle.py --check` already proves
# they match the files themselves. Checking them here would be checking the
# generator twice.
GENERATED = "documentation/guide/example-bundle.html"

# A complete configuration needs these to be a document rather than a fragment.
# A block naming only `[fitness]` is spliced onto this before it is loaded.
BASE_CONFIG = """population_size = 12
network_size = 20

crossover_rate = 0.9
mutation_rate = 0.2

[evolution]
type = "generational"
num_generations = 1

[scope]
type = "global"

[selection]
type = "tournament"
tournament_size = 3

[genome]
type = "edge_edit"
gene_length = 8

[fitness]
type = "epi_spread"
infection_rate = 0.5
num_epidemics = 1
"""


def skip(message):
    """Report a check that did not run, and make it visible where it matters.

    A skip is a pass that checked nothing. Printing the word into a log nobody
    opens is how the mypy probe, the pa11y step and this file's own TOML half all
    went green while testing nothing — the last of those for as long as CI has
    had no `get-run` binary. Under Actions a skip becomes an annotation on the run.
    """
    print(message)
    if os.environ.get("GITHUB_ACTIONS"):
        print(f"::warning title=Documentation check skipped::{message}")
        summary = os.environ.get("GITHUB_STEP_SUMMARY")
        if summary:
            with open(summary, "a", encoding="utf-8") as out:
                out.write(f"- :warning: {message}\n")
    return 0


def blocks(language):
    """`(page, line, text)` for every block on the site in `language`."""
    out = []
    for page in sorted(glob.glob("documentation/**/*.html", recursive=True)):
        if page == GENERATED or "_template" in page:
            continue
        text = open(page, encoding="utf-8").read()
        for found in BLOCK.finditer(text):
            if (found.group(1) or "") != language or found.group(2):
                continue
            body = html.unescape(re.sub(r"<[^>]+>", "", found.group(3))).strip()
            out.append((page, text[:found.start()].count("\n") + 1, body))
    return out


def check_python():
    """Type-check each page's Python against the installed package.

    A page's blocks are concatenated in document order, because they are written
    as one narrative: a later block uses the `evolver` an earlier one built. A
    block that opens on a name the page never binds in Python is a fragment by
    design, so an undefined name is reported as a note rather than a failure —
    everything else is a real disagreement with the shipped stub.
    """
    if shutil.which("mypy") is None and subprocess.run(
            [sys.executable, "-m", "mypy", "--version"],
            capture_output=True).returncode != 0:
        return skip("python examples: skipped, mypy is not installed")

    # `--ignore-missing-imports` is needed for numpy, which the objective
    # examples use and which nothing here requires. The cost is that an absent
    # `get` would also become `Any`, and every call would type-check against
    # nothing at all — a green result proving less than no result. Probe for it.
    probe = "import get\nreveal_type(get.GraphEvolver)\n"
    with tempfile.TemporaryDirectory() as work:
        path = os.path.join(work, "probe.py")
        open(path, "w", encoding="utf-8").write(probe)
        revealed = subprocess.run(
            [sys.executable, "-m", "mypy", "--ignore-missing-imports",
             "--cache-dir", os.path.join(work, ".mypy_cache"), "--no-error-summary", path],
            capture_output=True, text=True).stdout
    if "Any" in revealed or "GraphEvolver" not in revealed:
        return skip("python examples: skipped, mypy cannot resolve `get` "
                    "(maturin develop, or pip install the wheel)")

    by_page = {}
    for page, line, body in blocks("python"):
        by_page.setdefault(page, []).append((line, body))
    if not by_page:
        print("python examples: none found")
        return 0

    bad = 0
    checked = 0
    with tempfile.TemporaryDirectory() as work:
        for page, found in sorted(by_page.items()):
            path = os.path.join(work, os.path.basename(page).replace(".html", ".py"))
            elided = set()
            written = 0
            with open(path, "w", encoding="utf-8") as handle:
                for line, body in found:
                    handle.write(f"# {page}:{line}\n")
                    written += 1
                    start = written + 1
                    handle.write(f"{body}\n\n")
                    written += body.count("\n") + 2
                    if re.search(r"^\s*\.\.\.,?\s*$", body, re.M):
                        elided.update(range(start, written + 1))
            checked += len(found)
            result = subprocess.run(
                [sys.executable, "-m", "mypy", "--ignore-missing-imports",
                 "--cache-dir", os.path.join(work, ".mypy_cache"), "--no-error-summary", path],
                capture_output=True, text=True)
            for problem in result.stdout.splitlines():
                # A fragment continuing an earlier block, or eliding a call's
                # arguments with `...`, is how the pages are written.
                if "[name-defined]" in problem or "EllipsisType" in problem:
                    continue
                at = re.match(r"[^:]+:(\d+):", problem)
                if (at and int(at.group(1)) in elided
                        and ("[call-arg]" in problem or "[arg-type]" in problem)):
                    continue
                bad += 1
                print(f"  {page}: {problem.split(':', 1)[-1].strip()}")
    print(f"{checked} python blocks, {bad} with type errors")
    return bad


def check_toml():
    """Load each TOML block through the real parser and validator.

    A fragment — a block that is one `[section]` — is spliced into a minimal
    document so the loader sees something it can accept or reject for the block's
    own reasons rather than for missing keys. A block naming a type the code does
    not implement is an extension example, and is counted rather than failed.
    """
    binary = os.path.join("target", "release", "get-run")
    if not os.path.exists(binary):
        binary = os.path.join("target", "debug", "get-run")
    if not os.path.exists(binary):
        return skip("toml examples: skipped, no get-run binary "
                    "(cargo build --release --bin get-run --features cli)")

    bad = illustrative = checked = 0
    with tempfile.TemporaryDirectory() as work:
        for page, line, body in blocks("toml"):
            if body.lstrip().startswith("# Cargo.toml"):
                continue                      # a dependency stanza, not a GET config
            # `type = "..."` at line start, and inside an inline table like
            # `mutation = { type = "my_mutation" }`, which is how the mutation
            # chain's example names the operator a reader is about to add.
            names = re.findall(r'type\s*=\s*"([a-z_]+)"', body)
            invented = [n for n in names if n not in KNOWN_VARIANTS]
            if invented:
                illustrative += 1
                continue
            document = body if "population_size" in body else splice(body)
            path = os.path.join(work, "config.toml")
            open(path, "w", encoding="utf-8").write(document)
            # `--out work`, not the bare tempdir: without it `get-run` still
            # makes its own `<timestamp>-<seed>/` folder, and it makes it in
            # `os.chdir(REPO)`'s cwd — the repository root — leaving one
            # untracked directory behind per block checked.
            result = subprocess.run([binary, path, "7", "--out", work],
                                    capture_output=True, text=True)
            checked += 1
            combined = result.stdout + result.stderr

            if result.returncode == 0:
                continue

            # A `run failed:` past this point means the document parsed and
            # validated — this function's actual job — and only then hit a
            # limit of the harness rather than of the block. Two are expected
            # and not a doc defect: `type = "python"` correctly refuses to run
            # without a registered callable, which the page itself explains;
            # and a spliced `base_graph` fragment names a file this harness
            # never had to copy alongside it.
            expected_runtime_stop = (
                "no fitness function has been registered" in combined
                or "could not read" in combined and "base_graph" in body)
            if "run failed:" in combined and expected_runtime_stop:
                continue

            bad += 1
            # `get-run` echoes the path it was given before a parser failure,
            # so the first line naming the failure is the error, not the echo.
            # Anything unrecognised — a crash, a missing library, exit 127
            # with no output at all — falls back to the last non-empty line,
            # so a green result never hides behind a message shape this
            # function does not know about.
            message = next((l for l in combined.splitlines()
                            if "failed to load config" in l or "could not parse" in l), None)
            if message is None:
                tail = [l for l in combined.splitlines() if l.strip()]
                message = tail[-1] if tail else f"exit code {result.returncode}, no output"
            print(f"  {page}:{line}: {message.strip()}")
    print(f"{checked} toml blocks loaded, {bad} rejected; "
          f"{illustrative} extension examples skipped")
    return bad


def splice(fragment):
    """`fragment` merged into `BASE_CONFIG`, replacing the section it defines."""
    section = re.match(r"\s*\[([a-z._]+)\]", fragment)
    if not section:
        return BASE_CONFIG + "\n" + fragment
    name = section.group(1)
    kept, dropping = [], False
    for line in BASE_CONFIG.splitlines():
        if line.startswith("["):
            dropping = line.strip() == f"[{name}]"
        if not dropping:
            kept.append(line)
    return "\n".join(kept).rstrip() + "\n\n" + fragment.strip() + "\n"


# Every `type = "..."` the code accepts. An example naming anything else is
# demonstrating an extension the reader is about to write.
KNOWN_VARIANTS = {
    "generational", "steady_state", "global", "random_subset", "best", "tournament",
    "two_point", "edge_edit", "sda", "epi_spread", "epi_length", "epi_prof_match",
    "struct_match", "python", "worst", "random", "reroll_gene", "redraw_one",
}


def check_rust_paths():
    """Compile every `use get::…` line the site prints.

    Not the block bodies: most are extension examples of a variant the reader is
    about to add, so they cannot compile by design. The `use` lines are different
    — each is a claim that a public path exists, and a module rename breaks every
    one of them silently, since nothing inside the crate imports its own paths the
    way an external caller does.
    """
    if shutil.which("cargo") is None:
        return skip("rust paths: skipped, cargo is not installed")

    lines = set()
    for _, _, body in blocks("rust"):
        for line in body.splitlines():
            line = line.strip()
            if line.startswith("use get::"):
                lines.add(line.rstrip(";") + ";")
    if not lines:
        print("rust paths: none found")
        return 0

    # A temporary example rather than a scratch crate: `get/examples/` is already
    # a compilation target with the dependency wired up, so this needs no manifest
    # of its own and reuses whatever the last build left behind.
    path = os.path.join("get", "examples", "_doc_use_check.rs")
    body = ("//! Generated by tools/check_doc_examples.py. Deleted before it returns.\n"
            "#![allow(unused_imports)]\n" + "\n".join(sorted(lines)) + "\n\nfn main() {}\n")
    open(path, "w", encoding="utf-8").write(body)
    try:
        result = subprocess.run(
            ["cargo", "build", "-q", "-p", "graph-evolution-tool", "--features", "cli",
             "--example", "_doc_use_check"],
            capture_output=True, text=True)
    finally:
        os.remove(path)

    bad = 0
    for problem in result.stderr.splitlines():
        # E0432 is the unresolved import; anything else here is the harness.
        if "E0432" in problem or "unresolved import" in problem:
            bad += 1
            print(f"  {problem.strip()}")
    print(f"{len(lines)} rust use-paths, {bad} unresolved")
    return bad


# A representative combination, not a matrix. The exhaustive version is deferred
# as `config-builder-output-untested`; this covers the path a reader most likely
# takes and, more importantly, proves the generator produces something the real
# loader accepts at all.
BUILDER_COMBINATIONS = [
    # `scope` and `selection` are asked on both routes and both start unset, so
    # a combination that omits either never leaves the gate — which is the
    # check working, not a defect, and is why they are spelled out here.
    "evolution=generational,scope=global,selection=best,"
    "genome=edge_edit,fitness=epi_spread",
    # The other route, and the two branches that reveal extra fields:
    # steady_state shows `replacement`, tournament shows `tournament_size`.
    "evolution=steady_state,replacement=worst,scope=random_subset,"
    "selection=tournament,genome=sda,fitness=epi_length",
]


def check_config_builder():
    """Drive the config builder in a browser and load what it generates.

    The builder is the one place on the site whose output is generated rather
    than written, so nothing that reads the page's own text can check it. The
    generator is inline script with no export, which leaves driving the real
    controls as the only honest option — and Chromium is already here for the
    accessibility step.
    """
    driver = os.path.join("tools", "config_builder_smoke.js")
    if shutil.which("node") is None:
        return skip("config builder: skipped, node is not installed")
    if not os.path.exists(os.path.join("node_modules", "puppeteer")):
        return skip("config builder: skipped, puppeteer is not installed "
                    "(npm install --no-save pa11y@8)")

    binary = os.path.join("target", "release", "get-run")
    if not os.path.exists(binary):
        binary = os.path.join("target", "debug", "get-run")
    if not os.path.exists(binary):
        return skip("config builder: skipped, no get-run binary")

    bad = 0
    with tempfile.TemporaryDirectory() as work:
        server = subprocess.Popen(
            [sys.executable, "-m", "http.server", "8137", "--directory", "documentation"],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        try:
            # The server is a subprocess, not a promise — give it a moment or the
            # first navigation races it and fails for a reason that is not ours.
            time.sleep(2)
            for combination in BUILDER_COMBINATIONS:
                generated = subprocess.run(
                    ["node", driver, "http://localhost:8137", combination],
                    capture_output=True, text=True)
                if generated.returncode != 0:
                    bad += 1
                    print(f"  {combination}: {generated.stderr.strip()}")
                    continue

                path = os.path.join(work, "config.toml")
                open(path, "w", encoding="utf-8").write(generated.stdout)
                loaded = subprocess.run([binary, path, "7", "--out", work],
                                        capture_output=True, text=True)
                if loaded.returncode == 0:
                    continue

                bad += 1
                combined = loaded.stdout + loaded.stderr
                message = next((l for l in combined.splitlines()
                                if "failed to load config" in l
                                or "could not parse" in l), None)
                if message is None:
                    tail = [l for l in combined.splitlines() if l.strip()]
                    message = tail[-1] if tail else f"exit code {loaded.returncode}"
                print(f"  {combination}: generated TOML rejected: {message.strip()}")
        finally:
            server.terminate()
            server.wait(timeout=10)

    print(f"{len(BUILDER_COMBINATIONS)} builder combinations, {bad} rejected")
    return bad


def main():
    failures = (check_python() + check_toml() + check_rust_paths()
                + check_config_builder())
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
