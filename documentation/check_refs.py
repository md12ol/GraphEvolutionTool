#!/usr/bin/env python3
"""Check — and with --fix, repair — the site's `path:line` references.

Every `file.rs:NNN` on the site points at the `ADD A ... STEP n` marker for
*that* step — the exact line where the reader makes the edit the page is
describing. Both halves are checked: the chain the page belongs to, and the
step it names. Chain alone is not enough, because one file can carry several of
a chain's markers and landing on a neighbour's sends the reader to the wrong
edit. Run from the repository root.

    python3 documentation/check_refs.py          # report
    python3 documentation/check_refs.py --fix    # snap each reference to its marker

Also checks that each new-*.html's step table matches its chain's markers, and
that every `fn` signature shown on the site appears verbatim in get/src. Each
catches something the others cannot: a page that stops one step short is not a
wrong line number, and a stale signature is neither.

Any edit to get/src moves these. Merging one upstream PR shifted 18 of them at
once, so --fix exists to make that a command rather than an afternoon.
"""
import glob
import html
import os
import re
import sys

MOD = {
    "documentation/guide/new-genome.html": "get/src/genomes.rs",
    "documentation/guide/new-evolver.html": "get/src/evolver.rs",
}
CHAIN = {
    "new-fitness": "OBJECTIVE", "new-genome": "GENOME", "new-evolver": "STRATEGY",
    "new-selection": "SELECTION", "new-scope": "SCOPE", "new-replacement": "REPLACEMENT",
    "new-crossover": "CROSSOVER", "new-mutation": "MUTATION",
}
REF = re.compile(r"([a-z_]+\.rs|config\.example\.toml):(\d+)|<code>:(\d+)</code>")
# A card is navigation, not a section. site.js keeps their headings out of the
# contents panel, so they never receive a generated id.
CARD = re.compile(r'<a[^>]*\bclass="[^"]*\bcard\b[^"]*"[^>]*>.*?</a>', re.S)
# A *declaration* marker: the name, an optional branch qualifier, then an em
# dash introducing what to do. A cross-reference — `search `ADD A ... STEP 4``
# — is wrapped in backticks and carries no dash, and must not count: a
# reference that snapped to one would point at prose about a different step.
# The qualifier's first word is the branch — `(for SteadyState, Python half)`
# is SteadyState — and where a chain forks it is a third thing to match on:
# py_config.rs carries MUTATION step 4 for EdgeEdit *and* for SDA.
MARKER = re.compile(
    r"ADD AN? ([A-Z]+) STEP (\d+)(?: \((?:for )?([^),]+)[^)]*\))? \u2014")
# The step a reference belongs to, from whatever names it last before the
# reference itself: a step table's first cell, a code block's `// step n —`, or
# prose saying `Step n is ...`. A letter suffix is part of the page's numbering,
# not the marker's — 3a and 3b are both STEP 3. `Steps 4 and 5` is deliberately
# not a cue: the code block under that heading carries its own.
STEP_CUE = re.compile(r"<td>(\d+)[a-z]?</td>|[Ss]tep (\d+)[a-z]?\b")
# The branch a reference belongs to, from the last <h2> above it: on a forked
# page each branch gets its own heading and its own table. A heading naming no
# branch leaves the reference unnarrowed rather than guessing.
HEADING = re.compile(r"<h2[^>]*>.*?</h2>", re.S)


def cued_step(text, before):
    """The step number a reference at `before` belongs to, or None."""
    found = None
    for cue in STEP_CUE.finditer(text, 0, before):
        found = cue.group(1) or cue.group(2)
    return int(found) if found else None


def sources():
    """config.example.toml and every .rs under get/src."""
    out = ["config.example.toml"]
    for root, _, files in os.walk("get/src"):
        out += [os.path.join(root, f) for f in files if f.endswith(".rs")]
    return out


def branch_names(chain):
    """The branch qualifiers this chain's markers carry, e.g. EdgeEdit and SDA."""
    out = set()
    for source in sources():
        for line in open(source, encoding="utf-8").read().splitlines():
            found = MARKER.search(line)
            if found and found.group(1) == chain and found.group(3):
                out.add(found.group(3))
    return out


