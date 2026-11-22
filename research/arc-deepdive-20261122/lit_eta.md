# The two hands and the η-invariant: literature, computation, verdict (22 November 2026)

Thread: *the arc*, Part II.3: "the two hands have a number". The question I set myself: is the y-dependent
tie-breaking between Helike and her counter-rotating sister an η-type spectral asymmetry in the sense of
Atiyah–Patodi–Singer?

Tags:
- **PROVED**: a short argument written out here.
- **COMPUTED**: a script and receipt in this folder.
- **QUOTED**: fetched today, with a verbatim quote and the URL.
- **UNVERIFIED**: I could not fetch it, so it is not stated as fact.
- **IMAGINED**: an idea, not a result.

## Short answer

**No, not for the tie-break itself. Yes for two neighbouring things, and one of them is new and exact.**

1. **The Yang–Mills Hessian cannot see orientation, so it cannot carry an η.** (PROVED; §3.1.) η changes sign when the
   orientation is reversed (APS, quoted below). My Q(b) = Σ‖D_ij b‖² + 2⟨F_ij,[b_i,b_j]⟩ uses no Hodge star. So no
   function of the Hessian spectrum can be orientation-odd.
2. **The Hessian does factor through the operator whose η APS defined.** H = (⋆d_A)² + S_F on ad-valued 1-forms, exactly.
   ⋆d_A is the 1-form part of the twisted odd signature operator, the Hessian of Chern–Simons.
   - Helike is an exact eigenvector of ⋆d_A, with eigenvalue **μ = −(x + (j+2)/√t) = κ − √t**.
   - Her closed form is exactly **λ = μ² − B**. That is: her Chern–Simons-Hessian eigenvalue squared, minus the field
     along the fibre.
   - (COMPUTED, 36 points, 10⁻¹⁴; §3.2.)
3. **The two hands sit on the same side of that spectrum.** (COMPUTED; §3.3.)
   - At y = 0 the sister's ⋆d_A eigenvalue is −(−x + (j+2)/√t), the same formula with x → −x. For large j both are
     negative.
   - The exchange x ↔ −x at y = 0 is a **gauge** reflection (rotation by π about τ₁), not an orientation reversal, and
     η is gauge-invariant.
   - So the sister is not Helike's η-partner. An η-partner would have eigenvalue −μ and lives on the oppositely
     oriented sphere.
4. **The twisted η of the family is exactly Chern–Simons plus an integer.** (COMPUTED at 15 points; §3.4.)
   - The fitted law is **η(L_A) = 3·η_S(t) + 4·W(x,y,t) + N**, where N is an integer and
     W = −t x² − 2y² + √t·x y² is the Chern–Simons polynomial of the family (up to a convention constant).
   - Its x-odd part is **8√t·x y²**: zero at y = 0 and quadratic in y, the lowest pseudoscalar the family has.
   - But it is **local** (Chern–Simons), not spectral. The spectral part N jumps where ⋆d_A has zero modes, and
     Helike's and the sister's are at x = ∓(j+2)/√t, **with no y in them**.
5. **The stage itself is handed, by a known number.**
   - η of the untwisted odd signature operator on the Berger sphere (t,1,1) is **−(2/3)(t−1)²**. My own computation
     agrees with Bakas–Lüst and settles the factor-2 discrepancy they report against Hitchin (1997).
   - The Dirac η is −(t−1)²/6 for t < 16 (Hitchin 1974, quoted; reproduced numerically).
   - Since η ≠ 0 for t ≠ 1, **the stretched sphere has no orientation-reversing isometry**. Her stage is chiral in
     Kelvin's sense, with a number attached. (PROVED from the quoted APS sign rule plus the computed/quoted value.)

So the y-dependent tie-break lives in the **spin term ±B and the transverse mixing** of the sister's block, not in a
spectral asymmetry. An η-type quantity is a natural home for **the handedness of the stage** (η_S(t)) and for **the
x-odd Chern–Simons term √t·x y²**, but not for "which hand wins".

My bet on Part II.3 moves from *medium* to *low as stated, with a better question found* (§4).

