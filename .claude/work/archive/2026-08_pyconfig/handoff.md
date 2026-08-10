# Next session — 2026-08-09

**Where things stand:** #29 (Python config builder) is fully done, merged, and closed — PR #49
merged at 2026-08-09T12:59:11Z, issue #29 `CLOSED`. This task is being archived via `/done pyconfig`
right after this save; there is nothing left to resume here.

**Start here:** run `/start` for the next piece of work. There is no unblocked issue currently
assigned to shorinbonsai — #28 and #20 are both tier-6, blocked on #27 (Michael's, unstarted).
Check `gh issue list` and `collab.md`'s Open section, or ask Michael what's next.

**Watch out for:**
- **`cargo test -p get` exits 127** unless libpython is on the loader path:
  `export LD_LIBRARY_PATH="$(python3 -c 'import sysconfig; print(sysconfig.get_config_var("LIBDIR"))'):$LD_LIBRARY_PATH"`
- **`git add -A` would commit `docs/` and `GET GA planning session.md`**, untracked
  pre-spec-sheet files — use explicit paths.
- The `#[allow(dead_code)]` on `python_fitness` (`get/src/lib.rs`) is still in the tree, blocked on
  #26 (Michael's, unstarted). See `hotfixes.md`.

**⏰ Time-sensitive:** none. `collab.md` still has open items awaiting Michael (#35, #36, #37) and
James (#27); none block whatever is picked up next.
