# GET — Official Spec Sheet

The design of the Graph Evolution Tool: what each component **is**, the contract it offers, and
the invariants that are not obvious from its signature.

**This document does not sequence work.** No build order, no task list, no "what's left". Those
live in a separate planning document. Where this spec describes something not yet built, it says
so in the status table below and nowhere else — the design is the same either way.

Superseded `IMPLEMENTATION.md`, which mixed design with a build order and had gone stale on
fitness direction, steady-state replacement, and mutation. Started 2026-07-31.

| Component | Status |
|---|---|
| `Graph` | built |
| `EdgeEditGenome`, `SdaGenome` | built (mutation contract pending, §4) |
| `Selection`, population scoring, logging stats | built |
| `SteadyStateEvolver` | built |
| `GenerationalEvolver` | designed, not built |
| `sir_sim` + the three objectives | designed, not built |
| `Config` parsing | partly built; validation not built |
| Python interface | designed, not built |

---

## 1. Pipeline

```
Genome  --express(context)-->  Graph  --Fitness-->  f64
   ^                                                 |
   |          Evolver<G> selects / breeds / mutates  |
   +-------------------------------------------------+
```

Each stage is an independent trait, so representations and objectives swap without touching the
engine. The engine is **statically generic** over the genome: `Evolver::run<F>` is a generic
method, so `Box<dyn Evolver>` is not viable and runtime choice is resolved by a dispatch `match`
that instantiates concrete types (§8).

**The two dispatch axes are not symmetric, and only one of them is a match.** `Genome` cannot be a
trait object — `mutate` and `crossover` are generic over the RNG, `crossover` takes `&mut Self`,
`Clone` requires `Sized`, and `Context` is an associated type that differs per representation. So
strategy × genome stays a `match`. `Fitness` has none of those problems: no generic methods, no
`Self` in argument position, and `dyn Fitness` is `Send + Sync` through its supertrait. The
objective is therefore **erased to `Box<dyn Fitness>` before the evolver is instantiated** (§8),
which collapses dispatch from strategy × genome × objective to strategy × genome.

**Three bounds make the pipeline parallel, and each is load-bearing.** An implementor who sees
them undocumented will try to remove one.

- `Genome: Clone + Send + Sync` — the population is expressed across rayon workers. `Clone` is
  how an individual is copied; there is no `copy` method.
- `Genome::Context: Send + Sync` — one `&Context` is shared by every worker during expression.
  Parallel expression does not compile without it.
- `Fitness: Send + Sync` — the default batch scorer fans out with `par_iter`.

Net effect: expression and native scoring are both parallel and free of the Python GIL. That
property is what makes the Python fitness design in §8 matter.

---

## 2. `Graph`

An undirected **multigraph** on a fixed node count, stored as a symmetric adjacency matrix of
`u32` weights. Weight `0` means no edge; `k > 1` means `k` parallel edges. Every graph carries
its own `max_edge_multiplicity`; `1` makes it a simple unweighted graph.

| Behaviour | Consequence |
|---|---|
| `set_edge` **clamps** to the cap rather than rejecting | Feeding a cap-5 result into a cap-1 run silently collapses every weight to 1 (§8, base graph) |
| Self-loops and out-of-range vertices are **silently ignored** by `set_edge` and `weight` | Genome expression relies on this: a decoded vertex pair is never validated before use |
| `degree` counts **distinct neighbours**; `total_edge_multiplicity` counts **edge copies** | They differ on any multigraph, and the edit operations depend on which they use |
| `get_neighbor_at_index` wraps modulo the degree | Lets an arbitrary 32-bit payload always name a real neighbour. `None` only for an isolated or invalid node |
| `get_edge_list` returns each edge once, `u < v`, row-major | The same order SDA expression writes in, and the shape the Python `run` returns |

`add_edge` adds one parallel edge, saturating at the cap, and reports whether the multiplicity
actually changed. `remove_edge` removes one copy; `clear_edge` removes all.

---

## 3. Genomes

```rust
pub trait Genome: Clone + Send + Sync {
    type Context: Send + Sync;
    fn express(&self, context: &Self::Context) -> Graph;
    fn crossover<R: Rng + ?Sized>(&mut self, other: &mut Self, rng: &mut R);
    fn mutate<R: Rng + ?Sized>(&mut self, rng: &mut R);
    fn print(&self) -> String;
}
```

`crossover` recombines **in place**, leaving one child in each parent — it inherently produces
two children, and an engine that keeps only one wastes half of every crossover.

`print` is the **type-erasure hook**: it lets the non-generic Python entry point record which
individual won without knowing its representation (§8).

`Context` carries run-level expression configuration and is the authority on graph size and edge
cap — `SdaContext` states them directly, `EdgeEditContext` through its base graph.

### 3.1 `EdgeEditGenome` — an edit script over a base graph

A `Vec<u64>` of encoded operations. Each gene packs an **opcode in the low 4 bits** and a
**32-bit payload above it**; expression decodes the payload mixed-radix into four vertex
parameters, `payload % n`, `/n % n`, `/n² % n`, `/n³ % n`.

Expression clones `context.base_graph` and applies every gene in order. Unknown opcodes and
zero-node graphs are skipped. The genome is a *script*, not a graph — the same genome expressed
against a different base graph gives a different result.

The nine operations, all no-ops when their preconditions fail:

| | |
|---|---|
| `Add` / `Delete` | add or remove one edge copy between `v1, v2` |
| `Toggle` | absent → add; at cap → remove; otherwise `v3`'s parity decides |
| `LocalAdd` / `LocalDelete` / `LocalToggle` | the same three, but the far endpoint is reached by a **two-hop walk** from `v1` (neighbour `v2`, then its neighbour `v3`), rejecting a walk that returns to the start or passes a degree-1 node |
| `Hop` | move an edge: connect `v1` to its two-hop endpoint, then drop the edge to the intermediate neighbour. Only if the new edge actually took |
| `Swap` | 2-opt rewire. Given two non-adjacent vertices of degree > 2 and one neighbour each, cut both edges and cross-connect — but only if all four vertices are distinct and none of the three would-be edges already exists. Requires ≥ 4 nodes |
| `Null` | deliberate no-op; weight it to 0 to make every gene do something |