## 1. Literature (QUOTED unless marked)

### Atiyah, Patodi, Singer, *Spectral asymmetry and Riemannian geometry. I*, Math. Proc. Camb. Phil. Soc. 77 (1975) 43–69
URL: https://webhomes.maths.ed.ac.uk/~v1ranick/papers/aps001.pdf. This is a scan; I read pages 43–45 from the page
images.
- p. 44: "the total Laplace operator Δ is in fact the square of a self-adjoint first order operator B = ±(d* − *d).
  Thus the eigenvalues of Δ are of the form λ², where λ is an eigenvalue of B. Now, unlike Δ, the operator B is not
  positive so its eigenvalues can be positive or negative."
- p. 44: "η(s) = Σ_{λ≠0} (sign λ)|λ|^{−s}." (1·7)
- p. 44: "reversing the orientation of Y changes B into −B and hence η(s) into −η(s)."
- p. 45: "in computing η(s) the only relevant part of the operator B is d* acting on dΩ^{2k−1}, which is roughly in the
  middle dimension. Thus, in the classical situation discussed in the previous paragraph, only curl contributes to the
  computation of η(s)."
- p. 44: "for Ỹ = 3-sphere, f(Ỹ) = 0 but f(Y) ≠ 0 for a lens space Y".

### Atiyah, Patodi, Singer, *Spectral asymmetry and Riemannian geometry. II*, Math. Proc. Camb. Phil. Soc. 78 (1975) 405–432
URL: https://webhomes.maths.ed.ac.uk/~v1ranick/papers/aps002.pdf (scan; pages 405–406 read from images).
- p. 405: "Our main result (Theorem (2·4)) is that η̃_α(0) is independent of the Riemannian metric and so is a
  differential invariant of (Y, α)".
- p. 406, on operators other than B: "we get similar invariants independent of the metric but only after we have reduced
  modulo Z. This is because the 0-eigenvalue no longer represents cohomology (via harmonic forms) and so it produces
  integer jumps in η(0)."

### Hitchin, *Harmonic spinors*, Adv. Math. 14 (1974) 1–55
URL: https://users.math.msu.edu/users/parker/GT/Harmonic%20Spinors.pdf. This is a scan; p. 34 was read from the page
image, and the rest from its embedded OCR, which has small glitches.
- Introduction: "The S³ × S¹-invariant metrics are parametrized up to a constant multiple by a positive real number λ.
  For a generic λ, there are still no harmonic spinors but for certain values they do exist." (OCR)
- Metric convention, §3.1: "the metric with eigenvalues (λ², 1, 1)", "relative to the standard basis {i, j, k} of the
  Lie algebra" (OCR). That basis has [i, j] = 2k, the same as my frame, so **Hitchin's λ² is my t**.
- Corollary to Prop. 3.2: "Let m > 0 be an integer and take the metric (16m², 1, 1) on S³ … Then the dimension of the
  space of harmonic spinors relative to this metric is ≥2m." (OCR)
- p. 34 (page image): "for λ² < 16, the positive eigenvalues are p/λ + λ/2 and λ/2 + √(4pqλ² + (p − q)²)/λ and the
  negative ones λ/2 − √(4pqλ² + (p − q)²)/λ".
- p. 34 (page image): "η(0) = (−1 + 2λ² − λ⁴)/6." and "Modulo Q (and a sign convention), this agrees with the
  Chern-Simons invariant for this family of metrics".
- In my parameter, **η_D(t) = −(t−1)²/6 for t < 16**. Harmonic spinors first appear at **t = 16**.

### Bakas and Lüst, *Axial anomalies of Lifshitz fermions*, arXiv:1103.5693 (2011)
URL: https://arxiv.org/pdf/1103.5693. Text extracted locally.
- (D.11): "ds² = γ[(σ1)² + (σ2)² + δ²(σ3)²], where δ² = γ3/γ measures the degree of anisotropy of the associated
  Berger sphere." Since δ² is a ratio, **δ² = t**.
- (D.42): "ηD = −(1/6)(δ² − 1)²", for "Berger spheres with anisotropy parameter δ < 4, as in [39]". Their [39] is
  Hitchin 1974.
