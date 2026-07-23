use rand::Rng;
use rand::distr::Distribution;
use rand::distr::weighted::WeightedIndex;

use super::genome::{EdgeEditContext, Genome};
use crate::graph::Graph;

mod operations;

use operations::GraphOperation;

const OPERATION_COUNT: usize = 9;
const MAX_MUTATIONS: usize = 4;
const OPCODE_MASK: u64 = 0xF;

/// Relative probabilities for generating each edge-edit operation.
///
/// The default gives all nine operations equal probability. These values are
/// kept with the genome because the shared [`Genome::mutate`] interface only
/// supplies a random-number generator.
#[derive(Clone, Copy, Debug, PartialEq)]
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

    fn distribution(&self) -> WeightedIndex<f64> {
        WeightedIndex::new(self.values()).expect("edge-edit weights are validated at construction")
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

/// Edit-script genome: a list of encoded graph-edit operations.
///
/// Each gene stores its opcode in the low four bits and a random 32-bit payload
/// above it. During expression, that payload is decoded into four mixed-radix
/// vertex parameters.
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeEditGenome {
    pub genes: Vec<u64>,
    operation_weights: EdgeEditOperationWeights,
}

impl EdgeEditGenome {
    /// Construct a genome from encoded genes using equal operation weights.
    pub fn new(genes: Vec<u64>) -> Self {
        Self {
            genes,
            operation_weights: EdgeEditOperationWeights::default(),
        }
    }

    /// Construct a genome from encoded genes and validated operation weights.
    pub fn new_with_operation_weights(
        genes: Vec<u64>,
        operation_weights: EdgeEditOperationWeights,
    ) -> Result<Self, &'static str> {
        operation_weights.validate()?;
        Ok(Self {
            genes,
            operation_weights,
        })
    }

    /// Generate a random genome using equal operation weights.
    pub fn random<R: Rng + ?Sized>(length: usize, rng: &mut R) -> Self {
        Self::random_with_operation_weights(length, EdgeEditOperationWeights::default(), rng)
            .expect("equal edge-edit weights are valid")
    }

    /// Generate a random genome using validated operation weights.
    pub fn random_with_operation_weights<R: Rng + ?Sized>(
        length: usize,
        operation_weights: EdgeEditOperationWeights,
        rng: &mut R,
    ) -> Result<Self, &'static str> {
        operation_weights.validate()?;
        let distribution = operation_weights.distribution();
        let genes = (0..length)
            .map(|_| Self::generate_gene(rng, &distribution))
            .collect();
        Ok(Self {
            genes,
            operation_weights,
        })
    }

    /// Return the operation weights used by random generation and mutation.
    pub fn operation_weights(&self) -> EdgeEditOperationWeights {
        self.operation_weights
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
            let Some(operation) = GraphOperation::from_opcode((gene & OPCODE_MASK) as u8) else {
                continue;
            };
            let [v1, v2, v3, v4] = Self::decode_vertices(gene, graph.num_nodes);
            operation.apply(&mut graph, v1, v2, v3, v4);
        }

        graph
    }

    fn crossover<R: Rng + ?Sized>(&mut self, other: &mut Self, rng: &mut R) {
        let shared_length = self.genes.len().min(other.genes.len());
        if shared_length < 2 {
            return;
        }

        let start = rng.random_range(0..shared_length);
        let end = rng.random_range((start + 1)..=shared_length);
        for index in start..end {
            std::mem::swap(&mut self.genes[index], &mut other.genes[index]);
        }
    }

    fn mutate<R: Rng + ?Sized>(&mut self, rng: &mut R) {
        if self.genes.is_empty() {
            return;
        }

        let max_changes = self.genes.len().min(MAX_MUTATIONS);
        let change_count = rng.random_range(1..=max_changes);
        let distribution = self.operation_weights.distribution();

        // Partially shuffle the indices so every selected gene is unique.
        let mut indices: Vec<usize> = (0..self.genes.len()).collect();
        for selection in 0..change_count {
            let swap_with = rng.random_range(selection..indices.len());
            indices.swap(selection, swap_with);
            let gene_index = indices[selection];
            self.genes[gene_index] = Self::generate_gene(rng, &distribution);
        }
    }

    fn copy(&self) -> Self {
        self.clone()
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
        let genome =
            EdgeEditGenome::random_with_operation_weights(32, weights_for_add(), &mut rng).unwrap();

        assert!(genome.genes.iter().all(|gene| gene & OPCODE_MASK == 2));
        assert_eq!(genome.operation_weights(), weights_for_add());
    }

    #[test]
    fn random_gene_packing_matches_graph_refiner_exactly() {
        let distribution = EdgeEditOperationWeights::default().distribution();
        let mut actual_rng = StdRng::seed_from_u64(29);
        let mut reference_rng = StdRng::seed_from_u64(29);

        for _ in 0..64 {
            let actual = EdgeEditGenome::generate_gene(&mut actual_rng, &distribution);
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
        let context = EdgeEditContext { base_graph };
        let genome = EdgeEditGenome::new(vec![
            encode_gene(3, [0, 1, 0, 0], 4),
            encode_gene(2, [1, 2, 0, 0], 4),
            encode_gene(0, [0, 2, 0, 0], 4),
            encode_gene(8, [0, 0, 0, 0], 4),
        ]);

        let expressed = genome.express(&context);

        assert_eq!(expressed.weight(0, 1), 1);
        assert_eq!(expressed.weight(1, 2), 1);
        assert_eq!(expressed.weight(0, 2), 1);
        assert_eq!(context.base_graph, original);
    }

    #[test]
    fn express_preserves_an_unweighted_base_graph_cap() {
        let base_graph = Graph::new(3, 1);
        let context = EdgeEditContext { base_graph };
        let genome = EdgeEditGenome::new(vec![
            encode_gene(2, [0, 1, 0, 0], 3),
            encode_gene(2, [0, 1, 0, 0], 3),
        ]);

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
        };
        let invalid = EdgeEditGenome::new(vec![15]);
        assert_eq!(
            invalid.express(&empty_context),
            Graph::new(0, 5)
        );

        let one_node_context = EdgeEditContext {
            base_graph: Graph::new(1, 5),
        };
        let add_self = EdgeEditGenome::new(vec![encode_gene(2, [0, 0, 0, 0], 1)]);
        assert_eq!(
            add_self.express(&one_node_context),
            Graph::new(1, 5)
        );

        let mut base_graph = Graph::new(2, 5);
        base_graph.add_edge(0, 1);
        let context = EdgeEditContext {
            base_graph: base_graph.clone(),
        };
        let invalid = EdgeEditGenome::new(vec![encode_gene(15, [0, 1, 0, 0], 2)]);
        assert_eq!(invalid.express(&context), base_graph);
    }

    #[test]
    fn crossover_swaps_only_a_nonempty_shared_segment() {
        let mut left = EdgeEditGenome::new(vec![0, 1, 2, 3, 4]);
        let mut right =
            EdgeEditGenome::new_with_operation_weights(vec![10, 11, 12], weights_for_add())
                .unwrap();
        let left_tail = left.genes[3..].to_vec();
        let left_weights = left.operation_weights();
        let right_weights = right.operation_weights();
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
        assert_eq!(left.operation_weights(), left_weights);
        assert_eq!(right.operation_weights(), right_weights);
    }

    #[test]
    fn mutation_replaces_between_one_and_four_unique_genes() {
        let mut genome =
            EdgeEditGenome::new_with_operation_weights(vec![8; 10], weights_for_delete()).unwrap();
        let mut rng = StdRng::seed_from_u64(19);

        genome.mutate(&mut rng);

        let changed: Vec<_> = genome.genes.iter().filter(|gene| **gene != 8).collect();
        assert!((1..=4).contains(&changed.len()));
        assert!(changed.iter().all(|gene| **gene & OPCODE_MASK == 3));
    }

    #[test]
    fn mutation_of_an_empty_genome_is_a_noop() {
        let mut genome = EdgeEditGenome::new(Vec::new());
        let mut rng = StdRng::seed_from_u64(23);

        genome.mutate(&mut rng);

        assert!(genome.genes.is_empty());
    }

    #[test]
    fn copy_and_print_include_the_complete_genome() {
        let genome = EdgeEditGenome::new(vec![1, 2, 3]);
        assert_eq!(genome.copy(), genome);
        assert_eq!(genome.print(), "EdgeEditGenome(3 ops): [1, 2, 3]");
    }
}
