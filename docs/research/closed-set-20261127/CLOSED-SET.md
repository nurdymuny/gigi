# The closed set (the register)

27 November 2026. This file is the single source of truth for the statements I work from. Requirements come from Jaffe
and Witten's official Clay description and the Osterwalder–Schrader axioms it names. Every clause maps to a line
(table at the end).

**Two kinds of change are allowed, and nothing else.**
1. A status changes, upward when something is proved, downward when it fails.
2. A proof inside a line changes.

Any claim that a requirement is missing must quote the clause it comes from. Every change is logged below with its date.

Tags: [P] proved here, [PC] proved and cited, [C] computed, [Cd] conditional, [D] closed by deduction, [X] hypothesis.

## The lines

**A. The lattice theory**
1. [P] dμ_{a,L} = Z⁻¹ exp(−β Σ_p (1 − ½ tr U_p)) ∏ dU_ℓ, with β = 4/g². It is gauge invariant and lives on connections
   modulo gauge, with no gauge fixing.
2. [PC] The measure is reflection positive (Osterwalder–Seiler 1978). So there is a transfer matrix T = e^{−aH} with
   H ≥ 0, a Hilbert space and a vacuum.
3. [PC] For small β there is a gap m(β) > 0, with correlations decaying as e^{−m·distance}.
4. [P] The electric part H_E = (g²/2a) Σ_ℓ C₂(ℓ) has first excited level 3g²/(2a): one plaquette, spin ½, Casimir ¾.
5. [D] Line 4 is the leading strong-coupling term of the **Hamiltonian** lattice gap. The proof is Kato perturbation
   theory, since the magnetic term is bounded on a finite lattice. Uniformity in the volume is the strong-coupling
   cluster expansion [PC].
6. [P] In 2-D: ⟨χ_j⟩/d_j = (I_{2j+1}/I₁)^{|A|}, σ_j = 2C₂(j)/β + O(β⁻²), and σ₁/σ_½ → 8/3.

**B. The small room** (Berger sphere S³_t, radius R, homogeneous fields A = q_{ka} θᵏ T_a)
7. [P] The slow space is ℝ⁹/SO(3), with measure ∏_{i<j} |λ_i + λ_j|.
8. [P] V(λ q_MC) = (1 + 2/t)(λ − λ²)²/(2g²), CS = 3λ² − 2λ³, and q_bg = s·q_MC.
9. [P] Theorem A: V ≥ (κ/2)‖q‖² − S⁴/(2g²), with κ > 0 for 0 < s < (t+2)/(t+1). So the spectrum is discrete.
10. [PC] Helike is an exact eigenvector of the Hessian, with a closed-form eigenvalue, and the floor on the stated
    domain (*By Enough*; Floor v3.3).
11. [X = H1] S_C: in charge 0, at most two eigenvalues lie below ρ_C. The route is proved except the interval Lehmann
    step (R2).
12. [Cd on 11] E_Helike < E_competitor at 3 of 4 controls.

**C. The fast modes (M3)**
13. [P] By Peter–Weyl, H_YM = slow sector (j = 0, exactly H_red; μ_YM = 4ω²_red, F = 2gq) plus fast sectors (j ≥ 1,
    multiplicity j + 1).
14. [C] E₀ = −(b₀/8π²) log(Λ_cut R)·S_cl + E₀^fin, with b₀ = 22/3, to 10⁻⁴. (Sign corrected 27 Nov.)
15. [C, potential part only] δω_k = g² u·∇²E₀^fin·u/ω_k. The separation goes from 0.0165 to 0.0191 at g² = 0.002.
    A kinetic correction of the same order (the non-adiabatic mass plus the Coulomb term of the fast Gauss law) is
    not computed, so the sign of the O(g²) shift is open.
16. [X = H2] M3, in spectral form (H2 file): at s = 1, on a chart around q = 0, t ≠ 1, small g(R), the levels of the
    full Hamiltonian in a low window lie within C·g²(R)/R of the reduced model's, uniformly in Λ_cut.

