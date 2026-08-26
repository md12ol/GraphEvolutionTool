//! One SIR epidemic over an expressed graph.
//!
//! The infectious period is **one timestep**: a node infected during a step
//! spends the *following* step infectious, transmitting to each
//! still-susceptible neighbour with probability `infection_rate` per edge copy,
//! then recovers and never infects again. A single patient zero seeds the
//! outbreak, which runs until no infected nodes remain.

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::graph::Graph;

/// Parameters of one epidemic.
#[derive(Clone, Debug, PartialEq)]
pub struct SirParams {
    /// Per-contact probability of transmission along one edge in one timestep.
    /// Outside `0.0..=1.0` the per-node draw is silently not a probability.
    pub infection_rate: f64,
    /// Which node seeds the outbreak; `None` draws a fresh one per epidemic.
    pub patient_zero: Option<usize>,
}

/// How one graph's epidemics are sampled.
///
/// A graph's score averages over `num_epidemics` outbreaks, re-rolling any that
/// come out shorter than `min_epidemic_length`.
///
/// **That re-roll is a biased resample, not variance reduction.** It pushes
/// expected fitness up by an amount that depends on how often a graph fizzles,
/// so it is not a substitute for raising `num_epidemics`.
#[derive(Clone, Debug, PartialEq)]
pub struct SirSampleParams {
    /// The epidemic itself — rate and patient zero.
    pub epidemic: SirParams,
    /// How many outbreaks one graph's score averages over. At least 1.
    pub num_epidemics: usize,
    /// Outbreaks shorter than this are re-rolled. **Set to 1 to disable the
    /// re-roll**, since every epidemic has `length >= 1`.
    pub min_epidemic_length: usize,
    /// Attempts before keeping whatever came out. At least 1.
    pub max_epidemic_retries: usize,
}

/// The seeds one batch draws its epidemics from.
///
/// Epidemic `i` attempt `a` uses `seeds[i * max_epidemic_retries + a]`. Two
/// properties depend on that fixed position, and both break quietly:
///
/// - **Every graph in a batch draws from this same pool**, so fitness
///   differences reflect the graph rather than the dice. A retry only changes
///   *which* of the shared draws a graph stops on.
/// - **Extending appends.** Raising `num_epidemics` leaves earlier epidemics
///   replaying unchanged.
///
/// Both break under sequential drawing, where a retrying graph consumes extra
/// draws and shifts every later epidemic, and under `xor`, where nearby batch
/// seeds collide.
pub(crate) fn epidemic_seeds(
    batch_seed: u64,
    num_epidemics: usize,
    max_epidemic_retries: usize,
) -> Vec<u64> {
    let mut stream = ChaCha8Rng::seed_from_u64(batch_seed);
    let mut seeds = Vec::with_capacity(num_epidemics * max_epidemic_retries);

    for _ in 0..num_epidemics * max_epidemic_retries {
        seeds.push(stream.random::<u64>());
    }
    seeds
}

/// Run one graph's epidemics, re-rolling short ones.
///
/// # Panics
///
/// If `num_epidemics` or `max_epidemic_retries` is zero. With no epidemics at
/// all the objective would average nothing, producing the `NaN` the `Fitness`
/// contract forbids; config validation rejects both at load.
pub fn simulate_epidemics(
    graph: &Graph,
    params: &SirSampleParams,
    batch_seed: u64,
) -> Vec<Epidemic> {
    assert!(params.num_epidemics > 0, "num_epidemics must be at least 1",);
    assert!(
        params.max_epidemic_retries > 0,
        "max_epidemic_retries must be at least 1",
    );

    let seeds = epidemic_seeds(
        batch_seed,
        params.num_epidemics,
        params.max_epidemic_retries,
    );
    let mut epidemics = Vec::with_capacity(params.num_epidemics);

    for epidemic in 0..params.num_epidemics {
        let mut kept = None;

        for attempt in 0..params.max_epidemic_retries {
            let seed = seeds[epidemic * params.max_epidemic_retries + attempt];
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            let candidate = sir_sim(graph, &params.epidemic, &mut rng);
            let long_enough = candidate.length >= params.min_epidemic_length;

            // Keep every attempt, so the last one survives if all were short.
            kept = Some(candidate);
            if long_enough {
                break;
            }
        }

        epidemics.push(kept.expect("at least one attempt always runs"));
    }

    epidemics
}

