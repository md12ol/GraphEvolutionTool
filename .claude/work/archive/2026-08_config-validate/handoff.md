# Next session — 2026-08-06

Read `.claude/work/current/plan.md` and `.claude/work/decisions.md` first, then
`.claude/work/hotfixes.md`.

**Where things stand:** This task is **finished and archived**. GitHub #23 shipped as **PR #45**
(`5fd8dbc` + `2c590f4`), pushed and open, awaiting Michael's review — archived with the PR still
open, on the disposition recorded in `decisions.md` 2026-08-05 15:09, because the body's
`Closes #23.` was verified on the remote and the item owes this side nothing. `work/current/` was
archived to `.claude/work/archive/2026-08_config-validate/`, so this file is a record, not a live
instruction.

**Start here:** `/start` a new task. There is no in-flight work to resume.

**Watch out for:**

- **PR #45 is unmerged.** If review asks for changes, the branch `jsargant_config_validate` is
  still there and still tracks its remote — reopen the work rather than starting from `main`.
- **It supersedes a test #24 wrote.** `an_unknown_fitness_key_is_ignored_rather_than_rejected` is
  deliberately gone. Called out in the PR body; if a reviewer flags it as a regression, the answer
  is `decisions.md` 2026-08-06 00:07.
- **`collab.md` #27 is still waiting on James** — `Swap`'s degree floor. Carried forward untouched
  at this gate by explicit choice. The code already matches spec §3.1, so loosening it to the
  Java's `>= 2` needs a joint meeting; keeping `> 2` needs only a `decisions.md` entry.
- **`collab.md` #24 awaits Michael** (the `Profile*.dat` format, needed before #26 can turn a path
  into a `Vec<f64>`); **#25 is answered** and needs only his acknowledgement.
- **The SIR-batch-seed hotfix is still in the tree**, load-bearing, blocked on #18. Fifth cycle.
- **Switching branches moves the `.claude/` docs** — `decisions.md`, `traps.md`, `hotfixes.md`,
  `issues.md` and `collab.md` are tracked, so they differ per branch while `work/current/` does
  not. This task wrote docs on `main` and code on the branch deliberately; check
  `git branch --show-current` before appending to any of them.

**⏰ Time-sensitive:** nothing dated is outstanding.
