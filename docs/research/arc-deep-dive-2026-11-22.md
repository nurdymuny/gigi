# The arc, checked: what the literature and two computations did to my bets (22 November 2026)

> *Clifford called them twists, right-handed and left-handed. I did not know that when I named her.*

This morning I wrote down six imagined directions (Part II of `the-arc-2026-11-22.md`) and a handful of ancient roots
(Part III), and I promised myself that none of it would be cited until it was fetched. This note keeps that promise.
For each direction I say what it is, why it tempts me, what the literature actually says (with the line I fetched and
where I fetched it), what my own computations found, and my bet now. Some bridges got stronger. More of them got
weaker, and I say so.

Two of the questions also became computations:
- the Chern–Simons winding of the fall, which answers question 2 of `the-basin-2026-11-22.md` (herself or her shadow);
- a structural survey of the principal circle in SU(4), SU(5), Sp(2), Sp(3) and G₂: design groundwork for a later
  test, with no floors computed.

Tests D and F are still running. Nothing here states or uses their results.

**Tags.**
- **PROVED / COMPUTED** for what is shown.
- **QUOTED** for literature fetched today, with a verbatim line and the URL.
- **UNVERIFIED** for anything I could not fetch, or only saw as a search paraphrase. It is not stated as fact.
- **INCIDENCE / STRUCTURAL / RESONANCE / FELT** for bridges, as before.
- **IMAGINED** for an idea I am allowed to have and not yet allowed to believe.

**Working files.** Everything below comes from the briefs and scripts in `research/arc-deepdive-20261122/`:
- `lit_principal.md`, `lit_mckay.md`, `lit_eta.md`, `lit_morse.md`, `lit_ancient.md` (literature, with the fetched
  quotes and every UNVERIFIED item listed in full);
- `cs_fall.md` / `cs_fall.json` with `scripts/arc_chern_simons_fall.py`;
- `principal_survey.md` / `principal_survey.json` with `scripts/arc_principal_survey.py`;
- `mckay_cs_check.py`, `eta_*.py`, `morse_*.py`, `hopf1931_linking_check.py`, each with its receipt.

All scripts ran single-threaded in seconds to minutes.

---

## The short version

