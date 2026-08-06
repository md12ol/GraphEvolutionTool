# Plan — Issue #23: `Config::from_path` and one `Config::validate` for both front ends
_Started 2026-08-05 · last updated 2026-08-06_

## Objective

Implement `Config::from_path` (today a `todo!()` at `get/src/config.rs:208`) and add a single
`Config::validate` carrying every constraint spec §7 lists, so the TOML and Python front ends are
validated by the same function. Validation **returns an error naming the field and its constraint —
it never panics**, because a bad config is a user mistake and a panic crossing the FFI reaches the
user as an opaque `PanicException`.

GitHub **#23**, tier (2) — depends on #24, which merged as `988457e`. Spec §7; issue amended
2026-08-04 at the joint meeting to add the two re-roll checks.

**Out of scope, deliberately:**

- **Base-graph node count and cap narrowing** — §7's last validation bullet, but it belongs to
  `set_base_graph`, GitHub **#28**. #23's own check list omits it; do not add it here.
- **The Python exception mapping.** `validate` returns a Rust error; turning that into a real
  Python exception is the FFI work in **#19**/**#29**. This task proves the error *shape* is
  mappable, not that it maps.
- **Config-to-concrete-type dispatch** — #26, Michael's. `validate` must not build engine types.
- **The evolvers' `assert!`s stay.** Spec §7 keeps them as backstops for direct Rust use. This task
  makes a config-driven run never reach one; it does not delete them.

## Tasks

- [x] Branch `jsargant_config_validate` off `main` at `05e54a7`.

- [x] `ConfigError` gains `Validation { field, constraint }` plus `Display` and
      `std::error::Error`, which it had neither of. Verified: `a_validation_error_names_both_the_field_and_its_constraint`.

- [x] `Config::from_path` implemented — reads, delegates to `from_toml_str`, then **validates**
      (it is the TOML front end). Verified: loads `config.example.toml` from disk; a missing file
      gives `ConfigError::Io`, an invalid one gives `Validation`.

- [x] `Config::validate` — top-level checks. Verified: 4 tests, one per check, each asserting the
      `field` rather than message prose.

- [x] `Config::validate` — `tournament_size >= 4` steady-state only. Verified:
      `the_tournament_floor_of_four_applies_to_steady_state_only` proves 3 passes generational
      and fails steady-state.

- [x] `Config::validate` — genome checks; the weights delegate to the existing
      `EdgeEditOperationWeights::validate()` rather than restating it. Verified: 2 tests.

- [x] `Config::validate` — SIR checks, skipped entirely for `Python`. Verified: 5 tests, including
      `min_epidemic_length = 1` **passing** and `patient_zero = 99` passing at `network_size = 100`.

- [x] Stray `[fitness] seed` rejected in the TOML path, via a loose `toml::Value` parse.
      **Superseded #24's `an_unknown_fitness_key_is_ignored_rather_than_rejected`**, which pinned
      the opposite; the replacement pair pins both the rejection and the check's deliberate
      narrowness. Needs a `decisions.md` entry and an answer inside `collab.md` #25 — for `/save`.

- [x] Full verify pass. **154 tests** (was 135, +19), clippy `--all-targets` `diff`-identical to a
      baseline captured on the clean tree, rustdoc unchanged at 4 pre-existing warnings with none
      in `config.rs`, `rustfmt --check` clean and no other file touched.

- [x] `get/src/lib.rs:30` changed from `{err:?}` to `{err}` — a Debug-formatted `ConfigError` would
      reach Python as `Validation { field: ... }` instead of the message §7 requires. One line,
      outside `config.rs`; flagged because `lib.rs` is #26's file. For `/save`.

- [x] Committed (`5fd8dbc` code, `2c590f4` the `lib.rs` line), both pushed, and **PR #45** opened
      against `main`, authorized 2026-08-06. Verified from the remote: `state: open,
      mergeable: true`, 2 commits, 2 files, +528/−18; body byte-identical to source apart from
      GitHub's trailing newline, so the 5 `§`, 7 table rows and `Closes #23.` all survived.

- [x] Docs pushed to `main` as `ed198c4` — 2 decisions, `collab.md` #25 answered, `traps.md`
      clippy entry refined. Verified: `git show --stat` lists only `.claude/` paths; all five
      `uniq -d` audits clean.

- ~~Michael reviews and merges #45.~~ **Struck, not ticked — it was never a task of mine.** PR #45
  opens `Closes #23.` (verified on the remote 2026-08-06), so the merge closes the issue with
  nothing owed here. Same disposition as #15 and #24; reasoning in `decisions.md` 2026-08-05 15:09.

## Open questions

- **None.** Both shape questions were settled 2026-08-05 before any code — the error variant is a
  struct, and the `seed` check goes in the TOML path. Both are now recorded in `decisions.md`
  (2026-08-06 00:05 and 00:07).

## Out of scope

- **Base-graph validation** (§7's node-count and cap-narrowing bullet) — GitHub **#28**.
- **Python exception mapping** — **#19**/**#29**; this task only fixes the error shape.
- **Dispatch onto concrete engine types** — **#26**, Michael's.
- **`collab.md` #24** (the `Profile*.dat` format) — belongs to #26 and is awaiting Michael.
  `validate` must **not** open `target_profile_path`: parsing and validating a config stay pure,
  and the file's format is not settled.
- **`collab.md` #27** (`Swap`'s degree floor, `> 2` vs the Java's `>= 2`) — unrelated to config,
  and a spec change either way, so it needs a joint meeting rather than a task here.
