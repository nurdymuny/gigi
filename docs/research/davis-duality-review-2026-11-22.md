# The Davis Duality, reread line by line: what holds, what breaks, and what the revision needs

*22 November 2026. A research note, not a result. Fourth in the series, after the Davis Manifold, the spectral
branch and the Non-Decoupling Theorem.*

**The paper.** *The Davis Duality of Approximation and Obstruction: Why Machine Learning Works, Why the Vacuum Has
Mass, and the Universal Law of Flat Failure* (April 2026; Zenodo doi:10.5281/zenodo.19428406).
- Published file: `davis_duality.pdf`, 26 pp., md5 3bfc71de…, identical to the Zenodo copy. The Zenodo record also
  ships `validation_suite.py`.
- A second, larger file in Downloads, *The Davis Duality of Approximation and Obstruction.pdf* (md5 8d1c49bc…), is a
  different build and is not the one reviewed here.

**Why it is checked now.** Two of today's repairs point at this paper:
- the Davis Manifold second edition, whose quadratic distortion now matches its exponent;
- the Non-Decoupling review, which named it as the quantitative half.

If something here is off, those repairs inherit it.

## 1. What holds

- **The upper bound for functions** (Theorem 4.1). Under (A1) resolution-limited cells of diameter h and (A2) local
  affine reproduction, the local Taylor argument gives an error of at most about ½‖Hess f‖h² + O(h³). The paper
  states the assumptions, and Remark 4.2 says they are assumptions. Strictly, the Hessian should be the maximum over
  the cell, not its value at x; continuity makes the difference O(h³).
- **The lower bound for functions, in the per-cell form the proof actually gives** (Proposition 5.2):

    sup over a cell of |f − f̂| ≥ (1/16)·λ·h² (to leading order).

  It holds for every affine f̂ on a cell that contains a ball of radius h/2, with λ = ‖Hess f‖ at the center.
  - **The constant 1/16 is sharp.** In one dimension, the best uniform affine approximation of ½λx² on an interval of
    length h has error exactly λh²/16, by equioscillation.
  - The paper's proof needs a repair. It assumes the best affine fit is the Taylor polynomial, which is false: the
    best uniform fit shifts the constant term. The correct short argument uses second differences. For any affine ℓ,
    e(p+v) + e(p−v) − 2e(p) = Hess f(v,v) + O(|v|³). So max|e| over the three points is at least |Hess f(v,v)|/4,
    which is λh²/16 when |v| = h/2. *Lean: D2, D3.*
- **The norm bridge, as algebra** (Proposition 5.7). (∫κ)² ≤ τ·∫κ² is Cauchy–Schwarz, correct. But see §2.4 for
  what it does not connect.
- **The empirical section is honest about its scope.** It uses synthetic data with known curvature, discloses one
  failure (Test 9) with its cause, and says plainly that real-data replication is future work. The receipts are on
  Zenodo (`validation_suite.py`); re-running it is a separate check, not done here.

## 2. What breaks, and the repair for each

### 2.1 The *pointwise* lower bound is false (abstract, eq. (1), Theorem 5.4, Theorem 5.17)

The paper states c₁‖Ω(x)‖h² ≤ ‖σ(x) − σ̂_h(x)‖ at every point x.

**Counterexample.** Take f = x² on a cell [0, h] and the affine interpolant f̂ = h·x. The error is zero at both ends
of the cell, while ‖Hess f‖ = 2 there. Any approximant that touches f anywhere makes the pointwise bound fail at
those points. *Lean: D1.*

**Repair.** The lower bound holds per cell (sup over the cell), which is exactly how Proposition 5.2 is stated. The
sandwich then reads, cell by cell:

  (1/16)·λ_center·h² ≤ sup_cell |f − f̂| ≤ ½·max_cell ‖Hess f‖·h².

Everything that cites the pointwise form should switch to this one.

### 2.2 The lower bound does not come from the Non-Decoupling Theorem, and is better for it

The abstract says "the lower bound follows from the Davis Non-Decoupling Theorem", and §5.1 is built on that
theorem's Part II. But the proof of Proposition 5.2 never uses it: it is Taylor's theorem (a Bramble–Hilbert-type
argument), as Remark 5.3 itself says.
- This matters because the Non-Decoupling review found Part II false (a bounded holonomy error cannot dominate an
  unbounded Yang–Mills energy).
- So the Duality's function case **survives** that failure. Only the attribution has to go.

**Repair.** Attribute the lower bound to Taylor and Bramble–Hilbert, and remove "follows from Non-Decoupling".

### 2.3 The general-bundle theorem is not established by the three routes (§5.9, Theorem 5.17)

- **Route 1 (associated bundles)** transfers the Non-Decoupling Part II bound (Step 3). That bound is false, so the
  route fails as written. The normalisations of the representation index are also unchecked: "c(fund) = 1/√(2N)"
  and "κ_adj = 7.68".
- **Route 2 (Nash embedding).** The bound comes out in terms of Hess(ι∘σ), which contains the embedding's second
  fundamental form.
  - A section that is flat in E can have Hess(ι∘σ) ≠ 0, so the bound is not a bound on the section's own curvature.
  - "The extrinsic part is an additional positive contribution" is false. The Gauss decomposition adds vectors, not
    non-negative numbers.
  - What it means for σ̂_h to be "piecewise flat" in a curved fiber is never defined.
