# One note in every room: my theory of the Yang–Mills gap, where the problem stands, and what a proof needs

26 November 2026. **Research note. It states a theory and a programme, with every claim labelled.** It proves nothing
new. Proved results are cited with their scope, conjectures are marked as conjectures, and literature I have not
read in full is marked *verify*, following my literature check of 24 November
(`helike-literature-check-2026-11-24.md`). This is a living draft. I will reread and refine it as the work moves.

*Refined the same afternoon:* §4.4 is stated in its sharper form, §4.6 adds the corridor, octave and tone with what
the first controls show, and the milestones and falsifiers follow.

*Refined again after an outside review (26 November):*
- §4.0 separates the three hypotheses by strength, with carrier persistence at the centre.
- §4.1 corrects the drum: the folded space is not bounded, and the vacuum's weight does the work.
- §4.3 corrects the radius test.
- §4.6 says what the corridor predicts beyond ordinary spectral filtering, and measures it.
- §4.7 adds the exact weighted-geometry form of the gap in my reduction.
- §4.5 names the data that holomorphic gluing would have to connect.
- §5 adds the bridge from the sourced reduction to the unsourced theory.

*And after a second review:*
- §4.7 now carries the reduced measure in the curvature, and uses complex conjugates.
- It separates what curvature constrains from what decides the carrier.
- M8 allows a carrier to deform continuously.
- T5 is described as proposed, not computed.

*Added the same evening:* §4.9, the inherited note: the gap descends from the intrinsic geometry of one turn, with M9
and a falsifier. *Tightened after a third review:*
- what a gap does and does not price;
- the group-metric normalisation;
- M9 now needs a measurable inherited property, plus cutoff checks and a compact-U(1) control;
- M8's fingerprint and refinement are corrected (amendment 2).

*And a fourth review:*
- M8's switch and handover contract is made precise (amendment 3).
- The U(1) controls now declare the scale held fixed, and gap survival is separated from survival of the carrier's
  properties.

*Tightened before implementation:*
- A crossing needs a subdivision-consistent carrier path, not just a confident match between endpoints.
- String tension is a ruler only where it is nonzero. The four-dimensional U(1) Coulomb control has its own
  masslessness test; I do not divide by a vanished ruler.

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

**The continuum version of the same idea.** **Moncrief, Marini and Maitra**, "Orbit Space Curvature as a Source of
Mass in Quantum Gauge Theory", arXiv:1809.06318 (2018, revised 2019), take up Singer's proposal.
- The orbit space is positively curved but of infinite diameter.
- With the Yang–Mills potential included, the role of the Ricci tensor is played by a Bakry–Émery Ricci tensor
  computed from the ground-state wave functional.
- A positive lower bound on that tensor gives a gap.
- They state this as a programme. The infinite-dimensional tensor needs regularisation.

**Mondal**, "A Geometric Approach to the Yang–Mills Mass Gap", arXiv:2301.06996 (2023), argues for a gap in 2+1 and
3+1 dimensions from a regularised Bakry–Émery curvature.
- The argument assumes that a quantised theory exists and that the curvature satisfies a pointwise bound.
- The author calls the 3+1 case not yet rigorous.
- *verify* in full; I have read the introduction and the theorem statement.

## 4. My theory

I state it first in plain geometry, then as precisely as I can. It grew from the stretched sphere, from Helike, and
from a week of watching which of our readings were real.

### 4.0 Three questions, and three hypotheses of different strength

An energy curve answers only the first of three questions:
- What is the energy?
- What field carries it?
- Does that identification survive changes in how I represent and measure it?

Two curves can cross while their states keep distinct identities. A smooth curve can hide a complete change of the
state carrying it. That is why I follow eigenvectors, channel overlaps and degenerate subspaces, not only levels.

I hold three hypotheses, and they are separate. Each can fail without the others failing.
- **H1. Carrier persistence (the centre).** The lowest note in a sector has a carrier whose identity can be followed.
  When it changes, it changes at an identifiable switch, and I can say why one field stays cheapest or how another
  takes over (§4.2, §4.7).
- **H2. Gap survival.** Measured in a field scale fixed independently of the room, the lowest excitation stays
  bounded away from zero as the room grows (§4.3).
- **H3. Invariant tone (strongest).** The pattern of higher ratios is the same in every room (§4.6). A field can keep
  a positive gap while its higher ratios change. A tone shift would refute H3 and leave H1 and H2 standing.

