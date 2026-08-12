# Handoff — the documentation site

**Read this first if you are picking this work up in a new session.** `README.md` beside it is the
reference for *how the site works*; this file is what happened, why, and what to do next.

Written 2026-08-12 — Michael.

---

## Status in one line

**The site is complete as scoped and open as PR #64.** All 38 pages in `NAV` exist, every link and
anchor resolves, and it renders correctly in light and dark and at narrow widths. Nothing is
blocked, nothing is half-written.

| | |
|---|---|
| Branch | `mdube_initial_doc_site` |
| PR | **#64** — <https://github.com/md12ol/GraphEvolutionTool/pull/64> |
| Commits | 5, one per stage — shell · guide · reference · examples+project · README |
| Touches code? | **No.** Nothing under `get/src/`, `Cargo.toml`, `config.example.toml`, or `official_spec_sheet.md` |
| Related `collab.md` items | **#50** (announces the site, asks James one question) · **#51** (five spec-sheet items) |

James merges Michael's PRs — nobody merges their own — so the next event is his review, not
another push.

---

## What was asked for

Michael, 2026-08-12: a navigable documentation website in a `documentation/` folder, launchable
from a clone on a local machine, clean and modern, useful to both a *user* and someone *extending*
the package, covering all of the code, with examples and flowcharts in simple language — and a
README so a future session can continue it.

One instruction shaped the whole site and is worth restating because it is unusual:

> **"Anything not implemented yet, write as if it were implemented."**

