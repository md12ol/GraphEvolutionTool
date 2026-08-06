# config-validate — GitHub issue #23

**Dates:** 2026-08-05 to 2026-08-06, one working session (planned and built in the same sitting).
**Owner:** James. **Shipped as:** PR #45, `5fd8dbc` + `2c590f4` — **open and unmerged at archive
time**, awaiting Michael. Body carries `Closes #23.`

## Objective

Implement `Config::from_path`, a `todo!()` until now, and add one `Config::validate` carrying every
constraint in spec §7 — so the TOML and Python front ends validate through the same function rather
than the Python path silently accepting what serde would have rejected.

## Outcome

Delivered in full. `validate` covers §7's list in four helpers, with the cases that resist a blanket
check handled explicitly: the tournament floor of 4 is **steady-state only**, `elite_count` applies
only to **generational**, the edge-edit weights **delegate** to the existing
`EdgeEditOperationWeights::validate` rather than restating its rules, and a `python` objective
carries no SIR block so it skips the epidemic checks entirely. `min_epidemic_length = 1` is pinned
as **legal** — it is the only way to opt out of §5.2's re-roll bias, and a test exists so it is not
"corrected" to 2 later. Errors are `ConfigError::Validation { field, constraint }`, never panics.

154 tests, up from 135 (+19). Clippy `diff`-identical to a baseline captured on the clean tree
*before* editing; rustdoc back to its 4 pre-existing warnings with none in `config.rs`; rustfmt
clean with no other file touched.

## What it changed that was not purely additive

- **Superseded a test #24 wrote.** `an_unknown_fitness_key_is_ignored_rather_than_rejected` pinned
  the silent-ignore gap; #23 closes it, and that test's own comment nominated #23 to do so. Replaced
  by a pair pinning both the new rejection and the check's deliberate narrowness.
- **`from_toml_str`'s signature changed** to `Result<Self, ConfigError>` — the seed rejection is not
  a TOML error. `from_path` was its only external caller.
- **One line in `get/src/lib.rs`**, which otherwise belongs to #26: `{err:?}` → `{err}`, so a bad
  config reaches Python as a message rather than a Debug dump. Its own commit, called out in the PR.

## Archived with its PR still open

Third task closed this way, after #15 and #24, on the disposition in `decisions.md` 2026-08-05
15:09: the open item owed **this owner** no action, which was established by verifying the closing
keyword on the remote rather than assuming it. Had the PR closed unmerged, #23 would return as new
work.

## Left behind, outliving this task

- **`collab.md` #24** — the `Profile*.dat` format. Open, awaiting Michael, needed before **#26**.
- **`collab.md` #25** — **answered by this task**; needs only Michael's acknowledgement. The answer
  corrects what was originally claimed: the check could not live in `validate`, and went into
  `from_toml_str`, making it TOML-only and narrow to `seed` by name.
- **`collab.md` #27** — `Swap`'s degree floor, `> 2` vs the Java original's `>= 2`. Waiting on
  James; carried forward untouched by explicit choice at this gate. Loosening it is a spec
  amendment and needs a joint meeting.
- **SIR batch-seed hotfix** (`get/src/fitness.rs:162-164`) — Michael's, load-bearing, blocked on
  **#18**. Fifth cycle; `#23` never touched `fitness.rs`.

## One convention deviation, recorded not glossed

Michael's 2026-08-05 rule says commit each verified step of a feature branch separately. `config.rs`
was built in one sitting and landed as a single large commit rather than the six its plan items
imply, because splitting after the fact would have produced intermediate commits that fail to
compile or emit dead-code warnings. The reviewer's diff is what pays for that.

*Archived 2026-08-06 00:45 EDT — James, at `/done`.*
