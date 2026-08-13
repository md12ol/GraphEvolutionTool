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
different things per strategy — `collab.md` #4.

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
too, or the two logs are off by one row and cannot share an axis — `collab.md` #5.

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
checks — `collab.md` #12.

## 2026-07-31 — Michael — Two owners share one `.claude/`, tracked in the repo
**Chose:** James (shorinbonsai) uses this repo's `.claude/` on his own machine rather than keeping
a private copy. The machinery, the skills and the persistent docs are tracked and shared; only
`work/current/` and `settings.local.json` stay per-person.
**Why:** A second copy diverges silently, and the docs that matter most across a collaboration —
`decisions.md`, `traps.md`, `collab.md` — are exactly the ones that are worthless if each side has
its own. `work/current/` is the one thing that must NOT be shared: two people cannot hold one live
plan without fighting over it every session.
**Rejected:** Namespacing to `work/<user>/current/` so live plans are shared too. It hardcodes into
`session_brief.sh` and all five skills, and buys handoff-between-people that we don't need yet.
Revisit if we ever pass a half-finished task across.
**Affects:** `/.gitignore`, `/.gitattributes`, `.claude/CLAUDE.md` "Two people, one `.claude/`".

## 2026-07-31 — Michael — `merge=union` on the persistent docs, paid for with author stamps
**Chose:** `/.gitattributes` sets `merge=union` on `.claude/work/*.md`. Every entry in those files
now carries `— <author>` in its heading or stamp.
**Why:** They are append-only, so both owners write to the tail of the same file — the most
conflict-prone shape in git. Without this, every concurrent session ended in a merge conflict on
`decisions.md`. Union merge keeps both sides and never conflicts.
**Cost, accepted deliberately:** never conflicting means a genuine simultaneous edit of the *same*
entry silently yields both versions, interleaved. The author stamp is what makes that visible on
sight; `traps.md` carries the "read the tail after a merge" rule. Trading a loud, constant failure
for a quiet, rare one is the right side of that trade, but it is a real trade.
**Rejected:** One-file-per-entry directories (`decisions/2026-07-31-elitism.md`) — genuinely
conflict-free, but it rewrites all five skills for a problem two people don't have yet.
**Affects:** `/.gitattributes`, `decisions.md`, `traps.md`, `hotfixes.md`, `issues.md`, `collab.md`.

## 2026-07-31 — Michael — `work/archive/` is tracked; `hotfixes.md` entries name an owner
**Chose:** Un-ignored `.claude/work/archive/`. Added `Owner:` and `Machine:` to every `hotfixes.md`
entry. Renamed `collab.md` to `collab.md`.
**Why:** Three consequences of the same fact — the docs are now read by two people.
An archived task is *history*, and ignoring it stranded every `/done` on one laptop. A hotfix is
uncommitted code in *one* working tree, so unowned it reads as though it were in yours. And a file
titled "To discuss with James" instructs James's agent to log conflicts with James.
**Rejected:** Gitignoring `hotfixes.md` as per-machine state. The other owner's uncommitted hacks
are precisely what you want to know about before merging their branch — the fix is a name on the
entry, not hiding the file.
**Affects:** `/.gitignore`, `.claude/work/hotfixes.md`, `.claude/work/collab.md`.

## 2026-07-31 — Michael — `.claude/` hook and settings changes go through a PR
**Chose:** `settings.json` and `hooks/*.sh` are never pushed straight to `main`.
`show_hotfixes.sh` was given a real path pattern (the co-owned evolver surface) and now also fires
when the machinery itself is edited.
**Why:** Those files are executable code that runs on the other owner's machine at session start,
on their next pull, without them reading the diff. It is the one part of `.claude/` where "it's
just docs" is false, and the only part where a bad change is not self-correcting.
**Affects:** `.claude/settings.json`, `.claude/hooks/show_hotfixes.sh`, `.claude/CLAUDE.md`.

## 2026-07-31 — Michael & James — `official_spec_sheet.md` is the design authority
**Chose:** A new root-level `official_spec_sheet.md` describing the system's design, agreed on a
joint call. It supersedes `IMPLEMENTATION.md`, which had gone stale on fitness direction,
steady-state replacement and mutation. `.claude/CLAUDE.md` now instructs every session to read it
before touching `get/src/`, and states that **where the sheet and the code disagree the sheet is
the intent** — deliberately inverting the "the repo wins" rule, because parts of the sheet were
agreed before being implemented.
**Why:** `IMPLEMENTATION.md` mixed design with a build order, so the design rotted every time the
plan moved. Separating them means the design document has no reason to go stale. It was also
untracked and had never reached James.
**Rejected:** Putting it under `.claude/`. `/.gitattributes` applies `merge=union` to
`.claude/work/*.md` **by glob**, so any new `.md` there inherits a merge strategy built for
append-only files. A spec sheet is rewritten in place, which is exactly the silent-interleave case.
Also rejected keeping sequencing in the sheet — that gets its own document, not yet written.
**Affects:** `/official_spec_sheet.md`, `.claude/CLAUDE.md`, `/IMPLEMENTATION.md` (now superseded).

