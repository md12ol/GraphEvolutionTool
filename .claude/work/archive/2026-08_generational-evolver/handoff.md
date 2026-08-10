# Next session — 2026-08-06

Read `.claude/work/current/plan.md` and `.claude/work/decisions.md` first, then
`.claude/work/hotfixes.md`.

**Where things stand:** Issue **#25** is **implemented, tested and open as PR #46**, awaiting
Michael. Every code task on the plan is `[x]`. What is left is two things neither of which is code:
this save's doc entries are **uncommitted on `main`**, and one `traps.md` entry can only be dropped
once #46 merges. The working tree you will find is on **`main` at `841e79d`** — not on the feature
branch, because the docs belong on `main`.

**Start here:** commit and push the three doc files to `main` — `git diff --stat` should show only
`.claude/work/decisions.md`, `collab.md` and `traps.md`. They carry three `decisions.md` entries,
`collab.md` #36 plus an accepted reply in #31, and two `traps.md` changes. **This needs the user's
say-so**, like every push.

**Then, in priority order:**

1. **When #46 merges** — drop the `cargo clippy -- -D warnings cannot pass on main` entry from
   `traps.md`. Its condition is already met on the branch (`-D warnings` exits 0); it stays true for
   anyone on `main` until the merge lands, which is why it is not gone already. Verify with
   `cargo clippy -p get --all-targets -- -D warnings` on merged `main`, then `/done` the task.
2. **If review asks for changes**, they go on `jsargant_generational_evolver` at `7de4a66` — do not
   fold unrelated work in.

**Watch out for:**

- **`main` may be stale and nothing will say so.** It was 7 commits behind at the last session's
  start, and `pull_main.sh` refuses to pull whenever the tree is dirty — which it is right now, with
  three uncommitted doc files. `git fetch origin --dry-run` before trusting `main` or branching from
  it. Now a `traps.md` entry.
- **No `[~]` items** — nothing is done-but-unverified. Everything ticked was verified on this
  machine.
- **`collab.md` #36 is new and asks Michael a question**, about whether the two `outcome` methods'
  common part should move into `common.rs`. Not blocking; do not act on it unilaterally, since it
  means editing `steady_state.rs`.
- **`collab.md` #27 is still waiting on James** — `Swap`'s degree floor. Carried through four
  `/done` gates now, deliberately: the code already matches spec §3.1, so loosening it needs a joint
  meeting and keeping `> 2` needs only a `decisions.md` entry.
- **The SIR-batch-seed hotfix is still in the tree**, load-bearing, blocked on #18. Sixth cycle,
  Michael's.

**⏰ Time-sensitive:** nothing dated. PR #46 is the only thing another person is holding.
