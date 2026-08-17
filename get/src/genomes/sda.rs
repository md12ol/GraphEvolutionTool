use rand::Rng;

use super::genome::{Genome, SdaContext};
use crate::graph::Graph;

/// Self-driving-automaton genome: a finite-state machine whose run emits the
/// characters that get folded into a graph's adjacency triangle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SdaGenome {
    pub init_char: u8,
    /// `[state][char] -> next state`
    pub transitions: Vec<Vec<u16>>,
    /// `[state][char] -> chars appended to the output buffer`
    pub responses: Vec<Vec<Vec<u8>>>,
    /// Maximum length of a freshly generated response, used by
    /// [`SdaGenome::randomize`] and [`Genome::mutate`] when a response is
    /// regenerated. Unlike `num_states`/`num_chars`, this isn't observable
    /// from the current data, so it has to be stored rather than derived.
    pub max_resp_len: usize,
}

/// Largest alphabet size representable by [`SdaGenome`]'s `u8`-valued responses.
pub const MAX_NUM_CHARS: usize = u8::MAX as usize + 1;
/// Largest state count representable by [`SdaGenome`]'s `u16`-valued transitions.
pub const MAX_NUM_STATES: usize = u16::MAX as usize + 1;
/// Chance per [`Genome::mutate`] call of mutating the initial character
/// instead of a transition or response.
const INIT_CHAR_MUTATION_RATE: f64 = 0.04;

impl SdaGenome {
    /// Check that `num_states`, `num_chars`, and `max_resp_len` are usable
    /// dimensions for a genome: nonzero, and small enough to fit the storage
    /// types backing `transitions`/`responses`.
    fn validate_dimensions(
        num_states: usize,
        num_chars: usize,
        max_resp_len: usize,
    ) -> Result<(), &'static str> {
        if num_states == 0 || num_states > MAX_NUM_STATES {
            return Err("num_states must be between 1 and 65536");
        }
        if num_chars == 0 || num_chars > MAX_NUM_CHARS {
            return Err("num_chars must be between 1 and 256");
        }
        if max_resp_len == 0 {
            return Err("max_resp_len must be at least 1");
        }
        Ok(())
    }

    /// Build a genome with `num_states` states over an alphabet of
    /// `num_chars` characters, where each transition's response is a random
    /// length between 1 and `max_resp_len` characters, inclusive.
    ///
    /// Returns an error if the dimensions are zero or too large to fit the
    /// genome's storage types (see [`MAX_NUM_STATES`], [`MAX_NUM_CHARS`]).
    pub fn random<R: Rng + ?Sized>(
        num_states: usize,
        num_chars: usize,
        max_resp_len: usize,
        rng: &mut R,
    ) -> Result<Self, &'static str> {
        Self::validate_dimensions(num_states, num_chars, max_resp_len)?;

        let init_char = rng.random_range(0..num_chars) as u8;

        let mut transitions = Vec::with_capacity(num_states);
        for _ in 0..num_states {
            let mut state_transitions = Vec::with_capacity(num_chars);
            for _ in 0..num_chars {
                state_transitions.push(rng.random_range(0..num_states) as u16);
            }
            transitions.push(state_transitions);
        }

        let mut responses = Vec::with_capacity(num_states);
        for _ in 0..num_states {
            let mut state_responses = Vec::with_capacity(num_chars);
            for _ in 0..num_chars {
                let resp_len = rng.random_range(1..=max_resp_len);
                let mut response = Vec::with_capacity(resp_len);
                for _ in 0..resp_len {
                    response.push(rng.random_range(0..num_chars) as u8);
                }
                state_responses.push(response);
            }
            responses.push(state_responses);
        }

        Ok(Self {
            init_char,
            transitions,
            responses,
            max_resp_len,
        })
    }

    /// Build a genome sized for graphs expressed under `edge_multiplicity_cap`:
    /// the alphabet is fixed at `edge_multiplicity_cap + 1` characters (`0..=cap`),
    /// so every character value doubles as a legal edge weight and nothing is
    /// clamped by [`Graph::set_edge`] when the same cap is used to build the
    /// [`SdaContext`] this genome is later expressed against.
    pub fn random_with_edge_multiplicity_cap<R: Rng + ?Sized>(
        num_states: usize,
        edge_multiplicity_cap: u32,
        max_resp_len: usize,
        rng: &mut R,
    ) -> Result<Self, &'static str> {
        Self::random(
            num_states,
            edge_multiplicity_cap as usize + 1,
            max_resp_len,
            rng,
        )
    }

    /// Re-roll the initial character and every transition/response in place,
    /// keeping the current number of states, characters, and `max_resp_len`.
    pub fn randomize<R: Rng + ?Sized>(&mut self, rng: &mut R) -> Result<(), &'static str> {
        let num_states = self.transitions.len();
        let num_chars = self.num_chars();
        *self = Self::random(num_states, num_chars, self.max_resp_len, rng)?;
        Ok(())
    }

    /// The alphabet size implied by the current transition table's row width.
    /// 0 if there are no states yet (`transitions` is empty).
    fn num_chars(&self) -> usize {
        self.transitions.first().map_or(0, |row| row.len())
    }

    /// Run the automaton from `init_state`, producing exactly `output_len`
    /// characters. `output[0]` is `init_char`; each subsequent transition
    /// appends its response's characters (truncated if that would overshoot
    /// `output_len`) and advances `cur_state` before moving to the next
    /// unconsumed character. Every response is at least one character long,
    /// so this always terminates without needing a step cap.
    ///
    /// Callers must ensure `init_state` is a valid state index.
    fn run(&self, init_state: usize, output_len: usize) -> Vec<u8> {
        if output_len == 0 {
            return Vec::new();
        }

        let mut output = Vec::with_capacity(output_len);
        output.push(self.init_char);

        let mut cur_state = init_state;
        // Two cursors into the same growing buffer: tail_idx reads a
        // character already produced; output.len() is where the next one
        // gets written.
        let mut tail_idx = 0;
        while output.len() < output_len {
            let driver = output[tail_idx] as usize;
            for &val in &self.responses[cur_state][driver] {
                if output.len() >= output_len {
                    break;
                }
                output.push(val);
            }
            cur_state = self.transitions[cur_state][driver] as usize;
            tail_idx += 1;
        }

        output
    }
}

