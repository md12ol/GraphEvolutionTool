# `documentation/` — James's pending-edits queue

**Do not edit the site during an ordinary task. File the edit here instead.**

Created 2026-08-13 by Michael, on James's behalf, ahead of the ask in `collab.md` #53 — so the
convention was usable the moment James said yes, and so a session working under his identity had
somewhere to file instead of guessing.

**Agreed by James 2026-08-13**, at the joint meeting; the item is in `collab_settled.md`. The
rejection route described here originally — deleting the file — is spent, and the queue is now the
standing convention for both owners.

The companion file is `documentation/mdube_edits.md`, which carries the same rules and the same
format. Neither is the master copy; if the two ever disagree about *process*, `collab.md` #53 and
whatever it settles into is the authority.

## The rule

When a task changes something the site describes — a signature, a returned type, a name, a claim
about what does or does not exist yet — the task **does not** open the HTML. It appends an entry
below saying what is now wrong and what it should say. The site is then corrected in **one sweep**,
as its own task, when the owner says so.

**This is an explicit standing instruction to whoever is working, agent or owner: keeping this file
up to date is part of finishing a task, not optional tidying.** A task that changes the code and
files nothing here has left the site quietly lying, which is the exact failure the `badge-planned`
convention was built to prevent. The obligation moved; it did not go away.

**It replaces `CLAUDE.md`'s "de-badge its documentation in the same PR" rule** on *timing* only. The
de-badging still has to happen — badge, `.plan-note` callout, and the `status.html` row — just in the
sweep rather than in the shipping PR. That bullet was struck through in `CLAUDE.md` on 2026-08-13
(`a73af39`) once both owners agreed, so the two documents no longer disagree.

## Which file — check, do not assume

There is one queue **per owner**, because this is a churn list: an entry is *deleted* once the sweep
applies it, and `CLAUDE.md` already establishes that deletions are exactly what a union merge cannot
express. Separate files mean neither owner ever touches the other's, so no merge can silently
resurrect an applied entry.

Decide by identity, not by memory:

```bash
git config user.email
```

| Email | File |
|---|---|
| `shorinbonsai@gmail.com` | `documentation/jsargant_edits.md` — this file |
| `mdube04@uoguelph.ca` · `michael.dube@ovgu.de` · `35709889+md12ol@users.noreply.github.com` | `documentation/mdube_edits.md` |

**Anything else: stop and ask.** Do not pick the likelier one. Filing into the wrong owner's queue is
silent — the entry is neither lost nor found, and it surfaces only when someone sweeps a file they
did not expect to have work in it.

**A sweep reads every queue file, not just its own.** One page can be owed edits by both owners, and
applying half of them leaves the page wrong in a way that looks deliberate.

## Filing an entry

One `##` heading per edit, so two sessions appending concurrently cannot collapse into each other.
Say **where**, **what is now false**, and **what it should say** — enough that the sweep does not
have to re-derive it from the code.

```markdown
## <YYYY-MM-DD HH:MM> — <author> — <short title>

- **Trigger:** what shipped, and the issue number.
- **Files:** the pages and rough locations.
- **Now false:** the claim the site currently makes.
- **Should say:** what replaces it.
- **Badges:** any `badge-planned` span, `.plan-note` callout or `status.html` row to remove.
```

Delete an entry when the sweep has applied it — this is a queue, not a log. What was changed and why
belongs in `decisions.md`; this file only carries work that has not been done yet.

## Pending

## 2026-08-13 12:45 — James — `set_base_graph` exists now, so every "not yet" claim about it is false

- **Trigger:** GitHub #28, branch `jsargant_set_base_graph`, commits `3af041c` (the setter and its
  three checks) and `c99fa11` (threading it through `dispatch::evolve` into `edge_edit_start`).
  Shipped as `GraphEvolver.set_base_graph(num_nodes, edges)`, matching the signature the site
  already documents.
- **Files:** `guide/python-api.html` (the `#base-graph` section, ~L317-340); `examples/index.html`
  (the stacking example's `.plan-note`, ~L314-317); `status.html` (the "Supplying a base graph"
  row, ~L115); `HANDOFF.md` (the mirror row at ~L82).
