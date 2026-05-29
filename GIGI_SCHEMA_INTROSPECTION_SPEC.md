# GIGI Schema Introspection — Specification

**Owner:** Bee Rosa Davis / Davis Geometric
**Drafted:** 2026-05-24
**Status:** v0.2 DRAFT — architectural revision. v0.1 specced a single global `/schema` endpoint returning GraphQL SDL; that model was wrong. GIGI's actual schema introspection is **per-bundle** (`GET /v1/bundles/{name}/schema`) plus a **bundle list** (`GET /v1/bundles`). v0.2 documents the existing endpoints and adds license-tier annotation as the only new piece.
**Scope:** Documents the publicly-accessible schema introspection surface that GIGI Lang's translator, MCP clients, the geomstats GIGI loader, and any external integration depend on. Most of what this spec describes *already exists* in the GIGI server and the Python SDK; the new piece is a `license_tier` annotation on schema elements so callers know which operations require a commercial license.

---

## What changed from v0.1

| v0.1 (wrong) | v0.2 (corrected) |
|--------------|------------------|
| Single `GET /schema` endpoint returning GraphQL SDL | Existing `GET /v1/bundles` + `GET /v1/bundles/{name}/schema` (already implemented) |
| GraphQL SDL as canonical format with `@public` / `@gated` directives | JSON dicts (GIGI's native format) with a `license_tier` field on each element |
| Designed around a single global schema document | Per-bundle schemas; bundle list endpoint for discovery |
| Required Apollo/GraphQL ecosystem tooling | Works with standard HTTP/JSON clients today |

The architectural principles from v0.1 (no auth on introspection, show-everything-gate-at-execution, cacheable, agent-friendly) all carry forward. The wire format was the wrong part, not the strategy.

---

## 1. What this spec is for

The translator inside GIGI Lang needs to know what queries are valid. External tools (MCP servers, the geomstats GIGI loader, any agent integration) need to know what data shapes GIGI returns. Both need a stable, machine-readable description of *what's queryable.*

The existing GIGI server already provides this. v0.2 documents it as the canonical public surface and adds a single annotation (`license_tier`) so callers can distinguish free non-commercial operations from gated commercial ones.

This is **publication, not new functionality.** The endpoints exist; the annotation is the only new piece.

---

## 2. Design principles (unchanged from v0.1)

### G1. Discoverable without authentication
Anyone — researcher, agent, casual visitor — can hit the introspection endpoints and learn what GIGI exposes. No API key, no signup, no rate-limit-by-default. Commercial gating happens at the *query execution* layer, not at the *schema discovery* layer.

### G2. Machine-first, human-readable second
The primary consumers are translation layers, codegen tools, and SDK builders. JSON is the native format (matches what GIGI already returns). DHOOM is available via content negotiation when geometric structure matters to the consumer.

### G3. Show everything; gate at execution
Both public and gated capabilities appear in the schema. Gated capabilities are *marked* (so callers know they exist and can request commercial licensing), but the schema never hides them. The "I didn't know that was possible" problem is solved at the schema layer; the "can I actually call this" question is answered at the execution layer.

### G4. Cacheable
Schemas change slowly. ETag + Last-Modified headers let clients cache aggressively.

### G5. Self-describing
Schema responses include enough metadata (field names, types, indexes, license tiers) that a consumer never needs a separate documentation source to interpret them.

---

## 3. Existing endpoints

These are already implemented in the GIGI server and exercised by the Python SDK.

### 3.1 `GET /v1/bundles` — list bundles

**Already implemented.** Exercised by `GigiClient.list_bundles()`.

Returns a JSON array of bundle summaries:

```json
[
  {
    "name": "cities",
    "record_count": 50,
    "fields": {
      "city": "categorical",
      "lat": "numeric",
      "lng": "numeric",
      "population": "numeric"
    },
    "license_tier": "public"
  },
  ...
]
```

**v0.2 addition:** the `license_tier` field on each bundle entry. Tier is "public" by default; bundles that exist only behind a commercial license can be marked "commercial" so the schema lists them but execution refuses.

### 3.2 `GET /v1/bundles/{name}/schema` — per-bundle schema

**Already implemented.** Exercised by `GigiClient.schema(name)`.

Returns a JSON object describing one bundle's structure:

```json
{
  "name": "cities",
  "base_fields": [
    {"name": "city", "type": "categorical", "license_tier": "public"}
  ],
  "fiber_fields": [
    {"name": "lat", "type": "numeric", "license_tier": "public"},
    {"name": "lng", "type": "numeric", "license_tier": "public"},
    {"name": "population", "type": "numeric", "license_tier": "public"}
  ],
  "indexed_fields": ["city"],
  "license_tier": "public"
}
```

**v0.2 addition:** `license_tier` on the bundle itself, on each field, and on derived operations available for that bundle.

### 3.3 `GET /v1/operations` — available operations (proposed for v0.2)

**Not yet implemented.** This is the one new endpoint v0.2 proposes — a registry of operations callable across all bundles, each with its license tier.

```json
[
  {"name": "query", "license_tier": "public", "description": "Filter records by field conditions"},
  {"name": "count", "license_tier": "public", "description": "Count records matching filters"},
  {"name": "vector_search", "license_tier": "public", "description": "k-NN over stored embeddings"},
  {"name": "gql", "license_tier": "public", "description": "Execute a GIGI Query Language statement"},

  {"name": "curvature", "license_tier": "commercial", "description": "Kähler curvature decomposition (L4)"},
  {"name": "spectral_gap", "license_tier": "commercial", "description": "Cached spectral analysis (L3)"},
  {"name": "transport", "license_tier": "commercial", "description": "B-perturbed transport (L1.5)"},
  {"name": "hadamard_regions", "license_tier": "commercial", "description": "Hadamard substructure detection (L5)"},
  {"name": "morse_compress", "license_tier": "commercial", "description": "Morse compression (L6)"},
  {"name": "holonomy_debt", "license_tier": "commercial", "description": "Holonomy accumulation tracking"},
  {"name": "encode_chern", "license_tier": "commercial", "description": "Chern character encoding (L7.3)"},
  {"name": "quantum_cohomology", "license_tier": "commercial", "description": "Quantum cohomology (L7)"},
  {"name": "toeplitz_operator", "license_tier": "commercial", "description": "Berezin-Toeplitz operators (L7)"}
]
```

This is the only endpoint v0.2 *adds*. It complements the per-bundle schema with a global operations catalog so callers can see what's possible across the whole server without enumerating bundles.

---

## 4. License-tier annotation

The single new concept v0.2 introduces. Three values defined now (with `research` reserved for a possible future tier):

| Tier | Meaning |
|------|---------|
| `public` | Free for non-commercial use. The default. |
| `commercial` | Requires a commercial license to *execute.* Visible to everyone in the schema. |
| `research` *(reserved)* | Available to verified academic/research users. Not yet implemented; declared so it's available later. |

Schema elements that may carry a `license_tier`:
- Bundles (`/v1/bundles[].license_tier`)
- Individual fields within a bundle (`/v1/bundles/{name}/schema.base_fields[].license_tier`)
- Operations in the catalog (`/v1/operations[].license_tier`)
- The bundle's own derived operations (e.g., a `manifold_structure` accessor on a Kähler-bundle would carry `license_tier: "commercial"`)

**Enforcement model:** the schema is open; calling a `commercial`-tier operation without a valid commercial license returns a unified error envelope with `category: "license_required"` from the execution layer, not from the schema layer. The schema's job is to communicate the boundary, not to enforce it.

---

## 5. The license boundary tracks the geometric boundary

The `public` / `commercial` split is not arbitrary. It tracks where the engine's actual IP lives.

**Public-tier operations are infrastructure** — computationally cheap, format-conversion-only, or universally useful:

- Record CRUD (create / read / update / delete bundles + records)
- Filter-based record queries
- Counts
- Basic vector search (similarity, k-NN over stored embeddings)
- Schema introspection itself
- GIGI Query Language (the language is public; specific GQL statements that invoke commercial operations are gated at execution)
- Bundle export to JSON or DHOOM
- Subscriptions to standard event streams

**Commercial-tier operations are research** — they depend on the engine's serious geometric machinery (Davis Field Equation, Kähler upgrades, L1–L7 work):

| Operation | Layer | What it does |
|-----------|-------|--------------|
| `curvature` | L4 | Kähler curvature decomposition |
| `spectral_gap` | L3 | Cached spectral analysis |
| `transport` | L1.5 | B-perturbed transport |
| `hadamard_regions` | L5 | Hadamard substructure detection |
| `morse_compress` | L6 | Morse compression |
| `holonomy_debt` | — | Holonomy accumulation tracking |
| `encode_chern` | L7.3 | Chern character encoding |
| `quantum_cohomology` | L7 | Quantum cohomology operations |
| `toeplitz_operator` | L7 | Berezin-Toeplitz operators |

The license boundary tracks the IP boundary because that's how the engine is structured: classical fiber-bundle operations are *infrastructure;* geometrically-novel operations are *research.* Mapping `commercial` to "where the research lives" makes the licensing posture legible to a reader of the schema in the same way the math is legible to a reader of the framework papers — both surfaces are honest about where the real work is.

This also means the license-tier registry doesn't need clever logic. The annotation follows a simple rule: if the operation needs L1–L7 machinery, it's `commercial`; otherwise it's `public`. Adding new public operations is friction-free; adding new commercial operations is rare (the L-series cadence is the rate-of-arrival).

---

## 6. Content negotiation (optional, lower priority for v0.2)

JSON is the canonical and default format — it's what GIGI already returns. DHOOM is available via `Accept: application/dhoom` for callers who want fiber-shaped responses for downstream geometric tooling.

| `Accept` header | Response format |
|-----------------|-----------------|
| `application/json` (default) | JSON dict, as documented above |
| `application/dhoom` | DHOOM-shaped (single bundle with nested bundles per type) |

DHOOM serialization of the schema isn't blocking for v0.2 — JSON is sufficient for all currently-planned consumers (GIGI Lang translator, MCP server, geomstats loader). DHOOM support can land in v0.3 when there's a concrete consumer that benefits.

---

## 7. Open questions

1. **`/v1/operations` endpoint — implement now or defer?** The per-bundle schema already exists; the operations catalog is the one piece this spec *adds.* It's small. Lean toward implementing now so callers don't have to crawl every bundle to discover what operations are available.

2. **License-tier annotation rollout.** The annotation can be added incrementally: start with the bundle-level tier, then per-field, then operations. What's the order? Recommendation: bundle-level first (smallest change), then operations catalog (`/v1/operations`), then field-level last (most granular, most diff).

3. **Bundle-level vs. operation-level gating.** Some operations (e.g., `curvature`) are universally commercial regardless of bundle. Others (e.g., a specific bundle's content) might be commercial because the *bundle* is gated, not because the operation is. Make sure both axes are representable: a `commercial` bundle blocks all operations on it; a `commercial` operation is blocked regardless of bundle.

4. **Caching and staleness.** ETag + Last-Modified on `/v1/bundles` and `/v1/bundles/{name}/schema`. Refresh cadence — schemas don't change minute-to-minute, so a 5-minute browser cache + ETag revalidation should be fine.

5. **`research` tier — declare or defer?** Reserved in §4 but not yet defined. Lean toward declaring the enum value now so clients pattern-match permissively from day one; the actual enforcement comes later.

6. **Deprecation handling.** When a field or operation is being phased out, a `deprecated: true` boolean (plus optional `deprecation_reason`) on the schema element. Standard pattern; small to add. Worth specifying now to avoid retrofitting later.

---

## 8. Implementation notes (sketch, non-binding)

- The per-bundle endpoints exist. The only genuinely new code is the `license_tier` field on existing responses and the `/v1/operations` endpoint.
- License-tier registry can be a small static config file initially (per-bundle and per-operation annotations); could grow into a database-backed thing later if tier rules become more dynamic.
- The annotation should be added in one PR to GIGI server side (Rust) plus one PR to update the Python SDK to surface the field (`GigiClient.schema()` returns the new field as part of the dict).

---

## 9. References

- Companion spec: [GIGI_LANG_SPEC.md](GIGI_LANG_SPEC.md) (v0.1.2)
- Existing client methods: `GigiClient.list_bundles()`, `GigiClient.schema(name)` in `sdk/python/gigi/client.py`
- Prerequisite P3: `~/Documents/davis-contributions/PRE_REQS.md`
- DHOOM specification: https://dhoom.dev
- Davis Geometric licensing philosophy: applies — schema visibility is free for everyone; execution of `commercial`-tier elements requires commercial license

---

## 10. What to do with this spec

This is **v0.2, architecturally corrected.** It specifies the schema introspection surface as it actually is (mostly) and what one piece needs to be added (license-tier annotation + `/v1/operations`).

**Next steps:**
- Resolve the 6 open questions in §7 (most are small)
- Add `license_tier` to existing `/v1/bundles` and `/v1/bundles/{name}/schema` responses
- Implement `/v1/operations` endpoint
- Update the GIGI Python SDK to surface the new fields
- Update the GIGI Lang SDK skeleton (`lang.py`)'s `schema()` method docstring to reflect this real shape
- Announce in the MCP server docs (venue 04 in the contribution plan) once the annotation rollout is complete
