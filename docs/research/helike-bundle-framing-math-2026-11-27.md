# The bundle framing, in pure mathematics, and built in GIGI

27 November 2026. A companion to §4.10 of *One note in every room*. Everything here is standard mathematics, written
so that each step can be checked. The GIGI construction at the end tests the theorems against planted, exact answers.

## 1. The classical object: a connection on a principal bundle

**Definitions.** Let M be spacetime and G a compact Lie group, for example SU(2).
- **The bundle.** A principal G-bundle P → M: over each point x of the base, a copy of G (the fibre), on which G acts
  from the right.
- **A connection** A tells you how to move along a path in M while staying "level" in the fibres. Its output is
  parallel transport: for a path γ from x to y, an element U_γ that carries the fibre over x to the fibre over y.
  Transports compose: U_{γ₁γ₂} = U_{γ₁}U_{γ₂}.
- **The holonomy** of a closed loop C based at x is the transport around it: Hol_C(A) ∈ G.
- **The curvature** F = dA + A∧A is the holonomy of infinitesimal loops: for a small loop spanning an area element
  dS^{μν}, Hol ≈ exp(F_{μν} dS^{μν}).
- **A gauge transformation** g is a choice of g(x) ∈ G at each point, a change of how each fibre is labelled. It acts
  on transports by U_γ ↦ g(x) U_γ g(y)⁻¹ for a path from x to y.

**Lemma 1 (holonomy is gauge-covariant).** For a loop based at x, Hol_C ↦ g(x) Hol_C g(x)⁻¹.

*Proof.* Take y = x in the transport rule. ∎

**Theorem 1 (Wilson loops are gauge-invariant).** W_C(A) = tr_ρ Hol_C(A), for any representation ρ, is unchanged
by every gauge transformation, and does not depend on where on C the loop is based.

*Proof.* By Lemma 1 the holonomy is conjugated, and the trace is invariant under conjugation:
tr(gHg⁻¹) = tr(H). Changing the base point from x to x′ also conjugates the holonomy, by the transport from x to x′
along C. ∎

So the value of A at a point is not physical, since it changes under a gauge transformation, but the holonomy around
every loop, up to conjugation, is physical.

**Theorem 2 (Giles, 1981).** The Wilson loops of all loops determine the connection up to gauge equivalence (Giles,
Phys. Rev. D 24 (1981) 2160). *Cited, not proved here.*

So "querying every loop reaches every field" is a theorem: the gauge-invariant content of a connection is exactly its
loop data.

## 2. The lattice: a bundle sampled at keys

**Definition.** Replace M by a lattice Λ of spacing a.
- **The connection:** on each oriented edge ℓ = (x → y) store U_ℓ ∈ G, its transport, with U_{y→x} = U_ℓ⁻¹. This is
  Wilson's lattice gauge theory (Phys. Rev. D 10 (1974) 2445).
- **Gauge transformations:** g_x ∈ G at each vertex, acting by U_{x→y} ↦ g_x U_{x→y} g_y⁻¹.
- **The Wilson loop** of a closed path x₀ → x₁ → … → x_n = x₀ is W = tr(U_{x₀x₁} U_{x₁x₂} ⋯ U_{x_{n−1}x₀}).

**Theorem 3 (lattice gauge invariance).** W is invariant under every lattice gauge transformation.

*Proof.* The product telescopes: g_{x₀} U₀₁ g_{x₁}⁻¹ g_{x₁} U₁₂ g_{x₂}⁻¹ ⋯ g_{x_{n−1}} U_{n−1,0} g_{x₀}⁻¹ =
g_{x₀} (U₀₁ ⋯ U_{n−1,0}) g_{x₀}⁻¹. Take the trace. ∎

**The action.** S = β Σ_p (1 − ½ tr U_p), summed over plaquettes p, where U_p is the holonomy of a single square.
- U_p = exp(i a² g F_{μν} + O(a³)).
- For SU(2) with β = 4/g², S → (1/2) ∫ tr F_{μν}F^{μν} d⁴x as a → 0, which is the Yang–Mills action. A standard
  expansion; not proved here.