Five of the nine (`Swap`, `Hop`, the three `Local*`) are **inert on an empty base graph** — they
need existing structure to walk. A from-scratch edge-edit run therefore does nothing until
`Add`/`Toggle` have built something. Self-correcting, and not a bug.

**Operation weights** are relative, not percentages, and are validated once at startup into a
shared `Arc<EdgeEditOperators>` holding a prebuilt sampler. The whole population shares one
pointer rather than a copy of the weights, and a weight error surfaces from config rather than
mid-run. At least one weight must be positive; `0.0` disables an operation outright.

**Crossover:** two-point, over genes. Draw a segment within the shorter genome's length and swap
it. A genome shorter than two genes is left alone.

### 3.2 `SdaGenome` — a self-driving automaton generating a graph from scratch

A finite-state machine: an `init_char`, a `[state][char] -> next state` transition table, and a
`[state][char] -> Vec<char>` response table. `max_resp_len` is stored rather than derived because
it is not observable from the current data.

**Expression emits one character per upper-triangle pair, and each character's raw value *is*
that edge's weight.**

- Output length is `n(n-1)/2`. Index `i` maps onto the `i`-th pair in the same row-major order as
  `get_edge_list` — `(0,1), (0,2), …, (0,n-1), (1,2), …`. So `output[0]` is `init_char`, and the
  initial character directly sets edge `(0,1)`.
- The run starts in `init_state` and **consumes its own output as the driving tape**: the
  character at the read head selects a transition, that transition's response is appended
  (truncated if it would overshoot), the state advances, and the head moves on.
- Every response is at least one character long, so the run always terminates. No step cap is
  needed.
- Fewer than two nodes yields an empty graph without running the automaton.

**The alphabet is derived, not configured.** `num_chars = max_edge_multiplicity + 1`, so
characters are exactly `0..=cap` and every character is a legal edge weight that `set_edge` will
never clamp. This closes two silent failures: an alphabet larger than the cap biases the graph
toward the cap as surplus characters clamp down onto it, and a smaller one makes the upper
weights unreachable, quietly exploring less space than configured. Unweighted (`cap = 1`) gives a
binary alphabet reading as absent/present.

Consequence: `max_edge_multiplicity` must satisfy `1 <= cap <= 255` — `0` yields a one-character
alphabet and a permanently empty graph, and `256` exceeds the `u8` response storage.

**`init_state` is run configuration, not genome data.** It lives on `SdaContext` because
mutation and crossover never touch it. It must be `< num_states`: expression indexes the response
table with it, so an out-of-range value panics. Graphs of two nodes never run the loop, so the
bug hides at small sizes.

**Crossover:** two-point over **states** — swap a contiguous band of states, transitions and
responses together. Swapping state 0 also swaps `init_char`, since together they determine the
automaton's first transition.

---

## 4. Variation

**Crossover** is gated by `crossover_rate`, rolled once per pair. Both children are kept.

Each genome offers **one** crossover operator, fixed: two-point over genes for edge-edit,
two-point over states for SDA (§3).

**Additional operators are out of scope until everything else in this sheet is implemented.** This
is a sequencing decision, not a design limit: more operators per representation, and a config field
selecting between them, are wanted — they come *after* the sheet is delivered, not alongside it.
Until then there is nothing to select between, so no enum, no config field and no dispatch is built
for crossover. When it does land it follows `Selection`: one new variant plus one match arm,
mapping onto a `config.toml` field.

**Mutation is two independent rolls, both owned by the engine, never by the genome:**

1. `mutation_rate` — whether this child mutates at all.
2. `max_mutations` — if it does, how many mutations it takes, drawn uniformly from
   `1..=max_mutations`.

**`Genome::mutate` applies exactly one mutation per call.** This is a trait-level contract, not a
per-representation choice: a genome that rolls its own count internally makes `max_mutations`
mean nothing for that representation, and nothing would report it.

What "one mutation" *is* belongs to the representation:

- **edge-edit** — reroll one gene, opcode drawn from the operation mix.
- **SDA** — redraw either `init_char` (4% of calls), one transition target, or one response
  (even odds between the latter two).

Both rolls happen in one shared helper, so the two evolution strategies cannot drift apart on
mutation semantics the way they did on selection sampling (§6.1).

> *Change of 2026-07-31.* Edge-edit previously rolled `1..=4` internally against a hardcoded
> constant. At the default `max_mutations = 1` it now mutates less per event; set `4` to recover
> the old strength. SDA is unchanged at the default and correspondingly more disruptive above it.

---

## 5. Fitness

```rust
pub trait Fitness: Send + Sync {
    fn evaluate(&self, graph: &Graph) -> f64;
    fn direction(&self) -> Direction { Direction::Minimize }
    fn evaluate_population(&self, graphs: &[Graph]) -> Vec<f64> { /* par_iter over evaluate */ }
}
```

`evaluate` returns a score **in the objective's own units and sign**. `direction` declares
whether bigger or smaller is better. The engine minimizes internally, converting once, so logs
and the value handed back to Python stay in the user's units.

Declaring a direction rather than making each objective negate itself is deliberate: if
`evaluate` returned an already-negated value *and* the trait declared `Maximize`, the two could
silently disagree, and a run optimizing backwards is indistinguishable from one that simply is
not converging.

**Direction belongs to the objective, never to config.** One fitness function is always maximized
or always minimized — it is a property of what the function computes, not of the run. The native
objectives fix their own (§5.2). A user-supplied Python objective declares its direction **when
the callable is registered**, because nothing can infer it from the function.

**`NaN` is forbidden and enforced, not merely documented.** Every value entering the engine
passes through one gate, which panics on `NaN`. The reason is sharper than it looks: under
minimization `NaN` sorts beyond `+inf` and reads as worst, which is safe — but under `Maximize`
the value is negated, and `-NaN` sorts *below* `-inf`, making it the **best** individual in every
tournament it enters. One `NaN` from a maximizing objective fills the population with whatever
genome produced it and leaves a run that looks converged.

