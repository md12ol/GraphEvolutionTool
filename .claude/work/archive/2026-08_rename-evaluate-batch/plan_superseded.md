# Plan — superseded wording

Original wording of tasks now done, kept for reference only. Never actionable — see `plan.md` for
current status.

---

## Original task list, as written 2026-08-10

- [ ] Cut `mdube_rename_evaluate_batch` off `main`.
      **Verify by:** `git log --oneline -1` matches `main`'s current head.

- [ ] Rename `SirRun` → `Epidemic` across `get/src/fitness.rs` (8 occurrences) and `get/src/sir.rs`
      (7 occurrences). Mechanical; check doc comments and test names too, not just the type name.
      **Verify by:** `grep -rn 'SirRun' get/src/` returns nothing; `cargo build -p get` clean.

- [ ] Rename `Fitness::evaluate_population` → `evaluate_batch` across `get/src/fitness.rs` (33),
      `get/src/evolver/common.rs` (6), `get/src/evolver/generational.rs` (1), `get/src/lib.rs` (1).
      Include the trait method, every impl, every call site and doc comment.
      **Verify by:** `grep -rn 'evaluate_population' get/src/` returns nothing; `cargo build -p get`
      clean.

- [ ] Confirm `lib.rs`'s one occurrence and James's hotfix don't collide. His live hotfix
      (`#[allow(dead_code)]` on `GraphEvolver::python_fitness`, `lib.rs:303`, `hotfixes.md`) isn't
      one of the renamed identifiers, so it should be untouched — verify rather than assume.
      **Verify by:** `git diff` on `lib.rs` touches only rename occurrences; the `#[allow(dead_code)]`
      line is unchanged.

- [ ] Amend `official_spec_sheet.md` at the four cited lines (221, 269, 368, 794, 805 — issue body
      table). Sheet changes require the joint-meeting rule; this one is already covered by the
      2026-08-09 meeting, so the PR carries the amendment directly — no separate collab.md item.
      **Verify by:** `grep -n 'evaluate_population\|SirRun' official_spec_sheet.md` returns nothing.

- [ ] Full verify pass: `cargo test -p get` (213, unchanged count), `cargo clippy -p get
      --all-targets -- -D warnings` clean, `cargo fmt -- --check` clean.
      **Verify by:** all four commands green in one sitting, on this machine
      (`LD_LIBRARY_PATH` needed for `cargo test` — see `traps.md`).

- [ ] `decisions.md` entry for the sheet amendment, stamped with both names per the joint-meeting
      rule (`CLAUDE.md`, "Changing it is a `decisions.md` entry too").
      **Verify by:** entry present, `uniq -d` clean on `decisions.md`.

- [ ] Open PR, request James's review (he merges it — `CLAUDE.md`'s PR rule).
      **Verify by:** PR exists on GitHub, references issue #52.
