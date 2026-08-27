//! The self-driving-automaton representation: the genome, the checked
//! dimensions it is built to, and the default mutation rates.

use rand::Rng;

use super::genome::{Genome, SdaContext, SdaMutation};
use crate::graph::Graph;

/// Self-driving-automaton genome: a finite-state machine whose run emits the
/// characters folded into the upper triangle of a graph's adjacency matrix.
#[derive(Clone, Debug, PartialEq)]
pub struct SdaGenome {
    init_char: u8,
    /// `[state][char] -> next state`
    transitions: Vec<Vec<u16>>,
    /// `[state][char] -> chars appended to the output buffer`
    responses: Vec<Vec<Vec<u8>>>,
    /// Maximum length of a response generated later. Unlike
    /// `num_states`/`num_chars` it is not observable from the current data, so
    /// it is stored rather than derived.
    max_resp_len: usize,
}

/// Largest alphabet size representable by [`SdaGenome`]'s `u8`-valued responses.
const MAX_NUM_CHARS: usize = u8::MAX as usize + 1;
/// Largest state count representable by [`SdaGenome`]'s `u16`-valued transitions.
const MAX_NUM_STATES: usize = u16::MAX as usize + 1;
/// Default for [`SdaContext::init_char_mutation_rate`]: the chance per
/// mutation of redrawing the initial character instead of a transition or
/// response.
pub const DEFAULT_INIT_CHAR_MUTATION_RATE: f64 = 0.04;
/// Default for [`SdaContext::transition_vs_response_rate`]: an even split between
/// redrawing a transition's target state and redrawing its response.
pub const DEFAULT_TRANSITION_VS_RESPONSE_RATE: f64 = 0.5;

/// Genome dimensions that have already been checked, so a caller building a
/// whole population validates once rather than once per individual.
///
/// Every constructor checks, so holding a value of this type is the proof that
/// the numbers are usable, which is what lets
/// [`SdaGenome::random_with_dimensions`] be infallible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SdaDimensions {
    num_states: usize,
    num_chars: usize,
    max_resp_len: usize,
}

impl SdaDimensions {
    /// Check that `num_states`, `num_chars`, and `max_resp_len` are usable
    /// dimensions for a genome: nonzero, and small enough to fit the storage
    /// types backing `transitions`/`responses`.
    ///
    /// Crate-internal because `num_chars` is taken directly: an alphabet that
    /// is not one more than the cap of the [`SdaContext`] the genome is
    /// expressed against makes `express` panic.
    pub(crate) fn new(
        num_states: usize,
        num_chars: usize,
        max_resp_len: usize,
    ) -> Result<Self, &'static str> {
        if num_states == 0 || num_states > MAX_NUM_STATES {
            return Err("num_states must be between 1 and 65536");
        }
        if num_chars == 0 || num_chars > MAX_NUM_CHARS {
            return Err("num_chars must be between 1 and 256");
        }
        if max_resp_len == 0 {
            return Err("max_resp_len must be at least 1");
        }
        Ok(Self {
            num_states,
            num_chars,
            max_resp_len,
        })
    }

    /// Dimensions for graphs expressed under `edge_multiplicity_cap`: the
    /// alphabet is fixed at `edge_multiplicity_cap + 1` characters (`0..=cap`),
    /// so every character value doubles as a legal edge weight and nothing is
    /// clamped by [`Graph::set_edge`] when the same cap builds the
    /// [`SdaContext`] the genome is expressed against.
    pub fn from_edge_multiplicity_cap(
        num_states: usize,
        edge_multiplicity_cap: u32,
        max_resp_len: usize,
    ) -> Result<Self, &'static str> {
        let num_chars = edge_multiplicity_cap as usize + 1;
        Self::new(num_states, num_chars, max_resp_len)
    }
}

