# History — Implement config→concrete-type dispatch in `GraphEvolver::run` (#26)

Append-only session log for this task, newest session first.
Maintained by `/save`; archived by `/done`.

---

## Session 2026-08-11 (close-out): PR #60 merged, #26 closed, task archived

**Merge confirmed.** PR #60 merged 2026-08-11T16:00:19Z as `97d9e02`, by James — a review merge,
not a self-merge. GitHub #26 is `closed`. Verified on `main` post-merge on this machine:
`cargo test -p get` 231/231, working tree clean at the start of the session.

**Two commits had landed on `main` since the last save's handoff was written**, neither by this
task's branch: `0a3de27` (corrected a stale commit hash in `hotfixes.md`, added the
dispatch-vs-common trap) and `e745a26` (added `deferred.md` for post-v1 work, raised `collab.md`
#49 on packaging). The uncommitted `deferred.md` / `CLAUDE.md` / `.gitattributes` changes that
`/load` flagged as unexplained were these, since committed and pushed — the handoff predated them.

**Hotfix removed, condition met.** #19's `#[allow(dead_code)]` on `GraphEvolver::python_fitness`
had been waiting since 2026-08-07 for #26 to become its caller. Verified on `main`, not inferred:
the attribute is gone, and the method now lives at `get/src/dispatch.rs:160` (moved by PR #60's
dispatch-extraction commit, not by this session). The `hotfixes.md` entry is deleted, replaced by a
one-paragraph removal note under its section heading so the removal is legible rather than a gap.

**One follow-up staged.** `run`'s doc comment at `get/src/lib.rs:219-232` is still headed "For
whoever implements the dispatch (#26)" and tells the implementer to call `python_fitness` and
release the GIL — both already done in the body directly below it — and cites the
`#[allow(dead_code)]` that no longer exists. Staged in `issues.md` as ready-to-file, assigned to
Michael. Not fixed in-session: it is under `get/src/`, so it needs a branch and a PR.

**Artifact, not a doc.** Published a private artifact explaining the dispatch-vs-`common.rs` split
with flowcharts of the Python↔Rust run flow. It renders `decisions.md` 2026-08-11 11:26 and
`collab.md` #47; those stay the source of truth, the artifact is a view of them.

**Git manifest.** One repo, on `main` at `97d9e02` + this close-out's doc commit. No feature branch
outstanding — `mdube_run_dispatch` is merged. Nothing uncommitted at the point of the archive
beyond the close-out itself.

## Session 2026-08-11: #26 implemented end to end, PR #60 open

**Started** on `main` with `work/current/` empty; scaffolded the task for GitHub #26 and branched
`mdube_run_dispatch` off `main` at `7b22f1f`.

**#53 collision, reverted.** Began implementing the `target_profile_path` → `target_profile: Vec<f64>`
swap as a hotfix (needed for #26's `EpiProfMatch` arm) before learning James was already working #53
himself. Fully reverted **uncommitted** — `git checkout -- <files>` on all four touched files plus the
draft `collab.md`/`hotfixes.md` entries, confirmed `git diff main --stat` empty before continuing.
James's PR #57 landed #53 properly shortly after; rebased onto it (fast-forward, branch had no
commits yet).

**`/start`'s branch-as-task-0 gap, fixed.** Caught while planning #26: the written plan had six tasks
under `get/src/` and no task creating a branch first. Added a bullet to `/start`'s SKILL.md body
(`13fbba9`, direct to `main` — body-only, no frontmatter). Raised as `collab.md` #44; James
acknowledged and independently found the same gap in his own `#53` plan, then proposed — and I
agreed — that skill-body changes binding the other owner's practice get a `collab.md` item with an
explicit ACKNOWLEDGE ask rather than a PR.

**PR #59, a side fix.** James had noted `main` wasn't `cargo fmt`-clean (`common::best_index`'s
assertion, from my own `best_index` extraction under #51). Fixed on its own branch
`mdube_fmt_best_index`, not #26's, so the two reviews stayed separate. Commented on #56 (the sweep it
belongs to) to say the bullet was resolved without the rest of the sweep being touched. James merged
it mid-session (`152a5b8`).

**#26 implemented, four commits on `mdube_run_dispatch`:**
- `e1b97f5` — objective erased to `Box<dyn Fitness>` (`GraphEvolver::objective`,
  `sir_sample_params`); deleted #19's `#[allow(dead_code)]` on `python_fitness`, now called.
- `395e03c` — `edge_edit_start`/`sda_start` population + context builders; `run` draws the
  evolver's seed from the same stream rather than reusing `seed`, so the population and the
  evolution don't replay each other's draws.
- `34f4d6b` — extracted the whole dispatch surface into new `get/src/dispatch.rs`, rejecting
  `evolver/common.rs` as the home (it would drag `pyo3`/`config` into the genome-agnostic engine
  core). Pure move: test count unchanged at 226 across the commit. `decisions.md` 2026-08-11 11:26,
  `collab.md` #47 (also flags a stale `config.rs` doc line for James's #58 diff).
- `7e2f5d7` — the 2+2-arm strategy×genome dispatch (`run_strategy<G, F>` generic over genome, called
  once per genome arm) and `run` wiring: GIL released via `Python::attach(|py| py.detach(..))`,
  `best_fitness` cached in the **objective's own units** (confirmed with the user, not engine
  orientation — #27 removes the field regardless).

**Verified beyond `cargo test`.** Built the real extension module
(`cargo build -p get --features pyo3/extension-module --release`) and ran the shipped
`config.example.toml` from Python: 500 generations, 3.4s, reproducible at a fixed seed. Caught that
the end-to-end test's first assertion (`best_fitness >= 1.0`) would pass a fully-backwards search —
the shipped example's `1.47` is suspiciously close to that floor. Measured directly: flat at
`infection_rate = 0.05` (1.20/1.30/1.27/1.20 across 0/5/50/300 generations — no gradient, outbreaks
die immediately on a sparse graph), but climbs 11.5→22→52→71.5 at `0.5`. Confirmed maximization is
correct; the shipped example just has no signal. Replaced the weak assertion with
`a_maximizing_objective_actually_climbs_through_the_dispatch`. Staged the example finding in
`issues.md` (Parked) and raised the "what should the example demonstrate" question as `collab.md`
#48, parked behind the current issue set per the user's direction.

**Gate at session end:** 231/231 tests, clippy clean at `-D warnings`, `cargo fmt --check` clean
(`common.rs` too, now that #59 is merged).

**PR #60 opened**, `mdube_run_dispatch` → `main`, body re-read and verified intact (6164 chars, 6
sections). Not merged — James's call. Two follow-ups flagged in the PR body rather than left implicit:
drop the `hotfixes.md` `python_fitness` entry on merge, and take the one-line `config.rs` doc
correction in #58's diff.

**Two stale commit-hash references caught and fixed this save** — `hotfixes.md`'s `Last checked` and
several `plan.md` task lines cited pre-rebase hashes (`36ae59d` etc.) that moved when the branch was
rebased onto `main` twice mid-session. Corrected to current hashes; `traps.md` doesn't yet have an
entry for "don't trust a commit hash cited on an open branch that gets rebased" — worth adding if it
recurs.

**Git manifest at save:** `mdube_run_dispatch` at `7e2f5d7`, 4 commits ahead of `main`, pushed and
tracked (`origin/mdube_run_dispatch`). Working tree clean. `main` also carries, from this session: the
`/start` branch-as-task-0 fix (`13fbba9`), `collab.md` #44 through #48, `decisions.md` 2026-08-11
11:26, `hotfixes.md`'s `python_fitness` stamp plus this save's hash correction, `issues.md`'s
flat-example entry, `traps.md`'s dispatch-vs-common entry, and PR #59's merge (`152a5b8`, by James).
