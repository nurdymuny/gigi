# The principal circle: a structural survey (arc deep-dive, task C2, 22 November 2026)

This is groundwork for Part II.1 of `docs/research/the-arc-2026-11-22.md` ("she only lives on the right circle").
It is **structure only**. I computed no floor, no edge, no Hessian spectrum and no competitor, so nothing here can
spoil a later pre-registration. Everything is Lie-algebra bookkeeping.

- **Script:** `scripts/arc_principal_survey.py`. It extends `scripts/edge_wp7_groups.py` with:
  - Sp(3);
  - G2, as the stabiliser in so(7) of φ = e123 + e145 + e167 + e246 − e257 − e347 − e356, which solves to dimension 14;
  - SU(5), which uses the existing `su<n>` builder.
- **Receipt:** `principal_survey.json`, in this folder.
- **Run:** `OMP_NUM_THREADS=1 python scripts/arc_principal_survey.py research/arc-deepdive-20261122`. It takes seconds
  and is single-threaded.

**Labels.**
- **PROVED:** elementary argument written out here.
- **COMPUTED:** double precision, with asserts in the script.
- **QUOTED:** fetched today, verbatim.
- **IMAGINED:** my reading.

## Conventions (the same as the edge_wp7 code)

- **Inner product.** ⟨X, Y⟩ = −2 tr(XY) in the defining representation. For G2 that is the 7-dimensional one.
- **Torus.** I use an orthonormal torus basis. Roots are the eigenvalues of i·ad on it, viewed as vectors in R^rank.
- **Coroots.** α^∨ = 2α/|α|².
- **Positive system.** It is fixed by a random generic functional, so **the numbering of the simple roots is arbitrary**.
  The json records the Cartan matrix and each simple root's length so the numbering can be read. Below I name walls in
  words.
- **Her charge.** q_K = ⟨θ, e_K⟩ for a unit torus generator e_K. **This number depends on the normalisation.** I give it
  both in the code's −2tr convention and rescaled so that long roots have |α|² = 2.
- **Cross-check.** An independent check builds every simple root system of rank ≤ 8 from its Cartan matrix: A1–A8,
  B2–B8, C3–C8, D4–D8, E6, E7, E8, F4, G2. For the six groups it agrees with the matrix build on the angle, h, h^∨ and
  regularity (COMPUTED).

## Literature (QUOTED, fetched 2026-11-22)

**Brillon & Schechtman, "Coxeter element and particle masses"**, https://arxiv.org/abs/1612.04133, PDF pp. 3–4:
- "Let ρ^∨ ∈ 𝔥 be defined by ⟨α_i, ρ^∨⟩ = 1, i = 1, …, r."
- "It follows that for all α ∈ R ⟨α, ρ^∨⟩ = ht α."
- "h = Σ_{i=0}^r n_i = 1 + ht θ, is the Coxeter number of 𝔤"
- "One starts with a *cyclic element* in the sense of Kostant, [K]". The bracket [K] is their citation.

**Wikipedia, "Coxeter number"**, https://en.wikipedia.org/wiki/Coxeter_number:
- "If the highest root is ∑miαi for simple roots αi, then the Coxeter number is 1+∑mi."
- Table rows: A_n h = "n + 1", h^∨ = "n + 1"; B_n h = "2n", h^∨ = "2n − 1"; G2 h = "6", h^∨ = "4".
- The C_n row did not extract cleanly. **UNVERIFIED** from that page. I computed C3: h = 6, h^∨ = 4.

**Kostant (1959) itself, "The principal three-dimensional subgroup and the Betti numbers of a complex simple Lie
group":**
- I did not fetch the paper's text. The title and the reprint in Kostant's *Collected Papers* (Springer, 2009) appear
  in a search listing only.
- Any statement attributed to the 1959 paper beyond its title is **UNVERIFIED**.
- The statement used here, ⟨α, ρ^∨⟩ = ht α, rests on the Brillon–Schechtman quote and is checked by assert in the
  script for all six groups.

## The table (COMPUTED; −2tr convention unless marked)

