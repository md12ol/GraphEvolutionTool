#!/usr/bin/env python3
"""Draw a GET edge file as a PNG.

    python3 tools/graph_to_png.py best_individual.txt
    python3 tools/graph_to_png.py best_individual.txt out/winner.png

One required argument, the graph file. With a second, that is where the image
goes; without one it is the input path with its extension replaced by `.png`.

**Standard library only, like everything else in `tools/`.** No matplotlib, no
networkx, no install step. The PNG is encoded here with `zlib` and the layout
is a plain force-directed one, so this runs on the same bare interpreter that
runs the converter beside it. The cost is that it draws a readable diagram
rather than a publication figure; if you want the latter, read the edge list
into whichever library you already use.

Everything is deterministic: the same file always produces the same image, with
no seed to pass, because the layout starts from a circle rather than from
random positions. Two runs over the same graph are byte-identical, which is
what makes an image safe to check in beside a result.
"""

import math
import os
import struct
import sys
import zlib

# --- Drawing constants -------------------------------------------------------

# The invocation lines from the docstring above. Read out rather than indexed at
# the point of use: `__doc__` is `str | None`, and `python -OO` strips it.
USAGE = (
    (__doc__ or "").strip().split("\n\n")[1]
    if __doc__
    else "usage: python3 tools/graph_to_png.py <edges.txt> [out.png]"
)

SIZE = 900  # output edge length in pixels, square
SUPERSAMPLE = 3  # rendered at SIZE * this, then box-filtered down
MARGIN = 60  # keeps node circles off the edge of the image

BACKGROUND = (255, 255, 255)
EDGE_COLOUR = (150, 160, 175)
NODE_FILL = (40, 90, 160)
NODE_EDGE = (255, 255, 255)

NODE_RADIUS = 9  # in output pixels, before supersampling
LAYOUT_ITERATIONS = 300


def read_graph(path):
    """Return `(num_nodes, [(u, v, weight), ...])` from a GET edge file.

    The format is the one GET both reads and writes: `#` starts a comment, one
    of which must be `# nodes = N`, and every other line is `u,v,weight`. The
    header is required here for the same reason GET requires it: a node with
    no edges is invisible to the edge rows, so a count taken from them is short
    by exactly the nodes hardest to notice.
    """
    num_nodes = None
    edges = []

    with open(path, encoding="utf-8") as handle:
        for number, raw in enumerate(handle, start=1):
            line = raw.strip()
            if not line:
                continue

            if line.startswith("#"):
                body = line[1:].strip()
                if body.lower().startswith("nodes"):
                    _, _, value = body.partition("=")
                    try:
                        num_nodes = int(value.strip())
                    except ValueError:
                        raise SystemExit(
                            f"{path}:{number}: could not read a node count from {line!r}"
                        )
                continue

            fields = line.split(",")
            if len(fields) != 3:
                raise SystemExit(
                    f"{path}:{number}: expected 3 comma-separated fields, got {len(fields)}"
                )
            try:
                u, v, weight = (int(field) for field in fields)
            except ValueError:
                raise SystemExit(f"{path}:{number}: not three integers: {line!r}")
            edges.append((u, v, weight))

    if num_nodes is None:
        raise SystemExit(
            f"{path}: no '# nodes = N' header. Every file GET reads carries one, "
            "and it cannot be inferred from the edges."
        )

    for u, v, _ in edges:
        if not (0 <= u < num_nodes and 0 <= v < num_nodes):
            raise SystemExit(
                f"{path}: edge ({u}, {v}) is outside 0..{num_nodes - 1}. If your file "
                "is 1-indexed, the header and the indices disagree."
            )

    return num_nodes, edges


