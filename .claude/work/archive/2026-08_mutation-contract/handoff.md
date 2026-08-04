# Next session — 2026-08-03 (written after PR #30 opened)

Read `.claude/work/current/plan.md` and `.claude/work/decisions.md` first (two new entries at the
tail, both stamped James 2026-08-03 21:10 and 21:12), then `.claude/work/traps.md` — it gained an
entry this session. `hotfixes.md` is still empty, which is correct.

**Where things stand:** Issue **#10** is code-complete and shipped. Branch
`jsargant_mutation_contract` is pushed, commit `a8cbf27`, and **PR #30 is open against `main`** with
"Closes #10". 103 tests green, up from 97. **Every plan item is `[x]` except the PR**, which is `[~]`
only because #10 closes on merge. The task is done bar the merge.

**Start here:** check whether **PR #30 has merged** — `gh pr view 30`. Every plan item is now done
except that one, and there is no code left to write in this task. On merge: confirm #10 actually
closed, flip the PR item to `[x]`, then run `/done mutation-contract` to archive the task. If it has
not merged, there is genuinely nothing to do here — pick up a new task with `/start` rather than
inventing work on this branch.

**Then, in priority order:**
- If review asks for changes, note that `common.rs`, `mod.rs` and `steady_state.rs` overlap
  Michael's #14/#15. Rebase rather than merge.
- **Do not start #25 (generational) in this task.** It is the next task and needs `express_and_score`
  (#14, Michael's) to exist first. Open a new task with `/start`.

**Watch out for:**

- **One `[~]`: the PR item** (unverified since 2026-08-03). It is not broken — #10 simply cannot
  close before the merge. Do not tick it on the PR being open.
- **A bare `git add -A` will commit four stale untracked docs.** New trap this session, verified:
  `docs/IMPLEMENTATION.md` is the document the spec sheet *replaced*, and it is not gitignored.
  Stage explicit paths. Do not read those files as current design.
- **Seeded output changed, and that is expected.** `random_range(1..=1)` consumes RNG state, so runs
  before and after #10 are not comparable even at the default. A test failing on a hardcoded
  seed-derived number is showing this, not a regression — `decisions.md` 2026-08-03 18:45.
- **`max_mutations` is not wired at runtime yet**, and neither is any other config field: `lib.rs`
  dispatch is entirely `todo!()`. That is issue **#26**, not an omission of #10. Said so in the PR
  body already, so do not "fix" it here.
- **Never bare `cargo fmt`** — `generational.rs` and `sda.rs` are issue #22, Michael's (`traps.md`).
- **Check `git branch --show-current` before writing any `.claude/` doc.** The persistent docs are
  tracked, so this session's `decisions.md` and `traps.md` entries live on
  `jsargant_mutation_contract` and reach Michael only when PR #30 merges.

**⏰ Time-sensitive:** nothing dated. Two things are waiting on Michael and neither blocks: PR #30
review, and `collab.md` item 14 (raised 2026-08-03 16:40), which was an FYI and never a gate.
