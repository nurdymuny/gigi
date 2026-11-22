# Doors and walls as Morse theory (arc Part II.4), checked against the literature and computed

*22 November 2026. The thread: is "the door-to-wall crossing of the twist along the fall a stratum boundary"?*

Labels: **PROVED / COMPUTED / QUOTED** (literature, fetched today, verbatim, with URL) / **UNVERIFIED** (could not
fetch, so not stated as fact) / **IMAGINED**.

Scripts and receipts, all in this folder:
- `morse_family.py` → `morse_family_receipt.json` (critical points of the energy on her family, their indices, the
  separatrices, the Berger flow line);
- `morse_unstable_branch.py` → `morse_unstable_branch_receipt.json` (her price along both unstable branches of the
  saddle).

Both run single-threaded in seconds.

## The short answer

**The bridge, as worded, fails.** It fails on a point of definition, not on a number:
- in the Morse theory of Atiyah–Bott and Råde, a stratum is the set of starting fields that flow to one critical set;
- a flow line therefore never crosses a stratum boundary: it lies inside one stratum for its whole life;
- the fall from the Berger field lands on A = θ, so the whole fall, her door closing included, happens inside the
  interior of the open stratum of θ.

Her door-to-wall crossing is real (COMPUTED in the basin note), but it is not a Morse event. It is a sign change of one
Hessian eigenvalue at a **non-critical** field, and Morse theory does not grade non-critical points.

Two things do survive, and I think they are the useful part:
1. **The Morse picture of her family, computed exactly.** For every stretch, the energy on her family has three minima
   (A = 0, A = θ and its mirror) and two index-1 saddles (a meron-type field and its mirror), and nothing else.
   - The count 3 − 2 = 1 is the Euler characteristic of the plane.
   - **She is a wall at every one of these critical points**, and along both downhill branches of the saddle.
   - So the family's staircase is not made of her. Her doors exist only on the slopes between the floors.
2. **A correct Morse-theoretic home for the door count: spectral flow.** Along a path of fields, the net number of
   Hessian eigenvalues crossing zero is the spectral flow. In Floer's theory that number is the relative index.
   - Along the fall, her crossings are exactly her share of the spectral flow from the Berger field to A = θ.
   - That is a true statement, but it is bookkeeping. It is not a stratum boundary.

## Part A. What the literature says (QUOTED)

Atiyah–Bott itself could not be fetched. The Royal Society page returns HTTP 403, Semantic Scholar has the abstract
withheld, and Project Euclid PDFs are behind a bot check. So every Atiyah–Bott, Råde, Floer, Palais and Taubes
statement below comes from a fetched secondary source that quotes or summarises them, and it is marked as such.

**A1. Atiyah–Bott: the flow stratifies the space, one stratum per critical set.** This is the secondary source Wilkin,
*Morse Theory for the Space of Higgs Bundles*, arXiv:math/0611113 (PDF fetched), extending Atiyah–Bott:
- > "The results of this paper provide the analytic background for the use of Morse theory in the spirit of Atiyah
  > and Bott's approach for holomorphic bundles in [2]"
- > "Corollary 1.2 (Description of Analytic Stratification). The space B admits a stratification in the sense of [2]
  > Proposition 1.19 (1)-(4), which is indexed by the set of connected components of the critical points of the
  > functional YMH."
- The stratum of a critical component η is the set of starting points that flow to it, as in Theorem 1.1:
  > "for each connected component η of the set of critical points of YMH, the map r : {(A0 , φ0 ) ∈ B : r(A0 , φ0 ) ∈
  > η} → η is a G-equivariant continuous map."
- On a surface the strata match the algebra:
  > "The algebraic stratification of B by Harder-Narasimhan type is equivalent to the analytic stratification of B by
  > the gradient flow of the functional YMH."
- Here [2] is "M. F. Atiyah and R. Bott. The Yang-Mills equations over Riemann surfaces."
- URL: https://arxiv.org/pdf/math/0611113

What I take from A1: **strata are basins of critical sets.** A boundary between strata is where the flow's
destination changes. It is not where some eigenvalue changes sign along the way.

**A2. Råde: the Yang–Mills flow converges in dimensions 2 and 3.** This is the dimension that matters for S³. Quoted by
Wilkin, same PDF:
- > "the approach of Rade in [17] and [18] where it was shown that the gradient flow of the Yang-Mills functional
  > converges in the H 1 norm when X is a 2 or 3 dimensional manifold, thus providing a purely analytic stratification
  > of the space A. Rade's proof was based on a technique of Simon in [19], the key step being to show that a
  > Lojasiewicz-type inequality holds in a neighbourhood of each critical point."
