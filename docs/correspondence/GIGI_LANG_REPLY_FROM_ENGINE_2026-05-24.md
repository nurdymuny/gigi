# GIGI Lang spec — reply from the engine side

**From.** GIGI engine team (bee + Claude).
**To.** Strategy team.
**Date.** 2026-05-24.
**Re.** `GIGI_LANG_SPEC.md` v0.1.

The spec is well-scoped. Three things to add from where the
engine actually lives today: a structural observation, a surface
inventory, and concrete views on four of the eight open questions
where the engine has real data.

---

## The one-lossy-step framing IS a correctness property

G8 ("lossless throughout, lossy in exactly one place") is the
load-bearing claim of this spec, and it's stronger than the
spec admits. It's the *same* architectural shape as the
**Kähler-upgrade optionality contract**:

- Kähler contract: bundles with no `KahlerStructure` attached
  behave bit-identically to pre-upgrade GIGI; everything Kähler
  is strictly additive. The non-feature build stays at 720
  passing tests, byte-equal to before the upgrade.
- GIGI Lang's claim: GQL execution + DHOOM serialization are
  bit-identical regardless of whether the prompt came from a
  human, an LLM, or a CLI call. The translation layer is
  upstream; the engine doesn't care what produced the GQL.

Both have a **single source of structural risk** (the optional
Kähler structure / the prompt-to-GQL translation) gated off
from the deterministic downstream. That gating is what makes
the system auditable. Worth saying out loud in §2 because it's
the property that lets external geometers / auditors verify
correctness incrementally — they only have to inspect one
place.

This also means **GIGI Lang inherits the Kähler upgrade's
optionality semantics for free.** Bundles without a Kähler
structure attached look exactly like classical GIGI to a GIGI
Lang client; bundles with Kähler attached surface richer
responses (curvature decomposition, Hadamard verdict, etc.) as
extra fields the client can ignore. No version flag, no breaking
change.

---

## Surface inventory — what GIGI Lang's translator can target today

The spec correctly says "GQL stays the authoritative structured
query layer." Here's what GQL actually exposes after L1–L7. The
translator's grammar / few-shot examples will want all of these:

