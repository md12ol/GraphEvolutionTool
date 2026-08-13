# Plan — GitHub #28: `set_base_graph` and its three validation checks
_Started 2026-08-12 · last updated 2026-08-12_

## Objective
Add `GraphEvolver.set_base_graph(num_nodes, edges)` so an edge-edit run can start from a
caller-supplied graph (raw data, or a previous run's `best_edges`) instead of always starting
empty. Ship the three validation checks the issue calls out — node-count mismatch, edge-multiplicity
cap narrowing, and the already-correct unset-empty default — plus the doc note on empty-base
opcode inertness. Depends on #27 (closed) and the `RunResult`/`best_edges` shape it shipped.

**Out of scope:** SDA never has a base graph (§3.2) — `set_base_graph` on an SDA-configured
evolver is a rejected call, not a silent no-op. Anything about `max_cores`/replicate runs is #20,
a separate task.

**Verify by (whole task):** `cargo test -p get`, plus a stacked SDA→edge-edit run in a scratch
script showing the edge-edit run's starting population expresses the seeded edges.

## Tasks

- [x] Create branch `jsargant_set_base_graph` — cut from `da073aa`, confirmed level with
      `origin/main`. `git rev-parse --abbrev-ref HEAD` prints `jsargant_set_base_graph`.

- [x] Added `base_graph: Option<Graph>` to `GraphEvolver` (`lib.rs:56`), `None` in both
      constructors and all 5 test literals. `cargo test -p get`: 235 passed, 0 failed. Committed
      `f02cdce`, not pushed.

- [x] `set_base_graph` landed in `lib.rs` next to `set_fitness_function`, three `PyValueError`
      checks in the documented order, `Graph` built only after all three pass. `cargo doc` clean
      (the 8 remaining warnings all pre-date it); behaviour covered by task 5's tests. `3af041c`.

- [x] Threaded through dispatch: `base_graph: Option<&Graph>` on `dispatch::evolve` and
      `dispatch::edge_edit_start`, `run()` passes `self.base_graph.as_ref()`, stale "#28 will do
      this" comments replaced. `cargo test -p get` 235 passed; `dead_code` on the field gone.
      Existing `..._sizes_the_population_and_the_empty_base_graph` passes with its assertions
      untouched (its call site gained the new `None` argument, unavoidably).

- [x] Four tests in `dispatch.rs`, one per behaviour, plus a "nothing stored on rejection" assert
      on each rejecting case. `cargo test -p get`: 239 passed. Teeth verified by disabling the
      threading and all three checks — exactly those four failed, other 235 passed. `0e48a01`.

- [x] Doc note on empty-base opcode inertness — in `set_base_graph`'s doc comment and again in
      `edge_edit_start`'s, naming all five opcodes and saying it is self-correcting, not a defect.
      `cargo doc -p get --no-deps` clean for this item (8 warnings, all pre-existing, none in
      range). Python docstring reach to be confirmed in task 7, when maturin builds anyway.

- [x] `documentation/` consequences filed as two entries in `documentation/jsargant_edits.md` —
      `#set-base-graph-ships` (the badge, the plan-note, the `status.html` row) and
      `#set-base-graph-cap-rejects` (the site says a narrowed cap silently collapses; it raises
      now). Queued not applied, per that file's rule superseding `CLAUDE.md`'s de-badge *timing*.
      `HANDOFF.md`'s mirror row left to `collab.md` #50/#57. Rides the PR, as `mdube_edits.md` did.

- [ ] Manual stacked-run check: a small script (SDA config → `run()` → `best_edges`; edge-edit
      config with matching `network_size`/`max_edge_multiplicity` → `set_base_graph(n, edges)` →
      `run()`) confirming the edge-edit population's expressed graphs include the seeded edges
      before any mutation. Scratch-only, not committed.
      **Verify by:** printed edge list from the second run includes edges from the first at
      generation 0 / a 0-mutation-rate config.

- [ ] Open PR, request review from Michael. `CLAUDE.md`: I don't merge my own PR.
      **Verify by:** `gh pr view` shows it open against `main`.

## Open questions
- None currently — the issue's own text resolves "reject or warn" toward reject, matching every
  other validation path in this crate (`Config::validate`, `python_fitness`) failing loudly rather
  than warning into a log nothing reads.

## Out of scope
- #20 (replicate runs, `max_cores`) — separate issue, also assigned to me, not started.
- Whether `set_base_graph` should also accept a `Graph`-shaped Python object rather than raw
  `(num_nodes, edges)` — not raised anywhere, following the issue's own example signature.
