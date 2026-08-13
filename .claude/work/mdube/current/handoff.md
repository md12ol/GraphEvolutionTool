# Next session — 2026-08-13

**Machine:** `skynet` · saved 2026-08-13 17:05 · `b4e3bb7`

Read this task's `plan.md` and `.claude/work/decisions.md` first, then `.claude/work/traps.md` —
two entries there are worth knowing before you touch this checkout again.

**Where things stand:** GitHub #21 is done. Every task-list item is `[x]` and verified, including
the last one: **PR #71 is open** against `main`, body verified intact
(https://github.com/md12ol/GraphEvolutionTool/pull/71). Nothing left to build or verify on this
task — it's waiting on James to merge (nobody merges their own PR, per `CLAUDE.md`).

**Start here:** nothing code-side. Once #71 is merged, `/done run-output` closes the task out —
archives `work/mdube/current/` to `work/archive/2026-08_run-output/` and clears the desk for the
next task. Until then there is nothing to do here; check `gh pr view 71 --json state` if picking
this back up cold.

**Watch out for:**

- **`.venv/` is real but untracked-and-ignored** (`.gitignore`'s `.venv/`, on `main` since
  `092b944`, unrelated to #21). `source .venv/bin/activate` before any Python work; it has
  `maturin`, `pandas`, `matplotlib` installed.
- **Two sessions shared this checkout mid-task and it cost a stop-and-diagnose** —
  `traps.md`, `two-sessions-sharing-one-checkout-can-cross-wires-on-different-branches`. If a file
  you edit shows a stale-content warning, check `git reflog` before trusting anything.
- **The provenance TOML path is derived** (`{filename}.toml`), not a second argument to
  `save_results` — answers the plan's open question with its own stated default; James never
  weighed in, so this is worth confirming during PR review rather than assumed settled.

**⏰ Time-sensitive:** nothing dated on this task itself.
