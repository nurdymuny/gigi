# GIGI Lang — Specification (High-Level Goals)

**Owner:** Bee Rosa Davis / Davis Geometric
**Drafted:** 2026-05-24
**Status:** v0.1.1 DRAFT — goals-level spec, with reviewer-informed resolutions to 4 of 8 open questions. Mathematical machinery and implementation details defer to GIGI internals and Bee's published framework.
**Scope:** This document specifies *what GIGI Lang is for and what it must achieve.* It does not define wire protocols, codec details, or mathematical correctness conditions — those live in the GIGI codebase, the DHOOM SPEC, and the Davis Geometric framework papers.

---

## 1. What GIGI Lang Is

GIGI Lang is the **prompt-to-fiber-response language layer** for the GIGI database engine. It's the named, documented surface that ties together:

- **Natural language prompts** (from humans or LLMs)
- **GIGI's GraphQL query interface** (already exists; the structured query layer)
- **GIGI's fiber-bundle data substrate** (the geometric storage layer)
- **DHOOM serialization** (the wire format for fiber-shaped responses)

The user-facing contract is simple: **write a prompt, get back a fiber-shaped response.** Internally, the prompt is translated to GQL, the GQL executes against GIGI's fiber-bundle store, and the response comes back shaped by the fiber — serialized as DHOOM by default.

GIGI Lang is **not a new language in the lexer/parser sense.** No `.gigi` source files, no new syntax to learn. It's "language" in the sense PromQL, Cypher, and GraphQL itself are languages: a defined query surface plus a defined response shape, exposed through SDKs and APIs.

---

## 2. Design Goals

### G1. The prompt-to-query translation is the user's experience.
A user writes natural language ("show me the city pairs whose haversine distance is less than 500km") and gets a fiber response. They do not write GQL directly unless they want to. The GQL is the *intermediate representation*, not the user interface.

### G2. The response carries its own structure.
Fiber responses include the schema/type that shaped them, not just the values. A consumer (LLM, downstream tool, human reader) can interpret the response without external schema lookup. This is the DHOOM contract: fiber decomposition serialized inline, schema-and-data in one object.

### G3. The translation is auditable.
The GQL implied by any prompt is inspectable. Users — and especially LLMs operating with GIGI Lang as a tool — can see what their natural language compiled to before execution, and can refine the prompt or write GQL directly when they want precision. No black-box translation.

### G4. The interface is agent-friendly first, human-friendly second.
GIGI Lang is designed primarily for AI agents (LLMs using it as a tool, MCP clients, automated pipelines). Error messages, schema introspection, and response shapes are designed for machine consumption first; human consumption is a derived benefit. Most consumers in practice will be agents, and the spec should favor them.

### G5. Fiber-bundle math is the substrate, not the user's concern.
Users (whether human or LLM) interact at the prompt-and-response level. They do not need to understand the Davis Field Equation or fiber bundles to use the system. The math is what makes the responses *good* (structured, composable, semantically rich), but the math is the *engine,* not the *API.*

### G6. Licensing posture matches the rest of the stack.
GIGI Lang follows the Davis Geometric licensing model: free for non-commercial use, patent-protected for commercial. Documentation, schema, examples, and SDK are available without payment for research, education, and personal use. Commercial deployments are licensed through Davis Geometric.

### G7. One language, many transports.
Whether the user is a Python SDK call, a TypeScript SDK call, an HTTP request, a CLI invocation, or an MCP tool call — the prompt → GQL → fiber → DHOOM loop is the same. Transports vary; the language doesn't.

### G8. Lossless throughout, with optionality gated at the bundle level.
The prompt-to-GQL translation may be lossy (natural language is ambiguous), but everything downstream of the GQL is lossless: GQL execution returns the fiber response without information loss, and DHOOM serialization is round-trip lossless by its own spec. Lossiness lives in exactly one place, by design.

This is the same architectural shape GIGI already uses for the **Kähler-upgrade optionality contract**: a single source of structural risk gated off from a deterministic downstream. **GIGI Lang inherits the optionality semantics for free** — bundles with Kähler structure attached surface richer responses (curvature, Hadamard verdict, holonomy debt, etc.) as *additional* fields; bundles without Kähler look exactly like classical GIGI to a GIGI Lang client. No version flag, no breaking change. **Strict additivity all the way down** — the same discipline that keeps the Kähler upgrade behind a feature gate keeps the geometric machinery out of GIGI Lang's user-facing contract.

