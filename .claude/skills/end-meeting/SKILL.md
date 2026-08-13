---
name: end-meeting
description: Execute a closed meeting's consolidated action checklist — amend the spec sheet, CLAUDE.md, decisions.md and collab.md, apply code and documentation changes, file tracker issues — routing working docs direct to main and code and the spec sheet through a branch and a PR. Use after /start-meeting has closed a meeting and both owners want the agreed changes landed.
---

# End meeting

Take the consolidated checklist a closed meeting produced and actually make the changes. This is the
only skill in the set that touches real files.

**It runs after the meeting, not during it, and usually in a fresh session.** The checklist is the
contract: everything on it gets done, nothing off it does.

## 0. Set up and refuse the unsafe cases

```bash
MAIN_TREE="$(git rev-parse --show-toplevel)"
DOCS_WT="$(dirname "$MAIN_TREE")/$(basename "$MAIN_TREE")-docs"
cd "$DOCS_WT" && git pull
```

`/end-meeting [YYYY-MM-DD]`, defaulting to the most recent closed meeting file. Stop, with a plain
report, if any of these holds:

- **The header does not say `Status: closed`.** A meeting still `in progress` has a checklist that
  may be half-filled, and a half-filled checklist is indistinguishable from a complete one. Resume
  and close it first.
- **The header says `Status: executed`.** It has already run. Report what it did and stop; a second
  run would duplicate `decisions.md` entries and re-file issues.
- **The checklist is empty or every group reads `_(pending)_`.** There is nothing to execute. Say so
  rather than inferring actions from the item blocks — if `/start-meeting` did not roll a consequence
  up into the checklist, a person needs to decide whether it was dropped on purpose.
- **The main tree has uncommitted changes under `get/src/`, `documentation/` or the spec sheet.**
  This skill is about to create a branch and edit those files. Report what is dirty and stop.

## 1. Plan before touching anything

Read the whole meeting file — the checklist **and** every item's block, because the blocks carry the
reasoning that `decisions.md` needs and the checklist does not.

Then produce the execution plan and **show it before running it**: every change, grouped by route,
with its source item number. Get one confirmation. After that, work through it without stopping to
ask for each file — the confirmation covers the plan, and interrupting twenty times is how the last
few items get waved through.

Sort into the two routes. **This split is not negotiable and it is why this skill cannot simply
"push everything to main":**

| Route | What goes in it |
|---|---|
| **Direct to `main`** | `.claude/work/*.md` — `decisions.md`, `collab.md`, `traps.md`, `issues.md`, `deferred.md`, `hotfixes.md`, the meeting file itself · `.claude/CLAUDE.md` · skill **bodies** |
| **Branch + PR** | `official_spec_sheet.md` · anything under `get/src/`, `Cargo.toml`, `config.example.toml`, `examples/` · `documentation/` · skill **frontmatter** · `hooks/` |
| **Tracker** | New issues via `gh`, each confirmed individually |

The reason is in `CLAUDE.md`'s routing table: a defect in code is invisible until something
downstream reads a wrong number, and the spec sheet is the authority every future session builds
against. Both need a second reader — and a meeting is not a code review, even though both owners
were in it.

## 2. Working docs — direct to `main`, in this order

Order matters: `decisions.md` is what the other files point at.

**a. `decisions.md` — one entry per decision, not one per meeting.** Each carries the *why*, which
lives in the item block rather than the checklist. Union-merge formatting is mandatory:

```markdown
## <YYYY-MM-DD HH:MM> — Michael & James — <title, unique>

<what was decided, and the reasoning that decided it — including the option not taken and why>

*<slug> · decided at the joint meeting of <YYYY-MM-DD> — Michael & James.*
```

The heading and the closing stamp are the two lines a union merge treats as shared context, so both
must be unique. A bare `---` never closes an entry.

**b. `collab.md` — append the answer inside each item, then move the settled ones.**

For every item the meeting decided, append a stamped block *inside* that item, beneath the existing
text:

```markdown
**Settled at the joint meeting of <YYYY-MM-DD> — Michael & James.** <the decision, in one or two
lines, plus where the reasoning now lives.>

*(Settled inside #<N> · <YYYY-MM-DD> — Michael & James.)*
```

**Never edit what either owner already wrote.** Append only.

Moving items to `Settled` is the one place this skill restructures a file, and it does so **only if
the meeting decided how** — the `Open`/`Settled` boundary is itself a live question. If no decision
was taken on it, append the answers, leave every item where it sits, and say so in the report.

Then run both audits, because this is the largest append `collab.md` ever takes:

```bash
grep -vE '^\s*$' .claude/work/collab.md | sort | uniq -d
grep -n '^### [0-9]' .claude/work/collab.md
```

The first prints lines two entries could collapse onto; the second must list every item heading at
column 0, in the count you expect. **`uniq -d` cannot see a splice** — an entry grafted into the
middle of another's line duplicates nothing — which is why the second check is not optional.

