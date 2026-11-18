# S3: the SU(2) two-ring force in time, against the Ricci-flowed links (spec)

2026-11-18. Fixed **before** measurement. This is a research replica of the programme's
gate **BW07** (`docs/RICCI_LATTICE_CORRESPONDENCE_SPEC.md` §7): "integrate the native
lattice gradient with the RF metric supplied at time t … same initial metrics and
`[0,0.02]` window; keep the fixed-background experiment as a negative control. No
time-axis alignment." It follows S2a (accuracy: BW05 on Berger), S2b (stability: no
rough modes) and S2c (numerical range). A replica PASS is research evidence, **not** a
BW07 PASS, which needs the native engine path (BW08).

## In plain English

So far we have checked the force one instant at a time: at a single moment, does the
lattice push the links the way the continuum does? Here we let the links actually move.
We start from the exact links of the Berger metric, let the lattice force carry them
forward in small time steps while the metric itself flows by Ricci flow, and compare
where they end up with where the exact theory says they should be. The exact answer is
known because of the programme's continuum identity (RF2A): the Levi-Civita links of the
Ricci-flowed metric are themselves a solution of the Yang–Mills flow, with the clock
fixed to one.

Over the window the metric moves from (3, 1, 1) to about (2.45, 1.05, 1.05), and the
connection coefficients change by 28%, so there is a real trajectory to follow.

## Technical

**Exact trajectory.** g(t) comes from `e2e/probes/ricci_oracle.py` (independent Koszul
curvature, DOP853). The exact links are L(g(t)), the Levi-Civita link formula of
`ricci_lattice_convergence.reference_velocity`, evaluated at g(t).

**Lattice flow.** Links U (right-trivialised), with velocity ξ solving

  B(U, g(t)) ξ = −a(U; g(t)), U_e ← U_e exp(h ξ_e).

- **a** is the S2a two-ring Petrov–Galerkin force. Its quadrature metric and pairing are
  recomputed at every stage from g(t); the fit maps depend on geometry only and are cached.
- **B** is the native tangent mass, recomputed at every stage. Per cell its block (e, f) is
  4·M1_ef·Ad(root_e)ᵀAd(root_f), with roots (u₀, u₁, u₂, u₁, u₂, u₂) and the reversed-edge
  lift, exactly as `src/gauge/ricci_simplicial.rs::cell_tangent_mass` and
  `examples/ricci_operator_probe.rs`. M1 is the Whitney edge mass of g(t). The solve uses
  CG with a Jacobi preconditioner, at relative residual 1e-12.

**Stepper.** Runge–Kutta–Munthe-Kaas of order four (RKMK4), with
dexp⁻¹(Ω, v) = v − ½[Ω, v] + (1/12)[Ω, [Ω, v]]. The bracket convention is fixed numerically
from group commutators (control C-bracket). Step sizes h = 0.02/N with N = 16, 32, 64 at
n = 1, and the stability bound from S2c (RK4 imaginary-axis limit ≈ 2.8/w, w the numerical
radius) sets the smallest N at each finer level. Crank–Nicolson/Cayley stepping is S3b,
for the levels where explicit steps are unaffordable.

**Error.** For a state U and the exact L at the same time, the error is
δ_e = log(L_e⁻¹ U_e) (right-trivialised), and
E(t) = sqrt(δᵀ B δ / Vol), with B taken at the exact state. The **displacement** D(t) is
the same quantity with U replaced by the initial links L(g(0)). The reported error is
**R = E(0.02)/D(0.02)**, and also max over the sampled times of E(t)/D(t). As a diagnostic,
a gauge-invariant error is also reported, from face holonomy angles. Transverse link
components are never removed.

## Gates (thresholds fixed now)

- **G-traj.** For the two-ring force, R < 5% at the finest level run, and an observed
  order of at least 0.5 between consecutive spatial levels (computed from E, not R).
- **G-time.** Halving h changes E(0.02) by less than E/10 at every level reported.
- **G-quad.** q4 against q6 changes E(0.02) by less than E/10.
- **Negative control: fixed background.** g frozen at g(0) in both a and B. It must give
  R ≥ 10% at every level. It is the programme's known failure, and if it passes the
  experiment is not testing the moving background.
- **Negative control: native Whitney force.** Instantaneously 25% wrong (S2a), so it must
  give R ≥ 10%.
- **Removal: one ring (reported).** At the two-ring step size its rough eigenvalues
  (down to −8,621 at n = 1) put hλ far outside RK4's stability region, so it is expected to
  blow up. That is reported, not gated.

## Controls

- **C-link.** The Python L(g(0)) reproduces the frozen native links to 1e-12.
- **C-mass.** The Python B(L(g(0)), g(0)) reproduces the native `blocks` to 1e-12 relative.
- **C-force.** The force at t = 0 reproduces S2a's two-ring gradient to 1e-12 relative.
- **C-bracket.** The RKMK bracket matches the group commutator to second order.
- **C-exact.** Stepping the exact links' own velocity (from the continuum oracle, not the
  lattice) through the same RKMK4 stepper recovers L(g(0.02)) at fourth order in h. This
  tests the stepper and the error measure without the force.

## Honest cost

Measured force cost (S2a, including the fit) is 8 s, 59 s, 142 s and 635 s at
n = 1, 2, 3, 5. With the fits cached, each stage costs the force plus one sparse solve.
RKMK4 has four stages per step.

| n | stages |
| --- | --- |
| 1 | at most 4 × 64 = 256 |
| 2 | the S2c bound sets N; likely 64–128 steps |
| 3 | likely 128–256 steps |
| 5 | **out of scope for explicit stepping** |

**Run order:** n = 1 in full first, then n = 2. n = 3 runs only if the measured per-stage
cost projects it inside one background run. n = 5 waits for S3b. The gate is then judged
on the levels actually run, and the report says which.

## Scope

One metric (Berger (3, 1, 1)), unnormalised flow, window [0, 0.02]. Research script, not an
engine verb. No claim about the nonlinear flow beyond the window, other metrics, or the
quotient gates (BW06).