def layout(num_nodes, edges):
    """Node positions in [0, 1]^2, from a force-directed layout.

    Fruchterman-Reingold: every pair repels, every edge attracts, and the
    maximum displacement per step decays to zero so the thing settles. Started
    from an evenly spaced circle rather than random positions, which is what
    makes the result reproducible without a seed, and which also gives a
    graph with no edges at all a sensible picture instead of a heap.
    """
    positions = []
    for node in range(num_nodes):
        angle = 2 * math.pi * node / max(num_nodes, 1)
        positions.append([0.5 + 0.4 * math.cos(angle), 0.5 + 0.4 * math.sin(angle)])

    if num_nodes < 2:
        return positions

    # The distance at which attraction and repulsion balance, for a layout
    # filling the unit square.
    k = math.sqrt(1.0 / num_nodes)
    step = 0.1

    for iteration in range(LAYOUT_ITERATIONS):
        displacement = [[0.0, 0.0] for _ in range(num_nodes)]

        for i in range(num_nodes):
            for j in range(i + 1, num_nodes):
                dx = positions[i][0] - positions[j][0]
                dy = positions[i][1] - positions[j][1]
                distance = math.hypot(dx, dy) or 1e-9
                force = k * k / distance
                displacement[i][0] += dx / distance * force
                displacement[i][1] += dy / distance * force
                displacement[j][0] -= dx / distance * force
                displacement[j][1] -= dy / distance * force

        for u, v, _ in edges:
            if u == v:
                continue
            dx = positions[u][0] - positions[v][0]
            dy = positions[u][1] - positions[v][1]
            distance = math.hypot(dx, dy) or 1e-9
            force = distance * distance / k
            displacement[u][0] -= dx / distance * force
            displacement[u][1] -= dy / distance * force
            displacement[v][0] += dx / distance * force
            displacement[v][1] += dy / distance * force

        for node in range(num_nodes):
            dx, dy = displacement[node]
            length = math.hypot(dx, dy) or 1e-9
            limit = min(length, step)
            positions[node][0] += dx / length * limit
            positions[node][1] += dy / length * limit

        step *= 1.0 - (iteration + 1) / LAYOUT_ITERATIONS * 0.02

    # Rescale to fill the square, so a layout that settled small is not drawn
    # as a dot in the middle.
    #
    # Measured over the nodes that have an edge, not over every node. An
    # isolated node feels repulsion and no attraction, so it drifts to the rim
    # and stops; a span taken over all of them is set by those strays, and
    # dividing by it shrinks the connected part, the part worth looking at,
    # to a dot. That is the very outcome this rescale exists to avoid, and an
    # evolved graph almost always has a few isolated nodes.
    connected = set()
    for u, v, _ in edges:
        connected.add(u)
        connected.add(v)
    measured = sorted(connected) if connected else range(num_nodes)

    xs = [positions[node][0] for node in measured]
    ys = [positions[node][1] for node in measured]
    span_x = (max(xs) - min(xs)) or 1.0
    span_y = (max(ys) - min(ys)) or 1.0
    span = max(span_x, span_y)
    centre_x = min(xs) + span_x / 2
    centre_y = min(ys) + span_y / 2

    # Strays land outside the unit square once the span is the core's, so they
    # are held at the border rather than drawn off the canvas.
    for position in positions:
        position[0] = min(1.0, max(0.0, 0.5 + (position[0] - centre_x) / span))
        position[1] = min(1.0, max(0.0, 0.5 + (position[1] - centre_y) / span))

    return positions


