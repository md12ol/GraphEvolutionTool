# History — Implement the steady-state evolver

Append-only session log for this task, newest session first.
Maintained by `/save`; archived by `/done`.

---

## Session 2026-07-31 (later): Joint design call — `official_spec_sheet.md` written

**No code changed.** A working call with James, auditing `IMPLEMENTATION.md` against the tree and
replacing it with a design document. Verified state at start: 97 tests pass, 8 `todo!()`s remain
(`lib.rs` ×3, `fitness.rs` ×2, `config.rs` ×1, `generational.rs` ×2), branch `main`, tree clean.

### What the audit found

`IMPLEMENTATION.md` was accurate on architecture and stale on behaviour. Three contradictions,
all introduced by the steady-state task and never back-propagated:

- §2.1 stated `evaluate` does **not** orient — it does, and that is the whole design.
- §2.4 described steady-state as breeding **one** child and replacing the **global** worst.
- §2.3's `total_cmp` rationale said a `NaN` "should not panic the run"; policy is the reverse.

It also had no `Direction` concept at all, and its §6 recommended objectives negate themselves —
the approach explicitly rejected on 2026-07-31. Separately: `IMPLEMENTATION.md` is **untracked**
and had never reached James.

### What was produced

`/official_spec_sheet.md`, 822 lines, ten sections — pipeline and the three `Send`/`Sync` bounds,
`Graph`, both genomes, the mutation contract, fitness with a units diagram, the SIR simulator and
three objectives, both evolvers, config and validation, the Python interface with replicate runs,
non-goals. Design only; no sequencing, by agreement. §9 "Open decisions" reads **none**.

Twelve decisions appended to `decisions.md`, all stamped "Michael & James". The substantive ones:
the one-mutation trait contract with `max_mutations`; the SDA alphabet derived from the edge cap;
direction fixed per objective; the engine held in one orientation with conversion only at the
Python boundary; one SIR simulator behind three objectives; the atomic-counter epidemic seeding;
one master seed from the `run` call; replicates parallel only under a Rust objective; and the
Python-builds-TOML pipeline with a single `Config::validate`.

### Files touched

- `/official_spec_sheet.md` — **new, untracked**
- `.claude/CLAUDE.md` — new opening section making the spec sheet the design authority, and
  inverting "the repo wins" for it
- `.claude/work/issues.md` — one issue staged for Michael (format + readability pass)
- `.claude/work/decisions.md` — twelve entries
- `.claude/work/collab.md` — call outcomes; items 2,3,4,5,6,9,10,12 settled, **#11 reversed**
- `.claude/work/traps.md` — amended the `merge=union` trap: the attribute is a glob

### Then: committed, pushed, and filed the implementation work as issues

Two commits pushed to `main`:
- `c3c4226` — `official_spec_sheet.md` plus the working docs
- `d586358` — crossover recorded as a planned extension; the filing decision