| G | dim, rank, \|Φ⁺\| | θ in simple roots | ht θ | h | h^∨ | ∠(ρ^∨, θ^∨) | θ^∨ regular? | q_K unit ρ^∨ | q_K unit θ^∨ | ratio = cos∠ | (basic normalisation, long |α|²=2 (JSON key says "killing"; it is not the Killing form): q at ρ^∨, at θ^∨) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| SU(3) | 8, 2, 3 | α₁+α₂ | 2 | 3 | 3 | **0°** | **yes** | 1 | 1 | 1 | 1.4142, 1.4142 |
| SU(4) | 15, 3, 6 | α₁+α₂+α₃ | 3 | 4 | 4 | 18.4349° | no | 0.948683 | 1 | 0.948683 | 1.3416, 1.4142 |
| SU(5) | 24, 4, 10 | α₁+…+α₄ | 4 | 5 | 5 | 26.5651° | no | 0.894427 | 1 | 0.894427 | 1.2649, 1.4142 |
| Sp(2) | 10, 2, 4 | α_L + 2α_S | 3 | 4 | 3 | 18.4349° | no | 0.948683 | 1 | 0.948683 | 1.3416, 1.4142 |
| Sp(3) | 21, 3, 9 | 2α₁+2α₂+α₃ (α₃ long) | 5 | 6 | 4 | 32.3115° | no | 0.845154 | 1 | 0.845154 | 1.1952, 1.4142 |
| G2 | 14, 2, 6 | 2α_L + 3α_S | 5 | 6 | 4 | 19.1066° | no | 0.668153 | **0.707107** | 0.944911 | 1.3363, 1.4142 |

**θ is the unique top root**, both for the unit principal generator and for the unit θ^∨ generator, in all six groups
(COMPUTED; PROVED below).

**Closed forms** (PROVED from the quoted ⟨α, ρ^∨⟩ = ht α, and matched to 10⁻⁹):
- q_K(unit ρ^∨) = (h − 1)/|ρ^∨|. In this convention |ρ^∨|² = 4, 10, 20, 10, 35, 56.
- q_K(unit θ^∨) = |θ|, since θ^∨ ∥ θ.
- Their ratio is cos∠(ρ^∨, θ^∨).

**Under the unit principal generator, the charges of the positive roots are exactly their heights over |ρ^∨|.** So:
- the second-highest charge over the highest is (h − 2)/(h − 1) = ½, ⅔, ¾, ⅔, ⅘, ⅘;
- the height multiplicities are:

| G | roots at heights 1, 2, … |
| --- | --- |
| SU(3) | 2, 1 |
| SU(4) | 3, 2, 1 |
| SU(5) | 4, 3, 2, 1 |
| Sp(2) | 2, 1, 1 |
| Sp(3) | 3, 2, 2, 1, 1 |
| G2 | 2, 1, 1, 1, 1 |

**Check against pre-registration F §0** (`research/edge-theorem-20261122-f/wp7_prereg_F_principal_circle.md`), which
I am reading and not touching:
- its SU(4) and Sp(2) angle of 18.43° and q_K of 0.949 at ρ^∨ agree;
- its value 1 at θ^∨ agrees;
- its value 0.707 at the 45° wall agrees;
- its "Sp(2)'s ρ at 26.57°" agrees (the json field `angle_rho_theta_vee_deg`).

**The q_K caveat sharpens for G2.** In the code's −2tr convention on the 7-dimensional representation, |θ|² = ½.
So even on θ^∨ her unit charge is 1/√2, not 1, and her x² coefficient would come out ½ where it is 1 in SU(n) and
Sp(n).
- The ratio q(ρ^∨)/q(θ^∨) = cos∠ does not depend on the convention.
- In the basic normalisation (long roots |α|² = 2; not the Killing form, under which long roots have |α|² = 1/h^∨), q(θ^∨) = √2 in every group.
- **Any G2 test has to pre-register which metric normalisation it uses** before the x² coefficient means anything.

## The great arc θ^∨ → ρ^∨, continued to the next wall (COMPUTED)

I parametrise e(φ) = cos φ · û + sin φ · p̂:
- û is the unit θ^∨;
- p̂ is the unit part of ρ^∨ orthogonal to û;
- ρ^∨ sits at φ = ∠(ρ^∨, θ^∨).

**For every group, the segment from φ = 0 to the next wall stays in the closed dominant chamber** (checked at 400
points).

