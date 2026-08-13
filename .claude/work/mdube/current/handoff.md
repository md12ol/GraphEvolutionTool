# Next session — 2026-08-13

**Machine:** `MDUBE-Lenovo` · saved 2026-08-13 02:25 · 76bd7b9

Read this task's `plan.md` and `.claude/work/decisions.md` first, then `.claude/work/traps.md` —
two new entries there are about tooling you are likely to touch.

**Where things stand:** the per-owner work-directory change is **complete and in PR #69**, seven
commits on `mdube_per_owner_work_dirs`, unmerged. The layout is already live on this machine:
`.claude/work/mdube/current/` holds this task, `.claude/work/mdube/parked/result-object/` holds the
one blocked on James, and `work/current/` no longer exists. `collab.md` #55 is on `main` by itself
as the notification. Nothing is uncommitted except this save.

**Start here:** nothing in this task is actionable — PR #69 needs James's merge and `collab.md` #55
needs his reply. **Start a new task with `/start`, or resume the parked one if he has moved.** Check
first, in one command:

    gh pr list --state open

If #65 and #66 are gone, `result-object` is unblocked → `/load result-object` and finish it. If #69
is gone, this task is done → `/done per-owner-work-dirs`.

**Then, in priority order:**

1. Open and unblocked issues, once `result-object` closes: **#21** (5), then **#20** (6) and
   **#28** (6). #56 is (7); #67 and #68 are (8) and deliberately last.
2. The one open verification on this task: drive `/park` and `/load <slug>` as **skills**, not by
   hand. The file moves round-tripped losslessly, but nothing has yet exercised `/park`'s
   `/save`-first step, which is what writes the `Blocked on:` stamp the design depends on.

**Watch out for:**

- **`/save` now pushes on its own** — `.claude/work/mdube/` only, as its last step. That is a
  deliberate carve-out to the "don't commit or push unless asked" rule and it does **not** widen:
  code, the persistent docs and the sheet each still need their own instruction, every time.
- **`decisions.md` and `traps.md` go direct to `main`, not into PR #69.** They are appended in the
  working tree right now. Same rule `/done` follows while a code PR is open.
- **This session's task directory lives on the branch, not on `main`.** If you switch to `main`
  before #69 merges, `work/mdube/` disappears from the tree — that is the branch, not data loss.
- **`cargo test` needs Python on `PATH` on this machine**, and `python3` does not exist here — call
  the interpreter by full path. Both are in `traps.md`.
- **Do not edit `documentation/` during an ordinary task.** File it in `documentation/mdube_edits.md`
  instead. Both queues are empty.

**⏰ Time-sensitive:** nothing dated. `collab.md` #50, #51, #52, #53, #54 and now #55 all await
James; #51 wants the joint meeting Michael mentioned for "tomorrow" on 2026-08-13.
