# pyo3 + maturin — what we know, and where it came from

Reference notes for the Python boundary. **Not a working doc**: nothing here is a task, a decision
or a churn list, and it is not union-merged. It records how pyo3 behaves for this crate, so the next
person hitting a linker error or an empty `pyproject.toml` question does not re-derive it.

Two sources, and they are worth keeping distinct:

1. **Measured in this repo** — stated with the date and the command. Trust these.
2. **From `graph_refiner`**, James's separate Rust+pyo3+maturin project, supplied 2026-08-07 as a
   known-working reference. Trust these as *a working configuration*, not as this project's answer —
   its needs differ in one important way, noted below.

---

## 1. `extension-module` and `cargo test` are mutually exclusive

**Measured 2026-08-07, during #19.** This is the one that costs an afternoon.

`pyo3`'s `extension-module` feature tells the linker to leave the Python C API symbols
**unresolved**, for the interpreter to supply when it dlopens the built module. That is exactly
right for the shipped artifact and fatal for `cargo test`, which builds an ordinary executable with
no interpreter behind it. The failure is a wall of `undefined symbol: PyObject_GetAttr`,
`PyLong_AsLong`, `PyUnicode_FromStringAndSize`, … at **link** time, and it takes down the *whole*
suite — including the tests that never mention Python.

**The fix, and it is the maturin-native one:** keep the feature out of `[dependencies]` and let the
thing that builds the module supply it.

```toml
[dependencies]
pyo3 = { version = "0.27.2", features = ["abi3-py38"] }   # no extension-module

[dev-dependencies]
pyo3 = { version = "0.27.2", features = ["auto-initialize"] }
```

- **maturin** supplies it via `pyproject.toml`: `[tool.maturin] features = ["pyo3/extension-module"]`
- **cargo**, by hand: `cargo build -p get --features pyo3/extension-module`
- **`auto-initialize`** starts an embedded interpreter on first `Python::attach`, which is what lets
  a `#[test]` call a Python callable at all. Dev-dependency only — never in the shipped module.

`get/src/fitness.rs` carries `the_test_harness_can_call_a_live_python_interpreter`, a smoke test
whose only job is to fail loudly if someone moves the feature back.

### The runtime half: `LD_LIBRARY_PATH`

Linking is not the end of it. With `auto-initialize`, the test binary needs `libpython3.*.so` at
**run** time, and a pyenv-managed Python is not on the default loader path. Symptom, measured the
same day:

```
error while loading shared libraries: libpython3.11.so.1.0: cannot open shared object file
```

exit code **127**, before any test runs. Derive the path rather than hardcoding it:

```bash
export LD_LIBRARY_PATH="$(python3 -c 'import sysconfig; print(sysconfig.get_config_var("LIBDIR"))'):$LD_LIBRARY_PATH"
cargo test -p get
```

**Unverified on Windows.** Michael's machine links Python differently and this note is Linux/pyenv
only — see `traps.md` for the same warning in the place a session actually reads.

## 2. Calling Python from a rayon closure does not merely slow down — it deadlocks

**Measured 2026-08-07** by deleting `PyFitness`'s `evaluate_population` override and running
`a_python_objective_is_called_once_per_batch_not_once_per_graph`.

`Fitness::evaluate_population`'s default fans out over `par_iter()`. Without the override, each rayon
worker calls `Python::attach` while the calling thread already holds the GIL and is blocked waiting
for rayon to finish. The suite **hung** until killed at two minutes. It did not fail — which is
worse, because a hang carries no message pointing at the cause.

Spec §8 states the rule as "never call Python from inside a rayon closure" and argues it on
performance. The measured consequence is stronger than the stated one, and it is why
`impl Fitness for Box<dyn Fitness>` must forward `evaluate_population` explicitly: an unforwarded
box silently inherits that default.

## 3. GET's `pyproject.toml` — added 2026-08-07, and how it differs from `graph_refiner`'s

**Resolved during #19.** GET previously had none, so it could not be built or installed as a Python
package at all. It now has one at the repo root, and `maturin build` produces an importable wheel.

Two differences from `graph_refiner`'s, both forced by this repo's shape:

- **`manifest-path = "get/Cargo.toml"`.** GET is a cargo workspace whose crate is a member, so the
  manifest is not beside `pyproject.toml`. `graph_refiner` is a single crate at the root and needs
  no such line.
- **`features = ["pyo3/extension-module"]`.** `graph_refiner` has no `[tool.maturin]` section at all,
  because its `Cargo.toml` keeps `extension-module` in `[dependencies]`. GET cannot — see §1 — so the
  feature has to be supplied on the build path instead.

**What the feature does and does not buy, measured 2026-08-07.** Dropping the `features` line and
rebuilding produced a wheel that was, on this Linux/pyenv setup, indistinguishable: **75 undefined
`Py*` symbols in both**, no `libpython` in `ldd` for either, and `import get` succeeded either way.
An earlier version of the comment in `pyproject.toml` claimed the featureless wheel would fail to
import; that was wrong, and testing it is what caught it. The line stays because macOS and Windows
linkers reject undefined symbols by default and because it is the documented configuration — but
**a passing `import` on Linux is not evidence the line is unnecessary**, and nobody should conclude
that from a green run here.

Verified end to end on 2026-08-07 in a throwaway venv: `import get`, construct
`GraphEvolver(config)` against a `type = "python"` config, register a callable, and see both
rejection paths arrive as Python `ValueError`s with their messages intact.

## 4. Patterns worth copying from `graph_refiner`

Supplied 2026-08-07 as a known-working reference. What transfers:

- **Setters before the run.** `set_operation_weights`, `set_probabilities`, `set_targets` all mutate
  `#[pyclass]` state, then `evolve(generations, seed)` uses it. `set_fitness_function` is the same
  shape, which is why #19 follows it rather than inventing a registration mechanism.
- **A thin `#[pyclass]` over an engine struct.** `GraphRefiner` holds a `GeneticOptimizer` and
  forwards; the GA knows nothing about Python. GET's `GraphEvolver`/`Config` split matches.
- **`PyResult<()>` plus `?` for I/O.** `save_logs`/`save_results` let `std::io::Error` convert to a
  Python `OSError` for free — worth remembering for GET's own `save_logs`, still a `todo!()`.
- **Ordinary Rust types cross the boundary.** `Vec<Vec<f64>>`, `(f64, f64, f64)`,
  `Vec<(usize, usize)>` convert without hand-built `PyList`s. This is why `PyFitness` builds its
  batch as a plain `Vec<(usize, Vec<(usize, usize, u32)>)>` and hands it over whole.

What does **not** transfer:

- **Its tests never call Python**, so it has never hit §1. Its `Cargo.toml` keeps `extension-module`
  in `[dependencies]` and is fine — because `operations.rs`'s tests are pure Rust and `lib.rs`'s is
  `add(2, 2)`. GET's `PyFitness` tests must call a real callable to be worth anything, which is what
  forced the change.
- **No `[lib] crate-type`** — it takes maturin's default. GET sets `["cdylib", "rlib"]` so
  `cargo test` can link the crate at all.
- **Declarative module syntax.** `#[pymodule] mod graph_refiner { #[pymodule_export] use ...; }`
  against GET's older function-style `#[pymodule] fn get(m: &Bound<'_, PyModule>)`. Both work in
  0.27; not worth changing on its own.

*Started 2026-08-07 — James, during #19. Add to it when the boundary teaches you something.*