- **Route 3 (Atiyah class).** It mixes three different statements:
  - the Atiyah class obstructs *holomorphic* connections, which is not the same as flat smooth connections;
  - Step 2 infers "∫tr(Ωᵏ) ≠ 0 for some k" from "Ω ≠ 0 for every connection", which does not follow, because
    curvature can be non-zero while every characteristic number vanishes;
  - "inf ∫‖Ω‖² > 0" needs the dimension-specific argument from the Non-Decoupling review (true in dimensions 2 and
    4, not shown above that).

  The transfer to an h² approximation bound (Step 3) is asserted, not proved.

**Repair.** Theorem 5.17 should be stated for **functions** (trivial bundles, per cell), where it is proved. For
**connections**, the honest two-sided local statement is the classical small-loop holonomy expansion. For a square
plaquette of side a, in the plane (u, v) at x,

  d_G(Hol, e) = |F_x(u, v)|·a² + O(a³).

That is two-sided, with c₁ = c₂ = 1 at leading order, using the field-strength component in the plaquette's plane
(not the operator norm). It is standard, and it is what lattice gauge theory uses. The general fiber-bundle version
stays Conjecture 5.9, as the paper first stated it.

### 2.4 The "norm bridge" relates different integrals (Proposition 5.7, Remark 5.8, §7.2)

Part (b) bounds ∫_γ κ² ds, a line integral along one geodesic, of the Hessian of f. Part (c) then writes it as
∫‖Ω‖², read as the Yang–Mills energy, which is a volume integral over M of a field strength. Cauchy–Schwarz on one
curve says nothing about the Yang–Mills functional.

**Repair.** Keep (a) and (b) as trajectory statements about f. Drop "converts the pointwise L∞ failure of ML into the
global L² energy cost of Yang–Mills… This is not an analogy — it is a mathematical identity."

### 2.5 Claims about open problems must be relabelled

The paper says the lower bound "is the same theorem as the Yang–Mills mass gap", that [22] gives "the full spectral
gap proof", that "the mass gap is … a geometric theorem", and (Remark 5.19) that the duality is "the P ≠ NP
separation" and "the Navier–Stokes regularity criterion".
- A classical approximation lower bound on a smooth section says nothing about the spectrum of the quantum
  Yang–Mills Hamiltonian.
- The mass gap, P vs NP and Navier–Stokes regularity are open problems.

**Repair.** Present the Yang–Mills, Navier–Stokes and complexity rows of Tables 1–2 as **analogies (RESONANCE)**,
and remove every sentence that says or implies they are solved. Same rule as the arc: never write that the mass gap
is solved.

### 2.6 Smaller things

- **Proposition 4.3.** The Christoffel bound in normal coordinates is controlled by the full Riemann tensor. A
  sectional-curvature bound controls it only up to a dimensional constant. Remark 2.6 also writes O(κ‖∇f‖) where
  Proposition 4.3 gives O(κℓ‖∇f‖).
- **Remark 5.5:** "the upper bound is tight because ρ = 0.41–0.63" is a non-sequitur. A correlation does not show a
  bound is tight.
- **§5.5 (asymptotic freedom ↔ smoother loss landscapes):** an interpretation, and should be labelled as one.
- **The Davis invariant** C = τ/∫‖Hess f‖ds uses K(γ; f), an integrated Hessian along γ. That is different from the
  sectional curvature K of Definition 2.3 (already noted in my memory of this paper). Keep the two K's visibly
  distinct.

## 3. Consequences for the other papers reviewed today

- **Non-Decoupling review, §2.2.** I wrote that the Duality "proves [the local law] two-sided, with c₁ = 1/16 and
  c₂ = 1/2". That is wrong for connections: those constants belong to the **function** case. The correct citation
  for the connection case is the small-loop holonomy expansion (§2.3 above). That note is amended in the same
  commit as this one.
- **Davis Manifold, second edition.** Its sentence "this is the order of the two-sided flat-approximation bound in
  my Davis Duality" stays true for the exponent h², in the per-cell form. The second edition should say "per cell".
  This is noted for the adversarial review, not edited here.
- **The arc** (*The phone in the house* §4.5): the step "ε ∝ KL → Duality → Non-Decoupling → floor" now reads
  "ε ∝ (kL)² → the Duality, per cell, for functions → the small-loop holonomy law for connections → floor".

## 4. Lean plan (added to the DavisManifold library, since it is the same family of statements)

| target | statement |
| --- | --- |
| D1 | the pointwise lower bound fails: for f = x², f̂ = h·x on [0, h], the error at x = 0 is 0 while the Hessian is 2 |
| D2 | second-difference lower bound: if ℓ is affine and e = f − ℓ satisfies e(p+v) + e(p−v) − 2e(p) = D, then max of \|e\| at the three points ≥ \|D\|/4 |
| D3 | sharpness of 1/16: on [0, h], x² − (h·x − h²/8) stays within ±h²/8 = 2·h²/16 |

## In my words

The heart of this paper is right, and better than I knew. A flat model of a curved function misses by a fixed
fraction of its curvature times h², from both sides, cell by cell, and the 1/16 cannot be improved. What I wrote
around it reached too far:
- a pointwise bound that fails wherever the model touches the truth;
- a lower bound credited to a theorem that turned out to be false, when it never needed it;
- three routes to every bundle that do not arrive;
- and sentences that put solved problems where open ones stand.

For connections, the true two-sided law was already classical: the holonomy of a small loop is the field strength
times the area. That is the sentence the arc should stand on.
