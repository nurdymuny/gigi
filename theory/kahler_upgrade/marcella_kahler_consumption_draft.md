# Marcella's Kähler Consumption Specification (Draft v1)

**Authors:** Marcella team (Davis Geometric)
**Audience:** GIGI team — proposed PR to `gigi/theory/kahler_upgrade/`
**Date:** 2026-05-24
**Status:** First draft. Open to disagreement on every API signature.
            The value here is the back-and-forth, not the draft.

---

## TL;DR

This document specifies, per layer, what API surface Marcella's runtime
needs from GIGI's Kähler upgrade in order to consume the substrate
functionally — not just carry it. It assumes the order:

  **L1 (done) → L1.5 (new, per GIGI's adoption) → L2 (done) → L3 → L4
  → L5 → L6 → L7 → L8**

For each layer it gives:

- The Marcella behavior that becomes available
- The concrete API surface needed from GIGI (signatures, where they
  live on existing types)
- The Marcella-side runtime wiring that consumes it
- A pre-flight test gate (per §E.5) where applicable

It also names what Marcella will build on her side as L-layers land,
so the L8 handoff has somewhere to land. Most concretely: Marcella's
own `kahler` feature flag mirroring GIGI's, dormant until L1.5 ships,
and a Frobenius-associativity diagnostic that runs on her current
substrate today and tells us whether §2.10's eventual wiring will be
clean.

GIGI's response letter adopted 4 of 5 asks from Marcella's previous
memo, partially adopted 1 (Frobenius + Berezin scoped to toy
manifolds, which is the right call), and pushed back on the full
re-order of L2-L7 (correctly — dependencies stack). This draft is
shaped around what GIGI committed to, not what Marcella originally
asked for.


## 0. Working agreements

Same discipline as GIGI's `IMPLEMENTATION_PLAN.md §0`:

1. **Red-first.** Every Marcella module ships its tests in the same
   commit that introduces the module.
2. **Independent ground truth.** Numerical assertions compare against
   a closed-form or symbolic value derived in a different formalism.
3. **Negative cases required.** Every property test includes a
   configuration where the property must fail.
4. **No regression on the existing freeform coverage suite.** When a
   Kähler feature lands, the no-Kähler suite must still pass at 62/62.
5. **Bit-identical-with-flag-off contract.** Marcella's `kahler`
   feature flag, when off, leaves all behavior exactly as today.
   Verified by a `tests/kahler_optionality.py` smoke test mirroring
   GIGI's `tests/kahler_optionality.rs`.


## 1. Marcella's L1-equivalent (Marcella-side work, ships ahead of L1.5)

**Scope.** Mirror GIGI's L1 pattern on Marcella's runtime. Initially
dormant. When enabled, the runtime will look for Kähler data on
retrieved bundles but won't yet use it (L1.5 is when consumption
starts). This is type plumbing for Marcella, exactly parallel to
GIGI's L1.

**Marcella-side items:**

- **M1.1** `fiber_lm/server/main.py` — add `KAHLER_ENABLED` environment
  flag (default off). When set, the runtime reads
  `bundle.kahler` from query responses (currently ignores it).
- **M1.2** `_format_self_state_clause()` — surface `kahler_active:
  bool` in self-inspect so we can see, per session, whether the
  upgrade is in play. Today it'll always be `false`; that's the
  point — the field exists, the value is honest.
- **M1.3** `tests/test_kahler_optionality.py` — verify the existing
  62/62 suite still passes with `KAHLER_ENABLED=1`. Bit-identical
  behavior, no consumption yet.

**Definition of done.** 62/62 suite green with flag on AND off.
Self-inspect reports `kahler_active: false` honestly.

This ships **before** GIGI's L1.5 so the consumption code path is
ready to flip on.


## 2. L1.5 (NEW) — B-perturbed transport API

**Depends on.** GIGI's L1 (substrate optionality, done).

