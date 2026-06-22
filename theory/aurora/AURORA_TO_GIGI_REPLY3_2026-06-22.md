# AURORA → GIGI  |  Phase 2 Complete + KDK Finding  |  2026-06-22

## Status

Phase 2 is complete. 22/22 TDD gates green. `aurora-server` binary ships and runs.

---

## What shipped this sprint

**ShallowWaterHandle** (full gigi trait surface):

| Trait | Implementation |
|---|---|
| `HamiltonianForce` | `force()` → `[du_λ/dt, du_φ/dt, 0…]` — h-slice zeroed for KDK kick |
| `HamiltonianDrift` | `drift()` → h advanced by continuity, u held fixed |
| `ProjectionOperator` | `project_constraint()` — shape check; Poisson CG deferred to Phase 3 |
| `EnergyDecomposition` | `evaluate()` → BTreeMap of 7 Casimir + Kelvin keys |
| `HamiltonianHandle` | supertrait satisfied |

**ShallowWaterFactory** registered under `"SHALLOW_WATER"` (kind_tag, group_tag `"R"`) via `aurora::init()`.

**`aurora-server` binary** — live output from `cargo run --bin aurora-server`:

```
AURORA atmospheric Gi-System  v0.1.0
=====================================

GIGI Hamiltonian registry (1 registered):
  SHALLOW_WATER             kind=SHALLOW_WATER  group=R

Williamson T2  step 0  (64x32 A-grid):
  casimir_energy =   1.543713e22  J
  casimir_mass   =   1.205597e18  kg
  refusal_reason = None
  RECEIPT_TOL    = 1.78e-15

ShallowWater energy decomposition  (GIGI registry path):
  casimir_energy       = 1.543713e22
  casimir_mass         = 1.205597e18
  casimir_pv_l1        = 2.317497e-27
  casimir_pv_l2        = 2.474232e3
  kelvin_eq            = 1.541929e9
  kelvin_n30           = 1.137132e9
  kelvin_s30           = 1.137132e9

Williamson T2  step 1  (forward Euler  dt=60s):
  casimir_energy =   1.543713e22  J
  refusal_reason = Some("casimir_energy drift 4.979e-11 > 1.776e-15 (8× machine_eps)")
  -> receipt refused Euler drift (expected — A13 gate)

Williamson T2  step 1  (Stormer-Verlet KDK  dt=60s):
  casimir_energy =   1.543713e22  J
  rel_drift      = 3.527e-10      (RECEIPT_TOL = 1.78e-15)

Phase 2 complete. Next: Engine::open + gigi-stream (Phase 3).
```

---

## KDK non-separability finding — input for SYMPLECTIC_FLOW design

This is the key finding from Phase 2. Please read before designing SYMPLECTIC_FLOW.

**The result**: naive Störmer-Verlet KDK produces *larger* Casimir drift than forward Euler for Williamson T2:

| Integrator | Casimir energy drift (relative) |
|---|---|
| Forward Euler (refused by receipt) | 5.0 × 10⁻¹¹ |
| Störmer-Verlet KDK | **3.5 × 10⁻¹⁰** (~7× worse) |

Both exceed `RECEIPT_TOL = 1.78e-15`. But KDK being worse is the surprising part.

**Why**: The shallow water Hamiltonian is not separable in the (q, p) = (h, u) sense. Standard Störmer-Verlet assumes H = T(p) + V(q). Shallow water has:

```
T = ½ ∫ h |u|² dA     ← depends on BOTH h and u
V = ½g ∫ h² dA        ← depends on h
```

The advection terms (`u·∇u`) in the momentum equation also mix h and u. When we do the half-kick in u_φ (which has a small but nonzero discretization error du_φ/dt ≠ 0 on the A-grid), the perturbed u_φ propagates into the continuity equation during the drift step:

```
∂h/∂t = -(1/(a cos φ)) [∂(hu_λ)/∂λ + ∂(hu_φ cos φ)/∂φ]
```

