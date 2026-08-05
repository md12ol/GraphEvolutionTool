# History — Issue #15: convert fitness direction only at the boundary, not inside the engine

Append-only session log for this task, newest session first.
Maintained by `/save`; archived by `/done`.

---

## Session 2026-08-05: task closed with PR #41 still open; next task is #24

**No code was written this session.** `/load` verified the handoff against the repo and everything
in it held: still on `jsargant_direction_at_boundary`, tree clean apart from the two untracked
pre-spec-sheet documents, local `320fe68` identical to the remote ref, `origin/main` at `252347d`.

### PR #41, read from the remote on 2026-08-05

`state: open`, `merged: false`, `mergeable: true`. **Zero reviews, zero review comments, zero issue
comments** — Michael has not looked at it yet. Nothing has changed on it since it was opened on
2026-08-04.

### Why the task closed anyway

James asked to start another issue rather than wait. The plan's one remaining `[ ]` — "Michael
reviews and merges #41" — was checked for whether it actually owes James anything, and it does not:
the PR body's first line is `Closes #15.`, so the merge closes the GitHub issue by itself. The item
is a wait, not a task. Reasoning recorded in `decisions.md` 2026-08-05.

### Queue check — what is actually assignable

`gh issue list` on 2026-08-05: of the 14 open issues, **#24 is the only tier-(1) issue assigned to
shorinbonsai**. #22 is the other open tier-1 and is Michael's. James's #23, #25 and #19 are tier (2)
and gated behind a tier-1; #29, #6, #18 are tier (3); #28 and #20 are tier (6).

**#24 does not collide with PR #41.** #41 touches `common.rs`, `mod.rs`, `steady_state.rs`; #24
touches `config.rs` and `config.example.toml`. No shared file, so both can be in flight and neither
rebases.

### Noticed in passing — carried into #24's plan, not filed

`collab.md` **#21** (open, Michael's: do users supply drop-in Rust objective files?) reaches further
than its own text claims. It says it changes #26, which was true when raised on 2026-08-04 22:00 —
but #24 is what pins `FitnessConfig`'s variant list, and a closed match cannot name a type outside
the crate. Decision on 2026-08-05: build to the sheet as written, exactly as Michael did for #17,
and note the exposure in #24's plan. No `collab.md` entry, because the change would be **additive**
(one more variant) if #21 resolves the other way.

### Git manifest — 2026-08-05, unchanged from the previous session

| Repo | Branch | State |
|---|---|---|
| `GraphEvolutionTool` (root, only repo) | `jsargant_direction_at_boundary` | `320fe68`, pushed, open as **PR #41**. Clean tree, nothing unpushed |
| — | `main` | `252347d`, pushed |

Untracked and deliberately never staged: `docs/`, `GET GA planning session.md`.

---

## Session 2026-08-04: #15 implemented and verified in one sitting; nothing committed

**Planned and built in the same session.** `/start` wrote the plan; every task on it is `[x]`
except the commit/PR step, which is held pending explicit instruction.

### What changed — 3 files, +110/−41, all uncommitted

- `get/src/evolver/common.rs:255` — `generation_stats(iteration, fitnesses)`. Lost its `Direction`
  parameter and both `orient` calls. `Direction` dropped from the lib-scope import (it was only
  reachable from doc links and the test harness); the two intra-doc links now use the full
  `crate::fitness::Direction::…` path, matching what line 220 already did, and `Direction` is
  imported inside `mod tests` where `NodeCount` needs it.
- `get/src/evolver/mod.rs:96-118` — `EvolutionOutcome` gains `pub direction: Direction`;
  `best_fitness` → `best_fitness_engine`. `GenerationStats` gained a doc paragraph only.
- `get/src/evolver/steady_state.rs:132` — `outcome` stores `direction` instead of applying it;
  `evolve` no longer needs its `let direction` binding. Four test call sites renamed to
  `best_fitness_engine` (the five *history-row* `best_fitness` references are a different field and
  were correctly left alone).

### Validated — and what "validated" means here

- `cargo test -p get`: **128 pass**, up from 110 at the end of #14.
- **Sabotage check, run not assumed.** Reinstated both conversions (`-best`/`-mean` in
  `generation_stats`; `direction.orient(...)` in `outcome`) → 6 failures including exactly the two
  new guards, at `common.rs:388` and `steady_state.rs:618`. Reverted → 128 pass. This is the
  evidence behind both `[x]` marks; without it the guards were unproven.
- `cargo clippy -p get --all-targets`: **byte-identical to the `main` baseline**, which was captured
  by `git stash push -- get/src/` rather than by trusting the handoff's description of it. Same two
  dead-code warnings from the unbuilt #25.
- `cargo doc --no-deps`: 4 warnings, all pre-existing and all in `sda.rs`/`lib.rs`. None in the
  three files touched, which is what confirms the new `crate::fitness::Direction` links resolve.

### Not validated

Nothing. There are no `[~]` items on this task.

### Noticed in passing

`main` is not rustfmt-clean: `generational.rs` has a `use super::{...}` list rustfmt wants
reordered. That is precisely GitHub **#22** ("Format the tree once"), Michael's, so no new issue was
filed — but it is why the rustfmt trap below produced real diff instead of a no-op.

### Git manifest — updated 2026-08-04 22:2x, after the save

The save's manifest said "0 commits, nothing pushed". Work continued past it and that is no longer
true — this is the current state:

| Repo | Branch | State |
|---|---|---|
| `GraphEvolutionTool` (root, only repo) | `jsargant_direction_at_boundary` | `320fe68`, **pushed**, open as **PR #41**. Clean tree |
| — | `main` | `252347d`, pushed. `traps.md` + `decisions.md` appends only (+58/−0) |

**Nothing is uncommitted.** PR #41 verified from the remote: `state: open`, `merged: false`,
`mergeable: true`, 1 commit, 3 files, +110/−41. The body was diffed byte-for-byte against the source
file — identical but for a trailing newline GitHub appends, so the tables, fences and `§` all
survived, which is the check `CLAUDE.md` requires and the exit code does not give.

Untracked and deliberately never staged: `docs/`, `GET GA planning session.md` — stale pre-spec-sheet
documents. Stage explicit paths; never `git add -A`.

---