Even if H3 needs revision, an explanation of why a particular excitation stays cheapest is the result worth having.

### 4.1 The gap is the lowest note of a folded drum

Every shape a gauge field can take is a point in a vast space. Gauge symmetry folds that space: shapes that differ
only by gauge are pressed onto each other, with creases where the copies meet (Gribov, Singer). What is left is
curved: Singer showed that the orbit space is positively curved.

It is **not** bounded. The Yang–Mills orbit spaces have infinite diameter (Moncrief, Marini and Maitra,
arXiv:1809.06318, citing earlier work). So the drum is not small because its walls are close. It is small because of
where the vacuum puts its weight. The potential confines the vacuum, and the measure weighted by the vacuum's
probability is finite even where the space is not. Moncrief, Marini and Maitra make this a gap estimate. In the
Bochner identity weighted by |Ψ₀|², the Ricci tensor is replaced by a Bakry–Émery Ricci tensor built from the ground
state. If that tensor is bounded below by 1/ℓ₀², the gap is at least ħ²/(2m ℓ₀²). They state this as a programme: in
infinite dimensions the tensor needs regularisation, and its positivity is sufficient, not necessary.

**The mass gap is the pitch of the smallest drum that the folded space of field shapes, weighted by where the vacuum
actually lives, allows.** The unweighted picture is Feynman's in 2+1 dimensions. The weighting is Moncrief, Marini
and Maitra's. What I add is the next point.

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

The honest form of the test is concrete. **The gap survives if Δ(R), measured in a field scale fixed independently
of the room (Λ, or a physical mass held fixed), stays bounded away from zero as R grows.** That is H2 of §4.0.

The product Δ(R)·R is only a one-way test:
- If it flattens, the gap closes like 1/R, and H2 fails.
- If it grows, that is necessary for survival but not sufficient. A gap falling like 1/√R gives Δ·R ∝ √R, which grows
  without bound, while the gap still goes to zero.

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

**What is still undefined.** Spatial gluing, a change of renormalisation scheme and analytic continuation are three
different operations. For the conjecture to say anything, I must name the data each one acts on and the compatibility
condition that connects them. Until then it is a direction, not a statement.

The most concrete starting point is the one that uses positivity. Fei, Yeh and Gull, "Nevanlinna Analytical
Continuation", Phys. Rev. Lett. 126 (2021) 056402, continue Euclidean Green's functions while keeping the spectral
function positive and normalised. They treat the problem as Nevanlinna interpolation, with a Pick-type criterion for
when a continuation exists. Finitely many Euclidean values still allow many compatible continuations. Any claim that
the threshold is "read from the continuation" must therefore come with the whole family of continuations, not just
one of them. For M5, that means reporting the interval of thresholds that the Nevanlinna–Pick family allows.

### 4.6 The gap is the corridor: every disturbance of a stated class ends on one note

Strike a drum anywhere, in any way, and wait. The high ringing dies first. What is left is the lowest note the strike
could reach, and it is the same note whichever way it was struck.

In imaginary time this is the heat kernel. A disturbance φ, evolved by e^{−τH}, fades as
Σₙ |⟨n|φ⟩|² e^{−τ(Eₙ − E₀)}. After the transients, one rate survives: the first level φ touches.

**What is automatic, and what is not.** That a source with any overlap on the lowest level is eventually dominated by
it follows from the spectral expansion alone. That part is not a discovery. "Eventually" can also hide a great deal.
At g² = 0.002, 99.85% of source H″'s weight lies above the twelve retained levels, and it reaches the corridor through
a small component. A disturbance orthogonal to the lowest state can also stay in a higher level of the same sector
for ever. That would not disprove a gap.

**Conjecture (the corridor), in the form that can fail.** Fix a stated, physically meaningful class of disturbances in
each sector. In my reduction these are the registered monomial sources of the heat-signal experiment. Across changing
rooms, every disturbance in the class keeps a substantial share of its weight on one identifiable channel, and
reaches it in a comparable time. Three things are measured, not only the end level:
- the weight each source puts on the corridor;
- the imaginary time it takes for the corridor to dominate its signal;
- whether that weight and that time hold up across the switch.

The gap is then not one excitation among many. It is the channel this class of disturbances leaves through.

