# History — a navigable static documentation site for GET, in `documentation/`

Append-only session log for this task, newest session first.
Maintained by `/save`; archived by `/done`.

---

## Session 2026-08-12: built the whole documentation site, merged as PR #64

The task started and finished in one session. `documentation/` is on `main` at `d420b3e`.

**What was built** — 38 pages plus the shell, all under `documentation/`:

| | |
|---|---|
| Shell | `assets/style.css`, `assets/site.js`, `_template.html`, `serve.sh` |
| Guide | 19 pages — 9 concept, 6 practical, 4 on extending |
| Reference | 15 pages — one per file in `get/src/`, plus the module map |
| Examples | 10 complete runnable experiments |
| Project | `index.html`, `status.html`, `design-notes.html`, `README.md`, `HANDOFF.md` |

**How it was produced.** Four subagents surveyed `get/src/` in parallel, one per subsystem, each
reading its files in full and returning public surface with `path:line`, data flow, worked examples,
invariants, a spec-versus-code table and extension points. Guide pages were then written from
`official_spec_sheet.md`; reference pages were written by the same agents that had surveyed those
files. Two agents were killed partway by a spend limit, so `reference/lib.html` and
`reference/edge-edit-operations.html` were finished by hand — no gap in the result, but those two
read slightly differently from their siblings. The surveys were scratchpad-only and are not
checked in.

**Validated.** A Python checker (now in `documentation/README.md`) confirms every internal link and
anchor resolves, every `data-page` matches both its path and a `NAV` entry, no file is truncated and
no page carries a stray `<style>`/`<script>`: clean at 38 pages / 38 nav entries. All pages served
over `python3 -m http.server`. Rendered in headless Firefox at 1440px and 700px, light and dark.

**Two defects the screenshots caught, both fixed.** The on-page contents were picking up the
landing page's `.card` headings, so they duplicated the sidebar — `buildToc` now skips headings
inside `.card`. And an overflowing code block drew the default light scrollbar in dark mode, the
brightest thing on the page — scrollbars are now themed.

**Not validated.** The prose has not been cross-read across all 38 pages in one pass, so two pages
may state the same thing slightly differently. Recorded in `documentation/HANDOFF.md` as the
highest-value remaining task.

**Five spec-sheet discrepancies found while surveying**, raised as `collab.md` #51 and touched
nowhere else, since the sheet changes only at a joint meeting. Two of them are stale *status*
claims where the code is ahead: row 23 still says `GraphEvolver::run` is `todo!()` (it is
implemented at `lib.rs:218-243`, with all four dispatch arms tested at `dispatch.rs:823`), and §9
still lists the `max_mutations` contract and the derived SDA alphabet as not yet true (both are —
`sda.rs:103-115`, `dispatch.rs:266`, `edge_edit.rs:243-250`). The third is a real gap in §7's
constraint list: `crossover_rate`, `mutation_rate` and `infection_rate` are unvalidated in the
design as well as the code. The last two are wording mismatches — §3.1's `Swap` description versus
the third `has_edge` check at `operations.rs:191-193`, which has no test coverage, and §3.2's
derived-alphabet invariant holding by convention rather than by type.

**Git manifest.** Branch `mdube_initial_doc_site`, 6 commits (shell · guide · reference ·
examples+project · README · deps+handoff), pushed and opened as PR #64. **James merged it
2026-08-12 17:06 UTC**; merge commit `d420b3e`. Remote branch auto-deleted by GitHub, local branch
deleted by hand. `collab.md` #50 and #51 went direct to `main` as `6091def`. Working tree clean, on
`main`, nothing unpushed.

*Session logged 2026-08-12 18:58 — Michael.*
