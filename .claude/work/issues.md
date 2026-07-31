# Issues — work for other people

Staged for the tracker. Two tiers; the difference is whether it has been root-caused.

Maintained by `/save`. `/done` lists anything still `Filed: not yet` before archiving a task.
How issues get filed — tool, confirmation rule, target project — lives in `CLAUDE.md`.

Once an entry is filed, **the tracker is the source of truth.** Changes go to the tracker in the
same session; this file must not become a private fork of it.

---

## Parked — noticed, not investigated

### <what was noticed>
- **Where:** `path` or component, as far as it's known.
- **Impact:** why it matters — who or what it breaks.
- **Noticed:** <YYYY-MM-DD>, in <what you were doing when you hit it>

---

## Ready to file — root-caused and evidenced

### <title — imperative, issue-ready>
- **For:** teammate / team / unassigned
- **Project:** the tracker project it belongs to
- **Filed:** not yet
- **Component:** `path:line`
- **Body:**
  What's wrong, the mechanism with `path:line`, evidence, how to reproduce, candidate fixes.
