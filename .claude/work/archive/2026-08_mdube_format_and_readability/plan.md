# Plan — Land GitHub issue #22: format the tree once, then a readability pass
_Started 2026-08-05 · last updated 2026-08-06_

## Objective
Close out GitHub #22: one tree-wide `cargo fmt` commit (nothing else in it), decide the
`needless_return` lint policy, then a pure readability pass over already-correct code — naming,
function length, comment density. **No behavior changes anywhere in this task.** Anything that
turns out to be a real defect during the pass gets its own issue instead of a quiet fix here
(explicit in the issue body).

Out of scope: implementing `GenerationalEvolver` (#25, James's) — `generational.rs` gets swept by
the formatter like everything else, but its `todo!()` stubs are not readability-pass material
until #25 lands and there's real logic to read.

## Tasks
- [x] Confirm James's `generational.rs` is not currently being edited — his live branch is #24
      (`config.rs`, `config.example.toml` only). Re-check immediately before landing task 3, since
      this can change between sessions.
      **Verify by:** `git ls-remote --heads origin` shows no branch touching `generational.rs`;
      confirmed clean 2026-08-05.

- [x] Create feature branch `mdube_format_and_readability` off `main`.
      Verified: `git branch --show-current`, ancestry confirmed.

- [x] `needless_return = "allow"` decided and encoded in `get/Cargo.toml`. Commit `1409d60`.
      Verified: `git diff --stat` touched only `Cargo.toml`; clippy no longer flags it.

- [x] Tree-wide `cargo fmt` commit, nothing else in it. Commit `4898c51` — `generational.rs`
      (import reorder) and `sda.rs` (line-wrap) were the only offenders, matching `traps.md`.
      Verified: `cargo fmt -- --check` clean.

- [x] Readability pass, whole tree, two rounds. Round 1: commits `73fb554`, `01635c2`, `e9ff5ad`,
      `90c7eeb` (initial per-file pass; `best_of` extracted in `steady_state.rs`). `config.rs`
      unskipped once #24/PR #42 merged. Round 2: a deeper "would this confuse a non-Rust reader"
      pass via 12 parallel review agents, applied file-by-file with user approval each time —
      commits `2f04494`…`971feef` (9 files: `graph.rs`, `genome.rs`, `evolver/mod.rs`,
      `edge_edit.rs`, `sda.rs`, `common.rs`, `steady_state.rs`, `fitness.rs`, `sir.rs`). `config.rs`
      and `lib.rs` needed nothing in either round. `generational.rs` stayed out of scope throughout.
      Verified: `cargo test` 135/135, `cargo fmt -- --check` clean, clippy at the pre-existing
      `generational.rs` baseline — re-confirmed 2026-08-06 after round 2. Detail: `history.md`.

- [x] Opened **PR #43** against `main`, assigned to James, body carries `Closes #22`.
      Verified: `gh api repos/md12ol/GraphEvolutionTool/pulls/43` — base=`main`,
      head=`mdube_format_and_readability`, assignees=[`shorinbonsai`], `mergeable_state: clean`.
      Detail: `history.md`.

## Open questions
None currently — the lint policy, the readability scope, and the sequencing gate are all settled
by the issue text and current repo state.

## Out of scope
- Implementing `GenerationalEvolver::run`/`advance_generation` — GitHub #25, James's.
- Any behavior change surfaced while reading — gets filed as its own issue (`issues.md` staging
  first), not fixed inline.
