# History — `Genome::mutate` applies exactly one mutation; the engine owns both dice rolls (#10)

Append-only session log for this task, newest session first.
Maintained by `/save`; archived by `/done`.

---

## Session 2026-08-03 (second): #10 verified by fault injection and shipped as PR #30

Picked up from `handoff.md` with two `[~]` items — the helper and the config field, both built and
both untested. Both are now `[x]`, the branch is committed and pushed, and PR #30 is open.

### The tests, and why they are trusted

Four `mutate_child` tests in `get/src/evolver/common.rs`: rate 0.0 never mutates, rate 1.0 at max 1
mutates exactly once, counts at max 4 cover `1..=4` and never exceed, and `max_mutations = 0`
panics. Two config tests in `get/src/config.rs`: default-to-1 and explicit-4. **97 -> 103 tests.**

The fixture question took two rounds. The first proposal was a new `CountingGenome` stub; James
pushed back as overengineered and was right — checking showed the only `mutate` call in `common.rs`
is inside `mutate_child` itself, so no selection test runs an individual through a mutation path and
the existing `IndexGenome::mutate` could simply increment instead of being a no-op. One line rather
than twenty. Reasoning in `decisions.md` 2026-08-03 21:10.

Each test was then **fault-injected** rather than trusted for passing:

| Injected into | Result |
|---|---|
| `1..max_mutations.max(2)` (exclusive range) | count-bound test FAILED |
| `for _ in 0..=count` (extra pass) | exactly-one AND count-bound FAILED |
| `default_max_mutations() -> 2` | config default test FAILED |

All three reverted, each revert confirmed by reading `git diff` rather than assuming. Full suite
green at 103 afterwards. This is what took the two items from `[~]` to `[x]` — see `decisions.md`
2026-08-03 21:12.

### What else changed

- `config.example.toml:18` — `max_mutations` documented, including that one mutation is one gene for
  edge-edit and one transition for SDA, so equal count is not equal disruption.