**c. `.claude/CLAUDE.md`, `traps.md`, `issues.md`, `deferred.md`, `hotfixes.md`** as the checklist
requires. In `CLAUDE.md`, **supersede rather than overwrite**: strike the old line through, add the
new one with its date and reason. The reversal trail is worth more than a tidy file.

`traps.md`, `issues.md` and `hotfixes.md` are **not** union-merged, so a concurrent append conflicts
loudly. That is intended. Resolve by hand.

**d. Commit and push.** One commit per coherent group, not one for everything — the same
commit-per-verified-step discipline the code side uses.

```
<Group>: <what changed> — joint meeting <YYYY-MM-DD>

Co-Authored-By: James Sargant <shorinbonsai@gmail.com>
```

Both owners decided it, so both are on it. **Nothing else is appended** — no agent trailer, no
generated-with footer, ever.

## 3. Spec sheet and code — branch, commit, PR, stop

**a. Branch first, before the first edit.** Off current `main`, in the main tree:

```bash
cd "$MAIN_TREE" && git checkout main && git pull
git checkout -b <owner>_meeting_<YYYY-MM-DD>
git rev-parse --abbrev-ref HEAD
```

If the checklist splits cleanly into unrelated concerns — a sheet amendment and a validation fix
that share nothing — use a branch each. One PR carrying two unrelated concerns is what the
commit-per-step rule exists to prevent.

**b. The spec sheet.** This is the one route by which `official_spec_sheet.md` legitimately changes:
agreed at a joint meeting, amended, and paired with a `decisions.md` entry. Make exactly the
amendments the checklist names — no tidying of neighbouring lines, no fixing something true that
reads awkwardly. If applying an amendment reveals a *second* contradiction the meeting did not
discuss, leave it and raise a `collab.md` item.

**c. Code.** Every change needs its own verification before it is committed:

```bash
cargo test -p get && cargo clippy --all-targets -- -D warnings && cargo fmt -p get -- --check
```

**Report what actually happened.** A failing test is reported with its output, not worked around,
and the item it came from goes back on the list rather than being marked done.

**d. `documentation/`.** If a shipped feature lost its `planned` badge, grep for stragglers and drop
the `status.html` row in the same change:

```bash
grep -rn 'badge-planned' documentation/
```

Whether this happens here or in a batched sweep is itself a meeting decision — follow what the
checklist says rather than the habit.

**e. Commit per verified step, push the branch, open the PR, and stop.**

The PR body names the meeting date, links each change to its item number, and says which parts are
already on `main` as working-doc commits. **Never merge it.** The other owner merges — and where
both owners agreed the change in the room, that is a reason to say so in the PR, not a reason to
self-merge. If the PR touches `.claude/work/*.md`, note that it must be merged **locally**, because
`merge=union` runs in git and not on GitHub's servers.

## 4. Tracker

One issue at a time. Print the exact title, body and target repo — `md12ol/GraphEvolutionTool` —
and wait for an OK. **Never batch the confirmations.**

- **Derive the `(N)` level, never guess it.** Name the open issues it depends on, take the highest
  level among them, add one. Depends on nothing open → `(1)`. The level is topological, not a
  priority.
- **Pass no labels.** `gh` creates an unknown label as a side effect of using one.
- **Verify after filing** — the exit code is not evidence the body survived:

```bash
gh issue view <n> --json title,body -q '.title, .body'
```

The plain `gh issue view <n>` is broken on this repo (Projects classic deprecation) and fails for
every issue number. Assume any `gh` subcommand fetching or updating a whole issue or PR is affected;
use `--json` for reads and the REST API for writes.

## 5. Close out and report

Stamp the meeting file's header:

```markdown
**Status:** executed
**Executed:** <YYYY-MM-DD HH:MM> · docs on `main` at `<SHA>` · PR #<n> for code and sheet
```

Tick every checklist line that landed. **Leave unticked anything that did not, with one line saying
why** — a blocked item, a failing test, a decision that turned out to need a detail nobody had. Then
commit the meeting file itself to `main`.

Report, six lines maximum:

- What landed on `main`, and at what SHA.
- What is on a branch and in a PR, and who merges it.
- Issues filed, with numbers and levels.
- Anything on the checklist that did **not** land, and why.
- Anything found while executing that needs a person — a contradiction the meeting did not see, a
  test that failed, a file two decisions both claim.
- That the PR is not merged and will not be merged by an agent.

## Constraints

- **Never push code or the spec sheet to `main`.** However thoroughly both owners agreed it in the
  room, the routing table sends it through a branch and a PR, and a meeting is not a code review.
- **Never merge the PR.** Told to merge, merge — and delete the branch, remote then local, in the
  same breath, because `delete_branch_on_merge` does not fire on a locally-merged PR.
- **Never execute something that is not on the checklist**, however obviously correct. The checklist
  is what both owners confirmed. Anything else is a `collab.md` item for next time.
- **Never mark a checklist line done that you have not seen verified.** Compiled is not verified,
  and a meeting outcome silently half-applied is the most expensive failure this system has.
- **Never invent a decision to fill a gap.** A checklist line too vague to execute stops and asks.
- **Never edit another owner's `collab.md` text.** Append, stamped, always.
