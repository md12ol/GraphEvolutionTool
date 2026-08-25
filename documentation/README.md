# `documentation/` — the GET documentation site

A static, self-contained website for the Graph Evolution Tool. No build step, no package manager,
and no network access at any point.

---

## Reading it

Open `index.html` in a browser. That is all — `file://` works fully, because nothing on the site
uses `fetch`, a CDN or a module import.

For a local server instead:

```bash
./serve.sh            # then open http://localhost:8000
./serve.sh 9000       # a different port
```

Ctrl-C stops it. `serve.sh` wraps `python3 -m http.server`, so it needs Python 3 and nothing else;
any static server works equally well. **On Windows**, run `python -m http.server 8000` from inside
`documentation/`, or just open `index.html`.

**JavaScript must be enabled.** The sidebar, the on-page contents and the previous/next links are
built at load time by `assets/site.js`. With scripts blocked, each page still renders its own
content readably, but you lose the navigation between them.

You do **not** need Rust, maturin, the `get` extension module, npm, or any documentation generator.
The site is hand-written HTML and never reads the codebase at runtime.

---

## What is on it

The site has four jobs and every page belongs to exactly one.

| Section | Pages | For |
|---|---|---|
| **Overview** | `index.html` | What GET does, and where to go next. |
| **How It Works** | `pipeline` · `variation` · `fitness` | The ideas, safe to read in order: the loop and the two representations, how children are made and who survives, then the number's whole life from the epidemic that produces it to the log it lands in. |
| **Using GET** | `route-python-objects` · `route-python-toml` · `route-rust-library` · `route-rust-cli` · `configuration` | One page per route, each written for someone using that route and nothing else, plus the per-key configuration reference they all point at. |
| **Extending GET** | `extending` · `new-fitness` · `new-genome` · `new-evolver` · `new-selection` · `new-scope` · `new-replacement` · `new-crossover` · `new-mutation` | For someone editing GET itself — one page per extension chain, and every chain has one. |
| **Design Notes** | `design-notes.html` | Why GET is shaped the way it is, and the non-goals. The only page whose subject is *why*. |

**Every page describes what the code does today.** A feature that is designed but not yet in
`get/src/` does not appear here at all — not in the present tense, not behind a badge. The roadmap
lives in the issue tracker, which is where someone can act on it. If you find a page describing
something the code does not do, that is a bug in the page.

---

## Editing it

### The three things that will trip you up

1. **`data-page` on `<body>` is load-bearing.** It must be the page's path relative to
   `documentation/` — `guide/fitness.html`, `index.html` — and must match its `NAV` entry byte for
   byte. It drives every sidebar link's depth calculation, the active-page highlight and the pager.
   Getting it wrong produces a page whose sidebar links all 404.
2. **The sidebar lives in one place.** `NAV` at the top of `assets/site.js` is the site map. A page
   missing from it still renders, but has no sidebar entry and no previous/next links.
3. **If your change does not appear, it is the browser cache.** `site.js` and `style.css` are
   aggressively cached, and a stale `site.js` looks exactly like a broken fix. Hard-reload with
   Ctrl-Shift-R (Cmd-Shift-R on macOS).

### Adding a page

1. Copy `_template.html`. It marks the four things to fill in and lists every CSS class the site
   defines.
2. Set `<title>`, the two asset paths (`../` per directory level), and `data-page`.
3. Add it to `NAV` in `assets/site.js`, in its group. Order in `NAV` is the order of the pager. A
   group with `title: null` renders as bare links with no collapsing header, which is what Overview
   and Design Notes use.
4. Cross-link it from wherever a reader would be when they need it. The site's usefulness is mostly
   in its links.

Headings become the contents panel: `site.js` collects `h2` and `h3` and builds the right-hand list
from them. A page with fewer than three headings gets none and widens instead, which is correct for
a short page.

### Conventions

| | |
|---|---|
| **Voice** | Second person, plain language. Both owners read every page and one of them does not write Rust. Prefer a worked example to a paragraph. |
| **Callouts** | `.note` (blue, a fact worth pulling out) · `.tip` (green, advice) · `.warn` (amber, this will bite you) · `.callout` (neutral, worth stopping on without claiming it is a warning) |
| **Tables** | Always inside `<div class="table-wrap">`, so a wide table scrolls rather than breaking the page |
| **Code** | `<pre><code class="language-rust">` — also `python`, `toml`, `text`, `bash`. `site.js` tints them; add `data-no-tint` to a block it mangles, usually an ASCII diagram |
| **`data-example`** | Put it on a Rust block that *illustrates* code the reader is about to write, rather than quoting `get/src`. It changes nothing visually — the signature checker reads it, and without it an invented `fn` is reported as a stale signature |
| **Diagrams** | Inline SVG only, using the `d-fill-*` / `d-stroke*` / `d-text*` classes so they take their colours from the palette. No diagram libraries |
| **Escaping** | `<` and `>` inside signatures must be `&lt;` `&gt;`, or the generics disappear |

### Where pages come from

- **From the code, not from a design document.** Read the module you are documenting and say what it
  does. Where a page and the source disagree, the source is right.
- **Route pages come from the runnable examples** — `examples/*.py` and `get/examples/*.rs` — because
  a page whose code is lifted from a program CI runs cannot drift far from working. Lift the snippet;
  do not retype it.
- **A `file.rs:NNN` reference must land on an `ADD A ... STEP n` marker** in `get/src`. That is what
  makes these citations checkable rather than hopeful, and the checker below enforces it.

---

## Checking it

```bash
python3 documentation/check_refs.py          # from the repository root
python3 documentation/check_refs.py --fix    # repair shifted line references
```

Four checks, each catching something the others cannot:

| | |
|---|---|
| **Source references** | every `file.rs:NNN` lands on a marker **of that page's own chain** |
| **Step tables** | each `new-*.html`'s table matches its chain's markers — catches a page that stops one step short |
| **Signatures** | every `fn` line shown on the site appears verbatim in `get/src` |
| **Structure** | `data-page`, `NAV` membership, internal links and anchors |

**CI runs it on every pull request**, as the `documentation references` step of the
`test, clippy and rustfmt` job, so a shifted reference now fails a check rather than waiting for
someone to notice. Run it locally anyway before you push — it needs no build and answers in under a
second, where the runner takes a minute to tell you the same thing.

**Run it after touching `get/src`, not only after touching a page.** Any insertion moves the line
references: adding one marker chain shifted 28 of them at once, and merging one upstream pull request
that added two lines to `dispatch.rs` shifted 18 more. `--fix` is what makes that a command rather
than an afternoon.

Silence means clean. Then open the site and click through: a narrow window, and every sidebar link.

### What is not checked

- **The defaults in the configuration reference.** About twenty of them, transcribed from
  `config.rs` and verified by hand. A Rust test cannot read HTML, so a default changing in the code
  would not fail anything.
- **Prose, for redundancy.** Every page has been checked against the code, but nobody has read all
  19 in one pass looking for two pages saying the same thing slightly differently. The four route
  pages overlap deliberately — each is meant to be read alone — so keep those in step by hand.
