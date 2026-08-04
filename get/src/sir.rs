//! One SIR epidemic over an expressed graph.
//!
//! Ported from `Graph::SIR` in `legacy/Graph.cpp`, which is the model this
//! project has always simulated. That source is tracked and readable alongside
//! this file; `legacy/README.md` says what it is and where the Rust departs
//! from it.
//!
//! The mechanics are unchanged from the port: an adjacency scan accumulates
//! each susceptible node's total exposure, and one combined Bernoulli draw per
//! node decides infection. Only the reporting differs — see [`SirRun`] — and
//! the RNG is passed in rather than taken from a global, which is what lets one
//! seed drive a whole batch (spec §5.2).
//!
//! The model is SIR with a **one-timestep infectious period**. A node infected
//! during a step spends the *following* step infectious, transmitting to each
//! still-susceptible neighbour with probability `infection_rate` per edge, then
//! recovers and never infects again. A single patient zero seeds the outbreak,
//! which runs until no infected nodes remain.

use rand::Rng;

use crate::graph::Graph;

/// Parameters of one epidemic.
///
/// This is deliberately a plain struct rather than the config type: wiring the
/// simulator to `[fitness]` is a separate piece of work, and `sir_sim` has no
/// business depending on the config schema.
#[derive(Clone, Debug, PartialEq)]
pub struct SirParams {
    /// Per-contact probability of transmission along one edge in one timestep.
    pub infection_rate: f64,
    /// Which node seeds the outbreak; `None` draws a fresh one per epidemic.
    pub patient_zero: Option<usize>,
}

/// Everything the three SIR objectives read from one epidemic.
///
/// The epidemic is the expensive part and all three objectives want the same
/// one, so a single run reports all three readings (spec §5.2).
///
/// The three are consistent by construction: `profile[0]` is patient zero, so
/// `spread` is the sum of the profile and `length` is one less than its length.
/// An outbreak that infects nobody beyond patient zero has `length == 0` and
/// `spread == 1`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SirRun {
    /// Timesteps to burn out — that is, transmission steps, not counting the
    /// final step in which the last infectious node simply recovers.
    pub length: usize,
    /// Total ever-infected, including patient zero.
    pub spread: usize,
    /// Count of **newly infected** nodes at each timestep, `profile[0] == 1`
    /// being patient zero. Carries no trailing zero.
    pub profile: Vec<usize>,
}

/// One node's position in the epidemic.
///
/// `JustInfected` is the staging state the reference implementation uses to
/// keep a step's transmissions simultaneous: a node infected during a step must
/// not transmit until the following one, so it is held here until every
/// susceptible node has been resolved.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum State {
    Susceptible,
    Infectious,
    Removed,
    JustInfected,
}

/// Run one epidemic to completion.
///
/// `patient_zero` is assumed to be a valid node; a config-driven run has
/// already checked that, and an out-of-range value here yields an outbreak that
/// infects nobody rather than a panic.
pub fn sir_sim<R: Rng + ?Sized>(graph: &Graph, params: &SirParams, rng: &mut R) -> SirRun {
    let num_nodes = graph.num_nodes;
    if num_nodes == 0 {
        return SirRun {
            length: 0,
            spread: 0,
            profile: Vec::new(),
        };
    }

    let patient_zero = match params.patient_zero {
        Some(node) => node,
        None => rng.random_range(0..num_nodes),
    };
    if patient_zero >= num_nodes {
        return SirRun {
            length: 0,
            spread: 0,
            profile: Vec::new(),
        };
    }

    let mut state = vec![State::Susceptible; num_nodes];
    state[patient_zero] = State::Infectious;

    let mut profile = vec![1usize];
    let mut exposure = vec![0u32; num_nodes];
    let mut currently_infectious = 1;

    while currently_infectious > 0 {
        // Total edge copies connecting each node to an infectious one. A
        // multiplicity of `k` contributes `k`, because parallel edges are `k`
        // independent chances to transmit, not one.
        exposure.fill(0);
        for (node, node_state) in state.iter().enumerate() {
            if *node_state != State::Infectious {
                continue;
            }
            for (neighbor, count) in exposure.iter_mut().enumerate() {
                if neighbor != node {
                    *count += graph.weight(node, neighbor);
                }
            }
        }

        for node in 0..num_nodes {
            if state[node] == State::Susceptible
                && exposure[node] > 0
                && transmits(exposure[node], params.infection_rate, rng)
            {
                state[node] = State::JustInfected;
            }
        }

        // Advance every node at once: this step's infectious nodes recover, and
        // this step's new infections become next step's infectious set.
        currently_infectious = 0;
        for node_state in &mut state {
            match *node_state {
                State::Infectious => *node_state = State::Removed,
                State::JustInfected => {
                    *node_state = State::Infectious;
                    currently_infectious += 1;
                }
                State::Susceptible | State::Removed => {}
            }
        }

        // The final pass carries no new infections; recording its zero would
        // pad every profile with a trailing entry and put `length` one step
        // past the last transmission.
        if currently_infectious > 0 {
            profile.push(currently_infectious);
        }
    }

    SirRun {
        length: profile.len() - 1,
        spread: profile.iter().sum(),
        profile,
    }
}

