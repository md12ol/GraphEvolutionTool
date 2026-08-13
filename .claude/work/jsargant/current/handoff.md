# Next session — 2026-08-13

**Machine:** `pop-os` · saved 2026-08-13 17:18 · `8cba899`

Read this task's `plan.md` and `.claude/work/decisions.md` first, then `.claude/work/hotfixes.md`.

**Where things stand:** GitHub #20 is **code-complete**. All 8 plan items are `[x]`. PR #83 is open
against `main`, `mergeable=MERGEABLE`, review requested from Michael, and carries `get/src/lib.rs`,
`get/src/dispatch.rs` and `documentation/jsargant_edits.md` — no `.claude/work/` diff. The branch is
pushed and level with `origin` at `8cba899`; the working tree is clean. There is no code left to
write.

**Start here:** check whether PR #83 has merged — `gh pr view 83 --json state,mergedAt` (the plain
`gh pr view` is broken on this repo). If it has merged, run `/done` and close GitHub #20. If it has
not, there is nothing to do on this task — either park it (`/park replicate-runs`, matching how
`set-base-graph`/#28 was parked while waiting on PR #72) or leave it live and work on something
else, your call.

**Watch out for:**
- **PR #83 conflicts with PR #72 (also open) in `dispatch.rs` and `documentation/jsargant_edits.md`.**
  #72 adds a `base_graph: Option<&Graph>` parameter to `dispatch::evolve`; #83's `run_replicates`
  calls `evolve` twice at `dispatch.rs:442` and `:460` (line numbers as of `8cba899`, will drift once
  either merges). Whichever merges second needs those two calls updated to pass the base graph
  through. `jsargant_edits.md`'s "Nothing pending" placeholder is replaced differently by each
  branch — resolution is keeping all four queue entries, not picking one side.
- **Do not edit `.claude/work/` on the code branch.** It lives in the `../GraphEvolutionTool-docs`
  worktree. Editing it on the branch is what puts a `.claude/work/` diff back into a PR.
- `cargo test -p get` needs `LD_LIBRARY_PATH` exported first — `traps.md`,
  `cargo-test-cannot-link-python-unless-extension-module-is-off`.
- `.venv/` exists and is gitignored; `maturin develop --release` rebuilds it if you need to re-run
  anything through real Python.
- `documentation/jsargant_edits.md`'s `#replicate-runs-ship` entry flags (without filing) that
  `reproducibility.html:234` still badges something #71 already shipped — that one belongs in
  Michael's `mdube_edits.md`, not this task's to fix.

**⏰ Time-sensitive:** none. Blocked on a review with no deadline.
