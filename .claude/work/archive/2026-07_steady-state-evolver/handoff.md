# Next session — 2026-07-31

Read `/official_spec_sheet.md` first — it is now the design authority for this project, agreed by
both owners. Then `.claude/work/current/plan.md` and `.claude/work/decisions.md` (twelve new
entries at the tail, stamped "Michael & James").

**Where things stand:** The steady-state task is finished — all six items `[x]`, 97 tests passing
on `main`. A later session on 2026-07-31 ran a joint design call with James and produced
`/official_spec_sheet.md`, which supersedes the untracked `IMPLEMENTATION.md`. No code was written.
The spec deliberately records design the tree does not yet implement, so the two disagree by
construction right now.

**Start here:** run `/done steady-state-evolver` to archive the finished plan, then `/start` to
open a task for the spec-driven implementation. Do **not** append spec work to the old plan — it is
a different task, and the empty `work/archive/` is the exact symptom `CLAUDE.md` warns about.

**Watch out for:**

- **The spec outranks the code, and only here.** Where `/official_spec_sheet.md` and `get/src/`
  disagree, the sheet is the intent — the reverse of this project's usual "the repo wins" rule.
  Five things are specified and unbuilt: the one-mutation contract with `max_mutations`, the SDA
  alphabet derived from the edge cap, the `express_and_score` rename and sole-entry rule,
  `Config::validate`, and the entire fitness and Python layer.
- **`collab.md` #11 is reversed, not agreed.** It asks for a `direction` parameter on
  `generation_stats`; the spec removes it. Do not implement #11 as written.
- **`IMPLEMENTATION.md` is untracked and superseded.** The owners have a copy and want it deleted;
  it was left in place rather than deleted mid-session. Deleting it is unrecoverable via git.
- **Build order lives in GitHub issues, not a document.** 16 filed on 2026-07-31 (#14–#29) with
  estimates and assignees, plus three pre-existing ones enriched (#6, #10, #13). Do not write a
  sequencing markdown file — that decision is recorded in `decisions.md`.
- **`gh issue edit --body-file` replaces the whole body.** Both existing-issue enrichments
  preserved the original one-line body by hand, at the top, above a `---`. Anyone editing #6/#10/#13
  again must do the same or the author's original words vanish silently.

**⏰ Time-sensitive:** the `cargo fmt` + readability issue staged in `issues.md` needs James's tree
to be clean when it lands, and it touches `generational.rs`, which is his live work. Coordinate
before running it — and `generational.rs` is also where implementation work starts, so the two
collide if both move at once.
