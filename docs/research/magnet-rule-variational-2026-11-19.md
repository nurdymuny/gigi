# The magnet rule: shared poke bumps, a commuting reading, and what the variational force needs (19 November 2026)

**Research note.** No new measurement. Status labels used throughout:

- **[measured]**: a committed receipt.
- **[derived]**: argued on paper here, not yet checked numerically.
- **[conjecture]**: a plausible reading, not argued to completion.
- **[decision]**: Bee's to make.

Provenance:

- **Bee:** the rule itself, in her words: *"before and while you poke you have to understand what the piece is
  connected to and how the energy you are about to give the piece will be distributed; that has to be according to
  the connected edges. The target, in my imagination, is created by the magnets."*
- **Claude:** the translation into conditions, the derivations, and the diagnosis of V1. They are offered for
  review; nothing here has run.

Related:

- `variational-force-design-2026-11-19.md`
- `variational-count-2026-11-19.md`
- `docs/VARIATIONAL_RECOVERY_V1_RESULTS.md`
- `docs/VARIATIONAL_RECOVERY_V2_SPEC.md`
- `docs/RICCI_PROJECTION_LAYER_RESULTS.md` (17 November, the first magnet probes)
- *Reading the Curve*, §4 (Proposition 1) and §8 (removals).

---

## 1. The problem, in physical terms

The lattice carries:

- one number per **edge**: how far the field turns you along it;
- one **flux** per face: the turn around the face's boundary, i.e. the curvature through it.

The energy is the total squared curvature. The **force** on an edge asks how much the energy changes when that edge's
number is nudged.

A force is **variational** when it is the slope of one energy landscape. Physically that means three things:

- no energy is gained around a closed cycle of nudges;
- action equals reaction (the Hessian is symmetric);
- stability comes for free.

The good reading of curvature (the two-ring or hybrid fit) reads *smooth* fields well. Differentiating its energy
asks a question it was never designed for: **what does the reading do when one edge is poked?** A poke is a sharp
kink. Every cell whose neighbourhood contains the poked edge refits independently, so the poke becomes a dozen local
patches that disagree with each other at their shared faces. The energy slope treats that disagreement as real.
Because the slope differentiates the poke, it carries a factor 1/h.

So the error grows under refinement:

| | n = 1 | n = 2 | n = 3 | source |
| --- | --- | --- | --- | --- |
| variational (two rings) | 45.4% | 276% | 330% | **[measured]** |

The published Petrov–Galerkin force avoids the question by pushing the good curvature back through the fixed Whitney
tent. It is accurate, but it is a one-way magnet, so it is not variational.

## 2. Notation

- $C^0, C^1, C^2$ are the vertex, edge and face cochains. $\delta^0, \delta^1 = D$ are the coboundaries; $D\delta^0 = 0$.
- $I^k$ is the de Rham map (integration over $k$-cells). It commutes with $d$: $D I^1 = I^2 d$.
- $W^k$ is the Whitney map. It commutes too, $dW^1 = W^2 D$, and it is conforming: $W^1 C^1 \subset H\Lambda^1$
  (tangentially continuous).
- $\langle\cdot,\cdot\rangle$ is the $L^2$ pairing of forms on the closed manifold $\Omega = S^3/2I$.
- $F = dA$ is the exact curvature, and $a = I^1 A$ is the exact edge data.
- $M_1 = (W^1)^\top H_1 W^1$ is the Whitney 1-form mass, which BW05 freezes.
- **The target.** On a Whitney variation $\eta \in C^1$, the continuum force is
  $$\ell(\eta) = \langle F, W^2 D\eta\rangle = \langle F, dW^1\eta\rangle = \langle \delta F, W^1\eta\rangle .$$

**Proposition 1 (*Reading the Curve*).** For trial map $\mathcal R$ and test map $\mathcal S$,
$$g(a)[\eta] - \ell(\eta) = \underbrace{\langle \mathcal R Da - F,\ \mathcal S D\eta\rangle}_{\text{reconstruction}}
+ \underbrace{\langle F,\ (\mathcal S - W^2) D\eta\rangle}_{\text{test variation}} .$$