| G | walls at θ^∨ (φ = 0): roots that vanish | her top unique there? | next wall beyond ρ^∨ | roots that vanish there | top tied with | q_K there | ρ^∨ at fraction of segment |
| --- | --- | --- | --- | --- | --- | --- | --- |
| SU(3) | none (θ^∨ regular) | yes | **30°** | one simple root | θ ~ the other simple root (2-fold) | 0.866 | 0 |
| SU(4) | the middle simple root | yes | **45°** | both end simple roots | 4-fold: e_i − e_j, i ∈ {1,2}, j ∈ {3,4} | 0.7071 | 0.41 |
| SU(5) | the inner A₂ (3 positive roots) | yes | **45°** | both end simple roots | 4-fold, the same pattern | 0.7071 | 0.59 |
| Sp(2) | the long simple root | yes | **45°** | the short simple root e₁ − e₂ | 3-fold: 2e₁, e₁+e₂, 2e₂ | 0.7071 | 0.41 |
| Sp(3) | the C₂ in e₂, e₃ (4 positive roots) | yes | **46.5085°** | the end short root e₁ − e₂ | 3-fold: 2e₁, e₁+e₂, 2e₂ | 0.6882 | 0.69 |
| G2 | the short simple root | yes | **30°** | the long simple root | 2-fold: θ ~ α_L + 3α_S | 0.6124 | 0.64 |

Notes on the table:
- **SU(3).** ρ^∨ = θ^∨, so the arc's direction is not fixed by ρ^∨. The script takes the rank-2 perpendicular. Both
  walls lie at 30° and are mirror images; the 30° wall is the hypercharge circle of Test C.
- **Sp(3).** The angle 46.5085° is arccos(3/√19).
- **The rank-2 groups.** In Sp(2) and G2 the segment θ^∨ → wall is the whole Weyl chamber: its opening is 45° and 30°.
  In SU(3), θ^∨ = ρ^∨ bisects a 60° chamber.

**The pattern, which is PROVED below.**
1. Along the arc, θ stays the unique top from θ^∨ right up to the next wall.
2. The first top tie occurs **exactly at** that wall, in every group.
3. The walls at θ^∨ are top-unique walls. The walls beyond ρ^∨ are tied-top walls, like C's hypercharge wall.

**PROVED (the top-uniqueness criterion).** Let e be dominant, so α_i(e) ≥ 0 for all simple roots.
- For any positive root β, θ − β is a non-negative sum of simple roots. So θ(e) ≥ β(e), with equality exactly when
  every simple root in the support of θ − β vanishes at e.
- On the open chamber, therefore, θ is the strict top.
- **On a wall, θ is tied if and only if some simple root α_i vanishing there has ⟨θ, α_i^∨⟩ > 0** (is not orthogonal
  to θ).
  - **If such an α_i exists:** θ − α_i is a root by the α_i-string through θ, and it ties with θ.
  - **If every vanishing simple root is orthogonal to θ:** a tied β has θ − β ⊥ θ. Then (θ, β) = |θ|² ≥ |β|², and
    Cauchy–Schwarz forces β = θ.
- So **her frame top is unique on a wall exactly when every root vanishing there is orthogonal to θ.**
  - The θ^∨ wall is of this kind by construction.
  - The far wall beyond ρ^∨ is not: in all six groups its vanishing simple roots are among those not orthogonal to θ.

This is the structural line between the θ^∨ wall and C's wall that the review of pre-registration F found by hand.

## Where does ρ^∨ = θ^∨ happen? (the SU(3) coincidence)

**COMPUTED** over every simple type of rank ≤ 8: ρ^∨ ∥ θ^∨ only for **A1 and A2**, that is SU(2) and SU(3). The same
two are the only types in which θ^∨ is regular.

The angle grows with rank:

| type | ∠(ρ^∨, θ^∨) |
| --- | --- |
| A3 | 18.43° |
| A8 | 43.09° |
| B8 | 42.05° |
| C8 | 54.88° |
| D8 | 39.02° |
| E6 | 28.27° |
| E7 | 31.67° |
| E8 | 34.56° |
| F4 | 28.27° |
| G2 | 19.11° |

Some angles coincide across types. These are COMPUTED and I read nothing into them:
- A3 = B2;
- G2 = B3 = D4;
- C3 = A5;
- C4 = A7;
- E6 = F4.

