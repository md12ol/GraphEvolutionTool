# Traps — how this workspace actually behaves

Permanent gotchas. Distinct from `hotfixes.md`: a hotfix is *code you added and want to remove*; a
trap is *how this workspace behaves and always will*.

This file exists because durable warnings kept getting parked in `handoff.md`, which `/save`
overwrites every session — so they were deleted the moment they stopped being top-of-mind.

Read by `/load` and `/start`. Entries leave only when no longer true.

---

### <the trap, stated as the mistake it prevents>
- **Bites when:** the action that triggers it.
- **Do this instead:** the correct form.
- **Why:** the mechanism, one line.
- **Added:** <YYYY-MM-DD>
### Running bare `cargo fmt` rewrites files you did not touch
- **Bites when:** you run `cargo fmt` (no path) to tidy your own edit. It reformats the whole
  workspace, and part of the tree has never been formatted, so you get unrelated hunks in your
  diff — in files someone else is actively editing.
- **Do this instead:** format only what you changed —
  `rustfmt --edition 2024 get/src/path/to/file.rs`
- **Why:** these files predate any `cargo fmt` run. Check the current set with
  `cargo fmt -- --check 2>&1 | grep "^Diff in"`. As of 2026-07-31 it is
  `get/src/evolver/generational.rs` and `get/src/genomes/sda.rs` — `generational.rs` is James's
  live work, so sweeping it hands him a conflict. It was 4 files before `fitness.rs` and
  `steady_state.rs` were formatted as part of editing them.
- **The real fix** is one tree-wide `cargo fmt` commit, agreed with James — `collab.md` #7.
  Until that happens, this trap stands.
- **Added:** 2026-07-31 — running-bare-cargo-fmt-rewrites-files-you-di

### A `-0.0` fitness would make the selection tests disagree with the code
- **Bites when:** a fitness function returns `-0.0` alongside `0.0`, and a selection test fails in
  a way that looks impossible.
- **Do this instead:** if you hit it, fix the oracle — not the implementation. The implementation is
  right.
- **Why:** `Selection` orders with `f64::total_cmp`, which distinguishes `-0.0 < 0.0`. The test
  oracle `expected_winner` in `get/src/evolver/common.rs` compares with `<` and `==`, which treat
  them as equal, so it would predict a tie where the code picks a winner. The oracle is deliberately
  written differently from the implementation so it is an independent check of the tie-break rule;
  that independence is exactly what creates this gap. No current test data contains `-0.0`.
- **Added:** 2026-07-31 — a-0-0-fitness-would-make-the-selection-tests

### The `.claude/` docs split across branches, but `work/current/` does not
- **Bites when:** you switch branches mid-task, then write to `decisions.md`, `traps.md`,
  `hotfixes.md`, `issues.md` or `collab.md`. Those are **tracked**, so each branch has its own
  version — writing on the wrong branch appends to the wrong base and silently drops entries the
  other branch had. It happened on 2026-07-31: a checkout to `main` mid-`/save` put seven new
  `decisions.md` entries on a copy that was missing the two already committed on the feature branch.
- **Do this instead:** run `git branch --show-current` before writing any `.claude/` doc, and again
  after any long gap. If you are on the wrong branch, rescue the new text to the scratchpad,
  `git restore` the file, switch, and re-append.
- **Why:** `.gitignore` excludes only `.claude/work/current/` and tracks everything else under
  `.claude/`. So `plan.md` and `history.md` follow you across branches while `decisions.md` does
  not — the two halves of the docs system behave differently.
- **Added:** 2026-07-31 — the-claude-docs-split-across-branches-but-wo
- **Amended 2026-07-31:** `work/archive/` was un-ignored when James joined the repo, so archived
  task records now split across branches too. `merge=union` (below) makes the *merge* safe; it does
  nothing about writing to the wrong branch in the first place, so this trap stands unchanged.

