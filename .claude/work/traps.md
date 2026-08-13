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

### `cargo clippy -- -D warnings` IS a usable gate now, so a failure is yours
- **Bites when:** you carry forward the old habit of diffing clippy against a non-empty baseline and
  treat leftover warnings as pre-existing. They are not any more — **a warning on `main` is one you
  introduced.**
- **Measured 2026-08-07 — James, on `main` at `94a4679`:** `cargo clippy -p get --all-targets --
  -D warnings` exits 0. This **supersedes** the entry that stood here from 2026-08-04 to
  2026-08-07, which said the gate could not pass because `GenerationalEvolver` was an unbuilt shell
  emitting two `dead_code` warnings. Issue #25 built it (PR #46), and both warnings went with it,
  exactly as that entry predicted they would.
- **Do this instead:** just run the gate. `cargo clippy -p get --all-targets -- -D warnings`.
- **If a non-empty baseline ever comes back** — another shell landing ahead of its implementation —
  capture it on the clean tree *before* editing, and diff at the end. Do not use `git stash -u`
  for this: on a task that also changed `config.example.toml`, stashing leaves an example the
  stashed code cannot parse, so the "baseline" is contaminated by unrelated failures (hit on #24).

      cargo clippy -p get --all-targets 2>&1 | grep -E '^(warning|error)' | sort > /tmp/clippy_base.txt
      # ... do the work ...
      cargo clippy -p get --all-targets 2>&1 | grep -E '^(warning|error)' | sort | diff /tmp/clippy_base.txt -

- **Added:** 2026-08-07 — cargo-clippy-d-warnings-is-a-usable-gate-now

### Union merge can SPLICE two entries together without duplicating a line, so `uniq -d` says clean
- **Bites when:** you and the other owner each append a new item to `collab.md` (or `decisions.md`)
  on separate branches and the appends land near the same region. Union concatenates the two
  conflicting hunks, and the join can fall **inside a line of yours** — the other person's entire
  entry ends up embedded mid-sentence in yours.
- **Measured 2026-08-04, twice in one session, both on `main`.** The second was the bad one: my
  item 20's first bullet began `` - `## Settled` and the whole ... ``, and Michael's new item was
  spliced in immediately after the `` - ` ``. His heading absorbed my bullet prefix, so
  `grep '^### '` did not list his item at all — it was not a top-level entry — and my sentence
  resumed twenty lines later. Both entries read as corrupt; neither was recoverable by skimming.
- **The documented audit does not catch it.** `grep -vE '^\s*$' <file> | sort | uniq -d` returned
  **nothing** on the corrupted file, because a splice **repeats no line**. That check finds the
  dedup/duplicate failures; it is blind to this one. Do not treat a clean `uniq -d` as proof the
  file is intact.
- **Do this instead:** after any merge or pull touching these files, check the *structure*, not just
  for duplicates — every item heading is at column 0 and the count matches what you expect:

      grep -n '^### [0-9]' .claude/work/collab.md    # every item, none indented or prefixed
      git diff HEAD~1 -- .claude/work/               # and actually read it

  A heading that appears mid-line, or an item you know exists but which the grep does not list, is
  this trap.
- **Also collide on item numbers.** Both of us raised an item **20** the same day, because numbering
  is "one higher than the last" and neither had pulled. Check the other side's tail before choosing
  a number.
- **Why:** union resolves a conflicting hunk by concatenating both sides of it, at whatever
  granularity the diff produced. It has no notion of an "entry", so nothing stops a join landing
  mid-sentence. This is the same driver as the dedup and duplicate traps above — third distinct
  symptom, same cause, all three silent.
- **Added:** 2026-08-04 — union-merge-splices-entries-without-duplicating

### Per-file `rustfmt` is not per-file on a `mod.rs` — it reformats every submodule that file declares
- **Bites when:** you follow `CLAUDE.md`'s "never a bare `cargo fmt`, format the files you touched"
  and one of those files is a `mod.rs`. rustfmt parses the module tree from the file you hand it and
  descends into every `mod x;` it declares, so naming one file does **not** bound what it rewrites.
- **Measured 2026-08-04 — James, during #15.** `rustfmt --edition 2024 get/src/evolver/mod.rs`
  alone reformatted `get/src/evolver/generational.rs` (reordered a `use super::{...}` list). That
  file is **issue #22, Michael's**, and one of the files `collab.md` #14 flags as claimed by two
  workstreams. It reached `git diff` but not a commit; the only thing that caught it was reading
  `git diff --stat` and noticing a fourth file in a three-file change.
- **Do this instead:** pass the config option, and check the stat afterwards regardless.

      rustfmt --edition 2024 --config skip_children=true get/src/evolver/mod.rs
      git diff --stat        # confirm only the files you meant to touch appear

  Reproduced both ways on 2026-08-04: without the flag `generational.rs` shows
  `1 file changed, 1 insertion(+), 1 deletion(-)`; with it, untouched.
- **`--skip-children` is NOT a CLI flag on this toolchain** — it exits `Unrecognized option`. It
  moved into the config system, so it must be passed as `--config skip_children=true`. The stale
  flag form is what most search results show.
- **Why it matters here beyond tidiness:** the descent is silent either way, and a rustfmt-clean
  tree makes a stray one **harder** to notice, not safer. ~~The tree is not currently rustfmt-clean
  (that is exactly what #22 exists to fix), so a stray descent produces *real* diff, not a no-op.~~
  Superseded 2026-08-06: #22 shipped as PR #43 and `cargo fmt -- --check` reports no offenders on
  `main`, so most stray descents are now no-ops — which means the occasional one that *does* produce
  diff is the only signal there is, where before it might have been lost in the noise. Whatever it
  produces still lands in whichever file the other owner has claimed. Amendment proposed by Michael
  in `collab.md` #31 and applied by James, its author.
- **Added:** 2026-08-04 — rustfmt-descends-into-submodules-of-a-mod-rs

### `deny_unknown_fields` does nothing through a `#[serde(flatten)]`, and reports no error either
- **Bites when:** you assume a typo'd or stale key in a TOML table is a parse error. For any struct
  reached through a flattened field it is **silently discarded** — the attribute does not warn, does
  not fail to compile, and does not fire.
- **Do this instead:** if a key must be rejected, either do not flatten that block, or check the
  raw text before deserializing. Verify which you have with an actual test rather than reading the
  attribute and believing it.
- **Why:** serde deserializes a flattened field through a buffered content map, and the buffering
  loses the "this key was not claimed" information the deny relies on.
- **Where this is live in the tree:** `[fitness]` accepts and drops unknown keys, because
  `FitnessConfig`'s three epidemic variants flatten `SirParams` (spec §7 requires the flatten).
  `[genome.operation_weights]` **does** reject them — `EdgeEditOperationWeights` carries the
  attribute and is not flattened (`get/src/genomes/edge_edit.rs:27`). Two tables, two behaviours,
  neither wrong. Pinned by `an_unknown_fitness_key_is_ignored_rather_than_rejected` in
  `get/src/config.rs`.
- **Measured 2026-08-05** on `SirParams`: a stray `seed = 42` parsed clean both with and without
  `#[serde(deny_unknown_fields)]` on the struct. Full reasoning in `decisions.md` 2026-08-05 15:47.
- **Added:** 2026-08-05 — deny-unknown-fields-does-nothing-through-a-flatten

### `gh` is not on `PATH` on Michael's machine, in either shell
- **Bites when:** you call `gh` directly in Bash or PowerShell on this machine — both report
  `command not found` / `CommandNotFoundException`, even though `gh auth status` works fine once
  invoked correctly.
- **Do this instead:** use the full path — `"C:\Program Files\GitHub CLI\gh.exe"` (PowerShell) or
  `/c/Program Files/GitHub CLI/gh.exe` (Bash/Git Bash).
- **Why:** the install put `gh.exe` in `C:\Program Files\GitHub CLI\` without adding it to this
  user's `PATH`. Machine-specific, not a repo issue.
- **Also worth knowing:** the stored token only carries `gist`, `read:org`, `repo` scopes — reading
  `user/emails` (e.g. to check which addresses are verified) 404s and asks for
  `gh auth refresh -h github.com -s user`, which is an interactive browser flow and can't be run
  from a non-interactive session. `gh api repos/<owner>/<repo>/commits/<sha>` (public repo data,
  no extra scope) is a working substitute for checking whether a specific commit's email resolves
  to a GitHub login.
- **Added:** 2026-08-06 — gh-is-not-on-path-on-michaels-machine

### A dirty working tree means `pull_main.sh` does not pull, and you may not see it say so
- **Bites when:** you start a session on `main` with uncommitted changes to `decisions.md` or
  `collab.md` — the normal state after a `/save` that was not committed — and then branch for new
  work. `main` is silently whatever it was when you last pulled, and the branch is cut from there.
- **Measured 2026-08-06 — James, at the start of the generational-evolver session.** Local `main`
  was 7 commits behind `origin/main` (PR #45 merged, #17 archived, a trap deleted), with
  `collab.md` and `decisions.md` modified in the tree. The hook is right to refuse — `merge
  --ff-only` will not overwrite local changes, and refusing non-destructively is the design
  (`collab.md` #30) — but **no `pull_main:` line appeared in the session's context**, only
  `session_brief.sh`'s block. Whether the line was printed and not surfaced, or not printed, was
  not established from inside the session; what is certain is that the warning did not arrive.
- **Do this instead:** check for yourself before branching, rather than trusting that a silent
  session start means an up-to-date `main`.

      git fetch origin --dry-run     # prints nothing when you are actually current
      git status --short             # a dirty tree is the condition that suppresses the pull

  Then commit or stash the docs and `git pull` before cutting the branch. This session's plan named
  a base commit that was two merges stale by the time it was acted on.
- **Why:** `.claude/settings.json` runs the hook as `pull_main.sh 2>/dev/null || true`, and the
  script exits 0 on every failure path by design, so nothing downstream distinguishes "pulled",
  "refused", and "never ran".
- **Added:** 2026-08-06 — dirty-tree-means-pull-main-does-not-pull

### `cargo test` cannot link anything that touches Python, unless `extension-module` is off
- **Bites when:** you write a `#[test]` that calls `Python::attach`/`with_gil`, or add any pyo3-based
  test to `get/`. `extension-module` in `[dependencies]` tells pyo3 to leave the Python C API symbols
  **unresolved** — correct for the built module, which the interpreter dlopens and supplies them for,
  fatal for `cargo test`, which produces an ordinary binary with nothing to supply them. Failure is a
  wall of `undefined symbol: PyObject_GetAttr`, `PyLong_AsLong`, `PyDict_Type`, … at **link** time,
  and it takes down the **whole suite**, including tests that never mention Python.
- **Measured 2026-08-07, during #19.** Confirmed both the failure and the fix on this repo before
  writing `PyFitness`.
- **Do this instead:** the fix is already in `get/Cargo.toml` as of #19 — `extension-module` is out
  of `[dependencies]`, and `[dev-dependencies] pyo3` carries `auto-initialize`. The built module
  supplies the feature from outside the manifest instead: maturin via
  `[tool.maturin] features = ["pyo3/extension-module"]` (~~not yet set up — `issues.md`, the
  `pyproject.toml` gap~~ — **set up 2026-08-08 in `7a3aa7f`; the root `pyproject.toml` carries it
  along with `manifest-path = "get/Cargo.toml"`**), or by hand with
  `cargo build -p get --features pyo3/extension-module`.
  `fitness.rs`'s `the_test_harness_can_call_a_live_python_interpreter` is a permanent smoke test that
  fails loudly if the manifest is ever reverted.
- **The runtime half:** `cargo test` then needs `libpython3.*.so` at **run** time too. A pyenv-managed
  Python is not on the default loader path — symptom is `error while loading shared libraries:
  libpython3.11.so.1.0: cannot open shared object file`, exit code **127**, before any test runs.

      export LD_LIBRARY_PATH="$(python3 -c 'import sysconfig; print(sysconfig.get_config_var("LIBDIR"))'):$LD_LIBRARY_PATH"
      cargo test -p get

  ~~**Unverified on Windows** — Michael's machine links Python differently and this is Linux/pyenv
  only; `collab.md` should carry a heads-up rather than this trap silently not applying to him.~~
- **Windows half, measured 2026-08-09 on Michael's machine — the same trap, a different symptom
  and a different fix.** Superseding the line above, which was written before anyone had run it
  there. **Linking is fine on Windows**: `cargo test -p get --no-run` completes with exit 0 and no
  undefined `Py*` symbols, so the link-time failure this entry opens with is Linux-specific in
  practice. The **runtime** half does bite, as the exact analogue of exit 127: the test binary
  cannot find `python3.dll` and dies with `exit code: 0xc0000135, STATUS_DLL_NOT_FOUND` before any
  test runs. Fix is `PATH`, not `LD_LIBRARY_PATH` — put the Python install directory on it:

      $env:PATH = "C:\Users\micha\AppData\Local\Programs\Python\Python312;$env:PATH"
      cargo test -p get      # 213 passed, 0 failed

  **The root cause is local, not #19's**, which is why this is a machine note rather than an
  argument against the manifest change: bare `python` on that machine resolves to the Microsoft
  Store stub, and the real interpreters are reachable only through the `py` launcher. James's
  offered fallback — putting the pyo3-touching tests behind a cargo feature — is therefore **not
  needed**. `collab.md` #37 is settled by this.
- **Why:** full mechanism, and what transfers from `graph_refiner` versus what doesn't, in
  `.claude/reference/pyo3-maturin.md` §1.
- **Added:** 2026-08-07 — cargo-test-cannot-link-python-unless-extension-module-is-off

### Calling Python from inside a rayon closure doesn't just run slow — it deadlocks
- **Bites when:** a `Fitness` objective wraps a Python callable and its `evaluate_population` is
  left at the trait default, or any future rayon-parallel path calls into pyo3 without first
  releasing whatever GIL the calling thread holds.
- **Measured 2026-08-07**, during #19, by deleting `PyFitness`'s `evaluate_population` override and
  running the batching test. The default fans out over `par_iter()`; each worker calls
  `Python::attach` while the calling thread already holds the GIL and is blocked waiting for rayon
  to finish. The suite **hung** until killed at 2 minutes — no failure, no message, just silence.
- **Do this instead:** every objective that wraps Python **must** override `evaluate_population`
  and take the GIL exactly once per batch, never per graph. This is why
  `impl Fitness for Box<dyn Fitness>` forwards it explicitly rather than relying on the trait
  default — an unforwarded box reintroduces exactly this deadlock silently.
- **Why:** spec §8 states the rule ("never call Python from inside a rayon closure") and argues it
  on performance; the measured failure is stronger than the stated one. Full writeup in
  `.claude/reference/pyo3-maturin.md` §2.
- **Added:** 2026-08-07 — calling-python-from-a-rayon-closure-deadlocks

### `#[pyclass]` cannot go on `config`'s fitness enum, so `py_config.rs` is not duplication to tidy away
- **Bites when:** you look at `get/src/py_config.rs`, see it mirroring `get/src/config.rs` field for
  field, and try to collapse the two — either by putting `#[pyclass]` on `config`'s own types or by
  deriving `Serialize` on the mirror so the conversions can go. Both are dead ends, and the second
  error only appears after you have rewritten the conversions.
- **Do this instead:** leave the mirror in place. Add the field in **both** files; the tests below
  will tell you if you forget.
- **Why:** pyo3 and serde disagree about one variant. pyo3 rejects a **unit** variant in a complex
  enum and directs you to an empty tuple variant (`Python()`); serde then rejects that with
  `#[serde(tag = "...")] cannot be used with tuple variants`. The tag is what deserializes
  `type = "python"` for the hand-written TOML path, so annotating `config::FitnessConfig` breaks the
  file front end to serve the Python one. Measured 2026-08-08 on pyo3 0.27.2 and serde 1.0.228,
  both directions, while building #29.
- **The corollary, which looks like a bug and is not:** editing `config.rs` can break tests in
  `py_config.rs` you never touched. That is the drift guard working. The round-trip tests
  destructure `Config` with **no `..`**, so a new field fails to compile ("pattern does not mention
  field"), and a new `invalid("<field>", ...)` check with no Python attribute mapping fails
  `every_validation_field_maps_to_a_python_attribute`, which scrapes `config.rs`'s source. Both were
  confirmed by deliberately breaking them, then reverting. Fix the mirror; do not weaken the test by
  adding `..`.
- **Added:** 2026-08-08 — pyclass-cannot-go-on-configs-fitness-enum

### "Shared config→engine code goes in evolver/common.rs" is the wrong reflex
- **Bites when:** dispatch grows new config→concrete-type machinery (a population builder, an
  objective factory, a config-to-context mapping) and `common.rs` looks like the obvious shared
  home because it already holds cross-evolver helpers.
- **Do this instead:** put it in `get/src/dispatch.rs`. That module exists precisely to be the one
  place that knows both the config schema and pyo3, so the engine underneath does not have to.
- **Why:** `evolver/common.rs` is genome-*agnostic* generics over `G: Genome`, and the whole engine
  (`evolver/`, `genomes/`, `sir.rs`, `graph.rs`) deliberately has zero references to `pyo3` or
  `crate::config` — verified with `rg -ln 'pyo3|crate::config'` over those paths, empty. Config-aware
  helpers name concrete types (`EdgeEditGenome`, `SdaGenome`) and return `PyResult`; dropping them
  into `common.rs` drags both dependencies into the engine core and costs the ability to test the
  engine without a config or a Python interpreter. Reasoning in `decisions.md` 2026-08-11 11:26,
  the rejected-option writeup in `collab.md` #47.
- **Added:** 2026-08-11 — dispatch-goes-in-dispatch-rs-not-common-rs, while building #26's dispatch.

### GitHub's auto-delete does not fire on a PR you merged locally
- **Bites when:** you finish a PR the way this repo requires — `git merge --no-ff` locally, then
  `git push origin main` — and assume the repo setting cleaned the branch up, because GitHub does
  show the PR as merged.
- **Do this instead:** delete both copies yourself, as the last step of the merge:
  `git push origin --delete <branch> && git branch -d <branch>`. It is in `CLAUDE.md`'s merge
  snippet for that reason.
- **Why:** `delete_branch_on_merge` (enabled on this repo 2026-08-12) is a setting on *GitHub's own
  merge action*. A local merge reaches `main` as an ordinary push; GitHub notices the PR's commits
  are now on the base branch and flips its state to merged, but no merge action ran, so no cleanup
  runs either. Since every PR touching `.claude/work/*.md` **must** be merged locally — the
  `merge=union` driver is absent on GitHub's servers — the setting misses exactly the merges this
  repo does most.
- **Added:** 2026-08-12 — auto-delete-does-not-fire-on-a-locally-merged-pr, found after merging #63.

### A fix to `documentation/assets/site.js` that "did not work" is almost always the browser cache
- **Bites when:** you edit `site.js` or `style.css`, reload the docs site, and the old behaviour is
  still there — so you go back and "fix" code that was already correct.
- **Do this instead:** hard-reload (Ctrl-Shift-R / Cmd-Shift-R). When screenshotting headlessly,
  use a **fresh profile directory** per run: `firefox --headless --profile <new-dir> --screenshot
  out.png <url>` — a reused profile caches across invocations and will hand you a stale page with
  no warning.
- **Why:** both asset files are served with ordinary caching headers by `python3 -m http.server`
  and are requested by every page, so they are the two files a browser is most eager to keep. The
  failure is silent and looks exactly like a broken edit, because the HTML *is* current and only
  the behaviour is old. Hit 2026-08-12 while fixing the on-page contents: the fix was correct and
  two screenshots in a row showed it as not applied.
- **Added:** 2026-08-12 — docs-site-asset-caching-hides-your-edit.

### Changing an `eol` attribute makes files look modified while `git diff` shows nothing
- **Bites when:** you add or change a line like `*.sh text eol=lf` in `.gitattributes`. `git status`
  reports every matching file as ` M`, `git diff` on them prints **nothing at all**, and
  `git update-index --refresh` does not clear it. It looks like a corrupted index.
- **Do this instead:** confirm with `git ls-files --eol <paths>` before touching anything. Measured
  2026-08-13 on the seven `.sh` files: every one reported `i/lf w/lf` — index and working tree
  identical — so there was genuinely nothing to commit but `.gitattributes` itself. `git add` on
  them stages zero changes, which is the confirmation.
- **Why:** the attribute changes the *checkout representation* git would produce, so the entry is
  marked out of date even when the content matches. `git add --renormalize .` settles it, and a
  fresh checkout does too.
- **Added:** 2026-08-13 — eol-attribute-change-shows-phantom-modified-files

### `python3` does not exist on Michael's machine, and bare `python` is the Store stub
- **Bites when:** you run any snippet documented with `python3`, including
  `documentation/README.md`'s own site-verification script. `python3` is not found; bare `python`
  prints *"Python was not found; run without arguments to install from the Microsoft Store"* and
  exits **49**, which reads like a broken script rather than a missing interpreter.
- **Do this instead:** call the interpreter by full path —
  `/c/Users/micha/AppData/Local/Programs/Python/Python312/python.exe` from the Bash tool, or
  `& "C:\Users\micha\AppData\Local\Programs\Python\Python312\python.exe"` from PowerShell. The `py`
  launcher also works. **And do not pipe a heredoc containing backslashes through Bash into it** —
  `\` in `'\'` is eaten before Python sees it, producing an unterminated-string SyntaxError.
  Write the script to the scratchpad and run it by path instead.
- **Why:** the real interpreters are not on `PATH`; only the App Execution Alias stub is. Same root
  cause as the `cargo test` entry above, different symptom.
- **Added:** 2026-08-13 — python3-is-absent-and-bare-python-is-the-store-stub

### `grep -c` exits 1 when the count is zero, so `|| echo 0` prints TWO zeros
- **Bites when:** you write the obvious defensive idiom in a shell hook —
  `n=$(grep -c PATTERN file 2>/dev/null || echo 0)`. `grep -c` prints `0` **and** exits 1 on no
  match, so the fallback fires anyway and `$n` becomes the two-line string `0\n0`. A `printf` of
  several such counters then breaks across lines mid-sentence, which reads as a formatting bug
  somewhere else entirely.
- **Do this instead:** take the first line and default only if empty —
  `n=$(grep -c PATTERN file 2>/dev/null | head -1); n=${n:-0}`. `session_brief.sh` has a `count()`
  helper doing exactly this.
- **Why:** `grep`'s exit status reports whether a line matched, independently of what `-c` printed.
  It is not an error status, and `set -o pipefail` does not change it.
- **Measured:** 2026-08-13 in `.claude/hooks/session_brief.sh`, which had shipped the broken form
  since it was written. Found only because the parked-task work made the counts line print in a
  state where a counter was genuinely zero.
- **Added:** 2026-08-13 — grep-c-exits-1-on-zero-so-the-fallback-doubles-it

### A handoff's `Start here` is a bold label, not a heading — greps for `## Start here` find nothing
- **Bites when:** you write tooling that extracts the next action out of `handoff.md`. `/save`'s
  template writes `**Start here:** ...` inline, so an `awk`/`grep` anchored on `^## .*Start here`
  matches nothing and silently prints an empty block. It fails open, so the tool looks like it is
  working and merely has nothing to say.
- **Do this instead:** anchor on `^\*\*Start here` — or accept both forms, as `session_brief.sh`
  now does — and terminate on the next `## ` heading **or** the next `**Bold label:**`.
- **Why:** the handoff template uses bold labels for its sections and `##` for nothing but the
  title, so heading-shaped assumptions about it are wrong throughout, not only here.
- **Measured:** 2026-08-13. `session_brief.sh` had carried the heading-anchored form since it was
  written and had therefore never once printed a `Start here` block on any handoff in this repo.
- **Added:** 2026-08-13 — handoff-start-here-is-a-bold-label-not-a-heading