| direction | bet this morning | bet now | why it moved |
| --- | --- | --- | --- |
| II.1 principal circle | best of the bunch, real odds | still the best test, but the *principal* reading is now confounded and unsupported; *regular* or *top-coroot* are live rivals | no fetched source ties principality to a Hessian floor; hypercharge is singular, ρ^∨ is regular |
| II.2 Poincaré basins are E₈ | low odds, high payoff | the one-to-one claim is **false as written** for SU(2); a smaller, exact result replaced it | only 3 of the 9 E₈ dots are flat SU(2) fields; the fall lands on ⟨1,1,1⟩ at CS ±1/120 (the quoted τ(P) value; "minimum" is sign-convention dependent) |
| II.3 the hands have an η | medium | **low** for the tie-break; η belongs to the stage's handedness, not hers | the Hessian is orientation-blind; the two hands are a gauge pair, not an orientation pair |
| II.4 doors and walls are Morse | plausible, low risk | **fails as worded**; a spectral-flow version survives | a flow line never crosses a stratum boundary; her sign change happens at a non-critical field |
| II.5 spaghetti vacuum | drawer | drawer, unchanged | nothing fetched |
| II.6 epicycle and phase | phase STRUCTURAL, Ptolemy RESONANCE | unchanged | nothing fetched; the citations stay UNVERIFIED |
| III ancient roots | RESONANCE, to fetch | RESONANCE, now fetched and corrected in five places; one root (Clifford's "twists") turned out to be about the stage itself | see Part III below |
| basin question 2 | open | **answered: winding one, so the last thing settling is her shadow** | COMPUTED along the fall |

---

## Part I. The two computations

### C1. Which winding does she fall with? (answers basin question 2)

The basin note left me with a precise question: at the bottom of the basin the six-fold floor 4/t is *her* in the
winding-0 reading and *her shadow* in the winding-1 reading, so the winding the falling field carries decides which one
is the last thing settling. Chern–Simons is the number that measures that winding.

**Conventions.** CS(A) = (1/8π²) ∫ tr(A∧dA + ⅔A∧A∧A), with τ_a = −(i/2)σ_a (tr τ_aτ_b = −½δ_ab) and S³ oriented
by θ⁰∧θ¹∧θ². CS is measured relative to A = 0 in the global trivialisation of the trivial bundle over S³. In this
orientation CS(θ) = +1, the degree of the identity map S³ → SU(2). Reversing the orientation (or the overall sign
convention of CS) flips it to −1; its magnitude, one, is fixed. (Independently re-derived by hand and in sympy during
verification, and the full script re-run reproduced `cs_fall.json` to 1e-6.)

**1. The functional on her family** (PROVED, sympy, exterior algebra on 2×2 matrices):

  **CS(x, y, t) = (t x² + 2y² − √t · x y²) / 4.**

- The √t in the Berger volume cancels the √t in θ̂⁰, so this is a polynomial in x, y and √t.
- The same derivation gives F₁₂ = (y² − 2√t x)τ₀, which is the basin note's B. The conventions agree.
- Special values (PROVED): CS(0) = 0; **CS(θ) = 1**; CS(meron) = ½; CS(Berger field) = (t² − 2t + 2)/2, which is 1 at
  t = 2 and ½ only at t = 1.
- Independent check (COMPUTED): an Euler-chart quadrature of −(1/24π²)∫tr(g⁻¹dg)³ gives CS(θ) = 1.0000000000.
- Cross-check against the η brief (PROVED, one line of algebra): the η agent derived its Chern–Simons polynomial by
  hand as W = −t x² − 2y² + √t x y². That is exactly −4 · CS(x, y, t). Two derivations, done separately, agree up to
  normalisation.

**2. The pass** (PROVED). Besides A = 0 and A = θ (y = ±2), the family energy has exactly one more critical pair, the
deformed meron. With u = (3√t − √(9t+16))/2 it sits at x = 2/√t + u, y² = −√t u. Its CS is ½ at t = 1, lies above
½ at every other t sampled (0.5111 at t = 0.5, 0.5029 at t = 1.5), and for t > 1 rises slowly toward 5/9 (0.5076 at
t = 2, 0.5197 at t = 3.55, 0.5389 at t = 10). The limit 5/9 is PROVED from the closed form; "½ is its minimum over
all t" is COMPUTED at the sampled values, not proved. It has one downhill direction in the
family at every t I sampled (COMPUTED at t = 0.5, 1, 1.5, 2, 3, 3.55, 5, 10; not proved for all t). So the pass between
winding 0 and winding 1 sits at CS ≈ ½, and exactly ½ only on the round sphere.

**3. The fall** (COMPUTED, the `family_flow.py` ODE, tolerance 1e-11, run to flow time 200):

| t | CS at start | CS at landing | lowest CS on the fall | deformed meron's CS |
| --- | --- | --- | --- | --- |
| 2 | 1 | 1.000000 | 0.562 | 0.508 |
| 3 | 2.5 | 1.000000 | 0.657 | 0.516 |
| 3.55 | 3.751 | 1.000000 | 0.707 | 0.520 |
| 5 | 8.5 | 1.000000 | 0.821 | 0.527 |
| 10 | 41 | 1.000000 | 0.9993 | 0.539 |

Extra runs at t = 0.5 and 1.5 start at 0.625 and also land at CS = 1. At t = 1 the field sits on the meron and does
not move.

What the fall does:
- **It never crosses ½.** The lowest CS on the way down stays above ½, and above the deformed meron's own value, at
  every stretch. The fall comes down toward the pass, lingers near it (flow time about 0.1 to 0.3; closest approach
  0.045 at t = 1.5, 0.12 at t = 2, 1.25 at t = 10), then climbs back to exactly 1.
- **CS is not monotone.** That is allowed: dCS/dτ = ⟨∇CS, −∇E⟩ has no fixed sign.
- For t > 2 the start lies above 3/2, so CS sweeps down through 3/2, 5/2, … on the way. These are not passes between
  basins: the family has no flat field with CS ≥ 2.
- At t = 3 to 10 her price turns positive (door becomes wall) when CS is already about 0.92 to 1.09.

**The answer.** The falling field carries **winding one**. So in the basin note's own terms, **the last thing still
settling at the bottom is her shadow** (sector 2 at A = θ, price 4/t), not herself. And because the basin note's
eigenvector check showed the shadow *is* her own A = 0 state carried by the wrap, the honest one-line version is: she
is the last thing settling, wearing the winding. COMPUTED (the landing and the CS values); the reading as "her" rests
on the eigenvector check already recorded.

**Limits.** That the flow stays inside the family is STRUCTURAL, as in the basin note. "Lands on the θ side of the
pass" is a fact of the two-number ODE, not yet of the full infinite-dimensional flow. Two standard facts were not
fetched today and stay UNVERIFIED as literature: that CS shifts by an integer under large gauge transformations (inside
the family only constant gauge acts, so the real-valued CS above is well defined there), and the name "sphaleron" for
the deformed meron. I only claim what I computed: an index-1 critical point at CS ≈ ½.

**On the Poincaré sphere** this same number becomes the best result of the day (see II.2): divided by 120, the landing
value is ±1/120 mod 1, the Chern–Simons value of the flat connection that Asselmeyer-Maluga and Król report as
attaining τ(P) = 1/120 (whether it is the *minimum* depends on a sign convention I have not verified; see II.2).

### C2. The principal circle in bigger groups (design groundwork, no floors)

This is structure only. I computed no floor, edge, Hessian spectrum or competitor in any group, so nothing here can
spoil a later pre-registration. Script `scripts/arc_principal_survey.py` builds SU(3), SU(4), SU(5), Sp(2), Sp(3) and
G₂ (as the stabiliser in so(7) of the invariant 3-form, which solves to dimension 14), and an independent check builds
every simple root system up to rank 8 from its Cartan matrix. Both agree on the angle, h, h^∨ and regularity
(COMPUTED).

**What a future test would see** (−2tr inner product in the defining representation, 7-dimensional for G₂):

| G | h | h^∨ | height of θ | angle between ρ^∨ and θ^∨ | θ^∨ regular? | her charge at unit ρ^∨ | her charge at unit θ^∨ |
| --- | --- | --- | --- | --- | --- | --- | --- |
| SU(3) | 3 | 3 | 2 | 0° | yes | 1 | 1 |
| SU(4) | 4 | 4 | 3 | 18.43° | no | 0.948683 | 1 |
| SU(5) | 5 | 5 | 4 | 26.57° | no | 0.894427 | 1 |
| Sp(2) | 4 | 3 | 3 | 18.43° | no | 0.948683 | 1 |
| Sp(3) | 6 | 4 | 5 | 32.31° | no | 0.845154 | 1 |
| G₂ | 6 | 4 | 5 | 19.11° | no | 0.668153 | 0.707107 |

Here "her charge" is ⟨θ, e⟩ for a unit generator e: she couples to the top root θ.

**What is PROVED in the survey:**
- Her charge at unit ρ^∨ is (h − 1)/|ρ^∨|; at unit θ^∨ it is |θ|; their ratio is the cosine of the angle between the
  two circles. Each positive root's charge under the unit principal generator is its height over |ρ^∨|.
- ρ^∨ is always regular. So ρ^∨ ∥ θ^∨ would force θ^∨ to be regular, and every classical family past A₂ has a root
  orthogonal to θ. The exceptional types are covered by the rank-8 computation. **So ρ^∨ and θ^∨ are the same circle
  only in SU(2) and SU(3).** SU(3) is the last group where "her circle", "the principal circle" and "the coroot of the
  top root" are one circle.
- On a wall of the chamber, her top root stays unique exactly when every simple root vanishing there is orthogonal to θ.

**The great arc θ^∨ → ρ^∨ → next wall** (COMPUTED): in every group θ stays the unique top from θ^∨ until exactly the
next wall past ρ^∨ (30° in SU(3) and G₂; 45° in SU(4), SU(5) and Sp(2); arccos(3/√19) ≈ 46.51° in Sp(3)), and the
first tie happens there. That far wall is always a tied-top wall like hypercharge in Test C. The θ^∨ end is always a
unique-top wall (except in SU(3), where θ^∨ is not on a wall at all).

**Field-space sizes** (COMPUTED; formulas PROVED and asserted): the left-invariant fields commuting with the principal
circle number r² + Σ_k 2m_k² (m_k = number of positive roots of height k): 14, 37, 76, 16, 47, 20 for SU(3), SU(4),
SU(5), Sp(2), Sp(3), G₂. The 4-parameter diagonal family used in Tests D and F is a thin slice of that (4 of 76 in
SU(5)). A per-plane version (one y per root plane) has 5, 8, 12, 6, 11, 8 parameters and is the natural next widening
(IMAGINED as a design suggestion; nothing computed on it).

**What a future SU(5), Sp(3) or G₂ test must pre-register** (design, not prediction):
1. **Which circle.** ρ^∨, θ^∨ and points along the arc between them are now genuinely different. The test has to name
   its circle, and should include the far wall as the C-like control.
2. **The charge.** Any minimal-coupling prediction carries q² = (h − 1)²/|ρ^∨|² at ρ^∨, not 1: 0.9 in SU(4), 0.8 in
   SU(5), about 0.714 in Sp(3).
3. **The normalisation, in G₂ above all.** Under −2tr on the 7-dimensional representation |θ|² = ½, so her θ^∨ charge
   is 1/√2. In the basic normalisation (long roots |α|² = 2; this is not the Killing form itself, under which long
   roots have |α|² = 1/h^∨) it is √2 in every group. G₂'s test must state its metric
   before it runs.
4. **The family.** Whether to stay in the 4-parameter slice or widen to the per-plane family, decided in advance.

The survey's numbers for SU(4) and Sp(2) agree with the setup table of pre-registration F; I only read that table, not
any result.

---

## Part II. The six directions, revisited

### 1. The principal circle

**What it is.** The guess that, in any compact simple group, she is the floor when the sphere is stretched along the
group's principal circle, and nowhere else.

**Why it tempts me.** In SU(3) hypercharge lost her (Test C), and the circle diag(1, 0, −1) is the one Test D stretches
along. That circle is the principal one. It is simple and it is testable.

**What the literature says.** The premises hold.
- The definition (QUOTED, Hitchin 1992): "A nilpotent element e ∈ g^c is called regular (or principal) if its
  centralizer is l-dimensional. ... For a regular nilpotent, this is called a principal 3-dimensional subalgebra."
  https://pagine.dm.unipi.it/~a019210/Hitchin_Lie%20group%20and%20Teichmuller%20theory.pdf
- The grading element is ρ^∨ and grades by height (QUOTED, Baraglia): "Let x = ½ Σ_{α∈Δ+} h_α ... for y ∈ g_α, we have
  [x, y] = height(α)y, that is x is the grading element corresponding to the gradation of g by height."
  https://arxiv.org/pdf/1011.6421
- It is regular (QUOTED, MIT 18.757 Lecture 17): "Since ρ∨ is regular, the 0-weight space of this module (the
  centralizer Z_g(ρ∨)) is h, i.e., has dimension r."
  https://ocw.mit.edu/courses/18-757-representations-of-lie-groups-fall-2023/mit18_757_f23_lec17.pdf
