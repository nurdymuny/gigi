# S3b: the Cayley time step, and the SU(2) flow at n = 3 (spec)

2026-11-18. Written before any measurement in this file. Research script
`scripts/ricci_su2_pg_flow_cayley.py`, **not an engine run**. It follows S3
(`docs/RICCI_SU2_PG_FLOW_RESULTS.md`) and S3a (`docs/RICCI_SU2_PG_FLOW_FILTER_RESULTS.md`).
Provenance of the step: *Three Ways Across a Turn* (`docs/papers/chord-arc-tangent/`),
where the Crank–Nicolson step is the Cayley map, the tangent reading of a turn.

## In plain English

The explicit stepper used in S3 must take tiny steps because of the stiff directions S3a
found: at n = 3 it would need about 450 force evaluations per run for the two-ring force.
The Cayley step is implicit. It never blows up, whatever the step size, so the step can be
chosen for accuracy alone. Each step costs a small nonlinear solve. The plan is to check the
new stepper against the explicit one at n = 1 and n = 2, where both are affordable, and
only then use it at n = 3.

## The algorithm

The flow is U′ = U ξ(U, t) with ξ = −B(U, g(t))⁻¹ ∇(U, g(t)), right-trivialised, as in S3.

- **Cayley map.** For a pure quaternion v, cay(v) = (1 + v/2)(1 − v/2)⁻¹
  = ((1 − |v|²/4) + v) / (1 + |v|²/4). It is exactly unit length, so every update stays on
  SU(2), and it agrees with S3's exponential qexp(v) to second order.
- **Step (Lie implicit midpoint).** U_{k+1} = U_k · cay(Ω), where Ω solves
  F(Ω) = Ω − h ξ(U_k · cay(Ω/2), t_k + h/2) = 0.
  On the linearised flow this is the Crank–Nicolson rule, with stability function
  (1 − hλ/2)/(1 + hλ/2), bounded by 1 on the whole right half-plane. The inverse
  Cayley-derivative correction is dropped. That is still second order, because the dropped
  commutator [Ω/2, ξ] vanishes to leading order, Ω being parallel to hξ. C-order checks this.
- **Newton solve (quasi-Newton).** The iteration matrix is fixed once per run:
  A = B₀ + (h/2) K₀, with B₀ the native mass and K₀ the linearised gradient, both at t = 0
  (K₀ as in S2b/S2c; cached for n = 1, 2, built for n = 3 by the same column routine, run in
  parallel). Each iteration solves A Δ = −B₀ F(Ω) and sets Ω ← Ω + Δ. A is factored once.
  The start value is the previous step's Ω (zero at the first step).
- **Convergence.** Stop when ‖Δ‖ ≤ 10⁻¹⁰ ‖Ω‖ (Euclidean, all links). **Timeout:** 40
  iterations. A step that does not converge stops the run, with status "failed at step k".
  It is reported, never retried with a smaller step, and never silently accepted.
- **Cost, in force evaluations** (one evaluation = one gradient plus one mass solve): one
  per Newton iteration, plus four per sample time for the error. Iteration counts are
  recorded per step. The seconds per evaluation are measured and reported.

## Controls (thresholds fixed now)

- **C-cay.** |cay(v)| = 1 to 10⁻¹⁵, and |cay(v) − qexp(v)| / |v|³ is bounded as |v| → 0.
- **C-order.** Integrate the exact links' own velocity (S3's C-exact field, which does not
  depend on U) with the Cayley midpoint step, N = 4, 8, 16. The observed order must be
  between 1.8 and 2.2.
- **C-newton.** Every step converges within the timeout. The largest iteration count and
  the largest final ‖Δ‖/‖Ω‖ are reported.

## Gates (fixed now)

At n = 1 and n = 2, for the native and two-ring forces, against S3's RKMK4 runs (which
agree with their own step halving to 10⁻⁶ or better):
- **G-agree.** At the chosen N, |E_cay(t) − E_rk4(t)| < E_rk4(t)/20 at all four sample times.
- **G-time.** Halving h changes E_cay(0.02) by less than E/10.

N is the smallest of 4, 8, 16, 32 that passes both gates for both forces at n = 1 and at
n = 2. n = 3 uses that N, and G-time at n = 3 checks it there. If no N up to 32 passes, n = 3
is not run, and that is the result.

At n = 3, for the native, two-ring and frozen-metric runs, with final states saved:
- **G-traj** (S3's gate, judged again at the new finest level): two-ring R < 5%, and an
  observed order of E of at least 0.5 between n = 2 and n = 3.
- **G-time** at n = 3: halving h changes E by less than E/10 (two-ring only, for cost).
- **Frozen-metric control** (Amendment 1): Δ = ‖log(U_moving⁻¹ U_frozen)‖_B/√Vol, measured
  exactly from the saved states. Informative at n = 3 only if Δ > 2E_m. If informative:
  E_f ≥ Δ/2, and E_f does not converge between n = 2 and n = 3 (order below 0.25) while E_m
  does.
- **Native control** (Amendment 1, as written): crossover (E_pg < E_native) and stall
  (native order below 0.25), each reported.
- **S3a forecast, checked, not gated:** both trajectory errors keep falling between n = 2 and
  n = 3, and the two-ring order exceeds the native order.

## Not measured

n ≥ 4, longer windows, other metrics, the engine path (BW08).

## Amendment 1 (2026-11-18): the Newton tolerance was below the force's own precision

**Written after the first n = 1 attempt failed, and before any trajectory result existed.**
No gate, control or step-size rule changes; only the Newton tolerance does.

**What happened.** All four native n = 1 runs (N = 4, 8, 16, 32) stopped at step 1 with
"failed at step 1": the Newton increment fell about 100× per iteration down to about
10⁻⁹, then hovered at 2.6–4.0×10⁻¹⁰ for the rest of the 40 iterations, never reaching
10⁻¹⁰. The two-ring runs were stopped before they finished. The failed receipts are kept in
`research/su2-cayley-20261118/failed-tol-1e-10/`.

**Why (measured).** Scripts and output in `research/su2-cayley-20261118/diagnostics/`.
- The mass solve is not the cause: the iterative (CG) and a direct sparse solve give
  velocities that agree to 4.3×10⁻¹³, and Newton stalls at the same floor with either.
- The force evaluation itself is the floor. Nudging every link by a rounding-level
  amount (relative 2.2×10⁻¹⁶) moves the computed velocity by 3.3–3.9×10⁻¹⁰ and the
  gradient by 4.6–5.3×10⁻¹⁰, for both the native and two-ring forces at n = 1. Newton
  cannot converge below the noise of the function it solves, so 10⁻¹⁰ was unreachable.
  This was an error in the spec, not in the stepper.

**Change.** Newton converges when ‖Δ‖ ≤ 10⁻⁸ ‖Ω‖, about 30× above the measured floor. The
40-iteration timeout and the stop-on-failure rule are unchanged. A relative error of 10⁻⁸
in each step's Ω is far below every comparison the gates make (G-agree at E/20 with E of
order 10⁻²; G-time at E/10).

**Open, not addressed here (hypothesis, untested).** The gradient amplifies rounding by about
10⁶. The likely source is the face defect, computed as the holonomy minus one: the chord
reading U − 1 of a small turn, which loses digits in the same way as computing 1 − cos θ
directly. The usual cure is to form it from the vector part (the half-angle form
2 sin²(θ/2)). It would change the force in every SU(2) script at the 10⁻¹⁰ level, so it is
left for a separate, tested change.
