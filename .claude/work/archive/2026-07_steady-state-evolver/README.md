# steady-state-evolver — 2026-07-31

**Objective.** Make `SteadyStateEvolver` actually run: fill the `todo!()`s in
`get/src/evolver/steady_state.rs`, plus the shared helpers in `get/src/evolver/common.rs` it could
not run without. Done meant a seeded `run()` evolving a population against a `Fitness` and returning
a real `EvolutionOutcome`, proven by tests rather than by compiling.

**Dates.** Opened and closed 2026-07-31 — a single day, several sessions.

## Outcome

Delivered. `evolver/common.rs`, `evolver/steady_state.rs` and `fitness.rs` have no `todo!()`s left;
the test suite went 56 → 97, all passing on `main`. Merged via PR #12, with `b466e4e` carrying the
working-docs commit that PR branched before.

The design landed as tournament-local replacement — one tournament per mating event, its two best
breed, the two children overwrite its two worst — which makes the strategy self-elitist without
explicit elitism. Along the way the task also produced `Direction` and the fitness-orientation
contract, which both evolvers now depend on.

Every claim above was mutation-tested, not just observed green: inverting the selection comparator
fails 3 tests, pointing replacement at the tournament's best fails 4, a no-op `evolve` fails 2, and
`log_interval = 1` fails the cadence test. That process caught two tests that passed for the wrong
reason and were rewritten.

## What outlived the task

- **8 `todo!()`s remain in the crate** — `SirFitness` (2), `Config::from_path`,
  `GraphEvolver::run`/`save_logs`/`save_results`, generational (2). All were out of scope here.
  **"The steady-state evolver is finished" is not "GET runs"**; only the first is true.
- **`decisions.md`** — 9 entries from this task, plus 12 more from the later spec-sheet call.
  All persist; they constrain the codebase, not just this task.
- **`traps.md`** — 3 entries added here, all re-verified as still true at `/done`: bare `cargo fmt`
  sweeping unformatted files, the `-0.0` divergence between the selection test oracle and
  `total_cmp`, and `.claude/` docs splitting across branches.
- **`collab.md`** — items 7, 8 and 13 still open for James. Items 1–6 and 9–12 were settled by the
  spec-sheet call and compressed into the Settled table.
- **No hotfixes.** Both `NaN` entries were resolved the day they were written, by enforcement in
  `Direction::orient` rather than by documentation.
- **No unfiled issues.** 19 open in the tracker at `md12ol/GraphEvolutionTool`, which is the source
  of truth; `issues.md` is a staging area only.

## Notable incidents, recorded so they are not re-learned

- A mid-`/save` checkout to `main` wrote seven `decisions.md` entries onto the wrong branch's copy.
  Nothing lost — rescued and re-applied — and now a trap.
- PR #12 was merged on GitHub while a local merge of the same branch was in progress, and it
  branched one commit early, so `origin/main` was briefly missing the working-docs commit.
- The task was archived only after the spec-sheet call had already happened outside any plan. That
  work should have had its own `/start`; the empty `archive/` at the time was the symptom
  `CLAUDE.md` warns about.
