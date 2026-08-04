# Legacy C++ — reference, not a build target

The original implementation this project is a port of. It is **tracked deliberately**: the Rust
cites it ("ported from `Graph::SIR`"), decisions get argued against it, and while it was ignored at
the repo root those citations pointed at a file only one of us had.

**Nothing here is built, tested, or run by this repo.** It is not wired into Cargo, and `main.cpp`
cannot compile as it stands: it includes `../BitSprayer/Bitsprayer.h`, `setu.h`, `stat.h` and
`filesystem.hpp`, none of which are here. `Graph.cpp` needs Bitsprayer too. Read it; do not try to
make it green.

| File | | Ported to |
|---|---|---|
| `Graph.cpp` / `Graph.h` | the graph type and `Graph::SIR`, the epidemic model | `get/src/sir.rs`, `get/src/graph.rs` |
| `SDA.cpp` / `SDA.h` | the self-driving automaton genome — the only file here that compiles standalone | `get/src/genomes/sda.rs` |
| `main.cpp` | the driver: the evolution loop, and `fitnessBit` — how fitness is *actually* computed | `get/src/evolver/`, and #17 |

Note `main.cpp` evolves **Bitsprayers**, not `SDA` — the SDA class is carried here because the Rust
genome is a port of it, not because this driver uses it.

## These files are from two different generations and do not fit together

`Graph.cpp`/`Graph.h` are newer than `main.cpp`, and the two will not compile against each other.
Worth knowing before reading them as one program:

- **`SIR` changed shape.** It is now `int SIR(int p0, double alpha, vector<int> &epiProfile,
  int &totInf)` — returns the length, fills the profile and the total infected. `main.cpp:523` still
  calls the old `vector<int> SIR(double alpha, int p0)`.
- **The first two arguments swapped.** Old was `(alpha, p0)`, new is `(p0, alpha)`. Both are
  positional and one is a `double`, so a port done by eye will compile and be silently wrong.
- **`hammy_distance` is gone** from the new `Graph`, but `main.cpp:561` still calls it for
  `fitFun == 2`, the network-matching objective.
- **`SIRwithVariants` is new** — a multi-variant model with immunity, variant DNA and severity
  ordering. It is not in `official_spec_sheet.md` and no issue covers it. Noted here so it is not
  mistaken for something the Rust is behind on.

The Rust port in `get/src/sir.rs` predates the newer file but is unaffected: the state machine,
the exposure accumulation and the `1 - (1 - alpha)^n` draw are identical across both generations.
Only the return shape and who owns the profile buffer changed.

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
