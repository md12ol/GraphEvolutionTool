# Traps — how this workspace actually behaves

Permanent gotchas. Distinct from `hotfixes.md`: a hotfix is *code you added and want to remove*; a
trap is *how this workspace behaves and always will*.

This file exists because durable warnings kept getting parked in `handoff.md`, which `/save`
overwrites every session — so they were deleted the moment they stopped being top-of-mind.

Read by `/load` and `/start`. Entries leave only when no longer true.

---

### <the trap, stated as the mistake it prevents>
- **Bites when:** the action that triggers it.
- **Do this instead:** the correct form.
- **Why:** the mechanism, one line.
- **Added:** <YYYY-MM-DD>
### Running bare `cargo fmt` rewrites files you did not touch
- **Bites when:** you run `cargo fmt` (no path) to tidy your own edit. It reformats the whole
  workspace, and part of the tree has never been formatted, so you get unrelated hunks in your
  diff — in files someone else is actively editing.
- **Do this instead:** format only what you changed —
  `rustfmt --edition 2024 get/src/path/to/file.rs`
- **Why:** these files predate any `cargo fmt` run. Check the current set with
  `cargo fmt -- --check 2>&1 | grep "^Diff in"`. As of 2026-07-31 it is
  `get/src/evolver/generational.rs` and `get/src/genomes/sda.rs` — `generational.rs` is James's
  live work, so sweeping it hands him a conflict. It was 4 files before `fitness.rs` and
  `steady_state.rs` were formatted as part of editing them.
- **The real fix** is one tree-wide `cargo fmt` commit, agreed with James — `meeting_james.md` #7.
  Until that happens, this trap stands.
- **Added:** 2026-07-31

### A `-0.0` fitness would make the selection tests disagree with the code
- **Bites when:** a fitness function returns `-0.0` alongside `0.0`, and a selection test fails in
  a way that looks impossible.
- **Do this instead:** if you hit it, fix the oracle — not the implementation. The implementation is
  right.
- **Why:** `Selection` orders with `f64::total_cmp`, which distinguishes `-0.0 < 0.0`. The test
  oracle `expected_winner` in `get/src/evolver/common.rs` compares with `<` and `==`, which treat
  them as equal, so it would predict a tie where the code picks a winner. The oracle is deliberately
  written differently from the implementation so it is an independent check of the tie-break rule;
  that independence is exactly what creates this gap. No current test data contains `-0.0`.
- **Added:** 2026-07-31

### The `.claude/` docs split across branches, but `work/current/` does not
- **Bites when:** you switch branches mid-task, then write to `decisions.md`, `traps.md`,
  `hotfixes.md` or `issues.md`. Those four are **tracked**, so each branch has its own version —
  writing on the wrong branch appends to the wrong base and silently drops entries the other branch
  had. It happened on 2026-07-31: a checkout to `main` mid-`/save` put seven new `decisions.md`
  entries on a copy that was missing the two already committed on the feature branch.
- **Do this instead:** run `git branch --show-current` before writing any `.claude/` doc, and again
  after any long gap. If you are on the wrong branch, rescue the new text to the scratchpad,
  `git restore` the file, switch, and re-append.
- **Why:** `.gitignore` excludes `.claude/work/current/` and `.claude/work/archive/` but tracks
  everything else under `.claude/`. So `plan.md` and `history.md` follow you across branches while
  `decisions.md` does not — the two halves of the docs system behave differently.
- **Added:** 2026-07-31

