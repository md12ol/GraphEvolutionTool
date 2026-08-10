# generational-evolver — GitHub #25

**Objective:** replace the two `todo!()`s in `get/src/evolver/generational.rs` with a working
generational strategy — score, log, advance for `num_generations`, carrying `elite_count` elites
forward — indistinguishable from steady-state in every shared mechanism (same RNG, same scoring
gate, same mutation helper). Spec §6.2, §4, §5.1. Two cleanups were folded into the issue on
2026-08-04 and shipped with it.

**Dates:** planned 2026-08-06, implemented 2026-08-06 (evening), closed 2026-08-07. Three sessions.

## Outcome

Shipped as **PR #46** — four commits, one per verified step — merged by Michael as `74de0b5` on
2026-08-07T14:51:59Z. Issue #25 closed by the body's `Closes #25.` `advance_generation` carries
elites by `common::rank` and fills the rest through `Selection::select` + one crossover roll +
`common::mutate_child` per child, making **neither mutation roll locally**; `run` seeds `ChaCha8Rng`,
scores only via `express_and_score`, logs generation 0, and rescores every individual every
generation, elites included. `new` gained a backstop `assert!(elite_count < population.len())`.

**176 tests pass on the merged tree** — checked after merging, alongside Michael's #18 rewrite of
`fitness.rs`, not only on the branch. The issue's real gate was met: stubbing `advance_generation`
to a no-op fails **4 of the 13 new tests**.

**This task flipped the clippy gate.** `cargo clippy -p get --all-targets -- -D warnings` exits 0 on
`main` for the first time since 2026-08-04 — the two dead-code warnings every task in between
diffed against were this evolver's unbuilt shell. The trap recording them is retired, with a
successor entry that keeps the `git stash -u` pitfall it had accumulated.

## Left behind, deliberately

- **`collab.md` #35** — spec §6.2 says "track the best"; the code reports the best of the final
  population. Michael raised it reviewing the merged PR and thinks the code is right; James endorsed
  amending §6.2 rather than changing the code. **Needs the joint meeting** — the sheet is the
  authority and neither owner may amend it alone. The sharp edge is `elite_count = 0`, which §7
  permits and where the two readings genuinely diverge.
- **`collab.md` #36** — the two evolvers now have their own `outcome`, ~10 similar lines each,
  differing on graph reuse per §6.2. Factoring the common part into `common.rs` means editing
  `steady_state.rs`, which this task was scoped out of. Awaiting Michael.
- **`issues.md`** — Michael's `evaluate_population` → `evaluate_batch` and `SirRun` → `Epidemic`
  rename, **unfiled**, blocked on the same meeting because both names are in the sheet.
- **`collab.md` #27** — `Swap`'s degree floor, still James's, carried through five `/done` gates.
  Unrelated to this task.

**Nothing is left in `hotfixes.md`** — Michael's #18 removed the SIR batch-seed hotfix on
2026-08-07, after six cycles of being carried. First task to close with the file empty.

## Worth reading if you touch this code

`decisions.md`, 2026-08-06 21:03 / 21:04 / 21:05 — why generational's `outcome` takes the winner's
graph from the final scoring pass, why "track the best" was read as best-of-final, and why
`advance_generation` does not take the objective. The 2026-08-07 entry explains why the clippy trap
was retired with a successor rather than deleted.