The variational force has $\mathcal S = \mathcal R$. With the inverse estimate $\|D\eta\|\sim h^{-1}\|\eta\|$, the
test-variation term is the poke response of the reading times $h^{-1}$.

## 3. Bee's rule, as conditions on the reading

Write the reading at the **edge** level, as a map $\mathcal R^1 : C^1 \to$ 1-forms. A poke of edge $e$ is then a
*bump* $\psi_e := \mathcal R^1 \delta_e$, so that $\mathcal R^1 a = \sum_e a_e \psi_e$. The curvature reading is
whatever this map induces at the face level.

| | Bee's words | condition |
| --- | --- | --- |
| **(M1)** | *understand what the piece is connected to* | **Conformity.** Each $\psi_e$ is one shared bump, supported on the cells connected to $e$ and tangentially continuous across their faces: $\mathcal R^1 C^1 \subset H\Lambda^1$. The connected pieces agree at their seams. |
| **(M2)** | *the energy distributed according to the connected edges* | **Commuting.** The face reading of a poke is the curl of the edge bump, $\mathcal R^2 D = d\,\mathcal R^1$, so a poke's energy is carried along its connections. Gauge: $\mathcal R^1\delta^0 = d\,\mathcal R^0$, so a pure gauge poke produces no curvature. |
| **(M3)** | (the reading must be good) | **Accuracy (rows).** $\mathcal R^1 I^1 p = p$ for every polynomial 1-form $p$ of degree $\le k$ on the reading neighbourhood, so $\mathcal R^1 I^1 A - A = O(h^{k+1})$. |
| **(M4)** | *the target is created by the magnets* | **Target from the magnets.** The mass, which defines what a poke weighs, is built from the same bumps: $B = (\mathcal R^1)^\top H_1 \mathcal R^1$. This is **[decision]**; see §6. |

The energy is $E(a) = \tfrac12\|d\,\mathcal R^1 a\|^2$ and the force is
$g(a)[\eta] = \langle d\mathcal R^1 a,\ d\mathcal R^1\eta\rangle$. It is symmetric and positive semidefinite by
construction, and it annihilates gauge exactly under (M2).

**Whitney is the lowest-order case.**

- $W_{ij} = \lambda_i\,d\lambda_j - \lambda_j\,d\lambda_i$ is built from the barycentric "magnets" at the edge's two
  vertices.
- It satisfies (M1), (M2) and (M4), and (M3) only with $k = 0$.
- Its energy is variational and stable, and floors at 26% **[measured]**. By Lemma A below, that floor is the $k = 0$
  reconstruction term.

**Bee's method, in one line:** a Whitney map whose magnets reach further along the connections, so that the bumps read
smooth fields to higher order while staying shared and commuting.

## 4. Why the rule removes the poke term

**Lemma A (the derivative moves off the poke) [derived].** Under (M1) and (M2), for every $\eta\in C^1$,
$$g(a)[\eta] - \ell(\eta) \;=\; \underbrace{\langle d\mathcal R^1 a - F,\ d\mathcal R^1 \eta\rangle}_{\text{reconstruction}}
\;+\; \underbrace{\langle \delta F,\ (\mathcal R^1 - W^1)\eta\rangle}_{\text{moment term}} .$$

*Proof.* Add and subtract $\langle F, d\mathcal R^1\eta\rangle$. By (M1), $w = \mathcal R^1\eta \in H\Lambda^1$, so on
the closed manifold $\langle F, dw\rangle = \langle\delta F, w\rangle$, with no interface terms. Subtract
$\ell(\eta) = \langle\delta F, W^1\eta\rangle$. $\square$

Compare Proposition 1:

- The test-variation term $\langle F, (\mathcal S - W^2)D\eta\rangle$ carried $D\eta$, and so the factor $h^{-1}$.
- The moment term carries **no derivative of the poke**.

