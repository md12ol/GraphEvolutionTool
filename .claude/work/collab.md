# Collaboration agenda

Shared running agenda for checkpoints between the owners of this repo — Michael (md12ol) and
James (shorinbonsai). Both of us read and write this file; it is not addressed at either one.

An item belongs here when a decision on one side **conflicts with or overrides** work on the
other. Add items as they come up; mark them **Agreed** with a date once settled, and don't
delete — the trail is the point.

Stamp every item with who raised it: `*Raised YYYY-MM-DD — <author>.*` This file is `merge=union`
(see `/.gitattributes`), so two people appending in the same week merge cleanly, but only the
stamp makes an accidental duplicate visible.

Persistent: survives `/done`, because coordination outlives any one task.

Renamed from `meeting_james.md` on 2026-07-31 — the old name was written from one side and read
as self-referential on the other's machine.

---

## Open

### 1. I implemented `common.rs`, not you
`Selection::select`, `evaluate`, and `generation_stats` are being filled in as part
of the steady-state task — steady-state cannot run without all three. **Don't
duplicate them.** Generational should call the same helpers unchanged.
*Raised 2026-07-31 — Michael.*

### 2. `mating_event` breeds two children, not one
Your doc comment at `get/src/evolver/steady_state.rs:22` says "breed **a** child …
replace **the worst** individual". `Genome::crossover` recombines in place and
inherently produces **two** children, so discarding one wastes half of every
crossover. Steady-state now breeds two and replaces the two worst **of the
tournament they were drawn from**, not the two worst in the population. The
comment needs updating — flagging rather than silently rewriting your words.
*Raised 2026-07-31 — Michael.*

### 3. Steady-state pays per-event FFI cost
`Fitness::evaluate_population` exists so a Python-backed objective takes the GIL
once per generation. Steady-state scores incrementally — only the new children per
event — so it makes one FFI hop per mating event instead. Correct for native
fitness, potentially bad for Python. Generational won't have this problem.
**Decide:** accept it, or add a batched cohort mode later?
*Raised 2026-07-31 — Michael.*

### 4. RNG choice must match across strategies
Steady-state seeds `ChaCha8Rng` from the `run(seed)` argument. If generational
picks anything else, the same seed means different things per strategy and runs
stop being comparable. **Pick one and both use it.**
*Raised 2026-07-31 — Michael.*

### 5. Logging cadence for steady-state, and an iteration-0 row
`GenerationStats.iteration` is documented in `mod.rs:67` as counting mating events
for steady-state. Logging every event gives a 100k-row history on a long run, so
steady-state logs one row per `population_size` events — a "generation
equivalent", which also makes the two strategies' logs directly comparable.
**Confirm** that reading of `iteration`, since it is now sampled rather than
per-event.

Steady-state also logs the **starting population as iteration 0**, before any
breeding. Without it a log cannot show where a run began, and a run shorter than
one interval produced no rows at all. So
`history.len() == num_mating_events / population_size + 1`.

**Generational should do the same** — log the initial population as generation 0
— or the two strategies' logs are off by one row and cannot be plotted on the
same axes.
*Raised 2026-07-31 — Michael; iteration-0 row added the same day.*

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
*Raised 2026-07-31 — Michael, rewritten the same day once the two methods diverged.*

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
Verified in `fitness::tests::an_unchecked_negated_nan_would_have_sorted_best`.

**It is enforced, not assumed.** `Direction::orient` asserts on `NaN` and is the
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
*Raised 2026-07-31 — Michael; contract-only approach revised to enforcement the same day.*

### 10. `evaluate` now orients — reversing what your doc promised
Your doc on `common::evaluate` said it "deliberately says nothing about which
fitness is *best* — the lower-is-better convention lives with the caller". That
is now false, on purpose: `evaluate` applies `Direction::orient`, so the
fitnesses it returns are always lower-is-better regardless of the objective.

The reason is that it is the single place the whole population is scored, so it
is the only spot where the conversion can happen exactly once. Leaving it to
callers would mean every comparison site needing the direction, which is the
design we rejected.

Consequence for generational: `advance_generation` can compare the fitnesses
`evaluate` returns directly, without consulting direction. Only the reporting
boundary converts back — see #11.
*Raised 2026-07-31 — Michael.*

### 11. `generation_stats` needs a `direction` parameter
To log in the objective's own units, `generation_stats` has to convert back, so
its signature gains `direction`. Worth knowing the asymmetry: **`best_fitness`
and `mean_fitness` flip sign, `std_dev` does not** — deviation is invariant
under negation. It looks like a missed case; it is not.
*Raised 2026-07-31 — Michael.*

### 12. `config.rs` should reject `tournament_size < 4` for steady-state
Tournament-local replacement needs four distinct individuals per event: two
parents at the front of the tournament, two replaced at the back. Three still
preserves the tournament's best but makes the second parent one of the replaced;
two breaks the self-elitism guarantee outright, since both parents are replaced
by their own children.

