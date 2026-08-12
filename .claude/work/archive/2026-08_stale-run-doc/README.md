# stale-run-doc — archived 2026-08-12

**Objective:** delete the stale "for whoever implements the dispatch (#26)" doc block from
`GraphEvolver::run` (`get/src/lib.rs:219-246`). #26 had merged and both instructions in the block
were already carried out elsewhere in the method's own comments; the block also cited a
`#[allow(dead_code)]` attribute that no longer existed.

**Spanned:** 2026-08-11 to 2026-08-12, two sessions.

**Outcome:** the 29-line block was deleted outright — nothing rewritten, nothing else in `run`
touched. `cargo test -p get` 231/231, clippy and fmt clean, unchanged from `main` going in. Opened
as PR #62, merged by James 2026-08-11T17:38:31Z (`ffb0c9b`), closing GitHub #61.

**Left behind:** nothing. No hotfixes or issues were touched by this task.
