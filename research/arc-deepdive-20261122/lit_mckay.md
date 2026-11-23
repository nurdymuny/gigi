# The Poincaré sphere and E₈: what the literature says, and how it touches her basin

*Arc Part II.2 deep-dive, 2026-11-22. Literature fetched today. Every quote below is verbatim from the URL beside it.
Status labels: PROVED / COMPUTED / QUOTED / UNVERIFIED / IMAGINED.*

Script: `mckay_cs_check.py`; receipt: `mckay_cs_check.receipt.txt`. It runs single-threaded in a few seconds.

---

## 0. The short answer

1. **The exact Chern–Simons values are in the literature (QUOTED), and I recomputed them (COMPUTED).** The Poincaré
   sphere P = Σ(2,3,5) = S³/2I has exactly three flat SU(2) connections up to gauge:
   - the trivial one, with CS = 0;
   - the irreducible ⟨1,1,1⟩, with CS = 1/120 mod 1;
   - the irreducible ⟨1,1,3⟩, with CS = 49/120 mod 1.

   The overall sign depends on convention: some sources write −1/120 and −49/120 (§2).
2. **A correction to my arc note (II.2).** I wrote that "the ways of twisting around that particular space line up
   one-to-one with the dots of the E₈ diagram." That is wrong as written. McKay matches the *irreducible
   representations of 2I*, all nine of them, with the nine dots of affine E₈. The *flat SU(2) connections* (her
   basins) are only three of those dots:
   - the affine node (the trivial rep 1);
   - the node next to it (the defining rep 2);
   - the end of the short arm (the conjugate rep 2′).

   The other six dots have dimension 3 to 6. They would be basins only for a bigger gauge group. COMPUTED; see §3.
3. **The basin my Berger field falls into has the smallest nonzero Chern–Simons value on P, 1/120.** That value is
   the Fintushel–Stern invariant τ(P). PROVED by an elementary argument (§4), up to the sign convention. This is the
   nicest thing in the thread, and it is new to my notes.

---

## 1. McKay correspondence (2I ↔ affine E₈)

- **QUOTED**, nLab, https://ncatlab.org/nlab/show/McKay+correspondence:
  - the original correspondence (McKay 80) "states that in this case the corresponding McKay quivers are Dynkin
    quivers/Dynkin diagrams";
  - it is "a one-to-one correspondence between the McKay graphs of finite subgroups" and "the extended Dynkin
    diagrams of ADE type";
  - reference as nLab gives it: "John McKay, *Graphs, singularities, and finite groups* Proc. Symp. Pure Math.
    Vol. 37. No. 183. 1980".
  - Per the fetch summary, nLab's ADE *table* pairs 2I with E₈. I did not get a verbatim sentence for that pairing.