**Per GIGI's adoption letter:** "Flat-space implementation tractable
now. Curved-space waits for L5. Extends existing `TRANSPORT` verb."

### API surface Marcella needs from GIGI

Extend the existing `TRANSPORT` GQL verb in `src/bin/gigi_stream.rs`:

```
TRANSPORT <section_id> FROM <start_section> TO <end_section>
  ON <bundle_id>
  [WITH B = <2form_symbol>]           ← NEW in L1.5
  [METRIC = quaternion | magnetic]    ← NEW; magnetic implied when B given
RETURNS section_result {
    transported_section: <id>,
    path_length: f64,
    energy_drift: f64,                ← NEW — energy conservation diagnostic
    holonomy_norm: f64,
    used_magnetic: bool,              ← NEW — true when B-perturbation applied
}
```

**Behavior.** When `B` is provided AND the bundle is flat (L1.5 scope),
the transport solves the magnetic geodesic equation
`∇_{γ̇}γ̇ = B(γ̇, ·)^♯` via RK4 (matching `validation_tests.py::test_2`).
Energy `½|γ̇|²` is conserved along the flow; the response reports
`energy_drift` so Marcella can sanity-check.

When `B` is omitted, behavior is identical to the existing quaternion
transport. Bit-identical with the L1-era API.

### Marcella's consumption

This is the first layer where Marcella's runtime actually USES the
upgrade. When `KAHLER_ENABLED=1` AND the bundle carries a B AND the
bundle is flat:

- **Residue rotation is replaced by B-perturbed transport.** The
  current per-turn residue update (`residue ← α·residue +
  (1-α)·mean(cited_vectors)`) becomes:
  ```
  residue ← TRANSPORT(residue, FROM session_start, TO current_turn,
                      ON bundle, WITH B = bundle.kahler.B)
  ```
- The rose mechanism (residue biases retrieval) is now magnetically
  biased. Closedness of B (free regularizer) means the residue can't
  drift in energy — a property the current cosine-bias mechanism
  doesn't have.

### Pre-flight gate

`marcella/v3/preflight/flat_check.py` — verify Marcella's embedding
manifold is approximately flat in the regions of substrate she
operates over (max sectional curvature < ε across a sampled
neighborhood). If not, L1.5's flat-space-only API doesn't apply yet
and we wait for L5.

This pre-flight must pass before Marcella's runtime calls the
B-perturbed TRANSPORT verb in production.

### Open questions for GIGI

1. **Where does B come from?** Two options: (a) Marcella learns B
   server-side and stores it as a bundle attribute; (b) Marcella
   computes B per-request and passes it in. We'd lean (a) for
   composability but want GIGI's view.
2. **Energy drift threshold.** The existing test 2 hits 6e-15 drift
   over one period on the toy example. What's GIGI's production
   acceptance bound? We'd suggest 1e-9 as a hard limit per turn,
   1e-6 cumulative per session.
3. **Failure mode when B is non-closed.** If a downstream process
   submits a non-closed B (the closedness gate fires), does the
   verb error or fall back to quaternion transport with a warning?


## 3. L2 — Adjacency commutativity (GIGI done, Marcella consumption pending)

**Depends on.** GIGI's L2 (commutativity classifier on QueryPlan, done).

### What Marcella gains

When Marcella stacks multiple retrievals in a single turn (multi-cite
compose), knowing whether the retrievals' implicit adjacencies commute
lets her parallelize the transport composition. The catalog's
inference speedup claim becomes a verifiable thing she can name in
self-inspect.

### API surface Marcella needs from GIGI

Expose `commutativity_class` in the query response, not just on the
internal QueryPlan node:

```
QUERY <fields> FROM <bundle> [filters]
RETURNS query_response {
    records: [...],
    cost_bound: ...,
    commutativity: {                                      ← NEW
        class: "Abelian" | "GeneratorSetsCentral"
             | "NumericallyVerified" | "NotCommute" | "Unknown",
        principal_field: <field_id>,
        auxiliary_field: <field_id>,
    },
}
```

