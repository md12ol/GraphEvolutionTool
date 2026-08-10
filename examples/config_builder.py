"""Building GET configurations from Python.

The Python-side counterpart to `config.example.toml`: every configuration
that file shows in TOML, built here as typed objects instead. Both routes
converge on the same parser and the same validator (spec section 8), so
anything one accepts the other accepts, and anything one rejects the other
rejects with the same message.

Run it after installing the extension module:

    maturin develop        # or: pip install .
    python examples/config_builder.py

Every example below works today. What is NOT wired up yet is the run itself:
`GraphEvolver.run()` is still unimplemented pending GitHub #26, so this script
builds, validates and prints configurations rather than evolving anything.
The `evolve_with_a_custom_objective` example marks the one line that will
start working when #26 lands.
"""

import get


def the_shipped_example():
    """The direct equivalent of `config.example.toml`.

    An edit script applied to a base graph, evolved generationally, scored on
    how far an epidemic spreads.
    """
    return get.Config(
        population_size=200,
        network_size=100,
        crossover_rate=0.9,
        mutation_rate=0.2,
        evolution=get.EvolutionConfig.Generational(num_generations=500, elite_count=1),
        selection=get.SelectionConfig.Tournament(tournament_size=5),
        genome=get.GenomeConfig.EdgeEdit(gene_length=256),
        fitness=get.FitnessConfig.EpiSpread(
            sir=get.SirParams(infection_rate=0.05, num_epidemics=30)
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
        selection=get.SelectionConfig.Tournament(tournament_size=7),
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

    # edges = evolver.run(seed=1)   <- pending GitHub #26
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

    print("=== an invalid config ===")
    print(a_config_that_gets_rejected(), "\n")

    # `to_toml()` is the run's provenance record: write it beside the results
    # and it re-runs verbatim, whether or not the original was built in Python.
    print("=== provenance ===")
    print("write config.to_toml() next to your results to reproduce the run")


if __name__ == "__main__":
    main()
