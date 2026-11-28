# H8 and the switch at small coupling: a deduction

27 November 2026. **Status: conditional deduction.** Two named inputs: one cited semiclassical theorem, and one
property of the sourced potential. Nothing is computed here. This is a reduced-model statement; the physical room
still needs H2b.

## Statement

Fix t = 3. Write H(s) for the undressed reduced operator, −½Δ + V_s on colour-invariant functions on ℝ⁹ (the
natural realization). Write E_{n,m}(s, g) for its n-th eigenvalue in circle charge m (n = 0, 1, …).

**Claim.** Assume the two inputs below. Then there is g₀ > 0 such that, for every 0 < g < g₀:
1. **(H8, small coupling)** at s = 1, E_{1,0} < inf_{m≠0} E_{0,m}: the lowest excitation has charge 0;
2. **(a switch exists)** there is s* ∈ (1, 1.15) at which the second charge-0 level and the lowest charge-±2 level
   are exactly degenerate, with the order reversed on either side of it.

Here E_{1,0} is the second charge-0 eigenvalue: the first excitation above the charge-0 ground state.

## Inputs

**(S) Semiclassical harmonic limit, per symmetry sector.** Put q = p/g, so H = g⁻²(−½ħ²Δ_p + V¹_s(p)) with
ħ = g², and V¹_s does not depend on g. Suppose V¹_s − min V¹_s ≥ 0 vanishes exactly on finitely many group orbits,
each non-degenerate transversally, and grows at infinity. Then, for each fixed n and m, E_{n,m} − g⁻² min V¹_s tends
as g → 0 to the n-th element of the union, over the minimizing orbits, of the transverse harmonic levels in charge m,
counted with multiplicity, in the colour-invariant sector.
- For an isolated non-degenerate minimum this is Simon (1983).
- For an orbit, it is the same IMS localization argument in slice coordinates, where the orbit-volume weight is
  smooth and positive near full rank (H8 note, Step 5).
- I use (S) as a cited input and do not re-prove the orbit case here.

**(M) The minimum sets.**
- At s = 1 the zeros of V⁰ are exactly {0} ∪ q_MC·SO(3) (Theorem 8). Both are non-degenerate transversally (H8
  note, Steps 2 and 4). Proved.
- At s = 1.15 the minimum set of V¹ is the background orbit q_bg·SO(3), non-degenerate transversally. Evidence: the
  proved outside bound of S_C (report, page 15) gives inf V ≥ ρ_C off a tube around the orbit at the controls. Since
  V = g⁻²V¹(gq) with V¹ independent of g, this places the global minimum of V¹ inside that tube. I take (M) at
  s = 1.15 as an input.

## Proof

**1. The limits at s = 1** (two wells: the origin, and the orbit of q_MC). From the H8 note, Steps 2–4, every value
an exact harmonic number:
- origin well: ground at 3S/2; charge-0 two-quantum level at 3S/2 + 2/√t; charge-±2 pair at 3S/2 + 2/√t; charge ±1
  at 3S/2 + √t + 1/√t;
- q_MC well: its ground state (charge 0) is offset by exactly ω_C = ½(√(t+8) − √t) (Step 4). Its excitations add
  ω_C (charge 0), 2/√t (charge ±2) or √t + 1/√t (charge ±1).

So, as g → 0:
- E_{1,0} − E_{0,0} → ω_C = 0.79229 (t = 3);
- inf_{m≠0} E_{0,m} − E_{0,0} → min(2/√t, √t + 1/√t) = 2/√3 = 1.15470.

The two limits differ by 0.36241 > 0. Hence (1) holds for all g below some g₁.

**2. The limits at s = 1.15** (one well: the background orbit, by (M)). The transverse frequencies are
- the competitor ω_C(1.15) = 1.106657 (charge 0);
- Helike ω_H(1.15) = 1.090107 (charge ±2);
- the breathing partner 2.677744 (charge 0);
- the pair 2.429678 (charge ±1).

These are reproduced in `research/lowest-note-threshold-20261127/receipt.json`. So E_{1,0} − E_{0,0} → 1.106657 and
inf_{m≠0} E_{0,m} − E_{0,0} → 1.090107, a separation of +0.016550 > 0. Hence, for all g below some g₂, the lowest
charge-±2 level lies below the second charge-0 level at s = 1.15.

**3. The switch.** Fix g < g₀ = min(g₁, g₂). Line 19 makes H(s) a type-(A) analytic family on [1, 1.15], inside
Theorem A's window, so the ordered eigenvalues E_{1,0}(s) and E_{0,±2}(s) are continuous in s. The difference
E_{1,0} − E_{0,±2} is negative at s = 1 (step 1) and positive at s = 1.15 (step 2). By the intermediate value theorem
it vanishes at some s* ∈ (1, 1.15). The two levels lie in different charge sectors, which H(s) preserves, so the
degeneracy is an exact crossing (line 17). ∎

## Remarks
- **Where the switch sits.** The harmonic limits cross at s₀* = 1.14357, transversally (d(ω_C − ω_H)/ds = 2.569). By
  (S) applied on compact s-intervals away from s = 1, every switch at small g lies near s₀*. Uniqueness of the switch
  would need uniform control, and is not claimed.
- **Why s = 1 is not a continuity problem.** The second charge-0 level changes identity just above s = 1 (the two
  wells' grounds avoid each other near ε* ≈ 0.95g²). The argument uses only the eigenvalues at the two endpoints at
  fixed g, together with continuity in s. Identities are never followed.
- **Agreement with the origin splitting.** At the origin well, the charge-±2 pair is exact at 2/√t, and the charge-0
  two-quantum level moves by −2κ(1 + 1/t)g² (`h8-origin-splitting-proof-2026-11-27.md`, independently re-derived in
  `scripts/check_h8_origin_fock.py`). Both origin charge-0 candidates therefore stay below the pair through O(g²).
- **What this does not give.**
  - H8 at the registered couplings, which needs explicit g₀ or enclosures.
  - Physical H8, which needs H2b.
  - Line 12 at finite g, which still needs H1.
