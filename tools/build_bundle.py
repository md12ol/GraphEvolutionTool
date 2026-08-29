"""Build the downloadable example bundle and the page that shows what is in it.

Two outputs, both checked into the repository:

    documentation/get-examples.zip        what a reader downloads
    documentation/guide/example-bundle.html   the same files, readable in a browser

Both are built from `get-examples/`, plus the handful of files named in
`IMPORTED` that the bundle ships from elsewhere in the repository.

Run it after changing anything under `get-examples/`, or any imported file:

    python3 tools/build_bundle.py

`--check` compares what is committed against `get-examples/` and exits non-zero
on any difference. That is what CI runs, and it is the only thing stopping the
download from drifting a version behind the page describing it.

The archive is checked by its *contents*, not its bytes: DEFLATE output is not
fixed by its input, so an archive built against zlib-ng and one built against
stock zlib hold identical files and differ byte for byte.
"""

import argparse
import hashlib
import html
import os
import sys
import zipfile

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BUNDLE = os.path.join(REPO, "get-examples")
ZIP_PATH = os.path.join(REPO, "documentation", "get-examples.zip")
PAGE_PATH = os.path.join(REPO, "documentation", "guide", "example-bundle.html")

# Fixed so the archive is byte-identical between rebuilds. zipfile otherwise
# stamps each entry with its file's mtime, which makes every checkout produce a
# different zip and defeats the --check comparison.
FIXED_TIMESTAMP = (2026, 1, 1, 0, 0, 0)

# Files the bundle ships that do not live under `get-examples/`, as
# {name in the bundle: path on disk}. `analyze_output.py` imports
# `graph_to_png.py` to draw the winning network, so a reader who only ever
# unpacks the zip needs a copy of it — while the repository keeps exactly one,
# in `tools/`. A second copy under `get-examples/` would ship just as well and
# then drift from the original.
IMPORTED = {"graph_to_png.py": os.path.join(REPO, "tools", "graph_to_png.py")}

LANGUAGE = {".py": "python", ".toml": "toml", ".csv": None}

# A base graph is 100+ lines of `0,1,1`. Enough to see the shape and the header,
# not so much that it buries the configs below it.
CSV_PREVIEW_LINES = 12


def is_shipped(relative):
    """Whether a path under `get-examples/` belongs in the download.

    `output/` is where a reader's own runs land, and this script is normally run
    by someone who has been running the examples — so everything under it is
    excluded except the `.gitkeep` that carries the empty directory into the
    archive. Without that, the published zip would ship whoever built it last.
    """
    if relative.startswith("output/"):
        return relative == "output/.gitkeep"
    return not (relative.startswith("__pycache__/") or relative.endswith(".pyc"))


def source_path(relative):
    """Where a bundle-relative path is read from on disk.

    Almost everything is under `get-examples/`; `IMPORTED` names the few that
    are not, and every reader of a bundle file goes through here so that those
    few need no special case of their own.
    """
    if relative in IMPORTED:
        return IMPORTED[relative]
    return os.path.join(BUNDLE, relative)


def bundle_files():
    """Every file the bundle ships, as paths relative to `get-examples/`.

    Sorted, so the archive's entry order and the page's section order do not
    depend on how the filesystem happens to enumerate a directory.
    """
    found = []
    for directory, _subdirs, names in os.walk(BUNDLE):
        for name in names:
            full = os.path.join(directory, name)
            relative = os.path.relpath(full, BUNDLE).replace(os.sep, "/")
            if is_shipped(relative):
                found.append(relative)
    for name in IMPORTED:
        if name not in found:
            found.append(name)
    found.sort()
    return found


def build_zip(files):
    """The archive's bytes, unpacking to a `get-examples/` directory."""
    import io

    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w", zipfile.ZIP_DEFLATED) as archive:
        for relative in files:
            info = zipfile.ZipInfo(f"get-examples/{relative}", FIXED_TIMESTAMP)
            # 0o644, and the high half marks it a regular file — without this
            # entries unpack with whatever mode the running umask implies.
            info.external_attr = (0o100644 & 0xFFFF) << 16
            info.compress_type = zipfile.ZIP_DEFLATED
            with open(source_path(relative), "rb") as handle:
                archive.writestr(info, handle.read())
    return buffer.getvalue()


