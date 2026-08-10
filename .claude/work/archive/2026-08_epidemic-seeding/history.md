# History — Issue #18: epidemic seeding by per-run atomic evaluation counter

Append-only session log for this task, newest session first.
Maintained by `/save`; archived by `/done`.

---

## Session 2026-08-10: `/load` confirmed clean, `/done` invoked

**No task work.** `/load` re-verified every claim in the prior session's handoff against the repo
— `main` at `d28dcc3`, working tree clean, `plan.md`'s tasks all `[x]`, `issues.md` and
`hotfixes.md` unchanged — and nothing had moved since that handoff was written. User then ran
`/done epidemic-seeding` to close the task out, per the handoff's own instruction.

**Git manifest:** `main`, clean, at `d28dcc3`. No repos other than the root.

## Session 2026-08-10: caught this machine up — 43 commits pulled, nothing lost

**No task work.** This session only reconciled this machine with `main` after three days away and
work on another machine. The #18 task itself was already complete and merged; it remains
un-archived because `/done` has not been run.

**What was behind, and why it needed care.** Local `main` was 43 behind / 0 ahead at `19d6fee`,
with **70 uncommitted lines in `decisions.md`** — the three #18 design entries written at the
2026-08-07 save and never committed. Two hazards, both checked before touching anything:

- **`main` was force-pushed on 2026-08-09** (`collab.md` #39, Settled) to strip six
  `Co-Authored-By` trailers. Verified this machine was unaffected: `HEAD` is an ancestor of
  `origin/main`, all five #18 commits (`e870260 262b4e7 3757f31 fd0d920 19d6fee`) still present,
  and `19d6fee` predates the rewrite base `0731aa6`. **No `git reset --hard` was needed** — and
  running the one #39 prescribes would have destroyed the uncommitted `decisions.md`. That
  instruction is addressed to James, whose local `main` held pre-rewrite commits.
- **14 incoming commits touch `decisions.md`**, which is `merge=union`, so a merge could interleave
  silently rather than conflict.

**Sequence, backup first:** full backup to the session scratchpad (`git bundle --all`, verified as
"a complete history"; tarball minus `target/`; loose copies of `decisions.md` and all of
`work/current/`, which is gitignored and therefore not in git at all). Then commit the pending
entries (`0edc023`), merge `origin/main` locally rather than with the button (`d28dcc3`), audit,
test, push.

**Validated, not assumed** — the merge reported no conflicts, which for a union-merged file is when
to distrust it:
- `uniq -d` clean on both `decisions.md` and `collab.md`, run against the **remote** copy after
  pushing, not just locally.
- Structure check clean — 95 headings in `decisions.md`, 26 in `collab.md`, none spliced mid-line.
- Diffed every `^## ` heading in the pre-merge backup against the remote: **0 missing**, 75 → 95.
- The three #18 entries present exactly once each, headings at column 0, stamps intact.
- `work/current/` byte-identical to its backup.
- `cargo test -p get`: **213 passed, 0 failed**. `cargo fmt --check` clean.

**One cosmetic artifact, deliberately left alone:** the 2026-08-07 entries now sit *above* James's
2026-08-06 21:03 entry in `decisions.md`. Union merges by position, not date, so the file is no
longer strictly chronological at that seam. Nothing is lost; fixing it would mean editing entries
in place, which the rules ask to be announced first.

**`cargo test` changed while this machine was away.** #19's pyo3 work means a bare `cargo test`
exits **127** with `libpython3.11.so.1.0: cannot open shared object file` before any test runs.
`traps.md` already carries the fix (James's entry, Linux/pyenv half) and it works here verbatim —
export `LD_LIBRARY_PATH` from `sysconfig`'s `LIBDIR` first. The 213 count matches what that entry
records, so this machine agrees with his measurement. His entry was not edited to say so.

**What landed while away, affecting this task's leftovers:** the 2026-08-09 joint meeting
dispositioned all four collab items raised from #18 — **#32** both renames agreed and filed as
GitHub **#52**; **#33** FYI, verified intact; **#34** skill *frontmatter* now takes a PR while the
body does not; **#35** spec §6.2 amended to best-of-final for both strategies, carried by PR #50.
Also landed: #19 (PyFitness), #29 (Python config), #25's archive, and the `Co-Authored-By` ban
moved into the repo's own `CLAUDE.md`.

**Git manifest — working tree clean, in sync with `origin/main` at `d28dcc3`.** Two commits added
this session: `0edc023` (the three #18 decisions entries) and `d28dcc3` (the merge). Backup kept at
`scratchpad/backup-2026-08-10/`, session-scoped.

## Session 2026-08-07: #18 built, reviewed, restructured, PR'd, and merged by James

**Objective met.** `EpidemicScorer::batch_seed` (the frozen stub returning `run_seed` unchanged)
is gone; batches now seed from `mix_seed(run_seed, batches_scored)`, ticked once per batch via
`next_batch_seed`. All three SIR objectives override `evaluate_population` to route through it.

**Built, in order:**
1. Branch `mdube_epidemic_seeding` off `main` at `dda6069`.
2. `sir.rs` renamed first — `batch_epidemics` → `simulate_epidemics`, `SirBatchParams` →
   `SirSampleParams`, `coin_flip_batch` → `coin_flip_sample` — because "batch" there meant one
   graph's epidemics, colliding with the graphs-batch sense used everywhere else. `e870260`.
3. The counter, `mix_seed`, and wiring it through `mean_batch` so every objective's
   `evaluate_population` ticks once regardless of batch size. `262b4e7`.
4. The end-to-end reproducibility test `the_same_run_seed_replays_every_batch_of_a_run` — #18's
   own `Verify by`, five batches in sequence plus a fresh-vs-fresh divergence check. `3757f31`.
5. `hotfixes.md`'s SIR-batch-seed entry deleted, pushed to `main` directly. `f8673dc`.

**Mid-task detour, at the user's request:** three sub-agents reviewed the file from different
angles (which constraints are load-bearing; concrete refactor proposals; how established
frameworks handle this) after repeated confusion reading the comments cold. Verdict: the seeding
machinery (atomic counter, `mix_seed`, one-seed-per-batch) is the standard common-random-numbers
pattern and stays; the wrapper layer above it (`mean`, `mean_with_seed`, `epidemics` as a public
pass-through) was ours and got removed — `EpidemicScorer` down from five methods to two. A
per-objective `reading` method was tried to deduplicate each objective's epidemic-reading closure,
then reverted the same session on request: the file's primary audience is someone copying an
objective to write their own, and the indirection cost more than the duplication it removed. The
duplication is guarded instead by `both_entry_points_use_the_same_reading`.

**Terminology fixed throughout `fitness.rs`:** "batch of graphs" replaces "generation" as the
stated unit (a steady-state mating event scores two children, not a population, §6.3); `Direction`
now names **original** and **oriented** instead of describing an unnamed sign flip. Full rationale
in `decisions.md` 2026-08-07, three entries.

**Two issues raised for the joint meeting, not resolved here:**
- `collab.md` #32 / `issues.md` — `Fitness::evaluate_population` → `evaluate_batch`,
  `SirRun` → `Epidemic`. Both sheet-named, blocked until James agrees.
- `collab.md` #35 — generational's `outcome` (from PR #46, reviewed after merging) reports the
  best of the *final* population, where §6.2 says "track the best" — divergence only reachable at
  `elite_count = 0`. Recommended resolution is amending the sheet; not a code defect.

**Also, unrelated to #18 but touched this session:** five `SKILL.md` files (`done`, `load`,
`save`, `setup`, `start`) pinned to `model: sonnet`, pushed directly, logged as `collab.md` #34.

**PR flow:** #47 opened (`get/src/{fitness,sir,config}.rs`), merged by James as `fd0d920`. His
#46 (`GenerationalEvolver`) reviewed and merged by me the same session, locally with `--no-ff`
after running the suite (167 green) rather than trusting the GitHub button.

**Validated, not assumed:** every commit above was `cargo test` green and `cargo fmt --check`
clean before being made — 154 → 163 across the branch's own commits, 176 on `main` after both PRs
landed. Final state re-verified after the last merge, not inferred from the earlier count.

**Git manifest — working tree clean, nothing uncommitted, on `main` at `19d6fee`.** Sixteen
commits landed since the branch was cut, listed oldest-first:
`e870260 262b4e7 3757f31 f8673dc 6e91eae` (this branch) · `7de4a66 a30422e ab68796 349399e 841e79d`
(James's #25, merged in) · `74de0b5` (merge of #46) · `011480d 6b54fc0` (skills pin) ·
`fd0d920` (#47 merged by James) · `968da44` (collab #35) · `19d6fee` (final pull-merge).

**Two published artifacts, not part of the docs system** — an early one accumulated the day's
refactor history and proposals; a fresh one at a new URL answers only "what calls this file, what
happens inside, what comes out" for a cold reader. Neither is referenced from `.claude/work/`, and
neither needs to be — the file's own comments now carry the same explanation.
