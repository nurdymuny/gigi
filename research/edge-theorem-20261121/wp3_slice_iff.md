# WP3: the "iff" at the edge on a slice (21 November 2026)

- Script: `scripts/edge_wp3_slice_iff.py`. Runs in about 25 s.
- Receipt: `research/edge-theorem-20261121/wp3_slice_iff.json`. Log: `wp3_slice_iff_log.txt`.
- Slices: t = 3, with y = √3 and with y = 3/2. Operator: H + GG*, in the charge blocks of sectors j ≤ 12.
- Every number below comes from the receipt.

## Result

**PROVED (exact algebra plus rigorous interval arithmetic), for sectors j ≤ 12 on each slice.** Let x* be the unique
real root of P₂(x, y, √3) in [−3, 1]. Then:

| | y = √3 | y = 3/2 |
| --- | --- | --- |
| x* (isolating interval with rational endpoints, width < 4·10⁻³¹) | −0.3821684822419167 | −0.37158172515020627 |
| next real root of det(B₄,₀ − λ₄), r_next (a root of P₁) | −0.0650646510788456 | −0.008268340671784679 |
| x* minus edge_A in exact_edge.json (COMPUTED) | 9.4·10⁻¹⁶ | 2.4·10⁻¹³ |

1. m(x) = λ_min(B₄,₀(x)) − λ₄(x) is > 0 on [−3, x*), is 0 at x*, and is < 0 on (x*, r_next).
2. The zero at x* is transversal: x* is a simple zero of det(B₄,₀ − λ₄ I), and exactly one eigen-branch crosses λ₄,
   with a sign change.
3. Every other non-twist block (j, q) with j ≤ 12 satisfies λ_min(B_{j,q}) > λ_j on [−3, x* + 0.02]. That is 116
   blocks per slice.

**Consequence (PROVED, sectors j ≤ 12 only).** On [−3, r_next), the twist is the floor of every sector j ≤ 12 **iff
x ≤ x\***.
- "If": items 1 and 3.
- "Only if": for x in (x*, r_next), the (4, 0) block alone undercuts λ₄.
- At x = x*, λ₄ ties with one eigenvalue of B₄,₀.

## How each step is proved

- **E1. The blocks, exactly.**
  - The blocks are built exactly in sympy on the slice, with t = 3 and y substituted exactly.
  - B₄,₀ is exactly real symmetric.
  - For every j ≤ 12, both 1×1 blocks of charge ±(l + 2) equal λ_j exactly. So the twist value has a conjugate twin.
    Like exact_edge.py, I exclude both.
  - The exact blocks agree with `EE.Blocks` to 2.8·10⁻¹⁴ (COMPUTED).
- **E2. The determinant, exactly.** D(x) = det(B₄,₀ − λ₄ I) is computed over ℚ(√2, √3)[x].
  - Exact check: 3⁹·D = P(x, y, √3), where P is the full polynomial in `edge_polynomial.srepr`.
  - Exact factorisation: factor_list gives P = −16384·P₁·P₂, with degrees (5, 6, 12) and (7, 8, 16). Both factors are
    even in y.
- **E3. Root isolation, exactly.**
  - Write P_i(x, y, √3) = A + √3·B with A, B ∈ ℚ[x], and take the norm N = A² − 3B² ∈ ℚ[x].
  - Both norms are squarefree, so every real root of P₁ or P₂ is simple.
  - sympy isolates the real roots of N with rational endpoints. A root belongs to P_i iff P_i changes sign across its
    interval, and the sign of a + b√3 is decided exactly.
  - In [−3, 1]: P₂ has one root (x*); P₁ has two roots (r_next, and one more, near +0.19 or +0.13). The isolating
    intervals are disjoint.
  - Transversality, by interval enclosure on x* ± 10⁻⁶:
    - ∂ₓP₂ lies in [1.1032·10⁷, 1.1032·10⁷] (y = √3) and in [8.3613·10⁶, 8.3613·10⁶] (y = 3/2);
    - P₁ lies in [23434.09, 23434.34] and in [20392.46, 20392.68]. Both exclude 0.
- **E4. The (4, 0) block at x = −3.** B₄,₀(−3) − λ₄(−3) passes interval Cholesky, so it is positive definite.
  - Argument: D = ∏(μ_i − λ₄) over analytic eigen-branches μ_i (Rellich). D has no zero on [−3, x*), so all branches
    stay above λ₄ there.
  - At x*, D has a simple zero, so exactly one branch has a simple zero and the others are nonzero. That branch goes
    negative on (x*, r_next).
  - An independent interval-Cholesky cover of [−3, x* − 0.02] also passes.
  - The double-precision m(x* + 10⁻³) is −0.0110 (y = √3) and −0.0110 (y = 3/2) (COMPUTED).
- **E5. All other blocks.**
  - For each block, interval Cholesky of B_{j,q}(X) − λ_j(X) I passes on a cover of [−3, x* + 0.02] by sub-intervals.
    Entries are polynomials in x, with coefficients enclosed by iv.sqrt from exact sympy expressions, at 96-bit
    precision.
  - By Alefeld–Mayer, if interval Cholesky succeeds, every symmetric matrix in the interval hull is positive definite.
  - At most 14 pieces and bisection depth 2 were needed.
  - The smallest certified pivot lower bound belongs to block (1, 3/2): 0.0038 (y = √3) and 0.027 (y = 3/2). That block
    is the next competitor just above the edge.

## What is NOT proved here

- **Sectors j > 12 on this slice.** The large-j proof (family_large_j.json) covers only x ∈ [−3, −1.5]. Near the edge,
  plain Gershgorin needs l ≈ 640 (exact-edge doc), so on these slices the all-sector "iff" is open for
  x ∈ (−1.5, x*] and j > 12.
- **The rigour chain assumes** that the closed-form blocks are the charge blocks of H + GG*. That was checked
  numerically in earlier receipts and against EE.Blocks here. I did not re-derive it symbolically in this WP.
- **The gauge-orthogonal edge.** Not treated. Its edge on these slices is 0.027 (y = √3) and 0.020 (y = 3/2) higher (exact_edge.json:
  edge_S −0.3552 and −0.3516).

## What a 2-D (t, y) region needs (CONJECTURE / plan, not done)

1. **Root structure of P₂ in x over the region.**
   - The leading x-coefficient of P₂ must not vanish.
   - Disc_x(P₂) ≠ 0 and Res_x(P₁, P₂) ≠ 0 on the region. Prove this by interval evaluation or exact sign conditions on
     the region's boxes.
   - Then the real roots of D move continuously without colliding, and x*(t, y) is a single analytic branch.
   - Isolating it at one point, as here, fixes it everywhere in the region.
2. **(4, 0) block positive definite** at one point x₀ < x* of each connected component of {x < x*}. Rellich then gives
   m > 0 below the surface. Transversality across the whole surface follows from item 1, because a simple root of D
   persists.
3. **Other blocks.** A 3-D interval-Cholesky cover of {(x, y, t): −3 ≤ x ≤ x*(t, y) + δ}. The existing
   `ricci_su2_family_block_cert.py` does this on boxes. Near the edge the binding blocks are (1, ±3/2), (2, 0) and
   (2, 1), whose margins are small (pivots 0.004 to 0.04 on these slices). At small y the competitor changes to j = 2
   or j = 3 blocks, which needs their own polynomials.
4. **Sectors 12 < j < 80 near the edge** need block certificates, and **j ≥ 80** needs a Gershgorin bound that reaches
   the edge (l ≈ 640, or a sharper bound).
