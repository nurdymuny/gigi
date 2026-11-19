# B6: stability and numerical range of the SU(2) forces from the exact-derivative K (spec)

2026-11-19. Written before any B6 code ran. Research script `scripts/ricci_su2_exact_stability.py`
(new), **not an engine run**. Inputs: the B5 matrices in `outputs/su2-batched-K/` (receipts with
SHA-256 in `research/su2-batched-K-20261119/`). Receipts go to `research/su2-exact-stability-20261119/`.

## In plain English

The paper's stability results (S2b: no runaway modes) and its numerical-range results (S2c: no
hidden swell) carry three open points:

1. They were measured only at n ≤ 2.
2. The matrices behind them were built while the face-defect derivative was still a finite
   difference. That put errors of about 5 × 10⁻³ into each column, and the count of near-zero
   eigenvalues moved with the column step.
3. The finest level, n = 3, was never checked.

B5 now gives the same matrix with the exact derivative at n = 1, 2, 3 and 5. B6 repeats S2b and
S2c on those matrices:
- at n = 1, 2, to see whether anything changes;
- at n = 3, with the full dense method;
- at n = 5, with the one quantity that settles both questions at once, the left edge of the
  numerical range.

Every eigenvalue lies inside the numerical range. So if the left edge is above −10, there can be
no runaway eigenvalue below −10 either.

## Objects (unchanged from S2b/S2c)

- **K:** the B5 matrix at g(0) = (3, 1, 1), ε = 10⁻⁶, loaded from `.npz`. Its SHA-256 is checked
  against the kbuild receipt before use.
- **B:** the native SU(2) tangent mass, i.e. the same native blocks S2b used, from
  `output/ricci-shape-regular-20261117/berger/level{L}_berger_q4.output.json`.
- **The generalized spectrum** of (K, B) is the stability object (S2b).
- **Ĝ = L⁻¹KL⁻ᵀ**, with B = LLᵀ. Its numerical range is W(Ĝ).
- **Left edge:** min Re W(Ĝ) = λ_min(S), with S = ½(Ĝ + Ĝᵀ). Equivalently, it is the smallest
  generalized eigenvalue of (½(K + Kᵀ), B). That second form needs no dense whitening, and it is
  how n = 5 is computed.

## Methods by level

- **n = 1, 2 (426 and 3,372 unknowns): dense, exactly as S2c.**
  - `scipy.linalg.eig` for the spectrum.
  - `eigh` for S.
  - A 180-angle sweep for the numerical radius and the boundary polygon.
  - η and F1, and the step limits.
  - S2c's code path, with only K replaced.
- **n = 3 (11,358 unknowns): dense spectrum and dense left edge**, i.e. `eig` and `eigh(S)` on
  1 GB matrices. For the sweep, a full complex `eigh` at 180 angles is too costly, so the extreme
  eigenvalues of each H(θ) come from the generalized sparse problem with ARPACK `eigsh`
  (k = 1, `which='LA'`, M = B; tol 10⁻¹⁰; maxiter 10⁴).
  - Control C-sweep compares this with dense `eigh` at 6 of the angles.
  - η is computed dense.
- **n = 5 (52,530 unknowns): sparse only.** Write A = ½(K + Kᵀ).
  - **Certificate: no eigenvalue below σ (inertia).** A shift-invert solve finds the eigenvalues
    nearest the shift. It cannot show that none lie far below it. So the bound is certified
    separately, by Sylvester's law of inertia. A − σB is factorised as a sparse LU with a
    symmetric ordering and no pivoting (`splu`, `SymmetricMode=True`, `diag_pivot_thresh=0`).
    The number of negative pivots is then the number of generalized eigenvalues of (A, B)
    below σ. This is done at σ = −10, the gate line, and at σ = 0, the count of growing
    directions S2c reported.
  - **Value:** `eigsh` on (A, B) in shift-invert mode, σ = −20, k = 6, tol 10⁻¹⁰. Once the
    inertia at −20 is shown to be 0, these are the smallest eigenvalues, and the smallest of them
    is the left edge.
  - **Witness of failure (one ring).** A Rayleigh quotient xᵀAx / xᵀBx < −10 for any vector x
    proves the left edge is below −10. Here x comes from `eigsh(which='SA')`, which converges
    quickly when the left end is the extreme end of the spectrum. The witness is exact for the
    x it reports, whether or not the eigensolver converged.
  - If a sparse factorisation fails, for lack of memory or a zero pivot, the run is reported as
    failed. No other method is substituted without an amendment.
  - **Slowest eigenvalues:** the 20 eigenvalues of (K, B) nearest σ = −5, from ARPACK `eigs` in
    shift-invert mode. These are for comparison with the native force; they are not a gate.
  - The numerical radius, η and the full spectrum are **not computed** at n = 5.

