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

## R1.7 Results so far, and amendments (written after the runs)

**Amendments, disclosed.**
- **C₃ tightness gate replaced.** C₃ enters only the base (it lowers the reduced ω²). The R1.2 tightness gate is
  replaced by the test that matters: rerun the §13 floating-point VSCF homotopy with every enclosed constant
  substituted (C₃ upper bound, v_J lower bound, face-wise β upper bounds, each block using its own face's β).
  Driver: `scripts/vscf_with_r1_constants.py`.
- **Branch-and-bound budgets.** C₃, v_J and β return the heap-top bound when their box budget is reached. That bound
  is still a proven bound, only looser; the logs mark it `[box budget reached]` (or a negative box count).
- **Root isolation (R1.4), as implemented.** The count below E is the Prüfer count, #eig < E = Z(E) + [R′(L)/R(L) < β],
  taken once per l at the cut. Roots are bracketed by sign changes of F_l on a grid; the number of brackets must equal
  the count, and each bracket is refined by sign bisection to 10⁻¹³. This replaces interval Newton with derivative
  exclusion; uniqueness per bracket follows from the bracket count matching the Prüfer count.
- **Face-wise β.** The single 6-sphere β of the floating model is replaced by β_u on the soft 3-ball face and β_v on
  the stiff one (R1.3 already registered this; the floating §13 runs used the 6-sphere value).
- **KMAX.** At g² = 0.004 the enclosed base has about 180 levels below ρ_b, more than the 200 Ritz levels the
  planner tracked leave room for. KMAX was raised to 600 for that control.

**Enclosures (128-bit Arb).** Logs in `research/sc-r1-20261125/`.

| | g² = 0.002 | g² = 0.004 |
|---|---|---|
| R1.1 frame and P | all 7 checks PASS (coefficient radius 5.5·10⁻³⁵) | same |
| C₃ ≤ | 0.061602 (sampled 0.051817; budget reached) | 0.087118 (sampled 0.073280; budget reached) |
| v_J ≥ | −0.000939 | −0.001616 |
| β_u ≤ | 0.046285 (budget reached) | 0.071876 (budget reached) |
| β_v ≤ | 0.003756 | 0.005457 |
| J(0) vs floating det | rel. diff 1.6·10⁻¹⁵ | 1.7·10⁻¹⁵ |

R1.4 check at the floating constants: the soft block has 39 radial levels below 8 (l < 13) and the stiff block has 4
(l < 3). These match floating Galerkin to 8.6·10⁻⁷ (soft) and 2.2·10⁻¹² (stiff), with enclosure radius ≤ 4.6·10⁻¹⁴.

**Homotopy with the enclosed constants (floating point, not a bound).** Logs in
`research/sc-r1-constants-homotopy-20261125/`.
- **g² = 0.002: PASS.**
  - LOSS 0.01: real ℓ₁₁ − ρ_b = +0.0101 after 5 steps.
  - LOSS 0.03: real ℓ₁₁ − ρ_b = +0.0264 after 5 steps (N 249 → 47 → 23 → 13 → 12).
  - Residuals ≤ 1.5·10⁻² throughout.
  - LOSS 0.06 finds no plan.
- **g² = 0.004: FAIL.**
  - The enclosed base starts ℓ₁₁ at 5.51 against ρ_b = 7.73, a gap of 2.2; the gap at g² = 0.002 is 0.9.
  - LOSS 0.01 plans 24 steps. After 6 real steps (s = 0.061, N 596 → 391) the residual has grown to 3.6·10⁻² and no
    plan remains.
  - LOSS 0.03 and 0.06 find no plan from the start.
  - The §13 floating PASS at this control (+0.024) used the sampled C₃ = 0.0733. The enclosure is 19 % looser and
    stops at its box budget, so it is the first suspect.

**Isolation run (g² = 0.004).** I reran with every enclosed constant except C₃, which is set to the sampled value. The driver takes max(passed, model-sampled), so the run used C₃ = 0.07496.
- LOSS 0.01: **PASS**, real ℓ₁₁ − ρ_b = +0.0007 after 7 steps.
- LOSS 0.03: **PASS**, real ℓ₁₁ − ρ_b = +0.0244 after 9 steps.
- LOSS 0.06: no plan.

So the g² = 0.004 failure is caused by the looseness of the C₃ enclosure. v_J and the face-wise β cost nothing decisive. A rigorous C₃ near 0.075 would pass in floating point. Log: `stdout_g0.004_c3sampled.txt`.

**Status.** R1 is INCOMPLETE at both controls.
- R1.4 has not yet been run at the enclosed constants (reduced ω² = w2min − 2·C₃·√29, per-face β, v_J shift).
- The R1.5 tests and mutants are not yet written.
- g² = 0.004 additionally fails the replacement gate.

The next lever for g² = 0.004 is a tighter C₃ bound (more boxes, or a sharper box bound) before anything else.

## R1.8 Overnight, 25–26 November: C₃ tightened by symmetry; R1.4 at the enclosed constants; a ray-aware fold

Written as the work went, before the folded homotopy results. Every change is listed with its reason.

### R1.8.1 C₃ by symmetry reduction (rigorous)

**Why the old bound stopped.** P₃ is invariant under the charge rotation. The maximum of |P₃| on S⁵ is therefore
attained on whole circles, and branch and bound cannot close on a circle in five dimensions. It ran out of boxes at
both controls.

**The reduction.** The argument is exact and by hand, because the frame is explicit.
- Conjugation q ↦ LqLᵀ with L = diag(1, R_θ) fixes V and q_b: rows 2 and 3 share ω and src, and
  cof(LqLᵀ) = L cof(q) Lᵀ.
- In the frame it rotates (z₀, z₁) by 2θ (soft1 = (e₂₂ − e₃₃)/√2, soft2 = (e₂₃ + e₃₂)/√2) and (z₃, z₄) by θ
  (stiff1 and stiff2, with q₂ = q₃). It fixes z₂ and z₅.
- Conjugation by diag(1, 1, −1) fixes V and q_b, and flips z₁ and z₄.

So max over S⁵ of |P₃| equals the maximum over {z₁ = 0, z₀ ≥ 0, z₃ ≥ 0, z₄ ≥ 0} ∩ S⁵. That set is 7 faces of
dimension 4 (`c3_symmetric_faces`, `c3_bound_sym` in `scripts/helike_sc_r1.py`; the box bound is the same as
before). A floating check confirmed the invariance to 9 × 10⁻¹⁶, and the sampled maximum is the same over S⁵ and
over the reduced set.

| | g² = 0.002 | g² = 0.004 |
|---|---|---|
| C₃ ≤ (old, budget reached) | 0.061602 | 0.087118 |
| **C₃ ≤ (symmetry-reduced, converged)** | **0.053838** (10,829 boxes, 15 s) | **0.075753** (11,268 boxes, 16 s) |
| best sampled | 0.052854 | 0.074885 |

Log: `research/sc-r1-20261125/r1_2_c3_sym.txt`. The registered tolerance (within 10⁻³ of the sampled value) is now
met, so the original R1.2 tightness gate passes at both controls.

### R1.8.2 Homotopy with the tightened C₃ (the R1.7 replacement gate)

This uses `scripts/vscf_with_r1_constants.py`, with all other constants as in R1.7. Logs are in
`research/sc-r1-c3sym-homotopy-20261125/`.
- **g² = 0.002: PASS.**
  - LOSS 0.01: +0.0099 after 4 steps.
  - LOSS 0.03: **+0.0298** after 4 steps; the floating §13 value was +0.0299.
  - LOSS 0.06: no plan, as before.
- **g² = 0.004: PASS at LOSS 0.01 by +0.0003** after 8 steps (N 294 → 186 → 150 → 86 → 47 → 23 → 13 → 12). That is
  far too thin to survive interval widening. LOSS 0.03 was still running when this was written.

### R1.8.3 R1.4 at the enclosed constants (status item 1 of R1.7: done)

This uses `scripts/helike_sc_r1_base_enclosed.py`. Every constant is rounded so the base stays below the true
operator: ω²_red is the lower endpoint of w²_min − 2C₃√29, β is the upper bound, and v_J is the lower bound.

| | g² = 0.002 | g² = 0.004 |
|---|---|---|
| product levels below ρ_b (none ambiguous) | 38 | 92 (180 with the old C₃) |
| base λ₁ / λ₁₁ | 4.6114 / 6.8297 | 4.2753 / 6.0924 |
| floating Galerkin count, max diff (first 30) | 38, 6.5·10⁻¹⁰ | 92, 1.2·10⁻⁹ |
| max enclosure radius | 9.1·10⁻¹⁴ | 9.1·10⁻¹⁴ |

Logs: `research/sc-r1-20261125/r1_4_enclosed_g0.00{2,4}.txt`.

### R1.8.4 A ray-aware fold of the base (new; rigorous check built, homotopy pending)

**The picture.** The base is a folded-down copy of the valley. The uniform rule lowers ω² by 2C₃√(L_u² + L_v²) in
every direction, as if every ray ran 5.4 units out against the worst cubic tilt. The geometry says otherwise:
- the box is only 2 wide in the stiff directions, so a ray with a stiff component exits early;
- the cubic is weak in pure-soft directions (about 0.037) and strong only in the mixed and stiff ones, which are
  the steep ones;
- the quartic is non-negative and unfolds the tilt at the walls.

So fold each ray only by what it pays before it leaves the box. The base W₀ = ½(w²_s,min − a)|u|² +
½(w²_t,min − b)|v|² + v_J is valid if and only if D = P − W₀ ≥ 0 on the tube. Along z = s·x, with x on a max-norm
face, that is

    Q(x) + s P₃(x) + s² P₄(x) ≥ 0  for 0 < s ≤ min(L_u/|x_u|, L_v/|x_v|),   Q = P₂ − W₀ (diagonal, ≥ 0).

**Rigorous check** (`fold_check` in `scripts/helike_sc_r1.py`). Per box of x it uses enclosures of Q, P₃ and P₄
(natural and mean-value forms intersected), an upper bound S on the exit parameter, and the exact minimum over
s ∈ [0, S] of q_lo + s·p3_lo + s²·max(p4_lo, 0). The symmetry reduction of R1.8.1 applies, because W₀ and the tube
are block-radial. The same D ≥ 0 is what makes the homotopy H_s monotone.

**Ladder** (128-bit Arb; log `research/sc-r1-20261125/r1_2_fold_ladder.txt`):

| control | (a, b) | verdict | boxes |
|---|---|---|---|
| g² = 0.002 | (0.311, 0.124), the floating optimum | UNRESOLVED (touches 0 at the walls) | 200,000 |
| g² = 0.002 | **(0.36, 0.17)** | **PROVED** | 6,831 |
| g² = 0.002 | (0.40, 0.22) | PROVED | 3,234 |
| g² = 0.004 | (0.403, 0.193), the floating optimum | UNRESOLVED | 200,000 |
| g² = 0.004 | **(0.45, 0.25)** | **PROVED** | 17,075 |
| g² = 0.004 | (0.50, 0.30) / (0.55, 0.35) | PROVED | 6,943 / 3,824 |

The uniform rule's reductions are 0.580 and 0.816, applied to both blocks. In the floating base scan, the fold cut
the count of base levels below ρ_b from 38 to 13 (g² = 0.002) and from 92 to 23 (g² = 0.004). It raised the base
λ₁₁ from 6.83 to about 7.3, and from 6.09 to about 7.2. The VSCF homotopy at the proved folds, run through
`scripts/vscf_folded_base.py` (which sets `vscf_helike_sc_lehmann.FOLD`), is running.

### R1.8.5 Observation to register before the next results: S_C does not need ρ_b

Lemma R needs λ₅(H) ≥ ρ_C. Neumann bracketing gives λ₅(H) ≥ min(λ₅(H_in^N), inf H_out^N), and H_in^N ≥ H₁ ≥ H_s
for every homotopy parameter s (D ≥ 0 and the inside-minorant chain of §8.3). So **any homotopy step whose real
Lehmann bound has ℓ₅(H_s) ≥ ρ_C proves S_C at that control**, provided inf H_out^N ≥ ρ_C. The outside clears
ρ_C + 0.05 from tube radius 4, as shown in §8.3 of the Lehmann registration; its rigorous enclosure is still owed.
This needs neither ρ_b nor the separate Lehmann step on the full H with cut Hermite states. The margin to beat is
the S_C margin, about 0.27, not the base margin of about 0.03.

I am registering this now, before seeing any ℓ₅ values. The driver now logs ℓ₅ and μ₅ at every real step
(`scripts/replan_helike_sc_lehmann_path.py`; a logging change only, with no effect on decisions). **Proposed
decision rule** (R2 to confirm):
- **S_C PROVED at a control** if some real step has ℓ₅ ≥ ρ_C in interval arithmetic and the outside bound clears
  ρ_C.
- **Otherwise UNRESOLVED.**

ρ_b remains the registered target of the base step and is not changed.

## R1.9 Results, 26 November: the folded base, and S_C's own margin

### R1.9.1 A void run, disclosed

The first folded runs applied `FOLD` only to the VSCF executor. The planner's proxy Ritz curves
(`plan_helike_sc_lehmann_path.build`) still used the uniform base, so the planner charted a different family, and its
starting ρ (a uniform-base level) was not a lower bound for the folded family. The symptom was ℓ₁₁ = −∞ at the first
real step. The fix is `plan_helike_sc_lehmann_path.FOLD`, set by `scripts/vscf_folded_base.py`. The void logs are
kept as `research/sc-fold-homotopy-20261126/VOID_planner_mismatch_*`. They carry no result.

### R1.9.2 The folded homotopy (floating point; every input constant rigorous)

The inputs are:
- the proved folds (R1.8.4): (0.36, 0.17) at g² = 0.002 and (0.45, 0.25) at g² = 0.004;
- the enclosed v_J and face-wise β (R1.7).

Logs are in `research/sc-fold-homotopy-20261126/v2_*`. ρ_C here is E₀ + 2ω_H − ω_min/4, recomputed from the same
E₀ − V_bg as ρ_b: 7.3169 at g² = 0.002 and 7.3139 at g² = 0.004. The rigorous step must use the frozen ρ_C of the
Temple ordering receipt.

| control | base | LOSS | real steps | ℓ₁₁ − ρ_b | ℓ₅ − ρ_C (same last step) |
|---|---|---|---|---|---|
| g² = 0.002 | uniform, C₃ sym (R1.8.2) | 0.03 | 4 | +0.0298 | (not logged) |
| g² = 0.002 | **folded** | 0.01 | **2** | +0.0102 | **+0.212** |
| g² = 0.002 | **folded** | 0.03 | **2** | **+0.0300** | **+0.244** |
| g² = 0.004 | uniform, C₃ sym (R1.8.2) | 0.01 | 8 | +0.0003 | (not logged) |
| g² = 0.004 | **folded** | 0.01 | **3** | +0.0101 | **+0.209** |
| g² = 0.004 | **folded** | 0.03 | **3** | **+0.0296** | **+0.241** |

LOSS 0.06 finds no plan in any run. That is structural: μ₁₁(1) − ρ_b is only about 0.03.

At every logged last step, the real Lehmann bounds sit within 4·10⁻⁴ of the Ritz values, with residuals
≤ 1.5·10⁻².

**Reading.**
- With rigorous constants throughout, the base step now passes at both controls with the full floating margin of
  §13 (+0.030 and +0.030, against +0.030 and +0.024 there). It takes 2–3 real steps instead of 4–9.
- On the route of R1.8.5, the same homotopy gives ℓ₅(H_s) ≥ ρ_C with a margin of about **0.21–0.24** at both
  controls. That is the S_C margin itself, about 8 times the ρ_b margin. Interval widening of order 10⁻² would not
  threaten it.

### R1.9.3 Tests and mutants for the R1.8 pieces

`scripts/test_helike_sc_r1_fold.py`; log in `research/sc-r1-20261125/r1_8_tests_and_mutants.txt`.

**Five tests, all passing:**
- C₃ dominates the full-sphere sampled maximum and sits within 2·10⁻³ of it;
- the seven registered faces;
- the registered folds are proved;
- a fold that needs the quartic is proved: (0.41, 0.21) at g² = 0.004, where sampled D < 0 without P₄;
- an infeasible fold is not proved.

**Five mutants, all caught:**
- two dropped C₃ faces (caught only by the face-count test; a numerical catch would be better);
- the dropped quartic;
- the exit parameter halved;
- the reductions given the wrong sign.

### R1.9.4 What remains for S_C at the controls

1. **The outside bound, rigorous.** inf H_out^N ≥ ρ_C on the complement of the gauge-swept product tube. Floating
   point gives about 11 on the walls, against 7.31.
2. **R2: interval A₀, A₁ and A₂ for the VSCF trial functions, and a verified pencil**, at the 2–3 steps of the
   folded path. The decision rule is the one of R1.8.5: PROVED if some real step has ℓ₅ ≥ ρ_C in interval
   arithmetic and item 1 holds. Otherwise UNRESOLVED.
3. **Bookkeeping.** The frozen ρ_C, and the fold constants pinned as exact rationals; they are already strings into
   Arb.
