# sir-objectives — implement the three SIR objectives over `sir_sim` (issue #17)

**Dates:** 2026-08-04 (built, PR opened) → 2026-08-06 (closed out). Two sessions, on two different
machines — see "How this closed" below, which is the only unusual thing about the record.

**Owner:** Michael. **Shipped as:** PR **#40**, merged 2026-08-04 as `a53375e`. GitHub **#17** closed
completed.

## Objective

Implement `epi_spread`, `epi_length` and `epi_prof_match` as `Fitness` implementors over `sir_sim`,
each averaging `num_epidemics` runs, with the short-epidemic re-roll and position-indexed epidemic
seeding of spec §5.2.

## Outcome

`get/src/sir.rs` gained `SirBatchParams`, `epidemic_seeds` and `batch_epidemics` — the seeding
(`i * max_epidemic_retries + a`, never `xor`) and the re-roll, epidemics sequential.
`get/src/fitness.rs` gained `EpidemicScorer` plus the three objectives, each a thin reading over the
shared batch; the `SirFitness` placeholder and its `todo!()`s are gone. Every task item is `[x]` and
verified.

The seam is "sample an epidemic" (`sir.rs`) versus "read one" (`fitness.rs`), chosen so that adding a
fourth objective is ~15 lines next to the trait and copies none of the seeding logic — the part that
fails silently when reimplemented slightly wrong. Reasoning in `decisions.md` 2026-08-04 22:10.

**Verified at the gate**, on `main` at `ed198c4`, 2026-08-06: `cargo test` 135 pass / 0 fail, and
`git merge-base --is-ancestor 0dab610 main` passes, so the PR-lag trap stranded no commit. The 135 is
up from the 127 recorded on 2026-08-04 because #22, #15 and #24 landed in between.

## How this closed — worth reading, it is the reusable lesson

The work finished and merged on 2026-08-04, but `/done` never ran. In between, the other machine
completed and archived two unrelated tasks (`config-schema`, `mdube_format_and_readability`) without
ever being able to close this one: `work/current/` is gitignored, so **this task's record existed on
exactly one machine**, and only that machine could archive it.

That machine came back on 2026-08-06 **40 commits behind `origin/main`**, having seen neither its own
PR merge nor four others. The close-out therefore fast-forwarded `main` *before* writing any doc —
deliberately, because `hotfixes.md` stopped being union-merged when #33 narrowed the glob, so
appending to a two-day-stale base would have produced a real conflict rather than a silent merge.

**A merged PR is not a closed task here.** The evidence went into `collab.md` #30 as the case for the
new `pull_main.sh` hook, which fast-forwards `main` at session start and would have prevented all of
it.

## Left behind, deliberately

- **The SIR batch-seed hotfix** — `EpidemicScorer::batch_seed` returns the run seed unchanged at
  `get/src/fitness.rs:162-164`. Introduced by this task, blocked on **#18** (Michael's, next), and
  now committed and in both trees rather than branch-local. Carried forward at this gate, fifth
  check cycle. Common random numbers *within* a batch are correct; variation *across* batches is
  what is missing, so a full run is not yet research-usable.
- **`collab.md` #21** — do users supply their own Rust objective as a drop-in file? Raised while
  planning this task, still unanswered. It gates **#26**, not #18, so it blocks nothing immediate.
- **`collab.md` #31** — raised at this gate: one clause of James's `rustfmt`-descends-into-submodules
  trap went stale when #22 shipped. Announced rather than edited, per the shared-doc rule.

## Dropped at this gate

- The **`cargo fmt` trap** in `traps.md`, under its own written exit condition — #43 merged and
  `cargo fmt -- --check` reports zero offenders on `main`. The clippy trap was re-verified and
  **kept**: the same two `generational.rs` dead-code errors, since #25 is still unbuilt.

## Also from this task

Two standing conventions now in `CLAUDE.md`: **prefer explicit loops to iterator chains**
(`decisions.md` 2026-08-04 22:12), and **approving a plan is not authorization to push or open a
PR** — added after PR #39 was opened unprompted and closed again.
