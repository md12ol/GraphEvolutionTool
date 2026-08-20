"""Building GET configurations from Python.

The Python-side counterpart to `config.example.toml`: every configuration
that file shows in TOML, built here as typed objects instead. Both routes
converge on the same parser and the same validator (spec section 8), so
anything one accepts the other accepts, and anything one rejects the other
rejects with the same message.

Run it after installing the extension module:

    maturin develop        # or: pip install .
    python examples/config_builder.py

Every example below works today. `GraphEvolver.run()` works too, and returns a
`RunResult` carrying `best_fitness`, `best_edges`, `best_genome_repr` and the
convergence `history` — but this script deliberately stops at building,
validating and printing configurations, so that running it costs nothing. The
`evolve_with_a_custom_objective` example marks the one line that would evolve.
"""

import os

import get


def the_shipped_example():
    """The direct equivalent of `config.example.toml`.

    An edit script applied to a base graph, evolved generationally, scored on
    how far an epidemic spreads.

    This is the object-building route to the same configuration
    `config.example.toml` writes in TOML. The two are worth reading side by
    side: every key in that file appears here as a constructor argument.

    They match today, and nothing checks that they still do. That is
    deliberate: they exist to show the two routes, not to be locked together.

    `infection_rate` mirrors the TOML at 0.5, which is above a plausible
    per-contact figure and chosen deliberately: at 0.05 an outbreak on a
    100-node sparse graph dies almost immediately whatever the topology, so
    every individual scores alike and selection has no gradient to follow.
    """
    return get.Config(
        population_size=200,
        network_size=100,
        crossover_rate=0.9,
        mutation_rate=0.2,
        evolution=get.EvolutionConfig.Generational(num_generations=500, elite_count=1),
        # Generational breeds from the whole population; the tournament is what
        # applies the pressure.
        scope=get.ScopeConfig.Global(),
        selection=get.SelectionConfig.Tournament(tournament_size=5),
        genome=get.GenomeConfig.EdgeEdit(gene_length=256),
        fitness=get.FitnessConfig.EpiSpread(
            sir=get.SirParams(infection_rate=0.5, num_epidemics=30)
        ),
    )


def a_weighted_multigraph_from_an_automaton():
    """SDA genome, steady-state, parallel edges allowed.

    Note there is no `num_chars` to set: the automaton's alphabet is derived
    as `max_edge_multiplicity + 1`, so every character it can emit is a legal
    edge weight and none is ever clamped away (spec section 3.2).
    """
    return get.Config(
        population_size=200,
        network_size=128,
        max_edge_multiplicity=5,
        crossover_rate=0.9,
        mutation_rate=0.2,
        max_mutations=3,
        evolution=get.EvolutionConfig.SteadyState(num_mating_events=100_000),
        # Steady-state's pressure comes from the scope being small, so the two
        # parents are simply its best. `size` is the scope's own parameter and
        # must be at least 4: two parents and the two they replace, all
        # distinct.
        scope=get.ScopeConfig.RandomSubset(size=7),
        selection=get.SelectionConfig.Best(),
        genome=get.GenomeConfig.Sda(num_states=12, max_resp_len=4),
        fitness=get.FitnessConfig.EpiLength(
            sir=get.SirParams(
                infection_rate=0.05,
                num_epidemics=30,
                patient_zero=0,  # pinned; omit to draw a fresh node per epidemic
            )
        ),
    )


def tuning_the_edit_operations():
    """Reweighting the nine edit operations.

    Weights are relative, not percentages: all-equal is the same distribution
    whether every value is 1.0 or 10.0. Omitted fields stay at 1.0, and 0.0
    disables an operation outright.
    """
    weights = get.OperationWeights(
        null=0.0,  # drop the no-op, so every gene does something
        swap=2.0,  # and draw swaps twice as often as the rest
    )
    config = the_shipped_example()
    config.genome = get.GenomeConfig.EdgeEdit(gene_length=256, operation_weights=weights)
    return config


