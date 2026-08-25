"""Generate the documentation site's two real-output figures.

Everything else on the site is a hand-drawn conceptual diagram. These two are
the opposite: an actual GET run, plotted. Re-run this after anything that could
change a run's trajectory, and commit the regenerated SVGs.

    .venv/bin/python tools/make_doc_figures.py

Writes documentation/assets/convergence.svg and evolved-network.svg. Both use
the site's own `d-*` CSS classes rather than literal colours, so they follow the
light/dark theme like every other diagram.
"""

import math
import os

import get

# Pinned so the figures are reproducible: same seed, same config, same picture.
SEED = 20260821
HERE = os.path.dirname(os.path.abspath(__file__))
ASSETS = os.path.join(HERE, "..", "documentation", "assets")

# Small enough to draw legibly, long enough to show a curve that bends.
NETWORK_SIZE = 40
POPULATION = 60
GENERATIONS = 60


def build_config():
    return get.Config(
        evolution=get.EvolutionConfig.Generational(
            num_generations=GENERATIONS, elite_count=1
        ),
        population_size=POPULATION,
        network_size=NETWORK_SIZE,
        crossover_rate=0.9,
        mutation_rate=0.2,
        max_mutations=3,
        scope=get.ScopeConfig.Global(),
        selection=get.SelectionConfig.Tournament(tournament_size=5),
        genome=get.GenomeConfig.EdgeEdit(gene_length=128),
        # 0.5 for the same reason examples/config_builder.py uses it: at 0.05 an
        # outbreak on a sparse graph dies immediately and there is no gradient.
        fitness=get.FitnessConfig.EpiSpread(
            sir=get.SirParams(infection_rate=0.5, num_epidemics=30)
        ),
    )


def run():
    evolver = get.GraphEvolver.from_config(build_config())
    return evolver.run(seed=SEED)[0]


# --- convergence curve -------------------------------------------------------

W, H = 700, 320
PAD_L, PAD_R, PAD_T, PAD_B = 62, 18, 26, 46


