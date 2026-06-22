# AURORA → GIGI  |  Bracket spec + Q1/Q2-back answered  |  2026-06-22

## Status

22/22 gates green, T5 B1–B8 green. This reply answers §5 Q1-back and Q2-back from your Phase 2 confirmation and delivers the concrete Lie-Poisson bracket specification for the `HamiltonianPoissonBracket` trait.

---

## §1 — Closed items

**Q3 (stability marker):** Confirmed at 59cdad5. ✅

**Q2 (Engine::open):** `tempdir + open_empty` accepted as the dev pattern.
And yes — **please ship `Engine::open_memory()`**. In `aurora-server` dev/CI the process never inspects the WAL and has nothing to persist between runs. The tempdir pattern is honest but it's boilerplate we'll repeat in every test harness and every CI job. An explicit constructor documents intent. "Explicit > implicit" wins for production; in tests the intent IS "no files." ~30 LOC, every test uses it.

**§8 (separability gap note):** Heard. The receipt-driven discipline is exactly what surfaced the gap in 90 minutes. The documentation debt landing with the Lie-Poisson patch is the right call. ✅

---

## §2 — Q1-back: bracket-evaluator shape (AURORA answers)

**Option B.** `bracket_step(state: &mut [f64], dt: f64) -> Result<(), BracketPhysicsError>` — consumer ships the step, substrate orchestrates the loop.

One clarification on the error boundary: `bracket_step` returns `Err` for **physics invalidity** only — negative layer depth, CFL violation, NaN propagation. It does **not** check Casimir drift. **Casimir drift is the substrate's receipt responsibility.** The receipt contract stays:

```
substrate: call bracket_step → call evaluate → compare with prior-step values
           → if drift > RECEIPT_TOL → emit Refusal, halt
```

Same referee contract as the existing Euler path. `bracket_step` never needs to know `RECEIPT_TOL` exists. One place for physics, one place for conservation accounting.

---

## §3 — Q2-back: sequencing (AURORA answers)

**After.** The concrete spec is in §4. Design the trait surface against it; AURORA implements `ShallowWaterHandle::bracket_step` once the trait lands. This absorbs any rework cost on the AURORA side, not the substrate side.

---

## §4 — Concrete shallow water Lie-Poisson bracket specification

### State layout

Unchanged from Phase 2:

```
[u_lam_0, …, u_lam_{n-1} | u_phi_0, …, u_phi_{n-1} | h_0, …, h_{n-1}]
n = n_lam × n_phi, row-major (λ varies fastest)
```

### Parameters on `ShallowWaterHandle` (not in the state buffer)

`g`, `Ω`, `a`, `n_lam`, `n_phi`, grid arrays `(λ_i, φ_j)`, orography `h_s[i, j]` (zeros for T2, mountain field for T5).
The trait receives only `state: &mut [f64]` and `dt: f64` — `self` provides everything else.

### Equations of motion — vector-invariant form

```
ζ    = (1 / (a cos φ)) [∂u_φ/∂λ − ∂(u_λ cos φ)/∂φ]   (relative vorticity)
f    = 2Ω sin φ                                           (Coriolis)
B    = g(h + h_s) + ½(u_λ² + u_φ²)                      (Bernoulli function)

∂u_λ/∂t = +(ζ + f) u_φ − (1/(a cos φ)) ∂B/∂λ
∂u_φ/∂t = −(ζ + f) u_λ − (1/a) ∂B/∂φ
∂h/∂t   = −(1/(a cos φ)) [∂(h u_λ)/∂λ + ∂(h u_φ cos φ)/∂φ]
```

**Relation to the current `tendencies()` implementation:**

`src/integrator.rs` uses the **advective form**:

```
∂u_λ/∂t = −(u_λ/(a cos φ)) ∂u_λ/∂λ − (u_φ/a) ∂u_λ/∂φ + (u_λ u_φ/a) tan φ + f u_φ − (g/(a cos φ)) ∂h/∂λ
```

The two forms are mathematically equivalent for smooth solutions (related by the vector identity `u·∇u_λ + u_λ u_φ/a tan φ = (ζ+f)u_φ − (1/a cos φ) ∂(|u|²/2)/∂λ`). They differ in their **discrete conservation properties**. The vector-invariant form exposes the skew-symmetric bracket structure; the advective form does not. `bracket_step` will re-implement the tendency in vector-invariant form. No physics change — discretization change.

The orographic terms (`h_s ≠ 0`) are already absorbed by the Bernoulli function: `B = g(h + h_s) + ½|u|²`. No special-casing needed for T5 inside `bracket_step`.

### Casimir witnesses (the complete receipt set)

| Quantity | Formula | Role |
|---|---|---|
| mass | ∫ h dA | Casimir — conserved by every H-flow |
| PV L1 | ∫ (ζ + f) dA | Casimir — absolute vorticity integral |
| PV L2 | ∫ h q² dA  (q = (ζ+f)/h) | Casimir — potential enstrophy |
| energy | ∫ (½h\|u\|² + ½g h²) dA | Hamiltonian — conserved, not a Casimir |

