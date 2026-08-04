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

### 15. `sir_sim` reports `length` one step shorter than the reference C++ does

**FYI, and Confirm before you build #17 on top of it.** I have implemented **#16** on branch
`mdube_sir_sim`, in a new module `get/src/sir.rs`. No file you own is touched — one new file plus
`pub mod sir;` in `get/src/lib.rs`.

The mechanics are a straight port of `Graph::SIR` in the legacy `Graph.cpp`: the same adjacency
scan accumulating each susceptible node's total exposure, and the same single combined draw
against `1 - (1 - rate)^exposure`, so a multiplicity of `k` stays `k` independent chances. Two
things deliberately differ, and both change numbers #17 will consume.

1. **`length` is one smaller.** The C++ `len` increments on every loop pass including the final
   one, in which the last infectious node merely recovers and infects nobody. Spec §5.2 fixes the
   other convention — "an outbreak that infects nobody beyond patient zero has `length = 0`" —
   where the C++ gives `1`. I built to the sheet, per `CLAUDE.md`. Consequence: `epi_length`
   scores will sit one below any historical C++ result for the same graph. Constant offset, so it
   cannot change selection, but it will make old and new numbers look mismatched.
2. **`profile` carries no trailing zero.** The C++ pushes the terminating `0`; ours stops at the
   last real infection. This one is *not* neutral for `epi_prof_match` — an RMSE against a target
   captured from C++ output would be comparing vectors of different lengths.

The three readings are then consistent by construction: `profile[0]` is patient zero, `spread` is
the sum of the profile (total ever-infected, so `1` for a lone patient zero), and `length` is
`profile.len() - 1`. #16's own verify-by agrees — a 6-node path at rate 1.0 gives `length = 5`
and `spread = 6`.

If you would rather match the C++ exactly, that is a change to spec §5.2 and needs the joint
meeting, not a patch from either of us. Say so and I will re-raise it as a sheet amendment.

*#15 · raised 2026-08-04 10:53 — Michael.*