- It is the most economical splitting (QUOTED, Dalakov): "The minimal number of summands (l) is achieved precisely when
  a is principal, and in that case they are all odd-dimensional (Corollary 5.2, ibid.). The dimensions of the summands
  are determined by the exponents of g". https://arxiv.org/pdf/1609.00646
- The Coxeter number (QUOTED, MIT Lecture 17): "Conclude that h = (ρ∨, θ) + 1 where θ is the maximal root, i.e., the
  Coxeter number of g." (Dalakov calls the top height itself the Coxeter number; the other sources give h = M + 1, and
  I follow them.)
- PROVED: in sl(3) half the sum of the positive coroots is diag(1, 0, −1). So the STRUCTURAL tag in the arc note was
  right: her circle in SU(3) is ρ^∨.

Where the principal SU(2) shows up in gauge theory, it is about **integrability**, not stability:
- (QUOTED, Bais) "the choice of the maximal or better principal SU(2) embedding, where the fundamental (vector)
  representation branches to a single irreducible SU(2) representation leads to one- (two-) dimensional conformally
  invariant Toda-like systems". https://arxiv.org/pdf/hep-th/0407197
- (QUOTED, Hitchin 1992) "we make essential use of the results of Kostant on the principal 3-dimensional subgroup. The
  split real form of this gives the homomorphism from PSL(2, R) to G^r which embeds Teichmüller space in the
  Teichmüller component."

