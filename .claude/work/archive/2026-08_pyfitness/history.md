# History — Issue #19: `PyFitness` adapter and `set_fitness_function`

Append-only session log for this task, newest session first.
Maintained by `/save`; archived by `/done`.

---

## Session 2026-08-08 (load): PR #48 merged by Michael, `main` fast-forwarded

`/load` at the start of this session found `origin/main` 9 commits ahead of the last save's
recorded `90b624b` — **Michael merged PR #48** as `32ceb11` (2026-08-08T13:57:26Z, a plain merge
commit carrying all 8 branch commits in order). Issue #19 is `closed`.

Verified rather than assumed: the merge's own diff, and a fresh `uniq -d` audit on all five docs
read from `origin/main`, both clean. The `.claude/work/*.md` divergence the last save flagged
(branch had `decisions.md`/`hotfixes.md`/`issues.md` content, `main` had `collab.md`/`traps.md`
content) reconciled exactly as predicted: `decisions.md` +97, `hotfixes.md` +20 (the
`#[allow(dead_code)]` entry), `issues.md` +1 net (a staged-then-removed entry cancelling out).
Confirmed **#26 is still `open`, unstarted** — the hotfix's `Remove when:` condition genuinely has
not moved.

Local `main` fast-forwarded to `32ceb11` (`--ff-only`, non-destructive), then switched back to
`jsargant_pyfitness` to leave the working branch as `/load` found it. No code or doc edits this
entry — a verification session only, ahead of `/done`.

### Git manifest — 2026-08-08

- **`main`** at `32ceb11`, matching `origin/main`. `jsargant_pyfitness` untouched at `b1f8557`,
  fully merged.
- Working tree clean but for `docs/` and `GET GA planning session.md`, unchanged again.

*Session logged 2026-08-08 — James, at the `/done` gate.*

## Session 2026-08-08: remaining three tasks landed, then pushed and opened as PR #48

Continuation of the 2026-08-07 session, same conversation. `pyproject.toml`, the GIL-release note,
the full verify pass, a `collab.md` heads-up, and the PR — all eight plan tasks are now `[x]`.

### `pyproject.toml` — resolved rather than filed

Root-level manifest, `manifest-path = "get/Cargo.toml"` (the crate is a workspace member),
`features = ["pyo3/extension-module"]` (the only place left to supply it, since `get/Cargo.toml`
deliberately drops it). Verified past "it builds" — installed the wheel into a throwaway venv
(`$SP/verify_venv`, never the real pyenv environment) and drove the actual boundary: `import get`,
construct `GraphEvolver` against a `type = "python"` config, register a callable, both rejections
arrive as `ValueError`. First time anything in GET has been callable from Python.

**A claim measured false while writing it.** The first draft of the `features` comment asserted a
featureless wheel would fail to import. Tested by building one and installing it: identical wheel
on this Linux/pyenv setup — same 75 undefined `Py*` symbols via `nm -D --undefined-only`, no
`libpython` in `ldd`, imports fine. Comment rewritten to say what was actually observed, plus an
explicit warning against reading a green Linux import as proof the line is unnecessary — kept for
macOS/Windows linkers and because it is the documented maturin configuration, not because Linux
needs it. `decisions.md` 2026-08-07 22:10 has the full record; `.claude/reference/pyo3-maturin.md`
§3 rewritten from "what GET lacks" to what it now has.

The `issues.md` entry staged for this at the last save was removed as resolved, not filed.

### Task 6 — the `#26` heads-up, commit `b1f8557`

Two notes above `run`'s `todo!()`: use `python_fitness()` rather than the field directly, and
release the GIL with `Python::detach` around the evolve loop. Checked `detach` against pyo3 0.27's
own source (`marker.rs`) before writing it down — `allow_threads` carries
`#[deprecated(since = "0.26.0")]`.

### Task 7 — full verify pass, commit `b1f8557` (no code change, verification only)

- **198 tests**, and this time the delta was **counted from the diff**, not carried forward as a
  remembered number: `git diff cf00d04..HEAD | grep -c '#\[test\]'` → 22, `176 + 22 = 198`. Doing
  this arithmetic properly caught that `7115e2e`'s commit message says "11 tests" for `PyFitness`
  when the actual count is 10 — left uncorrected in the commit, corrected in `plan.md`.
- Clippy byte-identical to the pre-edit baseline; `-D warnings` exits 0; `cargo fmt --check` clean
  tree-wide; rustdoc unchanged at 4 pre-existing warnings, 0 unresolved links;
  `--features pyo3/extension-module` still builds; the wheel re-verified end to end after the
  `pyproject.toml`/task-6 changes, not just once at the start.

### `collab.md` #37, pushed to `main` as part of `90b624b`