**`±inf` is allowed.** `+inf` under `Minimize` is the sanctioned way to say "this individual is
invalid, never select it". Removing it would push users toward a magic large number, which is
worse because it stays silently comparable.

### 5.1 Where the conversion happens — one gate, named `express_and_score`

```
common::express_and_score(population, context, fitness) -> (Vec<Graph>, Vec<f64>)
```

Expresses every genome in parallel, scores the batch, and **applies the direction conversion
exactly once**. The fitnesses it returns are always lower-is-better, whatever the objective. Index
`i` of both vectors is `population[i]`.

This is the engine's single point of conversion, and the reason is that it is the only place the
whole population is scored — so it is the only place "exactly once" can be *guaranteed* rather
than remembered. The alternative, a direction-aware comparator, needs the direction at every
comparison site: tournament ordering, the generational argmin, the steady-state best. A missed
one fails **silently**, and a run optimizing backwards is indistinguishable from one that simply
is not converging.

It is also the `NaN` gate above. Orientation and rejection share one door by design.

**Invariant: the engine never calls `Fitness::evaluate` or `Fitness::evaluate_population`
directly.** `express_and_score` is the sole path from a population to a set of fitnesses —
generational scoring, steady-state child scoring, everything. The two trait methods exist to be
*implemented* by an objective and to be *called by* `express_and_score`, never by the engine.

This is what makes the guarantees above structural rather than habitual. A direct call would
bypass orientation (producing raw values that compare backwards under `Maximize`) and bypass the
`NaN` rejection, and neither failure announces itself. Any new evolution strategy scores its
population through this function or it is wrong.

The name says both phases out loud, and the split between them is load-bearing: expression is
parallel and GIL-free, scoring is the single batched crossing into Python. It also removes a
collision — `Fitness::evaluate` scores one graph in the user's units, and this scores a
population in the engine's, which are different enough jobs to deserve different names.

**Fixing the double flip: convert out once, at the edge of the system — not inside the engine.**

The wart was that `generation_stats` and the run outcome each converted *back* immediately, so a
number flipped twice on a short trip and every internal type had to know the direction. The rule
instead:

- Everything inside the engine — the fitness arrays, `GenerationStats`, `EvolutionOutcome` — is
  in **engine orientation**, lower-is-better. Nothing in the middle converts.
- The **Python boundary** converts once on the way out: the returned best fitness, the CSV log,
  the results file. It holds the direction because `EvolutionOutcome` carries it.

Three things fall out, and the third is the real prize:

1. Exactly **two conversions per run**, at the two edges, instead of one per stats row.
2. `generation_stats` loses its `direction` parameter entirely.
3. **The `std_dev` special case disappears.** Standard deviation is invariant under negation, so a
   converting `generation_stats` had to deliberately *not* convert it — an asymmetry that looked
   like a missed case and had to be defended by a test. When nothing converts internally, there is
   nothing to except.

Cost, stated plainly: a Rust embedder reading `EvolutionOutcome` directly gets engine-oriented
numbers. That is what the direction on the outcome is for, and the field names should say so.

#### Your units and the engine's, end to end

There are two number systems, and every value belongs to exactly one of them.

- **Your units** — whatever your objective naturally returns. `epi_spread` counts infected nodes,
  so bigger is better. `epi_prof_match` is an RMSE, so smaller is better. Nothing is normalized,
  scaled, or sign-flipped for you.
- **Engine orientation** — one rule, no exceptions: **lower is better**. Selection, elitism and
  replacement all assume it, which is what lets them work without ever asking what the objective
  was.

The conversion between them is a negation, and negation is its own inverse — so one function maps
both ways, and the entire design is about calling it in exactly two places.

```
  ┌───────────────────────────────────────────────────────────────────────┐
  │  YOUR UNITS                                                           │
  │  epi_spread → 1470 nodes infected          direction = Maximize       │
  └───────────────────────────────────────────────────────────────────────┘
                                   │
                       Fitness::evaluate  →  1470.0
                                   │
  ══════════════ common::express_and_score ══════════════════════════════
     express population in parallel  ·  score the batch
     reject NaN  ·  ORIENT — the one and only flip inward
                                   │
                          1470.0  ──→  −1470.0
                                   ▼
  ┌───────────────────────────────────────────────────────────────────────┐
  │  ENGINE ORIENTATION — lower is better. Nothing here converts.         │
  │                                                                       │
  │    tournament ranking   ·   elitism   ·   replace-worst   ·   argmin  │
  │    GenerationStats      ·   EvolutionOutcome                          │
  │                                                                       │
  │    None of these know, ask, or store which way the objective ran.     │
  └───────────────────────────────────────────────────────────────────────┘
                                   │
                         −1470.0  ──→  1470.0
                                   │
  ══════════════════════ Python boundary ═══════════════════════════════
     best_fitness()  ·  save_logs()  ·  save_results()
     ORIENT BACK — the one and only flip outward
                                   │
                                   ▼
  ┌───────────────────────────────────────────────────────────────────────┐
  │  YOUR UNITS AGAIN — 1470, exactly what your function returned.        │
  │  std_dev crosses unchanged: deviation is identical in either sign.    │
  └───────────────────────────────────────────────────────────────────────┘
```

Read the diagram as a claim about *where the flips are*, not how many arithmetic operations
happen. Two flips per run, at the two edges. Everything between them is a plain `f64` comparison
with no direction in sight — which is precisely why a missed conversion is impossible rather than
merely unlikely.

### 5.2 One simulator, three objectives

The expensive part is the epidemic; all three objectives want the same one. So a single
simulation returns everything, and each objective is a thin reading of it.

```
sir_sim(graph, params, rng) -> SirRun { length, spread, profile }
```

