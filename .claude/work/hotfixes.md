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

---

## Resolved

Kept briefly for the trail; delete once the reasoning is no longer live anywhere else.

### ~~Tournament selection defends against `NaN` fitness values~~ — resolved 2026-07-31
`total_cmp` is still used in `Selection::tournament_winner` and
`Selection::tournament_indices`, but it is no longer a shield. `sort_by` needs a total order
regardless, so `total_cmp` is simply the right comparator. `NaN` can no longer reach either
site — `Direction::orient` rejects it first.

### ~~`NaN` is forbidden by contract, not prevented by code~~ — resolved 2026-07-31
Replaced by enforcement the same day it was written. `Direction::orient` asserts on `NaN` and
is the single gate every objective value passes through on its way into the engine. Covered by
`orient_rejects_nan_when_minimizing` / `..._when_maximizing`. The reason the assert exists is
preserved by `an_unchecked_negated_nan_would_have_sorted_best`, which shows that a `NaN` slipping
through under `Maximize` would sort **best**, not worst.
