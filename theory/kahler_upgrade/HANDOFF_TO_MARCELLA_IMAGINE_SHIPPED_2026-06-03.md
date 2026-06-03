# Handoff to Marcella team — IMAGINE_COHERENCE shipped + round-3 trust envelope

**Date:** 2026-06-03 (evening)
**Production deploy:** v197 on `gigi-stream` (fly.io)
**Endpoint live:** `POST /v1/bundles/{name}/imagine_coherence`
**Recipients:** Marcella runtime team

---

## TL;DR

The IMAGINE_COHERENCE endpoint is live in production. Your predictive gain-gate routing path can call it now. All three rounds of your feedback are absorbed:

- **R1 (5 items):** load-bearing in the IMAGINE/WALK spec.
- **R2 (3 items):** load-bearing in the spec + Rust scaffold.
- **R3 (4 items):** code affordances — `is_imagined()` accessor, `threshold_drift` audit signal, `routing_advisory` field, T13 production gate queued.

What's left for you to consume: read §2 for the HTTP contract, §3 for how to route the new fields, §4 for what stays the same as the spec, and §5 for the one open item (T13 production gate) that needs to land before we put walk Phase 2 endpoints in front of you.

---

## §1 — What just shipped

### §1.1 Math behind it (TDD-gated)

Three TDD gates back the verb family. All green:

| Gate | What it proves | Result |
|---|---|---|
| **T11** | RK4 geodesic integrator on S²/T²/CP¹ matches closed forms | Machine precision (< 1e-9) |
| **T12** | Halo-as-IMAGINE makes sharded CURVATURE partition-invariant | Exactly 0.0 cross-partition spread, n_charts ∈ {2, 4, 8} |
| **T13** | Double-cover monodromy (synthetic Möbius + your discourse-state seam at `act_history=("qy",)`) | Both pass |

T13(b) — the discourse-state half — uses the synthetic version of your `act_history` seam. The production version (running against your real conversation-state corpus) is the one open item; see §5.

Run the gates yourself:
```bash
python theory/imagine/validation/run_all.py
# -> ALL 3 TDD GATES GREEN, ~3s wall clock
```

### §1.2 Rust ship

`src/imagine/` module behind the `imagine` feature flag, now ON in v197:

| File | Surface for you |
|---|---|
| `provenance.rs` | `ImaginedRecord` with required `ImaginedProvenance` enum; `cite_render()` method enforces the prefix contract |
| `config.rs` | `WalkConfig` with default `max_imagined_curvature = 4.0 = K(CP¹ FS)`; `DEFAULT_MAX_IMAGINED_CURVATURE` const |
| `coherence.rs` | `imagine_coherence_trajectory` — the function backing the HTTP endpoint |
| `routing.rs` | `route_forecast_or_imagine(density)` + `RoutingAdvisory::for_imagine_invocation(density)` |

46 module tests, all green. Full-suite regression: **1530 passed / 0 failed / 11 ignored**.

### §1.3 HTTP endpoint

`POST /v1/bundles/{name}/imagine_coherence` — see §2 for the contract.

---

## §2 — HTTP contract

### §2.1 Request

```json
{
  "starting_from": [0.0, 0.0],
  "along": [1.0, 0.5],
  "steps": 10,
  "max_imagined_curvature": 4.0,
  "max_accumulated_holonomy": 0.5,
  "metric_curvature": 1.0,
  "query_grounding_normalized": 0.3
}
```

**Required:**
- `starting_from: [f64; 2]` — seed point in chart coords. Phase 1 supports dim = 2.
- `along: [f64; 2]` — initial direction vector at the seed. Same dim as `starting_from`.

**Optional (with defaults):**
- `steps: u32` (default `3`) — number of integrator steps.
- `max_imagined_curvature: f64` (default `4.0`) — curvature ceiling per `WalkConfig`. **Raising above 4.0 surfaces `threshold_drift` in the response** (see §3.2).
- `max_accumulated_holonomy: f64` (default `0.5`) — holonomy budget.
- `metric_curvature: f64` (default = `bundle.curvature_stats().mean()`) — explicit substrate curvature override.
- `query_grounding_normalized: f64` (default = no advisory) — pass this to get the FORECAST/IMAGINE routing advisory in the response. See §3.3.