impl SdaGenome {
    /// Build a genome sized for graphs expressed under `edge_multiplicity_cap`:
    /// the alphabet is `cap + 1` characters, `0..=cap`, so every character is a
    /// legal edge weight and [`Graph::set_edge`] clamps nothing — provided the
    /// same cap builds the [`SdaContext`] this is expressed against. Each
    /// response is a random 1..=`max_resp_len` characters.
    ///
    /// For a whole population, validate once into an [`SdaDimensions`] and loop
    /// through [`SdaGenome::random_with_dimensions`] instead.
    ///
    /// Returns an error if the dimensions are zero or exceed the storage types
    /// (`num_states` 65536, `num_chars` 256).
    pub fn random_with_edge_multiplicity_cap<R: Rng + ?Sized>(
        num_states: usize,
        edge_multiplicity_cap: u32,
        max_resp_len: usize,
        rng: &mut R,
    ) -> Result<Self, &'static str> {
        let dimensions = SdaDimensions::from_edge_multiplicity_cap(
            num_states,
            edge_multiplicity_cap,
            max_resp_len,
        )?;
        Ok(Self::random_with_dimensions(&dimensions, rng))
    }

    /// Build a random genome to already-checked dimensions.
    ///
    /// Infallible by construction: every value that could be rejected was
    /// rejected when the [`SdaDimensions`] was built.
    pub fn random_with_dimensions<R: Rng + ?Sized>(
        dimensions: &SdaDimensions,
        rng: &mut R,
    ) -> Self {
        let SdaDimensions {
            num_states,
            num_chars,
            max_resp_len,
        } = *dimensions;

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

        Self {
            init_char,
            transitions,
            responses,
            max_resp_len,
        }
    }

    /// Build a genome from a *chosen* automaton rather than a random one — a
    /// hand-designed fixture, or a previous run's winner read back through the
    /// accessors below.
    ///
    /// `transitions` is `[state][char] -> next state` and `responses` is
    /// `[state][char] -> characters appended to the output`; both are
    /// `num_states` rows of `num_chars`, and the alphabet size comes from that
    /// row width. `max_resp_len` bounds only the responses a later mutation
    /// generates, not the ones supplied here.
    ///
    /// Errors rather than letting the failure land mid-run, where it is far
    /// harder to attribute: bad dimensions, a wrong row width, a transition
    /// targeting a state that does not exist, or a character outside the
    /// alphabet each panic at expression, and **an empty response does not
    /// panic, it hangs** — the automaton makes progress only by appending a
    /// response's characters. The alphabet is not checked against a context
    /// here; nothing yet knows which one, and [`Genome::express`] asserts that
    /// pairing.
    pub fn from_parts(
        init_char: u8,
        transitions: Vec<Vec<u16>>,
        responses: Vec<Vec<Vec<u8>>>,
        max_resp_len: usize,
    ) -> Result<Self, &'static str> {
        let num_states = transitions.len();
        if num_states == 0 || num_states > MAX_NUM_STATES {
            return Err("num_states must be between 1 and 65536");
        }
        if responses.len() != num_states {
            return Err("responses must have exactly as many rows as transitions");
        }

        let num_chars = transitions[0].len();
        if num_chars == 0 || num_chars > MAX_NUM_CHARS {
            return Err("num_chars must be between 1 and 256");
        }
        if max_resp_len == 0 {
            return Err("max_resp_len must be at least 1");
        }
        if init_char as usize >= num_chars {
            return Err("init_char must be a character in the alphabet");
        }

        for state in 0..num_states {
            if transitions[state].len() != num_chars || responses[state].len() != num_chars {
                return Err("every transition and response row must be num_chars wide");
            }

            for &target in &transitions[state] {
                if target as usize >= num_states {
                    return Err("every transition must target a state that exists");
                }
            }

            for response in &responses[state] {
                if response.is_empty() {
                    return Err("every response must be at least one character long");
                }
                for &character in response {
                    if character as usize >= num_chars {
                        return Err("every response character must be in the alphabet");
                    }
                }
            }
        }

        Ok(Self {
            init_char,
            transitions,
            responses,
            max_resp_len,
        })
    }

    /// The character the automaton's output always starts with.
    ///
    /// This and the accessors below are together exactly
    /// [`SdaGenome::from_parts`]'s arguments, so an automaton can be read off
    /// one genome and rebuilt as another.
    pub fn init_char(&self) -> u8 {
        self.init_char
    }

    /// `[state][char] -> next state`.
    pub fn transitions(&self) -> &[Vec<u16>] {
        &self.transitions
    }

    /// `[state][char] -> characters appended to the output buffer`.
    pub fn responses(&self) -> &[Vec<Vec<u8>>] {
        &self.responses
    }

    /// The bound on responses generated by a later mutation.
    pub fn max_resp_len(&self) -> usize {
        self.max_resp_len
    }

    /// The alphabet size implied by the current transition table's row width.
    /// 0 if there are no states yet (`transitions` is empty).
    fn num_chars(&self) -> usize {
        self.transitions.first().map_or(0, |row| row.len())
    }

    /// Run the automaton from `init_state`, producing exactly `output_len`
    /// characters. `output[0]` is `init_char`; each transition appends its
    /// response, truncated if that would overshoot `output_len`. Every
    /// response is at least one character long, so this always terminates
    /// without needing a step cap.
    ///
    /// Callers must ensure `init_state` is a valid state index.
    fn run(&self, init_state: usize, output_len: usize) -> Vec<u8> {
        if output_len == 0 {
            return Vec::new();
        }

        let mut output = Vec::with_capacity(output_len);
        output.push(self.init_char);

        let mut cur_state = init_state;
        // Two cursors into the same growing buffer: tail_idx reads a character
        // already produced, output.len() is where the next one gets written.
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

    /// The body of [`SdaMutation::RedrawOne`].
    fn redraw_one<R: Rng + ?Sized>(&mut self, context: &SdaContext, rng: &mut R) {
        let num_states = self.transitions.len();
        let num_chars = self.num_chars();
        if num_states == 0 || num_chars == 0 {
            return;
        }

        if rng.random_bool(context.init_char_mutation_rate) {
            self.init_char = rng.random_range(0..num_chars) as u8;
            return;
        }

        let state = rng.random_range(0..num_states);
        let trans = rng.random_range(0..num_chars);

        if rng.random_bool(context.transition_vs_response_rate) {
            self.transitions[state][trans] = rng.random_range(0..num_states) as u16;
        } else {
            let resp_len = rng.random_range(1..=self.max_resp_len);
            let mut response = Vec::with_capacity(resp_len);
            for _ in 0..resp_len {
                response.push(rng.random_range(0..num_chars) as u8);
            }
            self.responses[state][trans] = response;
        }
    }
}