def cued_branch(text, before, names):
    """The branch a reference at `before` belongs to, or None.

    The cue is the heading it sits under: on a forked page each branch has its
    own <h2> and its own table. Prose elsewhere names both branches freely, so
    only headings count, and a heading naming none of them — or more than one —
    yields None and leaves the reference unnarrowed.
    """
    def flat(text):
        return re.sub(r"[^a-z0-9]", "", re.sub(r"<[^>]+>", "", text).lower())

    heading = None
    for found in HEADING.finditer(text, 0, before):
        heading = found.group(0)
    if heading is None:
        return None
    hit = [name for name in names if flat(name) in flat(heading)]
    return hit[0] if len(hit) == 1 else None


def resolve(page, name):
    if name == "config.example.toml":
        return name
    if name == "mod.rs":
        return MOD[page]
    for root, _, files in os.walk("get/src"):
        if name in files:
            # Posix separators, so a reported path is the one the page cites.
            return os.path.join(root, name).replace(os.sep, "/")
    raise SystemExit(f"{page}: no source file named {name}")


def markers(path, chain, step=None, branch=None):
    """Line numbers in `path` carrying a marker for `chain`, 1-based.

    With `step` and `branch`, only that step's, on that side of a fork. One
    file can hold several markers even after both — `py_config.rs` carries
    REPLACEMENT step 3 twice — so this narrows the candidates rather than
    picking among them.
    """
    out = []
    for number, line in enumerate(open(path, encoding="utf-8").read().splitlines(), 1):
        found = MARKER.search(line)
        if found and (chain is None or found.group(1) == chain):
            if step is not None and int(found.group(2)) != step:
                continue
            if branch is not None and found.group(3) != branch:
                continue
            out.append(number)
    return out


def structure():
    """data-page, NAV membership, internal links and anchors, across every page."""
    # Posix separators: these are compared against `data-page` and NAV, which
    # use forward slashes on every platform.
    pages = [os.path.relpath(os.path.join(d, f), "documentation").replace(os.sep, "/")
             for d, _, fs in os.walk("documentation") for f in fs if f.endswith(".html")]
    nav = set(re.findall(r'\["([^"]+\.html)",',
                         open("documentation/assets/site.js", encoding="utf-8").read()))

    # Must match `slugify` in assets/site.js character for character: these are
    # the ids a browser will actually create, and an anchor is checked against
    # them. site.js reads `textContent`, so entities are decoded, not spelled
    # out — `&amp;` becomes `&` and is then stripped, never the word "and".
    def slug(text):
        text = html.unescape(re.sub(r"<[^>]+>", "", text))
        return re.sub(r"\s+", "-", re.sub(r"[^\w\s-]", "", text.lower()).strip())

    ids = {}
    for page in pages:
        body = open(os.path.join("documentation", page), encoding="utf-8").read()
        # site.js only assigns ids as a side effect of building the contents
        # panel, and that is skipped for card headings and for any page with
        # fewer than three of them. Synthesizing ids the browser never creates
        # passes a dead anchor: `index.html`'s six card headings are the case.
        headings = re.findall(r"<(h[23])[^>]*>(.*?)</\1>", CARD.sub("", body), re.S)
        synthesized = {slug(t) for _, t in headings} if len(headings) >= 3 else set()
        ids[page] = set(re.findall(r'\sid="([^"]+)"', body)) | synthesized

    bad = 0
    for entry in sorted(nav):
        if not os.path.exists(os.path.join("documentation", entry)):
            print(f"  NAV entry with no file: {entry}")
            bad += 1
    for page in sorted(pages):
        if page == "_template.html":
            continue
        body = open(os.path.join("documentation", page), encoding="utf-8").read()
        declared = re.search(r'data-page="([^"]+)"', body)
        if not declared or declared.group(1) != page:
            print(f"  bad data-page: {page}")
            bad += 1
        elif page not in nav:
            print(f"  not in NAV: {page}")
            bad += 1
        if not body.rstrip().endswith("</html>"):
            print(f"  truncated: {page}")
            bad += 1
        if "<style" in body:
            print(f"  stray <style>: {page}")
            bad += 1
        for href in re.findall(r'href="([^"]+)"', body):
            if href.startswith(("http", "data:", "mailto:")):
                continue
            target, _, fragment = href.partition("#")
            full = (os.path.normpath(os.path.join(os.path.dirname(page), target)).replace(os.sep, "/")
                    if target else page)
            if target and not os.path.exists(os.path.join("documentation", full)):
                print(f"  broken link: {page} -> {href}")
                bad += 1
            elif fragment and full in ids and fragment not in ids[full]:
                print(f"  broken anchor: {page} -> {href}")
                bad += 1
    print(f"{len(pages) - 1} pages, {len(nav)} nav entries, {bad} problems")
    return bad


