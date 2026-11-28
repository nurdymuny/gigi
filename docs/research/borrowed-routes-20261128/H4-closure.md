# H4′, sub-statement C₀: no intrinsic speed of light (perturbative, all orders): the proof note

*28 November 2026 (overnight). Proof note, not a register change. I don't touch `CLOSED-SET.md` or the paper. It
deduces sub-statement C₀ of `H4-borrowed-routes.md` §5 from Hollands (2008). While writing it I found one step of
that route that was **wrong as stated** (step 4, "u is not built from the metric"). §4 gives the correct argument, so
the route works and the conclusion holds. Status words are as in the register: [PC] proved and cited, [D] closed by
deduction, [Cd] conditional.*

Revised 28 Nov after the referee pass: clause (d)/Step 11 restates (HR) as matching one locally covariant prescription on each M_{R,t}, drops the invalid "by Step 10 it does not depend on (R, t)", and makes R/t-independence and constant coefficients an explicit extra regulator-locality hypothesis; (b)/Step 9 say "BV canonical transformation" instead of "field redefinition" and drop "X linear in antifields"; Step 6 no longer credits representatives (63) with tr F∧F; Step 4 credits T2 on both prescriptions (the Hadamard scale is a dimensionful constant); Step 8 says the 1/Scal term is excluded by smoothness across the category; (c) restated as "the marginal freedom is the single coupling g"; §5 no longer claims to be the content of line 14; the Theorem-2-for-YM caveat moved up to the outcome; §8 suggested text says Lorentzian, Hollands' gauge-fixed framework, and "in perturbation theory". No status upgraded.