That is why one field can carry it (§4.2): the carrier is the shape of the corridor. To the extent that every
disturbance in the stated class leaves through it, the gap is also what connects them. It is the secret corridor
between those disturbances. Whether the class can be widened, and how far, is part of what M6 measures.

**Octave and tone.** A reading of the notes has two parts.
- The **octave** is the overall scale Δ. In a small room it is set by the room; in infinite space it must be set by
  the field's own scale Λ.
- The **tone** is the pattern of ratios (Eₙ − E₀)/Δ. It belongs to the field.

As in music, one tone sounds in every octave. Each room plays the field's tone in its own octave. This is a
decomposition into scale and shape. I first called it a double cover, after Helike's two partners covering one
channel. It does not earn that name until I can specify a two-to-one map, so for now the name is an image, not a
theorem.

It splits the claim into two halves, and they are different hypotheses:
- **(i)** the tone is the same in every room. This is H3.
- **(ii)** the octave, read in the field's units, does not fall to zero as the room grows. This is H2.

A tone shift refutes (i) without touching (ii).

**What the reduction says so far.** The registered heat-signal experiment (`helike-heat-signal-registration-2026-11-26.md`)
ran at the weak-coupling controls, g² = 0.002 and 0.004.
- **The corridor holds in both sectors.** All five sources in Helike's sector end on her level, and all four Q = 0
  sources end on the competitor's.
- **The corridor agrees with the registered identification.**
- **The tone is stable** between the two couplings, to 0.005.
- **The signal does not move with the seams** (basis size and quadrature).

At these couplings the spectrum is nearly harmonic, so this checks the instrument, not the conjecture. The heat
curves are also built from the same computed levels and overlaps. Their agreement with that spectrum is a
consistency check, not an independent measurement of it. The grid points, where the switch lives, are the test.

| g² | Helike-channel plateau | competitor-channel plateau |
|---|---|---|
| 0.002 | 1.089042353 | 1.101143780 |
| 0.004 | 1.087968660 | 1.095559030 |

(model units, which are units of 1/R)

### 4.7 Why one field stays cheapest: the gap as weighted geometry

In my reduction this is exact, not a picture.
- The kinetic term is a Dirichlet form, ½ g^{ij} ∂ᵢψ̄ ∂ⱼψ, taken over the reduced model's own measure dμ.
- That measure is not the volume of the kinetic metric. In the chart it is dμ = w dvol_g, with a nonconstant density
  w. The density comes from the Gram coordinates, through the factor e^{3u₀+2u₁+u₂} and the metric's determinant.
- For the exact ground state, ψ₀ is positive.

Write any state as ψ = f ψ₀. Then, for the exact ground state, and for f in the form domain,

  ⟨ψ, (H − E₀) ψ⟩ = ½ ∫ g^{ij} ∂ᵢf̄ ∂ⱼf · ψ₀² dμ.

The gap is the smallest such value over f with ∫ f ψ₀² dμ = 0 and ∫ |f|² ψ₀² dμ = 1. This is the ground-state
transformation. H − E₀ becomes a weighted Laplacian on the field space, with measure ψ₀² dμ = w ψ₀² dvol_g.
- **For a computed ψ₀**, the identity gains a residual term, ⟨|f|²ψ₀, (H − E₀)ψ₀⟩. It must be controlled, not
  assumed away. M8 measures it.

Two consequences follow, and both can be measured.
- **The carrier is a shape on that geometry.** The ground state is invariant under the charge rotation, so the
  transformation respects charge. In each charge sector, the lowest state is ψ₁ = f₁ψ₀, where f₁ is the slowest
  function of that charge on the vacuum-weighted geometry. Helike's channel is the statement that, over a region, the
  slowest charge-2 function is, to leading order, her twist coordinate h₁ + i h₂. Following f₁ across the switch
  shows either why she stays cheapest, or exactly which direction becomes cheaper. This is H1 made geometric.
- **The constraint has a curvature.** The weighted curvature has to carry the reduced measure:

    Ric_g − Hess_g log(w ψ₀²),

  with the covariant Hessian of the kinetic metric. Moncrief, Marini and Maitra start from the metric's own volume, so
  their formula applies here only with w carried along. If this tensor is bounded below by κ > 0, every excitation,
  in every sector, costs at least κ/2 with this normalisation.

