# Plan — Issue #19: `PyFitness` adapter and `set_fitness_function`
_Started 2026-08-07 · last updated 2026-08-07_

## Objective

Let a user's Python callable act as a `Fitness` objective: a `PyFitness` adapter implementing the
trait over a registered callable + `Direction`, and `set_fitness_function` to register them on
`GraphEvolver` before a run. Batched contract only — one call per batch, never per individual —
because a per-individual callback loses all rayon parallelism and pays GIL contention on top.

GitHub **#19**, tier (2), depends on #24 (closed). Spec §5, §8.

**Scope boundary, agreed 2026-08-07:** `GraphEvolver::run` stays a `todo!()`. The whole
config→dispatch match is #26 (Michael's, tier 4, unstarted — confirmed no branch exists). #19 stops
at the boundary #26 will call into: a helper that resolves the registered Python objective into a
`Box<dyn Fitness>` or a clear error, testable without the dispatch match existing. #19's own
verify-by ("a Python objective drives a full run end to end") is read as *through a directly
constructed evolver*, not through the pyclass entry point — see `decisions.md`.

**Out of scope:**
- `GraphEvolver::run`'s dispatch match — #26, Michael's.
- Replicate runs, `max_cores`, the master-seed stream — §8.1, a separate issue.
- Anything in `steady_state.rs` / `generational.rs` — this task adds an objective, not an evolver
  change.

## Tasks

- [x] **Test harness fix**, `get/Cargo.toml`. `extension-module` moved out of `[dependencies]`
      (was: unconditional there, which skips linking libpython and fails `cargo test` with dozens
      of undefined `Py*` symbols); `[dev-dependencies] pyo3` carries `auto-initialize` instead.
      Commit `6e2d262`; a permanent smoke test guards the manifest against being reverted. Needs
      `LD_LIBRARY_PATH` at test time — task moved to task 8, below.

- [x] **`PyFitness` in `fitness.rs`.** Both trait methods route through an inherent `score_batch`
      rather than `evaluate` calling `evaluate_population` — that pairing is the latent stack
      overflow `collab.md` #33 documents. Commit `7115e2e`, 10 tests (its commit message says 11 —
      miscounted there, corrected here; the branch adds 22 in total, 176 → 198). **Found while testing, not
      predicted:** deleting the `evaluate_population` override doesn't just fall back to per-graph
      calls, it **deadlocks** — the rayon worker's `Python::attach` blocks on a GIL the calling
      thread already holds while waiting on rayon. Suite hung 2 minutes before being killed; no
      failure message. Written into the doc comment and `.claude/reference/pyo3-maturin.md` §2.

- [x] **`impl Fitness for Box<dyn Fitness>`.** Commit `aa92a09`, two tests — one on the impl
      directly, one through `common::express_and_score` (the path the engine actually takes).
      **Both confirmed non-vacuous**: removed each defaulted method in turn and re-ran; both
      failures caught, with the `direction` case visible as data (`[1.0..5.0]` vs `[-1.0..-5.0]`).

- [x] **`set_fitness_function(callable, direction: &str)`.** Commit `6e68bc1`, 5 tests. Also added
      `FitnessConfig::type_name()` so the config-mismatch message names the objective in the user's
      own words — #26's dispatch will want the same. Two test-fixture misses along the way (guessed
      `num_operations`/`network_size` field names and TOML table ordering instead of reading
      `config.example.toml` first) cost two cycles; corrected after the second, and by
      instruction — read the real schema before writing against it, going forward.

- [x] **The `python_fitness` boundary #26 calls into.** Commit `58e5781`, 4 tests, delivers #19's
      second verify-by (missing-callable → `ValueError` naming `set_fitness_function`, not a panic).
      Carries a temporary `#[allow(dead_code)]` — #26 is its only non-test caller and doesn't exist
      yet, and one warning would break the `-D warnings` gate #25 restored. `hotfixes.md` entry,
      `Remove when: #26 lands and calls it`.

- [x] **`pyproject.toml`** — added 2026-08-07 on instruction, not on the original plan. Root-level,
      `manifest-path = "get/Cargo.toml"`, `features = ["pyo3/extension-module"]`. Verified by
      installing the built wheel into a throwaway venv: `import get`, construct `GraphEvolver`,
      register a callable, both rejections arrive as Python `ValueError`s. Corrected one false claim
      in its own comment along the way — see `decisions.md` 2026-08-07 22:10.

- [x] **Doc-comment note above `run`'s `todo!()`.** Commit `b1f8557`: take the objective from
      `python_fitness`, release the GIL around the loop. Names `Python::detach` — checked against
      pyo3 0.27's source rather than assumed; `allow_threads` is deprecated since 0.26.

- [x] Full verify pass, 2026-08-08 on `b1f8557`. **198 tests** (176 + 22, counted from the diff, not
      inferred); clippy byte-identical to the pre-edit baseline and `-D warnings` exits 0;
      `cargo fmt --check` clean tree-wide; rustdoc unchanged at its 4 pre-existing warnings with no
      unresolved links; `--features pyo3/extension-module` still builds; and the freshly built wheel
      installs and drives the Python side end to end.

- [x] `traps.md` — two entries: the `extension-module`/`cargo test` linking trap (with the
      `LD_LIBRARY_PATH` requirement), and the rayon-deadlock finding from task 2. Written at the
      2026-08-07 save, ahead of the rest of the task list — the sweep surfaced them as worth
      capturing immediately rather than waiting.

- [x] Pushed and opened **PR #48**, `Closes #19.` in the body. Verified on the remote: `state: open`,
      `mergeable: true`, `head.sha` equal to local `HEAD` (`b1f8557`), body round-tripped through
      `--jq '.body'` and diffed identical to its source but for a trailing newline. Awaiting Michael.

## Open questions

- **None blocking.** The scope-boundary question (stop before #26's dispatch) was settled
  2026-08-07 before any code — see `decisions.md`.

## Out of scope

- **`GraphEvolver::run`'s dispatch** — #26, Michael's, unstarted (no branch exists).
- **Replicate runs / `max_cores` / master-seed stream** — §8.1, a separate issue.
- **`collab.md` #35** (§6.2 "track the best") and **#36** (`outcome` duplication) — both awaiting
  the joint meeting, unrelated to this task.
- **`collab.md` #27** (`Swap`'s degree floor) — still James's, unrelated, sixth gate.