The heads-up Michael needs before pulling #19: `extension-module` moving out of `[dependencies]`
changes how his `cargo test` builds, verified on Linux only. States the mechanism, the
`LD_LIBRARY_PATH` requirement, the false-claim correction (so he doesn't need to rediscover it), and
a fallback (gate pyo3 tests behind a cargo feature) if it breaks for him. Written and pushed to
`main` — required switching off the feature branch first, since `.claude/work/collab.md` and
`traps.md` are tracked and the branch was cut before this session's `main` commits existed.

### PR #48 opened, `jsargant_pyfitness` → `main`

`Closes #19.` in the body. Verified on the remote rather than assumed: `state: open`,
`mergeable: true`, `head.sha` equal to local `HEAD` (`b1f8557`), body diffed identical to its
source file but for GitHub's trailing newline. Eight commits, one per verified step.

### Git manifest at save — 2026-08-08 ~00:35 EDT

- Branch **`jsargant_pyfitness`** at `b1f8557`, pushed, **PR #48 open**, awaiting Michael.
- **`main`** at `90b624b`, pushed, in sync with `origin/main`.
- **The branch and `main` disagree on `.claude/work/*.md`, by design and expected**: the branch
  carries `decisions.md`/`hotfixes.md`/`issues.md` content from this task (97/20/1 lines, per
  `git diff --stat cf00d04..jsargant_pyfitness -- .claude/work/`) that `main` does not have yet;
  `main` carries `collab.md` #37 and two `traps.md` entries the branch does not have. Both files
  union-merge (`decisions.md`, `collab.md`) or don't conflict (`traps.md`, untouched by the branch),
  so this reconciles cleanly whenever #48 merges — `traps.md`'s "docs split across branches" entry
  names this exact situation. Re-run the `uniq -d` audit after that merge regardless.
- Working tree clean but for the two long-standing untracked files (`docs/`,
  `GET GA planning session.md`), unchanged again this session.

*Session logged 2026-08-08 ~00:35 EDT — James.*

## Session 2026-08-07: five of eight tasks landed — harness, `PyFitness`, the box impl, the setter, the seam

Planned and worked in one session. Branch `jsargant_pyfitness` off `main` at `cf00d04`, clippy
baseline captured clean (0 lines — the first task on this repo to start from a genuinely empty
baseline, per #25's clippy-trap retirement). Five commits, all pushed to nothing yet — the branch
itself is local only, nothing has been pushed or opened as a PR.

### What was built, and what each verify actually checked

- **`6e2d262`** — `extension-module` moved out of `[dependencies]`; `auto-initialize` added as a
  dev-dependency. Verified both directions: `cargo test -p get --lib` links and runs a real
  `Python::attach` call (177 tests, up from 176 — the new smoke test); `cargo build --features
  pyo3/extension-module` still produces the real module. Needs `LD_LIBRARY_PATH` at test time on
  this machine — a pyenv Python, not on the default loader path; confirmed the failure mode too
  (`exit 127`, `cannot open shared object file`) so the trap entry states both sides accurately.
- **`7115e2e`** — `PyFitness`. 11 tests, including one that had to be re-verified for a reason
  worth recording precisely: deleting the `evaluate_population` override to check the test wasn't
  vacuous didn't make the test *fail* — it **hung**, 2 minutes, no message, because the trait's
  default rayon fan-out deadlocks against a GIL the calling thread already holds. Restored from a
  scratchpad copy; confirmed 187 tests green afterward.
- **`aa92a09`** — `impl Fitness for Box<dyn Fitness>`. Two tests, both confirmed non-vacuous the
  same way: removed `evaluate_population`, then `direction`, re-ran each time, watched the specific
  failure each omission produces. The `direction` case is visible as data in the engine-path test —
  scores come back unnegated, `[1.0..5.0]` instead of `[-1.0..-5.0]`, which is the "runs backwards"
  failure spec §8 warns about, made concrete.
- **`6e68bc1`** — `set_fitness_function`. Cost two test-fixture cycles from guessing the config
  schema (`num_operations` instead of `gene_length`; then top-level keys placed after a `[table]`
  header, which TOML silently attributes to that table) instead of reading `config.rs` and
  `config.example.toml` first. Corrected after the second failure, and by explicit instruction for
  the remainder of the session: verify structure against the real file before writing tests against
  it, not before.
- **`58e5781`** — the `python_fitness` seam. Delivers #19's second verify-by directly. Clippy flagged
  it as dead code (nothing but its own tests calls it, since #26 doesn't exist) — `#[allow(dead_code)]`
  with a `hotfixes.md` entry, rather than losing the `-D warnings` gate #25 just restored.

### Also this session

- **`.claude/reference/pyo3-maturin.md`**, new — outside `work/` deliberately, so it can't be
  mistaken for a churn list or inherit a merge driver. Separates what was measured on this repo from
  what came from `graph_refiner` (James's separate pyo3+maturin project, supplied as a reference).
  Pointer added to `CLAUDE.md`.
- **Found, not yet acted on beyond staging:** GET has no `pyproject.toml` anywhere in the repo
  (confirmed by search) — it cannot currently be built or installed as a Python package at all. This
  is the real distance between #19's unit-tested adapter and the issue's literal "drives a full run
  end to end." Staged in `issues.md`, unfiled, after being raised and left unanswered for one
  exchange — caught by this save's sweep.
- **Union-merge near-miss, caught by this save's own audit, not by writing carefully the first
  time:** four `decisions.md` entries this session all closed with the byte-identical stamp
  `*#19 · recorded 2026-08-07 — James, during the PyFitness implementation.*` — exactly the
  collision `CLAUDE.md` warns dedupes silently on a union merge. Caught by running the `uniq -d`
  audit before finishing, not before writing; fixed by giving each heading a distinct `HH:MM` and
  each stamp its own wording.

### Git manifest at save — 2026-08-07 ~21:00 EDT

- Branch **`jsargant_pyfitness`** at `58e5781`, **not pushed**. `main` untouched, at `cf00d04`,
  matching `origin/main`.
- Uncommitted: this save's entries in `decisions.md`, `issues.md`, `traps.md`. `plan.md` and
  `history.md` are never committed at all — `.gitignore:16` excludes all of `work/current/`, per-person
  by design.
- Untracked and deliberately left alone: `docs/`, `GET GA planning session.md`.

*Session logged 2026-08-07 ~21:00 EDT — James.*
