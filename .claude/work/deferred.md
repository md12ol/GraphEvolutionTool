# Deferred — designed-shaped, deliberately not now

Three files already say something about work that does not exist yet, and each says a different
thing. This one fills the gap between them:

| | |
|---|---|
| `official_spec_sheet.md` §10 Non-goals | **never** — absences that are decisions, not oversights |
| **this file** | **not yet** — wanted, out of scope for the first release |
| GitHub `md12ol/GraphEvolutionTool` | **now** — filed, scoped, someone will start it |

**An entry names the change and what would have to be true to admit it. No dates, no ordering, no
priority.** That last part is not a style preference: `official_spec_sheet.md` opens by refusing to
sequence work, because `IMPLEMENTATION.md` mixed design with build order and rotted every time the
order changed. A ranked list here would be the same file growing back under a new name — and
GitHub is already the ranked list.

**If it has a shape and someone would plausibly start it this month, it is a GitHub issue instead**,
staged through `issues.md` if it needs root-causing first. An entry leaves this file the moment it
is filed, replaced by nothing — the tracker then carries it, same sync obligation `issues.md` runs
on.

**Not union-merged, and that is deliberate.** Entries are *deleted* when they graduate, and union
merge cannot express a deletion — a delete racing an edit to the same region is silently discarded
and the entry comes back (measured 2026-08-04, see `/.gitattributes`). This file is absent from
`.gitattributes` on purpose, so it takes git's normal 3-way merge and a concurrent append conflicts
loudly. Do not add it there.

**Nothing here binds the spec sheet.** Admitting an entry is a `collab.md` item and a joint meeting
like any other sheet change; an entry graduating to a GitHub issue is not itself agreement that the
sheet should change.

*Created 2026-08-11 — Michael, after §10 turned out to conflate "never" with "not yet": directed
graphs read as permanently closed when nobody had actually decided that.*

---

## Entries

### <the change, as a noun phrase>

- **What:** one paragraph. What the thing is, in enough detail that a cold reader knows whether a
  later idea is the same one.
- **Admitting it requires:** what would have to change to make it possible — the components, the
  invariants it breaks. Not an estimate, and not a plan.
- **Raised:** <YYYY-MM-DD> — <owner>, in <what surfaced it>.


### Directed graphs

