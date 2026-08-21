# `documentation/` — the GET documentation site

A static, self-contained documentation website for the Graph Evolution Tool. Clone the repo, run
one command, read it in a browser. No build step, no package manager, no network access at any
point.

Started 2026-08-12.

---

## Running it

```bash
git clone https://github.com/md12ol/GraphEvolutionTool.git
cd GraphEvolutionTool/documentation
./serve.sh            # then open http://localhost:8000
./serve.sh 9000       # a different port
```

Ctrl-C stops it.

### Dependencies: a browser, and Python 3 if you want a server

That is the whole list. **Nothing is installed, nothing is compiled, and nothing is fetched from
the network.** In particular you do **not** need Rust, maturin, the `get` extension module, npm,
or any documentation generator — the site is hand-written HTML and does not read the codebase at
runtime.

| To do this | You need |
|---|---|
| Read the site | Any modern browser. Open `documentation/index.html` directly — `file://` works fully, because nothing on the site uses `fetch`, a CDN, or a module import. |
| Serve it locally | Python 3.x, already present on macOS and every Linux distribution. `./serve.sh` is a wrapper around `python3 -m http.server`; any static server works equally well. |
| Edit it | A text editor. There is no build step — save the file and reload the page. |

JavaScript must be enabled: the sidebar, the on-page contents and the prev/next links are built at
load time by `assets/site.js`. With scripts blocked, each page still renders its own content
readably, but you lose the navigation between pages.

**On Windows**, `serve.sh` is a shell script; run `python -m http.server 8000` from inside
`documentation/` instead, or just open `index.html`.

**If you edit and the change does not appear**, it is the browser cache — `assets/site.js` and
`assets/style.css` are aggressively cached, and a stale `site.js` looks exactly like a broken fix.
Hard-reload (Ctrl-Shift-R, or Cmd-Shift-R).

---

## What is here

```
documentation/
├── index.html            the landing page
├── design-notes.html     the decisions behind the design, and the non-goals
├── _template.html        page skeleton — copy this to add a page
├── serve.sh
├── README.md             this file — how the site works
├── assets/
│   ├── style.css         the entire stylesheet. Tokens, layout, components
│   └── site.js           the entire behaviour. Nav, TOC, pager, theme, code blocks
└── guide/                every page except the landing page and design notes
```

The site has three jobs and a page belongs to exactly one. **How It Works** explains the ideas in
three pages, safe to read in order: `pipeline` is the loop plus the two things it moves between,
`variation` is how children are made and who survives, `fitness` is the number's whole life from
the epidemic that produces it to the log it lands in. **Using GET** is one page per route, each
written for someone using that route and nothing else — the Python pages do not discuss Rust beyond
noting that GET is implemented in it. **Extending GET** is for someone editing GET itself.

**There is no `reference/` directory.** It held one page per file in `get/src/` and was removed
2026-08-21: it documented eight functions that no longer existed and its `path:line` citations had
drifted by as much as 250 lines, because nothing anchored them. The crate's own doc comments are
the reference now.

**There is no `status.html` either**, removed 2026-08-21 with the same sweep. It indexed the
`planned` badges in one table, and every feature it named had shipped by the time it came out. The
badges went with it — see "There is no convention for unbuilt work" below.

---

## How the site works

Every page is a complete HTML document containing only `<main>`. Everything around it — the
sidebar, the on-page table of contents, the previous/next pager, the copy buttons on code blocks — is
built at load time by `assets/site.js`.

Three consequences worth internalising before you edit anything:

1. **The sidebar lives in one place.** `NAV` at the top of `assets/site.js` is the site map. A page
   that is not in `NAV` still renders, but has no sidebar entry and no prev/next links.
2. **`data-page` on `<body>` is load-bearing.** It must be the page's path relative to
   `documentation/` — `guide/sir.html`, `guide/route-rust-cli.html`, `index.html` — and it must match
   its `NAV` entry byte for byte. It drives the depth calculation for every sidebar link, the
   active-page highlight, and the pager. Getting it wrong produces a page whose sidebar links all
   404.
3. **Headings become the table of contents.** `site.js` collects `h2` and `h3`, gives them ids, and
   builds the right-hand TOC from them. A page with fewer than three headings gets no TOC and
   widens instead, which is correct for short pages.

### Adding a page

1. Copy `_template.html`. It has the four things to fill in marked, and lists every CSS class the
   site defines.
2. Set `<title>`, the two asset paths (`../` per directory level), and `data-page`.
3. Add the page to `NAV` in `assets/site.js`, in the group it belongs to. Order in `NAV` is the
   order of the pager. A group with `title: null` renders as bare links with no collapsing
   header — that is what `Overview` and `Design Notes` use, being one page each rather than
   sections.
4. Cross-link it from wherever a reader would be when they need it. The site's usefulness is
   mostly in its links.

### Conventions