**The quantum theory** on the lattice is the probability measure
dμ_β(U) = Z⁻¹ e^{−S(U)} ∏_ℓ dU_ℓ,
with dU the Haar measure on G. It is a measure on lattice connections, and since S is gauge-invariant it descends to
a measure on lattice connections modulo gauge, the finite version of A/𝒢. The continuum problem is to take a → 0
(and the volume to infinity) and show that the loop expectations ⟨W_C⟩ converge to a theory satisfying the
Osterwalder–Schrader axioms, with exponential clustering (§1.5 of the note).

## 3. Two dimensions, solved exactly

**Theorem 4 (the area law in two dimensions).** Take G = SU(2) and the Wilson action on a planar lattice with open
boundary. For every simple closed lattice loop C enclosing |A| plaquettes,

  ⟨½ tr Hol_C⟩ = (I₂(β) / I₁(β))^{|A|},

where I_n is the modified Bessel function.

*Proof.*
1. **Gauge-fix on a maximal tree.** A spanning tree of the lattice graph can be gauged to 1. Theorem 3 lets us do this
   without changing any Wilson loop. Take every vertical edge and the bottom row of horizontal edges.
2. **The plaquettes become free variables.** The remaining edges are in bijection with the plaquettes, through
   U_{(x,y+1),0} = U_p⁻¹ U_{(x,y),0}. So the plaquette variables {U_p} are free coordinates.
3. **They are independent.** The Haar measure on the remaining edges pushes forward to the product Haar measure on
   {U_p}. The action is a sum over plaquettes. So the U_p are independent, each with density ∝ e^{(β/2) tr U} = e^{β a₀}
   with respect to Haar measure, writing U = a₀ + i a·σ.
4. **The loop is a product of plaquettes.** In this gauge, the holonomy of C is an ordered product of the U_p inside
   C, each conjugated by elements built from the other, independent, variables. That is the lattice non-abelian Stokes
   theorem.
5. **Each density is a class function** (it depends on U only through tr U), so conjugation does not change its law.
6. **The averages factorise.** For independent U, V with class-function laws,
   E[χ(UV)] = E[χ(U)] E[χ(V)] / d. This follows from ∫ χ(U h V h⁻¹) dh = χ(U)χ(V)/d. Here χ is the trace in the
   fundamental representation, with d = 2.
7. **One plaquette.** For a single plaquette, E[½ tr U_p] = E[a₀]. Since the Haar measure gives a₀ the density
   √(1 − a₀²) on [−1, 1],
   E[a₀] = ∫ a₀ e^{βa₀} √(1−a₀²) da₀ / ∫ e^{βa₀} √(1−a₀²) da₀ = I₂(β)/I₁(β).
8. **Multiply** over the |A| plaquettes. ∎

**Corollary (the string tension is the Casimir).**
- ⟨W_C⟩ = e^{−σ|A|} with σ = −ln(I₂/I₁).
- As β → ∞, σ = 3/(2β) + O(β⁻²).
- With β = 4/g²_lat, σ = (3/8) g²_lat = (g²_lat/2)·C₂(½), where C₂(½) = 3/4 is the quadratic Casimir of the
  fundamental representation.
- In the continuum limit a → 0 at fixed physical coupling, the physical area law is exactly
  ⟨W_C⟩ = exp(−(g²/2)·C₂(½)·Area). This is the known exact continuum result in two dimensions.
- It is the same Casimir 3/4 that §4.9 identified as the note of one turn.

**What two dimensions does not show.**
- Two-dimensional Yang–Mills has no propagating gluons. Loops that enclose disjoint regions are exactly independent:
  their connected correlation is zero, not exponentially small.