- (4.45): for δ ≥ 4, "ηD = −(δ² − 1)²/6 + 2S(δ)", where S(δ) counts the modes that have crossed zero.
- (4.70), on the signature complex: "computed by Hitchin on Berger spheres [71] and takes the following form,
  ηS(δ) = −(2/3)(δ² − 1)² for all values of the anisotropy parameter δ."
- Footnote 19: "there is a discrepancy by a factor of 2 in the final expression for ηS compared to the result derived
  in [71] (it is apparently a typo)." Their [71] is Hitchin, "Einstein metrics and the eta-invariant", Boll. UMI (7)
  11-B (1997).
- Appendix D: "The first level crossing occurs when the anisotropy parameter passes the critical value δ = 4."

### Boden, Herald, Kirk, Klassen, *Gauge theoretic invariants of Dehn surgeries on knots*, Geom. Topol. 5 (2001) 143–226
URL: https://arxiv.org/pdf/math/9908020v2. Text extracted locally.
- Definition 2.3: "For an SU(2) connection A ∈ Ω¹_Y ⊗ su(2), define the odd signature operator D_A on Y twisted by A to
  be the formally self-adjoint first order differential operator … D_A(σ, τ) = (d*_A τ, d_A σ + ⋆d_A τ)."
- §5.5: "If A is a flat connection, then Atiyah, Patodi, and Singer show that the difference ϱ_X(A) = η_A(0) − η_Θ(0)
  is a real number which is independent of the metric."
- (5.4): "SF(A_t; X) = 2cs(A_1) + ½(ϱ_X(A_1) − dim(ker D_Θ) + dim(ker D_A1))". This uses the fundamental C²
  representation; the adjoint case differs by constants.

### Others, fetched
- Kling and Schueth, *On the Dirac spectrum of homogeneous 3-spheres*, arXiv:2204.12990: "Bär computed the Dirac spectra
  of Berger metrics S^3 and on lens spaces S^3/Z_N."
  - Bakas–Lüst cite Bär as Arch. Math. 59 (1992) 65 and GAFA 6 (1996) 899.
  - I did not open Bär's papers. His spectrum results are UNVERIFIED beyond this sentence.
- Weingart, *About the eta-invariants of Berger spheres*, arXiv:1707.06376.
  - Abstract, as returned by a search extract: "we calculate the generating function of the reduced Dirac and signature
    η-invariants for the family of Berger metrics on the odd dimensional spheres."
  - The abs page fetch confirmed the title and author, and the same content in paraphrase.
- Capoferri and Vassiliev, *Beyond the Hodge theorem: curl and asymmetric pseudodifferential projections*,
  arXiv:2309.02015: "The asymmetry operator generalises and contains the classical eta invariant". The η-invariant of
  curl is a live research topic.

### UNVERIFIED (not fetched; not stated as fact)
- Hitchin, *Einstein metrics and the eta-invariant* (1997): the original value of η_S on Berger spheres. I only have
  Bakas–Lüst's report of it, with the factor-2 remark.
- Gibbons–Pope–Römer (1979) on η for squashed S³ boundaries of gravitational instantons.
- Gilkey's computations of η on Berger or spherical space forms.
- Aharonov–Casher (1979).
- The general APS variation formula for a non-flat path (that dη/ds is local and, at fixed metric, proportional to
  d CS/ds). I use it only as a hypothesis, which §3.4 then tests numerically.

## 2. Dictionary to my conventions

| source | parameter | mine |
| --- | --- | --- |
| Hitchin 1974 | metric (λ², 1, 1) in {i, j, k}, [i, j] = 2k | t = λ² |
| Bakas–Lüst | σ1² + σ2² + δ²σ3² | t = δ² |
| APS / BHKK | B_ev, D_A = (d*_A τ, d_A σ + ⋆d_A τ) | L_A on (Ω⁰ ⊕ Ω¹) ⊗ su(2); its Ω¹ block is ⋆d_A ("curl_A") |

