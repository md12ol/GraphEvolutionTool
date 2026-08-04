# sir-sim — implement `sir_sim`, one epidemic returning length, spread and profile

**GitHub #16 · one session, 2026-08-04 · closed 2026-08-04 · branch `mdube_sir_sim` (deleted)**

## Objective

Add `get/src/sir.rs` providing `sir_sim(graph, params, rng) -> SirRun { length, spread, profile }`
per `official_spec_sheet.md` §5.2 — one SIR epidemic with a one-timestep infectious period, all
three readings from a single run. Chosen specifically because it shared no file with James's
concurrent #10, which was live on six files at the time.

## Outcome

Shipped and on `main` via PR #31 (merged by James, `4c85cd0`); issue #16 closed. The mechanics are a
port of `Graph::SIR` from the legacy C++, with two deliberate departures: the infection draw is
written `1 - (1 - rate)^k` rather than `1 - exp(k·ln(1-alpha))`, which avoids the `ln(0)` at
`infection_rate = 1.0`, and the RNG is a parameter rather than a global, which is what #18's
common-random-numbers scheme will need. Seven tests; the suite went 97 → 104, and 110 once James's
#10 merged.

The task also picked up work outside its objective: the legacy C++ moved from gitignored to tracked
under `legacy/`, PR #30 was reviewed and merged, and two previously-unrecorded union-merge failures
were measured and written up.

## What this task left behind, and where it went

**Three unfiled entries in `issues.md`** — all deliberately held, none lost:

- *Align `sir_sim`'s length and profile with whatever the meeting decides* — assigned to Michael.
  **This one is load-bearing.** `main` carries spec §5.2's conventions while the owners' stated
  position leans toward the C++, and this entry is the only thing protecting the decision to merge
  #31 before that was settled. Held because it cannot state its acceptance criteria until
  `collab.md` #15 lands.
- *Point `generational.rs`'s mutation doc at `common::mutate_child` before #25 is built* — the exact
  drift PR #30 existed to eliminate, found while reviewing it.
- *Give `IndexGenome` a separate mutation counter instead of overloading its index* — test-only.

**Five `collab.md` items open for the joint meeting** — #15 (epidemic `length` and the trailing
zero; `spread` is agreed by both implementations and not in dispute), #16 (`dyn` versus a match arm
for the fitness dispatch axis — **the one with a real deadline**, cheap now and a rewrite of the
whole dispatch layer once #26 builds the 16-arm match), #17 (the C++'s short-epidemic re-rolls,
recorded in neither the sheet nor the tracker), #18 (an FYI trace, no action), #19 (the union-merge
findings; its routing half is already settled).

**Two `traps.md` entries added**, both measured rather than inferred: GitHub's web merge does not
apply `.gitattributes` merge drivers, so `merge=union` is simply absent there; and union silently
*duplicates* a line when both sides edit the same one, which is the inverse of the dedup failure
already on record.

**No hotfixes.** Nothing temporary entered the tree.

## Two things worth knowing before touching this code

1. **Do not start #17 without reading `collab.md` #15 first.** `epi_length` reads `SirRun::length`
   and `epi_prof_match` computes RMSE over `SirRun::profile`, and both conventions are contested.
   `spread` is safe.
2. **`sir_sim` is one epidemic by contract.** Averaging over `num_epidemics`, and the short-epidemic
   re-roll if it is adopted, belong to the objectives in #17 — not here.
