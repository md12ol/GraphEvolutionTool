# Next session — 2026-08-10

Read `.claude/work/current/plan.md` first, then `.claude/work/decisions.md` and
`.claude/work/hotfixes.md`.

**Where things stand:** Issue #18 is finished and merged (PR #47, `fd0d920`) — but its task was
never closed out, so `work/current/` still holds it and `/start` will refuse to run. This machine
is fully caught up: in sync with `origin/main` at `d28dcc3`, working tree clean, **213 tests
green**. Michael intends to start a new issue this session.

**Start here: run `/done epidemic-seeding`.** The gate should pass without stopping — all three
things it checks were verified clean on 2026-08-10:

- `plan.md` has no `[ ]` or `[~]` items left. The last one, the `evaluate_batch` / `Epidemic`
  rename, was agreed at the 2026-08-09 meeting and filed as **GitHub #52**; it is a separate piece
  of work, not something this task owes.
- `issues.md` has nothing genuinely unfiled — the one `Filed: not yet` is the template placeholder.
- `hotfixes.md` has one live entry, `#[allow(dead_code)]` on `GraphEvolver::python_fitness`. It is
  **James's**, from #19, and its `Remove when:` is Michael's **#26**. Straight carry-forward.

**Then, `/start`.** Recommended: **GitHub #52**, the rename, and the reason is timing rather than
size. It is tier (1), already agreed at the meeting, a pure rename with no behaviour change — and
it touches `fitness.rs`, `sir.rs`, `common.rs` and the spec sheet, where **no branch of James's is
currently live** (`git ls-remote --heads origin` — every one of his is merged). His **#53** edits
`FitnessConfig::EpiProfMatch`, which reaches into `fitness.rs`; once he starts, the rename is
happening underneath him. The issue itself asked for "a quiet window rather than a volunteer".

Then, in priority order: **#51** (extract the argmin into `common::best_index`, tier 1, touches the
evolvers not `fitness.rs`), then **#26** (config-to-concrete-type dispatch in `GraphEvolver::run`,
tier 4, now unblocked since #19 and #29 landed — and the entry that clears James's hotfix).

**Watch out for:**
- **`cargo test` needs `LD_LIBRARY_PATH` on this machine now.** A bare `cargo test` exits 127 with
  `libpython3.11.so.1.0: cannot open shared object file`, before any test runs — #19's pyo3 work.
  Export `LD_LIBRARY_PATH` from `sysconfig`'s `LIBDIR` first; full entry in `traps.md`. Expect 213.
- **#52 amends `official_spec_sheet.md`**, so the rename PR carries the sheet change with it and
  needs a `decisions.md` entry stamped with both names. The meeting already authorised it — do not
  re-open the question, but do not push the sheet outside the PR either.
- `decisions.md` is no longer strictly chronological around the 2026-08-06/07 seam, from a union
  merge. Not corruption; do not "fix" it by editing entries in place.

**⏰ Time-sensitive:** #52's quiet window in `fitness.rs` lasts only until James starts #53.
