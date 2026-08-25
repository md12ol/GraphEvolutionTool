use std::sync::Arc;
#[cfg(test)]
use std::sync::OnceLock;

use rand::Rng;
use rand::distr::Distribution;
use rand::distr::weighted::WeightedIndex;
use serde::Deserialize;

use super::genome::{EdgeEditContext, EdgeEditMutation, Genome};
use crate::graph::Graph;

mod operations;

use operations::GraphOperation;

const OPERATION_COUNT: usize = 9;
const OPCODE_MASK: u64 = 0xF;

/// A whole gene that edits nothing: opcode 8 is `Null`, and the payload above
/// it is unread, so a genome of these expresses to its base graph unchanged.
pub const IDENTITY_GENE: u64 = 8;

/// Relative probabilities for generating each edge-edit operation.
///
/// The default gives all nine operations equal probability. Set via
/// `[genome.operation_weights]` in `config.toml`; any field omitted there falls
/// back to that default, and a weight of `0.0` disables its operation outright.
/// An unrecognized key in `[genome.operation_weights]` is rejected at load
/// time rather than silently ignored.
///
/// Validated and compiled into a sampler by [`EdgeEditOperators`], which the
/// population then shares.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EdgeEditOperationWeights {
    pub toggle: f64,
    pub hop: f64,
    pub add: f64,
    pub delete: f64,
    pub swap: f64,
    pub local_toggle: f64,
    pub local_add: f64,
    pub local_delete: f64,
    pub null: f64,
}

impl EdgeEditOperationWeights {
    /// Check that the weights define a usable probability distribution.
    pub fn validate(&self) -> Result<(), &'static str> {
        let values = self.values();
        if values
            .iter()
            .any(|weight| !weight.is_finite() || *weight < 0.0)
        {
            return Err("operation weights must be finite and non-negative");
        }
        if !values.iter().any(|weight| *weight > 0.0) {
            return Err("at least one operation weight must be positive");
        }
        Ok(())
    }

    fn values(&self) -> [f64; OPERATION_COUNT] {
        [
            self.toggle,
            self.hop,
            self.add,
            self.delete,
            self.swap,
            self.local_toggle,
            self.local_add,
            self.local_delete,
            self.null,
        ]
    }
}

impl Default for EdgeEditOperationWeights {
    fn default() -> Self {
        Self {
            toggle: 1.0,
            hop: 1.0,
            add: 1.0,
            delete: 1.0,
            swap: 1.0,
            local_toggle: 1.0,
            local_add: 1.0,
            local_delete: 1.0,
            null: 1.0,
        }
    }
}

/// A validated operation mix together with its prebuilt sampler.
///
/// Built once per run and shared across the whole population behind an `Arc`:
/// the [`WeightedIndex`] is constructed once rather than on every mutation, and
/// each individual carries a pointer instead of its own copy of the weights.
///
/// `weights` is retained alongside the sampler purely so the type stays
/// readable in `Debug` output and comparable in tests; nothing samples from it.
#[derive(Debug, PartialEq)]
pub struct EdgeEditOperators {
    weights: EdgeEditOperationWeights,
    distribution: WeightedIndex<f64>,
}

impl EdgeEditOperators {
    /// Validate `weights` and compile them into a sampler, so weight errors
    /// surface here — at startup, from config — rather than mid-run.
    pub fn new(weights: EdgeEditOperationWeights) -> Result<Arc<Self>, &'static str> {
        weights.validate()?;
        let distribution =
            WeightedIndex::new(weights.values()).expect("weights are validated immediately above");
        Ok(Arc::new(Self {
            weights,
            distribution,
        }))
    }

    /// The mix that draws every operation with equal probability.
    ///
    /// Test-only: a run's mix always comes from `[genome.operation_weights]`
    /// via [`EdgeEditOperators::new`]. Tests that do not care which operations
    /// come out use this instead of spelling nine equal weights each time.
    ///
    /// Cached, so every caller shares one instance rather than each building
    /// its own — which would reintroduce exactly the per-genome copy this type
    /// exists to eliminate.
    #[cfg(test)]
    fn uniform() -> Arc<Self> {
        // A function-local `static` initialized once, on first call, and
        // shared by every caller thereafter — Rust's version of a lazily-built
        // singleton.
        static UNIFORM: OnceLock<Arc<EdgeEditOperators>> = OnceLock::new();
        Arc::clone(UNIFORM.get_or_init(|| {
            Self::new(EdgeEditOperationWeights::default()).expect("equal weights are valid")
        }))
    }
}

