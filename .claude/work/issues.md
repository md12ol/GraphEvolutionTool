# Issues — work not yet in the tracker

**19 issues are open in the tracker** at `md12ol/GraphEvolutionTool` — that is the source of
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

### Align sir_sim's length and profile with whatever the meeting decides
- **For (align-sir-length-profile):** Michael (`md12ol`) — his to take and to assign on filing
- **Project (align-sir-length-profile):** `md12ol/GraphEvolutionTool`
- **Filed (align-sir-length-profile):** not yet — **deliberately held until the meeting settles
  `collab.md` #15**, since the issue cannot state its own acceptance criteria before then
- **Component (align-sir-length-profile):** `get/src/sir.rs:149-153`, and the seven tests below it
- **Body (align-sir-length-profile):** `sir_sim` was merged with the spec §5.2 conventions while
  Michael's stated position is that the legacy C++ is the intended behaviour; this issue carries
  the correction so that landing PR #31 early does not lose it.
  The engine currently reports `length = profile.len() - 1` and a profile with no trailing zero.
  `legacy/Graph.cpp:147-149` increments `epiLen` on the final burnout pass and writes
  `epiProfile[epiLen] = 0`, so the C++ length is one higher and its profile one element longer.
  `spread` is **not** in question — `totInf` at `legacy/Graph.cpp:98-102,148` matches ours exactly.
  If the meeting adopts the C++ convention the change is small: push the terminating zero, return
  `profile.len()` as the length, and update the seven tests in `get/src/sir.rs`. If it upholds the
  sheet, close this as no-change and record that in `decisions.md`. Either way #17 must not start
  consuming these two values first — `epi_length` reads the length directly and `epi_prof_match`
  computes RMSE against the profile, so both shift.

