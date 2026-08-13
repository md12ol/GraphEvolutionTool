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

- [ ] Implement `set_base_graph(&mut self, num_nodes: usize, edges: Vec<(usize, usize, u32)>) ->
      PyResult<()>` as a `#[pymethods]` fn in `lib.rs`, next to `set_fitness_function`. Three
      checks, all `PyValueError`:
      1. `num_nodes != self.config.network_size` → reject (§3.1 check 1).
      2. Genome is not `GenomeConfig::EdgeEdit` → reject (SDA has no base graph, §3.2).
      3. Any edge's multiplicity exceeds `self.config.max_edge_multiplicity` → reject rather than
         silently clamp (§3.1 check 2 — "the main stacking trap"). Build the graph with
         `Graph::new` + `set_edges` only after this check passes.
      Store the built `Graph` in `self.base_graph`.
      **Verify by:** unit tests below; `cargo doc -p get --no-deps` renders without warnings.

- [ ] Thread it through dispatch: add `base_graph: Option<&Graph>` param to
      `dispatch::evolve` and `dispatch::edge_edit_start` (`dispatch.rs:218,312`);
      `edge_edit_start` uses `base_graph.cloned().unwrap_or_else(|| Graph::new(config.network_size,
      config.max_edge_multiplicity))` in place of the current unconditional `Graph::new` at
      `dispatch.rs:236`. `lib.rs`'s `run()` (`lib.rs:253`) passes `self.base_graph.as_ref()`.
      Update the doc comment at `dispatch.rs:205` — it currently says #28 "will" do this.
      **Verify by:** `cargo test -p get` — existing
      `the_edge_edit_start_sizes_the_population_and_the_empty_base_graph` still passes unmodified
      (unset case).

- [ ] Tests in `dispatch.rs`'s `#[cfg(test)]` module: a set base graph is what `edge_edit_start`
      expresses against (context.base_graph equals what was set, not empty); node-count mismatch
      rejected with a message naming both numbers; cap-narrowing rejected (e.g. set with weight 3
      against a cap-1 config) with a message naming the offending edge; `set_base_graph` on an
      SDA-configured evolver rejected.
      **Verify by:** `cargo test -p get`, each new test named for the behaviour it checks.

- [ ] Doc note on empty-base opcode inertness, per the issue ("needs a doc note so it is not
      misread as a bug") — fold into `set_base_graph`'s own doc comment: unset means empty, and
      five of nine opcodes (`Swap`, `Hop`, the three `Local*`) are no-ops on an empty graph.
      **Verify by:** read the rendered doc, `cargo doc -p get --no-deps`.

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
