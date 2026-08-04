# History — Implement sir_sim: one epidemic returning length, spread and profile (issue #16)

Append-only session log for this task, newest session first.
Maintained by `/save`; archived by `/done`.

---

## Session 2026-08-04: sir_sim implemented, PR #31 open, three collab items raised

**What shipped.** `get/src/sir.rs` — `SirParams`, `SirRun`, `sir_sim` — plus `pub mod sir;` at
`get/src/lib.rs:6`. Mechanics ported from `legacy/Graph.cpp` `Graph::SIR`; reporting per spec §5.2.
Seven tests. Comment-only change to `get/src/fitness.rs:96-107`.

**Validated on this machine, 2026-08-04.** `cargo test` → 104 passed, 0 failed (97 prior + 7 new).
`cargo clippy --all-targets` → zero hits on `sir.rs`; three `needless_range_loop` warnings were
found and fixed by iterating rather than indexing. `rustfmt --edition 2024 get/src/sir.rs` only —
`cargo fmt -- --check` still shows the pre-existing `generational.rs` / `sda.rs` offenders and no
new ones. Union-merge audit clean on `collab.md`, `issues.md`, `decisions.md`.

**Nothing is unverified.** No `[~]` items.

**Two bugs of my own, caught and fixed.** A test oracle asserted `spread` varies with patient zero
on a fully-connected path at rate 1.0, where it cannot — `length` is what varies; fixed the oracle,
not the code. And the `collab.md` entry I wrote closed a fenced code block with a bare ` ``` `,
colliding with the fence in the file's own header — the audit caught it, converted to an indented
block.

**Findings that outlived the task.** Checking `legacy/main.cpp` turned up that every fitness draw
re-rolls outbreaks shorter than `mepl = 3`, up to `rse = 5` attempts — recorded in neither the sheet
nor any issue. Raised as `collab.md` #17. The newer `legacy/Graph.cpp` then confirmed `spread`
matches ours exactly (`totInf`) while `length` and the trailing zero still differ, which narrowed
`collab.md` #15. Also probed and confirmed that `Fitness` is object-safe while `Genome` is not
(`mutate` is generic over the RNG), which is the fact `collab.md` #16 turns on.

**Git manifest.** Branch `mdube_sir_sim`. `28b34d6` sir_sim · `4e15939` PR-merge rule ·
`7f814a1` collab 16 · `b898b50` collab 17 · `0212b45` legacy/ tracked · `94ab70e` legacy headers ·
`32b7839` newer Graph.cpp + staged follow-up · `56b9563` decisions.

**CORRECTION, same session.** The first draft of this entry said PR #31 was open. It was not —
**James merged it at 2026-08-04 12:38 UTC** (`4c85cd0`, 7 commits, 13 files), roughly 2.5 hours
before this save ran, and the save wrote its manifest from local state without re-checking the
remote. Two things follow, both now tasks in `plan.md`:

- **Issue #16 is still open.** `Closes #16` was added to the PR body at ~15:05 UTC, after the merge.
  GitHub applies closing keywords only at merge time, so it never fired. Needs closing by hand.
- **`56b9563` is stranded.** The four `decisions.md` entries were pushed after the merge, so they
  are on `mdube_sir_sim` and not on `main` — confirmed by
  `git show origin/main:.claude/work/decisions.md | grep -c 2026-08-04` returning 0.

**Both resolved before the session ended.** PR #32 carried the stranded `decisions.md` and
`traps.md` commits plus the `collab.md` #18 trace, and was **self-merged unreviewed** under the
exception in the rule PR #31 had just delivered — docs-only, and the union-merge audit was run by
hand in place of review. `mdube_sir_sim` deleted local and remote; working tree on `main`, clean,
104 tests passing. Issue #16 closed by hand at 15:42 UTC with a comment recording that `length` and
the trailing zero are still contested and that short-epidemic re-rolls are unowned. Nothing from
this task remains on a branch.

**Tooling.** `gh issue view` and `gh pr edit` both fail on this repo with a Projects-classic GraphQL
error — it is the whole default view, not one command. Reads via `--json`, writes via
`gh api ... -X PATCH -F body=@file`. Recorded in `CLAUDE.md`.

### Later the same session, beyond #16's objective

**Two union-merge failures measured, neither previously recorded.** (1) GitHub's web merge does not
apply `.gitattributes` merge drivers, not even `union` — verified three ways against PR #30: clean
locally with the file present, `CONFLICT` locally without it, `mergeable=false` on GitHub's API.
(2) Union silently *duplicates* a line when both sides edit the same one, reported as
`1 insertion(+)` with no conflict, on a 250-line file so not a small-file artifact. Both in
`traps.md`; `CLAUDE.md` gained "merge locally when the PR touches `.claude/work/*.md`" and a fifth
union-formatting rule. Raised as `collab.md` #19.

**A correction that reached a colleague's work.** I had called James's in-place amendment of the
jointly-stamped `decisions.md` entry "luck as much as care". The experiments showed that was wrong:
authorship is irrelevant to union safety, and the hazard is *concurrent* edits to one line — nobody
was editing his, so it was safe by construction. Corrected in `collab.md` #19 and the commit message
rather than left standing.

**Routing rule set by Michael:** code solving an issue (`get/src/`, `Cargo.toml`,
`config.example.toml`) goes through a feature branch and a PR; `.claude/work/*.md` may be pushed
direct, because a trap not on `main` protects nobody. In `CLAUDE.md` under "Pull requests".

**PR #30 reviewed and merged.** Checked clause by clause against spec §4 — both rolls in
`common::mutate_child`, `Genome::mutate` applies exactly one, edge-edit rerolls one gene, SDA's
`INIT_CHAR_MUTATION_RATE = 0.04` with an early `return` making it genuinely either/or. Two non-
blocking notes given: `generational.rs:24`'s doc still describes mutation without pointing at
`mutate_child`, which is the drift #25 could reintroduce; and the `IndexGenome` test stub now uses
one field as both slot index and mutation counter. Noted that #24's `#[serde(default)]` for
`max_mutations` is wrong — it yields 0 on a `usize` and `mutate_child` asserts on 0 — and James
correctly used `default = "default_max_mutations"` instead. Merged locally at 79f7948: 110 tests,
docs audit clean, both sides' entries intact. Issue #10 auto-closed.

**Git manifest, end of session.** On `main`, clean, nothing ahead of origin. `5e38b55` traps + rule,
`79f7948` merge of #30. James's branch `jsargant_mutation_contract` still exists on the remote — his
to delete, not mine.
