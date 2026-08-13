# Next session — 2026-08-13

**Machine:** `skynet` · parked 2026-08-13 · `fc66715`
**Blocked on:** nothing external — parked to let `result-object` go first, since it has real queued
work (tracker edits for #21 and #68) and its own blockers are cleared. Resume with
`/load run-output` whenever result-object is done or set down again.

Read this task's `plan.md` and `.claude/work/decisions.md` first, then `.claude/work/hotfixes.md`.

**Where things stand:** GitHub #21 is open and planned; nothing is built beyond the stacked branch.
You are on `mdube_run_output`, created off `mdube_result_object` (PR #65) with
`mdube_per_owner_work_dirs` (PR #69) merged in. **PRs #65, #66 and #69 are now all merged into
`main`** (confirmed this session) — the branch no longer needs to carry them as a stack. Only task 1
of 11 is done. No source file has been touched. `origin/main` is 8 commits ahead of this branch.

**Start here:** `git merge main` into `mdube_run_output` first — cheap now, and the longer the stack
stands unflattened the more a later rebase costs. Then add `ci_95` to the engine's
`GenerationStats` and compute it in `generation_stats()` at `get/src/evolver/common.rs:292` —
half-width `1.96 · s / √n` using the **sample** deviation (`n-1`), beside the existing population
`std_dev` (`n`). `n = 1` must give `0.0`, not NaN. Write the two unit tests named in the plan's
second task, then:

    $env:PATH = "C:\Users\micha\AppData\Local\Programs\Python\Python312;$env:PATH"
    cargo test -p get

**Watch out for:**

- **The two denominators differ on purpose.** `std_dev` divides by `n`, `ci_95` by `n-1`. It reads
  like an inconsistency and is not — do not "fix" it, and say why in the comment rather than citing
  where it was agreed.
- **`cargo test` needs Python on `PATH`**, or every test dies with `STATUS_DLL_NOT_FOUND` before
  running — `traps.md`, `cargo-test-cannot-link-python-unless-extension-module-is-off`.
- **`python3` does not exist here and bare `python` is the Microsoft Store stub** — call the
  interpreter by full path.
- **Shipped source must not reference `official_spec_sheet.md` or issue numbers** (amended
  2026-08-13). This branch inherits #65's clean comments; keep them clean.
- **Do not edit `documentation/`.** File what the site now gets wrong in
  `documentation/mdube_edits.md` — `collab.md` #53. Both queues are currently empty.
- **This branch has never been pushed** and no PR exists for it. Opening one needs an explicit
  instruction.

**Two open questions in `plan.md`, neither blocking yet:** whether `run_index` ships as a hard `0`
now (planned: yes, so the CSV schema does not change under users when #20 lands), and whether the
provenance TOML gets a derived filename or an explicit argument.

**⏰ Time-sensitive:** nothing dated on this task itself. `collab.md` #55–#57 landed on `main` since
this branch was cut and haven't been read from here yet — worth a skim after the merge.
