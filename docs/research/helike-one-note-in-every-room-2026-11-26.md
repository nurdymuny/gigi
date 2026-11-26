# One note in every room: my theory of the Yang–Mills gap, where the problem stands, and what a proof needs

26 November 2026. **Research note. It states a theory and a programme, with every claim labelled.** It proves nothing
new. Proved results are cited with their scope, conjectures are marked as conjectures, and literature I have not
read in full is marked *verify*, following my literature check of 24 November
(`helike-literature-check-2026-11-24.md`). This is a living draft. I will reread and refine it as the work moves.

*Refined the same afternoon:* §4.4 is stated in its sharper form, §4.6 adds the corridor and the double cover with
what the first controls show, and the milestones and falsifiers follow.

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
| The flux-edge grid (where the switch sits) | the same reduction, registered couplings | **registered; rerun next on my machine.** The domain gate now measures the cut's energy shift against the gap tolerance (flux-method amendment §5.7). The gluing qualifier of §5.5 is attached |
| The heat signal: corridor and tone (§4.6) | the same reduction; weak-coupling controls | **controls pass:** the corridor holds in both sectors and the tone is stable. Grid points run after the grid |

**What none of this is.** It is not four-dimensional Yang–Mills, not the construction of the theory, and not the
mass gap. The quantum results live in a finite-dimensional reduction: the fields that are the same at every point of
the sphere.

## 3. What others have done (the map I am working inside)

*Citations checked on 26 November against publisher pages, arXiv abstracts and the authors' own texts. Where a
source could not be read in full, the scope is stated from what could be read.*

**The constructive programme (existence).**
- Glimm, Jaffe and collaborators built interacting quantum field theories in two and three dimensions (φ⁴ and related
  models). Four dimensions resisted.
- **Bałaban** (Commun. Math. Phys., 1983–89; the four-dimensional ultraviolet-stability papers are 1987–89: CMP 109
  (1987) 249; 116 (1988) 1; 119 (1988) 243; 122 (1989) 175 and 355) gave a renormalization-group construction for
  pure four-dimensional lattice Yang–Mills in a finite periodic box. It proves **ultraviolet stability**: upper and
  lower bounds on the normalised partition function and effective actions, uniform in the lattice spacing.
  - **Not done:** continuum correlation functions or Wilson loops, the existence of the limit itself, the
    Osterwalder–Schrader axioms, infinite volume, a mass gap.
- **Magnen, Rivasseau and Sénéor**, "Construction of YM₄ with an infrared cutoff", Commun. Math. Phys. 155 (1993)
  325–383. They construct the Schwinger functions of pure SU(2) Yang–Mills in four dimensions, in the trivial
  topological sector, with the ultraviolet cutoff removed and an infrared cutoff kept. They used a continuum momentum
  cutoff, regularised axial gauge for large fields, and non-perturbative Slavnov identities. The OS axioms are not
  established, because the infrared cutoff is never removed.
- Two-dimensional Yang–Mills is exactly solvable (Migdal; Gross, King and Sengupta; and others).

**The geometry of the space of field shapes.**
- **Gribov**, "Quantization of non-Abelian gauge theories", Nucl. Phys. B139 (1978) 1–19, showed that gauge fixing
  cannot pick one representative per orbit: the space of fields modulo gauge is *folded*.
- **Singer**, "Some remarks on the Gribov ambiguity", Commun. Math. Phys. 60 (1978) 7–12, showed this is topological,
  not a defect of a method.
- **Babelon and Viallet**, "The Riemannian geometry of the configuration space of gauge theories", Commun. Math.
  Phys. 81 (1981) 515–525, computed the metric, connection, curvature tensor and sectional curvature of the orbit
  space, and tied the Gribov horizon to its focal points.