**What each tool decides.** The curvature and the comparison answer different questions, and neither substitutes for
the other.
- **The curvature bound constrains how cheap any excitation can be.** It does not say which one is cheapest. Two
  oscillators can exchange which is lower while a positive bound holds throughout. Losing the bound, which is only
  sufficient, need not produce a switch, or close the gap.
- **Comparing sector energies and the geometry of their states decides which excitation is cheapest.** It also shows
  how the carrier changes shape, where its weight lies, and how its cost moves across the switch. That is the carrier
  question, and M8 is built on it.

This is the finite-dimensional, rigorous counterpart of the Moncrief–Marini–Maitra programme, and of the
strong-coupling mechanism of Shen, Zhu and Zhu. The heat-signal registration proposes T5 to investigate a floating
version of this bound. T5 is deferred, and has not been implemented or computed.

### 4.8 The method



Geometric visualisation chooses the object: the stretched sphere, the twist, the fold, the ray to the wall. Rigorous
enclosures settle each claim: interval and ball arithmetic, branch and bound, Lehmann–Temple bounds. Registration
before computation keeps the readings honest: stated tolerances, refusals as results, mutants that must be caught,
and seams varied on purpose.

I do not think the method replaces analysis. I think it chooses what the analysis should be about, and it does not
let me fool myself on the way.

### 4.9 The inherited note: the gap descends from the intrinsic geometry of one turn

**The picture (motivation, not a deduction).** A thing that exists has to take up space equally in every direction.
So it is curved, just because it exists (my note `unit-curvature-2026-11-18.md`). A curved, closed shape has a lowest
nonzero note. My gut says the gap we see in every field is that note: the note of the first thing that turned,
inherited by everything after it. This is the picture that led me here. Nothing below is derived from it.

The same gap bears on two things that look opposite, and I state precisely what it does.
- **It does not price every change.** A state can carry an arbitrarily small admixture of an excitation, and so sit
  arbitrarily little above the vacuum on average. What the gap prices is the spectrum: every stationary state other
  than the vacuum costs at least Δ. As a consequence, correlations between distant regions die off like e^{−Δ·distance}.
- **So "not falling into one another" means this.** There is no massless mode, nothing that can carry an influence
  arbitrarily far for arbitrarily little. Distant regions stay effectively separate.
- **And yet they share.** Their influence passes through transport, and it fades at the rate the gap sets. Distinct
  but connected is the picture a gapped connection fits. Making "distinct things stay distinct" precise beyond the
  decay of correlations is open.

**Parallel transport is the gauge field.** A connection is the rule for carrying a frame from one point to another
and comparing it. Carry it around a small loop and it comes back turned, by a holonomy U, and that turn is the field
strength. Wilson's lattice energy prices the turn by its chord, 1 − ½ Re tr U. That is the subject of *Three Ways
Across a Turn: Chord, Arc and Tangent* (doi:10.5281/zenodo.24375338): the chord, the arc and the tangent are three
prices for the turn that transport makes.

**Proved: at strong coupling the gap is the note of the turn.** In the Hamiltonian lattice theory (Kogut and
Susskind, Phys. Rev. D 11 (1975) 395), the energy at strong coupling is the electric term.
- On each link, that term is g²/(2a) times the quadratic Casimir. It is the Laplacian on the group itself, SU(2) = S³,
  a compact, positively curved shape.
- **The normalisation, explicitly.** The Casimir is the group Laplacian in the bi-invariant metric for which S³ has
  radius 2. Its first nonzero value is j(j+1) = 3/4 at j = ½. On the unit-radius S³, the first nonzero value of the
  Laplacian is 3, four times larger. The energies below use the Casimir convention.
- Gauss's law allows only closed loops of flux. Take the electric-only Hamiltonian on a square or cubic lattice, with
  no external charges and no winding loops shorter than a plaquette. The lightest excitation is then the smallest
  loop, one plaquette of four links, each carrying j = ½.
- Its energy is 4 · (g²/2a) · 3/4 = 3g²/(2a). This is the leading strong-coupling result, not the exact gap: the
  magnetic term mixes the loop with other configurations at the next orders. (For the standard Hamiltonian
  treatment, see Zohar, Cirac and Reznik, Rep. Prog. Phys. 79 (2016) 014401.)
- The higher levels at that order follow from the same two ingredients: the Casimirs of the group (j = 1 gives 2, a
  ratio of 8/3 to the first) and the lengths of the loops (a two-plaquette loop gives 6/4).

