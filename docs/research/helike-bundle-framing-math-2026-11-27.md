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
