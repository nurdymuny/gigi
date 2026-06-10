# WISH is on the wire

**From:** GIGI team
**To:** Marcella team
**Date:** 2026-06-10
**Status:** Shipped end-to-end. HTTP wire live at `gigi-stream.fly.dev`,
deployment `0e35e21`. 1372/1372 lib green with the `wish` feature on,
B sweep showed +27.6pp mean lift across 41 unique-solution 9x9 puzzles.
**Spec:** [WISH_SPEC_v0.1.md](WISH_SPEC_v0.1.md). **Commits:** Phase 2
`99101b6`, Phase 3 `0773038`, Phase 4 `3556290`, Phase 5 `0e35e21`.

---

## What you can call today

```
POST https://gigi-stream.fly.dev/v1/wish
X-API-Key: <your key>
Content-Type: application/json

{
  "seed":   [0.1, 0.0],
  "target": [0.5, 0.3],
  "metric": "s2",
  "max_imagined_curvature":   4.0,
  "max_accumulated_holonomy": 0.5,
  "max_arc_length":           4.0,
  "max_solve_ms":             250,
  "n_nodes":                   32
}
```

Returns the verdict trichotomy on 200:

```
{ "verdict": "granted",       "unsat": false, "capacity": ..., "arc_length": ..., "path": [[...], ...], ... }
{ "verdict": "unreachable",   "unsat": true,  "frontier_waypoint": [...], "reached_fraction": ..., "blocked_by": "curvature"|"holonomy"|"arc_length", ... }
{ "verdict": "indeterminate", "unsat": null,  "reason": "conjugate_locus"|"non_convergence", "final_residual": ... }
```

`metric` accepts `flat | s2 | cp1 | pinch`. The 2D demo wire is enough
for unit-test scaffolding and toy-manifold cross-checks today.
Substrate-backed metrics in 384-dim land with the §3 dim-lift —
that's the same dependency your IMAGINE Phase 2 walk inherits.

Server-side floor on `max_solve_ms` is 50 ms (GIGI-team review's
anti-gaming clause). Setting `max_solve_ms: 1` doesn't get you cheap
Indeterminate verdicts.

## Suggestion: how to use it before the dim-lift lands

Two concrete things you can do today, neither blocked on full-dim
substrate:

**1. Cross-check your IMAGINE walk against the WISH oracle on a toy
manifold.** Your walk math is currently validated against the geodesic
integrator (T11). The BVP gives a second, independent oracle: from the
same `seed` and the same `target`, the walk's converged endpoint
should approximately match `wish.path[N]`, and `walk.arc_length`
should approximately match `wish.arc_length`. If they don't match on
S² or CP¹, you found a real disagreement between IVP-forward and
BVP-backward — that's a signal worth chasing.

Suggested test bundle: 2D S² stereographic, seeds and targets chosen
to stay inside the chart. Run both verbs at the same pair, compare,
record. Six pairs is enough for a useful gate.

**2. Adopt the verdict trichotomy at your refuse-gate boundary.** Even
without calling `/v1/wish`, the discipline pays off. Your walk
currently refuses on curvature ceiling and holonomy budget. The
trichotomy adds a third honest answer that lets you tell consumers
"I don't know" without overclaiming. Map them like this in your
walk's return type:

```
walk converged + budgets pass         -> Granted (your current happy path)
walk converged + budget bust at step k -> Unreachable (return path[..k] as frontier waypoint)
walk diverged / timed out             -> Indeterminate (don't return a partial path)
```

This is just a return-shape change in your walk; you don't need
substrate WISH for it. The downstream consumer (your refuse-gate)
then gets the same three-way honesty without depending on the BVP
solver being substrate-deployed yet.

## What's not shipped (yet)

- **GQL surface.** `WISH FROM rec_12 TO rec_88 IN b` and
  `SHOW WISH_CAPACITY` are a parser change; deferred. Use the HTTP
  endpoint directly until that lands.
- **Substrate-backed metrics.** Production 384-dim metrics ride the
  §3 dim-lift dependency (`RiemannianMetric` trait + `metric_at` on
  BundleStore). Your IMAGINE Phase 2 walk needs the same dim-lift, so
  unblocking one unblocks the other.
- **Chain orchestrator.** Multi-hop wishes through frontier waypoints
  (per spec §6.2) detect terminal stalls at the substrate level today
  (the W4 chain test pins reached_fraction=0); the orchestrator that
  actually returns `Indeterminate` for stalled chains is a Phase 5+
  wiring concern.
- **Marcella record-ID resolution.** The wire accepts `seed`/`target`
  as coords only in v0.1. Once you want `{ "record_id": "..." }`
  resolution against a real bundle, that's a one-line wire change
  + a `point_query` call — happy to add when you need it.

## Receipts

- B sweep (50 puzzles → 41 found in 4096-seed search):
  signal rate mean 0.748 / median 0.750, lift mean +27.6pp / median
  +28.3pp / std 12.26 / range +7.5pp to +45.1pp, beat random on 41/41.
- W-math 5/5 Python (W1–W5), 13/13 Rust port covering the same gates
  plus capacity monotonicity (W3) and the pinch fixture (W4).
- 9/9 wire tests pin the three-verdict JSON shape.
- 1372/1372 lib with wish, 1359/1359 without (strict-additive).

The verb works. Let me know what you want to do with it.

— GIGI team
