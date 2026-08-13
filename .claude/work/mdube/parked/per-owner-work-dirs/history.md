# History — Per-owner work directories, and a `/park` skill for blocked tasks

Append-only session log for this task, newest session first.
Maintained by `/save`; archived by `/done`.

---
## Session 2026-08-13: the layout shipped to PR #69, and testing it found two dead code paths

**What changed.** `.claude/work/current/` became `.claude/work/<owner>/current/`, with
`<owner>/parked/<slug>/` beside it, both tracked — the `.gitignore` line hiding the live task is
gone. New skill `.claude/skills/park/SKILL.md`. `load` gained an optional slug, the
unpark-is-a-swap rule and a cross-machine divergence check; `save` gained the `Machine:` stamp and
step 10, which commits and pushes `work/<owner>/`; `start` and `done` follow the new path, and
`done` refuses a parked task outright. `hooks/session_brief.sh` resolves the owner from
`git config user.email` and lists parked tasks with their blockers. `CLAUDE.md`, `README.md` and
`setup/SKILL.md` document all of it.

**Bootstrapped by hand**, because `/park` did not exist yet: the `result-object` task was moved to
`work/mdube/parked/result-object/` with its two queued tracker edits ticked first, and this task's
own directory sat at the old path until the skills could read the new one.

**Validated.** Park→unpark round trip is **lossless** — md5-identical across all five files, taken
against a `backup_docs.sh --force` snapshot plus a scratchpad copy. Two parked tasks with an empty
`current/` lists both slugs with their `Blocked on:` lines. A parked task with no handoff reports
`no blocker recorded`. An unrecognised `git config user.email` prints one line and exits 0.
`git check-attr merge` returns `unspecified` on both new paths. The committed `session_brief.sh`
blob has **no CR bytes**, so it runs on James's Linux machine whether or not PR #66 has landed.
**Not validated:** no session has driven `/park` or `/load <slug>` as skills — only the file moves
underneath them.

**Two pre-existing bugs found by testing, not by reading** — both in `session_brief.sh`, both
shipped since the file was written. `grep -c` prints `0` *and* exits 1, so `|| echo 0` appended a
second zero and split the counts line in two. And the "Start here" extraction matched a `## Start
here` heading, which no handoff has ever had — `/save`'s template writes `**Start here:**` inline —
so that block had never printed anything. Both are in `traps.md`.

**Git manifest.** Branch `mdube_per_owner_work_dirs`, 7 commits, pushed, **PR #69 open and
unmerged**. `main` carries `collab.md` #55 alone (`9d097eb`), pushed direct as the notification.
`decisions.md` (three entries) and `traps.md` (two entries) are appended in the working tree and go
direct to `main` under the routing table, not into the PR. No code outside `.claude/` was touched.

**UPDATE 2026-08-13, after the save:** parked. The next issue is being started on a branch cut from
`mdube_per_owner_work_dirs` rather than from `main`, on Michael's call — that branch is the only
place `work/mdube/` exists until #69 merges, so branching from it keeps the parked tasks visible and
the working layout intact. The cost, accepted knowingly: the new PR carries #69's commits until #69
lands, so **#69 must merge first**. Michael is telling James the order directly.

*Logged 2026-08-13 — Michael.*
