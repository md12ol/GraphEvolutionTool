# Next session — 2026-08-10

Read `.claude/work/current/plan.md` and `.claude/work/decisions.md` first, then `.claude/work/hotfixes.md`.

**Where things stand:** GitHub #53 is complete. All nine plan tasks are `[x]`. PR #57 merged by
Michael as part of `f25e33d`; issue #53 closed. `main` is at `9f4c3c3`, matching `origin/main`
exactly, clean apart from the two pre-spec-sheet untracked files. This save was run as `/done`'s
step 1 — if you are reading this, `/done` either has not yet run or was interrupted before
archiving; the correct next action is to run it, not to resume #53's work.

**Start here:** run `/done` (slug: `inline-target-profile` or similar — the objective is "replace
`target_profile_path` with an inline `target_profile` array"). The gate should find nothing
outstanding: no `[ ]`/`[~]` plan items, no unfiled issues, no new hotfixes, no untouched open
questions in `plan.md`.

**One thing to carry into the next `/start`, not into `/done`'s gate:** `collab.md` #44 has a
question still waiting on Michael — whether the new "practice-binding skill-body change → mandatory
`collab.md` item" rule gets written into `CLAUDE.md` now (direct push) or held for the next joint
meeting. Not blocking, not part of #53, and not something `/done` should try to resolve — just
don't lose it. Check `collab.md` #44 for his reply before assuming either way.

**The natural next task is GitHub #58**, assigned to James in `collab.md` #45: reject
`target_profile` supplied under a non-`epi_prof_match` objective, same flatten-can't-see-unknown-
keys mechanism as the stray `seed` check in #25. `reject_fitness_seed` in
`Config::from_toml_str` is the template — the fix is close to a copy of that shape, scanning for
`target_profile` instead of `seed` when the objective isn't `epi_prof_match`. Not started.

**Watch out for:** `main` still fails `cargo fmt -- --check` at `get/src/evolver/common.rs:45`
(`best_index`'s `assert!`, from #51) — pre-existing, staged then withdrawn from `issues.md` since
Michael filed the identical finding as a comment on GitHub #56 first. Not #53's to fix. The
`python_fitness` `#[allow(dead_code)]` hotfix (blocked on #26) and the `sda.rs` cargo-doc warning
(Parked, unfiled) are both unchanged and unrelated.

**⏰ Time-sensitive:** none.