- **Now false:** `examples/index.html` says "**Today:** there is no `set_base_graph`, so an
  edge-edit run always starts from an empty graph and stacking is not yet possible." Stacking is
  possible now, and the example above that note runs as written. `status.html` and `HANDOFF.md`
  both still list the feature as planned.
- **Should say:** the setter exists and takes `(num_nodes, edges)` with `edges` as
  `(u, v, multiplicity)` triples — the same shape `run` returns as `best_edges`, so an SDA run's
  output feeds an edge-edit run with no reshaping. Unset means an empty base graph, which is the
  default, and five of the nine opcodes (`Swap`, `Hop`, the three `Local*`) are inert on one until
  `Add`/`Toggle` build structure — self-correcting, not a defect.
- **Badges:** `guide/python-api.html` — drop `badge-planned` from the `#base-graph` heading (L317).
  `examples/index.html` — delete the `.plan-note` at ~L314-317 entirely. `status.html` — delete the
  "Supplying a base graph" row (~L115). `HANDOFF.md`'s row is the duplicate `collab.md` #57 raised;
  leave it to whatever #50 settles rather than patching the same table twice.

*#set-base-graph-ships · filed 2026-08-13 12:45 — James.*

## 2026-08-13 12:45 — James — cap narrowing raises now; the site still says it silently collapses

- **Trigger:** GitHub #28, commit `3af041c`. The decision is `decisions.md` 2026-08-12 — the
  cap-narrowing check **rejects** with `ValueError` rather than warning or clamping.
- **Files:** `guide/python-api.html` (the three-checks list under `#base-graph`);
  `examples/index.html` (the `.warn` block above the stacking plan-note, ~L307-313).
- **Now false:** `python-api.html` says cap narrowing "must be rejected **or warned**", which was
  the open question and is now settled. `examples/index.html`'s warning says setting edges
  "**clamps** rather than rejecting, so piping a cap-3 result into a cap-1 run silently collapses
  every weight to 1 and you get a plausible-looking network" — that is still true of
  `Graph::set_edge` itself, but no longer of the path a user can reach: `set_base_graph` checks
  every multiplicity before building anything and raises.
- **Should say:** `set_base_graph` raises `ValueError` naming the offending edge, its multiplicity
  and the configured cap, so the silent collapse is not reachable through the Python API. The
  advice to keep `max_edge_multiplicity` identical or raise it stands — it just fails loudly now
  instead of quietly. Worth keeping a sentence that `Graph::set_edge` still clamps, since that is
  why the setter has to check at all, and it is what a Rust-side embedder still faces.
- **Badges:** none — neither location carries a `badge-planned` span or a `.plan-note`. This is a
  correctness fix to prose, not a de-badging, and it is separable from `#set-base-graph-ships`.

*#set-base-graph-cap-rejects · filed 2026-08-13 12:45 — James.*

## 2026-08-13 14:41 — James — the setter owes five checks now, not three, and two of them are new

- **Trigger:** the joint meeting of 2026-08-13 settled `collab.md` #61 — `decisions.md` 20:16,
  "Caller-supplied graph data is rejected, not silently dropped". `set_base_graph` now rejects an
  out-of-range endpoint and rejects a self-loop, each raising `ValueError` naming the offending
  edge. Implemented on `jsargant_set_base_graph` for GitHub #28. `Graph::set_edge` is unchanged.
- **Files:** `guide/python-api.html` — the "The setter owes three checks" list under `#base-graph`.
- **Now false:** the list opens "the node count must match `network_size`, **or out-of-range edges
  are silently dropped**". That was the rationale for check 1 and is no longer what happens: the
  node count is still checked, and separately every edge is checked, so an out-of-range endpoint
  raises rather than disappearing. The framing "three checks" is also now wrong.
- **Should say:** the setter validates the declared node count against `network_size`, and then
  each edge for an out-of-range endpoint, a self-loop, and a multiplicity above
  `max_edge_multiplicity` — raising `ValueError` on the first failure and building nothing. Worth
  keeping the reason, because it is the non-obvious part: a node count equal to `network_size` does
  **not** make the edges in range, and a caller who takes `num_nodes` from their config rather than
  their data hits exactly that. The unset-base bullet is unaffected and stays.
