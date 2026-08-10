# GraphEvolutionTool
The Graph Evolution Tool (GET) generates and refines synthetic network data using a genetic algorithm. Users supply a custom fitness function and application-specific parameters, and GET evolves networks meeting those criteria. GET offers two GA representations, which can be stacked to produce higher-fitness networks.

## Configuring a run

A run is configured either by a TOML file or by typed objects in Python. Both
go through the same parser and the same validation, so neither accepts a
configuration the other would reject.

- **TOML** — copy [`config.example.toml`](config.example.toml) and adjust it.
  Every field is documented in place, with the alternatives commented out.
- **Python** — build the same thing as objects. Worked examples for all four
  objectives, both genomes and both evolution strategies are in
  [`examples/config_builder.py`](examples/config_builder.py).

```python
import get

config = get.Config(
    population_size=200,
    network_size=100,
    crossover_rate=0.9,
    mutation_rate=0.2,
    evolution=get.EvolutionConfig.Generational(num_generations=500),
    selection=get.SelectionConfig.Tournament(tournament_size=5),
    genome=get.GenomeConfig.EdgeEdit(gene_length=256),
    fitness=get.FitnessConfig.EpiSpread(
        sir=get.SirParams(infection_rate=0.05, num_epidemics=30)
    ),
)
evolver = get.GraphEvolver.from_config(config)
```

`config.to_toml()` returns the document that was actually parsed — write it
beside your results and the run reproduces verbatim.

Building the extension module needs [maturin](https://www.maturin.rs):
`maturin develop` for a working copy, or `pip install .`.
