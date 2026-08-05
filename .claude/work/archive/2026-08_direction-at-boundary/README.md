# direction-at-boundary — GitHub #15

**Objective.** Stop the engine converting fitness direction internally, so `express_and_score` is
the only place a direction is applied inside `get/src/`. `generation_stats` no longer takes a
`Direction`, `SteadyStateEvolver::outcome` no longer orients, and `EvolutionOutcome` carries the
`Direction` for a future boundary to convert once on the way out. Spec §5.1 and §6.4; implements
`decisions.md` 2026-07-31, "The engine is oriented internally; convert only at the boundary".

**Dates.** Two sessions — implemented 2026-08-04, closed 2026-08-05.

**Outcome.** Shipped as **PR #41** (`320fe68`, 3 files, +110/−41), which was **open, mergeable and
unreviewed when this task closed**. `cargo test -p get` 128 pass, up from 110 at the end of #14;
clippy `--all-targets` byte-identical to the `main` baseline; no new rustdoc warnings.

The `std_dev` special case and the test defending it disappeared along with the conversion that
caused them.

**Two things worth knowing about how it was verified**, both in `decisions.md` 2026-08-04:

- **The conversion was invisible to the test suite.** Every pre-existing steady-state test uses
  `NodeCount`, which is `Direction::Minimize`, and orienting a minimizing objective is the
  *identity* — so removing or restoring the conversion at `steady_state.rs:132` changed no test
  outcome. A `Maximize` harness (`MostNodes`) was added beyond what the issue asked for.
- **Both new guards were proven by sabotage, not assumed.** Reinstating the two conversions failed
  exactly the two new tests plus 4 collaterals; reverting returned 128 green.

`EvolutionOutcome.best_fitness` was renamed to **`best_fitness_engine`** deliberately, to break
every future reader at compile time — #27 builds the Python boundary that consumes it, and a
boundary that forgets to convert is the one failure this issue exists to prevent.
`GenerationStats.best_fitness` **keeps its name** and is a different field: it is a log column named
in spec §6.4's table and does not cross the boundary. Do not "finish" the rename.

## Closed with the PR still open — read this before copying the pattern

The plan's last item, "Michael reviews and merges #41", was **struck rather than ticked** on
2026-08-05, and the task archived while the PR was unmerged. That was allowed because the item owed
**nothing to this owner**: PR #41's body opens with `Closes #15.`, so the merge closes the GitHub
issue by itself. Checked on 2026-08-05, not assumed.

This does not generalize to any open `[ ]`. It turned on the closing keyword being present, and on
`work/current/` holding exactly one task — blocking on #41 would have blocked tier-1 issue #24
behind another person's availability. Full reasoning in `decisions.md` 2026-08-05 15:09.

**Resolved 2026-08-05 20:53 UTC — #41 merged as `0f999ee`, and GitHub closed #15 as `completed` by
itself.** The `Closes #15.` keyword did exactly what the disposition relied on, so the contingency
noted here (a PR closed *unmerged* would have reopened #15 as new work) never arose. The merge
brought only the three evolver files; `.claude/` was untouched.

## Left behind, deliberately — all carried forward

| | Where it lives | State at close |
|---|---|---|
| **PR #41 itself** | GitHub | Open, `mergeable: true`, **zero reviews and zero comments** as of 2026-08-05. Michael merges it; nothing owed by James |
| **Hotfix: SIR batch seed never changes between evaluations** | `hotfixes.md` | Michael's, at `get/src/fitness.rs:158-160`. **Second gate cycle unresolved.** Both passes re-verified on `main` at `252347d`: code still present, `Remove when:` not met — #18 still open. Load-bearing; a run currently optimizes against one frozen epidemic sample |
| **`collab.md` items 20 and 21** | `collab.md` | Both open with Michael. Neither blocked this task |
| **Untracked stale docs** | `traps.md` | `docs/` and `GET GA planning session.md` remain untracked in James's tree and must never be staged |

Nothing was left `Filed: not yet` — the only match in `issues.md` is its template placeholder.

## The one thing worth carrying into every later task

**`rustfmt` on a `mod.rs` is not per-file — it reformats every submodule that file declares.** Found
this task, and it reached `git diff` on `generational.rs`, which is Michael's #22. The fix is
`rustfmt --edition 2024 --config skip_children=true <files>`; `--skip-children` is **not** a CLI flag
here. Read `git diff --stat` afterwards regardless. Full entry in `traps.md` as
`per-file-rustfmt-is-not-per-file-on-a-mod-rs`.

**Next task:** GitHub **#24**, the `config.rs` schema — fitness variants, `max_mutations`, drop
`seed` and `num_chars`. Tier (1), James's, branches off `main` at `252347d`. It shares no file with
PR #41, so the two can be in flight at once.
