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
   theory, since the magnetic term is bounded on a finite lattice. This deduction is finite-volume:
   the Euclidean cluster expansion in line 3 does not by itself establish a volume-uniform Hamiltonian remainder.
6. [P] In 2-D: ⟨χ_j⟩/d_j = (I_{2j+1}/I₁)^{|A|}, σ_j = 2C₂(j)/β + O(β⁻²), and σ₁/σ_½ → 8/3.

**B. The small room** (Berger sphere S³_t, radius R, homogeneous fields A = q_{ka} θᵏ T_a)
7. [P] Homogeneous fields modulo constant colour gauge transformations form ℝ⁹/SO(3), with local signed-polar
   Jacobian ∏_{i<j} |λ_i + λ_j|. This reduction retains large-gauge copies; a global polar integral must account for
   chart multiplicities.
8. [P] V(λ q_MC) = (1 + 2/t)(λ − λ²)²/(2g²), CS = 3λ² − 2λ³, and q_bg = s·q_MC.
   The midpoint is a maximum along this path, not generally a critical point of the full Berger potential.
9. [P] Theorem A: V ≥ (κ/2)‖q‖² − S⁴/(2g²), with κ > 0 for 0 < s < (t+2)/(t+1). So the spectrum is discrete.
   [D, reduced extension to each fixed compact simple algebra] The cubic-quartic closure gives
   |C| <= c_g Q^(3/4) and V >= (kappa/2)||q||^2 - 27 S^4 c_g^4/(32 g^2), with the same
   positive-kappa window (t >= 1, g > 0). The natural Friedrichs realization and its reducing symmetry
   sectors have compact resolvent. Constants may depend on the group and norm. See the dated proof note.
10. [PC] Helike is an exact eigenvector of the Hessian, with a closed-form eigenvalue, and the floor on the stated
    domain (*By Enough*; Floor v3.3).
11. [X = H1] S_C: in charge 0, at most two eigenvalues lie below ρ_C. The route is proved except the interval Lehmann
    step (R2).
12. [Cd on 11] E_Helike < E_competitor at 3 of 4 controls.

**C. The fast modes (M3)**
13. [P] Peter–Weyl decomposes the fields and homogeneous-background Hessian into j = 0 and j ≥ 1 blocks
    (multiplicity j + 1). The homogeneous classical restriction gives H_red, with μ_YM = 4ω²_red and background
    coordinates M = 2gq. This is not a direct sum of independent sectors of the interacting quantum Hamiltonian.
14. [C] E₀ = −(b₀/8π²) log(Λ_cut R)·S_cl + E₀^fin, with b₀ = 22/3. The saved seven-point comparison has maximum
    relative discrepancy 7.95×10⁻⁴; the central five are within 6.66×10⁻⁵. These bound observed discrepancies,
    not all numerical errors. (Sign corrected 27 Nov.)
15. [C, potential part only] δω_k = g² u·∇²E₀^fin·u/ω_k. The separation goes from 0.0165 to 0.0191 at g² = 0.002.
    A kinetic correction of the same order (the non-adiabatic mass plus the Coulomb term of the fast Gauss law) is
    not computed, so the sign of the O(g²) shift is open.
16. [X = H2] M3, in spectral form (H2 file): at s = 1, on a chart around q = 0, t ≠ 1, small g(R), the levels of the
    full Hamiltonian in a low window lie within C·g²(R)/R of the reduced model's, uniformly in Λ_cut.

**D. The carriers**
17. [P] Different charges cannot mix: if their levels meet, their crossing is exact. Symmetry does not force a meeting.
18. [X; existence of a switch D conditional on opposite finite-g orderings at two source values, including H1/H8
    where used. The value g² ≈ 0.0075 is a first-order crossover estimate, not a proved bracket. At the registered
    couplings the proposed bracket uses s₁ ∈ (s*, 1.25) where Helike is lowest.] For s > s* Helike carries the note; for s < s* the competitor does.
    s* = 1.1674, 1.1809, 1.1931 at g² = 0.03, 0.05, 0.07. At s = 1 the carrier is the charge-0 competitor.

**E. The bridge, the room, the limit**
19. [D] The source bridge is s → 1⁺. The sources vanish at s = 1, and s = 1 is inside Theorem A's window. Kato:
    ordered eigenvalues are continuous on [1, s_controls]. Projections are continuous for isolated clusters,
    not necessarily for individually energy-ordered levels at a crossing.
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
    - as R → ∞, the correlation functions converge on compact regions to an ℝ⁴ theory with full Euclidean symmetry;
    - the observable identification includes a local or reflection-compatible algebra, dense reconstructed states
      and uniform bounds. Positive four-dimensional flow smears time; its classical sup norm alone does not supply
      a physical operator norm or reflection positivity.

    With line 22 and the remaining OS properties (clustering with line 20), reconstruction gives a relativistic theory. The ℝ⁴ theory is *defined* as
    the R → ∞ end of this section, so no box construction has to be matched.

