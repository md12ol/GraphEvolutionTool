//! One SIR epidemic over an expressed graph.
//!
//! Ported from `Graph::SIR` in `legacy/Graph.cpp`, which is the model this
//! project has always simulated. That source is tracked and readable alongside
//! this file; `legacy/README.md` says what it is and how the two now line up.
//!
//! The mechanics are unchanged from the port: an adjacency scan accumulates
//! each susceptible node's total exposure, and one combined Bernoulli draw per
//! node decides infection. **The reporting matches it too** — `length` counts
//! the burnout step and `profile` carries a terminating zero, as of the §5.2
//! amendment of 2026-08-04. Two things are genuinely ours: the draw is written
//! `1 - (1 - rate)^k` rather than `1 - exp(k · ln(1 - alpha))`, which avoids a
//! `ln(0)` at `infection_rate = 1.0`, and the RNG is passed in rather than
//! taken from a global, which is what lets one seed drive a whole batch of graphs.
//!
//! The model is SIR with a **one-timestep infectious period**. A node infected
//! during a step spends the *following* step infectious, transmitting to each
//! still-susceptible neighbour with probability `infection_rate` per edge, then
//! recovers and never infects again. A single patient zero seeds the outbreak,
//! which runs until no infected nodes remain.

// Both traits must be in scope to call their methods (.random(),
// .seed_from_u64()) below — Rust requires the trait import even when the
// concrete type (ChaCha8Rng) is already imported.
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

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

/// How one graph's epidemics are sampled (spec §5.2).
///
/// A graph's score averages over `num_epidemics` outbreaks, and re-rolls ones
/// that fizzle — a short outbreak says the dice went badly, not that the
/// network is poor.
///
/// **The re-roll is a biased resample, not variance reduction.** It pushes
/// expected fitness up by an amount that depends on how often a graph fizzles,
/// so it is not a substitute for raising `num_epidemics`. Kept for
/// comparability with the archived C++ results.
#[derive(Clone, Debug, PartialEq)]
pub struct SirSampleParams {
    /// The epidemic itself — rate and patient zero.
    pub epidemic: SirParams,
    /// How many outbreaks one graph's score averages over. At least 1.
    pub num_epidemics: usize,
    /// Outbreaks shorter than this are re-rolled. C++ default 3; **set to 1 to
    /// disable the re-roll**, since every epidemic has `length >= 1`.
    pub min_epidemic_length: usize,
    /// Attempts before keeping whatever came out. C++ default 5. At least 1.
    pub max_epidemic_retries: usize,
}