- So two dimensions exhibits confinement, the area law, exactly, but its gap is trivial. In four dimensions the
  plaquettes interact through shared edges in three dimensions of plaquette orientation, the tree trick fails, and
  both the area law and the gap are the open problem.

## 4. Built in GIGI

Script: `scripts/gigi_lattice_gauge_demo.py`; results: `research/gigi-lattice-gauge-20261127/`.

**The construction.**
- **The bundle.** A GIGI bundle `lattice_su2`:
  - base: the lattice edges (an id encoding the configuration, x, y and the direction);
  - fibre: an SU(2) element stored as a unit quaternion (a₀, a₁, a₂, a₃).

  Each gauge-field configuration is one section.
- **Sampling.** Configurations are sampled exactly, by the proof's construction: tree gauge, independent plaquettes
  with density e^{βa₀}. Each is then scrambled by a random gauge transformation, so nothing stored looks gauge-fixed.
- **Reading.** Every Wilson loop is computed only from GIGI point queries (`SECTION … AT id=…`) of the edges along the
  loop, multiplied in order. This is "a point query on the bundle", made gauge-invariant by querying along a loop.

**The planted checks.**
1. **The area law.** The measured ⟨½ tr W⟩ for rectangles must match (I₂/I₁)^{area}, within statistical error.
2. **Gauge invariance.** Every stored section is rewritten in place by a fresh random gauge transformation (`SECTIONS …
   UPSERT`). Every Wilson loop must be unchanged to rounding, while a single edge, the value at a "point", must change.

Results: see §5 once run.

## 5. GIGI results (27 November)

Run: L = 10, β = 4, 24 configurations, 43,010 GIGI point queries, 87 s, on a local `gigi-stream` (0.4.1).
Output: `research/gigi-lattice-gauge-20261127/result.json`.

**Area law.** The exact value is (I₂/I₁)^{area}, with I₂(4)/I₁(4) = 0.658047. Rectangles of the same size are taken
on disjoint tiles, so they are independent samples.

| loop | measured ⟨½ tr W⟩ | exact | z |
|---|---|---|---|
| 1×1 | 0.6607 ± 0.0064 | 0.6580 | +0.42 |
| 1×2 | 0.4275 ± 0.0137 | 0.4330 | −0.40 |
| 2×2 | 0.1803 ± 0.0236 | 0.1875 | −0.30 |
| 2×3 | 0.0782 ± 0.0292 | 0.0812 | −0.10 |
| 3×3 | −0.0409 ± 0.0331 | 0.0231 | −1.94 |

All five agree within 2σ. The lattice string tension is σ = −ln(I₂/I₁) = 0.4185 per plaquette.

**Gauge invariance.**
- Every stored section was rewritten in place by a fresh random gauge transformation.
- The largest change in any Wilson loop was **1.2·10⁻¹⁵**, which is rounding.
- A single edge, a point query of the connection itself, changed by **1.27**.

So the bundle stored in GIGI behaves as the theorems say. Point values are gauge artefacts, loop values are physical,
and the loop data carry the area law, with string tension equal to the Casimir.

**Engine fix found on the way.** `gigi-stream` did not build on macOS. Two `libc::malloc_trim` calls (glibc-only) were
gated on `cfg(unix)`; they are now gated on `cfg(target_os = "linux")`. Behaviour on Linux and Fly is unchanged.

---

# Part II: every cell of the grid (note §4.12)

## 6. The structure group: the double cover and the centre (layer 2)

**Facts.**
- SU(2) → SO(3) is a two-to-one homomorphism with kernel the centre Z = {±1} ≅ ℤ₂.
- The irreducible representations of SU(2) are labelled by spin j ∈ {0, ½, 1, …}, with dimension d_j = 2j+1, character
  χ_j and quadratic Casimir C₂(j) = j(j+1).
- The centre acts as −1 on half-integer j and as +1 on integer j. So the half-integer representations are exactly
  those that do not descend to SO(3): **they exist because of the double cover**.

