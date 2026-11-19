# S3a: why the native force does not stall in time (spec)

2026-11-18. Written before any measurement in this file. Research script
`scripts/ricci_su2_pg_flow_filter.py`, **not an engine run**. It follows S3
(`docs/RICCI_SU2_PG_FLOW_RESULTS.md`), where one amended prediction failed: the native
SU(2) Whitney force was expected to stall in time, as its instantaneous error stalls near
25% (S2a). Instead its trajectory error falls with observed order 1.63 between n = 1 and
n = 2.

## In plain English

A force can be 25% wrong at every instant and still leave the lattice close to the right
place, if the wrong part of the force pushes in directions the lattice snaps back from
almost immediately. Those are the fast, stiff directions: a push there is cancelled within
a tiny fraction of the time window, so it never builds up. Only the part of the error that
pushes in slow directions accumulates.

So the question is not "how wrong is the force" but "how wrong is the force in the slow
directions". This experiment splits each force's error into its slow and fast parts and
checks whether that split predicts the trajectory errors S3 measured.

## What is measured

At the exact links L(t) of the Ricci-flowed Berger metric, at t = 0, 0.005, 0.010, 0.015,
0.020, for the native and two-ring forces at n = 1 and n = 2:

- **The residual** r(t) = v_lat(L(t), g(t)) − v_exact(t): the lattice velocity at the exact
  links minus the exact links' own velocity (right-trivialised, central difference in t with
  ε = 10⁻⁶, as in S3's control C-exact).
- **Its size**, ρ(t) = ‖r‖_B / ‖v_exact‖_B. At t = 0 this is S2a's instantaneous error.
- **The linear filter.** With B = LLᵀ and Ĝ = L⁻¹KL⁻ᵀ the whitened linearised force of S2c
  (K cached from S2c, at t = 0), the error obeys, to first order,
  y′ = −Ĝ y + r̂(t), y(0) = 0, with y = Lᵀδ and r̂ = Lᵀr. The predicted error is
  E_pred(t) = ‖y(t)‖ / √Vol, with r̂ linear between the sample times, integrated exactly
  (augmented matrix exponential).
- **The undamped accumulation** E_naive(T) = ‖∫₀ᵀ r̂ dt‖ / √Vol (trapezoid), what the error
  would be if nothing snapped back. The **filter factor** is W = E_pred(T) / E_naive(T).
- **The slow share.** An orthonormal basis Q_s of the invariant subspace of Ĝ with
  Re λ < 1/T = 50 (sorted real Schur form). The slow share is
  f_slow = ‖Q_sᵀ r̂‖² / ‖r̂‖² at t = 0. For the non-normal two-ring operator this is an
  orthogonal projection onto an invariant subspace, so it is approximate; it is exact up to
  10⁻³ for native (η = 3×10⁻⁴).

## Controls (thresholds fixed now)

- **C-S2a.** ρ(0) reproduces S2a's relative error (native 26.25%, 24.74%; two-ring 36.25%,
  7.43% at n = 1, 2) to within 5% relative. S2a maps the continuum velocity to edges; this
  uses a time difference of exact links, so exact agreement is not expected.
- **C-eps.** v_exact with ε = 10⁻⁵ agrees with ε = 10⁻⁶ to 10⁻⁶ relative at t = 0.
- **C-K.** The cached K is the S2c matrix (its whitened left edge reproduces S2c's to 10⁻⁸).

## Gates (fixed now)

- **G-model.** The linear filter predicts the measured S3 error: |E_pred(t) − E(t)| / E(t)
  < 25% at all four sample times, for both forces at both levels. Where it fails, the
  explanation below is **not established at that level**, and that is reported.
- **H-wash (the explanation), judged at n = 2 only if G-model passes there.** The native
  residual sits mostly in fast directions and the two-ring residual does not, by a clear
  margin: f_slow(native) < 0.25, and W(native) < ½ W(two-ring).

## What it would mean

- If G-model and H-wash pass: the native force's 25% error is real but lives in stiff
  directions the dynamics filters out over this window. The failed stall prediction was a
  prediction about the wrong quantity. The native trajectory error is then set by its slow
  residual, and its order is reported as the forecast for n = 3. Not a gate.
- If G-model passes and H-wash fails: the linear picture holds, but the reason is something
  other than fast directions, and this file names what the numbers show instead.
- If G-model fails: nonlinearity, the moving metric in B, or the ∂B·B⁻¹∇ term this model
  omits matter at this window, and no explanation is claimed.

Not measured: n ≥ 3, longer windows, the engine path.
