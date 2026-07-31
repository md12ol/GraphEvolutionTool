# Decisions

Append-only. One entry per real decision, newest at the **bottom**. Never edit or delete a past
entry — if a later session reverses one, write a NEW entry that names and supersedes it. The
reversal trail is the value.

Maintained by `/save`. Survives `/done` — decisions constrain the codebase, not just one task.

Only log what a cold reader could not re-derive from the code. Skip the obvious.

---

## <YYYY-MM-DD> — <short title>
**Chose:** what we're doing.
**Why:** the reasoning, in the terms it was actually argued.
**Rejected:** the alternatives considered, and what ruled them out.
**Affects:** `path:line`, or the area it constrains.
**Supersedes:** <date + title of the earlier decision>   (only if applicable)

---

## 2026-07-31 — Cargo.lock stays tracked
**Chose:** Keep `Cargo.lock` under version control. Dropped the `Cargo.lock` ignore line that
PR #11 added while resolving that merge's `.gitignore` conflict.
**Why:** The crate builds a pyo3 extension module — application-like, not a library consumed by
other Cargo projects — so a committed lockfile is what makes a build reproducible across machines
and CI. The PR's line was also inert on its own: `Cargo.lock` was already tracked, so it would
have done nothing until someone ran `git rm --cached`.
**Rejected:** Honouring the PR's intent and untracking the lockfile. That is the right default for
a pure library, which this is not. Also rejected leaving the inert line in place — a `.gitignore`
entry that has no effect misleads the next reader.
**Affects:** `.gitignore`, `Cargo.lock`. This overrides a PR author's stated intent, so revisit it
with them rather than silently re-flipping.

---

## 2026-07-31 — `Genome::copy` removed from the trait
**Chose:** Accepted PR #11's removal of `fn copy(&self) -> Self` from the `Genome` trait, and the
addition of a `Send + Sync` bound on `Genome::Context`.
**Why:** `copy` had zero callers and `Genome: Clone` already supplies `.clone()`, so it was a
second name for one operation. The `Context: Send + Sync` bound is load-bearing: `evolver::common`
shares one `&Self::Context` across rayon worker threads when expressing a population, and parallel
expression does not compile without it.
**Rejected:** Keeping `copy` to match `Planning Notes.md`, which lists it as a required genome
method. The notes predate the trait actually existing; the duplication is not worth preserving.
**Affects:** `get/src/genomes/genome.rs:9`. Note this is a deliberate departure from
`Planning Notes.md` — update the notes rather than re-adding the method.

---

## 2026-07-31 — Steady-state uses tournament-local replacement
**Chose:** One tournament of distinct individuals per mating event. Its two best breed; the two
children overwrite its two worst. `Selection::tournament_indices` samples **without** replacement,
because "the worst two members" is undefined over a multiset.
**Why:** Self-elitist — the tournament's best is never among the replaced, so the population's best
is never discarded and no explicit elitism is needed. Diversity-preserving, because a globally poor
individual survives until it happens to be drawn. And O(k log k) per event rather than an
O(population) scan for the global worst, which matters at the configured 100,000 mating events.
This is Ashlock-style "single tournament selection", inferred from the SDA lineage of the codebase
(SDA = Self-Driving Automaton) — a strong hint, not a documented fact. Confirm with James.
**Rejected:** Global replace-worst (GENITOR-style), the textbook default: maximal selection
pressure, faster diversity loss, and an O(n) scan per event. Also rejected a separate third
tournament for choosing losers — more knobs, no benefit identified.
**Affects:** `get/src/evolver/steady_state.rs` `mating_event`; `Selection::tournament_indices`.
**Supersedes:** the /start decision to replace the two worst individuals *in the population*.

## 2026-07-31 — Fitness declares a direction; the engine minimizes
**Chose:** `Fitness::direction()` returns `Direction::Minimize` by default. `evaluate` returns the
objective's own value; `Direction::orient` converts once, in `common::evaluate`, and converts back
in `generation_stats` and `outcome`.
**Why:** Fitness functions are naturally either-better, and GET is meant to be used by people
writing their own. Declaring the direction means an implementor writes one optional line and their
numbers come back in their own units and sign — in logs, in `best_fitness`, everywhere.
**Rejected:** (a) Telling implementors to negate a maximizing objective themselves — `evaluate` and
`direction` could then silently disagree, and a run optimizing backwards is indistinguishable from
one that is not converging; it also sign-flips every number crossing back to Python. (b) A
direction-aware comparator, which keeps arrays in natural units and removes the `std_dev`
asymmetry, but requires direction at every comparison site, where a missed one fails silently.
(c) A `Minimized<F>` adapter — most faithful to the original `evaluate` doc, but pushes work onto
the implementor. Options (a) and (c) were rejected on implementor ergonomics; (b) on failure mode.
**Affects:** `get/src/fitness.rs`; `common::evaluate`; `common::generation_stats`.