Orientation: θ⁰∧θ¹∧θ² > 0, with θ⁰ along the long circle. The family is the edge paper's: M₀ = xτ₀, M₁ = yτ₁, M₂ = yτ₂.

## 3. What I found

### 3.1 The Hessian is orientation-blind (PROVED)
- Q(b) = Σ_{i<j}‖D_ij b‖² + 2⟨F_ij, [b_i, b_j]⟩ is built from the metric, the bracket and F. It uses no volume form.
- Reversing the orientation therefore leaves H, its spectrum and every eigenvector unchanged.
- η of any operator that flips sign with the orientation flips sign (APS I, quoted). So **no orientation-odd
  invariant is a function of the Hessian spectrum alone**.
- The tie-break between the hands is a statement about Hessian eigenvalues. So it is not, by itself, an
  orientation-type asymmetry.

### 3.2 The Hessian factors through the Chern–Simons Hessian (PROVED + COMPUTED)
- In dimension 3, d_A* = ⋆d_A⋆ on 2-forms, so d_A*d_A = (⋆d_A)² on 1-forms. Hence **H = (⋆d_A)² + S_F**, where S_F is
  the 2⟨F,[b,b]⟩ term. This is exact for every connection; the script checks it to 7×10⁻¹⁵ (C2).
- The family builder reproduces the Levi-Civita builder at the Berger point to 10⁻¹³ (C1). ⋆d_A is exactly Hermitian
  (C2).
- On Helike's two-dimensional eigenspace (her and her mirror), at 12 random (x, y, t) and j ∈ {1, 3, 6}, all COMPUTED to
  ≤ 4×10⁻¹⁴:
  - ⋆d_A acts as μ·I with **μ = −(x + (j+2)/√t)**;
  - S_F acts as **−B**, with B = y² − 2√t x the field along the fibre;
  - λ = μ² − B, which is the closed form of the edge paper's Prop. 3.2, term for term;
  - G*b = 0.
- So Helike is also an exact eigenvector of the odd signature operator L_A.
- In the anatomy's language, μ = κ − √t: fibre momentum minus the frame's twist.
- Proof route: she is alone in her charge block for ⋆d_A too, so μ is a diagonal entry. This is a two-line symbolic
  check, not yet run.
- Receipt: `eta_family.py` → `eta_family_receipt.json`.

### 3.3 Where the hands sit in the curl spectrum (COMPUTED)
- At y = 0, 8 random (x, t), j ∈ {1, 4}: the sister eigenspace (value λ(−x, 0, t, j)) has ⋆d_A = −(−x + (j+2)/√t)·I,
  to 6×10⁻¹⁵.
- The twist and her mirror share one sign of μ, and so do the sister and hers. The hands are not ± pairs of the
  first-order operator.
- At y = 0 the hands are exchanged by a constant gauge rotation (edge paper, Prop. 2.1: "(x,0) ∼ (−x,0)"), under which
  L_A is conjugated and η is unchanged.
- Computed: η(L_A) at (±0.6, 0) and (±1.5, 0) agree to 5×10⁻¹⁰ (§3.4).

### 3.4 The η of the family (COMPUTED; method validated on known cases)
**Method.**
- E(ε) = Σ sign(μ)·e^{−εμ²} over all sectors j ≤ 700 (or 900), with right multiplicity j + 1.
- Fitted on ε ∈ [2×10⁻⁴, 6×10⁻³] with powers ε^{−3/2} … ε²; the constant term is η(0).
- Spectra come from a charge-block solver (`eta_fastcurl.py`), checked against dense builds to 4×10⁻¹⁴.

**Controls.**
- Hitchin's Dirac spectrum, run through the same machinery, gives η_D = −0.04166, −0.16654, −1.4978 and −10.634 at
  λ² = 0.5, 2, 4 and 9. Hitchin's formula gives −0.04167, −0.16667, −1.5 and −10.667. The worst error is 0.3%, a small
  bias that grows with t.
- Untwisted curl (= η(B_ev)) at t = 1, 0.5, 2, 3, 4 and 6 gives 0.000, −0.1666, −0.6656, −2.6607, −5.9826 and −16.598.
  −(2/3)(t−1)² gives 0, −0.1667, −0.6667, −2.6667, −6 and −16.667.
