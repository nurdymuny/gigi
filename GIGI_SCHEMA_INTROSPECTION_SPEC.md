# GIGI Schema Introspection Endpoint — Specification

**Owner:** Bee Rosa Davis / Davis Geometric
**Drafted:** 2026-05-24
**Status:** v0.1 DRAFT — endpoint design and response contract. Companion to [GIGI_LANG_SPEC.md](GIGI_LANG_SPEC.md) and prerequisite P3 in `~/Documents/davis-contributions/PRE_REQS.md`.
**Scope:** Specifies the publicly-accessible endpoint that exposes GIGI's GraphQL schema for use by GIGI Lang's translator, MCP clients, the geomstats GIGI loader, and any external integration. Does not change what the schema *is* — only how it's published.

---

## 1. What this endpoint is for

GIGI Lang's translator needs to know what queries are valid before it can compile a prompt to GQL. External tools (MCP servers, the geomstats GIGI loader, any agent integration) need to know what data shapes GIGI returns. Both need a stable, machine-readable surface for *what's queryable.*

The endpoint provides that surface. One URL, one response, current as of the time of the request. It's the smallest possible thing that unblocks everything in §5 of GIGI_LANG_SPEC and several venues in the davis-contributions master plan.

This is **not** a new feature of GIGI — it's the publication of an existing internal capability (schema introspection that the engine already uses internally) as a stable public surface.

---

## 2. Design goals

### G1. Discoverable without authentication.
Anyone — researcher, agent, casual visitor — can hit the endpoint and learn what GIGI exposes. No API key, no signup, no rate-limit-by-default. The schema is a public artifact; commercial gating happens at the *query execution* layer, not at the *schema discovery* layer.

### G2. Machine-first, human-readable second.
The primary consumers are translation layers, codegen tools, and SDK builders. Response formats favor what machines parse cleanly. Human readability is a derived benefit (which DHOOM happens to provide for free).

### G3. Show everything; gate at execution.
Both public and gated capabilities appear in the schema. Gated capabilities are *marked* (so callers know they exist and can request commercial licensing), but the schema itself never hides them. This is the inverse of the typical "show what you can use" pattern — it's "show what exists, and what's behind which door."

### G4. Cacheable.
The schema doesn't change often. Standard HTTP caching (ETag, Last-Modified, immutable + versioned URLs for major versions) lets clients cache aggressively.

### G5. Self-describing across formats.
Available as GraphQL SDL (the standard), DHOOM (the geometry-native), and JSON introspection (the GraphQL ecosystem-compatible). Content negotiation picks the right one.

---

## 3. Endpoint design

### URL

```
GET /schema
```

Lives at the GIGI server root, sibling to the existing `/curvature`, `/spectral_gap`, `/betti`, etc.

### Methods

`GET /schema` — return the current schema in the requested format. Default is SDL.

`HEAD /schema` — return only headers (ETag, Last-Modified, Schema-Version), for cache validation.

No `POST` / `PUT` / `DELETE` — schema is published, not mutated through this endpoint.

### Authentication

**None required.** This is the public surface. (Commercial-license enforcement happens at the query-execution layer; discovery is open.)

### Content negotiation

| `Accept` header | Response |
|-----------------|----------|
| `application/graphql` (default) | GraphQL SDL — the canonical format |
| `application/dhoom` | DHOOM-shaped, fiber-bundle native |
| `application/json` | GraphQL introspection JSON — for tools expecting the standard introspection-query shape |
| (omitted) | SDL |

### Versioning

Schema version returned in the `Schema-Version: vMAJOR.MINOR` response header *and* embedded in the body. Major version changes are breaking; minor are additive.

Versioned URLs available for pinning:
```
GET /schema/v1
GET /schema/v1.3
```

The unversioned `/schema` returns the latest. Clients that want stability pin a specific version.

---

## 4. What's public vs. gated

Every type, field, and verb in the schema carries a directive or metadata field indicating its license tier:

- `@public` — free for non-commercial use (the default — most things)
- `@gated(tier: "commercial")` — requires commercial license to call
- `@research_only` — available to verified academic / research users (a possible future tier)

The schema **shows everything.** A type marked `@gated` is fully described in the schema; callers can see what it does, what arguments it takes, what it returns. They simply cannot execute it without the appropriate license token.