- **UNVERIFIED (primary source).** I found McKay's original paper as a scan
  (https://www-users.cse.umn.edu/~reiner/ReadingSeminars/McKaySeminar2016/McKaysOriginalPaper.pdf). The fetch
  returned only an interlibrary-loan cover sheet, so I have no verbatim sentence from McKay himself.
- **QUOTED**, Baez, "From the Icosahedron to E8", London Math. Soc. Newsletter 476 (2018) 18–23,
  https://arxiv.org/abs/1712.06436: "Here we describe two constructions of the E8 lattice from the icosahedron." Per
  the fetch summary, the abstract also names the "Poincare homology 3-sphere" among the objects these constructions
  connect.
- **COMPUTED** (`mckay_cs_check.py` §3). I built 2I explicitly as 120 SU(2) matrices and got its character table by
  Burnside's class-algebra method:
  - 9 classes, of sizes 1, 1, 12, 12, 12, 12, 20, 20, 30;
  - irreducible dimensions 1, 2, 2, 3, 3, 4, 4, 5, 6, with Σd² = 120.
  - **The McKay graph** (tensor with the defining 2) is a tree with exactly one branch vertex, the 6. It has three
    arms:
    - 6–5–4–3–2–1 (length 5);
    - 6–4′–2′ (length 2);
    - 6–3′ (length 1).

    That is affine E₈.
  - The matrix 2I − A has exactly one zero eigenvalue, so the graph is affine (the other eight are positive), and the
    vector of dimensions is its null vector: (2I − A)·dims = 0 exactly. Those dimensions are the familiar E₈ marks
    1, 2, 3, 4, 5, 6, 4, 2, 3.

## 2. Flat SU(2) connections on P and their Chern–Simons values

**Which flat connections exist.**
- 2I is perfect, so the only reducible flat SU(2) connection is the trivial one.
- 2I has exactly two 2-dimensional irreps, 2 and 2′, which are Galois conjugates (√5 ↦ −√5). Both are faithful and
  both land in SU(2). COMPUTED: their characters in the receipt differ by φ ↔ −1/φ.
- So there are 3 flat connections: 1 + 2 irreducible.

**Corroboration (QUOTED).** Saveliev, "Floer homology of Brieskorn homology spheres: solution to Atiyah's problem",
https://ar5iv.labs.arxiv.org/html/math/9701211, as rendered there:
"λ⁡(Σ⁡(2,3,5))=λ⁡(Σ⁡(2,3,7))=1 while λρ​(Σ⁡(2,3,5))=−1". With Casson's normalisation (half the signed count of
irreducibles), |λ| = 1 is consistent with two irreducibles. That consistency is my inference, not a quote.

**The Fintushel–Stern recipe (QUOTED).** Asselmeyer-Maluga and Król, arXiv:1401.4815, Appendix C. The PDF was
fetched and text-extracted today; https://arxiv.org/pdf/1401.4815.
- They cite [27] = "R. Fintushel and R.J. Stern. Instanton homology of Seifert fibred homology three spheres. Proc.
  London Math. Soc., 61:109–137, 1990."
- "a representation α : π1(Σ(p, q, r) → SU(2) is determined by a tripel of 3 numbers ⟨k, l, m⟩ with 0 < k < p,
  0 < l < q, 0 < m < r", subject to four inequalities.
- "Then the Chern-Simons invariant is given by CS(α) = e²/(4·p·q·r) mod 1 with e = k·q·r + l·p·r + m·p·q."
- "Now we consider the Poincaré sphere P with p = 2, q = 3, r = 5. Then we obtain ⟨1, 1, 1⟩ CS = 1/120,
  ⟨1, 1, 3⟩ CS = 49/120".
- "In [27] the authors define a further invariant τ(Σ) = min {CS(α) | α : π1(Σ) → SU(2)} … For P … τ(P) = 1/120".

**Independent value for one connection (QUOTED).** Guadagnini, Mathieu and Thuillier, "Flat connections in
three-manifolds and classical Chern-Simons invariant", arXiv:1710.09629 §7; PDF fetched and text-extracted,
https://arxiv.org/pdf/1710.09629.
- They build the connection from a genus-2 Heegaard splitting, with π₁ "the binary icosahedral (or dodecahedral)
  group of order 120".
- For a nontrivial representation: "cs[A] = −0.0083333333 · · · = −1/120 mod Z, (7.9) where the last identity is a
  consequence of the fact that |π1(P)| = 120."
- Different method, same value up to the sign convention.

**My check (COMPUTED,** `mckay_cs_check.py` §§1–2**).**
- Enumerating the quoted inequalities for (2,3,5) gives exactly two triples:
  - ⟨1,1,1⟩, with e = 31: e²/120 = 961/120 ≡ **1/120**;
  - ⟨1,1,3⟩, with e = 43: e²/120 = 1849/120 ≡ **49/120**.
