# Plan — Align `sir_sim`'s `length` and `profile` with the amended spec §5.2 (issue #34)
_Started 2026-08-04 · last updated 2026-08-04_

Branch: `mdube_sir_conventions`. Tier (1), assigned to Michael. Blocks #17.

## Objective

`get/src/sir.rs` was built to §5.2 as it read on 2026-08-04 morning; the joint meeting that
afternoon amended §5.2 to match the legacy C++, so the code on `main` contradicts the sheet. Done
when `sir_sim` emits a terminating zero in `profile` and reports `length` inclusive of the burnout
step, the doc comments say so, and the suite is green on the new expectations.

| | Now | Required |
|---|---|---|
| Lone patient zero | `length = 0`, `profile = [1]` | `length = 1`, `profile = [1, 0]` |
| 6-node path @ 1.0 | `length = 5`, `profile = [1;6]` | `length = 6`, `profile = [1,1,1,1,1,1,0]` |
| `spread` | unchanged | unchanged |

Governing decision: `decisions.md` 2026-08-04 17:40 — Michael & James. **`spread` is not in
question**; the C++ `totInf` already agreed with it, which is what narrowed this to two values.

### Out of scope
- The three SIR objectives — **#17**, and it must not start consuming `length`/`profile` until this
  lands.
- The short-epidemic re-roll and position-indexed seeding — also #17/#18. `sir_sim` stays one
  epidemic by contract.
- `SirParams` gaining `min_epidemic_length` / `max_epidemic_retries` — **#24**, config schema.

## Tasks

- [x] Terminating zero emitted — one guard deleted at `sir.rs:158`; `length` expression unchanged.
      Evidence: 4 of 7 tests failed with exactly the old values, proving the behaviour moved.

- [x] Docs corrected — `SirRun`, the guard comment, the module doc's "only the reporting differs"
      (now false), and `legacy/README.md`'s "Where the Rust deliberately differs" section.
      Evidence: grep for the old wording returns nothing.

- [x] 4 tests updated; the other 3 passed untouched as predicted. One assertion message cited the
      superseded spec and was corrected, not just renumbered.
      Evidence: `cargo test` → 110 passed; clippy clean on `sir.rs`.

- [x] PR #36 open, `Closes #34` present **before** merge — the failure mode PR #31 hit today.
      Evidence: `gh api .../pulls/36` → open, mergeable; #34 timeline shows the cross-reference.

- [x] Status-row caveat dropped via PR #37, self-merged (trace: `collab.md` #20). The predicted
      failure did occur — both #35 and #36 merged untouched, so the sheet cited a closed issue for
      ~6 minutes. Evidence: `grep "corrected by #34" official_spec_sheet.md` → 0 on `main`.

## Open questions
- None. The empty-graph question was settled 2026-08-04: `length = 0` stays, because zero now means
  "no epidemic existed to measure" rather than "no transmission", and only a nodeless graph can say
  it. Recorded on `sir_sim`'s doc comment and on the test, since the obvious tidy-up is to make it 1.

## Out of scope
- `cargo fmt` on the tree — **#22**, still three offenders. Format only this file:
  `rustfmt --edition 2024 get/src/sir.rs` (`traps.md`).

## Follow-up this creates
- Tracked as the open task above rather than a note, so `/done` gates on it.
