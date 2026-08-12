# Next session — 2026-08-12

Read `.claude/work/current/plan.md` and `.claude/work/decisions.md` first, then
`.claude/work/hotfixes.md`.

**Where things stand:** the documentation site is finished and merged — `documentation/` is on
`main` at `d420b3e`, 38 pages, PR #64 merged by James. Every task on the plan is `[x]`, the tree is
clean, and nothing is unpushed. This task is ready for `/done initial_doc_site`; if that has already
run, the record is in `.claude/work/archive/2026-08_initial_doc_site/`.

**Start here:** run `/done initial_doc_site` to archive the task. If it has already run, there is
**no queued work** — pick up something from GitHub issues instead, and read
`documentation/HANDOFF.md` if the next thing is docs-related.

**Watch out for:**

- **`collab.md` #50 is unanswered.** James merged PR #64 without ruling on whether unbuilt features
  should keep being documented in the present tense with `planned` badges. The convention therefore
  stands unopposed rather than agreed. It is cheap to reverse now and much more expensive once
  other people have edited pages — worth a nudge next time you speak to him.
- **`collab.md` #51 needs a joint meeting.** Five spec-sheet items, two of which are stale status
  claims where the code is ahead of the sheet. Nothing depends on them, but the sheet's own note
  says a stale status row is its whole signal.
- **If you edit `documentation/`, read `documentation/README.md` first.** Three things are silently
  load-bearing: the `NAV` table in `assets/site.js` is the site map, `data-page` must match both the
  file's path and its `NAV` entry, and `h2`/`h3` headings become the on-page contents. Run the
  verification script in that README before pushing.
- **The browser caches `site.js` and `style.css` hard.** An edit that looks like it did nothing is
  usually the cache, not the code — see `traps.md`,
  `docs-site-asset-caching-hides-your-edit`.
- Nothing is `[~]`, no hotfixes are in the tree, and no issues are staged unfiled.

**⏰ Time-sensitive:** nothing dated.