**D. The carriers**
17. [P] Different charges, so the crossing is exact.
18. [X; existence of a switch D only for g² ≲ 0.0075 and given H8; at the registered couplings it needs a control at
    s₁ ∈ (s*, 1.25) where Helike is lowest] For s > s* Helike carries the note; for s < s* the competitor does.
    s* = 1.1674, 1.1809, 1.1931 at g² = 0.03, 0.05, 0.07. At s = 1 the carrier is the charge-0 competitor.

**E. The bridge, the room, the limit**
19. [D] The source bridge is s → 1⁺. The sources vanish at s = 1, and s = 1 is inside Theorem A's window. Kato:
    eigenvalues and projections are continuous on [1, s_controls].
20. [X = H3] M2: Δ(R)/Λ ≥ c > 0 for every R, with Δ(R) the gap over the whole physical Hilbert space (so the vacuum
    of each continuum room is simple; this clause now lives here), starting from the unsourced room with the scalar carrier. *Proof route:*
    the three-part bundle skeleton (H3 §3.5): continuity along R, no zero in any room, control at both ends (small
    rooms by H2; large rooms by a finite-size criterion).
21. [PC, partial] The ultraviolet limit holds in a finite box with an infrared cutoff (Bałaban; Magnen–Rivasseau–
    Sénéor). On my geometry these are templates (see the 27 November merge).
22. [P] Reflection positivity survives limits (it is a closed condition).
23. [X = H4′] **Merged with line 29 (27 November): on my geometry.** The family of continuum theories on S³_R × ℝ:
    - for every R, the ultraviolet limit exists on the sphere, with the cutoff of line 16 removed and OS regularity
      bounds. On S³ × ℝ time and space renormalize separately, so the limit tunes **two** bare parameters, the coupling
      g₀ and a speed of light c₀;
    - as R → ∞, the correlation functions converge on compact regions to an ℝ⁴ theory with full Euclidean symmetry.

    With line 22, the OS axioms hold, and reconstruction gives a relativistic theory. The ℝ⁴ theory is *defined* as
    the R → ∞ end of this section, so no box construction has to be matched.

**F. What the problem requires**
24. [X = H7] Every compact simple G. Lines 1–3 already hold for every compact G [PC].
25. [X = H6] Local fields for the gauge-invariant polynomials in F (tr F² first), agreeing with perturbation theory and
    asymptotic freedom at short distance.
26. [Cd: D from 25 plus one finite perturbative fact, the leading three-point function of the trace anomaly ≠ 0]
    Non-triviality. Criterion: the short-distance correlations of tr F² carry the 1/log corrections of
    asymptotic freedom, which no free theory has.
27. [D from 28] A unique vacuum: the spectral measure has no atom at 0. In the reduction this holds by
    Perron–Frobenius [PC].
28. [P lemma + D on 20, 23] The gap survives the limit.
    - The lemma: if truncated ⟨A ; e^{−τH} A⟩ ≤ C_A e^{−mτ} on a dense set, then spec H ∩ (0, m) is empty.
    - The deduction: at each R the bound holds with rate Δ(R) ≥ cΛ (line 20) and constants bounded through the
      convergence of the line 23 family. It passes to the limit.
29. **Merged into line 23 (27 November).**

## The hypotheses (the closed working set)

| # | Hypothesis | Line | Needed for |
|---|---|---|---|
| H1 | S_C | 11 | the carrier form (a finite computation, R2) |
| H8 | at s = 1, the lowest excitation has charge 0 | 18 | the carrier form. True for the undressed reduced operator (derivation confirmed); in the physical theory decided only at O(g²) (needs the kinetic term, H2b) |
| H2 | M3, uniform dressing at small R | 16 | the small-room end of H3 |
| H3 | M2, Δ(R)/Λ ≥ c for every R | 20 | the gap |
| H4′ | the sphere family: the continuum limit in every room, and an ℝ⁴ end with full Euclidean symmetry | 23 | existence |
| H6 | local fields plus short-distance perturbation theory | 25 | field content, non-triviality |
| H7 | every compact simple G | 24 | generality |