- With the opposite sign convention these are 119/120 and 71/120.
- For each triple I built x, y, z ∈ SU(2) with x ~ e^{iπk/2}, y ~ e^{iπl/3}, z ~ e^{iπm/5} and
  x² = y³ = z⁵ = xyz = −1. The relations hold to 1.5 × 10⁻¹² and 1.2 × 10⁻¹⁴, and each image group has order 120
  (faithful).

**Sign caveat (UNVERIFIED which is "right").** A web-search summary reported a sign mismatch:
- Hikami (math-ph/0504082) lists −1/120 and −49/120;
- Guadagnini et al. give −1/120;
- Asselmeyer-Maluga and Król give +1/120 and +49/120.

When I fetched Hikami's abstract page it did not show these values, so that attribution is UNVERIFIED. The standard
Fintushel–Stern formula is often written −e²/(4pqr) (UNVERIFIED: I could not fetch the FS paper; the MPIM preprint
scan, https://archive.mpim-bonn.mpg.de/3333/1/preprint_1989_27.pdf, has no text layer). Safe statement: **the
values are ±1/120 and ±49/120 mod 1, with one common sign that depends on orientation and convention.**

**Kirk–Klassen.**
- **QUOTED** (as a citation), Dubois and Kashaev, arXiv:math/0510607, bibliography entry [11] (PDF text-extracted):
  "P. Kirk and E. Klassen, Chern–Simons invariants of 3-manifolds decomposed along tori and the circle bundle over
  the representation space of T², Commun. Math. Phys. 153 (1993) 521–557". The paper uses it to "compute the
  Chern–Simons invariant for torus knots."
- The 1990 Math. Ann. paper, "Chern-Simons invariants of 3-manifolds and representation spaces of knot groups", is
  **UNVERIFIED**: I could not fetch a record of it (a fetch model offered "Math. Ann. 287 (1990)" from memory, which
  I do not count).

**Floer homology of P.** The usual statement is that I_*(Σ(2,3,5)) has rank 2, one generator per irreducible, in
two gradings. **UNVERIFIED.** Saveliev's quoted Theorem 1 gives the rank formula, but I did not evaluate it from a
fetched statement of k(2,3,5).

## 3. Correcting arc II.2: basins are 3 of the 9 E₈ dots

**PROVED (elementary) + COMPUTED.**
- Flat SU(2) connections on P correspond to homomorphisms 2I → SU(2) up to conjugacy. These are the trivial one plus
  the two 2-dimensional irreps.
- On the affine E₈ diagram they sit at:

| E₈ dot (dim) | flat SU(2) connection? | CS (mod 1, up to common sign) | basin in my notes |
|---|---|---|---|
| affine node (1) | yes, A = 0 | 0 | trivial basin; she is the floor there for t ∈ [2.75, 10] |
| 2 (next to affine node) | yes, ⟨1,1,1⟩ (see §4) | 1/120 | **where the Berger field lands (A = θ)** |
| 2′ (end of the length-2 arm) | yes, ⟨1,1,3⟩ | 49/120 | not yet visited by my flow |
| 3, 3′, 4, 4′, 5, 6 | no (an SU(2) flat connection has a 2-dim holonomy rep) | n/a | basins only for SU(N), N ≥ 3 |

- So "her landing spots are the dots of E₈" overstates it. **The SU(2) basins are the dots of dimension ≤ 2:** the
  affine node and the two ends that carry a 2.
- The reformulation that survives: in an SU(N) theory on P, flat connections are N-dimensional reps of 2I, which
  are sums of the nine E₈ dots with dimensions adding to N. Then every dot does matter. IMAGINED as a direction;
  standard as math.

## 4. Her basin carries CS = 1/120 = τ(P)

**PROVED (elementary, up to the global sign convention).**
- On S³ = SU(2), the Maurer–Cartan field A = θ = g⁻¹dg is pure gauge. Its Chern–Simons form is −(1/3)tr(θ³) up to
  normalisation, and it integrates to ±1 (the degree of the identity map).
- That form is left-invariant. 2I acts freely by left multiplication, so θ descends to S³/2I and its integral there
  is exactly 1/120 of the S³ integral: CS(θ on P) = ±1/120.
- So θ is not gauge-equivalent to A = 0 on P: 1/120 is not an integer. That is the same conclusion my basin note
  reached from the holonomy, now with the exact Chern–Simons value attached.
- θ's holonomy is the defining rep 2 (basin note: PROVED to 7.5 × 10⁻¹³). Of the two irreducible values ±1/120 and
  ±49/120, the only one compatible with ±1/120 is the first, whichever global sign you pick (−1/120 ≢ ±49/120).
- **So the Berger field falls into ⟨1,1,1⟩, the flat connection that attains the Fintushel–Stern minimum
  τ(P) = 1/120. That is the lowest nonzero Chern–Simons level on the Poincaré sphere.** The energy flow and the
  topological minimum pick the same basin.
  - *Verifier's caveat (added 22 Nov):* "attains the minimum" holds in the A-M&K convention (+1/120, +49/120). In the
    opposite convention the representatives in (0,1) are 119/120 (for 2) and 71/120 (for 2′), and the minimum would be
    2′. The PROVED part is "lands on ⟨1,1,1⟩ with CS ±1/120 mod 1"; "that is τ(P), the minimum" is conditional on the
    convention, which stays UNVERIFIED until Fintushel–Stern is read.
- What I have *not* shown: whether the flow could ever reach 2′ (CS 49/120). I believe it cannot within her family,
  since the family is left-invariant and θ is the only left-invariant flat field besides 0, but I have not proved
  that.
- An observation I have not checked: 49 = 7², and e(2′)/e(2) = 43/31. A clean statement of "why 7" (the Galois twist
  as a degree-7 equivariant map?) is IMAGINED.

## 5. The dodecahedral-space cosmology

**QUOTED**, Luminet, Weeks, Riazuelo, Lehoucq, Uzan, "Dodecahedral space topology as an explanation for weak
wide-angle temperature correlations in the cosmic microwave background", Nature 425 (2003) 593,
https://arxiv.org/abs/astro-ph/0310253: "the Poincaré dodecahedral space -- which accounts for WMAP's observations
with no fine-tuning required."

Whether later data (Planck) disfavoured it is UNVERIFIED here: I did not fetch it, and I make no claim about it.

## 6. What this changes, and what I would do next

- The arc table row "Poincaré basins ↔ E₈ nodes (via McKay)" should become:
  - "SU(2) basins = the three E₈ dots of dim ≤ 2 (COMPUTED/PROVED); the Berger fall lands at CS = τ(P) = 1/120
    (PROVED)";
  - "whether she follows the full E₈ pattern needs SU(N ≥ 3) (IMAGINED)."
- One cheap next computation: start the flow from a 2′-twisted (non-left-invariant) field and see whether it can land
  at 49/120. That needs a field outside her left-invariant family, so it is not a quick extension of
  `poincare_fall.py`.

## Sources fetched today

- https://ncatlab.org/nlab/show/McKay+correspondence
- https://arxiv.org/abs/1712.06436 (Baez)
- https://arxiv.org/pdf/1401.4815 (Asselmeyer-Maluga and Król, App. C: the FS recipe and the values for P)
- https://arxiv.org/pdf/1710.09629 (Guadagnini, Mathieu and Thuillier, §7: −1/120)
- https://ar5iv.labs.arxiv.org/html/math/9701211 (Saveliev: Casson λ(Σ(2,3,5)))
- https://arxiv.org/pdf/math/0510607 (Dubois and Kashaev: Kirk–Klassen CMP 1993 citation)
- https://arxiv.org/abs/astro-ph/0310253 (Luminet et al., Nature 2003)
- Not readable as text: the McKay 1980 scan (UMN) and the Fintushel–Stern preprint (MPIM 1989-27).
