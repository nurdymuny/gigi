# One note in every room: my theory of the Yang–Mills gap, where the problem stands, and what a proof needs

26 November 2026. **Research note. It states a theory and a programme, with every claim labelled.** It proves nothing
new. Proved results are cited with their scope, conjectures are marked as conjectures, and literature I have not
read in full is marked *verify*, following my literature check of 24 November
(`helike-literature-check-2026-11-24.md`). This is a living draft. I will reread and refine it as the work moves.

---

## 1. The problem, as it is actually posed

The Clay problem (Jaffe and Witten's official description, 2000) has two halves, and both are hard.

1. **Existence.** For a compact simple gauge group G, construct quantum Yang–Mills theory on four-dimensional
   spacetime as a genuine quantum field theory. That means satisfying axioms at least as strong as Wightman's or
   Osterwalder–Schrader's, and being non-trivial (interacting).
2. **The gap.** Prove that this theory has a mass gap Δ > 0: every state other than the vacuum carries energy at least
   Δ.

Two infinities make it hard:
- **The ultraviolet:** infinitely many short-distance modes must be controlled, which is renormalization.
- **The infrared:** the gap must survive in infinite volume, where confinement makes the large scales strongly
  coupled.

Anything I do in a finite-dimensional or finite-volume setting has removed at least one of these by construction. I
say this first so that nothing below is read as more than it is.

## 2. Where I am (proved, computed, open)

| result | scope | status |
|---|---|---|
| The Berger connection map and the Ricci/Yang–Mills continuum identity, D Φ_g[−2 Ric(g)] = W_g(Φ(g)) | positive diagonal left-invariant metrics on S³, declared convention | **proved** (symbolic, independently checked) |
| The Davis–Wilson lattice correspondence | full tangent mass norm, < 5% bar | **open**. The variational force M1d gives 8.8% at n = 3, against 26% for Whitney; it passes the amended M1 gates |
| Helike: the maximal-charge twist is an exact eigenvector of the Yang–Mills Hessian on Berger spheres, alone in her charge block, with a closed-form eigenvalue; where she is the floor, and where she is not | Berger spheres; the edge theorem proved on a stated domain | **proved** (*By Enough*, doi:10.5281/zenodo.24472820; Floor v3.3, concept 10.5281/zenodo.24386613) |
| The algebra of her price (12 theorems) | algebraic identities only | **proved in Lean** (branch feat/helike-lean) |
| The Helike Ladder Theorem: helicity² − flux, top rung leak-free | Berger spheres | **written**, *By Enough* v2 §4, draft |
| Theorem A: discrete spectrum of the sourced condensate | homogeneous (left-invariant) fields; every g > 0 on the registered window | **proved** |
| The ordering: Helike's excitation below the competitor's | homogeneous reduction; weak-coupling controls; outward-rounded bounds | **proved at 3 of 4 controls, conditional on S_C** |
| S_C | the same reduction, at the controls | **in progress.** C₃, the ray-aware fold, the outside bound and the enclosed base are proved. The interval Lehmann stage (R2) is running |
| The flux-edge grid (where the switch sits) | the same reduction, registered couplings | **running** (`research/flux-grid-20261126/`), with the gluing qualifier of §5.5 |

**What none of this is.** It is not four-dimensional Yang–Mills, not the construction of the theory, and not the
mass gap. The quantum results live in a finite-dimensional reduction: the fields that are the same at every point of
the sphere.

## 3. What others have done (the map I am working inside)

**The constructive programme (existence).**
- Glimm and Jaffe and collaborators built interacting quantum field theories in two and three dimensions (φ⁴ and
  related models). Four dimensions resisted.
- For Yang–Mills in four dimensions, Bałaban's renormalization-group series (Commun. Math. Phys., mid-1980s) proved
  ultraviolet stability for lattice Yang–Mills in a finite box. As far as I know it is the deepest rigorous progress
  on the existence half (*verify* exact scope).
- Magnen, Rivasseau and Sénéor constructed four-dimensional Yang–Mills with an infrared cutoff (Commun. Math. Phys.
  155, 1993; *verify*).
- Two-dimensional Yang–Mills is exactly solvable (Migdal; Gross, King and Sengupta; and others).

**The geometry of the space of field shapes.**
- Gribov (1978) showed that gauge fixing cannot pick one representative per orbit: the space of fields modulo gauge
  is *folded*.
- Singer (1978) showed this is topological, not a defect of a method.
- Babelon and Viallet (Commun. Math. Phys. 81, 1981) computed the curvature of the orbit space (*verify* details).
- Feynman (Nucl. Phys. B188, 1981) argued qualitatively that in 2+1 dimensions this geometry makes the configuration
  space effectively bounded, so that fluctuations have a lowest note: a gap.
- Karabali, Kim and Nair (Nucl. Phys. B524, 1998, and related work) turned that picture into calculations of the 2+1
  string tension and vacuum wavefunctional. These are not proofs (*verify*).

**Small volume (the bridge).**
- Lüscher (Nucl. Phys. B219, 1983) showed that in a small box the low-energy physics of Yang–Mills reduces, to
  leading order, to a finite-dimensional quantum system of the constant modes, dressed by the stiff ones.
- Koller and van Baal (Nucl. Phys. B302, 1988) followed this into larger boxes, where tunnelling between
  gauge-equivalent vacua and the sphaleron pass shape the spectrum.
- On the three-sphere, van Baal and Hari Dass (Nucl. Phys. B385, 1992) treated the constant-mode potential and its θ
  boundary. My Phase 1–2 reproduce their eqs. (54), (88), (92) and (101).
- Van den Heuvel with van Baal (hep-lat/9604017 and related papers; Leiden thesis 1996) computed glueballs on S³,
  reporting a 0⁺/2⁺ crossing for f = g²(R)/2π² > 0.3. They read it as a breakdown of the one-loop effective
  Hamiltonian.
- Kumar, Lechtenfeld and Picanço Costa (Nucl. Phys. B973, 2021) have, on the round sphere, the closed form that
  matches my λ = h² − B on the isotropic slice.
- Van Baal's review "QCD in a finite volume" (hep-ph/0008206) is the overview.

**Scaling and step scaling.** Lüscher, Weisz and Wolff (Nucl. Phys. B359, 1991) defined the running coupling *by the
finite box itself* and measured how it changes as the box doubles. Chaining doublings walks numerically from the
small perturbative box to the large hadronic one.

**Lattice spectroscopy.** Large-scale lattice computations put the lightest glueball (0⁺⁺) of pure SU(3) near
1.7 GeV (Morningstar–Peardon, Phys. Rev. D60, 1999, and later work). Physically, the gap is not in doubt; its proof
is.

**Probability.** Chatterjee and collaborators have opened a probabilistic attack on lattice Yang–Mills (from 2016).
It includes rigorous large-N and strong-coupling results, and a careful restatement of what remains (*verify*
specifics).

## 4. My theory

I state it first in plain geometry, then as precisely as I can. It grew from the stretched sphere, from Helike, and
from a week of watching which of our readings were real.

### 4.1 The gap is the lowest note of a folded drum

Every shape a gauge field can take is a point in a vast space. Gauge symmetry folds that space: shapes that differ
only by gauge are pressed onto each other, with creases where the copies meet (Gribov, Singer). What is left is
curved, and in the sense that matters, bounded.

A bounded drum has a lowest nonzero note. **The mass gap is the pitch of the smallest drum the folded space of field
shapes allows.** This picture is not new; it is Feynman's in 2+1 dimensions. What I add is the next point.

### 4.2 The note has a carrier, and she has a name

On Berger spheres the floor of the Yang–Mills Hessian is carried, over a whole region, by one identifiable field.
She is the maximal-charge twist, Helike: an exact eigenvector with a closed-form price. In the quantum reduction, her
excitation sits below its nearest competitor at weak coupling.

**Conjecture (carrier persistence).** As the room grows, the lowest note keeps a carrier whose identity persists. It
is the continuation of Helike's channel, or a channel she hands over to at an identifiable switch. The gap does not
dissolve into an anonymous continuum. If true, the gap can be tracked by following a field, not only by bounding a
spectrum.

### 4.3 One note in every room, once normalized by the field's own scale

In a small room the lowest note is set by the room: Δ(R) ≈ c/R. In infinite space it must be set by the field: a
fixed Δ that does not care about R.

**Conjecture (one note).** There is a normalization, by the field's own scale (Λ, generated by the running coupling)
and not the room's, under which the curves of lowest notes from all rooms are one curve. The *translator* is the map
that carries a room's reading to the field's reading.

The honest form of the test is concrete:
- Δ(R)·R grows without bound as R grows if and only if the gap survives;
- if the gap closes like 1/R, Δ(R)·R flattens.

Normalizing by the room would hide the difference. The translator must be able to say no.

### 4.4 The gluing must not become part of the reading

**Principle.** A reading that belongs to the field does not change when the room is cut differently, when rooms are
stitched differently, or when scales are joined by a different scheme. That covers gauge invariance, chart
invariance and scheme independence (universality). A reading that moves with the stitching belongs to the stitching.

This is not a slogan in my work; it is already operational:
- the leakage gate, where cutoff leakage is at most 10⁻³ of the residual;
- Neumann bracketing, where bounds hold whatever the seam does;
- the dressed-overlap cross-check, where identification survives a change of reference vacuum;
- the gluing-invariance test registered for the grid (flux-method amendment §5.5).

It is also the lesson of the idea in *The Geometry of Fuel* that did not survive. There, a term I added to the action
was the seam, and the seam was the whole reading.

### 4.5 Real and imaginary parts rhyme: the gap is where they meet

A response measured in imaginary (Euclidean) time and the same response in real time are two faces of one
holomorphic object:
- the Osterwalder–Schrader reconstruction is the license to continue from one to the other;
- the Källén–Lehmann representation and the Kramers–Kronig relations say the real part at one frequency is fixed by
  the imaginary part at all others;
- the gap is where the imaginary part (the spectral weight) first switches on;
- in imaginary time, the gap is the rate at which correlations die.

**Conjecture (holomorphic gluing).** The consistency conditions that let rooms be glued are the same as the
conditions that let the readings continue holomorphically from imaginary to real time. Analytic continuation is the
purest gluing: local pieces determine the global function uniquely. Rhymes between the real parts of one room's
readings and the imaginary parts of another's are what the translator must preserve. My Crouzeix-type function in
GIGI is my instrument for hearing those rhymes. *Its implementation has not been reviewed in this note.*

### 4.6 The method

Geometric visualisation chooses the object: the stretched sphere, the twist, the fold, the ray to the wall. Rigorous
enclosures settle each claim: interval and ball arithmetic, branch and bound, Lehmann–Temple bounds. Registration
before computation keeps the readings honest: stated tolerances, refusals as results, mutants that must be caught,
and seams varied on purpose.

I do not think the method replaces analysis. I think it chooses what the analysis should be about, and it does not
let me fool myself on the way.

## 5. What a proof requires, and my answer to each part

| requirement | why it is hard | my answer | status |
|---|---|---|---|
| **Existence (UV)** | infinitely many short-distance modes; renormalization; the OS axioms in the limit | Zooming in is a gluing of scales. Asymptotic freedom flattens the fine scales, so the zoom should converge, and the reading at each scale must be scheme-independent (§4.4). Bałaban's programme is the existing road | **not addressed** beyond this principle. I claim no progress |
| **Gap in a small room** | the infinitely many stiff modes must be shown only to *dress* the slow ones, rigorously | Small-sphere reduction (Lüscher, van Baal) plus rigorous enclosures and an identified floor (Helike). Next: rigorous bounds on the stiff-mode dressing (adiabatic separation) | **reduced model:** proved, or conditional on S_C. **Full theory at small R:** the next milestone |
| **Gap uniform in the room (IR)** | confinement makes the large scales strongly coupled; gluing is not weak | The translator (§4.3), gluing invariance (§4.4), carrier persistence (§4.2), holomorphic gluing (§4.5) | **conjecture.** This is the wall |
| **Non-triviality** | the limit must interact | Helike's structure is non-abelian: the flux term and the floor law vanish in the abelian theory | **in the reduction only** |

## 6. Milestones (each registered before it is run; each can fail)

1. **M1. S_C proved at the controls.** R2 interval Lehmann with exact base eigenfunctions. *Fails if* the interval
   ℓ₅ does not clear ρ_C at the frozen value.
2. **M2. The radius sweep with the gluing qualifier.** Map the reduced model's g² to the room radius through the
   running coupling, as in van den Heuvel's f = g²(R)/2π². Register the curve of Δ(R)·R, and compare with van den
   Heuvel's crossing. *Fails if* Δ·R flattens, or if the reading is seam-sensitive (§5.5).
3. **M3. Full theory at small R.** A rigorous effective-Hamiltonian statement: bounds on how the stiff modes dress
   Helike's channel, at small radius, uniformly in the ultraviolet cutoff. *Fails if* the dressing cannot be bounded
   below the gap.
4. **M4. Carrier persistence.** Track the identity of the lowest Q = 0 and Helike-channel levels across the sweep with
   the dressed overlap and the S1 seam test. *Fails if* identity is lost without an identifiable switch.
5. **M5. Holomorphic consistency in the reduction.** Compute the Euclidean correlator and its spectral threshold in
   the reduced model. Check that the threshold read from the continuation agrees with the spectrum, inside stated
   bounds. *Fails if* they disagree beyond the bounds.

## 7. What would show me wrong

- **Δ(R)·R flattens** in the sweep. The note would be the room's, not the field's.
- **A reading moves** when only the stitching changes. The note would belong to the seams.
- **The carrier dissolves** without a switch. The gap would not be tracked by a field.
- **The van den Heuvel crossing** turns out to be the same object as my switch, and it is an artefact of the
  truncation. My switch would then be an artefact too.

## 8. How far, honestly

The levels I have solved are real, and the method has not yet met a wall. That is partly because the walls are on a
floor I have not climbed to: the ultraviolet construction and the infinite-volume gap. The small-sphere bridge is
where a real road begins: Lüscher's and van Baal's road, made rigorous and given an identified floor.

I believe the gap is a property of the shape of the folded space of field shapes. I believe it is carried by a field
whose identity persists, the same note in every room once the field's own scale is used, and invariant under how the
rooms are joined. I hold those beliefs as conjectures. Each milestone above is a place where the geometry gets to
tell me yes or no.
