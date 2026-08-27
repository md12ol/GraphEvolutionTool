# Re-exports the compiled extension module so `import get` gives the classes
# directly rather than `get.get.GraphEvolver`.
#
# maturin generates exactly this file when the package has no Python source of
# its own. It is written out here because the package now *does* have Python
# source — `py.typed` and `__init__.pyi`, which an editor needs beside the
# module — and declaring `python-source` means maturin stops generating it.
from .get import *

__doc__ = get.__doc__
if hasattr(get, "__all__"):
    __all__ = get.__all__
