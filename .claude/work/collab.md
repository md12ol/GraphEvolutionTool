# Collaboration log — questions, answers, and overrides

Shared by everyone who works in this repo (Michael / md12ol and James / shorinbonsai). Everyone
reads and writes it. It is not addressed at any one person.

## What goes here

- **A question** you want the other owner to answer before you build on it.
- **A decision on your side that conflicts with or overrides theirs** — the kind where proceeding
  silently wastes someone's work.

Not every disagreement. If it needs no answer from anyone, it is a `decisions.md` entry instead.

## How to use it

**Raising.** Append a new item at the end of **Open**, numbered one higher than the last. Say what
you want — **Confirm**, **Decide**, or **FYI** — in the first line. An item with no ask sits open
forever.

**Answering.** Append your reply *inside* that item, beneath the existing text, as its own stamped
line. Do not edit what the other person wrote; do not delete their words to make room for yours.

**Settling.** When an item is resolved, move the whole item to **Agreed** with the date, keeping
every stamp. Agreed items are never deleted — the trail is what stops the same argument recurring.

## Formatting — one rule that bites

`/.gitattributes` sets `merge=union` on `.claude/work/*.md`, so concurrent appends merge without
conflict markers — and **never conflict**, which means byte-identical lines on both sides fold
together and interleave two entries into one. So **close every item with its own number and a
time**: `*#7 · raised 2026-07-31 15:42 — Michael.*` — never a bare `*Raised <date> — <name>.*`.

Audit before pushing and after any merge; anything it prints could collapse:

```bash
grep -vE '^\s*$' .claude/work/collab.md | sort | uniq -d
```

Full rules: `CLAUDE.md`, "Formatting for union merge".

Persistent: survives `/done`, because coordination outlives any one task.

---

## Open

### 14. Starting issue #10 — it edits the two files issues #14 and #15 also edit

**FYI, and Confirm if you are about to start #14 or #15.** Not blocking: I am proceeding now
rather than waiting, so this is a heads-up you can act on, not a gate.

*(Numbering note: this is `collab.md` item 14, which is unrelated to GitHub issue #14. The
collision is coincidental — collab items are numbered one higher than the last, independently of
the tracker. Below, "#10/#14/#15" always mean tracker issues.)*

I have picked up **#10 — remove maximum mutations from genome to make configurable**. Per spec §4
it moves both mutation dice rolls into one shared helper, so it necessarily touches:

- `get/src/genomes/genome.rs` — the `mutate` contract, the load-bearing edit
- `get/src/genomes/edge_edit.rs` — delete `MAX_MUTATIONS`, reduce `mutate` to one gene reroll
- **`get/src/evolver/common.rs`** — the new shared helper
- **`get/src/evolver/mod.rs`** — `SharedEvolutionContext` gains `max_mutations`
- **`get/src/evolver/steady_state.rs`** — lines 58-62, the inline mutation loop
- `get/src/config.rs` and `config.example.toml` — the `max_mutations` field

The bolded three are the overlap. Your **#14** (rename `evaluate` to `express_and_score`) and
**#15** (convert direction only at the boundary) list `common.rs`, `mod.rs` and `steady_state.rs`
as their change sets too. These are source files, so `merge=union` does not apply and a genuine
conflict is possible if we work them in parallel.

What I am doing about it: my changes to `steady_state.rs` are confined to the mutation rolls in
`mating_event`, and I am not renaming or re-signaturing anything you own — my helper calls
`common::evaluate` under its current name so your rename sweeps it cleanly. If you would rather
land #14/#15 first, say so and I will rebase onto them instead; the reverse order also works and
costs me a rename.

Two things I am deliberately **not** touching, so you know they are still yours: the
`express_and_score` rename, and `generation_stats` losing its `direction` parameter.

One overlap worth naming: **#24 also specifies the top-level `max_mutations` config field.** I am
implementing that field under #10, because #10 cannot be verified without it. #24 keeps the rest of
the schema. Flagging it so it does not read as me having done half of #24 badly.

*#14 · raised 2026-08-03 16:40 — James.*

## Settled

Compressed 2026-07-31 after the spec-sheet call: the reasoning for each of these now lives in
`decisions.md` or `/official_spec_sheet.md`, so only the disposition is kept here. Nothing is
deleted that is not recorded somewhere durable.

| # | Item | Disposition | Reasoning lives in |
|---|---|---|---|
| 1 | `common.rs` implemented, not to be duplicated | generational calls the same helpers unchanged | spec §5–6 |
| 2 | two children per mating event | agreed as built | spec §6.3 |
| 3 | steady-state per-event FFI cost | accepted as a known limitation; prefer generational for stochastic or Python objectives | spec §6.3 |
| 4 | RNG must match across strategies | `ChaCha8Rng` in both | `decisions.md` |
| 5 | log cadence + iteration-0 row | agreed; generational logs generation 0 too | `decisions.md` |
| 6 | with or without replacement | James's call: `select` stays with replacement | spec §5 |
| 9 | `Fitness::direction()` | agreed and extended — fixed per objective, never a config field | `decisions.md` |
| 10 | `evaluate` orients | agreed; renamed `express_and_score`, sole scoring entry | `decisions.md` |
| 11 | `generation_stats` direction parameter | **reversed** — engine stays in one orientation, converts only at the Python boundary | `decisions.md` |
| 12 | config-layer validation | widened into one `Config::validate` for both front ends | `decisions.md` |
| 7 | tree is not `cargo fmt`-clean | superseded by tracker **#22**, which carries the sequencing ("land it when James's tree is clean") | GitHub #22 |
| 8 | `Cargo.lock` stays tracked | done and merged via PR #12; a courtesy heads-up, not a question | `decisions.md` |
| 13 | `merge=union` on the shared docs | not a code decision — `.gitattributes` scopes it to `.claude/work/*.md` only, so it never touches source. The interleaving risk is handled by the stamp-and-audit rules, not by sign-off | `CLAUDE.md`, "Formatting for union merge" |
| — | `Genome::copy` removed from the trait | accepted; `Planning Notes.md` is the stale side | `decisions.md` |

**README "Graph multiplicity" section deleted — correct (2026-07-31).** Kept in full because it is
recorded nowhere else: PR #11 removed it, and the section documented `Graph::unweighted()`,
`Graph::with_max_edge_multiplicity()` and `SdaContext::new/unweighted/...`, none of which exist —
commit `520500b` replaced them with the two-arg `Graph::new` and left the README stale. The deletion
was a fix, not a regression.

*Settled block compressed 2026-07-31 23:20 — Michael, after the spec-sheet call;
items 7, 8 and 13 closed out 2026-07-31 23:45 leaving Open empty.*
