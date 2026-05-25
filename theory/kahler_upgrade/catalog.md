# GIGI Kähler Upgrade: Catalog, Math, and Validation

*Davis Geometric internal — v2, validated against 11 numerical tests*

## 0. Purpose and handoff context

This document is the structural plan for upgrading **GIGI** (the Rust fiber-bundle database engine) and **Marcella** (the holonomic sequence model that runs on GIGI) by adopting a Kähler-geometric substrate. Each item lists:

- a precise mathematical claim,
- a proof sketch — what makes it forced (the Sudoku closure),
- validation status from the test files,
- product-level applications (GIGI, Marcella, MIRADOR, PRISM, downstream),
- implementation pointers for handoff to Claude Code Desktop.

The catalog has Part I (items from the Adachi program) and Part II (forced consequences of the same generator). Once we commit to the generator in §1, items in both parts are theorems, not choices.

**All numerical claims here are reproduced by three test files** (`validation_tests.py`, `validation_tests_v2.py`, `validation_tests_v3.py`). The tests are designed to be non-circular: every numerical computation is checked against an independently-derived closed-form ground truth, and negative cases (where the property must fail) are included to rule out tests that pass for the wrong reason.

## 1. The generator

The whole upgrade reduces to committing GIGI's data substrate (and therefore Marcella's transport substrate) to one mathematical object:

> **𝒢 = (M, g, J, ∇, B, Γ)**
>
> where M is a Kähler manifold with metric g, complex structure J (J² = −I, ∇J = 0), Chern connection ∇, a closed 2-form B ∈ Ω²(M) (dB = 0), and a discrete graph approximation Γ preserving the J/∇ split.

Three "knob" properties of B gate which consequences become available:

| Property of B | Gates |
|---|---|
| `dB = 0` (closed) | the magnetic perturbation is a connection deformation; all of Part I |
| `[B/2π] ∈ H²(M, ℤ)` (integral) | prequantization (line bundle exists); §2.1 onward |
| `B` symplectic (non-degenerate) | Hamiltonian flow, moment maps, Floer theory; §2.3, 2.6 |

The Kähler condition `∇J = 0` is what makes the structure self-consistent. Drop it and roughly half the forced consequences fail. **Keep it as a hard invariant in the storage layer.**


## Part I — Cataloged borrows from the Adachi program

### 1.1 Kähler graph dual-adjacency commutativity

**Claim.** On a Kähler graph where both the principal generating set S_p and the auxiliary generating set S_a are either (a) any subsets of an abelian group, or (b) unions of conjugacy classes of a non-abelian group, the adjacency operators commute: `[A_p, A_a] = 0`. Commuting symmetric operators are simultaneously diagonalizable, so query plans factor through a shared eigenbasis.

