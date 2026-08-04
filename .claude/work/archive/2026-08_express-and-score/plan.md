# Plan — `common::evaluate` becomes `express_and_score`, the sole scoring entry (#14)
_Started 2026-08-04 · last updated 2026-08-04_

## Objective

Rename `common::evaluate` to `common::express_and_score` and make it the **only** way the engine
turns a population into fitnesses. The rename is mechanical; the point is the invariant that comes
with it — **the engine never calls `Fitness::evaluate` or `Fitness::evaluate_population`
directly**, because a direct call bypasses both orientation and `NaN` rejection, and neither
failure announces itself.

Closes GitHub **#14** (tier 1). Spec §5.1.

**Done looks like:** `common::evaluate` no longer exists under that name, the sole-entry rule is
documented on `express_and_score` and on the `Fitness` trait, every call site is updated, and the
suite is green at 110.

**Verified 2026-08-04 before planning:** the invariant already holds — the only
`evaluate_population` call in `get/src/` is `common.rs:206`, inside the function being renamed, and
the only `.evaluate(` call is `fitness.rs:91`'s own default impl. So there are **no violations to
fix**, only a rename and the documentation that stops one appearing later.

**Out of scope** — see the section at the bottom. This task does **not** change orientation
behaviour: `express_and_score` still converts inward. Removing the conversions on the way back out
is #15, the next task.

## Tasks

- [x] **Branch `jsargant_express_and_score` off `main`** — done 2026-08-04, based on `2f8fc62`.
      Verified: `git branch --show-current` returns the branch, `main` is unmoved at `2f8fc62`, and
      the only untracked paths are the two stale pre-spec-sheet docs (`traps.md`).

- [x] **Rename the function and document the sole-entry rule** — done 2026-08-04,
      `common.rs:221`. Verified: `grep "^pub fn evaluate\|^fn evaluate" get/src/` returns nothing,
      and the doc names both silent failure modes (orientation, `NaN`) with spec §5.1.

- [x] **Document the invariant on the `Fitness` trait** — done 2026-08-04, `fitness.rs` on the
      trait plus both methods. Verified: `cargo doc --no-deps` generated with no unresolved links.

- [x] **Update every call site** — done 2026-08-04, `steady_state.rs` (1 `use`, 2 code, 6 test) and
      a stale doc reference at `genomes/genome.rs:12`. Verified: **110 tests**, identical to the
      pre-task baseline, so the rename moved no behaviour.

- [x] **Rename the tests that carry the old name** — done 2026-08-04, all four in `common.rs`.
      Verified: `grep -rn "fn evaluate_" get/src/` matches only the `evaluate_population` trait
      method. `NodeCount`/`Poisoned`'s own `Fitness::evaluate` impls correctly left alone.

- [x] **Format only the files this task touched** — done 2026-08-04, `rustfmt --edition 2024` on
      `common.rs` and `steady_state.rs` only (the two my longer lines dirtied). Verified:
      `cargo fmt -- --check` now names only `generational.rs` and `sda.rs`, which are issue #22.

- [x] **PR against `main`, and close #14** — **PR #38 merged** 2026-08-04 20:15 UTC as `168cc91`;
      Michael merged it. Verified from the remote, not local state: `.merged=true` and issue #14
      `state=closed, state_reason=completed`. Author/committer James Sargant, no trailers.

## Status: complete — ready for `/done express-and-score`

Every task is `[x]`. PR #38 merged 2026-08-04 20:15 UTC, issue #14 closed as completed. Nothing in
this task is outstanding.

## Open questions

- *Settled:* `EvolutionOutcome`'s field naming belongs to **#15**, not here — #15's change set asks
  for names saying the values are engine-oriented. Left untouched deliberately so it is not done
  twice. No longer blocks anything.

## Out of scope

- **#15, orientation at the boundary** — the next task, and the reason this one is small.
  `express_and_score` keeps converting inward; only the conversions *back* are #15's to remove.
- **#24, config schema** — the third tier-1 issue, its own task after #15.
- **`generational.rs`'s stale mutation doc** — staged in `issues.md`, deliberately folded into #25
  rather than filed. Not this task; `generational.rs` is also untouchable here per #22.
- **`IndexGenome`'s overloaded index/counter field** — staged in `issues.md` by Michael, test-only,
  unassigned. Not this task.
- **Issue #22, tree-wide `cargo fmt`** — Michael's.