All four are already in `ENERGY_KEYS` and returned by `evaluate()`. `SYMPLECTIC_FLOW` checks all four against `RECEIPT_TOL` after each `bracket_step` call.

### Discrete bracket-preservation requirement

The bracket preserves ∫h q² dA (potential enstrophy) iff the **vorticity flux** `(ζ+f)u⊥` and the **flux divergence** `∇·(hu)` are discretized with **consistent operators** — the Arakawa (1966) skew-symmetry condition. On the A-grid (all variables at cell centers):

1. Relative vorticity `ζ` computed at cell corners via finite-difference curl, averaged back to cell centers.
2. Divergence `∇·(hu)` at cell centers using the same averaging weights as step 1.
3. Pressure gradient `∇B` computed as the exact discrete gradient dual to the step-2 divergence.

AURORA owns this stencil — it goes inside `bracket_step` on `ShallowWaterHandle`. The trait itself is stencil-agnostic.

### Physics error type

```rust
pub enum BracketPhysicsError {
    /// Layer depth h ≤ 0 after the step.
    NegativeDepth { i: usize, j: usize, h: f64 },
    /// CFL condition violated.
    CflViolation { courant: f64, max_courant: f64 },
    /// Consumer-defined physics invalidity.
    Other(String),
}
```

---

## §5 — Proposed trait shape

```rust
/// Lie-Poisson bracket integrator for non-separable Hamiltonians.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
pub trait HamiltonianPoissonBracket: HamiltonianHandle {
    /// Advance state by one Lie-Poisson bracket step of duration dt.
    ///
    /// Consumer owns the bracket-preserving integration (skew-symmetric
    /// vorticity flux, consistent divergence, vector-invariant momentum).
    /// Substrate calls this in the SYMPLECTIC_FLOW loop, then checks receipt.
    ///
    /// Returns Err for physics invalidity (negative depth, CFL breach).
    /// Does NOT check Casimir drift — that is the substrate's receipt responsibility.
    fn bracket_step(
        &self,
        state: &mut [f64],
        dt: f64,
    ) -> Result<(), BracketPhysicsError>;
}
```

---

## §6 — Capabilities dispatch

The factory declares which integration surfaces the handle provides:

```rust
pub struct HamiltonianCapabilities {
    pub force_drift:      bool,   // Störmer-Verlet KDK (Force + Drift path)
    pub poisson_bracket:  bool,   // Lie-Poisson (bracket_step path)
}

// In HamiltonianFactory (additive — no change to existing surface):
fn capabilities(&self) -> HamiltonianCapabilities;
```

| Factory | Capabilities | SYMPLECTIC_FLOW path |
|---|---|---|
| `KogutSusskindFactory` | `{ force_drift: true, poisson_bracket: false }` | Störmer-Verlet KDK — no change |
| `ShallowWaterFactory` now | `{ force_drift: true, poisson_bracket: false }` | KDK — known wrong, receipt refuses |
| `ShallowWaterFactory` after trait | `{ force_drift: true, poisson_bracket: true }` | SYMPLECTIC_FLOW prefers bracket_step |

Keeping `force_drift: true` on ShallowWater after the bracket trait lands preserves the A17/A18 diagnostic gates and allows comparative receipt tests (KDK drift 3.5e-10 vs bracket drift ≤ 8×ε).

---

## §7 — Phase 3 scope alignment

| item | status | next |
|---|---|---|
| Q3 stability marker | ✅ closed | — |
| Q2 Engine::open | ✅ closed | GIGI ships `open_memory()` when convenient |
| Q1 SYMPLECTIC_FLOW bracket trait | ⏸ → **unblocked by §4 above** | GIGI ships trait; AURORA implements |
| CUBED_SPHERE A1 | ✅ confirmed (f62e46c) | — |
| T5 topographic dynamics B9+ | AURORA-only | no GIGI ask; see §8 |

---

## §8 — T5 topographic forcing (AURORA-only, in parallel)

While the bracket trait is being designed, AURORA is extending the existing integrator with topographic forcing so T5 dynamics gates (B9+) can run under the Force/Drift path. The orographic pressure-gradient terms are:

```
∂u_λ/∂t += −(g / (a cos φ)) ∂h_s/∂λ
∂u_φ/∂t += −(g / a) ∂h_s/∂φ
```

These are the only structural difference between T2 and T5 dynamics. In the vector-invariant form (`bracket_step`), they are already absorbed by the Bernoulli function `B = g(h + h_s) + ½|u|²` — no separate term needed. In the current advective-form `tendencies()` they are an additive correction that `bracket_step` will eventually supersede.

No GIGI dependency. Gates B9–B12 will test sign, magnitude, and grid-cell localization of the orographic tendency.

---

## One open question back

Optional follow-up only — not blocking:

The `capabilities()` field suggests a factory can declare both surfaces. If `SYMPLECTIC_FLOW` sees `poisson_bracket: true`, does it completely ignore the Force/Drift surface for time-stepping, or does it still use Projection after the bracket step? (AURORA assumes yes — Projection is the Gauss constraint cleaner and independent of the integrator choice. Just confirming.)

---

— Bee Rosa Davis & Rory, Principal AURORA Engineer (2026-06-22)
