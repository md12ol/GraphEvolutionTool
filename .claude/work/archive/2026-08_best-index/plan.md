# Plan — Extract the argmin in both evolvers' `outcome` into `common::best_index`
_Started 2026-08-10 · last updated 2026-08-10_

## Objective
GitHub #51. `GenerationalEvolver::outcome` (`get/src/evolver/generational.rs:124-129`) and
`SteadyStateEvolver::outcome` (`get/src/evolver/steady_state.rs:131-133`) both find the best index
in `fitnesses` using `rank`, but in two different styles — an explicit loop vs. `min_by` +
`expect`. `CLAUDE.md`'s "prefer explicit loops to iterator chains" asks for the loop form. Done
means both call one shared `common::best_index`, written as the explicit loop.

Full rationale, and what NOT to do (do not extract the rest of `outcome` — the two methods diverge
beyond the argmin), is in `decisions.md`'s "(1) Extract the argmin..." entry — this plan just turns
it into tasks. Scoped and agreed at the joint meeting of 2026-08-09; raised as `collab.md` #36.

**Out of scope:** any change to how the winner's `Graph` is obtained (swap_remove vs. re-expression)
— that's spec §6.2 behaviour, not this issue.

## Tasks
- [x] `common::best_index` added at `common.rs:39` (explicit loop, `assert!` on empty), both
      `outcome` methods call it, unused `Ordering`/`rank` imports dropped. Verified:
      `cargo test -p get` 213/213, `cargo clippy --all-targets -- -D warnings` clean.
- [x] Committed as `79c10aa`; PR #55 **merged by James** at 2026-08-10 20:52 (`9274f38` on `main`),
      issue #51 auto-closed. Not a self-merge — the review the PR rule exists for did happen.
- [x] Follow-up sweep filed as GitHub **#56**, body round-trips byte-identical via
      `gh issue view 56 --json`; 0 labels, 0 assignees. Raised as `collab.md` **#43** (FYI + open
      assignment + the meeting question); both union-merge audits clean, 30 headings at column 0.
- [x] `collab.md` #43 pushed to `main` in this task's close-out commit, alongside the archive —
      verified by `git log origin/main -1 --stat` listing `.claude/work/collab.md`.

## Open questions
None.

## Out of scope
- Merging the rest of `outcome`'s body between the two evolvers — decisions.md explains why it
  reads worse (diverging signatures, struct-building).
- How the winner's `Graph` is obtained — spec §6.2, unchanged.
