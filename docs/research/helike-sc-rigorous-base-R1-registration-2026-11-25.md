# S_C, rigorous build, stage R1: the constants of the inside model and the enclosed base spectrum

25 November 2026. **REGISTRATION before implementation.** This follows the floating-point PASS of gate G-base in
[the Lehmann–Goerisch registration, §13.5](helike-sc-lehmann-goerisch-registration-2026-11-25.md). Stage R1
replaces every floating-point constant of the inside model and of its explicit base with an Arb enclosure. Nothing
here has been computed.

All arithmetic is Arb (python-flint) at 128 bits, guarded by `checked()` as in `scripts/helike_cutoff_bounds.py`.
NumPy is used only to compare with the floating-point run.

## R1.1 The exact transverse frame and the polynomial P

At the background q_b = diag(q₁, q₂, q₃), with q₁ = s/(g√t) and q₂ = q₃ = s/g, the Hessian splits into blocks.
The floating-point frame (§8) is, exactly:

| mode | block | normal vector |
| --- | --- | --- |
| soft 1 | diagonal | (e₂₂ − e₃₃)/√2 |
| soft 2 | pair (2,3) | (e₂₃ + e₃₂)/√2, the normal (q₃, q₂)/‖·‖ of the gauge tangent (q₂, −q₃) |
| soft 3, stiff 3 | diagonal | the two eigenvectors of the 2×2 Hessian on span{e₁₁, (e₂₂ + e₃₃)/√2} |
| stiff 1, 2 | pairs (1,2), (1,3) | (q₂ e₁₂ + q₁ e₂₁)/‖·‖ and (q₃ e₁₃ + q₁ e₃₁)/‖·‖ |

Each of these is an exact formula, enclosed in Arb. The gauge zero modes are the tangents q_b G_k.

**Checks.**
- The enclosed frame is orthonormal, and orthogonal to the three gauge tangents. Every inner product must contain its
  exact value.
- The enclosed Hessian eigenvalues contain the floating w² of §10.1.

**P(z) = V(x₀ + Nz) − V(x₀)** is expanded symbolically from Vq, with Arb coefficients. Its parts are checked against
the floating least-squares fit, to 10⁻¹⁰:
- the linear part must contain 0;
- the quadratic part must be ½Σw²_k z_k²;
- the quartic part is identically (g²/2)|cof(Nz)|², which is ≥ 0 by algebra, with no computation needed.

## R1.2 C₃, rigorous

C₃ ≥ max over the unit sphere of |P₃(y)|.
- The sphere is covered by the 12 faces of [−1, 1]⁶. On each face, P₃(x/|x|) = P₃(x)/|x|³.
- Branch and bound proceeds over 5-cubes with interval (centred-form) evaluation, until the upper bound is within
  10⁻³ of the best sampled value.

C₃ affects only the base (ω²_red) and D ≥ 0, not H₁, so tightness matters only for the base's level count.

**Consequence: D ≥ 0 on the tube, rigorous.**

    D ≥ ½(w²_s,min − ω²_s,red)|u|² + ½(w²_t,min − ω²_t,red)|v|² − C₃|z|³ = C₃|z|²(L − |z|) ≥ 0

for |z| ≤ L = √(L_u² + L_v²), using P₄ ≥ 0.

## R1.3 v_J and the Robin coefficients, rigorous, on the product tube

J(z) = det[N | qG₁ | qG₂ | qG₃] with q = q₀ + Nz. J is a cubic polynomial in z. It is expanded exactly.

- **v_J (lower bound).** Since V_J = ΔJ/(4J) − |∇J|²/(8J²), the bound is
  v_J ≤ min over B_u(5) × B_v(2) of V_J.
  It is found by branch and bound on 6-boxes:
  - boxes are discarded when they lie outside the product of balls;
  - J > 0 is proved on every kept box;
  - each box is evaluated in centred form.
- **Robin coefficients (upper bounds), one per boundary face.** The floating script used the radial derivative on the
  6-sphere of radius L. The product tube's boundary has two faces, with normals û and v̂.
  - **β_u** ≥ max |∂_û J|/(2J) over |u| = 5, |v| ≤ 2.
  - **β_v** ≥ max |∂_v̂ J|/(2J) over |u| ≤ 5, |v| = 2.

  Each block's base uses its own coefficient. **This corrects the floating model**, which used one β on both blocks.

**Tightness gates.** v_J and β enter H₁ itself, not only the base, so a loose bound eats the 0.024 margin.
- The v_J lower bound must lie within 0.003 of the sampled minimum.
- The effect of β_u and β_v on λ₁₁(H₁) is estimated in floating point, by the §13 pipeline, at the enclosed values. It
  must be at most 0.003.
- If either gate fails, the subdivision is refined. If refinement cannot meet a gate, R1 reports which constant fails
  and stops.

## R1.4 The enclosed base spectrum

Each block's base is −½Δ + ½ω²r² + c on the 3-ball of radius L_b, with the Robin condition R′(L_b) = β_b R(L_b).
The radial solutions are separable: R_l(r) = r^l e^{−ωr²/2} M(a, l + 3/2, ωr²), with a = (l + 3/2)/2 − (E − c)/(2ω)
and M the Kummer function (Arb `hypgeom_1f1`).

The eigenvalues are the roots in E of F_l(E) = R_l′(L_b) − β_b R_l(L_b).
- **Isolation.** Each root is enclosed by a sign change on an Arb interval, then refined by interval Newton. Its
  uniqueness in the enclosure is proved by the interval derivative excluding 0.
- **Completeness.** Below the cut, for each l, the number of roots must equal the Sturm count: the number of zeros of
  R_l(·, E_cut) on (0, L_b), counted by sign changes on a certified grid with interval exclusion between grid points.
  The Robin condition shifts the count by at most one at the cut. That is handled by checking the sign of F_l at
  E_cut.
- **Range of l.** An l is excluded once its lowest root is proved above the cut, by l(l + 1)/(2L_b²) + min W > cut.
- **Product base.** The product spectrum, with multiplicity 2l + 1 per block, is then exact up to the cut, inside
  enclosures.

**Checks.**
- Every enclosed level contains the floating Galerkin level of §9, to 10⁻³.
- The enclosed count of levels below ρ_b reproduces 87 and 32.
- The enclosure of the base level used as the homotopy's first ρ must be narrow: width at most 10⁻¹².

## R1.5 Tests and mutants (before the code)

RED tests come first. Then a `check_helike_sc_r1_mutations.py` must reject each of these mutants:
- drop the √2 normalisation in the frame;
- swap a soft and a stiff mode;
- use the 6-ball β instead of the face-wise β;
- drop the |∇J|² term from V_J;
- skip the J > 0 proof on a box;
- accept a root without the derivative-exclusion check;
- skip the Sturm completeness count;
- truncate the l-range early.

## R1.6 Outcome

- **R1 COMPLETE** per control if all of R1.1–R1.4 produce enclosures that pass their checks and tightness gates.
- **Otherwise R1 INCOMPLETE**, naming the failing item.

R1 proves no eigenvalue bound for H. Its outputs are the inputs of R2: interval A₀, A₁ and A₂ for the homotopy steps,
and the verified pencil.
