# Halcyon → GIGI Substrate, Part I gates

**Status:** Sprint-locked
**Author:** Bee Rosa Davis, with Claude (Anthropic)
**Date:** June 2026
**Supersedes:** `GIGI_HALCYON_LATTICE_PRIMITIVES_SPRINT_SPEC.md` § 3.P0–P1 (incorporated with Q1–Q3 corrections from the 2026-06-17 engine-owner reply)
**Companion:** `HALCYON_TO_GIGI_REPLY_2026-06-17.md` (Q_surrogate spec + bit-identity contract)
**Goal:** lock the work breakdown for the first Halcyon-onto-GIGI sprint into named gates, with explicit receipts and pass criteria, so the implementation has a definition of done and the validator has a definition of regression.

---

## How to read this doc

Each **PART** is a coherent unit of work. Each part has:

- **Scope** — what is and isn't in.
- **Receipts** — concrete artifacts (code, tests, observables) that prove the gate is closed.
- **Pass criterion** — the explicit gate verdict.
- **Blocker on** — upstream parts that must close first.
- **Out of scope** — anti-receipts; things the gate explicitly does NOT promise.

The **measurement gate** at the end of Part III is the decision point on whether Part IV (`SYMPLECTIC_FLOW`) earns a sprint. It is *not* a default-yes.

---

## PART I — Generalized HOLONOMY on a declared LATTICE

Closes Q1 from the engine-owner reply: today's `HOLONOMY` walks a path from a record sequence and reads the connection off the bundle's geometric structure. Halcyon needs `HOLONOMY` to walk an arbitrary edge-list loop on a declared LATTICE and read the connection off a per-edge SU(2) group element committed at insert time.

### Scope

- New verb: `LATTICE`. Declares a graph topology by `(VERTICES n, EDGES [(u, v)…], FACES [(v₀, v₁, …)…], TOPOLOGY "S2"|"T2"|"R3"|…)` with optional shorthand `LATTICE buckyball FROM TRUNCATED_ICOSAHEDRON TOPOLOGY "S2"`. Storage: incidence table + Euler-characteristic precompute + face-cycle orientation table.
- Generalized `HOLONOMY` accepts an edge-list loop on a declared LATTICE. The connection source is the second new asks of Part II (`GAUGE_FIELD`); until that lands, the verb is unreachable from GQL but the storage and walker can be unit-tested against a synthetic per-edge connection.

### Receipts

- `parser/gql.rs::lattice_stmt` accepts the grammar in `GIGI_HALCYON_LATTICE_PRIMITIVES_SPRINT_SPEC.md § 3.P0.1`.
- `query/exec.rs::lattice_register` materializes incidence + face-cycle orientation tables. Round-trip: declare → introspect → declare again from the introspection → bit-identical.
- `bundle/holonomy.rs` generalized: a `walk(edge_list, connection: &dyn EdgeConnection)` signature where `EdgeConnection` is a trait the Levi-Civita and the SU(2)-per-edge implementations both satisfy. The Levi-Civita path is unchanged by inspection.
- Unit test: declare `LATTICE buckyball FROM TRUNCATED_ICOSAHEDRON`, inject a synthetic identity-connection on every edge, walk the 12 pentagonal faces and the 20 hexagonal faces; every face holonomy is the identity to machine precision; Euler check confirms V=60, E=90, F=32, χ=2.

### Pass criterion

Generalized HOLONOMY walks an arbitrary edge-list loop against an injected `EdgeConnection` and returns the same matrix path the Python kernel's `face_holonomy(graph, U)` returns, bit-identical at FP64 for at least one nontrivial U (Halcyon's heatbathed reference state at β = 2.5 in `inertia_damping/reports/run_20260617_110642/final_state.npz`). The cross-check against the on-disk SU(2) U_final from that sidecar is the gate's golden file.

### Blocker on

Nothing. Self-contained.

### Out of scope

- `GAUGE_FIELD` itself (Part II).
- Group taxonomy beyond SU(2). The `EdgeConnection` trait is group-erased at the type level (its method returns an opaque `Matrix`), but the only implementation that ships in Part I is SU(2).
- Face-cycle orientation discovery (Halcyon ships the orientation explicitly per face in the LATTICE declaration; an `ORIENT FACES AUTOMATICALLY` clause is a Part-V wish).

---

## PART II — `GAUGE_FIELD` with group-erased storage

Closes Q2 from the engine-owner reply: don't generalize to arbitrary structure groups at launch. Ship `SU2GaugeField` as a first-class primitive with a group-erased storage layer underneath, so when the next group lands (probably U(1) for the Maxwell-on-the-lattice toy) the verb math is the only new code, not the buffer layout.

### Scope

