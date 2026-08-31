"""Checks that `python/get/__init__.pyi` describes the module that ships beside it.

    python3 -m unittest discover -s tools -v

Needs the built `get` module and skips with a message when it is not importable,
like the converter's round-trip test -- this lives outside the crate, so
`cargo test` does not build it and a plain checkout has no wheel.

**This is not the same check as `generate_stubs.py --check`.** That one asks
whether regenerating produces the file already in the tree, which catches a
signature edited in Rust and not regenerated. It cannot catch a defect in the
generator itself: if the generator omits something, it omits it from both sides
and the comparison passes. These tests read the stub as text and the module by
introspection, so a generator that drops information has to disagree with one of
them.

The base-class test exists because that defect actually shipped. Every complex
enum's variants are real subclasses at runtime, the stub declared them as bare
nested classes, and every `get.Config(evolution=get.EvolutionConfig.Generational(...))`
-- the shape both documented Python routes use -- was a type error nobody saw,
because the generated file and the regenerated file agreed with each other.
"""

import ast
import os
import types
import unittest

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
STUB = os.path.join(REPO, "python", "get", "__init__.pyi")

try:
    import get
except ImportError:  # pragma: no cover - the skip path
    get = None  # type: ignore[assignment]


def stub_tree():
    with open(STUB, encoding="utf-8") as handle:
        return ast.parse(handle.read())


def stub_classes(node, prefix=""):
    """Every class in the stub, by dotted path."""
    found = {}
    for child in node.body:
        if isinstance(child, ast.ClassDef):
            path = f"{prefix}{child.name}"
            found[path] = child
            found.update(stub_classes(child, f"{path}."))
    return found


def module_classes(obj, prefix=""):
    """Every class reachable from the module, by the same dotted path."""
    found = {}
    names = getattr(obj, "__all__", None) or [n for n in dir(obj) if not n.startswith("_")]
    for name in names:
        value = getattr(obj, name, None)
        if isinstance(value, type):
            path = f"{prefix}{name}"
            found[path] = value
            for attr in sorted(vars(value)):
                nested = vars(value)[attr]
                if isinstance(nested, type):
                    found[f"{path}.{attr}"] = nested
    return found


def signature_params(text):
    """Parameter names from a `__text_signature__`, receiver dropped."""
    inner = text.strip()[1:-1].strip()
    for marker in ("$self", "$type"):
        if inner.startswith(marker):
            inner = inner[len(marker):].lstrip(", ")
    if not inner:
        return []
    parsed = ast.parse(f"def _({inner}): ...").body[0]
    assert isinstance(parsed, ast.FunctionDef)
    return [a.arg for a in parsed.args.args]


def stub_function(cls_node, name):
    for child in cls_node.body:
        if isinstance(child, ast.FunctionDef) and child.name == name:
            return child
    return None


@unittest.skipIf(get is None, "the `get` module is not built; run `maturin develop --release`")
class StubMatchesModuleTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.stub = stub_classes(stub_tree())
        cls.module = module_classes(get)

    def test_every_module_class_appears_in_the_stub(self):
        missing = sorted(set(self.module) - set(self.stub))
        self.assertEqual([], missing, "classes the module exports and the stub omits")

    def test_the_stub_invents_no_class(self):
        extra = sorted(set(self.stub) - set(self.module))
        self.assertEqual([], extra, "classes in the stub that the module does not have")

    def test_base_classes_match(self):
        """A variant passed where its enum is expected must type-check."""
        for path, cls in sorted(self.module.items()):
            with self.subTest(path=path):
                live = [b.__name__ for b in cls.__bases__ if b is not object]
                declared = [ast.unparse(b) for b in self.stub[path].bases]
                self.assertEqual(live, declared)

    def test_constructor_parameters_match(self):
        for path, cls in sorted(self.module.items()):
            signature = getattr(cls, "__text_signature__", None)
            if signature is None:
                self.assertIsNone(
                    stub_function(self.stub[path], "__init__"),
                    f"{path} has no constructor but the stub declares one",
                )
                continue
            with self.subTest(path=path):
                init = stub_function(self.stub[path], "__init__")
                self.assertIsNotNone(init, f"{path} is constructible and the stub says otherwise")
                self.assertEqual(
                    ["self"] + signature_params(signature),
                    [a.arg for a in init.args.args],
                )

    def test_methods_and_properties_match(self):
        for path, cls in sorted(self.module.items()):
            live = set()
            for attr, value in vars(cls).items():
                if attr.startswith("__") or isinstance(value, type):
                    continue
                if isinstance(value, (types.GetSetDescriptorType, types.MethodDescriptorType,
                                      types.BuiltinFunctionType, types.WrapperDescriptorType,
                                      staticmethod, classmethod)):
                    live.add(attr)
            declared = {
                child.name
                for child in self.stub[path].body
                if isinstance(child, ast.FunctionDef) and child.name != "__init__"
            }
            with self.subTest(path=path):
                self.assertEqual(live, declared)


class StubIsFullyAnnotatedTest(unittest.TestCase):
    """Reads the stub only, so it runs without the module built.

    `__text_signature__` carries no types, so annotations are written by hand and
    survive regeneration. A parameter added in Rust arrives here unannotated and
    silently typed `Any` -- which is the state the whole exercise exists to leave
    behind, and nothing else would report it.
    """

    def setUp(self):
        self.stub = stub_classes(stub_tree())

    def test_every_parameter_is_annotated(self):
        for path, node in sorted(self.stub.items()):
            for child in node.body:
                if not isinstance(child, ast.FunctionDef):
                    continue
                for arg in child.args.args:
                    if arg.arg == "self":
                        continue
                    with self.subTest(function=f"{path}.{child.name}", parameter=arg.arg):
                        self.assertIsNotNone(arg.annotation)

    def test_every_function_declares_a_return_type(self):
        for path, node in sorted(self.stub.items()):
            for child in node.body:
                if isinstance(child, ast.FunctionDef):
                    with self.subTest(function=f"{path}.{child.name}"):
                        self.assertIsNotNone(child.returns)

    def test_every_class_and_member_is_documented(self):
        """PyO3 drops `///` on complex-enum variants, so the stub is their only home."""
        for path, node in sorted(self.stub.items()):
            with self.subTest(path=path):
                self.assertIsNotNone(ast.get_docstring(node))
            for child in node.body:
                if isinstance(child, ast.FunctionDef) and child.name != "__init__":
                    with self.subTest(member=f"{path}.{child.name}"):
                        self.assertIsNotNone(ast.get_docstring(child))


if __name__ == "__main__":
    unittest.main()