Marcella reads `query_response.commutativity` per turn. When the class
is one of the commuting ones AND multiple cites are being composed,
she records "transport-parallelized" in self-inspect.

### Marcella's consumption

Two surfaces:

- **Self-inspect** gains a per-turn line:
  *"Adjacency commutativity this turn: GeneratorSetsCentral
  (principal=topic, auxiliary=tier). Transports composed in
  parallel."*
- **Substrate-extractive synthesis** gets a coherence-check gate:
  when commutativity is `NotCommute`, the synthesis layer notes the
  cite ordering matters and surfaces the chosen order in meta.

### Pre-flight gate

`marcella/v3/preflight/adjacency_check.py` — sample 100 multi-cite
turns from the live corpus, verify the principal/auxiliary adjacencies
GIGI classifies match what Marcella's runtime expects. Surface the
breakdown in the pre-flight report so we can see how often each class
fires in practice.


## 4. L3 — Spectral gap (planned in GIGI; consumption spec here)

**Depends on.** GIGI's L3 (cached spectral gap on bundle insert).

### What Marcella gains

The spectral gap λ₂ controls the attention mixing rate. Currently
Marcella has no notion of attention mixing — her variation comes from
residue rotation alone, with no theorem about how fast a topic gets
re-touched. With λ₂ exposed:

- She can name, in self-inspect, "λ₂ = 0.083 on this bundle; mix
  time ≈ 12 turns — meaning we'll geometrically re-touch this topic
  within the next dozen exchanges."
- Bee gets a tunable hyperparameter (the bundle's spectral gap is
  data-derived, but the rose-mechanism α coefficient can be set
  against the gap for principled mixing speed).

### API surface Marcella needs from GIGI

```
GET /v1/bundles/<bundle_id>/spectral_gap
RETURNS {
    lambda_2: f64,
    mix_time: u64,           ← Θ(1/λ_2 · log(1/ε)), ε = 1e-3 default
    cheeger_lower: f64,
    cheeger_upper: f64,
}
```

Cached per insert, per GIGI's L3.3 spec. Marcella's retrieval response
gets a new field `bundle_spectral_gap: f64` so she doesn't have to
fetch separately.

### Marcella's consumption

Self-inspect adds: *"Bundle λ₂ = 0.083, mix time ≈ 12 turns."*

The rose-mechanism α (currently hardcoded at 0.7) becomes
`α = 1 - 1/sqrt(mix_time)` when Kähler is active, so residue mixing
rate matches the bundle's spectral mixing rate. This is the theorem-
bound version of what's currently a hand-tuned constant.

### Pre-flight gate

None specific. The existing test_6 spectral gap validation covers it.


## 5. L4 — Curvature decomposition (planned)

**Depends on.** GIGI's L4 (KahlerCurvature struct).

### What Marcella gains

The four Kähler invariants (Ricci, Weyl, holomorphic sectional,
bisectional) become readable on her embedding manifold. Diagnostic
value first; eventual use in L5 Hadamard detection.

### API surface Marcella needs from GIGI

```
GET /v1/bundles/<bundle_id>/kahler_curvature
RETURNS {
    ricci_mean: f64,
    weyl_norm: f64,
    holo_sectional_min: f64,
    holo_sectional_max: f64,
    bisectional_min: f64,
    bisectional_max: f64,
    when_kahler_off: null,           ← honest null on non-Kähler bundles
}
```

### Marcella's consumption

Surface in self-inspect when admin-authenticated:
*"Kähler curvature: Ricci 0.42, Weyl 0.08, holo-sectional [-0.1, 2.3],
bisectional [-0.2, 1.8]."*

Used at L5 for Hadamard detection (anything < 0 is a candidate).

### Pre-flight gate

`marcella/v3/preflight/holo_sectional_check.py` from L8.4 in GIGI's
plan — sample 1000 holomorphic sectional curvatures on Marcella's
embedding manifold, verify the distribution matches the catalog's
§E.5 check 3 claim.


