# Documentation audit and redesign copy

This directory is an independent review copy of `documentation/` on branch
`mdube_doc-snippet-sweep`. The full audit used commit
`87eaaba0e8a9e4668c960cf6670d70d8a9fb9d77` on 2026-08-28; the redesign was then synchronized on
2026-08-29 with the documentation and behavior changes through
`11656850a8893c1e1d26c1e6f5831905534ad9b5`.

The copy documents Python `0.9.0`: TOML `base_graph`, `RunResult.save_config()`, and the revised
`save_results()` output contract are ordinary release behavior. The Rust crate remains source-only.

- `AUDIT.md` contains the evidence-backed content, usability, navigation, design, visual, and
  accessibility audit.
- `IMPLEMENTATION_PLAN.md` prioritizes changes to consider for the real site.
- `index.html` and `guide/` contain a working redesign that demonstrates the proposed structure.
- `get-examples.zip` and `guide/example-bundle.html` are the branch's downloadable and readable
  example bundle, integrated into that structure.

The original `documentation/` directory was not edited. This review copy is intentionally
uncommitted.

## Preview

Open `index.html` directly, or from the repository root run:

```bash
python3 -m http.server 8766 --directory documentation-audit
```

Then open <http://localhost:8766/>.

## Validate

From the repository root:

```bash
python3 documentation-audit/check_refs.py
```

The checker resolves source references against the parent GET checkout and validates this copy's
own navigation, pages, links, anchors, extension tables, and displayed Rust signatures.

## Generated assets

`assets/convergence.svg` and `assets/evolved-network.svg` are written by
`tools/make_doc_figures.py` from real runs. `evolved-network.svg` is embedded by
`index.html`; `convergence.svg` is currently referenced by no page, but it is a
generated artifact rather than an orphan — delete it and the next run of the
generator puts it back.
