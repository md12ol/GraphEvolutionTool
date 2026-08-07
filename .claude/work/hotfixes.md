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
## Suppressions waiting on another issue

### `#[allow(dead_code)]` on `GraphEvolver::python_fitness`
- **Owner:** `James` — it is my #19, and the #26 that removes it is Michael's.
- **Machine:** `branch jsargant_pyfitness` (PR for issue #19), so expect it in every tree once
  that merges.
- **Where:** `GraphEvolver::python_fitness` in `get/src/lib.rs`, in the non-`#[pymethods]` impl.
- **What it does:** silences `dead_code` on a method nothing in non-test code calls yet.
- **Why it's a hotfix:** the method is the seam **#26**'s config-to-concrete-type dispatch calls to
  turn a registered callable into `Box<dyn Fitness>`. #19 builds the seam; #26 is the only caller,
  and it is not written. Its own tests do exercise it, so the method is tested, not unused — but
  `dead_code` looks at the lib target and fires anyway.
- **Why it is not simply left warning:** #25 flipped `cargo clippy -p get --all-targets -- -D warnings`
  to passing on `main` (see `traps.md`), and one unsuppressed warning takes that gate away again for
  everyone.
- **Real fix:** issue **#26** — its `python` arm calls this method, at which point the attribute is
  deleted and nothing else changes.
- **Remove when:** #26 lands and calls `python_fitness`. Deleting the attribute and re-running
  `cargo clippy -p get --all-targets -- -D warnings` is the whole check.
- **Added:** 2026-08-07 — allow-dead-code-on-graphevolver-python-fitness
