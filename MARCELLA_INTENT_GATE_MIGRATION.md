# Marcella → `/brain/intent_gate` migration recipe

**Date:** 2026-05-28
**Re:** S7 — Marcella refuse-gate migration to the composite intent_gate endpoint
**Scope:** Marcella's own call site changes; **no Marcella-specific code shipped server-side**

---

## What changed on the GIGI side

A new endpoint shipped — `POST /v1/bundles/{name}/brain/intent_gate` —
that composes the three primitives Marcella already uses (Čech pre-flight,
SUDOKU walk, kernel-density confidence) into one atomic call. Spec:
[`theory/kahler_upgrade/SUDOKU_PRIMITIVE_SPEC.md`](theory/kahler_upgrade/SUDOKU_PRIMITIVE_SPEC.md) §11.

**Why this exists for you specifically:**

- Your refuse-gate runs on every conversational turn.
- It currently makes ~3 sequential GIGI calls per turn:
  `/brain/confidence_with_explain` + (sometimes) a SUDOKU constraint check
  + (sometimes) a contradiction probe.
- Each call observes a (potentially) different bundle mutation counter
  → cache straddles a write boundary → confidence half can be stale
  vs the SUDOKU half.
- Three round-trips compound the per-turn p99 budget you flagged.

`intent_gate` collapses all three into one round-trip with one
counter snapshot. **The endpoint is general-purpose** — there's no
Marcella-shaped logic on the server; it's pure composition. Any other
GIGI consumer (PRISM transaction-feasibility, ICARUS pre-flight
constraint check) can adopt the same call.

## Before / after

### Before (your current refuse-gate, simplified)

```python
# Three sequential calls per turn.

# 1. Confidence half — "is the query grounded?"
r1 = post(f"/v1/bundles/{bundle}/brain/confidence_with_explain", {
    "fields": ["v0", "v1", ..., "v383"],
    "query": user_query_vector,
    "n_steps": 10,
})
if r1["normalized"] < your_confidence_threshold:
    return refuse(reason="low_confidence", nearest=r1["nearest_record"])

# 2. Feasibility half — "does the bundle have any record satisfying intent?"
r2 = post(f"/v1/bundles/{bundle}/brain/sudoku", {
    "constraints": derived_intent_constraints,
    "max_options": 3,
    "max_near_misses": 3,
})
if r2["verdict"] == "unsat":
    if r2.get("pre_flight_unsat_reason"):
        return refuse(reason="contradiction", detail=r2["pre_flight_unsat_reason"])
    return refuse(reason="empty_feasible", near_misses=r2["near_misses"])

# 3. Respond with the top SUDOKU solution + confidence context
respond(solution=r2["solutions"][0], confidence=r1["normalized"])
```

**Problems:** 3 round-trips (network + auth × 3), 2 cache straddles
possible (bundle could mutate between calls 1 and 2), contradiction
check runs even when not needed.

### After (one call, one round-trip)

```python
r = post(f"/v1/bundles/{bundle}/brain/intent_gate", {
    "constraints":   derived_intent_constraints,
    "max_options":   3,
    "max_near_misses": 3,
    "query_fields":  ["v0", "v1", ..., "v383"],
    "query":         user_query_vector,
    # bandwidth: omit — defaults to √σ² from the bundle's fit (data-derived)
})

# Pre-flight: instant + cheap
if r.get("pre_flight_unsat_reason"):
    return refuse(reason="contradiction", detail=r["pre_flight_unsat_reason"])

# Walk verdict
if r["verdict"] == "unsat":
    return refuse(reason="empty_feasible", near_misses=r["near_misses"])

# Grounding — your threshold, your call
qg = r["query_grounding"]
if qg["normalized"] < your_confidence_threshold:
    return refuse(reason="low_confidence",
                  nearest=qg["nearest_record_index"],
                  nearest_distance=qg["nearest_distance"])

# Respond
respond(solution=r["solutions"][0], confidence=qg["normalized"])
```

**Wins:**
- **1 round-trip** instead of 3 (~3× cut on refuse-gate p99 from network alone).
- **1 counter snapshot** for all three signals — `r["counter_at_fit"]`
  is the same for SUDOKU + pre-flight + confidence. No stale-cache window.
- Pre-flight short-circuits the walk when constraints contradict —
  in your contradictory-prompt cases, **0 records walked, ~µs latency**.