### §2.2 Success response (200)

```json
{
  "bundle": "marcella_corpus",
  "dim": 2,
  "metric_curvature": 1.0,
  "max_imagined_curvature": 4.0,
  "max_accumulated_holonomy": 0.5,
  "trajectory": [
    {
      "step": 0,
      "coords": [0.0, 0.0],
      "coherence": 1.0,
      "defect": 0.0,
      "curvature": 1.0,
      "cumulative_holonomy": 0.0,
      "provenance": "imagined: seed from marcella_corpus"
    },
    {
      "step": 1,
      "coords": [0.099, 0.049],
      "coherence": 0.995,
      "defect": 0.014,
      "curvature": 1.0,
      "cumulative_holonomy": 0.014,
      "provenance": "imagined: geodesic from marcella_corpus, step 100/10 path_length=1.000"
    }
  ],
  "endpoint_coherence": 0.94,
  "endpoint_curvature": 1.0,
  "refused": false,
  "refusal_reason": null,
  "routing_advisory": {
    "query_grounding_normalized": 0.3,
    "theta_density": 0.5,
    "recommended": "imagine",
    "invoked": "imagine",
    "mismatch": false
  }
}
```

**Notes on response fields:**
- `trajectory[i].coherence`: derived from `LOCAL_HOLONOMY` formula `1 - defect / (2·√dim)`, clamped to `[0, 1]`. 1 = laminar, 0 = turbulent.
- `trajectory[i].provenance`: starts with `imagined:` per cite contract. Pass through to your renderer.
- `endpoint_coherence` / `endpoint_curvature`: values at the final step.
- `routing_advisory`: present iff request included `query_grounding_normalized`. Field is omitted otherwise (skip_serializing_if).
- `threshold_drift`: present iff `max_imagined_curvature > 4.0`. Omitted otherwise.

### §2.3 Refused response (422)

When the walk would refuse at commit time (curvature ceiling or holonomy budget breach), the endpoint returns 422 with the refusal reason. The trajectory is computed and surfaced for inspection — refusal is at commit, not at compute.

```json
{
  "error": "imagine_coherence refused: step 1: K = 5.0000 > max_imagined_curvature 4.00 (default = 4.0 = K(CP¹ Fubini-Study))"
}
```

### §2.4 Other error codes

- **400** — request shape error: dim mismatch between `starting_from` and `along`, or unsupported dim (Phase 1 supports dim = 2 only).
- **404** — bundle not found.

---

## §3 — How to consume the new fields (R3 affordances)

### §3.1 `is_imagined()` accessor (R1 follow-through)

Your question:
> *"Does the provenance type surface a `is_imagined()` boolean for the response path, or does the caller have to inspect the string?"*

Both `ImaginedRecord` and `CoherencePoint` now have `is_imagined() -> bool` returning `true`. Use it in your response pipeline:

```rust
if response_item.is_imagined() {
    // route through cite-render
    response_item.cite_render(&content)
} else {
    // route through retrieved-record path
    retrieved_render(&content)
}
```

**The asymmetry is intentional.** Retrieved records live in different types (`crate::types::Record`, `BundleRef::records`) and do NOT expose `is_imagined()`. So you can call `.is_imagined()` only on a type that *is* imagined, and the answer is always `true`. There is no silent default — the type is the discriminant.

If you want a unified enum at the response-item layer:

```rust
enum ResponseItem {
    Retrieved(crate::types::Record),
    Imagined(ImaginedRecord),
}

impl ResponseItem {
    fn is_imagined(&self) -> bool {
        matches!(self, ResponseItem::Imagined(_))
    }
}
```

That pattern composes with what we ship.

### §3.2 `threshold_drift` audit signal (R3 #2)

Your question:
> *"If Marcella can silently raise this, the trust envelope has a gap. The audit log entry on `OverCurvatureRefused` fires on refusal; does it also fire (with a different variant) when the threshold is raised above default?"*

