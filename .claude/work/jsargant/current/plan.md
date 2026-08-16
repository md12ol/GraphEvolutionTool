# Plan — GitHub #20: replicate runs (one master seed, Rust-only parallelism, `max_cores`)
_Started 2026-08-13 · last updated 2026-08-13_

## Objective
`GraphEvolver.run` takes `n_runs` and `max_cores` and returns a **list** of `RunResult`, one per
replicate — matching §8.1 literally. One master seed draws the per-run seed stream, so extending
`n_runs` never invalidates replicates already collected. Native-Rust objectives run replicates in
parallel through a locally-built rayon pool; `fitness = "python"` runs them sequentially. Depends on
#27 (closed) and #71's `RunResult` shape (merged, `main`).

**Decided 2026-08-13, before coding:** `run` always returns a list, even at `n_runs=1` — matching
§8.1 rather than keeping today's single-run case backward compatible. Blast radius checked: 7
`GraphEvolver::run` call sites in `dispatch.rs`'s test module, all internal to this crate. Nothing
external depends on the old shape yet.

**Out of scope:** the memory-multiplication doc note is already written on `run`'s doc comment
(added ahead of the code, 2026-08-04 meeting) — this task completes the last column it forward-
referenced, not the whole note. `documentation/`'s site text is already spec-accurate under a
`badge-planned` span; this task's doc consequence is de-badging via the queue, not rewriting prose.

**Verify by (whole task):** same master seed gives identical results at `max_cores=1` and
`max_cores=8`; requesting 50 runs reproduces the first 30 of a 30-run request exactly — both through
real Python, per the issue's own verify-by.

## Tasks

- [x] Branch `jsargant_replicate_runs`, cut from `main` (`042f282`). `git rev-parse --abbrev-ref
      HEAD` confirmed.

- [x] `dispatch::replicate_seeds(master, n_runs)`. Five tests, two load-bearing in different
      directions confirmed by mutation: `master ^ i` caught only by the collision test, a
      `n_runs`-dependent stream caught only by the prefix test. `6f8fc5c`.

- [x] `dispatch::run_replicates` + `effective_concurrency`: per-call rayon pool, gated on
      `config.fitness`, mode derived not passed in. Five tests. Inverting the gate **hangs the
      test suite** (GIL deadlock, `.claude/reference/pyo3-maturin.md` §2) — stronger than a
      failing test. `50e4f7b`.

- [x] `run(seed, n_runs=1, max_cores=None) -> Vec<PyRunResult>`, always a list. Doc comment
      rewritten, forward-reference to #20 resolved. `RunResult.seed` is the master, not the
      per-run draw — an *inherited* test caught this wrong on the first attempt; `(master,
      run_index)` is what reproduces a replicate, the per-run seed cannot. All 7 call sites
      updated in the same commit (signature change breaks the test target, so splitting would
      leave `cargo test` non-compiling). `1abd10f`.

- [x] Four replicate tests through `run()` itself. Mutation-checked: handing every replicate the
      master seed (all `n` runs identical) is caught **only** by the distinctness test — the three
      reproducibility tests all pass on that implementation. `aa3e05c`.

- [x] Real-Python verification (`maturin develop --release`, existing `.venv`), issue's verify-by
      verbatim plus a distinctness check and a `save_logs` row check on `run_index=17`. Wall-clock
      on a scoring-dominated config: 1.74s → 0.96s → 0.50s at `max_cores` 1→2→4, flat at 8 — the
      first (0.16s) config was too small to measure anything and was replaced. Scratch, not
      committed; see `history.md` for the full output.

- [x] `documentation/jsargant_edits.md`: `#replicate-runs-ship`. Bigger than planned — found a real
      defect (`runs=` vs shipped `n_runs=` in three copy-pasteable samples) and a stale `src`
      pointer, not just badges. `8cba899`.

- [x] **PR #83** open against `main`, review requested from Michael, `mergeable=MERGEABLE`, carrying
      exactly the three intended files. Body flags the #72/#83 `dispatch.rs` collision (`evolve`
      gains a `base_graph` param on #72; this branch calls it twice) with the two exact line
      numbers to fix on whichever merges second.

## Open questions
- None — all tasks complete and verified. Task list closes with PR #83's merge; see `handoff.md`.

## Out of scope
- `set_base_graph` / #28 — separate task, parked (`work/jsargant/parked/set-base-graph/`), blocked
  on PR #72 merging.
- Any change to `save_logs`/`save_results` to handle a *list* of results (concatenating replicate
  logs) — not raised in #20's text or §8.1; `save_logs` already writes `seed` and `run_index` per
  row specifically so replicate logs can be concatenated externally, per #71's own reasoning.