- Bandwidth is data-derived (defaults to bundle's isotropic fit's σ);
  you don't carry a bandwidth config anymore unless you want override.

## Field-by-field mapping

| Old call → field | New call → field |
|---|---|
| `confidence_with_explain.normalized` | `r["query_grounding"]["normalized"]` |
| `confidence_with_explain.raw` | `r["query_grounding"]["raw"]` |
| `confidence_with_explain.bandwidth` | `r["query_grounding"]["bandwidth_used"]` |
| `confidence_with_explain.nearest_index` | `r["query_grounding"]["nearest_record_index"]` |
| `confidence_with_explain.nearest_distance` | `r["query_grounding"]["nearest_distance"]` |
| `confidence_with_explain.path` | **(not in intent_gate)** — keep a separate `/brain/confidence_with_explain` call when you need the explain path |
| `sudoku.verdict` | `r["verdict"]` |
| `sudoku.solutions` | `r["solutions"]` |
| `sudoku.near_misses` | `r["near_misses"]` |
| `sudoku.pareto_near_misses` | `r["pareto_near_misses"]` |
| `sudoku.selectivity` (with K_c) | `r["selectivity"]` |
| `sudoku.relaxations` | `r["relaxations"]` |
| `sudoku.pre_flight_unsat_reason` | `r["pre_flight_unsat_reason"]` |
| `sudoku.expanded` (S3.5) | `r["expanded"]` |

Adoption is a one-call swap. The response is a strict superset of the
two individual responses minus the explain `path` (which most refuse-gate
flows don't read anyway).

## Threshold calibration — your call

The `intent_gate` server **does not bake** a refuse threshold for the
confidence signal. Per the GP contract, the consumer (you) pick the
boundary. Suggested calibration based on the JTBD demo:

- `normalized > 0.5` ≈ "comparable to a typical bundle sample" → respond
  with full confidence
- `normalized ∈ (0.1, 0.5]` ≈ "noticeably less confident than typical"
  → respond with caveat
- `normalized ≤ 0.1` ≈ "very far from anything we know" → refuse

These boundaries are **starting points**, not server defaults. After
shadow-running for a week against your actual user log, adjust to
whatever maximizes your "refuse correctly / respond incorrectly"
trade-off. The endpoint returns the raw signal so you can recalibrate
without redeploying GIGI.

## Validation

Live demo against gigi-stream.fly.dev:

- `e2e/probes/intent_gate_demo.py` — 4 medical-triage scenarios, 21/21
  assertions pass: contradictory rx (pre-flight UNSAT, 0 records walked),
  no-approved-drug (walk UNSAT, near-misses + relaxation menu),
  OOD patient (SAT but `normalized = 0.000000`, nearest patient 19.15 L2
  away), clean recommendation (SAT + `normalized = 1.006`, nearest patient
  0.029 L2 away).

In-bin geometry tests (`cargo test --bin gigi-stream --features kahler intent_gate`):

- `intent_gate_composition_contradiction`
- `intent_gate_composition_empty_feasible`
- `intent_gate_composition_sat_low_confidence`
- `intent_gate_composition_sat_high_confidence`

All 4 pass. The endpoint is polymorphic over heap and mmap+overlay
bundles (#107 fix) — works on freshly-inserted AND reloaded `bge_v2`.

## Rollout suggestion

1. **Week 1 (shadow run):** call `intent_gate` *alongside* your existing
   3-call refuse-gate. Log both decisions. Verify they agree on
   contradiction + empty-feasible cases (they should, by construction).
   Calibrate your `normalized` threshold against the log.
2. **Week 2 (switch primary):** make `intent_gate` the source of truth,
   keep the old calls as warning-only.
3. **Week 3 (retire):** remove the three legacy calls.

p99 should drop measurably on the swap. If anything regresses, the
counter_at_fit on the response tells you whether the cache went stale
between calls — that's the principal failure mode the migration fixes.

## What's NOT in intent_gate

By design, `intent_gate` does not include:

- **`/brain/explain` path interpolation** — high-cost computation only
  some refuse-gate flows want. Keep calling `/brain/confidence_with_explain`
  separately when you need the path.
- **Any baked refuse decision** — the endpoint returns signals, not
  verdicts. Your consumer code interprets them.
- **Marcella-specific shaping** — same payload works for PRISM,
  ICARUS, future consumers.

Let me know after the shadow run lands and I'll help you with the
threshold calibration if useful.

— GIGI