`SteadyStateEvolver::new` asserts it, and also asserts `population_size >=
tournament_size` — but that is a backstop, not the right home. **The config layer
is**, since it already knows both numbers and can report a bad file cleanly
instead of panicking. Note the constraint is strategy-specific: generational has
no such floor, so this cannot be a blanket validation on `tournament_size`.
*Raised 2026-07-31 — Michael.*

### 7. The tree is not `cargo fmt`-clean
`get/src/evolver/generational.rs`, `steady_state.rs`, `fitness.rs` and
`genomes/sda.rs` all differ from `cargo fmt` output. Anyone who runs `cargo fmt`
sweeps all four and hands the other person a pile of unrelated diff — it happened
on this task and the changes had to be reverted by hand.
**Decide:** run `cargo fmt` across the tree once, in its own commit, and keep it
clean from then on.
*Raised 2026-07-31 — Michael.*

### 8. `Cargo.lock` stays tracked
PR #11 added `Cargo.lock` to `.gitignore`; that line was dropped when merging.
The crate builds a pyo3 extension module — application-like, not a library — so a
committed lockfile is what makes builds reproducible. The line was also inert on
its own, since the file was already tracked. This overrides your stated intent, so
it deserves a word. See `decisions.md`, 2026-07-31.
*Raised 2026-07-31 — Michael.*

### 13. `.claude/` is now shared — read this before your first session
You're picking up this workflow on your machine, so `.claude/` went from one
person's notes to a shared, tracked directory. Five things changed for you:

- **Don't run `/setup`.** It rewrites `CLAUDE.md` from the template's FILL IN
  blocks and would destroy it. You're cloning an already-configured project —
  start with `/load`.
- **Stamp every entry with `— James`.** `decisions.md`, `traps.md`, `hotfixes.md`,
  `issues.md` and this file merge with `merge=union` (`/.gitattributes`), so our
  appends never conflict — but union merge never conflicts about *anything*,
  including two edits to the same entry. The stamp is what makes a silent
  duplicate visible. After any merge that touched them, read the tail.
- **`hotfixes.md` entries carry `Owner:` and `Machine:`.** An uncommitted hotfix
  of mine isn't in your tree. Check the owner before going to look for the code.
- **`work/current/` is yours alone** (gitignored); `work/archive/` is now shared,
  so finished tasks reach both of us. Personal settings →
  `settings.local.json`, also gitignored.
- **Hook and `settings.json` changes go through a PR, both ways.** Those files
  execute on the other person's machine at session start, on their next pull,
  without them reading the diff. Everything else in `.claude/` is just text; that
  part isn't.

Reasoning in `decisions.md`, 2026-07-31 (four entries). **Confirm** you're happy
with the union-merge trade — it is the one choice here with a real downside.
*Raised 2026-07-31 — Michael.*

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

### Spec-sheet call — items 2, 3, 4, 5, 6, 9, 10, 12 settled (2026-07-31)
Both owners on a call, working through `IMPLEMENTATION.md` against the code and writing
`/official_spec_sheet.md`. Dispositions, by the item numbers under **Open** above:

- **#2 two children per mating event** — agreed as built. The stale doc comment at
  `steady_state.rs:22` is superseded by the spec, §6.3.
- **#3 steady-state per-event FFI cost** — accepted, not fixed. Recorded in the spec as a known
  limitation with a recommendation to prefer generational for stochastic or Python objectives.
- **#4 RNG must match across strategies** — agreed, `ChaCha8Rng` in both.
- **#5 logging cadence and an iteration-0 row** — agreed; generational logs generation 0 too.
- **#6 with or without replacement** — **James's call, made: `select` stays with replacement.**
  The ~2% divergence in expected distinct entrants is documented in the spec rather than removed.
- **#9 `Fitness::direction()`** — agreed, and extended: direction is fixed per objective and never
  a config field. The `NaN` assert stays; `±inf` is explicitly allowed as the "invalid individual"
  idiom.
- **#10 `evaluate` orients** — agreed, and the function is renamed `express_and_score` and made the
  engine's sole scoring entry.
- **#12 config-layer validation** — agreed, and widened into a single `Config::validate` that both
  the TOML and Python front ends call.

**#11 is reversed, not agreed.** That item asked for a `direction` parameter on
`generation_stats`. The spec instead keeps the whole engine in one orientation and converts only at
the Python boundary, so `generation_stats` **loses** the parameter and the `std_dev` asymmetry it
described disappears. See `decisions.md`, "The engine is oriented internally".

**#7 `cargo fmt`** — still open, now staged in `issues.md` as a batched formatting + readability
pass owned by Michael, to land when James's tree is clean.
*Raised 2026-07-31 — Michael, from the joint spec call.*
