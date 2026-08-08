# pyfitness — GitHub #19

**Objective:** let a user's Python callable act as a `Fitness` objective — a `PyFitness` adapter over
a registered callable plus its `Direction`, and `set_fitness_function` to register them on
`GraphEvolver` before a run. Batched contract only: one call per batch, never per individual. Spec
§5, §8.

**Dates:** 2026-08-07 to 2026-08-08. Two sessions, one continuous conversation.

## Outcome

Shipped as **PR #48** — 8 commits, one per verified step — merged by Michael as `32ceb11` on
2026-08-08T13:57:26Z. Issue #19 closed by the body's `Closes #19.` **198 tests**, up from 176, the
delta counted from the diff rather than remembered.

Four pieces: `PyFitness` (both trait methods routing through an inherent `score_batch`, so removing
an override cannot become infinite recursion — the trap `collab.md` #33 documents);
`impl Fitness for Box<dyn Fitness>` forwarding every method including both defaulted ones;
`set_fitness_function` with three rejections, the useful one being registration against a config
that did not select `python`; and `python_fitness`, the seam #26's dispatch calls.

**Two things landed that were not on the original plan.** A root **`pyproject.toml`**, built rather
than filed on instruction — GET could not previously be built or installed as a Python package at
all, and this is what makes `import get` work for the first time. And **`.claude/reference/`**, a new
documentation lifetime for notes on how a dependency behaves, deliberately outside `work/` so it
cannot be mistaken for a churn list or inherit a merge driver.

## Two findings that outlived the code

- **Calling Python from inside a rayon closure deadlocks — it does not merely run slowly.** Found by
  deleting `PyFitness`'s `evaluate_population` override to check the batching test was not vacuous;
  the suite hung for two minutes with no failure message rather than failing. Spec §8 argues the
  batched contract on performance grounds; the measured consequence is worse than the stated one,
  and it is why the `Box` impl must forward `evaluate_population` explicitly. `traps.md`,
  `.claude/reference/pyo3-maturin.md` §2.
- **A claim written into `pyproject.toml`'s own comment, measured false the same session.** It said a
  wheel built without `features = ["pyo3/extension-module"]` would fail to import. It does not —
  same 75 undefined `Py*` symbols, no libpython in `ldd`, imports fine on Linux. The line stays for
  macOS/Windows linkers; the comment now says what was observed, plus a warning against reading a
  green Linux import as proof it is unneeded. `decisions.md` 2026-08-07 22:10.

## Left behind, deliberately

- **`hotfixes.md`: `#[allow(dead_code)]` on `GraphEvolver::python_fitness`.** Now committed and in
  every tree. #26 is its only non-test caller and is still `open` and unstarted, verified at this
  gate. Removing it early would cost the `cargo clippy -- -D warnings` gate that #25 spent a task
  restoring. **Carried forward.**
- **`issues.md`: the `evaluate_population` → `evaluate_batch` / `SirRun` → `Epidemic` rename**,
  Michael's, still **unfiled** and blocked on the joint meeting because both names are in the spec
  sheet. Third gate it has been carried through.
- **`collab.md` #37**, the heads-up that this PR changes how `cargo test` builds on Michael's
  Windows machine — verified on Linux only. He merged #48 without replying to it, and **a merge is
  not a reply**: left **Open** so the fallback offer (gate the pyo3 tests behind a cargo feature)
  stays visible if it does break for him.
- **`collab.md` #35, #36** await the joint meeting; **#27** still awaits James, sixth gate.

## Worth reading if you touch this code

`.claude/reference/pyo3-maturin.md` — why `extension-module` cannot live in `[dependencies]` (it
breaks `cargo test`'s linking outright), the `LD_LIBRARY_PATH` requirement that follows, and what
does and does not transfer from `graph_refiner`. Then `decisions.md` 2026-08-07 19:20 / 19:50 /
20:20 / 20:45 and 22:10 for the five choices behind the shape of this code.
