# config-schema — GitHub issue #24

**Dates:** 2026-08-05, single day (one build session plus a recovery session after a machine crash).
**Owner:** James. **Shipped as:** PR #42, `39c408a`, merged `988457e`. **Issue #24 closed**
2026-08-05T22:17:43Z.

## Objective

Make `get/src/config.rs` parse the schema `official_spec_sheet.md` §7 actually specifies —
`FitnessConfig`'s four real variants with the SIR block flattened, `SirParams` gaining
`num_epidemics` / `min_epidemic_length` / `max_epidemic_retries` and losing `seed`,
`GenomeConfig::Sda` losing `num_chars` — and rewrite `config.example.toml` to match.

## Outcome

Delivered in full and merged. `config.rs` now parses all four objectives including
`type = "python"`, which the old enum could not; omitted retry settings default to the C++
constants 3 and 5 via explicit `fn`s rather than to 0; `EpiProfMatch` carries a
`target_profile_path` that `config.rs` never opens, keeping parsing pure deserialization. 135 tests
green against a 128 baseline (+7), clippy `diff`-identical, no rustdoc warnings in `config.rs`.
`Config::validate` was deliberately untouched — it is issue #23.

**One criterion was dispositioned, not delivered.** #24's `Verify by` asked for a stray `seed` under
`[fitness]` to be rejected as an unknown key. Serde cannot: a `#[serde(flatten)]` field
deserializes through a buffered content map, so `deny_unknown_fields` never fires. Spec §7 requires
the flatten, so the verify line gave. The behaviour is pinned by a test, reasoned in `decisions.md`
2026-08-05 15:47, recorded in `traps.md`, and the check was handed to #23's `validate` via
`collab.md` #25.

## ⚠️ The history here is partly reconstructed

The machine crashed before this task's final `/save`, leaving `history.md` as a bare header and no
`handoff.md`. Both were rebuilt on 2026-08-05 from `plan.md`, the commit trail and the GitHub API,
and `history.md` says so at the top. **The narrative was lost; the rationale was not** — it had
already been written to `decisions.md` and `collab.md` before the crash. Treat the archived
`history.md` as accurate on facts and silent on anything that was only ever said out loud.

## Left behind, outliving this task

- **`collab.md` #24** — what a `Profile*.dat` actually contains (patient-zero prepend, `verts / 128`
  rescale). Open, awaiting Michael, needed before **#26** turns a path into a `Vec<f64>`.
- **`collab.md` #25** — unknown `[fitness]` keys parse silently. Open, awaiting Michael; the check
  belongs in **#23**'s `Config::validate`.
- **SIR batch-seed hotfix** (`get/src/fitness.rs:162-164`) — Michael's, load-bearing, blocked on
  **#18**. Verified still present at this gate; fourth cycle.
- **Not this task's, but live for James:** `collab.md` **#27** (`Swap`'s degree floor, `> 2` vs. the
  Java original's `>= 2`) and **#30** (review Michael's new `pull_main.sh` `SessionStart` hook).

*Archived 2026-08-05 22:30 EDT — James, at `/done`.*
