# H4′ by borrowed routes: the sphere family, c₀, and Euclidean symmetry

*28 November 2026 (written overnight). Research note, not a register change. I don't touch `CLOSED-SET.md` or the paper here.
Status words: **closed** means every step is cited with matching hypotheses or proved here. **Conditional** names its
condition. **Open** means open.*

---

## 1. The precise target

H4′ (line 23, with line 29 merged in) as the register states it on 27 Nov:

- **(U) UV limit in every room.** For every R > 0 (and the Berger parameter t), the continuum limit exists on
  S³_{R,t} × ℝ with the H2 cutoff removed and OS regularity bounds. It tunes **two** bare parameters, the coupling g₀ and a
  speed of light c₀, because on S³ × ℝ nothing mixes τ with space, so tr F₀ᵢ² and tr Fᵢⱼ² are separately allowed marginal
  operators (COHERENCE item 4).
- **(E) The flat end.** As R → ∞, the (flowed) correlation functions converge on compact regions to an ℝ⁴ theory with full
  Euclidean O(4) ⋉ ℝ⁴ invariance.
- **(O) Observables.** A local or reflection-compatible observable algebra, dense reconstructed states, uniform bounds.

The research question I was given: which parts of (U) and (E) can I take from established work, in particular **the
existence and value of the c₀ tuning** and **perturbative existence of the flowed correlators on S³ × ℝ**?

---

## 2. What others have done (verified citations, exact statements, hypotheses)

I only quote what I could check. Where I could not read the primary text, I say so and give only what I verified.

### 2.1 Perturbative Yang–Mills on any globally hyperbolic spacetime: Hollands (the main borrowing)

**S. Hollands, "Renormalized Quantum Yang–Mills Fields in Curved Spacetime", Rev. Math. Phys. 20 (2008) 1033,
arXiv:0705.3340, DOI 10.1142/S0129055X08003420.**

- *Statement (abstract / §1):* quantum Yang–Mills theory "can be consistently defined as a renormalized, perturbative
  quantum field theory on an arbitrary globally hyperbolic curved, Lorentzian spacetime". The output is an algebra of
  gauge-invariant renormalized observables (formal power series) and a space of states, built as BRST cohomology. The
  renormalized Ward identities make the interacting BRST current conserved and the BRST charge nilpotent (§§4.3–4.8).
- *Remaining freedom (§1.1, §3.2):* finite local terms "(possibly with curvature couplings)" added to the Lagrangian, and
  finite field redefinitions ("operator mixing with curvature"). The time-ordered products are "locally and covariantly
  constructed in terms of the metric" (axiom T1). In the extension step (Lemma 6), two extensions differ "at most by a
  distribution of the form" Lδ, where L is "a Lorentz-invariant partial differential operator". Scaling (T2) limits the
  dimension, up to logarithms.
- *Hypotheses:* Lorentzian and globally hyperbolic. No isometries assumed. Gauge group: compact, simple or semisimple, with
  no abelian factors in Thm 1 (abelian factors decouple, Remark 2). The principal bundle is trivial, M × G. The background
  connection is flat. Formal power series in g; no claim about convergence or non-perturbative existence.
- *Supporting uniqueness/existence theory:* S. Hollands and R. M. Wald, "Existence of Local Covariant Time Ordered Products
  of Quantum Fields in Curved Spacetime", Commun. Math. Phys. 231 (2002) 309–345, arXiv:gr-qc/0111108 (completes their
  earlier uniqueness theorem; scalar fields).

**Check of the hypotheses on my geometry.**
- ℝ × S³_{R,t} with the Lorentzian metric −dτ² + R²h_t is static with a compact Cauchy surface, so it is globally hyperbolic,
  for every R > 0 and every t > 0. (This includes the Berger t ≠ 1 that H2 needs.)
- Principal G-bundles over ℝ × S³ ≃ S³ are classified by π₂(G) = 0, so they are trivial for every compact connected G. The
  trivial-bundle assumption is automatic. Unlike the torus, there are no flux sectors.
- G compact simple matches H7's range.

### 2.2 Lattice perturbation theory converges to continuum perturbation theory: Reisz

