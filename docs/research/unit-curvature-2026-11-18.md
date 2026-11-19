# The curvature of one unit: zero, ±1, the gauge group and the mass gap

2026-11-18. Research note. It records an idea of mine, separates what is exact from
what is open, and computes the one piece that could be computed today. **No mass-gap
claim is made.** The one open link is tagged as open throughout.

Altitude tags, as in `docs/research/small-edges-2026-11-18.md`:

- **STRUCTURAL:** an exact statement or theorem, with its hypotheses checked;
- **RESONANCE:** the same question in a different setting. It suggests, and proves nothing;
- **OPEN:** the link this note would need, and does not have.

## The idea, in my words and in plain words

What I said on 2026-11-18, continuing from the chord–arc–tangent picture
(`docs/papers/chord-arc-tangent/`):

> 0 is nothing, there is no system, but if you take 1 unit of energy, 1 unit of anything
> real, 1 particle, anything, it has to expand equally in all … axes. … It can't take up
> negative space in one of the axes. And so therefore it has curvature. That's what the
> curvature at the very bottom of existence is. It itself has curvature just because it
> exists.

**In plain words.** Zero is not a thing; it is the absence of a system. The smallest
real thing has nothing to make it prefer one direction over another, so it extends the
same amount in every direction. "The same amount in every direction" is a sphere, and a
sphere is curved. So curvature is not something added to the smallest thing from
outside. It comes with existing at all. My proposal is that this curvature, at the
bottom of everything, is related to why some forces have a mass gap.

## Provenance

| Source | What it contributes | Where |
| --- | --- | --- |
| My messages, 2026-11-18 | The idea above. Also: residuals come in twins, and the "root" (where the construction is based) decides whether numbers are real or unreal. I related both to Bertrand's paradox, to my graph theory and to my grouping theory. | this session |
| The chord–arc–tangent paper | The turn U, the chord U − 1, the half-turn U = −1 where the tangent construction breaks, twins and Bertrand (§§ 2–6) | `docs/papers/chord-arc-tangent/paper.tex` |
| My Good Will Hunting graph-fold article | Folding with matching menus: AP = PB, hence AⁿP = PBⁿ. Connectedness and equitability are different properties. | math-website `notebook/content/good-will-hunting-graph-fold.html`, commit `8eb87c00` |
| *Zero Does Not Exist* (mine) | A count of n is the path P_{n+1}. Zero crossings is still one position (P₁), never empty. | cited in the article above |
| My connected-partition counting paper | Grouping into connected pieces, and contraction | cited in the article above |
| *The Double Cover Principle* (mine) | S + d² = 1 on the double cover | Zenodo 10.5281/zenodo.18895462 |
| Two-ring paper, Section 9 | The measured eigenvalue 4 on S³/2I is a geometric eigenvalue of a compact space, not a mass gap (the correction of 2026-11-17) | `docs/papers/two-ring-pg/paper.tex` |
| Halcyon | My SU(2) Yang–Mills lattice programme, the downstream home of any test | `davis-wilson-lattice/` |
| Computation (this note) | The double-cover fold on the binary icosahedral group 2I | `scripts/binary_icosahedral_fold.py`, receipt `research/binary-icosahedral-fold-20261118/fold.json` |

The outside literature is listed at the end. **The citations there are from memory**,
and their bibliographic details must be checked before any of this is published.

---

## 1. One unit in every direction is the unit sphere

*STRUCTURAL (definition).*

The points at distance one from the origin in n-dimensional space form the unit sphere
S^{n−1}:

| Space | "One unit in every direction" | What it is |
| --- | --- | --- |
| the line ℝ | {−1, +1} | the 0-sphere S⁰: two points, no curve |
| the plane ℂ | the unit circle | the turns e^{iθ}, the group U(1) |
| space ℝ³ | the unit sphere S² | the directions |
| the quaternions ℍ ≅ ℝ⁴ | the unit 3-sphere S³ | the unit quaternions, the group SU(2) |

The centre, 0, is never on the sphere. It has length zero, no inverse, and is no turn.

**In plain words.** On a line, "one step in every direction" gives exactly two places, −1
and +1. In the plane it gives a circle, and in four dimensions it gives a three-sphere,
which is precisely the set of turns that the SU(2) force is made of. Zero sits in the
middle of every one of these and is never part of any of them.

*RESONANCE.* In *Zero Does Not Exist* a count of zero is still one position, never an empty
object. Here the centre is likewise not an element. The two statements share a stance,
and neither implies the other.

## 2. Why ±1 are special among all the others

*STRUCTURAL.*

- **They are the centre of SU(2).** The unit quaternions that commute with every other unit
  quaternion are exactly +1 and −1.