def matching_a_target_profile():
    """The one objective that needs something beyond the shared SIR block.

    Minimized rather than maximized — direction is fixed by what the objective
    computes and is deliberately not configurable.

    The profile is an ordinary list of numbers, passed inline. It is compared
    verbatim: nothing is prepended for patient zero and nothing is rescaled for
    the network size, so give the curve you want at the size of the network you
    are building. It must be non-empty and every element finite.
    """
    config = the_shipped_example()
    config.fitness = get.FitnessConfig.EpiProfMatch(
        sir=get.SirParams(infection_rate=0.05, num_epidemics=30),
        target_profile=[1, 3, 8, 17, 24, 19, 11, 5, 2, 1],
    )
    return config


def evolve_with_a_custom_objective():
    """Scoring the graphs yourself, in Python.

    The config only *selects* Python; the callable and the direction it wants
    arrive through `set_fitness_function`.
    """
    config = the_shipped_example()
    config.fitness = get.FitnessConfig.Python()
    evolver = get.GraphEvolver.from_config(config)

    def total_edges(batch):
        """Score a whole batch at once, returning one float per graph.

        Taking the batch rather than one graph at a time is a hard requirement,
        not a convenience: the engine scores in parallel, and re-entering
        Python per graph deadlocks against the interpreter lock.

        Each item is `(num_nodes, [(u, v, multiplicity), ...])`.
        """
        return [float(len(edges)) for (num_nodes, edges) in batch]

    evolver.set_fitness_function(total_edges, "maximize")

    # result = evolver.run(seed=1)   -> RunResult
    return evolver


def seeding_from_a_file():
    """Starting an edit script from a graph you already have on disk.

    The base graph is not a config value — `config.example.toml` has no key for
    it — so it arrives through the evolver. `set_base_graph_from_file` reads one
    edge per line, `start,end,weight`, and `min_node_index` says where your own
    node numbering starts: `base_graph.csv` beside this script is 1-indexed, as
    graph files usually are.

    You never renumber anything by hand. Every index shifts to 0 on the way in,
    and the evolved graph comes back shifted the other way, so `best_edges` is
    in the numbering you wrote.

    `network_size` is 10 here rather than the shipped example's 100, because a
    base graph has to be the size the run evolves — the file names nodes 1 to
    10, so the network is 10 nodes.

    Two things follow from seeding that an empty run does not get. Generation 0
    keeps one individual that edits nothing, so the supplied graph is in the
    population from the start; and all nine edit operations are useful
    immediately, where an unseeded run leaves five of them inert until `add` or
    `toggle` have built some structure.
    """
    config = the_shipped_example()
    config.network_size = 10
    evolver = get.GraphEvolver.from_config(config)

    here = os.path.dirname(os.path.abspath(__file__))
    evolver.set_base_graph_from_file(os.path.join(here, "base_graph.csv"), min_node_index=1)

    # result = evolver.run(seed=1)[0]   -> result.best_edges is 1-indexed too
    return evolver


def a_config_that_gets_rejected():
    """Validation reports the offending field, whichever front end you used.

    A cap of 0 clamps every edge weight to nothing under any genome, so the run
    would look like a broken fitness function rather than a bad config.
    """
    config = the_shipped_example()
    config.max_edge_multiplicity = 0
    try:
        get.GraphEvolver.from_config(config)
    except ValueError as err:
        return str(err)
    raise AssertionError("that config should not have been accepted")


def main():
    for describe, build in [
        ("config.example.toml, in Python", the_shipped_example),
        ("weighted multigraph from an SDA", a_weighted_multigraph_from_an_automaton),
        ("reweighted edit operations", tuning_the_edit_operations),
        ("profile matching", matching_a_target_profile),
    ]:
        config = build()
        get.GraphEvolver.from_config(config)  # parses and validates
        print(f"=== {describe} ===")
        print(config.to_toml())

    evolve_with_a_custom_objective()
    print("=== custom Python objective: registered ===\n")

    seeding_from_a_file()
    print("=== base graph seeded from examples/base_graph.csv ===\n")

    print("=== an invalid config ===")
    print(a_config_that_gets_rejected(), "\n")

    # `to_toml()` is the run's provenance record: write it beside the results
    # and it re-runs verbatim, whether or not the original was built in Python.
    print("=== provenance ===")
    print("write config.to_toml() next to your results to reproduce the run")


if __name__ == "__main__":
    main()
