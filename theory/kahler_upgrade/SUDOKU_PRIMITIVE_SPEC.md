# SUDOKU Primitive — Specification v0.3

**Date:** 2026-05-27 (v0.3); 2026-05-26 (v0.2); 2026-05-26 (v0.1).
**Author:** Bee Davis + GIGI substrate, with feedback from Marcella team.
**Status:** Frozen for S2 implementation. All open decisions resolved.
H2 investigation closed (v0.3 update — see Appendix B); §6
preconditions empirically validated by today's seed-variation probe.

### Changelog v0.2 → v0.3

- **§6 preconditions empirically validated.** The multi-seed basin
  diversity check is now confirmed correct by the bge_v2 sweep: at
  T≥1 the bundle correctly fails multi-seed basin diversity (3
  prompts × 4 seeds produce 3 distinct doc-winners by seed),
  exactly the failure mode the precondition is designed to catch.
- **Appendix B rewritten** to reflect the H2 verdict: H1 falsified
  (mean-central claim refuted), H2 untested but irrelevant for
  bge_v2 (symptoms came from noise-dominated dynamics at T≥1, not
  fit pathology — H3-Gigi seed artifact confirmed).
- **§4.5 added** — per-primitive latency contract table. Informed
  by wave 1 load test + sweep measurements.
- **Floor-tunable spec** (`SPEC_FLOOR_TUNABLE_S1_WAVE2.md`)
  retained as defensive documentation for future bundles where
  H2 IS the actual mechanism; not built for bge_v2.

---

## 0. The shape, in one paragraph

SUDOKU is the brain primitive for **constrained inference on a learned
affordance manifold**. The consumer hands it a problem (a set of
constraints + context). The primitive enumerates the option-space surface
that the bundle has learned from data, finds the squares (options) that
satisfy all constraints, ranks them by the learned-norm prior, and
returns them with an **honest coverage estimate**. The honest-coverage
contract is the load-bearing API choice: `unsat: true | false | null`
where `null` means "I checked too little to claim either." Most solvers
in the wild fail this test. SUDOKU forces the consumer to distinguish
"feasible region is empty" from "I gave up early."

SUDOKU is the meta-primitive that orchestrates SAMPLE / ATTEND /
EPISODIC / SEMANTIC / INPAINT / EXPLAIN. Its value over composition is
**coverage estimation + near-miss ranking + the unsat:null contract** —
the three things a consumer would otherwise reinvent badly.

---

## 1. The said-vs-done framing for the prior π

Per Marcella's §1 feedback. **π is the *stated-norm prior*, not the
behavioral prior.** It reflects what appears in the bundle's records,
which (when records come from conversation) is what people *say* — not
what they *do*. The two are correlated but not identical, and the
selection bias goes both ways:

- **Defaults under-represented.** A commuter who always drives never
  mentions transport; "driving" gets low utterance mass even though it
  has high behavioral mass.
- **Exotics over-represented.** Rental scooter is worth mentioning
  *because* it's novel; it gets utterance mass disproportionate to
  actual use.
- **The "drone delivery" trap.** Tech-context bundles have high
  utterance mass for drone delivery and ~zero actual-use mass. π would
  propose it. SUDOKU consumers must treat solutions with rare-but-real
  prior mass as candidates for human review, not autopilot.

**Naming this honestly in the API.** The response field is
`stated_prior_mass`, not `prior_mass`. The advisory text for any
solution with `stated_prior_mass < 0.01` includes:
`"Low utterance frequency — verify this option is operationally
available before acting."`

### Conditional priors

π conditions on context fields the bundle's schema declares as
*conditioning fields*. Schema declares them via the existing
`base_fields` vs `fiber_fields` distinction:

- **base_fields** = conditioning context (user, time, weather)
- **fiber_fields** = the option space (mode, pace, eta)
- π(option | context) = restriction of the bundle's empirical
  distribution to records matching `context`

When no records match the context exactly, fall back to k-nearest in the
base manifold (Hadamard-local if available, Euclidean otherwise) and
report `prior_fallback: "knn_k=N, mean_dist=D"` in the response.

### Whose norms? — **Decision: Global π for v1**

Resolved 2026-05-26 per Marcella's pushback. Per-user and hierarchical
priors are deferred to v2 when the first real per-user use case lands.
The reasoning:

| Loop | Wants per-user prior? |
|---|---|
| Refuse-gate (voice manifold) | No — Marcella's voice, not user's |
| Intent classification | No — intents are corpus-universal |
| DREAM-extension pickers | No — Marcella's response patterns are global |
| Routing (R7 / R11 / R16) | No — global conversational moves |

Every actual v1 consumer wants global. Hierarchical Bayes adds
prior-of-prior estimation, shrinkage hyperparameters, per-user count
budgeting. None of which we need yet. **The API surface stays the
same** when hierarchical lands later: bundles fit per-user-with-shrinkage
return the shrunk prior; bundles fit globally return the global one.
Consumers don't need to know which.