**F. What the problem requires**
24. [X = H7] Every compact simple G. Lines 1–3 already hold for every compact G [PC].
    The reduced cubic-quartic sub-conjecture A_g is closed by deduction from Austing--Wheater (2003),
    eq. (2.2); line 9 records the consequence. This does not close H7 or identify a lowest carrier.
25. [X = H6] Local fields for the gauge-invariant polynomials in F (tr F² first), agreeing with perturbation theory and
    asymptotic freedom at short distance.
26. [Cd diagnostics on 25; the finite free four-point coefficient is now computed]
    The 1/log² trace-anomaly fingerprint distinguishes the stated finite canonical Wick-polynomial comparison;
    the nonzero leading connected four-point function excludes the anomaly field itself being generalized free.
    The original free Wick-triangle prediction is refuted: T_O = 0 at every separated triple in Euclidean R4.
    For O = sum over a,mu,nu of :F^a_mu,nu F^a_mu,nu:, the connected four-point coefficient at
    0, L e0, 2L e0, 3L e0 is 676 dim(g)/(27 pi^8 L^16), L > 0. See the dated proof note.
    These tests do not classify every possible Gaussian composite or the full observable algebra. Non-triviality
    of the reconstructed theory remains an H6 identification, not a deduction from the higher-point test alone.
27. [D from 28] A unique vacuum: the spectral measure has no atom at 0. In the reduction this holds by
    Perron–Frobenius [PC].
28. [P lemma + D on 20, 23] The gap survives the limit.
    - The lemma: for v_A = (A − ⟨Ω,AΩ⟩)Ω dense in Ω-perp, if 0 ≤ ⟨v_A,e^{−τH}v_A⟩ ≤ C_A e^{−mτ}
      with finite C_A, then the positive spectral measures vanish on [0,m), and spec H ∩ (0,m) is empty.
    - The deduction: line 20 supplies the rate Δ(R) ≥ cΛ; physical operators with uniformly bounded operator
      norms supply constants. Line 23 must identify their correlations and dense limiting states.
29. **Merged into line 23 (27 November).**

## The hypotheses (the closed working set)

| # | Hypothesis | Line | Needed for |
|---|---|---|---|
| H1 | S_C | 11 | the carrier form (a finite computation, R2) |
| H8 | at s = 1, the lowest excitation has charge 0 | 18 | the carrier form. Harmonic ordering is derived; the H8 note's finite-g reduced claim remains conjectural. Physical channels tie at harmonic order and need the complete O(g²) splitting, including H2b. The reduced part of that splitting at q = 0 is closed (charge 0 lower by 2κ(1 + 1/t)g², charge ±2 unshifted); H2b is not. The reduced charge tail is bounded: every charge-m level is at least (225 + m²)/(30√3) at t = 3 (|m| ≤ 15); it removes charges only once E_{2,0} has a certified upper bound. |
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
| "mass gap Δ > 0" and finite first excitation mass | 20, 25, 28 |
| it is Yang–Mills (the Wilson theory with the correct running) | 1, 14, 21 |

## Change log

