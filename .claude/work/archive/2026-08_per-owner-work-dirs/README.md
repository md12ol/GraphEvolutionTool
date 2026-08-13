# Archive — per-owner work directories, and a `/park` skill for blocked tasks

**Objective:** a task blocked on the other owner can be parked instead of held in `work/current/`,
so a second task starts without losing the first one's plan, history and handoff. Live task
directories became per-owner and tracked, so the same owner can pick a task up on another machine.

**Spans:** 2026-08-13 (single day, two sessions).

**Outcome.** Shipped as PR #69 (seven commits on `mdube_per_owner_work_dirs`), merged. New layout:
`.claude/work/<owner>/current/` and `.claude/work/<owner>/parked/<slug>/`, both tracked; new skill
`.claude/skills/park/SKILL.md`; `/load` gained an optional slug, the unpark-is-a-swap rule and a
cross-machine divergence check; `/save` gained the `Machine:` stamp and a step that commits and
pushes `work/<owner>/` as its last step; `/start` and `/done` updated to match, and `/done` refuses
a parked task. `hooks/session_brief.sh` resolves the owner from `git config user.email` and lists
parked tasks with their blockers — two pre-existing bugs in it were found and fixed along the way.
`collab.md` #55 raised for James as the notification.

**Verified for real, not just by file-move testing.** This closing session drove `/park` and
`/load <slug>` as the actual skills — `/park run-output`, `/load result-object`, `/load
per-owner-work-dirs` — confirming the design round-trips correctly end to end, including `/park`'s
`/save`-first step writing the `Machine:`/`Blocked on:` stamp.

**Closed with one loose end still open, deliberately not this task's business going forward:**
`collab.md` #55 is still unanswered by James as of the close — see `decisions.md` 2026-08-13,
"closed `per-owner-work-dirs` with `collab.md` #55 still unanswered". It remains a standing
`collab.md` **Open** item.

**Nothing carried forward to hotfixes.md or issues.md** — this task added neither.
