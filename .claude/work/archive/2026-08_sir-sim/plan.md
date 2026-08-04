# Plan — Implement `sir_sim`: one epidemic returning length, spread and profile (issue #16)
_Started 2026-08-04 · last updated 2026-08-04_

Branch: `mdube_sir_sim`.

## Objective

Add a new module `get/src/sir.rs` providing `sir_sim(graph, params, rng) -> SirRun`, per
`official_spec_sheet.md` §5.2 and GitHub #16: one SIR epidemic with a one-timestep infectious
period, returning `length`, `spread` and `profile` together. Done when the module exists, is
covered by deterministic tests, and `cargo test` is green.

**Chosen because it collides with nothing James owns.** `collab.md` #14 puts him on #10, which
touches `config.rs`, `config.example.toml`, `genome.rs`, `edge_edit.rs`, `evolver/common.rs`,
`evolver/mod.rs`, `evolver/steady_state.rs`; #14/#15 add the same evolver files; `traps.md` records
`generational.rs` as his live work. This task touches one new file plus a one-line `lib.rs` edit.

### Out of scope
- The three objectives over `sir_sim` — issue **#17**, and it edits `fitness.rs`.
- The atomic evaluation counter / CRN seeding — issue **#18**.
- `SirFitness` at `get/src/fitness.rs:107-129`, its `todo!()`s, and the `SirParams` config struct
  (**#24**). `sir.rs` defines its own plain params struct; wiring it to config is #24's job.

**Reference implementation:** `legacy/Graph.cpp` `Graph::SIR` — Michael confirmed 2026-08-04.
Mechanics carried over verbatim; reporting follows spec §5.2. Divergences raised as `collab.md` #15.

## Tasks

- [x] `get/src/sir.rs` created with `SirParams` / `SirRun`; `pub mod sir;` added at
      `get/src/lib.rs:6`. Evidence: `cargo test` builds and runs the module.

- [x] `sir_sim` implemented at `get/src/sir.rs:73` — ported state machine, exposure accumulation,
      and the combined `1 - (1 - rate)^exposure` draw. Evidence: 7 tests pass.

- [x] 7 tests in `get/src/sir.rs:167+` — path burn-through, isolated p0, zero rate, disconnected
      component cap, parallel-edge banding, unset p0, empty graph.
      Evidence: `cargo test` → **104 passed, 0 failed** (97 prior + 7 new), 2026-08-04.

- [x] `gh issue view` quirk recorded in `.claude/CLAUDE.md` "Filing issues".
      Evidence: the `--json title,body` workaround is the documented read path there.

- [x] `legacy/` tracked instead of gitignored — `Graph.cpp/.h`, `SDA.cpp/.h`, `main.cpp` + README.
      Evidence: `git check-ignore` returns nothing for `legacy/*`; commits `0212b45`, `94ab70e`.

- [x] PR #31 opened and **merged by James 2026-08-04 12:38 UTC** (`4c85cd0`), 7 commits, 13 files.
      Evidence: `gh api .../pulls/31` → `merged=true`; `origin/main` contains `32b7839`.

- [x] Issue #16 closed by hand 2026-08-04 15:42 UTC, with a comment recording the two follow-ups.
      Evidence: `gh api .../issues/16 --jq .state` → `closed`; comment body re-read and intact.

- [x] Stranded docs landed on `main` via PR #32, self-merged under the rule's exception and traced
      in `collab.md` #18. Branch `mdube_sir_sim` deleted, local and remote.
      Evidence: on `main`, `grep -c 2026-08-04 .claude/work/decisions.md` → 4; tree clean.

- [ ] Decide whether `sir_sim` should be `pub(crate)` or stay `pub`, once #17 shows what the
      objectives actually need from it. Currently `pub`.
      **Verify by:** #17 compiles against whichever visibility is chosen.

- [ ] After the meeting settles `collab.md` #15: file the staged follow-up in `issues.md`
      ("Align sir_sim's length and profile…"), assigned to Michael, then do the work or close it
      as no-change. **This is the whole safety net for merging #31 early — if it is skipped, the
      convention correction is lost.**
      **Verify by:** the issue exists in the tracker, or a `decisions.md` entry records no-change.

## Done outside this task's objective, recorded so `/done` does not lose it
- [x] Reviewed James's PR #30 against spec §4 clause by clause and merged it locally — 110 tests,
      docs audit clean. Issue #10 auto-closed. Detail in `history.md`.
- [x] Two union-merge traps measured and written to `traps.md` + `CLAUDE.md`; the code-vs-docs
      branch/PR routing rule added. Pushed direct to `main` (`5e38b55`), per that rule.

## Open questions
- **`collab.md` #15 — `length` and the trailing zero.** Ours follows spec §5.2; the C++ counts the
  burnout step. `spread` is **not** in dispute — `legacy/Graph.cpp`'s `totInf` matches ours exactly.
  Michael leans C++; decided at the meeting. Not blocking — #31 merged as-is with a staged follow-up.
- **`collab.md` #17 — short-epidemic re-rolls (`mepl`/`rse`).** In the C++, in neither the sheet nor
  any issue. A biased resample, so `num_epidemics` is not a substitute. Belongs in #17, not `sir_sim`.
- **`collab.md` #16 — `dyn` vs match for the fitness dispatch axis.** Cheap now, a rewrite after #26.
- **`collab.md` #19 — is announcing an in-place doc amendment a rule or a courtesy?** The routing
  half is settled; this half is not.
- **Constraint on all three of #15/#16/#17:** #17 must not start consuming `length` or `profile`
  until #15 lands.
- **`CLAUDE.md`'s "an agent never merges a PR at all" is too absolute** — it was overridden within
  hours, twice, both times correctly. Reword to "never merges unprompted" at the meeting.

## Out of scope
- `cargo fmt` on the tree — issue **#22**, blocked on James's tree being clean (`traps.md`).
  Format only the new file: `rustfmt --edition 2024 get/src/sir.rs`.