**PROVED for all ranks.**
- ρ^∨ is always regular, since ⟨α_i, ρ^∨⟩ = 1 > 0.
- So ρ^∨ ∥ θ^∨ forces θ^∨ to be regular, that is, no root orthogonal to θ.
- That fails in every classical family past A2:
  - A_n, n ≥ 3: e₁ − e_{n+1} ⊥ e₂ − e₃;
  - B_n, n ≥ 3: e₁ + e₂ ⊥ e₃;
  - C_n, n ≥ 2: 2e₁ ⊥ e₂ (this includes B2 = C2);
  - D_n: e₁ + e₂ ⊥ e₃ − e₄.
- The exceptional types are a finite list and are COMPUTED above: θ^∨ is singular in all of them.
- In A2, ρ^∨ = α₁^∨ + α₂^∨ = θ^∨ exactly: diag(1, 0, −1). In A1, ρ^∨ = ½θ^∨, which is parallel, so the circle is the same.

**So SU(3) is the last group where "her circle", "the principal circle" and "the coroot of the top root" are one
circle.** From SU(4) and Sp(2) on, the hypothesis has to choose among them, as pre-registration F already does.

## The field space: commutants and the diagonal sub-family (COMPUTED)

**Field space.** A left-invariant field is a linear map A: 𝔤 → 𝔤, so End(𝔤) has dimension d². "Invariant" means
A ↦ Ad_t A Ad_t⁻¹, computed as a null-space dimension. Which torus the stretched metric actually lets one impose is a
modelling choice, so I report several.

| G | dim End 𝔤 | max torus T | the circle ρ^∨ | the circle θ^∨ | the circle at the next wall | D-style diagonal (x, w, y_θ, y) | per-plane diagonal (x, w, one per root plane) |
| --- | --- | --- | --- | --- | --- | --- | --- |
| SU(3) | 64 | 10 | 14 | 14 | 24 | 4 | 5 |
| SU(4) | 225 | 21 | 37 | 59 | 81 | 4 | 8 |
| SU(5) | 576 | 36 | 76 | 174 | 128 | 4 | 12 |
| Sp(2) | 100 | 12 | 16 | 26 | 34 | 4 | 6 |
| Sp(3) | 441 | 27 | 47 | 155 | 69 | 4 | 11 |
| G2 | 196 | 16 | 20 | 50 | 34 | 4 | 8 |

**Formulas** (PROVED and matched in the script):
- **Maximal torus.** r² + 2|Φ⁺|. Distinct positive roots are not proportional, so each root plane contributes only its
  own 2-dimensional commutant, ℂ.
- **Principal circle.** r² + Σ_k 2 m_k², where m_k is the number of positive roots of height k. This is asserted in
  the script.
- **Singular circles.** The θ^∨ circle and the wall circles have larger commutants, because a zero frequency adds
  whole root planes to the Cartan block.

**The D-style sub-family used in Tests D and F is 4-dimensional in every group of rank ≥ 2.** It is a very thin slice:
- 4 of 36 in SU(5) under the maximal torus;
- 4 of 76 under the principal circle.

The per-plane variant (one y per root plane) is still small and still torus-invariant. It is the natural next
widening if one is wanted. (IMAGINED as a design suggestion. Nothing was computed on it.)

## What this means for Part II.1 (IMAGINED, my reading)

1. **SU(3) cannot tell the three circles apart, and no group after it can make them agree.** This survey adds nothing
   new about whether she lives on the principal circle. It only makes the test well-posed in more groups.
2. **The far wall beyond ρ^∨ is always a tied-top wall.** So the "C-like" end of any sweep is structurally the same
   kind of place in every group. The θ^∨ end is always a top-unique wall, except in SU(3), where it is not a wall.
3. **The charge at ρ^∨ falls with rank:** 1 → 0.949 → 0.894 in SU(3, 4, 5), and 0.845 in Sp(3). Any minimal-coupling
   prediction has to carry q_K² = (h − 1)²/|ρ^∨|², not 1.
4. **G2 needs its normalisation pre-registered.** Under −2tr on the 7-dimensional representation, her θ^∨ charge is
   1/√2.

No floor, edge, spectrum or competitor was computed for any group in this survey.
