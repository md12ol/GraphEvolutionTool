# Plan — Implement config→concrete-type dispatch in `GraphEvolver::run` (#26)
_Started 2026-08-10 · last updated 2026-08-11_

## Objective
`GraphEvolver::run` (`get/src/lib.rs:244`) is `todo!()`. Wire it up per spec §1/§6/§8 and
issue #26: erase the fitness objective to `Box<dyn Fitness>` first (one match, one arm per
objective), then dispatch strategy×genome as a 2×2 match (Generational/SteadyState ×
EdgeEdit/Sda), building each population from config since `Genome` has no uniform
constructor. Cache `best_fitness`, release the GIL around the run, return the best graph's
edge list.

**Done** = a full `config.toml` completes end to end for all four strategy×genome
combinations; an SDA `init_state >= num_states` is rejected at startup, not mid-run.

**Out of scope**: `save_logs`/`save_results` (still `todo!()`, no issue yet covers them),
`best_fitness()` removal (#27), `set_base_graph` (#28, so `EdgeEditContext.base_graph` is
always a fresh empty `Graph::new(network_size, max_edge_multiplicity)` here), replicate runs
and `max_cores` (#20), the convergence-log/result-object shape (#21/#27 — this task keeps
caching into the existing `best_fitness: Option<f64>` field and returning the plain edge
list `run` already promises).

## Sequencing note: #53 is done — the prerequisite is gone, not worked around
**Resolved 2026-08-11.** This plan originally carried a task to swap `target_profile_path` to
an inline `Vec<f64>` as a hotfix, because #26's `EpiProfMatch` arm cannot be built without it.
James was already working #53 on his own machine; that task was written, then **fully reverted
uncommitted** when we found out, and #53 landed properly as PR #57 (merged `f25e33d`, closed).

`FitnessConfig::EpiProfMatch` now carries `target_profile: Vec<f64>`, validated non-empty and
finite in `Config::validate` (`config.rs:491`). So the `EpiProfMatch` dispatch arm passes the
vector straight to `EpiProfMatch::new` with nothing to work around, and **no hotfix exists** —
`hotfixes.md` gained no entry.

Branch `mdube_run_dispatch` was rebased onto `origin/main` at `cb5590f`; it had no commits, so
this was a fast-forward with nothing to replay.

**Does not block this task, but shares the file:** GitHub #58 (James) adds rejection of
`target_profile` supplied under a non-`epi_prof_match` objective, in `Config::from_toml_str`'s
raw-text pass. #26 reads `config.rs` and modifies nothing in it, so there is no overlap.

## Tasks
- [x] Branch `mdube_run_dispatch` off `main` at `7b22f1f`, before any `get/src/` edit.
      `git rev-parse --abbrev-ref HEAD` prints `mdube_run_dispatch`. The gap that let this be
      forgotten is fixed in `/start`'s body; `collab.md` #44.

- [x] Rebase onto `origin/main` at `cb5590f` — #53's merge made this task's field-swap task moot;
      it was reverted uncommitted and dropped. `cargo test -p get` 216/216 and clippy clean on the
      rebased branch. Original wording in `plan_superseded.md`; see the sequencing note above.

- [x] **Side task, not #26** — restore the `cargo fmt`-clean tree: wrap `best_index`'s assertion,
      `get/src/evolver/common.rs:45`. Own branch `mdube_fmt_best_index` (`ca8b40e`), not #26's, so
      the dispatch PR stays reviewable. `cargo fmt -p get -- --check` clean, 216/216, clippy clean.
      Belongs to #56's comment thread, done ahead of its staged order because `traps.md`'s rustfmt
      entry needs a clean tree to make stray formatting visible. **Merged** as PR #59 (James,
      `152a5b8`) — `main` is `cargo fmt`-clean again; #56's remaining scope is unaffected.

- [x] Step 1 — objective erased to `Box<dyn Fitness>`: `GraphEvolver::objective` + `sir_sample_params`,
      commit "Erase the objective..." (`e1b97f5` on `mdube_run_dispatch` — hash moved once from later
      rebases; cite by commit message on an open branch, not hash). Forwarding impl already existed
      from #19, untouched. `run` calls it, so #19's `#[allow(dead_code)]` is gone. 221/221, 5 new tests.

- [x] `GenomeConfig` → context + population builders: `edge_edit_start`/`sda_start`, commit
      "Build the starting population..." (`395e03c`). `run` builds the population and draws the
      evolver's seed from the same stream. `init_state >= num_states` errors from `run`, not a panic.

- [x] Extract the dispatch layer into `get/src/dispatch.rs`, commit "Move the dispatch layer..."
      (`34f4d6b`) — pure move, test count unchanged at 226, engine still free of `pyo3`/`config`.
      `evolver/common.rs` rejected: it would invert that dependency. `decisions.md` 2026-08-11 11:26,
      `collab.md` #47.

- [x] Strategy×genome dispatch + `run` wiring, commit "Implement the strategy x genome dispatch..."
      (`7e2f5d7`) — genome outside, strategy inside a
      generic `run_strategy`, so 2+2 arms cover all four. `run` is no longer `todo!()`. GIL released
      around the evolution. `best_fitness` cached in the objective's own units, confirmed 2026-08-11.
      231/231, clippy clean, and the shipped `config.example.toml` runs end to end from Python in
      3.4s, reproducibly. Maximization verified climbing (11.5→71.5 over 400 generations at rate 0.5).

- [x] `#[allow(dead_code)]` on `python_fitness` deleted (same commit as step 1, above) — `objective`'s
      `python` arm is its non-test caller. The `hotfixes.md` entry stays until #26's PR merges, since
      `main` still carries the attribute; drop it then.

- [x] Full gate on the rebased branch: 231/231 tests, clippy clean at `-D warnings`, `cargo fmt
      --check` clean (`main`'s own PR #59 merged, so it's fmt-clean too).

- [x] **PR #60 merged** 2026-08-11T16:00:19Z (`97d9e02`). Verified on `main` post-merge:
      `cargo test -p get` 231/231, GitHub #26 `closed`.

## Open questions
- **Settled 2026-08-11:** `best_fitness` caches the objective's **own units**, not engine
  orientation — a maximizing run reports `1.47`, not `-1.47`. Engine orientation is an internal
  device that stops at this boundary (§5.1). #27 removes the field and its getter regardless.

## Out of scope
- #53 in its entirety — James did it as PR #57 while this task was open; the planned hotfix was
  reverted uncommitted. See the sequencing note above.
- `config.example.toml`'s flat search — staged in `issues.md`, discussion raised as `collab.md` #48,
  parked behind the current issue set. Not an engine defect.
- `save_logs`/`save_results`, replicate runs, `set_base_graph`, `best_fitness()` removal — each
  has (or will have) its own issue.