**Theorem 5 (every representation's area law in two dimensions).** In the setting of Theorem 4,

  ⟨χ_j(Hol_C)⟩ / d_j = (I_{2j+1}(β) / I₁(β))^{|A|}.

*Proof.* As for Theorem 4, with χ_{1/2} replaced by χ_j. The factorisation E[χ_j(UV)] = E[χ_j(U)]E[χ_j(V)]/d_j holds
for every irreducible character. For one plaquette, the Peter–Weyl expansion
e^{(β/2)tr U} = Σ_j d_j c_j(β) χ_j(U), with c_j(β) ∝ I_{2j+1}(β)/β, gives
E[χ_j(U_p)]/d_j = c_j/c₀ · (normalisation) = I_{2j+1}(β)/I₁(β). ∎

**Corollary (Casimir scaling).** Write σ_j = −ln(I_{2j+1}/I₁). Using I_n(β) ≈ (e^β/√(2πβ))(1 − (4n²−1)/(8β)),

  σ_j = 2 C₂(j)/β + O(β⁻²),  so σ_j / σ_{1/2} → C₂(j)/C₂(½).

In the continuum this ratio is exact: σ₁/σ_{1/2} = 2/(3/4) = 8/3. The tension of a loop is its representation's
Casimir, the note of that representation's turn.

**Theorem 6 (centre vortices flip exactly the double-covered loops).** Choose a set of plaquettes, a "vortex", and
multiply one edge of each by the centre element −1. Then every Wilson loop in representation j is multiplied by
(−1)^{2j·n}, where n is the number of times the loop winds around the vortex.

*Proof.* Each crossing multiplies the holonomy by −1, which is central, so the holonomy is multiplied by (−1)^n. Then
χ_j(−U) = (−1)^{2j} χ_j(U). ∎

So j = ½ loops feel centre vortices, and integer-j loops do not. Random vortices piercing a loop with density p per
plaquette give ⟨W_{1/2}⟩ ≈ (1 − 2p)^{|A|}, an area law, while leaving adjoint loops unchanged. That is the centre-vortex
picture of why the double-covered loops confine (the 't Hooft criterion). In two dimensions it is exact in the
following sense: the sign of each plaquette's a₀ is a ℤ₂ "vortex" variable.

## 7. Topology on the Maurer–Cartan line (layer 5)

**The setting.** Homogeneous connections on S³ are A = q_{ka} θᵏ T_a, where θᵏ is the left-invariant coframe and T_a
the generators. The Chern–Simons functional
CS(A) = (1/8π²) ∫ tr(A∧dA + ⅔ A∧A∧A)
is metric-independent. It changes by an integer under large gauge transformations, classified by
π₃(SU(2)) = ℤ.

**Lemma 7 (the winding-1 vacuum is homogeneous).** On S³ = SU(2), the Maurer–Cartan form Ω = U⁻¹dU, with U the
identity map, is a flat connection of winding number 1. In the left-invariant coframe it is homogeneous: q = q_MC.

*Proof.* Ω is left-invariant by construction. It satisfies dΩ + Ω∧Ω = 0, so F = 0. Its winding is the degree of the
identity map, which is 1. ∎

**Theorem 8 (the Chern–Simons number along the line).** For A_λ = λΩ,

  CS(λ) = 3λ² − 2λ³.

*Proof.* dΩ = −Ω∧Ω. So tr(A dA) = −λ² tr Ω³ and ⅔ tr A³ = ⅔ λ³ tr Ω³, giving
CS ∝ (−λ² + ⅔λ³) ∫ tr Ω³. Fixing CS(1) = 1 (winding 1) gives the formula. ∎

The curvature along the line is F_λ = (λ² − λ) Ω∧Ω. So the Yang–Mills energy is ∝ (λ − λ²)²: a double well with zeros
at λ = 0 (winding 0) and λ = 1 (winding 1), and the sphaleron at λ = ½, where CS = ½.

**Theorem 9 (the model's vacua, and where the background sits).** In the reduced model,
V(q) = Σ_k Σ_c ½(ω_k q_{kc} − g cof(q)_{kc})² − ½ Σ_k a_k Σ_c q_{kc}²,
with ω = (√t, 1/√t, 1/√t) and sources (a, b, b).
- **The vacua.** With the sources off, the zeros on the diagonal are q = 0 and q_MC = diag(1/√t, 1, 1)/g. The second
  is the winding-1 pure gauge of the Berger sphere.
- **The potential along the line.** V(λ q_MC) = (1 + 2/t)(λ − λ²)²/(2g²): exactly the double well of Theorem 8.
- **The background.** The registered background q_bg = diag(s/√t, s, s)/g equals s·q_MC exactly. It sits on the
  instanton–sphaleron line at λ = s, just past the winding-1 vacuum (1 < s < (t+2)/(t+1)), held there by the sources.
  Its Chern–Simons number is 3s² − 2s³: 0.953, 0.926, 0.891 and 0.800 at s = 1.12, 1.15, 1.18 and 1.24.

*Proof.*
1. **The vacua.** For diagonal q = diag(d₁, d₂, d₃), cof(q) = diag(d₂d₃, d₁d₃, d₁d₂). Vacuum means ω_k d_k = g·cof_k,
   which gives d₁² = ω₂ω₃/g², d₂² = ω₁ω₃/g² and d₃² = ω₁ω₂/g². With the given ω this is q_MC.
2. **The potential.** Substituting q = λ q_MC gives each term (λ − λ²)·ω_k m_k/… and the stated sum.
3. **The background.** The background is diag(x, y, y)/(2g) with (x, y) = (2s/√t, 2s), which is s·q_MC. ∎

**What this places.** The θ-angle of pure Yang–Mills is the Bloch phase for the shift λ ↦ λ + 1 (a large gauge
transformation) along this line. Instantons are the tunnelling paths through the sphaleron. All of it lies inside the
reduced configuration space ℝ⁹. The Helike family is a walk along this line, from the winding-1 vacuum back toward
the sphaleron: the topological sector is not missing from the construction, it is its spine.

**Open.**
- The quantum tunnelling amplitude through the sphaleron.
- The θ-dependence of the gap, through the Bloch band structure along λ.

Both are computable in the reduced model. Neither is computed.

## 8. The base's scale: running coupling and Λ (layer 1)

**Facts (one loop, standard).**
- With the action (1/2g²)∫tr F², the coupling runs as μ dg/dμ = −b₀ g³, with b₀ = 11N/(48π²) for SU(N).
- So 1/g²(μ) = 2b₀ ln(μ/Λ). The scale Λ is generated, and does not appear in the classical theory: this is
  dimensional transmutation.
- On a sphere of radius R, the coupling is g²(1/R), so RΛ = exp(−1/(2b₀g²)).

**What it places.** The gap must come out as Δ = c·Λ, with c a pure number. In the small room, the computed gaps are
Δ·R = f(g²(R)). The "translator" (§4.3) is the map f ↦ c as R → ∞. The infrared half of the proof (M2) is to show that
Δ·R/(RΛ) stays bounded away from zero.

**Open.** Everything beyond one loop, and the matching between the reduced model's g² and a scheme. Bloch's g²(R)/2π²
(van den Heuvel) is the existing bridge.

## 9. M3 as a bundle over the slow configurations (layer 8)

**The construction (Born–Oppenheimer).**
- Split the connection on the small sphere into slow modes (the homogeneous q ∈ ℝ⁹) and fast modes (the higher
  harmonics η).
- At fixed q, the fast modes see a Hamiltonian H_fast(q). Its stiffness grows like n/R for the n-th harmonic.
- The ground state |0(q)⟩ of H_fast(q) defines a **line bundle over the slow space** ℝ⁹/SO(3).
- Projecting onto that line bundle gives the effective slow Hamiltonian

  H_eff = ½ g^{ij}(q)(pᵢ − 𝒜ᵢ(q))(pⱼ − 𝒜ⱼ(q)) + V(q) + E₀(q) + Φ(q),

  where:
  - 𝒜ᵢ = i⟨0(q)|∂ᵢ 0(q)⟩ is the **Berry connection** of the fast ground state;
  - E₀(q) = ½ Σ_n ω_n(q) is the fast zero-point energy, a q-dependent potential;
  - Φ is the Born–Huang scalar correction.

**Where it meets the other cells.**
- **Layer 1.** The zero-point sum Σ_n ω_n(q) diverges in the ultraviolet. Its q-dependent divergent part has the form of
  the classical action, so it renormalizes g. The running coupling of §8 *is* the divergent part of the dressing.
- **Layer 7.** The finite part shifts the slow levels. M3 is the statement that these shifts are uniformly bounded below
  the gap and do not exchange Helike and the competitor except at an identifiable switch.

**Template, and what is missing.** Lüscher on the torus, and van Baal on S³, computed these corrections in
perturbation theory, and van Baal's effective Hamiltonian was accurate. What is missing is a bound uniform as the
cutoff on n goes to infinity, which is Bałaban's territory. A first computation is in reach: the Berger-sphere Hessian
spectrum at homogeneous q is already computed (*By Enough*, Floor), so E₀(q) can be summed over the lowest sectors as a
function of q and compared with the gaps.

## 10. The fold: two sheets and the boundary realization (layer 4)

**Facts.**
- G = qqᵀ does not see the sign of det q. The two sheets det q > 0 and det q < 0 meet where rank q < 3. Together they
  form an orientation double cover of the cone of Gram matrices.
- The potential is not sheet-symmetric. Its cubic term is −g(Σω) det q, so V(q) − V(−q) = −2g(Σω) det q, the signed
  cubic of report page 12.
- The physical operator on ℝ⁹ imposes matched values and a vanishing conormal sum across the fold. That is the
  **natural** realization, with no wall.
- The log-Cholesky basis vanishes at the fold faster than any power, so its form closure is the **Dirichlet**
  realization.
- The unfolded chart P covers both sheets and so approximates the natural one.

**What it places.**
- The two realizations differ by tunnelling to the fold, which is exponentially small at weak coupling.
- Through Theorem 9 the fold is also topological: det q is the Chern–Simons cubic, so the sheet structure and the
  winding structure are the same odd term seen twice.

**Open.** A bound on the realization difference, and its sign.

## 11. The spectrum's labels and the glueball link (layer 7)

**Facts.**
- States are labelled by representations of the base's isometry group. For the Berger sphere that is the residual
  U(1), with charge m; for the round sphere and in infinite volume, it is rotations and parity, giving J^PC.
- Helike is charge 2, a tensor-like state. The competitor is charge 0, a scalar-like state.
- In infinite volume, lattice studies find the lightest glueball is the scalar 0⁺⁺ and the tensor 2⁺⁺ is heavier.
  *verify* a citation before quoting ratios.

**The link (conjecture, falsifiable).** The registered predictions s*(g²) rise with g²: 1.1674, 1.1809 and 1.1931 at
g² = 0.03, 0.05 and 0.07. A larger g² is a larger room. The region s < s*, where the charge-0 competitor is lowest,
therefore grows with the room. The switch would be the small-volume precursor of the infinite-volume ordering
0⁺⁺ < 2⁺⁺. The test: the measured s* (§5.11, once the grid runs) must rise with g², and continue to rise along the M2
sweep.

## 12. Associated bundles and the base's geometry (layers 9 and 10)

- **Matter.** Matter fields are sections of associated vector bundles P ×_G V. Pure Yang–Mills, the Clay problem, has
  none, so the cell is empty by choice.
- **Gravity.** The base's own geometry is the metric, through the frame bundle. Gravity makes the base dynamical and
  lies outside the problem.