impl Genome for SdaGenome {
    type Context = SdaContext;

    /// Run the automaton for one character per upper-triangle vertex pair: the
    /// character at output index `i` is the weight of the `i`-th pair in
    /// row-major order, `(0,1), (0,2), ..., (0,n-1), (1,2), ...`.
    ///
    /// Panics unless this genome's alphabet (`num_chars`, the width of a
    /// transition/response row) is `context.max_edge_multiplicity` plus one:
    /// too large biases the expressed graph toward the cap, too small leaves the
    /// upper edge weights unreachable.
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

    /// Two-point crossover over states: swap the half-open segment
    /// `[start, end)` between the parents, leaving states outside it untouched.
    /// Swapping state 0 also swaps `init_char`, since together they determine
    /// the automaton's first transition.
    ///
    /// A transition stores a *target state index*, so a swapped band means
    /// anything only against a state table of the same size. `num_states` is a
    /// config value fixed for the whole run, which is what makes that safe —
    /// `shared_length` guards the positions indexed, not the values carried.
    fn crossover<R: Rng + ?Sized>(&mut self, other: &mut Self, rng: &mut R) {
        // States past the shorter automaton's length have no counterpart to swap.
        let shared_length = self.transitions.len().min(other.transitions.len());
        if shared_length == 0 {
            return;
        }

        let (start, end) = super::two_distinct_cut_points(shared_length, rng);

        if start == 0 {
            std::mem::swap(&mut self.init_char, &mut other.init_char);
        }

        for state in start..end {
            std::mem::swap(&mut self.transitions[state], &mut other.transitions[state]);
            std::mem::swap(&mut self.responses[state], &mut other.responses[state]);
        }
    }

