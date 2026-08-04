# Superseded task wording — #10

Original wording of tasks now done. Reference only, never actionable.
Created by `/save`; archived by `/done`.

---

## Rewrite the edge-edit mutation test — superseded 2026-08-03

Original wording, kept because it names the old test and the specific assertion that was deleted:

> **Rewrite `mutation_replaces_at_most_four_genes_using_the_shared_mix`** —
> `get/src/genomes/edge_edit.rs:466`. It asserts `(1..=4).contains(&changed.len())`, which is the
> behaviour being deleted. Rewrite as exactly-one, keeping the existing check that the new gene
> comes from the shared operation mix. Rename to match.
> **Verify by:** the test fails if `mutate` rerolls two genes — flip it to a 2-gene loop and watch
> it go red before fixing it back.

Landed as `mutation_replaces_exactly_one_gene_using_the_shared_mix`, swept over 64 seeds rather than
run at one. The "flip it to a 2-gene loop" mutation check in the verify-by was **not** performed —
the seed sweep was judged to cover it, since a 2-gene mutate fails `assert_eq!(changed.len(), 1)` on
the first iteration. Recorded here rather than claimed as done.

*Superseded 2026-08-03 18:50 — James, during #10 implementation.*
</content>