## 2026-07-31 — Michael & James — `Genome::mutate` applies exactly one mutation
**Chose:** A trait-level contract: `mutate` performs one atomic mutation. The engine owns both
dice rolls — `mutation_rate` (whether) and a new top-level `max_mutations` (how many, uniform
`1..=max`, default 1) — in one shared helper called by both strategies.
**Why:** The two genomes silently disagreed. `SdaGenome::mutate` already applied exactly one and
its doc said "callers that want more disruption call this multiple times"; `EdgeEditGenome::mutate`
rolled `1..=4` internally against a hardcoded `MAX_MUTATIONS`. The trait doc said only "mutate this
genome in place", which is how they drifted. A genome that rolls its own count makes
`max_mutations` meaningless for that representation and nothing reports it.
**Rejected:** Making `max_mutations` per-genome — one gene of 256 and one state of 12 are different
perturbations, but splitting the two rolls across two config tables is worse than the uneven
magnitude. Rejected default 4 (a magic number for every genome) and a required field.
**Consequences:** edge-edit mutates less at the default than it does today; SDA becomes more
disruptive above 1; every seeded run changes output because the draw order moves.
**Affects:** `get/src/genomes/genome.rs:26`, `edge_edit.rs` (`MAX_MUTATIONS` deleted),
`evolver/common.rs`, `evolver/steady_state.rs:58`, `config.rs`. Spec §4. ~~**Not yet implemented.**~~
**Implemented 2026-08-03 — James**, closing GitHub #10 on branch `jsargant_mutation_contract`:
`common::mutate_child` (`common.rs:155`) owns both rolls, `MAX_MUTATIONS` is deleted, both genomes
apply exactly one mutation per call, and `Config::max_mutations` defaults to 1. 103 tests green.
Steady-state is the only caller until the generational evolver (#25) exists — the "both strategies"
above is the contract the helper was built to, not a second call site that exists today.

## 2026-07-31 — Michael & James — SDA alphabet is derived from the edge cap
**Chose:** `num_chars` leaves `config.toml` entirely; the alphabet is `max_edge_multiplicity + 1`,
so characters are exactly `0..=cap`. Dispatch uses the constructor that already exists for this,
`SdaGenome::random_with_edge_multiplicity_cap`.
**Why:** Every character is then a legal edge weight and nothing is ever clamped. The two values
could previously disagree in two silent ways: an alphabet larger than the cap biases the graph
toward the cap as surplus characters clamp onto it, and a smaller one makes the upper weights
unreachable, exploring less space than configured. `config.example.toml` already carried the advice
as a comment; this promotes it into the type system.
**Rejected:** Keeping `num_chars` free and warning on mismatch — a warning nobody reads is not a fix.
**Affects:** `get/src/config.rs` `GenomeConfig::Sda`, `config.example.toml`, dispatch. Introduces a
required check: `1 <= max_edge_multiplicity <= 255`. Spec §3.2. **Not yet implemented.**

## 2026-07-31 — Michael & James — Direction is fixed per objective, never configurable
**Chose:** One fitness function is always maximized or always minimized; it is a property of what
the function computes. `epi_spread` and `epi_length` maximize, `epi_prof_match` minimizes. A Python
objective declares its direction **when the callable is registered**, since nothing can infer it.
No `direction` or `maximize` field in `config.toml`.
**Why:** Config that could contradict what the objective actually computes buys nothing and creates
a way to run backwards. The question was genuinely open — `epi_spread` looked either-directional
depending on the experiment — and was settled by deciding the objective, not the run, owns it.
**Rejected:** Splitting into `epi_spread_max` / `epi_spread_min`; a `maximize` config field.
**Affects:** `get/src/fitness.rs`, `config.rs` `FitnessConfig`, the Python registration surface.
Spec §5, §7.

## 2026-07-31 — Michael & James — The engine is oriented internally; convert only at the boundary
**Chose:** `common::evaluate` is renamed `common::express_and_score` and is the **sole** path from a
population to fitnesses — the engine never calls `Fitness::evaluate` or `evaluate_population`
directly. It orients once, inward. Everything inside the engine (fitness arrays, `GenerationStats`,
`EvolutionOutcome`) stays in engine orientation, lower-is-better, and **nothing in the middle
converts**. The Python boundary converts once, outward.
**Why:** Fixes the double flip, where `generation_stats` and the outcome each converted straight
back. Three consequences: two conversions per run instead of one per stats row; `generation_stats`
loses its `direction` parameter; and **the `std_dev` special case disappears** — it existed only
because a converting function had to deliberately not convert one field. The sole-entry rule makes
orientation and `NaN` rejection structural: a direct call bypasses both silently.
**Rejected:** A direction-aware comparator (needs direction at every comparison site, where a
missed one fails silently) and a `Cost` newtype (ceremony, already rejected 2026-07-31).
**Affects:** `get/src/evolver/common.rs`, `steady_state.rs`, `fitness.rs`. Spec §5.1.
**Supersedes:** the `generation_stats` half of "Fitness declares a direction; the engine minimizes"
(2026-07-31) — that entry has `generation_stats` converting back, which is now wrong.

## 2026-07-31 — Michael & James — One SIR simulator, three objectives
**Chose:** `sir_sim(graph, params, rng) -> SirRun { length, spread, profile }`, with three thin
objectives over it: `epi_spread` (maximize), `epi_length` (maximize), `epi_prof_match` (minimize,
**RMSE** against a user-supplied target). `profile[t]` is **newly infected** at step `t`. A dud
outbreak is `length = 0`, `spread = 1`. `num_epidemics` is a user parameter — one SIR draw is noisy
enough that selection would chase the dice instead of the graph.
**Why:** The expensive part is the simulation and all three objectives want the same one, so
computing all three outputs from one run costs nothing and a future multi-objective mode gets them
free. Settles the "what does `evaluate` return" question left open by `IMPLEMENTATION.md` §6.
**Affects:** `get/src/fitness.rs`, `config.rs` `FitnessConfig`. Spec §5.2. **Not yet implemented.**

## 2026-07-31 — Michael & James — Epidemic dice: shared within a batch, fresh across batches
**Chose:** The objective holds a run seed plus an **atomic evaluation counter**. Each batch
increments it once and derives that batch's epidemic seed from `(run_seed, counter)`. Every graph
in a batch is simulated from that one seed; the next batch gets a different one. The counter is
**per-run state** — each run owns its own objective instance.
**Why:** Two requirements pull opposite ways. Within one evaluation, all individuals must face the
same draws (common random numbers) so fitness differences reflect the graph and not the dice.
Across evaluations, the same network must not keep getting the same epidemic, or the run optimizes
against one frozen sample of the disease. The trait forces the mechanism: `evaluate` takes `&self`
and `Fitness: Sync`, so no mutable RNG can be held.
**Rejected:** Deriving the seed from the graph's contents or index — satisfies neither requirement,
and freezes each network's epidemic forever. A counter shared across concurrent runs — thread
scheduling would decide which run sees which seed.
**Consequence, accepted:** steady-state scores incrementally, so its stored fitnesses were computed
under different dice than the children compared against them, and a lucky individual keeps its
inflated score. Inherent to incremental scoring plus a stochastic objective; the alternatives trade
it for a different distortion. **Prefer generational when the objective is stochastic.**
**Affects:** `get/src/fitness.rs`. Spec §5.2. **Not yet implemented.**

## 2026-07-31 — Michael & James — One master seed, supplied to the Python `run` call
**Chose:** No seed anywhere in `config.toml`. One master seed is passed to `run`, and everything
derives from it: starting population, evolution loop, epidemics, and the per-replicate seed stream.
Replicate count and `max_cores` are likewise `run` parameters — they describe *this invocation*,
while config describes the experiment.
**Why:** A separately configured `[fitness] seed` meant replicates at different evolution seeds
still faced identical epidemic draws, which defeats the purpose of replicates. Per-run seeds come
from a stream drawn off the master, so **a run's seed does not depend on how many runs were
requested** — asking for 50 reproduces the first 30 exactly, and extending an experiment never
invalidates existing replicates.
**Rejected:** Asking the user for N seeds. `master ^ i` as the derivation — nearby masters collide
across run indices.
**Affects:** `config.rs` `FitnessConfig` (drops `seed`), the Python entry point. Spec §8.1.

## 2026-07-31 — Michael & James — Replicates parallelise only under a native Rust objective
**Chose:** The engine picks the mode from the fitness type: native Rust → replicates in parallel,
`python` → sequential. Not a user setting. The user's only knob is `max_cores` on the `run` call,
governing one **locally built** rayon pool. Results are collected in run order, never completion
order.
**Why:** `n` concurrent runs calling into Python is `n` threads contending for one GIL — slower
than sequential *and* contended. Exposing the mode as a setting only creates a way to choose wrong,
since the fitness type already determines the right answer. The pool must be local because rayon's
global pool configures once per process, and this is a module imported once per Python session: a
global pool would make `max_cores` a property of whichever `run` call happened first, and the
second call with a different cap would fail outright.
**Note:** sequential *replicates* still express their population in parallel — expression is pure
Rust and GIL-free; only the scoring call is serialized.
**Affects:** the Python entry point. Spec §8.1.

## 2026-07-31 — Michael & James — Python builds TOML; Rust parses it once
**Chose:** Python config objects **serialize to TOML**, and that TOML is what Rust parses. Python
is a builder for the config format, not a second parser of it. `run` returns a result object
(best fitness, best edge list, best genome string, log rows); there is no `best_fitness()` accessor.
Validation is an explicit `Config::validate` that runs **before anything else** and returns an error
naming the offending field — never a panic across the FFI boundary.
**Why:** The two front ends converge before parsing, so there is one parser and one validator and
no way for the Python path to accept what TOML rejects. The generated TOML doubles as re-runnable
provenance. Returning results rather than caching them removes a wart: a cached accessor must answer
something before any run, and infinity converts back into a suspiciously excellent score under a
maximizing objective.
**Rejected:** Two independent construction paths — serde currently *is* the validation, so a Python
path bypassing it would silently accept bad configs on the path users actually take.
**Affects:** `get/src/lib.rs`, `config.rs`. Spec §7, §8. **Not yet implemented.**

## 2026-07-31 — Michael & James — Generational details settled
**Chose:** `ChaCha8Rng` seeded from `run`'s seed (matching steady-state); log the initial population
as generation 0 (matching steady-state, so the two logs share an axis); `select` keeps sampling
**with** replacement; an odd `population_size - elite_count` means the last pair contributes **one**
child; elites are rescored every generation like everyone else.
**Why:** RNG and generation-0 logging both exist so a seed and a log row mean the same thing in
either strategy. Rescoring elites is correct under a stochastic objective — the new number is a
fresh sample of the same individual, and freezing the old one would let a lucky draw persist.
**Rejected:** Rewriting `select` to sample without replacement for consistency with
`tournament_indices`. The divergence is ~2% in expected distinct entrants at realistic sizes, which
is noise against a stochastic run; each method keeps the convention standard for its purpose.
Documented rather than removed. **Settles `collab.md` #6.**
**Affects:** `get/src/evolver/generational.rs`, `common.rs`. Spec §6.1, §6.2. **Not yet implemented.**

## 2026-07-31 — Michael & James — Run output schema
**Chose:** Per logged iteration: `iteration`, `best_fitness`, `mean_fitness`, `std_dev`
(**population**, divides by `n`), `ci_95` (half-width `1.96·s/√n` using the **sample** deviation,
`n-1`), plus the seed and run index on every row. Per run: the best genome via `Genome::print`, its
expressed network as a weighted edge list, and its fitness.
**Why:** The deviation/CI denominators differ deliberately — `std_dev` describes the population as
the complete thing it is, while a confidence interval is inherently a sampling statistic and needs
the sampling denominator to mean anything. `ci_95` is the **within-population** band; the
across-runs band is the user's to compute when aggregating, which the run index makes possible.
Seed and run index on every row let 30 replicate logs concatenate into one file and stay separable.
**Affects:** `get/src/evolver/mod.rs` `GenerationStats`, `lib.rs` `save_logs`/`save_results`.
Spec §6.4. **Not yet implemented.**

## 2026-07-31 — Michael — Implementation work is tracked in GitHub issues, not a sequencing doc
**Chose:** The build order the spec sheet defers to lives in **GitHub issues**, not a markdown
document. Filed 16 new issues (#14–#29) derived from the spec and the tree, each with a time
estimate, and enriched three that already existed (#6, #10, #13) rather than duplicating them.
Assigned by prior authorship: Michael takes `common.rs`, `sda.rs`, `steady_state.rs`, `fitness.rs`
(~38h); James takes `edge_edit.rs`, `config.rs`, `generational.rs`, `lib.rs` (~36h).
**Why:** A sequencing markdown file would be a third place work is described, after the spec and
the tracker, and it would go stale the moment an issue moved. Assigning by prior authorship means
each owner works in files they already know, which matters more than an exactly even hour split.
**Rejected:** Filing everything fresh — #10 already described the mutation contract almost exactly
("if we want more than one mutation the GA should call multiple times"), and #6 and #13 overlapped
too. Duplicating them would have had one of us do the work twice.
**Found while filing:** #7/#8/#9 want **configurable crossover types**, which the spec had nothing
about. Rather than silently leaving the gap, §4 now records crossover as one fixed operator per
genome today with selectable operators as a planned extension, following `Selection`'s enum shape.
**Affects:** the GitHub tracker; `official_spec_sheet.md` §4; `.claude/work/issues.md`.

## 2026-07-31 23:32 — Michael — Multiple crossover operators wait until the spec is delivered
**Chose:** No additional crossover operators, no crossover enum, no operator config field, until
everything else in `/official_spec_sheet.md` is implemented. One fixed operator per genome stands:
two-point over genes for edge-edit, two-point over states for SDA. `official_spec_sheet.md` §4
rewritten to say so; GitHub #7, #8 and #9 closed as *not planned*.
**Why:** Sequencing, not design. More operators are wanted and the shape is already agreed — follow
`Selection`, one variant plus one match arm — but nothing selects between operators until a second
one exists, so building the enum now would mean a config field with one legal value and a dispatch
with one arm. The sheet has a large amount of unbuilt surface (fitness, the Python layer,
`Config::validate`, generational); an extension point competes with delivering it.
**Rejected:** (a) Building the one-variant enum now to pin the shape — `Selection` is precedent, but
it earns its enum by mapping onto a config field users set today. (b) Leaving #7, #8 and #9 open —
they had empty bodies predating the sheet, and §4 carries the intent better than three stubs did.
**Affects:** `official_spec_sheet.md` §4; `get/src/genomes/genome.rs` `crossover` stays a plain
trait method with the logic inline per genome; GitHub #7, #8, #9.
**Note:** This predates the spec-sheet meeting and was settled by the repo owner, so it did not go
through `collab.md`. **From 2026-07-31 23:32 the meeting rule in `CLAUDE.md` governs: no further
change to the sheet without a joint meeting.**

## Task complete: steady-state-evolver — 2026-07-31
Archived to `.claude/work/archive/2026-07_steady-state-evolver/`. Entries below this line belong to
later tasks. The evolver shipped: `evolver/common.rs`, `evolver/steady_state.rs` and `fitness.rs`
have no `todo!()`s, 97 tests pass on `main`, merged via PR #12 and `b466e4e`.


## 2026-08-04 11:35 — Michael — `sir_sim` lands as its own module, porting the C++ mechanics but the sheet's reporting
**Chose:** `get/src/sir.rs` as a top-level module holding `SirParams`, `SirRun` and `sir_sim`. The
state machine, exposure accumulation and combined infection draw are ported from
`legacy/Graph.cpp` `Graph::SIR` unchanged. Two things deliberately differ: the draw is written
`1 - (1 - rate)^k` rather than `1 - exp(k·ln(1-alpha))`, and the RNG is a parameter rather than a
global. Reporting follows spec §5.2 — `length` excludes the burnout step, `profile` has no trailing
zero — with the divergence raised rather than silently chosen.
**Why:** The two formulations of the draw are algebraically identical, but the direct form has no
`ln(0)` at `infection_rate = 1.0`, which is a case the tests actually exercise. The RNG parameter is
what #18's common-random-numbers scheme needs — a global cannot be seeded per batch. §5.2's
reporting was followed because `CLAUDE.md` makes the sheet the intent where sheet and code disagree.
**Rejected:** (a) `fitness/sir.rs` as a submodule, which matches the `genomes/edge_edit/` idiom and
the spec's own nesting better — rejected because turning `fitness.rs` into a directory is a file
*move*, and #17 and #19 both rewrite that file heavily, so it would hand the next person a conflict
for no functional gain. Revisit when #17 lands. (b) Deleting the `SirFitness` stub — it is #17's to
replace with three objectives, and removing it now would delete the `todo!()`s that signal the work
is outstanding.
**Affects:** `get/src/sir.rs`; `get/src/lib.rs:6`; `get/src/fitness.rs:96-107` doc only.

## 2026-08-04 11:36 — Michael — Merge the simulator now and correct its conventions in a follow-up
**Chose:** PR #31 merges as-is with `Closes #16`, carrying spec §5.2's `length` and `profile`
conventions, and the correction is staged in `issues.md` as a follow-up assigned to Michael, to be
filed once the meeting settles `collab.md` #15.
**Why:** Michael's call, made with the trade stated. The alternative was recommended and declined,
which is recorded here because the risk is real and someone should be able to see it was taken
knowingly rather than missed.
**Rejected:** Holding the branch until the meeting — recommended on the grounds that the change is
~15 lines (push the terminating zero, return `profile.len()`, update seven tests), so waiting is
cheap, while merging first opens a window in which #17 could build on a convention about to be
reversed. Overruled in favour of not letting the branch go stale.
**Risk carried:** if the staged follow-up is never filed, the correction is lost and `epi_length`
and `epi_prof_match` are silently built on the wrong convention. The plan's final open task exists
solely to prevent that.
**Affects:** PR #31; `get/src/sir.rs:149-153`; `.claude/work/issues.md`; blocks the start of #17.

## 2026-08-04 11:37 — Michael — The legacy C++ is tracked in `legacy/`, not gitignored
**Chose:** `Graph.cpp`, `Graph.h`, `SDA.cpp`, `SDA.h` and `main.cpp` move to `legacy/` and are
tracked, with a `legacy/README.md` mapping each to what ports it. The root-level ignore entries are
removed.
**Why:** They were ignored, so the implementation the Rust is a port of existed on one laptop only —
and `sir.rs`'s own module doc cited a file no one else could open. A reference that only one owner
can read cannot settle an argument about intent, which is exactly what it kept being used for.
**Rejected:** Leaving them ignored and describing the model in prose in `sir.rs` — the C++ is the
tiebreaker for §5.2 questions and paraphrase is not.
**Also recorded in the README:** `Graph.cpp/.h` and `main.cpp` are from **different generations and
will not compile together** — `SIR` changed shape, its first two arguments swapped from
`(alpha, p0)` to `(p0, alpha)`, which compiles and is silently wrong if ported by eye, and
`hammy_distance` is gone while `main.cpp:561` still calls it. `SIRwithVariants` is new and no issue
covers it.
**Affects:** `legacy/`; `.gitignore`; `get/src/sir.rs` module doc.

## 2026-08-04 11:38 — Michael — Every PR is merged by the other owner
**Chose:** Nobody merges their own PR — James merges Michael's, Michael merges James's. Agents open
PRs and stop. Self-merge is allowed only when the other owner is unavailable and the change blocks,
and it must leave a note in `collab.md`.
**Why:** Three things in this repo fail *silently* and a second reader is the only detector:
`merge=union` never conflicts, so an interleaved doc entry has no automated check; `collab.md` #14
has three source files claimed by two workstreams at once; and hooks/settings execute on the other
person's machine at session start without them reading the diff.
**Rejected:** Leaving the existing rule scoped to `settings.json` and `hooks/` — that made the
riskiest case an exception rather than an instance, which is an edge the rule can fall off.
**Affects:** `.claude/CLAUDE.md`, new "Pull requests" section; PR #31 is itself subject to it.
## 2026-08-03 18:45 — James — The `max_mutations` count roll is unconditional; no special case at 1
**Chose:** `common::mutate_child` always draws `rng.random_range(1..=max_mutations)` once the
`mutation_rate` roll passes, including when `max_mutations == 1` and the draw has exactly one
possible outcome. A standalone free function, not a method on `Selection`, matching `evaluate` and
`generation_stats`.
**Why:** measured 2026-08-03 — `random_range(1..=1)` **consumes RNG state** despite being
deterministic, so every seeded run's output moves relative to the pre-`max_mutations` engine even at
the default. Skipping the draw at 1 would restore the old stream, and was rejected: a special case
that changes the RNG sequence depending on a config value is a worse thing to own than a one-time
change in seeded output, and the latter was already accepted when this was designed.
**Rejected:** `if max_mutations > 1 { ... }` around the draw, for the reason above. Also rejected
folding both rolls into `Genome::mutate` — that is the exact drift this task removes.
**Consequences:** seeded runs before and after this change are not comparable. The existing
reproducibility tests survive because they assert self-consistency, not specific values; anything
that ever hardcodes a fitness from a seed will not.
**Affects:** `get/src/evolver/common.rs` `mutate_child`, `get/src/evolver/steady_state.rs`
`mating_event`. Spec §4; implements GitHub #10.
*Recorded 2026-08-03 18:45 — James, during #10 implementation.*

## 2026-08-03 18:46 — James — Commits and PRs authored by James carry no agent co-attribution
**Chose:** No `Co-Authored-By: Claude ...` trailer on commits and no "Generated with Claude Code"
footer on PR bodies, for work authored under James's git identity. Recorded in `~/.claude/CLAUDE.md`
(global, James's machine only), deliberately **not** in this repo's `.claude/CLAUDE.md`.
**Why:** attribution on the permanent record is the author's to decide, and a co-author line adds a
party who cannot answer questions about the change. Kept out of the project file because it is a
personal preference about James's own commits, and the shared `CLAUDE.md` is not the place to bind
the other owner's practice.
**Rejected:** putting it in the project `CLAUDE.md` — would silently impose it on Michael. Also
rejected leaving it unrecorded here: "why do these commits have no trailer when the tool default
adds one" is exactly the question a future session would waste time on.
**Affects:** every commit and PR James makes in this repo; `~/.claude/CLAUDE.md`.
*Recorded 2026-08-03 18:46 — James, after the /start coordination call.*

## 2026-08-04 15:55 — Michael — Code goes through a branch and a PR; working docs may be pushed direct
**Chose:** Anything under `get/src/`, `Cargo.toml` or `config.example.toml` that solves an issue
goes through a feature branch and a PR, named `<owner>_<description>`. `settings.json` and `hooks/`
were already PR-only; the spec sheet needs a joint meeting on top. `.claude/work/*.md` may be pushed
to `main` directly, and `.claude/CLAUDE.md` may be, though a PR is preferred when the change binds
the other owner's practice rather than recording a fact.
**Why:** review is worth requesting where it catches something. A defect in `get/src/` stays
invisible until something downstream reads a wrong number, and the current issue set has several
files claimed by two workstreams at once. The docs carry no behaviour, and the one thing review
would catch in them — a union-merge interleave — has its own audit command that is cheaper to run
than a review is to request. A trap that is not on `main` also protects nobody, which is an active
argument for pushing them.
**Rejected:** Routing everything through PRs — it made the traps land after the merge they existed
to prevent. Also rejected leaving it implicit: three review bypasses happened in one day, each with
a good reason, which is exactly how a pattern establishes itself unnoticed.
**Affects:** `.claude/CLAUDE.md`, "Pull requests"; `collab.md` #19.

## 2026-08-04 16:05 — Michael — Union merge duplicates concurrently-edited lines; GitHub disables it entirely
**Chose:** Record both as traps and act on them: append rather than edit `.claude/work/*.md` in
place, and merge any PR touching those files locally rather than with the GitHub button.
**Why:** measured, not reasoned. GitHub reproduces the no-driver merge exactly — clean locally with
`.gitattributes`, `CONFLICT` without it, `mergeable=false` on its API — because merge drivers run in
your git, not on their servers, and this holds even for `union`, which is built in. Separately, two
branches editing the same existing line makes union keep **both** versions, reported as
`1 file changed, 1 insertion(+)` with no conflict; reproduced on a 250-line file, so it is not a
small-file artifact.
**Corrects:** an earlier claim in this session that James's in-place amendment of the 2026-07-31
joint entry was "luck as much as care". Authorship is irrelevant to union safety — the hazard is
*concurrent* edits to one line, and nobody was editing his, so it was safe by construction.
**Rejected:** dropping `merge=union` — it still does its job for the append case, which is the
overwhelming majority, and without it every concurrent session ends in a conflict.
**Affects:** `.claude/work/traps.md` (two entries); `.claude/CLAUDE.md` union-formatting rule 5 and
the "Pull requests" section; `collab.md` #19.

## Task complete: sir-sim — 2026-08-04
Archived to `.claude/work/archive/2026-08_sir-sim/`. Entries below this line belong to later tasks.
`sir_sim` shipped: `get/src/sir.rs` is on `main` via PR #31, GitHub #16 closed, 110 tests green.
Carried forward, not resolved: three unfiled entries in `issues.md`, and `collab.md` #15, #16, #17
and #19 awaiting the joint meeting.

## 2026-08-04 17:40 — Michael & James — Epidemic length counts the burnout step; profile carries a trailing zero
**Chose:** Spec §5.2 now matches `legacy/Graph.cpp`. A lone patient zero gives `length = 1`,
`spread = 1`, `profile = [1, 0]`. `length` counts every timestep the epidemic occupied, including
the final one in which the last infectious node recovers without transmitting, and `profile` ends
in a terminating zero.
**Why:** the legacy C++ is the intended behaviour, and it is what every historical result was
produced under. `Graph::SIR` increments `epiLen` on the burnout pass and writes
`epiProfile[epiLen] = 0`; matching it keeps our numbers comparable with the archive.
**Rejected:** keeping the sheet's original `length = 0` convention, which `get/src/sir.rs` was
built to. It is arguably tidier — `length` would equal `profile.len() - 1` — but tidiness does not
outweigh comparability with existing results.
**Consequences:** `spread` is unaffected; the C++ `totInf` already agreed with it, which narrowed
this decision to two values rather than three. `epi_length` scores shift by exactly one, a constant
offset that cannot change selection. `epi_prof_match` is **not** neutral — RMSE now runs against a
profile one element longer. `get/src/sir.rs` currently implements the old convention and is
corrected under its own issue.
**Affects:** `official_spec_sheet.md` §5.2; `get/src/sir.rs:149-153` and its seven tests; blocks
GitHub #17. Supersedes the §5.2 wording agreed 2026-07-31. Settles `collab.md` #15.

## 2026-08-04 17:41 — Michael & James — Epidemics within one evaluation run sequentially
**Chose:** the `num_epidemics` simulations behind one fitness evaluation are serial. Parallelism
comes only from replicates and from the population (§8.1).
**Why:** those two levels already yield far more independent work than any core count — 30
replicates × 200 individuals is 6,000 units before the epidemic loop is reached — so a third
nesting level buys nothing and costs scheduling overhead. Serial epidemics also make each
population-level task substantially larger, which improves amortization of the levels that do run
in parallel.
**Rejected:** parallelizing the epidemic loop. It is deterministically parallelizable in principle,
since common random numbers make epidemic *i*'s seed derive from `(batch_seed, i)` independently of
the graph — so this was rejected on value, not feasibility. Note it could not have helped the case
that appears to need it: under a Python objective replicates go sequential, but a Python objective
runs no epidemics at all.
**Affects:** `official_spec_sheet.md` §5.2; GitHub #17 and #18.

## 2026-08-04 17:42 — Michael & James — Fitness dispatch erases to `Box<dyn Fitness>` (option B)
**Chose:** the config's fitness variant becomes one `Box<dyn Fitness>` before the evolver is
instantiated, so dispatch is strategy × genome rather than strategy × genome × objective. A new
objective is one arm in the erasing match and never touches dispatch.
**Why:** the two axes are not symmetric and this had not been noticed. `Genome` cannot be a trait
object for four independent reasons — `mutate` and `crossover` are generic over the RNG,
`crossover` takes `&mut Self`, `Clone` requires `Sized`, and `Context` is an associated type that
differs per representation — so that axis must stay a match. `Fitness` has none of those problems;
object-safety and `Send + Sync` on `dyn Fitness` were verified by compilation, not inferred. The
arm count falls from 16 to 8 places today, and adding a fifth objective costs one line instead of
four arms.
**Rejected:** the nested three-axis match previously specified. Its stated justification — that
`Evolver::run<F>` is generic so `Box<dyn Evolver>` is not viable — is correct but concerns the
*evolver* axis; it was never an argument about the objective axis, which is the collapsible one.
**Three requirements, each of which fails silently:** the forwarding impl must live beside the
trait (orphan rule); every method must be forwarded including the defaulted `direction` and
`evaluate_population`, or a maximizing objective runs backwards and a Python objective falls back
to the per-individual rayon fan-out §8 forbids; and the erasing match must be a factory re-run per
replicate, since §8.1 requires per-run objective instances.
**Cost accepted:** one virtual call per `evaluate`, behind `num_epidemics` complete epidemics.
**Affects:** `official_spec_sheet.md` §1 and §8; `get/src/fitness.rs`; GitHub #26, #19.
Settles `collab.md` #16.

## 2026-08-04 17:43 — Michael & James — Network size, population size and replicate count multiply into memory
**Chose:** record in §8.1 that peak memory is about
`network_size² × 4 × population_size × min(max_cores, n_replicates)`, with a worked table, and
require the Python layer to document it on `run` beside `max_cores`.
**Why:** the three parameters interact rather than add, and nothing said so. Expression materializes
the whole population as `Vec<Graph>` before scoring, and `Graph` is a dense `n × n` matrix however
sparse the graph is (§2), so the cost is quadratic in network size and then multiplied by every
concurrently-executing replicate. The failure mode is unintuitive: a user whose configuration ran
fine raises `max_cores` to exploit a bigger machine and multiplies peak memory by the same factor.
**Also recorded:** replicate-level concurrency is preferred over population-level, because every
generation is a barrier and evaluation times vary widely under a stochastic objective. Steady-state
has no choice — it scores two children per mating event, so its population-level parallelism is
two-way regardless of `max_cores`, which is the same conclusion §6.3 reaches from the FFI side.
**Affects:** `official_spec_sheet.md` §8.1; GitHub #20; the Python interface work in #29.

## 2026-08-04 17:52 — Michael & James — Short epidemics are re-rolled, and both constants are config fields
**Chose:** port the `legacy/main.cpp` re-roll. An outbreak shorter than `min_epidemic_length` is
discarded and re-simulated, up to `max_epidemic_retries` attempts, keeping the final attempt
whatever it produces. Both are config fields under `[fitness]`, defaulting to the C++ constants —
`min_epidemic_length = 3` (`mepl`) and `max_epidemic_retries = 5` (`rse`).
**Why:** a fizzled outbreak says the dice went badly, not that the network is poor. Without the
re-roll a large share of evaluations return near-nothing and selection chases the dice rather than
graph structure. The C++ has always done this and every historical result was produced under it, so
matching the defaults keeps our numbers comparable.
**Exposed rather than hardcoded** because it is a **biased resample, not variance reduction** — it
shifts expected fitness upward by an amount that depends on how often a given graph fizzles. That
makes it categorically different from raising `num_epidemics`, and a user who wants the unbiased
behaviour must be able to turn it off. `min_epidemic_length = 1` disables it, since every epidemic
has `length >= 1` under the §5.2 convention.
**Rejected:** (a) dropping the mechanism as statistically unclean — the fizzle problem it solves is
real and would return. (b) Hardcoding the constants as the C++ did, which would make the bias
unavoidable and invisible.
**Affects:** `official_spec_sheet.md` §5.2 and §7 (schema, example, and the `>= 1` validation
checks); GitHub #17 and #24. Settles `collab.md` #17.

## 2026-08-04 18:07 — Michael & James — Epidemics are seeded by position, reusing the §8.1 replicate scheme
**Chose:** the batch seed seeds a generator whose output stream *is* the epidemic seed list, and
epidemic `i` attempt `a` takes draw `i × max_epidemic_retries + a`. Explicitly the same mechanism as
§8.1's replicate seeding, not a second one — §5.2 now points at §8.1 rather than restating it.
**Why:** the re-roll agreed earlier today is conditional on a graph's own outcome, so a graph that
retries consumes extra draws. Drawn sequentially from one stream, that offsets **every subsequent
epidemic** in the evaluation relative to graphs that did not retry, and the desynchronisation runs
to the end of the batch. Position-indexed seeds resynchronise at the next epidemic index — the same
property §8.1 already relies on, where asking for 50 replicates leaves the first 30 untouched.
**The framing matters and was corrected mid-discussion.** This is not "the re-roll breaks CRN".
Every graph draws from an *identical* pool of dice; what differs is which of those common draws it
stops on, and that is what a retry is. The randomness stays common; the stopping rule is
outcome-dependent by design.
**Rejected:** (a) drawing epidemic seeds sequentially from a per-graph RNG — the cascade above.
(b) Hashing the `(batch, i, a)` tuple with a bespoke mix, which was the first suggestion here and
was wrong to propose: §8.1 had already settled this shape, and a second description of one scheme
drifts. Drawing from a stream also sidesteps the collision hazard §8.1 warns about with `master ^ i`.
**Also:** position-indexing makes scoring order-independent, so a population evaluated across rayon
workers reproduces regardless of which worker reaches which graph first.
**Affects:** `official_spec_sheet.md` §5.2 (pointing at §8.1); GitHub #17 and #18. Builds on the
re-roll decision of 2026-08-04 17:52.

## 2026-08-04 18:13 — Michael & James — `epi_prof_match` RMSE is fixed by the target's length
**Chose:** iterate `0 .. target.len()`; where the run is shorter treat its value as `0`; where the
run is longer ignore the surplus; divide by `target.len()` and take the square root. Matches
`legacy/main.cpp:545-553`.
**Why:** comparability with the historical C++ results, consistent with adopting the C++ conventions
for `length` and `profile` earlier the same day. It also closes a question #17 had carried as
explicitly undecided since it was filed.
**Consequence worth stating, because it is asymmetric:** a run that burns out early is penalised by
the entire remaining target, while a run that outlasts the target is not penalised for the
overshoot at all. So the objective rewards *matching or exceeding* the target's tail rather than
matching it exactly. Inherited deliberately rather than corrected — changing it would make our
scores incomparable with the archive, which is the whole reason the C++ conventions were adopted.
**Rejected:** normalising by the longer of the two lengths, or penalising overshoot symmetrically.
Both are defensible statistically and both break comparability.
**Affects:** `official_spec_sheet.md` §5.2; GitHub #17.
## 2026-08-04 18:25 — Michael & James — Union merge narrows to the two append-only docs; in-place amendment must be announced
**Chose:** `merge=union` now applies to `decisions.md` and `collab.md` only. `issues.md`,
`hotfixes.md` and `traps.md` take git's normal 3-way merge. Separately, announcing an in-place
amendment to a shared doc in `collab.md` first is a **rule**, not a courtesy. And `CLAUDE.md`'s
"an agent never merges a PR at all" is reworded to "never merges *unprompted*".
**Why the narrowing:** the three removed files are **churn lists** — deleting an entry is a normal
operation there, and union merge cannot express a deletion. Measured 2026-08-04: a one-sided delete
survives, but a delete racing with any edit to the same region is silently discarded and the entry
returns. Worst case is `hotfixes.md` — you remove an entry because you reverted the code, the other
owner adds a `Last checked:` line, and the merge resurrects an entry claiming a hotfix is in the
tree that is not. The trade taken is a conflict on concurrent appends to those three: loud and
occasional beats silent and wrong.
**Why the announcement is a rule:** it is the only mechanism that prevents the concurrent-edit case,
because git will not warn you — two people editing the same existing line makes union keep both
versions, reported as one insertion with no conflict, measured the same day on a 250-line file.
**Why the merge rule was reworded:** the absolute form was overridden twice within hours of being
written, both times correctly. A rule that correct behaviour keeps violating is stated wrong.
**Rejected:** dropping `merge=union` entirely — it still does its job for the append-only case,
which is the overwhelming majority, and without it every concurrent session ends in a conflict.
**Affects:** `/.gitattributes`; `.claude/CLAUDE.md` ("Pull requests", union-formatting rule 5);
`.claude/work/traps.md`. Settles `collab.md` #18 and #19.

## 2026-08-04 19:05 — James — Three mutation-contract entries land after the meeting block, out of date order
**Chose:** The two entries below were written 2026-08-03 during GitHub #10 and were never committed
— they sat unstaged on `jsargant_mutation_contract` while `main` moved on. They are appended here,
at the tail, rather than inserted at their chronological position among the 2026-08-03 entries.
**Why:** inserting them mid-file is an in-place edit of a shared union-merged document, which the
2026-08-04 18:25 decision makes an announce-first operation. Appending out of order costs a reader
one confusing timestamp; inserting costs a silent duplicate if Michael is editing the same region.
The cheap failure is the right one to take.
**Affects:** the reading order of `decisions.md` only — no code. Recovered during `/start` on
2026-08-04, after PR #30 and PR #33 had both merged.
*Recorded 2026-08-04 19:05 — James, recovering the uncommitted #10 close-out.*

## 2026-08-03 21:10 — James — The `mutate_child` tests count mutations on the existing `IndexGenome` stub
**Chose:** `IndexGenome::mutate` in `common.rs`'s test module increments the index instead of being
a no-op, so the index doubles as a mutation counter and the four `mutate_child` tests read the count
directly. No second stub type was added.
**Why:** the helper's whole job is deciding *how many times* `Genome::mutate` is called, and the
existing stub's `mutate` was empty, so there was nothing to assert on. Checked before changing it:
the only `mutate` call anywhere in `common.rs` is inside `mutate_child` itself, so no selection test
runs an individual through a mutation path and the change is inert for all of them — 97 pre-existing
tests stayed green.
**Rejected:** (a) A separate `CountingGenome` stub — twenty lines of trait boilerplate to do what
one line on the existing stub does. Proposed first and rejected by James as overengineered, which it
was. (b) Inferring the count from a real `EdgeEditGenome` — a reroll can land on the value the gene
already held, so an applied mutation becomes invisible and the count-bound test would silently
undercount while still passing. That is the failure this task exists to prevent, so the test must
not share it.
**Consequences:** `IndexGenome` now serves two test suites, and a future selection test that mutates
its individuals would see their identities shift. The type's doc comment says so at
`common.rs:274` area.
**Affects:** `get/src/evolver/common.rs` test module. Implements the verification half of GitHub #10.
*Recorded 2026-08-03 21:10 — James, during #10 verification.*

## 2026-08-03 21:12 — James — Verification tests are fault-injected before an item is ticked `[x]`
**Chose:** Every test written to verify #10 was checked by breaking the code it guards and
confirming the test fails, then reverting. Three injections: an exclusive `1..max` count range (fails
the count-bound test), an extra `0..=count` loop pass (fails the exactly-one and count-bound tests),
and `default_max_mutations` returning 2 (fails the config default test). Each revert was verified by
reading the diff, not by assuming.
**Why:** a passing test proves nothing about whether it would catch the regression it was written
for, and `CLAUDE.md` makes `[x]` mean *seen verified*. A test that passes because it asserts
something trivially true is exactly how a false `[x]` gets recorded, which that file names as the
most expensive failure mode here. The injection is what turned two `[~]` items into `[x]`.
**Rejected:** ticking on "97 → 103 tests, all green" alone — that shows the tests run, not that they
bite.
**Affects:** the `[x]` claims on the `mutate_child` and config items in this task's `plan.md`;
evidence recorded in `work/current/history.md`.
*Recorded 2026-08-03 21:12 — James, at the #10 save.*

## Task complete: mutation-contract — 2026-08-03, recorded 2026-08-04 19:05
Archived to `.claude/work/archive/2026-08_mutation-contract/`. GitHub #10 shipped as **PR #30**
(`a8cbf27`), which **merged 2026-08-04 15:58 UTC as `79f7948`** and closed #10 — the original
wording of this marker said the merge was carried forward, and it has since happened.
`Genome::mutate` now means exactly one mutation, `common::mutate_child` owns both dice rolls,
`MAX_MUTATIONS` is deleted, and `Config::max_mutations` defaults to 1. 103 tests green, up from 97.
*Task marker · mutation-contract · recorded 2026-08-04 19:05 — James.*

## 2026-08-04 19:39 — Michael & James — A nodeless graph keeps `length = 0`; it is not an inconsistency
**Chose:** `sir_sim` returns `length: 0` with an empty profile for a graph with no nodes, and that
stays. Recorded on the function's doc comment and on `an_empty_graph_produces_no_epidemic`, both
saying explicitly not to "fix" it to 1.
**Why:** after the §5.2 amendment of the same day, every *real* epidemic has `length >= 1`, because
even a lone patient zero occupies the burnout step. So zero stopped meaning "no transmission" and
started meaning **"no epidemic existed to measure"** — a statement only a nodeless graph can make.
The two are different claims and the type should be able to express both.
**Rejected:** returning `length = 1` for consistency, so every call satisfies `length >= 1`. It is a
simpler invariant to state and to validate against `min_epidemic_length`, and it was rejected because
it asserts that a timestep elapsed in a graph where nothing could happen. Cheap consistency bought
with a false statement.
**Why it is written into the code and not only here:** the tidy-up is obvious and the test passes
either way, so nothing would stop a future reader from making it. The doc comment is the only thing
between that reader and a silent semantic change.
**Affects:** `get/src/sir.rs` `sir_sim` early return and its doc; the empty-graph test. Arises from
the amendment recorded 2026-08-04 17:40.

## Task complete: sir-conventions — 2026-08-04
Archived to `.claude/work/archive/2026-08_sir-conventions/`. Entries below this line belong to later
tasks. GitHub #34 closed: `sir_sim` now counts the burnout step and emits the terminating zero, on
`main` via PR #36, 110 tests green. Carried forward, not resolved: **PR #37**, the one-line spec
status-row tidy, still open.

## 2026-08-04 22:10 — Michael — The SIR seam is "sample an epidemic" vs "read one", chosen for fork-ease
**Chose:** `sir.rs` owns how epidemics are *sampled* — `SirBatchParams`, `epidemic_seeds`,
`batch_epidemics`, including the short-epidemic re-roll and the position-indexed seeding.
`fitness.rs` owns how one is *read* — `EpidemicScorer` plus the three objectives, each a thin
reading over the shared batch. A fourth epidemic objective is a closure over
`EpidemicScorer::mean`.
**Why:** the standing constraint on this work is that someone should be able to fork the repo and
add their own objective without deep Rust. The re-roll and the seeding are the only subtle parts,
and both fail *silently* when reimplemented slightly wrong — a broken common-random-numbers scheme
produces entirely plausible numbers and a run that selects on dice rather than graph structure.
Putting them in one public function means a forker cannot get them wrong by copying, because there
is nothing to copy.
**Rejected:** a separate `sir_fitness.rs` module, to stay clear of James's #15/#19 edits to
`fitness.rs` — that was merge-coordination reasoning dressed up as design, and not worth a
permanent split in the codebase. Also rejected: giving each objective its own batch loop, which
would hand a forker ~40 lines of seeding logic to get wrong.
**Affects:** `get/src/sir.rs`, `get/src/fitness.rs`. Issue #17, PR #40.

## 2026-08-04 22:12 — Michael — Prefer explicit loops to iterator chains in this codebase
**Chose:** write plain `for` loops with an accumulator where an iterator chain would need a
turbofish, a closure returning through an `Option`, or more than about two adapters. Keep comments
terse and aimed at someone new to the code; point at `official_spec_sheet.md` rather than restating
it. Applied across `batch_epidemics`, `epidemic_seeds`, `EpidemicScorer::mean` and
`EpiProfMatch::rmse`; comments over the two files went from 347 lines to 290.
**Why:** both owners have to be able to read every line of this repo, and one of us does not write
Rust. `runs.iter().map(read).sum::<f64>() / runs.len() as f64` is idiomatic and stops a reader
cold; the four-line loop does not. The cost is a few more lines, which is cheap next to a file one
owner cannot review.
**Rejected:** idiomatic-Rust-first, on the grounds that a reader can learn the idioms. True, and
irrelevant — the constraint is what gets reviewed today, not what is learnable. Also rejected:
leaving the comments long on the grounds that more explanation is safer, when most of the length
was restating the spec sheet, which is the authority and drifts the moment it is copied.
**Affects:** all of `get/src/`, and future sessions. Recorded in `CLAUDE.md` under Conventions.

## 2026-08-04 22:14 — Michael — The #18 batch-seed stub is one method body, deliberately
**Chose:** `EpidemicScorer::batch_seed` returns the run seed unchanged, and issue #18 replaces
exactly that one method with the run seed plus an atomic evaluation counter. No caller moves, and
no seed argument is threaded through the three objectives.
**Why:** `Fitness::evaluate` takes `&self`, so #18's counter has to live on the objective as an
`AtomicU64` — which means the seam belongs at the point where a batch seed is *produced*, not
where it is consumed. Threading a `batch_seed` parameter through `evaluate` would have meant
changing the trait, which is spec §5 and not #17's to change.
**What this does and does not break, which is narrower than "seeding is unfinished":** common
random numbers *within* a batch are already correct, because the seed does not vary with the
graph. What is missing is variation *across* batches, so a run currently optimizes against one
frozen sample of the disease. Relative fitness inside one evaluation is meaningful; a whole run is
not yet research-usable.
**Rejected:** implementing the counter inside #17 anyway, since it is small. It is #18's whole
content, and folding it in would leave #18 empty and the PR harder to review.
**Affects:** `get/src/fitness.rs` `EpidemicScorer::batch_seed`; `hotfixes.md`; issue #18.

## 2026-08-04 16:40 — James — #14 and #15 are two tasks, not one, because one is mechanical and one is semantic
**Chose:** Build GitHub #14 (`evaluate` → `express_and_score`) and #15 (stop converting direction
inside the engine) as **separate tasks with separate PRs**, #14 first. #15 waits for #38 to merge and
branches off `main` rather than stacking on `jsargant_express_and_score`.
**Why:** they touch the same three files, which first argued for combining them — but that argument
is about *concurrent* work and was left over from when #14/#15 were Michael's and #10 was mine.
Sequentially, by one person, the overlap costs nothing. What does cost something is mixing a
**mechanical rename that touches every call site and test name** with a **semantic change to what
numbers come out**: in one diff, the rename noise is exactly what hides the behaviour change from a
reviewer, and Michael reviews these. Each issue also has its own `Verify by:` and closes its own
gate, which `CLAUDE.md`'s "keep one task per task" asks for.
**Rejected:** (a) One combined task — the review hazard above, and a plan that closes on two issues
at once. (b) Stacking #15's branch on #14's so it could start immediately — rejected by James on
2026-08-04; a stacked branch means review changes on #38 land under #15 too, and the queue is not
urgent enough to pay that.
**Consequences:** #14 churns call sites that #15 then edits again — a few lines, deliberately
accepted. #15 is blocked on Michael merging #38, and that is the only thing blocking it.
**Affects:** GitHub #14 (PR #38), #15; `get/src/evolver/common.rs`, `mod.rs`, `steady_state.rs`.
*Recorded 2026-08-04 16:40 — James, at the #14 save.*

## 2026-08-04 16:42 — James — The sole-entry invariant already held, so #14 is documentation not repair
**Chose:** Implement #14 as a rename plus doc comments, and say so plainly in the PR body rather
than implying a fix. Verified before starting: the only `evaluate_population` call in `get/src/` was
already inside the function being renamed, and the only `.evaluate(` call is `fitness.rs`'s own
default impl.
**Why:** what the issue asks for is an invariant — the engine never calls `Fitness::evaluate` or
`evaluate_population` directly — and an invariant that already holds is worth *documenting at the
two places someone would break it*, because both failure modes are silent: a direct call skips the
`Direction` conversion, so under `Maximize` every comparison runs backwards, and it skips the `NaN`
rejection, where a negated `NaN` sorts below `-inf` and wins every tournament. A reviewer told this
is a bug fix would look for the bug and find nothing.
**Rejected:** enforcing it mechanically — sealing the trait methods or making them `#[doc(hidden)]`.
Both fight the design: objectives must implement them, and `PyFitness` (#19) must override
`evaluate_population`. Not worth it while the engine is small enough that one grep checks the rule.
**Consequences:** the verification for #14 is "110 tests, unchanged", not a new failing-then-passing
test. Nothing here can regress, which is the point.
**Affects:** `get/src/evolver/common.rs` `express_and_score`; `get/src/fitness.rs` trait docs.
*Recorded 2026-08-04 16:42 — James, at the #14 save.*

## Task complete: express-and-score — 2026-08-04
Archived to `.claude/work/archive/2026-08_express-and-score/`. GitHub **#14** shipped as **PR #38**
(`9c397eb`), merged 2026-08-04 20:15 UTC as `168cc91`, and #14 closed as completed. `common::evaluate`
is now `common::express_and_score`, documented as the engine's sole path from a population to
fitnesses, with the invariant stated on the `Fitness` trait and both its methods. The invariant
already held, so no behaviour changed: 110 tests before and after. Entries below this line belong to
later tasks — the next is **#15**, orientation at the Python boundary.
*Task marker · express-and-score · recorded 2026-08-04 17:05 — James, at `/done`.*

## 2026-08-04 22:07 — James — `best_fitness_engine`: the rename is the enforcement mechanism
**Chose:** `EvolutionOutcome.best_fitness` becomes `best_fitness_engine`, and the struct gains
`pub direction: Direction`. `GenerationStats.best_fitness` keeps its name.
**Why:** The spec (§5.1) only asks that the field names "say the values are engine-oriented", and a
doc comment would have satisfied that reading. The rename was chosen instead because it **breaks
every existing reader at compile time**, and the reader that matters does not exist yet: #27 builds
the Python boundary that consumes this value, and the one failure this whole issue exists to prevent
is a boundary that forgets to convert. A doc comment is advisory to someone who may never read it; a
changed field name is a compiler error they cannot skip. `GenerationStats` is left alone because it
is a log column named in spec §6.4's table and does not itself cross the boundary — only the outcome
does.
**Rejected:** Keeping `best_fitness` with a doc note (invisible to the one caller who will get this
wrong); `best_fitness_oriented` ("oriented" is ambiguous about *which* way, which is the exact
confusion being removed).
**Affects:** `get/src/evolver/mod.rs:96-118`, `steady_state.rs:132`. Consumed by GitHub #27.
*#15 · recorded 2026-08-04 22:07 — James, at the implementation save.*

## 2026-08-04 22:08 — James — A `Maximize` test harness for steady-state, because the existing one hides the bug
**Chose:** Added `MostNodes` (a `Maximize` objective) and the test
`the_outcome_stays_engine_oriented_and_carries_the_direction` to `get/src/evolver/steady_state.rs`,
beyond what GitHub #15 asked for. Also verified both orientation guards by **reinstating** the
conversions and confirming they fail.
**Why:** Every pre-existing steady-state test uses `NodeCount`, which takes the default
`Direction::Minimize` — and orienting a minimizing objective is the *identity*. So the whole
conversion at `steady_state.rs:132` was invisible to the suite: removing it, or putting it back,
changed no test outcome. A change nothing can detect is a change nothing protects, and this one
sits directly on the path a wrong number would take to the user. The sabotage check is the same
argument applied to the tests themselves — a guard that passes either way defends nothing, so it
was run rather than assumed.
**Rejected:** Changing `NodeCount` to `Maximize` (it would have silently rewritten the meaning of a
dozen unrelated assertions); trusting `cargo test` green as evidence the guards work.
**Affects:** `get/src/evolver/steady_state.rs` test module; `get/src/evolver/common.rs:381`.
*#15 · recorded 2026-08-04 22:08 — James, at the implementation save.*

## 2026-08-05 15:09 — James — A task closes when the work is done, not when someone else clicks merge
**Chose:** Ran `/done direction-at-boundary` on 2026-08-05 with **PR #41 still open, unreviewed and
unmerged**, and started GitHub #24 as a new task rather than waiting. The plan's last `[ ]` —
"Michael reviews and merges #41" — was struck rather than ticked, because it was never a task.
**Why:** `/done`'s gate exists to stop unfinished *work* being archived, and the test that matters
is whether the item owes **this owner** an action. This one does not: PR #41's body opens with
`Closes #15.`, so the merge closes the GitHub issue with nothing left to do on James's side. Had it
been left open, `work/current/` would have been held hostage to another person's availability — and
`work/current/` holds exactly one task, so blocking on #41 blocks #24 too. That is the "empty
`archive/` next to a plan that no longer fits in context" failure `CLAUDE.md` warns about, arriving
by a route the rule did not anticipate.
**What this does NOT license:** archiving over an item that owes *you* something. The disposition
turned on reading the PR body for a closing keyword, which was checked on 2026-08-05, not assumed.
A PR closed **unmerged** is a different situation — #15 would reopen as new work.
**Rejected:** Waiting for the merge (blocks a tier-1 issue on someone else's calendar for an item
with no action attached); self-merging to clear the gate (`CLAUDE.md` — the other owner merges
yours, and the unavailable-and-blocking exception does not apply when an unrelated issue is
available to work on instead).
**Affects:** `.claude/work/archive/2026-08_direction-at-boundary/`; the `/done` gate's reading of a
`[ ]` item that belongs to the other owner.
*#15 · recorded 2026-08-05 15:09 — James, at `/done`.*

## Task complete: direction-at-boundary — 2026-08-05
Archived to `.claude/work/archive/2026-08_direction-at-boundary/`. GitHub **#15** shipped as
**PR #41** (`320fe68`, 3 files, +110/−41), which was **open and unmerged** at archive time — see the
entry directly above for why that did not block the close. The engine no longer converts fitness
direction internally: `generation_stats` takes no `Direction`, `SteadyStateEvolver::outcome` stores
one instead of applying it, and `EvolutionOutcome` carries `direction` alongside
`best_fitness_engine`. 128 tests green, both guards proven by sabotage. Entries below this line
belong to later tasks — the next is **#24**, the `config.rs` schema.
*Task marker · direction-at-boundary · recorded 2026-08-05 15:09 — James, at `/done`.*

## 2026-08-05 15:47 — James — The flatten wins over #24's unknown-key rejection, because serde cannot do both
**Chose:** `FitnessConfig`'s three epidemic variants share one `#[serde(flatten)] sir: SirParams`
block, and a stray key under `[fitness]` — notably a leftover `seed` — is **silently ignored**.
Pinned by `an_unknown_fitness_key_is_ignored_rather_than_rejected` in `get/src/config.rs` so the
behaviour is recorded rather than rediscovered. Catching it moves to `Config::validate` (#23).
**Why:** GitHub #24 asks for both the flatten and for `seed` to be "rejected as an unknown key",
and those are mutually exclusive in serde: a `flatten` field is deserialized through a buffered
content map, so `deny_unknown_fields` never fires. Measured 2026-08-05 — a stray `seed = 42` parsed
clean, and adding `deny_unknown_fields` to `SirParams` itself changed nothing. Spec §7 states the
flatten as a requirement ("flatten the shared block rather than triplicating it"), while the
rejection appears only in the issue's `Verify by` line, so `CLAUDE.md`'s "the sheet is the intent"
decides it.
**Why this is not just a shrug:** the failure it leaves open is the silent kind. Someone migrating a
pre-#24 config keeps `seed = 42`, gets no error, and runs under a different seeding model than they
believe — the master seed now comes from the `run` call. That is worth catching, but it has to be
caught by something that sees the raw text, which `validate` does and the parser no longer can.
**Rejected:** (a) Triplicating the parameters per variant so `deny_unknown_fields` works — delivers
the verify line exactly and contradicts the sheet, which is the one thing an agent may not do here.
(b) A phantom `Option<u64> seed` field existing only to be rejected — keeps the flatten and catches
the exact case, but adds a field the sheet does not have, which is a joint-meeting change.
**Affects:** `get/src/config.rs` `FitnessConfig`/`SirParams`; GitHub #23's `Config::validate`;
`collab.md` item 25.
*#24 · recorded 2026-08-05 15:47 — James, during the config-schema implementation.*

## Task complete: mdube_format_and_readability — 2026-08-06
Archived to `.claude/work/archive/2026-08_mdube_format_and_readability/`. GitHub **#22** shipped as
**PR #43** (`971feef`, 16 commits, `mdube_format_and_readability` → `main`), open and unmerged at
archive time, assigned to James, body carries `Closes #22`. One tree-wide `cargo fmt` commit, the
`needless_return = "allow"` lint decision, and two rounds of pure readability pass across the whole
tree (naming, comment density, explicit-loop convention) — no behavior changes. `generational.rs`
stayed out of scope throughout, per the issue body. 135 tests green, `cargo fmt -- --check` clean on
the branch (not yet on `main` — `traps.md`'s bare-`cargo fmt` entry stands until #43 merges).
Carried forward, not resolved: `collab.md` #27 (`Swap`'s degree floor, `> 2` vs. the Java original's
`>= 2`), and the SIR-batch-seed hotfix (blocked on #18). Entries below this line belong to later
tasks.
*Task marker · mdube_format_and_readability · recorded 2026-08-06 — Michael, at `/done`.*

## 2026-08-06 — Michael — `pull_main.sh`: a `SessionStart` hook fast-forwards `main` automatically
**Chose:** A new hook, wired into `settings.json`'s `SessionStart` array ahead of `session_brief.sh`.
It fast-forwards local `main` to `origin/main` when the current branch is `main` and the
fast-forward is clean; on any other branch, or if `main` has diverged, it does nothing but print one
line. Never merges, rebases, or discards anything — verified in a scratch repo across all three
paths (clean fast-forward, local commits origin doesn't have, and a dirty working tree blocking the
merge) before wiring it in.
**Why:** `.claude/work/*.md` docs and `CLAUDE.md` itself route around a PR by design (routing table,
above), so they only reach a second machine on that machine's next `git pull main` — which nothing
was prompting anyone to run. That gap is what let `collab.md`'s two independent item-**20**s happen:
both authors were looking at their own stale last-synced copy when they picked the next number
(`collab.md` #20-collision, #29-collision). Pulling automatically at session start removes the stale
window for the common case — two sessions on different days — though not true same-minute
concurrency, which no sync-on-start scheme can close.
**Rejected:** (a) A step added to `/load`'s instructions instead of a hook — doesn't fire if a
session never runs `/load`, and the whole point is to close the gap unconditionally. (b) Auto-merging
or rebasing on divergence — silently rewriting history at session start is exactly the kind of
destructive-by-default behavior this project's hooks avoid elsewhere; warn-and-leave-untouched matches
`block_env_commands.sh` and `show_hotfixes.sh`'s existing non-destructive posture. (c) Redesigning
`collab.md`'s numbering scheme itself (e.g. composite `date-author` keys) to make collisions
structurally impossible rather than just less likely — bigger change, binds James's practice the same
way the numbering convention does, left as a possible future `collab.md` proposal rather than done
unilaterally here.
**Affects:** `.claude/hooks/pull_main.sh` (new); `.claude/settings.json` `SessionStart`;
`.claude/hooks/README.md`. Per the routing table, this is the strict PR case — opened as PR, not
pushed to `main` directly like the docs fixes earlier this session.

## Task complete: config-schema — 2026-08-05
Archived to `.claude/work/archive/2026-08_config-schema/`. GitHub **#24** shipped as **PR #42**
(`39c408a`, 2 files, +245/−18), **merged** as `988457e` by Michael; issue #24 closed
2026-08-05T22:17:43Z by the body's `Closes #24.`. `get/src/config.rs` now parses spec §7's schema:
`FitnessConfig`'s four variants with the SIR block flattened, `SirParams` gaining
`num_epidemics`/`min_epidemic_length`/`max_epidemic_retries` and losing `seed`, `GenomeConfig::Sda`
losing `num_chars`, and `config.example.toml` rewritten to match. 135 tests green on `e42ffde`.
`Config::validate` was deliberately **not** touched — that is #23.
One criterion was dispositioned rather than delivered: #24 asked for a stray `seed` under
`[fitness]` to be rejected, which serde cannot do through a `#[serde(flatten)]`. Reasoning in the
2026-08-05 15:47 entry above; the check moves to #23's `validate` per `collab.md` #25.
**Reconstructed record:** the machine crashed before this task's final `/save`, leaving
`history.md` empty and no `handoff.md`. The archived `history.md` was rebuilt from `plan.md`, the
commit trail and the GitHub API, and says so at the top — the narrative was lost, the rationale was
not, because it had already been written here and to `collab.md` before the crash.
Carried forward, not resolved: `collab.md` **#24** (the `Profile*.dat` format, for #26) and **#25**
(unknown `[fitness]` keys, for #23), both still awaiting Michael; and the SIR-batch-seed hotfix,
blocked on #18. Entries below this line belong to later tasks.
*Task marker · config-schema · recorded 2026-08-05 22:20 EDT — James, at `/done`.*

## 2026-08-06 00:05 — James — `ConfigError::Validation` keeps the field and the constraint apart
**Chose:** A struct variant, `Validation { field: &'static str, constraint: String }`, rather than
a single pre-formatted `String` or one enum variant per check. `ConfigError` also gained `Display`
and `std::error::Error`, which it had neither of before.
**Why:** Spec §7 requires a bad config to reach the user as a Python exception "naming the offending
field and its constraint". Keeping the two apart means the FFI can compose its own message instead
of passing an opaque blob through, and — the part that actually pays off daily — the twelve tests
assert on `field` rather than substring-matching prose, so rewording a constraint message does not
break a single test. `Display` exists so `get/src/lib.rs` can format with `{err}`; it was using
`{err:?}`, which would have sent Python `Validation { field: "max_mutations", .. }`.
**Rejected:** (a) `Validation(String)` — simplest, but every test then matches prose and the FFI can
only forward the blob. (b) A dedicated `ValidationError` enum with one variant per check — precise
and exhaustively matchable, but ~12 variants to maintain for a message the user reads once, and
heavier than §7 asks for.
**Affects:** `get/src/config.rs` `ConfigError`; `get/src/lib.rs:30`; the FFI work in #19/#29, which
inherits this shape.
*#23 · recorded 2026-08-06 00:05 — James, during the config-validate implementation.*

## 2026-08-06 00:07 — James — Parsing and validating stay separate, and the `[fitness] seed` check can only live in the parse path
**Chose:** `from_toml_str` parses **without** running §7's constraints; `from_path` parses and then
calls `validate`, because it is the TOML front end. The stray-`seed` check sits in `from_toml_str`,
reading the raw text through a loose `toml::Value` parse before deserialization.
**Why:** §7 says validation must be one function "both front ends call" — `from_path` for TOML,
PyO3 for Python — which is satisfied by the front end calling it, not by welding it onto every
parse. Keeping `from_toml_str` unvalidated is also what lets a test build a config that breaks
exactly one constraint. The `seed` check cannot be in `validate` at all: `validate` takes `&self` on
an already-parsed `Config`, and the key is gone by then — serde buffers a `#[serde(flatten)]`
field's content, so `deny_unknown_fields` never fires (`traps.md`, measured 2026-08-05). It follows
that the check is TOML-only by construction; the Python front end has no text to inspect, which is
acceptable because the hazard is specifically an *old TOML file* still carrying `seed = 42`.
**Rejected:** (a) A general unknown-key sweep over `[fitness]` — hand-rolls what serde does
everywhere else and would reject keys as the schema grows; the narrowness is pinned by
`an_unknown_fitness_key_other_than_seed_is_still_ignored`. (b) Leaving it out and keeping
`collab.md` #25 open — the migration failure is silent, which is the kind worth spending a check on.
(c) Making `from_toml_str` validate too — symmetrical, but removes the only clean way to construct
an invalid `Config` in a test.
**Supersedes:** the pinned behaviour in #24's `an_unknown_fitness_key_is_ignored_rather_than_rejected`
(2026-08-05 15:47, "The flatten wins over #24's unknown-key rejection"). That entry's *mechanism*
still holds exactly as written — serde still cannot do this. What changed is where the check went,
which that entry itself nominated. The test is replaced, not deleted.
**Affects:** `get/src/config.rs` `from_toml_str` / `from_path` / `validate` / `reject_fitness_seed`;
answers `collab.md` item 25.
*#23 · recorded 2026-08-06 00:07 — James, during the config-validate implementation.*

## Task complete: sir-objectives — 2026-08-06
Archived to `.claude/work/archive/2026-08_sir-objectives/`. GitHub **#17** shipped as **PR #40**,
merged 2026-08-04 as `a53375e` and closed completed: `epi_spread`, `epi_length` and `epi_prof_match`
in `get/src/fitness.rs`, over `SirBatchParams` / `epidemic_seeds` / `batch_epidemics` in
`get/src/sir.rs`, with the short-epidemic re-roll and position-indexed seeding of spec §5.2. The
`SirFitness` placeholder and its `todo!()`s are gone. Verified on Michael's machine 2026-08-06 on
`main` at `ed198c4`: 135 tests pass, and `0dab610` is an ancestor of `main`, so the PR-lag trap
stranded nothing.
**Closed two days late, from a machine 40 commits behind.** The work finished 2026-08-04 and merged
the same day, but the gate never ran; `work/current/` is gitignored, so this task's record lived on
one machine and no other could close it. Worth knowing that a merged PR is not a closed task here.
Carried forward, not resolved: the **SIR batch-seed hotfix**, still blocked on #18 (mine, next), now
committed and in every tree; and `collab.md` **#21** (user-supplied drop-in Rust objectives), which
gates **#26** and not #18. Dropped at this gate: the `cargo fmt` trap, per its own exit condition —
#43 merged and `cargo fmt -- --check` is clean on `main`. Anything below this line belongs to a
later task, not to sir-objectives.
*Task marker · sir-objectives · recorded 2026-08-06 — Michael, at `/done`.*
## Task complete: config-validate — 2026-08-06
Archived to `.claude/work/archive/2026-08_config-validate/`. GitHub **#23** shipped as **PR #45**
(`5fd8dbc` + `2c590f4`, 2 files, +528/−18), **open and unmerged at archive time**, awaiting Michael,
body carries `Closes #23.`. `Config::from_path` implemented over its `todo!()`, and `Config::validate`
added with every spec §7 constraint — the steady-state-only tournament floor, the generational-only
elite count, the weights delegated to `EdgeEditOperationWeights::validate`, and `python` skipping the
SIR checks entirely. A stray `[fitness] seed` is now rejected from the raw TOML, closing the
migration hazard #24 left open. 154 tests, up from 135; clippy `diff`-identical to a pre-edit
baseline; rustdoc and rustfmt unchanged.
Archived with its PR still open, on the disposition recorded 2026-08-05 15:09 — the item owes this
owner no action, since the body's closing keyword was verified on the remote. Third task closed this
way, after #15 and #24.
Superseded a test rather than only adding: #24's `an_unknown_fitness_key_is_ignored_rather_than_rejected`
is gone, replaced by a pair pinning both the new rejection and its deliberate narrowness.
Carried forward, not resolved: `collab.md` **#24** (`Profile*.dat` format, for #26) and **#27**
(`Swap`'s degree floor, awaiting James, a joint-meeting change either way); and the SIR-batch-seed
hotfix, blocked on #18, fifth cycle. `collab.md` **#25** was answered and needs only Michael's
acknowledgement. Entries below this line belong to later tasks.
*Task marker · config-validate · recorded 2026-08-06 00:38 EDT — James, at `/done`.*

## 2026-08-06 00:50 — James — `GenerationalEvolver::new` gets a backstop assert on `elite_count`
**Chose:** `GenerationalEvolver::new` will `assert!(elite_count < population.len())`, matching the
posture of `SteadyStateEvolver::new` (`get/src/evolver/steady_state.rs:156-178`). Settled before any
of #25 was written.
**Why:** spec §7 keeps the evolvers' `assert!`s as backstops for **direct Rust use** — tests and
embedding — precisely because an evolver is constructible without passing through `Config`. #23's
`Config::validate` now rejects `elite_count >= population_size` for every config-driven run, so this
assert can only fire on the direct path, which is exactly the path §7 says to keep covered. The
failure it catches is the silent kind: elites fill every slot, nothing breeds, and the run is a
fixed point that reads as a broken fitness function rather than a bad construction. Steady-state
already argues this in its own `new` — the two evolvers should not disagree about whether they trust
their caller.
**Rejected:** (a) Leaving `new` bare and relying on `validate` — correct for config-driven runs and
useless for the one path that can actually reach it. (b) Also asserting
`population.len() >= tournament_size`, for symmetry with steady-state — generational's tournament
sampling is **with replacement**, so a tournament larger than the population draws fine; that assert
would have no failure mode and would misinform a reader about what generational requires.
**Affects:** `get/src/evolver/generational.rs` `new`; the plan's `new` task in
`work/current/plan.md`.
*#25 · recorded 2026-08-06 00:50 — James, at `/start` for the generational evolver.*
## 2026-08-06 16:06 — Michael — Merged PR #45; the "archive with the PR still open" loop is now closed
**Recorded because James archived config-validate with this PR pending**, noting it "owes this owner
no action" — that disposition was right, and this is the other half of it. PR **#45** merged as
`334ef63`, and GitHub **#23** closed `completed` off the body's `Closes #23.` keyword, verified on
the remote rather than assumed.
**Checked before merging, not after:** the PR's `head.sha` and `git rev-parse origin/jsargant_config_validate`
both returned `2c590f4`, so the PR-lag trap had nothing to strand. Merged **locally** with `--no-ff`
and pushed, not with the GitHub button. The button would have been safe here — the PR touches only
`get/src/config.rs` and `get/src/lib.rs`, no `.claude/work/*.md` — but the habit is what stops the
one that isn't.
**Verified after merging:** `cargo test` gives **154 pass / 0 fail** on `334ef63`, matching the count
James recorded in his task marker exactly, so nothing regressed across the merge.
**Why this is worth an entry at all:** three tasks have now been archived with their code PR still
open (#15, #24, #23), and the archive README is written at archive time and never revisited. Without
a note on this side, the durable record of #23 would say "awaiting Michael" forever.
*Merge record · #23 / PR #45 · recorded 2026-08-06 16:06 — Michael.*

## 2026-08-07 — Michael — Batch seeds derive via `mix_seed` (SplitMix64), not a `ChaCha8Rng` stream position
**Chose:** `EpidemicScorer::next_batch_seed` derives a batch's seed as `mix_seed(run_seed, counter)`,
a small hand-written SplitMix64 (add a constant, two multiply-xor-shift rounds) — resolving the open
question left in `work/current/plan.md` when task 2 was written.
**Why:** the plan's own framing was right — the mix is O(1) and stateless, where a `ChaCha8Rng`
positioned by stream offset would need either replaying `counter` draws (O(counter) per batch, and
counter grows for the whole run) or an RNG whose implementation exposes exact stream positioning,
which `ChaCha8Rng` does not promise to keep stable across `rand_chacha` releases. §8.1 forbids
`run_seed ^ counter` outright — neighbouring run seeds would collide across batch numbers — and
explicitly permits `hash(master, i)` as the equivalent of its stream scheme applied to a different
index; SplitMix64 is exactly that hash.
**Rejected:** `run_seed ^ counter` (ruled out by §8.1, tested directly by
`neighbouring_run_seeds_share_no_batch_seed`); seeding a persistent `ChaCha8Rng` from `run_seed` and
drawing `counter` throwaway values to reach the batch's position (correct but O(counter) and adds a
second RNG type to the file for no benefit over the mix).
**Affects:** `get/src/fitness.rs` `mix_seed`, `EpidemicScorer::next_batch_seed`.
*#18 seed-derivation · recorded 2026-08-07 16:15 — Michael.*

## 2026-08-07 — Michael — `EpidemicScorer` restructured from five methods to two; per-objective duplication kept, not abstracted away
**Chose:** `EpidemicScorer` exposes only `next_batch_seed` and `mean_batch`. `mean`,
`mean_with_seed` and the `pub fn epidemics` pass-through — all added earlier the same session while
building #18 — were removed the same day, once the shape was actually exercised: each was a
single-caller wrapper, and three independent reviews (spawned specifically to test whether the
seeding machinery's complexity was forced or accidental) agreed the wrapper layer was the accidental
part. A second change went the opposite direction: a private `reading` method was added to each
objective to write its epidemic-reading closure once instead of twice, then **reverted the same
session** on request — the indirection cost more clarity for a reader copying an objective to write
their own than the duplication it removed. The duplication is guarded instead, by the test
`both_entry_points_use_the_same_reading`.
**Why:** the seeding mechanism itself (the atomic counter, `mix_seed`, one-seed-per-batch) was
independently confirmed to match the standard common-random-numbers pattern from
simulation-optimization and the counter-based-RNG recommendation for parallel reproducibility — not
a workaround, and left untouched. What was ours to simplify was the code built *around* it, and the
three-review process (constraint audit, refactor proposals, external-convention comparison) is what
distinguished the two rather than guessing. The `reading`-method reversal is a readability call, not
a correctness one: `evaluate` calling `self.evaluate_population(...)` directly was rejected earlier
in the same pass as a latent stack overflow, since the trait's *default* `evaluate_population` calls
`evaluate`.
**Rejected:** a blanket `EpidemicReading` trait enforcing the once-written reading at compile time —
works, but hides the real per-objective code inside a blanket impl, which cuts against the project's
"one owner does not write Rust" convention harder than three inline duplicated closures do.
**Affects:** `get/src/fitness.rs`, all of `EpidemicScorer` and the three `Fitness` impls.
**Detail:** `collab.md` #33 carries the full before/after and the sub-agent findings.
*#18 scorer-restructure · recorded 2026-08-07 16:15 — Michael.*

## 2026-08-07 — Michael — "Batch of graphs" and "original / oriented" are now `fitness.rs`'s stated vocabulary
**Chose:** two terminology fixes, both comment-and-naming only, no logic change. First, "batch of
graphs" is now used throughout for what an evolver scores in one call — explicitly **not** "a
generation", since a steady-state mating event scores only its two new children (§6.3), so
"generation" was wrong for the majority of a steady-state run. Second, `Direction`'s doc now names
**original** (what an objective's `evaluate` returns) and **oriented** (the value after
`Direction::orient`) as the two forms every fitness number takes, replacing prose that only
described a sign flip without naming either side of it.
**Why:** raised independently by the user reading the file cold and getting the wrong mental model
each time ("do these mean the same thing?", "is the run seed shared throughout evolution?") — both
questions the old wording invited. "Oriented" was picked over an invented term ("comparison score")
specifically because `orient`, "engine orientation" and `best_fitness_engine` already exist in
`evolver/common.rs`, `evolver/mod.rs` and `evolver/steady_state.rs` and in the sheet itself — so
`fitness.rs` now explains the codebase's existing vocabulary rather than adding a competing one.
**Rejected:** "comparison score" as the paired term for "original" — accurate, but would have made
`fitness.rs` the one file not using the word every other file and the sheet already use for the same
concept.
**Affects:** `get/src/fitness.rs` (comments and test names only); `get/src/sir.rs` — separately,
`batch_epidemics`/`SirBatchParams`/`coin_flip_batch` renamed to `simulate_epidemics`/
`SirSampleParams`/`coin_flip_sample`, since "batch" there meant one graph's epidemics, colliding with
the graphs-batch sense everywhere else. Neither `sir.rs` name is in the sheet, so no meeting was
needed. `Fitness::evaluate_population` and `SirRun` have the same defect and **are** sheet-named —
left untouched, raised as `collab.md` #32 for the joint meeting instead of changed here.
*#18 vocabulary · recorded 2026-08-07 16:15 — Michael.*
## 2026-08-06 21:03 — James — Generational's `outcome` takes the winner's graph from the final scoring pass
**Chose:** `GenerationalEvolver::outcome` moves the winner's `Graph` out of the vector
`express_and_score` returned on the last generation (`graphs.swap_remove(best)`), rather than
calling `best_genome.express(..)` the way steady-state's `outcome` does. It is a second, local
`outcome` method — not a shared helper.
**Why:** spec §6.2 asks generational specifically to use "the graph that scoring already built, so
the winner is never re-expressed", and generational is the strategy that has them: it scores every
individual every generation, where steady-state scores only the two children of each mating event
and so has nothing to reuse. Both paths return the identical graph — `Genome::express` takes
`&self` and `&G::Context` with no RNG, so it is deterministic — which makes this purely a choice of
which cost to pay: one extra expression per run, or one population's worth of `Graph`s held alive
across the loop.
**Rejected:** (a) Factoring the shared part of the two `outcome`s into `common.rs` — the right
long-term shape, but it means editing `steady_state.rs`, which #25 is explicitly scoped out of.
Raised as `collab.md` #36 instead of done unilaterally. (b) Re-expressing the winner like
steady-state, for symmetry — simpler, and contradicts the sheet on the one point where the sheet
speaks about generational in particular.
**Affects:** `get/src/evolver/generational.rs` `outcome`; `collab.md` #36 (renumbered from #32).
*#25 · recorded 2026-08-06 21:03 — James, during the generational implementation.*

## 2026-08-06 21:04 — James — §6.2's "track the best" is the best of the final population, not a running best-ever
**Chose:** the outcome reports the best individual of the **final** scored population. No best-ever
is tracked across generations.
**Why:** §6.2's phrase is a description of the per-generation loop, not a specification of the
report, and the two readings only differ in cases the engine already handles. At `elite_count >= 1`
under a deterministic objective they are identical, because the best is copied forward every
generation and cannot be lost. Where they differ — a stochastic objective — best-ever is actively
worse: it latches the luckiest sample of a genome, which is the same failure §6.2 gives as the
reason elites are rescored rather than keeping their old number. It would also let
`best_fitness_engine` disagree with the last row of `history`, and report a fitness no individual in
the returned population currently has.
**Rejected:** (a) A running best-ever genome/graph/fitness updated each generation — robust at
`elite_count = 0`, which is the one case it buys anything, and the config default is 1. (b) Raising
it as a `collab.md` item before implementing — the sheet is not contradicted by either reading, so
there was nothing to ask; recorded here instead so the reading is visible and reversible.
**Affects:** `get/src/evolver/generational.rs` `outcome` and `run`;
`the_outcome_reports_the_actual_best_and_its_graph`.
*#25 · recorded 2026-08-06 21:04 — James, during the generational implementation.*

## 2026-08-06 21:05 — James — `advance_generation` does not take the objective
**Chose:** dropped the `fitness: &F` parameter (and the `F: Fitness` bound) the `todo!()` stub
carried, leaving `advance_generation(&mut self, fitnesses: &[f64], rng: &mut R)`.
**Why:** generational rescores the whole population in `run` after advancing, so nothing inside
`advance_generation` scores anything — it only needs the *previous* generation's fitnesses, to rank
elites and to run selection. Keeping an unused `&F` would have meant keeping the stub's
`let _ = (..)` discard alive, which is exactly the kind of line that reads as an oversight later.
The asymmetry with steady-state's `mating_event`, which does take `fitness`, is real and correct:
that strategy scores its two children in place, because it never rescores anyone else.
**Rejected:** keeping the parameter for signature symmetry with `mating_event` — symmetry between
two methods that genuinely do different work is worth nothing, and it would have hidden that
generational scores in exactly one place.
**Affects:** `get/src/evolver/generational.rs` `advance_generation` and its caller in `run`.
*#25 · recorded 2026-08-06 21:05 — James, during the generational implementation.*

## 2026-08-07 — James — The clippy trap was retired with a successor, not deleted
**Chose:** replaced the `cargo clippy -- -D warnings cannot pass on main` entry with a shorter one
saying the opposite — the gate passes as of `94a4679`, so a warning is now yours — which keeps two
things the old entry carried: the `git stash -u` contamination pitfall hit on #24, and the
capture-the-baseline-before-editing recipe for the next time a non-empty baseline exists.
**Why:** #25's plan said "drop the entry once the warnings are gone", and dropping it outright would
have been the literal reading. But the entry's *exit condition* was about the claim in its title,
not about everything written underneath it, and the stash pitfall is still true and still expensive
— it is a property of `git stash -u` and this repo's `config.example.toml`, not of the dead-code
warnings. Deleting a churn-list entry also deletes whatever knowledge accreted in it, and this one
had accreted a technique from a different task.
**Rejected:** (a) Deleting it as written — loses the pitfall, and leaves nothing recording that the
gate flipped, so the next person diffing against a baseline has no way to know they need not. (b)
Keeping it and appending "no longer true" — a trap whose title contradicts its body is worse than
either, and `traps.md` is read by `/load` on every session.
**Affects:** `.claude/work/traps.md`; the `Verify by:` of any future task that says "clippy passes".
*#25 · recorded 2026-08-07 — James, at the `/done` gate for generational-evolver.*

## Task complete: generational-evolver — 2026-08-07
Archived to `.claude/work/archive/2026-08_generational-evolver/`. GitHub **#25** shipped as
**PR #46** (`349399e`, `ab68796`, `a30422e`, `7de4a66` — 2 files, +414/−17 on the test commit alone),
**merged** by Michael as `74de0b5` on 2026-08-07T14:51:59Z; issue #25 closed by the body's
`Closes #25.` `GenerationalEvolver::run` and `advance_generation` are implemented over their
`todo!()`s, `new` gained the `elite_count` backstop, and the two cleanups folded into the issue
landed with them. 176 tests green on the merged tree — verified *after* merging with Michael's
#18 rewrite of `fitness.rs`, not only on the branch.
The clippy gate flipped with this task: `cargo clippy -p get --all-targets -- -D warnings` exits 0
on `main` for the first time, because the two dead-code warnings every task since 2026-08-04 diffed
against **were** this evolver's unbuilt shell. The trap that recorded them is retired, with a
successor — see the entry above.
Two sheet questions leave this task **unresolved and pointed at the joint meeting**: `collab.md`
**#35** (§6.2's "track the best" versus the best-of-final-population that shipped — endorsed
Michael's amend-the-sheet option) and **#36** (whether the two evolvers' `outcome` methods should
share a helper in `common.rs`). Neither blocks anything; both are sheet or cross-file changes that
one owner may not make alone.
Carried forward, not resolved: `issues.md`'s `evaluate_population`/`SirRun` rename, unfiled and
blocked on the same meeting; `collab.md` **#27** (`Swap`'s degree floor), still James's, fifth gate.
**`hotfixes.md` is empty of live entries for the first time** — Michael's #18 removed the SIR
batch-seed hotfix after six cycles of carrying it. Entries below this line belong to later tasks.
*Task marker · generational-evolver · recorded 2026-08-07 — James, at `/done`.*

## 2026-08-07 19:20 — James — `extension-module` moves out of `[dependencies]`, gated behind the built module instead
**Chose:** `get/Cargo.toml`'s `pyo3` drops `extension-module` from `[dependencies]` entirely;
`[dev-dependencies] pyo3` carries `auto-initialize`. The built module supplies `extension-module`
from outside the manifest — maturin via `[tool.maturin] features = ["pyo3/extension-module"]`, or
`cargo build --features pyo3/extension-module` by hand.
**Why:** `extension-module` tells pyo3 to leave the Python C API symbols unresolved, for the
interpreter to supply at load time. `cargo test` produces an ordinary binary with no interpreter
behind it, so with the feature always on the whole suite fails to **link** — dozens of undefined
`Py*` symbols — however few tests touch Python. Measured on this repo before choosing the fix, not
assumed from pyo3's docs. The alternative sketched first (a local `[features] extension-module =
["pyo3/extension-module"]` passthrough) also worked, but the maturin-native form needs no such
passthrough and is what a real build will use anyway once `pyproject.toml` exists (`issues.md`).
**Rejected:** (a) The local `[features]` passthrough — functionally equivalent, verified working,
but reinvents what maturin already does. (b) Leaving `extension-module` on and testing `PyFitness`
only through a separate, non-`cargo test` harness — weaker verification, and this project's whole
posture is that untested code is unverified code.
**Affects:** `get/Cargo.toml`; every future pyo3-touching test; `traps.md` (new entry);
`.claude/reference/pyo3-maturin.md` §1; the `pyproject.toml` issue staged in `issues.md`, which now
must remember to set `[tool.maturin] features`.
*#19 · recorded 2026-08-07 19:20 — James, fixing the test harness before writing PyFitness.*

## 2026-08-07 19:50 — James — `PyFitness` routes both trait methods through one inherent `score_batch`
**Chose:** neither `Fitness::evaluate` nor `Fitness::evaluate_population` calls the other on
`PyFitness`; both call a private `score_batch`, which does the one call into Python.
**Why:** the trait's default `evaluate_population` calls `evaluate`. Had `evaluate` been written to
call `evaluate_population` — the seemingly natural "one graph is a batch of one" — deleting the
override would turn that pair into infinite recursion instead of a compile error, exactly the trap
`collab.md` #33 documents for `EpidemicScorer`. Routing both through an inherent method has no cycle
to fall into regardless of which override is present or absent.
**Rejected:** `evaluate` calling `evaluate_population` directly — reads more obviously correct and
is the pattern `EpidemicScorer`'s own `evaluate` used to use, before #18 restructured it for the
same reason.
**Affects:** `get/src/fitness.rs` `PyFitness`.
*#19 · recorded 2026-08-07 19:50 — James, writing PyFitness's evaluate/evaluate_population pair.*

## 2026-08-07 20:20 — James — `set_fitness_function` rejects registration against a non-Python config
**Chose:** `set_fitness_function` errors if `self.config.fitness` is not `FitnessConfig::Python`,
rather than storing the callable regardless of what the config selected.
**Why:** a stored-but-unused callable is indistinguishable, from Python, from a successfully
registered one — the run would score with whatever the config actually selected (an SIR objective)
while the user watched for their own function's numbers and never got them. Spec §8 already argues
this shape for the reverse case (a `python` config with nothing registered); the config-mismatch
direction is the same failure, just triggered from the other side. `FitnessConfig::type_name()` was
added so the rejection message names the configured objective in the words the user actually typed,
rather than a `Debug` dump of the variant's fields.
**Rejected:** storing unconditionally and letting `python_fitness()` be the only gate — technically
sufficient (the callable would simply never be read), but the config layer is meant to be the single
source of truth for which objective a run uses, and letting registration silently succeed against
the wrong config makes debugging "why isn't my function being called" a search through two files
instead of one error at the point of the mistake.
**Affects:** `get/src/lib.rs` `set_fitness_function`; `get/src/config.rs` `FitnessConfig::type_name`.
*#19 · recorded 2026-08-07 20:20 — James, adding set_fitness_function to GraphEvolver.*

## 2026-08-07 20:45 — James — `python_fitness` carries a temporary `#[allow(dead_code)]`, not a `#[cfg(test)]` hide
**Chose:** the seam #26 will call (`GraphEvolver::python_fitness`) is `pub(crate)`, always compiled,
and wears `#[allow(dead_code)]` with a comment and a matching `hotfixes.md` entry, rather than being
`#[cfg(test)]`-gated or left to produce a clippy warning.
**Why:** `#[cfg(test)]` would mean the method does not exist in the real build at all, so #26 could
not call it without first un-gating it — a needless extra step at the exact moment #26 lands. Leaving
the warning unsuppressed would break `cargo clippy -p get --all-targets -- -D warnings`, which #25
spent an entire task restoring to a real gate (`traps.md`). The `#[allow]` is scoped to the one
method, not the module, and both its removal condition and its owner are recorded.
**Rejected:** `#[cfg(test)]` — cheaper today, worse for #26. A crate-wide or module-wide
`#[allow(dead_code)]` — hides genuinely dead code elsewhere for the same span, which is a bigger
blind spot than one method needs.
**Affects:** `get/src/lib.rs` `python_fitness`; `hotfixes.md`.
*#19 · recorded 2026-08-07 20:45 — James, adding the seam #26's dispatch will call.*

## 2026-08-07 22:10 — James — GET gets a `pyproject.toml` at the workspace root, carrying the `extension-module` feature
**Chose:** a `pyproject.toml` at the repo root with `build-backend = "maturin"`,
`[tool.maturin] manifest-path = "get/Cargo.toml"` and `features = ["pyo3/extension-module"]`. Staged
as an unfiled issue earlier the same day, then built instead of filed, on instruction.
**Why:** the crate is a workspace member, so the manifest is not beside the pyproject and maturin has
to be pointed at it; and `extension-module` is deliberately absent from `get/Cargo.toml`
(2026-08-07 19:20) so `cargo test` can link, which means the build path is the only place left to
supply it. Verified end to end rather than by inspection: `maturin build` reports "Using build
options features from pyproject.toml", and the wheel installed into a throwaway venv imports,
constructs a `GraphEvolver` against a `type = "python"` config, registers a callable, and returns
both rejection paths to Python as `ValueError` with their messages intact. This is the first time
anything in GET has been callable from Python.
**Corrected in the doing:** the first version of the comment on that `features` line asserted that
removing it yields a wheel that fails to import. **Measured false** — with the line dropped, the
wheel has the same 75 undefined `Py*` symbols, no `libpython` in `ldd`, and imports fine on this
Linux/pyenv setup. The line is kept for macOS/Windows linkers and because it is the documented
configuration, and the comment now says exactly that, plus a warning not to read a green Linux
import as evidence it is unnecessary. Recording the correction because the wrong version is the
kind a later reader would reasonably trust.
**Rejected:** (a) Filing it as a tracker issue and leaving the repo unable to build a wheel —
overtaken by instruction, and the work turned out to be ~15 lines plus verification. (b) Putting the
pyproject inside `get/` beside the crate — then `pip install .` from the repo root does not work,
which is where a user would run it. (c) A `[features]` passthrough in `get/Cargo.toml` instead —
already rejected on 2026-08-07 19:20 for reinventing what maturin does.
**Affects:** `/pyproject.toml` (new); `.claude/reference/pyo3-maturin.md` §3, rewritten from "what
GET does not have" to what it now has; `issues.md`, whose staged entry was removed as resolved
rather than filed.
*#19 · recorded 2026-08-07 22:10 — James, after the pyproject.toml verification.*

## Task complete: pyfitness — 2026-08-08
Archived to `.claude/work/archive/2026-08_pyfitness/`. GitHub **#19** shipped as **PR #48** (8
commits, `6e2d262`..`b1f8557`), **merged** by Michael as `32ceb11` on 2026-08-08T13:57:26Z; issue
#19 closed by the body's `Closes #19.` `PyFitness` adapts a registered Python callable to the
`Fitness` trait on the batched contract, `impl Fitness for Box<dyn Fitness>` forwards every method
including both defaulted ones, `set_fitness_function` registers callable + direction with three
rejections, and `python_fitness` is the seam #26's dispatch calls. **198 tests**, up from 176 —
the delta counted from the diff rather than carried forward as a remembered number.
Two things landed that were not on the original plan. **`pyproject.toml`** (root, `manifest-path`
into the workspace member, `features = ["pyo3/extension-module"]`) — built rather than filed, on
instruction, making GET importable from Python for the first time. And **`.claude/reference/`**, a
new documentation lifetime for notes about how a dependency behaves, deliberately outside `work/`
so it cannot be mistaken for a churn list.
Two measured findings outlived the code. Calling Python from inside a rayon closure **deadlocks**
rather than merely running slowly — found by deleting the `evaluate_population` override to check a
test was not vacuous, and watching the suite hang for two minutes with no failure message. And a
claim written into `pyproject.toml`'s own comment was **measured false** the same session: a
featureless wheel imports fine on Linux, identical undefined symbols. Both are in `traps.md` and
`.claude/reference/pyo3-maturin.md`; the correction is in the 2026-08-07 22:10 entry above.
Carried forward, not resolved: the `#[allow(dead_code)]` on `python_fitness`, now committed and in
every tree, blocked on **#26** which is still open and unstarted; `issues.md`'s
`evaluate_population`/`SirRun` rename, unfiled and blocked on the joint meeting; and `collab.md`
**#35**, **#36**, **#37** all awaiting Michael, plus **#27** still awaiting James. Entries below
this line belong to later tasks.
*Task marker · pyfitness · recorded 2026-08-08 — James, at `/done`.*

## 2026-08-08 21:15 — James — The Python config schema is a mirror in `py_config.rs`, not `#[pyclass]` on `config`'s own types
**Chose:** `get/src/py_config.rs` holds a parallel set of `#[pyclass]` types — `PyConfig`,
`PyEvolutionConfig`, `PySelectionConfig`, `PyGenomeConfig`, `PyFitnessConfig`, `PySirParams`,
`PyOperationWeights` — mirroring `config.rs` field for field, and converts to TOML through explicit
`to_toml_value` matches rather than a `Serialize` derive.
**Why:** not a style preference — the two attribute sets are **mutually exclusive on the fitness
enum**, measured on pyo3 0.27.2 and serde 1.0.228 while building #29. pyo3 refuses a unit variant in
a complex enum ("not yet supported in a complex enum; change to an empty tuple variant instead"), so
`FitnessConfig::Python` would have to become `Python()`; serde then refuses exactly that with
"`#[serde(tag = "...")]` cannot be used with tuple variants". The tag is what deserializes
`type = "python"` for the hand-written TOML path, so annotating `config`'s enum directly would break
the file front end in order to serve the Python one. The same conflict rules out deriving
`Serialize` on the mirror, hence the explicit conversions — which also suit a codebase one owner
reads without writing Rust. Secondary benefit: every pyo3 attribute stays in one new file, which
matters with two owners editing the crate at once and #26 due to touch `lib.rs`.
**Rejected:** (a) `#[pyclass]` on `config.rs`'s types — impossible, above, and it was my first
preference because it eliminates drift. (b) A `Serialize` derive on the mirror — same blocker.
(c) Exposing config as plain dicts — loses the typed API #29 asks for and pushes schema errors to
run time.
**The cost is drift, and it is guarded rather than accepted:** the round-trip tests destructure the
parsed `Config` **exhaustively with no `..`**, so a field added to `config.rs` and not to the mirror
is a compile error in `py_config.rs`. Verified by adding a field and watching it fail with "pattern
does not mention field", not assumed.
**Affects:** `get/src/py_config.rs` (new); `get/src/lib.rs` `from_config` and the `#[pymodule]`
block. Spec §8. GitHub #29.
*#29 · recorded 2026-08-08 21:15 — James, building the Python config front end.*

## 2026-08-08 21:18 — James — Validation errors reach Python as attribute paths, guarded by scraping `config.rs`
**Chose:** `py_config::config_error_to_py` rewrites `ConfigError::Validation`'s field name into the
Python attribute path that produced it — `num_epidemics` becomes `config.fitness.sir.num_epidemics`
— leaving the constraint text untouched. A field with no Python equivalent keeps its original
wording rather than being given an invented path.
**Why:** spec §8 requires it in as many words: the Python front end reports against a TOML document
the user never wrote, so a bare field name leaves them to work out which of the objects they
assembled owns it. `Config::validate` is right to name the TOML field — that is the correct answer
for the file front end — so the rewrite belongs at the Python boundary, not in `config.rs`.
**The mapping is a hand-written match, so the real decision is how it is kept honest.**
`every_validation_field_maps_to_a_python_attribute` scrapes `config.rs`'s own `invalid("<field>",
...)` call sites out of `include_str!("config.rs")` and asserts each is mapped or explicitly exempt.
A second test guards the scraper itself, because one that silently matched nothing would make the
first pass while checking no fields at all. Verified by adding a `crossover_rate` check to
`config.rs` and confirming the suite failed naming it; then reverted.
**Rejected:** (a) A hand-maintained list of field names — the exact thing that goes stale, and the
failure is silent: an unmapped field degrades to a bare name rather than erroring. (b) Changing
`Config::validate` to emit Python paths — wrong for the TOML front end, which is the majority case
and has no Python attributes. (c) Leaving the bare names — cheapest, and the thing §8 names as
making errors useless.
**One exemption exists:** `seed`, raised by `reject_fitness_seed` against raw TOML text, is
unreachable from a front end that has no seed to write (spec §7). Exempt by name, so any *other*
unmapped field still fails.
**Affects:** `get/src/py_config.rs` `config_error_to_py` and `python_attribute_path`;
`get/src/lib.rs` `from_config`. Spec §8. GitHub #29.
*#29 · recorded 2026-08-08 21:18 — James, on the error-reporting half of #29.*

## 2026-08-09 — Michael & James — Skill frontmatter takes a PR; a skill's body does not
**Chose:** `.claude/skills/*/SKILL.md` splits across the routing table. Frontmatter — `model:`,
`allowed-tools:`, any hook-adjacent key — goes through a feature branch and a PR. The body is a
direct push. The governing test is written into `CLAUDE.md` as **"does this change what runs", not
"which directory is it in"**.
**Why:** rule 2 already sends `settings.json` and `hooks/` through review because they execute on
the other owner's machine at session start, without them reading the diff. Skill frontmatter has
exactly that property and was named nowhere — not in rule 2, not in the routing table — so a
`model:` change was permitted to land silently. The body does not have that property: it is prose
both owners read anyway, and putting a PR round-trip in front of a typo fix is how a rule stops
being followed at all.
**Stating it as a test rather than a directory list is the load-bearing part.** A table that names
directories goes stale the moment someone adds a fourth one; a principle routes the new case on its
own.
**Rejected:** (a) Extending rule 2 to all of `.claude/skills/` — would cover prose edits and buy
nothing. (b) Leaving it unwritten and relying on precedent — the thing that produced the situation
this settles. (c) Reverting the sonnet pin, offered by Michael and declined by James, who had run
`/load`, `/save` and `/done` under it without noticing a difference.
**Origin:** Michael pinned `done`, `load`, `save`, `setup` and `start` to `model: sonnet` in
`011480d` and pushed direct, logging it in `collab.md` #34 because the rule did not yet cover it.
James agreed the same day and drew the frontmatter/body line; both positions are stamped inside
that item. The amendment was written at the joint meeting of 2026-08-09.
**Affects:** `.claude/CLAUDE.md` routing table. `collab.md` #34.
*#34 · recorded 2026-08-09 — Michael & James, at the joint meeting.*

## 2026-08-09 — Michael & James — §6.2's "track the best" amended to best-of-final population
**Chose:** the reported best is the best of the **final** population, for both strategies. Spec
§6.2's "track the best" wording is amended to say so, and to say that at `elite_count = 0` the
divergence is a property of the configuration rather than of the report. **No code changes** —
`GenerationalEvolver::outcome` and `SteadyStateEvolver::outcome` already do this.
**Why:** the sentence read as a running best carried across generations, which neither evolver has
ever implemented. Two arguments against building one. Fitness is stochastic between batches since
the atomic batch counter landed (§8.1), so a running best is substantially a record of which
generation drew the luckiest sample — and §6.2 already rejects exactly that reasoning three
paragraphs later, where freezing an elite's old score is refused on the same grounds (§5.2). And a
non-elitist generational GA really can lose its best individual; the run ended without it, so a
report naming it describes a population that no longer exists.
**Rejected:** (a) Requiring `elite_count >= 1` in §7 — removes a legitimate configuration to avoid
amending a sentence, and non-elitist generational is deliberately run. James declined this
explicitly. (b) Implementing a running best — both owners rejected it, for the reason above.
**Arrived at independently from both ends**, which is the evidence the sentence rather than the
code was stale: James recorded best-of-final and the running-best he rejected in `decisions.md`
2026-08-06 21:04 while writing the generational evolver; Michael reached the same reading reviewing
PR #46 against §6.2 the following day, without having seen it.
**Affects:** `/official_spec_sheet.md` §6.2. No files under `get/src/`. `collab.md` #35.
*#35 · recorded 2026-08-09 — Michael & James, at the joint meeting.*
## 2026-08-09 — Michael & James — The self-merge exception widens to strict deletions of false text
**Chose:** a second permitted case for merging your own PR — **a strict deletion, or a one-line
correction, to a doc, where the change removes something already false.** The first case, "the other
owner is unavailable and the change is blocking", is unchanged. The trace obligation applies to
both: say it in the PR and in `collab.md`.
**The test is that the change subtracts a falsehood rather than asserting anything.** Dropping a
caveat that cites a closed issue, correcting a status row for a component that has shipped, fixing a
glob that names files it no longer covers. A sentence adding a new claim is not this case however
short it is, and that boundary is the whole reason the case can be safely widened — reviewing a
deletion of something false is a check nobody was ever going to fail.
**Why:** PR #37 was self-merged under case 1 when case 1 did not hold, and was logged honestly as a
self-merge of convenience (`collab.md` #29). A rule that gets correctly broken is stated wrong,
which is the same reasoning that reworded "an agent never merges a PR at all" on 2026-08-04. The
cost was measured on 2026-08-09: `official_spec_sheet.md`'s status table had been stale on **four of
nine rows** for days, each naming a shipped component as unbuilt, because correcting a fact needed
the full branch-and-review cycle.
**Rejected:** (a) Leaving the rule and treating #29 as a logged exception — precedent without a
rule is how the exception quietly becomes the norm. (b) Widening only for the spec status table —
too narrow to survive contact; the same argument applies to any doc, and a per-file carve-out
invites a second one.
**Affects:** `.claude/CLAUDE.md`, "Pull requests". `collab.md` #29.
*#29 · recorded 2026-08-09 — Michael & James, at the joint meeting.*

## 2026-08-09 — Michael & James — `Swap`'s degree floor stays at 3, one higher than the Java original
**Chose:** keep `graph.degree(v) <= 2` as the rejection test in
`get/src/genomes/edge_edit/operations.rs::swap` — both endpoints need **degree >= 3**. Spec §3.1's
"two non-adjacent vertices of degree > 2" already says this, so **no code and no sheet change**.
This entry exists only to stop the discrepancy being rediscovered and filed as an off-by-one.
**The discrepancy is real and was checked.** The 2019 Java predecessor (`Graph.java`/`GET.java`, in
Michael's OneDrive archive, not in this repo) rejects on `nbr.get(v1).size() < k` with its only
caller passing `MIN_DEG_SWAP = 2`, so the original required **degree >= 2**. Every other check in
the operation was ported verbatim — non-adjacent `v1,v2`, four distinct vertices, and none of
`v1-a2`, `v2-a1`, `a1-a2` already an edge — which is what made the single differing number look
like a slip rather than a choice. No comment in the Java explains why 2 was chosen.
**Why keep the stricter floor:** Michael's call at the meeting. `Swap` firing on a degree-2 vertex
strips a vertex to a single connection, and the stricter floor is what every run and every test in
this repo has been built and tuned against. Loosening it would change search behaviour on all of
them to match a number nobody can show was deliberate.
**Rejected:** (a) Loosening to `>= 2` to match the original — would need §3.1 reworded,
`operations.rs:169-170` changed, and new fixtures for
`swap_rejects_low_degree_and_conflicting_quartets`, all to adopt an unexplained constant.
(b) Parking it until the Java is readable by both owners — the current behaviour is not in doubt,
only its ancestry, and leaving the item open invites the same re-derivation later.
**Worth recording plainly:** the Java is **not verifiable from this repo** — `legacy/` holds only
`main.cpp`, `Graph.cpp/h` and `SDA.cpp/h`, none of which contains a swap operation. James agreed to
this on evidence only Michael can see. If the archive is ever added to the repo, this entry is what
a re-check should start from.
**Affects:** nothing. `get/src/genomes/edge_edit/operations.rs` and spec §3.1 both stand as written.
`collab.md` #27.
*#27 · recorded 2026-08-09 — Michael & James, at the joint meeting.*

## 2026-08-09 — Michael & James — Drop-in Rust objectives are supported, via the library, not the config
**Chose:** GET supports **two** user-extension routes for fitness. Python — register a callable
with `set_fitness_function` — for most users and any prototype. **Rust** — depend on `get` as a
crate, `impl Fitness` for your own type, and drive an evolver directly — for a hot native objective
without forking GET. Spec gains §5.3.
**The finding that made this cheap:** `Evolver::run<F: Fitness>` is generic over the objective and
`Fitness` is public, so a caller holding a concrete `F` instantiates the evolver and **never
touches §8's dispatch `match`**. The closed match turns a *config document* into concrete types; a
Rust user is a library consumer, not a config consumer. So supporting drop-in Rust objectives
required **no change to #26's design**, which was the thing `collab.md` #21 was raised to protect.
**A user objective deliberately gets no `FitnessConfig` variant.** The obvious alternative, a
string-keyed registry that config could name, would move validation out of serde — the exact
failure GitHub #13 and #23 exist to prevent. Keeping user objectives out of the schema means
nothing user-supplied is ever deserialized, so there is nothing new to validate.
**The one obligation it puts on #26:** dispatch must not become the only way to construct a run.
`Fitness`, `Direction`, the genome `Context` types, `SharedEvolutionContext`, each `TypeContext`,
`Evolver::new`, `Evolver::run` and `EvolutionOutcome` stay public. Narrowing any of them kills the
Rust route **silently** — there would be no compile error inside `get`. Noted on GitHub #26.
**Rejected:** (a) Declaring Python the only route and adding a §10 non-goal — James's lean when he
raised it, overtaken by the finding above, which removes the cost that made it attractive.
(b) A registry keyed by name in the config — see the validation argument. (c) Literally dropping a
`.rs` file into `get/src/` — that is a fork plus a match arm, which buys nothing over the library
route and costs a rebuild of GET itself.
**Open, deliberately:** the ergonomics of assembling a population and contexts by hand. A real cost
of the Rust route, possibly a builder later, and explicitly not a reason to route user objectives
through the config enum.
**Affects:** `/official_spec_sheet.md` §5.3 (new). GitHub #26. `collab.md` #21.
*#21 · recorded 2026-08-09 — Michael & James, at the joint meeting.*

## 2026-08-09 — Michael & James — The target profile is an inline config value, verbatim, with no C++ conventions
**Chose:** `epi_prof_match`'s target becomes **`target_profile`, an ordinary inline config value** —
a TOML array for the file front end, a Python list on `FitnessConfig.EpiProfMatch` for the Python
one. Both front ends hand over the same list of numbers and no setter is involved. It replaces
`target_profile_path`, which is deleted. Validated non-empty and finite by `Config::validate`.
**This reverses spec §8, which said the opposite** — "passed as a sequence of numbers through a
setter rather than serialized into the generated TOML ... a long inline array makes the provenance
document unreadable". Keeping it out of the document protected readability but cost the thing the
document exists for: a run whose target lived outside the config could not be reproduced from the
config alone, so `to_toml()`'s provenance was incomplete for exactly one objective. A verbose
`[fitness]` block is the smaller price.
**Neither C++ loading convention is reproduced.** `legacy/main.cpp:378-386` prepends patient zero,
so a stored `.dat` omits its own first element, and multiplies every value by `verts / 128` because
profiles were normalized to a 128-node network. Both dropped. The user supplies the profile they
want at the size of the network they are building, and GET compares against it unchanged. A silent
one-step shift and a silent rescale are two ways to produce a wrong number rather than an error,
and archived runs are usually not at 128 nodes anyway.
**Rejected:** (a) Keeping `target_profile_path` — a second file to lose, version separately and
omit from provenance. (b) Accepting either a path or an inline array — two ways to say one thing,
needing a new validation rule to reject both being set, and a provenance document whose shape
depends on which was used. (c) Reproducing the prepend and rescale for comparability with archived
C++ results — the argument that kept the short-epidemic re-roll (§5.2), rejected here because these
two conventions are invisible when wrong where the re-roll is a documented sampling policy.
**Note the sheet was the stale side and the code was too**, differently: §8 described a setter
nobody built, and `config.rs:129` stored a path §8 never authorized. Neither matched the other and
neither is what was agreed.
**Affects:** `/official_spec_sheet.md` §8, two passages. `get/src/config.rs`, `get/src/py_config.rs`,
`config.example.toml`, `examples/config_builder.py`. `collab.md` #24.
*#24 · recorded 2026-08-09 — Michael & James, at the joint meeting.*

## Task complete: pyconfig — 2026-08-09
PR #49 merged (`0731aa6`), issue #29 closed. All five planned tasks plus the examples file landed
and were verified on this machine across the task's sessions; see the archived
`work/archive/2026-08_pyconfig/` for the full plan and history. No open items remained at the
`/done` gate — both carry-forward issues (`sda.rs` doc-link warning; the
`evaluate_population`/`SirRun` rename, blocked on collab.md #32) and the one hotfix
(`python_fitness`'s `#[allow(dead_code)]`, blocked on #26) pre-date this task and are unaffected by
its close.
*Recorded 2026-08-09 23:46 — James, at the `/done pyconfig` gate.*

## 2026-08-10 17:48 — James — A stray `seed` on a Python-built config raises `AttributeError`, so collab #25's reply is wrong
**Measured, not reasoned.** `config.seed = 42` on a `get.Config` raises
`AttributeError: 'builtins.Config' object has no attribute 'seed'`. Run on this machine against a
`maturin develop` build of `de970ea` plus the working-tree changes for GitHub #53.
**What this corrects:** the reply inside `collab.md` #25 (2026-08-06 00:10, mine) says "a config
built in Python can still carry a stray `seed` attribute harmlessly", and offers that as a caveat
to be careful about in #26. It is false, and it is false in the *safe* direction: every `#[pyclass]`
in `py_config.rs` is declared without `dict`, so Python cannot set an attribute the class does not
declare. There is no silent carry to worry about. The 2026-08-09 meeting suspected this (meeting
note 3) but nobody had executed it; this is the execution.
**Why the original claim was plausible.** The mechanism it described is real and unchanged — the
`seed` check in `Config::from_toml_str` reads raw TOML text, and the Python front end has no text to
read, so the *check* genuinely does not run there. The error came from assuming that leaves the
Python side unguarded. A different guard covers it: pyo3's default attribute model, which is not
part of the config schema at all and so was not in view when the caveat was written.
**No code changes.** Both front ends reject a stray `seed`, by unrelated mechanisms, and the
narrowness recorded in `decisions.md` 2026-08-06 00:07 still holds for every *other* unknown
`[fitness]` key on the TOML side. What changes is one sentence of guidance for #26.
**Affects:** nothing in `get/src/`. Supersedes one claim in `collab.md` #25's reply; GitHub #26
should not plan around a Python-side stray-attribute hazard, because there is not one.
*#25 · recorded 2026-08-10 17:48 — James, while closing out GitHub #53's task list.*
## Task complete: epidemic-seeding — 2026-08-10
Issue #18 done and merged (PR #47, `fd0d920`) since 2026-08-07; this task's own record was only
archived now, on a session that did no new work — `/load` re-verified every claim in the prior
handoff against the repo (main clean at `d28dcc3`) and found nothing had moved. All planned tasks
plus the batch/reading renames and the `mix_seed` derivation landed; see
`work/archive/2026-08_epidemic-seeding/` for the full plan and history. No open items at the
`/done` gate — the one carry-forward hotfix (`python_fitness`'s `#[allow(dead_code)]`, James's,
blocked on Michael's #26) and the `sda.rs` doc-link warning pre-date this task and are unaffected
by its close.
*Recorded 2026-08-10 — Michael, at the `/done epidemic-seeding` gate.*

## 2026-08-10 22:14 — Michael & James — The sheet's scoring unit is a batch, and `SirRun` becomes `Epidemic`
**Chose:** `Fitness::evaluate_population` → `evaluate_batch`, and `SirRun` → `Epidemic`, in the
code and in `official_spec_sheet.md` (lines 225, 273, 372, 847, 857) in the same PR. Agreed at the
joint meeting of 2026-08-09, raised as `collab.md` #32 on 2026-08-07, filed as GitHub #52.
**Why:** the unit the engine scores together is a **batch whose size varies** — generational hands
over a whole population per cycle, steady-state hands over exactly two children per mating event
(`steady_state.rs:76`) plus its starting population once. "Population" was accurate in one of the
three cases and "generation" in none of the steady-state ones, while §5.1's prose already called it
a "batch scorer"; only the identifier disagreed. Separately, `sir_sim` returns **one epidemic**,
but "run" already meant a replicate (`run_seed`, §8.1) *and* the `GraphEvolver::run` API call —
three senses of one word, with `run_seed` sitting four lines from `|run| run.spread` in the same
impl block. No type named `Epidemic` existed, so the name was free.
**Rejected:** leaving the sheet and code to disagree until a later cleanup — the sheet is the
authority, so a rename that stops at the code makes the sheet wrong rather than stale. Also
rejected: renaming `sir.rs`'s test-local `run` bindings in a separate pass, since the helper rename
forced the loop bindings anyway and a half-swept file reads worse than either end state.
**Affects:** `get/src/fitness.rs`, `get/src/sir.rs`, `get/src/evolver/common.rs`,
`get/src/evolver/generational.rs`, `get/src/lib.rs`, `/official_spec_sheet.md` §3/§5.1/§8. Commit
`028440a` on `mdube_rename_evaluate_batch`.
*Recorded 2026-08-10 22:14 — Michael, stamped for both owners under the 2026-08-09 joint meeting
that agreed it.*

## 2026-08-10 22:15 — Michael — `express_and_score`'s `population` parameter becomes `batch`, outside the agreed scope
**Chose:** rename the parameter to `batch` and amend three further sheet lines (257, 274, 334),
committed separately as `8a8ed1b` so it can be dropped without disturbing the entry above.
**Why:** it is the same defect one layer up. `express_and_score` is the sole caller of the method
just renamed to `evaluate_batch`, and §5.1's invariant sentence had it mapping "a population" to
fitnesses four lines above a signature that `steady_state.rs:76` contradicts on every mating event
by passing two children. Renaming the trait method while leaving its only caller's parameter named
`population` fixes the identifier and leaves the misnomer sitting on top of it.
**Rejected:** renaming the code only and leaving the sheet — `CLAUDE.md` resolves a code/sheet
disagreement in the sheet's favour, so that tells the next reader the code is wrong. Also rejected:
deferring it to its own issue, which was the agent's recommendation; Michael judged the round-trip
not worth it for a parameter name already being renamed one call away.
**This entry deliberately carries ONE name, unlike the entry above.** The 2026-08-09 meeting
enumerated and verified **two** identifiers, so its authorisation does not stretch to a third, and
`CLAUDE.md` says the sheet changes only at a joint meeting — "not by one owner mid-task". This is a
departure, recorded as one rather than dressed as covered. It is not the self-merge rule's case 2
either: it *adds* a naming claim rather than subtracting a falsehood.
**Affects:** `get/src/evolver/common.rs`, `/official_spec_sheet.md` §5.1. `collab.md` #41 asks
James to acknowledge; the PR body repeats it.
*Recorded 2026-08-10 22:15 — Michael, pending James's acknowledgement in `collab.md` #41.*

## Task complete: rename-evaluate-batch — 2026-08-10
Issue #52 closed. PR #54 merged to `main` at `260f541` (2026-08-10T20:21:36Z), carrying both agreed
renames (`evaluate_population`→`evaluate_batch`, `SirRun`→`Epidemic`) and the third,
out-of-scope-but-isolated rename (`express_and_score`'s `population`→`batch`, `8a8ed1b`). Verified
on `main` post-merge: `grep -rn 'evaluate_population\|SirRun' get/src/ official_spec_sheet.md`
empty, 213 tests green. See `work/archive/2026-08_rename-evaluate-batch/` for the full plan and
history. Carried forward, not resolved: `collab.md` #41 still awaits James's acknowledgement of the
out-of-scope commit, and #40 awaits his acknowledgement of the `/done` push-behaviour change;
neither blocks this task's own close. `hotfixes.md`'s `python_fitness` suppression and the parked
`sda.rs` cargo-doc warning both pre-date this task and are unaffected by its close.
*Recorded 2026-08-10 — Michael, at the `/done rename-evaluate-batch` gate.*

## 2026-08-10 23:05 — Michael — `common::best_index` panics on an empty slice; #51 didn't ask for that
**Chose:** `best_index` (`get/src/evolver/common.rs:47`) opens with
`assert!(!fitnesses.is_empty(), "cannot pick a best of no individuals")`, and both callers carry a
comment stating why the slice can't in fact be empty at their call site.
**Why:** steady-state's old `.expect("population is non-empty, checked at construction")` was the
*only* thing guarding that case there. Generational's old loop had no such guard — an empty slice
would have silently returned `0`, then panicked one statement later on `self.population[0]` with a
message that says nothing about why. Moving both call sites onto one shared function meant picking
one behaviour; keeping the named panic was the smaller surprise, and it's the same reasoning
`generation_stats` already applies (`common.rs:289`, rejects an empty population).
**Rejected:** dropping the guard entirely to keep the extraction a pure move — decided against,
since it would have made generational's failure mode *worse* than before the refactor, not neutral.
**Affects:** `get/src/evolver/common.rs`, `get/src/evolver/generational.rs`,
`get/src/evolver/steady_state.rs`. Flagged in PR #55's body for review rather than assumed.
**Supersedes:** nothing — #51's own decisions.md entry described only the move, not this guard.
*Recorded 2026-08-10 23:05 — Michael, closing out `mdube_best_index`.*

## Task complete: best-index — 2026-08-10
Issue #51 closed. PR #55 merged by James (`9274f38`, 2026-08-10T20:52:32Z) — a real review merge,
not a self-merge. `common::best_index` replaces the two evolvers' separate argmin spellings;
verified on `main` post-merge: `cargo test -p get` 213/213, `cargo clippy -p get --all-targets --
-D warnings` clean. See `work/archive/2026-08_best-index/` for the full plan and history. One
follow-up filed rather than left loose: GitHub **#56** (sweep both evolvers for further divergence
and duplication), staged behind the currently open issue set, unassigned; raised as `collab.md`
**#43**. Carried forward, not resolved by this task: the `python_fitness` `#[allow(dead_code)]`
hotfix (blocked on #26), the parked `sda.rs` cargo-doc warning, and `collab.md` #40/#41 still
awaiting James's acknowledgement — all pre-date this task.
*Recorded 2026-08-10 — Michael, at the `/done best-index` gate.*

## Task complete: inline-target-profile — 2026-08-10
GitHub #53 closed via PR #57 (`f25e33d`, Michael's merge, reviewed on Windows). All nine planned
tasks landed and were verified on this machine across the task's two sessions: `cargo test -p get`
216/216, `cargo clippy -p get --all-targets -- -D warnings` clean, and `examples/config_builder.py`
run end to end against a `maturin develop` build. See `work/archive/2026-08_inline-target-profile/`
for the full plan and history.

One follow-up filed rather than left loose: GitHub **#58** (reject `target_profile` under a
non-`epi_prof_match` objective — spec §8's contradiction clause, same flatten mechanism as the
stray `seed` check in #25), assigned to James, raised as `collab.md` **#45**. Carried forward, not
resolved by this task: the `python_fitness` `#[allow(dead_code)]` hotfix (blocked on #26), the
parked `sda.rs` cargo-doc warning, `main`'s `cargo fmt -- --check` failure in `common.rs:45`
(pre-existing, tracked on GitHub #56, not this task's to fix), and `collab.md` #44's open
sub-question to Michael about where the practice-binding-skill-body rule gets written down — all
pre-date this task or fall outside it.
*Recorded 2026-08-10 — James, at the `/done inline-target-profile` gate.*

## 2026-08-11 11:26 — Michael — The dispatch layer is its own module, not `evolver/common.rs`
**Chose:** `get/src/dispatch.rs`, private to the crate, holding `objective`, `python_fitness`, the
two population builders and `sir_sample_params`. `lib.rs` keeps only the `#[pyclass]` surface and
falls from 1312 to 772 lines.
**Why:** this layer is the only place that knows both sides — it reads `crate::config` and returns
`PyResult`, so it depends on the config schema *and* on pyo3. The engine below it (`evolver/`,
`genomes/`, `sir`, `graph`) currently depends on neither, verified with
`rg -ln 'pyo3|crate::config'` over those paths returning nothing, and `sir.rs:32` states the
principle for its own params type. A dedicated module is what keeps that true while giving #26's
growing match a home. The spec never names a file for this — it says "the dispatch layer" at
§8 lines 552, 578 and 838 and nowhere gives a path — so this contradicts nothing and needed no
joint meeting; `config.rs`'s module doc had already named the layer.
**Rejected:** `evolver/common.rs`, which is the obvious candidate and the reason this entry exists.
It would invert the dependency above, dragging config and pyo3 into the engine core and costing the
ability to test the engine without a config or a Python interpreter. It is also the wrong *shape*:
`common.rs` is genome-agnostic generics over `G: Genome`, while these functions name
`EdgeEditGenome` and `SdaGenome` concretely. "Shared things go in common" is the trap here.
**Also chose:** the two population builders are free functions over `&Config` rather than methods,
so they are testable from a bare config; `objective` and `python_fitness` stay methods because they
read the registered callable too.
**Affects:** `get/src/dispatch.rs` (new), `get/src/lib.rs`. Commit `543b211` on
`mdube_run_dispatch`; pure move, test count unchanged at 226.
**Stale line left for #58:** `config.rs`'s module doc still says the mapping happens "in `lib.rs`".
True until this lands, false after. Not corrected here because James is editing that file for
GitHub #58 — flagged in `collab.md` instead of risking a conflict mid-flight.
*Recorded 2026-08-11 11:26 — Michael, mid-#26.*

## Task complete: run-dispatch — 2026-08-11
GitHub #26 closed via PR #60 (`97d9e02`, merged by James 2026-08-11T16:00:19Z — a real review merge,
not a self-merge). `GraphEvolver::run` is no longer `todo!()`: the objective erases to
`Box<dyn Fitness>` first, then genome × strategy dispatches through the new `get/src/dispatch.rs` in
2 + 2 arms rather than 4. Verified on `main` post-merge on this machine: `cargo test -p get`
231/231. All plan tasks landed; see `work/archive/2026-08_run-dispatch/` for the full plan and
history, and the 2026-08-11 11:26 entry above for why dispatch got its own module.

One hotfix **removed** rather than carried: #19's `#[allow(dead_code)]` on `python_fitness`, whose
`Remove when:` this task was the condition for — the method's caller now exists, and the attribute
is gone from `get/src/dispatch.rs:160`. One follow-up staged rather than left loose: `run`'s doc
comment at `get/src/lib.rs:219-232` still carries #19's "for whoever implements the dispatch (#26)"
instructions and cites the deleted attribute, staged in `issues.md` as ready-to-file, assigned to
Michael. Carried forward, not resolved by this task: `collab.md` #47's stale `config.rs` module-doc
line (James's to take in #58, still `open`), `collab.md` #48 (what `config.example.toml` should
demonstrate) and #49 (pip-installability as a v1 requirement) both parked for a joint meeting, the
`config.example.toml` flat-search finding in `issues.md`, and the parked `sda.rs` cargo-doc warning.
*Recorded 2026-08-11 — Michael, at the `/done run-dispatch` gate.*

## 2026-08-11 — Michael — Delete #61's stale doc block outright, don't rewrite it
**Chose:** delete `GraphEvolver::run`'s "For whoever implements the dispatch (#26)" doc block
(`get/src/lib.rs:219-246`, 29 lines) rather than rewriting it into present-tense documentation.
**Why:** issue #61 offered both, but checked first whether either instruction in the block said
anything not already stated elsewhere in the same method — it doesn't. `run`'s body already calls
`self.objective(seed)` (reaching `python_fitness`) and releases the GIL at `lib.rs:257-265` with
its own inline comment covering the identical argument, deadlock reference and all. Rewriting would
have produced a second copy of a comment the method already carries a few lines below.
**Rejected:** rewriting the block as present-tense documentation — the option #61 led with. Would
have been pure duplication, not documentation.
**Affects:** `get/src/lib.rs:219-246`. Closes GitHub #61. Branch `mdube_stale_run_doc`.
## 2026-08-11 20:15 — James — #58's contradiction check joins the `seed` sweep, and three smaller calls under it
**Chose:** one raw-text pass over both `[fitness]` keys serde's flatten hides — `reject_fitness_seed`
renamed `reject_stray_fitness_keys` (`get/src/config.rs:262`) — rather than a second function beside
it. Three calls sit under that:
1. A missing, misspelled or non-string `type` **falls through to the real parse** rather than being
   caught here.
2. The condition is an edition-2024 **let-chain**, not nested `if`s.
3. The three rejection cases share **one `for` loop** in one test, not three near-identical tests.
**Why:** the sweep shape was already agreed in `collab.md` #45 and needs no re-arguing. On (1), a
message about the profile would bury the real problem, and the function already hands invalid TOML
back to the real parse two branches above, so this is the existing behaviour rather than a new
policy. On (2), clippy's `collapsible_if` rejects the nested form under `-D warnings`, and every
flat alternative needed an `unwrap_or_default` to get the objective's name into the error message —
the let-chain is the only form that keeps the name and passes the gate, so it is documented with a
four-line plain-words comment above it for the owner who does not write Rust. On (3), GitHub #56 is
an open sweep for exactly the duplication three copies would add.
**Rejected:** catching a bad `type` here (loses the better error); a helper function taking the
parsed `fitness` table (flat and clippy-clean, but reintroduces the second function the issue warned
against); three separate rejection tests (names the failing objective slightly better, at the cost
of the duplication #56 exists to remove — the loop's assert message names it anyway).
**Affects:** `get/src/config.rs:262-330`, its two tests at `config.rs:840` and `config.rs:872`, and
the renamed narrowness test `an_unknown_fitness_key_outside_the_two_named_ones_is_still_ignored`.
Commits `bfa515b`, `d7cb289`, `7fc4c1a` on `jsargant_reject_stray_target_profile`; PR #63.
**Not decided here:** whether `config.example.toml` should demonstrate a converging run — that is
`collab.md` #48, untouched.
*Recorded 2026-08-11 20:15 — James, at #58's save.*
## Task complete: stale-run-doc — 2026-08-12
GitHub #61 closed via PR #62 (`ffb0c9b`, merged 2026-08-11T17:38:31Z — a real review merge, not a
self-merge). `GraphEvolver::run`'s stale "for whoever implements the dispatch (#26)" doc block
(`get/src/lib.rs:219-246`, 29 lines) is deleted; nothing else in the method changed. Verified on
`main`: `cargo test -p get` 231/231, clippy and fmt clean. No hotfixes or issues touched by this
task; nothing carried forward.
*Recorded 2026-08-12 — Michael, at the `/done mdube_stale_run_doc` gate.*

## 2026-08-12 15:30 — Michael — Branch cleanup is ours to do, in two places, for two different moments

Enabled `delete_branch_on_merge` on `md12ol/GraphEvolutionTool` after merging PR #63, then found it
covers almost none of the merges this repo performs. **The setting fires only when GitHub itself
executes the merge.** Our rule sends every PR touching `.claude/work/*.md` through a local
`git merge --no-ff` and a push, because `.gitattributes` merge drivers run in your git and not on
their servers. GitHub sees those commits arrive on `main` and flips the PR's state to merged, but no
merge action ran, so no cleanup runs either. The setting stays on — it is free and it covers the
button-merge case — but it is not the mechanism we rely on.

**So the deletion is written into two places, and that is deliberate rather than drift.**
`CLAUDE.md`'s merge snippet covers merging *the other owner's* PR; `/done`'s new step 7 covers
closing *your own* task. Neither path reaches the other, and both are needed because nobody merges
their own PR here. The mechanism is written once, in `traps.md`
(`auto-delete-does-not-fire-on-a-locally-merged-pr`); both places link to it rather than restating
it, so there is one copy to keep true.

**Which copy of the branch each place is actually deleting is the non-obvious part.** By the time
you close your own task, the other owner has usually merged your PR and deleted the remote branch
with it. What survives is your **local** branch, on your machine only — no one else can delete it
and no setting will. That is the whole argument for step 7 existing separately from the merge
snippet. Step 7 therefore runs `git fetch --prune`, deletes the local copy, and treats the remote
delete as a tolerated no-op: `remote ref does not exist` is the expected outcome there.

**Step 7 also fixed something that had only ever been implied.** `/done`'s Constraints said since
2026-08-10 that the close-out belongs on `main`, but no step told the skill to *switch* there — and
`/done` runs at the end of a task, when the task's code branch is the checked-out one. The archive
and the `decisions.md` marker would have landed on the feature branch and waited on someone's
review, which is precisely the stall the two-independent-tracks rule exists to prevent. It now
checks out `main` and pulls first, and stages `.claude/` only so the task's source changes cannot
be swept in.

**The deletion is gated twice, because `/done` may legitimately run before the PR is merged**
(`collab.md` #28, settled 2026-08-06 doing exactly that for issue #22 while PR #43 was open).
Deleting an open PR's branch closes it unmerged, which is destructive and unrequested, so step 7
checks `git branch --merged main` and `git branch -d` refuses an unmerged branch on its own. Not
merged → both copies are left alone and the report names the branch as waiting.

Commits: `0a95d44` (trap + `CLAUDE.md`), `8753156` (step 7), `083ed6a` (ordering correction).
Raised to James as `collab.md` #48, which can compress once this entry is on `main`.
*Recorded 2026-08-12 15:30 — Michael, after merging PR #63.*

## 2026-08-12 18:52 — Michael — The documentation site describes the sheet, not the code, and badges the gap

**Chose:** `documentation/` documents GET **as `official_spec_sheet.md` designs it**. Features that
are designed and agreed but not yet in `get/src/` are written in the present tense, as though they
work, and carry two markers: a `planned` badge where they appear, and a `.plan-note` callout naming
what happens today. `status.html` indexes every one in a single table — replicates and `max_cores`,
the result object, the convergence log reaching Python, `ci_95` and the per-row seed,
`save_logs`/`save_results`, and the base-graph setter.

**Why:** it follows this repo's existing rule — where the sheet and the code disagree, the sheet is
the intent — so the site does not need rewriting each time a designed feature lands, only
de-badging. It was also an explicit instruction when the work was commissioned.

**Rejected:** documenting only what exists today. That produces a site that is correct on the day it
is written and silently wrong afterwards, and it would have omitted the replicate layer, which is
how research actually uses GET. Also rejected: writing the future tense throughout ("will support"),
which reads as a roadmap rather than documentation and ages just as badly.

**The risk, stated plainly:** "written as though implemented" is one bad edit away from
"indistinguishable from implemented". The badge, the callout and `status.html` are the three guards,
and all three must survive any future edit. Anyone removing a badge without shipping the feature has
broken the contract.

**The opposite case exists and is not badged.** Several places where the sheet's status claims are
stale and the code is *ahead* are documented from the code, with no badge; `status.html` records
them separately. Check which way a disagreement runs before badging anything.

**Affects:** `documentation/` — every `badge-planned`, `documentation/status.html`,
`documentation/README.md`, `documentation/HANDOFF.md`.

**Open:** raised to James as `collab.md` #50. He merged PR #64 without ruling on it, so the
convention stands unopposed rather than agreed.
*Recorded 2026-08-12 18:52 — Michael, after PR #64 merged as `d420b3e`.*

## 2026-08-12 18:54 — Michael — No build step for the docs site, and one NAV table instead of a generator

**Chose:** hand-written static HTML with a single stylesheet and a single script. `assets/site.js`
holds the site map in one `NAV` array and builds the sidebar, the on-page contents, the prev/next
pager, the copy buttons and the theme toggle at load time; a page file contains only its `<main>`
plus a `data-page` attribute tying it to its `NAV` entry. Dependencies are a browser, and Python 3
only if you want a server.

**Why:** the brief was that a clone can launch it locally. Anything with a build step turns that
into "install the toolchain first", and a documentation generator that reads the codebase would put
the guide pages — which come from the sheet, not the code — outside its reach. Doing the chrome in
JS keeps 38 page files free of duplicated navigation, so adding a page is one `NAV` line rather than
38 edits.

**Rejected:** `cargo doc` (documents the code, cannot express the design, and says nothing about
config or the Python front end); mdBook or a static-site generator (a toolchain to install, and a
second source of truth for the nav); server-side includes or `fetch`-ing a nav fragment (breaks
`file://`, which is the cheapest way to read the site).

**The cost, accepted:** JavaScript is required for navigation between pages — each page still reads
fine without it, but the sidebar and pager vanish. And `data-page` is silently load-bearing: get it
wrong and the page renders while every sidebar link on it 404s. The verification script in
`documentation/README.md` exists to catch exactly that, along with broken links and anchors.

**Also decided:** `documentation/HANDOFF.md` is checked in, because `.claude/work/current/` is
gitignored and so the task's own state never reaches the other owner.

**Affects:** `documentation/assets/site.js`, `documentation/assets/style.css`,
`documentation/_template.html`, `documentation/serve.sh`.
*Recorded 2026-08-12 18:54 — Michael, alongside the sheet-versus-code entry above.*
## Task complete: reject-stray-target-profile — 2026-08-12
**Chose:** GitHub #58 done. `reject_stray_fitness_keys` (`get/src/config.rs:262`) rejects a
`target_profile` supplied under any objective but `epi_prof_match`, completing spec §8's
contradiction clause alongside the existing stray-`seed` check. 233 tests, clippy and fmt clean.
**Affects:** `get/src/config.rs`, `get/src/py_config.rs`, `config.example.toml`. PR #63, merged by
Michael as `b225f30` on 2026-08-12.
**Note on this gate:** the `#[allow(dead_code)]` hotfix on `python_fitness` this task expected to
find here had already been removed by Michael at the `/done run-dispatch` gate (2026-08-11, #26) —
checked and confirmed rather than assumed.
*Recorded 2026-08-12 — James, at the `/done` gate.*

## Task complete: initial-doc-site — 2026-08-13
Archived to `.claude/work/archive/2026-08_initial-doc-site/`. The documentation site shipped in one
session and merged as PR #64 (`d420b3e`). Entries below this line belong to a later task.
*Marker written 2026-08-13 00:47 — Michael.*

## 2026-08-13 01:47 — Michael — `run` returns a result object; the result types get their own module

**Chose:** `GraphEvolver.run` returns a `RunResult` carrying `best_fitness` (objective units),
`best_edges`, `best_genome_repr` and `history`; the `best_fitness` field and `best_fitness()`
accessor are deleted. The Python-facing types live in a new `get/src/py_result.rs`, exposed as
`RunResult` and `GenerationStats`, both `frozen` with read-only getters.

**Why the separate module rather than `lib.rs`.** It mirrors `py_config.rs`, and the reason points
the same way: `EvolutionOutcome` cannot carry `#[pyclass]` because it is generic over the genome
(§8), and `GenerationStats` must stay engine-oriented because the engine compares in oriented values
throughout (§5.1). Attaching `#[pyclass]` to either would drag pyo3 into the engine core and put a
Python-visible type on numbers deliberately not in the user's units. `lib.rs` stays the entry point.

**Why frozen and read-only.** The object records something that already happened, so there is
nothing a caller could correctly change. `history` rebuilds its list on each access, which is stated
on the getter so nobody reads it inside a loop.

**The part that could have failed silently:** `erase` now converts the log row by row, and converts
**only** `best_fitness` and `mean_fitness`. `std_dev` is left exactly as computed, because a spread
is identical under negation (§6.4) — orienting it too would keep it positive, so nothing would look
wrong while the log quietly disagreed with its own columns.

**Rejected:** reusing `EvolutionOutcome` at the boundary (generic, cannot be a `#[pyclass]`);
returning tuples or dicts (no names, no `repr`, and `history` rows would be positional); keeping
`best_fitness()` alongside the returned object (a second way to read state the caller already holds,
and it has to answer *something* before any run — infinity, which a maximizing objective converts
back into a suspiciously excellent score).

**Affects:** `get/src/py_result.rs`, `get/src/lib.rs:35` and `:235`, `get/src/dispatch.rs:57` and
`:411`. GitHub #27, PR #65. Unblocks #20 and #28.
*Recorded 2026-08-13 01:47 — Michael, with PR #65 open and unmerged.*

## 2026-08-13 01:49 — Michael — The reported best fitness is the final population's, not best-ever

**Chose:** leave the difference in place and document it rather than "fixing" it. `EvolutionOutcome`
reports the best individual of the **final** population, scored in that final pass. The convergence
log's best row can therefore be *better* than the reported `best_fitness`.

**Why this is correct rather than a bug.** Both numbers come from the same final scoring pass
(`generational.rs:208`), so the **last** log row always equals the headline number exactly. Earlier
rows can beat it because a SIR objective is stochastic and re-samples: an elite carried forward is
re-scored every generation, and spec §6.2 says that is deliberate — keeping the old number would let
a lucky draw persist as a permanent high score.

**Why it is written down.** It looks like an inconsistency and invites a "fix" that would either
cache a lucky draw or make the log disagree with the result. It cost real time here: the first
version of `the_erased_history_comes_out_in_the_objectives_own_units` asserted the log's *best* row
equalled `best_fitness` and failed at 3.0 against 2.0. The assertion was wrong, not the code.

**Rejected:** reporting best-ever (re-introduces the lucky-draw problem §6.2 rules out); re-scoring
the winner once more at the end (a third number, no more authoritative than the other two).

**Affects:** `get/src/dispatch.rs:411`, the test at `dispatch.rs` that pins the final row. Flagged
for GitHub #21, which documents the log columns a user reads when the two numbers disagree.
*Recorded 2026-08-13 01:49 — Michael, found while writing #27's orientation test.*

## 2026-08-13 01:51 — Michael — Documentation edits are staged in a per-owner queue, not applied per task

**Chose:** a task that invalidates something `documentation/` says no longer edits the HTML. It
appends an entry to a per-owner queue — `documentation/mdube_edits.md` or
`documentation/jsargant_edits.md` — naming what is now false and what it should say, and the site is
corrected in one sweep as its own task. Routing is by `git config user.email`, checked rather than
assumed, with anything unrecognised required to stop and ask.

**Why:** shipping #27, a three-hour issue about a return type, touched ten HTML files. Worse, the
first pass over them was not enough — a second pass found two pages nobody had edited still claiming
the convergence log never reaches Python, and about forty-five `src` line references invalidated by
the task's own edits. That is a whole task's worth of care hiding inside every code task, and it is
exactly the care that gets skipped under time pressure.

**Why per-owner rather than one shared file:** a queue is a **churn list** — an entry is deleted once
applied — which is precisely what `merge=union` cannot express (see 2026-08-04 18:25). Two files mean
neither owner ever touches the other's. The cost, written into both files: a sweep reads every queue
file, not only its own, or a page owed edits by both owners gets half-corrected.

**Rejected:** one shared queue under `merge=union` (a delete racing an edit silently resurrects the
entry); a queue under `.claude/work/` (it is about the site, and `documentation/` is where a sweep
starts); keeping the per-PR rule (measured above).

**Supersedes, in timing only:** the `CLAUDE.md` bullet of 2026-08-13, "When a `planned` feature ships,
de-badge its documentation in the same PR". The de-badging still happens — badge, `.plan-note`,
`status.html` row — but in the sweep. **The bullet is not yet amended**, because it binds James;
`collab.md` #53 asks him.

**Affects:** `documentation/mdube_edits.md`, `documentation/jsargant_edits.md`, `.claude/CLAUDE.md`
once #53 settles. #27's own docs were applied under the old rule, in PR #65.
*Recorded 2026-08-13 01:51 — Michael, on his own instruction mid-task.*

## 2026-08-13 02:16 — Michael — Shipped source does not reference the spec sheet, at all

**Chose:** strike the "link `official_spec_sheet.md` rather than restating it" half of the comment
convention. `get/src` references the sheet **not at all** — not by section number, not by name, not
as a link. Where a comment needs the reason something is correct, it states the reason rather than
citing where it was agreed. The rest of the 2026-08-04 convention stands: terse, written for someone
new to the code, never a copy of the sheet.

**Why:** the original clause was aimed at stopping *copies* of the sheet drifting out of step, and
linking was the cheap alternative to copying. It was a reasonable rule that produced an unreasonable
result. Measured 2026-08-13 on `main`: **135** sheet references in 10,251 lines of `get/src`,
alongside 19 GitHub issue numbers, 13 dated "agreed on `<date>`" notes and 8 pointers at `.claude/`
working docs. A reader of the published crate can open none of them, so each is a dead end rather
than a shortcut — the opposite of what the clause was for.

**The misreading worth recording.** "Keep comments terse … link the sheet rather than restating it"
plainly asks for brevity, and it was read as licence to cite the sheet freely. Both halves were
followed in letter and the result contradicts the first half. That is a rule stated wrong rather than
a rule broken — the same conclusion the agent-merge and self-merge rewordings reached.

**Rejected:** keeping the link clause and merely capping how often it is used (no threshold anyone
could apply while writing); moving the citations to a mapping file (a second document that also does
not ship); leaving it until the cleanup issues run (the convention would keep generating the debt the
issues exist to pay off).

**Affects:** `.claude/CLAUDE.md`, the "Prefer explicit loops" bullet — struck through and dated
rather than overwritten. Cleanup is GitHub #68 for `get/src` and #67 for `documentation/`, both
tier (8), behind #20, #21, #28 and #56. FYI to James in `collab.md` #54.
**Supersedes, in part:** 2026-08-04 22:12 — the comment convention. Only the linking clause.
*Recorded 2026-08-13 02:16 — Michael, pushed direct to `main` under collab #52(a)'s practice.*

## 2026-08-13 02:45 — Michael — `dispatch.rs` confirmed to fail the new comment rule; fix stays with #68

**Chose:** measured `dispatch.rs` against the 2026-08-13 comment amendment rather than assume it
passes because it predates the rule. It does not pass: the non-test region is **214 comment lines
against 210 of code** — more comment than code — with small functions worst affected (`selection`,
4 doc lines for a 6-line pass-through; `erase`, 9 for 5). Decided to leave the fix to GitHub #68
rather than pull it into PR #65 or cut it standalone.

**Why leaving it to #68 rather than fixing now:** offered as an option and declined. The case for
fixing now was that James is about to read this exact file for review; the case against, which
stood, is that #68 already owns the whole-crate pass and a one-file cut ahead of it risks a second,
inconsistent pass later.

**Affects:** `get/src/dispatch.rs` (unchanged). GitHub #68's body should carry the sharper
non-test figure in place of the diluted 29% currently cited — queued in `plan.md`, not yet pushed
to the tracker.
*Recorded 2026-08-13 02:45 — Michael.*

## 2026-08-13 03:05 — Michael — Live task directories go per-owner and tracked, and a blocked task is parked rather than held

**Chose:** `.claude/work/current/` is replaced by `.claude/work/<owner>/current/`, with
`.claude/work/<owner>/parked/<slug>/` beside it for tasks that cannot proceed. Both are **tracked**;
the `.gitignore` line that hid the live task is gone. `work/archive/` is unchanged — shared, no
owner in the path. `<owner>` is resolved from `git config user.email` against the table
`documentation/mdube_edits.md` already uses, and an unrecognised address stops and asks. A new
`/park <slug>` skill saves, stamps `handoff.md` with `Blocked on:`, and moves the task; `/load
<slug>` brings it back, parking whatever was live to make room; `/done` refuses a parked task.

**Why:** two problems with one shape. A task blocked on the other owner — #27, waiting on PR #65 and
#66 — either squatted in `current/` or had to be falsely closed. And an ignored `current/` lived on
one laptop, so a plan could not be resumed on another machine, which is the identical failure that
made `work/archive/` tracked on 2026-07-31. The original reason for ignoring it was that two people
must not fight over one live plan; the per-owner **path** serves that reason strictly better, since
neither owner's directory is ever written by the other. Once the path does the work, the ignore only
costs.

**Rejected:** (a) `work/current_<owner>/` and `work/parked_<owner>/` — flatter, but namespaces two
names instead of one and needs a second `.gitattributes` glob per owner. (b) Keeping `current/`
shared and ignored with only `parked/` namespaced — the cross-machine half then still does not work,
which was the actual driver. (c) A per-owner `archive/` — symmetric, but splits the project's
history in two for no gain. (d) Reading a parked task in place rather than moving it — every
downstream tool reads `current/` and nothing else, so two directories holding a live plan is exactly
the state this prevents.

**The new failure mode, accepted deliberately:** tracking live tasks makes the conflict *one person
against themselves from two machines*, not one owner against the other. `plan.md` is rewritten in
place, so no merge strategy helps and union merge would be actively wrong. Mitigation is a
`**Machine:** <hostname> · saved <ts> · <SHA>` stamp in `handoff.md` and a `/load` that **stops and
reports** on divergence instead of merging or resetting. Note `pull_main.sh` fast-forwards `main` at
session start but refuses on a dirty tree, so a divergence usually means it declined.

**Verified rather than assumed:** `git check-attr merge` reports `unspecified` for both
`work/mdube/current/plan.md` and a `parked/<slug>/` file, so the union driver does not reach the new
paths — gitattributes globs do not cross `/`. Park→unpark round trip is lossless, md5-identical
across all five files, tested against a `backup_docs.sh --force` snapshot.

**Affects:** `/.gitignore`, `.claude/CLAUDE.md`, `.claude/README.md`, `.claude/hooks/session_brief.sh`,
`.claude/skills/{park,load,save,start,done,setup}/SKILL.md`. PR #69; FYI to James in `collab.md` #55.
*Recorded 2026-08-13 03:05 — Michael, with PR #69 open and unmerged.*

## 2026-08-13 03:07 — Michael — `/save` and `/park` push `work/<owner>/` themselves; a narrow carve-out to the push rule

**Chose:** `/save` and `/park` commit and push `.claude/work/<owner>/` as their last step, without
asking. That path, at that step, and nothing else. `CLAUDE.md`'s "don't commit or push unless asked"
keeps its full force everywhere else, and a `/save` that finds uncommitted source leaves it alone
and says so in the brief.

**Why:** the directories were tracked so a task could be resumed on another machine, and that only
works if the save reaches `origin`. Leaving the push to a separate human step reintroduces the
failure the change was made to remove, in its worst form: it fails **silently**, and you find out on
the other laptop, usually a day late. A rule whose correct application depends on remembering an
extra step every single session is a rule that will be missed.

**Rejected:** (a) `/save` stages and asks for an OK — one prompt per session end, and the prompt is
skippable exactly when tired, which is when saves matter. (b) The user pushes manually — same silent
failure, now with the docs claiming they were saved. (c) Widening the rule generally — the rule is
load-bearing for code, where an unreviewed push is how a wrong number reaches `main`.

**Cost, stated plainly:** nobody reviews your own plan file. That is correct — a plan carries no
behaviour — but it is the part of this most likely to be objected to, so it is the part `collab.md`
#55 puts to James explicitly rather than burying.

**Affects:** `.claude/CLAUDE.md` Conventions and the routing table; `.claude/skills/save/SKILL.md`
step 10; `.claude/skills/park/SKILL.md` step 5.
*Recorded 2026-08-13 03:07 — Michael, amending the 2026-08-04 15:55 routing decision rather than superseding it.*

## 2026-08-13 03:09 — Michael — A change that alters how the other owner's session behaves goes through one PR, whatever the routing table permits

**Chose:** all of the per-owner work-directory change went through PR #69 — including the skill
bodies, `CLAUDE.md`, `README.md` and `.gitignore`, every one of which the routing table would have
allowed pushing straight to `main`. Only `collab.md` #55 went direct, ahead of the PR.

**Why:** the routing table's own stated test is **"does this change what runs"**, and a skill body
is what an agent executes. These particular bodies instruct it to move task directories between
paths and to push commits — categorically unlike the typo fix the direct-push row was written for.
Second and more concrete: splitting the change leaves a broken intermediate state on `main`, where
skills write to `work/jsargant/current/` while the session brief still reads `work/current/`. The
change is atomic in fact, so routing it as two things manufactures a window in which neither owner
has a working setup.

**Why `collab.md` #55 is the exception:** it is the notification. A notification sitting on a branch
notifies nobody — the same argument that puts traps on `main` before the merge they exist to
prevent. It went first so James can object before he is reviewing a fait accompli.

**Rejected:** following the table literally and splitting the push. Permitted, and wrong here — the
table routes by what a change *is*, and this is the case where the same change is both.

**Affects:** PR #69; `.claude/CLAUDE.md`'s routing table, which is unchanged — this entry records
when to route *above* it, not a new row.
*Recorded 2026-08-13 03:09 — Michael, on the user's question about where each half belonged.*

## 2026-08-13 02:41 — Michael — #21 is built on a stacked branch, not on `main`

**Chose:** branched `mdube_run_output` off `mdube_result_object` (PR #65, open) and merged
`mdube_per_owner_work_dirs` (PR #69, open) into it, rather than branching GitHub #21 off `main`.
The merge was clean — the two PRs' file sets are disjoint, `.claude/` + `.gitignore` against
`get/src` + `documentation/` + `examples/`.

**Why:** #21's scope changed underneath it when #27 landed the result object. `save_logs` and
`save_results` now have to be re-homed onto `RunResult`, and `RunResult` exists only on #65's
branch. Branching off `main` would mean writing #21 against the deleted API and then writing it
again. #69 comes along because its skills and hook are the workflow this session actually runs
under, and having them absent from the tree makes every `/save` and `/load` behave as the old
layout.

**Rejected:** waiting for #65 to merge — it is blocked on James, with no date, and #21 is the only
unblocked issue assigned to Michael. Also rejected cherry-picking just `py_result.rs` out of #65,
which would fork a file that is under review.

**Affects:** branch `mdube_run_output`, `db5d863`. A PR opened from it before #65 and #69 merge
shows their commits too; the PR body has to say so. Once both land, merge `main` in and the stack
flattens.
*Recorded 2026-08-13 02:41 — Michael, at the `/start` of #21.*
## 2026-08-13 — Michael — closed `result-object` (#27) with `collab.md` #53/#54 still unanswered

**Chose:** to run `/done` on the `result-object` task even though its last plan item's own
`Verify by:` named two conditions — PR #65/#66 merged, and `collab.md` #53/#54 answered — and only
the first had happened. The PR-merge half is done (confirmed via `gh pr list --state all`); #53 and
#54 carry forward as open `collab.md` items rather than blocking this task's close.

**Why:** #53 (per-owner doc-queue convention) and #54 (sheet-linking amendment, an FYI already
pushed direct) are both about future process, not about whether GitHub #27's shipped code is
correct. The task's own `Verify by:` bundled a real gate (the PR merges) with a courtesy check-in
that doesn't actually gate this task's deliverable. Holding a finished, merged feature open
indefinitely on an unrelated unanswered reply is the kind of program-not-task drift `CLAUDE.md`
warns `/done` exists to prevent.

**Rejected:** waiting for James's reply before closing — the safer literal reading of the plan's
`Verify by:`, but with no forcing function (no deadline, no dependency of #53/#54 on this task's
code), it would leave the task open indefinitely for no gain.

**Affects:** `.claude/work/mdube/current/plan.md` (now archived) — the "Waiting on James" item
marked `[x]` with the split noted. `collab.md` #53/#54 remain **Open**, unaffected by this task's
close.
*Recorded 2026-08-13 11:20 — Michael.*

## Task complete: result-object — 2026-08-13

## 2026-08-13 — Michael — closed `per-owner-work-dirs` with `collab.md` #55 still unanswered

**Chose:** to run `/done` on `per-owner-work-dirs` with PR #69 merged but `collab.md` #55 (the
`/save`/`/park` push carve-out and the rest of the layout write-up) still unreplied. Same reasoning
as the `result-object` close earlier the same day: the PR-merge half of the "Waiting on James" gate
is satisfied, and #55 is a courtesy notification rather than something #69's shipped code depends
on.

**Why:** the layout, hook fixes and skills are already live and working on this machine — this
session used `/park` and `/load` as real skills against `run-output` and `result-object` and both
round-tripped correctly, which is direct evidence the shipped design works independent of whether
James has replied.

**Affects:** `.claude/work/mdube/current/plan.md` (now archived) — all three remaining items closed:
the PR-merge half of "Waiting on James", the `/park`/`/load` exercise (satisfied by this session's
own real skill runs), and the hook/skill-frontmatter PR (already inside #69). `collab.md` #55
remains **Open**, unaffected.
*Recorded 2026-08-13 11:35 — Michael.*

## Task complete: per-owner-work-dirs — 2026-08-13

## 2026-08-13 — Michael — `.claude/work/<owner>/` moved into a dedicated `main` worktree (`collab.md` #58, PR #70)

**Chose:** a linked git worktree, permanently checked out to `main` at the fixed sibling path
`../<repo-name>-docs`, as the sole place every skill (`save`, `park`, `load`, `done`, `start`)
reads and writes `.claude/work/<owner>/` and the persistent docs. `session_brief.sh` reads
`git show main:.claude/work/...` instead, needing no worktree setup since it was read-only anyway.

**Why:** `.claude/work/<owner>/current/` and `parked/` are ordinary tracked files, so a feature
branch's copy is frozen at the moment it was cut. `main` moving afterwards — including a task
closing and archiving out of `parked/` — was invisible from that branch until merge. Hit directly
this session: `mdube_run_output` was cut mid-task, then `result-object` and `per-owner-work-dirs`
both closed and archived on `main`, and switching back to `mdube_run_output` showed both still
listed as parked. The routing table already said these directories go "direct push to `main`" —
the actual bug was `/save`'s own push step saying "`main`, or the current branch, if this session
is on one," which is what let the drift happen.

**Rejected:** always `git checkout main` in the same working tree before touching `.claude/work/`,
then checking back out to the code branch afterward (the "checkout dance") — this session did
exactly that by hand for the `result-object` and `per-owner-work-dirs` closes, and it produced a
real bug: a `git stash pop` after the dance restored stale pre-edit content over freshly-written
files, silently, because the stash captured the git index's stale staged content rather than the
edited working-tree content. A worktree has no equivalent failure mode — there's no stash, no
checkout, no shared index to race with itself. Also rejected: a fully separate `.claude/`-only repo
(the user's original suggestion) — decoupling the docs from the code's commit history the sheet
and `CLAUDE.md` rely on ("where the sheet and the code disagree, fix the code, or write a dated
entry") loses more than the branch-drift bug costs.

**Consequence for union merge:** `decisions.md` and `collab.md` stop being touched by feature
branches under this model, since every write goes straight to the worktree's `main`. A
feature-branch PR should now carry no diff to either file, so `merge=union` stops mattering for PR
merges specifically — it still matters for two people running `/save` on `main` around the same
real time, which this change doesn't touch.

**Migration performed this session:** `mdube_run_output`'s stale `.claude/work/mdube/` (frozen at
branch creation, still carrying `result-object` and `per-owner-work-dirs` as parked even though
both had since archived on `main`) was removed from that branch entirely (`git rm -r`), and its one
live task (`run-output`) was copied into the new `main` worktree by hand, since the branch's copy
was staged-but-uncommitted and had no history worth carrying across. Merging `main` into
`mdube_run_output` afterward produced four `rename/delete` conflicts, all auto-resolving to
`main`'s content (`git add` on files git had already written correctly) — confirms the branch
cleanup was sufficient for a clean merge.

**Affects:** `.claude/hooks/session_brief.sh`, `.claude/skills/{save,park,load,done,start}/SKILL.md`,
`.claude/CLAUDE.md`, `.claude/README.md` — PR #70, open. `collab.md` #58 is the notification to
James, including his one-time setup step.
*Recorded 2026-08-13 — Michael.*

## 2026-08-13 14:52 — Michael — Joint meetings get their own directory, and deciding is split from executing

**Decided:** `.claude/work/meetings/<YYYY-MM-DD>.md` — one file per joint meeting, no owner in the
path, direct push to `main`, **not** union-merged. Three skills operate on it: `/makeAgenda` derives
it from `collab.md`, `/startMeeting` walks it and records decisions into it, `/endMeeting` executes
the resulting checklist. Each file is an agenda before the meeting and the minutes of it afterwards,
carrying a `Status:` of `prepared` → `in progress` → `closed` → `executed`.

**Why a directory rather than more `collab.md`.** `collab.md` reached 2476 lines and its own
`Open`/`Settled` headings stopped being accurate around item #48 (`collab.md` #59). Turning it into
an agenda was manual every time, and the answers were landing in a session transcript rather than
back inside the items — the failure `collab.md` #52(b) names and #50 is the live instance of, since
PR #64 merged without ruling on the question its item asks. A derived agenda fixes the reading
problem without touching the source: `/makeAgenda` never edits `collab.md`, so reorganising that
file stays a joint decision rather than something a skill does on the way past.

**Why the meetings directory is not union-merged.** Union merge exists for the case where two people
append to the same file concurrently. One file per date makes that case impossible, and union's
silent failures — byte-identical lines deduplicating, an entry spliced mid-line — are pure cost with
no matching benefit. Same reasoning as the 2026-08-04 narrowing that took `traps.md`, `issues.md`
and `hotfixes.md` off the driver.

**Why deciding and executing are separate sittings.** `/startMeeting` edits exactly one file and
`/endMeeting` runs later, usually in a fresh session. A meeting that edits documents as it goes
leaves half-applied decisions when it overruns, and a half-applied meeting is indistinguishable from
a finished one to the next session — the same class of silent failure as the `/done` that archived
without pushing (`collab.md` #40). Decisions taken in the first half also routinely change what the
second half decides, so a document edited at item 3 and contradicted at item 14 costs more than one
edited once at the end.

**Rejected: letting `/endMeeting` push everything to `main`.** It was the original ask. The routing
table sends `official_spec_sheet.md`, `get/src/`, `documentation/` and skill frontmatter through a
branch and a PR, and both owners agreeing something in a room is not a code review — a defect in
code is invisible until something downstream reads a wrong number. So `/endMeeting` splits: working
docs direct to `main`, everything else on a branch, PR opened and never merged by the agent.

**Rejected: a separate PR for the three skills.** They all read through the `main` worktree PR #70
establishes, so landing them first would ship skills depending on a convention `CLAUDE.md` does not
yet document. Michael also asked not to have several workflow PRs open at once. Folded into #70 as
`f343402`; the PR was retitled to say so.

**Affects:** `.claude/skills/{makeAgenda,startMeeting,endMeeting}/SKILL.md` and `.claude/CLAUDE.md`
— PR #70, open. `.claude/work/meetings/2026-08-13.md` is on `main` already, covering `collab.md`
items 40–60. `collab.md` #60 is the notification to James, with an ACKNOWLEDGE ask.

*meetings-directory-and-skills · recorded 2026-08-13 14:52 — Michael.*

## 2026-08-13 16:40 — Michael — how #21's remaining tasks were implemented

**Chose:** `GraphEvolver` captures the config's raw TOML text itself, in both constructors, rather
than adding `Serialize` to `Config` or re-serializing it. `new()` reads the file text before parsing
(`Config::from_toml_str` + `.validate()` inline, replacing the `Config::from_path` call, since
`from_path` doesn't hand the text back); `from_config()` already had `text` in scope from
`PyConfig::to_toml()`.

**Why:** the provenance record has to be *exactly* what produced the run, byte for byte — a
re-serialization of the parsed `Config` risks losing comments, key order, or anything `Deserialize`
drops on the floor. The text that was actually parsed is the only thing that's certainly identical.

**Chose:** `save_logs` hand-rolls CSV with `writeln!` rather than adding the `csv` crate.

**Why:** every column is numeric (`iteration`, four `f64`s, `seed`, `run_index`) — nothing needs
quoting or escaping, so a dependency buys nothing here that isn't already free.

**Chose:** `save_results`'s provenance TOML lands at `{filename}.toml` — derived from the results
filename, not a second argument.

**Why:** answers the plan's open question; James never weighed in, so went with the plan's own
stated default. A derived path means the provenance record can't be forgotten by a caller who
only thinks to pass one filename — the same reasoning `CLAUDE.md`'s de-badge convention uses for
"the obligation moved; it did not go away."

**Chose:** `pymethods` needed `pub fn` (not just `fn`) on `save_logs`/`save_results` to call them
from a Rust test directly — `pyo3`'s `#[pymethods]` are private by default, only exposed to Python.
Matches existing precedent (`PyConfig::to_toml` is already `pub fn`), not a new pattern.

**Chose:** the "log's best row can beat reported `best_fitness`" plan task turned out to need no
site edit — `guide/evolvers.html`'s "What a run reports" section already carries the full reasoning,
correctly and without stale citations. Added only a doc comment on `PyRunResult::best_fitness`
pointing at the same reasoning, for readers who never leave the API.

**Chose:** `.gitignore`'s `.venv/` line landed on `main` directly (`092b944`), not on
`mdube_run_output`. It exists because task 8's real-Python verification needed a venv (this repo
has no `pyproject.toml` — `reference/pyo3-maturin.md` — and Debian's PEP 668 blocks a bare `pip
install`), but the entry isn't specific to #21 and every future task will want it. Landed by
temporarily widening the docs worktree's sparse-checkout to include `/.gitignore`, committing,
pushing, then narrowing back — `main` can't be checked out in two working trees at once, so this
was the only way to touch a `main`-only file without disturbing either checkout's branch.

**Affects:** `get/src/lib.rs`, `get/src/py_result.rs`, `get/src/dispatch.rs` (commits `d187d10`,
`79003ac`, `d30d31d`, `b4e3bb7` on `mdube_run_output`); `.gitignore` (`092b944` on `main`).

*run-output-remaining-tasks · recorded 2026-08-13 16:40 — Michael.*
