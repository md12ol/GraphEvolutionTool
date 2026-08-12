# History — GitHub #58: reject `target_profile` when the objective is not `epi_prof_match`

Append-only session log for this task, newest session first.
Maintained by `/save`; archived by `/done`.

---

## Session 2026-08-12: PR #63 confirmed merged; task closed via `/done`

**Ran `/start` first, correctly redirected.** `work/current/` was non-empty (this task, task 5
still open); `/start` stopped per its own rule rather than overwriting it.

**Verified task 5.** `gh pr view 63 --json state,mergedAt,mergeCommit` — `MERGED`, merge commit
`b225f3048773a15dc0e7b0324cfa71dbb2e9f883`. `gh issue list --state open` no longer lists #58.
`git merge-base --is-ancestor b225f30... origin/main` confirmed the merge commit is on `main`.
`plan.md` task 5 ticked `[x]` with that evidence.

**Sweep found one stale plan line.** Task 5's "Open questions" said `config.example.toml` would
say nothing about the rejection — true when written, false by the time task 4 landed (commit
`d7cb289` added `config.example.toml:91-94`). Corrected in `plan.md` rather than left misleading.

**Sweep found one unrelated hotfix whose condition was now met.** The `#[allow(dead_code)]` on
`GraphEvolver::python_fitness` (`hotfixes.md`) was waiting on GitHub #26. #26 closed on `main` via
`db8c71a`/`fcf06c6`, unrelated to this task — Michael's work, landed after this branch's last
`/save`. Verified by reading `origin/main:get/src/lib.rs` and `dispatch.rs` directly: the
attribute is gone, `python_fitness` now lives at `dispatch.rs:160`. Entry removed.

**`collab.md` #45 closed the loop.** Appended a short note (not an edit — appended after the
existing reply) recording that #58 shipped, so the item doesn't read as still-open next time
someone scans headings.

**Git manifest — `GraphEvolutionTool`, the only repo**

- Still on `jsargant_reject_stray_target_profile` locally; `origin/main` is ahead (`df9dbbd`),
  carrying `b225f30` (our merge) plus unrelated work (#26 close-out, `/done` skill's branch-cleanup
  step, #61 stale-run-doc). Nothing to pull into *this* branch — it's finished.
- `.claude/work/` edits this session, uncommitted: `current/plan.md`, `current/history.md`,
  `current/handoff.md`, `decisions.md`, `hotfixes.md`, `collab.md`.
- Untracked and **not touched**, pre-existing: `GET GA planning session.md`, `docs/`
  (`COMPLEXITY_REVIEW_HANDOFF.md`, `IMPLEMENTATION.md`, `PR_DRAFT.md`) — see `traps.md`,
  `untracked-pre-spec-sheet-docs-git-add-all`.

*Session logged 2026-08-12 14:15 — James.*

## Session 2026-08-11: #58 built, verified and shipped as PR #63 in one sitting

**Started and finished the code.** `/start` wrote the plan, tasks 0–4 all closed the same session.

**What changed**

- `get/src/config.rs:262` — `reject_fitness_seed` → `reject_stray_fitness_keys`, one raw-text pass
  over both `[fitness]` keys the `#[serde(flatten)]` on `SirParams` hides. New branch at
  `config.rs:308-330` rejects `target_profile` when `type` is present, reads as a string, and is not
  `epi_prof_match`. Doc comments on it and on `from_toml_str` rewritten.
- `get/src/config.rs:840` and `:872` — two tests, four cases. The three rejections share one `for`
  loop; the acceptance case checks `epi_prof_match` both parses **and** validates.
- `get/src/config.rs:889` — narrowness test renamed
  `an_unknown_fitness_key_outside_the_two_named_ones_is_still_ignored`. **Body untouched**, so what
  it pins is unchanged.
- `get/src/py_config.rs:402` and `:991` — two comments naming the old function name.
- `config.example.toml:87-94, 118-121` — documents the new rejection in both places a reader meets
  the profile, and names the narrowness.
- `get/src/config.rs:7` — module doc now says `dispatch.rs`, not `lib.rs`.

**Validated on Linux, not on Windows**

- `cargo test -p get` — **233 passed, 0 failed**. Baseline on `main` is **231**, measured by
  `git stash`ing the working tree and re-running, not assumed.
- `cargo clippy -p get --all-targets -- -D warnings` — exit 0.
- `cargo fmt -p get --check` — exit 0. `main` was already fmt-clean at 97d9e02, checked before
  starting, so the gate was meaningful rather than inherited-dirty.
- `cargo test` needs `LD_LIBRARY_PATH` here — hit the documented `libpython3.11.so.1.0` failure
  (exit 127) on the first run and used the `traps.md` incantation.

**One clippy round trip.** The first draft nested `if fitness.get("target_profile").is_some()`
around `if let Some(toml::Value::String(objective))` around `if objective != "epi_prof_match"`.
`collapsible_if` failed the gate. Reasoning for the let-chain that replaced it, and the flat
alternatives rejected, in `decisions.md` 2026-08-11 20:15.

**Two things the sweep caught that the issue did not mention**

1. `config.example.toml:87-89` claimed a leftover `[fitness] seed` "is silently ignored rather than
   reported" — false since #25 landed 2026-08-05. Found because the new paragraph goes directly
   below it. Corrected in the same PR; raised in `collab.md` #47's reply rather than left in a diff.
2. `config.rs:7`'s stale `lib.rs` reference, which Michael had explicitly assigned to #58's diff in
   `collab.md` #47 and in `decisions.md` 2026-08-11 11:26. **Missed on the first pass** and only
   found by the save's step-2 sweep reading the decisions tail — after PR #63 was already open. Took
   it as a third commit on the open PR, per the user's call.

**Git manifest — `GraphEvolutionTool`, the only repo**

- Branch `jsargant_reject_stray_target_profile`, **pushed**, nothing uncommitted in `get/` or
  `config.example.toml`.
- Three commits, all on the remote: `bfa515b` (check + tests), `d7cb289` (example docs),
  `7fc4c1a` (module doc line).
- **PR #63** open against `main`, body patched via the REST API after the third commit and re-read
  with `gh pr view 63 --json` — 6 sections, 3 commits, intact. Awaiting Michael.
- Untracked and **not touched this session**, pre-existing at session start:
  `GET GA planning session.md`, `docs/` (`COMPLEXITY_REVIEW_HANDOFF.md`, `IMPLEMENTATION.md`,
  `PR_DRAFT.md`).
- `.claude/work/` edits from this save are uncommitted: `current/plan.md`, `current/history.md`,
  `current/handoff.md`, `decisions.md`, `collab.md`.

*Session logged 2026-08-11 20:25 — James.*