**Michael's position, 2026-08-04 11:22:** the intended behaviour is what `legacy/main.cpp` does, so
my leaning is to match the C++ rather than the sheet — `length` gains the burnout step and `profile`
regains its trailing zero. That is a §5.2 amendment, so it stands as a discussion item for the
meeting and I have not changed the code. **Consequence worth stating plainly: until this is settled,
`sir_sim`'s `length` and `profile` are contested, so issue #16 should not be closed.** The
simulator is otherwise complete. *(Reply inside #15 · 2026-08-04 11:22 — Michael.)*

### 16. Before #26 is built: should the fitness axis be `dyn` rather than a match arm?

**Decide, at the next meeting.** Nothing is blocked today and I am not proposing to change the
sheet unilaterally — but this gets expensive the moment #26 exists, so it is worth ten minutes now.

**The trigger.** Adding a native Rust objective currently means editing three files: `fitness.rs`
for the `impl Fitness`, `config.rs:95` for a new `FitnessConfig` variant, and the dispatch in
`GraphEvolver::run`. #26 specifies that dispatch as a nested match over 2 strategies × 2 genomes ×
4 fitnesses, so a fifth objective adds **four** arms, not one.

**The fact that makes a choice available.** The two dispatch axes are not symmetric, and I think
this has not been noticed:

- `Genome` declares `fn mutate<R: Rng + ?Sized>(&mut self, rng: &mut R)` — a generic method, so
  `Genome` is **not** object-safe. The genome axis has to stay a match. This is not negotiable.
- `Fitness` has no generic methods. Probed on 2026-08-04: `Box<dyn Fitness>` constructs and
  dispatches, and `dyn Fitness` is `Send + Sync` through the supertrait, so rayon is unaffected.

#26's own text has a section headed "Why a match and not `dyn`", and its reason is that
`Evolver::run<F>` is generic so `Box<dyn Evolver>` is not viable. That is correct — and it is about
the **evolver** axis. It does not address the fitness axis, so the question may simply never have
been put. That is why I am asking rather than treating it as settled.

#### Option A — keep the match exactly as #26 specifies

Dispatch stays 2 × 2 × 4 = 16 arms, every combination naming its concrete fitness type.

- **Adding an objective:** `fitness.rs` + `config.rs` + 4 new dispatch arms across 3 files.
- **Performance:** static dispatch throughout; `evaluate` can inline into the rayon closure.
- **In its favour:** it is what the sheet says, it needs no new trait machinery, and the concrete
  type is visible at every call site, which is easier to read when debugging a specific run.
- **Against:** the arm count grows multiplicatively, and #26 already anticipates the problem —
  it suggests "a macro over the arms is the next step if it gets unwieldy". A macro over 16 arms is
  harder to read than either option here.

#### Option B — erase the fitness to `Box<dyn Fitness>` before instantiating the evolver

Each arm builds its objective, boxes it, and hands one type to the evolver. Dispatch collapses to
2 × 2 × 1 = 4 arms. Needs one small `impl Fitness for Box<dyn Fitness>` in `fitness.rs`, which must
live inside the crate — the orphan rule rejects it from a test or another crate, which I confirmed
by trying.

- **Adding an objective:** `fitness.rs` + `config.rs`. **Dispatch is never touched.**
- **Performance:** one virtual call per `evaluate`. For an SIR objective that call has an entire
  epidemic behind it, so the vtable cost is noise; `evaluate_population` is one virtual call per
  *batch*. If we ever add an objective cheap enough for the indirection to matter, that objective
  can still be dispatched statically as a special case.
- **In its favour:** it also makes the `PyFitness` adapter (#19) an ordinary objective rather than
  a shape the matrix has to accommodate, since it is already the case that only the boxed value
  differs.
- **Against:** the concrete objective type is no longer visible at the call site, and it is a
  departure from what the sheet currently records.

**What does not change under either option.** `config.rs` still needs a variant per objective, so
two files is the floor regardless. That is deliberate and I am not proposing to touch it: #13 and
#23 make "serde *is* the validation" load-bearing, and a string-keyed registry would move
validation out of serde, which is the precise failure those two issues exist to prevent.

**Why the timing matters.** Both options cost about the same to build today. Once the 16-arm match
exists, moving to B is a rewrite of the whole dispatch layer rather than a different way of writing
it the first time. My preference is **B**, but it is a sheet change (§6, §8) and therefore a joint
call — I have not touched the sheet or the code.

*#16 · raised 2026-08-04 11:13 — Michael.*

### 17. The C++ re-rolls short epidemics, and neither the sheet nor any issue mentions it

**Decide, at the next meeting — this changes fitness values.** Found 2026-08-04 while checking
`sir_sim` against `main.cpp`, which is the legacy driver. It is not in `official_spec_sheet.md`
§5.2 and it is not in issues #16, #17 or #18, so right now it would simply be lost.

**What the C++ actually does.** Every fitness draw, under both objectives, is a *rejection-resampled*
epidemic rather than a plain one (`main.cpp:520-531` for epidemic length, `537-542` for profile
matching):

    cnt = 0;
    do {
        profile = G.SIR(alpha, patient0);
        cnt++;
    } while (profile.size() - 1 < mepl && cnt < rse);

with `mepl = 3` (minimum epidemic length) and `rse = 5` (re-tries), both at `main.cpp:39-40`. So an
outbreak that burns out in under 3 steps is thrown away and re-rolled, up to five attempts; the
fifth is kept whatever it looks like.

**Why it is there, and why it is not the same as averaging.** A fizzled outbreak carries no
information about graph structure — it says the dice went badly, not that the network is poor. Left
in, a large share of evaluations return near-zero and selection chases the dice. But this is a
*biased* resample, not a variance reduction: it shifts the expected fitness upward, and by an amount
that depends on how often a given graph fizzles. Averaging more epidemics (`num_epidemics`, §5.2)
does **not** substitute for it — the two do different jobs and the C++ does both.

**Three ways to go, and the choice is yours as much as mine:**

- **Port it as-is**, with `mepl` and `rse` as config fields. Reproduces historical behaviour;
  carries the bias forward as a deliberate, documented choice.
- **Drop it** and rely on `num_epidemics` alone. Cleaner and unbiased, but the fizzle problem it
  was solving is real and will come back — and our numbers will not be comparable to old runs.
- **Replace it** with something unbiased that solves the same problem, e.g. requiring a patient zero
  with non-zero degree, or reporting the fizzle rate so it is visible rather than silently corrected.

**Where it would live.** Not in `sir_sim`. That function is one epidemic by contract (#16) and I
think it should stay that way — the retry is a *scoring policy* wrapping the simulator, so it
belongs with the objectives in #17. If we adopt it, #17 gains a requirement rather than #16
re-opening.

**Two more things `main.cpp` settles, worth capturing while we are here.**

1. **#17 has an open question the C++ already answers.** #17 asks how RMSE handles a target and a
   run of different lengths. `main.cpp:545-553` iterates over the *target* length `PL + 1`, treats
   the run as zero beyond its end, and always divides by `PL + 1`. That is a real answer to a
   question currently marked undecided.
2. **A legacy bug not to replicate.** In the profile-matching branch, `main.cpp:559` divides by
   `NSE` even when `finalTest` ran `FTL = 50` epidemics — the length branch gets this right at
   `main.cpp:535` by dividing by `tests`. So final-test profile scores in the old code are inflated
   by a factor of `FTL / NSE`. Worth knowing before anyone compares our numbers to archived results.

I have not touched the sheet, the code or the issues over any of this.

*#17 · raised 2026-08-04 11:17 — Michael.*

**Michael's position, 2026-08-04 11:22:** intended behaviour is what the C++ does, so my leaning is
the first option — port the re-roll as-is, with `mepl` and `rse` as config fields rather than
hardcoded. Raising it here as a discussion rather than acting on it, since adopting a mechanism the
sheet does not mention is a §5.2 amendment either way. Note the paths in this item moved: the C++
is now tracked at `legacy/main.cpp` and `legacy/Graph.cpp`, with `legacy/README.md` recording what
it is; the line numbers cited above are unchanged. *(Reply inside #17 · 2026-08-04 11:22 — Michael.)*

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