| | |
|---|---|
| **Voice** | Second person, plain language. Both owners read every page and one of them does not write Rust. Prefer a worked example to a paragraph. |
| **Callouts** | `.note` (blue, a fact worth pulling out) · `.tip` (green, advice) · `.warn` (amber, this will bite you) · `.callout` (neutral, worth stopping on without claiming it is a warning) |
| **Tables** | Always inside `<div class="table-wrap">` so wide tables scroll rather than breaking the page |
| **Code** | `<pre><code class="language-rust">` — also `python`, `toml`, `text`, `bash`. `site.js` tints them. Add `data-no-tint` to a block the tinter mangles (ASCII diagrams, mostly) |
| **Diagrams** | Inline SVG only, using the `d-fill-*` / `d-stroke*` / `d-text*` classes so they take their colours from the palette. No diagram libraries |
| **Escaping** | `<` and `>` inside signatures must be `&lt;` `&gt;`, or the generics disappear |

### There is no convention for unbuilt work, and that is deliberate

**Every page describes what the code does today.** A feature that is designed but not yet in
`get/src/` does not appear on the site at all — not in the present tense, not behind a badge, not in
a callout. The roadmap lives in the issue tracker, which is where someone can act on it.

This replaced a `badge-planned` / `.plan-note` / `status.html` scheme that wrote unbuilt features up
as though they worked and indexed them in one table. Every feature it marked has since shipped, so
the set it described was empty before it was removed; the classes and the index page went with it
2026-08-21. If the design gets ahead of the code again, file an issue — do not write the page early.

If you find a page describing something the code does not do, that is a bug in the page. Fix it, or
file it in the per-owner queue — `mdube_edits.md` or `jsargant_edits.md`, whichever is yours.

---

## Current state

**All 18 pages in `NAV` exist and the site is complete as scoped.** The site was written
2026-08-12 at 38 pages; `reference/` came out on 2026-08-21 and How It Works was condensed from
nine pages to three the same day.

| Area | Pages |
|---|---|
| Shell | `assets/style.css`, `assets/site.js`, `_template.html`, `serve.sh` |
| Landing | `index.html` |
| Guide — how it works | `pipeline` (the loop, the graph, both genomes) · `variation` (the three rolls, selection, both strategies) · `fitness` (orientation, SIR, seeding, logs and results) |
| Guide — using | `route-python-objects`, `route-python-toml`, `route-rust-library`, `route-rust-cli` — one per route |
| Guide — extending | `extending`, `new-fitness`, `new-genome`, `new-evolver`, `new-selection`, `new-scope`, `new-replacement`, `new-crossover`, `new-mutation` — one per extension chain, and every chain has one |
| Project | `design-notes.html` |

Verified 2026-08-21 with the script below: every `data-page` matches both its path and a `NAV`
entry, every `NAV` entry has a file, and **zero broken internal links and zero broken anchors**
across all 18 pages.

### Known thin spots

Honest gaps rather than bugs, for whoever extends this:

- **Anchors are verified mechanically; prose is not.** Each page was checked against the code
  page by page, but nobody has read all 16 in one pass looking for places where two of them say
  the same thing slightly differently. The nine-into-three merge removed the worst of that within
  How It Works; the boundary between a guide page and the route pages is less settled.
- **The four route pages overlap on purpose.** Each is meant to be read alone, so the config
  document and the seeding rule appear on more than one. Keep them in step.
- **The Python config classes' mutability is stated conservatively.** Four of them are pyo3 complex
  enums whose variant fields carry no explicit accessor annotation; the pages say to treat those as
  read-only and rebuild the variant, rather than guessing. Worth verifying against pyo3 0.27 and
  then stating plainly.
- **No search.** At 18 pages the sidebar is enough. If it grows past about 60, a build-free
  client-side index is the natural next step.

---

## How to work on it

Three rules, and they are the ones that keep independently-written pages looking like one site:

- **Pages come from the code, not from a design document.** Read the module you are documenting and
  say what it does. If a page and the source disagree, the source is right and the page is a bug.
- **Route pages come from the runnable examples** — `examples/*.py` and `get/examples/*.rs` —
  because a page whose code is lifted from a program CI runs cannot drift far from working. Lift the
  snippet; do not retype it.
- **Give any new page the template, the CSS class list and the `data-page` rule** before it is
  written. Those three are what the checker below enforces, and they are cheap to get right and
  tedious to retrofit.

### Verification

There is no test suite. This script is what was run before declaring the site done, and it catches
every mechanical failure the site can have — run it from `documentation/`:

```bash
python3 - <<'EOF'
import os, re
pages = [os.path.relpath(os.path.join(d, f), '.')
         for d, _, fs in os.walk('.') for f in fs if f.endswith('.html')]
nav = set(re.findall(r'\["([^"]+\.html)",', open('assets/site.js').read()))

def slug(s):
    s = re.sub(r'<[^>]+>', '', s).replace('&amp;', 'and')
    return re.sub(r'\s+', '-', re.sub(r'[^\w\s-]', '', s.lower()).strip())

ids = {}
for p in pages:
    h = open(p).read()
    ids[p] = set(re.findall(r'\sid="([^"]+)"', h)) | {
        slug(t) for _, t in re.findall(r'<(h[23])[^>]*>(.*?)</\1>', h, re.S)}

for n in sorted(nav):
    if not os.path.exists(n): print("NAV entry with no file:", n)

for p in sorted(pages):
    if p == '_template.html': continue
    h = open(p).read()
    m = re.search(r'data-page="([^"]+)"', h)
    if not m or m.group(1) != p:  print("bad data-page:", p)
    elif p not in nav:            print("not in NAV:", p)
    if not h.rstrip().endswith('</html>'): print("truncated:", p)
    if '<style' in h:             print("stray <style>:", p)
    for href in re.findall(r'href="([^"]+)"', h):
        if href.startswith(('http', 'data:', 'mailto:')): continue
        tgt, _, frag = href.partition('#')
        full = os.path.normpath(os.path.join(os.path.dirname(p), tgt)) if tgt else p
        if tgt and not os.path.exists(full):     print("broken link:", p, "->", href)
        elif frag and full in ids and frag not in ids[full]:
            print("broken anchor:", p, "->", href)
print("checked", len(pages), "pages against", len(nav), "nav entries")
EOF
```

### And the source references

Every `file.rs:NNN` on the site points at an `ADD A ... STEP n` marker in `get/src` — that is the
whole convention, and it is what makes them checkable rather than hopeful. Run this from the
repository root:

```bash
python3 - <<'EOF'
import re, os, glob
MOD = {'documentation/guide/new-genome.html':  'get/src/genomes/mod.rs',
       'documentation/guide/new-evolver.html': 'get/src/evolver/mod.rs'}
def resolve(page, fn):
    if fn == 'config.example.toml': return fn
    if fn == 'mod.rs': return MOD[page]
    for root, _, fs in os.walk('get/src'):
        if fn in fs: return os.path.join(root, fn)

bad = total = 0
for page in glob.glob('documentation/**/*.html', recursive=True):
    if '_template' in page: continue
    last = None
    for m in re.finditer(r'([a-z_]+\.rs|config\.example\.toml):(\d+)|<code>:(\d+)</code>',
                         open(page).read()):
        if m.group(1): fn, line, last = m.group(1), int(m.group(2)), m.group(1)
        elif last:     fn, line = last, int(m.group(3))
        else:          continue
        total += 1
        lines = open(resolve(page, fn)).read().splitlines()
        text = lines[line - 1] if line <= len(lines) else '<past EOF>'
        if 'ADD A' not in text:
            bad += 1
            print(f"{page} -> {fn}:{line} is not a marker: {text.strip()[:60]}")
print(f"{total} references, {bad} not on a marker")
EOF
```

### And the step counts

Each `new-*.html` page walks one marker chain, and its step table must match that chain's markers in
the source — same count, same numbers. This catches a page that stops one step short, which both the
crossover and strategy pages did (each was missing the `config.example.toml` step):

```bash
python3 - <<'EOF'
import re, glob, os, subprocess
CHAIN = {'new-fitness': 'OBJECTIVE', 'new-genome': 'GENOME', 'new-evolver': 'STRATEGY',
         'new-selection': 'SELECTION', 'new-scope': 'SCOPE', 'new-replacement': 'REPLACEMENT',
         'new-crossover': 'CROSSOVER', 'new-mutation': 'MUTATION'}
for page, chain in sorted(CHAIN.items()):
    path = f'documentation/guide/{page}.html'
    rows = {n.rstrip('ab') for n, _ in
            re.findall(r'<tr><td>(\d+[ab]?)</td>(.*?)</tr>', open(path).read(), re.S)}
    out = subprocess.run(['git', 'grep', '-ohE', f'ADD AN? {chain} STEP [0-9]+',
                          '--', 'get/src', 'config.example.toml'],
                         capture_output=True, text=True).stdout
    marks = {m.split()[-1] for m in out.splitlines()}
    print(f"{page:16} page={sorted(rows, key=int)}  source={sorted(marks, key=int)}"
          f"{'' if rows == marks else '   <-- MISMATCH'}")
EOF
```

Every step-numbered reference is also checked against the marker it names — that the marker belongs
to *this* page's chain, and that its number matches the row. A shifted line can easily land on a
marker from a different chain, which a bare "is it a marker" check happily accepts.

**A bare `:NNN` continuation inherits the filename from the reference before it**, which is how the
step tables are written and why the checker tracks `last`. Any edit to `get/src` can shift these —
adding the objective chain's six markers moved 28 of them at once — so run it after touching the
source, not only after touching a page.

Silence means clean. Then open it and click through: a narrow window, and every sidebar link.