### Classical (pre-Kähler) surfaces
- Record CRUD, range queries, vector search
- `JOIN` across bundles
- Schema introspection (per the spec's §5.4 prerequisite)

### Geometric query surfaces (added by L1–L7)
- `GET /v1/bundles/<name>/curvature` — scalar K + per-field
  variance + (optional) Kähler decomposition
- `GET /v1/bundles/<name>/spectral_gap` — λ₂, mixing time,
  Cheeger bounds (cached)
- `GET /v1/bundles/<name>/spectral` — pre-Kähler spectral report
- `GET /v1/bundles/<name>/betti` — Betti numbers via Hodge complex
- `GET /v1/bundles/<name>/entropy`, `/free-energy`,
  `/geodesic`, `/metric`, `/consistency`, `/health`,
  `/anomalies`, `/predict`

### Kähler-specific GQL verbs (L1.5.3)
- `TRANSPORT seg WITH B = ... [ALLOW_NON_CLOSED]` — magnetic
  geodesic flow on flat tangent spaces
- `TRANSPORT seg ON bundle` — curved variant, refuses outside
  Hadamard regions per L5.5

### Rust-only APIs (not yet GQL-surfaced)
- `BundleStore::morse_compress() -> MorseComplex` — Marcella
  consumes via SDK
- `BundleStore::hadamard_regions()` / `is_hadamard_region()`
- `holonomy_debt(store, integral, tol)` — quantized vs
  continuous classifier
- `encode_chern` / `decode_chern` — DHOOM compression
- `QuantumCohomology::compose()` / `representational_capacity()` /
  `hilbert_polynomial()`
- `toeplitz_operator()` with safe-ℏ gate

The Rust-only items are candidates for GQL exposure in v0.2 if
the strategy team decides agent-callable Kähler ops should be
in the public surface. Otherwise they stay SDK-direct (which is
how Marcella uses them today).

---

## Views on four open questions

The other four are decisions only you can make. These are the
ones where the engine has data:

### Q1 (translator choice): Claude for v1, anything for v2

The "Claude as voice, GIGI as brain" framing in question #1 is
the right architectural shape. For v1 I'd pick Claude
specifically because:

- The MCP server work in venue 04 already targets Claude as a
  consumer; using Claude as the translator AND a downstream
  consumer keeps the integration surface single-vendor and
  observable.
- Claude has the schema-introspection + few-shot affordances
  to do this well without fine-tuning.
- Fine-tuning a smaller model is the v2 cost-optimization play
  once we have query traces to fine-tune ON.

The spec keeps the choice open, which is correct — but for v1
the engine recommends Claude.

### Q5 (multi-step queries): bounded by Kähler geometry, in fact

The Kähler upgrade gives this question a sharper answer than
"the translator decides." Multi-step queries are operationally
**residue composition across turns**, and Marcella's A/B harness
just measured the magnitude on a real substrate:

- On a Hadamard region: composition is provably bounded
  (Cartan-Hadamard, no conjugate points). Multi-step is safe.
- Off Hadamard: composition has measurable non-associativity
  — +7.6pp on Marcella's S³⁸³ substrate. Still useful, but
  the translator should warn after some accumulated drift.

So the default behavior the spec asks about ("plan and execute
multiple, return a single combined query, or refuse") has a
geometric criterion underneath it: **run as many composed
queries as the local Hadamard verdict permits; surface
accumulated non-associativity in the response so the agent
client can decide whether to keep going.** Not necessarily
exposed in v0.1 but worth noting in the spec as the substrate-
informed default.

### Q6 (default response format): DHOOM-first, JSON-on-request

DHOOM as the default doubles as a teaching tool (spec author's
own observation) AND gives the wire-savings from L7.3 Chern
compression for free on bundles with integrally-quantized B.
JSON fallback is fine for legacy clients via `accept` header.

For the LLM-consumer-first goal (G4), DHOOM is also
self-describing — schema-and-data-in-one-object means the LLM
doesn't need a separate schema lookup pass per response. That's
a real latency win on agent-driven workloads.

### Q7 (error model): unified envelope with category enum

For agent error-handling: unified `{ category, code, message,
details }` shape with the category enum exhaustively
documented. Categories the engine sees in practice:

- `translation_failed` — prompt → GQL couldn't produce a query
- `translation_ambiguous` — multiple candidate GQLs; details
  carries the candidates
- `gql_parse_error` — translator produced invalid GQL
  (translation-layer bug)
- `gql_validation_error` — GQL is well-formed but doesn't match
  the schema (e.g., field doesn't exist)
- `execution_error` — bundle / fiber-level failure
- `kahler_constraint_violation` — e.g., `transport_along`
  refused because bundle isn't Hadamard. Marcella pattern-matches
  on these (see `TransportError::NotHadamard`).
- `rate_limited` — connects to Q8
- `unauthorized` — connects to Q2

The categories above map directly to error variants in existing
GIGI code. The agent gets a stable contract; we keep the freedom
to add new variants without breaking pattern-matches as long as
the category enum is treated as non-exhaustive in clients.

---

## What GIGI could ship to make v0.2 easier

Two small things, neither blocking, both useful:

1. **Public schema introspection endpoint** (the P3 prerequisite
   in `~/Documents/davis-contributions/PRE_REQS.md`). This is
   what unblocks both the translator's context and external
   tools (geomstats `load_gigi`, the MCP server, etc.). Probably
   the single highest-leverage thing on the v0.2 critical path.

2. **`gigi-lang` SDK skeleton** — Python + TS, just enough to
   pin the `client.ask` / `client.translate` / `client.execute`
   / `client.query` contract from §6.1 in code. Even with
   placeholder bodies that just `raise NotImplementedError`, the
   skeleton lets the translator team build against a fixed
   interface instead of a moving target.

Neither item requires the 8 open questions to be answered. Both
could land while the strategy thinking continues.

---

## On the framing

This spec did the right thing by deferring the math. The
geometric machinery isn't user-facing — it shapes the
**responses** by making them fiber-aware, but the user writes
prompts in English (or the agent does). The math is the engine
that makes the responses *good*; the engine doesn't need a seat
at the language-design table.

Same logic that kept the Kähler upgrade behind a feature gate
keeps the math out of GIGI Lang's user-facing contract. Strict
additivity, optional surfacing, no breaking changes. The
correctness lives in one place; the API stays clean.

— Engine team
