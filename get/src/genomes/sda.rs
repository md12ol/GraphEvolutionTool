use rand::Rng;

use super::genome::{Genome, SdaContext};
use crate::graph::Graph;

/// Self-driving-automaton genome: a finite-state machine whose run emits the
/// characters that get folded into a graph's adjacency triangle.
#[derive(Clone, Debug, PartialEq)]
pub struct SdaGenome {
    init_char: u8,
    /// `[state][char] -> next state`
    transitions: Vec<Vec<u16>>,
    /// `[state][char] -> chars appended to the output buffer`
    responses: Vec<Vec<Vec<u8>>>,
    /// Maximum length of a freshly generated response, used by
    /// [`SdaGenome::random_with_edge_multiplicity_cap`] and [`Genome::mutate`]
    /// when a response is generated. Unlike `num_states`/`num_chars`, this
    /// isn't observable from the current data, so it has to be stored rather
    /// than derived.
    max_resp_len: usize,
}

/// Largest alphabet size representable by [`SdaGenome`]'s `u8`-valued responses.
const MAX_NUM_CHARS: usize = u8::MAX as usize + 1;
/// Largest state count representable by [`SdaGenome`]'s `u16`-valued transitions.
const MAX_NUM_STATES: usize = u16::MAX as usize + 1;
/// Default for [`SdaContext::init_char_mutation_rate`]: the chance per
/// [`Genome::mutate`] call of mutating the initial character instead of a
/// transition or response. The value a run uses is configurable; this is what
/// it falls back to.
pub const DEFAULT_INIT_CHAR_MUTATION_RATE: f64 = 0.04;
/// Default for [`SdaContext::transition_vs_response_rate`]: an even split between
/// redrawing a transition's target state and redrawing its response.
pub const DEFAULT_TRANSITION_VS_RESPONSE_RATE: f64 = 0.5;

/// Genome dimensions that have already been checked, so a caller building a
/// whole population validates once rather than once per individual.
///
/// Every constructor is a check, so holding a value of this type *is* the proof
/// that the three numbers are usable, and
/// [`SdaGenome::random_with_dimensions`] can be infallible rather than
/// returning a `Result` no caller past the first iteration can ever see fail.
///
/// From outside the crate the only route is
/// [`SdaDimensions::from_edge_multiplicity_cap`], which derives `num_chars`
/// from the cap. `new` takes it directly and is crate-internal: a caller
/// choosing its own alphabet builds a genome that disagrees with the context it
/// is expressed against, and `express` panics on that.
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
    ///
    /// **`num_chars` is derived, never chosen.** A caller picking its own value
    /// builds a genome that disagrees with its context, which `express` panics
    /// on — so this is the constructor a run uses, and the only one outside the
    /// crate. The direct `new` exists for the checks themselves, which need a
    /// route that does not go through a cap: `cap + 1` is never zero, so the
    /// empty-alphabet branch would otherwise be unreachable and untested.
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
    /// the alphabet is fixed at `edge_multiplicity_cap + 1` characters
    /// (`0..=cap`), so every character value doubles as a legal edge weight and
    /// nothing is clamped by [`Graph::set_edge`] when the same cap is used to
    /// build the [`SdaContext`] this genome is later expressed against.
    ///
    /// Each transition's response is a random length between 1 and
    /// `max_resp_len` characters, inclusive.
    ///
    /// **The constructor to reach for, and the alphabet is why.** The cap
    /// decides `num_chars`, so it is never a free choice here: a caller picking
    /// its own value builds a genome that disagrees with the context it is
    /// expressed against, which `express` panics on. Building one individual at
    /// a time, this is the route; for a whole population, validate once into an
    /// [`SdaDimensions`] and loop through
    /// [`SdaGenome::random_with_dimensions`] instead.
    ///
    /// Returns an error if the dimensions are zero or too large to fit the
    /// genome's storage types (`num_states` up to 65536, `num_chars` up to
    /// 256).
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
    /// rejected when the [`SdaDimensions`] was built, so a population loop
    /// validates once at the top instead of re-checking three constants on
    /// every individual.
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
    /// same cap always satisfies it; one assembled field-by-field in-module is
    /// not checked until expressed, so a mismatch there would otherwise
    /// silently bias the expressed graph toward the cap (alphabet too large)
    /// or leave the upper edge weights unreachable (alphabet too small).
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
    /// `0..=shared_length` and swap the half-open interior segment
    /// `[start, end)` between the parents, leaving states outside that window
    /// untouched on both sides. Swapping state 0 also swaps `init_char`,
    /// since together they determine the automaton's first transition.
    ///
    /// Crosses even when only one state is shared, where the single possible
    /// pair of cut points forces the segment to state 0 alone. That is still
    /// worth doing here because `init_char` moves with it, so two automata
    /// genuinely exchange their starting behaviour.
    /// `EdgeEditGenome::crossover` declines at the same length, its genes
    /// having no equivalent passenger to carry.
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

    /// Apply one mutation: redraw the initial character with probability
    /// `context.init_char_mutation_rate`, otherwise redraw one transition's
    /// target state with probability `context.transition_vs_response_rate` and
    /// its response with the remainder. Callers that want more disruption per
    /// generation call this multiple times.
    ///
    /// The second draw was a plain coin flip before the rates were
    /// configurable. `random_bool(0.5)` and `random::<bool>()` do not consume
    /// the same RNG state, so a seeded run does not reproduce output from
    /// before this change even at the default rates.
    fn mutate<R: Rng + ?Sized>(&mut self, context: &Self::Context, rng: &mut R) {
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
        // small_genome has a 2-char alphabet, so the matching cap is 1 (§3.2).
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

        // small_genome has a 2-char alphabet, so the matching cap is 1 (§3.2).
        for num_nodes in [0, 1] {
            let context = express_context(num_nodes, 1);
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
        let context = express_context(2, 5);

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
}