---

## 3. Non-Goals

- **GIGI Lang is not a general-purpose programming language.** No loops, no conditionals, no user-defined functions. It's a query interface plus a translation layer.
- **GIGI Lang is not Marcella.** Marcella is one specific intelligence built on this stack — a *consumer* of GIGI Lang, not part of it. GIGI Lang contains no agent-specific prompts, history, or behavior.
- **GIGI Lang does not replace GraphQL.** GIGI's GQL stays the authoritative structured query layer. GIGI Lang adds the prompt-translation surface on top.
- **GIGI Lang does not include the language model that does the translation.** The translation layer may delegate to Claude, another LLM, or a Davis-Geometric-specific model. The spec defines the *contract;* the choice of translator is an implementation decision.
- **The math is out of scope for this spec.** Davis Field Equation, fiber bundle structure, geometric primitives — these are GIGI's internals. This spec describes the *language layer* on top, and assumes the math works.

---

## 4. Architecture

```
                       ┌─────────────────────────────────┐
       User / Agent    │  Prompt (natural language)      │
                       └────────────────┬────────────────┘
                                        │
                       ┌────────────────▼────────────────┐
                       │  GIGI Lang translation layer    │
                       │  (prompt → GQL)                 │
                       │  Lossy step: the only one.      │
                       └────────────────┬────────────────┘
                                        │
                       ┌────────────────▼────────────────┐
       GIGI engine     │  GQL execution                  │
                       │  (fiber-bundle traversal)       │
                       │  Lossless.                      │
                       └────────────────┬────────────────┘
                                        │
                       ┌────────────────▼────────────────┐
                       │  Fiber-shaped response          │
                       │  serialized as DHOOM (default)  │
                       │  Lossless.                      │
                       └────────────────┬────────────────┘
                                        │
                       ┌────────────────▼────────────────┐
       User / Agent    │  Consumes the response          │
                       └─────────────────────────────────┘
```

The translation layer is the new piece. Everything else exists today. GIGI Lang is the *name* and *spec* for the whole loop, with the translation layer being its distinctive contribution to the stack.

---

## 5. Components

### 5.1 Prompt Translation Layer
Takes natural language and produces GQL.

- **Inputs:** prompt (string), schema (from GIGI introspection), optional context (prior conversation, available fragments)
- **Outputs:** GQL query, or an ambiguity error with candidate queries
- **Implementation:** likely delegates to an LLM (Claude or equivalent) with the GIGI schema as context. The translator is replaceable; the contract is fixed.
- **Status:** new. To be built.

### 5.2 GIGI Execution Layer
The existing GIGI engine. Executes GQL against the fiber-bundle store.

- **Status:** exists. Not changed by this spec.

### 5.3 Response Serialization Layer
Encodes the fiber-shaped response as DHOOM (default) or JSON (compatibility).

- **Status:** DHOOM codec exists. Wiring into the response pipeline may need work; that's implementation, not spec.

### 5.4 Schema Introspection
Public-facing endpoint exposing GIGI's GQL schema. Required for the translation layer to know what queries are valid, and for external tools (MCP clients, the geomstats GIGI loader, etc.) to integrate.

- **Status:** depends on prerequisite P3 in `~/Documents/davis-contributions/PRE_REQS.md`. Decisions still needed about which parts of the schema are public vs. commercial-only.

### 5.5 Existing surfaces the translator can target today

The L1–L7 work already shipped in GIGI's main branch exposes a substantial surface that the translation layer can construct GQL/HTTP calls against. As of 2026-05-24:

**HTTP endpoints (publicly callable):**
- `/curvature`, `/spectral_gap`, `/betti`, `/entropy`, and others — direct read access to geometric properties of stored bundles

**GQL verbs:**
- `TRANSPORT seg WITH B = ... [ALLOW_NON_CLOSED]` — B-perturbed transport with optional non-closed-loop allowance (L1.5 + L1.5.3)

**Rust-only primitives (currently consumed by Marcella via SDK):**
- `morse_compress` — Morse compression (L6)
- `hadamard_regions` — Hadamard substructure detection (L5)
- `holonomy_debt` — holonomy accumulation tracking
- `encode_chern` — Chern character encoding (L7.3)
- `QuantumCohomology` — quantum cohomology operations (L7)
- `toeplitz_operator` — Berezin-Toeplitz operators (L7)

