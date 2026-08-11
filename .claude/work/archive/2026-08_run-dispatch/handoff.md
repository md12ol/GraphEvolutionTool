# Next session — 2026-08-11

Read `.claude/work/current/plan.md` and `.claude/work/decisions.md` first, then `.claude/work/hotfixes.md`.

**Where things stand:** #26 is implemented and out for review as **PR #60**
(`mdube_run_dispatch` → `main`, 4 commits, 231/231 tests, clippy + fmt clean). Nothing local is left
to do on it — it's waiting on James.

**Start here:** check `gh pr view 60 --repo md12ol/GraphEvolutionTool --json state,reviews -q '.state, .reviews'`.
- If merged: run `/done` to archive this task, then `/start` on the next issue (survey
  `gh issue list --repo md12ol/GraphEvolutionTool --state open` — #27, #28, #21, #20, #56 are open;
  #27 is the natural next pick, it touches code #26 just wrote).
- If review comments landed: address them on `mdube_run_dispatch`, one commit per fix, same branch.

**Watch out for:**
- `hotfixes.md`'s `python_fitness` entry says to remove it **when #26's PR merges** — do that as part
  of closing this task, not before. `main` still carries the `#[allow(dead_code)]` until then.
- `config.rs`'s module doc still says the dispatch layer is "in `lib.rs`" — deliberately left for
  James's #58 diff (`collab.md` #47). If #58 lands without fixing it, it's a one-line follow-up.
- `collab.md` #48 (what should `config.example.toml` demonstrate) is parked, not forgotten — surface
  it at the next joint meeting rather than deciding it solo.
- Rebasing an open branch moves commit hashes. This bit twice this session (`hotfixes.md` and
  `plan.md` both cited a pre-rebase hash) — cite by commit *message* in docs, not hash, until a
  branch is merged.

**⏰ Time-sensitive:** none.
