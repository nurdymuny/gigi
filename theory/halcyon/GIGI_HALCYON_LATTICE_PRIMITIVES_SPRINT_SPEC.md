# GIGI Halcyon-Lattice Primitives Sprint Spec

**Status:** Draft, ready for review
**Author:** Bee Rosa Davis, with Claude (Anthropic)
**Date:** June 2026
**Depends on:** `query/exec.rs`, `bundle/transport.rs`, `bundle/holonomy.rs`, `parser/gql.rs`
**Targets:** Halcyon validation pipeline migration (Davis Wilson Lattice), eventual unification of the SU(2) Yang–Mills lattice stack with the GIGI substrate Marcella already queries
**Goal:** add the minimum set of GIGI primitives + GQL verbs that lets Halcyon's lattice gauge theory state, dynamics, and analytical targets live inside GIGI, with the math expressed as queries on a `GAUGE_FIELD` rather than as a standalone Python harness.

---

## 0. Letter

Gigi —

Halcyon Stage 2 just shipped (davisgeometric.com/halcyon, schema 1.2, 8/10 PASS + 1 honest FAIL on the 93-DOF ergodicity caveat). The math is all in Python today: SU(2) Yang–Mills on the truncated icosahedron (V=60, E=90, F=32), symplectic leapfrog with covariant Gauss projection, Migdal–Witten analytical cross-check, β-envelope sweep, sector classifier with the π₂(SU(2))=0 caveat front-loaded.

It works. It also means the Davis Framework now has *two* sources of truth for the same geometric objects — your engine and Halcyon's Python — and that gap will widen as the framework grows. The roadmap is to put as much of the data directly into GIGI as we can and make the math a query on the substrate. The same pattern that closed the dryness gap for Marcella with `SAMPLE_TRANSPORT` (your sprint spec from May).

So here is what Halcyon would need to express its physics as GQL on a `GAUGE_FIELD`, prioritized P0–P4. The honest cut: **P0 + P1** is enough to host the state and the dynamics. **P2–P4** is the analysis pipeline; ship them later, or never, if you decide they belong outside the engine.

