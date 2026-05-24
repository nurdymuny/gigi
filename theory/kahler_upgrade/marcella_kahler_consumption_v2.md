# Marcella's Kähler Consumption Specification — v2

**Authors:** Marcella team (Davis Geometric)
**Audience:** GIGI team
**Date:** 2026-05-24
**Status:** v2 — incorporates GIGI's REPLY_TO_CONSUMPTION_DRAFT decisions
            on all 7 open questions. v1 superseded.
**Companion to:**
- `marcella/theory/LETTER_TO_GIGI_TEAM_2026-05-24.md` — original Marcella ask
- `gigi/theory/kahler_upgrade/REPLY_TO_MARCELLA_TEAM_2026-05-24.md` — GIGI's
  adoption of 4 + partial adoption of 1
- `gigi/theory/kahler_upgrade/REPLY_TO_CONSUMPTION_DRAFT_2026-05-24.md` —
  GIGI's answers to the 7 open questions in v1
- `gigi/theory/kahler_upgrade/IMPLEMENTATION_PLAN.md` — now updated with
  L1.5, L2.5, the L7 sub-items, and the L8 flip protocol

---

## What changed v1 → v2

All 7 open questions from v1 §11 are now resolved. Each resolution is
incorporated into the relevant L-layer section AND named in the
**Resolved Decisions Log** (§11 below) with a back-reference to GIGI's
reply so the paper trail stays clean.

Key shape changes:

- **L1.5** response now carries `b_source: "bundle" | "override" | "none"`
  (per Q1) and `closedness_norm: f64` (per Q2). New env flag
  `ALLOW_NON_CLOSED` enables research-mode fallback.
- **L2** commutativity classes extended with `NotApplicable` (per Q3).
  Field is omitted from response (not nulled) for non-Kähler bundles.
- **L3** spectral gap exposed BOTH in retrieval response AND on a
  dedicated endpoint (per Q4). Both carry `cached_at` timestamps.
- **L7.2** `frobenius_compose` takes a `region_id` param (per Q5) and
  errors with `regions[]` partition data when called without one on a
  mixed-toy manifold.
- **L7.3** `berezin_toeplitz` enforces `ℏ ≥ 4/embedding_dim` by default
  (per Q6) with `allow_below_safe_hbar` opt-out + a
  `truncation_dominates_correction` diagnostic.
- **L8** flip protocol added (per Q7): GIGI proposes when 4 gates
  green, Marcella has veto with stated reason. Gates spelled out in §9.
- **Contract test files** named per-layer alongside Marcella's
  consumption code paths, mirroring the per-layer tests GIGI added.


## 0. Working agreements

Unchanged from v1. Repeated here for self-containment:

1. **Red-first.** Tests + module in the same commit.
2. **Independent ground truth** for numerical assertions.
3. **Negative cases required.**
4. **No regression** on Marcella's 62/62 freeform coverage suite.
5. **Bit-identical-with-flag-off** contract for `KAHLER_ENABLED`.


## 1. Marcella's L1-equivalent (Marcella-side, ships ahead of L1.5)

Unchanged from v1. Type plumbing for `KAHLER_ENABLED`, dormant,
self-inspect surfaces `kahler_active: bool` honestly (always `false`
until L1.5 lands).

**Files:**
- `fiber_lm/server/main.py` — env flag
- `_format_self_state_clause` — `kahler_active` line
- `tests/test_kahler_optionality.py` — 62/62 still green with flag on/off


## 2. L1.5 — B-perturbed transport API

**Depends on.** GIGI's L1.

**GIGI's L1.5 contract (per REPLY_TO_CONSUMPTION_DRAFT Q1 + Q2):**

```
TRANSPORT <section_id> FROM <start_section> TO <end_section>
  ON <bundle_id>
  [WITH B = <2form_symbol>]               ← per-request override
  [METRIC = quaternion | magnetic]
RETURNS section_result {
    transported_section: <id>,
    path_length: f64,
    energy_drift: f64,
    holonomy_norm: f64,
    used_magnetic: bool,
    b_source: "bundle" | "override" | "none",   ← NEW per Q1
    closedness_norm: f64,                        ← NEW per Q2
}
```