**The model:** SIR with a **one-timestep infectious period**. A node infected during a step
spends the *following* step infectious — transmitting to each still-susceptible neighbour with
probability `infection_rate` per edge — then becomes recovered and never infects again. A single
`patient_zero` seeds the outbreak, which runs until no infected nodes remain.

`profile[t]` is the count of **newly infected** nodes at timestep `t`, with `profile[0] = 1` for
patient zero and a **terminating zero** as its last element. `length` counts every timestep the
epidemic occupied, including the final one in which the last infectious node recovers without
transmitting. So an outbreak that infects nobody beyond patient zero has `length = 1`,
`spread = 1`, and `profile = [1, 0]`; a 6-node path burning through at `infection_rate = 1.0` has
`length = 6`, `spread = 6`, and `profile = [1, 1, 1, 1, 1, 1, 0]`.

> **Amended 2026-08-04 — Michael & James.** This previously read `length = 0` for a lone patient
> zero and specified no trailing zero. It now matches `legacy/Graph.cpp`, which is the intended
> behaviour: `Graph::SIR` increments `epiLen` on the burnout pass and writes `epiProfile[epiLen] = 0`.
> `spread` is unchanged — the C++ `totInf` already agreed with it. Consequently `length` is one
> higher than `profile.len() - 1` under the old convention, and `epi_prof_match` compares against a
> profile one element longer. `get/src/sir.rs` was built to the old wording and is corrected by its
> own issue.

**Short epidemics are re-rolled.** Agreed 2026-08-04, porting `legacy/main.cpp`. An outbreak that
burns out in fewer than `min_epidemic_length` timesteps is discarded and re-simulated, up to
`max_epidemic_retries` attempts; whatever the final attempt produces is kept regardless. Both are
config fields, defaulting to the C++ constants — `max_epidemic_retries = 5` (`rse`) and
`min_epidemic_length = 3` (`mepl`).

```
attempts = 0
repeat
    run = sir_sim(graph, params, rng);  attempts += 1
until run.length >= min_epidemic_length or attempts >= max_epidemic_retries
```

**Why it exists:** a fizzled outbreak carries no information about the graph. It reports that the
dice went badly, not that the network is poor, and without the re-roll a large share of evaluations
return near-nothing and selection chases the dice instead of structure.

**Be clear about what it is: a biased resample, not variance reduction.** It shifts expected
fitness upward, by an amount that depends on how often a given graph fizzles — so it is *not*
interchangeable with raising `num_epidemics`, and the two do different jobs. This is accepted
deliberately, for comparability with the historical C++ results, and is why both values are
exposed rather than hardcoded.

**Disabling it** is `min_epidemic_length = 1`: every epidemic has `length >= 1` under the
convention above, so nothing is ever re-rolled. `max_epidemic_retries = 1` gives one attempt and is
equivalent. Both must be at least 1 (§7).

**Seed epidemics by position, using the same scheme as replicate seeding (§8.1).** The batch seed
seeds a generator whose output stream *is* the epidemic seed list, and epidemic `i` attempt `a`
takes draw `i × max_epidemic_retries + a`. Not a second mechanism — the identical one, applied to a
different index, and it inherits §8.1's reasoning wholesale, including why the index must not be
folded in with `xor`.

**This is what keeps the re-roll compatible with common random numbers**, and drawing the epidemics
sequentially from one stream instead would not. Whether a graph re-rolls depends on its own
outcome, so a graph that retries consumes extra draws — and under sequential drawing every
subsequent epidemic in that evaluation is then offset from the graphs that did not retry, for the
rest of the batch. Position-indexed seeds resynchronise at the next epidemic index, exactly as
asking for 50 replicates leaves the first 30 unchanged.

The property this preserves is worth stating precisely, because "the re-roll breaks CRN" is the
easy misreading. **Every graph in the batch draws from an identical pool of dice** —
`h(batch, i, 1) … h(batch, i, max_epidemic_retries)` are the same for all of them, and none is
graph-specific. What differs between graphs is only *which* of those common draws each one stops
on, and that is what a retry **is**. The randomness is common; the stopping rule is
outcome-dependent, deliberately.

Position-indexing is also what makes scoring order-independent, so a population evaluated across
rayon workers reproduces exactly regardless of which worker reaches which graph first.

**The `num_epidemics` simulations for one evaluation run sequentially**, never concurrently.
Parallelism comes from the two levels above — replicates and the population (§8.1) — which
together already provide far more independent work than any core count, and a third nesting level
would add scheduling overhead for nothing. Keeping the epidemics serial also makes each
population-level task substantially larger, which improves the amortization of the two levels that
do run in parallel.

| Objective | Reads | Direction |
|---|---|---|
| `epi_spread` | total ever-infected | **Maximize** |
| `epi_length` | timesteps to burn out | **Maximize** |
| `epi_prof_match` | **RMSE** between `profile` and the target | **Minimize** |

Each direction is fixed by the objective and is never configurable — see §5.

**`num_epidemics`** — how many independent epidemics one evaluation averages over, set by the
user. It is not a tuning nicety: a single SIR draw is very noisy, and selection will happily chase
that noise instead of graph structure. It also dominates run time, since everything else in an
evaluation is cheap by comparison.

#### Dice: shared within an evaluation, different across evaluations

Two requirements pull in opposite directions, and both matter.

- **Within one evaluation**, every population member must face the *same* epidemic draws. That is
  common random numbers: fitness differences then reflect the graph rather than the dice, which is
  what makes selection meaningful under a stochastic objective.
- **Across evaluations**, the same network must *not* keep getting the same epidemic. Otherwise
  the run optimizes against one frozen sample of the disease rather than the disease.

The trait forces the mechanism. `evaluate` takes `&self` and `Fitness: Sync`, so an objective
cannot carry a mutable RNG across a parallel population. So:

> The objective holds a **run seed** plus an **atomic evaluation counter**. Each call to the batch
> scorer increments the counter once and derives that batch's epidemic seed from
> `(run_seed, counter)`. Every graph in the batch is simulated from that one derived seed; the next
> batch gets a different one. Reproducible, shared within a batch, fresh across batches.