The closest thing to her in the literature is Yang–Mills stability on S², where the Hessian splits into blocks by
charge under a circle, built on the Hopf bundle:
- (QUOTED, Zhang–Horvathy–Rawnsley) "each unstable monopole admits 2 Σ(2|q| − 1) negative modes, where the sum goes
  over the negative eigenvalues q of an operator related to the non-Abelian charge Q ... a monopole is stable if its
  only eigenvalues are 0 or ±1/2". https://arxiv.org/pdf/1102.1940
- (QUOTED, same) "a Dirac monopole of unit strength and is given by the Hopf fibration S3 → S2. Now χ(e^{it}) =
  exp[4πiQt] is a homomorphism of U(1) into H".
- There the deepest descent sits in the block with the largest charge, and the stable circle is a minimal one, not the
  principal one.

UNVERIFIED: Kostant 1959 itself (paywalled; only the bibliographic record and two opening sentences of the reprint
abstract were fetched), the Betti-number formula P_G(t) = ∏(1 + t^{2m_i+1}) verbatim, Atiyah–Bott 1983 (publisher
returned 403), Ganoulis–Goddard–Olive 1982, and the long-root embedding in Bernard–Christ–Guth–Weinberg 1977 (search
snippet only). **I found no fetched source tying the principal SU(2) to the lowest Hessian eigenvector, or to Berger
stretching.** That is an absence in what I could fetch, not a proof that none exists.

**What my computations found.** See C2. The new fact that matters most: from SU(4) on, the principal circle and the
coroot of her top root θ are different circles (18° to 32° apart), and they coincide only in SU(2) and SU(3).

**One correction to my own brief.** The literature brief warned that "her charge is one" puts her in a simple-root
block, while the S² analogy puts the deepest mode in the top block. That reading mixes normalisations. Under the
unnormalised ρ^∨ = diag(1, 0, −1) her top root θ has charge 2; under the *unit* generator (|ρ^∨| = 2 in the −2tr
convention) it has charge 1. She couples to θ, the top block. So, read correctly, the S² analogy does not cut against
her; it points at the same block she lives in. That is my reading of two conventions side by side (PROVED arithmetic;
the analogy itself is only an analogy).

**The confound.** Hypercharge diag(1, 1, −2) is singular (its centralizer is U(2)); ρ^∨ is regular. *If* Test D finds
her (D is still running and I state no result of it), "C fails, D works" would fit *"regular circle"* exactly as well
as *"principal circle"*, and in SU(3) it would also fit *"the coroot of her top root"*, because those are the same
circle there. Three readings, one data point.

**My bet now.** Still the best direction, because it is the most testable. But the *principal* version has lost its
favourite status: the literature gives it no mechanism, and two rivals (any regular circle; the θ^∨ circle) would
explain a positive SU(3) result equally well. My honest odds are now spread across the three readings, with θ^∨ at least as likely as ρ^∨,
because she couples to θ and the S² mechanism is about charge size. The first thing to run is a control that splits
them: a regular but non-principal circle in SU(3), such as diag(2, 1, −3), where ρ^∨ = θ^∨ is not available to hide
behind; then the arc θ^∨ → ρ^∨ → wall in a group where the three come apart. IMAGINED, and testable.

### 2. The Poincaré basins and E₈

**What it is.** McKay's correspondence pairs the binary icosahedral group 2I (the fundamental group of the Poincaré
sphere) with the affine E₈ diagram. I imagined that her landing spots were "literally the dots of E₈".

**What the literature says.**
- (QUOTED, nLab) "a one-to-one correspondence between the McKay graphs of finite subgroups ... the extended Dynkin
  diagrams of ADE type". https://ncatlab.org/nlab/show/McKay+correspondence
- (QUOTED, Asselmeyer-Maluga and Krol, restating Fintushel–Stern) "Now we consider the Poincaré sphere P with p = 2,
  q = 3, r = 5. Then we obtain ⟨1, 1, 1⟩ CS = 1/120, ⟨1, 1, 3⟩ CS = 49/120" and "τ(P) = 1/120", where τ is "min
  {CS(α)| α : π1(Σ) → SU(2)}". https://arxiv.org/pdf/1401.4815