/// Whether a node with this total exposure is infected during one timestep.
///
/// `exposure` independent chances at `infection_rate` each, resolved as one
/// draw against `1 - (1 - rate)^exposure`. The reference implementation writes
/// this as `1 - exp(n * ln(1 - alpha))`, which is the same quantity; the direct
/// form avoids the `ln(0)` that an `infection_rate` of exactly 1.0 produces.
fn transmits<R: Rng + ?Sized>(exposure: u32, infection_rate: f64, rng: &mut R) -> bool {
    let escape = (1.0 - infection_rate).powi(exposure as i32);
    rng.random::<f64>() < 1.0 - escape
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    fn rng() -> ChaCha8Rng {
        ChaCha8Rng::seed_from_u64(20260804)
    }

    /// A path `0 - 1 - ... - (n-1)`, every edge at multiplicity 1.
    fn path_graph(num_nodes: usize) -> Graph {
        let mut graph = Graph::new(num_nodes, 1);
        for node in 0..num_nodes.saturating_sub(1) {
            graph.set_edge(node, node + 1, 1);
        }
        graph
    }

    #[test]
    fn a_certain_epidemic_burns_along_a_path_one_node_per_step() {
        let graph = path_graph(6);
        let params = SirParams {
            infection_rate: 1.0,
            patient_zero: Some(0),
        };

        let run = sir_sim(&graph, &params, &mut rng());

        assert_eq!(run.profile, vec![1, 1, 1, 1, 1, 1]);
        assert_eq!(run.length, 5, "one step per edge of the path");
        assert_eq!(run.spread, 6, "every node is reached");
    }

    #[test]
    fn an_isolated_patient_zero_infects_nobody() {
        let graph = Graph::new(4, 1);
        let params = SirParams {
            infection_rate: 1.0,
            patient_zero: Some(2),
        };

        let run = sir_sim(&graph, &params, &mut rng());

        assert_eq!(run.length, 0, "spec 5.2: no transmission is length 0");
        assert_eq!(run.spread, 1, "patient zero alone");
        assert_eq!(run.profile, vec![1]);
    }

    #[test]
    fn a_zero_infection_rate_never_transmits() {
        let graph = path_graph(5);
        let params = SirParams {
            infection_rate: 0.0,
            patient_zero: Some(0),
        };

        let run = sir_sim(&graph, &params, &mut rng());

        assert_eq!(run.length, 0);
        assert_eq!(run.spread, 1);
        assert_eq!(run.profile, vec![1]);
    }

    #[test]
    fn spread_is_capped_by_the_component_holding_patient_zero() {
        // Two disjoint triangles: 0-1-2 and 3-4-5.
        let mut graph = Graph::new(6, 1);
        for (u, v) in [(0, 1), (1, 2), (0, 2), (3, 4), (4, 5), (3, 5)] {
            graph.set_edge(u, v, 1);
        }
        let params = SirParams {
            infection_rate: 1.0,
            patient_zero: Some(0),
        };

        let run = sir_sim(&graph, &params, &mut rng());

        assert_eq!(run.spread, 3, "the far triangle is unreachable");
        assert_eq!(run.profile, vec![1, 2], "both neighbours infected at once");
        assert_eq!(run.length, 1);
    }

    #[test]
    fn parallel_edges_are_independent_chances_to_transmit() {
        // A single edge, at multiplicity 1 versus multiplicity 4. With a
        // per-contact rate of 0.3 the escape probabilities are 0.7 and
        // 0.7^4 = 0.2401, so the heavier edge should transmit far more often.
        let single = {
            let mut graph = Graph::new(2, 4);
            graph.set_edge(0, 1, 1);
            graph
        };
        let quadruple = {
            let mut graph = Graph::new(2, 4);
            graph.set_edge(0, 1, 4);
            graph
        };
        let params = SirParams {
            infection_rate: 0.3,
            patient_zero: Some(0),
        };

        let mut rng = rng();
        let trials = 2000;
        let count = |graph: &Graph, rng: &mut ChaCha8Rng| {
            (0..trials)
                .filter(|_| sir_sim(graph, &params, rng).spread == 2)
                .count()
        };

        let single_hits = count(&single, &mut rng);
        let quadruple_hits = count(&quadruple, &mut rng);

        assert!(
            quadruple_hits > single_hits,
            "multiplicity 4 transmitted {quadruple_hits}/{trials}, \
             multiplicity 1 transmitted {single_hits}/{trials}"
        );
        // Loose bands around the analytic 0.30 and 0.76; wide enough that a
        // seed change cannot fail them, tight enough to catch a weight that is
        // ignored or applied once.
        assert!((500..=700).contains(&single_hits), "{single_hits}");
        assert!((1420..=1620).contains(&quadruple_hits), "{quadruple_hits}");
    }

    #[test]
    fn an_unset_patient_zero_draws_a_fresh_node_per_epidemic() {
        // Six components of one node each, so `spread` is always 1 and the only
        // thing that varies between epidemics is which node was drawn. Every
        // node is reachable as patient zero over enough draws.
        let graph = Graph::new(6, 1);
        let params = SirParams {
            infection_rate: 1.0,
            patient_zero: None,
        };

        let mut rng = rng();
        for _ in 0..200 {
            let run = sir_sim(&graph, &params, &mut rng);
            assert_eq!(run.spread, 1);
        }

        // The draw consumes RNG state, so two epidemics from one generator are
        // not forced to agree, while the same seed replays identically.
        let mut first = ChaCha8Rng::seed_from_u64(7);
        let mut second = ChaCha8Rng::seed_from_u64(7);
        let path = path_graph(4);
        let runs_a: Vec<_> = (0..8)
            .map(|_| sir_sim(&path, &params, &mut first))
            .collect();
        let runs_b: Vec<_> = (0..8)
            .map(|_| sir_sim(&path, &params, &mut second))
            .collect();
        assert_eq!(runs_a, runs_b, "one seed replays the same epidemics");
        // At rate 1.0 the whole path is always reached, so `spread` cannot
        // distinguish the draws — `length` is what varies, since it measures
        // the distance from patient zero to the far end.
        assert!(
            runs_a.iter().any(|run| run.length != runs_a[0].length),
            "a fresh patient zero should not give identical outbreaks"
        );
    }

    #[test]
    fn an_empty_graph_produces_no_epidemic() {
        let graph = Graph::new(0, 1);
        let params = SirParams {
            infection_rate: 1.0,
            patient_zero: None,
        };

        let run = sir_sim(&graph, &params, &mut rng());

        assert_eq!(run.length, 0);
        assert_eq!(run.spread, 0);
        assert!(run.profile.is_empty());
    }
}
