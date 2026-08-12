# reject-stray-target-profile — GitHub #58

_Spans 2026-08-11 to 2026-08-12._

## Objective

Spec §8 requires `target_profile` be "rejected as a contradiction if supplied for any other
objective." #53 landed the first half of that clause (accepting an inline profile under
`epi_prof_match`); this task landed the second — rejecting one supplied under `epi_spread`,
`epi_length` or `python`.

## Outcome

`reject_fitness_seed` in `get/src/config.rs` was widened into `reject_stray_fitness_keys`, one
raw-text sweep over both named keys the `#[serde(flatten)]` on `SirParams` hides from
`deny_unknown_fields`. A `target_profile` under any objective but `epi_prof_match` is now a named
error; `epi_prof_match` with a profile is unchanged; the check stays a named-key check, not a
general unknown-key sweep (`collab.md` #25 pins that narrowness). 233 tests (231 → 233), clippy
and fmt clean.

Shipped as PR #63 (commits `bfa515b`, `d7cb289`, `7fc4c1a`), reviewed and merged by Michael as
`b225f30` on 2026-08-12. `config.example.toml` documents both the rejection and the narrowness.

## Left behind

- Nothing outstanding on this task specifically. One unrelated hotfix (`#[allow(dead_code)]` on
  `python_fitness`, waiting on GitHub #26) was checked at this `/done` gate and found already
  removed by Michael at an earlier gate (#26's own close-out, 2026-08-11) — noted in
  `decisions.md`'s task-complete entry for the record, not re-removed.
- `collab.md` #45 (Michael's note explaining why he merged #53 with this clause unimplemented,
  and filed #58) got a closing reply recording that the loop is now shut.
- Noticed but out of scope, staying in `collab.md`: item **#48** now names two different
  discussions under the same number (`config.example.toml`'s content, and auto-delete-on-merge) —
  a numbering collision between two sessions, not something this task caused or fixed.