This is deliberate: hiding things creates the "I didn't know that was possible" problem. Showing-with-gating creates the "I know exactly what I'd be licensing if I needed it" experience — which is the right experience for someone potentially becoming a commercial customer.

### Example fragment

```graphql
type City @public {
  name: String!
  lat: Float!
  lng: Float!
  population: Int
}

type ManifoldStructure @gated(tier: "commercial") {
  fiber: Fiber!
  curvature: Tensor!
  holonomyDebt: Float
}

type Query {
  city(name: String!): City @public
  cities(near: String, limit: Int): [City!]! @public
  manifoldOf(dataset: String!): ManifoldStructure @gated(tier: "commercial")
}
```

A non-commercial user can call `city` and `cities` freely. Calling `manifoldOf` returns an error from the *execution* layer (not the schema layer) with a category of `LICENSE_REQUIRED` and a pointer to davisgeometric.com for licensing.

### The license boundary tracks the geometric boundary

The `@public` / `@gated` split is not arbitrary — it tracks where the engine's actual IP lives. Operations that are computationally cheap, format-conversion-only, or infrastructure-level naturally classify as `@public`:

- Record CRUD (create / read / update / delete bundles + records)
- Basic vector search (similarity, k-NN over stored embeddings)
- Schema introspection itself
- Subscriptions to standard event streams

Operations that depend on the engine's serious geometric machinery — the Davis Field Equation, the Kähler upgrades, the fiber-bundle work shipped through L1–L7 — naturally classify as `@gated(tier: "commercial")`:

- `curvature(bundle)` — Kähler curvature decomposition (L4)
- `spectral_gap(bundle)` — cached spectral analysis (L3)
- `transport(seg, B, ...)` — B-perturbed transport (L1.5)
- `hadamard_regions(bundle)` — Hadamard substructure detection (L5)
- `morse_compress(bundle)` — Morse compression (L6)
- `holonomy_debt(bundle)` — accumulated non-associativity tracking
- `encode_chern(bundle)` — Chern character encoding (L7.3)
- `quantum_cohomology(bundle)` — quantum cohomology operations (L7)
- `toeplitz_operator(bundle)` — Berezin-Toeplitz operators (L7)

The license boundary tracks the IP boundary because that's how the engine is structured: classical fiber-bundle operations are infrastructure; geometrically-novel operations are research. Mapping `@gated` to "where the research actually lives" makes the licensing posture legible to a reader of the schema in the same way the math is legible to a reader of the framework papers — both surfaces are honest about where the real work is.

This also means the gating registry (§9 implementation note) doesn't need clever logic. The directive applied to each schema element follows a simple rule of thumb: if the operation needs L1–L7 machinery, it's `@gated`; otherwise it's `@public`. Adding new public operations is friction-free; adding new gated operations is rare (the L-series cadence is the rate-of-arrival).

---

## 5. Response shape (SDL is canonical)

When responding with SDL (the default), the body is GraphQL SDL with Davis Geometric-specific directives declared:

```graphql
# Davis Geometric directives
directive @public on FIELD_DEFINITION | OBJECT | INTERFACE | ENUM
directive @gated(tier: String!) on FIELD_DEFINITION | OBJECT | INTERFACE | ENUM
directive @research_only on FIELD_DEFINITION | OBJECT | INTERFACE | ENUM

# Schema metadata
schema {
  query: Query
  mutation: Mutation
  subscription: Subscription
}

# ... full type definitions ...
```

Plus standard headers:
```
Schema-Version: v1.3.2
ETag: "abc123def456"
Last-Modified: Wed, 24 May 2026 12:34:56 GMT
Cache-Control: public, max-age=3600
Content-Type: application/graphql
```

---

## 6. Examples

### Example 1 — fetch SDL

```
$ curl https://gigi-stream.fly.dev/schema
# Davis Geometric directives
directive @public on FIELD_DEFINITION | ...
# Schema-Version: v1.3.2
...
```

### Example 2 — fetch DHOOM-shaped

```
$ curl -H "Accept: application/dhoom" https://gigi-stream.fly.dev/schema
schema{name, version, types>, queries>}:
gigi-stream, v1.3.2,
  {name, kind, fields>, license|public}:
  City, OBJECT, ...
...
```

### Example 3 — cache validation

```
$ curl -I -H "If-None-Match: \"abc123def456\"" https://gigi-stream.fly.dev/schema
HTTP/1.1 304 Not Modified
```

