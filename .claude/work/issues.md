# Issues — work not yet in the tracker

**7 issues are open in the tracker** at `md12ol/GraphEvolutionTool` — that is the source of
truth for anything filed. `gh issue list` is the way to read them; they are deliberately **not**
mirrored here, because a second copy drifts and this file would become a private fork of the
tracker.

This file holds only what is **not filed yet**. Two tiers; the difference is whether it has been
root-caused. An entry leaves the moment it is filed — replaced by nothing, since the tracker then
carries it.

Maintained by `/save`. `/done` lists anything still `Filed: not yet` before archiving a task.
How issues get filed — tool, confirmation rule, target project — lives in `CLAUDE.md`.

*Reset 2026-07-31 23:35 — Michael: the one filed entry was removed once #22 existed, per the sync
obligation in `CLAUDE.md`.*

*Folded in 2026-08-04 19:00 — Michael: the two PR #30 review findings moved into **#25**, whose
implementer touches both `generational.rs` and the `common.rs` test module. Neither warranted a
standalone chore.*

*Synced 2026-08-04 18:55 — Michael: "Align sir_sim's length and profile" removed once it was filed
as **#34** (assigned to md12ol), same obligation. Open count refreshed from the tracker.*

*Synced 2026-08-10 — James: the `evaluate_population`/`SirRun` rename removed, filed as **#52**
(assigned to md12ol) once the 2026-08-09 meeting unblocked it — `collab_settled.md` #32. Its scope table
was already a stale fork: #52 found occurrences in `lib.rs` and `generational.rs`, and sheet line
269, that the staged entry did not list. Open count corrected 17 → 8, counted from
`gh issue list --state open --json number -q 'length'`.*

*Synced 2026-08-10 22:55 — Michael: no staged entry involved this cycle — #51 closed by PR #55's
merge, and its follow-up sweep went straight to the tracker as **#56** (unassigned) rather than
being staged here first, since it was already root-caused while scoping #51. Open count 8 → 7,
recounted the same way.*

*Synced 2026-08-11 17:20 — Michael: `run`'s stale #26-era doc comment removed from here once it
was filed as **#61** (`(1)`, assigned to md12ol), per the sync obligation. It was staged and filed
in the same `/done run-dispatch` gate rather than carried forward, since the text it corrects went
false the moment PR #60 merged. Open count **6 → 7**, counted from
`gh issue list --state open --json number -q 'length'`. The header's "7" above was stale, not
stable: #26 closing had already taken the true count to 6, and filing #61 has now put it back to 7
by coincidence. Recount, never carry the number forward.*

*Withdrawn 2026-08-10 18:38 — James: I staged `best_index`'s `cargo fmt` failure here and Michael
had already put the same finding on **#56** as a comment, reviewing PR #57 in the same hour. Two
people root-caused it independently within minutes, which the staging area cannot detect — the
tracker is the only place that can. Removed under the sync obligation rather than left as a
private second copy, and his diagnosis supersedes mine on the mechanism: it is rustfmt's
`fn_call_width` of 60, not the line's overall length, which my entry gave wrongly as 84 characters.
Open count still 7, recounted the same way; **#58** is assigned to me and not staged here because
it was filed directly.*

---

## Parked — noticed, not investigated

### <what was noticed>
- **Where:** `path` or component, as far as it's known.
- **Impact:** why it matters — who or what it breaks.
- **Noticed:** <YYYY-MM-DD>, in <what you were doing when you hit it>


### Three probabilities are unvalidated — negatives and values above 1 parse and run

- **Where:** `get/src/config.rs` — `crossover_rate` and `mutation_rate` at the top level,
  `infection_rate` on `SirParams`. None of the four `validate_*` helpers touches them.
- **Impact:** a `mutation_rate = 2.0` or a negative `infection_rate` is accepted silently and the
  run produces numbers that look plausible. It is the first thing a new user gets wrong, and there
  is no error to lead them back. Suggested constraint is `0.0 <= v <= 1.0` on all three.
- **Blocked on the sheet, not on effort.** §7's constraint list omits them too, so this is a gap in
  the design before it is one in the code — fixing `config.rs` alone would make the code stricter
  than the sheet it is built to. Raised as `collab_settled.md` #51 item 3, for a joint meeting; parked here
  so it survives if that meeting slips. Once §7 is amended this is a small, self-contained change.
- **Noticed:** 2026-08-12, surveying `config.rs` to write the documentation site's config reference.

### `cargo doc` warns twice on a private intra-doc link in `sda.rs`

- **Where:** `get/src/genomes/sda.rs`, the doc comment referencing [`INIT_CHAR_MUTATION_RATE`].
- **Impact:** cosmetic only — `cargo doc -p get --no-deps` emits "this item is private ... this link
  will resolve properly if you pass `--document-private-items`", twice. No gate covers `cargo doc`,
  so nothing fails; it just means the rendered docs carry a broken link. Whoever owns `sda.rs` can
  either make the constant public or write it as plain text rather than a link.
- **Noticed:** 2026-08-08, checking that #29's new memory-note table in `run`'s docstring rendered.
  Confirmed **pre-existing** by stashing the #29 changes and re-running — same two warnings, so it
  is not something #29 introduced.


### `config.example.toml`'s epidemic parameters give the search nothing to climb

- **Where:** `config.example.toml`, the active `[fitness]` block — `infection_rate = 0.05` with
  `network_size = 100` and an edge-edit genome.
- **Impact:** the shipped example runs 500 generations and does not improve. Measured 2026-08-11 on
  `mdube_run_dispatch` once `run` worked: best fitness at 0 / 5 / 50 / 300 generations is
  1.20 / 1.30 / 1.27 / 1.20, while the edge count climbs 74 → 116. At that infection rate an
  outbreak on a sparse graph dies almost immediately whatever the topology, so every individual
  scores about the same and selection has no gradient. **The engine is fine** — the same
  configuration at `infection_rate = 0.5` climbs 11.5 → 22 → 52 → 71.5 across 0 / 10 / 100 / 400
  generations. So this is a documentation and example-parameter problem, not a defect.
- **Why it matters more than it looks:** this is the first thing a new user runs, and the honest
  reading of a flat 500-generation run is "the tool does not work". It also gives the misleading
  impression that `epi_spread` is a weak objective.
- **Not investigated:** what the example *should* say. Raising the rate makes the example converge
  but stops it resembling a realistic epidemic; keeping it needs a comment saying the run is
  deliberately hard and why. That choice is `collab_settled.md` #48, parked behind the current issue set.
- **Noticed:** 2026-08-11, verifying #26's dispatch end to end rather than trusting the four arms
  to compile.

## Ready to file — root-caused and evidenced

### <title — imperative, issue-ready>
- **For:** teammate / team / unassigned
- **Project:** the tracker project it belongs to
- **Filed:** not yet
- **Component:** `path:line`
- **Body:** <open with a sentence on this line — a bare label is identical in every entry, which
  is what union merge folds together. See CLAUDE.md, "Formatting for union merge".>
  What's wrong, the mechanism with `path:line`, evidence, how to reproduce, candidate fixes.