The Clay statement rests on **H3, H4′, H6 and H7**, with H2 as the route into H3.

## Clause map (Jaffe–Witten official statement and the OS axioms)

| Clause | Line(s) |
|---|---|
| "for any compact simple gauge group G" | 24 |
| "exists on ℝ⁴" | 21, 23 |
| Euclidean invariance, reflection positivity, regularity, clustering | 2, 22, 23, 28 |
| relativistic theory by reconstruction | 23 |
| local field operators for the gauge-invariant polynomials in F | 25 |
| short-distance agreement with asymptotic freedom and perturbation theory | 14, 25 |
| "non-trivial" | 26 |
| unique vacuum | 27 |
| "mass gap Δ > 0" | 20, 28 |
| it is Yang–Mills (the Wilson theory with the correct running) | 1, 14, 21 |

## Change log

- **27 Nov, the list fixed.** 29 lines, mapped to the clause table above.
- **27 Nov, deductions.** Lines 5, 12, 18 (the existence of a switch), 19, 26, 27 and 28 closed or reduced by deduction.
- **27 Nov, two wording errors corrected** (the requirements are unchanged):
  - Line 26's criterion: "connected four-point of tr F² ≠ 0" also holds in a free theory. It is replaced by the
    1/log criterion.
  - Line 5 mixed the Euclidean and Hamiltonian lattices. It now refers to the Hamiltonian lattice gap.
- **27 Nov, H3 proof route.** The three-part bundle skeleton was added. Correction: strong coupling (lines 3–5) is not
  the large-R end of the continuum family. The large end is a finite-size criterion.
- **27 Nov, merge: H4 + H5 → H4′, line 29 into line 23.** Reason: I begin with the geometry (spheres throughout), so
  the ℝ⁴ theory is defined as the R → ∞ end of the sphere section. The box ≡ sphere equivalence clause disappears.
  What remains: the ultraviolet limit on each sphere, and full Euclidean symmetry at the R → ∞ end (the residue is the
  symmetry mixing time with space). Every clause still maps; the set goes from 8 hypotheses to 7. The cited box results
  (line 21) become templates. The detailed write-ups `H4-continuum-limit.md` and `H5-one-geometry.md` are kept as the
  two halves of H4′.
- **27 Nov, referee pass** (`COHERENCE.md`). Statuses and proofs only; no requirement changed.
  - Line 14: sign corrected to −(b₀/8π²) log(Λ_cut R).
  - Line 15: the widening is the potential part of the O(g²) shift. The kinetic term of the same order is uncomputed,
    so the sign is open. The report pages and §14 carry this caveat.
  - Line 16 / H2: adopted the spectral form, with its domain (s = 1, near q = 0, t ≠ 1).
  - Line 18: the switch brackets only at weak coupling (g² ≲ 0.0075). At the registered couplings it needs a control at
    s₁ ∈ (s*, 1.25).
  - Line 26: now conditional on one finite perturbative fact (a generalized free field can mimic the 1/log² two-point
    function).
  - Line 20 / H3: owns the simplicity of each room's vacuum (previously claimed by H4/H5 with no route).
  - H8: true for the undressed reduced operator. Physically q_MC is a gauge copy of 0, and near q = 0 the charge-0 and
    charge-2 states tie at harmonic order. Open at O(g²).
  - H4′: two bare parameters (g₀, c₀) on S³ × ℝ. This is the time–space residue named at the merge. The merge breaks the
    referee's cycle H4(b) ⇄ H5.
  - Proof-route fixes: H6 is stated on the sphere family (this breaks the cycle H5(b) ⇄ H6); flowed Wilson loops replace
    bare ones; H7's Young weight is (2α−1)/2; H6's plaquette count; H3 fixes t ≠ 1.
