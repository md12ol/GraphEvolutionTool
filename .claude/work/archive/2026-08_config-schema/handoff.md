# Next session — 2026-08-05

Read `.claude/work/current/plan.md` and `.claude/work/decisions.md` first, then
`.claude/work/hotfixes.md`.

**Where things stand:** This task is **finished and archived** — GitHub #24 shipped as PR #42,
merged as `988457e`, issue closed 2026-08-05T22:17:43Z. `work/current/` was archived to
`.claude/work/archive/2026-08_config-schema/`, so this file is a record, not a live instruction. The
previous session's machine crashed before its `/save`, and the archived `history.md` is
reconstructed from primary sources — it flags this at the top.

**Start here:** `/start` a new task. There is no in-flight work to resume.

**Watch out for:**

- **Two `collab.md` items are waiting on Michael and belong to *other* issues, not this one** —
  **#24** (what a `Profile*.dat` contains: the patient-zero prepend and the `verts / 128` rescale,
  needed before #26 turns a path into a `Vec<f64>`) and **#25** (a stray `seed` under `[fitness]`
  parses silently; the check moves to #23's `Config::validate`). Neither blocked closing #24.
- **Two `collab.md` items are waiting on *James*** — **#27** (`Swap`'s degree floor is `> 2` in the
  spec and code but `>= 2` in the original Java) and **#30** (Michael's new `SessionStart` hook,
  `.claude/hooks/pull_main.sh`, explicitly asks for review). #30 already merged as PR #44 and runs
  on every session start.
- **`pull_main.sh` now fast-forwards `main` automatically** at session start, but only when the
  current branch is `main` and the fast-forward is clean. On a feature branch it does nothing.
- **The SIR-batch-seed hotfix is still in the tree**, load-bearing and blocked on #18. Fourth cycle.
  Verified present 2026-08-05 at `get/src/fitness.rs:162-164`; #18 still `open`.
- **A stale empty stash** sits at `stash@{0}: WIP on main: 95a8bd0` — `git stash show -p` returns
  nothing. Left in place deliberately rather than dropped without being asked.

**⏰ Time-sensitive:** nothing dated is outstanding.
