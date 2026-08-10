# Plan — Build `Config` from Python objects that serialize to TOML (#29)
_Started 2026-08-08 · last updated 2026-08-09_

## Objective
Let a `GraphEvolver` be built from Python objects instead of only a `config.toml` path, by having
those objects serialize to the same TOML text `Config::from_toml_str` already parses — so there is
exactly one parser/validator and the two front ends can never diverge (spec §8). Hand-written TOML
keeps working unchanged.

**Out of scope:** base-graph / bulk-data setters (#28, blocked on #27) — `issues.md`/tracker only.
Actual dispatch logic in `run` (#26, still a `todo!()` in `get/src/lib.rs`) — this task only touches
its docstring, for the memory-multiplication note the 2026-08-04 meeting assigned to #29.

## Tasks
- [x] Pyclass mirrors of the config schema — `get/src/py_config.rs` (new), registered in `lib.rs`.
      Verified: `cargo build -p get --features pyo3/extension-module`, clippy `-D warnings`, 198
      tests, fmt clean. Mirror rather than annotating `config`'s own types is forced, not stylistic
      — measured pyo3/serde conflict recorded in the module header, for `decisions.md` at `/save`.
- [x] TOML emission + 7 round-trip tests — `get/src/py_config.rs`, commit `b0cd079`. Verified: 205
      tests (from 198), clippy `-D warnings`, extension-module build, fmt clean. Drift guard is an
      exhaustive destructure with no `..`; proved it fires by adding a field to `Config` and
      watching this module fail to compile. `to_toml()` is also exposed to Python for provenance.
- [x] `GraphEvolver.from_config(...)` — `get/src/lib.rs`, commit `7f3abce`. Verified: 209 tests
      (from 205) incl. the builder-vs-`config.example.toml` convergence check, proved non-vacuous
      by perturbing one value; plus end to end from a maturin wheel in a venv. Clippy, fmt clean.
- [x] Durable usage examples — `examples/config_builder.py` (new, runnable), README section,
      `py_config.rs` header. Commit `92ccdd6`; verified by running the script against the wheel.
      Added after the 5 planned tasks, at the user's request — examples were only in conversation.
- [x] `ConfigError` → Python attribute paths — `py_config.rs` `config_error_to_py`, commit
      `ce52556`. Verified: 213 tests; all 7 reachable failures read back from a wheel. Drift guard
      scrapes `config.rs`'s own `invalid(...)` calls; proved it fires by adding an unmapped check.
- [x] `run`'s memory-multiplication note (spec §8.1) — `get/src/lib.rs`, commit `22b9948`.
      Docs only. 4 bytes/cell checked against `graph.rs` (`Vec<Vec<u32>>`), not taken from the
      issue; noted the estimate is a floor. `cargo doc` renders the table.

- [x] PR opened — **#49**, 6 commits, `Closes #29.` Verified `head.sha` == local HEAD (`22b9948`),
      `mergeable_state: clean`, body intact. **Awaiting Michael's review; never merge it myself.**
- [x] After PR #49 merges: run `/done pyconfig` to archive this task. Verified 2026-08-09: PR #49
      `merged: true` (`merge_commit_sha` `0731aa6`, an ancestor of local `main` HEAD — no pull
      needed), issue #29 shows `CLOSED`.

## Open questions
- None currently blocking. Note for awareness: collab.md #21 (can users drop in their own Rust
  objective, vs. only Python?) is unresolved and pending the joint meeting — it bears on #26, not
  this task, since #29 only builds the Python *config* front end.

## Out of scope
- `set_base_graph` and the base-graph validation checks — GitHub #28, blocked on #27 (Michael's).
- `run`'s actual config-to-concrete-type dispatch — GitHub #26 (Michael's), still `todo!()`.