- **T. Reisz, "Lattice gauge theory: Renormalization to all orders in the loop expansion", Nucl. Phys. B318 (1989) 417–463,
  DOI 10.1016/0550-3213(89)90613-5.** It rests on Reisz's lattice power-counting theorem (Commun. Math. Phys. 1988): if
  lattice Feynman integrals satisfy UV power-counting conditions in Reisz's lattice degree, their continuum limit exists.
  Secondary sources (e.g. hep-lat/0606003) summarize the gauge result this way: the lattice perturbation series of SU(N)
  Yang–Mills (with Wilson fermions) can be renormalized and matched to a continuum scheme at every order.
- **M. Lüscher and P. Weisz, "Background field technique and renormalization in lattice gauge theory", Nucl. Phys. B452 (1995)
  213–233, arXiv:hep-lat/9504006.** Lattice gauge theory with a background field is renormalizable to all orders, with no
  counterterms beyond those needed without the background. The proof uses BRS, background-gauge and shift symmetries.
- *Hypotheses:* flat hypercubic lattice ℤ⁴ (momentum space, Brillouin zone), perturbation theory. I could not read Reisz's
  text, so I do not claim he treats anisotropic lattices. On a lattice with only cubic × time-reflection symmetry, his
  power-counting theorem applies to the integrals, but the counterterm bookkeeping would have to be redone with one more
  marginal operator.

### 2.3 Anisotropic lattices and the Hamiltonian limit: Karsch; Hasenfratz–Hasenfratz

- **F. Karsch, "SU(N) gauge theory couplings on asymmetric lattices", Nucl. Phys. B205 (1982) 285–300, DOI
  10.1016/0550-3213(82)90390-X.** This is the one-loop calculation of the separate spatial and temporal couplings on a
  lattice with a_s ≠ a_τ, and of the Λ parameter there relative to the symmetric lattice (as later Fermilab papers
  describe it).
- **A. Hasenfratz and P. Hasenfratz, "The scales of Euclidean and Hamiltonian lattice QCD", Nucl. Phys. B193 (1981) 210–220,
  DOI 10.1016/0550-3213(81)90526-5.** This is the one-loop relation between the Euclidean and the Hamiltonian
  (a_τ → 0) lattice scales. A textbook chapter on the Hamiltonian approach attributes the observation that "the speed of
  light may need to be renormalized" to Shigemitsu–Kogut (1981) and to this paper.
- *What I could NOT verify:* the numerical one-loop coefficients. I did not get the full texts, so I quote no numbers. I
  will not put any number for c₀⁽¹⁾ into the register until I have read them.
- *Hypotheses:* flat hypercubic, or anisotropic hypercubic, Wilson lattice; one loop; background-field method (Dashen–Gross
  style).

### 2.4 Non-perturbative tuning of the anisotropy: Klassen; Borsányi et al. (the Wilson-flow criterion)

- **T. R. Klassen, "The anisotropic Wilson gauge action", Nucl. Phys. B533 (1998) 557–575, arXiv:hep-lat/9803010.** He
  determines the renormalized anisotropy ξ(ξ₀, β) by comparing the finite-volume static potential with the quarks separated
  along a spatial direction against the one with them separated along the temporal direction. This is Euclidean symmetry
  imposed as a renormalization condition.
- **Sz. Borsányi, S. Dürr, Z. Fodor, S. D. Katz, S. Krieg, T. Kurth, S. Mages, A. Schäfer, K. K. Szabó, "Anisotropy tuning
  with the Wilson flow", arXiv:1205.0781 (prepared for JHEP; I could not confirm the journal page, so I cite the arXiv).**
  The exact statement I borrow (their eqs. 3.1–3.5): split the flowed action density into
  E_ss(τ) = ¼ Σ_{x,i≠j} F_ij(x,τ)² and E_st(τ) = ½ Σ_{x,i} F_i4(x,τ)². "In physical units the expectation values of these two
  parts are equal. Since for any τ > 0 these operators are renormalized, they offer a definition for the renormalized
  gauge anisotropy." Concretely, ξ_g² = R_E with R_E the ratio of τ d/dτ(τ²⟨E⟩) for the two parts, evaluated at τ = w₀²
  (with τ d/dτ(τ²⟨E_ss⟩) = 0.15 at τ = w₀²). The bare ξ₀ is tuned until ξ_g equals the target.
- *Hypotheses:* flat, periodic, numerical. Equality of ⟨E_ss⟩ and ⟨E_st⟩ uses the O(4) symmetry of the target theory and
  its translation-invariant vacuum.

### 2.5 Gradient flow: finiteness, renormalizability, stress tensor

