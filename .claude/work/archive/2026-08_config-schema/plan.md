# Plan — Issue #24: bring `config.rs` in line with the spec's schema
_Started 2026-08-05 · last updated 2026-08-05_

## Objective

Make `get/src/config.rs` parse the schema `official_spec_sheet.md` §7 actually specifies:
`FitnessConfig` gains the four real variants with the SIR block flattened, the SIR parameters gain
`num_epidemics` / `min_epidemic_length` / `max_epidemic_retries` and lose `seed`, and
`GenomeConfig::Sda` loses `num_chars`. `config.example.toml` is rewritten to match and still parses.

GitHub **#24**, tier (1) — depends on nothing. Spec §7, §5.2, §3.2.

**Out of scope — `Config::validate`.** Spec §7 lists a dozen constraints (`num_epidemics >= 1`,
`patient_zero < network_size`, …) and **none of them belong here**: they are GitHub **#23**, also
James's, tier (2). #24 changes what *parses*; #23 adds what *validates*. Keeping them apart is what
lets #24 land without waiting.

## Tasks

- [x] Branch `jsargant_config_schema` off `main` at `252347d`.

- [x] Both shape questions settled by James on 2026-08-05, before any code: the shared struct keeps
      the issue's name **`SirParams`**, and `epi_prof_match` takes a **path** to a profile file.
      Both went against the recommendation offered; see "Settled" below and `decisions.md`.

- [x] `collab.md` **item 24** raised — the `Profile*.dat` format, its patient-zero prepend and the
      `verts / 128` rescale (`legacy/main.cpp:370-388`). `uniq -d` audit clean after fixing a bare
      code fence the entry introduced.

- [x] `FitnessConfig` → `EpiSpread` / `EpiLength` / `EpiProfMatch` / `Python`, SIR block flattened,
      `EpiProfMatch` carrying `target_profile_path` (a path, never opened here).
      Verified: 4 new parse tests, including `type = "python"`, which the old enum could not parse.

- [x] `SirParams` carries the five fields and **no `seed`**; both retry defaults use explicit `fn`s.
      Verified: `omitted_retry_settings_default_to_the_cpp_constants` gets 3 and 5, not 0.

- [x] `GenomeConfig::Sda` drops `num_chars`; `SdaGenome::random` keeps its own argument, untouched.
      Verified: `grep -n num_chars get/src/config.rs` returns nothing; both SDA tests still pass.

- [x] `config.example.toml` rewritten — `[fitness]` block, two commented objective alternatives, and
      the SDA block. Verified: `the_example_config_parses` passes against it.

- [x] Full verify pass. `cargo test -p get` **133 pass**, against a **127** baseline captured by
      `git stash push -- get/src/ config.example.toml` (both paths — stashing only `get/src/` leaves
      an example the old enum cannot parse). Clippy `--all-targets` byte-identical to that baseline:
      the same 2 dead-code warnings from the unbuilt #25. `rustfmt --edition 2024 --config
      skip_children=true` touched no other file.

- [x] `max_mutations` — verified only, not reimplemented. `an_omitted_max_mutations_defaults_to_one`
      still passes.

- [x] **Stray-`seed` criterion dispositioned** — the one part of #24 not deliverable as written.
      Documented, not silently dropped: test `an_unknown_fitness_key_is_ignored_rather_than_rejected`
      pins the behaviour, `decisions.md` 2026-08-05 15:47 carries the reasoning and the two rejected
      alternatives, `collab.md` **25** hands the check to #23's `validate`, and
      `config.example.toml` gains a migration note. Decided by James 2026-08-05.

- [x] Working docs pushed to `main` — `ac1b025`, 8 files, docs only. Verified: `git show --stat`
      lists only `.claude/` paths; `uniq -d` clean on all five, and collab items 24/25 confirmed
      intact by heading structure, which `uniq -d` cannot check (item 23's lesson).

- [x] Branch moved onto `ac1b025` — a pointer move, since it carried no commits of its own.
      Verified against a **freshly recaptured** baseline, not the dead 127: **128 → 135** tests
      (+7), clippy `--all-targets` `diff`-identical to baseline, rustdoc's 4 warnings all
      pre-existing in `sda.rs`/`lib.rs` with none in `config.rs`, rustfmt touched no other file.

- [x] Code committed as `39c408a` (2 files, +245/−18), branch pushed, and **PR #42** opened against
      `main`, authorized 2026-08-05. Verified from the remote: `state: open, mergeable: true`,
      1 commit, 2 files; body diffed byte-for-byte against the source (only GitHub's trailing
      newline differs), so the tables, fences and `§` all survived.

- [x] **Michael reviewed and merged #42**, as `988457e`. Verified from the remote 2026-08-05:
      `state: closed, merged: true`, and issue **#24 closed** 2026-08-05T22:17:43Z — the body's
      `Closes #24.` did the work, so nothing is owed here.

- [x] Task closed out after the machine crash: local `main` fast-forwarded 26 commits to `e42ffde`,
      branch `jsargant_config_schema` deleted local and remote after `git branch --merged` confirmed
      it. Verified: `cargo test -p get` **135 pass** on `e42ffde`. Detail in `history.md`.

## Settled — decided 2026-08-05, before any code

- **The config struct keeps the issue's name, `SirParams`** — so `config::SirParams` and
  `sir::SirParams` (`sir.rs:33`) coexist with different fields. Legal Rust; the hazard is a reader
  conflating them, especially at #26's dispatch which touches both. **Mitigation is required, not
  optional:** a doc comment on the config type naming the other and saying which is which. Do not
  make `sir.rs`'s types `Deserialize` — its doc comment rejects exactly that.

- **`epi_prof_match` takes `target_profile_path`, not an inline array.** `config.rs` never opens it.

## Open questions

None blocking. The profile **file format** is a real gap but is #26's to consume, so it goes to
`collab.md` rather than holding up this task — see the task above.

## Out of scope

- **`Config::validate` and `Config::from_path`** — GitHub #23, James's, tier (2). `from_path` stays
  a `todo!()` here (`config.rs:139-142`).
- **Config-to-concrete-type dispatch**, including deriving `num_chars` from the edge cap — #26,
  Michael's.
- **`collab.md` #21** (open, Michael's: drop-in Rust objective files). Decided 2026-08-05 to build
  `FitnessConfig` to the sheet as written, exactly as Michael did for #17. If #21 resolves the other
  way the change is **additive** — one more variant — so it is noted here rather than raised as a
  new collab item.
