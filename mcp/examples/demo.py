#!/usr/bin/env python
"""Local demo of gigi-mcp tools against the public gigi-stream.fly.dev endpoint.

Calls each of the 6 v0 tools directly as Python functions, prints what comes
back. This is what an LLM (Claude, ChatGPT, etc.) sees when it calls the
tools via MCP — minus the protocol envelope.

Run from anywhere:

    python gigi/mcp/examples/demo.py

The script self-configures sys.path to find both `gigi_mcp` (this package)
and `gigi` (the underlying SDK) when running from a fresh checkout without
having pip-installed either one.

Requires:
    pip install "mcp[cli]" requests websockets
"""

from __future__ import annotations

import io
import os
import sys
import traceback
from pathlib import Path


# ─── Path setup (so we can run without pip install) ──────────────────────────


HERE = Path(__file__).resolve().parent
MCP_ROOT = HERE.parent          # ~/gigi/mcp/
GIGI_ROOT = MCP_ROOT.parent     # ~/gigi/
SDK_PATH = GIGI_ROOT / "sdk" / "python"
SRC_PATH = MCP_ROOT / "src"

for p in (str(SRC_PATH), str(SDK_PATH)):
    if p not in sys.path:
        sys.path.insert(0, p)


# Force UTF-8 stdout (Windows console defaults to cp1252 which can't print
# city names like "Tōkyō", "São Paulo", etc. from the public dataset).
if hasattr(sys.stdout, "buffer"):
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")


# Default to the public read-only endpoint unless caller overrides
os.environ.setdefault("GIGI_URL", "https://gigi-stream.fly.dev")


from gigi_mcp.server import (  # noqa: E402
    gigi_list_bundles,
    gigi_get_schema,
    gigi_query_bundle,
    gigi_count,
    gigi_gql,
    gigi_export_dhoom,
)


# ─── Pretty-print helpers ────────────────────────────────────────────────────


def section(n: int, title: str) -> None:
    print()
    print(f"  {n}. {title}")
    print(f"  {'─' * (len(title) + 4)}")


def trunc(s: str, n: int = 200) -> str:
    if len(s) <= n:
        return s
    return s[: n - 3] + "..."


def run(label: str, fn, *args, **kwargs):
    section_args = ", ".join(
        [repr(a) for a in args] + [f"{k}={v!r}" for k, v in kwargs.items()]
    )
    section(label, f"{fn.__name__}({section_args})")
    try:
        result = fn(*args, **kwargs)
        return result
    except Exception as e:
        print(f"    ✗ {type(e).__name__}: {e}")
        traceback.print_exc(limit=2)
        return None


# ─── Demo ────────────────────────────────────────────────────────────────────


def main() -> None:
    endpoint = os.environ.get("GIGI_URL")
    api_key = os.environ.get("GIGI_API_KEY")

    print("=" * 64)
    print("  gigi-mcp v0 local demo")
    print(f"  endpoint: {endpoint}")
    print(f"  api key:  {'(set via GIGI_API_KEY)' if api_key else '(none)'}")
    print("=" * 64)

    # 0. Health check (always unauthenticated)
    section(0, "GET /v1/health  (unauthenticated)")
    import requests
    try:
        r = requests.get(f"{endpoint}/v1/health", timeout=10)
        r.raise_for_status()
        h = r.json()
        print(f"    → status={h.get('status')!r}, engine={h.get('engine')!r}, "
              f"version={h.get('version')!r}")
        print(f"    → bundles={h.get('bundles')}, "
              f"total_records={h.get('total_records'):,}, "
              f"uptime_secs={h.get('uptime_secs'):,}")
    except Exception as e:
        print(f"    ✗ {type(e).__name__}: {e}")
        print("\n  Endpoint appears unreachable. Aborting.")
        return

    # 1. List bundles (requires auth on gigi-stream.fly.dev)
    bundles = run(1, gigi_list_bundles)
    if bundles is None:
        if not api_key:
            print()
            print("  ── Auth required for the remaining tools ─────────────────")
            print("  gigi-stream.fly.dev gates /v1/bundles and downstream tools")
            print("  behind an API key. To run the rest of the demo, either:")
            print()
            print("    A. Set GIGI_API_KEY in your environment:")
            print("         export GIGI_API_KEY=<your-key>")
            print("         python gigi/mcp/examples/demo.py")
            print()
            print("    B. Point at a local GIGI instance:")
            print("         export GIGI_URL=http://localhost:<port>")
            print("         export GIGI_API_KEY=<your-local-key>  # if needed")
            print("         python gigi/mcp/examples/demo.py")
            print()
        return
    if not bundles:
        print("\n  (endpoint authenticated but returned no bundles)")
        return

    print(f"    → {len(bundles)} bundle(s) returned")
    for b in bundles[:5]:
        if isinstance(b, dict):
            name = b.get("name", "?")
            n = b.get("record_count", b.get("count", "?"))
            print(f"      • {name}  ({n} records)")
        else:
            print(f"      • {b}")

    # Pick a bundle to demo against:
    #   1. honor GIGI_DEMO_BUNDLE env var if set
    #   2. else prefer 'worldfacts' if present (the curated showcase corpus)
    #   3. else first non-system bundle (skip _gigi_* internals)
    #   4. else first bundle of any kind
    preferred = os.environ.get("GIGI_DEMO_BUNDLE")
    names = [b.get("name") if isinstance(b, dict) else str(b) for b in bundles]
    if preferred and preferred in names:
        bundle_name = preferred
    elif "worldfacts" in names:
        bundle_name = "worldfacts"
    else:
        non_system = [n for n in names if not n.startswith("_gigi_")]
        bundle_name = non_system[0] if non_system else names[0]
    print(f"    → demo bundle: {bundle_name!r}")

    # 2. Get schema for that bundle
    schema = run(2, gigi_get_schema, bundle_name)
    if schema:
        if isinstance(schema, dict):
            print(f"    → base_fields: {schema.get('base_fields')}")
            print(f"    → fiber_fields: {schema.get('fiber_fields')}")
            print(f"    → indexed_fields: {schema.get('indexed_fields')}")
        else:
            print(f"    → {trunc(repr(schema))}")

    # 3. Count records
    count = run(3, gigi_count, bundle_name)
    if count is not None:
        print(f"    → {count} record(s)")

    # 4. Query a few records
    records = run(4, gigi_query_bundle, bundle_name, None, 3)
    if records:
        print(f"    → showing {len(records)} record(s):")
        for r in records:
            print(f"      • {trunc(repr(r), 120)}")

    # 5. Direct GQL — use COVER (confirmed valid; the docstring example uses it)
    result = run(5, gigi_gql, f"COVER {bundle_name} LIMIT 1")
    if result is not None:
        print(f"    → {trunc(repr(result), 200)}")

    # 6. DHOOM export
    #    NOTE: `GigiClient.export_dhoom()` currently raises JSONDecodeError because the
    #    endpoint returns plain DHOOM text but the client's `_get` helper tries to
    #    JSON-decode it. SDK-side fix needed before this tool is usable.
    dhoom_text = run(6, gigi_export_dhoom, bundle_name)
    if isinstance(dhoom_text, str):
        lines = dhoom_text.splitlines()
        print(f"    → DHOOM export: {len(lines)} lines, {len(dhoom_text)} chars")
        print(f"    → first 4 lines:")
        for line in lines[:4]:
            print(f"      | {line}")
    elif dhoom_text is not None:
        print(f"    → unexpected return type: {type(dhoom_text).__name__}")

    print()
    print("=" * 64)
    print("  done")
    print("=" * 64)


if __name__ == "__main__":
    main()