def convergence_svg(history):
    xs = [row.iteration for row in history]
    best = [row.best_fitness for row in history]
    mean = [row.mean_fitness for row in history]

    lo = min(min(best), min(mean))
    hi = max(max(best), max(mean))
    span = hi - lo or 1.0
    lo -= span * 0.08
    hi += span * 0.08

    def px(i):
        return PAD_L + (i - xs[0]) / max(xs[-1] - xs[0], 1) * (W - PAD_L - PAD_R)

    def py(v):
        return H - PAD_B - (v - lo) / (hi - lo) * (H - PAD_T - PAD_B)

    def path(values):
        return " ".join(
            ("M" if i == 0 else "L") + f"{px(xs[i]):.1f} {py(v):.1f}"
            for i, v in enumerate(values)
        )

    ticks = []
    for k in range(5):
        v = lo + (hi - lo) * k / 4
        y = py(v)
        ticks.append(
            f'  <line x1="{PAD_L}" y1="{y:.1f}" x2="{W - PAD_R}" y2="{y:.1f}" '
            f'class="d-stroke" stroke-width="0.6" opacity="0.35"/>\n'
            f'  <text x="{PAD_L - 8}" y="{y + 4:.1f}" class="d-text-sm" '
            f'text-anchor="end">{v:.0f}</text>'
        )

    xticks = []
    for i in (0, len(xs) // 2, len(xs) - 1):
        x = px(xs[i])
        xticks.append(
            f'  <text x="{x:.1f}" y="{H - PAD_B + 20:.0f}" class="d-text-sm" '
            f'text-anchor="middle">{xs[i]}</text>'
        )

    return f"""<svg viewBox="0 0 {W} {H}" role="img" aria-label="Convergence of a {GENERATIONS}-generation edge-edit run scored on epidemic spread: best fitness rises from {best[0]:.0f} to {best[-1]:.0f} infected nodes and the population mean follows it.">
  <g style="color:var(--border-firm)">
{chr(10).join(ticks)}
    <line x1="{PAD_L}" y1="{PAD_T}" x2="{PAD_L}" y2="{H - PAD_B}" class="d-stroke" stroke-width="1.2"/>
    <line x1="{PAD_L}" y1="{H - PAD_B}" x2="{W - PAD_R}" y2="{H - PAD_B}" class="d-stroke" stroke-width="1.2"/>
{chr(10).join(xticks)}
    <text x="{(W + PAD_L) / 2:.0f}" y="{H - 6}" class="d-text-sm" text-anchor="middle">generation</text>
    <text x="16" y="{(H - PAD_B + PAD_T) / 2:.0f}" class="d-text-sm" text-anchor="middle" transform="rotate(-90 16 {(H - PAD_B + PAD_T) / 2:.0f})">nodes infected</text>

    <path d="{path(mean)}" fill="none" class="d-stroke" stroke-width="1.6" opacity="0.75"/>
    <path d="{path(best)}" fill="none" class="d-stroke-acc" stroke-width="2" style="color:var(--accent)"/>

    <rect x="{W - PAD_R - 150}" y="{PAD_T}" width="140" height="44" rx="6" class="d-fill-bg"/>
    <rect x="{W - PAD_R - 150}" y="{PAD_T}" width="140" height="44" rx="6" class="d-stroke"/>
    <line x1="{W - PAD_R - 140}" y1="{PAD_T + 15}" x2="{W - PAD_R - 118}" y2="{PAD_T + 15}" class="d-stroke-acc" stroke-width="2" style="color:var(--accent)"/>
    <text x="{W - PAD_R - 112}" y="{PAD_T + 19}" class="d-text-sm">best_fitness</text>
    <line x1="{W - PAD_R - 140}" y1="{PAD_T + 33}" x2="{W - PAD_R - 118}" y2="{PAD_T + 33}" class="d-stroke" stroke-width="1.6" opacity="0.75"/>
    <text x="{W - PAD_R - 112}" y="{PAD_T + 37}" class="d-text-sm">mean_fitness</text>
  </g>
</svg>
"""


# --- the evolved network -----------------------------------------------------

GW, GH, R = 340, 340, 7


def network_svg(num_nodes, edges, title):
    cx, cy, rad = GW / 2, GH / 2 + 6, GW / 2 - 26
    pos = {}
    for i in range(num_nodes):
        a = 2 * math.pi * i / num_nodes - math.pi / 2
        pos[i] = (cx + rad * math.cos(a), cy + rad * math.sin(a))

    lines = []
    for u, v, _w in edges:
        if u in pos and v in pos:
            x1, y1 = pos[u]
            x2, y2 = pos[v]
            lines.append(
                f'    <line x1="{x1:.1f}" y1="{y1:.1f}" x2="{x2:.1f}" y2="{y2:.1f}" '
                f'class="d-stroke-acc" stroke-width="0.7" opacity="0.5" style="color:var(--accent)"/>'
            )

    dots = []
    for i in range(num_nodes):
        x, y = pos[i]
        dots.append(
            f'    <circle cx="{x:.1f}" cy="{y:.1f}" r="{R * 0.5:.1f}" class="d-fill-acc"/>'
            f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{R * 0.5:.1f}" class="d-stroke" stroke-width="0.8"/>'
        )

    return (
        f'  <text x="{GW / 2:.0f}" y="16" class="d-text-acc" text-anchor="middle">{title}</text>\n'
        + "\n".join(lines)
        + "\n"
        + "\n".join(dots)
    )


def evolved_network_svg(result):
    edges = result.best_edges
    empty = network_svg(NETWORK_SIZE, [], "generation 0 — an empty base graph")
    grown = network_svg(NETWORK_SIZE, edges, f"generation {GENERATIONS} — {len(edges)} edges")
    return f"""<svg viewBox="0 0 {GW * 2 + 40} {GH + 34}" role="img" aria-label="Two {NETWORK_SIZE}-node graphs side by side. The edge-edit run starts from an empty graph and ends with {len(edges)} edges, evolved to spread an epidemic as widely as possible.">
  <g style="color:var(--border-firm)">
{empty}
  </g>
  <g style="color:var(--border-firm)" transform="translate({GW + 40} 0)">
{grown}
  </g>
</svg>
"""


def main():
    result = run()
    history = result.history

    os.makedirs(ASSETS, exist_ok=True)
    with open(os.path.join(ASSETS, "convergence.svg"), "w") as handle:
        handle.write(convergence_svg(history))
    with open(os.path.join(ASSETS, "evolved-network.svg"), "w") as handle:
        handle.write(evolved_network_svg(result))

    print(f"seed {SEED}, {NETWORK_SIZE} nodes, {POPULATION} population, {GENERATIONS} generations")
    print(f"best_fitness {history[0].best_fitness:.1f} -> {history[-1].best_fitness:.1f}")
    print(f"edges in the winner: {len(result.best_edges)}")
    print("wrote convergence.svg and evolved-network.svg")


if __name__ == "__main__":
    main()