### Example 4 — pinned version for stability

```
$ curl https://gigi-stream.fly.dev/schema/v1
(returns the latest v1.x.y, never a v2.x.y; client code written against v1 stays working)
```

---

## 7. Integration points

### 7.1 GIGI Lang translator
On startup (or on cache miss), the translator fetches `/schema` with `Accept: application/graphql`, parses the SDL, builds a structured representation of valid queries, and uses it to ground prompt → GQL translations. Caches with ETag; refetches on cache invalidation.

### 7.2 GIGI Python SDK
The `GigiLang.schema()` method (see `lang.py`) calls this endpoint. Returns a `Schema` object (parsed) or raw SDL on request.

### 7.3 MCP server (venue 04)
Exposes `gigi_schema()` as an MCP tool that any LLM client can invoke when it needs to know what's queryable.

### 7.4 Geomstats GIGI loader (venue 02)
The loader's docs link to this endpoint so users can discover what `load_gigi(query)` can fetch.

### 7.5 GraphQL ecosystem tools
Tools expecting the GraphQL introspection query (Apollo Studio, GraphQL Playground, GraphiQL, codegen tools) work out of the box via `Accept: application/json` content negotiation.

---

## 8. Open questions

1. **Authentication on `/schema` itself.** G1 says no auth. Is there a case where even schema visibility should be gated (e.g., paid-tier-only types that competitors shouldn't even know exist)? Default to "no, show everything" but worth a sanity check.

2. **Rate-limiting on `/schema`.** Probably negligible traffic (cached aggressively), but unauthenticated public endpoints attract scanners and crawlers. Light rate limit (60 req/min per IP?) is sensible; aggressive limits would defeat the discoverability goal.

3. **Schema staleness window.** How fresh does the schema need to be? Real-time (regenerated on each request) is wasteful. Hourly (refreshed by a background job) seems sufficient. Specifying this prevents subtle bugs where a client gets a stale schema and the query it constructs fails at execution.

4. **DHOOM serialization shape.** SDL is the standard, well-defined. Introspection JSON is well-defined. What does "schema as DHOOM" look like exactly? Probably a single bundle with nested bundles per type, fields as records. Worth a small design exercise; not blocking.

5. **`@research_only` tier — yes/no?** Mentioned as a possible future. Decide whether to declare the directive now (for forward compatibility) or wait until the tier exists in practice. Lean toward declaring now; cheap to do, hard to retrofit.

6. **Deprecation handling.** When a type or field is deprecated, GraphQL's `@deprecated(reason: ...)` directive applies. Confirm this is honored in the SDL output and that GIGI Lang's translator avoids deprecated paths in its generated GQL.

---

## 9. Implementation notes (sketch, non-binding)

- GIGI already has internal schema introspection (the engine uses it to validate incoming GQL); this endpoint is the *publication* of that capability, not new functionality.
- Recommend implementing as a thin handler that calls the existing introspection, applies output formatting based on `Accept`, attaches `@public` / `@gated` directives from a license-tier registry (which is the only genuinely new piece).
- License-tier registry is a small static config initially (per-type/per-field annotations); could grow into a database-driven thing later if the tier structure becomes more dynamic.

---

## 10. References

- Companion spec: [GIGI_LANG_SPEC.md](GIGI_LANG_SPEC.md) (v0.1.1)
- Prerequisite P3: `~/Documents/davis-contributions/PRE_REQS.md`
- GraphQL SDL specification: https://spec.graphql.org/ (current edition)
- GraphQL introspection query: same spec, §4.5
- DHOOM specification: https://dhoom.dev
- Davis Geometric licensing philosophy: applies — schema visibility is free for everyone; execution of gated types requires commercial license

---

## 11. What to do with this spec

This is **v0.1, endpoint-level.** It specifies the surface and the contract but not the wire-level handler code.

**Next steps:**
- Resolve the 6 open questions in §8 (most are small)
- Implement the handler (single endpoint, leverages existing internal introspection)
- Publish on the davisgeometric.com infrastructure or wherever GIGI lives publicly
- Update GIGI Lang's SDK skeleton (`lang.py`) once the endpoint exists, replacing `schema()`'s `NotImplementedError` with a real fetch
- Announce in the MCP server docs (venue 04) once it's live — that venue depends on this