That is followed throughout. It is also the site's biggest risk, so it carries two guards — see
[The one judgement call](#the-one-judgement-call) below.

## How it was built

Four subagents surveyed `get/src/` in parallel, one per subsystem, each reading its files in full
and producing a structured note: public surface with `path:line`, data flow, worked examples,
invariants, a spec-versus-code table, and extension points. Then:

- **Guide pages** were written from `official_spec_sheet.md`, which one session can hold in
  context and which is the authority on the design.
- **Reference pages** were written by the same agents that had surveyed those files, so the
  signatures and line numbers came from something that had actually read them.

The surveys lived in the session scratchpad and are **not** checked in — they were working
material, and their content is now in the pages. If you resume this work, the same split is the
efficient one: guide from the sheet, reference from the code, one agent per subsystem.

Two of the four agents were terminated partway by a spend limit; `reference/lib.html` and
`reference/edge-edit-operations.html` were finished by hand afterwards. That left no gap in the
result, but it is why those two pages read slightly differently from their siblings.

---

## The one judgement call

Features that `official_spec_sheet.md` designs but the code does not yet have are written **in the
present tense, as though they work**. Each one carries:

- a `<span class="badge badge-planned">planned</span>` badge where it appears, and
- a `.plan-note` callout stating plainly what happens today and what to do instead.

`status.html` indexes every one in a single table. The list:

| Planned | Where it shows up |
|---|---|
| Replicate runs (`runs=`, the per-run seed stream) | `guide/python-api.html`, `guide/reproducibility.html`, `examples/` |
| `max_cores` and the per-call thread pool | `guide/performance.html` |
| A result object replacing `best_fitness()` | `guide/python-api.html` |
| The convergence log reaching Python | `guide/output.html` |
| `ci_95`, and the per-row seed and run index | `guide/output.html` |
| `save_logs` / `save_results` | `guide/output.html`, `reference/lib.html` |
| `set_base_graph` and its three checks | `guide/python-api.html#base-graph`, `examples/` |

**This is the thing to settle before the site is edited further.** `collab.md` #50 asks James
whether he would rather it described only what exists. Changing it is one pass now — find every
`badge-planned` and rewrite the surrounding section — and a much larger job once other people have
edited pages. If he says yes, `status.html` becomes the only page that mentions unbuilt features.

The **opposite** case also exists and is easy to trip over: several places where the spec sheet's
status claims are stale and the **code is ahead**. Those are documented from the code, with **no**
badge, and `status.html` records them at the bottom. If you find yourself about to badge something,
check which way the disagreement runs first.

---

## Where everything is

```
documentation/
├── index.html            landing page
├── status.html           built vs designed-not-built — the index of every `planned` badge
├── design-notes.html     why the design is as it is; the non-goals
├── HANDOFF.md            this file
├── README.md             how the site works, conventions, the verification script
├── _template.html        copy this to add a page
├── serve.sh
├── assets/
│   ├── style.css         the whole stylesheet — tokens, layout, components
│   └── site.js           the whole behaviour — NAV table, sidebar, TOC, pager, theme
├── guide/                19 pages: concepts, then practical, then extending
├── reference/            15 pages: one per file in get/src/, plus the module map
└── examples/             10 runnable experiments
```

The three things to internalise before editing are in `README.md` under **How the site works**: the
`NAV` table is the site map, `data-page` must match both the file's path and its `NAV` entry, and
`h2`/`h3` headings become the on-page contents.

---

## If you continue: what is actually worth doing

Roughly in order of value. None of it is required — the site is finished as scoped.

1. **Settle the `planned` question** (`collab.md` #50). Everything else is cheaper afterwards.
2. **Cross-read the prose in one pass.** Each page is internally accurate, but nobody has read all
   38 consecutively looking for two pages that say the same thing slightly differently. This is the
   single highest-value remaining task and it needs a human or one long session, not parallel
   agents.
3. **Add rendered output.** There are no screenshots — no actual convergence plot, no picture of an
   evolved network. One real figure would help more than another paragraph anywhere. Blocked in
   practice on the convergence log reaching Python.
4. **Verify the pyo3 config-class mutability.** Four of the Python config classes are pyo3 complex
   enums whose variant fields carry no explicit accessor annotation. The pages currently say to
   treat them as read-only and rebuild the variant, which is conservative rather than known. Check
   against pyo3 0.27 and state it plainly. Affects `guide/python-api.html` and
   `reference/py-config.html`.
5. **Update the site when any `planned` item ships.** Search for `badge-planned`, drop the badge
   and the `.plan-note`, and remove its row from `status.html`. Doing it in the same PR as the
   feature is the only way this stays true.
6. **Pages for `evolver/mod.rs` and `genomes/mod.rs`**, if anyone wants them. They are currently
   covered inside `evolver-common.html` and `genome-trait.html`, which the module map says.
7. **Search**, if the site outgrows the sidebar — past roughly 60 pages. A build-free client-side
   index is the natural approach; do not add a build step for it.

---

## Gotchas that cost time here

- **Browser cache.** `assets/site.js` and `assets/style.css` are cached hard. A fix to `site.js`
  can look completely broken because the browser is running the old one. Hard-reload, or use a
  fresh browser profile if you are screenshotting headlessly. This wasted a cycle during the
  build.
- **`data-page` is silently load-bearing.** Get it wrong and the page renders fine but every
  sidebar link on it 404s, because the depth prefix is computed from it. The verification script in
  `README.md` catches this; run it before every push.
- **A page missing from `NAV` still renders** — it just has no sidebar entry and no prev/next. So
  "it looks fine" is not evidence it is wired up.
- **`site.js` skips headings inside `.card`** when building the contents, deliberately: card
  headings are navigation, not sections, and without the skip the landing page's contents duplicate
  the sidebar. If you add another heading-bearing component, it probably needs the same treatment.
- **Escape `<` and `>` in signatures** as `&lt;` `&gt;`, or generics vanish silently into the DOM.
- **Wide tables need `<div class="table-wrap">`**, or the page scrolls horizontally as a whole.

---

## Verification

`README.md` carries a self-contained Python script that checks every link, anchor and `data-page`,
flags truncated files and stray `<style>` blocks, and confirms `NAV` and the filesystem agree. Run
it from `documentation/` before any push. Silence means clean — it was silent when this was
written.

Then look at it: both themes, a narrow window, and click through the sidebar.

---

## Project bookkeeping

The task's own working docs are in `.claude/work/current/` — `plan.md` has the task list, and it
is gitignored, so it exists only on the machine it was written on. This file is the checked-in
version of that state and is the one that reaches the other owner.

Per `.claude/CLAUDE.md`: `official_spec_sheet.md` changes only at a joint meeting, and an agent
that finds the sheet wrong writes a `collab.md` item rather than fixing the sheet. That is why
`collab.md` #51 exists and why nothing in the sheet was touched, despite two of its status claims
being demonstrably stale.