- **Issue #24 commented** after the save, once James OK'd the body: `max_mutations` landed via #30,
  so #24 is now fitness variants plus dropping `seed`/`num_chars`. It also names the two things #30
  deliberately left to others — zero-rejection in `Config::validate` (#23) and runtime wiring (#26).
  Verified by re-reading; no labels passed.
  https://github.com/md12ol/GraphEvolutionTool/issues/24#issuecomment-5174100675
- `.claude/work/decisions.md:218` — the 2026-07-31 joint entry's "**Not yet implemented.**" struck
  through and superseded with a dated implementation line. It also now records that steady-state is
  the *only* caller until #25, since the original text says "both strategies" and a cold reader
  would take that as a claim about today's tree.
- **Formatting: nothing to do.** `cargo fmt -- --check` named only `generational.rs` and `sda.rs`
  (issue #22, Michael's). All six files this task touched, including the new tests, were already
  clean. No bare `cargo fmt` was run.

### Found in passing

- **`lib.rs` dispatch is entirely `todo!()`** — nothing maps `Config` onto engine types for *any*
  field, so `max_mutations` is plumbed type-to-type but not yet wired at runtime. Not a gap this
  task introduced; it is issue **#26**, James's. Flagged in the PR body so it is not read as an
  omission of #10.
- Four untracked pre-spec-sheet docs would be swept in by a bare `git add -A`. Now a trap.

### Coordination check before pushing

Fetched `origin` at James's request: no branch of Michael's exists, `feature/ga-engine` is James's
own from 2026-07-29, no open PRs, and #14/#15 were last touched 2026-07-31 — before this task
started. Michael has not begun, so the three-file overlap is not yet a real conflict. Pushed on
that basis.

James also asked whether `decisions.md` is meant to be pushed by both owners the way `collab.md` is.
Answered from `/.gitattributes`: the union-merge driver is a **glob** over `.claude/work/*.md`, so
mechanically they are identical — both owners append to the tail of both files. The difference is
urgency, not mechanism: `collab.md` is a mailbox and must land ahead of the work (which is why
collab 14 went to `main` first), while `decisions.md` is a record and correctly rides with the
branch whose code makes it true. James chose to push the joint-entry supersession as-is and let the
PR diff be Michael's notification.

### Git manifest

- Branch `jsargant_mutation_contract`, pushed and tracking `origin/jsargant_mutation_contract`.
- **`a8cbf27`** — 8 files, +272/-32. Author and committer James Sargant; no `Co-Authored-By`
  trailer, no generated-by footer. Verified after the fact, not assumed.
- **PR #30** -> `main`, open, "Closes #10". Body re-read via `gh pr view` and diffed against source:
  identical bar a trailing newline, all 13 table rows intact.
- Working tree clean except the four untracked stale docs, deliberately left in place.
- `origin/main` has nothing this branch lacks.

## Session 2026-08-03: Task opened and most of #10 implemented; 97 tests still green

**Started from an empty `work/current/`** — the steady-state task had been archived and no task was
open. Chose #10 after reading the tracker rather than inferring from the spec.

### Getting the tracker readable

`gh` was not installed on this machine, so the build order — which `decisions.md` 2026-07-31 says
lives in GitHub issues and deliberately not in a markdown file — was unreadable. James installed
`gh` 2.97.0 and authed as `shorinbonsai` (scopes `gist`, `read:org`, `repo`, `workflow`). This
changed the plan materially: the inferred next task was going to be #25 (generational), and the
tracker showed #25 depends on `express_and_score` (#14), which is **Michael's**, not James's.

Assignment split, read from the tracker: James has #10, #13, #23, #24, #25, #26, #27, #28, #29;
Michael has #6, #14–#22.

### Coordination, done before any code

James asked how a `collab.md` item on a feature branch would reach Michael before the branch lands.
It would not — that is the whole gap. So collab item 14 was written and pushed **to `main`**
(`347d081`, docs-only) *before* branching, on the reasoning that a coordination file only works if
it is pushed ahead of the work. Michael's own docs commits go straight to `main`, and `merge=union`
on `.claude/work/*.md` exists precisely so both owners can append to the tail without conflicts.

Collab item 14 warns that #10 touches `evolver/common.rs`, `evolver/mod.rs` and
`evolver/steady_state.rs` — the same three files Michael's #14 and #15 list. Source files, so union
merge does not apply. It also flags that #24 specifies the same `max_mutations` config field, which
#10 implements because #10 cannot be verified without it.

Feature branch: `jsargant_mutation_contract`, off the pushed `main`.

### Design point clarified with James

James described the two rolls and said "for generational algorithm there would be a separate dice
roll". Asked whether that meant the count roll was generational-**only** — which would contradict
spec §4's shared-helper requirement and need a joint meeting under the `CLAUDE.md` rule — or was
just context. Answer: **(a)**, context. Shared helper, both strategies. No spec change.

### What changed

- `get/src/genomes/genome.rs:25` — `mutate` doc rewritten as the one-mutation contract. The
  load-bearing edit; everything else is code catching up to it.
- `get/src/genomes/edge_edit.rs` — `const MAX_MUTATIONS: usize = 4` deleted; `mutate` reduced from
  a `1..=4` loop to a single gene reroll.
- `get/src/genomes/edge_edit.rs` — `mutation_replaces_at_most_four_genes_using_the_shared_mix`
  became `mutation_replaces_exactly_one_gene_using_the_shared_mix`, swept over 64 seeds. The
  sentinel-8 / opcode-3 setup means a reroll can never coincidentally read as unchanged.
- `get/src/evolver/common.rs:125` — new `mutate_child`, holding both rolls.
- `get/src/evolver/mod.rs` — `SharedEvolutionContext.max_mutations`, documented as one knob with
  `mutation_rate`.
- `get/src/evolver/steady_state.rs` — `mating_event` calls the helper instead of rolling inline;
  four test construction sites gained `max_mutations: 1`.
- `get/src/config.rs` — top-level `max_mutations`, `#[serde(default = "default_max_mutations")]`
  to 1.
- `~/.claude/CLAUDE.md` — created. Global, outside this repo: no agent co-attribution on James's
  commits or PRs.

### Measured, not assumed

**`random_range(1..=1)` consumes RNG state.** Probed directly with a throwaway test on
`ChaCha8Rng` (written, run, removed in the same session): the generator's next output differs
depending on whether the singleton-range draw happened. A doc comment on `mutate_child` had claimed
the opposite — that `max_mutations = 1` would preserve the pre-change RNG stream — and was corrected
rather than left. This confirms the "every seeded run changes output" consequence already recorded
on 2026-07-31, and the decision not to special-case 1 is now recorded with the measurement.

**97 tests pass, identical to the pre-task baseline.** That is the evidence the refactor is
behaviour-preserving at `max_mutations = 1`: nothing in steady-state's suite hardcodes a
seed-derived value, so the moved RNG stream does not show up there.

### Tracker

**Commented on #23** (`Config::from_path` / `Config::validate`, James's, not started) recording that
`Config::validate` must reject `max_mutations = 0`, and that `common::mutate_child` already asserts
it as a backstop that fires at the first mating event rather than at startup. Confirmed intact by
re-reading with `gh issue view 23 --comments` — the exit code alone is not proof, per `CLAUDE.md`.
https://github.com/md12ol/GraphEvolutionTool/issues/23#issuecomment-5172792232

### Not done

The helper has no unit tests of its own, `config.example.toml` was not updated, there is no config
test for the new field, `decisions.md`'s 2026-07-31 "Not yet implemented" marker still stands, the
six touched source files are unformatted, and no PR exists.

### Git manifest

- Repo `GraphEvolutionTool`, branch **`jsargant_mutation_contract`**, **no commits ahead of
  `origin/main`** — everything below is uncommitted in the working tree.
- **Seven modified files, not six.** The six source files under `get/src/`, **plus
  `.claude/work/decisions.md`**, which is tracked and therefore lives on this branch rather than on
  `main`. Its two new entries reach Michael only when this branch lands; if the branch is ever
  abandoned, they go with it. See `traps.md`, "The `.claude/` docs split across branches".
- `origin/main` carries `347d081` (collab item 14), pushed this session — the only thing from this
  session that has reached anyone else.
- Untracked and untouched by this task, pre-existing: `GET GA planning session.md`, `docs/`.
- `.claude/work/current/` is gitignored, so this plan, history and handoff are local only.

*Session logged 2026-08-03 18:45, amended 19:05 — James, after posting the #23 comment.*
</content>