So at strong coupling, both the octave and the tone are read directly off the intrinsic geometry of one turn and the
shapes of the loops. The proof of Shen, Zhu and Zhu works the same way: the curvature of the group dominates.

**Conjecture (the inherited note).** The note of the turn is not replaced as the coupling weakens. It descends.
Followed continuously from strong coupling toward the continuum, the lowest note keeps an identifiable descent from
the group's own spectrum: the same carrier (H1), never reaching zero (H2), and perhaps a recognisable tone (H3). The
gap of the continuum theory would then be the inherited note of the turn, rescaled by the running coupling into the
field's own scale.

**What is open.** The whole of it below strong coupling. Classically, a small turn costs almost nothing (the chord
goes like θ²), so at weak coupling the gap cannot be read off the group. It has to survive the descent, and that is
the Clay problem. The cosmological reading, that a first particle set the note for everything after, is beyond
anything I can test. I state it as the picture that led me here, not as a claim.

**Where it can be tested.** On the lattice (Halcyon), not in my homogeneous reduction. At strong coupling the
reduction's kinetic term is a Laplacian on its reduced configuration space, not on the group, so the clean comparison
with Casimirs belongs to the lattice.

**What "inherited" must mean.** Following an eigenstate continuously is not enough. In a finite calculation, isolated
eigenstates can usually be followed as parameters change, so "we followed it" would prove nothing. Inheritance has to
be a **measurable property of the starting carrier** that persists as the dynamics change, and persists as the cutoffs
are removed. Candidates:
- **Representation content:** the share of the excitation's electric energy carried in j = ½ flux.
- **Loop structure:** the spatial extent of its excess electric energy, against the single plaquette.
- **Creation:** its overlap with the state the fundamental plaquette Wilson loop creates from the vacuum.

**The controls that keep me honest: compact U(1).** Compact U(1) also has a discrete electric spectrum on each link,
and its strong-coupling excitations are flux loops too. Two of its versions behave differently, and each tests
something different.