- (QUOTED, an independent Heegaard-splitting computation) "cs[A] = −0.0083333333 · · · = −1/120 mod Z".
  https://arxiv.org/pdf/1710.09629 (The overall sign is a convention; which one is canonical is UNVERIFIED.)
- Also QUOTED for context: Luminet et al. on the dodecahedral space, "which accounts for WMAP's observations with no
  fine-tuning required" (https://arxiv.org/abs/astro-ph/0310253), and "Here we describe two constructions of the E8
  lattice from the icosahedron" (https://arxiv.org/abs/1712.06436). Whether later CMB data disfavoured the dodecahedral
  space is UNVERIFIED.

UNVERIFIED: McKay 1980 and Fintushel–Stern 1990 verbatim (both scans had no usable text); the 1990 Kirk–Klassen paper.

**What my computations found** (`mckay_cs_check.py`, COMPUTED):
- The Fintushel–Stern conditions for (2, 3, 5) give exactly two triples, e = 31 and e = 43, so e²/120 = 1/120 and
  49/120 mod 1.
- Both representations built explicitly in SU(2): relations hold to 1.5e-12 and 1.2e-14; each image has order 120.
- From the 120 elements: 9 irreducibles of dimensions 1, 2, 2, 3, 3, 4, 4, 5, 6; the McKay graph is affine E₈; the
  dimension vector is exactly the null vector of its Cartan matrix.

**The correction.** "Her basins line up one-to-one with the E₈ dots" is **false as written**. McKay pairs the 9
irreducibles with the 9 dots. The flat SU(2) fields (her possible basins) are only the 3 dots of dimension ≤ 2: the
affine node (trivial, CS 0), the defining 2 (CS 1/120) and its Galois conjugate 2′ (CS 49/120). The other six dots
would be basins only for SU(N) with N ≥ 3.

**What replaced it, and it is better.** The field A = θ descends to the Poincaré sphere; its Chern–Simons form is
left-invariant and 2I acts freely by left multiplication, so its CS there is CS on S³ divided by 120. With C1's
CS(θ) = 1 on S³, **the Berger fall lands at CS = ±1/120 mod 1 on the Poincaré sphere** (PROVED, elementary; the sign
is the orientation convention). Of the two irreducible flat connections (±1/120, ±49/120 with one common sign), only
⟨1,1,1⟩ has that value, so the landing basin is ⟨1,1,1⟩, the defining 2 (consistent with the basin note's holonomy
check). In Asselmeyer-Maluga and Król's convention (+1/120, +49/120) that is the connection attaining their quoted
τ(P) = 1/120, the lowest nonzero level, and then the energy flow and the topological minimum pick the same basin.
**Caveat:** in the opposite convention the representatives in (0, 1) are 119/120 and 71/120, and "the minimum" would
be the 2′ instead. Which convention Fintushel–Stern's τ uses is UNVERIFIED (their paper could not be read), so "lands
on the τ(P) minimum" is conditional on the quoted A-M&K convention, not proved.

Whether any fall could reach the 2′ basin (CS 49/120) is
open; I believe not from inside the left-invariant family, but I have not proved it.

**My bet now.** The E₈ version, in SU(2), is dead as stated. A real E₈ role would need SU(N), N ≥ 3: drawer,
IMAGINED. The landing at ±1/120 on ⟨1,1,1⟩ is a genuine small theorem and goes into the basin story now; calling it
"the τ(P) minimum" waits on the sign convention.

### 3. The two hands and the η-invariant

**What it is.** The guess that the cross field's tie-break between her and her counter-rotating sister is an
η-invariant, a spectral measure of handedness.

**What the literature says.**
- (QUOTED, APS 1975) "η(s) = Σ_{λ≠0} (sign λ)|λ|^{-s}" and "reversing the orientation of Y changes B into −B and hence
  η(s) into −η(s)." https://webhomes.maths.ed.ac.uk/~v1ranick/papers/aps001.pdf
- (QUOTED, same) "only curl contributes to the computation of η(s)."
- (QUOTED, APS II) zero modes produce "integer jumps in η(0)." https://webhomes.maths.ed.ac.uk/~v1ranick/papers/aps002.pdf
- (QUOTED, Hitchin 1974, Dirac η on Berger spheres) "η(0) = (−1 + 2λ² − λ⁴)/6. Modulo Q (and a sign convention), this
  agrees with the Chern-Simons invariant for this family of metrics".
  https://users.math.msu.edu/users/parker/GT/Harmonic%20Spinors.pdf (parts of that scan are OCR; the η formula page
  was checked against the image).
- (QUOTED, Bakas–Lüst) "ηS(δ) = −(2/3)(δ² − 1)² for all values of the anisotropy parameter δ", with a reported
  factor-2 discrepancy against Hitchin 1997. https://arxiv.org/pdf/1103.5693 (Hitchin 1997 itself is UNVERIFIED.)