impl Genome for SdaGenome {
    type Context = SdaContext;

    /// Run the automaton for exactly one character per upper-triangle pair
    /// and fold the output into a graph: output index `i` maps onto the
    /// `i`-th pair in the same row-major order as [`Graph::get_edge_list`]
    /// (`(0,1), (0,2), ..., (0,n-1), (1,2), ...`), and each character's raw
    /// value becomes that edge's weight. [`Graph::set_edge`] clamps the value
    /// to the cap selected by `SdaContext`, so the same representation can
    /// express unweighted or bounded-multiplicity graphs.
    ///
    /// # Panics
    ///
    /// Panics if this genome's alphabet (`num_chars`, the width of a
    /// transition/response row) disagrees with `context.max_edge_multiplicity`
    /// plus one — the derived-alphabet invariant of §3.2. A genome built
    /// through [`SdaGenome::random_with_edge_multiplicity_cap`] against this
    /// same cap always satisfies it; one built by hand (`SdaGenome::random`,
    /// the Rust-library route) is not checked until expressed, so a mismatch
    /// there would otherwise silently bias the expressed graph toward the cap
    /// (alphabet too large) or leave the upper edge weights unreachable
    /// (alphabet too small).
    fn express(&self, context: &Self::Context) -> Graph {
        let num_chars = self.num_chars();
        let expected_num_chars = context.max_edge_multiplicity as usize + 1;
        assert_eq!(
            num_chars, expected_num_chars,
            "SdaGenome has {num_chars} characters but context.max_edge_multiplicity of {} \
             requires {expected_num_chars}; build the genome with \
             random_with_edge_multiplicity_cap against the same cap",
            context.max_edge_multiplicity,
        );

        let mut graph = Graph::new(context.num_nodes, context.max_edge_multiplicity);
        if context.num_nodes < 2 {
            return graph;
        }

        let output_len = context.num_nodes * (context.num_nodes - 1) / 2;
        let output = self.run(context.init_state, output_len);

        let mut idx = 0;
        for u in 0..context.num_nodes {
            for v in (u + 1)..context.num_nodes {
                graph.set_edge(u, v, output[idx] as u32);
                idx += 1;
            }
        }

        graph
    }