- **27 Nov, small-coupling H8 and switch (conditional deduction, reduced model).** Two named inputs:
  (S) the semiclassical harmonic limit per charge sector for wells that are group orbits (Simon 1983 for isolated
  minima; the orbit case by IMS in slice coordinates), and (M) the background orbit as the minimum set at s = 1.15
  (supported by S_C's proved outside bound via V = g⁻²V¹(gq)). Then, at t = 3, there is g₀ > 0 such that for
  0 < g < g₀: H8 holds at s = 1 (gaps → ω_C = 0.7923 vs 2/√3 = 1.1547), and some s* ∈ (1, 1.15) carries an exact
  crossing (gaps → 1.1067 vs 1.0901 at s = 1.15). Line 18's existence clause gains this small-coupling route without
  H1. The registered couplings, physical H8 (H2b) and uniqueness of the switch stay open. The seven hypotheses are
  unchanged. Proof: `docs/research/h8-small-coupling-switch-deduction-2026-11-27.md`. Independent cross-checks of the
  origin splitting and the Wick results: `scripts/check_h8_origin_fock.py`, `scripts/check_h6_wick_numeric.py`
  (receipts in `research/independent-checks-20261127/`).

- **27 Nov, charge tail bounded in the reduced model (bounded deduction).** Route Step 2. The
  horizontal barrier |∇W|² ≥ |X_hor|²/t holds at every q (every rank stratum, both sheets), every g,
  every compact simple metric colour algebra: rotating colour about X1 by g√t X1 gives
  t|∇W|² = |X - Γ|² + t²|X1 - (g/√t)[X2,X3]|². It is sharp, so the pointwise ruler is exactly |m|/√t.
  Adding the twisted (zero-point) inequality gives inf spec on charge m ≥ σ + m²/(4tσ), σ = nS/2;
  at t = 3, (225 + m²)/(30√3). With a certified U < 5.2731 for E_{2,0}, all |m| ≥ 7 are excluded.
  No eigenvalue is enclosed; Step 3 (the upper bound) is still open. H8 remains open; the seven
  hypotheses are unchanged. Proof, exact SU(2)/SU(3) checks, five mechanism removals and five Lean
  declarations: `docs/research/h8-charge-tail-proof-2026-11-27.md`.

- **27 Nov, origin splitting closed in the reduced model (bounded deduction).** H7 Step 6's O(g²)
  comparison at q = 0, s = 1, for every compact simple metric colour algebra and t > 1. Written on
  e^(-W), H - nS/2 = L0 + g L1 with L1 = -grad C . grad. L1 kills the charge ±2 pair, so its series is
  2/sqrt(t) exactly; the charge-0 level moves by e2 g² with e2 = -2κ(1 + 1/t), κ the adjoint Casimir.
  Quasimodes put spectrum within O(g³) and O(exp(-c/g²)) of those values. Counting and assignment to
  the q = 0 well stay the cited localization; the physical room still needs H2b. H7 and H8 remain
  open; the seven hypotheses are unchanged. Proof, exact SU(2)/SU(3) checks, four mechanism
  removals and six Lean declarations: `docs/research/h8-origin-splitting-proof-2026-11-27.md`.

- **27 Nov, the documented Wick-triangle conjecture resolved by refutation.** H6 Step 6 expected
  a nonzero free leading three-point coefficient. Hodge exchange proves it is zero at all separated triples
  in Euclidean R4. The already documented four-point alternative is nonzero, with coefficient 676 dim(g)/(27 pi^8)
  at 0, e0, 2e0, 3e0 in the unchanged canonical normalization. Exact connected Wick enumeration and nonzero
  controls verify the coefficients; Lean checks the finite-matrix trace argument, not a construction of QFT.
  The three-point remainder is additive, not a multiple of a zero leading coefficient. Line 26 now uses the
  conditional four-point diagnostic. H6 and the seven top-level hypotheses remain open. This is one original
  sub-conjecture settled against its prediction, not a new supporting lemma counted as an original closure.
  Proof: `docs/research/h6-wick-triangle-proof-2026-11-27.md`. The earlier expectations in `COHERENCE.md`
  and in the dated entries below are historical and are superseded by this correction.

- **27 Nov, strict vacuum-floor closure (bounded deduction).** For the
  unsourced reduced operator at every fixed t > 0, g > 0 and compact
  simple metric colour algebra of dimension n, E0 > nS/2. The previously
  proved confinement makes the bottom attained. Factorization forces
  equality states to be multiples of exp(-W); a nonzero bracket supplies
  a positive-measure cone on which that candidate is not normalizable.
  At g = 0 the Gaussian attains the floor exactly. This is not an
  excitation-gap or carrier-ordering result. H7 and H8 remain open;
  the seven top-level hypotheses are unchanged. Proof and bounded Lean
  scope: `docs/research/h8-vacuum-floor-proof-2026-11-27.md`.

- **27 Nov, larger-colour-space path transport (bounded deduction).** The
  inter-well path extends through each injective su(2) embedding into a
  compact simple metric Lie algebra. To its specified endpoint orbit the
  ambient action is I/(2 sqrt(t)), with I the measured pullback metric
  factor. Extra colour directions cannot lower this endpoint-specific
  action. This extends the reduced classical geometry of line 8; no
  carrier ordering, quantum gap or continuum status changes. H7 and H8
  remain hypotheses. Full deduction and formal boundary:
  `docs/research/h8-group-transport-proof-2026-11-27.md`.

- **27 Nov, inter-well action closure (bounded geometric step).** H8 Step 6's
  gradient-connection sub-conjecture is proved for the original unsourced
  reduced potential and every t > 0. The inward branch in diag(x,y,y)
  reaches 0 with the inherited metric dx^2 + 2 dy^2. Its action is exactly
  d = 1/(2 sqrt(t)) in p = gq coordinates; the straight-line excess squared
  is (1/t - 1)^2/18. Full proof: `docs/research/h8-interwell-proof-2026-11-27.md`.
  Lean checks the local algebra and calculus, not global orbit existence.
  Line 18 and H8 remain hypotheses; no quantum splitting or continuum
  conclusion is promoted. The old action interval in `COHERENCE.md` remains
  historical evidence.

- **27 Nov, cubic-quartic closure (bounded pure-math step).** Former Sub-conjecture A_g in H7 is
  [D], with alpha = 3/4, from Austing--Wheater, hep-th/0310170v2, equation (2.2), trace normalization
  and homogeneity. Line 9 now records the fixed-group reduced compact-resolvent consequence on the
  unchanged source window. The exact coercivity factor is 27/32 times S^4 c_g^4/g^2.
  The older general-alpha Young weight error is superseded by an exact nonnegative factorization.
  Line 24 remains [X = H7]; no continuum, gap-size, uniform-in-group or carrier-ordering claim is promoted.
  Full proof and scope: `docs/research/h7-cubic-quartic-proof-2026-11-27.md`.
  The historical `COHERENCE.md` describes the pre-closure version; its Young-weight finding is resolved here.

- **27 Nov, line 15 refined (a local estimate, no status change).** From the first-order dressing formula
  δω = v·(δK − ω²δG)v/(2ω v·Gv) added in the narrative, the kinetic correction enters as δG_kin = 2m^ren, so a unit slow
  mode shifts by −ω m^ren_uu. On a smooth gapped fast chart, Φ(q) = g²φ(2gq) has q-curvature O(g⁴), while its common
  O(g²) offset cancels in same-well excitation gaps. This is not a uniform cutoff or inter-well statement.
  Combining the saved anharmonic coefficient with local quadratic dressing at the held sourced background gives
  D_lin = 0.01655 + g²(−2.208 + 1.258 − κ), with κ = ω_C m̃_CC − ω_H m̃_HH uncomputed. In this linear estimate the
  competitor is lower if κ > 7.33 (g² = 0.002) or κ > 3.19 (g² = 0.004), with rounded thresholds and no remainder bound.
  The switch derivative uses coefficients at s0* = 1.14357. Freezing them instead at s = 1.15 gives
  s* ≈ 1.1436 + g²(0.950 + κ)/2.569. Its undressed slope 0.86 predicts 1.1693, 1.1865, 1.2037, not the registered
  1.1674, 1.1809, 1.1931 (which use varying coefficients). The shift is downward only when κ < 1.258.
  A displaced equilibrium at fixed sources can add an O(g²) stiffness change; it is not included here.
  The kinetic term stays OPEN; it now has a local diagnostic threshold. Narrative statement `b:threshold`;
  detailed replay and scope: `docs/papers/ym-theory-statement/THRESHOLD-REVIEW-2026-11-27.md`.

- **27 Nov, narrative accuracy review of The Lowest Note.** This review preserves the theory as a description,
  not a proof submission. It corrects the scope of existing lines; it does not add a research programme or close
  a hypothesis. Details and finite algebra checks: `docs/papers/ym-theory-statement/REVIEW-2026-11-27.md`.
  - Lines 5, 7, 8, 13: finite-volume Hamiltonian remainder; constant-gauge quotient and local polar measure;
    path maximum versus full saddle; classical/quadratic decomposition versus interacting Hamiltonian.
  - Line 14: seven-point and central-five discrepancies now match the saved receipt.
  - Lines 17–19 and H8: no avoided crossing does not force a meeting; 0.0075 is an estimate; projections need an
    isolated cluster. The local MC mode, the other well's ground state and a physical excitation are distinguished.
    This narrows the earlier referee entry saying H8 was proved: its finite-g statement is still conjectural.
  - Lines 23, 27, 28: spell out the observable identification already used in the limit argument, including positivity,
    density and uniform bounds. A four-dimensionally flowed classical observable is not automatically a time-slice operator.
  - Lines 25–26: the paper declares its Euclidean anomaly normalization; a nonzero composite three-point function
    alone does not establish non-triviality of the whole theory.
  - Clause-map clarification, not a new requirement: the official Clay description, printed p. 6, requires
    “the mass m, and we require m < ∞”. The gap row now records that finite nonempty threshold explicitly.
    Source: https://www.claymath.org/wp-content/uploads/2022/06/yangmills.pdf
  - Earlier entries below record their contemporary conclusions; where they differ, this entry and the current
    lines above give the revised reading. No old numerical evidence or failed attempt has been erased.

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