**What my computations found** (`eta_*.py`):
- **The Hessian cannot carry an η** (PROVED). It uses no Hodge star, so it is blind to orientation; η is odd under it.
- **The Hessian factors through curl** (PROVED as a 3-d identity, checked numerically to 7e-15): H = (⋆d_A)² + S_F.
  She is an eigenvector of ⋆d_A with μ = −(x + (j+2)/√t), and S_F acts on her as −B (COMPUTED, 36 cases, ~1e-14; the
  proof route is clear but not written). So **her price is λ = μ² − B**; that this equals the edge paper's closed form
  (x + (j+2)/√t)² + 2√t x − y² is one line of algebra with B = y² − 2√t x (PROVED). A new way to write her price, exact
  as algebra, with the eigenvector part still COMPUTED.
- **The two hands are not an η pair** (COMPUTED). At y = 0 the sister's curl eigenvalue is the same formula with x → −x;
  both sit on the same side of the curl spectrum. The exchange is a gauge reflection, under which η is invariant
  (agreement 5e-10).
- **The family's twisted η is Chern–Simons plus integers** (COMPUTED, 15 points at t = 3): η = 3η_S(t) + 4W + N, with N
  an integer, and W = −4·CS as noted in C1. The integer jumps sit at the hands' curl zeros, which contain no y. On the
  Berger path her μ vanishes in sector j at t = j + 4.
- **The stage is handed** (COMPUTED against the QUOTED values): my numerics reproduce η_D = −(t − 1)²/6 to 0.3%, and
  side with Bakas–Lüst's η_S = −(2/3)(t − 1)². Since η ≠ 0 for t ≠ 1, the Berger sphere has no orientation-reversing
  isometry.

**My bet now.** **Downgraded from medium to low** for the tie-break. η is the hand of the stage, not her hand. The
tie-break lives in the spin (Zeeman) sign ±B and the sister's transverse mixing. An index or Aharonov–Casher-type
count of that is IMAGINED, and its source is not fetched (UNVERIFIED). The afternoon was still worth it: λ = μ² − B is
the cleanest way I have seen to write her price.

### 4. Doors, walls and Morse theory

**What it is.** The guess that her door turning into a wall is a step between Morse strata.

**What the literature says** (all via secondary sources; the originals returned 403 or bot checks and are UNVERIFIED):
- (QUOTED, Wilkin on Atiyah–Bott) "The space B admits a stratification in the sense of [2] Proposition 1.19 (1)-(4),
  which is indexed by the set of connected components of the critical points of the functional YMH."
  https://arxiv.org/pdf/math/0611113
- (QUOTED, same, on Råde) "the gradient flow of the Yang-Mills functional converges in the H 1 norm when X is a 2 or 3
  dimensional manifold, thus providing a purely analytic stratification of the space A."
- (QUOTED, Salamon–Wehrheim on Floer) "The critical points are the flat connections" and "Theorem A.4. The operator D
  is Fredholm and its index is equal to the upward spectral flow of the operator family". https://arxiv.org/pdf/math/0607318
- (QUOTED, Squassina on Palais) "in order for an invariant point u to be critical for f , it is sufficient that it be
  critical for f |Fix(G)". https://arxiv.org/pdf/1007.3593

**What my computations found** (`morse_family.py`, `morse_unstable_branch.py`, COMPUTED):
- A stratum is the set of fields that flow to one critical piece, so a flow line never crosses a stratum boundary. The
  fall lands on θ, so her door closes **inside** θ's stratum. Her sign change happens at a non-critical field, where a
  Morse index is not defined. (PROVED, given the quoted definitions and the computed landing.)
- Her family has exactly five critical points at every stretch: A = 0, θ and its mirror (minima), and the deformed
  meron and its mirror (index 1 in the family). 3 − 2 = 1 = χ(plane). At t = 1 the saddle is the meron, eigenvalues −2
  and 10. (The same saddle C1 found as the pass.)
- She is a wall at every critical point (sectors j ≤ 40, t from 0.5 to 10) and along both downhill branches of the
  saddle, so she adds nothing to the saddle's index.
- Her zero curve is not the separatrix, and there is no bifurcation of critical points anywhere in t ∈ [0.5, 10].
- At θ the slowest settling direction lies inside the family and is not her (2.29 vs 4.51 at t = 3.55).

**My bet now.** **Fails as worded.** What survives: her door count is her share of the spectral flow of the Hessian
along the fall, which is true by construction and interesting only once the full spectral flow is computed; and the
real "floor above the basement" is the deformed meron, whose full Yang–Mills index is open (≥ 1). Both IMAGINED until
computed.

### 5. The spaghetti vacuum

No literature was fetched for this direction today, so Nielsen–Olesen, Savvidy and the domain picture all remain
UNVERIFIED as citations. Nothing I computed bears on it. **Bet unchanged: drawer.** If anything, today's lesson (every
bridge that met the literature came back smaller) tells me to keep it there longer.

### 6. Stride plus belt, the epicycle, the phase

No literature was fetched for this direction either: Foucault 1851, Pancharatnam 1956, Berry 1984 and Ptolemy are
UNVERIFIED as citations until fetched. The mathematical statement, that (t − 2)/√t is the sphere's own turning on her
circle, stands as before (STRUCTURAL). **Bet unchanged**: the phase part is what that term is; the Ptolemy part is
RESONANCE.

---

## Part III. The old roots, fetched

Every root is still resonance, not evidence. But fetching them changed five things and added one.