The Rust-only primitives are candidates for GQL exposure in GIGI Lang v0.2 if agent-callability is desired. They live behind the SDK today by design (Marcella's primary consumption pattern) but the geometric responses they produce are exactly the kind of structured output GIGI Lang's fiber-response contract is designed for. Exposing them is purely additive — no existing surface changes.

---

## 6. User Interface / API

### 6.1 Python SDK
```python
from gigi import GIGI

client = GIGI(endpoint="https://...", api_key="...")

# High-level: just a prompt
response = client.ask("show me the 10 nearest cities to Tokyo")
# response is a fiber-shaped object; iterate or access by attribute

# Mid-level: inspect the generated GQL before executing
gql = client.translate("show me the 10 nearest cities to Tokyo")
print(gql)
response = client.execute(gql)

# Low-level: write GQL directly
response = client.query("{ cities(near: \"Tokyo\", limit: 10) { name } }")
```

### 6.2 MCP Tool (for Claude and other LLM clients)
Exposes `ask` as an MCP tool. The LLM gets a prompt-in, fiber-out interface and can also introspect the schema for sophisticated queries it wants to construct directly. (Spec'd in detail in `~/Documents/davis-contributions/venues/04_anthropic_mcp_gigi.md`.)

### 6.3 HTTP API
Standard REST surface plus a GraphQL endpoint. The prompt-translation endpoint accepts `{prompt: "..."}` and returns `{gql: "...", response: {...}}` or a structured error.

### 6.4 CLI (sketch — subject to confirmation)
```
gigi ask "show me the 10 nearest cities to Tokyo"
gigi translate "show me the 10 nearest cities to Tokyo"   # GQL only
gigi query '{ cities(near: "Tokyo", limit: 10) { name } }'
gigi schema                                                 # introspect
```

---

## 7. Examples

### Example 1 — Simple lookup
**Prompt:** *"what's the population of Tokyo?"*

**Implied GQL:**
```graphql
{ city(name: "Tokyo") { population } }
```

**Response (DHOOM):**
```
city{name, population}:
Tokyo, 35676000
```

### Example 2 — Geometric / relational query
**Prompt:** *"show me city pairs within 500km of each other"*

**Implied GQL:**
```graphql
{ cityPairs(maxDistanceKm: 500) { from, to, distanceKm } }
```

**Response (DHOOM):**
```
cityPairs{from, to, distanceKm}:
Paris, London, 344
...
```

### Example 3 — Fiber-aware query
**Prompt:** *"give me the manifold structure of the cities dataset"*

**Implied GQL:**
```graphql
{ dataset(name: "cities") { fiber { fields }, baseSpace, sections { count } } }
```

**Response:** carries the full fiber metadata as a DHOOM bundle, ready for downstream geometric tools (e.g., geomstats `load_gigi`).

---

## 8. Open Questions

### 8a. Resolved (engine-informed, as of v0.1.1)

These were resolved by looking at what GIGI's existing engine already implies. They're substrate-grounded defaults, not arbitrary picks — flippable later if reasons emerge, but the engine has data and the data points to these answers.

**Q1. Translator choice → Claude for v1; fine-tune for v2 once we have query traces.**
- Matches the MCP venue (`venues/04_anthropic_mcp_gigi.md`) for single-vendor observability
- Keeps the "Claude as voice, GIGI as brain" architecture clean in v1
- Fine-tuning becomes interesting once there's a corpus of real prompt-to-GQL traces to learn from; until then, frontier general-purpose Claude is the right call

**Q5. Multi-step queries → Hadamard verdict is the substrate-informed default.**
- Compose as much as the local geometry permits; surface accumulated non-associativity as a returned field
- The substrate already knows when composition is safe (Hadamard regions) and when it accumulates obstruction (Marcella just measured +7.6pp on S³⁸³ — non-trivial)
- The translator doesn't need to invent a multi-step strategy from scratch; it asks the substrate and the substrate answers

**Q6. Default response format → DHOOM-first.**
- Self-describing → LLM consumers skip the schema-lookup pass that JSON would require
- L7.3 Chern compression gives meaningful wire savings on geometric responses
- JSON remains available via `accept: application/json` for compatibility-only callers

**Q7. Error model → unified envelope with category enum, non-exhaustive in clients.**
- Maps directly to existing `TransportError` / `IntegralityError` / `QuantumError` variants
- Single shape across translation errors, GQL errors, and execution errors
- Clients pattern-match on the category but treat the enum as open (forward-compatible)

### 8b. Still open (require strategic decisions)

These aren't substrate questions; they're business / strategy decisions only Bee can make. Flagged for when one starts blocking implementation work.

**Q2. Auth model for commercial vs. non-commercial.** How is the commercial-use boundary enforced at the API layer? Token tiers? Honor system + license file? IP-based? This isn't just a billing question — it determines whether the patent licensing is actually enforceable through normal API use.

**Q3. Schema visibility tiers.** Which parts of GIGI's schema are publicly introspectable? Some fields/types likely need to stay private to maintain commercial-licensing enforceability and to keep specific advanced capabilities (Marcella-grade primitives, L7/L8 features) gated.

**Q4. Conversation context.** Does the translation layer carry prior-prompt context across calls (session-aware), or is each prompt independent (stateless)? Stateful is more useful for agents; stateless is simpler to scale and reason about.

**Q8. Rate limiting and quota.** Non-commercial users get free access; what's the threshold beyond which they're presumed commercial? Per-month query count, per-day, never? Connects to Q2.

---

## 9. Relationship to Other Davis Geometric Components

- **GIGI:** GIGI Lang sits on top of GIGI's GraphQL surface. GIGI is the engine; GIGI Lang is the named user-facing language.
- **DHOOM:** DHOOM is the default wire format for fiber-shaped responses. GIGI Lang assumes DHOOM availability; falling back to JSON is supported.
- **Marcella:** Marcella consumes GIGI Lang as her interface to the GIGI substrate. Marcella is a *user* of GIGI Lang, not a component of it.
- **DGP (the graphene chip):** when DGP exists, GIGI's execution layer runs on it natively. GIGI Lang's spec is unchanged; only the implementation under it gets faster.
- **ICARUS:** ICARUS uses GIGI for state storage but does not (currently) go through GIGI Lang's prompt-translation layer — its queries are programmatic. Both share the underlying geometric primitive.
- **The Geometry of Flight (book):** the book describes the substrate (Davis Field Equation, fiber bundles, etc.) that GIGI Lang's responses are shaped by. Citation reference for users who want to understand *why* the response shapes look the way they do.

---

## 10. References

- GIGI's existing GraphQL surface: internal docs; public surface TBD per prerequisite P3 in the davis-contributions master plan
- DHOOM specification, v0.5: https://dhoom.dev
- The Davis Field Equation C = τ/K and the broader framework: davisgeometric.com + Zenodo papers cited in *The Geometry of Flight* (Davis 2026, ISBN 979-8-1983-7541-3)
- Cross-venue contribution plan: `~/Documents/davis-contributions/MASTER_PLAN.md`
- MCP server (which will expose GIGI Lang as a Claude-callable tool): `~/Documents/davis-contributions/venues/04_anthropic_mcp_gigi.md`
- Bee's licensing philosophy: applies uniformly to GIGI Lang — free for education / research / non-commercial; patent-protected commercial deployment via Davis Geometric

---

## 11. What to do with this spec

This is **v0.1, goals-level.** It captures what GIGI Lang must achieve and the architectural shape, but it deliberately stops short of:

- Wire-protocol details (HTTP method signatures, exact error JSON shape, etc.)
- The math of fiber-bundle responses (defers to existing GIGI internals and Davis Geometric framework papers)
- Translator-layer implementation specifics (prompt-engineering for the LLM translator, fallback strategies, etc.)

**Next steps** (when Bee is ready):
- Resolve the 4 still-open questions in §8b (the 4 substrate-grounded ones are resolved in §8a)
- Add a v0.2 technical spec covering wire protocol, error model wire shape, and auth specifics
- Publish the schema (prerequisite P3) so external tools can integrate
- Build the translation layer (the only genuinely new component)
- Integrate with the MCP server (venue 04 in the contribution plan)

**v0.2 prep work** (small, unblocking, doesn't need to wait on the still-open questions):

1. **Ship the public schema introspection endpoint** (prerequisite P3 in the davis-contributions plan). Highest-leverage single unblock for v0.2 — every downstream venue depends on this being public.

2. **Create a `gigi-lang` SDK skeleton** with `NotImplementedError` bodies pinning the `client.ask` / `translate` / `execute` / `query` contract in code. Lets the translation-layer build proceed against a fixed interface even while §8b decisions are still open. The interface is the spec; the skeleton makes it executable.

Neither blocks the spec from being useful as-is. They make v0.2 cheaper when the time comes.
