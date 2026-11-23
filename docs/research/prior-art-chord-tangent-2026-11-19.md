# Prior art for the chord–tangent energy of *Three Ways Across a Turn*

2026-11-19. Literature search closing the open item "a prior-art search for the energy
combination of Proposition 2 and for tangent-based lattice actions in general" of *Three Ways
Across a Turn* (doi:10.5281/zenodo.24375338). Done by web search of arXiv, INSPIRE and journal
pages; every source below was located, and the ones not read in full are marked.

## In plain English

The blend itself still looks new: nobody we found proposes (2C² + T²)/3, the 2:1 mix of
Wilson's chord energy and the tangent energy, as a lattice action, writes it algebraically in U,
or gives its coefficients 7/720 and 1/1008. But three neighbouring things are published, so the
paper's sentence "the one statement we have not found in the sources above" overstated it:

1. Cancelling the θ⁴ ("compactness") term of Wilson's action is known, by a different route.
2. The tangent energy is a known lattice action, studied for a different reason.
3. The inequality behind Proposition 2 is a known inequality in analysis.

## Findings

| Source | What it contains | Relation to Proposition 2 |
| --- | --- | --- |
| de Forcrand, Hetrick, Takaishi, van der Sijs, *Three topics in the Schwinger model*, Nucl. Phys. B Proc. Suppl. 63 (1998) 679–681, arXiv:hep-lat/9709104 | U(1) action ΔS = c Σ[(1−cos θ)/3 − (1−cos 2θ)/12] added to Wilson's; with c = 1 "the θ_p⁴ term … is cancelled (perturbative improvement)", explicitly distinct from O(a²) improvement. In our notation (4/3)·C(θ)²/2 − (1/12)·C(2θ)²/2 = θ²/2 − θ⁶/180 + … | **Closest prior art.** Same purpose (cancel the compactness θ⁴ term), different second ingredient (adjoint/doubled angle instead of tangent), opposite sign of the θ⁶ remainder. Equivalent to the U(1) fundamental–adjoint (Bhanot–Creutz) choice β_A/β_F = −1/16. |
| Lüscher, *Topology and the axial anomaly in abelian lattice gauge theories*, Nucl. Phys. B 549 (1999) 295–334, arXiv:hep-lat/9811032 (cited via the papers below, not read in full); Fukaya, Hashimoto, Hirohashi, Ogawa, Onogi, *Topology conserving gauge action and the overlap-Dirac operator*, Phys. Rev. D 73 (2006) 014503, arXiv:hep-lat/0510116; Fukaya & Onogi, PRD 68 (2003) 074503, arXiv:hep-lat/0305004 | Topology-conserving action S = β Σ (1 − Re P)/(1 − (1 − Re P)/ε). For U(1) and SU(2) (Re tr/2 = cos a), at ε = 2: (1 − cos θ)/(1 − (1 − cos θ)/2) = 2 tan²(θ/2) = T²/2 (checked numerically to 1e-16). | **The tangent energy itself**, under another name, at the boundary value where the barrier sits only at U = −1. Motivated by topology, not by the θ⁴ term; never blended with Wilson's. For SU(3) the Re tr/3 form is a different function. |
| Creutz, *Positivity and topology in lattice gauge theory*, Phys. Rev. D 70 (2004) 091501, arXiv:hep-lat/0409017 | Argues that admissibility conditions are incompatible with a positive transfer matrix. | Bears on the paper's question whether the U = −1 barrier is harmless; with topology freezing in Monte Carlo (Fukaya et al.), these are the known costs of such barriers. |
| Wu & Srivastava, *A weighted and exponential generalization of Wilker's inequality and its applications*, Integral Transforms Spec. Funct. 18 (2007) 529–535 (journal page not accessible; seen as quoted by Yang); Yang, *Some sharp Wilker type inequalities and their applications*, arXiv:1304.5392 (2013), Proposition 1 | (2/3)(sin x/x)² + (1/3)(tan x/x)² > 1 on (0, π/2), with x = θ/2 the inequality (2C² + T²)/3 > θ². | **The same inequality** as pure analysis. Neither gives the expansion θ² + 7θ⁶/720 + θ⁸/1008, the reading as plaquette energies, or the form in U. |
| Crane & Wardetzky, *A glimpse into discrete differential geometry*, Notices AMS 64 (2017) 1153–1159; Carroll et al., arXiv:1311.5862; Bergou et al., *Discrete Elastic Rods*, ACM TOG 27(3) (2008) | 2 sin(θ/2), θ and 2 tan(θ/2) as "equally worthy" discrete curvatures; elastic-rod bending energy quadratic in 2 tan(φ/2). | The three-way picture is known in discrete differential geometry; no 2:1 blend of squares found. |
| Chevyrev & Garban, *Villain action in lattice gauge theory*, arXiv:2404.09928 | Manton and Wilson actions within a larger family, rigorously. | Background only. |

Not found, with queries run: a Cayley-transform or stereographic plaquette action under those
names; actions written as |Im U|²/(1 + Re U)² or tr[(1 − U)(1 + U)⁻¹]²; Padé/rational plaquette
actions; the Cayley map replacing exp in HMC link updates; the coefficients 7/45 (in x) or 7/720,
1/1008 (in θ) for the 2:1 squared blend. Not checked: Lüscher 1999 in full, the XLF papers
(hep-lat/0409073, 0412017, 0502001), an SU(N) fundamental–adjoint tuning that cancels the same
term.

A remark from the search (arithmetic only, not a claim of the paper): mixing the adjoint-improved
action and the chord–tangent blend with weights 7/15 and 8/15 cancels θ⁶ as well.

## Changes to the paper

In English and Russian (merged into `feat/reading-the-curve`):

- Section 1, "What is classical": the sentence "the one statement we have not found" is replaced
  by the three findings above and the narrower claim: what we have not found is the chord–tangent
  blend proposed as a lattice energy, its algebraic form in U, and its coefficients.
- Section 4, the remark on the U = −1 barrier: cites the topology-conserving actions and Creutz.
- Section 12, Open: the prior-art search is replaced by the comparison it opened up: how the
  blend compares in non-Abelian simulations with the adjoint-improved and Lüscher-type actions,
  and whether its barrier inherits their known costs.
- Seven bibliography entries added.