def zip_differences(files):
    """How the committed archive differs from `get-examples/`, if it does.

    Compares the member list and each member's bytes rather than the archive's
    own bytes. DEFLATE output is not fixed by its input: zlib-ng and stock zlib
    compress identical files to different bytes, so two correct archives of the
    same directory disagree byte for byte and a hash comparison would fail on
    whichever machine did not build the committed one.
    """
    relative_zip = os.path.relpath(ZIP_PATH, REPO)
    try:
        archive = zipfile.ZipFile(ZIP_PATH)
    except FileNotFoundError:
        return [f"{relative_zip} is missing"]
    except zipfile.BadZipFile:
        return [f"{relative_zip} is not readable as a zip"]

    problems = []
    with archive:
        expected = [f"get-examples/{relative}" for relative in files]
        if archive.namelist() != expected:
            missing = sorted(set(expected) - set(archive.namelist()))
            extra = sorted(set(archive.namelist()) - set(expected))
            if missing:
                problems.append(f"{relative_zip} is missing {', '.join(missing)}")
            if extra:
                problems.append(f"{relative_zip} still holds {', '.join(extra)}")
            if not missing and not extra:
                problems.append(f"{relative_zip} lists its members in a different order")
            return problems

        for relative, name in zip(files, expected):
            with open(source_path(relative), "rb") as handle:
                if archive.read(name) != handle.read():
                    problems.append(f"{relative_zip} holds a stale {relative}")
    return problems


def section(relative):
    """One file's section on the page: a heading and its contents."""
    extension = os.path.splitext(relative)[1]
    language = LANGUAGE.get(extension)

    with open(source_path(relative), "r", encoding="utf-8") as handle:
        text = handle.read()

    note = ""
    if extension == ".csv":
        lines = text.splitlines()
        if len(lines) > CSV_PREVIEW_LINES:
            text = "\n".join(lines[:CSV_PREVIEW_LINES])
            note = (
                f'\n<div class="callout"><p>First {CSV_PREVIEW_LINES} lines of '
                f"{len(lines)}. Every remaining line is one more edge; download "
                f"the bundle for the whole file.</p></div>"
            )

    anchor = relative.replace(".", "-").replace("/", "-")
    opening = f'<pre><code class="language-{language}">' if language else "<pre><code>"
    return (
        f'<h2 id="{anchor}">{html.escape(relative)}</h2>\n'
        f"{opening}{html.escape(text)}</code></pre>{note}\n"
    )


def build_page(files):
    """The generated page's HTML.

    Empty files are in the archive but not on the page: `output/.gitkeep` exists
    to carry a directory, and has nothing to read.
    """
    readable = [f for f in files if os.path.getsize(source_path(f)) > 0]
    sections = "\n".join(section(relative) for relative in readable)
    return f"""<!doctype html>
<!-- GENERATED by tools/build_bundle.py from get-examples/. Do not edit by hand:
     CI rebuilds this file and fails if it differs from what is committed. Change
     the files under get-examples/ and re-run the script instead. -->
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>The Example Bundle · GET Docs</title>
<link rel="stylesheet" href="../assets/style.css">
<link rel="icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 32 32'><text y='26' font-size='26'>🧬</text></svg>">
</head>
<body data-page="guide/example-bundle.html">
<main>

<p class="page-kicker">Using GET</p>
<h1>The Example Bundle</h1>
<p class="lede">
  Every file in the download, readable here without unpacking it. These are the
  files themselves, not a description of them &mdash; the page is generated from
  the same directory the archive is built from.
</p>

<div class="tip">
  <b>Download it:</b> <a href="../get-examples.zip">get-examples.zip</a>. Unpack
  it, then follow the header in <code>python_from_config.py</code>. Run
  <code>01</code> through <code>04</code> as they are; <code>05</code> is the
  exercise and needs one block uncommented first.
</div>

{sections}
</main>
<script src="../assets/site.js"></script>
</body>
</html>
"""


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--check",
        action="store_true",
        help="compare against the committed outputs instead of writing them",
    )
    args = parser.parse_args()

    files = bundle_files()
    if not files:
        print(f"no files found under {BUNDLE}", file=sys.stderr)
        return 1

    page = build_page(files).encode("utf-8")

    if not args.check:
        archive = build_zip(files)
        with open(ZIP_PATH, "wb") as handle:
            handle.write(archive)
        with open(PAGE_PATH, "wb") as handle:
            handle.write(page)
        print(f"wrote {os.path.relpath(ZIP_PATH, REPO)} ({len(archive)} bytes)")
        print(f"wrote {os.path.relpath(PAGE_PATH, REPO)} from {len(files)} files")
        return 0

    stale = []
    stale.extend(zip_differences(files))

    try:
        with open(PAGE_PATH, "rb") as handle:
            committed_page = handle.read()
    except FileNotFoundError:
        stale.append(f"{os.path.relpath(PAGE_PATH, REPO)} is missing")
    else:
        if committed_page != page:
            stale.append(
                f"{os.path.relpath(PAGE_PATH, REPO)} is stale "
                f"(committed {hashlib.sha256(committed_page).hexdigest()[:12]}, "
                f"rebuilt {hashlib.sha256(page).hexdigest()[:12]})"
            )

    for problem in stale:
        print(f"  {problem}")
    if stale:
        print("run: python3 tools/build_bundle.py")
        return 1

    print(f"bundle is current: {len(files)} files, zip and page both match")
    return 0


if __name__ == "__main__":
    sys.exit(main())
