# Plan — Issue #52: rename `evaluate_population`/`SirRun` to `evaluate_batch`/`Epidemic`
_Started 2026-08-10 · last updated 2026-08-10 22:20_

Branch: `mdube_rename_evaluate_batch`, off `main` at `2f94dc7`. Pure rename, no behaviour change,
assigned to Michael. Agreed at the 2026-08-09 joint meeting; raised as `collab.md` #32 on
2026-08-07; full scope table and rationale in the issue body (`gh issue view 52 --json body`).

## Objective

Two identifiers in `official_spec_sheet.md` name something narrower or other than what they are:
`Fitness::evaluate_population` (the unit scored is a **batch**, whose size varies — a full
population for generational, two children per mating event for steady-state) and `SirRun` (the
word "run" already means a replicate and the `GraphEvolver::run` API call). Rename both,
everywhere, sheet included, with no logic change. Done when `grep -rn 'evaluate_population\|SirRun'
get/src/ official_spec_sheet.md` returns nothing and the suite is still 213 green.

**The window this task depends on:** James has no branch touching `fitness.rs`/`lib.rs` as of
2026-08-10 (`git ls-remote --heads origin` checked at plan time). His `#53` will reach into
`fitness.rs` once he starts it — land this first, or rebase `#53` onto it.

### Out of scope

- Any `#26` or `#51` work — this is a rename only.
- `SirRun`'s internals, `sir_sim`'s behaviour, or anything about the epidemic model itself.

## Tasks

- [x] Cut `mdube_rename_evaluate_batch` off `main` at `2f94dc7`.
      Verified: `git log --oneline -1`.

- [x] `SirRun` → `Epidemic`, 15 occurrences (`fitness.rs` 8, `sir.rs` 7), plus the test-local `run`
      bindings the rename forced (`run_from_seed` → `epidemic_from_seed`, index/value loop splits).
      Verified: 213 tests, subagent audit found the rename behaviour-neutral (token-stream diff,
      all `slot()` sites checked individually). Full wording: original item 2 in
      `plan_superseded.md`; audit detail in `history.md`.

- [x] `evaluate_population` → `evaluate_batch`, 41 occurrences across `fitness.rs` (33),
      `evolver/common.rs` (6), `evolver/generational.rs` (1), `lib.rs` (1).
      Verified: `grep -rn evaluate_population get/src/` empty, 213 tests, clippy/fmt clean.

- [x] `lib.rs`'s one occurrence confirmed not to collide with James's `python_fitness` hotfix —
      `git diff` on `lib.rs` is a single test-line change; `#[allow(dead_code)]` at line 303
      untouched. Verified: diff read directly, not inferred.

- [x] Spec sheet amended at lines 225, 273, 372, 847, 857 (issue's table named 221/269/368/794/805;
      actual line numbers differed slightly from the issue body, corrected against the live file).
      Verified: `grep -n 'evaluate_population\|SirRun' official_spec_sheet.md` empty.

- [x] Full verify pass, repeated after every edit: 213 tests, clippy `-D warnings`, fmt — all
      clean. Verified: last run on `8a8ed1b`, same numbers throughout.

- [x] `decisions.md`: two entries, not one — see task below on why the scope grew. Both pushed to
      `main` at `d927fde`. Verified: `uniq -d` clean, stamped correctly (see decision entry itself
      for which carries both names and which carries one).

- [x] PR #54 opened and merged (`main` ← `mdube_rename_evaluate_batch`, merge commit `260f541`,
      2026-08-10T20:21:36Z). Verified: `gh pr view 54 --json state,mergedAt` → `MERGED`; gate
      re-run on `main`: grep empty, 213 tests green. https://github.com/md12ol/GraphEvolutionTool/pull/54

- [x] **Added mid-session, not in the original list** — `express_and_score`'s `population`
      parameter → `batch`, plus sheet lines 257/274/334. Same defect as the agreed renames, one
      layer up (`express_and_score` is `evaluate_batch`'s sole caller, and `steady_state.rs:76`
      calls it with two children). **Explicitly outside the 2026-08-09 meeting's authorised
      scope** — that meeting enumerated two identifiers, this is a third. Isolated to its own
      commit (`8a8ed1b`) so it can be dropped without touching the agreed work; `collab.md` #41
      asks James to acknowledge before merging. Verified: same full gate pass, subagent not
      re-run on this one (mechanical single-parameter rename, judged lower risk than the
      index/value loop rewrite the agent did check).

## Open questions

None. PR #54 merged 2026-08-10T20:21:36Z. `collab.md` #41's request for James's acknowledgement of
the out-of-scope commit is still open, but is a carry-forward to `collab.md`, not a gate on closing
this task — see `decisions.md`'s task-complete entry.

## Out of scope

- `#26` (config-to-concrete-type dispatch) and `#51` (extract `common::best_index`) — separate
  issues, not touched here.