/// The seeds one batch draws its epidemics from.
///
/// Epidemic `i` attempt `a` uses `seeds[i * max_epidemic_retries + a]` — the
/// same scheme §8.1 uses for replicates, applied to the epidemic index. Do not
/// replace it with `xor`; nearby batch seeds would then collide.
///
/// Two properties depend on the fixed position, and both break quietly:
///
/// - **Every graph in a batch draws from this same pool**, so fitness
///   differences reflect the graph rather than the dice. A retry only changes
///   *which* of the shared draws a graph stops on.
/// - **Extending appends.** Raising `num_epidemics` leaves earlier epidemics
///   replaying unchanged.
///
/// Drawing sequentially instead would break both: a graph that retries consumes
/// extra draws, shifting every later epidemic out of step with graphs that did
/// not retry.
pub fn epidemic_seeds(
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
/// This is what an objective calls; each of the three SIR objectives is just a
/// reading over the epidemics it returns.
///
/// The epidemics run **sequentially** (§5.2) — parallelism comes from the
/// replicate and graph-batch levels above.
///
/// # Panics
///
/// If `num_epidemics` or `max_epidemic_retries` is zero. Config validation
/// rejects both (§7), and no epidemics at all would leave the objective averaging
/// nothing, producing the `NaN` the `Fitness` contract forbids.
pub fn simulate_epidemics(graph: &Graph, params: &SirSampleParams, batch_seed: u64) -> Vec<SirRun> {
    assert!(
        params.num_epidemics > 0,
        "num_epidemics must be at least 1; spec 7 validates this at config load",
    );
    assert!(
        params.max_epidemic_retries > 0,
        "max_epidemic_retries must be at least 1; spec 7 validates this at config load",
    );

    let seeds = epidemic_seeds(
        batch_seed,
        params.num_epidemics,
        params.max_epidemic_retries,
    );
    let mut epidemics = Vec::with_capacity(params.num_epidemics);

    for epidemic in 0..params.num_epidemics {
        // Starts empty and is filled on every attempt; the .expect() below is
        // safe because the inner loop always runs at least once
        // (max_epidemic_retries > 0 is asserted above).
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

/// Everything the three SIR objectives read from one epidemic.
///
/// The epidemic is the expensive part and all three objectives want the same
/// one, so a single run reports all three readings (spec §5.2).
///
/// The three are consistent by construction: `profile[0]` is patient zero and
/// the profile ends in a terminating zero, so `spread` is the sum of the
/// profile — the zero contributes nothing — and `length` is one less than its
/// length. An outbreak that infects nobody beyond patient zero has
/// `length == 1`, `spread == 1` and `profile == [1, 0]`.
///
/// These conventions match `legacy/Graph.cpp` deliberately, so scores stay
/// comparable with the archived C++ results. See `decisions.md` 2026-08-04
/// 17:40; the sheet previously specified the other convention.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SirRun {
    /// Timesteps the epidemic occupied, **including** the final one in which
    /// the last infectious node recovers without transmitting.
    pub length: usize,
    /// Total ever-infected, including patient zero. Unaffected by the trailing
    /// zero, and the one reading the C++ and the sheet never disagreed on.
    pub spread: usize,
    /// Count of **newly infected** nodes at each timestep. `profile[0] == 1` is
    /// patient zero, and the last element is the terminating zero.
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
///
/// **A graph with no nodes returns `length == 0` with an empty profile, and
/// that is deliberate — do not "fix" it to `1` for consistency.** Since the
/// §5.2 amendment every real epidemic has `length >= 1`, because a lone patient
/// zero still occupies the burnout step. Zero therefore means *no epidemic
/// existed to measure*, which is a different statement from *nobody was
/// infected*, and only a nodeless graph can make it. Agreed 2026-08-04.
// `R: Rng + ?Sized` accepts either a concrete RNG type or a `&mut dyn Rng`
// trait object — `?Sized` opts out of Rust's default "must have a known
// size" requirement, which a trait object doesn't satisfy. Same bound, same
// reason, on `transmits` below.
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
        // Iterating with .iter()/&mut state yields references, not values —
        // the * below dereferences to read or assign the underlying State.
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

        // The final pass carries no new infections, and its zero is recorded
        // rather than suppressed: §5.2 counts that burnout step in `length`,
        // and `profile` terminates with it. `length` needs no adjustment for
        // this — the profile grows by one element, so `profile.len() - 1`
        // becomes the burnout-inclusive count on its own.
        profile.push(currently_infectious);
    }

    SirRun {
        length: profile.len() - 1,
        spread: profile.iter().sum(),
        profile, // shorthand for `profile: profile` — the local var name matches the field name
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

        assert_eq!(run.profile, vec![1, 1, 1, 1, 1, 1, 0]);
        assert_eq!(
            run.length, 6,
            "one step per edge of the path, plus the burnout step",
        );
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

        assert_eq!(
            run.length, 1,
            "spec 5.2: the burnout step counts, so no transmission is length 1",
        );
        assert_eq!(run.spread, 1, "patient zero alone");
        assert_eq!(run.profile, vec![1, 0]);
    }

    #[test]
    fn a_zero_infection_rate_never_transmits() {
        let graph = path_graph(5);
        let params = SirParams {
            infection_rate: 0.0,
            patient_zero: Some(0),
        };

        let run = sir_sim(&graph, &params, &mut rng());

        assert_eq!(run.length, 1, "same shape as an isolated patient zero");
        assert_eq!(run.spread, 1);
        assert_eq!(run.profile, vec![1, 0]);
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
        assert_eq!(
            run.profile,
            vec![1, 2, 0],
            "both neighbours infected at once, then burnout",
        );
        assert_eq!(run.length, 2);
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
    fn run_from_seed(graph: &Graph, params: &SirSampleParams, seed: u64) -> SirRun {
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

        let runs = simulate_epidemics(&graph, &params, 2026);

        assert_eq!(runs.len(), 4, "one run per epidemic");
        for (epidemic, run) in runs.iter().enumerate() {
            let expected = run_from_seed(&graph, &params, seeds[slot(&params, epidemic, 0)]);
            assert_eq!(*run, expected, "epidemic {epidemic} used the wrong draw");
        }
    }

    #[test]
    fn two_graphs_under_one_batch_seed_face_an_identical_pool() {
        // Different graphs, so the outcomes differ; the dice must not.
        let (pair, params) = coin_flip_sample(6, 1, 5);
        let path = path_graph(5);
        let seeds = epidemic_seeds(4242, params.num_epidemics, params.max_epidemic_retries);

        let pair_runs = simulate_epidemics(&pair, &params, 4242);
        let path_runs = simulate_epidemics(&path, &params, 4242);

        for epidemic in 0..params.num_epidemics {
            let seed = seeds[slot(&params, epidemic, 0)];
            assert_eq!(pair_runs[epidemic], run_from_seed(&pair, &params, seed));
            assert_eq!(path_runs[epidemic], run_from_seed(&path, &params, seed));
        }
        assert_ne!(
            pair_runs, path_runs,
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

        let runs = simulate_epidemics(&graph, &params, 11);

        for (epidemic, run) in runs.iter().enumerate() {
            let first_attempt = run_from_seed(&graph, &params, seeds[slot(&params, epidemic, 0)]);
            assert_eq!(*run, first_attempt);
        }
        assert!(
            runs.iter().any(|run| run.length == 1),
            "vacuous unless some epidemic was short enough to reject",
        );
    }

    #[test]
    fn an_unreachable_min_length_exhausts_the_retries_and_keeps_the_last() {
        // `length` is at most 2 here, so nothing ever satisfies 3 and every
        // epidemic runs all five attempts. The kept run must be the last.
        let (graph, params) = coin_flip_sample(12, 3, 5);
        let seeds = epidemic_seeds(5150, params.num_epidemics, params.max_epidemic_retries);

        let runs = simulate_epidemics(&graph, &params, 5150);

        for (epidemic, run) in runs.iter().enumerate() {
            let last = slot(&params, epidemic, params.max_epidemic_retries - 1);
            let final_attempt = run_from_seed(&graph, &params, seeds[last]);
            assert_eq!(
                *run, final_attempt,
                "epidemic {epidemic} kept the wrong attempt"
            );
        }
        assert!(
            runs.iter()
                .any(|run| run.length < params.min_epidemic_length),
            "a short run must be kept rather than looped on forever",
        );
    }

    #[test]
    fn the_reroll_stops_on_the_first_long_enough_attempt() {
        let (graph, params) = coin_flip_sample(12, 2, 5);
        let seeds = epidemic_seeds(31337, params.num_epidemics, params.max_epidemic_retries);

        let runs = simulate_epidemics(&graph, &params, 31337);

        let mut stopped_early = 0;
        for (epidemic, run) in runs.iter().enumerate() {
            // Replay every attempt this epidemic could have made.
            let mut attempts = Vec::new();
            for attempt in 0..params.max_epidemic_retries {
                let seed = seeds[slot(&params, epidemic, attempt)];
                attempts.push(run_from_seed(&graph, &params, seed));
            }

            // It should have kept the first long-enough one, or the last.
            let first_ok = attempts
                .iter()
                .position(|run| run.length >= params.min_epidemic_length);
            let expected = first_ok.unwrap_or(params.max_epidemic_retries - 1);

            assert_eq!(*run, attempts[expected], "epidemic {epidemic}");
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

        let run = sir_sim(&graph, &params, &mut rng());

        assert_eq!(run.length, 0);
        assert_eq!(run.spread, 0);
        assert!(run.profile.is_empty());
    }
}