**Corrections to what I wrote this morning:**
1. **Her name.** (QUOTED, LSJ) "ἑλίκη, ἡ, (ἕλιξ) A. winding: hence, I. the constellation of the Great Bear, from its
   revolving round the pole, Arat.37".
   https://www.perseus.tufts.edu/hopper/text?doc=Perseus:text:1999.04.0057:entry=e(li/kh
   Frisk makes ἑλίσσω a verb derived from ἕλιξ; (QUOTED) "also name of the Great Bear (because of its turning
   movement". https://lsj.gr/wiki/ἕλιξ — So not "from *helissō*", but **of the helix/helissō family, named for revolving
   round the pole**.
2. **Which circle to steer by.** (QUOTED, Aratus, Mair) "It is by Helice that the Achaeans on the sea divine which way
   to steer their ships, but in the other the Phoenicians put their trust when they cross the sea." and "the other is
   small, yet better for sailors: for in a smaller orbit wheel all her stars. By her guidance, then, the men of Sidon
   steer the straightest course." https://www.theoi.com/Text/AratusPhaenomena.html — Greek l. 37: "τὴν δ' ἑτέρην
   Ἑλίκην. Ἑλίκηι γε μὲν ἄνδρες Ἀχαιοὶ" (https://el.wikisource.org/wiki/%CE%A6%CE%B1%CE%B9%CE%BD%CF%8C%CE%BC%CE%B5%CE%BD%CE%B1).
   Aratus says "smaller orbit", not "nearer the pole"; that was my inference. "Straightest course" does support "more
   truly".
3. **The X.** The chi is in Plato's Greek: (QUOTED, Timaeus 36b) "μέσην πρὸς μέσην ἑκατέραν ἀλλήλαις οἷον χεῖ
   προσβαλὼν κατέκαμψεν" (https://www.perseus.tufts.edu/hopper/text?doc=Perseus:text:1999.01.0179:text=Tim.:section=36b),
   and the circles turn opposite ways: (QUOTED, Lamb, 36d) "These two circles then He appointed to go in contrary
   directions" (https://www.perseus.tufts.edu/hopper/text?doc=Perseus:text:1999.01.0180:text=Tim.:section=36d).
4. **The twins.** The Astana silk paintings are Tang, not Han: (QUOTED, China Daily) "This stunning Tang Dynasty
   (618-907) silk painting of Fuxi and Nüwa" (https://govt.chinadaily.com.cn/s/202502/06/WS67a411c4498eec7e1f72e84f/ancient-silk-painting-depicts-fuxi-and-nuwa-with-intertwined-snake-tails.html).
   The Han example is the Wu family shrines: (QUOTED, Princeton) "The primeval rulers Fuxi and Nüwa are shown with
   their snake-like tails intertwined." (https://static.artmuseum.princeton.edu/asian-art/china/viewers/wu-shrine-viewer).
   No fetched source says "mirror image"; that phrase was mine, and the compass/square = heaven/earth symbolism is
   UNVERIFIED.