The transport-norms demo (school commute) IS a per-user case, but it's
a future demo, not a v1 ship target. When we build it, we add
per-user fit on its bundle. The primitive doesn't change.

---

## 2. Constraint vocabulary

Per Marcella's §4 + §5. v1 must support manifold-distance constraints
because that's exactly Marcella's voice-anchor use case. Cross-field
relations and soft constraints get explicit semantics.

### Constraint types

```json
// Type 1 — field predicate (scalar)
{ "type": "field", "field": "eta", "op": "lte", "value": "08:30" }
{ "type": "field", "field": "mode", "op": "is_in", "value": ["walk", "bike"] }
{ "type": "field", "field": "price", "op": "between", "value": [0, 50] }

// Type 2 — manifold distance (this is the Marcella-voice case)
{ "type": "manifold", "field": "embedding",
  "near_manifold": "marcella_voice_anchors", "epsilon": 0.30 }

// Type 3 — cross-field relation
{ "type": "relation", "expr": "eta < drop_off + buffer_min",
  "vars": { "drop_off": "08:30", "buffer_min": 5 } }

// Type 4 — composite (logical AND / OR / NOT over above)
{ "type": "and", "clauses": [<constraint>, <constraint>, ...] }
{ "type": "or",  "clauses": [<constraint>, <constraint>, ...] }
{ "type": "not", "clause": <constraint> }
```

### Soft constraints

`"hard": false` adds a *penalty function* to the sampler's energy:

- For field predicates: penalty = `max(0, violation_magnitude / scale)²`
  where `scale` is the field's MAD (mean absolute deviation).
- For manifold-distance: penalty = `max(0, (dist - epsilon) / epsilon)²`.
- For relations: penalty = `max(0, residual / scale)²` where `residual`
  is the LHS-RHS algebraic difference and `scale` is the average value
  of the larger side.

Penalty weight = `lambda` (default 1.0, configurable). The
log-probability the Langevin sampler sees becomes
`log π(x) - λ · Σ penalty_i(x)`. Soft constraints with very high
λ approximate hard constraints; the math is continuous between them.

### Cross-field relations

The `expr` field is a small expression language (arithmetic + comparison
+ logical). Parser borrows from the GQL `WHERE` clause already in
GIGI_LANG_SPEC.md. Allowed: `+ - * /`, `< <= > >= == !=`, `&& || !`,
field references, named vars, numeric and ISO-8601 datetime literals.

---

## 3. The honest-coverage contract

Per Marcella's §2 + §3. **One coverage metric, primary: stated-prior-mass
coverage.** Volume coverage is demoted to an internal diagnostic — not
in the response.

### Definition

```
prior_mass_explored =  Σ_{x ∈ visited} π(x | context)
                     / Σ_{x ∈ feasible_region} π(x | context)
```

Numerator and denominator both estimated from the sampler's trajectory
(numerator is exact; denominator uses the same trajectory with an
importance correction — standard MCMC estimator).

### UNSAT tristate policy

| Coverage | Solutions found | `unsat` |
|---|---|---|
| ≥ 0.80 | ≥ 1 | `false` |
| ≥ 0.80 | 0 | `true` |
| 0.50 ≤ coverage < 0.80 | ≥ 1 | `false` (with `coverage_advisory: "moderate"`) |
| 0.50 ≤ coverage < 0.80 | 0 | `null` |
| < 0.50 | (any) | `null` |

Rationale for the middle bucket: at ≥50% mass coverage *with* solutions
found, returning what we found is more useful than withholding;
explicit `null` is reserved for cases where the absence of a solution
might be a sampler failure rather than a real UNSAT.

**Decision: Fixed thresholds (0.80 / 0.50) in v1.** Resolved 2026-05-26
per Marcella's reasoning:

1. Contract semantics depend on consistent thresholds. "Coverage > 0.80
   means `unsat: true`" is a meaningful guarantee iff 0.80 means the
   same thing for every caller. Per-request tuning erodes the contract —
   `unsat: true` from caller A and caller B would mean different things.
2. No telemetry to tune against yet. Picking thresholds before we know
   what real-world coverage distributions look like is guessing with
   extra knobs.

**v1.1 upgrade path:** named regimes rather than raw float knobs:
`coverage_policy: "default" | "permissive" | "strict"`. Each maps to a
fixed (high, low) pair. Keeps the contract intact while letting
consumers pick a regime appropriate to their use case (e.g.
`strict` for irreversible decisions, `permissive` for exploration).

---

## 4. Latency profile + fast-path mode

Per Marcella's §6. Three modes selectable per request:

| Mode | `explore_budget_ms` | Behavior |
|---|---|---|
| **fast** | ≤ 50 | Single SAMPLE pass, ≤ 16 candidates, no near-miss enumeration, often returns `coverage: 0.1–0.3`. For tight loops (refuse-gate, per-turn intent). |
| **default** | 200–2000 | Multi-pass SAMPLE + ATTEND, near-miss enumeration, coverage estimate to ±0.05. For planning queries. |
| **thorough** | ≥ 5000 | Multi-restart SAMPLE from k=10 random seeds (catches multi-modal posteriors), full near-miss ranking. For irreversible decisions. |

The mode doesn't change the API surface; only the time budget changes.
The honest-coverage contract still works at 30ms — the response just
frequently has `coverage: 0.2, unsat: null`, which is fine because the
caller knew the budget when they set it.

---

## 4.5. Per-primitive latency contract (v0.3)

Per Bee's 2026-05-27 latency-as-product-property reframe: every
primitive carries an explicit SLO. Targets below are calibrated
against wave 1 load test measurements (bge_v2, n_fields ≤ 50) +
the bge_v2 sweep (n_fields = 384). All targets assume cache-warm
unless noted. Cache miss adds the cold-fit cost shown.

### Cold-fit cost (paid once per `(bundle, fit_mode, fields, ε, abs_floor)`)

| fit_mode | n_fields | Walk cost | Inversion cost | Total cold | Cached warm |
|---|---:|---:|---:|---:|---:|
| isotropic | any | µs (Welford lookup) | — | < 1 ms | < 1 µs |
| diagonal | any | µs (Welford lookup) | — | < 1 ms | < 1 µs |
| full | 10 | ~5 ms | ~1 ms | ~10 ms | < 1 µs |
| full | 50 | ~50 ms | ~10 ms | ~70 ms | < 1 µs |
| full | 384 | ~3 s | ~200 ms | ~3.3 s | < 1 µs |

Cold-fit happens at most once per cache key. Wave 1 single-flight
(planned wave 2) makes concurrent cold misses share the work.

### Per-primitive endpoint targets (cache-warm, network-included)

| Primitive | Endpoint | p50 target | p95 target | Notes |
|---|---|---:|---:|---|
| SAMPLE | `/brain/sample` | 100 ms | 200 ms | 1 Langevin step + n_samples draws |
| DREAM (1k steps, n=50) | `/brain/dream` | 500 ms | 700 ms | dominated by 1k Langevin steps |
| FORECAST | `/brain/forecast` | 100 ms | 250 ms | Hamilton flow, n_steps × matvec |
| RECONSTRUCT | `/brain/reconstruct` | 200 ms | 400 ms | T=0 descent to MAP |
| INPAINT | `/brain/inpaint` | 150 ms | 300 ms | Constrained Langevin |
| PREDICT | `/brain/predict` | 50 ms | 100 ms | Single natural-gradient step |
| ATTEND | `/brain/attend` | 100 ms | 200 ms | Softmax over n records |
| EPISODIC | `/brain/episodic` | 200 ms | 400 ms | Change-point detection over N |
| SEMANTIC | `/brain/semantic` | 150 ms | 300 ms | Morse compression |
| EXPLAIN | `/brain/explain` | 50 ms | 100 ms | Nearest-neighbor + interpolation |
| CONFIDENCE | `/brain/confidence` | 50 ms | 100 ms | Kernel density |
| CONFIDENCE+EXPLAIN | `/brain/confidence_with_explain` | 75 ms | 150 ms | Combined, one record walk |
| FIT_DIAGNOSTICS | `/brain/fit_diagnostics` | < 5 ms | 20 ms | Reads cached fit directly |
| DISTANCE_TO_FIT_MEAN | `/brain/distance_to_fit_mean` | 50 ms | 150 ms | Per-call record walk for distribution |

### SUDOKU composition latency targets (the meta-primitive)

| SUDOKU mode | Total target (cache-warm) | Cold-path penalty |
|---|---:|---|
| **fast** | ≤ 50 ms | + cold-fit (one-time) |
| **default** | 200–2000 ms | + cold-fit |
| **thorough** | ≥ 5000 ms | + cold-fit per restart-seed (5×) |

Decomposed for the **default** mode (200–2000 ms budget):
1. Precondition check (multi-seed basin diversity at low T): ~50 ms × 5 seeds = ~250 ms
2. Constraint-filter pass over records: ~50 ms (O(N) per record)
3. Candidate enumeration (ATTEND-biased SAMPLE): ~300–1500 ms
4. Coverage estimator: ~10 ms (importance sum over visited)
5. Near-miss enumeration: ~50–100 ms
6. Response encoding (DHOOM or JSON): ~5 ms

### Cache observability targets

