# Plan — `Genome::mutate` applies exactly one mutation; the engine owns both dice rolls (#10)
_Started 2026-08-03 · last updated 2026-08-03_

## Objective

Make `Genome::mutate` a trait-level contract meaning **exactly one mutation per call**, and move
both mutation dice rolls — `mutation_rate` (whether) and a new `max_mutations` (how many, uniform
`1..=max`) — out of the genomes and into one shared helper in `evolver/common.rs` that both
evolution strategies call.

Closes GitHub **#10**. Spec §4; `decisions.md` 2026-07-31 "`Genome::mutate` applies exactly one
mutation".

**Done looks like:** `MAX_MUTATIONS` is gone from `edge_edit.rs`, both genomes apply one mutation
per call, `SharedEvolutionContext` carries `max_mutations`, steady-state mutates through the shared
helper, `config.toml` accepts `max_mutations` defaulting to 1, and the suite is green with a test
that fails if a genome mutates more than once.

**Out of scope** — see the section at the bottom. In particular this task does **not** implement
generational (#25), and does **not** touch `express_and_score` or the orientation move (issues #14
and #15, assigned to Michael).

## Tasks

- [x] **Raise the file-collision item as `collab.md` item 14, on `main`, and push it** — done
      2026-08-03. Verified: `347d081` on `origin/main`, docs-only, union-merge audit clean.
      **Note:** collab item 14 is not GitHub issue #14 — numbering is `collab.md`'s own and the
      collision is coincidental. Say "collab 14" or "issue #14", never a bare "#14", in this task.

- [x] **Branch for the code work** — done 2026-08-03. Verified: on `jsargant_mutation_contract`,
      branched off the pushed `main`. PR at the end.

- [x] **State the one-mutation contract on the trait** — done 2026-08-03,
      `get/src/genomes/genome.rs:25`. Verified: doc states "exactly one", names the engine as owner
      of both rolls, and records that count is shared but strength is not.

- [x] **Reduce `EdgeEditGenome::mutate` to a single gene reroll** — done 2026-08-03. Verified:
      `grep -rn MAX_MUTATIONS get/src/` returns only the doc comment explaining its removal. SDA was
      already compliant and was not touched.

- [x] **Rewrite the edge-edit mutation test** — done 2026-08-03, now
      `mutation_replaces_exactly_one_gene_using_the_shared_mix`, swept over 64 seeds. Verified: 97
      tests green. Original wording in `plan_superseded.md`.

- [x] **Add the shared mutation helper** — `common::mutate_child` (`common.rs:155`), with four unit
      tests. Verified 2026-08-03: 101 tests green (was 97), and fault injection confirms they bite —
      an exclusive count range fails the bound test, an extra loop pass fails the exactly-one test.

- [x] **`SharedEvolutionContext` gains `max_mutations`** — done 2026-08-03, `mod.rs:44` area.
      Verified: `cargo build` clean and all four `steady_state.rs` test sites updated.

- [x] **Steady-state mutates through the helper** — done 2026-08-03, `steady_state.rs` `mating_event`.
      Verified: 97 tests green, unchanged from baseline, so the refactor is behaviour-preserving at
      `max_mutations = 1`.

- [x] **Config: top-level `max_mutations`, `#[serde(default)]` to 1** — field at `config.rs:41`,
      documented in `config.example.toml:18`. Verified 2026-08-03: default-to-1 and explicit-4 tests
      pass, the default test fails if the default is changed to 2, and the example still parses.

- [x] **Record the behaviour change** — done 2026-08-03, `decisions.md:218`. Verified: the
      2026-07-31 entry's "Not yet implemented" is struck through with a dated supersession line, and
      the duplicate-line audit is clean. The seeded-output consequence is already in the 18:45 entry.

- [~] **PR, and close issue #10** — PR #30 open 2026-08-03, `a8cbf27`, "Closes #10".
      Verified: author and committer are James Sargant, no `Co-Authored-By` trailer, no generated-by
      footer, body re-read intact. **Unverified: #10 closes only on merge** (unverified since
      2026-08-03). Awaiting Michael's review; nothing to do until then.

- [x] **Comment on issue #24 that `max_mutations` is already built** — done 2026-08-03. Verified by
      re-reading: body identical to source, 6 list items intact, no labels added, issue still open.
      https://github.com/md12ol/GraphEvolutionTool/issues/24#issuecomment-5174100675

- [x] **Format the files this task touched** — no action needed; all six were already clean.
      Verified 2026-08-03: `cargo fmt -- --check` names only `generational.rs` and `sda.rs`, which
      are issue #22 and stay untouched. No bare `cargo fmt` was run (`traps.md`).

- [x] **Comment on issue #23** — done 2026-08-03: `Config::validate` must reject
      `max_mutations = 0`; `common::mutate_child` asserts it as a backstop. Verified by re-reading
      with `gh issue view 23 --comments` — body intact.
      https://github.com/md12ol/GraphEvolutionTool/issues/23#issuecomment-5172792232

## Open questions

*Both original questions are settled — see `decisions.md` 2026-08-03 18:45. The helper is a
standalone free function, and `max_mutations = 0` panics in the helper with real rejection deferred
to `Config::validate` (issue #23).*

## Out of scope

- **Issue #25, generational evolver** — the next task, and the reason this one exists. It also needs
  `express_and_score` (#14), which is Michael's.
- **Issues #14 and #15** — `express_and_score` rename, orientation at the boundary. Assigned to
  md12ol. This task's helper calls `common::evaluate` under its current name; Michael's rename
  sweeps it.
- **Issue #24, config schema** (fitness variants, drop `seed` and `num_chars`) — mine, later. Only
  the `max_mutations` field is pulled forward, because #10 cannot be verified without it.
- **Issue #22, tree-wide `cargo fmt`** — Michael's, and it lands when this tree is clean. Format
  only files touched here, per `traps.md`: `rustfmt --edition 2024 <file>`.
</content>