- So **η_S(t) = −(2/3)(t−1)²** in my orientation, agreeing with Bakas–Lüst (4.70) and not with a halved value.
- Receipt: `eta_berger_scalar_fast.py` → `eta_berger_scalar_receipt.json`. The slower first attempt,
  `eta_berger_scalar.py`, did not converge at large t and is superseded.

**The twisted operator** (`eta_family_eta.py` → `eta_family_eta_receipt.json`; t = 3; x ∈ {±0.6, ±1.5},
y ∈ {0, 0.6, 1.2}, plus the trivial and two Maurer–Cartan multiples sθ):

| (x, y) | η(L_A) | W | N = η − (−8 + 4W) |
| --- | --- | --- | --- |
| (0, 0) trivial | −7.982 | 0 | 0.02 |
| (±0.6, 0) | −10.298 | −1.08 | 2.02 |
| (±1.5, 0) | −28.956 | −6.75 | 6.04 |
| (−0.6, 0.6) / (0.6, 0.6) | −13.670 / −10.683 | −2.174 / −1.426 | 3.03 / 3.02 |
| (−1.5, 0.6) / (1.5, 0.6) | −34.564 / −21.098 | −8.405 / −6.535 | 7.06 / 13.04 |
| (−0.6, 1.2) / (0.6, 1.2) | −26.782 / −4.836 | −5.457 / −2.464 | 3.04 / 13.02 |
| (−1.5, 1.2) / (1.5, 1.2) | −54.383 / −18.522 | −13.371 / −5.889 | 7.10 / 13.03 |
| sθ, s = ½ | −3.982 | −2 | 12.02 |
| sθ, s = 1 (flat, gauge-trivial) | −7.982 | −4 | 16.02 |

- −8 = 3η_S(3). All 15 values of N are integers to within the method's bias (≤ 0.1).
- The flat point s = 1 reproduces the trivial connection's η exactly: gauge invariance, with W = −4 absorbed by
  spectral flow 16.
- W = −t x² − 2y² + √t x y². I derived it by hand: ⟨A∧dA⟩ gives −(2√t x² + 4y²/√t), and ⅓⟨A∧[A∧A]⟩ gives
  2 det M = 2x y². Multiplied by vol = 2π²√t, that is W up to a constant.
- Check: on A = sθ it gives −4(3s² − 2s³), the standard Chern–Simons profile, independent of t as it must be.
- So, COMPUTED at these points: **η(L_A) = 3η_S(t) + 4W + N, with N ∈ ℤ**.
- The x-odd part, η(x,y) − η(−x,y) = +8√t x y² + ℤ (e.g. +2.987 at (0.6, 0.6), predicted 2.993), is **zero at y = 0 and exactly quadratic in y**.

**Where the integers jump.**
- N changes when an eigenvalue of L_A crosses zero.
- Helike's crossing in sector j is at x = −(j+2)/√t, multiplicity 2(j+1). The sister's (exactly, at y = 0) is at
  x = +(j+2)/√t.
- **Neither involves y.** For example, from (0.6, 0) to (1.5, 0) the sister in sector 0 crosses at x = 2/√3, and
  ΔN = +4 is exactly her two vectors turning positive.
- On the Berger path x = (2−t)/√t, Helike's μ vanishes in sector j at **t = j + 4**. That is an exact ad-valued harmonic
  1-form direction of the Chern–Simons Hessian along her family (COMPUTED from the closed form; INCIDENCE with nothing
  yet).

## 4. Verdict on the bridge, and the better question