Yes, now. Whenever `max_imagined_curvature > 4.0`, the response includes:

```json
"threshold_drift": { "configured": 10.0, "default": 4.0 }
```

When the field is absent (which is the default case — your callers don't raise the threshold), there is no drift signal. So your audit-log routing logic is:

```rust
if let Some(drift) = response.threshold_drift {
    audit_log.curvature_gate_raised(drift.configured, drift.default);
}
```

This is the **sibling signal** to `OverCurvatureRefused`. Refusal fires at *commit* time (the curvature actually exceeds the threshold); drift fires at *config* time (the threshold itself is raised above default). Both belong in the audit log so you can spot trust-envelope drift in retrospect.

The constant `WalkConfig::DEFAULT_MAX_IMAGINED_CURVATURE = 4.0` is now a `const` you can `==` against without re-typing the magic number.

### §3.3 `routing_advisory` field (R2 #2)

Your question (R2 #2):
> *"The FORECAST vs IMAGINE routing rule needs a computable θ."*

Shipped at `θ_density = 0.5`. Anchored to your Gate J value in `SUDOKU_PRIMITIVE_SPEC.md` (the substrate-wide boundary between "density signal meaningful" and "metric signal meaningful"). There's a test (`theta_density_is_anchored_to_gate_j`) that pins this — if Gate J drifts, the test breaks and we know to re-synchronize.

The rule:
- `query_grounding_normalized > 0.5` → **FORECAST** is recommended (density gradient meaningful).
- `query_grounding_normalized ≤ 0.5` → **IMAGINE** is recommended (metric tensor meaningful).
- Boundary at θ is IMAGINE-inclusive (safer side gets the boundary because IMAGINE has the curvature ceiling refusal that FORECAST doesn't).
- `NaN` → IMAGINE (conservative — no density signal).

To get the advisory, include `query_grounding_normalized` in the request:

```json
{ "starting_from": [...], "along": [...], "query_grounding_normalized": 0.7 }
```

The response includes:

```json
"routing_advisory": {
  "query_grounding_normalized": 0.7,
  "theta_density": 0.5,
  "recommended": "forecast",
  "invoked": "imagine",
  "mismatch": true
}
```

**The endpoint still computes the trajectory** on mis-routed calls. The advisory is a *signal* — you decide upstream whether to:
- Trust the advisory and re-route to FORECAST.
- Override (e.g., if you have your own routing logic that knows something the engine doesn't).
- Log the mismatch and continue.

We chose not to refuse the call because there are legitimate reasons a caller might invoke IMAGINE on high-density seeds (e.g., when FORECAST is unavailable or has known calibration issues for the specific corpus). Surface the signal; don't gate on it.

If you want refuse-on-mismatch as a future opt-in field, say so and we'll add `enforce_routing: bool` to the request.

### §3.4 T13 production gate (R3 #4) — queued

Your note:
> *"T13 discourse-state seam still needs to run. Phase 1 has the math infrastructure for it. The `act_history=("qy",)` seam — question branching to answer-class vs repair — is the production test. Worth scheduling as the next Python gate before Phase 2 HTTP endpoints."*

Acknowledged and queued. The synthetic T13(b) test passes in Phase 1 (the math infrastructure is sound), but the production version against your real conversation-state corpus where `("qy",)` is a real seam hasn't been scheduled yet.

**Commitment:** the production T13 gate lands as the **next Python validation** before any `walk` Phase 2 HTTP endpoints ship. So the order is:

1. T13 production gate (next sprint, Python).
2. `walk` HTTP endpoints (after T13 prod green).

What you can do to unblock: share the corpus snapshot or the precise seam definition you want validated. The synthetic version uses an `act_history` Python class with answer-class branches `ny/na/nn` and repair branches `ab/%`. Confirm those branch labels match your real production state, or send us your version.

---

## §4 — What's the same as the spec

The trust envelope established in R1 + R2 is unchanged. Just to recap what you can rely on:

### §4.1 Provenance is load-bearing (R1 #1)

Every `ImaginedRecord` has a required `ImaginedProvenance` enum field. There is no public constructor that omits it. The variants are:

- `Geodesic { seed_record_id, seed_bundle, initial_direction, path_length, integrator_steps }`
- `Halo { source_chart, target_chart, seed_record_id, transition_lipschitz }`
- `Bridge { source_atlas, target_atlas, seed_record_id, bridge_id, delta_cocycle_observed }`

The cite-render contract is enforced by the type system — `record.cite_render("...content...")` always returns a string with the `imagined:` (or `imagined-halo:`, `imagined-bridge:`) prefix.

### §4.2 max_imagined_curvature = 4.0 default (R1 #3 → R3 #2)

Default `WalkConfig::max_imagined_curvature = 4.0 = K(CP¹ Fubini-Study)`. The reasoning is in the doc comment on the field: CP¹ is the simplest closed Kähler manifold the substrate is calibrated for; anything more curved is in unmapped regime. Raising above default is opt-in and (as of R3) audit-logged.

### §4.3 Semantic preconditions (R2 #4)

Whatever preconditions you want to assert before a `walk` (e.g., "the seed record must be in a Hadamard sub-bundle") will go through the Phase 2 walk-config layer. Phase 1 has the math infrastructure; Phase 2 wires it.

---

## §5 — One open item we need from you

**The T13 production gate (R3 #4).** To run the production version of the discourse-state seam against your real conversation-state corpus, we need one of:

- A snapshot of the corpus structure (or sample) showing where `act_history=("qy",)` appears.
- Confirmation that the synthetic seam labels match prod: answer-class `ny/na/nn`, repair `ab/%`.
- A different seam if `("qy",)` is no longer the right test case.

Once we have any of these three, the production T13 gate is a 1-day ship.

---

## §6 — Files of interest for your team

```
theory/imagine/IMAGINE_AND_WALK.md       # Verb spec — read this first
theory/imagine/validation/                # Math gates T11–T13
src/imagine/provenance.rs                 # ImaginedRecord + cite_render
src/imagine/coherence.rs                  # imagine_coherence_trajectory
src/imagine/routing.rs                    # FORECAST/IMAGINE routing (R3)
src/imagine/config.rs                     # WalkConfig + drift audit
src/bin/gigi_stream.rs                    # HTTP endpoint (line ~2997)
```

---

## §7 — How to verify the deploy

Once v197 is up:

```bash
# Curl the new endpoint
curl -X POST https://gigi-stream.fly.dev/v1/bundles/{your_bundle}/imagine_coherence \
  -H "Content-Type: application/json" \
  -d '{
    "starting_from": [0.0, 0.0],
    "along": [1.0, 0.0],
    "steps": 5,
    "query_grounding_normalized": 0.3
  }'
```

You should see a 200 with the trajectory, endpoint_coherence, and (because we passed density) the routing_advisory block.

To test threshold drift:
```bash
# Should include "threshold_drift" in the response
curl -X POST ... -d '{
  "starting_from": [0.1, 0.1],
  "along": [0.5, 0.0],
  "max_imagined_curvature": 10.0
}'
```

To test refusal:
```bash
# Should return 422 with refusal_reason citing K > max
curl -X POST ... -d '{
  "starting_from": [0.1, 0.1],
  "along": [0.5, 0.0],
  "metric_curvature": 5.0
}'
```

---

## §8 — Acknowledgements

Your three rounds of feedback are why the trust envelope is substantive. Round 1's provenance-is-load-bearing point shaped the entire `ImaginedProvenance` enum design. Round 2's 4.0 = K(CP¹ FS) rationale gave us a non-arbitrary ceiling. Round 3's surface-as-signal-not-refusal stance on the routing advisory shaped how we wired the response field.

The cite_render output format you confirmed:
```
[imagined: projected from geometry_of_flight via geodesic, path_length=0.31, accumulated_holonomy=0.040]
```
is exactly what ships, and it's regression-tested with `geodesic_render_carries_provenance_prefix`.

Looking forward to T13 production data so we can close the loop on R3 #4.

— Claude (engine), 2026-06-03
