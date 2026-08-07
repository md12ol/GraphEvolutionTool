---
name: setup
description: Initialize the .claude working-docs system in a project for the first time — inspect the repo, interview the user, and fill in the FILL IN blocks in .claude/CLAUDE.md. Run ONCE, right after install.sh. Use when CLAUDE.md still contains FILL IN blocks, when the user says to set up / configure / initialize the docs system, or when they ask what to do after installing the template.
model: sonnet
---

# Setup

Turn the freshly-installed template into **this project's** rules. Run once, right after
`install.sh`. Not to be confused with:

| | |
|---|---|
| `/setup` | configures the **project**. Once, ever. ← you are here |
| `/start` | opens a **task**. Once per piece of work |
| `/done` | closes a task |

At the end of `/setup`, `.claude/CLAUDE.md` should contain no `FILL IN` blocks and no rule that
doesn't apply here.

## 0. Check it hasn't already run

```bash
grep -c 'FILL IN' .claude/CLAUDE.md
```

- **No `.claude/CLAUDE.md`** → the template isn't installed. Say so, point at
  `install.sh <project>`, stop.
- **Zero `FILL IN` blocks** → setup already ran. **Don't redo it.** Say so, show the section
  headings that exist, and ask whether they want to revise one specific section. Never regenerate a
  `CLAUDE.md` that already carries project rules — those rules were earned.
- **Some remain** → continue. A partial run is normal; only fill what's still open.

## 1. Investigate before asking

**Do this first.** Most of what the FILL IN blocks want is visible in the repo, and a question you
could have answered yourself is a bad question. Gather:

**Shape of the project**
```bash
ls; git rev-parse --show-toplevel; git branch --show-current
```
Look for the build/dependency manifest — `package.json`, `pyproject.toml`, `setup.py`, `Cargo.toml`,
`go.mod`, `Makefile`, `CMakeLists.txt`, `pom.xml`, `Gemfile`, `docker-compose.yml`, `*.nix`. That
gives you the language, and usually the run/test commands.

**Multiple repos?**
```bash
cat .gitmodules 2>/dev/null; find . -name .git -maxdepth 3 -not -path './.git' 2>/dev/null
```
Also check for vendoring manifests (`gitman.yml`, `vendir.yml`, `repo` manifests). If
work spans more than one repo, block 2 is mandatory — an agent running `git status` at the root
would otherwise believe it has seen everything.

**Tracker**
```bash
git remote -v
```
`github.com` → `gh` · `gitlab.*` → `glab` · neither → ask. Note the host if it's self-hosted; that
detail costs an hour when it's missing.

**Is `.claude/` tracked?**
```bash
git check-ignore -v .claude 2>/dev/null || echo "tracked"
```

**Existing conventions** — read `CONTRIBUTING.md`, `.editorconfig`, linter configs, an existing
root `CLAUDE.md` or `AGENTS.md`. If the project already documents a rule, adopt its wording rather
than inventing a competing one.

Then say what you found, in a few lines, before asking anything. It gives the user something to
correct instead of something to compose.

## 2. Ask — always through `AskUserQuestion`, never as prose

**Every question in this step MUST go through the `AskUserQuestion` tool.** Do not ask in prose, do
not present a numbered list and wait, do not bury a question in a paragraph. The user configures a
project once and should be able to do it by clicking, not by composing answers to an essay.

Batch up to 4 per call, one question per topic, and **repeat the call** until everything is answered.
Lead every question with your best inference marked `(Recommended)` so the common case is one click,
and give each option a `description` saying what it actually costs or implies.

**The one question you must always ask — block 1, who runs the environment.** Never infer it. The
repo shows you *what* the commands are; only the user knows which ones an agent must not run. Ask
concretely, using the commands you actually found:

> **Question:** I found `docker compose up`, `make test` and `./deploy.sh` in this repo. Which
> should I run myself, and which do you always run?
>
> - *Agent runs tests and builds, never deploy or anything shared* **(Recommended)**
> - *Agent runs everything*
> - *Agent runs nothing — hands off every command*

Get the **command names**, not a category — the rule is only enforceable if it names binaries. If
they pick a middle option, follow up with a second question listing the specific commands you found
so the boundary is exact.

**The other questions**, asked only when the repo makes them relevant — batch them with the first:

| Ask | Only if | Options |
|---|---|---|
| Does the agent file issues for you, and where? | a git remote exists | the detected project · a different one · never files issues |
| Which paths are off-limits or owned by someone else? | vendored dirs or multiple repos found | the detected paths · none · free-text |
| Track `.claude/` in git? | `.claude/` is currently ignored | track the machinery, ignore only `work/current` **(Recommended)** · keep it all ignored |
| Will anyone else use this `.claude/`? | a git remote exists | yes — add `merge=union` + keep the collab section **(Recommended if a remote has other contributors)** · no, solo |
| Enable the optional hooks? | always | see step 4 — one question per hook that applies |

**Ask about the tracker** only if a remote exists: does the agent file issues on their behalf, and
to which project? If they say no, delete block 3 outright.

**Ask about off-limits paths** only if you found vendored/shared directories or a multi-repo layout.

Don't ask about anything you can settle by reading the repo. Don't ask four questions when the
project is a single repo with no tracker and the answer to three of them is "delete that block".

## 3. Write `.claude/CLAUDE.md`

