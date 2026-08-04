# Issues — work not yet in the tracker

**17 issues are open in the tracker** at `md12ol/GraphEvolutionTool` — that is the source of
truth for anything filed. `gh issue list` is the way to read them; they are deliberately **not**
mirrored here, because a second copy drifts and this file would become a private fork of the
tracker.

This file holds only what is **not filed yet**. Two tiers; the difference is whether it has been
root-caused. An entry leaves the moment it is filed — replaced by nothing, since the tracker then
carries it.

Maintained by `/save`. `/done` lists anything still `Filed: not yet` before archiving a task.
How issues get filed — tool, confirmation rule, target project — lives in `CLAUDE.md`.

*Reset 2026-07-31 23:35 — Michael: the one filed entry was removed once #22 existed, per the sync
obligation in `CLAUDE.md`.*

*Synced 2026-08-04 18:55 — Michael: "Align sir_sim's length and profile" removed once it was filed
as **#34** (assigned to md12ol), same obligation. Open count refreshed from the tracker.*

---

## Parked — noticed, not investigated

### <what was noticed>
- **Where:** `path` or component, as far as it's known.
- **Impact:** why it matters — who or what it breaks.
- **Noticed:** <YYYY-MM-DD>, in <what you were doing when you hit it>


## Ready to file — root-caused and evidenced

### <title — imperative, issue-ready>
- **For:** teammate / team / unassigned
- **Project:** the tracker project it belongs to
- **Filed:** not yet
- **Component:** `path:line`
- **Body:** <open with a sentence on this line — a bare label is identical in every entry, which
  is what union merge folds together. See CLAUDE.md, "Formatting for union merge".>
  What's wrong, the mechanism with `path:line`, evidence, how to reproduce, candidate fixes.

### Point generational.rs's mutation doc at common::mutate_child before #25 is built
- **For (generational-mutation-doc):** whoever takes #25; found by Michael reviewing PR #30
- **Project (generational-mutation-doc):** `md12ol/GraphEvolutionTool`
- **Filed (generational-mutation-doc):** not yet — trivial, and best folded into #25 rather than
  filed as a standalone chore
- **Component (generational-mutation-doc):** `get/src/evolver/generational.rs:24`
- **Body (generational-mutation-doc):** the doc comment on `GenerationalEvolver` still says the
  evolver mutates "children by `mutation_rate`", with no mention of `max_mutations` or the shared
  helper that now owns both rolls.
  No bug today — `advance_generation` is still `todo!()`. But #25's implementer reads that doc, and
  re-rolling the dice inline is **precisely the drift PR #30 existed to eliminate**: spec §4 requires
  both rolls to live in `common::mutate_child` so the two strategies cannot disagree. Steady-state
  already routes through it (`steady_state.rs:59`). One line of doc, pointing at
  `crate::evolver::common::mutate_child`, closes the gap before it opens.

### Give IndexGenome a separate mutation counter instead of overloading its index
- **For (index-genome-counter):** unassigned; found by Michael reviewing PR #30
- **Project (index-genome-counter):** `md12ol/GraphEvolutionTool`
- **Filed (index-genome-counter):** not yet — test-only, no user impact
- **Component (index-genome-counter):** `get/src/evolver/common.rs`, `IndexGenome` in `mod tests`
- **Body (index-genome-counter):** the test stub's single field is both the slot index (so a winner
  reports its own slot) and the mutation counter (`mutate` increments it, which is how the
  `mutate_child` tests observe the count).
  Correct today, and James documented the hazard honestly on the type. It breaks quietly the first
  time someone writes a selection test whose individuals pass through a mutation path — the index
  no longer identifies the slot, and the failure looks like a selection bug. A second field costs
  nothing and removes the coupling.

