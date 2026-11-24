# Two grains, Phase 1: the exact potential, and a correction

24 November 2026. **COMPUTED (exact averages, numerical checks). Phase 1 of the
[two-grain scope](helike-two-grain-scope-2026-11-24.md). No spectra were computed.**
Code: `scripts/ym_two_grain_potential.py`. Tests: `scripts/test_ym_two_grain_potential.py` (11 passing).
Mutation check: `scripts/check_ym_two_grain_mutations.py` (4 of 4 caught by assertion).

## How it was derived

On S³ = SU(2) I take the left-invariant frame Xⱼ = x τⱼ and the right-invariant frame Yⱼ = τⱼ x = Dⱼₖ(x) Xₖ,
where D(x) = Ad(x⁻¹) and X_m D = −D ε_m. For the connection A = M eT + N ēT, I write A(Xₖ) = B(x)ₖ with
B = M + N D(x), and compute the curvature in the left frame from the Lie algebra alone:

  F_{a,jk} = X_j B_{ak} − X_k B_{aj} − ε_{jkl} B_{al} + ε_{abc} B_{bj} B_{ck}.

The energy is the S³ average of ½|F|². The density depends on x only through D, as a polynomial of degree ≤ 4,
so averaging over the 120-element binary icosahedral group (60 rotations) is exact. That group has no invariant
harmonic with 1 ≤ l ≤ 5. Checks:
- ⟨D⟩ = 0, ⟨DD⟩ = δδ/3, ⟨D₁₁⁴⟩ = 1/5 and ⟨D₁₁D₂₂D₃₃⟩ = 1/6, all exact;
- a Monte Carlo over S³ agrees;
- with N = 0 the result is the one-grain model at g = 1, so the model normalization is fixed by the geometry, not
  chosen.

## The potential

In model units (H = −½(Δ_q + Δ_p) + V₁₈, with M = gq and N = gp):

  **V₁₈(q,p) = V(q; g) + V(p; −g) + (g²/3) Σᵢⱼ |qᵢ × pⱼ|²**,

where V(q; g) = ½T − 3g det q + (g²/4)(T² − P) is the one-grain potential and qᵢ, pⱼ are the colour vectors
(columns). Equivalently, the cross term is (g²/3)[T_q T_p − tr(qqᵀppᵀ)]. The ⅓ is ⟨DD⟩. The cross term is the
colour cross product of every left mode with every right mode, the same shape as the one-grain quartic term
¼Σⱼₖ|qⱼ × qₖ|².

## The expectations, checked

| | Expected | Found |
| --- | --- | --- |
| E1 | No quadratic or cubic mixing | Confirmed. The only bidegrees are (2,0), (3,0), (4,0), their mirrors and (2,2). |
| E2 | The oriented Chern–Simons restriction is a difference | Confirmed. The projected *F is (−∇W_L(q), +∇W_R(p)), so CS₁₈ ∝ W_L(q) − W_R(p), up to one overall convention sign. This is the revised scope's E2 and van Baal–Hari Dass eq. (101), Q(c − ĉ) = Q(c) − Q(d). |
| E3 | V₁₈ = ½\|∇CS₁₈\|² + C = ½\|∇U₁₈\|² + C, C ≥ 0 | Confirmed exactly, with U₁₈ = W_L + W_R the auxiliary sum and C the cross term above. The magnetic gradient is not closed in the eighteen modes: C = ½\|(1 − P)*F\|² > 0 off the single-grain subspaces. |
| E4 | Vacua | Exactly three sets: 0, (R/g, 0) and (0, −R/g). The mixed point (R/g, −R′/g) costs 2/g². |
| E5 | Parity | V₁₈(q,p) = V₁₈(−p,−q). The two second vacua have identical Hessian spectra. |

**The two grains are the two helicities of the curl.** The opposite signs in W₁₈ say that left-invariant forms
are eigenforms of the curl operator with one sign and right-invariant forms with the other. The razor's sweep
direction is helicity.