**The counter is per-run state, not shared.** Each run owns its own objective instance and
therefore its own counter. A counter shared across concurrently executing runs (§9) would let
thread scheduling decide which run sees which epidemic seed, and reproducibility would evaporate
in a way that only shows up under load.

**There is one seed for the whole run**, supplied to the Python `run` call, and everything derives
from it: the starting population, the evolution loop, and the epidemics. `[fitness]` carries no
seed of its own — a separately configured fitness seed would mean replicate runs at different
evolution seeds still faced *identical* epidemic draws, which is exactly the thing replicates
exist to vary.

Deriving the seed from the graph's contents or its index instead would satisfy neither
requirement — it reintroduces exactly the between-individual noise CRN removes, *and* freezes each
network's epidemic forever.

**Known limitation, accepted: steady-state carries stale fitnesses.** Steady-state scores only the
two new children per mating event, so the batch counter advances per *event*. The fitness values
sitting in the population were computed under different dice than the children being compared
against them, and an individual that drew lucky dice keeps its inflated score for the rest of the
run because nothing rescores it. Generational is immune — it rescores everyone each generation
under one seed.

This is inherent to combining incremental scoring with a stochastic objective: the alternatives
(rescoring the population periodically, or freezing the seed across a generation-equivalent) each
trade it for a different distortion rather than removing it. Accepted 2026-07-31. **Prefer
generational when the objective is stochastic and the comparison needs to be fair.**

`patient_zero` may be pinned by the user, and must be a real node — validated against
`network_size` in §7. Unset, a fresh node is drawn per epidemic.

**Guidance:** keep hot objectives native in Rust. The Python adapter (§8) is for prototyping,
where developer speed matters more than run time.

---

## 6. Evolvers

```rust
pub trait Evolver<G: Genome> {
    type TypeContext;
    fn new(shared: SharedEvolutionContext<G>, type_context: Self::TypeContext, population: Vec<G>) -> Self;
    fn run<F: Fitness>(&mut self, fitness: &F, seed: u64) -> EvolutionOutcome<G>;
}
```

**The evolver does not build genomes.** Constructors differ per representation and some are
fallible, so a generic evolver cannot mint a `G`. The dispatch layer builds the starting
population, where genome-specific knowledge already lives, and hands it in. This keeps `run`'s
signature clean, surfaces bad dimensions at startup rather than mid-run, and makes evolvers
directly testable from a hand-built population.

**Every value has one source of truth.** `SharedEvolutionContext` carries only the genome
context, the rates, `max_mutations` and the selection strategy. Population size is
`population.len()`. Network size and edge cap belong to the genome context. Config still carries
all three, but the dispatch layer is the only place they are read — a second copy on the context
could drift, and nothing would report the disagreement.

`EvolutionOutcome` carries the best genome, **its already-expressed graph**, its fitness, and the
run history.

**Reproducibility:** both strategies seed `ChaCha8Rng` from `run`'s `seed`. Not `StdRng`, whose
algorithm may change between `rand` releases — which would defeat the entire purpose of a seed
argument. The same seed must mean the same thing in both strategies.

### 6.1 Selection

Tournament selection, as an enum so a new mechanism is one variant plus one match arm. Ordering
is by fitness, ties broken by **lower index**, so a tournament's outcome depends only on which
indices were drawn and not on the order the RNG produced them.

Two draw methods, sampling **differently**, deliberately:

| Method | Sampling | Used by |
|---|---|---|
| `select(count)` | **with** replacement | generational |
| `tournament_indices()` | **without** replacement | steady-state |

`tournament_indices` has no choice — it feeds tournament-local replacement, and "the worst two
members" is undefined over a multiset. `select` keeps the textbook default. The divergence means
one `tournament_size` implies marginally different pressure per strategy (≈4.90 expected distinct
entrants at `k=5`, population 100 — roughly 2%, noise against a stochastic run). Accepted
2026-07-31.

Consequence for generational: two picks can be the same individual, so crossover between a genome
and its own clone does nothing but mutate. Steady-state cannot self-mate by construction.

### 6.2 Generational

The whole population is replaced each generation. Per generation: score the population, log a
stats row, track the best, then advance — copy `elite_count` best individuals forward unchanged,
then fill the remaining slots by selecting parent pairs, recombining by `crossover_rate`, and
mutating by §4.

Tracking the best uses the graph that scoring already built, so the winner is never re-expressed.
Comparison is by total order rather than `partial_cmp().unwrap()`.

**Odd slot count.** Crossover yields two children, but `population_size - elite_count` may be odd.
The last pair contributes **one** child; the other is discarded. Only the final pair is ever
affected.

**Elites are rescored every generation**, like everyone else — they are simply copied forward
unchanged, not exempted from scoring. Under a stochastic objective this means an elite's recorded
fitness moves between generations while its genome does not. That is correct: the new number is a
fresh sample of the same individual, and freezing the old one would let a lucky draw persist
(§5.2).

**Logs the initial population as generation 0**, matching steady-state, so the two strategies'
histories share an axis.

### 6.3 Steady-state

One mate-and-replace event at a time; most of the population persists between events.

**Each event draws a single tournament of distinct individuals. Its two best breed; the two
children overwrite that same tournament's two worst.** Tournament-local, not global.

- **Self-elitist.** The tournament's best is never among the replaced, so the population's best
  is never discarded and no explicit elitism is needed.
- **Diversity-preserving.** A globally poor individual survives until it happens to be drawn.
- **Cheap** — `O(k log k)` per event rather than an `O(population)` scan for the global worst,
  which matters at 100,000 events.
- **Replacement is unconditional** — a child takes its slot even if it scores worse than what it
  displaces. The best-never-discarded guarantee comes from the tournament structure, not a
  comparison.

**`tournament_size >= 4` is required**, and the population must be at least that large. Two
parents and the two individuals they replace must be distinct: three still preserves the
tournament's best but makes the second parent one of the replaced; two breaks self-elitism
outright, since both parents are replaced by their own children. The constraint is
**strategy-specific** — generational has no such floor.

