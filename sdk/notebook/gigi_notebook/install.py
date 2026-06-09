"""Install the GIGI kernelspec so Jupyter can discover this kernel.

Usage::

    python -m gigi_notebook --install            # user-scope install
    python -m gigi_notebook --install --user     # explicit (same default)
    python -m gigi_notebook --install --prefix=/path/to/env
    python -m gigi_notebook --uninstall

The kernelspec is a small JSON file plus optional logos that tells
Jupyter how to launch this kernel. We generate it in a temp dir and hand
it to ``jupyter_client.kernelspec.KernelSpecManager`` to install in the
right scope.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import sys
import tempfile
from pathlib import Path
from typing import List


KERNEL_NAME = "gigi"
DISPLAY_NAME = "GIGI (GQL)"


def _build_kernelspec(python_executable: str) -> dict:
    """The JSON object Jupyter reads to launch this kernel."""
    return {
        "argv": [
            python_executable,
            "-m",
            "gigi_notebook.kernel",
            "-f",
            "{connection_file}",
        ],
        "display_name": DISPLAY_NAME,
        "language": "gql",
        "metadata": {
            "debugger": False,
            "kernel_provisioner": {"provisioner_name": "local-provisioner"},
        },
    }


def install(prefix: str | None, user: bool) -> Path:
    """Install the kernelspec; return the path it was installed to."""
    try:
        from jupyter_client.kernelspec import KernelSpecManager
    except ImportError as e:
        raise RuntimeError(
            "jupyter_client not installed. Install it (or jupyterlab / "
            "notebook) before running --install."
        ) from e

    with tempfile.TemporaryDirectory() as tmpdir:
        spec_dir = Path(tmpdir) / KERNEL_NAME
        spec_dir.mkdir()
        spec_path = spec_dir / "kernel.json"
        spec_path.write_text(
            json.dumps(_build_kernelspec(sys.executable), indent=2),
            encoding="utf-8",
        )

        manager = KernelSpecManager()
        installed_to = manager.install_kernel_spec(
            str(spec_dir),
            kernel_name=KERNEL_NAME,
            prefix=prefix,
            user=user and prefix is None,
            replace=True,
        )
        return Path(installed_to)


def uninstall() -> None:
    """Remove a previously installed kernelspec, if present."""
    try:
        from jupyter_client.kernelspec import KernelSpecManager
    except ImportError as e:
        raise RuntimeError("jupyter_client not installed.") from e

    manager = KernelSpecManager()
    try:
        manager.remove_kernel_spec(KERNEL_NAME)
        print(f"Removed kernelspec {KERNEL_NAME!r}.")
    except KeyError:
        print(f"No kernelspec named {KERNEL_NAME!r} was installed.")


def main(argv: List[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="gigi-notebook")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument(
        "--install",
        action="store_true",
        help="Install the GIGI kernelspec so Jupyter can discover it.",
    )
    group.add_argument(
        "--uninstall",
        action="store_true",
        help="Remove a previously installed GIGI kernelspec.",
    )
    parser.add_argument(
        "--user",
        action="store_true",
        default=True,
        help="User-scope install (default).",
    )
    parser.add_argument(
        "--prefix",
        default=None,
        help="Install into <prefix>/share/jupyter/kernels/ (overrides --user).",
    )
    args = parser.parse_args(argv)

    if args.install:
        dest = install(prefix=args.prefix, user=args.user)
        print(
            f"Installed GIGI kernelspec to:\n  {dest}\n"
            f"Launch JupyterLab and pick 'GIGI (GQL)' from the kernel menu."
        )
        return 0
    elif args.uninstall:
        uninstall()
        return 0
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