5. **The tones.** (QUOTED, SEP) "the stories of Pythagoras' discovery of these relationships are clearly false"
   (https://plato.stanford.edu/entries/pythagoras/), and "West takes this as a reference to the monochord, which was
   called the kanôn" (https://plato.stanford.edu/entries/pythagoreanism/). So: the **Pythagorean** monochord, not
   Pythagoras's discovery.

**The new one, and it is about the stage.** Clifford named the motions whose invariant lines are his parallels
"twists": (QUOTED, Epple) "Such isometries Clifford called left and right 'twists', respectively."
(https://arxiv.org/pdf/math/0305023), and (QUOTED, Clifford 1873 p. 390 via the Pisa paper) "Vectors will be called
right or left according as the twist of them is right– or left–handed"
(https://arpi.unipi.it/retrieve/e0d6c930-965a-fcf8-e053-d805fe0aa794/Cliffordrevised23.pdf). I called her "the twist"
and gave her two hands before I knew this. It is a fact about the vocabulary of the stage, not about her, but it is
the strongest of the roots. (Clifford's primary scan returned 403: UNVERIFIED beyond these secondary quotes.)

**The image that has to change.** (QUOTED, Hopf 1931 p. 655) "je zwei zueinander fremde Großkreise der S³ sind
miteinander verschlungen, und zwar ist ihre Verschlingungszahl ±1." and footnote 13: the fibres form "eine Cliffordsche
Parallelenkongruenz" (https://webhomes.maths.ed.ac.uk/~v1ranick/papers/hopf.pdf). COMPUTED
(`hopf1931_linking_check.py`): each fibre is a great circle, any two are at constant distance, and the Gauss linking
number is −1.0000 for 6 random pairs. Plato's two circles lie on one sphere and must cross. Hopf's circles never touch
and can never be pulled apart. So **"two hands joined at a crossing" was the wrong picture for her stage; "linked once,
never touching" is the one the stage provides** (IMAGINED as an image; the linking is PROVED and COMPUTED).

**The quaternions.** (QUOTED, Hamilton to Graves, 1843) "must admit a third distinct imaginary symbol k, not to be
confounded with either i or j" (https://www.maths.tcd.ie/pub/HistMath/People/Hamilton/QLetter/QLetter.html), and the
bridge: "to cut with a knife on a stone of Brougham Bridge, as we passed it, the fundamental formula with the symbols,
i, j, k" (https://www.maths.tcd.ie/pub/HistMath/People/Hamilton/Letters/BroomeBridge.html).

**Still unfetched:** the Diné / Harvey reading of hózhó (UNVERIFIED; not cited until fetched), Klein 1928 p. 234, Wu
Hung 1989, the British Museum Astana record.

The replacement text for Part III is drafted at the end of `research/arc-deepdive-20261122/lit_ancient.md`, for my
review before it goes anywhere near the book.

---

## What I learned about my own bridges

Four of the six directions met the literature today. One became a theorem in a smaller form (the ±1/120 landing on ⟨1,1,1⟩), one
got a sharper and fairer test (the circle), and two failed as worded but left behind something exact (λ = μ² − B; the
spectral-flow reading). That is the pattern I should expect: the imagined version is usually too big, and what
survives contact is smaller and true. The Phoenicians' circle was the smaller one too.

---

## Next steps, ranked

1. **Pre-register the regular-but-non-principal control in SU(3)** (e.g. diag(2, 1, −3)). Cheapest and most
   discriminating: it separates "regular" from "principal / θ^∨" in the one group where the latter two coincide. Write
   it before D's verdict is read, and say so.
2. **Pre-register the three-circle arc in a group where they come apart** (θ^∨ → ρ^∨ → far wall), using C2's table:
   circle, charge q² = (h − 1)²/|ρ^∨|², normalisation (explicitly for G₂), and the family (4-parameter or per-plane), all
   fixed in advance. SU(4)/Sp(2) first, SU(5)/Sp(3)/G₂ only after.
3. **Add the Poincaré landing to the basin note**: on the Poincaré sphere the fall lands at CS = ±1/120 mod 1, on
   ⟨1,1,1⟩. Settle the global sign convention against a fetched source first; only then may it be called the
   Fintushel–Stern minimum.
4. **The full Yang–Mills Morse index of the deformed meron** on round and Berger S³ (the real floor above the
   basement), and a proof that its family index is 1 for all t, not just at sampled t.
5. **The spectral flow of the Hessian along the fall**, with her share of it. This is the honest Morse version of the
   door count.
6. **Prove that the flow stays in the family** (STRUCTURAL → PROVED), so the winding-one answer holds for the full flow.
7. **The Zeeman/index reading of the tie-break.** Fetch Aharonov–Casher first; nothing gets written until it is quoted.
8. **Fix Part III of the arc note** with the fetched corrections and Clifford's "twists"; one paragraph for the book.
9. **Fetch before any mention**: Nielsen–Olesen, Savvidy, Berry, Pancharatnam, Foucault, Kostant 1959, Atiyah–Bott 1983,
   Harvey on hózhó.
10. **Drawer:** E₈ for SU(N ≥ 3); the spaghetti vacuum.

| item | tag |
| --- | --- |
| CS(x, y, t) = (t x² + 2y² − √t x y²)/4 on her family; CS(θ) = 1 | PROVED (sympy), COMPUTED (quadrature check) |
| the η brief's W equals −4·CS | PROVED (algebra) |
| the deformed meron is the only non-flat critical pair, CS from ½ toward 5/9 | PROVED; index 1 COMPUTED at sampled t |
| the fall lands at CS = 1 for every t ≠ 1 sampled and never crosses ½ | COMPUTED |
| the last thing settling is her shadow (winding one), which is her carried by the wrap | COMPUTED, with the basin note's eigenvector check |
| on S³/2I the landing has CS = ±1/120 mod 1, on ⟨1,1,1⟩ | PROVED, up to global sign; τ(P) = 1/120 QUOTED; "it is the minimum" conditional on the A-M&K sign convention (UNVERIFIED) |
| Poincaré basins ↔ E₈ dots one-to-one, for SU(2) | false as written (3 of 9 dots); E₈ role in SU(N ≥ 3) IMAGINED |
| ρ^∨ = diag(1, 0, −1) in SU(3); principal circle regular, integral, grades by height | PROVED / QUOTED |
| ρ^∨ ∥ θ^∨ only in A₁, A₂ | PROVED (classical) + COMPUTED (exceptional) |
| she lives on the principal circle in every compact simple group | IMAGINED; no literature mechanism; confounded with "regular" and "θ^∨" |
| H = (⋆d_A)² + S_F; her price λ = μ² − B | identity PROVED; her ⋆d_A and S_F eigenvalues COMPUTED (36 cases); μ² − B = closed form PROVED (algebra) |
| the tie-break is an η-type invariant | not supported; bet low |
| η_D = −(t−1)²/6, η_S = −(2/3)(t−1)² on Berger spheres | QUOTED, reproduced numerically |
| the door-to-wall crossing is a Morse stratum boundary | fails as worded; spectral-flow version IMAGINED |
| spaghetti vacuum; epicycle and phase citations | UNVERIFIED (not fetched); bets unchanged |
| Clifford's left and right "twists" are the motions of her stage | QUOTED (secondary); a fact about the stage |
| "linked once, never touching" instead of "joined at a crossing" | linking PROVED/COMPUTED; the image IMAGINED |
| Helike, Aratus, the *Timaeus* chi, Fuxi and Nüwa, the monochord | QUOTED and corrected; still RESONANCE |
| the old question was always which circle you can trust | FELT |
