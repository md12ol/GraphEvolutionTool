# History — GitHub #20: replicate runs (one master seed, Rust-only parallelism, max_cores)

Append-only session log for this task, newest session first.
Maintained by `/save`; archived by `/done`.

---

## Session 2026-08-13: all 8 tasks done, PR #83 open

Full task list executed and committed one task at a time, each paused for approval before
committing. `cargo test -p get`: 250 passed throughout (up from 237 at branch cut). fmt, clippy
`--all-targets` and `cargo doc` clean at every commit.

**Design fork resolved before coding, not mid-task:** `run` returns a list unconditionally,
matching §8.1 literally rather than keeping `n_runs=1` backward compatible — see `decisions.md`.

**Mutation-checked every new test**, which paid for itself four times:
- `replicate_seeds`: `master ^ i` satisfies the reproducibility test perfectly and is caught only
  by the nearby-master collision test; a stream depending on `n_runs` is caught only by the prefix
  test.
- `run_replicates`: inverting the python/native gate **hung the test suite** rather than failing
  it — a real GIL deadlock, not a hypothetical one (`.claude/reference/pyo3-maturin.md` §2).
- Replicate tests: handing every replicate the master seed (all `n` runs identical) is caught
  **only** by the distinctness test; the three reproducibility tests all pass on that
  implementation regardless.

**One design error, caught by an inherited test rather than a new one:** `RunResult.seed` was
first wired to the derived per-run seed. `run_returns_a_complete_result_object` (written for #71,
not this task) failed immediately. See `decisions.md` for why the master is correct and the
derived seed is not replayable.

**Real-Python verification**, `maturin develop --release` into the existing `.venv`:
```
run(seed) with no n_runs -> list of 1
max_cores=1 vs 8, 8 runs   -> identical results
50 runs vs 30 runs         -> first 30 identical, element for element
                              31 distinct fitness values across 50 runs
provenance                 -> seed == master on all 50, run_index 0..49 in order
n_runs=0     ValueError: n_runs must be at least 1; ...
max_cores=0  ValueError: max_cores must be at least 1 if given; ...
save_logs on replicate 17  -> ...,seed,run_index
                              0,2.75,...,20260813,17
```
First wall-clock attempt used a 0.16s workload and showed only 2.0x speedup — too small to measure
anything, not a ceiling. Repeated on a scoring-dominated config (network_size=120,
num_epidemics=60): 1.74s → 0.96s → 0.50s at max_cores 1→2→4, flat at 8 (expected — 8 replicates,
4+ cores has nothing left to parallelize).

**PR #83** opened against `main`, review requested from Michael. Body verified by read-back
(byte-identical to what was sent, apart from GitHub's trailing newline). Flags a real merge
collision with **PR #72** (still open, `set_base_graph`): both branches touch `dispatch::evolve`'s
call sites — #72 adds a `base_graph` parameter, #83's `run_replicates` calls `evolve` twice at
`dispatch.rs:442` and `:460`. Whichever merges second needs those two calls updated. Same file's
`documentation/jsargant_edits.md` placeholder conflicts too — both branches replace the same
"Nothing pending" text; resolution is keeping all four entries (three from #72, one from #83).

**`collab.md` #64 replied to.** Michael's message (21:22 his time) asking for the #61
implementation crossed with the fix already having shipped 20 minutes earlier (14:59 EDT).
Replied inside #64 pointing at the commit and PR #72's two review comments.

**Git manifest at end of session:** branch `jsargant_replicate_runs`, 5 commits ahead of `main`
(`6f8fc5c`, `50e4f7b`, `1abd10f`, `aa3e05c`, `8cba899`), **pushed**, level with `origin`. Working
tree clean. `main` unchanged at `042f282` throughout — no merge was needed. `.venv/` exists,
gitignored, rebuildable via `maturin develop --release`.

**Next:** nothing but Michael's review and merge of PR #83 — same shape as `set-base-graph`/#28,
parked in `work/jsargant/parked/set-base-graph/` waiting on PR #72. This task should probably be
parked too, once the user says so.
