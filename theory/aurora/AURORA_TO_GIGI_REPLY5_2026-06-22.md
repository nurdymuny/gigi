# AURORA → GIGI  |  Phase 3 complete — honest drift table + Phase 4 DEC ask  |  2026-06-22

## Status

41/41 gates green. Phase 3 deliverables: Arakawa-Lamb bracket_step, Engine::open_memory() wired,
aurora-server Phase 3 output running. This letter reports the measured result and names the next gap.

---

## §1 — Phase 3 measurements (honest)

Full output from `cargo run --bin aurora-server` on the 64×32 A-grid at dt=60s:

| Integrator | Casimir energy drift |
|---|---|
| Forward Euler | 4.98 × 10⁻¹¹ |
| Störmer-Verlet KDK (non-separable) | 3.53 × 10⁻¹⁰ |
| **Arakawa-Lamb bracket\_step** | **4.59 × 10⁻¹¹** |
| RECEIPT\_TOL (science goal) | **1.78 × 10⁻¹⁵** |

Gap to receipt gate: **25,845×**.

The Arakawa-Lamb bracket_step beats KDK by ~8× and is comparable to (marginally better than)
forward Euler. For Williamson T2 with u_φ = 0 in the initial condition, this is the correct
result — the Arakawa scheme improves the nonlinear PV flux term (`q × hu_φ`), but that term
is zero in T2 (u_φ = 0 everywhere in the IC). The T2 drift is dominated by a different source:
the **O(Δφ²) discrete geostrophic imbalance** in ∂u_φ/∂t, which the Arakawa scheme cannot remove.

### Why the A-grid floor is irremovable

The continuous equations for T2 have zero tendency everywhere (steady geostrophic state).
On the discrete A-grid, the discrete geostrophic balance condition is:

```
(ζ_h + f) u_λ  ≈  −(1/a) ∂B_h/∂φ
```

where ζ_h is the centered-difference vorticity and B_h is the centered-difference Bernoulli.
These operators are consistent (both O(Δφ²)), but their discrete kernels differ. The residual
`(ζ_h+f)u_λ + (1/a)∂B_h/∂φ` is O(Δφ²) ≈ (π/32)² ≈ 10⁻² and is nonzero even for the
exact analytical IC. This residual drives a nonzero ∂u_φ/∂t which accumulates into energy drift.

No A-grid finite-difference integrator (Euler, KDK, Arakawa-Lamb, Runge-Kutta) removes this
residual — it is a property of the spatial operators, not the time-stepping. The bracket_step
reduces it only where the Arakawa consistency condition matters (nonlinear flows with u_φ ≠ 0).

### What Arakawa-Lamb IS buying us (T5 and T6)

For **T5** (zonal flow over a mountain) and **T6** (Rossby-Haurwitz wave), u_φ ≠ 0 after the
first step. The Arakawa consistency condition:

> Use the same edge-averaged mass flux `F = (F_λ, F_φ)` in the PV vorticity flux AND
> in the continuity divergence.

eliminates discrete energy production from the cross-term `q × F_φ`. For nonlinear dynamics
where PV is actively rearranged by the flow (T5, T6), this matters. The Arakawa-Lamb bracket_step
should show measurably lower drift than Euler for T5/T6 multi-step runs. We haven't gated this
yet (T5 multi-step is not in the current pre-registered C-suite), but the implementation is ready.

---

## §2 — The DEC path: what closes the gap

The receipt gate (RECEIPT_TOL = 1.78e-15) is achievable only via **discrete exterior calculus
on the cubed-sphere**. Here is why, and what AURORA needs from GIGI to get there.

### Why DEC works where A-grid doesn't

On the cubed-sphere with DEC, the velocity `u` is a **1-form on edges** and the height `h`
is a **0-form on faces** (or its Hodge dual on vertices, depending on primal/dual choice).
The discrete vorticity is `ζ = dᵢu` — the discrete exterior derivative of the edge 1-form,
giving a 2-form on faces. The key identity:

```
d ∘ d = 0  (discrete Poincaré lemma — exact on any cell complex)
```

means `d(ζ+f) = 0` **exactly at the discrete level**. The PV (ζ+f)/h is exactly transported
by the discrete flow (no truncation error). This is what machine-precision Casimir preservation
means: the discrete dynamics is literally a finite-dimensional Hamiltonian system where the
Casimir invariants are exactly conserved by construction, not approximately.

The A-grid centered-difference vorticity `ζ_h` satisfies `d_h(ζ_h) ≈ 0` only up to O(Δφ²)
— there is no exact discrete Poincaré lemma for the standard finite-difference operators.

### AURORA's specific Phase 4 asks

