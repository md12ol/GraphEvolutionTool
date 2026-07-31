# Hotfixes — temporary code in the tree

Every band-aid, stub, sleep, hardcoded value and workaround currently in the working tree. Each
needs an exit condition or it lives forever.

Maintained by `/save`. `/done` stamps `Last checked:` — an entry with an old or missing stamp has
not been assessed recently.

⚠️ in a `Remove when:` marks a **load-bearing** hotfix: something breaks today without it. Do not
delete those on a tidying pass.

Group entries under `## <theme>` headings by what unblocks them — that is the axis on which they
actually get removed, in batches.

---

## <theme — e.g. blocked on upstream, blocked on someone's work, ours to fix>

### <what was hacked>
- **Where:** `path` or symbol name — prefer function names over line numbers, they survive edits.
- **What it does:** the mechanism, if not obvious from the title. Optional.
- **Why it's a hotfix:** the problem it papers over, and why the proper fix wasn't done here.
- **Real fix:** what would make this unnecessary, and **who owns it** if it's someone else.
- **Remove when:** the concrete condition that makes it unnecessary.
- **Added:** <YYYY-MM-DD>
- **Last checked:** <YYYY-MM-DD>
