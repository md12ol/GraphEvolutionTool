# Next session — 2026-08-12

**Machine:** `pop-os` · saved 2026-08-12 · `f02cdce`

Read this task's `plan.md` and `.claude/work/decisions.md` first, then `.claude/work/hotfixes.md`.

**Where things stand:** Branch `jsargant_set_base_graph` is cut from `main` (`da073aa`). Tasks 1–2
are done and verified: the branch exists, and `GraphEvolver` carries a `base_graph: Option<Graph>`
field (`get/src/lib.rs:56`), currently unread. Committed as `f02cdce`, not pushed. The design call
for task 3 is already made — see `decisions.md` 2026-08-12: the cap-narrowing check rejects, it
does not warn.

**Start here:** task 3 in `plan.md` — implement `set_base_graph(&mut self, num_nodes: usize, edges:
Vec<(usize, usize, u32)>) -> PyResult<()>` as a `#[pymethods]` fn in `get/src/lib.rs`, next to
`set_fitness_function`. Three `PyValueError` checks, in this order: node-count mismatch against
`self.config.network_size`; genome is not `GenomeConfig::EdgeEdit`; any edge multiplicity exceeds
`self.config.max_edge_multiplicity`. Only build and store the `Graph` after all three pass.

**Watch out for:**
- `cargo test -p get` needs `LD_LIBRARY_PATH` exported first — see `traps.md`,
  `cargo-test-cannot-link-python-unless-extension-module-is-off` — or it dies at exit 127 before
  any test runs.
- `f02cdce` is unpushed. Push before opening the PR (task 7), not before.
- Tasks 3 and 4 are coupled: the field stays `dead_code` until task 4 threads it through
  `dispatch::evolve`/`edge_edit_start`. Don't be surprised by the warning persisting after task 3
  alone.

**⏰ Time-sensitive:** none.
