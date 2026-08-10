# Plan — GitHub #53: replace `target_profile_path` with an inline `target_profile` array
_Started 2026-08-09 · last updated 2026-08-10_

## Objective

`FitnessConfig::EpiProfMatch` loses `target_profile_path: PathBuf` and gains
`target_profile: Vec<f64>`, in both front ends, with the target compared **verbatim** — neither
C++ loading convention (patient-zero prepend, `verts / 128` rescale) is reproduced. `Config::validate`
gains the two checks the old path field made impossible. Agreed at the joint meeting of 2026-08-09;
spec §8 is already amended on `main` (`762b1d2`), so this is the code catching up.

**Out of scope:** everything else the meeting produced. #51 and #52 are Michael's; #26, #27, #21 are
his too. #28 and #20 are mine but sit at tier (6) behind his tier-4/5 work.

## Tasks

- [x] **Closed the pyconfig `/done` sweep, which had never reached `main`** — `fd9fcb4`, pushed
      direct per the routing table. `git ls-files` now returns the four archive files; the
      pre-spec-sheet docs stayed untracked. Union audits on `decisions.md` clean.

- [x] **Branched `jsargant_inline_target_profile` off `main`** at `9bba043` (not `0b3ad71` — two
      further doc commits had landed since the plan was written). `git rev-parse --abbrev-ref HEAD`
      returns the branch name.

- [x] **`config.rs` variant + `py_config.rs` mirror — done together, they are one compile unit.**
      Both tests renamed and rewritten to an inline array; `PathBuf` import dropped from both files.
      `cargo test -p get config::` → 47 passed (46 + one new), `py_config::` → 11 passed.
      This absorbed the task below on `py_config.rs`; see `history.md` for why they could not split.

- [x] **The two new validation checks** — non-empty and all-finite, in `validate_fitness`;
      `validate`'s doc comment no longer claims it never opens a path. The scraper test forced a
      third change: `python_attribute_path` needed a `target_profile` row or the field would reach a
      Python user unmapped. `cargo test -p get` → 216 passed (213 + 3), clippy and fmt clean.

- [x] **`get/src/py_config.rs` — the mirror** — folded into the item above. The constructor, the
      field and `to_toml_value` all changed; `all_four_fitness_variants_survive_the_round_trip` now
      round-trips `[0.0, 2.5, 7.0, 1.25]`. `cargo test -p get py_config::` → 11 passed.

- [x] **`config.example.toml` and `examples/config_builder.py`** — both now show an inline profile
      and say it is compared verbatim. The grep returns nothing. Went beyond the grep: built the
      module with `maturin develop` into a scratchpad venv and ran `config_builder.py` end to end —
      it emits `target_profile = [1.0, 3.0, ...]`, and both new checks surface in Python naming
      `config.fitness.target_profile`.

- [x] **The stray `seed` attribute question — answered by running it.** `config.seed = 42` raises
      `AttributeError`; the meeting's suspicion was right and #25's reply was wrong, in the safe
      direction. No code changes. Recorded in `decisions.md` 2026-08-10 17:48.

- [x] **Full verify sweep** — all three clean on `1ea2f6e`. 216 passed, 0 failed: the documented 213
      baseline plus exactly the 3 tests this task added (empty profile, non-finite element, and the
      integer-coercion test). Nothing was removed; the one test rename does not move the count.

- [x] **PR #57 open against `main`, referencing #53. Stopped there — Michael merges mine.**
      `gh pr view 57 --json` reports `OPEN`, `MERGEABLE`, body intact (41 lines, tables and fences
      survived). `main` was merged into the branch first, so it is verified against `6552d25`.

## Open questions

- None blocking. #53's body is unusually complete and the spec side already landed.

## Out of scope

- **#51, #52** — Michael's, both tier (1). #52 renames `evaluate_population`/`SirRun` and touches
  `lib.rs`; it does not touch `config.rs` or `py_config.rs`, so it will not collide with this.
- **#26** — reads `config.rs` and `py_config.rs` but does not modify them (issue #53, "Coordination").
  This change makes its dispatch *simpler*: it passes the vector straight through instead of reading
  and parsing a file.
- **The two carry-forward items** — `sda.rs`'s doc-link warning, and `python_fitness`'s
  `#[allow(dead_code)]` hotfix, which is blocked on #26. Both pre-date this task; see `hotfixes.md`.
- **The pre-spec-sheet docs** (`docs/`, `GET GA planning session.md`) — stale, must stay untracked.