- **M. Lüscher, "Properties and uses of the Wilson flow in lattice QCD", JHEP 1008 (2010) 071, arXiv:1006.4518.**
- **M. Lüscher and P. Weisz, "Perturbative analysis of the gradient flow in non-abelian gauge theories", JHEP 1102 (2011)
  051, arXiv:1101.0963.** "For any matter multiplet and to all loop orders, we show that the correlation functions are
  finite", with no renormalization beyond the usual four-dimensional one. *Hypotheses:* flat ℝ⁴. The proof writes the flow
  as a local field theory on ℝ⁴ × [0, ∞).
- **K. Hieda, H. Makino, H. Suzuki, "Proof of the renormalizability of the gradient flow", Nucl. Phys. B918 (2017),
  arXiv:1604.06200, DOI 10.1016/j.nuclphysb.2017.02.017.** An alternative perturbative proof for vector-like gauge
  theories. Flat space.
- **H. Suzuki, "Energy–momentum tensor from the Yang–Mills gradient flow", arXiv:1304.0533, PTEP (DOI
  10.1093/ptep/ptv094 for the erratum/version).** A small-flow-time formula for the correctly normalized conserved EMT.
  Perturbative, flat.

### 2.6 Constructive and probabilistic templates

- **T. Bałaban, "Renormalization group approach to lattice gauge field theories. I. Generation of effective actions in a
  small field approximation and a coupling constant renormalization in four dimensions", Commun. Math. Phys. 109 (1987)
  249–301; II. "Cluster expansions", CMP 116 (1988) 1–22** (the later parts of the series are as on line 21; I re-verified I
  and II only). Periodic finite box, isotropic hypercubic lattice, UV stability (partition function and effective actions).
- **J. Magnen, V. Rivasseau, R. Sénéor, "Construction of YM₄ with an infrared cutoff", Commun. Math. Phys. 155 (1993) 325,
  DOI 10.1007/BF02097397.** SU(2) Schwinger functions, UV cutoff removed, fixed IR cutoff, regularized axial gauge,
  Slavnov identities checked non-perturbatively.
- **S. Cao and S. Chatterjee, "A state space for 3D Euclidean Yang–Mills theories", arXiv:2111.12813 (to appear in CMP).**
  Random distributional gauge orbits on the 3D unit torus, defined through Yang–Mills heat flow. It assumes the theory is
  "GFF-like".
- **A. Chandra, I. Chevyrev, M. Hairer, H. Shen, "Stochastic quantisation of Yang–Mills–Higgs in 3D", Invent. Math. 237
  (2024) 541–696, arXiv:2201.03487.** A nonlinear metric space of distributions; a unique choice of counterterms makes the
  renormalized dynamics gauge covariant in law; a Markov process on gauge orbits (up to possible blow-up). 3D torus,
  regularity structures (a subcritical setting).
- **Z. Davoudi and M. J. Savage, "Restoration of rotational symmetry in the continuum limit of lattice field theories",
  Phys. Rev. D 86 (2012) 054505, arXiv:1204.4146.** At tree level and one loop, rotation-violating contributions of smeared
  operators are O(a²) (φ⁴ and QCD, with smeared gauge fields). Perturbative, flat, hypercubic.

### 2.7 The conformal map and the trace anomaly

- Classical fact (checked by hand): with r = R e^{τ/R}, dr² + r² dΩ₃² = e^{2τ/R}(dτ² + R² dΩ₃²). So round S³_R × ℝ is
  Weyl-equivalent to ℝ⁴∖{0}, with Weyl factor e^{τ/R}. The equivalence fails for Berger t ≠ 1.