**Outcome.** C₀ is closed **in renormalized perturbation theory (formal power series in g), on the Lorentzian static
spacetimes ℝ × S³_{R,t}, for locally covariant prescriptions in Hollands' sense**: [PC + D], *modulo the vector/ghost
analogue of Hollands–Wald uniqueness (Theorem 2), which Hollands asserts by analogy (§3.5, §4.3) and uses himself in
§4.4, but does not prove separately.* The clause about
non-covariant regulators (H2's spatial cutoff) stays [Cd]. Nothing non-perturbative and nothing Euclidean is proved
here (§7).

---

## 1. Precise statement

**Setting.**
- G is a compact, connected, simple Lie group, with Lie algebra 𝔤 and its Killing form.
- For R > 0 and t > 0, h_t is the Berger metric on S³ (round at t = 1) and
  M_{R,t} = (ℝ × S³, g_{R,t} = −dτ² + R² h_t), with the orientation of S³ and the time orientation ∂_τ.
- The static vector field u = ∂_τ is the unit normal of the slices τ = const.
- A **locally covariant prescription** is an assignment of time-ordered products T = {T_n} to every oriented,
  time-oriented, globally hyperbolic 4-dimensional spacetime. For the Yang–Mills auxiliary theory (fields A, C, C̄, B,
  antifields, Lorentz gauge, trivial bundle, flat background connection) it satisfies Hollands' axioms T1–T11 and the
  Ward identities T12a–c (Hollands 2008, §§3.2, 4.3).
- **The anisotropy class.** On M_{R,t} the 4-form
  𝒜 := u^μ u^ν tr(F_{μρ} F_ν{}^ρ) vol_g
  is gauge invariant. Adding it to (1/2g²) tr F∧⋆F changes the relative weight of tr F₀ᵢ² and tr Fᵢⱼ². This is the
  speed-of-light parameter c₀ of register line 23 (H4′) and COHERENCE item 4. I say **"a prescription change carries
  a c₀"** when its action counterterm, modulo ŝ-exact and d-exact local forms, has a nonzero 𝒜-component that is not
  part of a multiple of tr F∧⋆F. In other words, its class is not in span{[tr F∧⋆F], c-numbers}.

**Theorem C₀.** For every compact simple G, every R > 0 and every t > 0:

(a) *(Existence.)* Renormalized perturbative Yang–Mills theory exists on M_{R,t} to all orders in the coupling. It is
an algebra of gauge-invariant interacting fields (formal power series) built as the cohomology of a nilpotent
interacting BRST charge.

(b) *(Form of the freedom.)* Let T and T̂ be two locally covariant prescriptions. The first order at which they
differ is ħ^m with m > 0. In any region where the interaction cutoff equals 1, the change of the interaction at that
order is

  D_m = a · tr F∧⋆F + κ(Riem, ∇Riem) · 𝟙 + ŝX + dY,

where a ∈ ℝ is a constant, κ is a real linear combination of the dimension-4 curvature scalars (a c-number), and
X, Y are local covariant forms. X is built from g, ε and the fields alone, with no curvature factor. The constants
(a, κ's coefficients) are the same for every (R, t) and for Minkowski space. Iterating order by order, T̂ is T
followed by a finite renormalization of g, a BV canonical transformation (field redefinition and/or change of gauge
fermion) and a c-number shift.

(c) *(No c₀.)* Consequently no prescription change carries a c₀, at any order. Put plainly: the marginal freedom of
locally covariant schemes on Lorentzian ℝ × S³_{R,t} is the single coupling g (plus ŝ-exact changes and c-numbers);
there is no second, anisotropic marginal parameter. (In that sense every locally covariant renormalized theory on
M_{R,t} has c₀ ≡ 1, for every R and t; a covariant scheme has no bare c₀ to compare.)

(d) *(Non-covariant regulators: conditional.)* Suppose a regulator that breaks local covariance comes with bare
parameters (g₀, c₀). If, after tuning, its renormalized perturbation series on each M_{R,t} equals the restriction of
a single locally covariant prescription T* (a Reisz-type theorem for that regulator), then (b) fixes c₀ order by
order, separately on each (R, t), by matching. **R- and t-independence of the bare c₀ does not follow**: the bare
c₀(ε; R, t) is not a natural transformation, and nothing cited forbids dependence on R (through ε²/R² heat-kernel
terms) or on ε. It needs an extra regulator-locality hypothesis, for example that the regulator's local UV structure
is the flat one up to O(ε²·Riem) corrections that vanish as ε → 0 order by order. A formal series
c₀ = 1 + Σ_k c₀^{(k)} g₀^{2k} with constant coefficients is likewise part of that hypothesis. **This clause is
conditional on the Reisz-type theorem and, for R/t-independence, on regulator locality; neither is proved.**

---

## 2. Borrowed results (verified; quotes are from arXiv:0705.3340v4, page/equation numbers of that version)

**[H] S. Hollands, "Renormalized Quantum Yang-Mills Fields in Curved Spacetime", Rev. Math. Phys. 20 (2008)
1033–1172, arXiv:0705.3340, DOI 10.1142/S0129055X08003420.** I checked the metadata on arXiv and read the v4
full text.

- **[H-abs]** Abstract: "We present a proof that quantum Yang-Mills theory can be consistently defined as a
  renormalized, perturbative quantum field theory on an arbitrary globally hyperbolic curved, Lorentzian
  spacetime." The algebra "is constructed as the cohomology of an auxiliary algebra describing a gauge fixed theory
  with ghosts and anti-fields."
- **[H-bundle]** §2.2: "For the sake of simplicity, we will assume that the principal bundle is toplogically trivial,
  i.e., of the form M × G." Also "the background gauge connection ∇̄ has been chosen as the standard flat
  connection in the bundle M × G".
- **[H-Cauchy]** §4.1.2: "For simplicity, we again assume that M has compact Cauchy-surfaces". The non-compact case
  is said to go "by complete analogy". *My spacetime has compact Cauchy surface S³, so I use exactly the case written
  out.*
- **[H-T1]** Axiom T1, Locality and covariance: "if ψ : M → M′ is a causality preserving isometric embedding … then
  α_ψ ∘ T_n = T′_n ∘ ⊗ψ_*". The category is "oriented and time-oriented spacetimes" with "orientation and causality
  preserving isometric embedding[s]" (§3.2).
- **[H-T2]** Axiom T2, Scaling: homogeneous scaling under g ↦ μ²g "up to logarithms", with N_d the dimension
  counter for couplings, fields and curvature terms.
- **[H-T4/T5]** Smooth and analytic dependence on the metric.
- **[H-T11]** Action Ward identity: T_n(… ⊗ d_k O(x_k) ⊗ …) = d_k T_n(…), "derivatives can be freely pulled inside the
  time-ordered products".
- **[H-§3.6]** Renormalization ambiguities. The prescriptions are related by T̂(e_⊗^{iF/ħ}) = T(e_⊗^{i[F+D(exp_⊗F)]/ħ})
  (eq. 254). The D_n are "supported on the total diagonal" and "local and covariant", with
  ψ^* ∘ D′_n = D_n ∘ (ψ^* ⊗ … ⊗ ψ^*). Then: "It follows from the smoothness and analyticity properties T4, T5 and
  the scaling property T2 that the D_n depend only polynomially on the Riemann curvature tensor, the mass parameter
  m², and the field". The dimension constraint (257) is
  (N_d + sd) D_n(⊗O_i) = Σ_i D_n(… N_d O_i …), with sd the scaling degree.
- **[H-Thm2]** Theorem 2 ("main theorem of renormalization theory", quoting Hollands–Wald [HW1, HW2]): prescriptions
  satisfying T1–T11 exist, and any two differ by such a hierarchy D (eq. 263). Conversely, every such D gives a
  prescription. *Hypothesis caveat:* Hollands proves Theorem 2 for the scalar field. For the vector and ghost fields
  of §3.5 he says the construction "is completely analogous to the scalar case", and in §4.3 that the YM time-ordered
  products "will satisfy the analog of conditions T1–T11". I take the YM version of Theorem 2 from those sentences.
- **[H-L4]** Lemma 4 (Thomas replacement). A local covariant functional can be written in terms of
  g, R, …, ∇^{p−2}R and the fields and their covariant derivatives.
- **[H-L1]** Lemma 1. If sO = 0, then up to sO′, O is a combination of
  ∏ r_t(g, R, ∇R, …) ∏ p_r(C) ∏ Θ_{r_j}(F, DF, …, D^l F), with p_r, Θ "invariant polynomials of the Lie-algebra of G".
- **[H-Thm1]** Theorem 1 (from Barnich–Brandt–Henneaux [BBH]). "Let the Lie-group G be a simple or semi-simple
  compact Lie group with no abelian factors". Then every class in H(ŝ|d, P^n) is a combination of the forms of
  Lemma 1 and of the representatives (63) (curvature closed forms × Chern–Simons forms × invariant monomials of F).
  Remark 1 after it: the r_t are "analytic functions of the curvature tensor and its derivatives" and "may be chosen
  to be characteristic classes".
- **[H-L11]** Lemma 11. Suppose D and the anomaly A both start at order ħ^m. Then
  Â_m(e_⊗^F) = A_m(e_⊗^F) + (S₀ + F, D_m(e_⊗^F)) + ½ D_m((S₀+F, S₀+F) ⊗ e_⊗^F).
- **[H-Rem3]** The remark after Theorem 3 (§4.1.3): the YM Lagrangian "is the only gauge invariant field at ghost
  number 0 of this dimension", so it "does not mix with other field up to Q_I-exact terms under the
  renormalization group flow". The flow of λ "automatically must be exactly the same as in Minkowski spacetime!"
  *This is a remark with a short argument. It is not a numbered theorem.*
- **[H-L6]** Lemma 6, the extension step in the tangent space: "Two different extensions can differ at most by a
  distribution of the form Lδ, where L is a Lorentz-invariant partial differential operator … containing derivatives
  of degree ρ − 4n." *Note:* this is a statement about distributions on ℝ^{4n} (Riemann-normal-coordinate scaling
  expansion), so it is an ingredient of the construction. The global form of the freedom is [H-§3.6]/[H-Thm2], and
  that is what I use.

**[HW1] S. Hollands and R. M. Wald, "Local Wick Polynomials and Time Ordered Products of Quantum Fields in Curved
Spacetime", Commun. Math. Phys. 223 (2001) 289–326, arXiv:gr-qc/0103074, DOI 10.1007/s002200100540.** This is the
uniqueness half of Theorem 2.

**[HW2] S. Hollands and R. M. Wald, "Existence of Local Covariant Time Ordered Products of Quantum Fields in Curved
Spacetime", Commun. Math. Phys. 231 (2002) 309–345, arXiv:gr-qc/0111108, DOI 10.1007/s00220-002-0719-y.** This is the
existence half of Theorem 2.

**[BBH] G. Barnich, F. Brandt and M. Henneaux, "Local BRST cohomology in gauge theories", Phys. Rept. 338 (2000)
439–569, arXiv:hep-th/0002245, DOI 10.1016/S0370-1573(00)00049-1.** This is the source of [H-Thm1].

Hollands' field dimensions (§2.2 table): A: 1, C: 0, C̄: 2, B: 2. Antifields: A‡: 3, C‡: 4, C̄‡: 2, B‡: 2. Ghost
numbers: A 0, C +1, C̄ −1, B 0. Antifields: A‡ −1, C‡ −2, C̄‡ 0, B‡ −1. The coupling λ is dimensionless, and pure YM has
no mass parameter.

---

## 3. Proof of (a): Hollands' hypotheses hold on every M_{R,t}

**Step 1 (global hyperbolicity, compact Cauchy surface) [P].** Fix R, t. Let γ be a future-inextendible causal curve,
γ(s) = (τ(s), x(s)). Causality gives τ′² ≥ R² h_t(x′, x′) and τ′ ≠ 0 wherever γ′ ≠ 0. Since γ is future-directed,
τ is strictly increasing, so I may reparametrize γ by τ. Suppose τ were bounded above by τ₊ along γ. The h_t-length of
x on [τ₀, τ₊) is at most (τ₊ − τ₀)/R < ∞. Since (S³, h_t) is compact and hence complete, x(τ) has a limit as τ → τ₊.
Then γ would extend to (τ₊, lim x), contradicting inextendibility. The same holds to the past. So every inextendible
causal curve meets each slice {τ = c} × S³ exactly once, and that slice is a compact Cauchy surface. M_{R,t} is
oriented and time-oriented. ∎

**Step 2 (the bundle is trivial) [PC, standard topology].** Principal G-bundles over a space homotopy equivalent to S³
are classified by [S³, BG] = π₃(BG) = π₂(G). (BG is simply connected because G is connected, so free and based classes
agree.) π₂ of every Lie group vanishes. So every G-bundle on ℝ × S³ is isomorphic to M × G, which is [H-bundle].
There are no flux sectors and no loss of generality. Expanding about the flat connection is Hollands' choice. In the
reduced model of my programme it is the point q = 0. ∎

**Step 3 (apply Hollands) [PC].** G is compact simple, so it has no abelian factors ([H-Thm1]). The spacetime is
globally hyperbolic with compact Cauchy surface ([H-Cauchy]) and the bundle is trivial with a flat background
connection ([H-bundle]). So [H-abs] applies: Hollands constructs prescriptions satisfying T1–T12, the interacting
BRST charge Q_I with Q_I² = 0 (§4.7), and the algebra of physical interacting fields as its cohomology. This holds on
every M_{R,t}, R > 0, t > 0, including the Berger t ≠ 1 that H2 needs. Hollands assumes no isometries. ∎ (a)

---

## 4. Proof of (b) and (c)

**Step 4 (the change is a local covariant polynomial of dimension 4) [PC].** Let T, T̂ be locally covariant
prescriptions. By [H-Thm2] (YM version, see the caveat in §2) they are related by a hierarchy D. By [H-§3.6] it is
supported on the diagonal and natural under all orientation- and causality-preserving isometric embeddings. It is
polynomial in the Riemann tensor (and, by [H-L4], in its covariant derivatives) and in the fields, and it obeys the
dimension constraint (257).

The interaction is F = ∫(f λL₁ + f²λ²L₂), with L₁, L₂ of dimension 4 and λ of dimension 0. Write
D_n(L^{⊗n}) = Σ_α c_α O_α ∂^α δ, with sd(∂^α δ) = 4(n − 1) + |α|. Then (257) gives N_d(c_α O_α) = 4 − |α|. After
smearing with f and integrating by parts, the change of the interaction, D(exp_⊗F), restricted to the region where
f = 1, is the integral of a local covariant 4-form of **dimension exactly 4**. Here curvature counts 2, each ∇ counts
1, and the fields count as in the table. The construction does contain one dimensionful constant, the length scale of
the Hadamard parametrix (which is why T2 holds only "up to logarithms"). What forbids terms such as
ℓ²·Ric^{μν} tr F_{μρ}F_ν{}^ρ is T2 applied to *both* T and T̂: their difference D is polynomial in curvature, so it
cannot carry logarithms, so it scales exactly, which is (257). Pure YM has no m² and λ is dimensionless, so no other
constant enters. So no term of dimension ≠ 4 occurs.

**Step 5 (the leading change is a cocycle) [D from H-L11].** Both prescriptions satisfy the Ward identities, so
their anomalies vanish, A = Â = 0. Let ħ^m be the first order where D ≠ 0. On the region where f = 1, S₀ + F agrees
with the full auxiliary action S of eq. (57), which satisfies (S, S) = 0. [H-L11] then gives
(S, D_m(e_⊗^F)) = 0 there. At the level of densities, ŝ D_m = d(…), so the density of D_m is a cocycle of
H⁰(ŝ|d, P⁴), with ghost number 0 and dimension 4. Locality makes the natural transformation D_m determined by its
densities on such regions in all spacetimes. ∎

**Step 6 (enumerating the cocycles) [PC from H-Thm1, H-L1 + D].** By [H-Thm1] (G compact simple), modulo ŝ-exact and
d-exact local forms the class of D_m is a combination of:
- forms ∏ r_t ∏ p_r(C) ∏ Θ_j(F, DF, …) ([H-L1]). At ghost number 0 there are no p_r(C) factors. The Θ_j are invariant
  polynomials on 𝔤. A **linear** ad-invariant polynomial on a semisimple 𝔤 vanishes, because ℓ([X,Y]) = 0 and
  [𝔤,𝔤] = 𝔤. So every nonconstant Θ is at least quadratic in F and its D-derivatives, and has dimension ≥ 4. With total
  dimension 4 (Step 4), either Θ is quadratic in undifferentiated F and r has dimension 0, or Θ = 1 and r has
  dimension 4. The dimension-0 local covariant tensors are built from g and ε alone ([H-L4]: everything else involves
  R or ∇R, of dimension ≥ 2). The only 4-forms quadratic in F built with g and ε are tr F∧⋆F and tr F∧F, by the first
  fundamental theorem for SO(1,3): its invariant tensors are generated by g and ε. For G simple there is one
  invariant quadratic form, so one trace. The r of dimension 4 are the curvature scalars Riem², Ric², Scal², □Scal
  and the Pontryagin and Euler densities (c-numbers).
- the representatives (63). They contribute **nothing** at ghost number 0 in form degree 4: the total degree (form +
  ghost) of q_r is 2m(r) − 1 ≥ 3, and any extra factor r_t or f_s raises it by at least 2, so no product reaches total
  degree exactly 4. (tr F∧F comes from [H-L1] with Θ = tr F∧F, already listed above, not from (63).)
- tr F∧F = d CS(A) with CS(A) a local form, so its class modulo d is **zero**. (With the action Ward identity [H-T11]
  it drops out of the local interacting theory where f = 1. It is in any case the E·B density, not an E²/B² split, so
  it is irrelevant to c₀ whether or not Θ/CP is imposed.)

So **[D_m] ∈ span{[tr F∧⋆F], [dimension-4 curvature c-numbers]}**. ∎

**Step 7 (the ŝ-exact part has no curvature factor) [D].** Write D_m = a tr F∧⋆F + κ𝟙 + ŝX + dY. Here X is a local
covariant 4-form of ghost number −1 and dimension 4, ad-invariant in the colour indices. A term of X containing a
curvature factor (dimension ≥ 2) would have field content of ghost number −1 and dimension ≤ 2. From the table the
only candidates are a single C̄^I or a single B‡_I (dimension 2). A single adjoint-valued field cannot be contracted
to a colour singlet, since 𝔤 has no invariant vector. So X is built from g, ε and the fields only. (I can choose X
this way. Any curvature-dependent piece would have to be ŝ-closed on its own and can be moved into the class.) ∎

**Step 8 (where the static direction could hide, and why it cannot) [D].** *This corrects step 4 of the route in
`H4-borrowed-routes.md` §5.* That step said that u = ∂_τ "is not built from the metric locally". **On M_{R,t} that is
false.** The product metric has Ric(∂_τ, ·) = 0. On the round sphere, Ric = (2/R²)(g + u♭⊗u♭) and Scal = 6/R², so

  u_μ u_ν = (3/Scal) Ric_{μν} − g_{μν},

which is a local expression in the curvature. Then 𝒜 = [(3/Scal) Ric^{μν} − g^{μν}] tr F_{μρ}F_ν{}^ρ vol. Hence the
anisotropy class *is* locally constructible from the metric on this geometry. What excludes it is the correct
pair of facts from [H-§3.6]:
1. **Polynomiality.** D depends polynomially on the curvature. The factor 1/Scal is not polynomial. It is ruled out
   by smoothness across the whole category (T4/T5 hold on spacetimes where Scal = 0, where a 1/Scal coefficient is not
   smooth), not by anything on M_{R,t} itself.
2. **Dimension.** The polynomial candidates that use the curvature direction, such as Ric^{μν} tr F_{μρ}F_ν{}^ρ vol
   and C^{μνρσ} tr F_{μν}F_{ρσ} vol (Weyl), have dimension 6. With no dimensionful constant in pure YM, Step 4 forbids
   them. The same holds for the Berger metric (t ≠ 1), whose Ricci tensor also has ∂_τ in its kernel and spatial
   eigenvalues that need not be equal.

Steps 6 and 7 encode exactly these facts: with dimension 4, any curvature factor leaves field content of dimension
≤ 2, and no gauge-invariant (Step 6) or ŝ-exact (Step 7) term of that kind exists. Gauge-variant curvature terms such
as Ric^{μν} tr A_μ A_ν (dimension 4) are not ŝ-closed. By Step 5 they cannot appear in D_m except inside ŝX + dY, and
by Step 7 the X there carries no curvature. So **D_m carries no c₀**. ∎

**Step 9 (all orders) [D, modelled on Hollands §4.4].** Write the leading change as in Step 6. In the region where
f = 1:
- the term a · tr F∧⋆F changes the interaction by a multiple of the YM Lagrangian. That is a finite renormalization
  of g (with the accompanying rescaling of A), consistent with [H-Rem3]: the YM Lagrangian is the unique gauge-
  invariant ghost-0 field of dimension 4;
- κ𝟙 shifts c-numbers only;
- dY is removed by [H-T11];
- ŝX: modify T by D′ = −ŝX at order ħ^m. This is allowed (local, covariant, dimension 4, polynomial; the same checks
  Hollands does after eq. (390)). [H-L11] with A_m = 0 gives the new anomaly (S, (S, X)) = 0 by the Jacobi identity
  and (S, S) = 0, so T12 still holds. ŝX is a BV canonical transformation: a field redefinition and/or a change of gauge
fermion (X may contain gauge-fermion-type terms such as tr C̄ d⋆A, ghost −1, dimension 4, a colour singlet). Either
way it is ŝ-exact and does not change the physical cohomology or c₀.
After these moves the two prescriptions agree through order ħ^m. Repeating Steps 4–8 at the next order gives (b) by
induction. At no order does a c₀ appear, which is (c). ∎

**Step 10 (independence of R and t) [D].** D is one natural transformation on the whole category, so its coefficients
(a, the coefficients of κ, the coefficients in X) are universal constants. The same numbers serve on every M_{R,t} and
on Minkowski space. In particular c₀ ≡ 1 holds uniformly in R > 0, t > 0, and the scheme change of g between two
covariant schemes is the same constant in every room. ∎

**Step 11 (clause (d), conditional) [Cd].** Take a regulator that breaks local covariance, with bare (g₀, c₀). Assume
the hypothesis (HR): for a suitable tuning (g₀(ε), c₀(ε)) the regulated perturbation series converges, order by
order, to time-ordered products satisfying T1–T12. Then by (b) the limit is a covariant scheme, and by (c) its
c₀-class vanishes. At each order the coefficient c₀^{(k)} is the unique value that cancels the regulator's
anisotropic class. (b) fixes it uniquely on each M_{R,t}. (HR) here means: on each M_{R,t} the limit coincides with
the restriction of one fixed locally covariant prescription T* (T1 is a condition across spacetimes, so a regulator
defined only on the sphere family cannot satisfy it by itself). Step 10 does **not** give R- or t-independence of the
bare c₀^{(k)}: Step 10 concerns differences of covariant prescriptions, and the bare c₀ of a non-covariant regulator is
not one. That needs the extra regulator-locality hypothesis of (d). Without (HR) nothing is claimed. (HR) is the curved-space, anisotropic analogue of Reisz's lattice theorem (flat, hypercubic). For H2's
spatial spectral cutoff with continuous time it has not been proved. ∎

---

## 5. A corollary I get for free: the running along R is the flat running (perturbative; cited remark)

Under g ↦ μ²g, M_{R,t} becomes (ℝ × S³, −μ²dτ² + μ²R²h_t). With τ′ = μτ this is isometric to M_{μR,t}, and the isometry
preserves orientation and time orientation. So by [H-T1], **Hollands' renormalization-group flow g ↦ μ²g is exactly
motion along my base coordinate R, at fixed t.** [H-Rem3] says that this flow of the coupling "must be exactly the same
as in Minkowski spacetime". So to all orders in perturbation theory the running of g with R on the sphere family is
the flat β-function. Line 14 concerns a *non-covariant* cutoff, so only the scheme-independent leading coefficient
(the universal b₀ in −(b₀/8π²) log(Λ_cut R)) transfers without the matching of clause (d). *Status:*
[H-Rem3] is a remark with a short argument, not a numbered theorem. I record this as cited support, not as a closure.
This fits my bundle framing: the UV data (the β-function, and c₀ ≡ 1) are constant along the base, and only the IR
(the vacuum and the spectrum in each room) moves with R.

---

## 6. Corrections to the route as handed to me

1. **Step 4 of the route is wrong as stated.** u = ∂_τ *is* locally recoverable from the Ricci tensor on M_{R,t}
   (§4, Step 8). The exclusion comes from polynomial dependence on curvature together with dimension 4 and the
   absence of a mass. The conclusion stands, by a different argument.
2. **Θ/CP is not needed.** tr F∧F is d-exact, trivial in H⁰(ŝ|d, P⁴), and in any case it is not an electric/magnetic
   split.
3. **"Dimension ≤ 4" should read "dimension = 4".** With no dimensionful constant, (257) fixes the dimension exactly.
   A Scal · 𝟙 term (dimension 2) is excluded for the same reason.
4. **Lemma 6 is not the global statement.** It concerns distributions on ℝ^{4n}. The global form of the freedom is
   [H-§3.6] with [H-Thm2], which is what the proof uses.

---

## 7. What this does not give

- **Nothing non-perturbative.** Everything is formal power series in g, around the flat connection A = 0. My slow
  room at large R (the reduced model on ℝ⁹/SO(3)) is non-perturbative. C₀ says nothing about whether a
  non-perturbative continuum limit on S³_{R,t} × ℝ needs only g₀. That is (U3)/(U4) of `H4-borrowed-routes.md`: the
  non-perturbative locality of UV counterterms, which a Bałaban-type sphere RG would have to deliver.
- **Nothing Euclidean.** Hollands is Lorentzian and algebraic. H4′ is stated for Euclidean correlation functions
  (OS). Going there needs a ground state of the interacting perturbative theory on the static M_{R,t} and a Wick
  rotation. I have not verified a citation for either.
- **No states and no spectrum.** Hollands gives an algebra and "a space of corresponding quantum states". C₀ says
  nothing about the vacuum, the gap or H3.
- **No value of c₀ for my regulator.** Clause (d) is conditional on (HR). The one-loop number c₀^{(1)} for H2's
  spatial heat-kernel or spectral cutoff is an uncomputed flat one-loop calculation, the analogue of
  Hasenfratz–Hasenfratz / Karsch. I quote no number.
- **No O(4) at the R → ∞ end.** C₀ removes a *marginal* source of anisotropy in covariant schemes. The curvature and
  global-closing effects on S³_R (O(τ/R²), Casimir terms) and the IR convergence to the ℝ⁴ vacuum are untouched
  (H5 Step 6, H4′ second bullet).
- **The vector/ghost version of Theorem 2** is taken from Hollands' "completely analogous" (§3.5) and "the analog of
  conditions T1–T11" (§4.3). It is not a separately written proof.
- **Step 9's induction** is my deduction in the pattern of Hollands' anomaly removal (§4.4, Lemma 11, eq. (390)). It
  is not a numbered theorem of his.
- **Flowed observables.** Finiteness of gradient-flow correlators on S³ × ℝ (U5) is not addressed.

---

## 8. Suggested register reading (I do not edit the register here)

H4′, first bullet: the sentence "the limit tunes two bare parameters, the coupling g₀ and a speed of light c₀" could
carry the note: *In perturbation theory, in Hollands' gauge-fixed framework, every locally covariant scheme on
Lorentzian ℝ × S³_{R,t} has c₀ ≡ 1 for all R, t (Hollands 2008 + this note, [PC + D], modulo the YM analogue of
Hollands' Theorem 2). c₀ is regulator data, needed only for non-covariant regulators such as H2's spatial cutoff, where
it is fixed by matching [Cd on a Reisz-type theorem; R-independence needs a further regulator-locality hypothesis]. The
non-perturbative, Euclidean statement remains part of H4′.* The labelled conjecture in paper §8 ("c₀ is controlled by
the trace anomaly") should be replaced by "in perturbation theory, c₀ is regulator data: covariant schemes have no c₀",
as `H4-borrowed-routes.md` §3 already proposes.
