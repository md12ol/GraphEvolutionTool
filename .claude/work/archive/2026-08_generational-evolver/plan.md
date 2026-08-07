# Plan — Issue #25: implement `GenerationalEvolver::run` and `advance_generation`
_Started 2026-08-06 · last updated 2026-08-07 · **complete**_

## Objective

Replace the two `todo!()`s in `get/src/evolver/generational.rs` (`:31` and `:53`) with a working
generational strategy: score, log, advance, repeat for `num_generations`, carrying `elite_count`
elites forward each time. It must be **indistinguishable from steady-state in every shared
mechanism** — same RNG, same scoring gate, same mutation helper — because the two strategies
drifting apart is the specific failure this design guards against.

GitHub **#25**, tier (2). Spec §6.2, §4, §5.1. Two cleanups folded into the issue on 2026-08-04.

**Out of scope:**

- **Config-to-evolver dispatch** — #26, Michael's. This task never reads a `Config`.
- **Run output shape** — `EvolutionOutcome` already exists and steady-state already populates it;
  changing it is #21/#27.
- **Anything in `steady_state.rs`.** Read it as the reference, do not edit it.

## Tasks

Agreed 2026-08-06 and worked straight through. Branch `jsargant_generational_evolver`, four commits,
**merged as PR #46** on 2026-08-07; issue #25 closed. All items complete.

- [x] Branched off `main` at `841e79d` — **not** the `d8892e9` this plan was written against; `main`
      had moved 7 commits (PR #45 merged, #17 archived). Clippy baseline captured on the clean tree
      first, per `traps.md`.

- [x] **Cleanup 1:** `advance_generation`'s doc comment now points at `common::mutate_child` and
      names both rolls. Commit `349399e`; rustdoc clean for the file.

- [x] **Cleanup 2:** `IndexGenome` split into `index` + `mutations` with an `IndexGenome::new`
      constructor. Commit `ab68796`; all 24 `common` tests still pass, unchanged in intent.

- [x] `new` asserts `elite_count < population.len()`, with the reasoning and the deliberate absence
      of a `tournament_size` twin in the code comment. Pinned by a `#[should_panic]` test naming
      both numbers. In commit `a30422e`.

- [x] `advance_generation` — elites via `rank`, then pairs from `Selection::select`, one crossover
      roll, and `mutate_child` per child. No mutation roll made locally. Commit `a30422e`.

- [x] `run` — `ChaCha8Rng`, `express_and_score` only, generation 0 logged, rescoring every
      generation. Added a local `outcome` that **moves the winner's graph out of the final scoring
      pass** rather than re-expressing it, which spec §6.2 asks of generational specifically and is
      the one place it deviates from steady-state's `outcome`. Commit `a30422e`.

- [x] 13 tests in `generational.rs`. Stubbed to a no-op, **4 of 13 fail** — elites-are-the-best,
      max-mutations, run-improves, seeds-diverge. Commit `7de4a66`.

- [x] Full verify pass on `7de4a66`: **167 tests** (up from 154); clippy diff shows the four
      baseline lines **gone and nothing added**, and `-D warnings` now exits 0; `cargo fmt --check`
      clean tree-wide; rustdoc unchanged at its 4 pre-existing warnings.

- [x] `traps.md` — the clippy entry is **retired, not deleted**: replaced by a successor saying the
      gate now passes, keeping the `git stash -u` pitfall. Verified on `main` at `94a4679` —
      `cargo clippy -p get --all-targets -- -D warnings` exits 0. Commit `2ae232a`.

- [x] Pushed and opened **PR #46**, `Closes #25.` in the body. Verified on the remote: `state: open`,
      `mergeable: true`, `head.sha` equal to local `HEAD` (`7de4a66`), body round-tripped through
      `--jq '.body'` and diffed identical to its source but for a trailing newline.

- [x] Docs committed (`3b925aa`), `origin/main` merged on top (`94a4679`), pushed as `2ae232a`.
      My `collab.md` #32 renumbered to **#36** — Michael published his own #32 first. Union tails
      read by hand; one eaten blank line re-inserted, `uniq -d` clean on all five.

## Open questions

- **None blocking.** The one fork — whether `new` gets a backstop assert — was settled 2026-08-06
  before any code: it does. Needs a `decisions.md` entry at the next `/save`.

- **Worth a line in the PR, not a question:** `outcome` now exists once per evolver, ~10 similar
  lines each. They agree on the result — `express` is deterministic, so re-expressing the winner
  (steady-state) and moving its graph out of the final scoring pass (generational, per spec §6.2)
  return the same graph. The difference is only which cost you pay: one extra expression per run,
  or one population of `Graph`s held alive across the loop. Mild duplication, no divergence.

- **Spec reading, recorded rather than asked:** §6.2's "track the best" is implemented as *best of
  the final population*, taken from the last scoring pass's graphs. A running best-ever was
  rejected: it can report a fitness no current individual has and disagree with the last history
  row, and at `elite_count >= 1` under a deterministic objective the two are identical anyway.

## Out of scope

- **Dispatch from `Config`** — #26, Michael's.
- **`steady_state.rs`** — reference only, not to be edited.
- **The SIR batch-seed hotfix** (`fitness.rs:162-164`) — Michael's, blocked on #18. Untouched here.
- **`collab.md` #27** (`Swap`'s degree floor) — still waiting on James, unrelated to this task.
- **PR #45 (#23)** — ~~open and awaiting Michael~~ **merged 2026-08-06 16:05 UTC as `334ef63`**;
  issue #23 closed. If follow-up is asked for, that is a separate
  branch and a separate piece of work; do not fold fixes into this one.