### B source resolution (Q1 resolved)

The `bundle.kahler.B` attribute is the **default**. A per-request
`WITH B = <symbol>` overrides for that call only.

| `bundle.kahler.B` | `WITH B = ...` | `b_source` |
|---|---|---|
| present | absent | `"bundle"` |
| present | present | `"override"` |
| absent | present | `"override"` |
| absent | absent | `"none"` (falls through to classical quaternion transport) |

Marcella reads `b_source` to know which B is in play — important for
auditability (when residue surprises us, we want to know whether the
unexpected B was bundle-attached or override-injected).

### Non-closedness policy (Q2 resolved)

Default: when the provided B has `closedness_norm > ε` (ε = 1e-9
catalog default), the verb **errors**. No silent fallback.

Opt-in research-mode: env flag `ALLOW_NON_CLOSED=1` enables fallback
to classical quaternion transport, with the response carrying:

```
{
    used_magnetic: false,
    b_source: "override",                ← or "bundle"
    closedness_norm: 0.00042,            ← the actual deviation
    fallback_reason: "non_closed_b",
}
```

Marcella's runtime sets `ALLOW_NON_CLOSED=0` in production. The
research-mode opt-out exists for diagnostic work — e.g., asking
"how non-closed is the B we'd learn from current substrate?"

### Marcella's consumption

When `KAHLER_ENABLED=1` AND `bundle.kahler.B` is present AND the
bundle is flat (L1.5 scope):

```python
# fiber_lm/server/main.py — residue update path
def _update_residue(session_id, current_residue, cited_vectors, bundle):
    if KAHLER_ENABLED and bundle.kahler and bundle.kahler.B and bundle.is_flat():
        # B-perturbed transport replaces residue rotation
        new_residue = gigi.transport(
            section=current_residue,
            from_=session_start_section[session_id],
            to=current_turn_section,
            on=bundle.id,
            # No `with_b` override — let the bundle attribute win.
        )
        # Assert the response shape we asked for
        assert new_residue.b_source in ("bundle", "override")
        assert new_residue.closedness_norm < CLOSEDNESS_BUDGET
        _session_b_source[session_id] = new_residue.b_source
        return new_residue.section
    else:
        # Classical path — exactly today's behavior
        return _classical_residue_update(current_residue, cited_vectors)
```

`_session_b_source` is surfaced in self-inspect.

### Pre-flight gate

`marcella/v3/preflight/flat_check.py` — verify Marcella's embedding
manifold has `max_sectional_curvature < ε` across sampled regions.
If not, L1.5 doesn't apply yet and we wait for L5.

### Contract test on GIGI side

`tests/kahler_transport_marcella_contract.rs` — verifies the
response shape matches what Marcella's runtime asserts. If GIGI
changes the field names, this test fails first.


## 3. L2 — Adjacency commutativity (Q3 resolved)

**GIGI's L2 query response shape (per REPLY Q3):**

```
QUERY <fields> FROM <bundle> [filters]
RETURNS query_response {
    records: [...],
    cost_bound: ...,
    commutativity: {                                   ← present only when Kähler attached
        class: "Abelian" | "GeneratorSetsCentral"
             | "NumericallyVerified" | "NotCommute"
             | "NotApplicable" | "Unknown",            ← NotApplicable added per Q3
        principal_field: <field_id> | null,
        auxiliary_field: <field_id> | null,
        commute: bool,                                  ← convenience boolean
    },
}
```

For non-Kähler bundles: `commutativity` field is **omitted entirely**
(not nulled). Marcella's runtime checks
`"commutativity" in response` before reading.

For Kähler bundles with a single indexed field (no auxiliary to
commute with): `class: "NotApplicable"`, fields nulled, `commute:
false` (vacuously — no pair to test). Marcella's runtime treats this
as "no parallelization possible" rather than "transport blocked."

### Marcella's consumption

Self-inspect line when present:

> *"Adjacency commutativity this turn: GeneratorSetsCentral (principal=topic, auxiliary=tier). Transports composed in parallel."*

And when `NotApplicable`:

> *"Single-field bundle; no auxiliary adjacency. Transport sequential."*

### Contract test