Scoring is incremental: only the two new children are evaluated per event, in one batch. Correct
and cheap for a native objective; for a Python-backed one it means one FFI hop per *event*
instead of per generation, which makes steady-state a poor fit for Python fitness.

**Logging cadence:** the starting population as iteration 0, then one row per `population_size`
events — a "generation equivalent". So `history.len() == num_mating_events / population_size + 1`.
Logging every event would give a 100,000-row history; the iteration-0 row is what makes a log
self-contained, and without it a run shorter than one interval produced nothing at all.

### 6.4 Run output

Every run produces two things: a **convergence log** and a **best individual**.

**The log** — one row per logged iteration, which is every generation for generational and every
`population_size` events for steady-state:

| Column | |
|---|---|
| `iteration` | generation number, or mating-event number |
| `best_fitness` | best in the population at that iteration |
| `mean_fitness` | population mean |
| `std_dev` | **population** deviation (divides by `n` — these are all the individuals there are, not a sample, so a single individual has deviation zero) |
| `ci_95` | 95% confidence interval half-width on the mean — see the caveat below |

**The best individual** — the most fit genome (via `Genome::print`), the network it expressed as
a weighted edge list, and its fitness.

Everything in the log is in **engine orientation** while inside the engine; the Python boundary
converts `best_fitness` and `mean_fitness` on the way out and leaves `std_dev` and `ci_95` alone,
since a spread is identical under negation.

**`ci_95` is within-population, per iteration.** Half-width `1.96 · s / √n`, where `s` is the
**sample** deviation (divides by `n-1`). It answers "how tightly is this generation clustered
around its mean". Note the deliberate mismatch with the `std_dev` column beside it, which is the
**population** deviation: `std_dev` describes the population as the complete thing it is, while a
confidence interval is inherently a sampling statistic and needs the sampling denominator to mean
anything. Reporting both is fine; computing the interval from the population deviation would not
be. A single individual has `std_dev = 0` and an undefined `ci_95`, which is reported as `0`.

This is the *within-run* band. The **across-run** band — mean best-fitness over `n` replicates with
a spread showing run-to-run variability, which is the usual published convergence plot — is a
different number and is computed when the replicate logs are aggregated, not here. Both are
legitimate; they answer different questions, and the run index on every row (below) is what makes
the second one possible.

**Also worth carrying, and cheap:** the run's seed and run index on every row, so 30 runs can be
concatenated into one file and still be separable; and the generated TOML (§8) written alongside
the results as the provenance record of what produced them.

---

## 7. Configuration and validation

```toml
population_size       = 200
network_size          = 100
max_edge_multiplicity = 1     # 1..=255; 1 = unweighted (default)
crossover_rate        = 0.9
mutation_rate         = 0.2   # roll 1: is this child mutated?
max_mutations         = 1     # roll 2: how many, uniform 1..=max (default 1)

[evolution]                   # generational | steady_state
type            = "generational"
num_generations = 500
elite_count     = 1

[selection]                   # tournament
type            = "tournament"
tournament_size = 5

[genome]                      # edge_edit | sda
type        = "edge_edit"
gene_length = 256
# [genome.operation_weights]  relative, all default 1.0, 0.0 disables

[fitness]                     # epi_spread | epi_length | epi_prof_match | python
type           = "epi_spread"
infection_rate = 0.05
num_epidemics  = 30           # epidemics averaged per evaluation
# patient_zero = 0            optional; omit for a random node per epidemic
# min_epidemic_length = 3     re-roll outbreaks shorter than this; 1 disables
# max_epidemic_retries = 5    attempts before keeping whatever came out
```

**No seed appears in the config.** One master seed is supplied to the Python `run` call and
everything derives from it — the starting population, evolution, epidemics, and the per-replicate
seed stream (§8.1). Run count is likewise a `run` parameter, not config: both describe *this
invocation*, while the config describes the experiment.

The SDA genome takes `num_states`, `max_resp_len` and `init_state` (default 0) — **not**
`num_chars`, which is derived from the edge cap (§3.2).

The three SIR objectives share their parameters; only `epi_prof_match` adds a target. Flatten the
shared block rather than triplicating it, so the TOML stays flat and the Rust stays DRY.

**Validation is a function, not a side effect of parsing.** Today deserialization *is* the
validation — missing fields, wrong types, unknown keys. A second construction path from Python
(§8) bypasses serde entirely, so unless validation is an explicit `Config::validate` that **both
front ends call**, the Python path silently accepts configurations the TOML path rejects. That is
the worse direction, because Python is the path users actually take.

Everything belongs there, not scattered through dispatch:

- `init_state < num_states`
- `1 <= max_edge_multiplicity <= 255`
- `tournament_size >= 4` **for steady-state only**, and `population_size >= tournament_size`
- `max_mutations >= 1`
- operation weights finite, non-negative, at least one positive
- `patient_zero < network_size` when pinned — a node index that isn't in the network
- `num_epidemics >= 1`
- `min_epidemic_length >= 1` and `max_epidemic_retries >= 1` — both default to the C++ constants
  (3 and 5); `min_epidemic_length = 1` disables the re-roll rather than being an error (§5.2)
- `elite_count < population_size` — equal means no breeding happens and the run is a fixed point
- base graph node count and cap narrowing (§8)

**It runs in Rust, before anything is done** — before a population is minted, a graph allocated,
or a generation run. Everything downstream may then assume a valid config.

**It returns an error; it does not panic.** A bad config is a user mistake, not a bug, so it must
surface as a proper Python exception naming the offending field and its constraint — not as a
panic crossing the FFI boundary, which reaches the user as an opaque `PanicException` they cannot
act on. The `assert!`s inside the evolvers are backstops for direct Rust use (tests, embedding);
a config-driven run must never reach one.

Note `max_edge_multiplicity` is not an SDA-only concern despite motivating the alphabet in §3.2 —
a cap of `0` clamps every edge to zero weight and yields a permanently empty graph under *any*
genome, and the resulting run looks like a broken fitness function rather than a bad config.

