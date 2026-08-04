# Next session — 2026-08-04

Read `.claude/work/current/plan.md` and `.claude/work/decisions.md` first, then
`.claude/work/traps.md`.

**Where things stand:** issue #34 is implemented, tested and pushed on `mdube_sir_conventions`;
**PR #36** is open with `Closes #34` in the body *before* merge, so it will close the issue on
merging. 110 tests pass, clippy clean, nothing unverified, no hotfixes. Everything remaining is
waiting on James, not on work.

**Start here:** check whether PR #36 has merged —
`gh api repos/md12ol/GraphEvolutionTool/pulls/36 --jq '.state, .merged'`. If it has, do the one open
plan task below, then `/done sir-conventions`. If it has not, do not add to this branch — pick up a
different tier-1 issue instead.

**The one open task, and it is a real trap if skipped.** PR #35 (also open, different branch) marks
the `sir_sim` row of the spec status table *"corrected by #34"*. Once #36 merges that caveat is
false. **If both PRs merge untouched, the sheet cites a closed issue as pending work.** Whichever
lands second owns the fix — drop the caveat so the row reads plain `built`. Verify with
`grep -n "corrected by #34" official_spec_sheet.md` returning nothing on `main`.

**Watch out for:**

- **Do not start #17 until #36 is merged.** `epi_length` reads `SirRun::length` and
  `epi_prof_match` computes RMSE over `SirRun::profile`; both values change in this PR. `spread` is
  safe — it never moved.
- **`sir_sim`'s empty-graph return is deliberate**, not an oversight. A nodeless graph gives
  `length = 0`; every real epidemic now gives `>= 1`. The obvious tidy-up to `1` is wrong and the
  test passes either way, which is why the reason is on the function's doc comment. `decisions.md`
  2026-08-04 19:39.
- **Merge `.claude/`-touching PRs locally, never with the GitHub button** — `traps.md`. PR #36 is
  code-only so the button is fine for it.
- **A closing keyword added after a merge never fires.** #36 has it up front; do not "improve" the
  body post-merge and assume it worked. `traps.md`.
- Do not run bare `cargo fmt` — still three offenders, still #22.

**⏰ Time-sensitive:** two PRs are open and both are James's to merge — **#36** (this task) and
**#35** (spec status table). Neither has anything left for you until they land.