First, a distinction the controls require. "Surviving" has to say **which physical scale is held fixed** as the cutoff
is removed. And **gap survival** and **survival of the starting carrier's properties** are separate predictions:
either can hold without the other. For the confining SU(2) and three-dimensional U(1) comparisons, the reference
energy scale is √σ, where σ is the nonzero string tension, and the gap is read as m/√σ. The continuum comparison
holds that string-tension scale fixed. It is not a usable ruler in a phase where σ vanishes.
- **Four-dimensional compact U(1).** Its weak-coupling Coulomb phase has a massless photon and zero asymptotic
  confining string tension. I therefore report m/√σ as **not applicable**, with the reason σ = 0, not as zero or
  infinity. The control instead follows the photon channel at fixed bare coupling in the Coulomb phase while the
  spatial volume grows. Nonzero momenta approach zero with increasing box size. Their energies test the low-momentum
  dispersion E(p)² = mγ² + c²|p|², with lattice corrections and finite-volume effects accounted for, rather than
  assuming a zero energy on one finite box. The required outcome is masslessness in the infinite-volume limit;
  phase identification and the loss of confining string tension are reported separately. For the potential and its
  finite-size corrections, see Cella, Heller, Mitrjushkin and Viceré, *The Coulomb law in the pure gauge U(1) theory on
  a lattice*, [arXiv:hep-lat/9704012](https://arxiv.org/abs/hep-lat/9704012).
- **Three-dimensional compact U(1): the subtle control.** The Polyakov mechanism gives a gap and confinement at
  finite lattice coupling; the statement is not merely that a finite box has discrete levels. But m and √σ do not
  scale together. In the standard continuum limit, m/√σ → 0 (Athenodorou, Bonati and Soler Calero,
  [arXiv:2608.27976](https://arxiv.org/abs/2608.27976), 2026). Hold the string tension fixed and the gap goes to zero;
  hold the gap fixed and the string tension diverges. So gapped at every cutoff is not inheritance in the field's
  scale. A diagnostic that reads "positive at every cutoff" as "kept" fails this control.
- **SU(2) in four dimensions.** Lattice studies find m/√σ finite in the continuum limit. That is the gap's survival
  in the field's scale. Whether the starting carrier's properties survive with it is the separate question M9 asks.

A diagnostic that calls SU(2)'s note inherited must also report four-dimensional U(1)'s gap as lost, and
three-dimensional U(1)'s gap as lost in the string-tension scale. Otherwise it measures nothing. The carrier's
properties are reported separately in all three. These are distinct control protocols, not three curves forced onto
one ruler. Before M9 runs, I will register the actions, couplings, volumes, momentum channels, fit windows,
discretization and uncertainty checks, and numerical acceptance criteria. A zero-mass fit inside an unresolved
uncertainty is OPEN, not evidence of masslessness. No U(1) control result is claimed here.

## 5. What a proof requires, and my answer to each part

| requirement | why it is hard | my answer | status |
|---|---|---|---|
| **Existence (UV)** | infinitely many short-distance modes; renormalization; the OS axioms in the limit | Zooming in is a gluing of scales. Asymptotic freedom flattens the fine scales, so the zoom should converge, and the reading at each scale must be scheme-independent (§4.4). Bałaban's programme is the existing road | **not addressed** beyond this principle. I claim no progress |
| **Gap in a small room** | the infinitely many stiff modes must be shown only to *dress* the slow ones, rigorously | Small-sphere reduction (Lüscher, van Baal) plus rigorous enclosures and an identified floor (Helike). Next: rigorous bounds on the stiff-mode dressing (adiabatic separation) | **reduced model:** proved, or conditional on S_C. **Full theory at small R:** the next milestone |
| **Gap uniform in the room (IR)** | confinement makes the large scales strongly coupled; gluing is not weak | The translator (§4.3), gluing invariance (§4.4), carrier persistence (§4.2, §4.7), holomorphic gluing (§4.5), the corridor (§4.6); the invariant tone is a separate, stronger claim | **conjecture.** This is the wall |
| **From the sourced reduction to the unsourced theory** | my reduction is a *sourced* condensate: the sources hold the background, and they change the spectrum | The sources must be removed, controlled, or matched to an effective description of the unsourced theory. Assigning a running coupling to a radius does not supply this connection by itself. The first step is to track every reading as the sources are turned down, to the edge of the window where Theorem A holds | **open.** The next mathematical question, and a condition on M2 |
| **Non-triviality** | the limit must interact | Helike's structure is non-abelian: the flux term and the floor law vanish in the abelian theory | **in the reduction only** |

## 6. Milestones (each registered before it is run; each can fail)

1. **M1. S_C proved at the controls.** R2 interval Lehmann with exact base eigenfunctions. *Fails if* the interval
   ℓ₅ does not clear ρ_C at the frozen value.
2. **M2. The radius sweep with the gluing qualifier.** Map the reduced model's g² to the room radius through the
   running coupling, as in van den Heuvel's f = g²(R)/2π². Register Δ(R) in a field scale fixed independently of the
   room, and Δ(R)·R as the one-way check (§4.3). Compare with van den Heuvel's crossing. *Fails if* Δ·R flattens,
   Δ in the field scale trends to zero, or the reading is seam-sensitive (§5.5). It is reported as a statement about
   the sourced reduction until the source bridge (§5) is crossed.
3. **M3. Full theory at small R.** A rigorous effective-Hamiltonian statement: bounds on how the stiff modes dress
   Helike's channel, at small radius, uniformly in the ultraviolet cutoff. *Fails if* the dressing cannot be bounded
   below the gap.
4. **M4. Carrier persistence.** Track the identity of the lowest Q = 0 and Helike-channel levels across the sweep with
   the dressed overlap and the S1 seam test. *Fails if* identity is lost without an identifiable switch.
5. **M5. Holomorphic consistency in the reduction.** Compute the Euclidean correlator and its spectral threshold in
   the reduced model. Check that the threshold read from the continuation agrees with the spectrum, inside stated
   bounds. *Fails if* they disagree beyond the bounds.
6. **M6. The corridor across the switch (H1).** The registered heat signal at the grid points, measuring for each
   registered source its weight on the corridor and its time to dominance, as well as its end level. *Fails if* a
   sector splits (its sources end on different notes), or the corridor weight of the registered source class
   collapses across the switch.
7. **M7. The tone (H3, separately).** The ratio vector across the grid. *Fails if* the tone shifts where only the
   octave should move. This refutes H3 only.
8. **M8. The weighted geometry of the carrier (H1, §4.7).** From the computed ψ₀ and ψ₁, the slow function f₁ = ψ₁/ψ₀.
   Follow each channel's carrier across the s-sweep by its overlap in the physical Hilbert space, with degenerate
   partners followed together. The decided quantity is a measurable property of the carrier: its own-block share, the
   fraction of its excitation energy spent in its own coordinates.
   - *Fails if* that share falls below one half while the carrier is still the channel's lowest level, and no verified
     level crossing has handed the channel to another state.
   - A change in which level an overlap selector picks is not a crossing.
   - Even a confident endpoint match must agree with tracking through intermediate points after refinement. A false
     exchange introduced by a coarse match cannot verify itself through its own reversed energy labels. Unresolved
     matching or energy ordering gives OPEN, not a handover.
   - Once a verified crossing hands the channel over, the new carrier is followed from then on.
   - HANDS OVER records that transfer; it does not test indefinite retention of the original geometry by its successor.
   - Smoothness alone decides neither persistence nor failure. The own-block share decides the geometric prediction;
     continuity describes how it changes. The thresholds and model are unchanged.
   Registered: `helike-M8-weighted-geometry-registration-2026-11-26.md`.
9. **M9. The inherited note (§4.9), proposed.** On the SU(2) lattice (Halcyon), follow the lowest levels from strong
   coupling, where they are fixed by the Casimirs and the loop lengths, toward weak coupling.
   - Measure the starting carrier's properties along the way: representation content, loop extent and Wilson-loop
     overlap.
   - Check them as the lattice spacing, the volume and the representation cutoff change.
   - In three-dimensional compact U(1), hold the string-tension scale fixed and require m/√σ → 0 under cutoff
     removal, despite a positive gap at each finite coupling.
   - In four-dimensional compact U(1), identify the Coulomb phase and test masslessness through photon dispersion
     and increasing volume at fixed bare coupling. Its vanishing string tension makes m/√σ not applicable there.
     Do not demand an exactly zero excitation energy on a finite box.
   - Report the carrier's properties separately from gap survival in every theory. Register the control-specific
     acceptance criteria before running; the two U(1) controls do not use a common nonzero string-tension scale.
   - The control tests must reject division by zero string tension, a finite-box spacing called a mass, a positive
     finite-cutoff gap called continuum survival, and a carrier overlap used in place of the masslessness test. Each
     rejection needs its own planted fixture and mechanism-removal check.

   *Fails if* the starting carrier's properties are lost at weak coupling with no identifiable switch, or if the
   diagnostic cannot tell four-dimensional U(1) from SU(2). A clean handover to another loop or representation does
   not fail it: that would be a switch, as with Helike. Not yet registered.

## 7. What would show me wrong

- **Δ(R)·R flattens** in the sweep. The note would be the room's, not the field's.
- **A reading moves** when only the stitching changes. The note would belong to the seams.
- **The carrier dissolves** without a switch. The gap would not be tracked by a field.
- **The corridor splits**, or its weight collapses, for the stated source class. The gap would not be a common exit.
- **The tone shifts** where only the octave should move. This would refute H3, the strongest form, and leave the
  carrier and the gap standing.
- **The inherited note breaks.** At weak coupling the lowest note keeps none of the strong-coupling carrier's
  measurable properties, and there is no identifiable switch. Then the gap would be made at weak coupling, not
  inherited.
- **The van den Heuvel crossing** turns out to be the same object as my switch, and it is an artefact of the
  truncation. My switch would then be an artefact too.

## 8. How far, honestly

The levels I have solved are real, and the method has not yet met a wall. That is partly because the walls are on a
floor I have not climbed to: the ultraviolet construction and the infinite-volume gap. The small-sphere bridge is
where a real road begins: Lüscher's and van Baal's road, made rigorous and given an identified floor.

I believe the gap is a property of the shape of the folded space of field shapes. I believe it is carried by a field
whose identity persists, and that the reason it stays cheapest is a curvature of the space of field shapes weighted
by where the vacuum lives. I believe it is the corridor a stated class of disturbances leaves through, and that it is
invariant under how the rooms are joined. More boldly, I believe it plays the same tone in every room once the field's
own scale sets the octave. Underneath all of it, I believe the note is inherited: it is the intrinsic note of one
turn, the thing parallel transport does, carried down from strong coupling to the continuum. I hold the carrier as
the centre, and the tone as the strongest and most exposed claim. I hold those beliefs as conjectures. Each milestone above is a place where the geometry gets to
tell me yes or no.