- New verb: `GAUGE_FIELD field ON LATTICE lat GROUP SU(2) INIT (IDENTITY | HAAR_RANDOM SEED int | FROM other_field);`. Group-erased storage layout (`Group::SU(N)|U(1)|Z(N)` enum tag + `[(n_edges, repr_dim)]` buffer). At launch, only `Group::SU(2)` with `repr_dim = 4` (quaternion) has a verb math implementation; the other tags compile-fail with `unimplemented_for_group!()` at use sites.
- Insert-time per-edge group element commit. The connection that Part I's generalized HOLONOMY reads via `EdgeConnection` is this primitive's `edge_element(e: EdgeId) -> Matrix`.
- `INIT IDENTITY` and `INIT HAAR_RANDOM SEED n` both required at launch (Halcyon's thermalization starts at IDENTITY, the gauge-invariance gate starts at HAAR_RANDOM). The Haar draw goes through the same CSPRNG path SAMPLE_TRANSPORT uses; seed reuse across verbs composes cleanly.

### Receipts

- `parser/gql.rs::gauge_field_stmt` accepts the grammar.
- `query/exec.rs::gauge_field_register` allocates the buffer, calls the init routine, returns a `FieldId`.
- `bundle/gauge_field.rs::SU2GaugeField` implements `EdgeConnection`. The Part I walker reads through it.
- Unit test: `GAUGE_FIELD U_t ON LATTICE buckyball GROUP SU(2) INIT IDENTITY` followed by `SELECT HOLONOMY OF U_t ON LOOP (face_0_edges)` returns identity to FP64 epsilon. Repeat with `INIT HAAR_RANDOM SEED 20260616` and confirm the holonomy is NOT identity (sanity that the storage and the walker both see the same U).
- Cross-check against Halcyon's `inertia_damping/buckyball_action.py::identity_links` and `inertia_damping/buckyball_heatbath.py::sample_haar_link`. Bit-identical at the same seed, same OS, same BLAS — per `HALCYON_TO_GIGI_REPLY_2026-06-17.md § A2`.
- **Cross-engine contract pin (Halcyon-side).** `inertia_damping/test_gigi_part_ii_gauge_field.py::test_G2_A_identity_field_round_trip` is the receipt for INIT FROM byte-equality across the engine boundary: `declare → introspect → declare FROM_FIELD → introspect → bit-equal`. Its enforcement power is *contingent on the live-binding swap* in `gigi_client/mock.py` — until Halcyon imports the embedded PyO3 binding in place of `MockGIGIClient`, the test pins the contract against the Python kernel mock, not the Rust engine. That's expected and named here (per the post-Part-II completeness-critic finding) so the cross-engine coverage is not implicit. When the swap lands, no test rewrite is required.

### Pass criterion

A LATTICE + GAUGE_FIELD declaration round-trips: declare → introspect → re-declare from introspection → re-introspect → exactly the same incidence table, group tag, repr dim, and per-edge element buffer. Plus the Part I gold-file check now runs against `GAUGE_FIELD` rather than the synthetic `EdgeConnection`.

### Blocker on

Part I (the `EdgeConnection` trait and the generalized walker).

### Out of scope

- The verb math for any group other than SU(2).
- Electric field E (the Lie-algebra-valued partner to U). That lives in `SYMPLECTIC_FLOW` (Part IV), not here.
- Automatic IDENTITY → vacuum determination on arbitrary groups (we have a constant, this is fine).

---

## PART III — `PLAQUETTE`, `GIBBS_SAMPLE`, observable batteries

Closes Q5 (GIBBS_SAMPLE naming) and lets Halcyon's heatbath thermalization phase and observable measurements move onto the substrate.

### Scope

- `PLAQUETTE OF field` — primitive that desugars (with Part I's generalized HOLONOMY) to `HOLONOMY ON FACES OF LATTICE OF field`. Three call shapes:
  - `SELECT PLAQUETTE OF U_t` → per-face Re tr(U_f) / 2, shape `(n_faces,)`
  - `SELECT SUM(PLAQUETTE OF U_t)` → scalar Wilson action / β
  - `SELECT MEAN(PLAQUETTE OF U_t)` → ⟨P⟩, the published observable
- `GIBBS_SAMPLE field BETA β [N_SWEEPS n] [MEASURE_EVERY k] [MEASURE (...)] [SEED int]` — Gibbs sampling on a `GAUGE_FIELD` at inverse-temperature β. Per-edge conditional update uses the staple `S_e = Σ_b U_a · U_b · U_c^†` over the four faces touching edge e. Returns updated field + measurement history.
- Observable battery for the `MEASURE` clause at launch:
  - `MEAN(PLAQUETTE)` — ⟨P⟩
  - `Q_SURROGATE` — per spec in `HALCYON_TO_GIGI_REPLY_2026-06-17.md § A1`. Desugars to `SUM(ARCCOS(CLAMP(2 * PLAQUETTE - 1, -1, 1))) / (2 * PI)` if `PLAQUETTE` returns Re tr/2, or `SUM(ARCCOS(CLAMP(REAL(PLAQUETTE), -1, 1))) / (2 * PI)` if `PLAQUETTE` returns the full quaternion's q0.
  - `H_TOTAL` — only meaningful once an E field exists; rejected at parse time with a helpful error in Part III.

### Receipts

- `parser/gql.rs::plaquette_expr`, `gibbs_sample_stmt`.
- `query/exec.rs::gibbs_sample` runs the sweep loop, threads the CSPRNG seed, and emits measurement rows.
- `bundle/heatbath.rs::staple_update` — the per-edge conditional. Pure SU(2) algebra, no new geometry. Reference implementation: `inertia_damping/buckyball_heatbath.py::heatbath_sweep` (200 lines, well-tested at fixed seed across the existing 717 GIGI tests; same staple, same conditional distribution).
- End-to-end golden: a 200-sweep thermalization at β=2.5 SEED 20260616 reproduces Halcyon's `<P>_meas = 0.501598 +/- 0.001463` (from the v1.2 production sidecar) to within the published Flyvbjerg–Petersen SEM. Bit-identical at the same seed, same OS, same BLAS.

### Pass criterion

The first ~80 lines of `inertia_damping/run_validation_report.py` (the build_graph → heatbath_thermalize → measure phase) replaces with a 3-statement GQL block:

```sql
LATTICE buckyball FROM TRUNCATED_ICOSAHEDRON TOPOLOGY "S2";

GAUGE_FIELD U ON LATTICE buckyball GROUP SU(2) INIT IDENTITY;

GIBBS_SAMPLE U
  BETA 2.5
  N_SWEEPS 200
  MEASURE_EVERY 1
  MEASURE (MEAN(PLAQUETTE), Q_SURROGATE)
  SEED 20260616;
```

…and the GQL response carries the same measurement history (within SEM) the Python kernel produces. Halcyon's existing R_A regeneration-reproducibility gate is the contract; the GIGI substrate side passes it or doesn't.

### Blocker on

Part II.

### Out of scope

- `SYMPLECTIC_FLOW` (Part IV).
- HMC with Metropolis acceptance. We don't have that today and the ask is correct that we shouldn't conflate it with the symplectic-flow verb.
- `BLOCKED_SEM` as a substrate primitive. For Part I+II+III, the SEM stays as a post-processing scalar Halcyon computes from the measurement history; if `MEASURE` includes a sample chain, the SEM is a one-liner on the response. Promoting `BLOCKED_SEM` into a verb is a P3 ask from the original spec and deferred.

---

## MEASUREMENT GATE — decide on Part IV

This is the architectural decision point Q4 from the engine-owner reply turned into a gate.

### Inputs to the gate

1. **Coverage receipt:** what % of `inertia_damping/run_validation_report.py` collapsed into GQL after Parts I–III. Expected: 60–80%. The leapfrog (`SYMPLECTIC_FLOW`) is the residual.
2. **Performance receipt:** end-to-end wall-time of one Halcyon trajectory with the Python leapfrog AND the GIGI substrate for thermalization + observables, vs. the all-Python baseline (~46 min wall). Hypothesis: equal-or-better, because the cross-process boundary is just the trajectory frames.
3. **Correctness receipt:** the v1.2 production report (`reports/run_20260617_110642`) re-rendered against the GIGI-substrate thermalization phase passes 8/10 PASS + 1 FAIL with the same single FAIL (Section 5 microcanonical-vs-canonical, the 93-DOF ergodicity caveat). Same FAIL is the success case here, not a regression.
4. **Engineering-cost receipt:** an honest estimate, with hours and the named sub-pieces from the engine-owner reply's Q4 (the three load-bearing pieces of P1.2: (a) (U, E) tangent-space machinery, (b) covariant Gauss projector with the exposed `PROJECT_GAUSS { tikhonov, cg_tol, cg_max_iter }` struct from Q3, (c) symplecticness verification under long integration).

### Decision

If the coverage receipt is ≥ 75% AND the Python leapfrog's wall-time share is < 30% of the total trajectory, the **default is to NOT ship Part IV**. The architectural-unification work is mostly already done; the residual Python integrator is acceptable and the engineering-cost ratio doesn't justify the spend.

If coverage is < 75% OR the Python leapfrog is the dominant wall-time cost OR Marcella's geometric channel develops a need for `SYMPLECTIC_FLOW` as a verb (most likely path: a future-tense lattice-trajectory sampling primitive for the language model), then Part IV ships next.

The gate is **explicitly not a default-yes**. The engine-owner reply called this out specifically: "Python integrator stays" is a legitimate outcome, not a failure.

### Receipts at the gate

`HALCYON_PART_I_MEASUREMENT_GATE_FINDINGS_<date>.md` is the artifact, in the style of `theory/kahler_upgrade/REPLY_TO_GATE_2_FINDINGS_2026-05-24.md`. Numbers, not vibes.

---

## PART IV — `SYMPLECTIC_FLOW` (conditional)

Not in this sprint. Spec is in `GIGI_HALCYON_LATTICE_PRIMITIVES_SPRINT_SPEC.md § 3.P1.2` with Q3's `PROJECT_GAUSS` struct correction folded in. Order-of-magnitude estimate per the engine-owner reply: 2–3× Parts I–III combined.

The three load-bearing pieces, each its own sub-gate if Part IV ships:

1. **`(U, E)` tangent-space machinery.** E lives in `su(2)` (the Lie algebra), U in `SU(2)` (the group). `exp(dt · E) · U` step requires the exponential map and the right type discipline so the verb math doesn't silently round U off the group manifold.
2. **Covariant Gauss projector — CG with Tikhonov.** Exposed as `PROJECT_GAUSS { tikhonov: 1e-12, cg_tol: 1e-10, cg_max_iter: 200 }` (Q3). The verb that decides whether the integrator preserves the constraint surface to machine precision or just close to it. Hard-coded defaults will fail in some β regime; the exposed knobs are necessary, not nice-to-have.
3. **Symplecticness verification.** Explicit `H_total` drift bound under a long integration as the receipt that the verb is what it claims. Halcyon's existing tolerance `dH/H_0 < 1e-3` over 1000 steps at dt=0.02, β=2.5 is the reference contract.

Bit-identity contract per `HALCYON_TO_GIGI_REPLY_2026-06-17.md § A2`. CG iteration count varies across β at fixed seed; that's a diagnostic, not a regression.

---

## Out-of-sprint, named for forward-tracking

- **`MIGDAL_WITTEN` (P2 in the original spec).** Possibly already expressible as a `SPECTRAL` Bessel-kernel ratio. If yes, parser sugar; if no, defer. Either way, not load-bearing for Parts I–III.
- **`BLOCKED_SEM` (P3.1).** Post-processing scalar for now; promote later if multiple consumers need it.
- **`HAAR_RANDOM_GAUGE_TRANSFORM` (P3.2).** Needed for Halcyon's Section 4 gauge-invariance gate. Trivial verb once GAUGE_FIELD ships — applies vertex-indexed Haar `g_v` to a field. Defer to a follow-on sprint or fold into Part II if the bandwidth is there.
- **`ENSEMBLE_FROM_TRAJECTORY` / `KNN_LOO` / `LABEL_PERMUTATION_NULL` / `FEATURE_ABLATION` (P4).** Probably don't belong in GIGI. Stay external. The Halcyon Stage 2 sector classifier is the reference implementation; future consumers can read its source.

---

## Forward-tracking content note

`GIGI Solves: The Clay Seven`, Vol 4, Yang–Mills mass-gap chapter, wants this substrate as live engine output rather than transcribed Python. The Part III ship is the gating event for the chapter's "worked example" section to render against `live engine`, not `python_kernel_v2026_06`. Logging here so the sprint and the chapter's outline stay aligned.

The Halcyon Stage 2 single-FAIL on Section 5 (93-DOF ergodicity caveat) is the honest number that wants to appear in the chapter alongside the PASSes. Per the reply, that gap between "working numerical pipeline" and "proof of the mass gap" stays visible. Not papered over.

---

## Summary

| Part | Scope | Pass criterion | Blocker |
|---|---|---|---|
| I  | LATTICE verb + generalized HOLONOMY (per-edge `EdgeConnection`) | Bit-identical walker vs. Halcyon's `face_holonomy` on the production U_final | — |
| II | `SU2GaugeField` with group-erased storage; IDENTITY / HAAR_RANDOM init | Round-trip; gold-file check now reads through GAUGE_FIELD | I |
| III | PLAQUETTE primitive + GIBBS_SAMPLE verb + observable battery (incl. Q_SURROGATE per A1) | 3-statement GQL block reproduces Halcyon's β=2.5 thermalization within SEM | II |
| Measurement gate | Coverage + performance + correctness + cost receipts | Decision on Part IV; explicit default-no | III |
| IV (conditional) | SYMPLECTIC_FLOW with exposed PROJECT_GAUSS struct | dH/H_0 < 1e-3 over 1000 steps; bit-identity contract per A2 | Measurement gate decision = yes |

Engine-owner reply's Q1–Q3 receipts are absorbed; Q4 is structured as the measurement gate; Q5 naming locked. Both asks back answered in the companion reply.

—Bee + Claude
