# Next session — 2026-08-04 (written after PR #38 merged)

Read `.claude/work/current/plan.md` and `.claude/work/decisions.md` first (two new entries at the
tail, stamped James 2026-08-04 16:40 and 16:42), then `.claude/work/traps.md` — it gained an entry
this session that invalidates an audit command you have been trusting. `hotfixes.md` has one entry
of Michael's; nothing of yours.

**Where things stand:** GitHub **#14 is done, merged and closed.** PR #38 merged 2026-08-04 20:15
UTC as `168cc91`; issue #14 is `closed / completed`. **Every plan item is `[x]`** and there is
nothing outstanding in this task. `main` is clean and pushed; the branch
`jsargant_express_and_score` is merged and can be deleted.

**Start here: run `/done express-and-score`.** The task is complete and the archive gate has nothing
to hold it. Do this before starting anything new — the previous task's close-out sat uncommitted for
a day because the archive step was skipped, and Michael could not see any of it.

**Then, in priority order:**

- **Start #15** — "Convert fitness direction only at the Python boundary". Tier (1), yours, and the
  next task. Branch off `main` **after** `/done`, named `jsargant_<short-description>`. It was
  agreed on 2026-08-04 *not* to stack it on `jsargant_express_and_score`; that branch is merged now
  anyway, so branch off `main` as normal. Open a new task with `/start` — do not extend this plan.
- `#24` (config schema) is your other tier-(1) issue. All tier (1) work comes before tier (2).

**Watch out for:**

- **`uniq -d` is not sufficient to audit the shared docs any more.** Measured twice on `main` today:
  union merge can splice one entry into the middle of a line of another, which duplicates **no**
  line, so the documented check returns clean on a corrupted file. Check structure too —
  `grep -n '^### [0-9]' .claude/work/collab.md` — every item heading at column 0, count as expected.
  Full entry in `traps.md`.
- **Check `collab.md`'s tail before choosing an item number.** Michael and I both raised an item
  **20** today because neither had pulled. The collision is flagged in the file and is his to
  resolve; do not renumber his entry.
- **`collab.md` item 20 is open and waiting on Michael** — one stale line in `CLAUDE.md` (the
  "Byte-identical lines in `.claude/work/*.md`" bullet in "Pull requests"). Decided 2026-08-04 to
  leave it with him. Do not fix it unilaterally.
- **A PR can merge mid-session and nothing local tells you.** #38 merged while this save was being
  written. Read PR and issue state from the remote —
  `gh api repos/md12ol/GraphEvolutionTool/pulls/<n> --jq '.state, .merged'` — never from a doc.
- **`gh issue view <n>` and `gh pr edit` are broken on this repo** (projectCards deprecation). Use
  `--json` on reads and the REST API on writes. In `CLAUDE.md` under "Filing issues".
- **Never a bare `cargo fmt`** — `generational.rs` and `sda.rs` are issue #22, Michael's. Format per
  file: `rustfmt --edition 2024 <path>`.
- **Never a bare `git add -A`** — `docs/` and `GET GA planning session.md` are untracked stale
  pre-spec-sheet documents that must not be committed. Stage explicit paths.
- **`cargo clippy -- -D warnings` cannot pass on `main`** — two pre-existing dead-code errors in
  `generational.rs` from the unbuilt #25. Compare against the baseline, don't expect zero.

**⏰ Time-sensitive:** nothing dated. Two things sit with Michael and neither blocks you:
`collab.md` item 20, and item 21 (drop-in Rust objectives — a meeting question that changes #26,
not #15).