def signatures():
    """Every `fn` line in a Rust block on the site must appear verbatim in get/src.

    Catches the one thing the reference and step-table checks cannot: a
    signature that is simply out of date. `Genome::mutate` lost its `context`
    argument on two pages this way and no amount of reading them found it.
    """
    source = ""
    for root, _, files in os.walk("get/src"):
        for name in files:
            if name.endswith(".rs"):
                source += "\n" + open(os.path.join(root, name), encoding="utf-8").read()

    def flat(text):
        text = text.replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
        text = re.sub(r"\s+", "", text)
        # A multi-line signature in the source carries a trailing comma before
        # its `)`; the one-line form on the page does not.
        return text.replace(",)", ")")

    flat_source = flat(source)
    checked = bad = 0
    for page in sorted(glob.glob("documentation/**/*.html", recursive=True)):
        if "_template" in page:
            continue
        # `data-example` marks a block that illustrates code the reader is
        # about to write, so its signatures are not claims about get/src. It is
        # declared in the page rather than guessed from the text: guessing by
        # looking for invented names skipped the whole `Genome` trait block,
        # which is where the one real staleness this check has found was.
        for block in re.finditer(
                r'<code class="language-rust"( data-example)?>(.*?)</code>',
                open(page, encoding="utf-8").read(), re.S):
            if block.group(1):
                continue
            body = html.unescape(re.sub(r"<[^>]+>", "", block.group(2)))
            for line in body.splitlines():
                line = line.strip()
                if not re.match(r"^(pub )?fn \w+", line):
                    continue
                sig = re.sub(r"\{.*$", "", line.rstrip(";").rstrip("{")).strip()
                if "..." in sig or not sig:
                    continue
                checked += 1
                if flat(sig) not in flat_source:
                    bad += 1
                    print(f"  {os.path.basename(page)}: not in get/src: {sig[:88]}")
    print(f"{checked} signatures, {bad} not found in get/src")
    return bad


# A `git grep` the pages tell a reader to run, and the count its comment claims.
# Only the `# N steps, M sites` shape is checked: a comment saying anything else
# is prose, and inventing a reading for it would be guessing.
GREP_LINE = re.compile(
    r'git grep (?P<flags>-[a-zA-Z]+) "(?P<pattern>[^"]*)" (?P<paths>[^#\n]*?)\s*'
    r'#[^#\n]*?(?P<steps>\d+) steps, (?P<sites>\d+) sites')


def grep_counts():
    """Every `git grep … # N steps, M sites` on a page must return M lines.

    The command is the checklist a reader actually runs, so a stale count sends
    them looking for sites that are not there — or, worse, tells them they have
    seen the whole chain when the grep printed more. Nothing else here checks a
    *command*: the reference and step-table checks read what a block cites, not
    what running it would print.

    The step count is checked too, and it is the one that catches a chain
    growing a site: steps and sites differ whenever one step lives in two
    places, which is normal, so only comparing both catches either moving.
    """
    import subprocess

    checked = bad = 0
    for page in sorted(glob.glob("documentation/guide/new-*.html")):
        text = open(page, encoding="utf-8").read()
        for found in GREP_LINE.finditer(text):
            checked += 1
            pattern = html.unescape(found.group("pattern"))
            paths = found.group("paths").split()
            flags = found.group("flags")
            argv = ["git", "grep", "-h"]
            if "E" in flags:
                argv.append("-E")
            argv += [pattern, "--"] + paths
            lines = subprocess.run(
                argv, capture_output=True, text=True).stdout.splitlines()
            sites = len(lines)
            # A step is (number, branch), not a number: MUTATION's eight are
            # four steps on each of two branches, and counting numbers alone
            # would call that four.
            steps = len({(m.group(2), m.group(3))
                         for m in (MARKER.search(l) for l in lines) if m})
            want_steps = int(found.group("steps"))
            want_sites = int(found.group("sites"))
            if (steps, sites) != (want_steps, want_sites):
                bad += 1
                print(f"  {page}: `{pattern}` says {want_steps} steps, {want_sites} sites "
                      f"but returns {steps} steps, {sites} sites")
    print(f"{checked} grep commands, {bad} miscounted")
    return bad