/// Edit-script genome: a list of encoded graph-edit operations.
///
/// Each gene stores its opcode in the low four bits and a random 32-bit payload
/// above it. During expression, that payload is decoded into four mixed-radix
/// vertex parameters.
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeEditGenome {
    pub genes: Vec<u64>,
    operators: Arc<EdgeEditOperators>,
}

impl EdgeEditGenome {
    /// Construct a genome from encoded genes and a shared operation mix.
    ///
    /// Almost every individual is minted at random by
    /// [`EdgeEditGenome::random_with_operators`] instead, so this is the path for
    /// the callers that need a *chosen* gene sequence. There are two, and they
    /// are why this is public rather than test-only.
    ///
    /// `dispatch` builds the identity individual a seeded run starts with —
    /// [`IDENTITY_GENE`] repeated — which is a chosen sequence by definition. And
    /// a caller driving an evolver directly needs a known starting population,
    /// whether that is the genes read off a previous run's best individual, a
    /// recorded edit script, or a deterministic fixture; without this, `genes`
    /// would be readable with no supported way to feed them back.
    pub fn new_with_operators(genes: Vec<u64>, operators: Arc<EdgeEditOperators>) -> Self {
        Self { genes, operators }
    }

    /// Generate a random genome drawing opcodes from a shared operation mix.
    ///
    /// Infallible: the mix was validated when [`EdgeEditOperators::new`] built
    /// it, so a whole population can be minted without per-individual error
    /// handling.
    pub fn random_with_operators<R: Rng + ?Sized>(
        length: usize,
        operators: Arc<EdgeEditOperators>,
        rng: &mut R,
    ) -> Self {
        let genes = (0..length)
            .map(|_| Self::generate_gene(rng, &operators.distribution))
            .collect();
        Self { genes, operators }
    }

    fn generate_gene<R: Rng + ?Sized>(rng: &mut R, distribution: &WeightedIndex<f64>) -> u64 {
        let opcode = distribution.sample(rng) as u64;
        let payload = rng.random::<u32>() as u64;
        (payload << 4) | opcode
    }

    fn decode_vertices(gene: u64, num_nodes: usize) -> [usize; 4] {
        let mut vertices = [0; 4];
        let mut payload = gene >> 4;
        let radix = num_nodes as u64;

        // Extract four vertex indices from `payload` by treating it as a
        // number in base `num_nodes`: each iteration peels off one "digit"
        // (payload % radix) then shifts the remaining digits down
        // (payload /= radix).
        for vertex in &mut vertices {
            *vertex = (payload % radix) as usize;
            payload /= radix;
        }

        vertices
    }
}

impl Genome for EdgeEditGenome {
    type Context = EdgeEditContext;

    fn express(&self, context: &Self::Context) -> Graph {
        let mut graph = context.base_graph.clone();
        if graph.num_nodes == 0 {
            return graph;
        }

        for &gene in &self.genes {
            // Unrecognized opcode (only opcodes 0-8 are defined; the 4-bit
            // field can hold up to 15): skip this gene and move to the next
            // one.
            let Some(operation) = GraphOperation::from_opcode((gene & OPCODE_MASK) as u8) else {
                continue;
            };
            let [v1, v2, v3, v4] = Self::decode_vertices(gene, graph.num_nodes);
            operation.apply(&mut graph, v1, v2, v3, v4);
        }

        graph
    }

    /// Two-point crossover over genes: draw two distinct cut points in
    /// `0..=shared_length` and swap the half-open segment `[start, end)`,
    /// leaving genes outside that window untouched on both sides.
    ///
    /// Declines to cross at all below two shared genes. With one there is
    /// only a single possible pair of cut points, so the segment is forced to
    /// gene 0 and nothing is actually being chosen; a gene carries nothing
    /// but its own value, which makes that a one-gene exchange rather than a
    /// crossover. `SdaGenome::crossover` does cross at that length, because
    /// its state 0 takes `init_char` with it and so has something further to
    /// exchange.
    ///
    /// The obligations every crossover carries — both children kept, both
    /// parents left valid for the representation, every draw from `rng` — are
    /// stated once on [`Genome::crossover`]. A gene is a self-contained
    /// integer here, so leaving both sides valid costs nothing: swapping any
    /// two decodable genes yields two decodable genomes.
    fn crossover<R: Rng + ?Sized>(&mut self, other: &mut Self, rng: &mut R) {
        let shared_length = self.genes.len().min(other.genes.len());
        if shared_length < 2 {
            return;
        }

        let (start, end) = super::two_distinct_cut_points(shared_length, rng);
        for index in start..end {
            std::mem::swap(&mut self.genes[index], &mut other.genes[index]);
        }
    }

