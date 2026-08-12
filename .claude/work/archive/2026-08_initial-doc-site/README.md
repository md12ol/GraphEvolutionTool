# initial-doc-site — a navigable static documentation site for GET

**Dates:** 2026-08-12 (single session, closed 2026-08-13).

## Objective

A self-contained static website in `/documentation/` that someone can get by cloning the repo,
serve locally with one command, and read to learn how GET works — both as a *user* (Python
interface, config, running an evolution) and as an *extender* (graph, genomes, evolvers, fitness).

## Outcome

Shipped and merged. `documentation/` is on `main` at `d420b3e` — 38 cross-linked pages: 19 guide
pages, 15 reference pages (one per file in `get/src/`, plus a module map), 10 worked examples, and
a status page and design notes. No build step and no external assets: the dependencies are a
browser, plus Python 3 only if you want a server.

Opened as PR #64 in six commits and merged by James on 2026-08-12 17:06 UTC. Verified before
merge: every internal link and anchor resolves, every `data-page` matches both its path and its
`NAV` entry, and the site renders correctly in headless Firefox at 1440px and 700px in both
themes. The verification script lives in `documentation/README.md`.

**The one judgement call**, and the thing to know before editing the site: features that
`official_spec_sheet.md` designs but the code does not yet have are documented **in the present
tense, as though they work**, each carrying a `planned` badge and a callout naming what happens
today, with `status.html` indexing all of them. Reasoning in `decisions.md` 2026-08-12 18:52.

## Left behind, deliberately

- **`documentation/HANDOFF.md`** is the checked-in continuation record, written because
  `.claude/work/current/` is gitignored and so a task's own state never reaches the other owner. It
  carries what was built, the seven things worth doing next, and the gotchas. Read it before
  extending the site — not this file.
- **Three parked issues carried forward**, none root-caused: the unvalidated probabilities
  (`crossover_rate`, `mutation_rate`, `infection_rate`), the `cargo doc` intra-doc warning in
  `sda.rs`, and `config.example.toml`'s epidemic parameters giving the search nothing to climb.
- **No hotfixes.** Nothing temporary was added to the tree by this task.
- **Three `collab.md` items open.** #50 asks whether the present-tense convention should stand —
  James merged PR #64 without ruling on it, so it stands unopposed rather than agreed. #51 raises
  five spec-sheet discrepancies found while surveying `get/src/`; **James agreed all five on
  2026-08-12** and they await the joint meeting. #52 carries two agenda items for that meeting.
- **One rule added to `CLAUDE.md`:** when a `planned` feature ships, its badge and its
  `status.html` row come out in the same PR. Contingent on #50.

## Not done

The prose was never cross-read across all 38 pages in one pass, so two pages may state the same
thing slightly differently. That is the highest-value remaining task and is recorded as such in
`documentation/HANDOFF.md`.
