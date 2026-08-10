# History — Build `Config` from Python objects that serialize to TOML (#29)

Append-only session log for this task, newest session first.
Maintained by `/save`; archived by `/done`.

---

## Session 2026-08-09: PR #49 confirmed merged; task ready to close

Purely a status check — no code or docs changed beyond `plan.md`. `/start` found `work/current/`
non-empty (per its own rule, stopped rather than overwriting) and reported the open task; user chose
to close it before starting anything new.

`gh api repos/md12ol/GraphEvolutionTool/pulls/49 --jq '{merged, merged_at, merge_commit_sha, head}'`
→ `merged: true`, `merged_at: 2026-08-09T12:59:11Z`, `merge_commit_sha: 0731aa6bc2cb93c2aae0be808e0
7466bee955835`. `gh issue view 29 --json state -q .state` → `CLOSED`. `git merge-base --is-ancestor
0731aa6b HEAD` confirmed the merge commit is already in local `main` — no pull needed.
`git log --oneline` shows it at `0731aa6 Merge pull request #49 from md12ol/jsargant_python_config`.
Working tree clean except the two known untracked pre-spec-sheet files (`docs/`,
`GET GA planning session.md`).

Last plan item ticked. Proceeding to `/done pyconfig`.

## Session 2026-08-08: all five tasks built and committed; #6 closed as already-delivered

**Started with a different issue than it ended with.** `/start` found `work/current/` empty. Of the
four issues assigned to shorinbonsai, #28 and #20 are tier-6 and blocked on #27 (Michael's,
unstarted), leaving #6 and #29. Picked #6 first — and found **every item in its scope already in the
tree**, delivered as a side effect of #23 and #24: `num_chars` gone from `GenomeConfig::Sda`,
`config.example.toml` rewritten, the `1..=255` check live in `Config::validate` with two tests, and
`SdaGenome::random_with_edge_multiplicity_cap` predating all of it. Closed on the tracker with a
comment naming the delivering PRs; verified `CLOSED` and the body intact afterwards. The one
remaining line item, "the SDA dispatch arm", is not SDA-specific — no `GenomeConfig` variant is
wired to a concrete genome anywhere yet — and is #26's, which will consume what exists rather than
require reopening #6. Then started #29 properly.

### What was built, in commit order on `jsargant_python_config`

| Commit | |
|---|---|
| `2676dd4` | `get/src/py_config.rs` — pyclass mirrors of the whole schema, registered in `lib.rs` |
| `b0cd079` | TOML emission + 7 round-trip tests; `to_toml()` exposed to Python for provenance |
| `7f3abce` | `GraphEvolver.from_config`, wired through `from_toml_str` + `validate` |
| `92ccdd6` | `examples/config_builder.py`, README section, module-header example |
| `ce52556` | `config_error_to_py` — errors by Python attribute path, with the scraper guard |
| `22b9948` | `run`'s §8.1 memory-multiplication docstring |

**Tests 198 → 213.** Clippy `--all-targets -D warnings`, `cargo build --features
pyo3/extension-module` and `cargo fmt --check` all clean at every commit.

### The finding that shaped the design

`#[pyclass]` and serde's internal tagging are **mutually exclusive on the fitness enum**. pyo3
rejects a unit variant in a complex enum and directs you to `Python()`; serde rejects that tuple
variant under `#[serde(tag = "type")]`. Measured both directions in throwaway integration tests
before writing any real code. This forced the mirror **and** ruled out a `Serialize` derive, so the
TOML emission is explicit `to_toml_value` matches. Reasoning in `decisions.md` 2026-08-08 21:15;
the standing constraint is in `traps.md`.

Two other things were measured rather than assumed, both in scratch tests deleted afterwards:
`toml` 0.8.23 **reorders scalars ahead of tables**, so there is no `ValueAfterTable` problem and an
explicitly built `Value::Table` renders as a proper document; and `PathBuf`, `Option<usize>` and a
nested pyclass struct all work as complex-enum variant fields.

### Verification worth recording, because it was not assumed

- **The drift guard fires.** Added a field to `Config` and confirmed `py_config.rs` fails to compile
  with "pattern does not mention field". Reverted; `config.rs` is untouched in the branch diff.
- **The attribute-path guard fires.** Added a `crossover_rate` check to `config.rs` and confirmed
  the suite failed with `["crossover_rate"]. Add them to python_attribute_path`. Reverted.
- **The convergence test is not vacuous.** `the_python_builder_and_config_example_toml_agree` parses
  the shipped `config.example.toml` via `include_str!`; moved `num_generations` to 501 and watched
  it fail on `left: 500, right: 501`. Reverted.
- **End to end from a real wheel, not only from cargo.** `maturin build --release` → `pip install`
  into a throwaway venv → built an evolver from a Python config, printed its provenance document,
  registered a Python objective, and read back all **seven** reachable validation failures, one per
  validation site. `examples/config_builder.py` runs clean against that wheel.
- **The `cargo doc` warning is pre-existing.** Confirmed by stashing the #29 changes and re-running
  — same two warnings from `sda.rs`. Parked in `issues.md` rather than claimed as mine.

### Two mistakes made and caught, recorded so the catch is repeatable

A python splice that removed a temporary test **failed silently** (`ValueError: substring not
found` after rustfmt had reindented it), leaving a scratch test in `py_config.rs`. Caught only
because the test count read 206 where 205 was expected — the count is the check, not the exit code.
Separately, the first `from_config` wrapped `validate`'s error in `format!("invalid config: {err}")`
when `ConfigError`'s `Display` already opens with that, which would have double-prefixed every
message; caught by reading `Display` rather than by a test.

### Git manifest at save time

- **Branch `jsargant_python_config`** — 6 commits, `2676dd4`..`22b9948`, 4 files, +1562. **Being
  pushed and opened as a PR this session, on instruction.** Not merged; Michael merges.
- **`main`** — working docs only: `collab.md` **#38** (what #29 leaves for #26, and the `lib.rs`
  region we both touch), two `decisions.md` entries, one `traps.md` entry, one parked `issues.md`
  entry. Committed and pushed this session, decoupled from the code PR per the routing table.
- **Untracked and deliberately not staged:** `docs/` and `GET GA planning session.md`, the
  pre-spec-sheet files `traps.md` warns `git add -A` would sweep in. Every commit used explicit
  paths.
- `hotfixes.md` unchanged — the `#[allow(dead_code)]` on `python_fitness` is still in the tree and
  still blocked on #26. #29 did not touch it.

*Session 2026-08-08 — James, closing out #29's implementation.*
