# GraphEvolutionTool

The Graph Evolution Tool (GET) generates and refines synthetic network data using a genetic algorithm. Users supply a custom fitness function and application-specific parameters, and GET evolves networks meeting those criteria. GET offers two GA representations, which can be stacked to produce higher-fitness networks.

**Documentation: <https://md12ol.github.io/GraphEvolutionTool/>** — what every configuration key
means, one page per route, and how to add your own objective. Start at
[The Pipeline](https://md12ol.github.io/GraphEvolutionTool/guide/pipeline.html) for how a run works,
or [Configuration](https://md12ol.github.io/GraphEvolutionTool/guide/configuration.html) for the
per-key reference.

## Configuring a run

A run is configured either by a TOML file or by typed objects in Python. Both
go through the same parser and the same validation, so neither accepts a
configuration the other would reject.

- **TOML** — copy
  [`config.example.toml`](https://github.com/md12ol/GraphEvolutionTool/blob/main/config.example.toml)
  and adjust it. It holds two complete setups, one live and one commented out beneath it;
  switching between them is commenting one block out and uncommenting the
  other, not editing scattered keys. What each key means is on the
  [Configuration page](https://md12ol.github.io/GraphEvolutionTool/guide/configuration.html).
- **Python** — build the same thing as objects. Worked examples for all four
  objectives, both genomes and both evolution strategies are in
  [`examples/config_builder.py`](https://github.com/md12ol/GraphEvolutionTool/blob/main/examples/config_builder.py).

**`examples/` and `config.example.toml` live in the repository, not in the wheel.** Installing the
package gets you the extension module and nothing else, so the links above go to GitHub rather than
to a path on your machine.

```python
import get

config = get.Config(
    population_size=200,
    network_size=100,
    crossover_rate=0.9,
    mutation_rate=0.2,
    evolution=get.EvolutionConfig.Generational(num_generations=500),
    scope=get.ScopeConfig.Global(),
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

## Installing

Building the extension module needs [maturin](https://www.maturin.rs):
`maturin develop` for a working copy, or `pip install .`.

## Contact

Questions, bug reports and feature requests are welcome as
[issues](https://github.com/md12ol/GraphEvolutionTool/issues). If you would rather write
to us directly, or you have a network problem you would like to point GET at:

- Michael Dubé, <michael.dube@ovgu.de>
- James Sargant, <js17sy@brocku.ca>
