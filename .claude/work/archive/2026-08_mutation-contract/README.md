# mutation-contract — `Genome::mutate` applies exactly one mutation (GitHub #10)

**Dates:** 2026-08-03 (opened and closed the same day, two sessions).
**Owner:** James · **Branch:** `jsargant_mutation_contract` · **Commit:** `a8cbf27`

## Objective

Make `Genome::mutate` a trait-level contract meaning **exactly one mutation per call**, and move
both mutation dice rolls — `mutation_rate` (whether) and a new `max_mutations` (how many, uniform
`1..=max`) — out of the genomes and into one shared helper in `evolver/common.rs`.

## Outcome

Delivered in full and shipped as **PR #30**. `MAX_MUTATIONS` is deleted from `edge_edit.rs`, both
genomes apply one mutation per call, `common::mutate_child` (`common.rs:155`) owns both rolls,
`SharedEvolutionContext` and `Config` carry `max_mutations` defaulting to 1, and steady-state
mutates through the helper. **103 tests green, up from 97.**

The six verification tests were each **fault-injected** before their plan items were ticked `[x]` —
an exclusive count range, an extra loop pass, and a wrong config default were all confirmed to make
the relevant test fail. That is what distinguishes these `[x]`s from "the suite is green".

## ⚠ Carried forward — this task closed with the PR unmerged

**PR #30 was `OPEN` with zero reviews when this task was archived, and GitHub #10 was still open.**
Archiving was a deliberate call on 2026-08-03: the work was complete and the only thing outstanding
was Michael's review, which is not the task's to finish.

**So whoever merges #30 should:** confirm #10 actually closed with it, and be aware that the plan's
final item shipped as `[~]`, not `[x]`. If review demands changes, the plan and history are here,
moved rather than deleted.

Also note: `common.rs`, `mod.rs` and `steady_state.rs` overlap Michael's **#14** and **#15**. He had
not started either as of 2026-08-03 (both untouched since 2026-07-31, no branch on origin). Rebase
rather than merge if that has changed.

## Left behind for other tasks

- **GitHub #24** — commented 2026-08-03 to record that `max_mutations` landed here rather than
  there, leaving #24 as fitness variants plus dropping `seed`/`num_chars`.
- **GitHub #23** — `Config::validate` must reject `max_mutations = 0`. `mutate_child` only asserts
  it as a backstop. Commented during this task.
- **GitHub #26** — nothing wires `Config` onto engine types yet, for any field; `lib.rs` dispatch is
  entirely `todo!()`. So `max_mutations` is plumbed type-to-type but inert at runtime. Not a gap
  this task introduced.
- **GitHub #25, generational evolver** — the next task, and the reason this one existed. Needs
  `express_and_score` (#14, Michael's) first.
- **`collab.md` item 14** — left under **Open** unchanged; Michael never answered, and the PR body
  now carries the same information for him.
- **New trap** — four untracked pre-spec-sheet docs (`docs/`, `GET GA planning session.md`) are not
  gitignored, so a bare `git add -A` would commit them. `docs/IMPLEMENTATION.md` is the document
  `official_spec_sheet.md` replaced.

**No hotfixes and no unfiled issues were left behind** — `hotfixes.md` and `issues.md` were
template-only throughout this task.

## Key decisions (in `decisions.md`, not here)

- 2026-08-03 18:45 — the `max_mutations` count roll is unconditional; seeded runs before and after
  this change are **not comparable**, even at the default.
- 2026-08-03 21:10 — the tests count mutations on the existing `IndexGenome` stub rather than a new
  type.
- 2026-08-03 21:12 — verification tests are fault-injected before an item is ticked `[x]`.