- **Badges:** none — this list carries no `badge-planned` span or `.plan-note`. Prose correctness
  only, and it stacks with `#set-base-graph-ships`, which de-badges the section this list sits in.

*#set-base-graph-five-checks · filed 2026-08-13 14:41 — James.*

## 2026-08-13 16:43 — James — replicate runs shipped: de-badge six pages, and three code samples name the wrong parameter

- **Trigger:** GitHub #20, branch `jsargant_replicate_runs`. `run(seed, n_runs=1, max_cores=None)`
  now returns a **list** of `RunResult`, always — even at `n_runs=1` — with the master seed feeding
  a draw stream, native-Rust replicates running through a per-call rayon pool, and `python` fitness
  running them sequentially. Commits `6f8fc5c`, `50e4f7b`, `1abd10f`, `aa3e05c`.
- **Files:** `guide/python-api.html` (the "Replicates" section, ~L240, and its `.plan-note` at
  ~L288); `guide/reproducibility.html` ("Replicate seeding", ~L94); `guide/performance.html` (the
  `max_cores` heading, ~L112); `reference/lib.html` ("Replicate runs" ~L393 and the `api-item` at
  ~L403); `examples/index.html` (section 9, ~L320, and its `.plan-note` at ~L352); `status.html`
  (the "Replicate runs" row ~L83-90 and the `max_cores` row ~L92-97).
- **Now false:** every "Today:" note describing a single-run `run(seed)`. `python-api.html` says
  "`run(seed)` takes exactly one seed and performs one run, returning a single `RunResult` rather
  than a list" and advises a Python loop; `examples/index.html` says "`run(seed)` performs a single
  run and returns the edge list"; `status.html`'s two rows list both features as unbuilt.
  `reference/lib.html`'s api-item says "the `seed` parameter exists today; the other two are
  designed" — all three exist now.
- **Should say:** the prose describing the design is already accurate and needs no rewriting — this
  is badge and callout removal, not correction. The one substantive addition worth making is that
  the list is returned **unconditionally**, so `run(seed=1)` gives a one-element list rather than a
  bare result; that is the only part of the shipped behaviour the site does not already describe.
- **Naming correction — this one is a defect, not a badge.** Three pages show
  `evolver.run(seed=20260812, runs=30, max_cores=8)`: `guide/output.html` L140,
  `guide/python-api.html` L247, `examples/index.html` L329. The shipped parameter is **`n_runs`**,
  as `reference/lib.html` L403 already has it, so those three samples raise `TypeError` if copied.
  Rename `runs=` to `n_runs=` in all three.
- **Stale `src` reference:** `reference/lib.html` L405 points at `lib.rs:235` for `run`; the
  signature has moved with this change and the line is no longer right.
- **Badges:** drop `badge-planned` from `python-api.html` L240, `reproducibility.html` L94,
  `performance.html` L112, `lib.html` L393 and L406, `examples/index.html` L320; delete the
  `.plan-note` blocks at `python-api.html` ~L288 and `examples/index.html` ~L352; delete both
  `status.html` rows.
- **Adjacent, and not mine to file:** `reproducibility.html` L234 still badges "the seed and the run
  index on every log row" as planned. That shipped with GitHub #21 (PR #71), so it belongs in
  `mdube_edits.md` — flagged here because it sits four lines from a badge this entry does remove,
  and whoever sweeps this page will be looking straight at it.

*#replicate-runs-ship · filed 2026-08-13 16:43 — James.*

## Base-graph file loaders, `min_node_index`, and the identity individual — GitHub #107

- **Pages:** `guide/python-api.html` (the "Supplying a base graph" section, ~L316-345, its
  `badge-planned` at L316 and the `.plan-note` immediately after the `api-item`); `status.html`
  (the "Supplying a base graph" row ~L114-124 and the "Base-graph validation" row ~L125-131);
  `guide/genomes.html` (~L36, ~L51, the edge-edit section from ~L80); `guide/evolvers.html` (~L176,
  "Generation 0 is the initial population"); `guide/configuration.html` (wherever it says what is
  and is not a config value).
