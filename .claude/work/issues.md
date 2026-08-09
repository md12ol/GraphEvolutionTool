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


### `cargo doc` warns twice on a private intra-doc link in `sda.rs`

- **Where:** `get/src/genomes/sda.rs`, the doc comment referencing [`INIT_CHAR_MUTATION_RATE`].
- **Impact:** cosmetic only — `cargo doc -p get --no-deps` emits "this item is private ... this link
  will resolve properly if you pass `--document-private-items`", twice. No gate covers `cargo doc`,
  so nothing fails; it just means the rendered docs carry a broken link. Whoever owns `sda.rs` can
  either make the constant public or write it as plain text rather than a link.
- **Noticed:** 2026-08-08, checking that #29's new memory-note table in `run`'s docstring rendered.
  Confirmed **pre-existing** by stashing the #29 changes and re-running — same two warnings, so it
  is not something #29 introduced.

## Ready to file — root-caused and evidenced

### <title — imperative, issue-ready>
- **For:** teammate / team / unassigned
- **Project:** the tracker project it belongs to
- **Filed:** not yet
- **Component:** `path:line`
- **Body:** <open with a sentence on this line — a bare label is identical in every entry, which
  is what union merge folds together. See CLAUDE.md, "Formatting for union merge".>
  What's wrong, the mechanism with `path:line`, evidence, how to reproduce, candidate fixes.

### Rename `evaluate_population` to `evaluate_batch` and `SirRun` to `Epidemic`

- **For:** unassigned — a rename touching files #19 and #25 both claim, so it wants an owner and a
  quiet window rather than a volunteer
- **Project:** `md12ol/GraphEvolutionTool`
- **Filed:** not yet — **blocked on the joint meeting**, `collab.md` #32. Both names are in
  `official_spec_sheet.md`, so neither may be changed by one owner. Do not file before it is agreed
- **Component:** `get/src/fitness.rs`, `get/src/sir.rs`, `get/src/evolver/common.rs:244`
- **Body:** Both identifiers name something other than what they hold, and the sheet has to be
  amended in the same change. The unit the engine scores in one call is a **batch of graphs**,
  whose size varies by evolver: the whole population for a generational cycle or either evolver's
  starting population, but only the two new children for a steady-state mating event
  (`steady_state.rs:75-76`, sheet line 509 and §6.3). `Fitness::evaluate_population` (sheet 221,
  794, 804) therefore misdescribes its own argument for most of a steady-state run — it should be
  `evaluate_batch`. Separately `SirRun` (sheet 368) is **one epidemic**, while "run" already means a
  replicate (`run_seed`, §8.1) and the API call `GraphEvolver::run`; three meanings, two of them
  within four lines of each other in `fitness.rs`. It should be `Epidemic`. Pure rename, no
  behaviour change, so it should land between workstreams — #19 and #25 both edit these files.
  Scope table and the already-done local renames are in `collab.md` #32.
- **Noticed:** 2026-08-07, writing the comments for #18 — rename-evaluate-population-and-sirrun


