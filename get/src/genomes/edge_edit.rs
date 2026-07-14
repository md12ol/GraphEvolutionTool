use rand::Rng;

use super::genome::{EdgeEditContext, Express, Genome};
use crate::graph::Graph;

/// Edit-script genome: a list of encoded graph-edit opcodes.
///
/// Low 4 bits are the opcode, upper bits are mixed-radix vertex indices.
#[derive(Clone)]
pub struct EdgeEditGenome(pub Vec<u64>);

impl Genome for EdgeEditGenome {
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

    fn print(&self) -> String {
        format!("EdgeEditGenome({} ops): {:?}", self.0.len(), self.0)
    }
}

impl Express for EdgeEditGenome {
    type Context = EdgeEditContext;

    fn express(&self, _context: &Self::Context) -> Graph {
        // TODO: clone the base graph, decode each opcode, and apply its edit.
        todo!("edge-edit express")
    }
}