/// The readings one epidemic reports.
///
/// Consistent by construction: `spread` is the sum of `profile`, and `length`
/// is one less than its length. An outbreak that infects nobody beyond patient
/// zero has `length == 1`, `spread == 1` and `profile == [1, 0]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Epidemic {
    /// Timesteps the epidemic occupied, **including** the final one in which
    /// the last infectious node recovers without transmitting.
    pub length: usize,
    /// Total ever-infected, including patient zero.
    pub spread: usize,
    /// Count of **newly infected** nodes at each timestep. `profile[0] == 1` is
    /// patient zero, and the last element is the terminating zero.
    pub profile: Vec<usize>,
}

/// One node's position in the epidemic.
///
/// `JustInfected` is the staging state that keeps a step's transmissions
/// simultaneous: a node infected during a step must not transmit until the
/// following one, so it is held here until every susceptible node has been
/// resolved.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum State {
    Susceptible,
    Infectious,
    Removed,
    JustInfected,
}

/// Run one epidemic to completion.
///
/// A `patient_zero` outside the graph yields an outbreak that infects nobody
/// rather than a panic.
///
/// **A graph with no nodes returns `length == 0`, deliberately — do not "fix"
/// it to `1`.** Every real epidemic has `length >= 1`, so `0` means *no
/// epidemic existed to measure*, not *nobody was infected*.
pub fn sir_sim<R: Rng + ?Sized>(graph: &Graph, params: &SirParams, rng: &mut R) -> Epidemic {
    let num_nodes = graph.num_nodes;
    if num_nodes == 0 {
        return Epidemic {
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
        return Epidemic {
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

        // Advance every node at once, so a node infected during this step does
        // not transmit until the next.
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

        // The final pass carries no new infections, and its zero is recorded
        // rather than suppressed: `length` counts that burnout step and
        // `profile` terminates with it.
        profile.push(currently_infectious);
    }

    Epidemic {
        length: profile.len() - 1,
        spread: profile.iter().sum(),
        profile,
    }
}

/// Whether a node with this total exposure is infected during one timestep.
///
/// `exposure` independent chances at `infection_rate` each, resolved as one
/// draw against `1 - (1 - rate)^exposure`.
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

        let epidemic = sir_sim(&graph, &params, &mut rng());

        assert_eq!(epidemic.profile, vec![1, 1, 1, 1, 1, 1, 0]);
        assert_eq!(
            epidemic.length, 6,
            "one step per edge of the path, plus the burnout step",
        );
        assert_eq!(epidemic.spread, 6, "every node is reached");
    }

    #[test]
    fn an_isolated_patient_zero_infects_nobody() {
        let graph = Graph::new(4, 1);
        let params = SirParams {
            infection_rate: 1.0,
            patient_zero: Some(2),
        };

        let epidemic = sir_sim(&graph, &params, &mut rng());

        assert_eq!(
            epidemic.length, 1,
            "spec 5.2: the burnout step counts, so no transmission is length 1",
        );
        assert_eq!(epidemic.spread, 1, "patient zero alone");
        assert_eq!(epidemic.profile, vec![1, 0]);
    }

    #[test]
    fn a_zero_infection_rate_never_transmits() {
        let graph = path_graph(5);
        let params = SirParams {
            infection_rate: 0.0,
            patient_zero: Some(0),
        };

        let epidemic = sir_sim(&graph, &params, &mut rng());

        assert_eq!(epidemic.length, 1, "same shape as an isolated patient zero");
        assert_eq!(epidemic.spread, 1);
        assert_eq!(epidemic.profile, vec![1, 0]);
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

        let epidemic = sir_sim(&graph, &params, &mut rng());

        assert_eq!(epidemic.spread, 3, "the far triangle is unreachable");
        assert_eq!(
            epidemic.profile,
            vec![1, 2, 0],
            "both neighbours infected at once, then burnout",
        );
        assert_eq!(epidemic.length, 2);
    }

    #[test]
    fn exposure_accumulates_across_every_infectious_neighbour() {
        // A square: patient zero 0 reaches 3 only through 1 and 2, so when
        // both are infectious in the same timestep node 3 faces two
        // independent chances, not one. Every other test in this module gives
        // a susceptible node at most one infectious neighbour, so an exposure
        // that overwrites instead of accumulating scores identically in all
        // of them — the profile stays the right length and the curve is wrong.
        let mut graph = Graph::new(4, 1);
        for (u, v) in [(0, 1), (0, 2), (1, 3), (2, 3)] {
            graph.set_edge(u, v, 1);
        }
        let params = SirParams {
            infection_rate: 0.5,
            patient_zero: Some(0),
        };

        let mut rng = rng();
        let trials = 2000;
        let mut reached_all_four = 0;
        for _ in 0..trials {
            if sir_sim(&graph, &params, &mut rng).spread == 4 {
                reached_all_four += 1;
            }
        }

        // Every route to all four nodes, at rate 0.5:
        //   both 1 and 2 infected (0.25), then 3 against exposure 2 (0.75)
        //     -> 0.1875
        //   only 1 infected (0.25), then 3 (0.5), then 3 infects 2 the long
        //     way round (0.5) -> 0.0625, and the same again for only 2
        // which is 0.3125, or about 625 of 2000. Seeing one neighbour instead
        // of both makes the first route's second factor 0.5, for 0.25 overall
        // and about 500 — below this band.
        assert!(
            (555..=695).contains(&reached_all_four),
            "{reached_all_four} of {trials} reached every node"
        );
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
            let epidemic = sir_sim(&graph, &params, &mut rng);
            assert_eq!(epidemic.spread, 1);
        }

        // The draw consumes RNG state, so two epidemics from one generator are
        // not forced to agree, while the same seed replays identically.
        let mut first = ChaCha8Rng::seed_from_u64(7);
        let mut second = ChaCha8Rng::seed_from_u64(7);
        let path = path_graph(4);
        let epidemics_a: Vec<_> = (0..8)
            .map(|_| sir_sim(&path, &params, &mut first))
            .collect();
        let epidemics_b: Vec<_> = (0..8)
            .map(|_| sir_sim(&path, &params, &mut second))
            .collect();
        assert_eq!(
            epidemics_a, epidemics_b,
            "one seed replays the same epidemics"
        );
        // At rate 1.0 the whole path is always reached, so `spread` cannot
        // distinguish the draws — `length` is what varies, since it measures
        // the distance from patient zero to the far end.
        assert!(
            epidemics_a
                .iter()
                .any(|epidemic| epidemic.length != epidemics_a[0].length),
            "a fresh patient zero should not give identical outbreaks"
        );
    }

    // --- The epidemic runner: position-indexed seeding and the re-roll ------

    /// Two connected nodes at rate 0.5: an epidemic is `length == 2` when it
    /// transmits and `length == 1` when it does not. That split is what lets
    /// these tests tell *which* attempt was kept.
    fn coin_flip_sample(
        num_epidemics: usize,
        min_len: usize,
        retries: usize,
    ) -> (Graph, SirSampleParams) {
        let mut graph = Graph::new(2, 1);
        graph.set_edge(0, 1, 1);
        let params = SirSampleParams {
            epidemic: SirParams {
                infection_rate: 0.5,
                patient_zero: Some(0),
            },
            num_epidemics,
            min_epidemic_length: min_len,
            max_epidemic_retries: retries,
        };
        (graph, params)
    }

    /// Where epidemic `i` attempt `a` sits in the seed pool.
    fn slot(params: &SirSampleParams, epidemic: usize, attempt: usize) -> usize {
        epidemic * params.max_epidemic_retries + attempt
    }

    /// The epidemic one pool seed produces — what `simulate_epidemics` must be
    /// shown to have used.
    fn epidemic_from_seed(graph: &Graph, params: &SirSampleParams, seed: u64) -> Epidemic {
        sir_sim(
            graph,
            &params.epidemic,
            &mut ChaCha8Rng::seed_from_u64(seed),
        )
    }

    #[test]
    fn extending_the_seed_pool_leaves_the_earlier_epidemics_untouched() {
        let short = epidemic_seeds(99, 30, 5);
        let long = epidemic_seeds(99, 50, 5);

        assert_eq!(
            short,
            long[..short.len()],
            "more epidemics must append, not resequence — spec 8.1",
        );
    }

    #[test]
    fn each_epidemic_takes_its_own_position_in_the_pool() {
        let (graph, params) = coin_flip_sample(4, 1, 5);
        let seeds = epidemic_seeds(2026, params.num_epidemics, params.max_epidemic_retries);

        let epidemics = simulate_epidemics(&graph, &params, 2026);

        assert_eq!(epidemics.len(), 4, "one epidemic per slot");
        for (index, epidemic) in epidemics.iter().enumerate() {
            let expected = epidemic_from_seed(&graph, &params, seeds[slot(&params, index, 0)]);
            assert_eq!(*epidemic, expected, "epidemic {index} used the wrong draw");
        }
    }

    #[test]
    fn two_graphs_under_one_batch_seed_face_an_identical_pool() {
        // Different graphs, so the outcomes differ; the dice must not.
        let (pair, params) = coin_flip_sample(6, 1, 5);
        let path = path_graph(5);
        let seeds = epidemic_seeds(4242, params.num_epidemics, params.max_epidemic_retries);

        let pair_epidemics = simulate_epidemics(&pair, &params, 4242);
        let path_epidemics = simulate_epidemics(&path, &params, 4242);

        for index in 0..params.num_epidemics {
            let seed = seeds[slot(&params, index, 0)];
            assert_eq!(
                pair_epidemics[index],
                epidemic_from_seed(&pair, &params, seed)
            );
            assert_eq!(
                path_epidemics[index],
                epidemic_from_seed(&path, &params, seed)
            );
        }
        assert_ne!(
            pair_epidemics, path_epidemics,
            "common dice, but the graphs should still score differently",
        );
    }

    #[test]
    fn the_same_batch_seed_replays_the_same_epidemics() {
        let (graph, params) = coin_flip_sample(8, 3, 4);

        assert_eq!(
            simulate_epidemics(&graph, &params, 7),
            simulate_epidemics(&graph, &params, 7),
        );
        assert_ne!(
            simulate_epidemics(&graph, &params, 7),
            simulate_epidemics(&graph, &params, 8),
            "a different batch seed is a different set of dice",
        );
    }

    #[test]
    fn a_min_length_of_one_never_rerolls() {
        let (graph, params) = coin_flip_sample(20, 1, 5);
        let seeds = epidemic_seeds(11, params.num_epidemics, params.max_epidemic_retries);

        let epidemics = simulate_epidemics(&graph, &params, 11);

        for (index, epidemic) in epidemics.iter().enumerate() {
            let first_attempt = epidemic_from_seed(&graph, &params, seeds[slot(&params, index, 0)]);
            assert_eq!(*epidemic, first_attempt);
        }
        assert!(
            epidemics.iter().any(|epidemic| epidemic.length == 1),
            "vacuous unless some epidemic was short enough to reject",
        );
    }

    #[test]
    fn an_unreachable_min_length_exhausts_the_retries_and_keeps_the_last() {
        // `length` is at most 2 here, so nothing ever satisfies 3 and every
        // epidemic runs all five attempts. The kept epidemic must be the last.
        let (graph, params) = coin_flip_sample(12, 3, 5);
        let seeds = epidemic_seeds(5150, params.num_epidemics, params.max_epidemic_retries);

        let epidemics = simulate_epidemics(&graph, &params, 5150);

        for (index, epidemic) in epidemics.iter().enumerate() {
            let last = slot(&params, index, params.max_epidemic_retries - 1);
            let final_attempt = epidemic_from_seed(&graph, &params, seeds[last]);
            assert_eq!(
                *epidemic, final_attempt,
                "epidemic {index} kept the wrong attempt"
            );
        }
        assert!(
            epidemics
                .iter()
                .any(|epidemic| epidemic.length < params.min_epidemic_length),
            "a short epidemic must be kept rather than looped on forever",
        );
    }

    #[test]
    fn the_reroll_stops_on_the_first_long_enough_attempt() {
        let (graph, params) = coin_flip_sample(12, 2, 5);
        let seeds = epidemic_seeds(31337, params.num_epidemics, params.max_epidemic_retries);

        let epidemics = simulate_epidemics(&graph, &params, 31337);

        let mut stopped_early = 0;
        for (index, epidemic) in epidemics.iter().enumerate() {
            // Replay every attempt this epidemic could have made.
            let mut attempts = Vec::new();
            for attempt in 0..params.max_epidemic_retries {
                let seed = seeds[slot(&params, index, attempt)];
                attempts.push(epidemic_from_seed(&graph, &params, seed));
            }

            // It should have kept the first long-enough one, or the last.
            let first_ok = attempts
                .iter()
                .position(|candidate| candidate.length >= params.min_epidemic_length);
            let expected = first_ok.unwrap_or(params.max_epidemic_retries - 1);

            assert_eq!(*epidemic, attempts[expected], "epidemic {index}");
            if first_ok == Some(0) {
                stopped_early += 1;
            }
        }
        assert!(
            stopped_early > 0,
            "at rate 0.5 some epidemic should have passed on its first attempt",
        );
    }

    #[test]
    #[should_panic(expected = "num_epidemics must be at least 1")]
    fn no_epidemics_at_all_is_rejected_rather_than_averaged() {
        let (graph, params) = coin_flip_sample(0, 1, 5);
        simulate_epidemics(&graph, &params, 1);
    }

    #[test]
    #[should_panic(expected = "max_epidemic_retries must be at least 1")]
    fn no_attempts_at_all_is_rejected() {
        let (graph, params) = coin_flip_sample(4, 1, 0);
        simulate_epidemics(&graph, &params, 1);
    }

    /// `sir_sim` is public and documents that a stray patient zero is an
    /// outbreak that infects nobody rather than a panic. A config-driven run
    /// has validated the index long before this, so the guard inside is the
    /// only thing standing between a direct caller and `state[patient_zero]`.
    #[test]
    fn an_out_of_range_patient_zero_yields_no_epidemic_rather_than_panicking() {
        let graph = path_graph(4);
        let params = SirParams {
            infection_rate: 1.0,
            patient_zero: Some(9),
        };

        let epidemic = sir_sim(&graph, &params, &mut rng());

        assert_eq!(epidemic.length, 0);
        assert_eq!(epidemic.spread, 0);
        assert!(epidemic.profile.is_empty());
    }

    /// Zero here means *no epidemic existed*, not *nobody was infected* — since
    /// the §5.2 amendment a lone patient zero is `length == 1`, so only a
    /// nodeless graph can produce `0`. Deliberate; see `sir_sim`'s doc comment.
    #[test]
    fn an_empty_graph_produces_no_epidemic() {
        let graph = Graph::new(0, 1);
        let params = SirParams {
            infection_rate: 1.0,
            patient_zero: None,
        };

        let epidemic = sir_sim(&graph, &params, &mut rng());

        assert_eq!(epidemic.length, 0);
        assert_eq!(epidemic.spread, 0);
        assert!(epidemic.profile.is_empty());
    }
}
