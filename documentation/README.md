# `documentation/` — the GET documentation site

A static, self-contained documentation website for the Graph Evolution Tool. Clone the repo, run
one command, read it in a browser. No build step, no package manager, no network access at any
point.

Started 2026-08-12.

---

## Running it

```bash
cd documentation
./serve.sh            # http://localhost:8000
./serve.sh 9000       # a different port
```

`serve.sh` is a two-line wrapper around `python3 -m http.server`. Any static server works, and
`file://` works too — opening `documentation/index.html` directly in a browser is fully functional,
because nothing on the site uses `fetch`, a CDN, or a module import. The server is only nicer
because history and relative links behave the way you expect.

---

## What is here

```
documentation/
├── index.html            the landing page
├── status.html           what is built vs designed-but-not-yet-built
├── design-notes.html     the decisions behind the design, and the non-goals
├── _template.html        page skeleton — copy this to add a page
├── serve.sh
├── README.md             this file
├── assets/
│   ├── style.css         the entire stylesheet. Tokens, layout, components
│   └── site.js           the entire behaviour. Nav, TOC, pager, theme, code blocks
├── guide/                concept and how-to pages, written for a reader
└── reference/            one page per file in get/src/, written for someone in the code
```

The split is deliberate. **`guide/` explains ideas** and is safe to read in order.
**`reference/` documents code** — signatures, `path:line`, invariants — and is where you land from
a search. Each reference page links back to its concept page rather than repeating the
introduction.

---

## How the site works

Every page is a complete HTML document containing only `<main>`. Everything around it — the
sidebar, the on-page table of contents, the previous/next pager, the copy buttons on code blocks,
the light/dark toggle — is built at load time by `assets/site.js`.

Three consequences worth internalising before you edit anything:

1. **The sidebar lives in one place.** `NAV` at the top of `assets/site.js` is the site map. A page
   that is not in `NAV` still renders, but has no sidebar entry and no prev/next links.
2. **`data-page` on `<body>` is load-bearing.** It must be the page's path relative to
   `documentation/` — `guide/sir.html`, `reference/config.html`, `index.html` — and it must match
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
   order of the pager.
4. Cross-link it from wherever a reader would be when they need it. The site's usefulness is
   mostly in its links.

### Conventions

| | |
|---|---|
| **Voice** | Second person, plain language. Both owners read every page and one of them does not write Rust. Prefer a worked example to a paragraph. |
| **Callouts** | `.note` (blue, a fact worth pulling out) · `.tip` (green, advice) · `.warn` (amber, this will bite you) · `.plan-note` (violet, what exists today under a planned feature) |
| **Badges** | `.badge-built` · `.badge-partial` · `.badge-planned`, used inline in headings and table rows |
| **API items** | `.api-item` > `.api-sig` (the signature) + `.api-body` (prose, with a `.src` span carrying `path:line`) |
| **Tables** | Always inside `<div class="table-wrap">` so wide tables scroll rather than breaking the page |
| **Code** | `<pre><code class="language-rust">` — also `python`, `toml`, `text`, `bash`. `site.js` tints them. Add `data-no-tint` to a block the tinter mangles (ASCII diagrams, mostly) |
| **Diagrams** | Inline SVG only, using the `d-fill-*` / `d-stroke*` / `d-text*` classes so they follow the theme. No diagram libraries |
| **Escaping** | `<` and `>` inside signatures must be `&lt;` `&gt;`, or the generics disappear |

### The `planned` convention

The site documents GET **as `official_spec_sheet.md` designs it**, which is the repository's own
rule: where the sheet and the code disagree, the sheet is the intent. So features that are designed
and agreed but not yet in `get/src/` are written **in the present tense, as though they work**, and
carry two markers:

- a `<span class="badge badge-planned">planned</span>` badge where they appear, and
- a `.plan-note` callout saying plainly what exists today and what to do instead.

`status.html` lists every one of them in a single table. This was an explicit instruction from the
repo owner on 2026-08-12; the badge and the status page exist so that "written as though
implemented" never becomes "indistinguishable from implemented".

The **opposite** case also occurs: several places where the spec sheet's status claims are stale
and the code is *ahead* of the sheet. Those are documented from the code, with no badge. They are
listed at the bottom of this file, because they want a `collab.md` item from an owner — an agent
does not edit the sheet.

---

## Current state

Written 2026-08-12. **All 38 pages in `NAV` exist and the site is complete as scoped.**