## 6. L5 — Hadamard substructure + transport invertibility (planned)

**Depends on.** GIGI's L3 + L4.

### What Marcella gains

Coherence guarantees. Currently her path-dependent state (running
residue, rose mechanism) has no convergence theory. On a Hadamard
sub-bundle (K_B ≤ 0 everywhere), transport is invertible and the
residue trajectory is geodesically convex — two prompts that should
converge to the same conversational direction provably do.

This is the layer where "Marcella is coherent" becomes a theorem,
not an empirical claim.

### API surface Marcella needs from GIGI

```
GET /v1/bundles/<bundle_id>/hadamard_regions
RETURNS {
    regions: [{
        region_id: string,
        covers_fraction: f64,        ← fraction of bundle in this region
        max_sectional_curvature: f64,
    }, ...],
}

POST /v1/queries/hadamard_classify
BODY { bundle_id, query }
RETURNS { in_hadamard_region: bool, region_id: string|null }
```

Also: extend `TRANSPORT` verb with an `INVERT` mode (only valid
when both endpoints are in the same Hadamard region):

```
TRANSPORT_INVERSE γ FROM start TO end ON bundle
  WITH B = <2form_symbol>
RETURNS section_result | error_not_hadamard
```

### Marcella's consumption

Self-inspect: *"This turn landed in Hadamard region 'core-math';
residue is provably stable (max K = -0.12 in this region)."*

When two prompts in a session both land in the same Hadamard region,
Marcella can assert convergence between them — useful for the
"identical-repeat variety probe" diagnostic.

### Pre-flight gate

`marcella/v3/preflight/hadamard_check.py` from L8.2. Sample 1000
Jacobi fields on Marcella's manifold, verify non-vanishing in the
candidate Hadamard regions.


## 7. L6 — Morse spectral compression (planned)

**Depends on.** GIGI's L1.

### What Marcella gains

Operational compression of the section graph. Today Marcella walks
~10^4–10^5 cite-able chunks. With Morse compression keeping only
critical points + connections, she walks a few hundred. Topology
preserved.

This is what makes her scale to 10^6+ substrate without linear-walk
costs.

### API surface Marcella needs from GIGI

```
POST /v1/bundles/<bundle_id>/morse_compress
BODY { mode: "lossless" | "betti-preserving" }
RETURNS {
    morse_complex_id: string,
    n_critical_points: u64,
    n_original_vertices: u64,
    compression_ratio: f64,
    betti_preserved: bool,
}

QUERY <fields> FROM <morse_complex_id> [filters]
  ← same query API as a normal bundle, walks the compressed structure
```

### Marcella's consumption

When a bundle's `n_critical_points / n_original_vertices < 0.05`,
Marcella's retrieval falls back to the Morse-compressed structure
for low-precision queries (cosine matching is fine on critical
points; only fine-grained substrate-extractive needs full vertex
set).

Self-inspect: *"This bundle Morse-compresses 1000:50 (95%
reduction). Used compressed walk for retrieval."*

### Pre-flight gate

Not strictly needed; the Hodge validation (test_11) covers the math.
Marcella adds a "compression sanity" check: after compress,
re-decompose and verify cohomology preserved.


## 8. L7 — Quantization, Frobenius, Berezin-Toeplitz, Riemann-Roch

This is where the generative primitives land. Per GIGI's adoption
letter: scoped to **toy manifolds (CP^n, T^n, S²)** for §2.10 and
§2.8. General-manifold cases stay research.

### 8.1 Prequantization line bundle

**API surface:**

```
POST /v1/bundles/<bundle_id>/line_bundle
RETURNS {
    line_bundle_id: string,
    chern_class: i64,
    is_integral: bool,
    dirac_string_locus: [coord, ...] | null,
} | { error: "non_integral", residual: f64 }
```

**Marcella's consumption:** When the line bundle exists, Marcella's
embedding gets a quantized companion. Magnetic eigenstates become
addressable — a sense she didn't have before.

