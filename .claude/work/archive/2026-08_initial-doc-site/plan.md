# Plan — a navigable static documentation site for GET, in `documentation/`
_Started 2026-08-12 · last updated 2026-08-12_

## Objective

A self-contained static website in `/documentation/` that someone can get by cloning the repo,
serve locally with one command, and read to learn how GET works — both as a *user* (Python
interface, config, running an evolution) and as an *extender* (graph, genomes, evolvers, fitness).
Every module in `get/src/` gets a page; pages cross-link; concepts are explained in plain language
with worked examples and flowcharts. Plus a `documentation/README.md` telling a future session how
the site is built and how to extend it.

Per the user's instruction (2026-08-12): **anything designed in `official_spec_sheet.md` but not
yet in the code is documented as though it works**, with a small `planned` marker and a status page
so a reader can still tell what they can call today.

**Out of scope:** changing anything under `get/src/`, `Cargo.toml`, or the spec sheet. Generating
API docs with `cargo doc`. Hosting the site anywhere but the reader's own machine.

## Tasks

- [x] Survey the codebase with parallel subagents — one per subsystem (config/dispatch/py boundary ·
      genomes · evolvers · fitness/SIR/graph/lib), each returning structured notes: public surface,
      data flow, invariants, gotchas, and what the spec sheet designs but the code lacks.
      Notes land in the scratchpad, not the repo.
      **Verify by:** four note files exist and each names the real types in its files (spot-check
      against `get/src/`).
- [x] Build the site shell — `documentation/index.html`, shared `assets/style.css` and
      `assets/site.js`, sidebar nav, search-free but fully cross-linked, light/dark aware.
      **Verify by:** `python3 -m http.server` in `documentation/`, load `index.html`, click every
      sidebar link and get a page.
- [x] Concept pages — what GET is, the evolution loop end to end, the graph model, the two genomes,
      the mutation contract, fitness and orientation, config and validation.
      **Verify by:** each page's claims trace to a spec sheet § or a `path:line`.
- [x] Reference pages — 15, one per `get/src/` file plus a module map.
      **Verify by:** every `.rs` file has a page reachable from the sidebar — link/NAV check clean.
      Prose not cross-read against siblings; noted in `documentation/HANDOFF.md`.
- [x] Flowcharts — inline SVG (no external libs, CSP-safe, theme-aware) for: the evolution loop,
      generational vs steady-state, a mutation applying to a genome, the fitness pipeline, and the
      Python call path.
      **Verify by:** open each page in both light and dark; text legible, no clipped labels.
- [x] Examples — runnable TOML config walkthroughs and Python snippets, each annotated line by line.
      **Verify by:** config examples parse against `config.example.toml`'s key set.
- [x] `documentation/README.md` — how to serve it, how the pages are structured, the conventions
      (where the `planned` marker goes, how to add a page), and what is still thin.
      **Verify by:** a cold reader could add a new module page from it alone.
- [x] Status page listing every `planned` item and the spec § it comes from.
      **Verify by:** cross-check against the spec sheet's own status table.

Added 2026-08-12 during the session, all on explicit instruction:

- [x] Visual check — headless Firefox, light/dark, 1440px and 700px. Two fixes fell out: card
      headings polluting the on-page contents, and an unthemed scrollbar in dark mode.
- [x] Branch `mdube_initial_doc_site`, 6 commits, PR #64 — **merged by James 2026-08-12 17:06 UTC**
      as `d420b3e`; remote branch auto-deleted, local deleted by hand.
- [x] `collab.md` #50 (the site + the `planned`-tense question) and #51 (five spec-sheet items),
      pushed direct to `main` as `6091def`. Audits clean.
- [x] README dependency section, and `documentation/HANDOFF.md` — the checked-in task state, since
      `work/current/` is gitignored and never reaches James.
      **Verify by:** both on `main` under `documentation/`.

## Open questions
- **`collab.md` #50 is unanswered.** James merged PR #64 without ruling on whether unbuilt features
  should keep being documented in the present tense with `planned` badges. Cheap to change now, far
  more expensive once the pages have been edited. Not blocking; needs a nudge, not a decision here.
- `collab.md` #51 — **James replied 2026-08-12 14:12 and agreed all five**, with two calls of his
  own: on `Swap`, keep the code and fix §3.1's wording (his reading of the third `has_edge` check is
  an anti-clustering guard, offered as reasoning rather than confirmed); on the SDA alphabet,
  enforce the invariant in code rather than by convention. Still awaiting the joint meeting before
  the sheet itself changes. Nothing depends on it.
- `collab.md` #52 — two agenda items raised 2026-08-13 for the next meeting: direct-push for
  practice-binding `CLAUDE.md` edits, and having the agent chase unanswered `collab.md` items.

## Out of scope
- Publishing to claude.ai as an Artifact — the deliverable is the clonable folder. Can be added
  later on request.
- Any edit to `official_spec_sheet.md`; discrepancies found go to `collab.md` instead.
