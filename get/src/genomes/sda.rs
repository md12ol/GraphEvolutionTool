use rand::Rng;

use super::genome::Genome;
use crate::graph::Graph;

/// Self-driving-automaton genome: a finite-state machine whose run emits the
/// characters that get folded into a graph's adjacency triangle.
#[derive(Clone)]
pub struct SdaGenome {
    pub init_char: u8,
    /// `[state][char] -> next state`
    pub transitions: Vec<Vec<u16>>,
    /// `[state][char] -> chars appended to the output buffer`
    pub responses: Vec<Vec<Vec<u8>>>,
}

impl Genome for SdaGenome {
    fn express(&self) -> Graph {
        // TODO: run the automaton and fold its output into a graph.
        todo!("SDA express")
    }

    /// One-point crossover over states: choose a cut point and swap every state
    /// row (transitions and responses) from the cut onward between the parents.
    fn crossover<R: Rng + ?Sized>(&mut self, other: &mut Self, rng: &mut R) {
        // States past the shorter automaton's length have no counterpart to swap.
        let states = self.transitions.len().min(other.transitions.len());

        // Need at least two states for a cut that keeps one from each parent.
        if states >= 2 {
            // cut in 1..states, so the swapped suffix is always non-empty.
            let cut = rng.random_range(1..states);
            for state in cut..states {
                std::mem::swap(&mut self.transitions[state], &mut other.transitions[state]);
                std::mem::swap(&mut self.responses[state], &mut other.responses[state]);
            }
        }

        // Give the starting character a chance to cross as well.
        if rng.random::<bool>() {
            std::mem::swap(&mut self.init_char, &mut other.init_char);
        }
    }

    fn mutate<R: Rng + ?Sized>(&mut self, _rng: &mut R) {
        // TODO: perturb a transition, response, or the init char.
        todo!("SDA mutate")
    }

    fn copy(&self) -> Self {
        self.clone()
    }

    fn print(&self) {
        println!(
            "SdaGenome(init={}, {} states)",
            self.init_char,
            self.transitions.len()
        );
    }
}
