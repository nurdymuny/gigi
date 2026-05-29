# gigi-mcp

MCP server for [GIGI](https://davisgeometric.com) — fiber-bundle persistent memory and geometric reasoning for Claude and other MCP-aware LLM clients.

The "Claude as voice, GIGI as brain" pattern, working out of the box.

## Quick start

**Step 1.** Install:

```bash
pip install gigi-mcp
# or, for one-line trial without persistent install:
uvx gigi-mcp
```

**Step 2.** Add to your Claude Desktop config. Open Claude Desktop → Settings → Developer → Edit Config, and add:

```json
{
  "mcpServers": {
    "gigi": {
      "command": "uvx",
      "args": ["gigi-mcp"]
    }
  }
}
```

**Step 3.** Restart Claude Desktop. That's it — Claude can now query GIGI directly.

Out of the box, gigi-mcp connects to the public read-only `gigi-stream.fly.dev` instance, so you can try it before setting up your own GIGI server. To point at your own GIGI instance, set environment variables:

```json
{
  "mcpServers": {
    "gigi": {
      "command": "uvx",
      "args": ["gigi-mcp"],
      "env": {
        "GIGI_URL": "https://your-gigi-instance.example.com",
        "GIGI_API_KEY": "your-key-here"
      }
    }
  }
}
```

## Try it

After restarting Claude Desktop, ask Claude things like:

- *"What datasets are in GIGI?"*
- *"Show me the schema for the cities bundle."*
- *"How many cities have population over 10 million?"*
- *"Use GIGI Query Language to cover the cities bundle where lat > 40."*
- *"Export the cities bundle as DHOOM so I can see the format."*

Claude will discover bundles via `gigi_list_bundles`, examine schemas via `gigi_get_schema`, run filtered queries via `gigi_query_bundle`, construct expressive queries with `gigi_gql`, and export to DHOOM when round-tripping data through other geometric tools.

## What it exposes

| MCP tool | What it does |
|----------|-------------|
| `gigi_list_bundles()` | Discover what bundles exist in this GIGI instance |
| `gigi_get_schema(name)` | Read a bundle's base fields, fiber fields, and indexes |
| `gigi_query_bundle(name, filters, limit)` | Filtered record-level queries |
| `gigi_count(name, filters)` | Fast counts without fetching records |
| `gigi_gql(query)` | Execute GIGI Query Language statements directly |
| `gigi_export_dhoom(name)` | Export a bundle as a DHOOM-formatted string |

### About "GQL"

**The "GQL" in `gigi_gql` is GIGI Query Language — GIGI's own SQL-flavored DSL.** It is *not* GraphQL. GQL statements look like:

```sql
CREATE BUNDLE events BASE (id CATEGORICAL) FIBER (ts TIMESTAMP, val NUMERIC)
INSERT INTO events (id, ts, val) VALUES ('e1', 1700000000, 42.0)
COVER events WHERE val >= 10
SCAN events LIMIT 100
```

Claude reads bundle schemas via `gigi_get_schema` before constructing complex GQL, so the queries it builds are grounded in actual data structure.

## What it doesn't do (yet)

This is **v0** — intentionally minimal but covering the daily-driver surface. The roadmap:

- **v0.1:** Resources (`gigi://bundles`, `gigi://schema/{name}`) so Claude reads schemas as context rather than via tool calls
- **v0.2:** Vector search tool (`gigi_vector_search`) once we settle the embedding-vector serialization shape
- **v0.3:** Aggregation tool (`gigi_aggregate`) for GROUP BY / summary statistics
- **v1.0:** Commercial-tier operations surfaced (`gigi_curvature`, `gigi_spectral`, `gigi_holonomy`, `gigi_transport`) returning `LICENSE_REQUIRED` from the GIGI execution layer for non-commercial callers
- **v1.x:** Streamable HTTP transport, hosted endpoint option

## Configuration

| Env var | Default | Description |
|---------|---------|-------------|
| `GIGI_URL` | `https://gigi-stream.fly.dev` | GIGI instance to connect to |
| `GIGI_API_KEY` | (none) | API key, if your GIGI instance requires one |
| `GIGI_TIMEOUT` | `30` | Request timeout in seconds |

## Why GIGI?

GIGI is a geometric database built on fiber-bundle structure. For LLMs, it provides:

- **Persistent structured memory** with schema that survives serialization
- **Geometric reasoning primitives** — curvature, spectral gap, holonomy, transport (commercial-tier operations; not exposed in v0 of this MCP server)
- **GIGI Query Language** for expressive SQL-flavored queries grounded in fiber-bundle semantics
- **DHOOM-native export** for round-tripping through geomstats and other geometric tooling

See [davisgeometric.com](https://davisgeometric.com) for the broader stack.

## License

MIT — see [LICENSE](../LICENSE).

GIGI itself is free for non-commercial use; commercial deployments are patent-protected (US Provisional Patent Application 64/045,889). This MCP server is free to use unconditionally — gating happens at the GIGI execution layer, not in this server.

## Status

**v0.0.1** — covers the six listed tools against the configured GIGI endpoint. Issues and feature requests welcome at the [main repo](https://github.com/nurdymuny/gigi/issues).