- **J. C. Collins, A. Duncan, S. D. Joglekar, "Trace and dilatation anomalies in gauge theories", Phys. Rev. D 16 (1977)
  438–449, DOI 10.1103/PhysRevD.16.438.** The operator trace anomaly θ^μ_μ ∝ (β(g)/g) tr F² (the paper's own normalization),
  derived for gauge theories with fermions.

---

## 3. Alignment with my framing (honest)

| Borrowed piece | Aligns? | Why, and what the transfer needs |
|---|---|---|
| Hollands 2008 | **Yes, strongly, for (U) at the level of counterterms.** | It is built on exactly my kind of background: curved, no isometries assumed, trivial bundle. It says the renormalized theory is locally covariant: every finite ambiguity is a local covariant term built from the metric. *Transfer needs:* (i) a Wick rotation, because Hollands is Lorentzian and H4′ is Euclidean (my spacetime is static, so this is the standard ground-state step, but it is not in Hollands); (ii) a ground state of the interacting perturbative theory on ℝ × S³ (Hollands builds states by deformation; I did not verify a perturbative interacting ground state on static spacetimes); (iii) it is perturbative about A = 0. My slow room is non-perturbative at large R. |
| Reisz; Lüscher–Weisz 1995 | Partly. | Perturbative O(4) restoration on the flat isotropic lattice, as a template for the R → ∞ end. It does not cover a cell complex of S³ (no momentum space) or an anisotropic lattice without redoing the counterterm bookkeeping. |
| Karsch; Hasenfratz–Hasenfratz | Yes, as the **flat values** of c₀. | My H2 regulator (spatial cutoff, continuous time) is the sphere analogue of the Hamiltonian limit. By locality (§4) the UV counterterm c₀(g₀) is the flat one for the same local regulator. A spectral cutoff on S³ is not a lattice, so H–H's number is not my number. Only the mechanism transfers. |
| Klassen; Borsányi et al. | **Yes, very well.** | The flow criterion is exactly my observable class (flowed densities). It turns "Euclidean symmetry" into a renormalization condition fixing c₀, which is how I should read c₀. On S³_R × ℝ the equality ⟨E_ss⟩ = ⟨E_st⟩ is *not* exact (curvature and the Casimir energy split them at O(τ/R²) and O(1/R⁴)). So I must apply it in the flat theory or at τ ≪ R², and carry c₀ to every R by locality. |
| Lüscher–Weisz 2011; Hieda–Makino–Suzuki | Template only. | Flat ℝ⁴ × [0,∞). Flowed correlators on S³ × ℝ need the same 5D-local proof with the flow of the metric R²h_t + dτ². I found no curved-space version. |
| Bałaban; MRS | Template (as on line 21). | Torus and continuum momentum cutoff. Nothing new here. |
| Cao–Chatterjee; CCHS | **Alignment of idea, not of result.** | They define the state space through the YM heat flow, which matches my flowed observables. But they work in 3D, which is subcritical (CCHS) or GFF-like (CC). The time-zero field of a 4D theory has covariance ~(−Δ₃)^{−1/2}, rougher than the 3D GFF (−Δ₃)^{−1}. So even as a state space for my *spatial slices* S³, their GFF-like hypothesis fails by half a derivative. A clear negative for direct use. |
| Davoudi–Savage | Weak. | One-loop O(a²) rotation breaking for smeared operators. Supporting evidence, not proof. |
| Conformal map + CDJ anomaly | **Mostly does not align** with the paper's §8 idea. | See below. |

**On the conformal-map conjecture in the paper (§8, "An idea for the time–space residue").** The borrowed pieces point the
other way from what I conjectured, and I want to record that honestly. Hollands' theorem says that in a locally covariant
scheme there is **no** c₀ at all. The ambiguity in each extension is a Lorentz-invariant differential operator Lδ, so a term
that separates tr F₀ᵢ² from tr Fᵢⱼ² (which needs the static Killing field ∂_τ) is never generated. So c₀ is a property of a
regulator that breaks local covariance, such as a spatial cutoff or a cell complex × εℤ. It is not a property of the
geometry, and the trace anomaly does not control it. What the anomaly does control is the *R-dependence of the coupling*
under the Weyl map. The map sends S³_R × ℝ to ℝ⁴∖{0} with a position-dependent effective coupling g(r). That is the running
I already use to set g(R), not a mechanism for O(4). The conjecture "c₀ is controlled entirely by the anomaly" should be
replaced by "c₀ is regulator data fixed by local covariance" (see §5). This is a correction to a labelled conjecture, which
my living-drafts rule allows.

---

## 4. A reduced route for H4′ using the borrowed pieces

I split H4′ into seven pieces and say for each what is borrowed and what is left.

**(U1) Counterterm structure on S³_{R,t} × ℝ.** *Closed in perturbation theory (§5).* In any locally covariant scheme the
only marginal freedom is the coupling, plus the θ-term, which Θ/CP excludes, plus curvature c-numbers. There is no c₀.

**(U2) c₀ for my actual regulators.** My regulators (the H2 spatial spectral cutoff with continuous time; a cell complex ×
εℤ) break local covariance down to the static symmetry. I borrow the *structure* from the flat anisotropic and Hamiltonian
lattices (Karsch; Hasenfratz–Hasenfratz): one extra finite renormalization c₀ = 1 + O(g₀²). *Conditional* on a
Reisz-type statement for my regulator: its renormalized perturbation series, after tuning (g₀, c₀), satisfies Hollands'
axioms. Given that condition, uniqueness up to covariant terms forces c₀ order by order, and every coefficient is finite.
The divergent parts are covariant and are absorbed by g₀ alone.

**(U3) Locality: one c₀ for every room.** Perturbatively, the counterterms of a local regulator are local functionals. So
c₀(g₀; regulator) is computed in the UV and does **not** depend on R or t. Curvature corrections to a dimensionless
coefficient carry (ε²·Riem) → 0. This fits my bundle picture: c₀ lives at the UV end of the fibre and is constant along the
base coordinate R. *Consequence:* tune c₀ once, in the flat theory, by the Borsányi et al. flow criterion (or Klassen's
potential criterion), and carry the same bare function to every sphere. Perturbatively this is a deduction from locality.
Non-perturbatively it is a hypothesis: UV renormalization of the sphere theory is local, which is what a Bałaban-type
sphere RG would show.

**(U4) UV stability on the sphere.** Open. The template is Bałaban, and H5 Step 3 carries the curvature estimate
(L^kε/R)². Nothing new is borrowed here.

**(U5) Flowed correlators on S³ × ℝ.** Open, with a clear path: redo Lüscher–Weisz's 5D-local argument with the flow of the
metric R²h_t + dτ², in Hollands' covariant framework. The bulk 5D theory is local and covariant, and the flow-time boundary
needs no new counterterms in flat space.

**(E1) Euclidean symmetry at R → ∞.** With (U1)–(U3), Euclidean symmetry at the flat end no longer needs a separate
"restoration" theorem. The sphere family breaks O(4) in exactly two ways: (a) the regulator's anisotropy, which is removed
once c₀ is tuned by a criterion that is itself the flat Euclidean condition (Borsányi et al.); and (b) curvature and global
closing, which give O(τ/R²) and e^{−mρR} corrections in H5 Step 6 and vanish as R → ∞. So (E) reduces to H5(b), the local
flat limit with the spatial clustering sub-conjecture [SC], plus the non-perturbative version of (U3). This is a genuine
reduction: the "time–space residue" of the 27 Nov merge becomes a UV-local tuning problem, and UV-local problems are
exactly where perturbation theory is reliable, by asymptotic freedom.

**(E2) Perturbative flat end.** Conditional. At the algebra level, local covariance plus smooth dependence on the metric gives
convergence of local covariant products as the metric on a fixed ball tends to flat. The vacuum state on S³_R converging to
the Minkowski vacuum is extra (it is the IR problem again) and is not borrowed.

**(O)** Not touched by these borrowings. The state-space idea of Cao–Chatterjee and CCHS aligns in spirit, but does not apply
(the regularity mismatch above).

---

## 5. The most closable sub-statement now

**Statement C₀ (no intrinsic speed of light; perturbative, all orders).**
Let G be compact and simple, t > 0, R > 0. On the Lorentzian static spacetime M_{R,t} = (ℝ × S³, −dτ² + R²h_t),
renormalized perturbative Yang–Mills theory exists to all orders in g. Any two locally covariant renormalization
prescriptions differ by finite local covariant counterterms built from the metric, the curvature and the fields. Among the
dimension-4 gauge-invariant terms, those are exactly tr F_{μν}F^{μν} (a finite renormalization of g), tr F∧F (θ; excluded
when the prescription respects time reflection Θ / CP), and curvature-only c-numbers. In particular, **no counterterm
separates tr F₀ᵢ² from tr Fᵢⱼ²**. The speed-of-light parameter c₀ of H4′ is therefore absent in every locally covariant
scheme. It is required only by regulators that break local covariance, and for those it is a finite renormalization
fixed by matching to the covariant theory.

**Borrowed:** Hollands 2008 (existence on any globally hyperbolic spacetime; locality and covariance T1; scaling T2;
Lemma 6: extension ambiguity = Lorentz-invariant differential operator times δ; gauge group compact semisimple; trivial
bundle). Hollands–Wald 2002 for the underlying local covariant time-ordered products.

**Deduction.**
1. *Hypotheses hold.* M_{R,t} is static with compact Cauchy surface S³, so it is globally hyperbolic. G-bundles over
   ℝ × S³ are trivial (π₂(G) = 0), so Hollands' M × G assumption holds with no loss. G is compact simple. So Hollands'
   theorem gives the renormalized perturbative theory on M_{R,t}, for every (R, t).
2. *Form of the ambiguity.* By Hollands (T1, Lemma 6), each new ambiguity at each order is Lδ, with L a local
   differential operator built covariantly from the metric (Lorentz invariant in the tangent space) and with polynomial
   coefficients in the curvature and its covariant derivatives. By T2 and pure YM's lack of a mass, the coefficients are
   dimensionless up to logarithms. So the counterterms in the action are local covariant scalars of dimension ≤ 4.
3. *Enumeration.* Gauge-invariant local covariant scalars of dimension ≤ 4 in pure YM are 1, tr F_{μν}F^{μν},
   tr F_{μν}F̃^{μν}, and curvature monomials (Riem², Ric², Scal², □Scal, Scal·1). A product (curvature, dimension 2) ×
   (gauge-invariant field monomial of dimension 2) does not exist, because pure YM has no gauge-invariant local scalar of
   dimension 2. BRST-exact terms are trivial in the cohomology Hollands uses.
4. *No anisotropy.* The term tr F₀ᵢF₀ᵢ − c·tr FᵢⱼFᵢⱼ with c ≠ the covariant ratio can only be written covariantly as
   tr(F_{μρ}F_ν{}^ρ)u^μu^ν with a unit timelike vector field u = ∂_τ. Such a u is not built from the metric locally (it is
   a global Killing field of the static metric, not a local covariant functional; local covariance requires naturality
   under all isometric embeddings, and these do not preserve u). So it is not in the list of step 3.
5. *Hence:* c₀ ≡ 1 in every locally covariant scheme, for every R and t. For a regulator that breaks covariance, the
   statement "after tuning (g₀, c₀) the renormalized theory is the covariant one" fixes c₀ uniquely order by order, by the
   uniqueness of step 2 applied to the difference. That last clause is **conditional** on the regulator's renormalized
   series being shown to satisfy Hollands' axioms (a Reisz-type theorem for my regulator), which is not done.

**What C₀ does and does not close.**
- *Closes (by citation + deduction):* the existence and structure of the speed-of-light question at the perturbative level.
  c₀ is regulator data, not a parameter of the theory on S³ × ℝ, and it does not depend on R or t. This also corrects the
  conformal-anomaly conjecture in §8 of the paper.
- *Does not close:* the one-loop numerical value of c₀ for my H2 spatial spectral cutoff. That is a finite one-loop
  computation on the flat space (by locality), the analogue of Hasenfratz–Hasenfratz for a gauge-covariant spatial
  heat-kernel cutoff. I list it as the next small computation, not heavy, but I have not done it tonight.
- *Does not close:* the Wick rotation and Euclidean ground state on ℝ × S³ at the interacting perturbative level (I did not
  verify a citation). It does not close anything non-perturbative.

**Hollands' status and mine.** C₀ is closed only in perturbation theory. Its value to the programme is that it shrinks the
"time–space residue" named at the 27 Nov merge to two things: (i) a flat, UV-local tuning (borrowable from Karsch,
Hasenfratz–Hasenfratz, Klassen and Borsányi et al.); and (ii) the non-perturbative locality of UV counterterms on the sphere,
which is part of what a Bałaban-type sphere RG (U4) must deliver anyway. It is not a separate symmetry-restoration problem.

---

## Next small steps (light; none needs heavy compute)
1. Read the Karsch and Hasenfratz–Hasenfratz texts for the one-loop coefficients and the precise definitions of
   g_σ, g_τ. Then write c₀⁽¹⁾ for the flat Hamiltonian Wilson lattice with a verified number.
2. The one-loop c₀⁽¹⁾ for a gauge-covariant spatial heat-kernel cutoff on flat ℝ³ × ℝ (the flat limit of the H2 regulator):
   one background-field diagram, two orientations of the background field strength.
3. Look for a curved-space version of Lüscher–Weisz flow finiteness (U5). I found none tonight.
4. Find (or rule out) a published perturbative interacting ground state on static globally hyperbolic spacetimes, for the
   Wick-rotation step.
