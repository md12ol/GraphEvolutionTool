# Next session — 2026-08-06

Read `.claude/work/current/plan.md` and `.claude/work/decisions.md` first, then `.claude/work/hotfixes.md`.

**Where things stand:** Issue #22 is fully done and archived. PR #43 (`mdube_format_and_readability`
→ `main`) is open, assigned to James, `Closes #22` — awaiting his review and merge. Nothing else is
outstanding on this task.

**Start here:** There is no active task. Run `/start` for the next piece of work.

**Watch out for:**
- `traps.md`: "A PR can merge mid-session, and `/save`'s git manifest will not notice" and "GitHub's
  PR object lags the branch" — both apply once James reviews #43. Check the PR's actual merged state
  with `gh api` rather than assuming from local `git status`.
- `collab.md` #27 is still open (unanswered by James): `Swap`'s degree floor is `> 2` here but was
  `>= 2` in the 2019 Java predecessor. Not blocking anything, just sitting there.
- The `gh` CLI is not on `PATH` on this machine in either shell — use the full path
  `C:\Program Files\GitHub CLI\gh.exe` (see `traps.md`).

**⏰ Time-sensitive:** None.
