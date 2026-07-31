# To discuss with James (shorinbonsai)

Running agenda for collaboration checkpoints. Add items as they come up; mark them
**Agreed** with a date once settled, and don't delete — the trail is the point.

Persistent: survives `/done`, because coordination outlives any one task.

---

## Open

### 1. I implemented `common.rs`, not you
`Selection::select`, `evaluate`, and `generation_stats` are being filled in as part
of the steady-state task — steady-state cannot run without all three. **Don't
duplicate them.** Generational should call the same helpers unchanged.
*Raised 2026-07-31.*

### 2. `mating_event` breeds two children, not one
Your doc comment at `get/src/evolver/steady_state.rs:22` says "breed **a** child …
replace **the worst** individual". `Genome::crossover` recombines in place and
inherently produces **two** children, so discarding one wastes half of every
crossover. Steady-state now breeds two and replaces the two worst. The comment
needs updating — flagging rather than silently rewriting your words.
*Raised 2026-07-31.*

### 3. Steady-state pays per-event FFI cost
`Fitness::evaluate_population` exists so a Python-backed objective takes the GIL
once per generation. Steady-state scores incrementally — only the new children per
event — so it makes one FFI hop per mating event instead. Correct for native
fitness, potentially bad for Python. Generational won't have this problem.
**Decide:** accept it, or add a batched cohort mode later?
*Raised 2026-07-31.*

### 4. RNG choice must match across strategies
Steady-state seeds `ChaCha8Rng` from the `run(seed)` argument. If generational
picks anything else, the same seed means different things per strategy and runs
stop being comparable. **Pick one and both use it.**
*Raised 2026-07-31.*

### 5. Logging cadence for steady-state
`GenerationStats.iteration` is documented in `mod.rs:67` as counting mating events
for steady-state. Logging every event gives a 100k-row history on a long run, so
steady-state logs one row per `population_size` events — a "generation
equivalent", which also makes the two strategies' logs directly comparable.
**Confirm** that reading of `iteration`, since it is now sampled rather than
per-event.
*Raised 2026-07-31.*

### 6. With or without replacement — **your call for generational**
`Selection` now has two draw methods, and they sample differently:

| Method | Sampling | Used by |
|---|---|---|
| `select(count)` | **with** replacement | generational (yours) |
| `tournament_indices()` | **without** replacement | steady-state |

`tournament_indices` has no choice: it feeds tournament-local replacement, and
"the worst two members" is undefined over a multiset with duplicates.

`select` is a different matter, and **I am not prescribing this one.** It samples
with replacement because that is the textbook default (Goldberg & Deb's analysis
assumes it; DEAP's `selTournament` and ECJ both default to it), not because
generational needs it. Generational makes N independent picks to fill a
population, and whether each tournament samples with or without replacement is
orthogonal to that. Nothing depends on the current behaviour yet —
`advance_generation` is still `todo!()`.

**The only real question is whether you want one rule across both strategies.**
Both read the same `[selection] tournament_size` from config, so today that one
number means marginally different selection pressure in each. The gap is small:
expected distinct entrants for `k = 5` is 4.90 at population 100, 4.80 at 50,
4.52 at 20 — roughly 2% at realistic sizes, i.e. noise against a stochastic run.

- Want consistency? Say so and I'll rewrite `select` to sample without
  replacement. One rule, one meaning for `tournament_size`, runs directly
  comparable. Cheap now, annoying later.
- Happy with per-purpose conventions? Leave it, and we document the divergence.

Related, and now moot for steady-state: distinct sampling means the two parents
are always different individuals, so self-mating is impossible there by
construction. Generational still has the question — with replacement, two of its
picks can be the same individual, and crossover between a genome and its own
clone does nothing but mutate.
*Raised 2026-07-31, rewritten the same day once the two methods diverged.*

### 9. `Fitness` gained a `direction()` — your trait, so flagging it
Fitness functions can be either-better, so `Fitness` now carries:

```rust
fn direction(&self) -> Direction { Direction::Minimize }   // new, defaulted
```

plus a `Direction` enum with `orient()`. **Nothing breaks**: the default is
`Minimize`, `SirFitness` doesn't override it, and the old behaviour is unchanged.

The contract is now: `evaluate` returns the objective's **natural** value in its
own units; the engine minimizes internally and converts once via
`Direction::orient`, so logs and the number handed back to Python stay in the
user's units and sign. Rejected the alternative where each objective pre-negates
its own output — `evaluate` and `direction` could then silently disagree, and a
run optimizing backwards looks exactly like one that isn't converging.

**`NaN` is now forbidden by contract.** Worth knowing why, because it is sharper
than it looks: `total_cmp` puts `NaN` beyond `+inf`, so under minimization it is
worst, which is safe. Under `Maximize` we negate — and `-NaN` sorts *below*
`-inf`, making it the **best** individual in every tournament. One `NaN` from a
maximizing objective silently fills the population with whatever genome made it.
Verified in `fitness::tests::a_negated_nan_sorts_best_...`.

**It is enforced, not assumed.** `Direction::to_cost` asserts on `NaN` and is the
single gate every objective value passes through on its way into the engine, so
a `NaN` fails loudly at the moment it is produced.

Chosen over the two alternatives on the grounds that a `NaN` is a bug in the
objective, and both alternatives hide it: a direction-aware comparator that
special-cases `NaN` would quietly rank it worst and carry on, and trusting the
contract would leave the `Maximize` hazard live. A panic naming the likely
arithmetic (`0.0/0.0`, `inf - inf`, division by a possibly-zero count) is more
useful than either.

Consequence for you: **a maximizing objective that can produce `NaN` will now
panic mid-run rather than silently converge.** That is intended. If you hit it,
the fix is in the objective, not the engine.

**Decide:** happy with the assert, or would you rather it degraded gracefully?
Also worth agreeing that direction belongs on the trait rather than in
`config.toml` — config could contradict what the objective actually computes.
*Raised 2026-07-31; contract-only approach revised to enforcement the same day.*

### 7. The tree is not `cargo fmt`-clean
`get/src/evolver/generational.rs`, `steady_state.rs`, `fitness.rs` and
`genomes/sda.rs` all differ from `cargo fmt` output. Anyone who runs `cargo fmt`
sweeps all four and hands the other person a pile of unrelated diff — it happened
on this task and the changes had to be reverted by hand.
**Decide:** run `cargo fmt` across the tree once, in its own commit, and keep it
clean from then on.
*Raised 2026-07-31.*

### 8. `Cargo.lock` stays tracked
PR #11 added `Cargo.lock` to `.gitignore`; that line was dropped when merging.
The crate builds a pyo3 extension module — application-like, not a library — so a
committed lockfile is what makes builds reproducible. The line was also inert on
its own, since the file was already tracked. This overrides your stated intent, so
it deserves a word. See `decisions.md`, 2026-07-31.
*Raised 2026-07-31.*

---

## Agreed

### `Genome::copy` removed — accepted (2026-07-31)
PR #11 dropped `fn copy(&self)` from the `Genome` trait. Zero callers, and
`Genome: Clone` already provides `.clone()`. Accepted as-is. Consequence:
`Planning Notes.md` still lists `copy` as a required genome method and is now the
stale side — update the notes, don't re-add the method.

### README "Graph multiplicity" section deleted — correct (2026-07-31)
PR #11 removed it. Verified it documented `Graph::unweighted()`,
`Graph::with_max_edge_multiplicity()`, and `SdaContext::new/unweighted/...`, none
of which exist — commit 520500b replaced them with the two-arg `Graph::new` and
left the README stale. The deletion was a fix.
