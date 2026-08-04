# Next session — 2026-08-04

Read `.claude/work/current/plan.md` and `.claude/work/decisions.md` first, then
`.claude/work/traps.md` — two new entries there change how you merge.

**Where things stand:** issue #16 is done and closed; `sir_sim` is on `main`. James's #10 is done
and closed too — his PR #30 was reviewed and merged locally at `79f7948`. `main` is clean, nothing
ahead of origin, **110 tests passing**. Everything left is waiting on the joint meeting, not on work.

**Start here:** run `/done sir-sim`. The task is finished — its two remaining plan items are a
decision deferred to #17 and a post-meeting follow-up, neither of which is work to do now.

**Do not start #17 yet.** It is the obvious next issue and it is blocked in a way that is easy to
miss: `epi_length` reads `SirRun::length` and `epi_prof_match` computes RMSE over
`SirRun::profile`, and **both conventions are contested** (`collab.md` #15). `spread` is safe —
both implementations agree. Starting #17 first means building on numbers expected to shift.

**Watch out for:**

- **Merge `.claude/` PRs locally, never with the GitHub button.** `.gitattributes` merge drivers do
  not run on GitHub's servers, so `merge=union` is simply absent there. `traps.md` has the
  measurement and the command.
- **Union duplicates concurrently-edited lines** as silently as it dedups identical ones. Append;
  do not edit an existing entry in place. Audit after every merge:
  `grep -vE '^\s*$' .claude/work/<file>.md | sort | uniq -d`.
- **One unfiled issue is load-bearing.** `issues.md` stages "Align `sir_sim`'s length and profile
  with whatever the meeting decides", assigned to Michael, held until `collab.md` #15 settles. It is
  the only thing protecting the decision to merge #31 before that argument was resolved.
- **Code goes through a feature branch and a PR** — `get/src/`, `Cargo.toml`,
  `config.example.toml`. `.claude/work/*.md` may be pushed direct. `CLAUDE.md`, "Pull requests".
- **`gh issue view` and `gh pr edit` are broken here** — Projects-classic GraphQL error across the
  whole default view. Reads `--json`; writes `gh api ... -X PATCH -F body=@file.md`.
- **Re-check remote state before writing any doc about a PR.** This bit once today: a save described
  PR #31 as open 2.5 hours after it had merged.
- Do not run bare `cargo fmt` — still true, still #22.

**⏰ Time-sensitive:** five `collab.md` items are queued for the meeting — #15 (length/trailing
zero), #16 (`dyn` vs match for fitness dispatch), #17 (short-epidemic re-rolls), #18 (FYI trace, no
action), #19 (union findings; routing half already settled). **#16 is the one with a real deadline:**
cheap now, a rewrite of the whole dispatch layer once #26 builds the 16-arm match.

Also for the meeting: `CLAUDE.md`'s "an agent never merges a PR at all" was overridden twice in one
day, both times correctly. It should read "never merges unprompted".