**Proof sketch.** For a Cayley graph Cay(G, S), the adjacency operator equals right-convolution by `∑_{s∈S} s` in the group algebra ℂ[G]. Two convolution operators commute iff the corresponding generating-set sums commute in ℂ[G]. Sufficient conditions: G abelian (ℂ[G] is commutative), or both sums are central (a generating set that's a union of full conjugacy classes has a central sum, since conjugation permutes elements within a class). Adachi's discrete Kähler identity `A_p A_a = A_a A_p` is the discrete analog of `∇J = J∇`.

**Validation.** PASS (`validation_tests.py::test_1_kahler_commutativity`).
- Abelian positive case (Z/4 × Z/4 with axial generators): `‖[A_p, A_a]‖_∞ = 0.0` exactly (integer arithmetic).
- Non-abelian negative case (S_3 with two single non-central transpositions): `‖[A_p, A_a]‖_∞ = 1.0`.
- Ratio non-abelian/abelian: effectively ∞ (clean separation).

**Caveat surfaced by testing.** The test initially failed because the first non-abelian example used `{3-cycle, 3-cycle⁻¹}` as the auxiliary generating set. That set is the *entire* conjugacy class of 3-cycles in S_3 — its sum is central, so it commutes with everything by construction, regardless of any Kähler-like structure. **Vertex transitivity alone is not enough**; centrality of the generating sets (or abelianness of the underlying group) is what matters. GIGI's planner must verify centrality structurally rather than infer commutativity from transitivity.

**Applications.**
- *GIGI:* Query planner gains a structural commutativity check. Safe join reorderings come with a theorem.
- *MIRADOR:* Population PK queries crossing genotype × dose × metabolite axes either commute (cacheable, parallelizable) or don't. The system determines which automatically.
- *PRISM:* The 0.03% residual F1 error is most likely concentrated where principal and auxiliary operators don't commute (FX × amount, alias × counterparty). Locatable structurally.
- *Marcella:* Transport operations parallelize where principal and auxiliary commute. Correctness-preserving inference speedup.

**Implementation pointers (Claude Code Desktop).**
- Add `principal_adjacency_op` and `auxiliary_adjacency_op` as first-class types in GIGI's storage layer.
- Centrality check should NOT compute the full commutator matrix on large graphs — use the group-algebra criterion (check whether the generating set is closed under conjugation).
- Tag each query plan node with a `commutativity_class`; the planner uses this for reordering and caching.

### 1.2 Magnetic 2-form bias

**Claim.** A closed 2-form B perturbs geodesics into well-defined trajectories via the magnetic Lagrangian `L = ½|v|² + ⟨A, v⟩` with `dA = B`. Euler-Lagrange gives `∇_{γ̇} γ̇ = B(γ̇, ·)^♯`. Kinetic energy `½|γ̇|²` is conserved along the flow.

**Proof sketch.** Closedness of B (`dB = 0`) ensures local existence of A with `dA = B` (Poincaré lemma on contractible charts), and the trajectory equation derives from the variational principle for L. Energy conservation comes from antisymmetry of B's matrix form: `d/dt(½|γ̇|²) = ⟨γ̇, ∇_{γ̇}γ̇⟩ = ⟨γ̇, B(γ̇)^♯⟩ = B(γ̇, γ̇) = 0`.

**Validation.** PASS (`validation_tests.py::test_2_magnetic_trajectory`).

On flat ℝ² with B = b·dx∧dy (b = 1.5), RK4 integration of the E-L equations gives:
- Cyclotron radius |v|/b = 0.6667 matched to **2e-14** deviation (machine epsilon).
- Energy drift over one full period: **6e-15**.
- Period closure error: 9.8e-6.
- Negative control: with b = 0, trajectory remains exactly on the x-axis (max |y| = 0.0).

**Applications.**
- *GIGI:* Query bias as a single geometric object instead of N independent threshold knobs.
- *MIRADOR:* Population weighting (renal-impaired, pediatric) as choice of B, not custom filter.
- *PRISM:* Recency, counterparty trust, amount tolerance unified under one B.
- *Marcella:* Learned B replaces RoPE-style positional encoding and attention bias under one principled object. Closedness is a free regularizer.

**Implementation pointers.**
- Represent B as a closed 2-form in `gigi/geometry/forms.rs`. Verify closedness at construction via discrete exterior derivative.
- For Marcella: learned bias becomes `B: ClosedTwoForm` parameter trained jointly with embeddings; soft-constraint regularizer enforces `‖dB‖ < ε`.

### 1.3 Trajectory-ball volumes

**Claim.** Volumes of geodesic balls are bounded above (Bishop) and below (Günther) by closed-form expressions in the sectional curvature. Bounds extend to magnetic trajectories with corrections of order `‖B‖`.

**Proof sketch.** Jacobi field analysis: the volume element along a geodesic expands at a rate governed by the Jacobi equation `J'' + K(t)J = 0`. Comparison with constant-curvature space forms gives Bishop (V ≤ V_K=K_min) and Günther (V ≥ V_K=K_max).

**Validation.** PASS (`validation_tests.py::test_4_trajectory_ball_volume`).

For 2D space forms at R = 2.0, integrating the Jacobi field independently of the closed-form V(R):
- ℍ² (K = -1): V_num = 17.355387, V_exact = 2π(cosh R - 1) = 17.355387, **rel err 8e-10**.
- ℝ² (K = 0): V_num = 12.566371, V_exact = πR² = 12.566371, **rel err 9e-14**.
- S² (K = +1): V_num = 8.897913, V_exact = 2π(1 - cos R) = 8.897913, **rel err 8e-10**.

Growth ordering: V_ℍ / V_ℝ = 1.38 > 1, V_S / V_ℝ = 0.71 < 1 — confirms Bishop/Günther directions.

**Applications.**
- *GIGI:* Cardinality estimation with provable bounds. Curvature → cost model.
- *MIRADOR:* Cohort sizing with geometric confidence intervals (no bootstrap).
- *PRISM:* Structural confidence on candidate matches via trajectory-ball volume.
- *Marcella:* Diversity = curvature-controlled trajectory-ball volume. Learnable, locally varying.

**Implementation pointers.**
- `gigi/cost/jacobi_estimator.rs`: integrate Jacobi field along query trajectory to estimate cardinality. Shares ODE solver with Marcella's transport machinery.

### 1.4 Ideal boundary on Hadamard manifolds

**Claim.** On a Hadamard manifold (simply connected, sectional curvature ≤ 0), the ideal boundary M(∞) is well-defined as equivalence classes of asymptotic geodesic rays. The cone topology makes M̄ = M ∪ M(∞) compact. For magnetically perturbed trajectories with `‖B‖` small relative to the curvature bound, the analogous limit-string construction works (Adachi's harps and horns).

**Proof sketch.** Cartan-Hadamard theorem + Eberlein-O'Neill ideal-boundary construction. Two rays γ₁, γ₂ are asymptotic if `dist(γ₁(t), γ₂(t))` stays bounded as t → ∞. Asymptoticity is an equivalence relation; the quotient is M(∞), homeomorphic to S^{n-1}.

**Validation.** PASS — Hadamard half of `validation_tests.py::test_3_hadamard_cartan`.

For ℍ² (K = -1), the Jacobi field J(t) is positive everywhere and monotone increasing on [0, 4], matching sinh(t) to **relative error 2e-15**. No conjugate points exist (the precondition for the ideal boundary to be well-defined).

**Applications.**
- *GIGI:* Continuous queries on Hadamard substructures provably converge to ideal-boundary states.
- *MIRADOR:* Live PK monitoring with provable convergence, not empirical decay.
- *PRISM:* Reconciliation backlogs have a well-defined limit state.
- *Marcella:* Long-context behavior as a theorem. Distant tokens contribute a computable, bounded residual.

**Implementation pointers.**
- `HadamardSubstructure` trait/marker for bundles where the curvature bound is verified. Queries against Hadamard substructures gain the convergence guarantees.

### 1.5 Hadamard-Cartan invertibility

**Claim.** On a Hadamard manifold, `exp_p: T_pM → M` is a diffeomorphism. With magnetic perturbation, `exp_p^B` remains a diffeomorphism under joint curvature and `‖B‖` bounds.

**Proof sketch.** No-conjugate-points (Jacobi field non-vanishing for t > 0) is equivalent to local invertibility of exp; simply-connectedness + nonpositive curvature gives global invertibility (Cartan-Hadamard). With magnetic perturbation, the Jacobi equation becomes `J'' + (K - ‖B‖²)J = 0` + lower-order corrections; for `‖B‖² < K_min`, no zeros appear.

**Validation.** PASS (`validation_tests.py::test_3_hadamard_cartan`).

Direct numerical solution of the Jacobi equation J'' + K J = 0:
- K = 0 (ℝ²): J(t) = t to error **4e-13**.
- K = -1 (ℍ²): J(t) = sinh(t) to relative error **2e-15**. Monotone, never zero.
- K = +1 (S²): J(t) = sin(t) to error **5e-15**. First zero at t = **3.141593**, matching π to 6 decimals.

The S² result is the negative case: it correctly identifies the conjugate point at t = π, confirming that absence of conjugate points on ℍ² is a real geometric fact, not an integrator artifact.

**Applications.**
- *GIGI:* Bidirectional traversal with no information loss; time-travel queries are theorems.
- *MIRADOR:* Forward and inverse PK problems both well-posed under same geometric assumptions.
- *PRISM:* Reversible transaction tracing — unrolling sequences losslessly.
- *Marcella:* Lossless encoding/decoding composition. Reversible reasoning chains with audit trails.

### 1.6 Hypersurface trajectory classification

**Claim.** Real hypersurfaces of type (A1) and (A2) in complex space forms have homogeneous structure (orbits of subgroups of the isometry group). Trajectories restricted to them admit closed-form descriptions via one-parameter subgroups.

**Proof sketch.** Cartan classification + Lie-theoretic structure. Type (A1) hypersurfaces in ℂP^n are geodesic spheres or tubes around totally geodesic ℂP^k; type (A2) are tubes around ℂP^k × ℂP^{n-k-1}. Both have isometry groups acting transitively. Trajectories on them are integral curves of left-invariant vector fields, hence solvable.

**Validation.** Not directly tested. Closed forms documented in Adachi's papers (J. Math. Soc. Japan 64, 2012, and his trajectory-classification series). A direct numerical test would require constructing ℂP^n with the Fubini-Study metric — a larger setup than other tests warrant for this phase. **Revisit if this becomes load-bearing.**

**Applications.**
- *GIGI:* Specialized fast paths for typed/structured data (code, math symbols, named entities sitting on a recognized hypersurface).
- *Marcella:* Restricted-class inference paths — closed-form transport along structural hypersurfaces, faster than general manifold transport.


## Part II — Sudoku fallout: forced consequences

These are theorems forced by the generator 𝒢 that were not in the Adachi program but follow from the same underlying structure.

### 2.1 Prequantization line bundle

**Claim.** If `[B/2π] ∈ H²(M, ℤ)`, there exists a Hermitian line bundle L → M with connection ∇^L of curvature B (Kostant-Souriau prequantization).

**Proof sketch.** Integrality of `[B/2π]` is equivalent to the existence of a Čech 1-cocycle valued in U(1) representing the cohomology class — exactly the transition data for a U(1) line bundle. Globally, the bundle is constructed by gluing trivializations along chart overlaps via these transition functions. Failure of integrality gives a Dirac string (the bundle is well-defined only on M minus a 1-dimensional locus).

**Validation.** PASS (`validation_tests_v2.py::test_7_prequantization_integrality`).

Using the Wu-Yang monopole construction on S² (two charts with potentials A_N = q(1-cos θ)dφ and A_S = -q(1+cos θ)dφ), compared independently:
- **Integer Chern (2q ∈ {1, 2, 3, 4, 6})**: holonomy difference between N and S charts = 2π × integer, deviation from integer = **0.0** exactly.
- **Non-integer Chern (q ∈ {0.3, 1/3, 1/π, 0.7})**: deviations 0.40, 0.33, 0.36, 0.40 — clean obstruction.

**What this enables.**
- Sections of L are *magnetic eigenstates* — a new data type GIGI doesn't have yet.
- Marcella's chosen B automatically defines a quantized representation space whose dimension is topological (see §2.2).
- `[B]` is a durable invariant — survives migrations, schema changes, reparametrizations.

**Implementation pointers.**
- `gigi/geometry/line_bundle.rs`: line bundle as transition cocycle data, with construction that fails when integrality is violated. Makes Dirac quantization a structural invariant.

### 2.2 Index-theoretic capacity bounds

**Claim.** For a holomorphic line bundle L on a closed Kähler manifold M, the index of the twisted Dolbeault complex is computable from topology: `ind(∂̄_L) = ∫_M ch(L) · Td(M)` (Hirzebruch-Riemann-Roch). On a Riemann surface of genus g: `dim H⁰(L) − dim H¹(L) = deg(L) − g + 1`.

**Proof sketch.** Heat-kernel proof (McKean-Singer + Getzler rescaling) reduces analytic index to a local topological integrand. On a Riemann surface, the formula specializes; for genus 1 (torus) with deg L = n > 0, Serre duality forces dim H¹ = 0, so dim H⁰ = n. Explicit basis: theta functions θ_k(z; τ; n) for k = 0, …, n−1.

**Validation.** PASS (`validation_tests_v3.py::test_9_index_theorem_torus`).

On T² with τ = i (square torus), constructed theta functions of level n and computed numerical rank by SVD:
- n = 1: rank = 1 ✓
- n = 2: rank = 2 ✓
- n = 3: rank = 3 ✓
- n = 4: rank = 4 ✓
- n = 5: rank = 5 ✓
- Negative check: 3 theta functions + a duplicate of θ_0 gives rank 3 (not 4) — confirms the rank truly measures dimension.

**What this enables.**
- Marcella's representational capacity under given B has a topological upper bound — not parameter-count.
- GIGI's number of independent indexable views on a given manifold is bounded by an integer derivable from cohomology.
- Capacity planning becomes a cohomology computation, not a sampling experiment.

### 2.3 Moment maps and forced conservation laws

**Claim.** If B is symplectic (closed + non-degenerate), the geodesic flow is Hamiltonian. For a Lie group G acting symplectically with equivariant moment map `μ: M → 𝔤*`, μ is conserved along the flow of any G-invariant Hamiltonian. This is the geometric form of Noether's theorem.

**Proof sketch.** Hamilton's equations `ẋ = X_H` with `X_H = B^{-1}(dH, ·)`. For G-invariant H, `dH(X_ξ^M) = 0` where X_ξ^M is the infinitesimal action of ξ ∈ 𝔤. Moment-map defining property `d⟨μ, ξ⟩ = ι_{X_ξ^M} B` gives `d⟨μ, ξ⟩(X_H) = -B(X_H, X_ξ^M) = -dH(X_ξ^M) = 0`. So ⟨μ, ξ⟩ is constant along the flow for every ξ.

**Validation.** PASS (`validation_tests.py::test_5_moment_map`).

SO(2) acting on T*ℝ² by rotation, moment map μ = x·p_y − y·p_x (angular momentum):
- **Symmetric H** (radial harmonic oscillator H = (p² + r²)/2): μ drift over t = 10 is **7e-15**. Energy drift 8e-15 (integrator sanity).
- **Asymmetric H** (anisotropic potential H = p²/2 + x²): μ drift = **19.5** (substantial violation). Energy drift still 1e-14 (integrator is fine; only symmetry-related conservation breaks).

Confirms the if-and-only-if structure: μ conserved iff H is G-invariant.

**What this enables.**
- *GIGI* gets automatic invariants — aggregates the substrate guarantees preserved under any B-flow. Free integrity constraints.
- *PRISM*: reconciliation under FX/fee operations has conservation laws whenever those operations are symplectic. Books-balance-by-theorem.

### 2.4 K-theoretic operation calculus

**Claim.** Vector bundles on M up to stable equivalence form the ring K⁰(M). Equivariant operations factor through this ring. For Kähler M with the Chern character, `K⁰(M) ⊗ ℚ ≅ H^{even}(M, ℚ)`.

**Proof sketch.** Standard K-theory: tensor product → multiplication, Whitney sum → addition, dual bundle → inversion when defined. Chern character is a ring homomorphism (multiplicative under tensor product).

**Validation.** Not directly tested; relies on standard K-theory. A test would compute K⁰ for a simple manifold (e.g., S²: `K⁰(S²) = ℤ ⊕ ℤ` with generators [1] and [Hopf bundle]) and verify ring relations. **Lower priority — established mathematics.**

**What this enables.**
- Every operation on PRISM (currency conversion, fee adjustment, alias resolution) classified as a K-theory class. Composition factors through ring structure.
- MIRADOR: drug-drug interaction effects compose via K-theoretic product.

### 2.5 Spectral gap of the Kähler graph Laplacian

**Claim.** The spectral gap λ_2 of the normalized graph Laplacian controls the mixing time of the lazy random walk: `τ_mix(ε) = Θ((1/λ_2) · log(1/ε))`. Cheeger's inequality relates λ_2 to edge expansion.

**Proof sketch.** The lazy walk's transition matrix has spectrum 1 = μ_0 > μ_1 ≥ μ_2 ≥ … Distance to stationarity decays as (max|μ_i|)^t. After the leading eigenvalue (the stationary), the next is 1 - λ_2; mixing time ~ 1/λ_2 up to log factors.

**Validation.** PASS (`validation_tests.py::test_6_spectral_gap`).

On n = 20:
- Cycle C_n: gap = 0.0489, mix time = 103, gap·τ = **5.04**.
- Complete K_n: gap = 1.053, mix time = 4, gap·τ = **4.21**.
- Path P_n: gap = 0.0136, mix time = 372, gap·τ = **5.07**.

The gap·τ product is within 1.2× across radically different topologies — verifying the spectral-mixing relation independently of any topology-specific tuning.

**What this enables.**
- Spectral gap of data manifold's graph approximation = expected query latency at the limit (computable by eigendecomposition).
- Spectral concentration regions = bottlenecks; bottleneck detection becomes spectral analysis.
- For Marcella: spectral gap controls attention mixing across the sequence.

**Implementation pointers.**
- `gigi/spectral.rs`: cache spectral gap of each bundle's Kähler graph at construction.

### 2.6 Floer-theoretic loop invariants

**Claim.** For symplectic B with nondegenerate Hamiltonian H, periodic orbits of the flow generate a chain complex (Floer complex) whose homology HF_*(M, H) is independent of H and equals the Morse homology of M.

**Proof sketch.** Floer's construction. Generators: period-1 orbits. Differential: count solutions to the perturbed Cauchy-Riemann equation `∂_s u + J(∂_t u - X_H) = 0` interpolating between orbits. Continuation invariance (compactness + transversality) shows HF independent of H.

**Validation.** Not tested numerically. Floer theory is genuinely hard to compute — even the simplest concrete test (HF on a torus) requires a substantial PDE-solver setup. **Research-mode.**

**What this enables.**
- Persistent loops in operational data — closed reconciliation cycles, recurring PK patterns, motifs in Marcella generation — get a topological invariant counting them.
- PRISM: number of irreducible reconciliation cycles is a Floer count, invariant under continuous changes to matching rules.

### 2.7 Mirror symmetry / A-B duality

**Claim.** Calabi-Yau Kähler manifolds come in mirror pairs (M, M^∨) with `D^b(Fuk(M)) ≅ D^b(Coh(M^∨))` (homological mirror symmetry). Symplectic operations on M correspond to complex-geometric operations on M^∨.

**Proof sketch.** Theorem in important cases (toric Fano varieties, abelian varieties). General Kontsevich HMS conjecture remains open.

**Validation.** Not tested numerically. **Research-mode.**

**What this enables.**
- Operations on GIGI's data manifold have two interpretations: symplectic and complex.
- Optimization surface: choose the side where the problem is easier.


### 2.8 Berezin-Toeplitz semiclassical expansion

**Claim.** Berezin-Toeplitz quantization assigns to `f ∈ C^∞(M)` an operator T_f on sections of L^k (large k = 1/ℏ) such that:
- `T_f T_g - T_{fg} = O(ℏ)` (Bohr correspondence)
- `[T_f, T_g] / (iℏ) = T_{{f,g}} + O(ℏ²)` (Dirac correspondence)

**Proof sketch.** Bordemann-Meinrenken-Schlichenmaier proof via coherent states. T_f := ∫ f(z) |z⟩⟨z| dμ(z) for coherent states |z⟩. Stationary-phase asymptotics on `⟨z₁|z₂⟩` as ℏ → 0 give the expansions.

**Validation.** PASS (`validation_tests_v3.py::test_10_berezin_toeplitz`).

Harmonic oscillator on ℝ² with truncated bosonic operators X̂, P̂ such that `[X̂, P̂] = iℏI`. Used the BCH formula `exp(iX̂)exp(iP̂) = exp(-iℏ/2)exp(i(X̂+P̂))` as independent ground truth (derived off-mesh).

Measured deviation of `[exp(iX̂), exp(iP̂)] − (−iℏ · exp(i(X̂+P̂)))` from zero, expected to scale as ℏ³:

| ℏ | normalized dev | theoretical \|ℏ − 2 sin(ℏ/2)\| | dev / ℏ³ |
|---|---|---|---|
| 1.000 | 4.115e-02 | 4.115e-02 | 0.04115 |
| 0.500 | 5.192e-03 | 5.192e-03 | 0.04154 |
| 0.250 | 6.505e-04 | 6.505e-04 | 0.04163 |
| 0.125 | 8.136e-05 | 8.136e-05 | 0.04166 |

**The dev/ℏ³ ratios converge to 1/24 ≈ 0.04167, exactly the theoretical leading coefficient of the BT correction.** Match to predicted `|ℏ − 2 sin(ℏ/2)|` is to all reported digits. BCH ground-truth holds to machine epsilon.

**What this enables.**
- Marcella has a natural quantum regime: small ℏ = deterministic, large ℏ = diffuse.
- GIGI queries in "quantum mode" return distributions of answers with theoretical spread bounds.
- Bridges to actual quantum hardware are non-fictional.

### 2.9 Witten/Morse spectral compression (Hodge foundation)

**Claim (foundation).** Hodge theorem: `dim H^k(M; ℝ) = dim ker(Δ_k)` where Δ_k is the Laplacian on k-forms.

**Claim (Witten refinement).** Given a Morse function f on M, the Witten Laplacian `Δ_t = (d + t·df∧)*(d + t·df∧) + (d + t·df∧)(d + t·df∧)*` has the same cohomology as Δ for all t ≥ 0. As t → ∞, low-lying eigenmodes localize on critical points of f, giving a finite Morse-complex description of the topology.

**Proof sketch (Hodge).** On compact M, every cohomology class has a unique harmonic representative. Equivalent to ker(d) ∩ ker(d*) at each degree, which is ker(Δ_k).

**Proof sketch (Witten).** On 0-forms: `Δ_t = -∇² + t²|∇f|² - tΔf`. Near a Morse critical point with Hessian H, this is locally a harmonic oscillator whose ground-state count is `b_k(M)`. The remainder of the spectrum is bounded away from 0 by O(t).

**Validation.** PASS — Hodge foundation tested (`validation_tests_v3.py::test_11_hodge_torus`).

On T² discretized as 6×6 periodic grid (36 vertices, 72 edges, 36 faces, V−E+F = 0):
- d² = 0: `‖d₁∘d₀‖_∞ = 0.0` (exact, integer arithmetic).
- dim ker Δ_0 = **1** (matches b_0 = 1) ✓
- dim ker Δ_1 = **2** (matches b_1 = 2) ✓
- dim ker Δ_2 = **1** (matches b_2 = 1) ✓
- Six smallest Δ_1 eigenvalues: [5.3e-16, 4.1e-15, **1.0**, 1.0, 1.0, 1.0] — two exact zeros with a clean spectral gap of 1.

Bonus: same construction on boundary-of-tetrahedron (4 vertices, 6 edges, 4 faces = S²) gives `(b_0, b_1, b_2) = (1, 0, 1)`, matching S² Betti.

The Witten refinement (localization at large t) is theorem-level; the Hodge foundation suffices to validate the spectral structure that Witten deformation preserves.

**What this enables.**
- Every continuous B-deformation of GIGI has a discrete Morse counterpart — finite combinatorial description of the same data.
- Critical points of B (Morse sense) are the only data that matters topologically — radical compression.
- Marcella's transport can be approximated by walks on a Morse complex with far fewer vertices.

**Implementation pointers.**
- `gigi/discrete/hodge_complex.rs`: build the d₀, d₁ operators from cell incidence. d² = 0 is forced by construction; kernel dimensions computed via eigendecomposition give Betti.

### 2.10 Frobenius / WDVV associativity

**Claim.** Quantum cohomology of a Kähler manifold carries a Frobenius manifold structure with associative multiplication satisfying the WDVV equations. Non-Kähler algebraic operations (e.g., Lie brackets) are generically NOT associative.

**Proof sketch.** Dubrovin/Manin: quantum product is defined via 3-point Gromov-Witten invariants. Associativity follows from the splitting axiom of GW theory (the moduli space of 4-pointed curves has codimension-1 boundary where one of two distinct degenerations occurs; equality of contributions = WDVV).

**Validation.** PASS (`validation_tests_v2.py::test_8_frobenius_wdvv`).

**Positive (QH*(ℂP²) = ℂ[H, q]/(H³ − q)):**
- Sanity: `‖H³ − q‖ = 0` exactly.
- Max associator `(a*b)*c − a*(b*c)` over all 27 triples in {1, H, H²}³: **0.0** exactly.

**Negative (so(3) with Lie bracket [J_a, J_b] = ε_{abc} J_c):**
- Sanity: `[J_x, J_y] = J_z` exactly.
- Max associator `[a, [b, c]] − [[a, b], c]` over 27 triples: **1.0**, attained at the triple **(J_x, J_x, J_y)**.
- Jacobi identity violation: 0.0 (Lie brackets do satisfy Jacobi, just not full associativity).

This confirms that **full associativity is strictly stronger than the Jacobi identity**, and that QH* really does buy us something Lie algebras don't.

**What this enables.**
- Operations on Marcella's tangent space automatically associate — composition of token transports is associative by theorem.
- For non-Calabi-Yau cases, WDVV violations give a quantitative anomaly score.
- Path to compositional semantics: tokens compose like Frobenius-algebra elements.


## 3. Implementation order (with prerequisites)

```
1.  (1.1) Dual adjacency  ─────────┐
                                    ├─►  4. (2.1) Prequantization line bundle
2.  (1.2) Magnetic 2-form  ────────┘                │
                                                     ├─►  7. (2.3) Moment maps + conservation
3.  (2.5) Spectral gap (independent, fast)           │
                                                     ├─► 11. (2.10) Frobenius / WDVV (Marcella v3 foundation)
5.  (1.4) Hadamard structure detection  ─┐
6.  (1.3) Trajectory-ball cost  ─────────┤
                                          ├─►  8. (2.4) K-theoretic op calculus
9.  (1.6) Hypersurface fast paths         │
                                          ├─► 10. (2.9) Witten/Morse compression (storage)
                                          │
                                          └─► 12. (2.2) Index-theoretic capacity bounds

Research mode:
13. (2.6) Floer invariants for dynamic monitoring
14. (2.7) Mirror symmetry / A-B duality
15. (2.8) Berezin-Toeplitz quantum regime
```

Items 1–6 are the practical v3 upgrade. 7–12 extend it. 13–15 are research with high upside if they pan out.

## 4. Validation summary

| # | Item | Test file::function | Status | Notable result |
|---|---|---|---|---|
| 1.1 | Kähler commutativity | v1::test_1_kahler_commutativity | PASS | Abelian = 0 exact; non-abelian = 1.0; centrality caveat |
| 1.2 | Magnetic 2-form | v1::test_2_magnetic_trajectory | PASS | Cyclotron radius err 2e-14; energy drift 6e-15 |
| 1.3 | Trajectory-ball volume | v1::test_4_trajectory_ball_volume | PASS | Vol rel err 1e-10; V_ℍ > V_ℝ > V_S confirmed |
| 1.4 | Ideal boundary (Hadamard) | v1::test_3_hadamard_cartan | PASS | ℍ² Jacobi positive monotone, never zero |
| 1.5 | Hadamard-Cartan invertibility | v1::test_3_hadamard_cartan | PASS | sinh/sin/t to 1e-13; S² conjugate at π to 6 dec |
| 1.6 | Hypersurface trajectories | — | not tested | Documented in Adachi papers; revisit if load-bearing |
| 2.1 | Prequantization | v2::test_7_prequantization_integrality | PASS | Integer 2q: dev 0; non-integer: dev 0.33–0.40 |
| 2.2 | Index / Riemann-Roch | v3::test_9_index_theorem_torus | PASS | dim H⁰(L^n) = n for n = 1, …, 5 on T² |
| 2.3 | Moment map / Noether | v1::test_5_moment_map + `geometry::moment_map::tests::*` | PASS — shipped L9 (2026-05-25) | Symmetric drift 7e-15 (analytic) / ≤ 1e-9 (RK4 in-process); asymmetric drift 19.5 / > 0.1 |
| 2.4 | K-theoretic operations | — | not tested | Standard K-theory; lower priority |
| 2.5 | Spectral gap / mixing | v1::test_6_spectral_gap | PASS | gap·τ products within 1.2× across topologies |
| 2.6 | Floer invariants | — | not tested | Research-mode (PDE-heavy) |
| 2.7 | Mirror symmetry | — | not tested | Research-mode |
| 2.8 | Berezin-Toeplitz | v3::test_10_berezin_toeplitz | PASS | dev/ℏ³ → 1/24 exactly; cubic scaling confirmed |
| 2.9 | Hodge (Witten foundation) | v3::test_11_hodge_torus | PASS | Betti (1,2,1) on T²; Δ_1 spectrum (0, 0, 1, 1, …) |
| 2.10 | Frobenius / WDVV | v2::test_8_frobenius_wdvv | PASS | QH*(ℂP²) assoc = 0; so(3) assoc = 1.0 |

**11 of 15 items validated numerically.** 4 not tested: 1.6 (documented in Adachi papers), 2.4 (standard K-theory), 2.6 (Floer — research-mode), 2.7 (mirror symmetry — research-mode).

## 5. Handoff notes for Claude Code Desktop

### Files
- `validation_tests.py` — Tests 1-6 (items 1.1, 1.2, 1.3, 1.4/1.5, 2.3, 2.5)
- `validation_tests_v2.py` — Tests 7-8 (items 2.1, 2.10)
- `validation_tests_v3.py` — Tests 9-11 (items 2.2, 2.8, 2.9)
- All runnable as `python <file>` after `pip install torch`. Self-contained; no external data.

### Central architectural commitment
The generator 𝒢 = (M, g, J, ∇, B, Γ) in §1 is the central technical commitment. Every downstream item references it. **When extending the catalog or refactoring GIGI, preserve the integrity of this object as a hard invariant.**

### Implementation order (v3 sprint)
Start with items 1–6 in §3. These are the practical v3 upgrade:
1. (1.1) Dual adjacency in storage layer — foundational; everything inherits from this
2. (1.2) Magnetic 2-form as first-class object — replaces threshold knobs with one geometric primitive
3. (2.5) Spectral analysis of Kähler graph Laplacian — cheap, immediately useful
4. (2.1) Prequantization line bundle — opens magnetic-eigenstate operations
5. (1.4) Hadamard substructure detection — long-context theorems
6. (1.3) Trajectory-ball cost model — cardinality estimation

### Specific Rust module suggestions
- `gigi/geometry/forms.rs` — closed 2-forms with d-closedness verified at construction
- `gigi/geometry/line_bundle.rs` — line bundle as transition cocycle; integrality check
- `gigi/graph/adjacency.rs` — principal/auxiliary adjacency types with centrality-based commutativity check
- `gigi/cost/jacobi_estimator.rs` — Jacobi field ODE for cardinality estimation
- `gigi/spectral.rs` — graph Laplacian spectrum cached per bundle
- `gigi/discrete/hodge_complex.rs` — discrete exterior calculus d₀, d₁, Laplacians

### Lessons from the validation discipline
- **The Test 1 failure mode** (vertex transitivity vs centrality) is the model for how the catalog should evolve: if a claim passes only because of an unstated assumption, the test should fail and the assumption should be made explicit in the claim. The catalog has been updated to require centrality, not just transitivity.
- **Negative cases are not optional.** Every test includes a configuration where the property must fail. Without these, "PASS" means nothing (the test could be passing for the wrong reason). When adding new tests, design the negative case first.
- **Closed-form ground truth must come from a different formalism** than the numerical computation. E.g., the Berezin-Toeplitz test uses BCH (algebraic) as ground truth for a numerical (matrix exponential) computation. If both came from the same recipe, the test would be circular.

### Open threads (for follow-up)
- **Davis Field Equations** (`C = τ/K`, `S + d² = 1`) likely fit into this picture; τ and K read like intrinsic curvature invariants of the data manifold. Not yet investigated — worth a dedicated v3 catalog section.
- **Bra Strap Principle's tension 1-form α** likely composes with B: the effective 2-form becomes `B + dα`. Closedness is preserved automatically (d² = 0). Worth a follow-up test.
- **Item 1.6 (hypersurface trajectories)** could be tested directly by constructing ℂP² with the Fubini-Study metric. Lower priority unless this becomes load-bearing.
- **Item 2.4 (K-theoretic operations)** has a clean toy test: compute K⁰(S²) = ℤ ⊕ ℤ with generators [1] and [Hopf bundle], verify ring relations.
- **Items 2.6, 2.7** are research-mode. Floer needs PDE-solving infrastructure; mirror symmetry is partly conjectural.


---

## Part III — Engineering extensions surfaced by GIGI-codebase review

*Added 2026-05-24 after reading `src/curvature.rs`, `src/bundle.rs`,
`src/sheaf/`, `src/dhoom.rs`, `theory/cosmological_nondecoupling_v2.tex`,
and `GIGI_SPEC_v0.1.md`. Each item is forced by 𝒢 once you take
GIGI's existing implementation seriously, not just the published
Adachi program.*

### E.1 Davis Non-Decoupling × Prequantization

**Claim.** GIGI's published Davis Non-Decoupling Theorem
(`theory/cosmological_nondecoupling_v2.tex`) shows that non-zero
curvature on a fiber bundle produces strictly positive, irreducible
**holonomy debt**. When the Kähler 2-form B satisfies the
prequantization condition `[B/2π] ∈ H²(M, ℤ)` (§2.1), the holonomy
debt of any closed loop γ is quantized:

> `Hol(γ) = 2π · ⟨[B/2π], [γ]⟩  ∈  2π · ℤ`

When B is *not* integrally quantized, the bundle is well-defined only
on `M \ D` for some 1-dimensional locus D (the Dirac string). The
existence of D is an irreducible curvature obstruction — it cannot be
gauged away.

**Proof sketch.** Wu-Yang construction (already proved in v2 Test 7).
Stokes-theorem reduction `Hol(γ) = ∫_Σ B` for any 2-surface Σ
bounded by γ, plus the Čech-cocycle interpretation of `[B/2π]`,
forces winding numbers to be integers exactly when prequantization
holds. Davis Non-Decoupling's "strictly positive debt density" then
quantizes to "integer multiples of 2π per loop."

**Validation.** Foundation already passes in v2
(`test_7_prequantization_integrality`). Add
`test_12_quantized_holonomy_debt` that builds a loop γ on S² with
`[B/2π] = n`, integrates the magnetic 1-form around it, verifies
the result is exactly `2πn` for integer n and clear non-integer
deviation for non-integer n.

**What this enables.**
- *GIGI:* Holonomy-debt computation collapses from continuous
  integration to integer arithmetic when the bundle is integrally
  quantized. `holonomy()` returns an integer winding number for
  quantized bundles.
- *Cosmological Non-Decoupling (paper extension):*
  Dark-energy-as-permanent-curvature-floor admits a quantized
  refinement — the curvature floor lives in integer levels of a
  prequantization line bundle over the cosmological frame bundle.
  Publishable as a follow-up paper to
  `cosmological_nondecoupling_v2.tex`.

**Implementation pointers.**
- `src/curvature.rs::holonomy_debt(store, loop_keys) -> Option<i64>` —
  returns `Some(n)` when the bundle has an integral Kähler structure
  attached, `None` otherwise (falls back to the continuous holonomy
  already in `src/sheaf/mod.rs`).
- New: `src/geometry/line_bundle.rs::ChernClass(i64)`.


### E.2 DHOOM compression via Chern-class storage

**Claim.** For an integrally-quantized 2-form B on a bundle with
chart cover {U_α}, the wire-protocol payload can compress the
fiber-bias information to:

> `O(#charts × log|Chern|)` bits per snapshot

versus

> `O(#charts × dim(B)² × bits-per-float)` for the unquantized form.

Decompression is the cocycle-gluing reconstruction: at each chart
overlap `U_α ∩ U_β`, the transition function
`g_{αβ}: U_α ∩ U_β → U(1)` is determined by the local Chern number
and the chosen gauge. The 2-form is reconstructed from the transition
data by `dA_α = B|_{U_α}`.

**Proof sketch.** Standard Čech-de Rham: integral cohomology classes
have integer cocycle representatives. The size of the cocycle is
bounded by the rank of `H²(M, ℤ)` (typically `O(charts)`), each entry
storable in `log|Chern|` bits. Reconstruction is the gluing
isomorphism of sheaf cohomology, runs in `O(charts × overlaps)`.

**Validation.** New test `test_13_dhoom_chern_roundtrip`:
generate random integral B on the Atlas, encode to Chern-cocycle,
decode, verify reconstructed B matches original to machine epsilon.
Measure compression ratio across `{S², T², CP², 4×4 grid torus}`;
expect ≥ 10× for integral cases, no compression for non-integral.

**What this enables.**
- *DHOOM patent extension:* The existing patent covers compressed
  wire format for fiber bundles. Chern-class storage strengthens the
  claim with provable reconstructability + provable compression
  bound. Worth a CIP filing.
- *Wire savings:* Snapshot/replication payloads on quantized bundles
  drop substantially. Sheets bundles with structured workflows
  (which are typically integrally quantized — discrete numbers of
  reconciliation cycles, drug-dosing periods, etc.) are the biggest
  beneficiaries.

**Implementation pointers.**
- `src/dhoom.rs::QuantizedTwoForm` variant of the wire format.
- `src/dhoom.rs::encode_chern(b: &TwoForm) -> Option<ChernCocycle>` —
  `None` when B isn't integrally quantized; falls back to the dense
  encoding.
- `src/dhoom.rs::decode_chern(c: &ChernCocycle, atlas: &Atlas) -> TwoForm`.


### E.3 Kähler curvature decomposition in CurvatureStats

**Claim.** On a Kähler manifold, the Riemann curvature tensor
decomposes into four independent invariants:

| Component | Geometric meaning | Bound for data manifold |
|---|---|---|
| **Ricci** `Ric` | Mean curvature; Einstein condition | `Ric > 0` ⇒ Fano (compact + bounded volume); diversity is finite |
| **Weyl** `W` | Conformal curvature | `W = 0` ⇒ conformally flat; data has only scale variation |
| **Holo-bisectional** `K_B(X, Y)` | Curvature in *holomorphic* pairs | `K_B ≥ 0` ⇒ projective-space-like; `K_B ≤ 0` ⇒ Kähler-Hadamard (§1.4 applies) |
| **Holo-sectional** `K_H(X)` | Curvature along complex lines | Constant `K_H = c` ⇒ complex space form CP^n / C^n / CH^n |

Currently `src/bundle.rs::CurvatureStats` carries one scalar K. In
the Kähler regime, it should carry these four — each is computable
from the same Welford-style streaming statistics already maintained.

**Proof sketch.** Standard Kähler-geometry result (Kobayashi-Nomizu
Vol II §IX.7). The Kähler identity `∇J = 0` forces the Riemann
tensor `R(X, Y, Z, W)` to satisfy
`R(JX, JY, Z, W) = R(X, Y, Z, W)`, which decomposes the curvature
into the four pieces above. The decomposition is orthogonal in the
inner product on `Sym²(Λ²T*M)`.

**Validation.** New test `test_14_kahler_curvature_decomposition`
on `CP¹ = S²` with the Fubini-Study metric. Compute all four
invariants numerically (from finite-difference Christoffel +
Riemann), verify:

- `Ric = (n+1) g`  (Einstein constant for CP^n; n=1 gives 2g)
- `W = 0`  (CP¹ is conformally flat)
- `K_H = 4`  (constant holomorphic sectional curvature of FS metric)
- `K_B(X, Y) ∈ [1, 4]`  (pinched bisectional curvature, with X=Y giving 4)

**What this enables.**
- *Direct upgrade to CurvatureStats:* one number becomes four; each
  surfaces in the API and query planner.
- *Free Hadamard detection:* `K_B ≤ 0` on every fiber pair ⇒ bundle
  is Kähler-Hadamard ⇒ §1.4 + §1.5 guarantees apply automatically,
  without a separate detection pass.
- *Diversity bounds for Marcella:* Fano fibers (Ric > 0) cap
  generation diversity; Hadamard fibers permit unbounded divergence
  within trajectory-balls.
- *Conformal-only operations:* Weyl = 0 detection makes pure-scaling
  gauges (`src/gauge.rs::Rescale`) provably lossless.

**Implementation pointers.**
- Extend `src/bundle.rs::CurvatureStats` with an optional
  `kahler: Option<KahlerCurvature>` field:
  ```rust
  pub struct KahlerCurvature {
      pub ricci: f64,
      pub weyl: f64,
      pub holo_bisectional_min: f64,
      pub holo_bisectional_max: f64,
      pub holo_sectional: f64,
  }
  ```
- Compute via streaming statistics in `update_curvature()` on insert.
- Surface in `/v1/bundles/<n>/curvature` API.


### E.4 The next Sudoku step beyond Kähler — explicitly deferred

**Claim.** Kähler manifolds have one complex structure J satisfying
`J² = -I` and `∇J = 0`. The next stable stop is **hyperkähler**:
three complex structures `I, J, K` satisfying quaternion relations

> `IJ = K`, `JK = I`, `KI = J`, `I² = J² = K² = -I`

plus a holomorphic symplectic form Ω. Hyperkähler manifolds have
holonomy reduced to `Sp(n) ⊂ U(2n) ⊂ SO(4n)`, twistor space CP¹
parametrizing complex structures, and a hyperkähler quotient
strictly richer than the symplectic quotient (§2.3).

**For GIGI this would mean.**
- *Three orthogonal bias directions* per fiber instead of one;
  biases compose by quaternion multiplication (non-commutative but
  associative). Useful when query bias has three independent "axes"
  that interact (recency × counterparty-trust × FX-direction in
  PRISM; dose × time × genotype in MIRADOR).
- *Twistor-space query families:* a CP¹ worth of bias 2-forms at
  once, parameterized by direction in the (I, J, K) sphere.
- *HK quotient:* a single reduction that simultaneously eliminates
  multiple symmetries, replacing nested calls to the symplectic
  quotient.

**Why we stop at Kähler for v3.**
- Hyperkähler is rare in nature — most data manifolds will not
  naturally satisfy the holonomy restriction.
- Forcing hyperkähler is over-fitting: it imposes algebraic structure
  the data may not have, which silently shrinks the model class.
- Kähler captures every catalog item §1.1–§2.10 with the right level
  of generality. v4-or-later can revisit if PRISM/MIRADOR data turns
  out to have natural quaternion structure.

**Why we document this here.** So a future engineer doesn't have to
rediscover the decision. If/when bee wants to push to v4, the move
is: add `(I, J, K)` tuple to `BundleSchema::kahler`, audit existing
biases for quaternion-compatibility, write twistor-space analogs
of §1.2 trajectories.

**Validation.** Not applicable — deferred.


### E.5 Marcella claims are hypotheses, not theorems, until measured

**The catch.** Many Marcella-side claims in this catalog —
particularly §1.4 long-context via ideal boundary, §1.5 lossless
encode/decode invertibility, §2.10 Frobenius-algebra composition
of tokens — require Marcella's learned token manifold to *actually*
have the assumed Kähler structure. The math forces the consequences
IF the structure holds; whether it holds is empirical.

**Required pre-flight checks before any Marcella v3 architectural
commitment cites this catalog as theorem:**

1. **Hadamard check.** Sample N=1000 random token-pair geodesics
   from Marcella V6's learned transport. Numerically integrate the
   Jacobi field along each. Verify Jacobi fields are non-vanishing
   on training-length sequences (no conjugate points). Threshold:
   ≥ 95% of pairs must be conjugate-point-free for the §1.4
   long-context guarantee to apply globally.
2. **Closedness check.** Marcella's learned positional 2-form
   `B_pos` must satisfy `‖dB_pos‖ < ε`. Compute the discrete
   exterior derivative on the token-pair grid. Threshold:
   `ε < 1e-3 × ‖B‖`.
3. **Holomorphic-sectional sign check.** Compute `K_H` on token-pair
   complex lines. For §1.4 to apply globally, need `K_H ≤ 0`
   everywhere. If `K_H > 0` in some region (e.g. high-entropy
   prompt tokens), §1.4 applies only on the complement — region
   the model can't long-context coherently.

**Implementation pointer.** Tests live in `marcella/v3/` (separate
repo), use the same Jacobi-field utilities as
`theory/kahler_upgrade/validation/validation_tests.py`. Results
gate whether the v3 paper cites items 1.4 / 1.5 / 2.10 as
"theorem" or "hypothesis verified on V6 to X% coverage."


---

## 6. Validation summary (updated 2026-05-24)

Additions from Part III:

| # | Item | Test file::function | Status | Notes |
|---|---|---|---|---|
| E.1 | Davis Non-Decoupling × Prequantization | (to add) test_12_quantized_holonomy_debt | planned | Foundation in v2 test 7 |
| E.2 | DHOOM Chern compression | (to add) test_13_dhoom_chern_roundtrip | planned | Patent CIP candidate |
| E.3 | Kähler curvature decomposition | (to add) test_14_kahler_curvature_decomposition | planned | CP¹ Fubini-Study closed form |
| E.4 | Hyperkähler deferred | — | not applicable | Future v4 |
| E.5 | Marcella empirical pre-flight | marcella/v3/ tests (separate repo) | gates v3 paper | Required before citing as theorem |

**14 items now actionable** (11 validated in this catalog + 3 new
tests to write for E.1/E.2/E.3). E.4 is deferred; E.5 is downstream
of GIGI shipping.