## 2026-07-31 — `NaN` fitness is rejected, not tolerated
**Chose:** `Direction::orient` asserts the value is not `NaN`. It is the single gate every objective
value passes through into the engine.
**Why:** Under minimization `total_cmp` sorts `NaN` beyond `+inf`, so it reads as worst and is
harmless. Under `Maximize` the value is negated, and `-NaN` sorts *below* `-inf` — making it the
**best** individual in every tournament it enters, filling the population with whatever genome
produced it and leaving a run that looks converged. Verified by
`fitness::tests::an_unchecked_negated_nan_would_have_sorted_best`.
**Rejected:** Trusting a documented contract (leaves the `Maximize` hazard live), and a comparator
that special-cases `NaN` as worst (hides a bug in the objective rather than reporting it). A `NaN`
is a defect in the fitness function; the panic names the likely arithmetic.
**Affects:** `get/src/fitness.rs` `Direction::orient`.

## 2026-07-31 — "Cost" vocabulary dropped
**Chose:** One function, `orient`, and one word. A score is "in the objective's own units"; the
engine works in an order where "lower is better". `Direction::to_cost` was folded into `orient`.
**Why:** "Cost" was never a type — just an `f64` that had been through `orient` — and nothing in the
compiler or the prose distinguished it from one that had not. Two names for one number read as
precision but caused confusion.
**Rejected:** A `Cost(f64)` newtype, which would make mixing them a compile error. Real ceremony:
`Vec<Cost>`, ordering impls, unwrapping for mean/deviation arithmetic, and conversion at the
`evaluate_population` boundary. Not worth it at this size.
**Affects:** `get/src/fitness.rs`, `get/src/evolver/common.rs`.

## 2026-07-31 — `ChaCha8Rng` for run reproducibility
**Chose:** `SteadyStateEvolver::run` seeds `ChaCha8Rng` from its `seed` argument. `rand_chacha` was
already a declared, unused dependency.
**Why:** `StdRng`'s algorithm is explicitly allowed to change between `rand` releases, so a seeded
run that reproduces today might not after an upgrade — which defeats the entire purpose of the
`seed` argument. Test code still uses `StdRng`, where cross-version stability does not matter.
**Rejected:** `StdRng`, which is what the existing genome tests use, on those grounds.
**Affects:** `get/src/evolver/steady_state.rs` `run`. **Generational must match**, or a seed means
different things per strategy — `meeting_james.md` #4.

## 2026-07-31 — Steady-state logs iteration 0, then one row per population_size events
**Chose:** `evolve` records the starting population as iteration 0 and thereafter one row per
`population_size` mating events, so `history.len() == num_mating_events / population_size + 1`.
**Why:** Logging every event gives a 100,000-row history on a configured run. Sampling at one
"generation equivalent" keeps it comparable to a generational log. The iteration-0 row makes a log
self-contained — without it there is no way to see where a run started, and a run shorter than one
interval produced no rows at all.
**Rejected:** Logging every event (unusable size); no initial row (silent short runs); a
configurable `log_interval` in `config.toml` (expands James's schema for a knob nobody asked for).
**Affects:** `get/src/evolver/steady_state.rs` `evolve`. **Generational should log generation 0**
too, or the two logs are off by one row and cannot share an axis — `meeting_james.md` #5.

## 2026-07-31 — Minimum tournament size of 4, asserted at construction
**Chose:** `SteadyStateEvolver::new` asserts `tournament_size >= 4` and
`population_size >= tournament_size`.
**Why:** Two parents plus the two individuals they replace must be distinct. Three still preserves
the tournament's best but makes the second parent one of the replaced; two breaks self-elitism
outright, since both parents are replaced by their own children. Both checks are at construction
because `tournament_indices` would catch the second one only at the first mating event — the
mid-run failure `Evolver::new`'s own doc promises to avoid.
**Rejected:** Allowing 3 or 2; checking per event. Changing `Evolver::new` to return `Result` was
also considered and left alone — it is James's trait signature and touches both evolvers.
**Affects:** `get/src/evolver/steady_state.rs` `new`. The config layer is the proper home for both
checks — `meeting_james.md` #12.