- **Now false, and two of these were already false before this change:** `python-api.html`'s
  `.plan-note` says "there is no base-graph setter, so every edge-edit run starts from an empty
  graph", and `status.html`'s row says "There is no setter" — `set_base_graph` has existed and been
  tested for some time, so both were stale independently of #107. The "Base-graph validation" row
  says validation is "Deferred explicitly in the code, since there is nothing to validate yet";
  every check it lists now runs. `python-api.html` also lists only *three* checks and describes cap
  narrowing as "rejected or warned" — it is rejected, flatly, and there are four checks.
- **Should say:** the setter exists, and a second one beside it reads the graph from a file —
  `set_base_graph_from_file(path, min_node_index=0)`, one edge per line as `start,end,weight`.
  Worth its own paragraph: `min_node_index` is where the caller's own numbering starts, every index
  shifts to 0 on the way in, and **only the evolved output graph shifts back**, so `best_edges`
  comes back in the numbering the user wrote. One numbering per run — a second loader given a
  different value is rejected. The file setter takes no node count, because a file has no such
  argument to get wrong.
- **The warning behaviour is new to the site entirely.** A repeated edge (canonical, so `2,5` and
  `5,2` are one edge) overwrites with a `UserWarning` and the last occurrence wins; a zero-weight
  edge and an empty file warn too. Nothing in `get/src` warned about anything before this, so no
  page mentions `warnings` at all — and `set_base_graph` itself now warns, which is the one part of
  its *existing* documented behaviour that changed.
- **`guide/evolvers.html` L176 needs a clause:** generation 0 is the initial population, *except*
  that a seeded edge-edit run fills one slot with the identity individual — every gene `Null`, so
  it expresses to exactly the supplied base graph. Unconditional, no config flag. Say what it buys
  and what it does not: a soft floor under a stochastic objective, since elites are rescored every
  generation and a bad draw can still evict it, and a genuine monotone guarantee only under a
  deterministic one.
- **Cross-check when sweeping:** `config.example.toml` gained a block under `[genome]` explaining
  why there is no base-graph key and pointing at `examples/config_builder.py`'s
  `seeding_from_a_file`. If `guide/configuration.html` walks that file key by key, it should pick
  the block up rather than skipping it as a comment.
- **Stale `src` references are likely:** `lib.rs` gained a field and a method ahead of `run`, so any
  `lib.rs:NNN` on `reference/lib.html` past the base-graph setter has shifted.

*#base-graph-file-loaders · filed 2026-08-17 23:58 — James.*

## 2026-08-18 20:05 — James — `set_base_graph` takes `min_node_index` too, and the numbering is shared by all three entry points

- **Trigger:** GitHub #107, `jsargant_graph_file_loaders`, answering Michael's review point 2 on
  PR #118. The in-memory setter now takes the same `min_node_index` the two file loaders take, so a
  caller whose dataset numbers nodes from 1 does not have to renumber it on any route in.
- **Files:** `guide/python-api.html:320` (the `set_base_graph` signature block and the prose under
  it); `examples/index.html:304` (the `refiner.set_base_graph(100, topology)` sample) and `:315`
  (the "Today: there is no `set_base_graph`" callout, already false before this change).
- **Now false:** `python-api.html:320` prints
  `GraphEvolver.set_base_graph(num_nodes, edges)`. The signature is
  `set_base_graph(num_nodes, edges, min_node_index=0)`. Nothing on the site says that the run's
  node numbering is *shared* — that one numbering is declared by whichever of the three entry
  points is called first, that a later call disagreeing with it raises, and that the numbering is
  what the evolved graph is shifted back into on the way out.
- **Should say:** the new signature, with `min_node_index=0` as the default, and one short
  paragraph on the shared numbering: `set_base_graph`, `set_base_graph_from_file` and
  `load_reference_graphs` all declare it; the first call wins; a second that disagrees raises
  `ValueError`; results come back in the numbering the data went in as. Worth an explicit line that
  an out-of-range message names the index as the caller wrote it, not as it is after shifting, and
  that the range it prints is inclusive (`1..=8` for 1-indexed data in an 8-node run).
- **Badges:** `examples/index.html:315`'s "Today: there is no `set_base_graph`" callout goes — it
  was already stale, and it is the one claim on these pages a reader would act on.

