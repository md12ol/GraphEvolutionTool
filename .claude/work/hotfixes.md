# Hotfixes — temporary code in the tree

Every band-aid, stub, sleep, hardcoded value and workaround currently in the working tree. Each
needs an exit condition or it lives forever.

Maintained by `/save`. `/done` stamps `Last checked:` — an entry with an old or missing stamp has
not been assessed recently.

⚠️ in a `Remove when:` marks a **load-bearing** hotfix: something breaks today without it. Do not
delete those on a tidying pass.

Group entries under `## <theme>` headings by what unblocks them — that is the axis on which they
actually get removed, in batches.

**This file is shared; the working trees it describes are not.** Two people use this repo, and an
*uncommitted* hotfix exists on exactly one machine — so an entry here does not mean the code is in
*your* tree. Every entry therefore carries **`Owner:`** and **`Machine:`**. Read them first: if the
owner isn't you, the entry is information, not something to go and find. Don't delete another
owner's entry — ask in `collab.md`.

A hotfix that has been **committed** is in everyone's tree; say so in `Machine:` and it becomes
everybody's problem to remove.

---

## <theme — e.g. blocked on upstream, blocked on someone's work, ours to fix>

### <what was hacked>
- **Owner:** who put it there and who removes it — `Michael` / `James`.
- **Machine:** `owner's working tree, uncommitted` · `committed — in every tree` · `branch <name>`.
  This is what tells the other person whether to expect the code locally.
- **Where:** `path` or symbol name — prefer function names over line numbers, they survive edits.
- **What it does:** the mechanism, if not obvious from the title. Optional.
- **Why it's a hotfix:** the problem it papers over, and why the proper fix wasn't done here.
- **Real fix:** what would make this unnecessary, and **who owns it** if it's someone else.
- **Remove when:** the concrete condition that makes it unnecessary.
- **Added:** <YYYY-MM-DD>
- **Last checked:** <YYYY-MM-DD>

## Blocked on a later issue in the same workstream

### The SIR batch seed never changes between evaluations
- **Owner:** `Michael` — it is my #17, and #18 that removes it is mine too.
- **Machine:** `branch mdube_sir_objectives` (PR for issue #17), so expect it in every tree once
  that merges.
- **Where:** `EpidemicScorer::batch_seed` in `get/src/fitness.rs`.
- **What it does:** returns the run seed unchanged, where the design is a run seed plus an atomic
  evaluation counter incremented once per batch.
- **Why it's a hotfix:** `Fitness::evaluate` takes `&self`, so the counter has to be an
  `AtomicU64` on the objective — and that whole mechanism is issue **#18**, the next one in this
  workstream. #17 builds everything that sits *above* a batch seed and takes the seed as given.
- **What is and is not already correct** — worth being precise, because the honest failure is
  narrower than "seeding is not done". Common random numbers **within** a batch already hold:
  `batch_seed` does not vary with the graph, so every individual in one evaluation faces identical
  dice, which is the property selection depends on. What is missing is variation **across**
  evaluations, so a run currently optimizes against one frozen sample of the disease rather than
  the disease. Fitness values are meaningful *relative to each other*; a whole run is not yet
  research-usable.
- **Real fix:** issue #18 — the per-run atomic evaluation counter, replacing this one method body.
- **Remove when:** #18 lands. It is a one-method change and no caller moves, which is why the
  seam was put here rather than threading a seed argument through the three objectives.
- **Added:** 2026-08-04 — sir-batch-seed-never-changes-between-evaluations
- **Last checked:** 2026-08-05 — James, at the `/done` gate for direction-at-boundary. Both passes
  verified on `main` at `252347d`, not inferred: the code is **still present**, `fn batch_seed`
  returning `self.run_seed` unchanged at `get/src/fitness.rs:158-160`; and the `Remove when:` is
  **not met** — GitHub #18 is still `open`. Carried forward unchanged, second cycle running.
  (Previously checked 2026-08-04 at the express-and-score gate, same result on both passes.)
- **Last checked:** 2026-08-06 — Michael, at the `/done` gate for issue #22. Verified, not inferred:
  `fn batch_seed` still returns `self.run_seed` unchanged at `get/src/fitness.rs:162-164`; GitHub
  #18 still `open` (`gh api repos/md12ol/GraphEvolutionTool/issues/18` → `state: open`). Carried
  forward unchanged, third cycle running.
- **Status note added 2026-08-04 — James (not a rewrite of Michael's lines above):** the `Machine:`
  line anticipates the merge that has now happened. PR #40 merged `mdube_sir_objectives` into `main`
  on 2026-08-04, so this hotfix is **committed and in every tree**, no longer branch-local. Michael's
  original wording is left intact; this line supersedes only the tense.

