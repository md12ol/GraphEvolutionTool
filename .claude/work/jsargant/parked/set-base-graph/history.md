# History — GitHub #28: `set_base_graph` and its three validation checks

Append-only session log for this task, newest session first.
Maintained by `/save`; archived by `/done`.

---

## Session 2026-08-13: #28 finished, PR #72 opened, then reopened for the #61 ruling

**Tasks 3–8 all closed.** `set_base_graph` landed with its three checks (`3af041c`); threaded
through `dispatch::evolve`/`edge_edit_start` as `Option<&Graph>` (`c99fa11`); four tests
(`0e48a01`); doc note verified; stacked run verified; PR #72 opened.

**Verification worth repeating rather than re-deriving.** The four tests were checked for *teeth*
by disabling the threading and all three checks — exactly those four failed, other 235 passed. The
stacked SDA→edge-edit run was made decisive rather than suggestive by running stage 2 with `null` as
the only non-zero operation weight and mutation/crossover at 0, so expression is a guaranteed no-op:
all **358** seeded edges expressed exactly, and a control run with no base graph expressed **0**,
which is what stops the assertion being vacuous.

**Then the meeting reopened it.** `collab.md` #61 — which this session raised, after measuring that
`set_base_graph(8, [(0,1,1),(2,9,1),(3,3,1),(4,5,1)])` returned `Ok` and stored two edges — was
settled as *reject* both out-of-range endpoints and self-loops (`decisions.md` 2026-08-13 20:16).
Implemented in `0604323`: two checks, two tests, and a doc comment that had stated the opposite and
would have shipped documenting the rejected behaviour. 243 tests pass. A hold comment went on PR #72
before the work started and a clearance comment after it, so the PR was never merge-able in the
wrong state.

**Repo housekeeping this session, all on `main`:** merged PR #70 (docs worktree — `.gitignore`
conflict, `.venv/` vs `*.code-workspace`, both kept) and PR #71 (run output, after verifying the
merge result rather than the branch: 237 tests, no `todo!()` left). Ran the worktree setup, which
needed the script extracted out of `main` because it is unreachable from a branch cut before it
existed. Signed off `collab.md` #58 with that as a caveat; agreed #53; recorded #62's joint
agreement and its `decisions.md` entry. Corrected two doc lines that had gone false —
`CLAUDE.md`'s reference row claiming GET lacks a `pyproject.toml`, and `jsargant_edits.md`'s header
claiming the queue was unagreed (`bcc2f13`).

**Observed but not recorded as a trap:** both PRs merged locally today had their remote branch
already deleted, which `traps.md` says should not happen for a locally-merged PR. Evidence is
ambiguous — the branch was never seen to exist post-merge, so Michael or GitHub could have removed
it — so the trap is left alone rather than corrected on one observation.

**Git manifest at end of session:** branch `jsargant_set_base_graph`, 11 commits ahead of `main`,
**pushed** and level with `origin`. Working tree clean. `main` at `bcc2f13`. `.venv/` exists and is
gitignored — `maturin develop --release` rebuilds it. PR #72 open, `MERGEABLE`, review requested
from Michael, carrying `get/src/lib.rs`, `get/src/dispatch.rs`, `documentation/jsargant_edits.md`.

**Next:** nothing but Michael's review and merge — which is why this task is parked.

## Session 2026-08-12: branch cut, `base_graph` field landed

`/start` agreed the task against #28 (unblocked once #27/PR#65 closed) over the alternative, #20
(replicate runs) — also assigned to James, deferred rather than dropped.

**Task 1 — branch.** `git fetch origin --dry-run` printed nothing and local `HEAD` matched
`origin/main` at `da073aa`, so `main` was current (the `pull_main.sh`-declines-on-dirty-tree trap
did not apply — only untracked files were present). Cut `jsargant_set_base_graph` from there.

**Task 2 — `base_graph` field.** Added `base_graph: Option<Graph>` to `GraphEvolver`
(`get/src/lib.rs:56`), `None` in both real constructors (`new`, `from_config`) and all 5 test
struct literals across `lib.rs` and `dispatch.rs` (`grep -n "GraphEvolver {"` found them all).
Added the `crate::graph::Graph` import. `cargo test -p get` — needed
`LD_LIBRARY_PATH=$(python3 -c 'import sysconfig; print(sysconfig.get_config_var("LIBDIR"))')`
per the existing `cargo-test-cannot-link-python-unless-extension-module-is-off` trap — ran clean:
235 passed, 0 failed, one expected `dead_code` warning on the unread field.

Committed as `f02cdce` on `jsargant_set_base_graph`, author/committer both James Sargant, no
co-attribution trailer (checked with `git log -1 --format='%B' | grep -i claude`). **Not pushed.**

**Decision made:** `set_base_graph`'s cap-narrowing check rejects rather than warns — the issue
text left this open. Full reasoning in `decisions.md` 2026-08-12.

**Git manifest at end of session:** branch `jsargant_set_base_graph`, HEAD `f02cdce`, 1 commit
ahead of `main` (`da073aa`), not pushed. Working tree clean except untracked
`.claude/work/jsargant/` (this save) and a pre-existing untracked `GET GA planning session.md` at
repo root, unrelated to this task.

**Next:** task 3 — the `set_base_graph` pymethod itself, per `plan.md`.