- Reference [18] is "Johan Råde. On the Yang-Mills heat equation in two and three dimensions."
- URL: https://arxiv.org/pdf/math/0611113
- So **on my Berger spheres every Yang–Mills flow line has a limit, and the space is stratified by those limits.**
  This is QUOTED, from a secondary source. I have not read Råde's paper; the exact hypotheses are **UNVERIFIED** by me.

**A3. Floer: on a 3-manifold the critical points of Chern–Simons are the flat fields, and indices are spectral flows.**
Salamon–Wehrheim, *Instanton Floer homology with Lagrangian boundary conditions*, arXiv:math/0607318 (PDF fetched),
describing Floer's setup:
- > "The critical points are the flat connections in A(Y, L). If we fix a Riemannian metric g on Y then the gradient
  > flow lines of the Chern–Simons functional with respect to the L2 inner product are smooth maps R → A(Y ) : s 7→ A(s)
  > satisfying the differential equation ∂s A + ∗FA = 0"
- > "As in Floer's original work [13] the main idea is to use the solutions of (2) to construct a boundary operator on
  > the chain complex generated by the gauge equivalence classes of the nontrivial flat connections"
- > "The proof of (ii) will be based on an identification of the index with the spectral flow of the Hessian."
- > "Theorem A.4. The operator D is Fredholm and its index is equal to the upward spectral flow of the operator family
  > s 7→ A(s)."
- > "the spectral flow can be defined as the sum of the crossing indices"
- URL: https://arxiv.org/pdf/math/0607318
- Floer's own paper (Commun. Math. Phys. 118, 1988) could not be fetched (bot check): **UNVERIFIED** at first hand.
- **Caution.** Floer's Morse function is Chern–Simons, whose Hessian is the first-order operator ∗d_A. Mine is
  Yang–Mills, with a second-order Hessian. Both have the flat fields as critical points (Yang–Mills minima on S³), but
  the flows are different flows. The fall is Yang–Mills flow on S³, not a Chern–Simons flow line on S³ × R.

**A4. Palais: symmetric critical points are critical points.** This is what lets the 2D family computation say
something about the full space. Squassina, arXiv:1007.3593 (PDF fetched):
- > "For G-invariant functionals of class C 1 the principle was rigorously formulated by R.S. Palais in 1979 in a
  > celebrated paper [17]"
- > "in order for an invariant point u to be critical for f , it is sufficient that it be critical for f |Fix(G)"
- URL: https://arxiv.org/pdf/1007.3593
- Palais's own CMP paper could not be fetched: **UNVERIFIED** at first hand.

**A5. Taubes: Morse theory for Yang–Mills in dimension 4.** This is a secondary description in Feehan,
arXiv:1505.06995 (PDF fetched):
- > "The purpose of Taubes' article is to explain [95, p. 329] how the Morse theory for the Yang-Mills L2 -energy
  > functional can be recovered by examining the restriction of the functional to a countable set of
  > finite-dimensional, non-compact subvarieties of the Banach manifold of connections"
- > "The primary difficulty one must confront when analyzing the L2 -energy functional on a principal G-bundle over a
  > four-dimensional manifold is non-compactness of the critical sets ... due to the well-known phenomenon of energy
  > bubbling"
- URL: https://arxiv.org/pdf/1505.06995
- This is relevant only to say that the hard analytic problem (bubbling) is a 4D problem. In 3D, Råde's convergence
  (A2) is the operative fact. Taubes's *Stability in Yang–Mills theories* (CMP 1983): **UNVERIFIED** (not fetched).

**Not fetched, so not used as fact:**
- Atiyah–Bott's "equivariantly perfect" statement and their index formula for the strata (codimension equals the Morse
  index): **UNVERIFIED** here. I recall it, but I did not fetch a verbatim line today.
- Any published Morse index of the meron A = θ/2 on the round S³: **UNVERIFIED**.

## Part B. Why the literal bridge fails (PROVED, given A1–A2)

1. On the Berger sphere the Yang–Mills flow from any smooth field converges (A2). The space is stratified by the
   critical set each field flows to (A1).
2. A flow line's destination is the same at every point of the line, because the flow is a semigroup: the field at
   time τ flows to the same limit as the field at time 0. So **the whole flow line lies in one stratum.**
