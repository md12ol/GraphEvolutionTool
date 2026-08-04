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

*Folded in 2026-08-04 19:00 — Michael: the two PR #30 review findings moved into **#25**, whose
implementer touches both `generational.rs` and the `common.rs` test module. Neither warranted a
standalone chore.*

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