**The bound needs each grain swept with its own grain.** Factoring with the oriented functional CS₁₈ gives
ΔCS₁₈ = 9 − 9 = 0, and only H ≥ 0 + C. Factoring with the auxiliary sum U₁₈ (each grain with its own W) gives

  H = ½A_L†A_L + ½A_R†A_R + 9 + C ≥ 9,

on the flat-coordinate domain with the orthonormal kinetic metric (the two mode families are orthonormal under the
S³ average). Both are exact identities of the same Hamiltonian. As the revised scope says, the choice of
factorization exposes or hides the stronger bound rather than changing it. Large-gauge boundary conditions must be
checked before applying it to a physical quotient.

## The correction

At the second vacuum (R/g, 0):
- the left modes keep the one-grain Hessian, with frequencies 1, 2 (×5) and 0 (×3);
- the right modes are stiffened by the cross term, from ω² = 1 to 7/3.

So the zero-point energy there exceeds the origin's by

  4.5√(7/3) − 3.5 ≈ 3.37,

not 1.

| Truncation | Second-vacuum local offset |
| --- | --- |
| One grain (9 modes) | 1 |
| Two grains (18 modes) | 4.5√(7/3) − 3.5 ≈ 3.37 |
| Full theory | 0 (the two vacua are gauge copies) |

Both offsets are gaps within one Hamiltonian: the second-vacuum local zero-point energy minus the origin's, in the
same model. So the comparison is matched, not a shift of absolute zero-point between models. The offset depends on
the truncation and does not approach the full-theory value monotonically. My weak-coupling
prediction, that the lowest scalar sits on the second vacuum with gap → 1, was therefore a one-grain artefact. With
both grains the second-vacuum states sit about 3.37 up, and at weak coupling the lowest excitations are the
origin's oscillator states (scalar and tensor both at gap 2 to leading order). The one-grain model was cutting too
close. The [weak-coupling note](helike-second-vacuum-weak-coupling-2026-11-24.md) now carries this correction at
its top.

What is not affected:
- the exact identities (the one-grain superpotential, the colour–space-locking Hessian, the double-cover reading);
- the registered one-grain measurements.

What is now open: whether the one-grain scalar–tensor crossing survives with both grains. Near g² ≈ 1.76 the two
vacua overlap, so harmonic offsets do not decide it. Phase 3 spectra must.

## Check against van Baal and Hari Dass (NPB 385 (1992) 185–226)

Read from the published paper (pages 209–213), after Phase 1 had been computed independently:
- **Eq. (92)**: V(c,d) = V(c) + V(d) + (2π²/3){(cᵢᵃ)²(dⱼᵇ)² − (cᵢᵃdⱼᵃ)²}. The brace is Σᵢⱼ|cᵢ × dⱼ|², and 2π²/3 is
  one third of vol(S³). This is our cross term, with the same ⅓, which their eq. (91), ∫V V = (2π²/3)δδ, traces to
  ⟨DD⟩ = δδ/3 as ours does. The overall normalization of V(c) (their eq. (64)) has not been compared yet.
- **Eq. (101)**: Q(c − ĉ) = Q(c) − Q(d). The oriented difference, as in E2.
- **Eq. (88)**: three perturbative vacua and two sphalerons in the eighteen modes, as in E4.
- **Eq. (87)**: the winding −1 gauge transformation carries the line c = −uδ to d = (u − 2)δ. They impose the
  vacuum identifications as boundary conditions on the fundamental modular domain (eqs. (96)–(108)), not through
  local Hessians. That is consistent with the conclusion above: the local offsets of a truncation are not the
  physical statement.

## Next

- **Phase 2 is largely done by these tests:** the vacua, Hessians and parity. Still to do: the sphaleron (the
  saddle between 0 and R/g) with the cross term included, and the barrier with both grains.
- **Phase 3:** sector solvers for (j_L, j_R). The (0,0) invariants now include the cross invariant
  T_q T_p − tr(qqᵀppᵀ), and the potential is known exactly in invariants.
- **Phase 0 (the literature)** should be done before Phase 3, to compare with van Baal's analysis of the same
  eighteen modes.
