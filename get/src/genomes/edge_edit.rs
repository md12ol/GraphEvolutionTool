use rand::Rng;

use super::genome::Genome;

/// Edit-script genome: a list of encoded graph-edit opcodes.
///
/// Low 4 bits are the opcode, upper bits are mixed-radix vertex indices
/// (see `GET_architecturev2.md` §3.1).
#[derive(Clone)]
pub struct EdgeEditGenome(pub Vec<u64>);

impl Genome for EdgeEditGenome {
    fn crossover<R: Rng + ?Sized>(a: &mut Self, b: &mut Self, _rng: &mut R) {
        // TODO: two-point crossover over the opcode vector.
        todo!("edge-edit crossover")
    }
}
