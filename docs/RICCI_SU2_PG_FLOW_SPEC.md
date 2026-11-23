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

---

## Amendment 1 (2026-11-18): correcting the two negative controls

**Written after the n = 1 results were seen.** The original gates above are unchanged and
will be reported as written, including where they failed or were uninformative. This
amendment adds corrected controls alongside them. It does not replace them, and it does not
relax G-traj, G-time or G-quad. The n = 1 numbers that prompted it are in
`research/su2-flow-20261118/`.

### In plain English

Two of my negative controls were badly designed. A negative control is a run that is
*supposed* to fail, so that we know the test can tell right from wrong. Both of mine were
written as "this run must have at least 10% error". At the coarsest mesh, every run has
more than 10% error, the right one included, so passing that bar proves nothing. A
control only means something once the right answer's own error is smaller than the gap the
control is meant to detect. The corrected controls say exactly that, and they say when a
control is too weak to judge, instead of letting it pass.

### What went wrong at n = 1

| Run | R at t = 0.02 | E at t = 0.02 |
| --- | ---: | ---: |
| two-ring, moving metric | 13.81% | 0.06886 |
| two-ring, frozen metric (control) | 14.02% | 0.06988 |
| native Whitney force (control) | 7.01% | 0.03493 |
| one ring (removal) | 2,541% | 12.67 |

- **The frozen-metric control passed its gate (R ≥ 10%), but it measured nothing.** The
  moving and frozen runs differ in E by only 0.0010, against a two-ring error of 0.069.
  Freezing the metric changes the result by about 1.5% of the lattice's own error, so at
  n = 1 the lattice cannot see the effect the control exists to detect. The original gate
  passed only because both runs are dominated by spatial error.
- **The native control failed its gate (R = 7.0%, not ≥ 10%).** This is not a defect of the
  experiment. It is the pre-asymptotic behaviour already seen in S2a: native is better than
  two rings at n = 1 (26.25% against 36.25% instantaneously), its error then stalls, and the
  two-ring error keeps falling. A control that must fail *at every level* cannot allow for a
  crossover the programme has already measured. The gate should ask for the crossover.

### Corrected control 1: the frozen metric must be resolvable before it is judged

Let E_m and E_f be the errors of the moving and frozen runs at the same level, step and
quadrature, at t = 0.02. Their gap Δ = ‖log(U_moving⁻¹ U_frozen)‖_B / √Vol measures how
far apart the two lattice trajectories actually are.

- **Resolvability.** The control is *informative* at a level only if Δ > 2 E_m: the effect of
  freezing the metric must be at least twice the moving run's own error. Where Δ ≤ 2 E_m,
  the control is reported as **NOT INFORMATIVE at that level**. It is neither a pass nor a
  fail.
- **Gate, at informative levels.** The frozen run must stay away from the exact trajectory:
  E_f ≥ Δ/2, and E_f must not converge (observed order of E_f below 0.25 between consecutive
  informative levels) while E_m does.
- **Until final states are saved,** Δ is bounded from both sides by
  |E_f − E_m| ≤ Δ ≤ E_f + E_m. At n = 1 the lower bound is 0.0010 and the upper bound is
  0.139, so the control cannot be declared informative there. The script now saves final
  link states (`*.final.npy`) so that Δ can be computed exactly. The n = 2 runs started
  before this amendment do not save them; for those, the bounds are reported, and Δ is
  measured on a rerun if the bounds cannot decide.

### Corrected control 2: the native force must be overtaken, and must stall

- **Crossover.** At the finest level run, the two-ring error is below the native error:
  E_pg < E_native.
- **Stall.** The native error does not converge: observed order of E_native below 0.25
  between the last two levels, matching S2a, where native stayed near 25% at every level.
- If neither the crossover nor the stall appears by the finest level run, that is reported
  as a finding against the two-ring force in time.

### Status of each gate at n = 1, as written and as amended

| Gate | As written | As amended |
| --- | --- | --- |
| G-traj (R < 5% at finest; order ≥ 0.5) | not judged at one level (n = 1: 13.81%) | unchanged |
| G-time (halving h changes E by < E/10) | **PASS** (N = 16 vs 32: 1.6×10⁻⁶ relative) | unchanged |
| G-quad (q4 vs q6 changes E by < E/10) | **PASS** (q4 vs q6 at N = 32: 4.6×10⁻⁸ relative) | unchanged |
| Frozen metric | passed trivially (14.02% ≥ 10%) | **NOT INFORMATIVE** at n = 1 |
| Native force | **FAIL** (7.01% < 10%) | judged at the finest level (crossover and stall) |
| One ring (reported) | blew up as predicted | unchanged |