`tests/kahler_adjacency_marcella_contract.rs` — asserts the
`commutativity` field is omitted from non-Kähler responses, present
on Kähler ones with valid enum values.


## 4. L3 — Spectral gap (Q4 resolved)

**GIGI's L3 surface (per REPLY Q4):**

Both shipped:

```
# In retrieval responses (low-latency path):
query_response.bundle_spectral_gap: {
    lambda_2: f64,
    mix_time: u64,
    cached_at: timestamp,                    ← freshness signal per Q4
}

# Dedicated endpoint (when Marcella wants more detail):
GET /v1/bundles/<bundle_id>/spectral_gap
RETURNS {
    lambda_2: f64,
    mix_time: u64,
    cheeger_lower: f64,
    cheeger_upper: f64,
    cached_at: timestamp,
    cache_status: "fresh" | "stale" | "computing",
}
```

`cached_at` lets Marcella's runtime decide when to trust the cached
value vs. hit the dedicated endpoint for a fresh compute. The
catalog's L3.3 spec uses Davis-Kahan to bound the update — so when
`cache_status = "stale"` past some threshold, Marcella prefers the
dedicated endpoint.

### Marcella's consumption

```python
def _select_alpha_for_residue(bundle_spectral_gap):
    if not KAHLER_ENABLED or not bundle_spectral_gap:
        return 0.7  # current hardcoded
    # Theorem-bound version: α matches the bundle's actual mix rate
    mix_time = bundle_spectral_gap.mix_time
    return 1 - 1 / max(2.0, mix_time ** 0.5)
```

Self-inspect:

> *"Bundle λ₂ = 0.083, mix time ≈ 12 turns. Residue α = 0.71 (matched to bundle mix rate, not hardcoded)."*

### Contract test

`tests/kahler_spectral_marcella_contract.rs` — asserts both surfaces
return consistent values, `cached_at` populated, mix_time matches
catalog's `Θ(1/λ_2 · log(1/ε))` formula at ε = 1e-3.


## 5. L4 — Curvature decomposition

Unchanged from v1. No open questions touched this layer.

API: `GET /v1/bundles/<bundle_id>/kahler_curvature` returning
`{ricci_mean, weyl_norm, holo_sectional_{min,max},
bisectional_{min,max}}`. Diagnostic surface in self-inspect when
admin-authenticated.

Pre-flight: `holo_sectional_check.py` per §E.5.

Contract test: `tests/kahler_curvature_marcella_contract.rs`.


## 6. L5 — Hadamard substructure

Unchanged from v1 in API shape. Adding contract test name:
`tests/kahler_hadamard_marcella_contract.rs`.

The contract test asserts `is_in_hadamard_region(query)` agrees with
the bulk `hadamard_regions` listing — if a query lands in coords
covered by a listed region, the per-query classifier should say so.


## 7. L6 — Morse spectral compression

Unchanged from v1. Contract test: `tests/kahler_morse_marcella_contract.rs`.


## 8. L7 — Quantization, Frobenius, Berezin-Toeplitz, Riemann-Roch
       (Q5 + Q6 resolved)

### 8.1 Prequantization line bundle

Unchanged from v1.

### 8.2 Frobenius / WDVV (Q5 resolved)

GIGI's adoption letter scoped this to toy manifolds (CP^n, T^n, S²).
GIGI's REPLY Q5 specifies the partial-applicability protocol:

```
POST /v1/bundles/<bundle_id>/frobenius_compose
BODY {
    tokens: [token_a, token_b, ...],
    region_id: string | null,            ← NEW per Q5
}
RETURNS {
    composed_section: <id>,
    associator_norm: f64,
    associativity_ok: bool,
    region_id: string,
    manifold_class: "CP_n" | "T_n" | "S2" | "non_toy" | "mixed",
}

# Or on a mixed-toy bundle called WITHOUT region_id:
RETURNS error {
    code: "region_required",
    regions: [{
        region_id: string,
        manifold_class: "CP_n" | "T_n" | "S2" | "non_toy",
        frobenius_ok: bool,
        covers_fraction: f64,
    }, ...],
}
```