- **They are the only two that are their own twins under the double cover.** The map
  SU(2) → SO(3) sends U and −U to the same rotation of space. So the turn group is
  "folded in two": every rotation of space has two quaternions, a twin pair {U, −U}. The
  pair {+1, −1} is the twin pair of "no rotation at all".
- **In the chord–arc–tangent paper, U = −1 is the half turn**, where the outside corner
  goes to infinity and the Cayley map (built from the tangent leg) cannot reach.
- **Computed for the group of the Poincaré sphere.** The binary icosahedral group 2I has
  120 elements, and its centre is exactly {(1,0,0,0), (−1,0,0,0)} (receipt `fold.json`).
  The Poincaré homology sphere on which our meshes live is S³/2I.

**In plain words.** Among all the turns, +1 and −1 are the only two that get along with
every other turn: doing them before or after anything makes no difference. They are also
the two that both mean "don't rotate at all" once you look at ordinary space, which is
why the turn group is a double cover. The half turn, −1, is exactly where the detour of
our paper breaks.

## 3. Existing as a unit means curvature, and the curvature is the commutator

*STRUCTURAL.*

- **A sphere's curvature is set by its size.** A sphere of radius r has sectional
  curvature 1/r², so the smaller the unit, the more curved it is.
- **A one-dimensional circle has no intrinsic curvature at all.** Sectional curvature needs
  at least two directions. So the circle U(1), the turn group of the photon, is
  intrinsically flat.
- **For a group with a bi-invariant metric, curvature is the commutator** (Milnor 1976):
  for orthonormal X, Y in the Lie algebra,

  K(X, Y) = ¼ |[X, Y]|².

  - For SU(2), with unit quaternions as the unit sphere, [i, j] = 2k, so K = ¼ · 4 = 1.
    The unit three-sphere has curvature 1, as it should.
  - For U(1), every bracket is zero, so K = 0.

**In plain words.** A curve with only one direction to move in cannot be curved from the
inside; you need two directions for that. So the photon's circle of turns is flat in the
only sense that matters to someone living on it. The SU(2) turns are genuinely curved, and
the amount of curvature is exactly how much "this turn, then that one" differs from "that
one, then this". My "curvature at the bottom" has an exact name here: the commutator.

## 4. Curvature and gaps: what is a theorem, what is observed, what is open

**Theorems** *(STRUCTURAL, for Laplacians):*

- **Lichnerowicz (1958).** On a compact n-manifold with Ricci curvature ≥ (n−1)K > 0, the
  first nonzero eigenvalue of the Laplacian is at least nK. Positive curvature forces a gap.
  On the unit S³ (K = 1) the bound is 3, and it is attained.
- **Ollivier (2009).** On a graph or Markov chain whose coarse Ricci curvature is at
  least κ > 0 everywhere, the spectral gap of the random walk is at least κ.
- **Fiedler (1973).** A finite graph is connected if and only if its Laplacian's second
  eigenvalue is positive. Being one connected thing already forces a gap. In a family of
  growing graphs the gap can still close: a ring of n vertices has gap 2 − 2cos(2π/n) → 0,
  so a long flat ring loses it.

**Observed** *(STRUCTURAL as observation):*

- The photon, whose group U(1) is flat and commutative, is massless.
- Pure SU(2) and SU(3) gauge theory on the lattice has massive glueballs: a gap
  (Morningstar–Peardon 1999 for SU(3); Lucini–Teper–Wenger 2004 across SU(N)).
