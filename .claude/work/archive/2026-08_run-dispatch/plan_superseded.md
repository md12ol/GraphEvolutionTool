# Superseded — original wording of tasks now done or dropped

Reference only, never actionable. Created 2026-08-11 by the first task this plan displaced.

---

## Dropped 2026-08-11 — the `target_profile` field-swap hotfix

Written as task 1 of the #26 plan on 2026-08-10, when `FitnessConfig::EpiProfMatch` still carried
`target_profile_path: PathBuf`. It was implemented in full, then **reverted uncommitted** on
2026-08-10 once we learned James had uncommitted work on GitHub #53 — the issue it was working
around, assigned to him. #53 then landed properly as PR #57 (merged `f25e33d`), so the swap exists
on `main` as real work rather than as a hotfix. No commit of this ever existed, and `hotfixes.md`
never gained the entry it briefly described.

Original wording:

> - [ ] Hotfix: `FitnessConfig::EpiProfMatch.target_profile_path: PathBuf` → `target_profile: Vec<f64>` — `get/src/config.rs:126`, `get/src/config.rs:686` (parse test)
>       **Verify by:** `cargo build -p get`; `grep -n target_profile_path get/src/config.rs` shows nothing left uncommented as live code (py_config.rs untouched — its own mirror is #53's job).
>       Logged in `hotfixes.md`, owner Michael, `Remove when: #53 lands and does the full job (validation, py_config.rs mirror, config.example.toml, examples/config_builder.py, round-trip test)`.

Two things it got wrong, both worth keeping because they would recur:

- **"py_config.rs untouched" was false.** The round-trip test destructures
  `config::FitnessConfig::EpiProfMatch` exhaustively, so the mirror fails to *compile* the moment
  the config side moves — the alarm #53's own body predicted. The two halves cannot even be split
  across two commits: once `to_toml_value` emits `target_profile`, a rendering still saying
  `target_profile_path` no longer parses.
- **Calling it a hotfix was the wrong shape.** The swap was permanent, so the entry's `Remove when:`
  would have removed only the entry, not any code. `hotfixes.md` is for temporary code; this was
  cross-owner coordination, which is `collab.md`'s job.

Also measured while it was live, and independently pinned by James in #53's
`a_whole_number_in_the_target_profile_may_be_written_without_a_decimal_point`: `toml` widens integer
array elements into `f64`, so `target_profile = [1, 3, 7]` parses into a `Vec<f64>`.