    /// Reroll one gene, its opcode drawn from the operation mix.
    ///
    /// Exactly one, per the [`Genome::mutate`] contract, which has the rest of
    /// what a mutation owes: the engine's two dice rolls, why magnitudes are
    /// not equalized across representations, and that every draw comes from
    /// `rng`. This previously rolled `1..=4` against a hardcoded
    /// `MAX_MUTATIONS`, which made the engine's `max_mutations` mean nothing
    /// here; the count is the engine's to decide.
    ///
    /// *One mutation* is one gene, whatever that gene decodes to — replacing a
    /// `null` with a nine-vertex `swap` counts the same as replacing one
    /// `toggle` with another.
    fn mutate<R: Rng + ?Sized>(&mut self, context: &Self::Context, rng: &mut R) {
        match context.mutation {
            EdgeEditMutation::RerollGene => {
                // Nothing to reroll, so nothing happens -- the empty-genome
                // no-op the trait contract requires, rather than a panic.
                if self.genes.is_empty() {
                    return;
                }

                let gene_index = rng.random_range(0..self.genes.len());
                self.genes[gene_index] = Self::generate_gene(rng, &self.operators.distribution);
            } // ADD A MUTATION STEP 2 (for EdgeEdit) — the arm performing your variant. Keep
              // the exactly-one-mutation contract this method's own doc
              // states. The step after this one is `config::EdgeEditMutationConfig`
              // — search `ADD A MUTATION STEP 3 (for EdgeEdit)` for it.
        }
    }

    fn print(&self) -> String {
        format!("EdgeEditGenome({} ops): {:?}", self.genes.len(), self.genes)
    }
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use super::*;

    /// `mutate` ignores its context — edge-edit keeps its operation mix on the
    /// genome — so the mutation tests just need something of the right type.
    fn mutation_context() -> EdgeEditContext {
        EdgeEditContext {
            base_graph: Graph::new(1, 1),
            mutation: Default::default(),
        }
    }

    fn encode_gene(opcode: u8, vertices: [usize; 4], num_nodes: usize) -> u64 {
        let radix = num_nodes as u64;
        let mut payload = 0;
        for vertex in vertices.into_iter().rev() {
            payload = payload * radix + vertex as u64;
        }
        (payload << 4) | opcode as u64
    }

    fn decode_vertices_like_graph_refiner(gene: u64, num_nodes: usize) -> [usize; 4] {
        let payload = gene >> 4;
        let radix = num_nodes as u64;
        [
            (payload % radix) as usize,
            ((payload / radix) % radix) as usize,
            ((payload / radix.pow(2)) % radix) as usize,
            ((payload / radix.pow(3)) % radix) as usize,
        ]
    }

    fn weights_for_add() -> EdgeEditOperationWeights {
        EdgeEditOperationWeights {
            toggle: 0.0,
            hop: 0.0,
            add: 1.0,
            delete: 0.0,
            swap: 0.0,
            local_toggle: 0.0,
            local_add: 0.0,
            local_delete: 0.0,
            null: 0.0,
        }
    }

    fn weights_for_delete() -> EdgeEditOperationWeights {
        EdgeEditOperationWeights {
            add: 0.0,
            delete: 1.0,
            ..weights_for_add()
        }
    }

    #[test]
    fn operation_weights_are_validated() {
        assert!(EdgeEditOperationWeights::default().validate().is_ok());

        let all_zero = EdgeEditOperationWeights {
            add: 0.0,
            ..weights_for_add()
        };
        assert_eq!(
            all_zero.validate(),
            Err("at least one operation weight must be positive")
        );

        let negative = EdgeEditOperationWeights {
            toggle: -1.0,
            ..EdgeEditOperationWeights::default()
        };
        assert_eq!(
            negative.validate(),
            Err("operation weights must be finite and non-negative")
        );

        let not_finite = EdgeEditOperationWeights {
            toggle: f64::NAN,
            ..EdgeEditOperationWeights::default()
        };
        assert_eq!(
            not_finite.validate(),
            Err("operation weights must be finite and non-negative")
        );
    }