*#set-base-graph-takes-min-node-index · filed 2026-08-18 20:05 — James.*

## 2026-08-18 21:22 — James — the crate now documents the objective chain, and `new-fitness.html` is the second copy of it

- **Why:** GitHub #97 put the whole "adding an objective" chain into `get/src/fitness.rs`'s module
  doc, because a route-4 reader inside the crate had no way to find the other five files. That
  makes `guide/new-fitness.html`'s "Route 2" section a **second maintained copy of the same list**,
  which is the failure mode the one-sweep convention exists for. This entry is about reconciling
  them, not about a page that says something false.
- **Files:** `guide/new-fitness.html` — the "Route 2 — a native objective inside the crate" section
  and its "Checklist, whichever route"; `reference/fitness.html`, which should pick up the module
  doc's new sections.
- **The site was right and the crate was wrong, not the other way round.** `new-fitness.html`
  listed `type_name` and the Python attribute-path mapping; the issue's own table of six did not,
  and the crate doc was written from that table. Checked against `main` and the page is correct on
  both, so the crate doc was corrected to match rather than the page. Worth recording because it
  inverts the usual direction of these entries — do not "fix" the page toward the issue text.
- **Now false, or at least off:** the section says "Six small edits" and then lists **seven** items
  (the seventh is `config.example.toml`, introduced with "Then document it in"). It also omits the
  dispatch test that asserts a new variant erases to a box carrying its own `Direction`, which the
  crate doc now names as the last step. And it orders `impl Fitness` fourth, where the crate walks
  it first — a reader moving between the two meets the same chain in two orders.
- **Should say:** one ordering, matching the crate's — implement the trait, then `config.rs` (the
  variant, the `type_name` arm, validation), then the dispatch arm, then the Python mirror and its
  attribute path, then `config.example.toml`, then the test. Fix the count so the number matches
  the list, or drop the number. Add the test step. Consider making the page point at the crate doc
  for the canonical list and keeping the page for the things it does better — the three routes, the
  worked examples, and the trap about calling `evaluate_batch` from inside `evaluate`, none of
  which are in the crate doc.
- **Not urgent and nothing on the page misleads a user today.** Both lists now produce a working
  objective; they simply disagree about order and count.

*#fitness-chain-documented-in-crate · filed 2026-08-18 21:22 — James.*

## 2026-08-19 11:05 — James — `[genome]` rejects unknown keys now, so "the one table that refuses typos" is false in six places

- **Why:** GitHub #114 / PR #128 (merged `eaf7ace`) put `deny_unknown_fields` on the new
  `EdgeEditGenomeConfig`. `SdaGenomeConfig` already had it from #108, so as of now **the whole
  `[genome]` table rejects unrecognized keys, under either `type`**. The site still tells the reader
  that exactly one table in the document does that, and names a different one.