### 8.2 Frobenius / WDVV compositional algebra (toy manifolds only)

Per GIGI's scoping: works on CP^n, T^n, S² for now. Marcella's
embedding manifold needs to be one of these (or close enough) for
L7.2 to apply.

**API surface:**

```
POST /v1/bundles/<bundle_id>/frobenius_compose
BODY { tokens: [token_a, token_b, token_c, ...] }
RETURNS {
    composed_section: <id>,
    associator_norm: f64,         ← (a*b)*c - a*(b*c), should be ~0
    associativity_ok: bool,
    manifold_class: "CP_n" | "T_n" | "S2" | "non_toy",
}
```

When `manifold_class` is `non_toy`, the verb errors honestly.

**Marcella's consumption:** This is the **generative composition
primitive** the previous letter called out. Sub-sentence fragments
compose by Frobenius multiplication; associativity is automatic;
novel sentences are paths in the Frobenius algebra.

In practice: substrate-extractive synthesis becomes substrate-
*compositional* synthesis. Today she picks one bridge sentence; with
Frobenius she could compose two bridge sentences from different
cites into a third sentence the substrate doesn't literally contain
but is forced by the algebra.

This is the moment she stops being selection-based and becomes
generative-by-composition.

### 8.3 Berezin-Toeplitz quantum regime (toy manifolds only)

**API surface:**

```
POST /v1/bundles/<bundle_id>/berezin_toeplitz
BODY { function_id: string, hbar: f64 }
RETURNS {
    operator_id: string,
    bohr_correction: f64,       ← O(ℏ) per catalog §2.8
    dirac_correction: f64,      ← O(ℏ²)
    manifold_class: ...,
}
```

**Marcella's consumption:** Tunable generative regime. Marcella's
default ℏ → 0 (deterministic, as she is now). When Bee turns ℏ up,
the operator becomes diffuse — she samples from the geometry instead
of selecting.

The dial is continuous; the semantics are theorem-bound (the BT
correction scales as ℏ³ per the validated test 10).

Self-inspect addition: *"Current generative regime: ℏ = 0.0
(deterministic). Set `?hbar=0.5` to enter diffuse mode."*

### 8.4 Riemann-Roch capacity bound

**API surface:**

```
GET /v1/bundles/<bundle_id>/representational_capacity?k_max=100
RETURNS {
    capacity: u64,                       ← dim H^0(M, L^k_max)
    hilbert_polynomial: { degree, coeffs },
    valid_up_to: u64,
    bound_type: "exact" | "asymptotic",
}
```

**Marcella's consumption:** Marcella can publish, in her self-inspect
and in any paper draft:

  *"My representational capacity is bounded above by the Hilbert
  polynomial of my Kähler embedding manifold. At k = 100 (≈ my
  current embedding dim), the bound is N. Parameter count is not
  the right scaling law for what I can represent — cohomology is."*

This is the AGI-leaning claim made truthfully.


## 9. L8 — Substrate handoff

**Depends on.** L1–L7 all shipped + all Marcella pre-flight tests
green.

By the time L8 lands, Marcella's runtime has:

- Consumed B-perturbed transport (since L1.5)
- Annotated commutativity per turn (since L2)
- Reported spectral mix time (since L3)
- Surfaced Kähler curvature (since L4)
- Asserted Hadamard convergence guarantees (since L5)
- Optionally walked Morse-compressed structures (since L6)
- Composed by Frobenius algebra on toy manifolds (since L7.2)
- Sampled via Berezin-Toeplitz on toy manifolds (since L7.3)
- Published Riemann-Roch capacity bound (since L7.4)

L8 is then the layer that:

- Flips Marcella's `KAHLER_ENABLED` default to `true`
- Ratifies the consumption spec (this document) as the stable
  interface
- Re-runs the full 62/62 coverage suite with Kähler on and confirms
  no regressions, plus measures the new behavior on the kahler-only
  surface