After the half-kick, u_φ ≈ 0.84 × 10⁻³ m/s (small but nonzero). This drives dh/dt ≠ 0 in the drift step, changing h. The second half-kick is then applied at a different h, creating an asymmetric error that accumulates. Euler avoids this because it computes dh/dt at the *original* state (where u_φ = 0 → dh/dt ≈ 0).

**What this means for SYMPLECTIC_FLOW**:

The shallow water equations on a rotating sphere have semidirect-product Lie-Poisson structure:

```
∂u/∂t = {u, H}  on  T*Diff(S²) ⋉ F(S²)
```

The correct structure-preserving integrator is a **Lie-Poisson integrator** (or Euler-Poincaré integrator), not particle KDK. The specific split depends on which part of H you drift vs. kick:

- If you split into "rotation" (Coriolis + advection) and "pressure" (geopotential gradient), you get a Poisson-bracket-preserving split that conserves PV on the sphere.
- The Casimir functionals (mass, PV L1, PV L2, energy) are invariants of the Lie-Poisson bracket; a bracket-preserving integrator conserves them exactly (up to solve tolerance).

This is different from the Yang-Mills / Kogut-Susskind KDK, which works because the lattice gauge action separates cleanly into plaquette terms (position) and electric field terms (momentum).

**Recommendation**: Before designing SYMPLECTIC_FLOW for AURORA, please confirm whether the design is:
1. Generic KDK (works for Yang-Mills; does NOT work for ShallowWater)
2. Configurable Poisson split (needed for ShallowWater Lie-Poisson structure)
3. Hamiltonian-agnostic bracket evaluator (the right long-term answer; heavier)

AURORA can provide the concrete Poisson bracket for the shallow water system if that helps the design.

---

## Phase 3 asks (priority order)

1. **SYMPLECTIC_FLOW redesign** (above) — needed before Williamson T2 can pass the receipt gate under time integration
2. **Engine::open scaffold** — minimal config to connect `aurora-server` to a local gigi instance; `aurora-server` already calls `aurora::init()` as required by Q5
3. **CUBED_SPHERE lattice** (A1 from the original ask) — unblocked on GIGI side per prior discussion

---

## Williamson Test 5 starting in parallel

We are not waiting on Phase 3 to start T5. First 8 TDD gates (B1–B8) are written:

- B1: Mountain peak height = HMNT (2000 m) at (LAM_MNT, PHI_MNT)
- B2: Mountain height = 0 outside angular radius RMNT = π/9
- B3: Orography ≥ 0 everywhere on the grid
- B4: Layer depth h > 0 everywhere (mountain does not pierce the surface)
- B5: Total height η = h + h_s geostrophically balanced to < 1 m pointwise
- B6: T5 Casimir mass < T2 Casimir mass (T5 has shallower layer, H₀_T5 ≈ 608 m vs 2999 m)
- B7: Mountain footprint is non-empty on the 64×32 grid
- B8: Zonal velocity field matches T5 parameters (u_λ = U₀_T5 cos φ, u_φ = 0)

T5 does not have an exact solution, so the next gates after B8 will test conservation properties under forward-Euler (baseline) and — once SYMPLECTIC_FLOW is designed — under the Lie-Poisson integrator.

---

## Open questions for GIGI

1. **SYMPLECTIC_FLOW**: Is the current design generic KDK, or does it have a configurable split for Lie-Poisson systems? If KDK, when does the Lie-Poisson branch start?
2. **Engine::open**: What's the minimal config struct for a local-only gigi instance? We want to call `Engine::open(...)` in `aurora-server` with an in-memory or temp-file store.
3. **Stability comment**: We asked for `"stability: EVOLVING until 0.1.0 tag"` on `src/gauge/action.rs` per Q6 — has this landed?

— AURORA (Bee Rosa Davis, 2026-06-22)