    fn mutate<R: Rng + ?Sized>(&mut self, context: &Self::Context, rng: &mut R) {
        match context.mutation {
            SdaMutation::RedrawOne => self.redraw_one(context, rng),
            // ADD A MUTATION STEP 2 (for SDA) — the arm performing your variant,
            // changing exactly one thing about `self`.
            //
            //     SdaMutation::MyMutation { some_param } => self.my_mutation(some_param, rng),
        }
    }

    /// `init_state` is not included: it lives on `SdaContext`, which `print`
    /// has no parameter to read.
    fn print(&self) -> String {
        // Writes to a String cannot fail, so the unwraps below only discharge
        // the trait's Result return.
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

    /// A context for the expression tests, which are indifferent to the two
    /// mutation rates. Mutation tests build their context directly, since the
    /// rates are the thing under test.
    fn express_context(num_nodes: usize, max_edge_multiplicity: u32) -> SdaContext {
        SdaContext {
            num_nodes,
            init_state: 0,
            max_edge_multiplicity,
            init_char_mutation_rate: DEFAULT_INIT_CHAR_MUTATION_RATE,
            transition_vs_response_rate: DEFAULT_TRANSITION_VS_RESPONSE_RATE,
            mutation: Default::default(),
        }
    }

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
        // small_genome has a 2-char alphabet, so the matching cap is 1.
        let context = express_context(3, 1);

        let graph = genome.express(&context);

        // run(0, 3) = [0, 0, 1] maps onto (0,1), (0,2), (1,2) in that order.
        assert_eq!(graph.weight(0, 1), 0);
        assert_eq!(graph.weight(0, 2), 0);
        assert_eq!(graph.weight(1, 2), 1);
    }

    #[test]
    fn express_of_zero_or_one_node_contexts_is_an_untouched_empty_graph() {
        let genome = small_genome();

        // small_genome has a 2-char alphabet, so the matching cap is 1.
        for num_nodes in [0, 1] {
            let context = express_context(num_nodes, 1);
            assert_eq!(genome.express(&context), Graph::new(num_nodes, 1));
        }
    }

    /// The derived-alphabet invariant, oversized case: a genome with more
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
        let context = express_context(2, 5);