def step_tables():
    """Each new-*.html's step table must match its chain's markers in the source."""
    bad = 0
    for page, chain in sorted(CHAIN.items()):
        path = f"documentation/guide/{page}.html"
        rows = {n.rstrip("ab") for n, _ in re.findall(
            r"<tr><td>(\d+[ab]?)</td>(.*?)</tr>", open(path, encoding="utf-8").read(), re.S)}
        marks = set()
        for source in sources():
            for line in open(source, encoding="utf-8").read().splitlines():
                found = MARKER.search(line)
                if found and found.group(1) == chain:
                    marks.add(found.group(2))
        if rows != marks:
            bad += 1
            print(f"  {page}: page has steps {sorted(rows, key=int)}, "
                  f"source has {sorted(marks, key=int)}")
    print(f"{len(CHAIN)} step tables, {bad} mismatched")
    return bad


def main(fix):
    pages = []
    for root, _, files in os.walk("documentation"):
        # Posix separators, because these paths are also MOD's keys.
        pages += [os.path.join(root, f).replace(os.sep, "/") for f in files
                  if f.endswith(".html") and "_template" not in f]

    total = wrong = repaired = 0
    for page in sorted(pages):
        chain = CHAIN.get(os.path.basename(page)[:-5])
        names = branch_names(chain) if chain else set()
        # newline="" both here and on the write below, so --fix leaves the
        # file's existing line endings alone instead of rewriting every line.
        text = open(page, encoding="utf-8", newline="").read()
        out, last, cursor, touched = [], None, 0, False

        for ref in REF.finditer(text):
            out.append(text[cursor:ref.start()])
            cursor = ref.end()
            if ref.group(1):
                last, line = ref.group(1), int(ref.group(2))
                template = f"{last}:%d"
            elif last:
                line = int(ref.group(3))
                template = "<code>:%d</code>"
            else:
                out.append(ref.group(0))
                continue

            total += 1
            source = resolve(page, last)
            lines = open(source, encoding="utf-8").read().splitlines()
            here = lines[line - 1] if line <= len(lines) else ""
            found = MARKER.search(here)
            step = cued_step(text, ref.start())
            branch = cued_branch(text, ref.start(), names)
            # The step and the branch matter as much as the chain. Several
            # markers of one chain sit in one file, and a reference that lands
            # on the wrong one of them is a live failure rather than a near
            # miss: it sends the reader to a different edit than the one the
            # page is describing. py_config.rs carries four MUTATION step 4
            # markers, two per representation, so all three must agree.
            right_chain = found and (chain is None or found.group(1) == chain)
            right_step = found and (step is None or int(found.group(2)) == step)
            right_branch = found and (branch is None or found.group(3) == branch)

            if right_chain and right_step and right_branch:
                out.append(ref.group(0))
                continue

            wrong += 1
            # Snap to the nearest marker for this page's chain, step *and*
            # branch. Those narrow first, so a whole-file nearest match cannot
            # pull a reference onto its neighbour's marker — or, on a forked
            # chain, onto the same step of the other representation.
            # `chain` is None on any page outside CHAIN. Those pages may still
            # carry a reference, and this is the reporting path for a stale
            # one, so it has to render rather than raise.
            wanted = chain if chain is not None else "matching"
            if step is not None:
                wanted += f" step {step}"
            if branch is not None:
                wanted += f" (for {branch})"
            candidates = markers(source, chain, step, branch)
            if not candidates:
                # Usually the wrong file rather than the wrong line — the page
                # names a source that carries no such marker at all. Report and
                # leave it: a repair here could only snap to some other step's
                # or branch's marker, which is a write that fails the very
                # check that prompted it.
                print(f"  {page} -> {last}:{line}: no {wanted} marker in {source}")
                out.append(ref.group(0))
                continue
            best = min(candidates, key=lambda n: abs(n - line))
            print(f"  {page} -> {last}:{line} is not a {wanted} marker"
                  f"{f'; nearest is :{best}' if fix else ''}")
            out.append(template % best if fix else ref.group(0))
            repaired += fix
            touched = touched or fix

        out.append(text[cursor:])
        # Only pages that actually changed, so a repair run does not restamp
        # every file on the site.
        if touched:
            open(page, "w", encoding="utf-8", newline="").write("".join(out))

    print(f"{total} references, {wrong} wrong" + (f", {repaired} repaired" if fix else ""))
    bad_tables = step_tables()
    bad_sigs = signatures()
    bad_counts = grep_counts()
    bad_structure = structure()
    return 1 if (wrong and not fix) or bad_tables or bad_sigs or bad_counts or bad_structure else 0


if __name__ == "__main__":
    sys.exit(main("--fix" in sys.argv))
