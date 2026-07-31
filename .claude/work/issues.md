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

---

### Format the tree and do a readability pass over the source
- **For:** Michael (md12ol)
- **Project:** `md12ol/GraphEvolutionTool`
- **Filed:** not yet
- **Component:** whole tree; `get/src/evolver/generational.rs`, `get/src/genomes/sda.rs` are the
  currently unformatted files.
- **Raised:** 2026-07-31 — Michael, agreed with James on the spec-sheet call.
- **Body:**

  Three related pieces of work, deliberately batched into one pass so the tree churns once
  rather than three times.

  **1. One tree-wide `cargo fmt` commit, on its own.**
  Part of the tree has never been formatted, so anyone running bare `cargo fmt` to tidy their own
  edit sweeps unrelated files and hands the other owner a pile of foreign diff. It already
  happened once and had to be reverted by hand. Current offenders —
  `cargo fmt -- --check 2>&1 | grep "^Diff in"` — are `get/src/evolver/generational.rs` and
  `get/src/genomes/sda.rs`; it was four files before `fitness.rs` and `steady_state.rs` were
  formatted as a side effect of being edited.

  **Sequencing matters:** `generational.rs` is James's live work. Land this when his working tree
  is clean, or it lands as a conflict. Do it in a commit that contains *nothing else*, so the
  diff is reviewable as "formatting only".

  Once merged, `.claude/work/traps.md` "Running bare `cargo fmt` rewrites files you did not
  touch" stops being true and should be removed, and `collab.md` #7 marked Agreed.

  **2. Explicit returns, against clippy's preference.**
  Clippy's `needless_return` fires on explicit `return` at the tail of a function and prefers the
  implicit form. We want the **explicit** form where it aids readability — in this codebase the
  functions that end in a bare expression after twenty lines of setup are genuinely harder to
  scan. Adopting this means suppressing the lint deliberately rather than accumulating
  `#[allow]`s ad hoc:

  ```toml
  # Cargo.toml, [lints.clippy] — one place, reviewable, not scattered
  needless_return = "allow"
  ```

  Decide this before the formatting commit, since it changes what "formatted" means.

  **3. General readability rewrite.**
  Opportunistic, and lower priority than the two above. Naming, function length, and comment
  density, applied to code that is already correct — this is not a behaviour change and must not
  become one. Anything that turns out to be a real defect gets its own issue rather than being
  fixed quietly inside a readability pass.

  **Verify by:** `cargo fmt -- --check` exits clean; `cargo clippy -- -D warnings` passes; the
  full suite still reports 97 passed (`cargo test`).
