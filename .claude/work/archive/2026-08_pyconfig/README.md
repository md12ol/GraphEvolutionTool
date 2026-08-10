# Archived task — pyconfig

**Objective:** Let a `GraphEvolver` be built from Python objects instead of only a `config.toml`
path, by having those objects serialize to the same TOML text `Config::from_toml_str` already
parses — one parser/validator, two front ends that can never diverge (spec §8). Closes GitHub #29.

**Span:** 2026-08-08 → 2026-08-09 (two sessions).

**Outcome:** Delivered in full. Pyclass mirrors of the config schema, TOML emission with round-trip
tests, `GraphEvolver.from_config(...)`, a runnable `examples/config_builder.py` and README section,
and `ConfigError` → Python attribute-path mapping — plus `run`'s memory-multiplication docstring
note assigned to #29 at the 2026-08-04 meeting. Landed as six commits on `jsargant_python_config`,
opened as PR #49, merged by Michael 2026-08-09 (`0731aa6bc2cb93c2aae0be808e07466bee955835`). Issue
#29 closed. 213 tests at close, up from 198 at task start. Verified on Linux only (noted alongside
#19 in `collab.md` #37).

**Left behind, outliving the task (unaffected by its close, both pre-existing):**
- `issues.md` — Parked: `cargo doc` warns twice on a private intra-doc link in `sda.rs` (cosmetic,
  pre-existing, unrelated to #29).
- `issues.md` — Ready to file, blocked: renaming `evaluate_population`→`evaluate_batch` and
  `SirRun`→`Epidemic`, blocked on the joint meeting (`collab.md` #32) since both names are in the
  spec sheet.
- `hotfixes.md` — `#[allow(dead_code)]` on `GraphEvolver::python_fitness` (`get/src/lib.rs`),
  blocked on GitHub #26 (Michael's, unstarted). #29 deliberately did not touch it.

**Handoff for #26 (Michael's next work in the same file):** `collab.md` #38 records what #29
leaves for #26 to build on — `from_config`'s validated-`Config` path, the still-live
`python_fitness` hotfix, and the `max_cores` docstring note that's ahead of #20's actual signature.
