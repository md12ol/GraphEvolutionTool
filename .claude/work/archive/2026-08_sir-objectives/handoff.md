# Next session — 2026-08-04

Read `.claude/work/current/plan.md` and `.claude/work/decisions.md` first, then
`.claude/work/hotfixes.md`.

**Where things stand:** Issue #17 is built and PR #40 is open, assigned to James — the three SIR
objectives in `get/src/fitness.rs` over a shared batch runner in `get/src/sir.rs`, 127 tests
passing. All docs are pushed to `main` (`29b3f6a`) and issue #22's verify-by is corrected. The task
is *not* closeable: #40 is unmerged, so one `[~]` remains. **Nothing is left uncommitted or
unpushed** — check anyway, `main` has moved twice today.

**Start here:** nothing is blocked on you. Confirm the state, then wait or move on:

```bash
git checkout main && git pull && git log origin/main..main --oneline   # expect empty
gh pr view 40 --json state,mergeable,reviewDecision
```

**Then, in priority order:**

1. **Wait on #40; do not merge it.** James merges Michael's PRs. If he asks for changes, note that
   `main` moved under this branch mid-task — his #38 landed and touches `fitness.rs`. A test-merge
   auto-merged with 127 passing, but **re-run it rather than trusting that result**, because `main`
   may have moved again since.
2. **Issue #18 is the natural next task** and removes the only hotfix this task added — it replaces
   `EpidemicScorer::batch_seed` and nothing else. Start it with `/start`, not by extending this
   plan. Do not start it *on top of* `mdube_sir_objectives` unless #40 has merged.
3. **`collab.md` #21 and #22 are waiting on James**, not on you. #21 (drop-in Rust objectives)
   shapes **#26**, which is yours — do not design #26's dispatch until it is answered.

**Watch out for:**

- **`cargo clippy -- -D warnings` cannot pass on `main`** — two dead-code errors in
  `generational.rs` from James's unbuilt #25. Do not treat them as your regression, and do not
  write "clippy passes" in a PR. New `traps.md` entry with the baseline command.
- **The batch-seed stub is live in the tree** on `mdube_sir_objectives`. CRN *within* a batch is
  correct; variation *across* batches is not, so a full run is not research-usable until #18. See
  `hotfixes.md`.
- **Never bare `cargo fmt`** — it rewrites `generational.rs` and `sda.rs`, which are not yours.
  `rustfmt --edition 2024 <file>` only.
- **Approving a plan is not authorization to push or open a PR.** PR #39 was opened that way this
  session and had to be closed. Ask each time.

**⏰ Time-sensitive:** nothing dated. But `main` has moved twice on 2026-08-04 while work was in
flight, so re-check `git log origin/main..main` and the PR head sha before assuming anything about
either.