class Canvas:
    """A flat RGB pixel buffer with the three shapes this script needs."""

    def __init__(self, size, background):
        self.size = size
        self.pixels = bytearray(bytes(background) * size * size)

    def set(self, x, y, colour):
        if 0 <= x < self.size and 0 <= y < self.size:
            offset = (y * self.size + x) * 3
            self.pixels[offset : offset + 3] = bytes(colour)

    def line(self, x0, y0, x1, y1, colour, width):
        """A thick line, drawn as a run of discs along the segment.

        Discs rather than Bresenham with a width: they join without gaps at any
        angle, and at supersampled resolution the result is smooth once the
        image is filtered down.
        """
        steps = int(max(abs(x1 - x0), abs(y1 - y0))) + 1
        radius = max(width // 2, 0)
        for step in range(steps + 1):
            t = step / steps
            x = round(x0 + (x1 - x0) * t)
            y = round(y0 + (y1 - y0) * t)
            self.disc(x, y, radius, colour)

    def disc(self, cx, cy, radius, colour):
        if radius <= 0:
            self.set(cx, cy, colour)
            return
        for y in range(cy - radius, cy + radius + 1):
            span = int(math.sqrt(max(radius * radius - (y - cy) ** 2, 0)))
            for x in range(cx - span, cx + span + 1):
                self.set(x, y, colour)

    def downsample(self, factor):
        """Box-filter to 1/`factor` of each dimension, the anti-aliasing."""
        out_size = self.size // factor
        out = bytearray(out_size * out_size * 3)
        area = factor * factor
        for y in range(out_size):
            for x in range(out_size):
                totals = [0, 0, 0]
                for dy in range(factor):
                    row = (y * factor + dy) * self.size
                    for dx in range(factor):
                        offset = (row + x * factor + dx) * 3
                        totals[0] += self.pixels[offset]
                        totals[1] += self.pixels[offset + 1]
                        totals[2] += self.pixels[offset + 2]
                offset = (y * out_size + x) * 3
                out[offset] = totals[0] // area
                out[offset + 1] = totals[1] // area
                out[offset + 2] = totals[2] // area
        return out_size, out


def write_png(path, size, pixels):
    """Write an 8-bit RGB PNG. `pixels` is `size * size * 3` bytes, row-major."""

    def chunk(kind, payload):
        return (
            struct.pack(">I", len(payload))
            + kind
            + payload
            + struct.pack(">I", zlib.crc32(kind + payload) & 0xFFFFFFFF)
        )

    # Every scanline is prefixed with filter type 0 ("none"), which is what
    # makes this an encoder rather than a compressor: zlib does the rest.
    raw = bytearray()
    stride = size * 3
    for y in range(size):
        raw.append(0)
        raw.extend(pixels[y * stride : (y + 1) * stride])

    header = struct.pack(">IIBBBBB", size, size, 8, 2, 0, 0, 0)
    with open(path, "wb") as handle:
        handle.write(b"\x89PNG\r\n\x1a\n")
        handle.write(chunk(b"IHDR", header))
        handle.write(chunk(b"IDAT", zlib.compress(bytes(raw), 9)))
        handle.write(chunk(b"IEND", b""))


def draw(num_nodes, edges, positions):
    scale = SIZE * SUPERSAMPLE
    margin = MARGIN * SUPERSAMPLE
    usable = scale - 2 * margin

    canvas = Canvas(scale, BACKGROUND)

    def to_pixels(position):
        return (
            round(margin + position[0] * usable),
            round(margin + position[1] * usable),
        )

    heaviest = max((weight for _, _, weight in edges), default=1) or 1
    for u, v, weight in edges:
        if u == v:
            continue
        x0, y0 = to_pixels(positions[u])
        x1, y1 = to_pixels(positions[v])
        # Weight is a multiplicity, so thickness scales with it: a parallel
        # edge should look like more than one edge.
        width = SUPERSAMPLE * (1 + 2 * weight // heaviest)
        canvas.line(x0, y0, x1, y1, EDGE_COLOUR, width)

    radius = NODE_RADIUS * SUPERSAMPLE
    for node in range(num_nodes):
        x, y = to_pixels(positions[node])
        canvas.disc(x, y, radius, NODE_EDGE)
        canvas.disc(x, y, radius - SUPERSAMPLE, NODE_FILL)

    return canvas.downsample(SUPERSAMPLE)


def main(argv):
    if not 2 <= len(argv) <= 3:
        print(USAGE, file=sys.stderr)
        return 1

    source = argv[1]
    destination = argv[2] if len(argv) == 3 else os.path.splitext(source)[0] + ".png"

    num_nodes, edges = read_graph(source)
    positions = layout(num_nodes, edges)
    size, pixels = draw(num_nodes, edges, positions)

    parent = os.path.dirname(destination)
    if parent:
        os.makedirs(parent, exist_ok=True)
    write_png(destination, size, pixels)

    print(f"{source}: {num_nodes} nodes, {len(edges)} edges -> {destination}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