| Metric | Target | Action if breached |
|---|---|---|
| `gigi_brain_cache_hit_rate` | ≥ 0.85 steady-state | Investigate working-set vs cap; bump max_entries or switch to LRU |
| `gigi_brain_cache_evictions_total` rate | ≤ 1/min steady-state | Same as above |
| `gigi_brain_fit_total_us` slope | Decreasing then flat after warm-up | If still rising, cache key has a bug (always missing) |

---

## 5. Near-miss output

Per Marcella's §7. Structured, not prose. Each near-miss carries which
constraint(s) it violates and by how much:

```json
{
  "option": { "mode": "walking", "pace": "normal", "eta": "08:45" },
  "violates": [
    { "constraint_idx": 1, "field": "eta",
      "violation": "08:45 > 08:30", "relax_to": "08:45",
      "relax_delta": "15min", "relax_cost": 0.15 }
  ],
  "stated_prior_mass": 0.41,
  "would_unlock_if_relaxed": [1]
}
```

- `constraint_idx` indexes into the request's `constraints` array
- `relax_to` is the smallest relaxation that would make this option
  feasible
- `relax_cost` is a normalized [0, 1] measure of the relaxation
  magnitude (using the soft-constraint penalty function)
- `would_unlock_if_relaxed` lists constraint indices that, if relaxed
  to `relax_to`, would make this option feasible

