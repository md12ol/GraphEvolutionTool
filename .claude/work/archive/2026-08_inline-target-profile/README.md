# inline-target-profile — GitHub #53

## Objective

`FitnessConfig::EpiProfMatch` loses `target_profile_path: PathBuf` and gains
`target_profile: Vec<f64>`, in both front ends, with the target compared **verbatim** — neither
C++ loading convention (patient-zero prepend, `verts / 128` rescale) is reproduced.
`Config::validate` gains the two checks the old path field made impossible. Agreed at the joint
meeting of 2026-08-09; spec §8 had already been amended on `main` ahead of the code.

## Dates spanned

2026-08-09 (spec sheet amended, plan written) through 2026-08-10 (branch, implementation, PR,
merge, and four collab replies), across two sessions.

## Outcome

Shipped as three separately-verified commits (`de970ea`, `a80ec87`, `1ea2f6e`) on
`jsargant_inline_target_profile`, merged by Michael as PR #57 (`f25e33d`) after review on
Windows — 216 tests, clippy clean. GitHub #53 is closed. One follow-up spun out rather than
folding into this task: **GitHub #58**, rejecting `target_profile` under a non-`epi_prof_match`
objective (spec §8's contradiction clause), assigned to James.

## Left behind

- **GitHub #58** — assigned to James, not started. See `handoff.md` for the shape of the fix
  (`reject_fitness_seed` in `Config::from_toml_str` is the template).
- **`collab.md` #44** — an open sub-question to Michael (write the new practice-binding-skill-body
  rule into `CLAUDE.md` now, or hold it for the next joint meeting). Not this task's to resolve.
- **`python_fitness`'s `#[allow(dead_code)]` hotfix** — unchanged, still blocked on open #26.
  Re-verified at this task's `/done` gate; see `hotfixes.md`.
- **`main`'s `cargo fmt -- --check` failure** in `get/src/evolver/common.rs:45` — pre-existing,
  from #51, not this task's code. Staged in `issues.md` and withdrawn once it turned out Michael had
  already filed the identical finding as a comment on GitHub #56 in the same review pass.
- **The `sda.rs` cargo-doc warning** — pre-existing, Parked, unfiled. Untouched.