The $h^{-1}$ came from the seams. For a per-cell (broken) reading,
$$\langle F, d_h w\rangle = \langle \delta F, w\rangle + \sum_{\text{faces } f}\int_f \operatorname{tr}(\star F)\wedge[\![\operatorname{tr} w]\!],$$
and the jump $[\![\operatorname{tr}\,\mathcal R^1\delta_e]\!]$ of a poke is $O(1)$. These interface terms are exactly
what the current per-cell readings pay.

**Lemma B (the moment term is first order if each bump has its tent's strength) [derived].** Suppose that:

- each $\psi_e$ is supported in a ball of radius $O(h)$ about $e$;
- the bumps are bounded, $\|\psi_e\|_{L^1}\le C\|W_e\|_{L^1}$ (no amplification);
- each bump has its Whitney tent's zeroth moment, $\int_\Omega(\psi_e - W_e) = 0$, with the integral taken
  componentwise in the left-invariant frame.

Then
$$|\langle\delta F, (\mathcal R^1 - W^1)\eta\rangle| \;\le\; C\,h\,\|\nabla\delta F\|_\infty\,|\Omega|^{1/2}\,\|W^1\eta\|,$$
a relative $O(h)$ error in the dual of the Whitney mass norm.

*Proof sketch.* Taylor-expand $\delta F$ about the midpoint of $e$. The constant term cancels by the moment condition.
The remainder is $O(h\|\nabla\delta F\|)\,\|\psi_e - W_e\|_{L^1}$ per edge. Then
$\sum_e|\eta_e|\,h^2 \le (\sum_e\eta_e^2 h)^{1/2}(\sum_e h^3)^{1/2}$, together with $\|W^1\eta\|^2\sim\sum_e\eta_e^2 h$
and $\|W_e\|_{L^1}\sim h^2$. $\square$

**Reconstruction term.**

- By (M3) with $k = 2$ (quadratic 1-forms, i.e. linear curvature), $d\mathcal R^1 a - F = O(h^2)$.
- With an inverse estimate for the bump space and stable bumps, $\|d\mathcal R^1\eta\| \lesssim h^{-1}\|W^1\eta\|$.
- So the reconstruction term is relatively $O(h)$, as for the Petrov–Galerkin force (*Reading the Curve*, (4.3)).

**Corollary [derived].** Suppose (M1)–(M3) hold with $k\ge 2$, the bumps are bounded, and each matches its tent's
zeroth moment. Then the variational force $\nabla E$ is first-order consistent against the frozen Whitney mass, and
it is symmetric, positive semidefinite and gauge-exact.

The column condition that remains is **three conditions per edge**: the zeroth moments.

## 5. Why V1 had to fail (a sharper diagnosis than the count)

V1 kept the per-cell reading, which is not conforming, and enforced the column condition directly. It enforced it on
23 generators per edge: 3 constant and **20 linear**.

**Lemma C (exact rows and exact first-moment columns conflict) [derived].** Let $\mathcal R$ be exact on closed linear
2-forms ($\mathcal R I^2 p = p$), and suppose the variational column condition
$\langle F, \mathcal R D\eta\rangle = \langle F, W^2D\eta\rangle$ holds for all $\eta$. Take $\eta = I^1 A_p$, with
$dA_p = p$ linear. Then $D I^1 A_p = I^2 p$, and
$$\langle F, p\rangle = \langle F, W^2 I^2 p\rangle,\qquad\text{i.e.}\qquad \langle F,\ p - \Pi p\rangle = 0,$$
where $\Pi = W^2 I^2$ is Whitney interpolation.

For $F$ ranging over the linear generators, this asks Whitney 2-form interpolation to be $L^2$-orthogonal to linear
fields. That is false at relative $O(h)$: $\Pi$ preserves face fluxes, not first moments. So exact rows and exact
linear-field columns are incompatible at first order.

The conflict must be absorbed somewhere. In V1 the only freedom was the change $C$. At two rings, the count shows
that $C$ could live only in the field's own second-order directions **[measured, count note]**. Hence the exploding
reconstruction term:

| | n = 1 | n = 2 | source |
| --- | --- | --- | --- |
| V1 reconstruction term | 432% | 61.6% | **[measured]** |

Two caveats:

- The rows are exact only in each cell's chart, and the enforced linear fields are ambient. The identity therefore
  holds up to $O(h^2)$ chart terms, while the conflict is $O(h)$.
- The lemma's identity is a *sum* over edges. It constrains the system; it does not say which edge fails. That is
  why V1's per-edge solve could still converge at n = 1 **[measured]**.

**Why the conforming route avoids the conflict.** Lemma A removes the derivative from the poke, so only **zeroth**
moments are needed (Lemma B). Zeroth-moment matching is compatible with exact constants, since Whitney reproduces
constants. The first-moment conflict of Lemma C never arises.

**A cheap falsification test of this diagnosis [conjecture, not run]:** V1 with only the 3 constant generators
enforced. Prediction:

- the reconstruction term stays near its base value (well under 10%);
- the test-variation term does **not** fall to the first-order level, because for a non-conforming reading the
  constants alone are not enough under $D$.

If instead the test-variation term falls and the reconstruction term stays small, then the seams are not the whole
story.

## 6. The mass: the one decision

Bee's *"the target is created by the magnets"* points to (M4). Two options:

- **(A) Keep the Whitney mass $M_1$ (BW05 as frozen).** By the Corollary, first order is reachable, provided each
  bump matches its tent's zeroth moment. The gate is unchanged.
- **(B) Build the mass from the magnets, $B = (\mathcal R^1)^\top H_1\mathcal R^1$.**
  - The flow becomes $\langle\mathcal R^1\dot a,\ \mathcal R^1\eta\rangle = -\langle d\mathcal R^1 a,\ d\mathcal R^1\eta\rangle$,
    a Galerkin method in the space spanned by the bumps.
  - Repeating Lemma A with the exact velocity $u=-\delta F$ gives the residual
    $$\langle \mathcal R^1 I^1u,\ \mathcal R^1\eta\rangle + \langle d\mathcal R^1 a,\ d\mathcal R^1\eta\rangle
    = \underbrace{\langle u+\delta F,\ \mathcal R^1\eta\rangle}_{=\,0} + O(h^{k+1})\|\mathcal R^1\eta\| + O(h^{k-1})\|\mathcal R^1\eta\|,$$
    so it is first order at $k = 2$ **with no moment matching at all**. The target is self-consistent.
  - Higher order may follow from the elliptic (Galerkin) structure. That is not argued here.
  - The cost: the gate's mass changes. BW05's comparison would have to be restated for the magnet mass.
    **[decision]**

This is the same question the 17 November projection-layer notes left open ("whether that layer may change is a
contract decision").

## 7. The hard part: keeping the pieces compatible at every level

The natural construction blends local fits with a partition of unity. Take $\lambda_v$ to be the barycentric hats
(conforming), and let $P_v$ be a polynomial 1-form fitted to the edge data around vertex $v$ (for example, the hybrid
fit one degree down). Then set $\mathcal R^1 a = \sum_v \lambda_v P_v a$. This gives (M1) and (M3). But since
$\sum_v d\lambda_v = 0$,
$$d\,\mathcal R^1 a \;=\; \sum_v\lambda_v\,dP_v a \;+\; \underbrace{\sum_v d\lambda_v\wedge(P_v a - P_w a)}_{\text{seam term}}$$
for any reference vertex $w$.

The seam term is the **disagreement between neighbouring fits**, weighted by $d\lambda\sim h^{-1}$. For a pure gauge
poke $a = \delta^0\phi$, with $P_v\delta^0\phi = dQ_v\phi$, it gives
$$d\,\mathcal R^1\delta^0\phi = \sum_v d\lambda_v\wedge d(Q_v\phi - Q_w\phi)\neq 0 .$$

So naive blending breaks gauge exactness, and with it (M2). This is the precise content of *"understand what the
piece is connected to"*: neighbouring fits must be compatible where they overlap, at the vertex, edge **and** face
levels.

The standard tool is the Čech–de Rham zig-zag. A correction term built from $d\lambda_v$ and *scalar* local readings
$S_v$, with $S_v\delta^0\phi = Q_v\phi$ and constants anchored consistently, makes the blend commute with $d$. This is
Weil's proof of the de Rham theorem (see Bott–Tu, *Differential Forms in Algebraic Topology*). Whitney's map arises
from this construction when the local data are constants **[conjecture as stated; the lowest-order identity is
standard]**. Designing the higher-order zig-zag on our meshes is the core construction problem.

**Existence elsewhere.** Smooth, commuting, higher-order de Rham complexes exist on tensor-product meshes:
B-spline / isogeometric discrete differential forms (Buffa, Rivas, Sangalli, Vázquez, 2011). On curved
unstructured tetrahedral meshes, built from neighbourhood cochains as here, we know of no construction.

**A caution.** Wardetzky, Mathur, Kälberer and Grinspun, *Discrete Laplace Operators: No Free Lunch* (2007): for the
scalar Laplacian on general surface meshes, no operator has symmetry, locality, linear precision and positive weights
together. Our four demands (variational, local, accurate, stable) are their four. Caveats:

- their setting is scalars on surfaces with positive *weights*, not a positive spectrum;
- our meshes are shape-regular quotients, not general;
- the full text was not obtained (17 November notes).

Bending one property is possible. The bumps here are deliberately *less* local than Whitney (Bee's wider magnets), and
(A) needs moment matching only to first order.

## 8. Relation to work in flight

- **V2 / V2h (Windows) continue unchanged.** They test whether three rings give the per-cell change room. This note
  predicts that the per-cell route can at best fight the seam terms, not remove them. V2's result is therefore a
  direct test of the note: if V2h passes P-recon and V2-acc, the seams matter less than Lemma A suggests.
- **The hybrid reading** (`docs/RICCI_PG_HYBRID_SPEC.md` on `feat/reading-the-curve-update`) supplies the local
  second-order data $P_v$ a magnet construction would blend. It passed every gate as a Petrov–Galerkin force
  **[measured]**: 0.158% at n = 5, observed orders 2.84, 1.88 and 1.83, S3 stable at n ≤ 3, μ > 0 at n ≤ 2.

## 9. Proposed next steps (pending Bee's review)

1. **[decision]** Confirm the translation (M1)–(M4), and choose the mass: (A) Whitney, frozen; or (B) from the
   magnets.
2. **Spec M1 (a magnet reading):**
   - the commuting partition-of-unity construction with the zig-zag correction, $k = 2$, at n = 1 and 2;
   - gates fixed before code: gauge annihilation to $10^{-10}$; Proposition-1 terms (now reconstruction and moment);
     BW05 against the chosen mass; S3; zeroth-moment defect per edge; bump amplification.
3. **The cheap falsification test of §5** (V1 with constants only). It is a few minutes at n = 1 and 2, and it tests
   the seam diagnosis before anything new is built.

## Open questions for Bee

- At which level do the magnets sit: vertices (as Whitney's barycentric hats do), edges, or cells? The construction
  in §7 places them at vertices, because that is where gauge lives.
- Should a poke's energy reach exactly the edges sharing a face with $e$ (one ring of connections), or wider? (M3)
  with $k = 2$ needs enough connected data to fit quadratic 1-forms locally. The count for that has not been done.

---

## Addendum (same evening): the §5 falsification test was run

`docs/VARIATIONAL_RECOVERY_V1C_RESULTS.md`, spec fixed before the run. V1 with the constants only:

| | n = 1 | n = 2 |
| --- | --- | --- |
| reconstruction term | 5.20% | 3.82% |
| test-variation term | 82.1% | 58.4% |
| ‖C‖ median | 3.7 | 6.8 |

V1 needed ‖C‖ of 288 and 65, with reconstruction terms of 432% and 61.6%. Both predictions pass:

- dropping the linear generators removes the explosion (Lemma C);
- the constants alone leave the poke term at tens of percent (Lemma A's seams).

**[measured]** This is the outcome the note predicted, and it points to conformity (the magnet rule), not to more
constraints. One caveat: there are two levels only, and the test-variation term fell from n = 1 to n = 2. n = 3 would
settle its trend.
