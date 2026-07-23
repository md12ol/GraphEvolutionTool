# GraphEvolutionTool
The Graph Evolution Tool (GET) generates and refines synthetic network data using a genetic algorithm. Users supply a custom fitness function and application-specific parameters, and GET evolves networks meeting those criteria. GET offers two GA representations, which can be stacked to produce higher-fitness networks.

## Graph multiplicity

GET uses one graph representation for both unweighted graphs and multigraphs.
`Graph::new(num_nodes)` retains the default maximum multiplicity of five,
whereas `Graph::unweighted(num_nodes)` enforces weights in `0..=1`.
`Graph::with_max_edge_multiplicity(num_nodes, cap)` accepts an explicit cap in
`1..=5`.

SDA callers select the same behavior with `SdaContext::new`,
`SdaContext::unweighted`, or `SdaContext::with_max_edge_multiplicity`.
Edge-edit genomes inherit the cap from their base graph.
