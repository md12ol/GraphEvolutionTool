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

- [x] Stacked SDA→edge-edit run verified through real Python (`maturin develop --release` into a
      gitignored `.venv`). Stage 2 ran null-only operation weights with mutation and crossover at
      0, so expression is a guaranteed no-op and the check is exact equality, not overlap: all
      **358** seeded edges expressed, none added or lost. Control with no base graph expressed 0
      edges, so the assertion is not vacuous. All three checks surface as `ValueError` across the
      FFI, and `save_logs`/`save_results` wrote the CSV and the provenance TOML. Script is
      scratch-only, not committed.

- [x] **PR #72** open against `main`, review requested from Michael, `mergeable=MERGEABLE`,
      carrying exactly `get/src/lib.rs`, `get/src/dispatch.rs` and `documentation/jsargant_edits.md`
      — no `.claude/work/` diff, per `collab.md` #58. Body verified by read-back (tables and code
      spans intact). The body flags `collab.md` #61 as the one thing for the reviewer to weigh.
      Not merged: `CLAUDE.md`, I don't merge my own PR.

## Open questions
- None currently — the issue's own text resolves "reject or warn" toward reject, matching every
  other validation path in this crate (`Config::validate`, `python_fitness`) failing loudly rather
  than warning into a log nothing reads.

## Out of scope
- #20 (replicate runs, `max_cores`) — separate issue, also assigned to me, not started.
- Whether `set_base_graph` should also accept a `Graph`-shaped Python object rather than raw
  `(num_nodes, edges)` — not raised anywhere, following the issue's own example signature.
