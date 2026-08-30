"""Python route 1 of 2: one evolution, with every parameter written in this file.

Run it after installing the extension module:

    maturin develop        # or: pip install .
    python examples/python_inline.py

This is the route for someone who wants a run they can read top to bottom, with
nothing to look up in another document. There is no `config.toml` anywhere in
it: the configuration is built as typed objects, handed to `GraphEvolver` and
run. The trade is that the run's provenance lives in this file rather than in a
document you can archive beside the results — `result.config_toml` is written
out below for exactly that reason.

Its counterpart, `python_from_config.py`, reads the same parameters from a TOML
file instead. Between them they are the two Python routes; `config_builder.py`
beside them builds configurations without running anything.

Edge-edit genome, generational strategy, `epi_spread` objective — the same
combination `config.example.toml`'s live setup uses, deliberately, so the two
can be read side by side.
"""

import get

from _output_layout import experiment_output_dir, run_output_dir, utc_stamp

# --- What you would change ---------------------------------------------------

# Where results go. Each run lands in OUTPUT_DIR/<timestamp>-<seed>/, and each
# replicate in a run_<index>/ of its own inside that.
OUTPUT_DIR = "./output"

# The master seed, and how many replicates to draw from it. A replicate is
# reproduced by re-running with the same master seed and reading the same
# run_index — its own derived seed will not reproduce it.
SEED = 7
N_RUNS = 2

# Small enough to finish while you watch. The comparable numbers in
# `config.example.toml` are 200 / 100 / 500.
POPULATION_SIZE = 60
NETWORK_SIZE = 40
NUM_GENERATIONS = 50


def build_config():
    """The whole run, as typed objects rather than a TOML document.

    Every argument here is a key `config.example.toml` writes under a section
    heading; the two routes converge on the same parser and the same validator,
    so a configuration either of them accepts is one the other accepts too.
    """
    return get.Config(
        population_size=POPULATION_SIZE,
        network_size=NETWORK_SIZE,
        crossover_rate=0.9,
        mutation_rate=0.2,
        evolution=get.EvolutionConfig.Generational(
            num_generations=NUM_GENERATIONS,
            elite_count=1,
        ),
        # Generational breeds from the whole population; the tournament is what
        # applies the pressure.
        scope=get.ScopeConfig.Global(),
        selection=get.SelectionConfig.Tournament(tournament_size=5),
        genome=get.GenomeConfig.EdgeEdit(gene_length=256),
        # 0.5 is above a plausible per-contact rate and is chosen on purpose:
        # at 0.05 an outbreak on a sparse graph dies before topology matters,
        # so every individual scores alike and selection has no gradient.
        fitness=get.FitnessConfig.EpiSpread(
            sir=get.SirParams(infection_rate=0.5, num_epidemics=30)
        ),
    )


def main():
    config = build_config()

    # `from_config` renders the objects to a TOML document and parses it back,
    # so this route runs through exactly the same front end as the other one.
    evolver = get.GraphEvolver.from_config(config)

    # One stamp for the whole invocation, taken before anything runs.
    stamp = utc_stamp()

    # One call, every replicate. GET derives a seed per replicate from the
    # master, so asking for more of them does not change the ones you had.
    results = evolver.run(seed=SEED, n_runs=N_RUNS)

    for run_index, result in enumerate(results):
        if N_RUNS > 1:
            print(f"\n=== run_index {run_index}, of {N_RUNS} ===")

        print(f"best_fitness = {result.best_fitness}")
        print(f"edges        = {len(result.best_edges)}")

        first = result.history[0].best_fitness
        last = result.history[-1]
        print(
            f"best-of-generation went {first:.3f} -> {last.best_fitness:.3f} "
            f"over {last.iteration} generations"
        )

        directory = run_output_dir(OUTPUT_DIR, stamp, SEED, run_index, N_RUNS)

        # The convergence log and the winner as a loadable edge list — one
        # pair per replicate.
        result.save_logs(f"{directory}/run_log.csv")
        result.save_results(f"{directory}/best_individual.txt")
        print(f"wrote {directory}")

    # The config document this run was built from, written once for the whole
    # invocation rather than once per replicate — every replicate shares it.
    # This is what makes the run reproducible from a directory alone, which
    # matters more on this route than on the other one: here the parameters
    # live in a program someone will edit, not a file that travels with the
    # results on its own.
    result.save_config(experiment_output_dir(OUTPUT_DIR, stamp, SEED))


if __name__ == "__main__":
    main()
