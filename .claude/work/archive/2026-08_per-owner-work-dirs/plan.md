# Plan — Per-owner work directories, and a `/park` skill for blocked tasks
_Started 2026-08-13 · last updated 2026-08-13_

## Objective

A task blocked on the other owner can be **parked** instead of held in `work/current/`, so a second
task starts without losing the first one's plan, history and handoff. Live task directories become
**per-owner and tracked**, so the same owner can pick a task up on another machine.

Done looks like: `.claude/work/<owner>/current/` and `.claude/work/<owner>/parked/<slug>/` exist and
are tracked; `/park <slug>` moves `current/` into `parked/`; `/load [slug]` resolves which task to
open, unparks it, and stops on cross-machine divergence rather than merging a plan file; `/save`
commits and pushes `work/<owner>/` as its last step.

**Out of scope:** any change to `get/src/`, the archive layout (`work/archive/` stays shared and
un-namespaced), and the persistent docs (`decisions.md`, `collab.md`, `traps.md`, …) which stay
directly under `work/`.

**Design settled 2026-08-13** with Michael, in seven answered questions — layout, owner routing,
visibility, archive, push policy, divergence handling and the machine stamp. `/save` records it in
`decisions.md`; do not re-litigate it here.

## Tasks

- [x] Bootstrap: park the `result-object` task by hand, since `/park` does not exist yet. Its three
      files are in `.claude/work/mdube/parked/result-object/`, handoff stamped with the machine and
      what it is blocked on. `work/current/` now holds this task.

- [x] Branch `mdube_per_owner_work_dirs` created off `main`.

- [x] `.gitignore` no longer ignores `.claude/work/current/`; the comment block now explains why the
      per-owner directories are tracked. Verified: `git check-ignore` exits 1 on the new path.

- [x] Union globs confirmed not to reach the new paths — `git check-attr merge` prints
      `merge: unspecified` for both a `current/` and a `parked/<slug>/` file.

- [x] `hooks/session_brief.sh` resolves the owner from `git config user.email`, reads
      `work/<owner>/current/`, lists parked tasks with their `Blocked on:` lines, and counts the
      other owner's. Verified by running it on all three paths (active, empty, unrecognised email).
      Also fixed a pre-existing `grep -c` bug that split the counts line in two.

- [x] New skill `.claude/skills/park/SKILL.md` — save, stamp `Blocked on:` and `Machine:`, `git mv`
      into `parked/<slug>/`, push. Refuses an empty `current/` and a slug already taken.

- [x] `/load` takes an optional slug, with the full resolution order, the unpark-is-a-swap rule and
      the cross-machine divergence check that stops rather than merging.

- [x] `/save` writes the `Machine:` stamp and gained step 10 — commit and push `work/<owner>/`,
      scoped to that path only, with the carve-out stated where the old constraint was.

- [x] `/start` and `/done` read the per-owner path; `/done` refuses a parked task and keeps the
      shared archive. Verified: `grep -rn "work/current" .claude/skills/` is clean but for `setup`.

- [x] `.claude/CLAUDE.md` and `.claude/README.md` — working-docs table, archive note, a new routing
      row, the push carve-out, `/park` in the loop. Verified: no `work/current/` references left
      beyond the one line that documents the change.

- [x] `collab.md` #55 raised for James. Audited: `uniq -d` clean, 43 `### <n>` headings at column 0.

- [x] This task's own directory moved to `.claude/work/mdube/current/`; `work/current/` is gone and
      the hook reads the new location.

- [x] Swap tested end to end, against a backup: park-then-unpark round trip is **lossless** (md5 of
      all five files identical before and after), and the two-parked case lists both slugs with
      their blockers instead of guessing. Backups at `~/.claude-backups/.../2026-08-13` and in the
      scratchpad.

- [x] Two bugs found by that test, both fixed: the hook's "Start here" extraction matched only a
      `## Start here` heading and so had **never** fired against a real handoff, and `/park`'s
      `Machine:` stamp used a shape the hook's grep could not see. Both verified by re-running.

- [x] `/setup` no longer recommends ignoring the live task directory, and explains why the path
      rather than the ignore is the fix. Verified: no stale `work/current/` references left in it.

- [x] Committed in seven reviewable steps and **PR #69 opened**, body verified via `--json`.
      `collab.md` #55 went direct to `main` (`9d097eb`) ahead of it, as the notification.
      Routing reasoning in `decisions.md` 2026-08-13 03:09.

- [x] **Waiting on James** — PR #69 merged 2026-08-13T00:53 (confirmed via `gh pr list --state
      all`). `collab.md` #55 remains unanswered; Michael decided 2026-08-13 not to hold this task on
      it, same reasoning as `result-object`'s close (`decisions.md` 2026-08-13). Carries forward as
      a standing collab item.

- [x] **Exercised `/park` and `/load <slug>` for real** — `/park run-output` (2026-08-13, this
      session) ran the skill proper, writing `Machine:`/`Blocked on:` via its own step 3; `/load
      result-object` and `/load per-owner-work-dirs` (same session) both drove the real unpark path.
      No hand-run file moves.

- [x] Open the PR for the hook and the new skill's frontmatter — already satisfied: PR #69's file
      list includes `hooks/session_brief.sh` and `skills/park/SKILL.md`, merged.

## Open questions

- None blocking. The seven design questions were answered 2026-08-13 before this plan was written.

## Out of scope

- `get/src/` — untouched by this task.
- The `result-object` task itself, parked at `work/mdube/parked/result-object/` and blocked on
  James. Resume it with `/load result-object` once PR #65 and #66 merge.
- A third owner. The layout admits one by adding a directory; nothing here hardcodes two.