**Phase 4a** (blocking): The DEC operators on the cubed-sphere, as specified in GIGI's reply 2:
- `d_0: LatticeWithMetric → (0-form → 1-form)` (exterior derivative)
- `hodge_star_0`, `hodge_star_1`, `hodge_star_2` (discrete Hodge star)
- `delta_0 = ⋆₁ d_0 ⋆₀` (formal adjoint, gives divergence)

These were spec'd in GIGI reply 2 as landing in `src/lattice/dec/`. We need them to rewrite
bracket_step with DEC-native operators.

**Phase 4b** (blocking): A state layout for shallow water on the cubed-sphere. AURORA needs
to know the discrete analog of `[u_λ, u_φ, h]`:

| A-grid field | DEC analog | Location on cubed-sphere |
|---|---|---|
| `u_λ[i,j]`, `u_φ[i,j]` | velocity 1-form `α_e` | one value per **edge** e |
| `h[i,j]` | height 0-form `h_f` | one value per **face** f |

The state buffer layout would then be: `[α_{e_0}, …, α_{e_{N_e-1}}, h_{f_0}, …, h_{f_{N_f-1}}]`.
This is a different shape from the current `[u_lam, u_phi, h]` flat buffer (3 × N_LAM × N_PHI).
AURORA needs GIGI's `CubedSphereMesh` to expose: edge count, face count, adjacency map
(which faces border each edge, with orientation signs), and the DEC operators above.

**Phase 4c** (non-blocking, can parallelise): When is the Halcyon workflow whjzzpwfl resolved?
AURORA has no action but wants to know when the receipt surface signals from that path will land
(per the earlier note: "will surface receipts when landed").

---

## §3 — AURORA v0.1 scope decision (AURORA-side)

Given the Phase 4 timeline, AURORA's v0.1 delivery will:

1. **Ship with A-grid** at the 4.6e-11 drift level for T2/T5/T6. Receipt is refused every step —
   refusal_reason is printed, not a crash. The dycore runs.
2. **Gate T5 multi-step** (D-suite) under Arakawa-Lamb, documenting the improvement over Euler.
3. **Hold the receipt-pass claim** until DEC lands. The receipt gate is not negotiable. We will
   not weaken RECEIPT_TOL or soften the gate — it is the main claim.
4. **v0.2 target**: DEC bracket_step on cubed-sphere; T2 passes receipt gate.

This is the honest version: AURORA v0.1 is a Casimir-tracking dycore that always refuses,
but tracks the Casimir witnesses on every step. v0.2 is the first version that passes.

The Davis Field Equations (C = τ/K and S + d² = 1) require the Casimir witnesses to be
accurate enough to compute sectional curvature K. At 4.6e-11 drift, K is corrupted by
~5 orders of magnitude relative to the machine-precision ideal. Regime classification
from first principles is therefore deferred to v0.2. The receipt discipline is correct:
a refused receipt is not a failure of the dycore — it is the dycore doing exactly what
it should (refuse to classify regimes on Casimir-approximate dynamics).

---

## §4 — Open question answered (from §7 of Reply 4)

> If SYMPLECTIC_FLOW sees poisson_bracket: true, does it completely ignore Force/Drift
> for time-stepping, or does it still use Projection after bracket_step?

AURORA's assumption confirmed by the A-grid experience: Projection is independent of the
integrator. bracket_step advances the state; Projection (Gauss constraint cleaner) is applied
after to enforce any constraints the time-stepping may have drifted slightly. For T2 on the
A-grid, the Projection step isn't needed (no gauge constraint), but AURORA will call it
after bracket_step in the full pipeline. The trait contract is:

```
loop:
  bracket_step(state, dt)        ← consumer advances
  project_constraint(state)      ← consumer constrains (if applicable)
  evaluate(state)                ← substrate measures Casimir witnesses
  if drift > RECEIPT_TOL → Refusal, halt
```

No open questions on this — AURORA will implement the Projection call.

---

## §5 — Phase 3 gate summary

| Gate | Description | Result |
|---|---|---|
| C1 | capabilities() reports both surfaces | ✅ |
| C2 | as_poisson_bracket() returns Some | ✅ |
| C3 | bracket_step Ok on T2 IC | ✅ |
| C4 | h > 0 after bracket_step | ✅ |
| C5 | drift < KDK (3.5e-10) AND < Euler (5.5e-11) | ✅  4.59e-11 |
| C6 | NegativeDepth on thin-layer stress | ✅ |
| C7 | Engine::open_memory() opens and drops | ✅ |

41 of 41 gates green (A1–A22, B1–B12, C1–C7). Phase 3 closed.

---

— Bee Rosa Davis & Rory, Principal AURORA Engineer (2026-06-22)
