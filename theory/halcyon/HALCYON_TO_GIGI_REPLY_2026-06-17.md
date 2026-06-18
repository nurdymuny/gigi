# Halcyon → GIGI reply, 2026-06-17

**Re:** Engine-owner response to GIGI_HALCYON_LATTICE_PRIMITIVES_SPRINT_SPEC.md
**Status:** Closure on Q1–Q5 + answers to the two asks back
**Author:** Bee Rosa Davis, with Claude (Anthropic)

---

## Accepts (Q1–Q5)

All five pushbacks taken without amendment. The original spec was wrong at each spot you flagged. Codifying the corrected version in `HALCYON_PART_I_GATES.md` (this commit). Brief receipt-by-receipt:

- **Q1.** PLAQUETTE-as-sugar is not over today's HOLONOMY; today's HOLONOMY can't read a per-edge group element. Generalized HOLONOMY (accept per-edge SU(2) connections on a declared LATTICE) is a precondition in PART I, not buried under "thin wrapper."
- **Q2.** SU2GaugeField first-class; group-erased storage layer (`Group::SU(N)|U(1)|Z(N)` tag + `[(n_edges, repr)]` buffer); only SU(2) math at launch. The other groups stay tag-only until someone needs them.
- **Q3.** PROJECT_GAUSS is a struct, not a boolean. `{ tikhonov: 1e-12, cg_tol: 1e-10, cg_max_iter: 200 }` as defaults; `PROJECT_GAUSS TRUE` desugars to those defaults; `PROJECT_GAUSS FALSE` skips entirely; named clauses override individual fields. This is wired into PART III's measurement-gate criterion — if the Python integrator is still alive after P1.1 ships, the reason is almost always going to be a regime where the defaults don't converge in budget and we need the knobs exposed to push through it.
- **Q4.** Sprint shape over week count, P1.2 at 2–3× P0+P1.1, order P0 → P1.1 → measurement gate → conditional P1.2. Locked.
- **Q5.** GIBBS_SAMPLE over HEATBATH_SWEEP. Taxonomy alignment with SAMPLE_TRANSPORT is the right reason. SYMPLECTIC_FLOW stays (HMC implies Metropolis acceptance, which isn't in the ask).

## Answers (your two asks back)

### A1 — `Q_surrogate` spec

One paragraph, as requested. `Q_surrogate(U)` is a pure post-processing scalar on plaquette values; the verb signature needs nothing it doesn't already get from PLAQUETTE.

> **Definition.** For an SU(2) gauge field U on a LATTICE with F faces, compute the face holonomy `U_f ∈ SU(2)` for each face (the ordered product of edge group elements around the face loop, with edge-orientation signs from the LATTICE's incidence). Write `U_f` in quaternion form `(q0, q1, q2, q3)` with `q0 = cos(θ_f / 2)`. Then
>
> ```
> Q_surrogate(U) := (1 / 2π) · Σ_{f ∈ faces} arccos(clamp(q0(U_f), -1, 1))
>                 = (1 / 4π) · Σ_f θ_f
> ```
>
> where `θ_f ∈ [0, 2π]` is the rotation angle of the face holonomy on SU(2)'s double cover of SO(3). Range: `[0, F/2]` (= `[0, 16]` on the truncated icosahedron). Zero at `U = I`. Gauge-invariant per-face (face holonomy is a conjugacy class up to cycle base point, and `arccos(q0)` is the class invariant). **Not topologically quantized** — π₂(SU(2)) = 0 on S², so this is a smooth scalar, not a Chern number. It tracks the *cumulative angular distance* of the face holonomies from the vacuum and serves as the binning observable for the Halcyon Stage 2 sector classifier.

Implementation reference: `inertia_damping/buckyball_observables.py:Q_surrogate` (~10 lines, `acos`-based; no kernel state, no seed).

GQL-surface form, if you want it as a top-level observable on `MEASURE` clauses:

```sql
MEASURE (Q_SURROGATE)
-- desugars to: SUM(ARCCOS(CLAMP(REAL(PLAQUETTE OF U), -1, 1))) / (2 * PI)
```

If `PLAQUETTE` returns the full quaternion per face, the desugaring is parser-level. If it returns only `Re tr(U_f) / 2 = q0`, same thing.

### A2 — Bit-identity contract for `SYMPLECTIC_FLOW`

Committed explicitly:

| Re-run condition | Contract |
|---|---|
| Same seed, same β, same dt, same n_steps, **same process** | **Bit-identical** on every step's `(U, E)` state and on `H_total`, `gauss_residual_max`. |
| Same seed, same β, same dt, same n_steps, **different process, same OS, same BLAS** | Bit-identical. |
| Same seed, same β, same dt, same n_steps, **cross-OS** (Linux ↔ macOS x86_64 vs. Windows MSVC) | Up to 2 ULPs of drift in trig reductions tolerated. Documented, not enforced. |
| Same seed, **different β** | **NOT bit-identical and does not claim to be.** The constraint operator's spectrum depends on β, the CG iteration count changes with the condition number, and the projector's intermediate states differ. The verb's output `(U, E)` will track the correct symplectic flow up to the documented energy-drift and Gauss-residual tolerances, but step-by-step bit-identity is explicitly waived. |
| Same seed, **different dt** | NOT bit-identical (different discretization), tolerances reapply. |
| Same seed, **different n_steps** | First `min(n1, n2)` steps must be bit-identical to the longer run. |

The receipt the validator should NOT read as a regression: CG iteration count varies across β at fixed seed. Halcyon's existing report's `cg_iterations_per_step_p99` is a *diagnostic*, not a gate.

The receipt the validator SHOULD read as a regression: `max_energy_drift_rel` or `gauss_residual_max` violating the published TOL — those are the symplecticness and constraint-preservation receipts, and they need to hold across all (β, dt) Halcyon sweeps.

## Content note acknowledged: Solves Vol 4 (YM mass gap chapter)

Logged. The three-way receipt — Halcyon's PASS, GIGI's GQL result, Solves Vol 4's worked example — is exactly the architectural shape the Davis Framework's mass-gap argument should stand on. The P0+P1.1 sprint that lets `run_validation_report.py`'s first ~400 lines collapse into a GQL block is the same sprint that lets the mass-gap chapter render as live engine output rather than transcribed Python. I'll flag this on the chapter's outline so the deadline pressure goes the right way (book pulls on engine, engine pulls on substrate, substrate pulls on Halcyon Stage 3 spec).

Side note inside the side note: the Stage 2 single-FAIL on Section 5 (Microcanonical vs Canonical, the 93-DOF finite-trajectory ergodicity caveat) is the **honest** number that wants to appear in the chapter alongside the PASSes. It documents the gap between "we have a working numerical pipeline" and "we have a proof of the mass gap." Worth keeping that visible, not papered over.

## Closing

Sprint gate (`HALCYON_PART_I_GATES.md`) is the artifact. Ship P0 + P1.1 against it; the measurement gate at the end of P1.1 decides P1.2.

—Bee + Claude
