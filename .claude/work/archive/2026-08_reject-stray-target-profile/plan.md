# Plan — GitHub #58: reject `target_profile` when the objective is not `epi_prof_match`
_Started 2026-08-11 · last updated 2026-08-11_

## Objective

Spec §8 (line 996) requires the target profile be "rejected as a contradiction if supplied for any
other objective". #53 landed the first half of that clause; this task lands the second. Done means a
`[fitness]` block carrying `target_profile` under `epi_spread`, `epi_length` or `python` is an error
naming `target_profile`, `epi_prof_match` with a profile is unchanged, and the check stays a
**named-key** check rather than a general unknown-key sweep.

**Out of scope:** the Python front end (`PyFitnessConfig` makes this structurally impossible —
`target_profile` only exists on the `EpiProfMatch` variant, so there is nothing to reject; the
asymmetry is deliberate, same as the `seed` check). Any widening to other unknown keys — see the
narrowness recorded in `collab.md` #25 and pinned by an existing test.

## Tasks

- [x] 0. Branch `jsargant_reject_stray_target_profile` created off 97d9e02.
      `git rev-parse --abbrev-ref HEAD` prints it.

- [x] 1. `reject_fitness_seed` → `reject_stray_fitness_keys`, one raw-text sweep over both named
      keys — `get/src/config.rs:262`. Doc comments on it and `from_toml_str` rewritten; two stale
      references to the old name fixed in `py_config.rs`. `cargo build -p get` clean.

- [x] 2. Two tests, four cases — `config.rs:840` (three objectives in one loop) and `config.rs:872`
      (the `epi_prof_match` acceptance case, parse **and** validate). Narrowness test renamed
      `an_unknown_fitness_key_outside_the_two_named_ones_is_still_ignored`, body untouched, passes.
      231 → 233 tests, baseline measured by stashing.

- [x] 3. All three gates pass: 233 tests, `clippy --all-targets -- -D warnings` exit 0,
      `cargo fmt -p get --check` exit 0. `cargo test` needs `LD_LIBRARY_PATH` here — `traps.md`.

- [x] 4. Three commits — `bfa515b` check + tests, `d7cb289` example docs, `7fc4c1a` the
      `collab.md` #47 doc line. **PR #63** open and pushed; body re-read with `gh pr view 63 --json`,
      6 sections intact. Michael merges it — never me (`CLAUDE.md`, Pull requests).

- [x] 5. Michael merged PR #63 (`b225f30`, 2026-08-12). `gh issue list --state open` no longer
      lists #58; `b225f30` confirmed an ancestor of `origin/main`.

## Open questions

- Settled while writing: the merged function is `reject_stray_fitness_keys`, and the three
  rejection cases share one loop rather than being three near-identical tests (the plan said four
  tests; it is four *cases* in two, which #56's duplication sweep would otherwise inherit).
  Both for `/save` to record.
- Superseded by task 4: `config.example.toml:91-94` now documents the rejection and the
  named-key narrowness, landed in commit `d7cb289`. The line above was true when the plan was
  written and stale by the time the PR closed.

## Out of scope

- `PyFitnessConfig` — structurally cannot carry the contradiction. Reasoning in GitHub #58.
- A general unknown-key sweep — `collab.md` #25; would hand-roll what serde does elsewhere and start
  rejecting keys as the schema grows.
- `config.example.toml`'s flat-run problem — separate, parked as `collab.md` #48 and staged in
  `issues.md`.
