# Archive — result-object (GitHub #27)

**Objective:** `GraphEvolver.run(seed)` returns one result object carrying the best fitness
(objective units), the best individual's edge list, its `Genome::print()` string and the
convergence history — the `best_fitness` field and `best_fitness()` method removed so a run's
state lives in the value the caller holds, never on the evolver. Spec §8.

**Spans:** 2026-08-13 (single day, five sessions).

**Outcome.** Shipped as PR #65 (three commits on `mdube_result_object`) plus PR #66 (a one-line
`.gitattributes` fix picked up along the way); both merged into `main`. `PyRunResult` /
`PyGenerationStats` added in a new `get/src/py_result.rs`; `ErasedOutcome` widened to carry the
genome string and history; ten documentation pages updated across two passes. 235 tests pass,
clippy and fmt clean. Two follow-on issues filed at tier (8): #67 (`documentation/` comment
rework) and #68 (`get/src` comment rework). GitHub #21 and #68 both received body updates recording
what this task learned (the `save_logs`/`save_results` re-homing requirement, the sharper
`dispatch.rs` comment-density figure).

**Closed with two loose ends still open, deliberately not this task's business going forward:**
`collab.md` #53 (per-owner documentation-edit queue, `jsargant_edits.md`) and #54 (FYI: `get/src`
no longer references the spec sheet) are both still unanswered by James as of the close. Neither
gates #27's shipped correctness — see `decisions.md` 2026-08-13, "closed `result-object` (#27)
with `collab.md` #53/#54 still unanswered". They remain standing `collab.md` **Open** items.

**Nothing carried forward to hotfixes.md** — this task added none. Three pre-existing `issues.md`
Parked entries (unvalidated config probabilities, a cosmetic `cargo doc` warning, the
non-converging example config) were reviewed at the close gate and left as-is; none were touched
by this task.
