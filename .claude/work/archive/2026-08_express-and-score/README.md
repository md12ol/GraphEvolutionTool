# express-and-score — GitHub #14

**Objective.** Rename `common::evaluate` to `common::express_and_score` and make it the engine's
only path from a population to a set of fitnesses, with the invariant stated where someone would
break it: **the engine never calls `Fitness::evaluate` or `Fitness::evaluate_population` directly.**
Spec §5.1.

**Dates.** Single session, 2026-08-04. Opened and closed the same day.

**Outcome.** Shipped as **PR #38** (`9c397eb`), merged 2026-08-04 20:15 UTC as `168cc91`; issue #14
closed as completed. Every plan item is `[x]`.

The invariant **already held** — checked before starting, there were no violations to fix — so this
landed as a rename plus documentation and **no behaviour changed**: 110 tests before and after,
which is the verification that matters for a behaviour-preserving rename. That was stated plainly in
the PR body so no reviewer would hunt for a bug that was never there. Reasoning in `decisions.md`
2026-08-04 16:42.

Also decided here: **#14 and #15 are two tasks, not one** (`decisions.md` 2026-08-04 16:40). The
initial recommendation to combine them was wrong — it rested on file overlap, which only matters for
*concurrent* work, and it would have mixed a mechanical rename with a semantic change in one diff,
where the rename noise hides the behaviour change from a reviewer.

**Housekeeping absorbed by this task**, none of it in the original plan:

- **PR #33 merged** (`0729019`) — Michael's transcription of the 2026-08-04 joint meeting, reviewed
  at his request and merged locally with `--no-ff`.
- **The previous task's close-out was recovered** (`687bc7d`). `archive/2026-08_mutation-contract/`
  was untracked and its `decisions.md` / `traps.md` appends unstaged on a stale branch — none of it
  had reached Michael.
- **Two union-merge splices on `main`, both repaired by hand** (`2f8fc62`, `f652df1`).

## Left behind, deliberately — all carried forward

| | Where it lives | State at close |
|---|---|---|
| **Hotfix: SIR batch seed never changes between evaluations** | `hotfixes.md` | Michael's, at `get/src/fitness.rs:158`. Both passes verified at the gate: code still present, and `Remove when:` **not met** — GitHub #18 still open. Committed to `main` via PR #40, so it is in every tree |
| **`collab.md` item 20** | `collab.md` | Open with Michael — one stale line in `CLAUDE.md`. Blocks nothing |
| **`collab.md` item 23** | `collab.md` | Raised at this gate. `uniq -d` cannot detect a splice, yet `CLAUDE.md` and `collab.md`'s header both still present it as sufficient |
| **Untracked stale docs** | `traps.md` | `docs/` and `GET GA planning session.md` remain untracked in James's tree and must never be staged |

Nothing was left `Filed: not yet` — `issues.md` was empty at close.

## The one thing worth carrying into every later task

**`uniq -d` returned clean on a genuinely corrupted `collab.md`.** Union merge spliced one entry into
the middle of a line of another; a splice repeats no line, so the documented audit is structurally
blind to it. Check heading structure as well —
`grep -n '^### [0-9]' .claude/work/collab.md`. Full mechanism in `traps.md` as
`union-merge-splices-entries-without-duplicating`; the two places that still send people to the
insufficient command are what `collab.md` item 23 asks about.

**Next task:** GitHub **#15**, convert fitness direction only at the Python boundary. Tier (1),
James's, branches off `main`.