### `merge=union` on `decisions.md` and `collab.md` means a merge can never tell you it went wrong
- **Scope narrowed 2026-08-04:** this applies to **`decisions.md` and `collab.md` only**. `traps.md`,
  `issues.md` and `hotfixes.md` were removed from the union glob because they are churn lists and
  union cannot express a deletion — they now take a normal 3-way merge and *do* conflict.
- **Bites when:** you and the other owner append to `decisions.md` or `collab.md` on separate
  branches, then merge. There is no conflict, no marker and
  no prompt — git keeps both sides' lines and calls it clean.
- **Measured 2026-07-31**, two branches each appending one entry to `decisions.md`:
  - Two entries with **distinct** text merge **correctly** — both survive whole and in order. The
    only damage is the **blank line between them is eaten**, because it is a line common to both
    sides. Cosmetic; re-insert it.
  - Lines that are **byte-identical** on both sides are **deduplicated and the entries interleave**.
    Two entries that share a boilerplate line collapse into one block that reads as a single
    coherent entry and is not one. This is the dangerous case, and it is silent.
- **Do this instead:** after any merge that touched `.claude/work/*.md`, **read the tail of the
  changed files** before trusting them — `git diff HEAD~1 -- .claude/work/` is enough. Keep entries
  textually distinctive: the `— <author>` stamp, the date and a real `**Affects:** path` line are
  what stop two entries deduplicating into each other. Bare boilerplate (`reasoning`, `TODO`,
  a lone `---`) is what makes them collide.
- **Why:** `/.gitattributes` sets `merge=union` on those two files deliberately — they are
  append-only, both owners write to the tail, and without it every concurrent session ended in a
  conflict. Union merge trades "conflicts constantly" for "never conflicts", and the second failure
  mode is quieter than the first. That is the trade, not an accident.
- **Added:** 2026-07-31 — merge-union-on-the-claude-work-md-docs-means
- **Amended 2026-07-31 — Michael:** the attribute is a **glob**, `.claude/work/*.md`, not a list of
  five filenames. **Any new `.md` dropped into `.claude/work/` silently inherits union merge**,
  including documents that are edited in place rather than appended to — which is the dangerous
  shape, since two rewrites of the same section merge into one block with no conflict. This is why
  `official_spec_sheet.md` lives at the repo root and not under `.claude/`. Before adding a document
  there, ask whether it is append-only; if it is not, it does not belong in that directory.


### `install.sh --update` overwrites this project's customized hooks
- **Bites when:** you refresh the working-docs machinery from `~/.claude-template` with
  `install.sh --update /home/mdube/GraphEvolutionTool` to pick up a skills change. It copies
  **`skills/` and `hooks/`** — and both of this project's hooks are customized:
  `block_env_commands.sh` carries the real BLOCK/WARN/ALLOW patterns, and `show_hotfixes.sh`
  carries the co-owned-file pattern. `--update` replaces both with the template's examples, and
  the security hook silently reverts to blocking nothing that matters.
- **Do this instead:** sync skills only —
  `cp -r ~/.claude-template/template/skills/. .claude/skills/` — then diff the hooks by hand:
  `install.sh --diff /home/mdube/GraphEvolutionTool`.
- **Why:** `MACHINERY_DIRS=(skills hooks)` in `install.sh`, and `update` does a plain `cp -r` over
  the destination. The template treats hooks as machinery it owns, but ships them with
  "EDIT THE PATH PATTERN BELOW BEFORE ENABLING" — so any project that follows that instruction has
  local edits `--update` will destroy. Reported for the template; not yet fixed there.
- **Added:** 2026-07-31 — install-sh-update-overwrites-this-project-s-

### A PR can merge mid-session, and `/save`'s git manifest will not notice
- **Bites when:** you open a PR, keep working, then run `/save`. The manifest is built from local
  state — `git status`, `git log` — none of which changes when the other owner merges on GitHub. It
  happened 2026-08-04: PR #31 was merged at 12:38 UTC and the save written at ~15:05 UTC described
  it as open, in `plan.md`, `history.md` and `handoff.md` at once.
