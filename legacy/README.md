# Legacy C++ — reference, not a build target

The original implementation this project is a port of. It is **tracked deliberately**: the Rust
cites it ("ported from `Graph::SIR`"), decisions get argued against it, and while it was ignored at
the repo root those citations pointed at a file only one of us had.

**Nothing here is built, tested, or run by this repo.** It is not wired into Cargo and it will not
compile as it stands — `main.cpp` includes `../BitSprayer/Bitsprayer.h` and `Graph.h`, neither of
which is in this repo. Read it; do not try to make it green.

| File | |
|---|---|
| `Graph.cpp` | the graph type and `Graph::SIR`, the epidemic model. Ported to `get/src/sir.rs` |
| `main.cpp` | the driver: the evolution loop, and `fitnessBit` — how fitness is *actually* computed |

## Where the Rust deliberately differs

Both differences are open questions, not settled positions — `collab.md` #15 and #17 carry them.

- **`length`.** `Graph::SIR` pushes a trailing zero and `main.cpp` reads the epidemic length as
  `profile.size() - 1`, which counts the final burnout step. Spec §5.2 fixes the other convention,
  and `get/src/sir.rs` follows the sheet, so its `length` is one lower.
- **Short-epidemic re-rolls.** Every fitness draw in `main.cpp` re-rolls an outbreak that burns out
  in under `mepl = 3` steps, up to `rse = 5` attempts (`main.cpp:520-531`, `537-542`). Nothing in
  the spec or the tracker records this. It is a biased resample, so averaging `num_epidemics` is
  not a substitute for it.

## Two things in here not to copy

- `main.cpp:559` divides by `NSE` in the profile-matching branch even when `finalTest` ran
  `FTL = 50` epidemics; the length branch divides by `tests` correctly at `main.cpp:535`. Archived
  final-test profile scores are inflated by `FTL / NSE`.
- `main.cpp:123` leaves `accu` uninitialized before `accu += trial` in the debug block near the top
  of `main`. That block is scratch code, but the value it prints is meaningless.