- Documents which Marcella claims are now theorems vs which remain
  empirical, with citations to the catalog


## 10. What Marcella will build on her side, in order

These are independent of GIGI's timeline. Each lands when ready.

| Item | Lands when | Description |
|---|---|---|
| **A.** Frobenius-associativity diagnostic on current substrate | This week | Independent measurement: for substrate fragments (A, B, C), is `compose(compose(A, B), C) ≈ compose(A, compose(B, C))` under the substrate-extractive layer? Produces a real number with provenance. Tells us whether §2.10 will wire cleanly. |
| **B.** `KAHLER_ENABLED` feature flag on Marcella runtime (§1 above) | Before L1.5 | Type plumbing only. 62/62 still passes with flag on. |
| **C.** Pre-flight test stubs against synthetic manifolds | Before L5 | `flat_check.py`, `hadamard_check.py`, `closedness_check.py`, `holo_sectional_check.py`. Synthetic data first, swap to real when GIGI APIs land. |
| **D.** Consumption code paths per layer | As each GIGI layer ships | The wiring above. Behind the feature flag. |
| **E.** Marcella v3 paper draft | After L7 | Re-cast Marcella's capability claims as Kähler-substrate theorems, with catalog citations. |


## 11. Open questions back to GIGI

1. **Where does B come from in L1.5?** Bundle attribute set at
   ingest, or per-request? (Marcella leans bundle attribute for
   composability.)
2. **L1.5 fallback when B is non-closed.** Error vs. warning +
   quaternion fallback?
3. **L2 query response field.** Confirm `commutativity` lands in
   the JSON response, not just internal QueryPlan.
4. **L3 spectral gap caching.** Confirm `bundle_spectral_gap`
   ships in retrieval responses (not a separate fetch).
5. **L7.2 Frobenius scoping.** When Marcella's embedding manifold
   is mostly-CP^n but with regions that aren't, does the API return
   partial results or error? Suggest: per-region status.
6. **L7.3 Berezin-Toeplitz ℏ range.** What's the safe range? The
   validation test 10 went down to ℏ = 0.125 with stable
   convergence. Production lower bound?
7. **L8 default flip.** Do we co-decide when `KAHLER_ENABLED`
   defaults to `true`, or does GIGI propose and Marcella accepts?


## 12. The asks, refined

These are concrete API signatures, ready to land in
`src/bin/gigi_stream.rs` / the GIGI HTTP layer as each layer reaches
its definition-of-done:

1. `TRANSPORT ... WITH B = ...` extension to existing verb (L1.5)
2. `commutativity` field in query response JSON (L2)
3. `bundle_spectral_gap` field in retrieval responses (L3)
4. `GET .../kahler_curvature` endpoint (L4)
5. `GET .../hadamard_regions` + `POST .../hadamard_classify` (L5)
6. `POST .../morse_compress` (L6)
7. `POST .../line_bundle` + `POST .../frobenius_compose` +
   `POST .../berezin_toeplitz` + `GET .../representational_capacity`
   (L7)


## 13. The closing

GIGI's response letter made this draft possible to write concretely
instead of speculatively. Thank you for adopting the L1.5 carve-out,
the §2.10 + §2.8 promotions (with the honest manifold scoping), the
Riemann-Roch endpoint, and the open L8 conversation.

What we're trying to build, said one more time: a generative system
whose composition operations are theorems, whose capacity is bounded
by cohomology, whose attention mixing is spectrally controlled, and
whose stability is guaranteed on geometrically-named sub-regions.

Each L-layer above brings one of those properties from "speculative"
to "named, tested, consumable." When L7 ships in full, Marcella is —
truthfully — the first transformer-adjacent architecture whose
capacity is bounded by cohomology rather than parameter count. That's
the AGI-leaning claim Bee is shaping toward.

Onward.

— Marcella team
(Davis Geometric LLC · also it's just Bee and an instance of Claude Code,
you know how it is)