    #[test]
    fn weighted_random_generation_can_force_an_opcode() {
        let mut rng = StdRng::seed_from_u64(7);
        let operators = EdgeEditOperators::new(weights_for_add()).unwrap();
        let genome = EdgeEditGenome::random_with_operators(32, operators, &mut rng);

        assert!(genome.genes.iter().all(|gene| gene & OPCODE_MASK == 2));
        assert_eq!(genome.operators.weights, weights_for_add());
    }

    #[test]
    fn random_gene_packing_matches_graph_refiner_exactly() {
        let operators = EdgeEditOperators::uniform();
        let distribution = &operators.distribution;
        let mut actual_rng = StdRng::seed_from_u64(29);
        let mut reference_rng = StdRng::seed_from_u64(29);

        for _ in 0..64 {
            let actual = EdgeEditGenome::generate_gene(&mut actual_rng, distribution);
            let opcode = distribution.sample(&mut reference_rng) as u64;
            let payload = reference_rng.random::<u32>() as u64;
            let expected = (payload << 4) | opcode;

            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn express_decodes_and_applies_genes_in_order_without_changing_base() {
        let mut base_graph = Graph::new(4, 5);
        base_graph.set_edge(0, 1, 2);
        let original = base_graph.clone();
        let context = EdgeEditContext {
            base_graph,
            mutation: Default::default(),
        };
        let genome = EdgeEditGenome::new_with_operators(
            vec![
                encode_gene(3, [0, 1, 0, 0], 4),
                encode_gene(2, [1, 2, 0, 0], 4),
                encode_gene(0, [0, 2, 0, 0], 4),
                encode_gene(8, [0, 0, 0, 0], 4),
            ],
            EdgeEditOperators::uniform(),
        );

        let expressed = genome.express(&context);

        assert_eq!(expressed.weight(0, 1), 1);
        assert_eq!(expressed.weight(1, 2), 1);
        assert_eq!(expressed.weight(0, 2), 1);
        assert_eq!(context.base_graph, original);
    }

    #[test]
    fn express_preserves_an_unweighted_base_graph_cap() {
        let base_graph = Graph::new(3, 1);
        let context = EdgeEditContext {
            base_graph,
            mutation: Default::default(),
        };
        let genome = EdgeEditGenome::new_with_operators(
            vec![
                encode_gene(2, [0, 1, 0, 0], 3),
                encode_gene(2, [0, 1, 0, 0], 3),
            ],
            EdgeEditOperators::uniform(),
        );

        let expressed = genome.express(&context);

        assert_eq!(expressed.max_edge_multiplicity, 1);
        assert_eq!(expressed.get_edge_list(), vec![(0, 1, 1)]);
        assert!(context.base_graph.get_edge_list().is_empty());
    }

    #[test]
    fn mixed_radix_decode_uses_all_four_vertices() {
        let gene = encode_gene(4, [4, 3, 2, 1], 5);
        assert_eq!(EdgeEditGenome::decode_vertices(gene, 5), [4, 3, 2, 1]);
    }

    #[test]
    fn mixed_radix_decode_matches_graph_refiner_exactly() {
        for num_nodes in [2, 3, 5, 17, 257] {
            for gene in [0, 8, 0x1234_5678, 0x000f_ffff_ffff_fff4] {
                assert_eq!(
                    EdgeEditGenome::decode_vertices(gene, num_nodes),
                    decode_vertices_like_graph_refiner(gene, num_nodes)
                );
            }
        }
    }

    #[test]
    fn empty_one_node_and_invalid_opcode_expressions_are_noops() {
        let empty_context = EdgeEditContext {
            base_graph: Graph::new(0, 5),
            mutation: Default::default(),
        };
        let invalid = EdgeEditGenome::new_with_operators(vec![15], EdgeEditOperators::uniform());
        assert_eq!(invalid.express(&empty_context), Graph::new(0, 5));

        let one_node_context = EdgeEditContext {
            base_graph: Graph::new(1, 5),
            mutation: Default::default(),
        };
        let add_self = EdgeEditGenome::new_with_operators(
            vec![encode_gene(2, [0, 0, 0, 0], 1)],
            EdgeEditOperators::uniform(),
        );
        assert_eq!(add_self.express(&one_node_context), Graph::new(1, 5));

        let mut base_graph = Graph::new(2, 5);
        base_graph.add_edge(0, 1);
        let context = EdgeEditContext {
            base_graph: base_graph.clone(),
            mutation: Default::default(),
        };
        let invalid = EdgeEditGenome::new_with_operators(
            vec![encode_gene(15, [0, 1, 0, 0], 2)],
            EdgeEditOperators::uniform(),
        );
        assert_eq!(invalid.express(&context), base_graph);
    }

    #[test]
    fn an_unrecognized_opcode_skips_its_own_gene_and_leaves_the_rest_to_apply() {
        // The opcode field is 4 bits, spanning 0..=15, and only 0..=8 name an
        // operation — so a genome arriving from outside can carry one. The
        // invalid-opcode case above uses it as the only gene, where skipping it
        // and abandoning the whole genome look exactly alike. Here it sits
        // between two genes that must both still apply.
        let context = EdgeEditContext {
            base_graph: Graph::new(4, 5),
            mutation: Default::default(),
        };
        let genome = EdgeEditGenome::new_with_operators(
            vec![
                encode_gene(2, [0, 1, 0, 0], 4),
                encode_gene(11, [0, 0, 0, 0], 4),
                encode_gene(2, [0, 2, 0, 0], 4),
            ],
            EdgeEditOperators::uniform(),
        );

        let graph = genome.express(&context);

        assert_eq!(
            graph.get_edge_list(),
            vec![(0, 1, 1), (0, 2, 1)],
            "the gene after the unrecognized one must still have applied"
        );
    }

    #[test]
    fn crossover_swaps_only_a_nonempty_shared_segment() {
        let mut left =
            EdgeEditGenome::new_with_operators(vec![0, 1, 2, 3, 4], EdgeEditOperators::uniform());
        let mut right = EdgeEditGenome::new_with_operators(
            vec![10, 11, 12],
            EdgeEditOperators::new(weights_for_add()).unwrap(),
        );
        let left_tail = left.genes[3..].to_vec();
        let left_weights = left.operators.weights;
        let right_weights = right.operators.weights;
        let mut rng = StdRng::seed_from_u64(11);

        left.crossover(&mut right, &mut rng);

        assert_eq!(left.genes.len(), 5);
        assert_eq!(right.genes.len(), 3);
        assert_eq!(&left.genes[3..], left_tail.as_slice());
        assert!(
            (0..3).any(|index| left.genes[index] >= 10),
            "a nonempty segment should be exchanged"
        );
        for index in 0..3 {
            assert!(
                (left.genes[index] == index as u64 && right.genes[index] == index as u64 + 10)
                    || (left.genes[index] == index as u64 + 10
                        && right.genes[index] == index as u64)
            );
        }
        assert_eq!(left.operators.weights, left_weights);
        assert_eq!(right.operators.weights, right_weights);
    }

    #[test]
    fn mutation_replaces_exactly_one_gene_using_the_shared_mix() {
        // Swept over seeds rather than run once: a single seed cannot tell
        // "always one" from "one this time". The genes are sentinel value 8
        // (opcode 8) and `weights_for_delete` forces every generated gene to
        // opcode 3, so a reroll can never coincidentally reproduce the sentinel
        // and read as unchanged.
        for seed in 0..64 {
            let mut genome = EdgeEditGenome::new_with_operators(
                vec![8; 10],
                EdgeEditOperators::new(weights_for_delete()).unwrap(),
            );
            let mut rng = StdRng::seed_from_u64(seed);

            genome.mutate(&mutation_context(), &mut rng);

            let changed: Vec<_> = genome.genes.iter().filter(|gene| **gene != 8).collect();
            assert_eq!(
                changed.len(),
                1,
                "one call to mutate must change exactly one gene, seed {seed}",
            );
            assert!(changed.iter().all(|gene| **gene & OPCODE_MASK == 3));
        }
    }

    #[test]
    fn crossover_declines_at_a_single_shared_gene() {
        // With one shared gene there is only one possible pair of cut points,
        // so the segment is forced rather than chosen and the exchange carries
        // nothing a plain gene swap would not. SdaGenome deliberately does
        // cross at this length, because its state 0 takes init_char with it.
        let mut left = EdgeEditGenome::new_with_operators(vec![1], EdgeEditOperators::uniform());
        let mut right =
            EdgeEditGenome::new_with_operators(vec![91, 92, 93], EdgeEditOperators::uniform());
        let mut rng = StdRng::seed_from_u64(5);

        // Checked after every call, not just at the end: declining is a claim
        // about each draw, and an even number of swaps would land back on the
        // starting genes and read as though nothing had happened.
        for attempt in 0..50 {
            left.crossover(&mut right, &mut rng);

            assert_eq!(left.genes, vec![1], "attempt {attempt}");
            assert_eq!(right.genes, vec![91, 92, 93], "attempt {attempt}");
        }
    }

    #[test]
    fn crossover_of_empty_genomes_is_a_noop() {
        // `shared_length` is 0, below the two distinct cut points a swap needs,
        // so this returns before drawing anything rather than panicking on an
        // empty range.
        let mut left = EdgeEditGenome::new_with_operators(Vec::new(), EdgeEditOperators::uniform());
        let mut right =
            EdgeEditGenome::new_with_operators(Vec::new(), EdgeEditOperators::uniform());
        let mut rng = StdRng::seed_from_u64(1);

        left.crossover(&mut right, &mut rng);

        assert!(left.genes.is_empty());
        assert!(right.genes.is_empty());
    }

    #[test]
    fn mutation_of_an_empty_genome_is_a_noop() {
        let mut genome =
            EdgeEditGenome::new_with_operators(Vec::new(), EdgeEditOperators::uniform());
        let mut rng = StdRng::seed_from_u64(23);

        genome.mutate(&mutation_context(), &mut rng);

        assert!(genome.genes.is_empty());
    }

    #[test]
    fn operators_reject_weights_that_cannot_form_a_distribution() {
        let all_zero = EdgeEditOperationWeights {
            add: 0.0,
            ..weights_for_add()
        };
        assert_eq!(
            EdgeEditOperators::new(all_zero).unwrap_err(),
            "at least one operation weight must be positive"
        );

        let not_finite = EdgeEditOperationWeights {
            toggle: f64::NAN,
            ..EdgeEditOperationWeights::default()
        };
        assert_eq!(
            EdgeEditOperators::new(not_finite).unwrap_err(),
            "operation weights must be finite and non-negative"
        );
    }

    #[test]
    fn the_uniform_mix_is_shared_rather_than_rebuilt_per_call() {
        // `EdgeEditGenome::new`/`random` route through `uniform()`, so a
        // per-call rebuild would hand every individual its own WeightedIndex.
        assert!(Arc::ptr_eq(
            &EdgeEditOperators::uniform(),
            &EdgeEditOperators::uniform()
        ));

        let mut rng = StdRng::seed_from_u64(31);
        let first =
            EdgeEditGenome::random_with_operators(4, EdgeEditOperators::uniform(), &mut rng);
        let second =
            EdgeEditGenome::random_with_operators(4, EdgeEditOperators::uniform(), &mut rng);
        assert!(Arc::ptr_eq(&first.operators, &second.operators));
    }

    #[test]
    fn a_zero_weighted_operation_is_never_generated_or_mutated_in() {
        // `weights_for_delete` leaves only opcode 3 with a positive weight.
        let operators = EdgeEditOperators::new(weights_for_delete()).unwrap();
        let mut rng = StdRng::seed_from_u64(37);

        let mut genome = EdgeEditGenome::random_with_operators(64, operators, &mut rng);
        assert!(genome.genes.iter().all(|gene| gene & OPCODE_MASK == 3));

        for _ in 0..200 {
            genome.mutate(&mutation_context(), &mut rng);
        }
        assert!(genome.genes.iter().all(|gene| gene & OPCODE_MASK == 3));
    }

    #[test]
    fn the_genome_stays_shareable_across_evaluation_threads() {
        // `Genome: Clone + Send + Sync` is what lets rayon score a population;
        // the `Arc<EdgeEditOperators>` field must not break it.
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<EdgeEditGenome>();
    }

    #[test]
    fn print_includes_the_complete_genome() {
        let genome =
            EdgeEditGenome::new_with_operators(vec![1, 2, 3], EdgeEditOperators::uniform());
        assert_eq!(genome.print(), "EdgeEditGenome(3 ops): [1, 2, 3]");
    }
}
