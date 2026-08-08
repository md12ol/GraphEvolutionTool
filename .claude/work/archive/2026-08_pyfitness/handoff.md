# Next session — 2026-08-08

Read `.claude/work/current/plan.md` and `.claude/work/decisions.md` first, then
`.claude/work/hotfixes.md`.

**Where things stand:** All eight tasks on **#19** are `[x]`. **PR #48** is open against `main`
(`jsargant_pyfitness` at `b1f8557`), `Closes #19.`, awaiting Michael — nobody merges their own.
`main` is at `90b624b`, carrying `collab.md` #37 (a heads-up to Michael that this PR changes how
`cargo test` builds, verified on Linux only) and two `traps.md` entries.

**Start here:** there is no code task left on #19. Either wait for review, or run `/done
pyfitness` to archive — the established pattern (`collab.md` #28, and #15/#23/#24 before it)
archives with a code PR still open, since the docs record and the code review are independent
tracks. If you `/done` before the merge, the `#[allow(dead_code)]` hotfix and `collab.md` #37 both
carry forward as-is; nothing about them changes until #26 lands or Michael replies.

**Then, in priority order:**
1. If Michael requests changes on #48, that's a separate branch/session — don't fold fixes into a
   `/done`'d task's record.
2. When #48 merges: delete the `#[allow(dead_code)]` note is **not** yours to act on — that's #26's
   job, triggered by #26 calling `python_fitness`. Nothing to do here at merge time except note it
   merged.

**Watch out for:**

- **`.claude/work/*.md` currently disagrees between `jsargant_pyfitness` and `main`, on purpose.**
  The branch has `decisions.md`/`hotfixes.md`/`issues.md` content `main` lacks; `main` has
  `collab.md`/`traps.md` content the branch lacks. Both reconcile cleanly on merge (union or
  no-conflict) — re-run `grep -vE '^\s*$' .claude/work/<file>.md | sort | uniq -d` on each after it
  happens, don't assume.
- **`cargo test` needs `LD_LIBRARY_PATH` set, every session, this machine:**
  `export LD_LIBRARY_PATH="$(python3 -c 'import sysconfig; print(sysconfig.get_config_var("LIBDIR"))'):$LD_LIBRARY_PATH"`
- **`#[allow(dead_code)]` on `GraphEvolver::python_fitness`** — load-bearing until #26 exists.
  `hotfixes.md`, `Remove when: #26 lands and calls it`. Not yours to remove.
- **No `[~]` items.**

**⏰ Time-sensitive:** nothing dated. `collab.md` #35/#36/#37 all await Michael; #27 still waits on
you, sixth gate now.
