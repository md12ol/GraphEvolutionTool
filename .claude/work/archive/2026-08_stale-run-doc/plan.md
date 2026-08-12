# Plan — Delete the stale #26-implementer doc block from `GraphEvolver::run` (#61)
_Started 2026-08-11 · last updated 2026-08-12_

## Objective
`GraphEvolver::run`'s doc comment (`get/src/lib.rs:219-246`) carries a "For whoever implements
the dispatch (#26)" block written under #19, before #26 existed. #26 has since merged (PR #60,
`97d9e02`) and both instructions in the block are already carried out in `run`'s body: it calls
`self.objective(seed)` (which reaches `python_fitness`) at line 255, and releases the GIL at
257-265 with its own inline comment covering the same argument. The block also cites a
`#[allow(dead_code)]` attribute that no longer exists. Delete the block outright — confirmed
nothing in it is load-bearing that isn't already said elsewhere in the same method.

**Done** = the block is gone, the surviving doc comment (memory-cost table, §8.1 reference)
compiles clean, and nothing else in `run`'s behavior or comments changes.

**Out of scope:** the memory-cost table and §8.1 reference above the deleted block — untouched.
Any other doc staleness elsewhere in the codebase — not surveyed, not this task's to find.

## Tasks
- [x] Branch `mdube_stale_run_doc` off `main`, before any `get/src/` edit.
      **Verify by:** `git rev-parse --abbrev-ref HEAD` prints `mdube_stale_run_doc`. Done.

- [x] Deleted the `# For whoever implements the dispatch (#26)` doc block, `get/src/lib.rs:219-246`
      (29 lines). `git diff --stat`: one file, 29 deletions, 0 insertions — nothing else touched.

- [x] Full gate: `cargo test -p get` 231/231, `cargo clippy -p get --all-targets -- -D warnings`
      clean, `cargo fmt -p get -- --check` clean. Test count unchanged from `main`'s last known 231.

- [x] **PR #62 merged.** `gh pr view 62 --json state,mergedAt` → `MERGED`, 2026-08-11T17:38:31Z
      (`ffb0c9b`). Verified on `main` 2026-08-12.

## Open questions
None.

## Out of scope
- Any rewrite-vs-delete judgment call beyond this block — issue #61 offered both; deletion was
  chosen because the content is duplicated, not because rewriting was rejected on other grounds.