**16 issues filed, #14–#29**, each with a time estimate and an assignee, derived from the spec and
the tree. Split by prior authorship rather than by even hours: Michael takes `common.rs`, `sda.rs`,
`steady_state.rs`, `fitness.rs` (#14–#22, ~38h); James takes `edge_edit.rs`, `config.rs`,
`generational.rs`, `lib.rs` (#23–#29, ~36h).

**Three existing issues were enriched, not duplicated.** #10 already described the mutation
contract nearly word for word — *"If we want more than one mutation the GA should call multiple
times."* #6 (SDA edge-multiplicity cap) and #13 (config away from TOML) also overlapped. Filing
fresh copies would have had one of us do the work twice. #10 and #13 gained James as assignee.

**Gap found while filing:** #7/#8/#9 want configurable crossover types, which the spec described
nowhere — it presents one fixed operator per genome. Rather than leave the two records disagreeing,
§4 now records crossover as fixed today with selectable operators as a planned extension, pointing
at `Selection`'s enum as the shape to follow.

**`gh` quirk, cost real care:** `gh issue edit --body-file` replaces the **entire** body. The
original one-line bodies of #6/#10/#13 were preserved by hand above a `---`. Anyone editing them
again must do the same or the author's words vanish with no warning.

### Git manifest

Branch `main`, in sync with origin at `d586358`. Working tree clean apart from
`IMPLEMENTATION.md`, still untracked and deliberately not deleted. `.claude/work/current/` is
gitignored, so `plan.md`, `history.md` and `handoff.md` exist on this machine only — by design.

### Not done

The spec records design the code does not implement — `max_mutations`, the derived SDA alphabet,
`express_and_score`, `Config::validate`, the whole fitness and Python layer. Nothing was
implemented deliberately; this session was design only.

`/done` was **not** run. `.claude/work/archive/` is still empty and the finished steady-state plan
is still in `work/current/`, with a note at its head explaining why. That is the next session's
first action.

---

## Session 2026-07-31: Steady-state evolver implemented end to end

**Objective met.** `SteadyStateEvolver` runs. `get/src/evolver/common.rs`,
`get/src/evolver/steady_state.rs` and `get/src/fitness.rs` have no `todo!()`s.
Test count 56 -> 97, all passing.

### What changed

- `common.rs` — `Selection::select` (N tournaments, with replacement),
  `Selection::tournament_indices` (one tournament, without replacement, best first),
  `rank` as the single comparator, `evaluate` (parallel express + batch score + orient),
  `generation_stats` (best/mean converted back, `std_dev` deliberately not).
- `fitness.rs` — `Direction` enum, `Direction::orient` (converts both ways, asserts non-`NaN`),
  `Fitness::direction()` defaulting to `Minimize`.
- `steady_state.rs` — `mating_event` (tournament-local, two children replace the tournament's two
  worst), `evolve` (event loop + log cadence), `outcome` (best selection + one final expression),
  `run` as four named steps. `MIN_TOURNAMENT_SIZE = 4` and `population >= tournament_size`
  asserted in `new`.

### Validated

- 97 tests pass; `cargo fmt`-clean and clippy-clean on the files touched.
- **Mutation-tested, not just green.** Inverting the selection comparator fails 3 tests; pointing
  replacement at the tournament's best fails 4; a no-op `evolve` fails 2; `log_interval = 1` fails
  the cadence test.
- Two vacuous tests were caught this way and fixed:
  `the_tournaments_best_is_never_replaced` ran at 0.5 rates where neither operator fired for its
  seed, so the child was a clone and overwriting the best slot was invisible — both rates pinned to
  1.0. And `a_run_actually_improves_the_population` was added after discovering the other 12 run
  tests all passed against an engine that never bred a child.

### NOT validated

- Nothing runs end to end. 8 `todo!()`s remain: `SirFitness` (2), `Config::from_path`,
  `GraphEvolver::run`/`save_logs`/`save_results`, generational (2). All were out of scope at
  `/start`, but "the steady-state evolver is finished" and "GET runs" are different claims.
- No Python-side exercise of the pyclass.

### Incident

Mid-`/save` the repo was checked out to `main`, and seven `decisions.md` entries were written onto
`main`'s copy, which lacked the two already committed on the feature branch. Caught by a file-state
reminder showing `steady_state.rs` back at its stub. Nothing was lost — all 6 commits were safe
locally and on origin. Entries were rescued to the scratchpad, `main` restored with `git restore`,
and re-applied on the feature branch. Recorded in `traps.md`.

### Merged to main (2026-07-31, after the first save)

Merged twice, by two routes. The user opened and merged **PR #12** on GitHub from the branch at
`858af92` while a local `git merge` of the same branch was in progress, so the local merge commit
was redundant and was discarded with `git reset --hard origin/main` — nothing was lost, since
`031fb7d` was already on the origin feature branch. PR #12 predated `031fb7d`, so `origin/main` was
missing the seven `decisions.md` entries and three `traps.md` entries; `b466e4e` merges just that
commit on top. No code differences between the two routes.

Lesson recorded: check `git log origin/main..HEAD` before merging locally, in case the same branch
has already been merged upstream through the tracker.

### Git manifest (end of session)

- Repo: `/home/mdube/GraphEvolutionTool`, branch **`main`**, in sync with origin at `b466e4e`.
  Working tree clean; nothing uncommitted.
- `main` carries the whole task: PR #11 (James's stubs), PR #12 (the steady-state evolver), and
  `b466e4e` (the working-docs commit PR #12 was branched before).
- `mdube_steady_state_implementation` is merged and still exists locally and on origin, as does
  James's `feature/ga-engine` from PR #11. Neither has been deleted.
- `work/current/plan.md`, `history.md` and `handoff.md` are gitignored, so they are not in any
  commit.

---
