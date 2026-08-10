# best-index — extract both evolvers' argmin into `common::best_index`

**GitHub issue:** #51 (closed) · **PR:** #55, merged by James at 2026-08-10T20:52:32Z as `9274f38`
**Dates:** single session, 2026-08-10

## Objective

`GenerationalEvolver::outcome` and `SteadyStateEvolver::outcome` both opened by answering the same
question — which index in `fitnesses` is best? — in two different styles: an explicit `for` loop in
one file, `(0..len).min_by(..).expect(..)` in the other. `CLAUDE.md` prefers the explicit loop.
Done meant both calling one shared helper written that way. Scoped at the joint meeting of
2026-08-09; raised as `collab.md` #36 by James.

## Outcome

`common::best_index` added at `get/src/evolver/common.rs:47` as the explicit loop, beside `rank`;
both `outcome` methods call it. Removing the two local argmins left `std::cmp::Ordering` unused in
`generational.rs` and `rank` unused in `steady_state.rs` — both imports dropped, while `rank` stays
in `generational.rs` for the elite sort. Verified on Linux before and after merge: `cargo test -p
get` 213 passed / 0 failed (count unchanged), `cargo clippy -p get --all-targets -- -D warnings`
clean.

One deliberate departure from the issue text: `best_index` carries an `assert!` on the empty-slice
case. Steady-state's `.expect(..)` was the only guard on that path and generational had none, so
sharing the code meant choosing one behaviour. Flagged in PR #55's body for review rather than
slipped in; reasoning in `decisions.md` 2026-08-10 23:05.

Spec §6.2's deliberate divergence — generational `swap_remove`s the winner's graph, steady-state
re-expresses it — was left untouched, as was the rest of `outcome`.

## Left behind, still live after this task

- **GitHub #56** — the follow-up sweep of both evolver files for further divergent style and
  outright duplication, identified while scoping this task and filed rather than dropped. Contains
  `diff`-verified evidence (byte-identical `best_of`/`mean_of` helpers, the `ChaCha8Rng` comment,
  the `Self { .. }` tail of both `new` methods). Unassigned, and **deliberately staged behind the
  currently open issue set** — it is a cleanup pass over files several open issues still touch.
  Raised as `collab.md` **#43**, which also carries a question for the next joint meeting about
  whether other file pairs deserve the same treatment.
- **`hotfixes.md`** — `python_fitness`'s `#[allow(dead_code)]`, blocked on #26. Re-verified and
  stamped at this gate; untouched by this task, which never went near `lib.rs`.
- **`issues.md`** — the parked `sda.rs` private intra-doc-link warning. Pre-dates this task,
  re-confirmed still reproducing.
- **`collab.md` #40 and #41** — still awaiting James's acknowledgement, carried over from the
  previous task. Neither blocks anything.