- **Do this instead:** `git fetch` and read the PR state from the remote before writing any doc that
  mentions it — `gh api repos/md12ol/GraphEvolutionTool/pulls/<n> --jq '.state, .merged'`.
- **Why:** two owners, and merges happen on the server. Nothing local is stale in a way git reports;
  `git status` says "up to date with origin/<branch>" whether or not that branch has been merged.
- **The expensive corollary — a closing keyword added after the merge never fires.** GitHub applies
  `Closes #N` only at merge time. Editing the PR body afterwards links the issue but leaves it open,
  and the PR reads as though it closed it. Check the issue's actual state, never the PR's body.
- **Added:** 2026-08-04 — a-pr-can-merge-mid-session-and-save-manifest

### GitHub's web merge ignores `merge=union`, so merging a `.claude/` PR on the website conflicts
- **Bites when:** you click Merge on a PR that touches `.claude/work/*.md`. GitHub reports a
  conflict and offers its web resolution editor — where you would hand-resolve an append-only log
  in a textarea, which is how one side's entries get dropped.
- **Do this instead:** merge locally, where `.gitattributes` is read.
  `git checkout main && git pull && git merge --no-ff origin/<branch> && git push origin main`,
  then read the tail: `git diff HEAD~1 -- .claude/work/`.
- **Why:** `.gitattributes` merge drivers are applied by *your* git, not by GitHub's servers. This
  is true even for `union`, which is built into git rather than custom.
- **Measured 2026-08-04 on PR #30**, three ways: locally with `.gitattributes` present →
  `Auto-merging`, 0 conflicts. Locally with it removed → `CONFLICT (content) in decisions.md`.
  GitHub's API → `mergeable=false, mergeable_state=dirty`. GitHub reproduces the no-driver case
  exactly.
- **Added:** 2026-08-04 — githubs-web-merge-ignores-merge-union

### Union merge silently DUPLICATES a line when both sides edit the same one
- **Bites when:** two people edit the *same existing line* of a `.claude/work/*.md` file on separate
  branches — typically both closing out the same task, one striking a status and one superseding it.
  Union keeps **both** versions, one after the other, and reports `1 file changed, 1 insertion(+)`.
  No conflict, no marker, no prompt.
- **Measured 2026-08-04** on a 250-line file, so it is not a small-file artifact:

      **Affects:** foo.rs. ~~**Not yet implemented.**~~ **Superseded — Michael.**
      **Affects:** foo.rs. ~~**Not yet implemented.**~~ **Implemented — James.**

  The entry now claims to be both superseded and implemented, and reads as deliberate.
- **Do this instead:** append; do not edit in place. If an existing entry genuinely must change,
  **announce it in `collab.md` first** — the announcement is the only mechanism that prevents the
  concurrent case, because git will not warn you. Then audit after merging:
  `grep -vE '^\s*$' .claude/work/<file>.md | sort | uniq -d` prints the duplicated line.
- **Why:** union resolves a conflicting hunk by concatenating both sides of it. It has no idea who
  wrote a line, so **authorship is irrelevant to safety** — editing "your own" entry is fine
  socially and buys nothing mechanically. What matters is whether both sides touched the region.
- **Note this is the opposite failure from the existing dedup trap above.** Byte-identical lines are
  *removed*; concurrently-edited lines are *doubled*. Same driver, both silent, opposite symptoms.
- **Small files are worse.** In a 5-line file a *lone* in-place edit also scrambled — the original
  line survived unedited while the edit was orphaned onto the end of an unrelated entry. That
  disappears once there is enough surrounding context for git to localize the hunk, so short files
  like `hotfixes.md` and `issues.md` are more fragile than `decisions.md`.
- **Added:** 2026-08-04 — union-merge-silently-duplicates-a-line