3. The fall from the Berger field lands on A = θ for every t ≠ 1 (COMPUTED, basin note). So every point of the fall,
   before and after her door closes, lies in the stratum of the flat field θ.
4. Therefore **the door-to-wall crossing is not a stratum boundary.** It is an interior point of the minimum's
   stratum.

This is a short argument, and its only inputs are the literature definitions (QUOTED, secondary) and my own landing
computation.

A sharper way to say it: the Morse index is defined at critical points. Along the fall the gradient is not zero until
the very end, so "how many downhill directions the field has" is not a Morse index there. Her negative price before
the crossing is a fact about the curvature of the slope (the Floor paper's "landscape, not dynamics"), not a floor of
the building.

## Part C. What the family's actual Morse picture is (COMPUTED / PROVED)

The family is (x, y) with energy E = ½(B² + 2c²), where B = y² − 2√t·x and c = y(x − 2/√t), and L² metric
diag(1, 2), as in `docs/research/basin-2026-11-22/family_flow.py`.

**C1. Exactly five critical points for every t > 0** (PROVED: sympy solves ∇E = 0 and returns five solutions):

| critical point | energy | family Hessian eigenvalues (G⁻¹H) | family index |
| --- | --- | --- | --- |
| A = 0, (0, 0) | 0 | 4t, 4/t (exact) | 0 |
| A = θ, (2/√t, ±2) | 0 | 2t + 8 ∓ 2√(t² + 8t) (exact) | 0 |
| saddle, (x_s, ±y_s), y_s² = (2√t·√(9t+16) − 6t)/4 | 0.22 to 2.28 (decreasing in t) | one negative, one positive | 1 |

- At t = 1 the saddle is (1, 1), the meron A = θ/2. Its gradient is exactly zero; the eigenvalues are −2 and 10.
- The saddle is the continuation of the meron to every stretch, from t = 0.5 to 10. COMPUTED.
- y → −y is a constant gauge rotation, so the ± pairs are the same field up to gauge.

**C2. A Morse count that closes.** E is proper on the plane: on a circle of radius R in the metric, its minimum grows
like R² (checked at R = 5, 20, 100 and t = 0.5, 1, 3.55, 10). With three minima and two index-1 saddles,
3 − 2 = 1 = χ(R²). COMPUTED.
- The two unstable branches of each saddle land on A = θ and on A = 0, at every t checked.
- So inside the family, the stratification is:
  - two open basins, of θ and of 0;
  - separated by the stable curves of the saddles;
  - which are the only stratum boundaries.

**C3. By Palais (A4), the saddle is a genuine Yang–Mills critical point on the Berger sphere**, not only in the family.
- The family is the fixed set of a compact symmetry group: left translations together with the long-circle rotation.
- STRUCTURAL: this rests on the edge paper's statement that the family is all circle-symmetric left-invariant fields up
  to constant gauge. I have not written out the fixed-set identification here.
- Its full Morse index is at least 1. Its exact full index is **open** and was not computed here.

**C4. She is a wall at every critical point and along both branches.**
- At the saddle, her price (x + (j+2)/√t)² + 2√t·x − y² is positive in every sector j ≤ 40, at every t from 0.5 to 10.
  COMPUTED. Her lowest prices at the saddle are 21.28 at t = 0.5, 10.00 at t = 1, 3.12 at t = 3, 2.62 at t = 3.55 and
  0.904 at t = 10.
- Along both unstable branches of the saddle, her lowest price stays positive, and its minimum is attained at an end:
  - on the θ branch, at the saddle end;
  - on the 0 branch, at A = 0 itself, where it is 4/t.
  - COMPUTED (`morse_unstable_branch_receipt.json`).
- So **she contributes nothing to the index of the saddle.** The saddle's downhill direction is a family direction,
  the one that turns the meron into θ or into 0. The staircase of her family is not made of her.

**C5. Her door-closing curve is not a separatrix.** The curve where her lowest price crosses zero, {min_j P_j = 0},
and the saddle's stable curve are different curves.
- Along the stable curve, her lowest price takes both signs, from about −98 to +210 out to radius 12. COMPUTED.
- The Berger flow line crosses her zero curve at energies 5.8 (t = 2, where it starts on the edge), 3.8 (t = 2.75),
  3.4 (t = 3), 2.8 (t = 3.55), 1.8 (t = 5), 1.2 (t = 7) and 0.79 (t = 10).
  - That is always above the saddle's energy at the same t (0.91, 0.70, 0.65, 0.57, 0.42, 0.31, 0.22).
  - So her door closes while the field is still higher than the saddle level. It never crosses the saddle's stable
    curve, and lands on θ.
- Closest approach of the Berger line to the saddle (L² family metric): 0.004 at t = 0.9 and 0.003 at t = 1.1, but
  0.29 at t = 3, 0.38 at t = 3.55 and 1.25 at t = 10. Near the meron the fall brushes the saddle; at the stretches
  where she is a door, it does not. COMPUTED.

**C6. A cross-check with the basin note** (COMPUTED; INCIDENCE). The family Hessian eigenvalues at the minima are
exactly the sector-0 floors found yesterday:
- at A = 0, {4/t, 4t}: Henkel–Lauret's Proposition 3.6 values, as recorded in the basin note's addendum;
- at A = θ, 2t + 8 − 2√(t² + 8t): the sector-0 floor at θ that the basin note found in closed form.
- So the slowest settling direction at the bottom of the fall (sector 0 at θ) lies **inside her family**. It is the
  family's own soft direction, not her: at t = 3.55 it is 2.29, while her price there is 16/t = 4.51.

## Part D. What a correct Morse statement about her looks like (IMAGINED, with what each would need)

1. **Spectral flow, not strata.** "Her door count at the Berger field equals her share of the spectral flow of the
   gauge-fixed Yang–Mills Hessian along the fall to θ."
   - This is true by construction: at θ she is a wall, and at the Berger field she is a door in the sectors where her
     price is negative.
   - It becomes interesting only if the full spectral flow (all shapes, not only her) along the fall can be computed
     and compared with her share. Status: IMAGINED; computable with the Floor-paper builders.
2. **The meron saddle as the real "floor above the basement".** In the family, the only non-minimal floor is the
   meron-type saddle, at every stretch.
   - A Morse-theoretic floor law would be a statement about the full index of that saddle as t varies:
     - which shapes are its downhill directions;
     - whether any of her sectors ever joins them at larger t (up to t = 10 and j = 40, none does).
   - Status: IMAGINED; the next computation would be the full Hessian at the saddle, sector by sector.
3. **The edge as a bifurcation.** Her edge (for example t = 2 in sector 0, where the Berger field starts exactly on her
   zero curve) is a property of a non-critical field.
   - In the family there is no bifurcation of critical points anywhere in t ∈ [0.5, 10]: always 3 + 2. COMPUTED.
   - So the edge is **not** a Morse bifurcation inside the family.
   - Whether some critical point outside the family bifurcates at her edge is open: IMAGINED, with no evidence either
     way.

## Verdict on the bridge

| claim | status |
| --- | --- |
| Atiyah–Bott/Wilkin: strata are indexed by critical components, each stratum = the fields flowing to it | QUOTED (secondary: Wilkin) |
| Råde: Yang–Mills flow converges in dimensions 2 and 3, giving an analytic stratification | QUOTED (secondary: Wilkin); primary UNVERIFIED |
| Floer: flat fields are the Chern–Simons critical points; relative index = spectral flow of the Hessian | QUOTED (secondary: Salamon–Wehrheim); primary UNVERIFIED |
| Palais symmetric criticality | QUOTED (secondary: Squassina); primary UNVERIFIED |
| Atiyah–Bott perfection and the stratum codimension = index formula | UNVERIFIED (not fetched) |
| the fall lies entirely in the stratum of θ, so the door-to-wall crossing is not a stratum boundary | PROVED (given the quoted definitions and the computed landing) |
| her family has exactly 3 minima + 2 index-1 meron-type saddles for every t; 3 − 2 = χ(plane) | PROVED (count), COMPUTED (indices, properness) |
| the meron-type saddle is a full Yang–Mills critical point on each Berger sphere | STRUCTURAL (Palais + the family's fixed-set identification) |
| she is a wall at every critical point of the family and along the saddle's unstable branches (j ≤ 40, t ∈ [0.5, 10]) | COMPUTED |
| her zero curve is not the separatrix; the door closes above the saddle level and inside θ's basin | COMPUTED |
| family Hessian at A = 0 and θ reproduces the sector-0 floors 4/t, 4t and 2t + 8 − 2√(t² + 8t) | COMPUTED (INCIDENCE with the basin note) |
| "the door-to-wall crossing is a Morse stratum boundary" | **fails as stated** |
| "her door count is her share of the spectral flow along the fall" | true by construction; interesting only once the full spectral flow is computed (IMAGINED) |