Consumers render: *"...if you can be 15 min late, walking-normal works
(this is the high-prior default mode you'd otherwise choose)."*

---

## 6. Preconditions — fit-diagnostic gate

Per Marcella's §8. SUDOKU's prior π depends on the bundle's fit
machinery. If the fit is pathological (the `double_cover_v3` H2-style
attractor Marcella has been investigating), then `stated_prior_mass`
becomes a lie — the sampler reports 0.91 while actually only sampling
one basin.

### Preconditions checked before accepting a SUDOKU request

| Check | Threshold | Failure mode |
|---|---|---|
| Variance ratio | `max(σ²_per_axis) / median(σ²_per_axis) < 100` | Detects axes that have collapsed to a delta. |
| Fit-mean distinctness | No record sits within `0.01 · √σ²` of `fit_mean` | Detects the "every Langevin walk lands at the fit mean" pathology. |
| Multi-seed basin coverage | Langevin from 5 random seeds reaches ≥ 3 distinct basins (k-means cluster count) | Detects multi-modal posteriors that single-seed sampling misses. |

If any check fails, SUDOKU returns:

```json
{
  "precondition_failed": "bundle prior is pathological",
  "details": {
    "check": "fit_mean_distinctness",
    "diagnostic": "record_id=42 lies at distance 0.003 from fit_mean (threshold: 0.01·√σ²=0.018)",
    "remediation": "Refit with fit_mode='full' or use different fields."
  }
}
```

**Decision: Strict default + narrow opt-in via `acknowledge_pathology`.**
Resolved 2026-05-26 per Marcella's reasoning. Pure strict has a UX
failure mode: when SUDOKU rejects a bundle as pathological, the
consumer can't fix the prior themselves (substrate-side issue). Narrow
opt-in preserves the honest-coverage contract while giving an escape
hatch for legitimate cases (probing a new bundle, calibrating
thresholds, debugging).

- **Default behavior: strict.** Production callers refuse pathological
  bundles. Honest-coverage contract preserved.
- **Override: `acknowledge_pathology: true`** in the request body.
  Server logs every override with caller identity. Production deploys
  can disable the override at the policy layer.
- **Error message MUST be actionable.** Not just "pathological" — but
  e.g. `"variance_ratio = 4.2e6, exceeds 1e4 threshold; refit with
  fit_mode=full or use different fields"`. The consumer needs to know
  the remediation.

Why narrow opt-in matters: if the **H2 attractor investigation
(confirmed real on bge_v2)** comes back as "diagonal-fit is broadly
pathological on semantic bundles," every probe + every dev loop hits
strict and we can't even run diagnostics through SUDOKU. The escape
valve preserves the strict-default contract while not making the
primitive unusable during the very investigation that proves whether
strict is the right default.

**Status update:** Marcella's 2026-05-26 letter
(`marcella/theory/kahler_upgrade/LETTER_TO_GIGI_DREAM_ATTRACTOR_2026-05-26.md`)
confirms `double_cover_v3` is a universal attractor on bge_v2 at T≥0.3
for every probe prompt. H2 is the leading hypothesis. S1 (fit
diagnostic + remediation) is now **upstream-critical** — SUDOKU is
non-functional on semantic bundles until S1 lands.

---

## 6.5. Puzzle expansion — making the 9×9 a 10×10

**Per Bee's clarification: "if you can't solve the puzzle as a 9x9 you
are welcome to make it a 10x10 (expand)."** This is structural, not a
nice-to-have. The difference between an honest solver and a *creative*
solver is the willingness to ask "what other puzzle is this?"

A 9×9 puzzle is one specific manifold (the bundle's option space under
the request's constraints). A 10×10 is a related, larger manifold. The
primitive's permission is to operate on the larger one when the smaller
one has no solution.

### Three flavors of expansion

| Type | What changes | Cost dimension |
|---|---|---|
| **constraint_relaxation** | Drop or weaken a hard constraint; re-solve | `relax_cost` ∈ [0, 1] (already computed for near-misses) |
| **bundle_hop** | Query a related bundle for options absent from the current one | `bundle_hop_cost` (semantic distance between bundles, normalized) |
| **field_extension** | Add a base_field condition that opens up previously-excluded records | `prior_shift_cost` (KL divergence between original and expanded prior) |

**v1 ships constraint_relaxation + bundle_hop. field_extension is v1.1.**

Constraint_relaxation comes essentially for free — near-misses already
identify which constraint to relax and by how much. Bundle_hop is the
high-leverage one Bee's framing wants; it requires bundle-graph
metadata which exists (same-namespace bundles, plus SEMANTIC distance
between bundle Morse-compressed gists).

### The expansion contract

Expansion is **opt-in by default**. A consumer who didn't ask for it
shouldn't get surprise expansion — that would violate the
honest-coverage contract by silently changing the problem the user
asked. Default: original puzzle only.

```json
{
  "expansion": {
    "allowed": true,
    "max_constraint_relaxations": 1,
    "max_bundle_hops": 1,
    "related_bundles": null   // null = auto-discover via bundle graph; or explicit whitelist
  }
}
```

### When expansion triggers

Only after `unsat: true` (with `coverage ≥ 0.80` — we genuinely
exhausted the 9×9) OR `unsat: null` with coverage exhaustion ruled out
by mode='thorough'. Expansion uses **leftover budget** after the
original puzzle is solved or determined UNSAT. If the original puzzle
ate the full budget, the response says
`expansion_skipped: "insufficient remaining budget"`.

### Response shape for expanded solutions

The response distinguishes expanded solutions from original ones
clearly — they ARE answers to a different puzzle:

```json
{
  "solutions": [],          // original 9×9 puzzle
  "near_misses": [...],
  "unsat": true,            // 9×9 is UNSAT
  "coverage": 0.94,

  "expanded": {
    "attempted": true,
    "type": "bundle_hop",
    "from_bundle": "transport_norms",
    "to_bundles": ["school_norms", "calendar"],
    "hop_rationale": "auto-discovered via shared namespace 'school_commute' + SEMANTIC distance < 0.4",
    "solutions": [
      {
        "option": { "type": "remote_school_day", "via": "principal_approval" },
        "source_bundle": "school_norms",
        "stated_prior_mass": 0.18,
        "expansion_cost": 0.27,
        "rationale": "transport_norms feasible region empty under hard constraints; school_norms has 'remote_day' option matching context.user.role=student. Cost 0.27 = bundle hop + 1 context-field addition for principal_approval.",
        "confidence": 0.71
      }
    ],
    "coverage_in_expanded": 0.83
  }
}
```

### When expansion ALSO finds nothing

```json
{
  "unsat": true,
  "expanded": {
    "attempted": true,
    "type": "constraint_relaxation+bundle_hop",
    "solutions": [],
    "advisory": "Expanded puzzle is also UNSAT under coverage 0.81 across 2 related bundles. The problem as stated has no solution; consider reformulating context or asking a human."
  }
}
```

That last sentence — "consider asking a human" — is the right output
when geometry has honestly exhausted itself. Most solvers don't say
this; SUDOKU should.

### Composition with other brain primitives

- **Constraint_relaxation** uses ATTEND on near-misses to choose the
  cheapest relaxation; uses EPISODIC if the consumer enables
  `use_episodic` to bias toward relaxations that worked previously.
- **Bundle_hop** uses SEMANTIC to discover related bundles (Morse-gist
  similarity) and ATTEND to rank candidates from the hopped bundle.
- **Field_extension** (v1.1) uses INPAINT to predict reasonable values
  for the added context field.

### Riemann-Roch / Hadamard connection (for the paper, not v1)

Expansion has a deeper mathematical interpretation worth recording for
§4 of the Kähler-substrate paper:

- **Riemann-Roch capacity (L7).** UNSAT in the original puzzle = "the
  line bundle's section space is too small for these constraints."
  Bundle_hop = "lift to a higher-degree line bundle" (literally adding
  capacity via a divisor). This is structurally why expansion can
  succeed where the original failed.
- **Hadamard substructure (L5).** Within a Hadamard region, geodesics
  are unique → constraints have clean unique answers. Outside Hadamard,
  you either restrict (shrink the puzzle) or expand to a covering
  manifold. Bundle_hop is the covering-manifold case.

These aren't required to implement v1, but they're the right
mathematical justifications for the API choice. Worth noting in the
paper's §4 ("where the +7.6pp prediction comes from").

---

## 7. API surface

### Request

```http
POST /v1/bundles/{name}/brain/sudoku
Content-Type: application/json
X-API-Key: ...

{
  "context": { "user": "alex", "time_now": "08:00", "distance_km": 2.5 },
  "constraints": [
    { "type": "field", "field": "mode", "op": "is_in",
      "value": ["walking", "walking_plus"], "hard": true },
    { "type": "field", "field": "eta", "op": "lte",
      "value": "08:30", "hard": true }
  ],
  "max_options": 5,
  "max_near_misses": 3,
  "explore_budget_ms": 500,
  "mode": "default",
  "expansion": { "allowed": true, "max_constraint_relaxations": 1, "max_bundle_hops": 1 },
  "acknowledge_pathology": false,
  "seed": 42
}
```

`acknowledge_pathology: true` is the narrow opt-in that bypasses the
strict precondition gate. Use only when you genuinely need to probe a
bundle the substrate has flagged as pathological. Server logs every
override with caller identity.

When `expansion.allowed` is omitted or false (default), the response
contains no `expanded` field — the consumer gets a clean answer to the
9×9 they asked, including honest UNSAT if applicable. Setting
`allowed: true` opts in to 10×10 expansion if the 9×9 is UNSAT.

### Response (happy path)

```json
{
  "solutions": [
    {
      "option": { "mode": "walking_plus_scooter", "pace": "normal", "eta": "08:27" },
      "satisfies_all": true,
      "confidence": 0.78,
      "stated_prior_mass": 0.04,
      "stated_prior_advisory": "Low utterance frequency — verify this option is operationally available before acting.",
      "rationale": "Rental scooter stations along route present in 12 records; pace J-coupled to mode.",
      "trace": { "via": ["SAMPLE", "ATTEND"], "n_candidates_considered": 47 }
    }
  ],
  "near_misses": [
    {
      "option": { "mode": "walking", "pace": "normal", "eta": "08:45" },
      "violates": [
        { "constraint_idx": 1, "field": "eta", "violation": "08:45 > 08:30",
          "relax_to": "08:45", "relax_delta": "15min", "relax_cost": 0.15 }
      ],
      "stated_prior_mass": 0.41,
      "would_unlock_if_relaxed": [1]
    }
  ],
  "unsat": false,
  "coverage": 0.91,
  "coverage_method": "stated_prior_mass",
  "exploration_budget_used_ms": 487,
  "precondition_warnings": []
}
```

### Response (true UNSAT)

```json
{ "solutions": [], "near_misses": [...],
  "unsat": true, "coverage": 0.94,
  "advisory": "Feasible region is empty under given hard constraints. Relaxation options surfaced in near_misses." }
```

### Response (honest "don't know")

```json
{ "solutions": [], "near_misses": [],
  "unsat": null, "coverage": 0.31,
  "advisory": "Stopped at budget. Coverage too low to claim UNSAT. Retry with explore_budget_ms >= 2000 or mode='thorough'." }
```

### Response (precondition failed)

```json
{
  "precondition_failed": true,
  "check": "fit_mean_distinctness",
  "diagnostic": "variance_ratio = 4.2e6 exceeds threshold 1e4; record_id=2871 (double_cover_v3) lies 0.003 from fit_mean (threshold 0.01·√σ²=0.018)",
  "remediation": "Refit with fit_mode='full', use different fields, or pass acknowledge_pathology: true to probe anyway.",
  "override_available": true
}
```

---

## 8. How the brain primitives compose inside SUDOKU

```
SUDOKU(bundle, context, constraints, budget):
  1. preconditions = fit_diagnostic(bundle)
     if preconditions.failed and policy == "strict": return precondition_response
  2. π_context = condition_prior(bundle, context)
     # uses base_field matching + k-NN fallback
  3. feasible_region = constraint_filter(bundle.records, constraints)
     # hard constraints are exclusion; soft constraints become λ-weighted penalties
  4. candidates = []
     while budget_remaining and len(candidates) < max_options * 5:
        x = SAMPLE(π_context, energy = -log π + λ·Σ penalty_i)
        if all_hard_constraints_satisfied(x): candidates.append(x)
  5. coverage = prior_mass_visited(candidates) / prior_mass_total(feasible_region)
  6. solutions = top_k(candidates, key=stated_prior_mass, k=max_options)
  7. near_misses = []
     if budget_remaining:
        for c in candidates_violating_one_constraint:
           near_misses.append(annotate_violation(c, constraints))
  8. unsat = unsat_tristate(coverage, len(solutions))
  9. return SudokuResponse(...)
```

`SAMPLE` is the existing brain SAMPLE primitive operating on the
context-restricted prior. `ATTEND` is implicit in step 4's importance
sampling. `EPISODIC` would be invoked optionally (`use_episodic: true`
in the request) to bias the sampler toward known-good prior solutions
from the same context. `SEMANTIC` is the optional Morse-compression
of the prior used by `condition_prior` when the bundle is large.
`EXPLAIN` is invoked per-solution to produce `rationale`.

---

## 9. Sprint shape

| Layer | What | Cost | Depends on |
|---|---|---|---|
| S0 | Resolve [OPEN-1] (whose-norms), [OPEN-2] (thresholds), [OPEN-3] (precondition strictness) | 1 conversation | — |
| S1 | H2 fit-diagnostic — investigate bge_v2 attractor, write `fit_diagnostic` function, ship `/v1/bundles/.../fit_diagnostic` endpoint | 1-2 days | — (upstream) |
| S2 | `src/geometry/sudoku.rs` — `complete_record`, `solve_constraints`, coverage estimator, near-miss enumerator, with TDD | 2 days | S0, S1 |
| S3 | HTTP `/v1/bundles/{name}/brain/sudoku` endpoint + contract tests + manifold-distance constraint type | 1 day | S2 |
| S3.5 | **Puzzle expansion** — constraint_relaxation + bundle_hop (per §6.5). Uses near-misses (free) + SEMANTIC bundle-graph traversal. | 1.5 days | S3 |
| S4 | Soft-constraint penalty calibration on real bundles + cross-field relation parser | 1 day | S3 |
| S5 | `examples/sudoku_school_commute_demo.rs` + literal 9×9 puzzle as a bundle (`examples/sudoku_puzzle_demo.rs`) + 10×10 expansion demo | 1.5 days | S3.5 |
| S6 | Add SUDOKU to `BRAIN_PRIMITIVES_CONSUMER_GUIDE.md`, `brain_tour_demo`, `kahler_tour` summary | 0.5 day | S5 |
| S7 | Marcella migration — refuse-gate / intent / R7-R11-R16 routing rewritten as SUDOKU calls, side-by-side latency + accuracy | 1-2 days | S6 |

**Total: ~9-11 days of focused work** (was 7-9 before S3.5 expansion layer added).

### Sequencing relative to other work

Per Marcella's closing: **don't queue SUDOKU behind the 4 open ops
fixes if the H2 attractor diagnostic comes back positive.** S1 is
upstream of:
- This spec (which assumes a working fit)
- The DREAM-extension picker design Marcella has drafted

If H2 is real, S1 unblocks more than it blocks and should ship first.

---

## 10. Why this is the right primitive

(Restated from earlier discussion for the spec record.)

1. **Closes the "Sudoku 10×" metaphor.** The stack is named for this;
   shipping the literal version completes the naming.
2. **Inverse of EXPLAIN.** EXPLAIN says "here's the path." SUDOKU says
   "here's the squares."
3. **Real ETL use case.** Sparse-data imputation with calibrated
   "I don't know" — most consumers reinvent this badly.
4. **One math machinery, many consumer surfaces.** The J / B / ∇
   structure from L1–L7 + the brain primitives from L10–L13 + the
   honest-coverage contract from this spec = a single primitive that
   absorbs many one-off solvers.
5. **Marcella migration target.** Refuse-gate, intent classification,
   R7/R11/R16 routing, DREAM-extension pickers (`pick_drift_opener`,
   `pick_contrast`, `pick_closing`) — all become SUDOKU calls. Tight
   loops stop being Python decision tables and become substrate writes.
6. **The honest-coverage contract is novel and correct.** Most solvers
   shrug. SUDOKU refuses to shrug. That's the consumer-facing virtue
   that justifies the primitive.
7. **Expansion turns a solver into a creative solver.** Per Bee's
   "make it a 10×10 if 9×9 doesn't work" framing: SUDOKU is willing to
   ask "what other puzzle is this?" Bundle_hop in particular — querying
   *related* bundles when the current one is UNSAT — is the structural
   feature that lifts SUDOKU above any conventional constraint solver.
   Most solvers fail UNSAT and stop; SUDOKU fails UNSAT, then asks
   "is there a different puzzle whose solution would address your
   actual problem?" and offers it with cost annotation. This is also
   what makes it the right primitive for Marcella's tight loops — her
   real workflow constantly involves "this constraint doesn't fit, what
   else?" reformulations she currently hand-codes.

---

## Appendix A — Decisions resolved (changelog)

| ID | Decision | Resolution | Rationale |
|---|---|---|---|
| OPEN-1 | Whose norms | **Global π for v1** | Every v1 consumer wants global; hierarchical defers to v2 with no API surface change |
| OPEN-2 | Coverage thresholds | **Fixed (0.80 / 0.50) in v1; named regimes in v1.1** | Contract semantics depend on consistent thresholds; no telemetry to tune yet |
| OPEN-3 | Precondition strictness | **Strict default + narrow `acknowledge_pathology` opt-in + actionable error messages** | Strict preserves honest-coverage; narrow opt-in prevents unusable diagnostics; actionable errors prevent dead-end UX |
| EXPAND | 9×9 → 10×10 puzzle expansion (Bee, 2026-05-26) | **v1 ships constraint_relaxation + bundle_hop, opt-in** | Bee's "make it a 10×10 if 9×9 doesn't work" framing; structural not cosmetic; bundle_hop is the high-leverage one |

All four decisions are folded into the spec body above. No remaining
open decisions for v1.

---

## Appendix B — H2 attractor verdict (closed 2026-05-27)

### v0.2 prediction (preserved for reference)

Marcella's 2026-05-26 letter hypothesized that the `double_cover_v3`
universal attractor at T≥0.3 was the H2 mechanism (diagonal-fit
eigenvalue pathology). The spec v0.2 expected S1 to either confirm H2
(full-fit + eigenvalue floor diffuses the attractor) or H1 (the
substrate is genuinely concentrated at that cite).

### v0.3 resolution — neither H1 nor H2

Cheap diagnostic probes on 2026-05-27 falsified both:

| Probe | Result |
|---|---|
| **fit_mean_distance for `double_cover_v3`** (119 chunks) | Median percentile rank = 0.53; **NOT mean-central**. H1's mechanism claim falsified. |
| **Seed-variation sweep** (3 prompts × 4 seeds at T=2, fit_mode=diagonal) | seed=7 → `double_cover_v3` for all 3 prompts. seed=42, 1234, 9999 → 3 different doc-winners. **`double_cover_v3` wins only at seed=7.** |

**Verdict: H3-Gigi (seed=7 artifact) confirmed.** At T=2 the Langevin
noise dominates the gradient, so the walk's destination is determined
by the noise sequence rather than the prompt. Same seed → same
destination regardless of prompt; different seeds → different
"attractors." The substrate is fine. The fit machinery is fine.
The probe was fine — it just used a single seed and we read the
result as a substrate property.

Also discovered: the H2 vs H1 test was degenerate because BOTH fit
modes operated on the same `0.03·I` Σ. The absolute Euler-stability
floor (`3·dt = 0.03`) is 30× above bge_v2's natural per-axis variance
(~0.001), so the spectrum never surfaced. Floor-tunable spec
(`SPEC_FLOOR_TUNABLE_S1_WAVE2.md`) retained as defensive documentation
for future bundles where H2 IS the actual mechanism; not needed for
bge_v2 because the symptoms came from elsewhere.

### Impact on SUDOKU — preconditions empirically validated

The multi-seed basin diversity precondition in §6 does **exactly the
right work** on bge_v2 at T≥1: 3 prompts × 4 seeds → 1 basin per
seed (different basin per seed) → fails ≥3 distinct basins check.
SUDOKU correctly declines this bundle at high T. The `acknowledge_pathology`
opt-in is the correct UX for the "I know it's noisy here, give me
what you have" diagnostic case.

**At T≤0.3 the substrate IS multi-modal** — different prompts give
different drift winners. SUDOKU at low T on bge_v2 should work
cleanly. Document bge_v2 (and likely other normalized 384-D embedding
bundles) as "constrain to T≤0.3 for SUDOKU consumption" in the
consumer guide.

### S1 deliverables — all shipped in wave 1

1. ✅ `POST /v1/bundles/{name}/brain/fit_diagnostics` — full eigenvalue
   spectrum + variance + floor diagnostics. Commit `8be8f84`.
2. ✅ `FitMode::Full` variant + full-covariance Langevin. Commits
   `1376cb2` (geometry) + `78f45dd` (server wiring).
3. ✅ `POST /v1/bundles/{name}/brain/distance_to_fit_mean` — H1
   mean-centrality check. Commit `302f5d6`. **This is the probe that
   falsified H1.**
4. ✅ Eigenvalue floor on Σ. Commit `22a35b6`. Retained as defensive
   for future bundles where H2 applies; didn't apply for bge_v2 because
   the absolute Euler-stability floor dominated.
5. ✅ Seed-variation re-run (Marcella's Step A). Used existing
   `seed` request param — no code change needed.

### Reframe for paper §4 (Task #75)

The empirical story is now: **GIGI's geometry adds signal at the
user's natural exploration scale (T≈0.1) where the gradient dominates
noise; at high-T noise-dominated regimes the walk goes wherever the
noise sequence takes it, regardless of fit machinery.** That's a
cleaner story than either "full-fit fixes diagonal" (H2) or "geometry
finds the main attractor" (H1) would have been. The +7.6pp prediction
holds for T≈0.1 retrieval against cosine baseline; high-T is where
ALL approaches degrade similarly.

---

*End of spec v0.3.*