- **Files:** `reference/config.html` (`:254`, `:610-611`, and the `pub enum GenomeConfig` signature
  at `:476`), `guide/configuration.html` (`:195-197`, and the whole "Unknown keys, and the two GET
  checks for" section from `:307`), `guide/troubleshooting.html` (`:51-53`).
- **Now false — the uniqueness claim, stated six times:**
  - `reference/config.html:254` — "This is the one table in the whole document that refuses typos."
  - `reference/config.html:610-611` — "Unknown keys are ignored nearly everywhere. The single
    exception is `[genome.operation_weights]`."
  - `guide/configuration.html:195-197` — "the one sub-table that **rejects unknown keys** …
    Everywhere else in the config, a stray key is ignored."
  - `guide/configuration.html:307` — the section is titled "Unknown keys, and the two GET checks
    for". There is now a third, and it is not the same *kind* of check: `[fitness] seed` and
    `target_profile` are rejected **by name**, whereas `[genome]` rejects **anything unrecognized**.
  - `guide/troubleshooting.html:51-53` — "Two exceptions: `[genome.operation_weights]` rejects
    unknown keys, and two migration hazards are checked by name."
- **Now false — the Rust signature:** `reference/config.html:476` prints
  `pub enum GenomeConfig { EdgeEdit { gene_length, operation_weights }, Sda { … } }`. Both variants
  are newtypes over named structs now — `EdgeEdit(EdgeEditGenomeConfig)` and `Sda(SdaGenomeConfig)`.
  The `Sda` half has been wrong since #108.
- **Should say:** `[genome]` and `[genome.operation_weights]` both reject unknown keys, at either
  `type`. `[fitness]` does not, and the reason is worth keeping — it uses `#[serde(flatten)]` for
  the shared SIR block, and flatten consumes unrecognized keys into the flattened field's content
  map, so `deny_unknown_fields` cannot fire there. That is the distinction the pages should draw:
  **flattened tables cannot reject typos; the rest now do.** The `[genome]` key table at
  `reference/config.html:192-232` has no "any unrecognised key" row at all and should gain one.
- **Half of this predates #128.** `SdaGenomeConfig` got the attribute in #108 and no entry was
  filed, so the claim has been wrong for `type = "sda"` for some time and is only now wrong for
  both. Not a new lapse — but it means the sweep should not assume the pre-#128 text was correct.
- **Related:** `#reference-pages-describe-the-pre-108-api` in `mdube_edits.md` also names
  `reference/config.html`, for `Config::from_path`. One page, two queues — apply both together.

*#genome-table-now-rejects-unknown-keys · filed 2026-08-19 11:05 — James, reviewing PR #128.*

## 2026-08-19 11:51 — James — crossover has a shared helper now (`breed_pair`), and the page says twice that it does not

- **Why:** GitHub #56 / PR #129 (merged `1d2dc3e`) extracted the crossover-and-mutate sequence out
  of `generational.rs`'s `advance_generation` and `steady_state.rs`'s `mating_event` into
  `common::breed_pair`. The page was written when each strategy really did spell the sequence out
  itself, and it says so in the strongest available terms. **Filed by James, not the task's owner** —
  PR #129 changed no file under `documentation/`, and this is the third consecutive PR to skip the
  filing (#108, #128, #129), so it is filed here rather than waiting.
- **Files:** `reference/evolver-common.html` — the prose at `:583-588`, the common.rs function table
  at `:58-61`, and the `mutate_child` signature at `:544`.
- **Now false — the prose, and it is emphatic about it** (`:583-588`):

      Crossover has no shared helper — it is a single roll each strategy makes for itself,
      immediately before calling `mutate_child` on each of the two children.

  Both halves are now wrong. There *is* a shared helper, and the strategies do **not** make the roll
  for themselves: `breed_pair` owns the crossover roll and both `mutate_child` calls, and each call
  site is one line.
- **Still true, and worth keeping:** the sentence that follows — "Both strategies make the rolls in
  the identical order (crossover, then child one, then child two) so the two consume randomness the
  same way." That is now guaranteed structurally rather than by two copies agreeing, which is a
  stronger claim and the actual point of the change. Rewrite around it rather than deleting it.
- **Now incomplete — the function table** (`:58-61`) lists `express_and_score`, `mutate_child`,
  `generation_stats` and `rank`/`best_index`, and omits `breed_pair`. It should gain a row:
  *"one crossover roll and both mutation rolls, in the order every strategy must draw them"*.
- **Now false, and NOT from #129** (`:544`): the page prints

      pub fn mutate_child<G, R>(child: &mut G, mutation_rate: f64, max_mutations: usize, rng: &mut R)

  The real signature takes `context: &G::Context` as its second parameter (`common.rs:186-192`).
  Pre-existing staleness, folded in here because it is the same page and the same section, and a
  sweep that corrects the prose around it and leaves the signature wrong is the half-corrected page
  the one-sweep convention exists to prevent.
- **Should say:** `breed_pair` is the shared helper for recombination and mutation; each strategy
  selects its parents and then calls it. Keep the draw-order sentence, reattributed to the helper.
  Add the table row. Fix `mutate_child`'s signature.
- **What is NOT owed an edit:** the `mutate_child` narrative at `:585` about the two dice rolls, and
  the reproducibility callout at `:578-580`, are both still accurate. `test_support.rs` is new but is
  `#[cfg(test)]` and never reaches the shipped crate, so it wants no page at all.

*#crossover-now-has-a-shared-helper · filed 2026-08-19 11:51 — James, reviewing PR #129.*