## Gates (thresholds fixed now)

- **G-rough at n = 3** (as S2b). No eigenvalue of the two-ring operator has Re λ < −10.
- **G-W at n = 3 and n = 5** (as S2c). The two-ring left edge is at least −10, and at least twice
  the native left edge.
  - At n = 5 this is the stability gate too: every eigenvalue lies in W, so G-W implies G-rough.
- **Removal (one ring), n = 3 and n = 5.** The one-ring operator fails G-W. At n = 3 it also
  fails G-rough. If it passes, the gate is not measuring roughness, and that is reported.
- **G-eps: the caveat test, n = 1 and n = 2, all three forces.** K is rebuilt by the B5 code at
  ε = 10⁻⁵ and ε = 10⁻⁷, besides 10⁻⁶. The paper's finite-difference caveat is removed only if
  both of these hold at every ε:
  - The count of eigenvalues with modulus < 1 is identical.
  - The left edge agrees to 10⁻⁶ relative.

  If a count still moves, the receipt reports the eigenvalues nearest the unit circle. In that
  case the caveat is reworded, not dropped.

## Comparisons (reported, not gated)

- **Against S2b/S2c at n = 1, 2:**
  - The saved eigenvalues: worst distance to the nearest new eigenvalue, relative to max|λ|.
  - The left edge, slowest eigenvalue, numerical radius, η and F1, old against new.
  - The verdicts of G-rough and G-W, which are expected to be unchanged.
- **Small-modulus count:** two-ring against native at each level, as S2b reported it.

## Controls

- **C-hash.** Every K loaded has the SHA-256 in its kbuild receipt.
- **C-chol.** ‖LLᵀ − B‖/‖B‖ < 10⁻¹³ at n ≤ 3 (as S2c).
- **C-W.** As S2c, at n ≤ 3:
  - Boundary points lie inside every supporting line.
  - Every eigenvalue has Re λ ≥ left edge − 10⁻⁹‖Ĝ‖.
- **C-sparse, n = 3.** The whole n = 5 method is run at n = 3, for all three forces.
  - The inertia counts at σ = −10 and σ = 0 equal the dense counts of eigenvalues of S below σ,
    exactly.
  - For native and two-ring, the shift-invert left edge equals the dense λ_min(S) to 10⁻⁸
    relative.
  - For one ring, the witness is below −10.

  This is what licenses the n = 5 numbers.
- **C-sweep, n = 3.** At 6 of the 180 angles, the sparse extreme eigenvalues of H(θ) equal the
  dense ones to 10⁻⁸ relative.
- **C-resid, n = 5.** Every returned eigenpair has a relative residual ‖Ax − μBx‖/(‖A‖‖x‖_B)
  below 10⁻⁸.

## Cost (honest, measured, not gated)

- **n = 3:** dense `eig` of a real 11,358 × 11,358 matrix and `eigh` of S, a few GB of RAM.
- **n = 5:** one sparse LU of a symmetric matrix of 52,530 unknowns. The two-ring K has 14.1
  million nonzeros, so fill-in is the risk. This machine has 63.6 GB.

Seconds are recorded for every stage.

## What this is meant to close

- Paper Limitation: "its stability and numerical range are measured at n ≤ 2."
- Paper caveat, S2b: "these operators were built while the defect derivative was still a finite
  difference…"

Either can close only if its gate passes. A failure is reported as a finding against the force or
against the earlier result, as it falls.

## Not done here

Other metrics. The nonlinear flow. The numerical radius and η at n = 5. Time integration at n = 5
(C9).
