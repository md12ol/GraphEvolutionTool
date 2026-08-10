# Plan — Issue #18: epidemic seeding by per-run atomic evaluation counter
_Started 2026-08-06 · last updated 2026-08-06_

Branch: `mdube_epidemic_seeding`, off `main` at `dda6069`. Tier (3), assigned to Michael,
depends on #17 (merged as PR #40).

## Objective

Replace the `EpidemicScorer::batch_seed` stub at `get/src/fitness.rs:162-164` with the real
mechanism spec §5.2 specifies: the scorer holds the run seed **plus an atomic evaluation
counter**, each call to the batch scorer ticks it once, and that batch's epidemic seed derives
from `(run_seed, counter)`. Done when one batch's graphs all face identical dice, consecutive
batches face different dice, the same run seed reproduces the whole sequence exactly, and
`hotfixes.md`'s "The SIR batch seed never changes between evaluations" entry is deleted because
the code it describes is gone.

**The counter ticks per *batch*, not per graph** — that is the part the current code shape does
not support. `Fitness`'s default `evaluate_population` fans `evaluate` out across rayon, so a
counter read inside `evaluate` would tick once per individual, in rayon's order, destroying both
common random numbers and reproducibility. All three objectives therefore override
`evaluate_population`: draw one batch seed, then score every graph from it.

### Out of scope

- **Per-run objective instances.** §8.1 requires each replicate to own its objective (and hence
  its counter); constructing them is the config dispatch in **#26** (mine, next) and the
  replicate driver in **#20** (James). This task makes the counter correct *within* one instance
  and documents the requirement on the type.
- **Steady-state's stale fitnesses.** §5.2 accepts this explicitly ("Known limitation, already
  accepted" in #18's body). Do not try to fix it here.
- **Any `official_spec_sheet.md` change.** None is needed — see task 2. If one turns out to be,
  it is a `collab.md` item and a joint meeting, not an edit.

## Tasks

- [x] Cut `mdube_epidemic_seeding` off `main` at `dda6069`. Verified: `git log --oneline -1`.

- [x] `EpidemicScorer` gained `evaluations: AtomicU64` and
      `pub fn next_batch_seed(&self)`, deriving via SplitMix64 (`mix_seed`) — chosen over xor
      per §8.1, decision to be recorded by `/save`. Verified: three new tests, suite 157.
      The `batch_seed` stub is deliberately still in place and still frozen — task 3 wires the
      counter in, because ticking it before the objectives score a whole batch per call would
      give every individual its own dice.

- [x] **Added 2026-08-06, not in the original list** — comment pass over `get/src/fitness.rs`:
      shortened to the agreed terse style (`decisions.md` 2026-08-04 22:12), and disambiguated
      "run", which was covering three things at once. Renamed `EpidemicScorer::runs` →
      `epidemics` and every local `run`/`short_run`/`long_run` binding to match. Doc-only plus
      local renames; no logic touched. Verified: suite 157, `cargo fmt --check` clean.
      `SirRun` itself is left alone — the sheet names it at §5.2 line 368, so it is a joint
      meeting, not mine. Own commit, separate from task 2.

- [x] **Added 2026-08-07** — named the two forms a fitness number takes, which the comments had
      only described as a sign flip: **original** (what the objective returned) and **oriented**
      (negated under `Maximize`, so smaller always wins). Defined once on `Direction` and used
      throughout `fitness.rs`; four test names renamed to match. Verified: suite 160.
      Michael picked "oriented" over a new coinage precisely because `orient`, "engine
      orientation" and `best_fitness_engine` already exist in three other files and the sheet —
      so `fitness.rs` explains the codebase's vocabulary rather than competing with it.

- [x] Counter wired in and the frozen stub deleted. `EpidemicScorer` gained `mean_population`
      (one seed, one tick, rayon fan-out) and `mean_from`; `epidemics` now takes the seed as a
      parameter; all three objectives override `evaluate_population`. Verified: suite 160, with
      three new tests — counter ticks once per evaluation regardless of population size, every
      graph in an evaluation scores identically, consecutive evaluations differ.

- [x] No existing test relied on the frozen seed — all of them use the deterministic
      `certain_batch`, and passed unchanged. Verified: suite 160, up from 154 on `main`.
      Note the new seeding tests needed a *varying* setup, so `chancy_batch` (rate 0.15) and
      `complete_graph` joined the helpers; the rate is measured, not guessed — higher saturates
      the graph, lower quantizes onto one average.

- [x] End-to-end reproducibility landed as `the_same_run_seed_replays_every_batch_of_a_run` —
      five batches in order, plus a fresh-vs-fresh check that a different run seed diverges.
      Verified: suite 163, commit `3757f31`.

- [x] Hotfix entry deleted and pushed to `main` as `f8673dc`, with `collab.md` #32/#33 and the
      staged issue in the same commit. Verified: `grep -c batch_seed hotfixes.md` is 0, and the
      stub is gone from the code, not merely assumed.

- [x] `cargo fmt --check` clean and the suite green before the PR — 163 on the branch, 176 on
      `main` after James's #46 merged in alongside it.

- [x] PR #47 opened and **merged by James** as `fd0d920`. Body verified after creation — the
      section marks and code spans survived.

- [x] **Added 2026-08-07** — the scored unit is a **batch of graphs**, not a generation: 200 for a
      generational cycle or either evolver's starting population, 2 for a steady-state mating event
      (`steady_state.rs:75-76`, sheet line 509). Comments reworded throughout; renamed what is ours
      — `mean_population` → `mean_batch`, `mean_from` → `mean_with_seed`, the counter field →
      `batches_scored`, three test names. Verified: suite 160, zero "evaluation" left in the file.

- [x] **Added 2026-08-07** — simplified the structure after three sub-agent reviews found the
      seeding machinery forced but the wrapper layer redundant. `EpidemicScorer` 5 methods → 2
      (`next_batch_seed`, `mean_batch`); `mean` and `mean_with_seed` gone, `epidemics` inlined;
      Verified: suite 162, two new tests including `both_entry_points_use_the_same_reading`.
      A per-objective `reading` method was tried and reverted the same day — the readings stay
      inline in both entry points, guarded by that test, because the file's main audience is
      someone copying an objective to write their own. `collab.md` #33 carries the detail.

- [x] **Resolved 2026-08-09 at the joint meeting, in this task's absence** — both renames agreed
      (`evaluate_population` → `evaluate_batch`, `SirRun` → `Epidemic`, sheet amendment in the same
      PR) and filed as **GitHub #52**, assigned to Michael. The `issues.md` staging entry was
      dropped in `9bba043` once the issue existed. Nothing is owed by this task — #52 is a separate
      piece of work, and `collab.md` #32 is Settled.

## Open questions

- ~~Does `hash(run_seed, counter)` need to be a named hash, or is a fixed multiply-xor mix
  enough?~~ **Resolved 2026-08-07:** SplitMix64 mix, not a `ChaCha8Rng` stream position. Recorded
  in `decisions.md` 2026-08-07 "Batch seeds derive via `mix_seed`".

## Out of scope

- Config wiring of the run seed into the objectives — **#26**, next task, mine.
- Replicate-level seeding and `max_cores` — **#20**, James's.
- `PyFitness` also lands in `fitness.rs` (**#19**, James's, tier 2, no branch pushed as of
  2026-08-06). Overlap risk in one file — re-check `git ls-remote --heads origin` before opening
  the PR.
