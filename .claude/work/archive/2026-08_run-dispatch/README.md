# run-dispatch — GitHub #26

**Objective:** implement the config→concrete-type dispatch in `GraphEvolver::run`, which was
`todo!()`. Erase the fitness objective to `Box<dyn Fitness>` first, then dispatch strategy × genome
across Generational/SteadyState × EdgeEdit/Sda, building each starting population from config since
`Genome` has no uniform constructor. Cache `best_fitness`, release the GIL around the run, return
the best graph's edge list.

**Dates:** 2026-08-10 (start) → 2026-08-11 (close-out). Two sessions.

## Outcome

Shipped. `run` is no longer `todo!()` — a full `config.toml` completes end to end for all four
strategy × genome combinations, and an SDA `init_state >= num_states` is rejected at startup rather
than panicking mid-run, which was the task's stated done-condition. Merged as **PR #60**
(`97d9e02`, 2026-08-11T16:00:19Z, merged by James — a review merge, not a self-merge); GitHub #26
closed. Verified on `main` post-merge on Michael's machine: `cargo test -p get` 231/231, clippy
clean at `-D warnings`, `cargo fmt --check` clean.

The one structural decision worth reading before touching this code: the dispatch layer became its
own module, `get/src/dispatch.rs`, rather than joining `evolver/common.rs` — folding it in would
have dragged `pyo3` and the config schema into the genome-agnostic engine core and cost the ability
to test the engine without a Python interpreter. Full reasoning in `decisions.md` 2026-08-11 11:26;
the rejected option is recorded because "shared things go in common" is the obvious call and the
next person will make it. Also in `traps.md`.

Two side pieces landed off this task's branch, deliberately: **PR #59** (wrap `best_index`'s
assertion, restoring a `cargo fmt`-clean `main`) went on its own branch so the dispatch review
stayed reviewable, and `/start`'s SKILL.md gained a branch-as-task-0 bullet after this task's own
plan was written without one (`collab.md` #44).

## Left behind

**Hotfix removed, not carried:** #19's `#[allow(dead_code)]` on `python_fitness` had been waiting
since 2026-08-07 for #26 to become its caller. Condition met and verified on `main`; the entry is
gone from `hotfixes.md`, replaced by a removal note. **No hotfixes remain in the tree.**

**Filed at the gate:** GitHub **#61** `(1)` — `run`'s doc comment still carries #19's "for whoever
implements the dispatch (#26)" instructions and cites the now-deleted attribute. Assigned to
md12ol. Noticed while verifying the merge.

**Carried forward, not resolved here:**

- `collab.md` **#47** — `config.rs`'s module doc still says the dispatch layer is "in `lib.rs`",
  false since PR #60. Left for James to take in **#58**, which is still open and edits that file.
- `collab.md` **#48** — what `config.example.toml` should actually demonstrate. Parked for a joint
  meeting. The measured evidence behind it stays in `issues.md`: the shipped example is flat over
  500 generations at `infection_rate = 0.05` (1.20 → 1.20) while the same config at `0.5` climbs
  11.5 → 71.5. The engine is fine; the example parameters are the problem.
- `collab.md` **#49** — pip-installability as a v1 requirement; needs a §8 packaging clause and a
  `pyproject.toml`. Raised during this task's window but not part of it.
- `issues.md` parked — the `sda.rs` private intra-doc-link `cargo doc` warning, cosmetic and
  pre-existing since 2026-08-08.

**Written down during the close-out:** `CLAUDE.md` now documents the tracker's `(N)` title prefix
as a **dependency level** — `(1)` depends on nothing, `(2)` on one or more `(1)`s, and so on. It
had lived only in the owners' heads across 28 issues, and a cold session confidently misread it as
a priority ranking, which fits the visible evidence and is wrong.
