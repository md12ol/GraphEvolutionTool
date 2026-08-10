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

**Run this second check too — the first one cannot see a splice** (added 2026-08-09, item #23).
Union merge can graft one entry into the middle of a line of another, which duplicates no line, so
the audit above returns clean on a corrupted file. It happened here on 2026-08-04:

      grep -n '^### [0-9]' .claude/work/collab.md | wc -l   # then eyeball the list itself

A heading that shows up mid-line, or one you know exists but which this does not list, is the
splice. Two formatting notes, both learned by tripping over them while adding this paragraph:
indented rather than fenced, because a second ```bash fence makes the `uniq -d` audit print the
fence lines forever; and worded differently from the identical command inside item #23, because two
byte-identical lines are the very thing the audit is looking for.

Full rules: `CLAUDE.md`, "Formatting for union merge".

Persistent: survives `/done`, because coordination outlives any one task.

---

## Open

**Nothing is open.** Items 1–19 were settled at the joint meeting of 2026-08-04 and items 20–38 at
the meeting of 2026-08-09; all of them sit under **Settled** below, bodies and stamps intact, behind
their disposition tables. **Append the next item as 40**, numbering continuing across both sections.

*(Item 39 — the trace of that meeting's self-merge, two direct spec pushes and the force-push — was
raised and settled the same day, and sits at the very end of this file. It needed no decision once
both owners had been through all three in the room.)*

### 40. The `/done` skill body told the agent not to push, contradicting `CLAUDE.md` — corrected

**FYI, no answer needed — but your next `/done` will behave differently, so read this before you
run one.**

**What was wrong.** `.claude/skills/done/SKILL.md` ended its Constraints section with a bare
"Do not commit or push." `CLAUDE.md`'s PR-routing section says the opposite, in detail: the `/done`
sweep — the task-complete marker in `decisions.md`, `hotfixes.md`'s `Last checked` stamps,
`traps.md` updates, and the archive directory itself — "commits and pushes straight to `main` right
then". That is the rule your own item **#28** established on 2026-08-06, closing #22 while PR #43
was still open. The skill body was the template's generic default and had simply never been
reconciled with it.

**What it cost.** `/done epidemic-seeding` ran to completion on 2026-08-10 and then stopped with
the archive sitting unpushed in my working tree. `work/archive/` is tracked precisely so a finished
task's record reaches you — left uncommitted it reaches nobody, and the failure is silent, because
a `/done` that archived-but-did-not-push looks completely successful from inside the session.
Michael caught it by asking; nothing in the skill would have surfaced it.

**The fix, and the part worth reading twice.** The constraint is struck through in place and
replaced, per this repo's supersede-don't-overwrite convention, so the reversal trail survives. The
replacement says the close-out belongs on `main` — **but that the agent must ask before pushing it,
every time.** Michael corrected me on exactly this while I was writing the entry: my first draft
had `/done` push automatically as its last step, which would have quietly created a standing
authorization inside a skill. `CLAUDE.md` already forbids that shape — "every commit, push and PR
needs its own explicit instruction, each time, no matter what the plan says" — and its stated
reason is precisely that a plan or skill step reading "push" is what makes the rule *look*
satisfied when it is not. Reaching the end of `/done` is not the instruction.

So the corrected behaviour is: `/done` prepares the close-out, reports, and **stops with a
question**. It also restates that the close-out is docs-only and needs no PR, and that **the task's
own code still goes through its branch and PR** — two independent tracks, which is the part most
worth not losing.

**Route taken:** direct push to `main`, no PR. Skill **bodies** are the direct-push row of the
routing table; only frontmatter (`model:`, `allowed-tools:`) needs review, since that is what
changes what executes on your machine. This change is prose the agent reads. It is also close to
the self-merge rule's case 2 — it subtracts a line that was already false — though as a body edit
it never needed a PR for that argument to matter.

**Amended 2026-08-10 22:10 — Michael: the ask above is upgraded from FYI to ACKNOWLEDGE.**
Appended rather than rewritten, because the original is already on `main` in `2f94dc7` and editing
a line that has shipped is the case union merge duplicates without telling either of us.

**Please reply in this item confirming you have read it.** The FYI framing undersold it: this
changes what `/done` *does* on your machine on your next pull, without you having read the diff —
the same property that makes `settings.json` and `hooks/` PR-only under rule 2. It went direct to
`main` because a skill body is prose rather than configuration the harness executes, which I still
think is the right route, but "right route" and "no acknowledgement needed" are different claims
and I conflated them. Concretely: your next `/done` will prepare the close-out, report, and **stop
to ask** before committing and pushing it to `main` — where the old body told it not to push at
all, leaving the archive stranded in your working tree.

*#40 · raised 2026-08-10 21:33 — Michael · amended 2026-08-10 22:10 — Michael.*

**Read and acknowledged. 2026-08-10 18:47 — James.** Both parts: the corrected constraint, and
`/done` stopping to ask rather than pushing on its own.

**The catch you made on yourself is the more important half.** A skill step reading "push" is
indistinguishable, from inside the session, from having been told to push — which is the exact
failure `CLAUDE.md`'s "every commit, push and PR needs its own explicit instruction" was written
against, after PR #39. Putting the authorization inside the skill would have made the rule
unenforceable by the only party it constrains, and it would have looked fine for months.

**On the stranded archive:** worth noting the failure had no local symptom at all. A `/done` that
archives and does not push is indistinguishable from a successful one *on the machine that ran it*
— the archive is right there. It only exists as a defect from the other person's side, which is
why nothing in the skill could have surfaced it and why you found it by asking. That asymmetry is
the argument for the tracked `work/archive/` in the first place.

*(Reply inside #40 · 2026-08-10 18:47 — James.)*

### 41. I amended the sheet outside a meeting: `express_and_score(population, …)` → `(batch, …)`

**ACKNOWLEDGE, please — reply in this item so there is a record you have seen it.** Not a
question about whether it was right; a request that you confirm you know it happened, because it
is a sheet change that did not go the documented route and I do not want it discovered at merge
time.

**What I changed.** In #52's PR, on top of the two renames the 2026-08-09 meeting agreed:

- `get/src/evolver/common.rs` — `express_and_score`'s first parameter `population: &[G]` is now
  `batch: &[G]`, with its doc prose and the test name `..._of_an_empty_population_...` following.
- `official_spec_sheet.md` — **line 257** (the §5.1 signature), **line 274** ("the sole path from
  a population to a set of fitnesses" → "from a batch of genomes to …"), and the §5.1 diagram at
  **line 334** ("express population in parallel" → "express the batch in parallel").

**Why it belongs with #52.** It is the same defect #52 exists to fix, one layer up. The unit
`express_and_score` receives is a **batch, whose size varies**: `steady_state.rs:76` calls it with
exactly **two children**, and the sheet's own §5.1 invariant sentence had it mapping "a population"
to fitnesses four lines above a signature that steady-state contradicts on every mating event.
Renaming `Fitness::evaluate_population` while leaving its sole caller's parameter named
`population` would have fixed the identifier and left the misnomer sitting on top of it.

**Why it is nonetheless a departure, stated plainly.** `CLAUDE.md` says the sheet changes only at
a joint meeting — "not by one owner mid-task, not by an agent" — and that an agent finding the
sheet wrong "writes a `collab.md` item, it does not fix the sheet". #52's scope was enumerated and
verified on 2026-08-09 and names **two** identifiers; this is a third, so the meeting's
authorisation does not stretch to cover it. Michael directed the change during the session after
the agent flagged the conflict and declined to route around it. It is deliberate, not an oversight.

**What I am not claiming.** Not that this is the self-merge rule's case 2 — it *adds* a naming
claim rather than subtracting a falsehood, so that exception does not apply. You still merge the
PR, and you can reject this part of it without touching the other two renames; it is confined to
`common.rs` and three sheet lines.

*#41 · raised 2026-08-10 22:04 — Michael.*

**Acknowledged and ratified as it stands. 2026-08-10 18:52 — James.** Seen, and I am not asking
for a revert or a re-raise. The change is right: `steady_state.rs:76` passes two children, so the
sheet's own §5.1 invariant contradicted the signature four lines below it, and renaming
`evaluate_population` while leaving its only caller's parameter named `population` would have left
the misnomer one layer up.

**What I am ratifying is the outcome, not a new route.** The joint-meeting rule stands unchanged —
this does not become the precedent that a sheet edit may ride along with a PR whenever the code
makes it convenient. I am closing this one on its merits and because backing out a merged rename
costs more than it buys, not because the rule bent.

**The thing that makes it ratifiable is what you did around it.** You raised it as its own item,
named the departure in the rule's own words, said the meeting's authorisation does not stretch to
a third identifier, and declined the self-merge case-2 defence yourself rather than making me
notice it did not apply. A departure that arrives already labelled is a different object from one
that is discovered — the first is a decision I can accept or reject on the facts, the second is an
erosion. If you had folded it silently into #52 I would be asking for the revert.

*(Reply inside #41 · 2026-08-10 18:52 — James.)*

### 42. Proposal: an SDA run's best graph should feed an edge-edit run as its base graph, automatically

**PARKED — no answer needed now, and nothing is blocked on it.** Recording the idea and its open
questions while the reasoning is fresh, for a meeting whenever we next want it. Likely future work
rather than current. Not filed to the tracker: a GitHub issue nobody is scheduled to pick up is
just a second copy of this that drifts.

**This item is the park, because the sheet has nowhere to put one.** Worth stating, since the
obvious instinct is to note it in `official_spec_sheet.md` and neither section fits: **§9** opens
"Open decisions — **none**" and asserts everything raised while writing the sheet is settled, so
parking a live question there contradicts the section's first word; **§10 Non-goals** is for
deliberate exclusions, "stated so their absence reads as a decision rather than an oversight",
which would say we decided *against* this. It is desired, just not now — a third thing the sheet
has no section for. Adding one would itself be a sheet change needing a meeting, so `collab.md` is
where this lives until it is genuinely picked up.

**The want, in one line.** Run SDA to generate a network from scratch, then run edge-edit to
refine it — with the first run's best graph becoming the second's `base_graph` **automatically**,
not by the user exporting an edge list and hand-feeding it back in.

**Why it fits the existing design.** The two genomes are already complementary and the sheet
already says so: §3.2's `SdaGenome` generates "a graph from scratch", while §3.1's
`EdgeEditGenome` is "an edit script over a base graph" and is explicit that "the same genome
expressed against a different base graph gives a different result". §3.1 also notes that five of
the nine operations — `Swap`, `Hop`, and the three `Local*` — are **inert on an empty base
graph**, so a from-scratch edge-edit run wastes its early generations building structure those
operators need. An SDA-generated starting network is exactly the structure that makes them useful
on generation 1. The pieces exist; nothing joins them.

**What the sheet does not currently support, and would have to say.** These are the questions I
want us to settle together, not answers I am proposing:

1. **Is this one run or two?** It bears on §10's non-goals. "Fixed-length runs" and "One
   population" both read as within-a-run statements; a two-stage pipeline is arguably two runs in
   sequence rather than a violation, but the sheet should say which, or the next reader will treat
   the feature as contradicting a non-goal.
2. **Config shape.** `[genome]` currently picks exactly one of `edge_edit | sda` (§7). A chain
   needs a way to express both, in order — an array of stages, a `[pipeline]` section, or a
   `base_graph = "previous_stage"` sentinel. This is the biggest open question and it drives §7's
   validation rules.
3. **Which graph carries over.** §6.2 was amended on 2026-08-09 to best-of-final-population. The
   chain should presumably hand over exactly what §6.4's "best individual" reports, but it should
   be stated rather than inferred.
4. **Replicate semantics (§8.1).** With `n` replicates, does replicate `i` of the edge-edit stage
   consume replicate `i` of the SDA stage, or do all edge-edit replicates start from one shared
   SDA winner? These give different variance structures and different reproducibility guarantees,
   and picking wrong silently makes the across-run band in §6.4 mean something other than it says.
5. **Consistency constraints that must become validation.** `network_size` has to agree across
   stages (§10, "Fixed node count"), and so does `max_edge_multiplicity` — §3.2 derives the SDA
   alphabet from the cap (`num_chars = cap + 1`), so a stage-2 cap below stage 1's would clamp
   weights the first stage deliberately produced.
6. **Same objective for both stages, or one each?** Refining toward a different objective than the
   one that generated the network is a legitimate thing to want and a very different feature.
7. **Log and provenance (§6.4).** One log per run today. A chained run needs stage identity on
   every row, or two logs, and the generated-TOML provenance record has to capture both stages.

**One implementation note that is already in the tree.** `get/src/config.rs:337` refers to
`set_base_graph` as the thing that owns base-graph validation, and **no such setter exists yet** —
`grep -rn set_base_graph get/src/` returns only that comment. §8 also says a base graph "is better
delivered through a setter than serialized into the document". So the delivery path this feature
would extend is itself unbuilt; whoever specs this should decide whether the chain writes through
that setter or bypasses it.

**Not urgent, and not blocking anything current.** Raising it now so it is captured while the
reasoning is fresh, not to interrupt #52 or #51.

*#42 · raised 2026-08-10 22:06 — Michael.*

### 43. Filed GitHub #56 — sweep both evolvers for divergent style and duplication. Open to anyone

**FYI, one open assignment, and one question for the next meeting.** No action needed before then.

**Why it exists.** #51 (PR #55) fixed **one** instance of a pattern rather than the pattern: both
`outcome` methods asked "which index is best?" in two different spellings, and the fix was a shared
`common::best_index`. While scoping that, it became clear the rest of `generational.rs` and
`steady_state.rs` have the same problem in several more places, so #56 is the sweep — two questions
asked of every parallel site: same job different spelling (unify, or say why not), and same job
same code (delete one, share it).

**It is evidenced, not a hunch.** Verified with `diff` on `mdube_best_index` at `79c10aa`:
`best_of` and `mean_of` are **byte-identical** twenty-line test helpers in both files; so is the
four-line `ChaCha8Rng` rationale comment in both `run` methods; so is the `Self { .. }` tail of
both `new` methods. The `Val` and `Walk` test genomes match in code and differ only in doc-comment
wording. History row 0 is seeded by the same two lines but from `run` in one file and `evolve` in
the other, for no stated reason.

**The standard #56 asks for already exists in these files**, so nobody needs to invent one — both
`new` methods do it right, and generational's says outright that it has no matching
`population.len() >= tournament_size` assert *because* it samples with replacement. Either match,
or differ and say why, naming the other file.

**Assignment: deliberately unassigned, take it if you want it.** It has no dependency on either
owner's in-flight work once the current set is clear.

**Sequencing: staged behind the currently open issues**, per Michael's instruction when raising it.
It is a cleanup pass over two files that several open issues still touch, so rebasing it underneath
them is wasted work. It also wants to land after PR #55, which removes one divergence it would
otherwise re-find. Note this staging lives only in #56's body and in this item — no milestone was
set, since Projects (classic) is half-broken on this repo and a label would have been invented as a
side effect.

**The meeting question — this is the part that needs both of us.** Are there other file pairs that
deserve their own version of #56? The two evolvers are the obvious pair because they implement one
trait two ways, but they are unlikely to be the only place two files answer one question in two
spellings. Two candidates to discuss, neither investigated: `config.rs` and `py_config.rs`, which
mirror each other by construction — though `traps.md` already records that the mirror **cannot** be
collapsed, which makes it a different situation worth separating explicitly rather than assuming;
and the genome implementations. If the answer is yes, each pair gets its own issue rather than
being folded into #56.

*#43 · raised 2026-08-10 22:55 — Michael.*

### 44. `/start` now requires the branch as task 0 — please acknowledge, it changes your `/start` too

**Acknowledgement wanted, not a decision.** This is a skill *body* change, so the routing table
permits the direct push it went in on, but it binds how your `/start` behaves on your next pull and
you did not read the diff. Hence this item rather than silence.

**What happened.** Starting GitHub #26 on 2026-08-10, the agent wrote a `plan.md` whose six tasks
all edit `get/src/` — and no task created a branch. The tree was still clean and on `main` when I
caught it, so nothing was lost, but the next step in that plan was an edit to `config.rs` on `main`,
which the routing table forbids outright.

**Why the existing rules did not catch it.** The routing table in `CLAUDE.md` is written in terms of
**pushing and merging**, which are several steps downstream of the first edit. A plan can therefore
comply with every line of that table and still put the first `get/src/` change on `main`, because
nothing in the planning step asks the question. `/start`'s body went straight from "agree the
objective" to "write the plan" and mentioned branches nowhere. That is a gap in the *skill*, not a
rule anybody broke.

**The change.** One bullet added to `/start`'s §3 task rules, beside the existing "every task needs
a `Verify by:`": if any task touches something the routing table sends through a PR, the **first
task is creating the branch**, with `git rev-parse --abbrev-ref HEAD` as its verify-by. Put beside
the verify-by rule deliberately — it rides a discipline we already have rather than adding a new
one, and it makes the branch a checkable task instead of an assumption.

**Nothing else changed** — no frontmatter, no `allowed-tools`, no `model:`, so what executes is
unchanged and this needed no PR under the 2026-08-09 split in `collab.md` #34. Only the prose the
agent reads is different.

**For you:** just say you have seen it. If you would rather this had gone through a PR anyway, say
so and I will route body changes that bind practice — as opposed to fixing typos — that way in
future; the `.claude/CLAUDE.md` row already draws exactly that distinction for itself, and it may
belong on the skills-body row too.

*#44 · raised 2026-08-10 23:15 — Michael.*

**Seen and acknowledged, and the gap is real — this session was sitting in it. 2026-08-10 18:56 —
James.** #53's own `plan.md` had the branch as **task 2**, after the objective and after a
docs-only cleanup task. It came out fine only because task 1 happened to touch nothing the routing
table covers. Under the new bullet it would have been task 0, so your #26 near-miss is not a
one-off — two of two plans written before the change had the same shape.

**Your diagnosis is the part worth keeping.** The routing table is written in terms of *pushing and
merging*, and the first edit is several steps upstream of both. A plan can satisfy every line of
that table and still put the first `get/src/` change on `main`. Hanging the fix off the existing
`Verify by:` discipline is right for the same reason — it makes the branch a checkable task rather
than an assumption, and `git rev-parse --abbrev-ref HEAD` either prints the branch or it does not.

**On your question — no PR, but I want the notification to stop being a courtesy.** Direct push
stays correct for skill bodies: a body is prose we both read anyway, and a review round-trip in
front of it is how the route starts getting skipped, which is the argument that drew the row in the
first place. What I do not want to rely on is you choosing to raise an item. So: **a skill-body
change that binds the other owner's practice — as opposed to fixing a typo or rewording for clarity
— goes direct to `main` *and* gets a `collab.md` item with an ACKNOWLEDGE ask.** Same test the
`.claude/CLAUDE.md` row already applies, one notch cheaper than a PR, and it closes the same gap:
the failure mode is not an unreviewed change, it is an unnoticed one.

**Concretely, that makes what you already did the rule** — items #40 and #44 are both exactly this
shape, and both were raised voluntarily. I will write the row into `.claude/CLAUDE.md` unless you
would rather it waited for the next meeting; it binds your practice as much as mine, so say if you
want it discussed rather than written.

*(Reply inside #44 · 2026-08-10 18:56 — James.)*

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

### Settled at the joint meeting of 2026-08-09

Every item that was open going into this meeting is dispositioned below. Following the precedent
set on 2026-08-04, this block is the disposition and the item bodies stay where they are under
**Open** for now — relocating eighteen long entries in one pass is the operation that spliced items
20 and 29 into each other, and the table below is what a reader needs.

| # | Disposition agreed 2026-08-09 | Where it landed |
|:---:|:----------|:----------|
| 20 | **Fixed.** The last stale glob now names `decisions.md` and `collab.md` rather than all five | `CLAUDE.md`, "Pull requests" |
| 21 | **Both extension routes supported.** Python for most users, a drop-in Rust objective for advanced ones — and it needs **no change to #26**, because `Evolver::run<F>` is generic so a Rust user never reaches dispatch | spec §5.3 (new); `decisions.md`; noted on GitHub #26 |
| 23 | **Agreed.** `grep -n '^### [0-9]'` added beside the `uniq -d` audit in both front doors, since a splice duplicates no line | `CLAUDE.md`; this file's header |
| 24 | **Inline `target_profile`, verbatim.** Replaces `target_profile_path`; **neither** C++ convention (patient-zero prepend, `/128` rescale) is reproduced | spec §8 amended; `decisions.md`; GitHub #53 |
| 25 | **Closed** by the reply inside it — `reject_fitness_seed` in `from_toml_str` is the right home | `decisions.md` 2026-08-06 00:07 |
| 26 | **Acknowledged**, no change. The commit-each-verified-step convention stands | `CLAUDE.md`, Conventions |
| 27 | **Ratified as deliberate.** `Swap` keeps its degree floor of 3; no code and no sheet change | `decisions.md` 2026-08-09 |
| 28 | **Acknowledged**, no change. `/done`'s sweep pushes direct regardless of an open code PR | `CLAUDE.md`, routing table |
| 29 | **Rule widened to two cases** — the second being a strict deletion or one-line correction that removes something already false | `CLAUDE.md`; `decisions.md` 2026-08-09 |
| 30 | **Settled.** Hook merged as PR #44 and reviewed; both halves complete | `decisions.md` 2026-08-06 |
| 31 | **Already applied by James** on 2026-08-06. Michael must not re-apply it | `traps.md`, the rustfmt entry |
| 32 | **Both renames agreed.** `evaluate_population` → `evaluate_batch`, `SirRun` → `Epidemic`; the sheet amendment rides in the same PR | GitHub #52 |
| 33 | **FYI only**, verified intact. `EpidemicScorer` is `new` / `next_batch_seed` / `mean_batch` | `decisions.md`; no action |
| 34 | **Frontmatter takes a PR, the body does not.** Written as a test — "does this change what runs", not "which directory is it in" | `CLAUDE.md`, routing table; `decisions.md` |
| 35 | **§6.2 amended to best-of-final** for both strategies, with the `elite_count = 0` case stated | PR #50; `decisions.md` |
| 36 | **Scoped down.** Only the argmin moves to `common::best_index`; the rest of `outcome` stays duplicated on purpose | GitHub #51 |
| 37 | **Verified on Windows.** Linking is fine; the runtime failure is `STATUS_DLL_NOT_FOUND` and the fix is `PATH`, not `LD_LIBRARY_PATH`. James's cargo-feature fallback is **not** needed | `traps.md` |
| 38 | **Stale and corrected** — #29 landed as PR #49, merged `0731aa6`, so its "not yet a PR" line no longer holds | see the note appended inside #38 |

**Three things came out of the meeting that were not on the agenda.**

1. **The spec status table was stale on four of nine rows** — `GenerationalEvolver`, the three SIR
   objectives, `Config` parsing and the Python interface all read "designed, not built" for
   components that had shipped. The sheet says status lives there "and nowhere else", so it was the
   only signal a reader had. Corrected in PR #50.
2. **Agent co-attribution was banned in this repo, not just in one home directory.** James wrote
   the rule into `~/.claude/CLAUDE.md` on 2026-08-03, which is global and per-machine, so it bound
   his sessions and reached nobody else; six commits made during this meeting carried a
   `Co-Authored-By` trailer before it was caught. History was rewritten to strip them and the rule
   now lives in the repo. **James: `git fetch origin; git reset --hard origin/main`** — `main` was
   force-pushed, so `pull_main.sh` will decline the fast-forward and print rather than fixing it.
3. **A correction to #25's reply, for James.** It states that a config built in Python "can still
   carry a stray `seed` attribute harmlessly". That looks wrong: every pyclass in `py_config.rs` is
   declared without `dict`, so a pyclass instance has no `__dict__` and `config.seed = 42` raises
   `AttributeError` rather than being silently carried. Reasoned from the declarations, not
   executed — worth two minutes to confirm before relying on either version.

*Meeting block · #20-#38 dispositioned 2026-08-09 — Michael & James.*


### Items 20–38, moved from Open on 2026-08-09 — Michael & James

Full text, relocated intact once the joint meeting settled all eighteen. The disposition table
directly above is the summary; these are the originals, every stamp preserved and nothing edited.
Relocated at the meeting with both owners present, which is what the announce-first rule on
in-place edits asks for.

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

**Done in #23, 2026-08-06 00:10 — James. One correction to what I said above, and it matters for
where you look for the check.** I wrote that it belonged in `Config::validate`, "which can look at
the raw text before it is deserialized". It cannot: `validate` takes `&self` on an already-parsed
`Config`, by which point the key is gone. The check went into **`Config::from_toml_str`** instead,
reading the raw text through a loose `toml::Value` parse before deserialization.

Two consequences worth knowing in #26:

- **It is TOML-only, by construction.** The Python front end has no text to inspect, so a config
  built in Python can still carry a stray `seed` attribute harmlessly. Acceptable, because the
  hazard is specifically an *old TOML file* — but do not assume the guarantee is universal.
- **It is `seed` by name, not a general unknown-key sweep.** Any other unknown `[fitness]` key is
  still silently ignored, exactly as before, and that narrowness is deliberate: a general sweep
  hand-rolls what serde does everywhere else and would start rejecting keys as the schema grows.
  Pinned by `an_unknown_fitness_key_other_than_seed_is_still_ignored`.

The mechanism you were told before is unchanged — serde genuinely cannot do this through a
`#[serde(flatten)]`. Reasoning and the rejected alternatives are in `decisions.md` 2026-08-06 00:07.
This closes the item unless you want it somewhere else. *(Reply inside #25 · 2026-08-06 00:10 —
James.)*

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

### 30. New `SessionStart` hook: `pull_main.sh` fast-forwards `main` automatically — needs your review

**Confirm — this runs on your machine too, at every session start.** Opened as a PR
(`mdube_pull-main-hook`), not pushed direct, per the routing table's own rule that `settings.json`
and `hooks/` changes always go through review.

**What it does:** on `main` only, fetches and fast-forwards to `origin/main` if it can do so
cleanly. On any other branch, or if `main` has diverged (local commits not on origin, or a dirty
working tree the incoming commit would overwrite), it prints one line and touches nothing — no
merge, no rebase, no discard. Verified all three paths in a scratch repo before opening the PR.

**Why:** `.claude/work/*.md` and `CLAUDE.md` route around a PR by design, so a change one of us
pushes direct only reaches the other's machine on their next `git pull main` — nothing was prompting
that. That gap is exactly how the two independent `collab.md` item-**20**s happened (see
#20-collision / #29-collision): both of us were reading our own stale copy when we each picked "the
next number." This closes the stale-window case; it doesn't and can't close true same-minute
concurrency.

Full reasoning and rejected alternatives in `decisions.md` 2026-08-06, "`pull_main.sh`: a
`SessionStart` hook fast-forwards `main` automatically." Please review the PR when you get a chance
— it's the one place in this session's work that binds your session start, not just a doc you can
skim later.

*#30 · raised 2026-08-06 — Michael.*

**Confirmed, 2026-08-06 00:55 — James. Reviewed and no objection; treat this as settled.** Read the
merged script at `.claude/hooks/pull_main.sh` rather than only the PR description. What I checked:

- **The guards do what the description says.** It exits 0 unless `git branch --show-current` is
  exactly `main`, so any feature branch is untouched — which is the case that matters, since a
  mid-task branch is where an unexpected move would hurt.
- **`--ff-only`, and every failure path is non-destructive.** Unreachable origin, divergent local
  `main`, dirty tree blocking the merge: each prints one line and exits 0. No merge, no rebase, no
  discard, no reset.
- **`set -uo pipefail` without `-e` is correct here, not an oversight.** The bare
  `[[ "$local_sha" == "$remote_sha" ]] && exit 0` test returns non-zero when the shas differ, which
  under `-e` would abort the script before the fast-forward it exists to perform.

Two things worth stating rather than leaving implicit. First, **it moves `main` under a session
that is sitting on `main`** — correct and wanted, but it means `git log` can differ between the
start of a session and the middle of one, which is worth knowing before it surprises someone.
Second, the item was **effectively already answered by PR #44 being merged**; this reply exists
because a merge is not a review, and #30 asked for the review.

Nothing to change. *(Reply inside #30 · 2026-08-06 00:55 — James.)*
**Status from my second machine, 2026-08-06 — Michael. The hook has merged, and this session was
the exact case it exists to prevent.** PR #44 is in as `e42ffde`, so `pull_main.sh` is live in both
trees now; your confirmation on the behaviour is still the open half of this item.

Worth recording because it is evidence rather than argument: I opened a session on my other machine
to close out #17 and found `main` **40 commits behind `origin/main`** — it had never seen PR #40's
merge, nor #41, #42, #43 or #44. The stale-window this hook closes is not hypothetical and is not
small. Two concrete consequences in that one session, before any work started:

- The local `decisions.md`, `traps.md`, `collab.md` and `hotfixes.md` were all two days stale. Since
  #33 narrowed the union glob, `hotfixes.md` no longer merges by union — appending to that stale
  base would have produced a genuine conflict, not a silent merge.
- `traps.md` carried a `cargo fmt` entry whose own exit condition ("drop once #43 merges") had been
  met on `origin/main` for hours. Working from the stale copy, I would have kept following a trap
  that no longer exists.

I re-read the script before pulling it in rather than trusting my own PR from the other machine, and
the three paths hold as described: it exits unless `branch == main`, it only ever runs
`merge --ff-only`, and the non-fast-forwardable case prints and returns 0 without touching anything.

*#30 · status update 2026-08-06 14:05 — Michael, from the second machine.*

### 31. One clause in your `rustfmt`-descends-into-submodules trap went stale when #22 shipped

**Announcing before touching your text, per rule 5 — the trap itself is right and stays.** Your
entry "Per-file `rustfmt` is not per-file on a `mod.rs`" closes with **"the tree is not currently
rustfmt-clean (that is exactly what #22 exists to fix), so a stray descent produces *real* diff, not
a no-op"**. #22 shipped as PR #43, and `cargo fmt -- --check` now reports zero offenders on `main` —
verified on `ed198c4` at 2026-08-06.

**What that changes and what it does not.** The mechanism is untouched: rustfmt still parses the
module tree and still descends into every `mod x;`, so `--config skip_children=true` is still the
right instruction and the `--skip-children` CLI note is still worth having. What changed is only the
stated *consequence* — on a clean tree a stray descent is now usually a no-op rather than real diff.
That makes the trap read as less urgent than it is, which is the wrong direction for a trap.

**Suggested amendment, yours to accept or ignore:** replace that final bullet with one saying the
descent is silent either way, and that a clean tree makes it *harder* to notice, not safer — the
diff that does appear is now the only signal, where before you might have caught it in the noise.
I have not edited a character; say the word and I will make exactly that change.

**Related sweep at the same gate:** I deleted the `cargo fmt` trap outright, under its own written
exit condition ("drop this entry once #43 merges"). Flagging it here because it is a deletion from a
shared churn list that you may go looking for.

*#31 · raised 2026-08-06 14:05 — Michael, at the sir-objectives `/done` gate.*

**Accepted, 2026-08-06 21:06 — James. Your reading is right and I have made the edit myself**, since
it is my entry and doing it from this side avoids the concurrent-edit case rule 5 exists to prevent
— do not also apply it. The replacement bullet says what you proposed: the descent is silent either
way, and a rustfmt-clean tree makes a stray one *harder* to notice rather than safer, because the
diff that appears is now the only signal where before it might have been lost in the noise. The
mechanism sentences are untouched, so `--config skip_children=true` still reads as the instruction.

Noted on the `cargo fmt` deletion too: correct under its own exit condition, and I found it gone
rather than went looking for it, which is the outcome the flag was for.

*(Reply inside #31 · 2026-08-06 21:06 — James, at the generational-evolver save.)*

### 36. Both evolvers now have their own `outcome`, and only one of them can be right about graphs

**Not blocking #25 — this is a question about where the shared part should live**, raised because
#25 shipped the second copy and the reason will otherwise leave with PR #46.

`GenerationalEvolver::outcome` and `SteadyStateEvolver::outcome` are now ~10 similar lines each,
differing on one point: steady-state re-expresses the winner with `best_genome.express(..)`, while
generational moves the winner's graph out of the vector its final `express_and_score` already
returned. That difference is deliberate and comes from the sheet — §6.2 asks generational not to
re-express, because it is the strategy that has the graphs to hand. Both return the identical graph;
`express` is deterministic, so it is a cost choice, not a behavioural one.

**The ask:** whether the common part (rank the fitnesses, clone the winner, `mem::take` the history)
should move into `common.rs` with the graph step left to each caller. I did not do it, because it
means editing `steady_state.rs` and #25 is scoped out of that file — and `collab.md` #14 already
records that overlapping edits to these files are how work gets silently overwritten here. Reasoning
for the generational side is in `decisions.md` 2026-08-06 21:03.

**Renumbered 2026-08-07 — James. This was written as #32 and is now #36.** Michael raised his own
#32 (the `evaluate_population` / `SirRun` renames) on `main` at 2026-08-07 14:28, hours before this
one was written, and mine sat uncommitted in the meantime so neither side could see the other. His
is the published number — it is referenced from #33 and from an `issues.md` entry — so this one
moved. Third collision after #20 and #29, and the first where the two items were *days* apart
rather than minutes: an uncommitted entry ages badly in a way an unpushed commit does not.

*#36 · raised 2026-08-06 21:07, renumbered from #32 on 2026-08-07 — James, at the generational-evolver save.*

### 32. Two spec-named identifiers misname the unit of work: `evaluate_population` and `SirRun`

**Needs a joint meeting, because both names are written into the sheet — and an issue once we
agree.** Not blocking #18; I have used the right words in the `fitness.rs` comments and renamed
only the identifiers the sheet does *not* name. Raising it rather than fixing it, per the rule at
the top of `CLAUDE.md`.

**The unit the engine scores together is a batch of graphs, and its size varies.** Generational
hands over the whole population per cycle; steady-state hands over the **two new children** per
mating event (`get/src/evolver/steady_state.rs:75-76`), plus its starting population once
(`:194-195`). The sheet already says this at line 509 and §6.3. So "population" is right in exactly
one of the three cases, and "generation" in none of the steady-state ones.

Two names contradict that:

- **`Fitness::evaluate_population`** — sheet line 221, again at 794 and 804. It is handed a batch of
  two for most of a steady-state run. `evaluate_batch` is what it does.
- **`SirRun`** — sheet line 368, the return of `sir_sim`. It is **one epidemic**, but "run" already
  means a replicate (`run_seed`, §8.1) *and* the API call `GraphEvolver::run`. Three meanings, and
  `run_seed` sits four lines from `|run| run.spread` in the same impl block. `Epidemic` is what it
  is.

**Why they are one item and not two:** identical shape — a spec-named identifier whose name
describes something narrower or other than the concept, where the fix is mechanical but the
authority is the sheet.

**What changes if we agree**, so the issue can be scoped rather than discovered:

| | `evaluate_population` → `evaluate_batch` | `SirRun` → `Epidemic` |
|:--|:--|:--|
| Sheet | lines 221, 794, 804 | line 368 |
| Code | `fitness.rs` (trait + 3 overrides), `evolver/common.rs:244` | `sir.rs`, `fitness.rs` |
| Prose | §5.1's "batch scorer" wording already agrees | §5.2's `SirRun { … }` block |

**Already done on my side, needing no meeting** — these identifiers are ours, not the sheet's:
`EpidemicScorer::mean_population` → `mean_batch`, `mean_from` → `mean_with_seed`, the counter field
→ `batches_scored`, and the comments throughout `fitness.rs` now say "batch of graphs" and name all
three shapes. That is on branch `mdube_epidemic_seeding`.

**My ask:** agree or reject at the next meeting. If agreed, I will file one issue covering both
renames and amend the sheet in the same PR. It is a pure rename with no behaviour change, so it
wants to land between workstreams rather than on top of an open branch — #19 and #25 both touch
these files.

*#32 · raised 2026-08-07 12:38 — Michael, while writing #18's comments.*

### 33. I restructured `fitness.rs` inside #18 — `EpidemicScorer` is 5 methods down to 2

**Done, not proposed — read this before you start #19, which lands `PyFitness` in the same file.**
On branch `mdube_epidemic_seeding`. No signature named in the sheet changed, so this needed no
meeting; I am telling you because it moves code you are about to edit, and because the *reason* is
worth having on the record.

**Why it happened.** Reading #18's own comments back, I could not follow my own file. Three
sub-agent reviews from different angles agreed on the split verdict, and it is not the one I
expected:

- **The seeding machinery is conventional and forced — untouched.** The counter-per-batch plus
  `mix_seed` derivation is the standard common-random-numbers scheme from simulation-optimization,
  and the counter-based seeding recommended for parallel reproducibility. The `AtomicU64` is not a
  smell: `Fitness: Sync` is required for the rayon fan-out, `Cell` is not `Sync`, and a `Mutex` is
  worse for something provably never contended. §8.1's reproducibility argument was checked and
  holds.
- **The wrapper layer above it was ours, and was earning nothing.** That is what I removed.

**What changed, all inside `get/src/fitness.rs`:**

| Before | After | Why |
|:-:|:-:|:-|
| `mean`, `mean_batch`, `mean_with_seed` | `mean_batch` alone | Three methods that all averaged; two were single-caller pass-throughs |
| `pub fn epidemics(graph, seed)` | inlined | One-line forward to `simulate_epidemics`, one caller, no external user |
| Drift between an objective's two entry points was silent | test `both_entry_points_use_the_same_reading` | Each objective still writes its reading twice, inline in `evaluate` and `evaluate_population` — a `reading` method was tried and reverted the same day, because the indirection cost more clarity than the duplication did for someone copying an objective to write their own. The test is the guard instead |
| `evaluate` had its own path via `mean` | `evaluate` calls `mean_batch(slice::from_ref(graph))[0]` | A single graph is a batch of one; same tick, same seeding, one code path |

**The trap I avoided, which is the part worth knowing if you touch this.** The obvious version is
`evaluate` calling `self.evaluate_population(...)`. That is a latent stack overflow: the trait's
**default** `evaluate_population` calls `evaluate`, so any objective forwarding without also
overriding the batch method recurses until the stack dies. Routing through `mean_batch` on the
scorer has no cycle. I also rejected a blanket `EpidemicReading` trait that would enforce the
pairing at compile time — it works, but it puts the real code in a blanket impl where neither of us
would think to grep, which cuts against our own "one owner does not write Rust" rule harder than the
duplication it removes.

**Evidence:** 162 tests green, `cargo fmt --check` clean. Two new tests lock in what the change
bought — `scoring_one_graph_ticks_the_counter_once_like_any_other_batch`, and
`both_entry_points_use_the_same_reading`, which fails if an objective's two entry points ever
disagree. Nothing outside `fitness.rs` was touched by this item.

**What it means for #19.** `PyFitness` overrides `evaluate_population` and does not use
`EpidemicScorer` at all, so nothing here blocks you — but the bottom half of the file moved. Pull
the branch before you start, or merge #18 first, rather than resolving it afterwards.

**Related, still needing the meeting:** the renames in **#32** are untouched by this —
`evaluate_population` and `SirRun` are sheet-named and stayed exactly as they are.

*#33 · raised 2026-08-07 14:28 — Michael, during #18.*

### 34. I pinned the five working-docs skills to sonnet — it changes execution on your machine

**FYI, no action needed unless you disagree.** Pushed to `main` as `011480d`. `model: sonnet` added
to the frontmatter of `done`, `load`, `save`, `setup` and `start`. They are bookkeeping skills —
read the working docs, update a plan, archive a task — and none of them needs the larger model.

**Why this is a `collab.md` entry rather than a silent push.** `CLAUDE.md` rule 2 requires a PR for
`settings.json` and `hooks/` because they execute on your machine at session start without you
reading them. `.claude/skills/` is not named in that rule and not in the routing table either, so a
direct push is permitted — but the *reason* behind rule 2 applies to it just as much: on your next
pull these five skills run under a different model than they did today, and nothing announces it.
So: permitted, pushed, and logged rather than left to be discovered.

**If you think skills belong under rule 2**, that is worth deciding rather than leaving to
precedent, and it would be a `CLAUDE.md` amendment binding both of us. I have no strong view; I
lean toward "yes, but only when the frontmatter changes", since the body of a skill is prose we both
read anyway.

**Revert is one line each** if any of them reads worse to you at this size — say so and I will drop
it, or drop it yourself, no discussion needed.

*#34 · raised 2026-08-07 16:52 — Michael, closing out #18.*

**Agreed, and your lean is the right line, 2026-08-07 — James. Frontmatter yes, body no.** No
objection to the sonnet pin itself; I have run `/load`, `/save` and `/done` under it this session
and noticed nothing.

The distinction that makes this workable is the one you already drew: **frontmatter changes what
executes on my machine without my reading it, which is precisely rule 2's stated reason, while the
body is prose we both read anyway.** `model:`, and any future `allowed-tools:` or hook-adjacent
key, should take a PR. Rewording a step inside a skill should not — that would put a PR round-trip
in front of typo fixes, and the rule would start being skipped, which is worse than not having it.

I am not writing the `CLAUDE.md` amendment in this session, since it binds your practice as much as
mine and the routing table is yours by authorship. If you write it, this reply is the agreement it
can cite; the one thing worth spelling out in it is that the test is *"does this change what runs"*,
not *"which directory is it in"* — otherwise the next person adds a fourth directory and we do this
again.

*(Reply inside #34 · 2026-08-07 — James, at the generational-evolver `/done` gate.)*

### 35. §6.2 says "track the best"; generational reports the best of the final population

**Decide — I think your code is right and the sheet's wording is stale, but the sheet is the
authority so it is not mine to call.** Found reviewing PR #46 against §6.2 after merging it. Not
urgent and nothing is broken at the settings we actually use.

**The gap.** §6.2 reads "score the population, log a stats row, **track the best**, then advance",
which reads as a running best carried across generations. `GenerationalEvolver::outcome`
(`get/src/evolver/generational.rs:115`) instead takes the best of the **final** population.

At `elite_count >= 1` the two nearly always agree, because the best is copied forward. At
**`elite_count = 0` they can differ**, and §7 permits zero — its only constraint is
`elite_count < population_size`, and `config.example.toml` happening to use 1 is not a guarantee.
In that configuration a strong individual can appear in generation 40, fail to be selected, and the
run reports something worse with no record that it existed. Your own test name concedes the edge:
`the_logged_best_never_worsens_while_an_elite_is_carried`.

**Why I think the code is right anyway, and this is the sheet's problem.** #18 landed the atomic
batch counter, so fitness is now genuinely stochastic between batches. A running best under a
stochastic objective is substantially a record of which generation drew lucky dice — and §6.2
itself rejects exactly that reasoning two paragraphs later, when it says freezing an elite's old
score would let a lucky draw persist (§5.2). Best-of-final is the more honest number, and it is
what `SteadyStateEvolver::outcome` already reports, so the two strategies agree with each other.
The sentence in §6.2 predates the seeding work.

**Three ways to settle it, cheapest first:**

- **Amend §6.2** to say the reported best is the best of the final population, for both strategies,
  and say why. No code changes. My preference.
- **Require `elite_count >= 1` in §7**, which makes the divergence unreachable rather than
  resolving it. Cheap, but it removes a legitimate configuration to dodge a wording problem.
- **Implement a running best**, and accept that under a stochastic objective the reported winner is
  chosen partly by its luckiest sample. I would not.

Either of the first two is a sheet change, so it needs the meeting — same one #32 is waiting on.

*#35 · raised 2026-08-07 16:59 — Michael, reviewing PR #46 after merging it.*

**Endorsed, 2026-08-07 — James: your first option, amend §6.2. Taking this to the meeting as a
position rather than a question.** I wrote the code and I reached the same conclusion from the other
direction — `decisions.md` 2026-08-06 21:04 records best-of-final and the running-best it rejected,
written before your review and independently of it. Two people arriving at the same reading from
opposite ends is the best evidence either of us has that the sheet's sentence is what is stale.

**On `elite_count = 0`, which is the honest part of your objection:** you are right that §7 permits
it and that the two readings genuinely diverge there. I do not think that argues for a running best.
It argues that a non-elitist generational GA *can* lose its best individual, which is a true fact
about the algorithm and not a reporting bug — the run really did end without that individual, and a
report claiming otherwise describes a population that no longer exists. What the amendment should
say is that the reported best is the best of the **final** population for both strategies, and that
at `elite_count = 0` this is a property of the configuration rather than of the report.

You are right that my test name concedes the edge; `the_logged_best_never_worsens_while_an_elite_is_carried`
is named that way because the guarantee genuinely is conditional, and I would rather the name say so
than have it read as unconditional and be quietly wrong at zero.

**Not endorsing your option 2** — requiring `elite_count >= 1` in §7 removes a legitimate
configuration to avoid amending a sentence, and non-elitist generational is a real thing people run
deliberately. Two sheet changes are on the table for the same meeting now, this and #36.

*(Reply inside #35 · 2026-08-07 — James, at the generational-evolver `/done` gate.)*

### 37. #19 changes how `cargo test` builds — and I could only verify it on Linux

**Heads-up before you pull, not a question.** PR for #19 moves `extension-module` out of
`get/Cargo.toml`'s `[dependencies]` and adds a root `pyproject.toml` that supplies it instead. This
changes your `cargo test`, and **I have verified none of it on Windows.**

**Why it had to change.** `extension-module` suppresses linking libpython, leaving the Python C API
symbols for the interpreter to resolve when it loads the module. Correct for the built wheel;
**fatal for `cargo test`**, which links an ordinary executable with nothing behind it — the entire
suite fails to *link*, with dozens of undefined `Py*` symbols, however few tests touch Python. #19
needed real tests calling a real callable, so the feature had to come off the library build. It now
lives in `pyproject.toml` as `[tool.maturin] features = ["pyo3/extension-module"]`, which is
maturin's own idiom, and `cargo build -p get --features pyo3/extension-module` is the by-hand
equivalent. Full mechanism in `.claude/reference/pyo3-maturin.md` §1.

**What you may hit, in rough order of likelihood.** `[dev-dependencies] pyo3` now carries
`auto-initialize`, so a test build wants a real Python that pyo3's build script can find. On Linux I
also need `LD_LIBRARY_PATH` pointed at libpython (pyenv build, not on the loader path) or the test
binary dies with `exit 127` before running anything:

    export LD_LIBRARY_PATH="$(python3 -c 'import sysconfig; print(sysconfig.get_config_var("LIBDIR"))'):$LD_LIBRARY_PATH"

Windows resolves Python differently and I do not know whether you need an equivalent, nothing at
all, or whether the cdylib link behaves differently without the feature. Two `traps.md` entries
cover the Linux side and say plainly that Windows is unverified.

**One measured thing that surprised me, so you do not draw the wrong conclusion from a green run.**
Dropping the `features` line entirely and rebuilding produced a wheel that was, on Linux,
indistinguishable — same 75 undefined `Py*` symbols, no libpython in `ldd`, and `import get` worked.
An earlier version of my own comment claimed it would fail to import; testing it is what caught
that. The line stays because macOS and Windows linkers reject undefined symbols by default, but **a
passing import on Linux proves nothing about your machine.**

**If it breaks for you, say so rather than working around it.** The fallback is to put the
pyo3-touching tests behind a cargo feature so your default `cargo test` never builds them — cheap to
do, and worth doing properly rather than you carrying a local edit. I would rather hear it than have
you discover it mid-task on #26.

*#37 · raised 2026-08-08 00:15 — James, before opening #19's PR.*

### 38. What #29 leaves for #26, and the `lib.rs` region we will both have touched

**Heads-up, not a question — worth reading before you start #26.** #29, the Python config front
end, is six commits on `jsargant_python_config` and not yet a PR. It edits `get/src/lib.rs`, which
is where #26 works, so this is the "source files genuinely overlap" case `CLAUDE.md` warns about
rather than a courtesy note.

**Where we collide, both in `lib.rs`:** `run`'s **docstring**, where I added the §8.1
memory-multiplication note the 2026-08-04 meeting assigned to #29 — its *body* is untouched and
still `todo!()`, so the textual conflict sits directly above the code you are replacing. And the
`#[pymodule]` block, which gains seven `add_class` lines for the config builders.

**Three things #29 hands you, each of which changes what #26 has to do:**

1. **`from_config` gives you a validated `Config` by the same path as the file front end.** It
   renders the Python objects to TOML and parses that through `Config::from_toml_str` and
   `Config::validate`, so by the time dispatch runs there is no Python-specific config shape left to
   handle — one `Config`, however it was built, and nothing in #26 needs to know which front end
   produced it.
2. **`python_fitness`'s `#[allow(dead_code)]` is still there and still yours to delete.** #29 did
   not touch it, and `hotfixes.md`'s entry stands unchanged: it goes when #26's `python` arm calls
   the method.
3. **`run`'s new memory note names `max_cores`, which does not exist yet** — that parameter is
   **#20**'s, mine. The note is written to spec §8.1 rather than to today's signature, so it
   describes a `run` taking a replicate count and a core cap. If #26 lands first the note reads
   slightly ahead of the code; that is deliberate, and #20 closes the gap.

**One standing obligation #29 creates, worth knowing before you add anything to the config schema.**
`config::FitnessConfig` and its neighbours now have a Python mirror in `get/src/py_config.rs`,
because pyo3 and serde cannot both annotate the same fitness enum: pyo3 rejects a unit variant in a
complex enum and directs you to an empty tuple variant, and serde's `tag` then rejects exactly that.
So a new field or variant in `config.rs` needs a matching one in the mirror. **It fails loudly
rather than silently** — the round-trip tests destructure `Config` exhaustively with no `..`, so an
unmirrored field is a compile error, and a new validation check with no attribute mapping fails
`every_validation_field_maps_to_a_python_attribute`, which scrapes `config.rs`'s own `invalid(...)`
call sites. Both guards were verified by deliberately breaking them rather than assumed to work.

*#38 · raised 2026-08-08 21:13 — James, closing out #29's implementation.*

**One line is now stale, corrected at the 2026-08-09 meeting — Michael. Everything else in this
item still holds and was re-verified.** The opening says #29 is "six commits on
`jsargant_python_config` and not yet a PR." It landed as **PR #49, merged 2026-08-09 as
`0731aa6`**, so the `lib.rs` overlap it warns about is a fact on `main` rather than a pending
branch: #26 now works on top of it instead of around it. James's wording above is left exactly as
written, per append-don't-edit.

Re-checked on `main` at `0731aa6`, all three still true: `from_config` at `get/src/lib.rs:87` routes
through `Config::from_toml_str` then `.validate()`; `python_fitness`'s `#[allow(dead_code)]` is
still at `lib.rs:303` and still carried in `hotfixes.md` for #26 to delete; and `max_cores` appears
only in `run`'s doc comments, never as a parameter, with `run`'s body still `todo!()`. The standing
`config.rs` ↔ `py_config.rs` mirror obligation is unchanged and is about to be exercised by
GitHub #53, which replaces `target_profile_path` with an inline array.

*#38 · stale line corrected 2026-08-09 — Michael, at the joint meeting.*


### 39. Trace: PR #50 was self-merged, and `main` was force-pushed — two things to know before you pull

**FYI, and one action for James at the bottom.** Two departures from the written route happened at
the 2026-08-09 meeting, both authorized by Michael in the room, both recorded here because the rules
they depart from ask for a trace rather than silence.

**1. PR #50 self-merged.** Michael's PR, merged by Michael, carrying spec §6.2's best-of-final
amendment and the four stale status-table rows. Both halves were agreed at the joint meeting that
produced them, so the review the PR rule exists to obtain had already happened; the status-table
half is additionally a strict deletion of text that had become false, which is the second permitted
case added to the self-merge rule earlier in that same meeting (#29). Merged **locally**, not with
the GitHub button, because it touched `decisions.md` — the `uniq -d` audit and the entry-heading
check were both clean after the merge.

**2. Two spec sections went direct to `main` rather than through a PR.** §5.3 (the two extension
routes) and §8's target-profile amendment. The routing table says the spec sheet takes a PR after a
joint meeting; Michael directed the direct push during the meeting itself, and both commit messages
say so explicitly rather than leaving it to be noticed.

**3. `main` was force-pushed** — this is the part that affects your machine. Six commits made
earlier in the meeting carried a `Co-Authored-By: Claude` trailer, because the rule banning it lived
only in `~/.claude/CLAUDE.md` on James's machine and could not be seen from Michael's. History was
rewritten to strip them, the rule now lives in the repo's `CLAUDE.md` where it binds both of us, and
every commit on `main` is authored solely by Michael. Content is byte-identical to before the
rewrite; only the messages changed.

**James: `git fetch origin` then `git reset --hard origin/main`.** `pull_main.sh` will *decline* the
fast-forward and print one line, which is correct behaviour and not a failure — it will not fix this
for you. Check nothing of yours is unpushed on `main` first; the rewrite base is `0731aa6`, PR #49's
merge, so anything before that is untouched.


**Moved here from Open on 2026-08-09 — Michael & James.** Both owners went through all three
departures in the room, so the item had no reader left waiting on it; it is kept in full rather
than deleted because it is the only single place that states them together. The per-change traces
also stand where they are load-bearing: PR #50's merge commit states the self-merge and why the
review it substitutes for had already happened, and each of the two spec commits states its own
route departure.

*#39 · moved to Settled 2026-08-09 — Michael & James, at the close of the joint meeting.*

### 45. Merged your #53 with one spec line unimplemented — filed as #58 rather than held

**FYI, and one thing to disagree with if you want to.** Reviewed
`jsargant_inline_target_profile` on Windows before merging: 216 tests pass, `cargo clippy
--all-targets -- -D warnings` is clean, and no `target_profile_path` survives anywhere in `get/`,
`config.example.toml` or `examples/`. PR #57 is in; GitHub #53 closed.

**The work is good and two parts are worth saying so about.** The reason non-finite elements are
rejected is the right one and is written down where the next reader will find it — RMSE against a
`NaN` or an infinity is `NaN` or infinite for *every* individual, so the population scores
identically and selection quietly stops discriminating. And
`a_whole_number_in_the_target_profile_may_be_written_without_a_decimal_point` records a measured
fact whose opposite is the obvious guess: `toml` widens an integer element into the `f64` the field
asks for, so a hand-written `[1, 3, 8]` is accepted. That is exactly the shape of thing that costs
an hour when it is not written down.

**What I merged anyway, and why.** Spec §8 has a second clause the PR does not implement — the
target profile must be *"rejected as a contradiction if supplied for any other objective"*. Today a
`target_profile` under `epi_spread` is silently discarded, because `SirParams` is
`#[serde(flatten)]`ed and `deny_unknown_fields` cannot fire through a flattened field. Identical
mechanism to the stray `seed` in item **#25**, and the fix is the same shape as your own
`reject_fitness_seed`. Filed as **GitHub #58**, assigned to you.

I judged #53's own scope complete and holding a clean PR for a separate spec line to be the kind of
stall that makes review feel expensive. If you would rather I had blocked it, say so — that is a
reasonable preference and I would rather know it now than discover it on the next one.

**Unrelated, found in the same pass: `main` is not `cargo fmt`-clean.**
`get/src/evolver/common.rs:45`, from `79c10aa` — my `best_index` extraction under #51, not yours.
rustfmt wants the `assert!` wrapped, its arguments exceeding the default `fn_call_width` of 60.
Noted on GitHub **#56**, which already sweeps those files and is unassigned. Flagging it because
`traps.md`'s rustfmt entry leans on the tree being clean to make a stray formatting change
noticeable, and a dirty tree removes that signal.

*#45 · raised 2026-08-10 — Michael, after merging PR #57.*

**Acknowledged, and no — you called it right. 2026-08-10 18:45 — James.** Merging was the better
judgement and I would not have wanted #57 held. The unimplemented clause is a *different* defect
from the one #53 was about: #53 replaced a path with a value, and rejecting `target_profile` under
`epi_spread` is the flatten-can't-see-unknown-keys problem wearing a new hat. Holding a verified
PR until a second, separately-mechanised fix is written is how a two-person review queue seizes up.

**One thing your framing gets right that I want on the record.** You did not merge it as "close
enough" — you named the gap, filed it with the mechanism identified, and assigned it. That is a
smaller unit of work arriving with its reasoning intact, which is strictly better than a bigger PR
where the same clause is one bullet in a description. I will pick up **#58** and it should be
cheap: `reject_fitness_seed` in `Config::from_toml_str` is the shape, and the raw-text pass it
already does can look for `target_profile` under a non-`epi_prof_match` objective in the same
sweep.

**Where I would want you to block instead.** If the unimplemented clause made the *merged* code
wrong rather than incomplete — a silently accepted value that changes a run's numbers, rather than
one silently discarded — I would rather eat the stall. The test is whether someone can get a wrong
answer and not know it. A discarded `target_profile` under `epi_spread` cannot change a score;
that objective never reads one.

*(Reply inside #45 · 2026-08-10 18:45 — James.)*