One deliberate non-ask: **the falsifiability validator (Halcyon's report-generator) stays external**. Its PASS verdict means "two independent codepaths agree." If both codepaths become GIGI's `HOLONOMY` verb, a bug in `HOLONOMY` looks like a PASS — the whole point of the cross-check evaporates. Everything else can and should move onto the substrate.

Pushback welcome on every verb name, grammar, response shape, and priority. The point of the letter is to start the conversation, not to dictate the sprint.

—Bee + Claude

---

## 1. Motivation

The Davis Wilson Lattice repository (`inertia_damping/`) currently holds:

- `buckyball_graph.py` — truncated-icosahedron incidence (60v / 90e / 32f, S² topology, Euler χ = 2 confirmed)
- `buckyball_action.py` — link-variable storage + Wilson action S(U) = β Σ_f Re tr(U_f)
- `buckyball_heatbath.py` — Gibbs sampling at inverse-temperature β
- `buckyball_integrator.py` — symplectic leapfrog on (U, E) with covariant Gauss projection (Tikhonov-shifted CG)
- `buckyball_observables.py` — plaquette mean, Q_surrogate, gauge-invariant feature vectors
- `buckyball_yangmills_exact.py` — closed-form Migdal–Witten target ⟨P⟩_exact via Bessel ratios

These are *graph-theoretic objects with a gauge group attached*. That is GIGI's home turf. The buckyball is a fiber bundle (B = truncated icosahedron, F = SU(2), structure group = SU(2)), and the Wilson action is a curvature functional on the connection. Halcyon's verdicts are GQL queries waiting for verbs that don't exist yet.

The Marcella architecture memo settled the analogous question for transport: GIGI was deterministic; we asked for a stochastic verb; Marcella's geometric channel got the entropy it needed. The Halcyon ask is the same shape — add primitives where the engine doesn't yet have them, keep using GIGI for everything it already does.

## 2. What's already in GIGI (no new work)

Confirming the inventory so the new asks don't duplicate:

- **`HOLONOMY`** — parallel transport around a closed loop on a fiber. Used in the Davis Framework for the curvature budget `‖Hol − I‖ < τ`. **For Halcyon, this *is* the plaquette holonomy when the loop is a face boundary.** If `HOLONOMY` accepts an arbitrary edge-list loop, the Wilson action reduces to a `SUM HOLONOMY OVER FACES` query.
- **`TRANSPORT`** — geodesic on a fiber bundle. Underlies Marcella's geometric channel.
- **`SAMPLE_TRANSPORT`** — stochastic transport with a curvature-bounded creativity budget. Already specced (May 2026).
- **`SPECTRAL`** — eigenstructure of a bundle Laplacian / heat kernel. Useful for Migdal–Witten if cast as a kernel ratio (see P2).
- **`BETTI`** — homology dimensions. Not used by Halcyon directly but confirms π₂(SU(2)) = 0 on S² is a query the engine already understands.

If `HOLONOMY` already accepts an arbitrary edge-loop on a declared graph, P0.3 (Plaquette) below is a thin wrapper, not a new primitive.

## 3. Sprint asks (P0 → P4)

### P0 — Lattice state representation (must-have)

#### P0.1  `LATTICE` (verb)

Declare a graph topology with incidence data. Buckyball is a specific instance; the verb should accept arbitrary graphs so this isn't a Halcyon-specific hack.

```ebnf
lattice_stmt
  : "LATTICE" ident
    "VERTICES" integer
    "EDGES" "(" edge_list ")"           // pairs of vertex indices
    [ "FACES" "(" face_list ")" ]       // ordered vertex-tuples per face
    [ "TOPOLOGY" string ]               // optional hint: "S2", "T2", "R3", ...
    ";"
  ;
```

Example (truncated icosahedron, abbreviated):

```sql
LATTICE buckyball
  VERTICES 60
  EDGES ((0,1), (0,2), (0,3), ..., (58,59))
  FACES ((0,1,2,3,4), ..., (55,56,57,58,59))
  TOPOLOGY "S2";
```

For Halcyon specifically we'd also welcome a shorthand:

```sql
LATTICE buckyball FROM TRUNCATED_ICOSAHEDRON TOPOLOGY "S2";
```

since that's a canonical mathematical object.

#### P0.2  `GAUGE_FIELD` (verb + primitive)

Attach link variables to a `LATTICE`. The structure group is a first-class parameter so the same verb later carries SU(3) / U(1) / Z_N.

```ebnf
gauge_field_stmt
  : "GAUGE_FIELD" ident
    "ON" "LATTICE" ident
    "GROUP" group_id                    // SU(2) | SU(3) | U(1) | Z_N
    [ "INIT" init_spec ]                // "IDENTITY" | "HAAR_RANDOM" SEED int | "FROM" ident
    ";"
  ;
```

Example:

```sql
GAUGE_FIELD U_t
  ON LATTICE buckyball
  GROUP SU(2)
  INIT IDENTITY;
```

The engine holds U_t as `(n_edges, 4)` SU(2) link variables in quaternion / matrix representation. Bit-identity across re-creations at fixed seed is the contract.

#### P0.3  `PLAQUETTE` (primitive, OR sugar over `HOLONOMY`)

Sum of `HOLONOMY` over the face-loops of a `LATTICE`. Returns either the total Wilson action `S(U) = β Σ_f Re tr(U_f)` or per-face plaquette holonomies, depending on context.

```sql
SELECT PLAQUETTE OF U_t;                         -- per-face Re tr(U_f), shape (n_faces,)
SELECT SUM(PLAQUETTE OF U_t);                    -- scalar Wilson action / β
SELECT MEAN(PLAQUETTE OF U_t);                   -- ⟨P⟩, the published observable
```

If `HOLONOMY` already accepts an arbitrary face-loop, this can be a parser-level sugar (`PLAQUETTE OF U_t := HOLONOMY ON FACES OF LATTICE OF U_t`).

### P1 — Dynamics (necessary to run Halcyon on the substrate)

#### P1.1  `HEATBATH_SWEEP` (verb)

Gibbs sampling on a `GAUGE_FIELD` at inverse-temperature β. One call = one full sweep over edges. Returns the updated field. For chain construction the caller iterates externally OR there is a `N_SWEEPS` clause (preferred — one round-trip beats N).

```ebnf
heatbath_stmt
  : "HEATBATH_SWEEP" ident                      // target GAUGE_FIELD
    "BETA" number
    [ "N_SWEEPS" integer ]                      // default 1
    [ "MEASURE_EVERY" integer ]                 // record observable history
    [ "MEASURE" "(" observable_list ")" ]
    [ "SEED" integer ]
    ";"
  ;
```

Example:

```sql
HEATBATH_SWEEP U_t
  BETA 2.5
  N_SWEEPS 200
  MEASURE_EVERY 1
  MEASURE (MEAN(PLAQUETTE), Q_SURROGATE)
  SEED 20260617;
```

Response includes the field state (latest) and the measurement history. The deterministic-seed contract is the same one Halcyon already enforces in Python.

#### P1.2  `SYMPLECTIC_FLOW` (verb)

Leapfrog integration of (U, E) at fixed dt for n_steps, with covariant Gauss projection at each step. This is the hardest of the asks — it carries the lattice's dynamical content — but it's also the one that converts "Halcyon's state lives in GIGI" into "Halcyon's *physics* lives in GIGI."

```ebnf
symplectic_stmt
  : "SYMPLECTIC_FLOW" ident                     // target GAUGE_FIELD
    "FROM" "(" "U" "=" ident "," "E" "=" ident ")"
    "BETA" number
    "DT" number
    "N_STEPS" integer
    [ "PROJECT_GAUSS" boolean ]                 // default TRUE
    [ "MEASURE_EVERY" integer ]
    [ "MEASURE" "(" observable_list ")" ]
    [ "SEED" integer ]                          // seeds the CG projector's Tikhonov shift, not stochastic
    ";"
  ;
```

Example:

```sql
SYMPLECTIC_FLOW U_t
  FROM (U = U_init, E = E_init)
  BETA 2.5
  DT 0.02
  N_STEPS 1000
  PROJECT_GAUSS TRUE
  MEASURE_EVERY 20
  MEASURE (H_TOTAL, GAUSS_RESIDUAL_MAX);
```

Response shape:

```json
{
  "U_final": <gauge_field>,
  "E_final": <electric_field>,
  "history": {
    "steps": [...],
    "H_total": [...],          // energy
    "gauss_residual_max": [...]
  },
  "diagnostics": {
    "max_energy_drift_rel": 5.762e-05,
    "max_gauss_residual": 5.135e-15,
    "cg_iterations_per_step_p99": ...
  }
}
```

The reason this is the high-value verb: every Halcyon trajectory query reduces to one `SYMPLECTIC_FLOW`. The Stage 2 β-envelope sweep is 5 of them.

### P2 — Analytical targets (high value, lower urgency)

#### P2.1  `MIGDAL_WITTEN` (primitive)

Closed-form 2D YM partition-function ratio. For SU(2) on a closed surface with F faces:

```
⟨P⟩_exact(β, F, χ) = ∑_j (2j+1)² (sinh((j+½)β) / sinh(β/2))^F · (...character expansion)
```

Halcyon uses the F→∞ limit `⟨P⟩_∞ = I_2(β) / I_1(β)`, which `SPECTRAL` could plausibly already express as a Bessel-kernel ratio.

```sql
SELECT MIGDAL_WITTEN(beta, lattice = buckyball, group = SU(2));
-- returns ⟨P⟩_exact at the finite-F lattice. Optional limit = "F_TO_INFINITY".
```

This is honest math, no sampling, no seeds. If `SPECTRAL` can already return Bessel-kernel ratios, this is a thin sugar.

### P3 — Statistical instrumentation (deferable)

#### P3.1  `BLOCKED_SEM` (primitive)

Flyvbjerg–Petersen autocorrelation-corrected SEM on a sample chain. Returns the blocking-plateau value and an honest `available: true/false` flag (FALSE if the chain is too short to see a plateau).

```sql
SELECT BLOCKED_SEM(P_history);
-- {"sem": 0.001463, "plateau_block": 6, "n_eff": 2048, "regime": "plateau_detected"}
```

#### P3.2  `HAAR_RANDOM_GAUGE_TRANSFORM` (verb)

Applies a vertex-indexed Haar-random g_v to a `GAUGE_FIELD` and returns the transformed field. Gauge invariants survive; declared-variant observables (edge_phase mean, etc.) change. Halcyon's Section 4 gauge-invariance gate uses this.

```sql
HAAR_RANDOM_GAUGE_TRANSFORM U_t SEED 42;
-- returns U_t' = g_v · U_e · g_w^†.
```

### P4 — Calibration & classification (defer indefinitely if you want)

These are the Section 9 sector classifier ingredients. They might not belong in GIGI at all — k-NN and permutation-null tests aren't really geometric primitives. Listed for completeness, not advocacy.

- `ENSEMBLE_FROM_TRAJECTORY` — windowed binning of a chain by an observable threshold
- `KNN_LOO` — k-NN leave-one-out classifier (with Mahalanobis distance)
- `LABEL_PERMUTATION_NULL` — N-shuffle null test
- `FEATURE_ABLATION` — leave-one-out and single-feature LOO

These almost certainly belong in a Python / Rust analysis layer that *reads from* GIGI, not in GIGI itself.

## 4. What we get back

If P0 + P1 ship, the entire Halcyon production trajectory expressible as:

```sql
-- One Halcyon trajectory becomes one GQL block.

LATTICE buckyball FROM TRUNCATED_ICOSAHEDRON TOPOLOGY "S2";

GAUGE_FIELD U
  ON LATTICE buckyball
  GROUP SU(2)
  INIT IDENTITY;

HEATBATH_SWEEP U
  BETA 2.5
  N_SWEEPS 200
  SEED 20260616;

GAUGE_FIELD E
  ON LATTICE buckyball
  GROUP SU(2)
  INIT HAAR_RANDOM SEED 20260617;
-- (E lives in the Lie algebra; the verb's group parameter would extend to su(2))

SYMPLECTIC_FLOW U
  FROM (U = U, E = E)
  BETA 2.5
  DT 0.02
  N_STEPS 1000
  PROJECT_GAUSS TRUE
  MEASURE_EVERY 20
  MEASURE (H_TOTAL, MEAN(PLAQUETTE), Q_SURROGATE, GAUSS_RESIDUAL_MAX);
```

The Davis Wilson Lattice repo's `inertia_damping/run_validation_report.py` is currently ~550 lines of Python orchestrating the same five operations. If those lines become GQL, the orchestrator's job collapses to "open a GIGI connection, run the queries, hand the response to the external validator." The validator stays Python (per the letter's non-ask above).

## 5. Bit-identity and reproducibility contract

Same contract Halcyon already enforces:

- Fixed-seed re-runs in the SAME process MUST be bit-identical.
- Cross-process re-runs at the same seed on the same OS MUST be bit-identical.
- Cross-OS bit-identity is a stretch goal; Linux ↔ macOS x86_64 should match, Windows MSVC may diverge in the last 2 ULPs of trigonometric reductions. Halcyon documents that.
- Random number generation MUST go through one CSPRNG path (the same one `SAMPLE_TRANSPORT` uses) so seed reuse across verbs composes cleanly.

## 6. Out-of-scope, deliberately

- **Multi-GPU**: Halcyon runs CPU-only today. GPU is a separate question.
- **Other gauge groups beyond SU(2)**: the verbs are *parameterized* by group, but only SU(2) needs to work at launch.
- **Non-Wilson actions**: improved actions, fermions, etc., are downstream questions.
- **The external validator**: Halcyon's report generator stays Python, per Section 0.

## 7. Open questions for you

1. **Does `HOLONOMY` already accept arbitrary edge-list loops on a declared graph?** If yes, P0.3 (`PLAQUETTE`) is parser sugar. If no, we'd want it generalized as a precondition to P0.3.
2. **Group-parameterized `GAUGE_FIELD`**: is the structure-group taxonomy a thing the engine already has (for `SAMPLE_TRANSPORT` on SU(2) fibers, presumably yes), or does each group need its own implementation?
3. **`SYMPLECTIC_FLOW`'s CG projector**: the Tikhonov shift is a hyperparameter Halcyon ships at 1e-12. Is that something the verb should expose as `TIKHONOV` clause, or hard-code? Halcyon will need control if the spectrum of the constraint operator changes with β; for now any default is fine.
4. **Sprint sizing**: P0 alone (3 verbs, mostly composing existing primitives) is probably a one-week sprint. P0 + P1 (5 verbs, including `SYMPLECTIC_FLOW` which is genuinely new) is more like 2–3 weeks. Which fits your current backlog, post the May sample-transport ship?
5. **Naming**: `HEATBATH_SWEEP` vs `GIBBS_SAMPLE`. `SYMPLECTIC_FLOW` vs `HMC_TRAJECTORY` vs `LEAPFROG`. All taste, no math. Yours.

## 8. Targets (what this unblocks)

- **Halcyon validation pipeline** consolidates onto GIGI; Python becomes the falsifiability harness only.
- **Marcella** gains access to Yang-Mills lattice samples as a queryable substrate — useful for any subsequent geometric-language-modeling work that wants gauge-field examples in its corpus.
- **The Davis Framework** gets one engine instead of two for the same mathematical objects. The "C = τ/K master equation" already lives on GIGI's `HOLONOMY` / `TRANSPORT` surface; Halcyon's lattice work joining that surface closes the architectural gap.

---

Pushback is invited on every point above. If P1.2 (`SYMPLECTIC_FLOW`) turns out to be a multi-month engineering investment, the right answer might be to ship P0 + P1.1 first and keep the Python integrator for now. If P2 (`MIGDAL_WITTEN`) is already expressible as a one-line `SPECTRAL` query, even better — fewer new verbs is always the right answer.

The order we'd suggest: P0 → P1.1 → measure how much of Halcyon collapsed → decide whether to add P1.2.

—Bee + Claude