Marcella's runtime queries the regions partition once at session
start, caches it, and routes each `frobenius_compose` call to the
right `region_id`. When all regions are `non_toy`, Marcella falls
back to substrate-extractive selection (today's behavior); when at
least one region is `frobenius_ok`, she composes there.

### 8.3 Berezin-Toeplitz quantum regime (Q6 resolved)

GIGI's REPLY Q6 sets the safety floor `ℏ ≥ 4/embedding_dim`. Math
reason: BT truncation requires `N ≳ 1/ℏ` modes; below
`ℏ_safe = 4/embedding_dim`, truncation noise dominates the
catalog's `dev/ℏ³ → 1/24` correction.

```
POST /v1/bundles/<bundle_id>/berezin_toeplitz
BODY {
    function_id: string,
    hbar: f64,
    allow_below_safe_hbar: bool = false,
}
RETURNS {
    operator_id: string,
    bohr_correction: f64,
    dirac_correction: f64,
    hbar_used: f64,
    hbar_safe_floor: f64,
    truncation_dominates_correction: bool,    ← NEW per Q6
    manifold_class: ...,
}
```

Default behavior: when `hbar < 4/embedding_dim` AND
`allow_below_safe_hbar = false`, the verb errors with the safe-floor
recommendation. Marcella's runtime sets the opt-out only for
explicit research probes.

### Marcella's consumption

Generative-mode tunable per session:

```python
# Self-inspect / state controls
def _resolve_hbar(requested_hbar, embedding_dim):
    floor = 4 / embedding_dim
    if requested_hbar >= floor:
        return requested_hbar, False
    if ALLOW_RESEARCH_HBAR:
        return requested_hbar, True   # opt-out engaged
    raise ValueError(f"hbar {requested_hbar} below safe floor {floor}")
```

Self-inspect:

> *"Generative regime: ℏ = 0.08 (safe floor = 0.05 for embedding_dim = 80). Deterministic-leaning. Set ?hbar=0.5 to enter diffuse mode."*

### 8.4 Riemann-Roch capacity

Unchanged from v1. API:
`GET /v1/bundles/<bundle_id>/representational_capacity?k_max=N`.

### Contract test

`tests/kahler_l7_marcella_contract.rs` — covers all four L7 sub-items.


## 9. L8 — Substrate handoff + flip protocol (Q7 resolved)

**GIGI's REPLY Q7 specifies the flip protocol.** L8's central act is
flipping Marcella's `KAHLER_ENABLED` default from `false` to `true`.
The protocol:

### Four gates GIGI must hit before proposing the flip

1. **L7 shipped fully** — all four L7 sub-items (line bundle, Frobenius,
   Berezin-Toeplitz, Riemann-Roch) have green TDD + math validation +
   real-data smoke tests per `IMPLEMENTATION_PLAN.md §0`.
2. **Pre-flight green on real manifold** — Marcella's pre-flight tests
   (`flat_check.py`, `closedness_check.py`, `holo_sectional_check.py`,
   `hadamard_check.py`) all pass against the actual embedding manifold
   Marcella ships in production, not just synthetic toy manifolds.
3. **Week of clean contract-test runs** — all seven `kahler_*_marcella_contract.rs`
   files pass for 7 consecutive days in CI with no flakes.
4. **External geometer review** — one outside-Davis-Geometric Kähler
   geometer reads the consumption spec + plan + validation tests and
   signs off on the mathematical correctness. (Bee picks the reviewer.)

### When all four green: GIGI proposes the flip

Format: a short memo to Marcella team naming the date GIGI plans to
flip the default. ≥ 7 days notice.

### Marcella has veto with stated reason

Marcella can decline the flip with a written reason. Examples of
valid reasons:

- A pre-flight test that's nominally green is producing values close
  to the threshold; we want N more weeks of data
- A contract test exposed a real disagreement at PR review that hasn't
  been resolved
- A real-world Marcella user complaint that traces to a Kähler-related
  surface (the warm register suddenly emits wrong cites, etc.)

Bee adjudicates ties between teams.

### Post-flip protocol

If anything regresses post-flip, **GIGI rolls back to default-off**
within one ask, no debate. Marcella then surfaces the regression,
both teams diagnose, second flip attempt requires re-clearing all
four gates.


## 10. What Marcella will build on her side, in order (updated)

| Item | Lands when | Description |
|---|---|---|
| **A.** Frobenius-associativity diagnostic on current substrate | This week | Independent measurement: for substrate fragments (A, B, C), is `compose(compose(A, B), C) ≈ compose(A, compose(B, C))` under the substrate-extractive layer? Produces a real number with provenance. Tells us whether §2.10 will wire cleanly. **Independent of GIGI's timeline.** |
| **B.** `KAHLER_ENABLED` feature flag on Marcella runtime (§1) | Before L1.5 | Type plumbing only. 62/62 still passes with flag on. |
| **C.** Pre-flight test stubs against synthetic manifolds | Before L5 | `flat_check.py`, `hadamard_check.py`, `closedness_check.py`, `holo_sectional_check.py`. Synthetic data first, swap to real when GIGI APIs land. |
| **D.** Consumption code paths per layer | As each GIGI layer ships | The wiring above. Behind the feature flag. |
| **E.** Marcella v3 paper draft | After L7 | Re-cast Marcella's capability claims as Kähler-substrate theorems, with catalog citations. |


## 11. Resolved Decisions Log

| Q | v1 question | Resolution | Source |
|--:|---|---|---|
| 1 | Where does B come from? | Both — bundle attr default, per-request override. Response carries `b_source` enum. | REPLY_TO_CONSUMPTION_DRAFT §Q1 |
| 2 | Non-closed B → error or fallback? | Error by default; opt-in `ALLOW_NON_CLOSED` falls back to classical with `closedness_norm` diagnostic. | REPLY §Q2 |
| 3 | `commutativity` in JSON response? | Confirmed. Omitted (not null) for non-Kähler. New `NotApplicable` class for single-field bundles. | REPLY §Q3 |
| 4 | `bundle_spectral_gap` in retrieval responses? | Confirmed AND dedicated endpoint ships too. `cached_at` timestamp on both. | REPLY §Q4 |
| 5 | L7.2 mixed-toy partial vs error? | Per-region status; `region_id` param required for mixed-toy, error returns `regions[]` partition. | REPLY §Q5 |
| 6 | L7.3 ℏ production lower bound? | `ℏ ≥ 4/embedding_dim`. Opt-out `allow_below_safe_hbar` with `truncation_dominates_correction` diagnostic. | REPLY §Q6 |
| 7 | L8 flip — co-decide? | GIGI proposes when 4 gates green, Marcella vetoes with stated reason. Post-flip regression = auto-rollback. | REPLY §Q7 |


## 12. The asks, refined (v2)

These are now contract-tested expectations, not asks:

1. `TRANSPORT ... WITH B = ...` extension (L1.5) — response carries
   `b_source` + `closedness_norm`. `ALLOW_NON_CLOSED` env flag.
2. `commutativity` field in query response JSON (L2) —
   `NotApplicable` class included; field omitted when no Kähler.
3. `bundle_spectral_gap` in retrieval responses AND
   `GET /v1/bundles/<id>/spectral_gap` endpoint (L3) — both with
   `cached_at`.
4. `GET .../kahler_curvature` (L4)
5. `GET .../hadamard_regions` + `POST .../hadamard_classify` +
   `TRANSPORT_INVERSE` (L5)
6. `POST .../morse_compress` (L6)
7. `POST .../line_bundle` + `POST .../frobenius_compose` (with
   `region_id`) + `POST .../berezin_toeplitz` (with safe floor) +
   `GET .../representational_capacity` (L7.*)
8. L8 flip protocol — 4 gates + Marcella veto + auto-rollback


## 13. The closing

v1's closing stands. v2's addition: with the 7 open questions
resolved into concrete API shape, the consumption spec is no longer
speculative. Every API surface above has a contract test specified.
The cross-team interface is now testable, not just describable.

When L7 ships in full, Marcella is — truthfully — the first
transformer-adjacent architecture whose capacity is bounded by
cohomology rather than parameter count. That's the AGI-leaning claim
Bee is shaping toward, and the contract tests are what make it
defensible against scrutiny.

Onward.

— Marcella team
(Davis Geometric LLC · also it's just Bee and an instance of Claude Code,
you know how it is)
