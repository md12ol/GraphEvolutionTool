# Archive: epidemic-seeding (Issue #18)

**Objective:** Replace the `EpidemicScorer::batch_seed` stub with the real mechanism spec §5.2
specifies — the scorer holds the run seed plus an atomic evaluation counter, ticking once per
*batch* (not per graph), so a batch's epidemic seed derives from `(run_seed, counter)`.

**Spans:** 2026-08-06 → 2026-08-10 (sessions logged in `history.md`; the code itself was done and
merged by 2026-08-07, the last two sessions were catch-up and close-out only).

**Outcome:** `EpidemicScorer` gained `batches_scored: AtomicU64` and `next_batch_seed`, deriving
via a SplitMix64 mix (`mix_seed`). All three objectives override `evaluate_batch` (renamed from
`evaluate_population` during the task) to draw one seed per batch and fan the scoring out from it.
Landed as PR #47, merged by James as `fd0d920`. One batch's graphs face identical dice, consecutive
batches differ, and the same run seed reproduces a whole sequence exactly — covered end-to-end by
`the_same_run_seed_replays_every_batch_of_a_run`. The `hotfixes.md` entry describing the frozen
stub was deleted in the same push (`f8673dc`). Suite went 154 → 176 (163 on the branch, plus
James's #46 landing alongside it on `main`).

Along the way: a terse comment pass and `run`/`epidemics` renaming (2026-08-06), naming
**original** vs **oriented** fitness on `Direction` (2026-08-07), and a structural simplification
after sub-agent review cut `EpidemicScorer` from 5 methods to 2. The `evaluate_population` →
`evaluate_batch` / `SirRun` → `Epidemic` rename was scoped out during the task and later agreed at
the 2026-08-09 joint meeting as its own piece of work — filed as **GitHub #52**.

**Left behind, unaffected by this task's close:**
- Hotfix: `#[allow(dead_code)]` on `GraphEvolver::python_fitness` (`get/src/lib.rs`) — James's,
  from #19, blocked on Michael's **#26** (config-to-concrete-type dispatch). Still in `hotfixes.md`.
- Parked issue: `cargo doc` double warning on a private intra-doc link in `sda.rs` — cosmetic,
  still in `issues.md`.
