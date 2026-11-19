# From the Windows machine: what is done, what is running, who has what (19 November 2026, evening)

A handoff so the two machines don't duplicate work. Branches as named; every number cites a receipt.

## Who has what

| work | machine | branch | state |
| --- | --- | --- | --- |
| **V2: all three bases (linear, quadratic-aware, hybrid), each with its removal arm** | Windows | `feat/variational-force` | n = 1 running; n = 2 next |
| S2d (n = 3 spectrum), S3c (long window) | Mac | their branches | finishing |
| B6 (exact-derivative stability, n = 5) | Windows | `feat/reading-the-curve` | background only; nothing new will be added |

**Please don't start V2 runs on the Mac:** they are running here. The script is
`scripts/ricci_variational_v2.py` (ba1e028a). One process runs every requested arm, sharing the
cells and the per-edge geometry. Receipts will go to `research/variational-v2-20261119/`.

## V2 so far (n = 1, the controls)

- **C-repro passes.** At two rings with C = 0 and the linear base, the script gives a variational
  force of 45.405% and a PG force of 6.223%, against the paper's 45.4% and 6.22%
  (`L0_rings2_c0.json`). The quadratic-aware base at two rings gives 614%, matching the base probe.
- **The C = 0 three-ring numbers match the hybrid note's probe exactly:**

  | base | variational | reconstruction | test variation | PG |
  | --- | --- | --- | --- | --- |
  | linear | 39.0% | 1.49% | 37.6% | 4.02% |
  | quadratic-aware | 53.6% | 6.87% | 48.2% | 4.83% |
  | hybrid | 60.7% | 4.26% | 61.0% | 5.47% |

- **C-copies passes at n = 1:** no cell's three-ring neighbourhood sees the same edge twice, even
  though three rings cover a quarter of the 120-cell quotient. So n = 1 is admissible, and the gates
  are judged at both levels as the spec says.
- **Solve.** lsqr from zero, stopping at a residual of 10⁻⁹, with V1's dense eigh of AAᵀ as the
  cross-check. Both residuals, both iteration counts and the difference between the two solutions go
  in each receipt.

## B6: answered, and it agrees with S2d

Receipts: `research/su2-exact-stability-20261119/` on `feat/reading-the-curve`.

- **The paper's finite-difference caveat can come out (G-eps passes, every force, n = 1 and 2).**
  K was rebuilt with the exact defect derivative at column steps 10⁻⁵, 10⁻⁶ and 10⁻⁷:
  - The near-zero eigenvalue counts are identical: 62 / 62 / 62 at n = 1, and 483 / 481 / 483
    (native / two-ring / one-ring) at n = 2.
  - The left edge agrees to about 10⁻¹⁰ relative.
  - The nearest eigenvalue to the unit circle sits at least 0.009 away, so the counts are not
    marginal.

  The movement the paper describes came from the old finite-difference defect derivative, not from
  the operators.
- **Old matrices against new, n ≤ 2.** Every eigenvalue moves by at most 9 × 10⁻⁶ relative, and every
  count and verdict is unchanged. The published S2b/S2c numbers stand.
- **n = 3, a cross-check between the two pipelines.**
  - B6 builds K by batched assembly and certifies the left edge by sparse inertia; S2d uses the
    loop-built K and a dense eigensolver.
  - Native left edge: B6 −4.6762861, against S2d's −4.676286084. The count of growing directions is
    774 in both.
  - Two-ring left edge: −4.66561 (B6), against −4.666 (S2d). Below −10 the count is 0, certified by
    inertia, so G-W passes independently of S2d.
  - One ring at n = 3: 4,320 eigenvalues below −10, with a witness direction at −45,922. The removal
    fails, as required.
- **Still running in the background:** dense n = 3, which checks the sparse method directly, then the
  n = 5 left edge.

**For *Reading the Curve* v3.** With S2d and B6, the limitation "stability and numerical range at
n ≤ 2" becomes n ≤ 3, and n = 5 is pending. The finite-difference caveat in the S2b paragraph can be
replaced by the G-eps result. This is to be folded in only when B6's results doc is written; I will
write it once n = 5 lands.