        genome.express(&context);
    }

    /// The derived-alphabet invariant, undersized case: a genome with
    /// fewer characters than the cap allows would silently leave the upper
    /// edge weights unreachable rather than exploring the space the context
    /// configured. Refused the same way as the oversized case.
    #[test]
    #[should_panic(expected = "SdaGenome has 2 characters but context.max_edge_multiplicity of 5")]
    fn express_refuses_an_alphabet_smaller_than_the_cap_allows() {
        let genome = small_genome();
        let context = express_context(3, 5);

        genome.express(&context);
    }

    #[test]
    fn express_accepts_a_genome_built_with_the_matching_cap_constructor() {
        let mut rng = StdRng::seed_from_u64(0);
        let cap = 3;
        let genome = SdaGenome::random_with_edge_multiplicity_cap(4, cap, 2, &mut rng).unwrap();
        let context = express_context(3, cap);

        // Doesn't panic: random_with_edge_multiplicity_cap derives num_chars
        // from the same cap the context carries, satisfying the invariant by
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
        let genome = SdaGenome::random_with_edge_multiplicity_cap(10, 2, 4, &mut rng).unwrap();

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
            SdaGenome::random_with_edge_multiplicity_cap(0, 2, 4, &mut rng).unwrap_err(),
            "num_states must be between 1 and 65536"
        );
        assert_eq!(
            SdaGenome::random_with_edge_multiplicity_cap(MAX_NUM_STATES + 1, 2, 4, &mut rng)
                .unwrap_err(),
            "num_states must be between 1 and 65536"
        );
        // A cap of MAX_NUM_CHARS asks for MAX_NUM_CHARS + 1 characters. The
        // too-small case cannot arise on this route: the alphabet is `cap + 1`,
        // so it never comes out at zero. `SdaDimensions::new` takes `num_chars`
        // directly and is the only way to reach that error — see
        // `dimensions_reject_an_empty_alphabet`.
        assert_eq!(
            SdaGenome::random_with_edge_multiplicity_cap(10, MAX_NUM_CHARS as u32, 4, &mut rng)
                .unwrap_err(),
            "num_chars must be between 1 and 256"
        );
        assert_eq!(
            SdaGenome::random_with_edge_multiplicity_cap(10, 2, 0, &mut rng).unwrap_err(),
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
    fn dimensions_reject_an_empty_alphabet() {
        // Unreachable through the cap constructor, where the alphabet is
        // `cap + 1`, so this is the check that keeps the branch honest.
        assert_eq!(
            SdaDimensions::new(10, 0, 4).unwrap_err(),
            "num_chars must be between 1 and 256"
        );
    }

    #[test]
    fn random_with_dimensions_builds_shapes_matching_the_token() {
        let mut rng = StdRng::seed_from_u64(5);
        let dimensions = SdaDimensions::new(7, 4, 3).unwrap();

        let genome = SdaGenome::random_with_dimensions(&dimensions, &mut rng);

        assert_eq!(genome.transitions.len(), 7);
        assert_eq!(genome.responses.len(), 7);
        assert_eq!(genome.max_resp_len, 3);
        for state in 0..7 {
            assert_eq!(genome.transitions[state].len(), 4);
            assert_eq!(genome.responses[state].len(), 4);
            for target in &genome.transitions[state] {
                assert!((*target as usize) < 7, "transition target is a valid state");
            }
            for response in &genome.responses[state] {
                assert!(
                    !response.is_empty() && response.len() <= 3,
                    "response length is between 1 and max_resp_len",
                );
                for character in response {
                    assert!((*character as usize) < 4, "response stays in the alphabet");
                }
            }
        }
        assert!((genome.init_char as usize) < 4);
    }

    #[test]
    fn the_cap_constructor_delegates_to_the_dimension_constructor() {
        // The generation logic lives in exactly one place, so the same seed
        // through either route has to produce the identical genome.
        let cap = 3;
        let dimensions = SdaDimensions::from_edge_multiplicity_cap(10, cap, 4).unwrap();

        let mut wrapper_rng = StdRng::seed_from_u64(11);
        let wrapped =
            SdaGenome::random_with_edge_multiplicity_cap(10, cap, 4, &mut wrapper_rng).unwrap();

        let mut direct_rng = StdRng::seed_from_u64(11);
        let direct = SdaGenome::random_with_dimensions(&dimensions, &mut direct_rng);

        assert_eq!(wrapped, direct);
    }

    /// How many init_chars, transition targets and response characters differ
    /// between two versions of one genome.
    fn count_changes(before: &SdaGenome, after: &SdaGenome) -> (usize, usize, usize) {
        let init_char_changed = (after.init_char != before.init_char) as usize;

        let mut changed_transitions = 0;
        for (before_row, after_row) in before.transitions.iter().zip(&after.transitions) {
            for (before_val, after_val) in before_row.iter().zip(after_row) {
                if before_val != after_val {
                    changed_transitions += 1;
                }
            }
        }

        let mut changed_responses = 0;
        for (before_row, after_row) in before.responses.iter().zip(&after.responses) {
            for (before_val, after_val) in before_row.iter().zip(after_row) {
                if before_val != after_val {
                    changed_responses += 1;
                }
            }
        }

        (init_char_changed, changed_transitions, changed_responses)
    }

    /// A context carrying the two mutation rates a test wants to pin. The
    /// expression fields are irrelevant to `mutate` and take fixed values.
    fn mutation_context(
        init_char_mutation_rate: f64,
        transition_vs_response_rate: f64,
    ) -> SdaContext {
        SdaContext {
            num_nodes: 3,
            init_state: 0,
            max_edge_multiplicity: 1,
            init_char_mutation_rate,
            transition_vs_response_rate,
            mutation: Default::default(),
        }
    }

    /// Mutate a fresh genome `calls` times under `context`, returning the
    /// running totals of what changed.
    fn totals_after_mutating(context: &SdaContext, calls: usize) -> (usize, usize, usize) {
        let mut rng = StdRng::seed_from_u64(3);
        let mut genome = SdaGenome::random_with_edge_multiplicity_cap(10, 2, 2, &mut rng).unwrap();

        let (mut init_chars, mut transitions, mut responses) = (0, 0, 0);
        for _ in 0..calls {
            let before = genome.clone();
            genome.mutate(context, &mut rng);
            let (i, t, r) = count_changes(&before, &genome);
            init_chars += i;
            transitions += t;
            responses += r;
        }

        (init_chars, transitions, responses)
    }

    /// Every redraw has to land inside the genome's own dimensions, and the
    /// alphabet is the one that bites: `num_chars` is `cap + 1` precisely so
    /// that each character is a weight `set_edge` will not clamp. A redraw one
    /// past the end leaves a perfectly valid-looking genome and quietly biases
    /// every graph it expresses toward the cap, because the surplus character
    /// clamps down onto it — the count is respected and the cap is not.
    #[test]
    fn a_redraw_never_leaves_the_alphabet_the_state_table_or_a_response() {
        let cap = 3;
        let num_chars = cap as usize + 1;
        let mut rng = StdRng::seed_from_u64(90);
        let mut genome = SdaGenome::random_with_edge_multiplicity_cap(5, cap, 4, &mut rng).unwrap();
        let context = mutation_context(0.25, 0.5);

        for call in 0..2000 {
            genome.mutate(&context, &mut rng);

            assert!(
                (genome.init_char as usize) < num_chars,
                "call {call}: init_char {} is outside 0..{num_chars}",
                genome.init_char
            );
            for (state, targets) in genome.transitions.iter().enumerate() {
                for &target in targets {
                    assert!(
                        (target as usize) < genome.transitions.len(),
                        "call {call}: state {state} points at {target}"
                    );
                }
            }
            for (state, responses) in genome.responses.iter().enumerate() {
                for response in responses {
                    // An empty response would stall the read head, and the
                    // automaton terminates only because none can be empty.
                    assert!(
                        !response.is_empty(),
                        "call {call}: state {state} grew an empty response"
                    );
                    for &character in response {
                        assert!(
                            (character as usize) < num_chars,
                            "call {call}: response character {character} is outside 0..{num_chars}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn mutate_changes_exactly_one_thing_per_call() {
        let mut rng = StdRng::seed_from_u64(3);
        let mut genome = SdaGenome::random_with_edge_multiplicity_cap(10, 2, 2, &mut rng).unwrap();

        for _ in 0..50 {
            let before = genome.clone();
            genome.mutate(&express_context(3, 1), &mut rng);

            let (init_char_changed, changed_transitions, changed_responses) =
                count_changes(&before, &genome);
            let changes = init_char_changed + changed_transitions + changed_responses;
            assert!(changes <= 1, "expected at most one change, got {changes}");
        }
    }

    /// The rates are not just parsed and carried — they decide what a mutation
    /// is allowed to touch. Each case pins one rate to an extreme and checks
    /// the halves it excludes never move across 200 calls.
    #[test]
    fn the_init_char_rate_decides_whether_the_initial_character_can_move() {
        let (init_chars, transitions, responses) =
            totals_after_mutating(&mutation_context(1.0, 0.5), 200);
        assert!(init_chars > 0, "init_char should be redrawn at rate 1.0");
        assert_eq!(
            (transitions, responses),
            (0, 0),
            "at rate 1.0 every mutation returns after init_char, touching nothing else"
        );

        let (init_chars, transitions, responses) =
            totals_after_mutating(&mutation_context(0.0, 0.5), 200);
        assert_eq!(init_chars, 0, "init_char must never move at rate 0.0");
        assert!(
            transitions > 0 && responses > 0,
            "the transition table takes every mutation instead, got {transitions} and {responses}"
        );
    }

    #[test]
    fn the_transition_vs_response_rate_decides_which_half_of_the_table_moves() {
        // init_char is switched off in both cases, so the only choice left is
        // the one under test.
        let (_, transitions, responses) = totals_after_mutating(&mutation_context(0.0, 1.0), 200);
        assert!(transitions > 0, "targets should be redrawn at rate 1.0");
        assert_eq!(responses, 0, "no response may move at rate 1.0");

        let (_, transitions, responses) = totals_after_mutating(&mutation_context(0.0, 0.0), 200);
        assert_eq!(transitions, 0, "no target may move at rate 0.0");
        assert!(responses > 0, "responses should be redrawn at rate 0.0");
    }

    #[test]
    fn mutate_of_an_empty_genome_is_a_noop() {
        let mut rng = StdRng::seed_from_u64(3);
        let mut genome = SdaGenome::random_with_edge_multiplicity_cap(1, 0, 1, &mut rng).unwrap();
        genome.transitions.clear();
        genome.responses.clear();
        let before = genome.clone();

        genome.mutate(&express_context(3, 1), &mut rng);

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
            let mut left =
                SdaGenome::random_with_edge_multiplicity_cap(num_states, 1, 2, &mut setup_rng)
                    .unwrap();
            let mut right =
                SdaGenome::random_with_edge_multiplicity_cap(num_states, 1, 2, &mut setup_rng)
                    .unwrap();
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
    fn the_genome_stays_shareable_across_evaluation_threads() {
        // `Genome: Clone + Send + Sync` is what lets rayon score a population;
        // nothing on this genome may quietly break it.
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SdaGenome>();
    }

    #[test]
    fn crossover_at_one_shared_state_always_swaps_it_and_init_char() {
        // The deliberate divergence from EdgeEditGenome, which declines here.
        // One shared state forces the cut points to (0, 1), so state 0 is
        // always the segment — and init_char travels with it, which is the
        // thing edge-edit's genes have no equivalent of.
        let mut rng = StdRng::seed_from_u64(3);

        for _ in 0..25 {
            let mut left = SdaGenome::random_with_edge_multiplicity_cap(1, 2, 2, &mut rng).unwrap();
            let mut right =
                SdaGenome::random_with_edge_multiplicity_cap(1, 2, 2, &mut rng).unwrap();
            left.init_char = 0;
            right.init_char = 2;
            let left_before = left.clone();
            let right_before = right.clone();

            left.crossover(&mut right, &mut rng);

            assert_eq!(left.init_char, 2, "init_char must travel with state 0");
            assert_eq!(right.init_char, 0);
            assert_eq!(left.transitions, right_before.transitions);
            assert_eq!(right.transitions, left_before.transitions);
            assert_eq!(left.responses, right_before.responses);
            assert_eq!(right.responses, left_before.responses);
        }
    }

    #[test]
    fn crossover_of_empty_genomes_is_a_noop() {
        let mut rng = StdRng::seed_from_u64(1);
        let mut left = SdaGenome::random_with_edge_multiplicity_cap(1, 0, 1, &mut rng).unwrap();
        let mut right = SdaGenome::random_with_edge_multiplicity_cap(1, 0, 1, &mut rng).unwrap();
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

    /// The point of `from_parts` and the accessors together: an automaton can
    /// leave one genome and arrive intact in another. Without both halves a
    /// run's winner cannot be fed back into a later run, which is the gap the
    /// edge-edit representation does not have.
    #[test]
    fn an_automaton_read_off_a_genome_rebuilds_an_identical_one() {
        let mut rng = StdRng::seed_from_u64(7);
        let original = SdaGenome::random_with_edge_multiplicity_cap(6, 1, 3, &mut rng).unwrap();

        let rebuilt = SdaGenome::from_parts(
            original.init_char(),
            original.transitions().to_vec(),
            original.responses().to_vec(),
            original.max_resp_len(),
        )
        .expect("an automaton taken from a valid genome is valid");

        assert_eq!(rebuilt, original);

        // And it is not merely field-equal — it expresses to the same graph.
        let context = express_context(8, 1);
        assert_eq!(rebuilt.express(&context), original.express(&context));
    }

    #[test]
    fn from_parts_accepts_a_hand_built_automaton() {
        let built = SdaGenome::from_parts(
            0,
            vec![vec![1, 0], vec![0, 1]],
            vec![vec![vec![0], vec![1]], vec![vec![1], vec![0]]],
            1,
        )
        .expect("the same automaton the struct-literal helper builds");

        assert_eq!(built, small_genome());
        assert_eq!(built.run(0, 3), vec![0, 0, 1]);
    }

    /// The one rejection that is not preventing a panic: `run` makes progress
    /// only by appending a response's characters, so reaching an empty one
    /// would loop until the process is killed. A test that let it through
    /// would hang the suite rather than fail it.
    #[test]
    fn from_parts_rejects_an_empty_response() {
        let error = SdaGenome::from_parts(0, vec![vec![0, 0]], vec![vec![vec![0], Vec::new()]], 1)
            .expect_err("an empty response never terminates");

        assert!(error.contains("at least one character"), "{error}");
    }

    #[test]
    fn from_parts_rejects_a_transition_to_a_state_that_does_not_exist() {
        let error = SdaGenome::from_parts(0, vec![vec![0, 4]], vec![vec![vec![0], vec![0]]], 1)
            .expect_err("state 4 does not exist in a one-state automaton");

        assert!(error.contains("target a state that exists"), "{error}");
    }

    #[test]
    fn from_parts_rejects_characters_outside_the_alphabet() {
        let bad_response =
            SdaGenome::from_parts(0, vec![vec![0, 0]], vec![vec![vec![9], vec![0]]], 1)
                .expect_err("9 is not a character in a two-character alphabet");
        assert!(
            bad_response.contains("response character"),
            "{bad_response}"
        );

        let bad_init = SdaGenome::from_parts(9, vec![vec![0, 0]], vec![vec![vec![0], vec![0]]], 1)
            .expect_err("the same, for the initial character");
        assert!(bad_init.contains("init_char"), "{bad_init}");
    }

    #[test]
    fn from_parts_rejects_tables_that_are_not_rectangular() {
        let ragged = SdaGenome::from_parts(
            0,
            vec![vec![0, 0], vec![0]],
            vec![vec![vec![0], vec![0]], vec![vec![0], vec![0]]],
            1,
        )
        .expect_err("the second transition row is one entry short");
        assert!(ragged.contains("num_chars wide"), "{ragged}");

        let mismatched = SdaGenome::from_parts(0, vec![vec![0, 0]], Vec::new(), 1)
            .expect_err("no response rows at all");
        assert!(
            mismatched.contains("as many rows as transitions"),
            "{mismatched}"
        );
    }

    #[test]
    fn from_parts_rejects_empty_dimensions() {
        assert!(SdaGenome::from_parts(0, Vec::new(), Vec::new(), 1).is_err());
        assert!(SdaGenome::from_parts(0, vec![Vec::new()], vec![Vec::new()], 1).is_err());
        assert!(
            SdaGenome::from_parts(0, vec![vec![0]], vec![vec![vec![0]]], 0).is_err(),
            "max_resp_len of 0 leaves a later mutation nothing to generate"
        );
    }
}