- The **sign** of that curvature I take from Orland (hep-th/0402003, 2004): sectional, Ricci and scalar curvature are
  formally non-negative (O'Neill-type). This is in infinite dimensions, and regularisation is needed. Vergeles
  (Lett. Math. Phys., 1983) reports a positive Ricci tensor.
- **Feynman**, "The qualitative behavior of Yang–Mills theory in 2+1 dimensions", Nucl. Phys. B188 (1981) 479, argued
  qualitatively that this geometry makes the configuration space effectively bounded, so that fluctuations have a
  lowest note: a gap.
- **Karabali and Nair**, Nucl. Phys. B464 (1996) 135–152, and **Karabali, Kim and Nair**, Nucl. Phys. B524 (1998)
  661–694 and Phys. Lett. B434 (1998) 103–109, turned that picture into a gauge-invariant Hamiltonian analysis in 2+1
  dimensions. It has a mass gap in the kinetic term and a string tension "in very good agreement" with lattice
  results. These are physics approximations, not proofs.

**Small volume (the bridge).**
- **Lüscher**, "Some analytic results concerning the mass spectrum of Yang–Mills gauge theories on a torus", Nucl.
  Phys. B219 (1983) 233–261: in a small box, the low-energy physics reduces at weak coupling to the constant modes,
  dressed by the stiff ones. The expansion is worked to one loop.
- **Koller and van Baal**, "A non-perturbative analysis in finite volume gauge theory", Nucl. Phys. B302 (1988) 1–64,
  followed this into larger boxes. There, tunnelling between gauge-equivalent vacua and the sphaleron shape the
  spectrum.
- **van Baal and Hari Dass**, "The theta dependence beyond steepest descent", Nucl. Phys. B385 (1992) 185–226, treated
  the three-sphere constant-mode potential and its θ boundary. My Phase 1–2 reproduce their eqs. (54), (88), (92) and
  (101).
- **van den Heuvel**, "One-loop effective action for SU(2) gauge theory on S³", Phys. Lett. B386 (1996) 233–240
  (hep-lat/9604017), computed glueballs on S³. Near f = g²(R)/2π² ≈ 0.25 he found m(2⁺)/m(0⁺) ≈ 1.5, in agreement
  with the lattice. He reads the scalar/tensor crossing beyond f = 0.3 explicitly as **a sign that the approximation
  breaks down, not physics**. See also his "Glueballs on the three-sphere", hep-lat/9608101 (Nucl. Phys. B488,
  1997).
- **Kumar, Lechtenfeld and Picanço Costa**, "Instability of cosmic Yang–Mills fields", Nucl. Phys. B973 (2021) 115583,
  has the round-sphere closed form that matches my λ = h² − B on the isotropic slice.
- **van Baal**, "QCD in a finite volume" (hep-ph/0008206; Ioffe Festschrift, World Scientific 2001), is the overview.

**Scaling and step scaling.**
- **Lüscher, Weisz and Wolff**, "A numerical method to compute the running coupling in asymptotically free
  theories", Nucl. Phys. B359 (1991) 221–243, defined the coupling *by the finite box itself* and measured its change
  as the box doubles. This was first done for the O(3) sigma model.
- The SU(2) gauge theory followed (Lüscher, Sommer, Weisz and Wolff, Nucl. Phys. B389, 1993).

**Lattice spectroscopy.** **Morningstar and Peardon**, "The glueball spectrum from an anisotropic lattice study",
Phys. Rev. D60 (1999) 034509, give SU(3) glueballs below 4 GeV, extrapolated to the continuum. The lightest 0⁺⁺ is
near 1.7 GeV. Physically, the gap is not in doubt; its proof is.

**Probability (from 2015).** **Chatterjee** and collaborators have produced rigorous results of four kinds:
- large-N strong-coupling results (CMP 366, 2019; arXiv 2015);
- weak-coupling Wilson loops for finite gauge groups (Chatterjee; Cao);
- a conditional confinement mechanism, where a strengthened mass gap would imply an area law (CMP 385, 2021);
- Gaussian scaling limits with a Higgs field.

"Yang–Mills for probabilists" (arXiv:1803.01950) is the careful restatement of what remains. None of it constructs a
non-Gaussian four-dimensional continuum theory or proves a mass gap at weak coupling.

**A curvature route to the gap, already proved where it is easiest.** **Shen, Zhu and Zhu**, "A stochastic analysis
approach to lattice Yang–Mills at strong coupling", Commun. Math. Phys. 400 (2023) 805–851, prove a unique
infinite-volume measure, log-Sobolev and Poincaré inequalities, and exponential decay of correlations (a positive
mass gap) for lattice Yang–Mills at **strong coupling**, in any dimension d > 1, uniformly in N.
- The mechanism is a Bakry–Émery-type curvature condition: the Ricci curvature of the group dominates the Hessian of
  the Wilson action when the coupling is strong.
- This is the dimension-free geometric mechanism my heat-signal registration (T5) probes in the reduction. The hard
  case is the opposite regime, weak coupling, where the continuum limit lives.

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
- the domain gate, where cutting a state to the physical domain may move its energy by at most a tenth of the
  tolerance I report energies to (flux-method amendment §5.7);
- Neumann bracketing, where bounds hold whatever the seam does;
- the dressed-overlap cross-check, where identification survives a change of reference vacuum;
- the gluing-invariance test registered for the grid (flux-method amendment §5.5).

It is also the lesson of the idea in *The Geometry of Fuel* that did not survive. There, a term I added to the action
was the seam, and the seam was the whole reading.

**The sharper form.** A check on a seam must be stated in the units of the reading, never in the units of the
instrument. The question is always: can this seam move the number I report, by an amount that matters at the
tolerance I report it to? A check written as a fraction of the instrument's own error tightens as the instrument
improves. Then the better I measure, the more the stitching decides the verdict, and the gluing has become part of
the reading after all.

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

### 4.6 The gap is the corridor: every disturbance ends on one note

Strike a drum anywhere, in any way, and wait. The high ringing dies first. What is left is the lowest note the strike
could reach, and it is the same note whichever way it was struck.

In imaginary time this is the heat kernel. A disturbance φ, evolved by e^{−τH}, fades as
Σₙ |⟨n|φ⟩|² e^{−τ(Eₙ − E₀)}. After the transients, one rate survives: the first level φ touches.

**Conjecture (the corridor).** In each sector, every disturbance that can reach the lowest level does, and they all end
on the same note. The gap is not one excitation among many. It is the corridor that every disturbance of the sector
leaves through.

That is why one field can carry it (§4.2): the carrier is the shape of the corridor. And because every field that
couples to the sector must pass through it, the gap is also what connects them. It is the secret corridor between
fields.

**The double cover: octave and tone.** A reading of the notes has two parts.
- The **octave** is the overall scale Δ. In a small room it is set by the room; in infinite space it must be set by
  the field's own scale Λ.
- The **tone** is the pattern of ratios (Eₙ − E₀)/Δ. It belongs to the field.

As in music, one tone sounds in every octave. Each room plays the field's tone in its own octave, and the readings
from all rooms cover the one note. This is the second double cover in the work, after Helike's two partners covering
one channel. It gives §4.3 a sharper form, in two halves:
- **(i)** the tone is the same in every room;
- **(ii)** the octave, read in the field's units, does not fall to zero as the room grows.

Half (i) is the corridor seen from the side. Half (ii) is the gap.

**What the reduction says so far.** The registered heat-signal experiment (`helike-heat-signal-registration-2026-11-26.md`)
ran at the weak-coupling controls, g² = 0.002 and 0.004.
- **The corridor holds in both sectors.** All five sources in Helike's sector end on her level, and all four Q = 0
  sources end on the competitor's.
- **The corridor agrees with the registered identification.**
- **The tone is stable** between the two couplings, to 0.005.
- **The signal does not move with the seams** (basis size and quadrature).

At these couplings the spectrum is nearly harmonic, so this checks the instrument, not the conjecture. The grid
points, where the switch lives, are the test.

### 4.7 The method

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
| **Gap uniform in the room (IR)** | confinement makes the large scales strongly coupled; gluing is not weak | The translator (§4.3), gluing invariance (§4.4), carrier persistence (§4.2), holomorphic gluing (§4.5), the corridor and the invariant tone (§4.6) | **conjecture.** This is the wall |
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
6. **M6. The corridor across the switch.** The registered heat signal at the grid points: the corridor in both
   sectors, and the tone stable while the octave moves. *Fails if* a sector splits (its disturbances end on different
   notes), or the tone shifts where only the octave should move.

## 7. What would show me wrong

- **Δ(R)·R flattens** in the sweep. The note would be the room's, not the field's.
- **A reading moves** when only the stitching changes. The note would belong to the seams.
- **The carrier dissolves** without a switch. The gap would not be tracked by a field.
- **The corridor splits.** Disturbances of one sector end on different notes. The gap would not be a common exit.
- **The van den Heuvel crossing** turns out to be the same object as my switch, and it is an artefact of the
  truncation. My switch would then be an artefact too.

## 8. How far, honestly

The levels I have solved are real, and the method has not yet met a wall. That is partly because the walls are on a
floor I have not climbed to: the ultraviolet construction and the infinite-volume gap. The small-sphere bridge is
where a real road begins: Lüscher's and van Baal's road, made rigorous and given an identified floor.

I believe the gap is a property of the shape of the folded space of field shapes. I believe it is carried by a field
whose identity persists, that it is the corridor every disturbance leaves through, that it is the same tone in every
room once the field's own scale sets the octave, and that it is invariant under how the rooms are joined. I hold those beliefs as conjectures. Each milestone above is a place where the geometry gets to
tell me yes or no.
