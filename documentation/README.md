# `documentation/` — the GET documentation site

A static, self-contained website for the Graph Evolution Tool. No build step, no package manager,
and no network access at any point.

---

## Reading it

**The published site is <https://md12ol.github.io/GraphEvolutionTool/>**, deployed from `main` by
`.github/workflows/pages.yml`. That is the copy to send someone; everything below is for working on
the site rather than reading it.

From a clone, open `index.html` in a browser. That is all — `file://` works fully, because nothing
on the site uses `fetch`, a CDN or a module import.

For a local server instead — which is how you check a page before it is published:

```bash
./serve.sh            # then open http://localhost:8000
./serve.sh 9000       # a different port
```

Ctrl-C stops it. `serve.sh` wraps `python3 -m http.server`, so it needs Python 3 and nothing else;
any static server works equally well. **On Windows**, run `python -m http.server 8000` from inside
`documentation/`, or just open `index.html`.

**JavaScript must be enabled.** The sidebar, the on-page contents, the page filter and the
previous/next links are built at load time by `assets/site.js`. With scripts blocked, each page
still renders its own content readably, but you lose the navigation between them.

**Light is the default theme, dark an explicit choice** (`decisions.md`, 2026-08-30) — the toggle in
the header sets `data-theme` and remembers it; the un-toggled default follows the system otherwise.

You do **not** need Rust, maturin, the `get` extension module, npm, or any documentation generator
to *read* the site. `check_doc_examples.py` and the accessibility check below do need a build — see
"Checking it".

---

## What is on it

The site has six groups; every page belongs to exactly one. Restructured 2026-08-30 around what a
reader is trying to do rather than around the codebase — full reasoning in `decisions.md`.

| Section | Pages | For |
|---|---|---|
| **Overview** | `index.html` | What GET does, and where to go next. |
| **Start Here** | `getting-started` · `choosing-a-route` · `concepts` · `troubleshooting` | A reader who has never used GET: install, pick a route, learn the vocabulary, and what to do when something fails. |
| **How It Works** | `pipeline` · `variation` · `fitness` | The ideas, safe to read in order: the loop and the two representations, how children are made and who survives, then the number's whole life from the epidemic that produces it to the log it lands in. |
| **Use Python** | `route-python-objects` · `route-python-toml` · `example-bundle` · `config-builder` · `data-and-inputs` | Every way of driving GET from Python: typed objects, a TOML file, the downloadable example bundle, an interactive config builder, and the data model — node counts, edge files, identifier mapping — a reader needs before choosing GET for a network. |
| **Use Rust** | `route-rust-library` · `route-rust-cli` · `configuration` | The two Rust routes, source-only until crates.io publication, plus the per-key configuration reference every route points at. |
| **Extend GET** | `extending` · `contributing` · `new-fitness` · `new-genome` · `new-evolver` · `new-selection` · `new-scope` · `new-replacement` · `new-crossover` · `new-mutation` | For someone editing GET itself — a contributor setup page, then one page per extension chain. |
| **Design Notes** | `design-notes.html` | Why GET is shaped the way it is, and the non-goals. The only page whose subject is *why*. |