- **What:** `Graph` is undirected today and it is not a configuration choice anywhere — spec §2
  states it as design ("an undirected **multigraph** on a fixed node count, stored as a symmetric
  adjacency matrix"), and the code enforces it structurally rather than with a flag. There is no
  route in the API that writes an asymmetric matrix. A directed mode would mean edges where
  `(u, v)` and `(v, u)` carry independent weights, which is what an epidemic on a contact network
  with asymmetric transmission would want.
- **Admitting it requires:** more than `graph.rs`. Every write funnels through `set_edge`, which
  sets `adjacency[u][v]` and `adjacency[v][u]` together, so the symmetry is an invariant four other
  methods rely on rather than a line to delete. `get_edge_list` reads only the upper triangle, and
  that ordering is load-bearing twice over — SDA expression writes in exactly that order (§3.2),
  and it is the shape the Python boundary hands to a user's objective (§8). `sir_sim` walks
  neighbours with no notion of direction. The edge-edit operations `hop`, `swap` and the three
  `local_*` variants are all defined over undirected neighbourhoods (§3.1). Realistically it is a
  second `Graph` variant or a type parameter, not a mode flag.
- **Raised:** 2026-08-11 — Michael, asking whether undirected was a decision or an accident.


### The demo Python layer runs replicates and plots them

- **What:** the Python interface (§8) currently gets you as far as `run(seed)` returning an edge
  list. The demo layer we want on top drives several runs and then visualizes without further
  input: **boxplots of best fitness across replicates**, so the spread between seeds is visible
  rather than a single lucky number, and **a rendering of the best graph itself**. Both are
  straightforward with networkx and matplotlib over the edge list the boundary already returns —
  the graph comes back as `(u, v, multiplicity)` tuples, which `add_weighted_edges_from` takes
  directly.
- **Admitting it requires:** mostly things already filed rather than anything new in the engine.
  Replicate runs are **#20** (one master seed, Rust-only parallelism, `max_cores`) and the run
  output — the convergence log and the best individual — is **#21**; `save_logs` and `save_results`
  are `todo!()` until that lands. A per-generation series would make convergence curves possible
  alongside the boxplots, which is the same #21. The open question this file cannot settle: whether
  the demo layer is a Python module shipped in the repo or an example script, and whether
  matplotlib becomes a real dependency of the package or stays a documented extra for the demo
  only.
- **Raised:** 2026-08-11 — Michael, scoping what the Python layer is actually for.


### A `get.Graph` wrapper, so Python objectives stop rebuilding what Rust already has

- **What:** `Graph` is a plain Rust struct with no `#[pyclass]`, so none of its twelve methods
  cross the boundary. A Python objective receives `(num_nodes, edge_list)` — the output of one of
  them, `get_edge_list` — and rebuilds whatever it needs, typically by constructing a networkx
  graph per individual. Nothing is *lost*: the upper-triangle list is complete and the adjacency
  matrix is exactly recoverable. What is lost is the cost. `degree`, `has_edge`, `weight` and
  `total_edge_multiplicity` are row scans over a dense matrix in Rust; in Python they are a
  reconstruction first, paid once per graph per generation.
- **Admitting it requires:** deciding what the wrapper *is* before deciding it exists, because the
  cheap version defeats the point. Handing out an object whose methods each cross the FFI boundary
  per call is worse than the rebuild it replaces — the whole reason the objective takes a batch is
  that a per-graph crossing deadlocked when measured on 2026-08-07 (`reference/pyo3-maturin.md`
  §2). So the shape is probably a read-only view over the batch — one crossing, with the dense
  matrix exposed as a buffer Python can address without copying — rather than a per-graph object
  with getters. It also widens the public API surface: today the only contract is a tuple, and
  anything richer is a thing to keep stable.
- **Raised:** 2026-08-11 — Michael, working through what a Python objective actually receives.


### Pluggable selection, crossover and mutation operators

- **What:** all three variation and selection stages are effectively fixed. `SelectionConfig` is
  an enum with exactly one variant, `Tournament` — and it is an enum rather than a bare integer
  *specifically* so a second scheme does not change the shape of the Python API
  ([py_config.rs:209](../../get/src/py_config.rs#L209) says so outright). Crossover and mutation
  have no equivalent axis at all: §4 fixes one contract, and the only tunables are
  `crossover_rate`, `mutation_rate` and `max_mutations`. Wanted: alternatives at each of the three
  stages, selectable from config the way the genome and fitness axes already are — roulette or
  rank selection beside tournament, and named crossover and mutation operators rather than the
  single implied one.
- **Admitting it requires:** the selection axis is nearly free — the enum, the config variant and
  the validation already have the shape, and §6.1 is the only text to extend. Crossover and
  mutation are the harder half, because the operator is per-genome, not global: an edit-script
  crossover and an SDA crossover are different functions over different data (§3.1, §3.2), so a
  config-level operator name has to mean something in both representations or be rejected as an
  invalid pairing. That is the same strategy × genome dispatch problem `dispatch.rs` already
  solves once, and doing it a second and third time is the actual cost. Note §4's mutation
  contract — one mutation is one gene for edge-edit and one transition for SDA — is exactly the
  kind of per-representation meaning a shared operator name would have to preserve.
- **Raised:** 2026-08-11 — Michael, alongside the `get.Graph` entry.


### Objectives beyond the three SIR ones

- **What:** `epi_spread`, `epi_length` and `epi_prof_match` read one simulator three ways (§5.2).
  Other objectives are wanted; **which ones is not decided, and this entry is not a proposal for
  any particular one.** Recorded so the question is visibly open rather than looking closed by the
  three that exist.
- **Admitting it requires:** possibly nothing structural — §5.3 already documents both extension
  routes, native Rust and a registered Python callable, and a new native objective is an enum
  variant plus a `Fitness` impl. The real question is which belong *in the tool* versus which
  belong in a user's own Python, and that is a judgement about scope rather than a build problem.
  An objective that does not read an epidemic at all — anything purely structural over the graph —
  would be the first to test whether `SirParams` sitting inside `FitnessConfig` still fits.
- **Raised:** 2026-08-11 — Michael, explicitly as an open question with nothing settled.