### Untracked pre-spec-sheet docs sit in the tree and `git add -A` will commit them
- **Bites when:** you stage with a bare `git add -A` or `git add .`. Four untracked files are in
  James's working tree — `docs/IMPLEMENTATION.md`, `docs/COMPLEXITY_REVIEW_HANDOFF.md`,
  `docs/PR_DRAFT.md` (all 2026-07-29) and `GET GA planning session.md` (2026-07-26). Verified
  2026-08-03: `git ls-files` returns none of them and `git check-ignore` matches none of them, so
  they are eligible for staging and nothing stops it.
- **Do this instead:** stage explicit paths — `git add -A -- <path> <path>` — or check
  `git status --short` for `??` lines before committing. This is how `a8cbf27` was staged.
- **Why they must not be committed:** `docs/IMPLEMENTATION.md` is the document
  `official_spec_sheet.md` **replaced**. `.claude/CLAUDE.md` records that it "mixed design with
  build order and rotted every time the order changed", which is the exact failure the sheet exists
  to prevent. Its own opening paragraph describes the `feature/ga-engine` stub era, when every body
  was a `todo!()`. Committing it would restore a second, contradictory design authority.
- **Do not read them as current.** They predate the 2026-07-31 spec sheet and the tracker. Where
  they disagree with `official_spec_sheet.md`, the sheet governs and they are simply stale.
- **Machine:** James's working tree only, uncommitted — Michael will not see these files.
- **Added:** 2026-08-03 — untracked-pre-spec-sheet-docs-git-add-all

### GitHub's PR object lags the branch, so a merge can strand a commit you already pushed
- **Bites when:** you push to a branch that has an open PR, and the other owner merges within the
  next minute or so. GitHub snapshots the PR's `head.sha` and refreshes it asynchronously — merging
  uses the **cached** head, so any commit pushed in that window is silently left out of `main` while
  `git status` cheerfully reports "up to date with origin/<branch>".
- **Measured 2026-08-04 on PR #36.** `git rev-parse origin/<branch>` and
  `gh api .../branches/<branch> --jq .commit.sha` both returned `3c794b6`, while
  `gh api .../pulls/36 --jq .head.sha` still returned `0fec0d8`. James merged in that gap; the merge
  commit contains `0fec0d8` and the `decisions.md` entry in `3c794b6` never reached `main`.
- **Do this instead:** after pushing to a branch with an open PR, confirm GitHub has caught up
  before anyone merges — `gh api repos/md12ol/GraphEvolutionTool/pulls/<n> --jq .head.sha` must
  match `git rev-parse HEAD`. If a merge already happened, check with
  `git merge-base --is-ancestor <sha> main` and recover by cherry-picking onto `main`.
- **Why this is not the same as the mid-session-merge trap above:** that one is about *your* view of
  the world going stale. This is about *GitHub's* view going stale, and it loses work rather than
  just misreporting it.
- **Added:** 2026-08-04 — githubs-pr-object-lags-the-branch

### `cargo clippy -- -D warnings` cannot pass on `main`, so it is not a usable gate
- **Bites when:** your task's `Verify by:` says "clippy passes" and you treat the failure as a
  regression you introduced. Issue **#22**'s verify-by says exactly that, so this will bite whoever
  picks it up.
- **Measured 2026-08-04 22:05 by Michael**, on branch `mdube_sir_objectives` and again on `main`
  with the branch stashed. Both produce the identical two errors, and nothing else:
  `fields shared, context, population, and history are never read` and
  `method advance_generation is never used`, both in `get/src/evolver/generational.rs`.
- **Why:** `-D warnings` promotes `dead_code` to an error, and `GenerationalEvolver` is a built
  shell whose `run` is still unimplemented — issue **#25**, James's. The dead code is the unbuilt
  work, so this clears when #25 lands and not before.
- **Do this instead:** compare against the baseline rather than expecting zero.
  `git stash -u && cargo clippy --manifest-path get/Cargo.toml --all-targets -- -D warnings; git stash pop`
  and check your branch adds nothing new. Say so explicitly in the PR — "fails identically to
  `main`" is a reviewable claim, "clippy passes" would be false.
- **Added:** 2026-08-04 — cargo-clippy-d-warnings-cannot-pass-on-main
