"""Round-trip tests for the TUDataset converter.

The conversion tests are stdlib-only and always run. The round-trip test needs
the built `get` module and skips with a message when it is not importable --
this lives outside the crate, so `cargo test` does not build it and a plain
checkout has no wheel.

    python3 -m unittest discover -s tools -v
"""

import csv
import os
import shutil
import tempfile
import unittest
import warnings

import tudataset_to_get as converter

FIXTURE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "testdata", "tiny_tu")

# What `tiny_tu/README.md` says the fixture holds, in graph order.
EXPECTED = [
    {"num_nodes": 4, "num_edges": 4},
    {"num_nodes": 4, "num_edges": 3},
    {"num_nodes": 3, "num_edges": 0},
    {"num_nodes": 4, "num_edges": 3},
]


def edge_rows(path):
    """The `u,v,weight` lines of a converted file, header and blanks dropped."""
    rows = []
    with open(path) as handle:
        for line in handle:
            line = line.strip()
            if line and not line.startswith("#"):
                rows.append(line)
    return rows


def stated_nodes(path):
    """The count in the file's `# nodes = N` header, or None if it has none."""
    with open(path) as handle:
        for line in handle:
            line = line.strip()
            if line.startswith("#") and "=" in line:
                key, value = line[1:].split("=", 1)
                if key.strip().lower() == "nodes":
                    return int(value.strip())
    return None


class ConversionTest(unittest.TestCase):
    def setUp(self):
        self.workspace = tempfile.mkdtemp(prefix="tu_convert_")
        self.output = os.path.join(self.workspace, "reference")
        self.manifest = os.path.join(self.workspace, "manifest.csv")
        self.rows = converter.convert(FIXTURE, self.output, self.manifest, quiet=True)

    def tearDown(self):
        shutil.rmtree(self.workspace, ignore_errors=True)

    def test_one_file_per_graph(self):
        files = sorted(os.listdir(self.output))
        self.assertEqual(len(files), len(EXPECTED))

    def test_manifest_matches_the_fixture(self):
        with open(self.manifest) as handle:
            rows = list(csv.DictReader(handle))

        self.assertEqual(len(rows), len(EXPECTED))
        for row, expected in zip(rows, EXPECTED):
            self.assertEqual(int(row["num_nodes"]), expected["num_nodes"], row["file"])
            self.assertEqual(int(row["num_edges"]), expected["num_edges"], row["file"])

    def test_the_manifest_is_not_inside_the_folder(self):
        # `load_edge_folder` reads every regular file in the folder, so a
        # manifest inside it would be parsed as edges and rejected.
        self.assertNotIn("manifest.csv", os.listdir(self.output))

    def test_every_row_has_three_comma_separated_fields(self):
        # GET splits on ',' and rejects anything else. The issue text says
        # whitespace; the parser disagrees, and the parser is what runs.
        for name in sorted(os.listdir(self.output)):
            for number, row in enumerate(
                edge_rows(os.path.join(self.output, name)), start=1
            ):
                self.assertEqual(
                    len(row.split(",")), 3, "{} row {}".format(name, number)
                )

    def test_every_file_states_its_node_count(self):
        # The header is the only place a node with no edges can appear, and GET
        # refuses a file without one -- so this is what makes the output
        # loadable at all, not merely accurate.
        for name, expected in zip(sorted(os.listdir(self.output)), EXPECTED):
            self.assertEqual(
                stated_nodes(os.path.join(self.output, name)),
                expected["num_nodes"],
                name,
            )

    def test_each_undirected_edge_appears_once(self):
        for name in sorted(os.listdir(self.output)):
            seen = set()
            for row in edge_rows(os.path.join(self.output, name)):
                low, high, _ = row.split(",")
                pair = (int(low), int(high))
                self.assertLess(pair[0], pair[1], name)
                self.assertNotIn(pair, seen, name)
                seen.add(pair)

    def test_the_isolated_node_graph_states_four_nodes_and_writes_three_edges(self):
        # Graph 2 is a triangle plus a trailing isolated node. The edges alone
        # describe three nodes; the header is the only thing that says four,
        # which is why an inferred count shifts every reference histogram.
        second = os.path.join(self.output, sorted(os.listdir(self.output))[1])
        self.assertEqual(stated_nodes(second), 4)
        self.assertEqual(len(edge_rows(second)), 3)

    def test_a_graph_with_no_edges_is_a_header_and_nothing_else(self):
        third = os.path.join(self.output, sorted(os.listdir(self.output))[2])
        self.assertEqual(stated_nodes(third), 3)
        self.assertEqual(edge_rows(third), [])


