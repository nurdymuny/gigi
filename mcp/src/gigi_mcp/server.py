"""
gigi-mcp server — FastMCP app + v0 tool definitions.

v0 surface (mapped onto the actual GigiClient API as of sdk/python/gigi v0.5.0):

    gigi_list_bundles()                              — discover what's queryable
    gigi_get_schema(name)                            — read a bundle's structure
    gigi_query_bundle(name, filters, limit)          — record-level filtered query
    gigi_count(name, filters)                        — fast count
    gigi_gql(query)                                  — direct GIGI Query Language execution
    gigi_export_dhoom(name)                          — export a bundle as DHOOM text

The "GQL" in `gigi_gql` is **GIGI Query Language** — GIGI's own SQL-flavored
DSL with statements like `CREATE BUNDLE`, `INSERT INTO`, `COVER WHERE`,
`SCAN`, etc. It is **not** GraphQL. The LLM constructs GQL having discovered
the schema via the bundle/schema tools.

Commercial-tier operations (curvature, spectral, holonomy, transport) are
intentionally NOT exposed in v0 — they require a commercial GIGI license,
and surfacing them through this free MCP server would invite confused
LICENSE_REQUIRED errors. They re-enter the surface in v1 once the gating
story is testable end-to-end.
"""

from __future__ import annotations

from typing import Any

from mcp.server.fastmcp import FastMCP

from .config import get_client


mcp = FastMCP("gigi")


# ─── Discovery ───────────────────────────────────────────────────────────────


@mcp.tool()
def gigi_list_bundles() -> list[dict[str, Any]]:
    """List the bundles (datasets) available in this GIGI instance.

    Use this first when the user asks what data is available or before
    constructing any query. Each bundle's name lets you fetch its schema.

    Returns
    -------
    list of dict
        One entry per bundle with summary info (name, record_count, etc.).
    """
    return get_client().list_bundles()


@mcp.tool()
def gigi_get_schema(name: str) -> dict[str, Any]:
    """Get a bundle's schema — its base fields, fiber fields, and indexes.

    Use this to understand a bundle's structure before querying it. The
    returned dict tells you what fields exist and what types they are, so
    you can construct valid filter expressions or GQL statements.

    Parameters
    ----------
    name : str
        Bundle name (from `gigi_list_bundles()`).

    Returns
    -------
    dict
        Keys: ``base_fields`` (the index/key fields), ``fiber_fields``
        (the value fields), ``indexed_fields`` (which fields have indexes).
    """
    return get_client().schema(name)


# ─── Query ───────────────────────────────────────────────────────────────────


@mcp.tool()
def gigi_query_bundle(
    name: str,
    filters: list[dict[str, Any]] | None = None,
    limit: int = 100,
) -> list[dict[str, Any]]:
    """Query records from a bundle with filter expressions.

    Use this for typed record-level queries. For more expressive queries
    (joins, aggregations, GIGI's geometric operations), use `gigi_gql` instead.

    Parameters
    ----------
    name : str
        Bundle name.
    filters : list of dict, optional
        Each filter has the shape
        ``{"field": <field_name>, "op": <op>, "value": <value>}``.
        Supported ops: ``eq``, ``gt``, ``lt``, ``gte``, ``lte``, ``in``.
        Example: ``[{"field": "population", "op": "gt", "value": 1000000}]``.
    limit : int
        Maximum records to return (default 100). Keep responses bounded.

    Returns
    -------
    list of dict
        Records matching the filters, up to `limit`.
    """
    return get_client().query(name, filters=filters or [], limit=limit)


@mcp.tool()
def gigi_count(
    name: str,
    filters: list[dict[str, Any]] | None = None,
) -> int:
    """Count records in a bundle matching optional filters.

    Faster than `gigi_query_bundle` when you only need the count, not the
    records themselves.

    Parameters
    ----------
    name : str
        Bundle name.
    filters : list of dict, optional
        Same shape as in `gigi_query_bundle`.

    Returns
    -------
    int
        Number of records matching the filters.
    """
    return get_client().count(name, filters=filters or [])


@mcp.tool()
def gigi_gql(query: str) -> dict[str, Any]:
    """Execute a GIGI Query Language statement directly.

    GIGI Query Language is GIGI's own SQL-flavored DSL — not GraphQL.
    Example statements::

        CREATE BUNDLE events BASE (id CATEGORICAL) FIBER (ts TIMESTAMP, val NUMERIC)
        INSERT INTO events (id, ts, val) VALUES ('e1', 1700000000, 42.0)
        COVER events WHERE val >= 10
        SCAN events LIMIT 100

    Use this when filter-based queries (`gigi_query_bundle`) aren't expressive
    enough. The LLM should examine the bundle schema (via `gigi_get_schema`)
    before constructing complex GQL.

    Parameters
    ----------
    query : str
        A GIGI Query Language statement.

    Returns
    -------
    dict
        The query result, shape-dependent on the statement type.
    """
    return get_client().gql(query)


# ─── Export ──────────────────────────────────────────────────────────────────


@mcp.tool()
def gigi_export_dhoom(name: str) -> str:
    """Export a bundle as a DHOOM-formatted string.

    DHOOM (Davis Human-readable Optimized Object Markup) is a compact,
    fiber-bundle-native serialization format — see https://dhoom.dev.
    Use this when the user wants to inspect, share, or archive a bundle
    in its natural geometric form, or when round-tripping data through
    other DHOOM-aware tools (geomstats, etc.).

    Parameters
    ----------
    name : str
        Bundle name.

    Returns
    -------
    str
        The bundle serialized as a DHOOM document.
    """
    return get_client().export_dhoom(name)


# ─── Entry point ─────────────────────────────────────────────────────────────


def main() -> None:
    """Run the gigi-mcp server (stdio transport, MCP default)."""
    mcp.run()


if __name__ == "__main__":
    main()
