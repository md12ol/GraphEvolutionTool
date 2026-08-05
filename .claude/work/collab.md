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

`/.gitattributes` sets `merge=union` on this file and `decisions.md` (narrowed 2026-08-04 — the
other three working docs no longer use it), so concurrent appends merge without
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

Items 1–19 are all dispositioned; 14–19 were settled at the joint meeting of 2026-08-04 and moved
into **Settled** below with every stamp intact. Item **20** was raised the same evening and is the
only thing open. Append the next item here as **21**, numbering continuing across both sections.

*(The "Nothing is open" line that stood here was written before item 20 landed, and is corrected
rather than removed — see the merge-repair note in item 20's tail.)*

### 20. `CLAUDE.md` still says all five working docs are union-merged; only two are now

**Decide — it is your amendment, so I have not touched it.** Not blocking; I am starting #14 and
this changes no code. Found reviewing PR #33 before merging it, and worth fixing soon because the
stale text is in the document every session loads first.

PR #33 narrowed `merge=union` to `decisions.md` and `collab.md`. Confirmed on `main` after the
merge, with `git check-attr merge -- .claude/work/*.md`: those two report `union`, and `traps.md`,
`issues.md` and `hotfixes.md` report `unspecified`. That is exactly what the `.gitattributes`
comment intends — the three churn lists take a normal 3-way merge so a delete can no longer be
silently discarded.

Three places in `.claude/CLAUDE.md` still describe the old, wider behaviour:

- **"Two people, one `.claude/`", rule 1** — "`decisions.md`, `traps.md`, `hotfixes.md`,
  `issues.md` and `collab.md` … `/.gitattributes` sets `merge=union` on them". Three of those five
  no longer.
- **The routing table**, `.claude/work/*.md` row — permits a direct push to `main` on the grounds
  that the files are "append-only observations, **union-merged**". The permission is still right;
  the reason given is now wrong for three of the five, and they are the three where a concurrent
  append will genuinely conflict.
- **"Pull requests"**, first bullet of the three silent failures — "Byte-identical lines in
  `.claude/work/*.md` dedupe and interleave". True only of the two remaining files.

Why I am raising it rather than editing: the fix reads as recording a fact, which the routing table
permits me to push directly. But it is *your* amendment, one sentence of it is a live permission
rather than a fact, and rule 5 as you have just written it makes an in-place edit of a shared
document announce-first. Editing your text silently to fix the consequences of your own PR is
exactly the shape that rule exists to stop.

What I suggest, if you want me to take it: amend the three passages to name the two union files
explicitly, keep the direct-push permission unchanged but re-justify it on the audit command rather
than on union, and add one line saying the other three now conflict on concurrent appends — which
is the behaviour change a reader most needs to know about. Say the word and I will do it; say
nothing and it is yours.

*#20 · raised 2026-08-04 15:28 — James.*

**Merge repair, 2026-08-04 15:40 — James. This item is itself a worked example of item 19.** I
raised it and pushed; the push was rejected because you had meanwhile pushed `7efdd79` (which fixes
exactly what this item asks for) and `27c863a` (which relocates items 14–19 into **Settled**). The
`git pull` that followed union-merged your relocation against my copy, which still had 14–19 under
**Open**. Result, with no conflict and no warning:

- `## Settled` and the whole 2026-08-04 meeting block appeared **twice** — your revised wording near
  the top, my stale copy of the pre-relocation wording at the tail.
- **This item was swallowed.** It landed inside your "Items 14–19, moved from Open" block, between
  item 19 and the duplicate `## Settled` — an *open* item filed under settled ones, which is the
  version a reader would have believed.

The item bodies themselves were each intact and unduplicated; the damage was confined to the
structural headings and the block you had edited in place. Repaired by hand: deleted the duplicate
tail section, kept your wording, and moved this item back under **Open**. The `uniq -d` audit is
clean, verified before pushing. The pre-repair file is kept out-of-tree in this session's scratchpad.

**Nothing of yours was lost** — your wording is the copy that survived, and your "Nothing is open"
placeholder was corrected rather than deleted, since item 20 makes it untrue. If you would rather it
read differently, it is yours to reword.

*#20 · merge repair 2026-08-04 15:40 — James.*

**Mostly overtaken by your own fix, 2026-08-04 15:45 — James. Two of the three are already done.**
Checked `.claude/CLAUDE.md` on `main` after the merge: `7efdd79` rewrote rule 1 to "The two
append-only docs merge by union" and added the paragraph saying the other three are **not**, and it
re-justified the routing-table row while keeping the direct-push permission. Both are exactly what
this item asked for, and you had written them before I raised it — we found the same thing
independently within the hour.

**One line is still stale**, `CLAUDE.md` line 197 in the "Pull requests" section: "**`merge=union`
never conflicts.** Byte-identical lines in `.claude/work/*.md` dedupe and interleave" — the glob
still names all five, where only `decisions.md` and `collab.md` behave that way now. Trivial, and
the surrounding point stands.

**Narrowed ask:** just that one line, and it is a fact rather than a permission, so I am happy to
push it directly if you would rather not bother. Say nothing and I will leave it to you.

*#20 · narrowed 2026-08-04 15:45 — James.*

### 29. PR #37 self-merged — the one-line spec status tidy

**FYI, no action needed — this is the trace `CLAUDE.md` requires for an unreviewed merge.**

PR #37 dropped a single caveat from the `sir_sim` row of the spec status table. The row had read
"corrected by GitHub #34"; #34 closed when PR #36 merged, so the sheet was citing a closed issue as
pending work. One line, `official_spec_sheet.md` only, no `.claude/work/*.md`, so union merge was
not involved.

Michael merged it himself at 2026-08-04 19:52 rather than waiting, because it was blocking the
`/done` gate on the sir-conventions task and the change is a strict deletion of text that had become
false. Reviewed by nobody, which is the reason this entry exists.

**Worth noting against the rule as written:** the exception in `CLAUDE.md` is "the other owner is
unavailable", and James was demonstrably available — he had merged #35 and #36 six minutes earlier.
So this is a self-merge of convenience, not of necessity. Recording it honestly rather than dressing
it as the documented case. If that reads as the rule being too tight for one-line doc corrections,
that is worth deciding rather than repeating.

*#29 (renumbered from the duplicate 20 on 2026-08-06 — see resolution note below) · raised
2026-08-04 19:52 — Michael.*

**Merge-repair note, 2026-08-04 16:31 — James. This item was spliced into the middle of my
item 20 and I have lifted it back out.** Union merge concatenated your entry into the first
bullet of my merge-repair note, so your heading lost its line and my sentence was torn in half.
Neither entry was readable and yours was not a top-level item at all. Your text and stamp are
reproduced here **exactly**; only the position changed, placed by your 19:52 stamp so it sits
between my item 20 and your item 21.

**The `uniq -d` audit did not catch this** — the splice duplicated no line, so the documented
check came back clean on a corrupted file. That is new, and it is in `traps.md`.

**We both numbered an item 20.** I have not renumbered yours, since renumbering your entry is
your call — but the two need distinguishing, and mine is the earlier stamp (15:28 against your
19:52). Say which you want and I will do it.

*#20-collision · repair note 2026-08-04 16:31 — James.*

**Renumbering resolved, 2026-08-06 — Michael.** Took the call James left open above: my item (the
PR #37 self-merge trace, 19:52 stamp) is now **#29**, the next free number, rather than reusing 20.
James's item keeps its original **20** unchanged — his text and stamp above are untouched. This is a
heading and closing-stamp change only, on my own entry, so no meeting or further announcement was
needed beyond this note.

*#29-collision · renumbering resolved 2026-08-06 — Michael.*

### 21. Do users supply their own Rust objective as a drop-in file? The sheet says no; Michael says yes

**Decide at the next meeting — it changes #26 far more than it changes #17.** Raised while planning
#17, when Michael said the intended model is that "people provide a Rust file for their own fitness
functions if they want to add one". I could not find that anywhere in the sheet, so this item is to
settle which of the two is the real intent before #26 is built to the wrong one.

**What the sheet actually says.** The only user-extension route described for fitness is the
**Python** adapter — §5's "a user-supplied Python objective declares its direction when the callable
is registered", and §8's adapter, tracked as issue #19. §5.2 closes with *"keep hot objectives native
in Rust. The Python adapter is for prototyping"*, which reads as guidance to **us** about where to
implement the objectives we ship, not as a documented extension point for users. §10's non-goals do
not mention drop-in objectives either way, so this is a genuine silence rather than a stated no.

**Why it is not a cosmetic question.** Issue #26 erases the objective to `Box<dyn Fitness>` via a
closed `match config.fitness` with one arm per objective — the amendment agreed at the 2026-08-04
meeting, and the thing that took dispatch from 16 arms to 4. A closed match over a config enum
cannot name a type that is not in the crate. So a user-supplied Rust objective means forking and
recompiling, and at that point adding the match arm is the smaller edit — which makes the drop-in
file mechanism buy nothing unless something more is intended, such as a registration API or a
build-time hook. **If the drop-in model is real, #26's step 1 is designed wrong**, and it is far
cheaper to know that before it is built than after.

**Worth noting the Python adapter may already be the answer.** It is the documented route, it needs
no recompilation, and §5.2's own guidance concedes it is slower — so the honest question may be
whether "hot objective, no fork" is a real user need or a hypothetical one.

**Not blocking #17.** I am building the three SIR objectives to the sheet as written: they go in
`get/src/fitness.rs` beside the trait, which is also what a drop-in example would want to look like,
so neither answer wastes the work.

*#21 · raised 2026-08-04 22:00 — Michael.*

### 24. What is in a `Profile*.dat`? The C++ loader adds patient zero and rescales by `verts / 128`

**For you, before #26 reads one — not blocking #24, which only stores the path.** Raised while
building the `config.rs` schema. `epi_prof_match` needs a target profile, and the sheet says only
that it "adds a target" (§7); §7's TOML block does not show the field and issue #24's enum sketch
omits it. James settled the config side on 2026-08-05: the variant carries
**`target_profile_path`**, a path that `config.rs` never opens, so parsing stays pure
deserialization. That leaves the question of what the file actually contains, and **whoever builds
#26 has to answer it** — the dispatch is what turns a path into the `Vec<f64>` that
`EpiProfMatch::new` (`get/src/fitness.rs:251`) requires.

**What the C++ does, which is more than "one number per line"** — `legacy/main.cpp:370-388`, reading
`./Profiles/Profile<n>.dat`:

    for (int i = 0; i < PL; i++) { PD[i] = 0; }        // pre-fill
    PD[0] = 1;                                         // patient zero is NOT in the file
    for (int i = 0; i < PL; i++) {
        inp.getline(buf, 19);  val = strtod(buf, nullptr);
        PD[i + 1] = val * ((double) verts / 128);      // rescale to the network size
    }

So a stored profile **omits its own first element** and is **normalized to a 128-node network**.
Two conventions, neither in the sheet, and both silent if got wrong: forget the prepend and every
target is shifted one timestep; forget the rescale and a 512-node run is compared against
128-node counts, which is a wrong number rather than an error.

**The ask:** decide whether GET reproduces both conventions, one, or neither — and if the file
format changes, say so before #26 is built rather than after. My own read is that reproducing both
is right for comparability with the archived C++ results, which is the same argument that kept the
short-epidemic re-roll (§5.2), but this is your issue and the profiles are your data.

**Also worth noting the length interaction.** The 2026-08-04 amendment to §5.2 gave `profile` a
terminating zero and made `length` one higher than before, so a target captured from older output is
already one element out of step. `EpiProfMatch`'s doc comment at `fitness.rs:236-238` says as much.
Whatever is decided here should say which convention a `.dat` on disk is in.

*#24 · raised 2026-08-05 15:09 — James, while planning GitHub issue #24.*

### 25. Unknown keys under `[fitness]` are silently ignored, and cannot be made to error

**FYI, and one thing to not assume in #26 — no decision needed from you unless you disagree.**
Measured while building GitHub #24 on 2026-08-05: a `[fitness]` block carrying a leftover
`seed = 42` **parses clean and the key is discarded**. Issue #24's `Verify by` line asked for it to
be rejected as an unknown key; that is not achievable. Serde deserializes a `#[serde(flatten)]`
field through a buffered content map, so `deny_unknown_fields` never fires — confirmed by putting
the attribute on `SirParams` itself, which changed nothing.

Spec §7 requires the flatten in as many words, so the flatten stayed and the verify line is what
gave. Reasoning and the rejected alternatives are in `decisions.md` 2026-08-05 15:47; the behaviour
is pinned by `an_unknown_fitness_key_is_ignored_rather_than_rejected` in `get/src/config.rs`.

**Why you may care, in #26.** The natural assumption when reading `config.rs` is that a typo in a
`[fitness]` key is a parse error. It is not, for that table specifically — `[genome]`'s operation
weights *do* reject unknown keys, because `EdgeEditOperationWeights` carries
`deny_unknown_fields` and is not flattened (`get/src/genomes/edge_edit.rs:27`). So the two tables
behave differently and neither is wrong.

**Where I think the check belongs:** `Config::validate` (#23, mine), which can look at the raw text
before it is deserialized and reject a `seed` under `[fitness]` by name. I will pick it up there
unless you would rather it went somewhere else. Flagging it because the migration case is the
silent kind — an old config keeps `seed = 42`, sees no error, and runs under a different seeding
model than its author believes, since the master seed now comes from the `run` call.

*#25 · raised 2026-08-05 15:47 — James, during GitHub issue #24.*

### 26. Pushed a `CLAUDE.md` convention straight to `main` — commit each verified feature-branch step separately

**FYI, no action needed — this is the trace for a direct push per the routing table's own
exception.** `CLAUDE.md` says prefer a PR when a change binds the other owner's practice; I judged
this one not worth a branch and review cycle, so it went direct, same as the two conventions in
item 22.

**The rule:** commit each verified task-list step on a feature branch separately — a lint-policy
decision, a formatting sweep, one file of a readability pass — rather than batching everything into
one commit at PR time. Landed live on issue #22's branch this session: the `needless_return` lint
decision and the tree-wide `cargo fmt` sweep are already two separate commits rather than one.

**Why now:** working through #22 file-by-file with the user surfaced the question directly — small
reviewable commits make each step independently bisectable and reviewable, rather than one large
diff to audit at PR time. Push back here if you'd rather this weren't standard.

*#26 · raised 2026-08-05 23:45 — Michael.*

### 27. `Swap`'s degree floor is `> 2` in the spec and code, but the original Java required only `>= 2`

**Decide.** Found while explaining `operations.rs::swap` to Michael during the #22 readability pass —
not blocking anything, but it's a real, checkable numeric discrepancy rather than a style question.

**Where it's from.** `GraphEvolutionTool/src/Graph.java` and `GET.java`, a 2019-era Java
predecessor kept locally in `OneDrive - University of Guelph/Coding Projects/Archive/`, not on
GitHub. `swap(int a, int b, int k)` rejects when `nbr.get(v1).size() < k` — i.e. it requires
**degree >= k** — and the only caller passes a named constant: `MIN_DEG_SWAP = 2`. So the original
requirement was **degree >= 2**.

**What we have now.** `official_spec_sheet.md` §3.1 says "two non-adjacent vertices of **degree >
2**", and `get/src/genomes/edge_edit/operations.rs::swap` implements exactly that —
`graph.degree(first_vertex) <= 2` rejects, i.e. requires **degree >= 3**. One higher than the
original, on both vertices.

**The other four checks match exactly**, verbatim in spirit — non-adjacent `v1,v2`, all four
vertices distinct, and none of `v1-a2`, `v2-a1`, `a1-a2` already an edge. Only the degree floor
differs, which is what makes it look deliberate rather than a slip — everything else was ported
faithfully.

**No comment in the Java explains why `2` was chosen**, so I can't tell if `> 2` here is an
intentional tightening or an off-by-one from the port. Worth deciding: match the original (`>= 2`),
or keep the current stricter `> 2` and drop a line into `decisions.md` saying so on purpose.

*#27 · raised 2026-08-06 00:09 — Michael, transcribed by Claude during a readability-pass session.*

### 28. Documented: `/done`'s doc sweep pushes to `main` directly, decoupled from the task's own code PR

**FYI, no action needed — clarifying an existing rule, not creating one.** Came up closing out #22:
the `/done` sweep (task-complete marker in `decisions.md`, `hotfixes.md`'s `Last checked` stamps,
`traps.md` updates, the archive itself) was written while PR #43, carrying #22's code, was still
open and unmerged. The question was whether that meant committing to `main` directly or waiting for
the PR — the routing table already answered "direct push is fine" for `.claude/work/*.md`, but not
the *timing* relative to an open PR, so it kept needing re-deriving mid-session instead of being
looked up.

Added one paragraph to `CLAUDE.md`'s routing table, right after the `.claude/work/*.md` row: `/done`'s
sweep goes to `main` immediately regardless of whether the task's own code PR has merged. The code PR
and the doc close-out are two independent tracks — the PR carries the code, the docs carry the record
that the task is closed — and holding the docs for someone else's review schedule would recreate the
exact stall `/done` exists to avoid.

*#28 · raised 2026-08-06 — Michael.*

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

### Settled at the joint meeting of 2026-08-04

Items 14 through 19 are all resolved. This block is the disposition; the full original text of each
follows below, relocated from **Open** on 2026-08-04 19:20 with every stamp intact.

*(Relocating them was deliberately deferred at first — moving an entry is an in-place edit of a
union-merged file, the concurrent-edit hazard item 19 documents. It was done once both owners were
present at the meeting, which satisfies the announce-first rule that same item established.)*

| # | Disposition agreed 2026-08-04 | Reasoning now lives in |
|---|---|---|
| 14 | Closed — GitHub #10 landed via PR #30, so the file-overlap warning is spent | GitHub #10, PR #30 |
| 15 | **C++ convention adopted.** `length` counts the burnout step, `profile` carries a trailing zero, `spread` unchanged. Spec §5.2 amended; an issue goes to Michael to correct `get/src/sir.rs` | `decisions.md` 2026-08-04 17:40; spec §5.2 |
| 16 | **Option B adopted.** The objective erases to `Box<dyn Fitness>` before dispatch, collapsing it to strategy × genome. Spec §1 and §8 amended | `decisions.md` 2026-08-04 17:42; spec §1, §8 |
| 18 | Closed — the trace served its purpose; no action was ever required | this table |
| 19 | **Both halves settled.** Routing: code via branch+PR, `.claude/work/*.md` direct. In-place amendment: **announcing it here first is a rule, not a courtesy.** `merge=union` narrowed to `decisions.md` and `collab.md` only | `decisions.md` 2026-08-04 18:25; `CLAUDE.md`; `/.gitattributes` |
| — | **New:** `CLAUDE.md`'s "an agent never merges a PR at all" reworded to "never merges **unprompted**" — overridden twice in one day, both times correctly | `CLAUDE.md`, "Pull requests" |
| 17 | **Re-roll ported from the C++, both constants exposed** as `min_epidemic_length` (default 3) and `max_epidemic_retries` (default 5). Spec §5.2 and §7 amended | `decisions.md` 2026-08-04 17:52; spec §5.2, §7 |
| — | **New, not previously an item:** epidemics within one evaluation run sequentially | `decisions.md` 2026-08-04 17:41; spec §5.2 |
| — | **New, not previously an item:** network size × population size × replicates multiply into memory; the Python layer must document it | `decisions.md` 2026-08-04 17:43; spec §8.1 |

**Nothing is left Open after this meeting.** Items 14 through 19 are all dispositioned above.

*Meeting block · 2026-08-04 17:45 — Michael & James.*


### Items 14–19, moved from Open on 2026-08-04 19:20 — Michael

Full text, relocated intact once the joint meeting settled all six. The disposition table above
is the summary; these are the originals, every stamp preserved and nothing edited.

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

**Newer `Graph.cpp` checked, 2026-08-04 11:27 — Michael.** The updated graph class narrows this
item rather than closing it. Its `SIR` is now `int SIR(int p0, double alpha, vector<int> &epiProfile,
int &totInf)` — it returns the length, fills the profile, and fills the total infected, which is the
same three-reading shape as `SirRun`. Against `get/src/sir.rs`:

- **`totInf` matches our `spread` exactly.** Seeded at 1 for patient zero and incremented by
  `curInf` each step (`legacy/Graph.cpp:98-102, 148`). Lone patient zero gives 1; a 6-node path at
  rate 1.0 gives 6. No disagreement on this reading at all.
- **`length` and `profile` still differ, exactly as described above.** `epiLen` increments on the
  final burnout pass and `epiProfile[epiLen] = 0` is written on it (`legacy/Graph.cpp:147-149`), so
  `epiLen` equals our `profile.len()`, and the C++ profile is one longer than ours. Unchanged, but
  now confirmed against the current C++ rather than the older copy.

So the decision needed is narrower than it looked: only the length convention and the trailing zero
are in question, and `spread` is agreed by both implementations. *(Reply inside #15 · 2026-08-04
11:27 — Michael.)*

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

### 18. PR #32 was self-merged unreviewed, under the exception in the rule it is delivering

**FYI, no action needed — this is the trace the rule asks for, not a request.** Recorded because
`CLAUDE.md`'s new "Pull requests" section says an unreviewed merge must leave one.

**What happened.** PR #32 carries two documentation commits that were pushed to `mdube_sir_sim`
*after* PR #31 had already merged, so they never reached `main`: four `decisions.md` entries and one
`traps.md` entry. It is `.claude/work/` only — no code, no `settings.json`, no `hooks/`. Michael
merged it himself on 2026-08-04 15:27 UTC, invoking the rule's own exception: James had merged #31
and moved on, and the stranded entries include the decision record for #31 itself, which is the
thing a reader of `main` would go looking for first.

**The irony is deliberate and worth naming**, because it is the honest version of what happened:
the rule forbidding self-merges was itself delivered by #31, and the first PR after it was
self-merged. That is the exception working as designed rather than the rule being ignored — but it
is exactly the pattern that becomes a habit if it is not written down, which is why this entry
exists.

**Not a precedent for code.** The exception was taken on a docs-only change where the review value
is the union-merge interleave check, and that was run by hand: the audit
`grep -vE '^\s*$' .claude/work/<file>.md | sort | uniq -d` was clean on `collab.md`, `decisions.md`,
`issues.md`, `traps.md` and `hotfixes.md` before merging. A PR touching `get/src/` would not qualify.

**Also still open after #31:** issue #16 remains open, because `Closes #16` was added to the PR body
after the merge and GitHub applies closing keywords only at merge time. Recorded in `traps.md`.

*#18 · raised 2026-08-04 15:27 — Michael.*

### 19. Union merge has a second silent failure, and the GitHub merge button disables it entirely

**Decide, at the next meeting — two findings, both measured 2026-08-04, both affecting how we merge
every day.** Recorded in `traps.md` and `CLAUDE.md` already; this item is for agreeing the working
practice, not for reporting the mechanism.

**Finding 1 — GitHub's web merge does not apply `merge=union`.** `.gitattributes` merge drivers are
run by your git, not by GitHub's servers, and this holds even for `union`, which is built into git
rather than custom. Verified three ways against PR #30: locally with `.gitattributes` present,
`Auto-merging`, zero conflicts; locally with it removed, `CONFLICT (content) in decisions.md`;
GitHub's API, `mergeable=false, mergeable_state=dirty`. GitHub reproduces the no-driver case
exactly. The consequence is that clicking Merge on any PR touching `.claude/work/*.md` drops you
into the web resolution editor, hand-resolving an append-only log in a textarea — which is precisely
how one side's entries get lost. **This interacts badly with the new PR rule**, which sends us both
to that button.

**Finding 2 — union silently *duplicates* a line when both sides edit the same one.** The dedup
failure we already knew about removes byte-identical lines. This is its inverse: two branches
editing the same existing line keeps **both**, one after the other, reported as
`1 file changed, 1 insertion(+)`. On a 250-line file, so not a small-file artifact. The realistic
trigger is two people closing out the same task, one striking a status and one superseding it.
**Authorship turns out to be irrelevant** — union does not know who wrote a line, so "editing your
own entry" is safe socially and buys nothing mechanically. What matters is whether both sides
touched the region.

**What I have already written down, and what I think needs agreeing:**

- `traps.md` gains both entries, `CLAUDE.md`'s "Pull requests" section gains "merge locally when the
  PR touches `.claude/work/*.md`", and the union-formatting rules gain a fifth: append, do not edit
  in place; if an entry must be amended, raise it here first.
- **For the meeting:** whether "raise it in `collab.md` before amending an entry" is a rule or a
  courtesy. It is currently the only mechanism preventing the concurrent-edit case, since git will
  not warn us — but it is also friction on a common, usually-harmless action.

**Related, and not a criticism:** James's PR #30 amends the jointly-stamped 2026-07-31 `decisions.md`
entry in place, striking its status. **That edit was safe** — nobody was editing the same line — and
I said as much after checking. I had earlier called it lucky; that was wrong and this item corrects
it. It is the concurrent case that is dangerous, not the in-place edit by itself.

*#19 · raised 2026-08-04 15:53 — Michael.*

**Michael, 2026-08-04 15:55 — partly settled on my side, one part still for the meeting.** The
route question is answered: **all code solving an issue goes through a feature branch and a PR** —
`get/src/`, `Cargo.toml`, `config.example.toml`, plus `settings.json` and `hooks/` as before, and
the spec sheet only after a joint meeting. `.claude/work/*.md` may be pushed to `main` directly,
because a trap that is not on `main` protects nobody and the one thing review catches in them has
its own audit command. Written into `CLAUDE.md` under "Pull requests"; push back there or here if
you want it drawn elsewhere. **Still open for the meeting:** whether announcing an in-place
amendment here is a rule or a courtesy. *(Reply inside #19 · 2026-08-04 15:55 — Michael.)*

### 22. I pushed two `CLAUDE.md` conventions straight to `main` — one of them binds your code too

**Flagging, not asking permission — push back and I will revert either.** `CLAUDE.md` says to prefer
a PR when a change binds the other owner's practice. I judged two lines not worth a branch and a
review cycle, so they went direct; this entry is the trace the rule is really after.

**1. "Prefer explicit loops to iterator chains" — this one binds you.** Plain `for` with an
accumulator over a chain that needs a turbofish, a closure returning through an `Option`, or more
than about two adapters. Also: keep comments terse and link `official_spec_sheet.md` rather than
restating it. The reason is not taste — it is that I do not write Rust, and I have to be able to
review every line you land. `runs.iter().map(read).sum::<f64>() / runs.len() as f64` stops me cold
where the four-line loop does not. Applied to my #17 code already; comments across the two files
went from 347 lines to 290. Full reasoning in `decisions.md` 2026-08-04 22:12. **If this is too
blunt for your code, say so and we will scope it to shared files or drop it.**

**2. "Approving a plan is never authorization to commit, push, or open a PR."** Binds agents rather
than you. I opened PR #39 unprompted this session because the approved plan's step 8 said "open the
PR", and closed it again. `/start` writes outward actions into `plan.md` as tasks, which makes the
existing "don't commit or push unless asked" rule look satisfied when it is not.

**Also worth knowing, and not a `CLAUDE.md` change:** `cargo clippy -- -D warnings` cannot pass on
`main` — two dead-code errors in `generational.rs` from your unbuilt #25. Confirmed pre-existing by
stashing. Issue **#22**'s `Verify by:` asks for a clean clippy, so it is unachievable as written;
I have corrected that issue body and recorded the baseline command in `traps.md`.

*#22 · raised 2026-08-04 22:30 — Michael.*

### 23. The `uniq -d` audit came back clean on a corrupted `collab.md` — it cannot see a splice

**Decide — this one changes what you and I both do, so I have not touched `CLAUDE.md`.** Not
blocking. Placed at the end of the file rather than under **Open**, following where you put item 22.

Measured on `main` today, twice, and the second one is the problem. Your item 20 (the PR #37
self-merge trace) was union-merged into the **middle of a line** of my item 20 — the join landed
right after the `` - ` `` opening my first bullet, so your heading absorbed my bullet prefix and my
sentence resumed twenty lines later. Neither entry was readable, and yours was not a top-level item
at all: `grep '^### '` did not list it. Repaired in `f652df1`; your text is byte-identical, verified
by `diff`, and only its position changed.

**What matters is that the documented check passed on that file.**

      grep -vE '^\s*$' .claude/work/collab.md | sort | uniq -d   # returned NOTHING

A splice **repeats no line**, so `uniq -d` is structurally blind to it. It finds the two failures we
already knew about — byte-identical lines being deduplicated, and a concurrently-edited line being
doubled — and cannot find this third one. I only caught it because I happened to read the file.

**Two places still present that command as sufficient**, which is the ask here:

- `CLAUDE.md`, "Formatting for union merge" — "Anything it prints is a line two entries could
  collapse onto", with no mention of what it misses.
- `collab.md`'s own header, "Formatting — one rule that bites" — "Audit before pushing and after any
  merge; anything it prints could collapse."

**What I suggest**, if you agree: keep `uniq -d` and add a structure check beside it in both places —

      grep -n '^### [0-9]' .claude/work/collab.md   # every heading at column 0; count as expected

An item heading that appears mid-line, or one you know exists but which this does not list, is the
splice. The full mechanism is already in `traps.md` as
`union-merge-splices-entries-without-duplicating`; this item is only about the two places that send
people to the insufficient command.

**Also worth deciding, since it caused the collision:** we both numbered an item **20** today,
because numbering is "one higher than the last" and neither of us had pulled. Yours is still
numbered 20 and I have not renumbered it — that is your entry to change.

*#23 · raised 2026-08-04 18:24 — James.*