**Direction is not a config field, and this is settled.** One fitness function is always maximized
or always minimized; it is a property of what the function computes. `epi_spread` and `epi_length`
maximize, `epi_prof_match` minimizes, and a config that could say otherwise would only let it
contradict the objective. A Python objective supplies its direction when the callable is
registered (§8).

---

## 8. Python interface

A pyo3 extension module (`abi3-py38`). The user fills typed configuration objects in Python and
passes them to an evolver. A hand-written TOML file is the other way in.

**The Python objects serialize to TOML, and that TOML is what Rust parses.** Python is a *builder*
for the config format, not a second parser of it:

```
Python config objects ──serialize──> TOML ──serde──> Config ──validate──> run
                                              hand-written TOML ─┘
```

The two front ends therefore converge **before** anything is parsed, so there is exactly one
parser and one validator and no way for the Python path to accept something the TOML path rejects.
It also gives run provenance for free: the generated TOML *is* the record of what was run, and can
be written next to the results and re-run verbatim.

Two consequences to design around. Error messages will reference a TOML document the user never
wrote, so field errors need mapping back to the Python attribute that produced them or they are
useless. And anything large or awkward in TOML — a long target profile, a base graph — is better
delivered through a setter than serialized into the document (see below).

**The objective is erased before dispatch; the genome is not.** Agreed 2026-08-04. The config's
fitness variant is turned into one `Box<dyn Fitness>` *first*, in a match that touches neither
strategy nor genome, and the dispatch below then instantiates only strategy × genome. A new
objective is one arm in that first match and never touches dispatch at all — where a
three-axis match would have cost one arm per strategy × genome combination.

Three things this requires, each of which fails silently if missed:

- **A forwarding `impl Fitness for Box<dyn Fitness>`**, which must live beside the trait — the
  orphan rule rejects it anywhere else. It exists because `Evolver::run<F>` needs `F: Fitness`, and
  a `Box` holding a `Fitness` is not itself one until said so.
- **Every method forwarded, including the defaulted ones.** `direction` and `evaluate_population`
  both have defaults, so omitting either compiles and is wrong: a maximizing objective would
  silently run backwards, and a Python objective would fall back to the per-individual rayon
  fan-out that §8 exists to prevent.
- **A factory, not a value.** Replicates need per-run objective instances (§8.1), so the erasing
  match must be re-run once per replicate rather than producing one shared box — a single boxed
  objective handed to `n` concurrent runs is exactly the shared-counter bug §8.1 warns about.

The cost is one virtual call per `evaluate`, which for an SIR objective sits behind
`num_epidemics` complete epidemics and is not measurable. The parallelism story is unaffected:
`dyn Fitness` is `Send + Sync`, and `PyFitness`'s `evaluate_population` override is still reached
through the box.

**The entry point cannot be generic.** `#[pyclass]` types cannot carry a type parameter, but
`EvolutionOutcome<G>` does. So each dispatch arm must **erase the genome type before it returns**,
keeping only: best fitness, the run history, the best graph's edge list, and the best genome's
`print()` string. That last one is what `Genome::print` exists for — it lets a non-generic entry
point record which individual won without knowing its representation.

**`run` returns a result object, and there is no `best_fitness()` accessor.** The erased outcome
above *is* the return value — best fitness, best edge list, best genome string, and the log rows —
so a separate getter would be a second way to read state the caller already holds. Returning it
also removes a wart: a cached-fitness accessor has to answer *something* before any run has
happened, and the honest answer is infinity, which under a maximizing objective converts back into
a suspiciously excellent score.

State lives in the returned value, not on the evolver. The evolver is then reusable across
replicate runs without stale results from the previous one hanging off it.

### 8.1 Replicate runs

Research use is `n` runs at identical parameters, so run count is a user parameter and `run`
returns a **list** of results, one per replicate.

**Seeding: one master seed, not `n` of them.** The master seed seeds a generator whose output
stream *is* the per-run seed list — run `i` takes draw `i`. One parameter, fully reproducible, and
with one property worth preserving deliberately: **a run's seed must not depend on how many runs
were requested.** Drawing from a stream gives that for free — asking for 50 runs reproduces the
first 30 exactly, so extending an experiment never invalidates the replicates you already have.
Deriving from `master + i` or `hash(master, i)` works equally well; `master ^ i` does not, because
nearby masters collide across run indices.

**Replicates run in parallel only with a native Rust objective.** Replicates are independent, so
concurrency across them is nearly free — but only when the scoring is Rust. With
`fitness = "python"`, `n` concurrent runs means `n` threads contending for one GIL: slower than
sequential *and* contended. So parallel replicates are a **Rust-fitness-only** capability.

**The engine chooses the mode; the user does not.** The fitness type already determines the
correct answer, so exposing it as a setting only creates a way to choose wrong:

| Objective | Replicates |
|---|---|
| native Rust (`epi_spread`, `epi_length`, `epi_prof_match`) | **parallel** |
| `python` | **sequential** |

**The user caps the cores, and that is the only knob.** A `max_cores` parameter on the `run` call —
like the seed and run count, it describes *this invocation*, not the experiment. Unset means all
available; `1` means fully sequential. There is no point exceeding the replicate count, so the
effective figure is `min(max_cores, n)`. Under a Python objective the cap is moot for replicates,
since those are sequential regardless.

**Sequential replicates does not mean a sequential run.** Expression is pure Rust and GIL-free, so
a Python-fitness run still expresses its population across threads — only the single scoring call
per batch is serialized. The cap governs both levels: one thread pool of at most `max_cores`
threads, from which replicate-level and population-level work both draw, so total concurrency stays
bounded no matter how the two nest.

> **Build that pool locally, not globally.** Rayon's global pool can be configured only once per
> process. This crate is a Python extension module imported once per session, so a global
> configuration would make `max_cores` a property of whichever `run` call happened first — and the
> second call with a different cap would fail outright.