**Every page describes what the code does today.** A feature that is designed but not yet in
`get/src/` does not appear here at all — not in the present tense, not behind a badge, except the
`not-in-release` marker on the two Rust routes and most extension chains, which is the one
convention allowed to name unbuilt work (`collab.md` #118/#119). The roadmap otherwise lives in the
issue tracker, which is where someone can act on it. If you find a page describing something the
code does not do, that is a bug in the page.

---

## Editing it

### The three things that will trip you up

1. **`data-page` on `<body>` is load-bearing.** It must be the page's path relative to
   `documentation/` — `guide/fitness.html`, `index.html` — and must match its `NAV` entry byte for
   byte. It drives every sidebar link's depth calculation, the active-page highlight and the pager.
   Getting it wrong produces a page whose sidebar links all 404.
2. **The sidebar lives in one place.** `NAV` at the top of `assets/site.js` is the site map. A page
   missing from it still renders, but has no sidebar entry, no page-filter match, and no
   previous/next links.
3. **If your change does not appear, it is the browser cache.** `site.js` and `style.css` are
   aggressively cached, and a stale `site.js` looks exactly like a broken fix. Hard-reload with
   Ctrl-Shift-R (Cmd-Shift-R on macOS).

### Adding a page

1. Copy `_template.html`. It marks the four things to fill in and lists every CSS class the site
   defines.
2. Set `<title>`, the two asset paths (`../` per directory level), and `data-page`.
3. Add it to `NAV` in `assets/site.js`, in its group. Order in `NAV` is the order of the pager. Give
   it search keywords too — that third array element is what the sidebar filter matches against.
4. Cross-link it from wherever a reader would be when they need it. The site's usefulness is mostly
   in its links.

Headings become the contents panel: `site.js` collects `h2` and `h3` and builds the right-hand list
from them. A page with fewer than three headings gets none and widens instead, which is correct for
a short page. The same headings are what `check_refs.py`'s `heading_ids` check slugs — see below.

### Conventions

| | |
|---|---|
| **Voice** | Second person, plain language. Both owners read every page and one of them does not write Rust. Prefer a worked example to a paragraph. |
| **Callouts** | `.note` (blue, a fact worth pulling out) · `.tip` (green, advice) · `.warn` (amber, this will bite you) · `.callout` (neutral, worth stopping on without claiming it is a warning) |
| **Tables** | Always inside `<div class="table-wrap">`, so a wide table scrolls rather than breaking the page |
| **Code** | `<pre><code class="language-rust">` — also `python`, `toml`, `text`, `bash`. `site.js` tints them; add `data-no-tint` to a block it mangles, usually an ASCII diagram |
| **`data-example`** | Put it on a Rust block that *illustrates* code the reader is about to write, rather than quoting `get/src`. It changes nothing visually — the signature checker reads it, and without it an invented `fn` is reported as a stale signature |
| **Diagrams / figures** | Inline SVG only, using the `d-fill-*` / `d-stroke*` / `d-text*` classes so they take their colours from the palette. No diagram libraries. Every content SVG needs `role="img"` and an accessible name — `figure_labels` checks it |
| **Escaping** | `<` and `>` inside signatures must be `&lt;` `&gt;`, or the generics disappear |
| **Present tense only** | Say how GET works now. A rewrite carries the old wording out into `git log` and `decisions.md`, not into the page — see `decisions.md`, 2026-08-30, "the site says how GET works now, not how it used to" |

### Where pages come from

- **From the code, not from a design document.** Read the module you are documenting and say what it
  does. Where a page and the source disagree, the source is right.
- **Route pages come from the runnable examples** — `examples/*.py`, `get-examples/*` and
  `get/examples/*.rs` — because a page whose code is lifted from a program CI runs cannot drift far
  from working. Lift the snippet; do not retype it.
- **A `file.rs:NNN` reference must land on an `ADD A ... STEP n` marker** in `get/src`. That is what
  makes these citations checkable rather than hopeful, and the checker below enforces it.
- **`assets/convergence.svg` and `assets/evolved-network.svg` are generated**, by
  `tools/make_doc_figures.py` from real runs. Neither is hand-drawn; regenerate rather than editing
  either file directly.

---

## Checking it

```bash
python3 documentation/check_refs.py          # from anywhere — it locates the repo root itself
python3 documentation/check_refs.py --fix    # repair shifted line references
python3 tools/check_doc_examples.py          # run the Python, TOML and Rust the site prints
```

**`check_refs.py`** — ten checks, each catching something the others cannot:

| | |
|---|---|
| **Source references** | every `file.rs:NNN` lands on a marker **of that page's own chain** |
| **Step tables** | each `new-*.html`'s table matches its chain's markers — catches a page that stops one step short |
| **Signatures** | every `fn` line shown on the site appears verbatim in `get/src` |
| **Grep counts** | every `git grep … # N steps, M sites` a chain page tells a reader to run returns exactly that |
| **Documented defaults** | the configuration reference's Default column matches what `config.rs` actually implements |
| **Stated versions** | every version string on the site agrees with `pyproject.toml` |
| **Heading ids** | no page generates the same contents-panel anchor twice |
| **Figure labels** | every content `<svg>` carries `role="img"` and an accessible name |
| **External links** | every outward link still resolves — three retries, skipped rather than failed if the network is down entirely |
| **Structure** | `data-page`, `NAV` membership, internal links and anchors |

**`check_doc_examples.py`** runs the code rather than reading it, since none of the above does:
Python blocks are type-checked against the installed `get` module (`maturin develop --release` or
`pip install` it first — the check skips itself, rather than passing vacuously, if it cannot resolve
`get`), TOML blocks are fed to a built `get-run` binary (`cargo build --release --bin get-run
--features cli`), and every `use get::…` path in a Rust block is compiled against the crate. Bash
blocks are out of scope — checking one means running it, and most install or clone something.

**CI runs both** on every pull request, `check_refs.py` as `documentation references` and
`check_doc_examples.py` as `documentation examples run`, the latter after `cargo test` so the
binaries it needs already exist. **The `pages` workflow runs `check_refs.py` again before
deploying**, because a `workflow_dispatch` run can publish a commit no pull request ever gated. An
accessibility pass (`pa11y`, `continue-on-error` for now) runs alongside them on four representative
pages. Run the checks locally anyway before you push — `check_refs.py` needs no build and answers in
under a second, where the runner takes a minute to tell you the same thing.

**Run them after touching `get/src`, not only after touching a page.** Any insertion moves the line
references: adding one marker chain shifted 28 of them at once, and merging one upstream pull request
that added two lines to `dispatch.rs` shifted 18 more. `--fix` is what makes that a command rather
than an afternoon.

Silence means clean. Then open the site and click through: a narrow window, both themes, and every
sidebar link.

### What is still not checked

- **Prose, for redundancy.** Every page has been checked against the code, but nobody has read all
  27 in one pass looking for two pages saying the same thing slightly differently. The route pages
  overlap deliberately — each is meant to be read alone — so keep those in step by hand.
- **Bash blocks.** Checking one means executing it, and most install something or clone the repo.
