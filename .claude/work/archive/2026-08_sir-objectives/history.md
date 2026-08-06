# History — Implement the three SIR objectives over sir_sim (issue #17)

Append-only session log for this task, newest session first.
Maintained by `/save`; archived by `/done`.

---

## Session 2026-08-06: closed out on a machine that had never seen the merge — 40 commits behind

**Git manifest.** One repo, `/home/mdube/GraphEvolutionTool`. Branch `main`, working tree clean
throughout. `main` was at `29b3f6a` on arrival and **40 commits behind `origin/main`**; fast-forwarded
to `ed198c4`. `mdube_sir_objectives` is still present locally and fully merged.

**Why this session existed at all.** The task was finished on 2026-08-04 and PR #40 merged the same
day, but the close-out never ran — and the other machine, which did several tasks in between, could
not run it either. `work/current/` is gitignored, so the #17 task record existed on exactly one
machine and `/done` had to happen there. Nothing was lost or duplicated; the two machines archived
disjoint tasks (`config-schema` and `mdube_format_and_readability` on the other one).

**The one `[~]`, resolved by evidence rather than inference.** PR #40 merged as `a53375e` and issue
#17 closed completed. Checked against the PR-lag trap: `git merge-base --is-ancestor 0dab610 main`
passes, so the branch head reached `main` and nothing was stranded. `cargo test` gives **135 pass / 0
fail** on `ed198c4` — up from the 127 recorded on 2026-08-04, because #22, #15 and #24 landed since.

**Docs read from `origin/main`, not from disk.** The local copies of `decisions.md`, `traps.md`,
`collab.md` and `hotfixes.md` were two days stale, and `hotfixes.md` is no longer union-merged — so
appending to the stale base would have produced a real conflict rather than a silent merge. The
fast-forward came before any doc write, deliberately.

**Reviewed the new `SessionStart` hook before pulling it in.** `.claude/hooks/pull_main.sh` (PR #44,
James's `collab.md` #30) is `--ff-only`, exits unless the branch is `main`, and has no merge, rebase
or reset path — safe. It is also the direct fix for how this session started; had it been installed,
`main` would not have been 40 behind. A proper answer to #30 is still owed.

**Swept:** the `cargo fmt` trap was deleted per its own 2026-08-06 exit condition — #43 merged and
`cargo fmt -- --check` is clean on `main`. The clippy trap was re-verified and **stands**: the same
two `generational.rs` dead-code errors, since #25 is still unbuilt. The batch-seed hotfix was checked
and carried forward, fifth cycle.

*Session logged 2026-08-06 — Michael, at the `/done` gate.*

## Session 2026-08-04: #17 built end to end; PR #40 open after #39 was opened unprompted and closed

**Git manifest.** One repo, `/home/mdube/GraphEvolutionTool`.

| Branch | State |
|---|---|
| `mdube_sir_objectives` | pushed, 3 commits, PR #40 open. Head `0dab610`, and GitHub's PR head sha matches — checked against the PR-staleness trap |
| `main` | `3d78d2b` local, **not yet pushed**: cherry-picked docs commit plus this session's `decisions.md`, `traps.md` and `CLAUDE.md` writes |

Branch commits: `c4ef054` implementation, `8285669` readability pass, `0dab610` working docs.

**What was built.** `get/src/sir.rs` gained `SirBatchParams`, `epidemic_seeds` and
`batch_epidemics` — the position-indexed seeding (`i * max_epidemic_retries + a`, never `xor`) and
the short-epidemic re-roll, epidemics sequential per §5.2. `get/src/fitness.rs` gained
`EpidemicScorer` plus `EpiSpread`, `EpiLength` and `EpiProfMatch`, each a thin reading over the
shared batch; the `SirFitness` placeholder and its `todo!()`s are gone. The seam and its rationale
are in `decisions.md` 2026-08-04 22:10.

**Validated:** `cargo test` 127 pass / 0 fail, 17 of them new (9 `sir::`, 8 `fitness::`).
`cargo fmt -- --check` adds no offenders beyond the two known. Test-merged `main` into the branch
after discovering James's #38 had landed: auto-merged, 127 still pass.

**Not validated:** the real merge of #40 — James's to run, and `main` may move again.

**Three things worth carrying forward.**

1. **Clippy cannot pass on `main`.** Two dead-code errors in `generational.rs` from the unbuilt
   #25, confirmed pre-existing by stashing and re-running on `main`. Issue #22's `Verify by:` asks
   for a clean clippy, which is not achievable today — now a `traps.md` entry.
2. **`main` moved under the branch mid-task.** James merged PR #38 (`express_and_score`, issue
   #14), which edits `fitness.rs` — the overlap `collab.md` #14 predicted. It merges clean, but it
   is why the test-merge was run rather than assumed.
3. **PR #39 was opened unprompted** on the strength of the approved plan's step 8 reading "open the
   PR", and closed at Michael's request. The remote-branch delete was blocked by the permission
   classifier and not retried; the branch was reused for #40, so it is moot. The rule is now
   explicit in `CLAUDE.md` Conventions.

**Readability pass, at Michael's request.** Plain `for` loops replaced iterator chains in
`batch_epidemics`, `epidemic_seeds`, `EpidemicScorer::mean` and `EpiProfMatch::rmse`; comments
across the two files went 347 → 290 lines, cutting restatement of the spec sheet and keeping the
warnings that stop someone breaking it. Tests gained a `slot(params, epidemic, attempt)` helper.
Now a standing convention — `decisions.md` 2026-08-04 22:12, `CLAUDE.md` Conventions.

*Session logged 2026-08-04 22:25 — Michael.*