| Area | Pages |
|---|---|
| Shell | `assets/style.css`, `assets/site.js`, `_template.html`, `serve.sh` |
| Landing | `index.html` |
| Guide — concepts | `pipeline`, `graph`, `genomes`, `variation`, `fitness`, `sir`, `evolvers`, `reproducibility`, `output` |
| Guide — using | `getting-started`, `glossary`, `configuration`, `python-api`, `performance`, `troubleshooting` |
| Guide — extending | `extending`, `new-fitness`, `new-genome`, `new-evolver` |
| Reference | `index` (module map), `lib`, `graph`, `genome-trait`, `sda`, `edge-edit`, `edge-edit-operations`, `evolver-common`, `generational`, `steady-state`, `fitness`, `sir`, `config`, `py-config`, `dispatch` |
| Examples | `examples/index.html` — ten complete runnable experiments |
| Project | `status.html`, `design-notes.html` |

Verified 2026-08-12: every `data-page` matches both its path and a `NAV` entry, every `NAV` entry
has a file, **zero broken internal links and zero broken anchors** across all 38 pages, no page
carries a stray `<style>` or `<script>` block, and every page serves over
`python3 -m http.server`.

### Known thin spots

Honest gaps rather than bugs, for whoever extends this:

- **No screenshots or rendered output.** Everything is described in prose and SVG. A page showing
  an actual convergence plot and an actual evolved network would help more than another paragraph
  anywhere.
- **Anchors were verified, prose was not cross-read.** Each page is internally accurate against the
  code or the spec sheet, but nobody has read all 38 in one pass looking for places where two pages
  say the same thing slightly differently.
- **`reference/` has no page for `evolver/mod.rs` or `genomes/mod.rs`.** They are covered inside
  `evolver-common.html` and `genome-trait.html` respectively, which is noted on the module map.
- **The Python config classes' mutability is stated conservatively.** Four of them are pyo3 complex
  enums whose variant fields carry no explicit accessor annotation; the pages say to treat those as
  read-only and rebuild the variant, rather than guessing. Worth verifying against pyo3 0.27 and
  then stating plainly.
- **No search.** With 38 pages the sidebar is still enough. If it grows past about 60, a
  build-free client-side index is the natural next step.

---

## How this was produced, and how to continue it

The source material is:

1. **`official_spec_sheet.md`** at the repository root — the authority on the design, and the
   source for every guide page. Where it and the code disagree, it wins.
2. **Four subagent surveys of `get/src/`**, one per subsystem, each reading its files in full and
   producing a structured note: public surface with `path:line`, data flow, worked examples,
   invariants, a spec-vs-code table, and extension points.

The surveys lived in the session scratchpad and are **not** checked in — they were working
material, and their content is now in the reference pages. If you are continuing this work, the
efficient shape is the same one that produced it:

- Guide pages come from the spec sheet, which one agent can hold in context.
- Reference pages come from reading the actual module, and parallelise well one-agent-per-subsystem.
- Give agents the template, the CSS class list, the `data-page` rule, and the `planned` convention
  — those four things are what keep independently-written pages looking like one site.

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

Silence means clean. Then open it and click through: both themes, a narrow window, and every
sidebar link.

---

## For the owners: three things the sheet should hear about

Found while surveying the code, and **not acted on** — `official_spec_sheet.md` is changed only at
a joint meeting, and an agent that finds the sheet wrong writes a `collab.md` item rather than
fixing the sheet. Raising these is an owner's call:

1. **Status table row 23 is stale.** It says the Python interface is built "except dispatch" and
   that `GraphEvolver::run`'s body is still `todo!()`. `run` is fully implemented and all four
   strategy × genome dispatch arms are tested end to end. The sheet's own note says a stale status
   row "is the whole signal", so this one matters more than most.
2. **§9's closing paragraph is stale.** It lists the one-mutation contract with `max_mutations` and
   the cap-derived SDA alphabet as "decided here but not yet true of the code". Both are now true,
   and the paragraph contradicts the sheet's own status table.
3. **Three probabilities are unvalidated, and §7 does not ask for them to be.**
   `crossover_rate`, `mutation_rate` and `infection_rate` accept negative values and values above
   1. This is a gap in the design as much as in the code, which is why it is a sheet question
   rather than an issue.

Two smaller ones, for whoever is next in the relevant file: §3.1's description of `Swap` says "none
of the three would-be edges already exists", but the code checks three pairs while creating only
two edges — the third check is an extra rejection whose purpose is not documented anywhere and
which has no test coverage. And the SDA derived-alphabet invariant holds by convention rather than
by type: the constructor still accepts an arbitrary `num_chars`, so a hand-assembled population via
the Rust route can silently reintroduce the clamping bias §3.2 exists to prevent.
