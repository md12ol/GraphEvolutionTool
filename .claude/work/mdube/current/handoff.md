# Next session — 2026-08-13

**Machine:** `skynet` · saved 2026-08-13 · `007d3cf`

Read this task's `plan.md` and `.claude/work/decisions.md` first, then `.claude/work/traps.md` —
one new entry there is about tooling you may touch (VS Code + the docs worktree).

**Where things stand:** GitHub #21, on branch `mdube_run_output`. Tasks 1–3 of 11 done: the stacked
branch, `ci_95` in the engine (`evolver/mod.rs`, `evolver/common.rs`), and `ci_95` carried through
erasure unconverted (`dispatch.rs`, `py_result.rs`). Committed at `007d3cf`. 235 tests pass, clippy
and fmt clean. **3 local commits not yet pushed to `origin/mdube_run_output`** — pushing needs its
own explicit instruction.

A large side-quest this session built `.claude/work/`'s new home: a dedicated, sparse `main`
worktree at `../GraphEvolutionTool-docs`, plus a setup script and a two-folder VS Code workspace.
It's PR #70 on `mdube_docs_worktree`, **open, not merged** — unrelated to #21's own code but
touched every session from here on, since `/save`/`/park`/`/load`/`/done` all use it now.

**Start here:** task 4 — `seed`, `run_index` and the generating config TOML on `RunResult`
(`get/src/lib.rs`, `get/src/dispatch.rs`, `get/src/py_result.rs`). Run-level fields, not per-row;
`run_index` ships as a hard `0` until #20. Verify with a Rust test reading `result.seed` and
`result.run_index` after a run, and that the TOML round-trips through `Config::from_toml_str`.

**Watch out for:**

- **`.claude/work/` is read/written from `$DOCS_WT` (`../GraphEvolutionTool-docs`) now, never from
  whatever branch this tree has checked out.** Every skill handles this itself — `cd`s there first —
  so you don't need to think about it, but if you ever see `.claude/work/mdube/current/` missing or
  empty in *this* tree, that's expected: it only exists in the worktree now, not here.
- **`cargo test` needs Python on `PATH`** — `traps.md`,
  `cargo-test-cannot-link-python-unless-extension-module-is-off`.
- **The two denominators differ on purpose.** `std_dev` divides by `n`, `ci_95` by `n-1`. Already
  shipped correctly; don't "fix" it if you're back in this code.
- **Shipped source must not reference `official_spec_sheet.md` or issue numbers** (amended
  2026-08-13).
- **Do not edit `documentation/`.** File what the site now gets wrong in
  `documentation/mdube_edits.md` — `collab.md` #53.
- **`collab.md` #59, filed this session, not fixed:** `## Open`/`## Settled` stopped bounding
  anything around item `#48`, and `### 48` is used twice. Don't be surprised the file looks
  disorganized past that point — it's known, and flagged for the next joint meeting, not a sign
  something else broke.

**Two open questions in `plan.md`, neither blocking yet:** `run_index` as a hard `0` (planned: yes),
provenance TOML naming (planned: derived from `save_results`'s path).

**⏰ Time-sensitive:** PR #70 needs James's setup and review — `collab.md` #58 (3 addenda + a
summary) has his one-time command. `collab.md` #59 (the reorg) and the still-meeting-bound items it
lists (`#51`, `#52`, `#56`, `#42`, the first `#48`, `#49`) are all candidates for the next joint
meeting.
