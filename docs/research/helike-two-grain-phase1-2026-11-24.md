# Two grains, Phase 1: the exact potential, and a correction

24 November 2026. **COMPUTED (algebraically exact quadrature rule, floating-point evaluation). Phase 1 of the
[two-grain scope](helike-two-grain-scope-2026-11-24.md). No quantum Hamiltonian spectra were computed.**
Code: `scripts/ym_two_grain_potential.py`. After review, `scripts/test_ym_two_grain_potential.py` has 15 passing tests.
Mutation check: `scripts/check_ym_two_grain_mutations.py` (5 of 5 caught by assertion).
Receipts: [bounded review](../../research/two-grain-review-20261124/).

## How it was derived

On S³ = SU(2) I take the left-invariant frame Xⱼ = x τⱼ and the right-invariant frame Yⱼ = τⱼ x = Dⱼₖ(x) Xₖ,
where D(x) = Ad(x⁻¹) and X_m D = −D ε_m. For the connection A = M eT + N ēT, I write A(Xₖ) = B(x)ₖ with
B = M + N D(x), and compute the curvature in the left frame from the Lie algebra alone:

  F_{a,jk} = X_j B_{ak} − X_k B_{aj} − ε_{jkl} B_{al} + ε_{abc} B_{bj} B_{ck}.

The energy is the S³ average of ½|F|². The density depends on x only through D, as a polynomial of degree ≤ 4,
so averaging over the 120-element binary icosahedral group (60 rotations) is exact. That group has no invariant
harmonic with 1 ≤ l ≤ 5. Checks:
- the exact moment targets ⟨D⟩ = 0, ⟨DD⟩ = δδ/3, ⟨D₁₁⁴⟩ = 1/5 and ⟨D₁₁D₂₂D₃₃⟩ = 1/6 agree at the stated
  floating-point test tolerances;
- the curvature average and invariant formula agree at independently chosen coefficient matrices;
- with N = 0 the result is the one-grain model at g = 1, so the model normalization is fixed by the geometry, not
  chosen. This fixes consistency with our existing model units; it is not yet a complete normalization match to
  another paper's radius, volume and Hamiltonian conventions.

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
| E3 | V₁₈ = ½\|∇CS₁₈\|² + C = ½\|∇U₁₈\|² + C, C ≥ 0 | The identity is supported by the quadrature and projection tests, with U₁₈ = W_L + W_R. C is generally positive, but vanishes when all nonzero columns of both families share one colour axis. Both families can therefore be nonzero with C=0. |
| E4 | Vacua | Three sets: 0, (R/g, 0) and (0, −R/g). The mixed point (R/g, −R′/g) costs 2/g²; exhaustion follows from the nonnegative decomposition, not from sampling those points alone. |
| E5 | Parity | V₁₈(q,p) = V₁₈(−p,−q). The two second vacua have identical Hessian spectra. |

**The two grains are the two helicities of the curl.** The opposite signs in W₁₈ say that left-invariant forms
are eigenforms of the curl operator with one sign and right-invariant forms with the other. The razor's sweep
direction motivates this helicity comparison; the physical shaving direction is not literally a field helicity.

For the vacuum count, every term in V₁₈ is nonnegative. A zero must therefore be a zero of each one-grain square
and of C. For g>0 the one-grain equation q=g cof(q) permits only q=0 or q=R/g: ranks one and two are incompatible
with that equation, while full rank gives qqᵀ proportional to I and the stated scale. The right-grain equation
gives p=0 or -R/g. Of the four combinations, the simultaneous nonzero orbits have C=2/g² and are excluded.

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
| Full theory | Not computed here; gauge-related fluctuation operators require a common gauge-covariant regulator and domain for comparison |

Each offset is a difference between local harmonic zero-point estimates within the same Hamiltonian, not a computed
global spectral gap. Thus the comparison is not just an arbitrary shift of the zero-point energy between models.
The one-grain local estimate does not survive this enlargement unchanged. This does not refute a conjecture
explicitly confined to the one-grain Hamiltonian, but prevents exporting its proposed gap limit of 1 to the larger
model. The origin's scalar and tensor oscillator excitations at gap 2 are now plausible lower candidates than the
second-orbit local ground estimate. Establishing the global lowest levels still needs state construction and
spectral control. The [weak-coupling note](helike-second-vacuum-weak-coupling-2026-11-24.md) records this restriction.

What is not affected:
- the one-grain superpotential and colour-space-locking Hessian identities, and the actual SU(2) to SO(3) map;
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

- **Phase 2 has local checks:** the vacua, Hessians and parity. Still to do: the sphaleron (the
  saddle between 0 and R/g) with the cross term included, and the barrier with both grains.
- **Phase 3:** sector solvers for (j_L, j_R). The (0,0) invariants now include the cross invariant
  T_q T_p − tr(qqᵀppᵀ), and the potential is known exactly in invariants.
- **Phase 0 (the literature)** should be done before Phase 3, to compare with van Baal's analysis of the same
  eighteen modes.
