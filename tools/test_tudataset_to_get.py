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
    {"num_nodes": 4, "num_edges": 4, "sentinel": "no"},
    {"num_nodes": 4, "num_edges": 3, "sentinel": "yes"},
    {"num_nodes": 3, "num_edges": 0, "sentinel": "yes"},
    {"num_nodes": 4, "num_edges": 3, "sentinel": "no"},
]


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
            self.assertEqual(row["sentinel"], expected["sentinel"], row["file"])

    def test_the_manifest_is_not_inside_the_folder(self):
        # `load_edge_folder` reads every regular file in the folder, so a
        # manifest inside it would be parsed as edges and rejected.
        self.assertNotIn("manifest.csv", os.listdir(self.output))

    def test_every_row_has_three_comma_separated_fields(self):
        # GET splits on ',' and rejects anything else. The issue text says
        # whitespace; the parser disagrees, and the parser is what runs.
        for name in sorted(os.listdir(self.output)):
            with open(os.path.join(self.output, name)) as handle:
                for number, line in enumerate(handle, start=1):
                    line = line.strip()
                    if not line:
                        continue
                    self.assertEqual(
                        len(line.split(",")), 3, "{} line {}".format(name, number)
                    )

    def test_each_undirected_edge_appears_once(self):
        for name in sorted(os.listdir(self.output)):
            seen = set()
            with open(os.path.join(self.output, name)) as handle:
                for line in handle:
                    line = line.strip()
                    if not line:
                        continue
                    low, high, _ = line.split(",")
                    pair = (int(low), int(high))
                    self.assertLess(pair[0], pair[1], name)
                    self.assertNotIn(pair, seen, name)
                    seen.add(pair)

    def test_the_isolated_node_graph_carries_a_zero_weight_sentinel(self):
        # Graph 2 is a triangle plus a trailing isolated node. Without the
        # sentinel it loads as 3 nodes; every reference histogram then shifts.
        second = sorted(os.listdir(self.output))[1]
        with open(os.path.join(self.output, second)) as handle:
            lines = [line.strip() for line in handle if line.strip()]
        self.assertEqual(lines[-1], "0,3,0")


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

    def test_a_one_node_graph_is_reported_as_unrepresentable(self):
        # One node, no edges: a sentinel needs two endpoints, so the count
        # cannot be carried. It must be reported, not silently written as empty.
        rows, _ = self.convert("1\n2\n2\n", "2, 3\n3, 2\n")
        self.assertEqual(rows[0]["num_nodes"], 1)
        self.assertEqual(rows[0]["sentinel"], "no")


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
            # The sentinel is a zero-weight edge, which warns by design.
            warnings.simplefilter("ignore")
            return evolver.load_reference_graphs(self.output, 0)

    def test_graph_count_matches_the_source(self):
        self.assertEqual(len(self.load()), len(EXPECTED))

    def test_node_counts_survive_the_round_trip(self):
        # This is the assertion the sentinel exists for. `load_reference_graphs`
        # returns edges only, and GET sizes a graph as `highest index + 1` --
        # exactly what the loaded edges are checked against here.
        for (source, edges), expected in zip(self.load(), EXPECTED):
            highest = -1
            for low, high, _ in edges:
                if high > highest:
                    highest = high
            self.assertEqual(
                highest + 1, expected["num_nodes"], os.path.basename(source)
            )

    def test_edge_counts_survive_the_round_trip(self):
        for (source, edges), expected in zip(self.load(), EXPECTED):
            real = 0
            for _, _, weight in edges:
                if weight > 0:
                    real += 1
            self.assertEqual(real, expected["num_edges"], os.path.basename(source))

    def test_the_sentinel_adds_no_edge(self):
        # Weight 0 means `Graph::set_edge` writes 0 into the matrix and degree
        # counts only weights above 0, so the padded node stays isolated.
        for source, edges in self.load():
            for low, high, weight in edges:
                if weight == 0:
                    self.assertEqual((low, weight), (0, 0), os.path.basename(source))


if __name__ == "__main__":
    unittest.main()