Edit in place, block by block. **Delete each `FILL IN` comment as you resolve it** — including its
`<!-- -->` wrapper and the worked example inside. A leftover example is worse than nothing; a future
session cannot tell the template's `docker compose up` from a real rule.

- **Block 1 — environment.** Write it in the imperative, name the actual commands, and say what to
  do *instead* ("hand off the exact command and the log markers for success/failure"). If the answer
  was "run everything", delete the block; don't write a rule that permits everything.
- **Block 2 — repo layout.** Fill the table only if work genuinely spans repos. **Date the branch
  column** (`Branch (as of <YYYY-MM-DD>)`). Add the 3–5 key paths you found. Single repo → delete.
- **Block 3 — filing issues.** Tool + host, who owns the credential, the per-issue confirmation
  rule, target project, and the label warning if the tracker creates unknown labels as a side
  effect. No tracker → delete.
- **Block 4 — files outside your scope.** Paths, and the pointer to `hotfixes.md` for per-file
  disposition. Nothing off-limits → delete.
- **House style** — fill the small `FILL IN` under Conventions with anything the linter configs or
  `CONTRIBUTING.md` told you. Nothing found → delete.

Leave everything not marked `FILL IN` alone. The working-docs model, the workflow, the three task
states and the conventions are the system itself, not project settings.

Replace `<PROJECT>` in the title with the real name.

## 3b. Check the backup wiring

`.claude/hooks/backup_docs.sh` ships with the template and is wired to `Stop` + `SessionEnd` in
`settings.json`. It derives its destination automatically —
`~/.claude-backups/<name-of-the-directory-containing-.claude>/<date>/` — so there is normally
nothing to configure. **Verify it rather than assume it:**

```bash
.claude/hooks/backup_docs.sh --force
```

It should print the destination. Confirm the project name in that path is the one you expect — it
comes from the directory name, so a checkout called `src` or `repo` produces a useless bucket. If it
is wrong, set `CLAUDE_DOCS_BACKUP_DIR` in `settings.local.json` under `env`, and say so in the
report.

If the user chose to **track `.claude/` in git**, tell them the backup is now belt-and-braces and
they may delete the two hooks from `settings.json`. Don't delete them unasked.

## 4. Offer the optional hooks

Read `.claude/hooks/README.md` and offer only the ones that now apply. **Ask about each
through `AskUserQuestion`** — one question per hook, not a prose list:

- **Block dangerous commands** — offer this whenever the answer to block 1 was anything other than
  "run everything", and **build the regex from the commands they named**. This is the difference
  between a rule that is written down and a rule that holds; prose rules do get violated.
- **Show hotfixes before editing owned files** — offer only if block 4 was filled in, with their
  paths in the pattern.
- **Session-start brief** — offer always. It's cheap and makes stale `[~]` items visible.

Merge accepted hooks into `.claude/settings.json`, preserving the two backup hooks, then verify:

```bash
python3 -m json.tool .claude/settings.json > /dev/null && echo OK
```

A malformed `settings.json` disables every hook in it silently, so don't skip that check.

## 5. Settle version control

If `.claude/` is gitignored, raise it once — it's the system's biggest fragility, and the moment to
fix it is now, before there's history to lose:

> `.claude/` is gitignored here, so `CLAUDE.md`, the skills, `hotfixes.md` and `decisions.md` get no
> version control, no recovery, and teammates never see them. Recommended instead:
> ```gitignore
> .claude/settings.local.json
> .claude/work/current/
> ```

`work/current/` is the only per-person thing — two people cannot hold one live plan. **Do not
ignore `work/archive/`**: a finished task's record is shared history, and ignoring it strands every
`/done` on one laptop.

### If more than one person will use this `.claude/`

Then also offer to add to the repo root `.gitattributes`:

> ```gitattributes
> .claude/work/*.md merge=union
> ```

`decisions.md`, `traps.md`, `hotfixes.md`, `issues.md` and `collab.md` are append-only, so everyone
writes to the tail of the same file — the most conflict-prone shape in git. Without this, every
concurrent session ends in a merge conflict.

State the trade when you offer it, because it is real: **union merge never conflicts**, so a genuine
collision on the same entry merges silently and interleaved. The mitigation is an author stamp on
every entry, which the "More than one person uses this `.claude/`" section of `CLAUDE.md` mandates —
keep that section, and keep `work/collab.md`. If they say they work alone, delete both.

**Ask before editing `.gitignore`** — it's a tracked file and this is their call. If they decline,
leave the backup hooks enabled and say why they now matter more.

If they accept and `.claude/` is untracked, offer to `git add` it — but **do not commit** unless they
ask.

## 6. Report, then hand off to `/start`

Short. What each block says now, which blocks you deleted and why, which hooks are live, and the
version-control disposition. Then:

> Setup is done — `CLAUDE.md` is now this project's rules. Start your first piece of work with
> `/start`.

Don't create `work/current/plan.md`. `/setup` configures the project; `/start` opens the task, and it
needs an objective agreed with the user that `/setup` has no way to know.

## Constraints

- **Run once.** Step 0 is a real gate, not a formality.
- **Never invent a rule the user didn't agree to.** An unasked-for constraint in `CLAUDE.md` is
  obeyed silently by every future session. When unsure, delete the block rather than guessing at it
  — an absent rule is visible, a wrong one isn't.
- Don't write `decisions.md`, `issues.md`, `hotfixes.md` or `traps.md` entries. They are empty
  because nothing has happened yet.
- Don't commit or push.