    /// Two-point crossover over states: draw two distinct cut points in
    /// `0..=states` and swap the half-open interior segment `[start, end)`
    /// between the parents, leaving states outside that window untouched on
    /// both sides. Swapping state 0 also swaps `init_char`, since together
    /// they determine the automaton's first transition.
    fn crossover<R: Rng + ?Sized>(&mut self, other: &mut Self, rng: &mut R) {
        // States past the shorter automaton's length have no counterpart to swap.
        let states = self.transitions.len().min(other.transitions.len());
        if states == 0 {
            return;
        }

        // Rejection sampling for two distinct cut points. `loop` is itself an
        // expression here — `break`'s argument becomes the value of (start, end).
        let (start, end) = loop {
            let a = rng.random_range(0..=states);
            let b = rng.random_range(0..=states);
            if a != b {
                break (a.min(b), a.max(b));
            }
        };

        if start == 0 {
            std::mem::swap(&mut self.init_char, &mut other.init_char);
        }

        for state in start..end {
            std::mem::swap(&mut self.transitions[state], &mut other.transitions[state]);
            std::mem::swap(&mut self.responses[state], &mut other.responses[state]);
        }
    }

    /// Apply one mutation: redraw the initial character with probability
    /// `INIT_CHAR_MUTATION_RATE`, otherwise an even chance of redrawing
    /// one transition's target state or one transition's response. Callers
    /// that want more disruption per generation call this multiple times.
    fn mutate<R: Rng + ?Sized>(&mut self, rng: &mut R) {
        let num_states = self.transitions.len();
        let num_chars = self.num_chars();
        if num_states == 0 || num_chars == 0 {
            return;
        }

        if rng.random_bool(INIT_CHAR_MUTATION_RATE) {
            self.init_char = rng.random_range(0..num_chars) as u8;
            return;
        }

        let state = rng.random_range(0..num_states);
        let trans = rng.random_range(0..num_chars);

        // coin flip: turbofish spells out the type `random` can't infer from an argument
        if rng.random::<bool>() {
            self.transitions[state][trans] = rng.random_range(0..num_states) as u16;
        } else {
            let resp_len = rng.random_range(1..=self.max_resp_len);
            self.responses[state][trans] = (0..resp_len)
                .map(|_| rng.random_range(0..num_chars) as u8)
                .collect();
        }
    }

