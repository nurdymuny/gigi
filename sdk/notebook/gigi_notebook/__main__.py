"""Allow ``python -m gigi_notebook --install`` to work directly."""

from .install import main


if __name__ == "__main__":
    raise SystemExit(main())
