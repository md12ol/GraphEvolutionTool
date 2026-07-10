use rand::Rng;

use super::genome::Genome;
use crate::graph::Graph;

/// Edit-script genome: a list of encoded graph-edit opcodes.
///
/// Low 4 bits are the opcode, upper bits are mixed-radix vertex indices.
#[derive(Clone)]
pub struct EdgeEditGenome(pub Vec<u64>);

impl Genome for EdgeEditGenome {
    fn express(&self) -> Graph {
        // TODO: decode each opcode and apply it to a base graph.
        todo!("edge-edit express")
    }

    fn crossover<R: Rng + ?Sized>(&mut self, _b: &mut Self, _rng: &mut R) {
        // TODO: two-point crossover over the opcode vector.
        todo!("edge-edit crossover")
    }

    fn mutate<R: Rng + ?Sized>(&mut self, _rng: &mut R) {
        // TODO: perturb one or more opcodes.
        todo!("edge-edit mutate")
    }

    fn copy(&self) -> Self {
        self.clone()
    }

    fn print(&self) {
        println!("EdgeEditGenome({} ops): {:?}", self.0.len(), self.0);
    }
}