class MalformedInputTest(unittest.TestCase):
    """Input that must fail loudly, or be dropped with a count -- never pass through."""

    def setUp(self):
        self.workspace = tempfile.mkdtemp(prefix="tu_malformed_")

    def tearDown(self):
        shutil.rmtree(self.workspace, ignore_errors=True)

    def write_dataset(self, indicator, edges):
        source = os.path.join(self.workspace, "ds")
        os.makedirs(source)
        with open(os.path.join(source, "ds_graph_indicator.txt"), "w") as handle:
            handle.write(indicator)
        with open(os.path.join(source, "ds_A.txt"), "w") as handle:
            handle.write(edges)
        return source

    def convert(self, indicator, edges):
        source = self.write_dataset(indicator, edges)
        output = os.path.join(self.workspace, "out")
        manifest = os.path.join(self.workspace, "manifest.csv")
        return converter.convert(source, output, manifest, quiet=True), output

    def test_a_self_loop_is_dropped_rather_than_failing_the_file(self):
        # GET rejects a whole file for one self-loop, taking a usable graph with
        # it, so the converter drops it and reports a count instead.
        rows, output = self.convert("1\n1\n1\n", "1, 2\n2, 1\n2, 2\n2, 3\n3, 2\n")
        self.assertEqual(rows[0]["num_edges"], 2)
        with open(os.path.join(output, "graph_1.txt")) as handle:
            body = handle.read()
        self.assertNotIn("1,1,", body)

    def test_an_edge_spanning_two_graphs_is_rejected(self):
        with self.assertRaises(converter.ConversionError) as caught:
            self.convert("1\n1\n2\n2\n", "2, 3\n3, 2\n")
        self.assertIn("spans graphs", str(caught.exception))

    def test_an_edge_naming_a_node_the_indicator_does_not_have_is_rejected(self):
        with self.assertRaises(converter.ConversionError) as caught:
            self.convert("1\n1\n", "1, 9\n9, 1\n")
        self.assertIn("not in the indicator file", str(caught.exception))

    def test_a_missing_indicator_file_is_named(self):
        source = os.path.join(self.workspace, "bare")
        os.makedirs(source)
        with open(os.path.join(source, "ds_A.txt"), "w") as handle:
            handle.write("1, 2\n")
        with self.assertRaises(converter.ConversionError) as caught:
            converter.convert(source, os.path.join(self.workspace, "out"), quiet=True)
        self.assertIn("graph_indicator", str(caught.exception))

    def test_a_one_node_graph_is_expressible(self):
        # A header needs no endpoints, so the case the zero-weight sentinel
        # could not express is ordinary now: one node, no edges, stated.
        rows, output = self.convert("1\n2\n2\n", "2, 3\n3, 2\n")
        self.assertEqual(rows[0]["num_nodes"], 1)

        first = os.path.join(output, sorted(os.listdir(output))[0])
        self.assertEqual(stated_nodes(first), 1)
        self.assertEqual(edge_rows(first), [])


class RoundTripTest(unittest.TestCase):
    """Convert, then load through GET itself and check what it actually built."""

    @classmethod
    def setUpClass(cls):
        try:
            import get  # noqa: F401
        except ImportError as error:
            raise unittest.SkipTest(
                "the built `get` module is not importable ({}); build it with "
                "`maturin develop` to run the round trip".format(error)
            )

    def setUp(self):
        self.workspace = tempfile.mkdtemp(prefix="tu_roundtrip_")
        self.output = os.path.join(self.workspace, "reference")
        self.manifest = os.path.join(self.workspace, "manifest.csv")
        converter.convert(FIXTURE, self.output, self.manifest, quiet=True)

        repo = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        self.config = os.path.join(repo, "config.example.toml")

    def tearDown(self):
        shutil.rmtree(self.workspace, ignore_errors=True)

    def load(self):
        import get

        evolver = get.GraphEvolver(self.config)
        with warnings.catch_warnings():
            # A graph with no edges warns by design, and the fixture has one.
            warnings.simplefilter("ignore")
            return evolver.load_reference_graphs(self.output, 0)

    def test_graph_count_matches_the_source(self):
        self.assertEqual(len(self.load()), len(EXPECTED))

    def test_node_counts_survive_the_round_trip(self):
        # The count GET reports is the file's header, so this is the assertion
        # the header exists for: graph 2's fourth node is in no edge and would
        # be lost by any count taken from the data.
        for (source, num_nodes, _), expected in zip(self.load(), EXPECTED):
            self.assertEqual(num_nodes, expected["num_nodes"], os.path.basename(source))

    def test_edge_counts_survive_the_round_trip(self):
        for (source, _, edges), expected in zip(self.load(), EXPECTED):
            self.assertEqual(
                len(edges), expected["num_edges"], os.path.basename(source)
            )

    def test_no_zero_weight_rows_reach_get(self):
        # Every row the converter writes is a real edge now. A zero-weight row
        # would mean the sentinel had come back.
        for source, _, edges in self.load():
            for _, _, weight in edges:
                self.assertGreater(weight, 0, os.path.basename(source))


if __name__ == "__main__":
    unittest.main()
