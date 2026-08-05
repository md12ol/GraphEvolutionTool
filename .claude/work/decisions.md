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
