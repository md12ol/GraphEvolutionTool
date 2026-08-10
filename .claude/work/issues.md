# Issues — work not yet in the tracker

**7 issues are open in the tracker** at `md12ol/GraphEvolutionTool` — that is the source of
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

*Synced 2026-08-10 — James: the `evaluate_population`/`SirRun` rename removed, filed as **#52**
(assigned to md12ol) once the 2026-08-09 meeting unblocked it — `collab.md` #32. Its scope table
was already a stale fork: #52 found occurrences in `lib.rs` and `generational.rs`, and sheet line
269, that the staged entry did not list. Open count corrected 17 → 8, counted from
`gh issue list --state open --json number -q 'length'`.*

*Synced 2026-08-10 22:55 — Michael: no staged entry involved this cycle — #51 closed by PR #55's
merge, and its follow-up sweep went straight to the tracker as **#56** (unassigned) rather than
being staged here first, since it was already root-caused while scoping #51. Open count 8 → 7,
recounted the same way.*

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


### Reformat `best_index` — `cargo fmt -- --check` fails on `main`
- **For:** Michael — it is #51's code, and the fix is one `assert!`.
- **Project:** `md12ol/GraphEvolutionTool`.
- **Filed:** not yet — staged 2026-08-10 by James, during GitHub #53.
- **Component:** `get/src/evolver/common.rs:45`, the `assert!` at the top of `best_index`.
- **Body:** `cargo fmt -- --check` exits non-zero on `main`, and has since `79c10aa`. rustfmt wants
  the one-line `assert!(!fitnesses.is_empty(), "cannot pick a best of no individuals");` split
  across four lines; it is 84 characters, four past the default width. One file, one hunk, no
  other diff in the workspace.
- **Why it matters more than its size:** it is a **gate that is now red for everyone**, and the
  repo treats these as binary — `traps.md` records #25 flipping
  `cargo clippy -p get --all-targets -- -D warnings` to passing precisely so a single new warning
  would stand out. A permanently failing `cargo fmt -- --check` trains people to skip the check,
  and then the next real formatting drift arrives invisibly.
- **How it was found and bounded:** running the verify sweep for #53 on a branch merged up to
  `6552d25`. Confirmed **not** mine by running `cargo fmt -- --check` on `79c10aa`, `9274f38` and
  `6552d25` with #53 not checked out — fails at all three, and was clean at `9bba043` immediately
  before Michael's push. `79c10aa` is the commit that introduced the `assert!`, inside PR #55.
- **Fix:** `cargo fmt` and commit; the diff is the four lines rustfmt already prints. Nothing to
  design.
- **Relationship to #56:** adjacent but not the same, and it should not wait for it. #56 sweeps
  `generational.rs` and `steady_state.rs` for divergent style and duplication — a judgement task
  over two other files. This is a mechanical reformat of `common.rs` that unblocks a gate today.
  Folding it into #56 is reasonable if the sweep starts immediately; leaving the gate red until
  the sweep is scheduled is not.