- In pure SU(2) theory, the centre {±1} carries the standard order parameter of
  confinement: the Polyakov loop and centre symmetry ('t Hooft 1978; Polyakov 1978).

**Open** *(OPEN):*

- That the curvature of the gauge group (the fibre) is *what produces* the gap in flat,
  infinite spacetime. This is the Clay Millennium problem (Jaffe–Witten 2000). Nothing
  here proves it.

**Prior art in the same direction** (cite before any priority language):

- Feynman (1981) argued, for 2+1 dimensions, that the configuration space of Yang–Mills
  fields, once gauge copies are identified, is compact and curved, and that this makes
  the theory gapped.
- Singer (1978, 1981) studied the geometry of that orbit space and its positive curvature.
- Karabali, Kim and Nair (1998) built a Hamiltonian treatment of 2+1 Yang–Mills with a
  mass gap along these lines.

So "curvature of the configuration space ⇒ gap" is an established line of thought. My
specific picture (the unit's own curvature, the centre twins ±1, and the commutator as the
curvature) has to be positioned against it, not presented as its origin.

**Why this is better than the reading corrected on 2026-11-17.** The eigenvalue 4
measured in the two-ring paper comes from the curvature of the *space* (S³/2I, of fixed
size). In infinite flat space that curvature, and any gap borrowed from it, disappears.
My idea puts the curvature in the *fibre*, the group of turns, which stays curved however
flat and large spacetime is. That is the right place for it to be. It is also where the
2+1 literature above puts it.

**In plain words.** Mathematicians have proved that curvature forces a gap for the
simplest vibrations on a curved space or graph. Physicists observe the pattern I
describe: the flat, commuting force (light) has no gap, and the curved, non-commuting
ones do. Nobody has proved that the curvature *causes* the gap for the real force in real,
infinite space; that is the million-dollar problem. Feynman and others had a related idea
in 1981, so this belongs in that conversation. My version places the curvature in the
right spot: in the turns themselves, not in the space around them.

## 5. The double cover as a fold (computed)

*STRUCTURAL: exact computation. Script `scripts/binary_icosahedral_fold.py`, receipt
`research/binary-icosahedral-fold-20261118/fold.json`.*

This is my graph-fold idea applied to the double cover. In the Good Will Hunting
article, stations with matching menus share one label, and AP = PB proves that every walk
count survives. Here the stations are the 120 elements of 2I, the roads are
multiplication by the generators a = (1+i+j+k)/2, b = (φ + φ⁻¹i + j)/2 and their inverses
(a 4-regular Cayley graph), and the fold puts each U with its twin −U.

| Check | Result |
| --- | --- |
| order of the group generated | 120 |
| centre | exactly {+1, −1} |
| fold cells {U, −U} | 60 |
| **AP = PB** | **exact** (integer equality) |
| walk identity AᵏP = PBᵏ, k ≤ 12 | error 0 |
| folded (even) spectrum = the 60-vertex quotient's spectrum | yes |
| full spectrum = folded part ∪ twin (odd) part | yes, 60 + 60 |
| Laplacian gap of the full graph | 0.381966 (= 2 − φ) |
| gap of the folded graph | 0.381966, the same: the gap lives in the folded half |
| lowest twin (odd, spinor) eigenvalue | 1.381966: the twin half never comes near zero |
| graph distance between twins U and −U | 3, for every pair |
| diameter | 8 |
| Ollivier edge curvature (uniform non-lazy walk) | −0.5 on every edge |
| random-walk gap | 0.0955 |

What it shows:

1. **The double cover is an exact fold.** AP = PB holds, so the 60-label quotient (the
   icosahedral rotation group of ordinary space) reproduces every walk count of the
   120-element group, as in my article.
2. **The twin half of the spectrum is gapped from zero by construction.** A function with
   f(−U) = −f(U) cannot be constant. Here its lowest eigenvalue, 1.382, sits a full unit
   above the graph's gap, 0.382.
   - *RESONANCE.* The spinor (twin) sector sits strictly above the bottom of the spectrum.
3. **Matching menus without connectedness.** Twins are always three steps apart, so the fold
   cells are not connected groups, yet the fold is exact. This is exactly the distinction
   in my article: connectedness says a group holds together, while matching menus say it
   can act as one state. The double cover is the case where far-apart twins act as one.
4. **An honest surprise: curvature is sufficient, not necessary.** With these four
   generators, every edge has Ollivier curvature −0.5, so Ollivier's theorem gives nothing,
   and yet the graph is gapped (Fiedler: it is connected and finite). Graph curvature depends
   on which generators are chosen, that is, on the grouping. Gaps also exist without
   positive curvature (expander families are the standard example). So the defensible
   statement is "positive curvature forces a gap", never "a gap requires curvature".

**In plain words.** Folding every turn with its twin is an exact fold in my sense: the
smaller map counts every journey correctly. The spectrum splits cleanly into the folded
half and the twin half, and the twin half can never sink to zero. The twins are never
neighbours, always three steps apart, and they still act as one, which is precisely the
lesson of the subway article. With these particular roads the graph turns out negatively
curved, and it has a gap anyway. So curvature is one way to guarantee a gap, not the only
one, and we should say exactly that.

## 6. What our current tools can and cannot see

*STRUCTURAL.*

- **The linear spectra we measure cannot distinguish U(1) from SU(2) at the vacuum.** At
  U = 1 the commutator term is quadratic, so it drops out of any linearisation. The S2b
  spectra are taken at the Berger field, not at the vacuum, and they are a geometry test,
  not a gap test.
- **The gap of interest is non-perturbative.** Seeing whether fibre curvature matters means
  either strong fields (where the chord–arc difference of the paper becomes large) or a
  genuinely nonlinear computation, Halcyon's territory.

**In plain words.** Everything we have measured so far is a small-wiggle analysis. For
small wiggles around "nothing happening", the curved turns and the flat ones behave
identically, because the curvature only shows up when turns are combined. To test my
idea we would need big turns, or the full nonlinear theory.

## 7. Next tests (proposed, not run)

1. **Curvature and the grouping.** Repeat §5 with generating sets that are whole conjugacy
   classes of 2I (normal Cayley graphs). Does some natural choice give positive Ollivier
   curvature, so that the theorem's bound applies, and how tight is it? *STRUCTURAL if it
   holds.*
2. **The fold on the meshes.** Our meshes of S³/2I come from 2I. Check whether the
   two-ring stencil's cell classes form an equitable partition under the group action
   (idea 2 of the small-edges note). That would make the n = 3 spectrum accessible by
   symmetry blocks.
3. **U(1) against SU(2) on the same lattice at strong coupling (Halcyon).** The same mesh,
   both groups, coupling raised until face angles are of order one. Compare the lowest
   excitations. This is the one experiment here aimed at the open link, and it would be
   reported as evidence, never as proof.
4. **Bertrand on the group: computed, see §8.**

## 8. Bertrand's question, asked of the group itself (computed)

*STRUCTURAL: exact integral plus simulation. Script `scripts/haar_sameness.py`.*

A "random turn" in SU(2) has a natural meaning: a uniformly random unit quaternion, under
the group's own Haar measure. Its geodesic angle a has density (2/π) sin²a on [0, π].
Changing variables to sameness S = cos²(a/2) gives

  ρ(S) = (8/π) √(S(1 − S)) on [0, 1],

the **semicircle law** (normalisation 1, checked symbolically). Bertrand's event "chord
longer than √3" is a > 2π/3, which is S < 1/4. Its probability is

  P = 1/3 − √3/(4π) ≈ 0.19550.

That is none of Bertrand's three answers (1/3, 1/2, 1/4). A simulation of 4 million Haar
turns gives 0.19545, and the histogram of S matches the semicircle to within 0.023 in
density over 20 bins.

**In plain words.** Bertrand showed that "pick a random chord" has no single answer until
you say how you pick. A group of turns comes with its own fair way to pick a turn at
random, with no choice left to us. Asking Bertrand's question that way gives a fourth
answer, about 0.1955. And the sameness of a random turn is spread out in a perfect
semicircle, the same shape that appears for the eigenvalues of large random matrices
(Wigner's semicircle). *RESONANCE:* that shared shape is noted, not explained.

## What is claimed, and what is not

**Claimed:**
- the definitions and exact statements of §§1–3;
- the theorems of §4 as cited;
- the computation of §5, from its receipt;
- the limits stated in §6.

**Not claimed:**
- that fibre curvature produces the mass gap;
- any priority for the "curvature ⇒ gap" idea (see the prior art in §4);
- that the 2I graph's gap is physical.

## References (from memory; verify before publication)

- J. Bertrand, *Calcul des probabilités*, 1889. E. T. Jaynes, The well-posed problem, *Found. Phys.* 3 (1973) 477–492.
- M. Fiedler, Algebraic connectivity of graphs, *Czech. Math. J.* 23 (1973) 298–305.
- R. P. Feynman, The qualitative behavior of Yang–Mills theory in 2+1 dimensions, *Nucl. Phys. B* 188 (1981) 479–512.
- A. Jaffe, E. Witten, Quantum Yang–Mills theory, Clay Mathematics Institute Millennium Problem description, 2000.
- D. Karabali, C. Kim, V. P. Nair, Planar Yang–Mills theory: Hamiltonian, regulators and mass gap, *Nucl. Phys. B* 524 (1998) 661–694.
- A. Lichnerowicz, *Géométrie des groupes de transformations*, Dunod, 1958.
- B. Lucini, M. Teper, U. Wenger, Glueballs and k-strings in SU(N) gauge theories, *JHEP* 06 (2004) 012.
- J. Milnor, Curvatures of left invariant metrics on Lie groups, *Adv. Math.* 21 (1976) 293–329.
- C. Morningstar, M. Peardon, The glueball spectrum from an anisotropic lattice study, *Phys. Rev. D* 60 (1999) 034509.
- Y. Ollivier, Ricci curvature of Markov chains on metric spaces, *J. Funct. Anal.* 256 (2009) 810–864.
- A. M. Polyakov, Thermal properties of gauge fields and quark liberation, *Phys. Lett. B* 72 (1978) 477–480.
- I. M. Singer, Some remarks on the Gribov ambiguity, *Commun. Math. Phys.* 60 (1978) 7–12; The geometry of the orbit space for non-abelian gauge theories, *Physica Scripta* 24 (1981) 817–820.
- G. 't Hooft, On the phase transition towards permanent quark confinement, *Nucl. Phys. B* 138 (1978) 1–25.