    /// Dump `init_char` followed by one line per `state + char -> target
    /// [ response ]`. `init_state` isn't included since it lives on
    /// `SdaContext`, not the genome, and `print` has no context parameter to
    /// read it from.
    fn print(&self) -> String {
        // Brings write!/writeln! for String into scope; writes to a String
        // can't actually fail, so the .unwrap()s below just satisfy the
        // trait's Result return.
        use std::fmt::Write as _;

        let mut out = String::new();
        writeln!(out, "init_char: {}", self.init_char).unwrap();
        for (state, (state_transitions, state_responses)) in
            self.transitions.iter().zip(&self.responses).enumerate()
        {
            for (trans, (target, response)) in
                state_transitions.iter().zip(state_responses).enumerate()
            {
                write!(out, "{state} + {trans} -> {target} [").unwrap();
                for val in response {
                    write!(out, " {val}").unwrap();
                }
                writeln!(out, " ]").unwrap();
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use super::*;

    /// A hand-built 2-state, 2-char genome used to hand-verify `run`/`express`
    /// without relying on RNG output:
    /// - state 0: char 0 -> state 1, emits [0]; char 1 -> state 0, emits [1]
    /// - state 1: char 0 -> state 0, emits [1]; char 1 -> state 1, emits [0]
    /// - init_char = 0
    fn small_genome() -> SdaGenome {
        SdaGenome {
            init_char: 0,
            transitions: vec![vec![1, 0], vec![0, 1]],
            responses: vec![vec![vec![0], vec![1]], vec![vec![1], vec![0]]],
            max_resp_len: 1,
        }
    }

    #[test]
    fn run_matches_a_hand_traced_execution() {
        let genome = small_genome();

        // init_state = 0:
        //   output = [0]                              (init_char)
        //   responses[0][0] = [0]  -> output = [0, 0]; transitions[0][0] = 1
        //   responses[1][0] = [1]  -> output = [0, 0, 1]; transitions[1][0] = 0
        assert_eq!(genome.run(0, 3), vec![0, 0, 1]);
        assert_eq!(genome.run(0, 0), Vec::<u8>::new());
        assert_eq!(genome.run(0, 1), vec![0]);
    }

    #[test]
    fn express_folds_the_run_into_the_upper_triangle_in_row_major_order() {
        let genome = small_genome();
        // small_genome has a 2-char alphabet, so the matching cap is 1 (§3.2).
        let context = SdaContext {
            num_nodes: 3,
            init_state: 0,
            max_edge_multiplicity: 1,
        };

        let graph = genome.express(&context);

        // run(0, 3) = [0, 0, 1] maps onto (0,1), (0,2), (1,2) in that order.
        assert_eq!(graph.weight(0, 1), 0);
        assert_eq!(graph.weight(0, 2), 0);
        assert_eq!(graph.weight(1, 2), 1);
    }

    #[test]
    fn express_of_zero_or_one_node_contexts_is_an_untouched_empty_graph() {
        let genome = small_genome();

        // small_genome has a 2-char alphabet, so the matching cap is 1 (§3.2).
        for num_nodes in [0, 1] {
            let context = SdaContext {
                num_nodes,
                init_state: 0,
                max_edge_multiplicity: 1,
            };
            assert_eq!(genome.express(&context), Graph::new(num_nodes, 1));
        }
    }

    /// §3.2's derived-alphabet invariant, oversized case: a genome with more
    /// characters than `context.max_edge_multiplicity + 1` allows is refused
    /// at `express` rather than silently letting `Graph::set_edge` clamp the
    /// surplus characters onto the cap. `Graph::set_edge`'s own clamping
    /// behaviour is covered directly in `graph.rs`'s tests.
    #[test]
    #[should_panic(expected = "SdaGenome has 9 characters but context.max_edge_multiplicity of 5")]
    fn express_refuses_an_alphabet_larger_than_the_cap_allows() {
        let genome = SdaGenome {
            init_char: 8,
            transitions: vec![vec![0; 9]],
            responses: vec![vec![vec![0]; 9]],
            max_resp_len: 1,
        };
        let context = SdaContext {
            num_nodes: 2,
            init_state: 0,
            max_edge_multiplicity: 5,
        };

        genome.express(&context);
    }

    /// §3.2's derived-alphabet invariant, undersized case: a genome with
    /// fewer characters than the cap allows would silently leave the upper
    /// edge weights unreachable rather than exploring the space the context
    /// configured. Refused the same way as the oversized case.
    #[test]
    #[should_panic(expected = "SdaGenome has 2 characters but context.max_edge_multiplicity of 5")]
    fn express_refuses_an_alphabet_smaller_than_the_cap_allows() {
        let genome = small_genome();
        let context = SdaContext {
            num_nodes: 3,
            init_state: 0,
            max_edge_multiplicity: 5,
        };

        genome.express(&context);
    }

    #[test]
    fn express_accepts_a_genome_built_with_the_matching_cap_constructor() {
        let mut rng = StdRng::seed_from_u64(0);
        let cap = 3;
        let genome = SdaGenome::random_with_edge_multiplicity_cap(4, cap, 2, &mut rng).unwrap();
        let context = SdaContext {
            num_nodes: 3,
            init_state: 0,
            max_edge_multiplicity: cap,
        };

        // Doesn't panic: random_with_edge_multiplicity_cap derives num_chars
        // from the same cap the context carries, satisfying §3.2 by
        // construction.
        genome.express(&context);
    }

    #[test]
    fn print_dumps_init_char_then_one_line_per_transition() {
        let genome = small_genome();

        assert_eq!(
            genome.print(),
            "init_char: 0\n\
             0 + 0 -> 1 [ 0 ]\n\
             0 + 1 -> 0 [ 1 ]\n\
             1 + 0 -> 0 [ 1 ]\n\
             1 + 1 -> 1 [ 0 ]\n"
        );
    }

    #[test]
    fn equality_compares_every_field() {
        let base = small_genome();

        let mut different_init_char = base.clone();
        different_init_char.init_char = 1;
        assert_ne!(different_init_char, base);

        let mut different_transitions = base.clone();
        different_transitions.transitions[0][0] = 0;
        assert_ne!(different_transitions, base);

        let mut different_responses = base.clone();
        different_responses.responses[0][0] = vec![1];
        assert_ne!(different_responses, base);

        let mut different_max_resp_len = base.clone();
        different_max_resp_len.max_resp_len += 1;
        assert_ne!(different_max_resp_len, base);
    }

    #[test]
    fn random_builds_shapes_matching_the_requested_dimensions() {
        let mut rng = StdRng::seed_from_u64(5);
        let genome = SdaGenome::random(10, 3, 4, &mut rng).unwrap();

        assert!((genome.init_char as usize) < 3);
        assert_eq!(genome.transitions.len(), 10);
        assert_eq!(genome.responses.len(), 10);
        for (state_transitions, state_responses) in
            genome.transitions.iter().zip(genome.responses.iter())
        {
            assert_eq!(state_transitions.len(), 3);
            assert_eq!(state_responses.len(), 3);
            for &target in state_transitions {
                assert!((target as usize) < 10);
            }
            for response in state_responses {
                assert!(!response.is_empty() && response.len() <= 4);
                assert!(response.iter().all(|&c| (c as usize) < 3));
            }
        }
    }

    #[test]
    fn random_rejects_unusable_dimensions() {
        let mut rng = StdRng::seed_from_u64(5);

        assert_eq!(
            SdaGenome::random(0, 3, 4, &mut rng).unwrap_err(),
            "num_states must be between 1 and 65536"
        );
        assert_eq!(
            SdaGenome::random(MAX_NUM_STATES + 1, 3, 4, &mut rng).unwrap_err(),
            "num_states must be between 1 and 65536"
        );
        assert_eq!(
            SdaGenome::random(10, 0, 4, &mut rng).unwrap_err(),
            "num_chars must be between 1 and 256"
        );
        assert_eq!(
            SdaGenome::random(10, MAX_NUM_CHARS + 1, 4, &mut rng).unwrap_err(),
            "num_chars must be between 1 and 256"
        );
        assert_eq!(
            SdaGenome::random(10, 3, 0, &mut rng).unwrap_err(),
            "max_resp_len must be at least 1"
        );
    }

    #[test]
    fn random_with_edge_multiplicity_cap_derives_num_chars_from_the_cap() {
        let mut rng = StdRng::seed_from_u64(5);
        let cap = 3;
        let genome = SdaGenome::random_with_edge_multiplicity_cap(10, cap, 4, &mut rng).unwrap();

        let num_chars = cap as usize + 1;
        assert_eq!(genome.transitions[0].len(), num_chars);
        assert_eq!(genome.responses[0].len(), num_chars);
    }

    #[test]
    fn randomize_keeps_dimensions_but_changes_contents() {
        let mut rng = StdRng::seed_from_u64(9);
        let mut genome = SdaGenome::random(10, 3, 2, &mut rng).unwrap();
        let before = genome.clone();

        genome.randomize(&mut rng).unwrap();

        assert_eq!(genome.transitions.len(), before.transitions.len());
        assert_eq!(genome.responses.len(), before.responses.len());
        assert_eq!(genome.max_resp_len, before.max_resp_len);
        for state_transitions in &genome.transitions {
            assert_eq!(state_transitions.len(), 3);
            assert!(
                state_transitions
                    .iter()
                    .all(|&target| (target as usize) < 10)
            );
        }
        for state_responses in &genome.responses {
            assert_eq!(state_responses.len(), 3);
            for response in state_responses {
                assert!(!response.is_empty() && response.len() <= 2);
            }
        }
        assert!(
            genome.transitions != before.transitions || genome.responses != before.responses,
            "randomize should change at least one transition or response"
        );
    }

    #[test]
    fn randomize_of_an_empty_genome_propagates_the_dimension_error() {
        let mut rng = StdRng::seed_from_u64(9);
        let mut genome = SdaGenome::random(1, 1, 1, &mut rng).unwrap();
        genome.transitions.clear();
        genome.responses.clear();

        assert_eq!(
            genome.randomize(&mut rng).unwrap_err(),
            "num_states must be between 1 and 65536"
        );
    }

    #[test]
    fn mutate_changes_exactly_one_thing_per_call() {
        let mut rng = StdRng::seed_from_u64(3);
        let mut genome = SdaGenome::random(10, 3, 2, &mut rng).unwrap();

        for _ in 0..50 {
            let before = genome.clone();
            genome.mutate(&mut rng);

            let init_char_changed = (genome.init_char != before.init_char) as usize;

            let mut changed_transitions = 0;
            for (before_row, after_row) in before.transitions.iter().zip(&genome.transitions) {
                for (before_val, after_val) in before_row.iter().zip(after_row) {
                    if before_val != after_val {
                        changed_transitions += 1;
                    }
                }
            }

            let mut changed_responses = 0;
            for (before_row, after_row) in before.responses.iter().zip(&genome.responses) {
                for (before_val, after_val) in before_row.iter().zip(after_row) {
                    if before_val != after_val {
                        changed_responses += 1;
                    }
                }
            }

            let changes = init_char_changed + changed_transitions + changed_responses;
            assert!(changes <= 1, "expected at most one change, got {changes}");
        }
    }

    #[test]
    fn mutate_of_an_empty_genome_is_a_noop() {
        let mut rng = StdRng::seed_from_u64(3);
        let mut genome = SdaGenome::random(1, 1, 1, &mut rng).unwrap();
        genome.transitions.clear();
        genome.responses.clear();
        let before = genome.clone();

        genome.mutate(&mut rng);

        assert_eq!(genome.init_char, before.init_char);
        assert!(genome.transitions.is_empty());
        assert!(genome.responses.is_empty());
    }

    /// Tag every state in `genome` with a marker value (kept under
    /// `u8::MAX` so it fits both `transitions` and `responses`) so a later
    /// crossover can tell which parent each state came from.
    fn tag_states(genome: &mut SdaGenome, init_char: u8, base: u16) {
        genome.init_char = init_char;
        for (state, (state_transitions, state_responses)) in genome
            .transitions
            .iter_mut()
            .zip(genome.responses.iter_mut())
            .enumerate()
        {
            let marker = base + state as u16;
            for target in state_transitions.iter_mut() {
                *target = marker;
            }
            for response in state_responses.iter_mut() {
                *response = vec![marker as u8];
            }
        }
    }

    #[test]
    fn crossover_swaps_only_a_contiguous_segment_and_ties_init_char_to_state_zero() {
        let num_states = 6;
        let mut setup_rng = StdRng::seed_from_u64(1);

        for trial_seed in 0..200 {
            let mut left = SdaGenome::random(num_states, 2, 2, &mut setup_rng).unwrap();
            let mut right = SdaGenome::random(num_states, 2, 2, &mut setup_rng).unwrap();
            tag_states(&mut left, 111, 100);
            tag_states(&mut right, 222, 200);

            let mut trial_rng = StdRng::seed_from_u64(trial_seed);
            left.crossover(&mut right, &mut trial_rng);

            let swapped: Vec<usize> = (0..num_states)
                .filter(|&state| left.transitions[state][0] >= 200)
                .collect();

            // The transition and response markers for a state always match,
            // proving they swap together rather than independently.
            for state in 0..num_states {
                let left_marker = left.transitions[state][0];
                assert!(
                    left.responses[state]
                        .iter()
                        .all(|r| r[0] as u16 == left_marker)
                );
                let right_marker = right.transitions[state][0];
                assert!(
                    right.responses[state]
                        .iter()
                        .all(|r| r[0] as u16 == right_marker)
                );
            }
            for state in (0..num_states).filter(|s| !swapped.contains(s)) {
                assert!(left.transitions[state].iter().all(|&t| t < 200));
                assert!(right.transitions[state].iter().all(|&t| t >= 200));
            }

            // The swapped set must be a single contiguous run, if non-empty.
            if let (Some(&first), Some(&last)) = (swapped.first(), swapped.last()) {
                let mut expected = Vec::new();
                for state in first..=last {
                    expected.push(state);
                }
                assert_eq!(swapped, expected);
            }

            // init_char swaps iff state 0 was part of the swapped segment.
            let state_zero_swapped = swapped.first() == Some(&0);
            assert_eq!(left.init_char == 222, state_zero_swapped);
            assert_eq!(right.init_char == 111, state_zero_swapped);
        }
    }

    #[test]
    fn crossover_of_empty_genomes_is_a_noop() {
        let mut rng = StdRng::seed_from_u64(1);
        let mut left = SdaGenome::random(1, 1, 1, &mut rng).unwrap();
        let mut right = SdaGenome::random(1, 1, 1, &mut rng).unwrap();
        left.transitions.clear();
        left.responses.clear();
        right.transitions.clear();
        right.responses.clear();
        let left_before = left.clone();
        let right_before = right.clone();

        left.crossover(&mut right, &mut rng);

        assert_eq!(left.init_char, left_before.init_char);
        assert_eq!(right.init_char, right_before.init_char);
    }
}
