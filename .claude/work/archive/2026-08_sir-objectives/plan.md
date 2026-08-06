# Plan — Implement the three SIR objectives over `sir_sim` (issue #17)
_Started 2026-08-04 · last updated 2026-08-04_

## Objective

Implement `epi_spread`, `epi_length` and `epi_prof_match` as `Fitness` implementors over
`sir_sim`, each averaging `num_epidemics` runs, with the short-epidemic re-roll and
position-indexed epidemic seeding of spec §5.2. Done when all three score a hand-built
deterministic epidemic correctly, `direction()` is asserted per objective, and the target-length
mismatch behaves as §5.2 documents.

Branch `mdube_sir_objectives`, PR opened and left for James to merge.

**Out of scope** — each has its own issue and owner:

- The atomic evaluation counter that produces the batch seed — **#18**, mine, next. #17 implements
  everything *given* a batch seed; a stub supplies one until then (`hotfixes.md`).
- Config schema for `num_epidemics` / `min_epidemic_length` / `max_epidemic_retries` /
  the fitness variants — **#24**, James. The objectives take a plain params struct, exactly as
  `SirParams` in `get/src/sir.rs` takes one rather than depending on the config schema.
- Wiring objectives into `GraphEvolver::run`'s dispatch — **#26**, tier 4.
- The Python setter delivering the target profile — later issue. #17 validates the target in the
  constructor; nothing calls it from Python yet.
- Any restructure of `fitness.rs` into a directory. James's #15 and #19 both edit it
  (collab.md #14), so the diff stays as small as the work allows — but the objectives themselves
  go **in** `fitness.rs`, beside the trait they implement. See "file placement" below.
- Whether users supply their own Rust objective as a drop-in file — raised in `collab.md`, and it
  is a sheet question affecting **#26** far more than this issue. Build to the sheet meanwhile.

## File placement — a seam at "run an epidemic" vs "read one"

`sir.rs` owns **how epidemics are sampled**: `sir_sim`, `SirParams`, and now the batch runner with
its re-roll and seeding. `fitness.rs` owns **how one is read**: the trait and the three thin
objectives, beside the `SirFitness` placeholder they replace.

Chosen to make forking easy, which is the standing constraint on this issue (Michael,
2026-08-04). The re-roll and position-indexed seeding are the subtle part and fail *silently* when
copied wrong, so they live in exactly one public function that a fourth objective calls. Someone
adding "time to peak infection" then writes a reading of ~15 lines with three worked examples next
to the trait to copy.

Rejected: a separate `sir_fitness.rs`, and duplicating the batch loop per objective — the first was
merge-coordination dressed as design, the second hands a forker 40 lines of seeding logic to get
wrong.

## Tasks

### In `get/src/sir.rs` — how epidemics are sampled

- [x] `SirBatchParams` — `epidemic`, `num_epidemics`, `min_epidemic_length`,
      `max_epidemic_retries`. Plain struct, no config dependency.

- [x] `pub fn epidemic_seeds(batch_seed, num_epidemics, max_epidemic_retries)` — position-indexed,
      no `xor`. Public so a fourth objective can reason about the pool.
      Verified: `extending_the_seed_pool_leaves_the_earlier_epidemics_untouched`.

- [x] Re-roll inside `batch_epidemics`, keeping the last attempt regardless. Verified by three
      tests: stops early, exhausts an unreachable minimum, and `min_epidemic_length = 1` never
      re-rolls.

- [x] `pub fn batch_epidemics(graph, params, batch_seed) -> Vec<SirRun>`, epidemics sequential.
      Verified: `two_graphs_under_one_batch_seed_face_an_identical_pool`; 16 `sir::` tests pass.

### In `get/src/fitness.rs` — how one is read

- [x] `EpidemicScorer` + `EpiSpread` / `EpiLength` / `EpiProfMatch`, each a thin reading.
      Verified: 6-node path scores `spread = 6`, `length = 6`; `direction()` asserted per
      objective.

- [x] `epi_prof_match` RMSE, target-fixed, plus non-empty/finite validation at construction.
      Verified by four tests including `the_divisor_is_the_target_length_not_the_overlap`.

- [x] `SirFitness` placeholder deleted; fork-path module doc added to `fitness.rs`.
      Verified: `grep -rn SirFitness get/src/` empty.

### Close out

- [x] Formatted the two touched files with `rustfmt --edition 2024`. `cargo test` 127 pass / 0
      fail; `cargo fmt -- --check` shows no new offenders. Clippy fails **identically to `main`** —
      now a `traps.md` entry, since #22's verify-by wrongly expects it to pass.

- [x] Readability pass — plain loops replace iterator chains in the four spots a reader must
      follow; comments 347 → 290 lines. Commit `8285669`, 127 tests still pass. Now a standing
      convention: `decisions.md` 2026-08-04 22:12 and `CLAUDE.md` Conventions.

- [x] PR **#40** opened and assigned to James, head sha verified against the branch. (PR #39 was
      opened unprompted and closed; see `CLAUDE.md` Conventions, first bullet.) Docs commit
      `0dab610` cherry-picked to `main` as `3d78d2b`, so `collab.md` #21 reaches James now.

- [x] Branch merges cleanly with James's #38 (`express_and_score`). PR **#40 merged** as `a53375e`;
      issue #17 closed completed. Verified on Michael's machine 2026-08-06 on `main` at `ed198c4`:
      135 tests pass, and `0dab610` is an ancestor of `main` — no commit stranded by the PR-lag trap.

- [x] Filed issue **#22**'s `Verify by:` corrected — it wanted "97 passed" *and* a clean clippy,
      both wrong. Patched via the REST API and re-read to confirm the body survived.

- [x] Docs on `main` and pushed (`29b3f6a`): three `decisions.md` entries, the clippy `traps.md`
      entry, two `CLAUDE.md` conventions, and `collab.md` #22 flagging the one that binds James.

## The one stub, carried through the whole task

`Fitness::evaluate(&self, graph)` takes no seed, so the three objectives need a batch seed from
somewhere. **#18** supplies it via the atomic counter; until then a fixed run seed stands in — one
line, marked, logged in `hotfixes.md` with #18 as its removal condition. Every test above pins the
batch seed explicitly, so **no test depends on the stub**.

## Open questions

- **Target profile element type — settled as `Vec<f64>`, implemented, blocks nothing.** Validated
  non-empty and finite at construction. Raised for James in PR #40 in case #29 (Python config)
  wants integer counts; a one-line change if so.

- **Do users supply their own Rust objective as a drop-in file?** `collab.md` #21, now on `main`.
  Affects **#26** far more than this task — its closed `match config.fitness` cannot name a type
  outside the crate. Built to the sheet as written meanwhile.

## Out of scope

- Issue #22 (format the tree) — tier 1 and also mine, but its `cargo fmt` commit rewrites
  `generational.rs`, which James's open #25 is actively rewriting. Deferred until his tree is
  confirmed clean; the issue body already carries that gate.
  **Overtaken 2026-08-06:** #22 was picked up and closed on Michael's other machine as PR #43, and
  archived as `2026-08_mdube_format_and_readability`. The tree is rustfmt-clean on `main`.
