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