**`max_cores` is a memory knob as much as a CPU one, and the three sizes multiply.** Agreed
2026-08-04. Expression materializes the whole population as `Vec<Graph>` before scoring, and
`Graph` is a **dense** `network_size × network_size` matrix however sparse the graph actually is
(§2). Peak memory is therefore roughly

```
network_size² × 4 bytes × population_size × min(max_cores, n_replicates)
```

because each concurrently-executing replicate holds its own expressed population. The three
parameters interact rather than adding, so raising any one of them scales the whole product:

| `network_size` | one graph | population of 200 | × 8 concurrent replicates |
|---|---|---|---|
| 100 | 40 KB | 8 MB | 64 MB |
| 500 | 1 MB | 200 MB | 1.6 GB |
| 1000 | 4 MB | 800 MB | 6.4 GB |

The failure mode is unintuitive and worth stating plainly: a user who has run a configuration
successfully and then raises `max_cores` to use a bigger machine multiplies peak memory by the same
factor, and can exhaust it on hardware that handled the smaller setting comfortably. **The Python
layer must surface this** — documented on `run`, beside `max_cores` and the run count, not left for
the user to derive from the Rust internals.

**Where the parallelism actually comes from, which is not symmetric across strategies.** Replicates
are embarrassingly parallel — independent, long-running, no synchronization. Population-level work
is not: every generation is a barrier, so a generation costs as long as its slowest individual, and
under a stochastic objective evaluation times vary widely (a fizzled outbreak returns almost
immediately; one that burns through the graph does not). Prefer replicate-level concurrency where
there is a choice.

Steady-state has no choice: it scores only the two new children per mating event (§6.3), so its
population-level parallelism is **two-way regardless of `max_cores`**. A single steady-state run on
a large machine will leave most of it idle, and replicates are the only way to use it. This is the
same conclusion §6.3 reaches from the FFI-cost side.

Two further constraints make parallel replicates correct rather than merely fast:

1. **Per-run state must be per-run.** Each run needs its own objective instance, hence its own
   epidemic counter (§5.2). Sharing one across concurrent runs lets thread scheduling determine
   which run sees which epidemic seed, and reproducibility disappears in exactly the conditions
   that are hardest to debug.
2. **Results are collected in run order**, never completion order, or the same master seed yields
   differently ordered output on different machines.

Runtime configuration that cannot live in a config file is selected by config and delivered
through setters before `run`:

- **A Python fitness callable, with its direction.** Config says `type = "python"`; the callable
  and its `Direction` are registered together, since nothing can infer whether the user's function
  is meant to be maximized (§5). It must implement the **batched** contract — one call receiving
  the whole population — because the alternative serializes every callback behind the GIL, losing
  all rayon parallelism *and* paying lock contention, on top of Python being far slower for the
  same arithmetic. A naive per-individual objective can be hundreds of times slower wall-clock
  than parallel Rust; batched, only the speed of the Python body remains.

  The surface is modelled on **pymoo's `Problem._evaluate`** — a batch of solutions in, an array
  of objective values out, vectorized in numpy. That idiom is already familiar to the audience,
  and it is the same shape the batching argument above demands, so the ergonomic choice and the
  performance choice agree.

- **The target profile for `epi_prof_match`.** A user-supplied vector of newly-infected counts,
  passed as a sequence of numbers through a setter rather than serialized into the generated TOML —
  it is bulk data, and a long inline array makes the provenance document unreadable. Validated
  non-empty and finite at registration, alongside the other config checks, so a malformed target
  fails before any evolution starts rather than producing an RMSE against nothing. Required
  whenever `type = "epi_prof_match"`, and rejected as a contradiction if supplied for any other
  objective.
- **An edge-edit base graph.** Not a config value: it is either data the user brings or the output
  of a previous run. Since `run` already returns `(u, v, multiplicity)` triples, the two
  representations **stack with no new data format** — evolve a topology with SDA, then refine it
  with edge-edit.

Two structural rules keep the parallelism real: expression happens in parallel *before* any Python
is called — never call Python from inside a rayon closure — and the long Rust portion releases the
GIL, with the Python adapter re-acquiring it per batch.

**Three checks the base-graph setter owes**, all of which fail quietly if skipped: the node count
must match the configured network size, or out-of-range edges are silently dropped; **cap
narrowing must be rejected or warned**, because `set_edge` clamps and piping a cap-5 result into a
cap-1 run silently collapses every weight (this is the main stacking trap); and with no base graph
supplied, the run starts from an empty graph, which leaves five of the nine opcodes inert until
`Add`/`Toggle` build some structure (§3.1).

---

## 9. Open decisions — none

**None.** Every design question raised while writing this sheet is settled. What remains is
implementation, and the sequencing for that lives in a separate document.

Two things are decided here but not yet true of the code, and will read as discrepancies until
built: the one-mutation contract with `max_mutations` (§4), and the SDA alphabet derived from the
edge cap (§3.2). Where this sheet and the code disagree, **this sheet is the intent** — that is
the one place the usual "the repo wins" rule is inverted, because the code has not caught up yet.

---

## 10. Non-goals

Stated so their absence reads as a decision rather than an oversight.

- **Single objective.** One `f64` per individual. The pymoo-shaped interface (§8) is about the
  batched call signature, not multi-objective optimization — there is no Pareto front, no
  non-dominated sorting, no crowding distance.
- **Fixed-length runs.** A run ends after `num_generations` or `num_mating_events`. There is no
  convergence detection, no stagnation cutoff, no target-fitness early stop.
- **Fixed-size genomes.** `gene_length` and the SDA dimensions are set once per run. Crossover
  tolerates unequal lengths defensively, but nothing produces them.
- **No random immigrants or restarts.** Adding them would require reintroducing genome-minting
  capability into the engine, which §6 deliberately removes.
- **Fixed node count.** Every graph in a run has `network_size` nodes.
- **One population.** No islands, no migration.

---

## Conventions

Lower is better inside the engine, always — selection, elitism and replacement all assume it.
Fitness values cross back into the objective's own units only at the reporting boundary.
All randomness derives from one user seed. Absolute dates, never relative.
