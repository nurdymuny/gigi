"""gigi-notebook — Jupyter kernel for GIGI.

A thin kernel that lets you write GQL queries in notebook cells against a
running gigi-stream server, with cell magics for the verb-shaped HTTP
endpoints (commutator, transport, etc.) and custom rich renderers for
GIGI's structured response types.

See ``GigiKernel`` for the kernel implementation and ``install.main`` for
the kernelspec installer.
"""

from .kernel import GigiKernel

__version__ = "0.1.0"
__all__ = ["GigiKernel", "__version__"]