**Is an η-type quantity the natural home for the y-dependent tie-break? No.** Three independent reasons:
1. The Hessian is orientation-blind (§3.1).
2. The two hands are a gauge pair, not an orientation pair, and sit on the same side of the curl spectrum (§3.3).
3. The family's η depends on x and y only through a **local** Chern–Simons polynomial plus integer jumps whose
   locations (the hands' curl zeros) contain no y (§3.4).

The tie-break comes from two things the η does not see:
- the **spin term**: S_F = −B on Helike and +B on the sister at y = 0, the Zeeman sign;
- the cross field's **transverse mixing** of the sister's block, measured in Test E to first order.

**What η does give me, honestly:**
- **The stage is handed, by a number.** (QUOTED + COMPUTED.)
  - η_S(t) = −(2/3)(t−1)² and η_D(t) = −(t−1)²/6 (t < 16).
  - Both vanish only at the round sphere, so the stretched sphere has no orientation-reversing isometry.
  - This is the exact version of "the Berger sphere is a choice of hand" in *the two hands*. Hitchin's harmonic spinors
    at t = 16 are the Dirac operator's own level crossing on the same family.
- **The family's pseudoscalar is √t·x y².** (COMPUTED.)
  - It is the first gauge-invariant, orientation-odd quantity in the family, and it vanishes exactly where the hands
    tie (y = 0).
  - It is **not** evidence that η breaks the tie. Every gauge-invariant analytic quantity here is even in y, because
    y → −y is a gauge rotation about τ₀. So "quadratic in y" is forced.
  - Test E's descriptive onset ratio near 4.5 for doubling y is likewise expected from any y² law. I count it as zero
    evidence for an η link.
- **Her price is a square of a first-order price.** (COMPUTED.)
  - λ = μ² − B, with μ her eigenvalue under the Chern–Simons Hessian.
  - That puts her in the exact setting of APS spectral flow: she is a zero mode of L_A at x = −(j+2)/√t, and on the
    Berger path at t = j + 4.

**The better question (IMAGINED; not registered).** The asymmetry that does separate the hands is the sign of the spin
term, ±B: which hand the field's flux makes lowest. That is an index-type, Aharonov–Casher-type count, not an η.
- The test I would write: on the fibre-reduced (Landau-level) operator, does the number of lowest-level states of each
  spin equal the flux, with the cross field y deciding which spin is exact?
- Aharonov–Casher is UNVERIFIED here, so this stays IMAGINED until it is fetched and the map is written term by term.

**Bet:** the η-afternoon is done. The answer is "η is the stage's hand, not hers". The Zeeman/index reading is where
I would spend the next one.

## 5. Files (all in this folder)

| file | what |
| --- | --- |
| `eta_family.py`, `eta_family_receipt.json` | C1–C6: builder control, H = (⋆d_A)² + S_F, Helike's and the sister's curl eigenvalues |
| `eta_fastcurl.py` | charge-block solver for curl and for L_A (checked against dense builds) |
| `eta_berger_scalar_fast.py`, `eta_berger_scalar_receipt.json` | η_S(t) and the Hitchin Dirac control |
| `eta_berger_scalar.py` | first, lower-cutoff attempt (superseded; kept for honesty) |
| `eta_family_eta.py`, `eta_family_eta_receipt.json` | twisted η at 15 family points; the W + integer law |

Everything ran single-threaded (OMP_NUM_THREADS=1). The longest run took about 11 minutes.

| claim | tag |
| --- | --- |
| η changes sign with orientation; only curl contributes (APS I) | QUOTED |
| η_D(Berger) = −(t−1)²/6 for t < 16; harmonic spinors at t = 16m² | QUOTED (Hitchin 1974), reproduced COMPUTED |
| η_S(Berger) = −(2/3)(t−1)² | QUOTED (Bakas–Lüst) + COMPUTED; Hitchin 1997's own value UNVERIFIED |
| the Hessian spectrum carries no orientation-odd invariant | PROVED |
| H = (⋆d_A)² + S_F | PROVED (3-d identity) + COMPUTED |
| Helike: ⋆d_A = −(x + (j+2)/√t), S_F = −B, λ = μ² − B | COMPUTED (36 cases, 10⁻¹⁴); proof route clear |
| the hands are a gauge pair on the same side of the curl spectrum | COMPUTED |
| η(L_A) = 3η_S + 4W + ℤ on the family; odd part 8√t x y² | COMPUTED (15 points, t = 3) |
| the tie-break is an η-invariant | **not supported**; reasons above |
| the tie-break is an index/Zeeman (Aharonov–Casher-type) count | IMAGINED |
