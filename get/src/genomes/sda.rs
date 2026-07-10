use rand::Rng;

use super::genome::Genome;

/// Self-driving-automaton genome: a finite-state machine whose run emits the
/// characters that get folded into a graph's adjacency triangle
/// (see `GET_architecturev2.md` §3.2).
#[derive(Clone)]
pub struct SdaGenome {
    pub init_char: u8,
    /// `[state][char] -> next state`
    pub transitions: Vec<Vec<u16>>,
    /// `[state][char] -> chars appended to the output buffer`
    pub responses: Vec<Vec<Vec<u8>>>,
}

impl Genome for SdaGenome {
    fn crossover<R: Rng + ?Sized>(a: &mut Self, b: &mut Self, _rng: &mut R) {
        // TODO: swap a contiguous block of states between the two automata.
        todo!("SDA crossover")
    }
}
