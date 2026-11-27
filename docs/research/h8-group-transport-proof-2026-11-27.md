# The curve survives the larger colour space

27 November 2026. **A reduced-model path theorem, proved by transport.**

The curve does not work because I happened to draw it in three colour
directions. It works because those directions close under the Lie bracket.
Putting that structure inside a larger colour space preserves the actual
gradient equations. I still need to ask whether leaving it offers a cheaper
route. The answer, for the endpoints specified below, is no: the ambient
superpotential supplies the same lower bound that the embedded curve attains.

## 1. Objects and normalization

Let G be a compact connected group with simple real Lie algebra g and a
fixed positive Ad-invariant inner product B. Work on E = g^3 with the
product metric. Fix t > 0, u = t^(-1/2), and

$$\omega=(1/u,u,u),\qquad
W_G(X)=\frac12\sum_k\omega_k B(X_k,X_k)-B(X_1,[X_2,X_3]),\qquad
V_G=\tfrac12\|\nabla W_G\|^2.$$

These are the original unsourced reduced equations, in scaled coordinates
p = gq (the italic coupling g is a number, not the Lie algebra). The
kinetic normalization is one half the negative Laplacian. No new source,
effective interaction, quotient or parameter change is introduced.

Choose an injective Lie-algebra homomorphism i: su(2) -> g. In the source,
fix an orthonormal basis e1,e2,e3 with [e2,e3] = e1 and its cyclic versions.
There is a measured scalar I>0 such that

$$B(i(a),i(b))=I\langle a,b\rangle_2.$$

Here is an elementary reason. Put Ta = i(ea). Bracket preservation gives
[T2,T3] = T1 cyclically. Invariance and symmetry give, for example,
B(T1,T2) = B([T2,T3],T2) = -B(T2,T1), so off-diagonal entries vanish.
Also B(T3,T3) = B([T1,T2],T3) = B(T1,[T2,T3]) = B(T1,T1), and cyclically.
Their common value is positive because i is injective and B is positive.
Thus the entire Gram matrix, not just one chosen length, is I times identity.

I is a metric factor. It agrees with an integer embedding index only after
the corresponding source and target normalization conventions are fixed.
I do not silently equate arbitrary invariant inner products to that convention.

For a real 3 by 3 matrix p, define (Jp)_k = sum_a p_ka Ta. Let

$$P=\operatorname{diag}(u,1,1),\qquad
\mathcal O_i=\{(\operatorname{Ad}_h(uT_1),\operatorname{Ad}_h T_2,
\operatorname{Ad}_h T_3):h\in G\}.$$

This is a specified endpoint orbit in the reduced space, not an assertion
that its configurations and 0 are distinct physical Yang--Mills vacua.

## 2. The theorem

Over continuous piecewise C1 ambient paths with endpoints 0 and any point
of O_i, define

$$d_G(0,\mathcal O_i)=\inf_\gamma\int_0^1
\sqrt{2V_G(\gamma(r))}\,\|\gamma'(r)\|\,dr.$$

**Theorem.** For every t > 0 and every embedding as above,

$$\boxed{d_G(0,\mathcal O_i)=Iu/2=\frac{I}{2\sqrt t}.}$$

An ambient negative-gradient trajectory joins JP to 0 in infinite flow
time. Its total action is the displayed number. The formula concerns this
orbit; it is not a minimum over different embeddings or all possible wells.

## 3. Derive the full gradient before restricting it

For a variation Z = (Z1,Z2,Z3), Ad-invariance gives

$$D\,B(X_1,[X_2,X_3])[Z]
=B(Z_1,[X_2,X_3])+B(Z_2,[X_3,X_1])+B(Z_3,[X_1,X_2]).$$

Consequently

$$ (\nabla W_G)_k=\omega_kX_k-[X_{k+1},X_{k+2}], $$

with cyclic indices. This derivative tests every direction of g^3, including
directions outside the embedded algebra. It is not a projected gradient.

Let W_2 be the nine-dimensional superpotential of the preceding proof,
W_2(p) = sum_k omega_k ||p_k||^2/2 - det p. Linearity and bracket preservation
give

$$W_G(Jp)=I W_2(p),\qquad \nabla W_G(Jp)=J\nabla W_2(p),\qquad
V_G(Jp)=I V_2(p).$$

The second equality has **no I factor**. In particular, the image subspace
is invariant under the full gradient flow. The induced metric on
diag(x,y,y) is I(dx^2 + 2dy^2), while the restricted superpotential is
I[x^2/(2u)+uy^2-xy^2]. The factors cancel in the gradient equations:

$$x'=y^2-x/u,\qquad y'=(x-u)y.$$

Using I times the potential but the old metric would give the wrong speed.
Dividing the correct vector field by I would also be a different equation.

## 4. Transport the curve, then compare every path

The [previous inter-well theorem](h8-interwell-proof-2026-11-27.md) supplies
a global negative-gradient trajectory p(tau) with limits P and 0. By the
gradient identity, X(tau) = Jp(tau) is an ambient solution with limits JP
and 0. This uses the already proved global existence and convergence; a
new local phase portrait is not substituted for that theorem.

The product norm satisfies ||Jv|| = sqrt(I)||v||. Therefore the action
density on any embedded curve scales as

$$\sqrt{2V_G(Jp)}\,\|Jp'\|
=\sqrt I\sqrt{2V_2(p)}\,\sqrt I\|p'\|
=I\sqrt{2V_2(p)}\,\|p'\|.$$

The transported trajectory thus has action Iu/2, not sqrt(I)u/2.

For **every ambient path**, Cauchy--Schwarz and the chain rule give

$$\int\|\nabla W_G\|\,\|dX\|\ge
|W_G(\text{end})-W_G(\text{start})|.$$

Ad(G) preserves both B and the bracket, so W_G is constant on O_i, with
value Iu/2; W_G(0) = 0. This proves the ambient lower bound. It does not
require a list of other critical points, an absence of centralisers or
positive curvature in every transverse direction.

For the matching upper bound, truncate the infinite-time trajectory and
add straight endpoint connectors to JP and 0, then reverse orientation.
Both endpoints are critical by gradient intertwining. Near either one,
smoothness gives ||grad W_G(Z)|| <= L||Z-c||, so a connector of length r
has action at most Lr^2/2. These costs vanish as the truncation expands.
Hence finite admissible paths approach Iu/2, proving the infimum formula.

The result excludes a cheaper route through additional colour directions
to this orbit. It does not exclude a second route with equal action.

## 5. Why this applies across compact simple algebras

Every compact simple real Lie algebra contains a rank-one su(2) subalgebra.
The external structural input is Conrad--Landesman, *Compact Lie Groups*,
sections 17--19 and Appendix R, specifically the construction of the
rank-one commutator subgroup associated with a root; it is SU(2) or SO(3).
[Source](https://math.stanford.edu/~conrad/210CPage/handouts/lie_groups_notes.pdf).

The differential of either construction gives the required injective
su(2) map. A group homomorphism SU(2) -> G may have kernel {+1,-1}; this
does not invalidate its Lie-algebra embedding. The theorem needs the
algebra map, not a globally injective group map or a choice of global form.
This structural fact plus the argument above, not testing a list of
matrices, is the general deduction. Only this paragraph uses simplicity;
the transport itself works for any compact metric Lie algebra containing
such a subalgebra.

Returning to the original q coordinates gives W_(G,g)(q) = g^(-2)W_G(gq)
and action I/(2 sqrt(t) g^2). No uniform-in-group or uniform-in-t lower bound
is asserted for freely varying metrics or t tending to infinity.

## 6. Exact examples that distinguish the normalization

All matrix fixtures use anti-Hermitian traceless matrices and B(A,B) =
-2 tr(AB). Their generators are explicit in `scripts/check_h8_group_transport.py`.

| Target and embedding | Measured I | Action at t = 3, in scaled coordinates |
| --- | ---: | --- |
| SU(3), fundamental 2 by 2 block | 1 | 1/(2 sqrt(3)) |
| SU(3), irreducible spin-1 representation | 4 | 2/sqrt(3) |
| SU(4), two fundamental blocks | 2 | 1/sqrt(3) |
| SU(3), conjugated fundamental block | 1 | 1/(2 sqrt(3)) |

These action entries are applications of the theorem, not independently
simulated trajectory integrals. Exact tests measure the complete Gram
matrix and cyclic brackets, derive W and V on all nine source coordinates,
and differentiate W in all 24 SU(3) ambient coordinate directions for
three embeddings. They also check the induced planar metric, endpoints,
conjugation, coupling conversion and rejected malformed embeddings.

The spin-1 case is essential: it catches an omitted I even when all
fundamental-block tests pass. The conjugated case prevents reliance on
the visible upper-left placement of a block.

## 7. What is verified, and what is not claimed

The [frozen contract](h8-group-transport-contract-2026-11-27.md) precedes
the proof. Exact examples and compiler evidence are retained under
`research/h8-group-transport-20261127/`.

`H8GroupTransport.lean` checks conditional linear-map, bracket and metric
transfer algebra, local dissipation, action factors, and a conditional
distance squeeze. Its pairings are explicit algebraic parameters; their
interpretation as positive invariant inner products is supplied by the
written proof. The existence of embeddings, the ambient differential,
the transported global orbit, the integrals and the path infimum are
**not end-to-end Lean verified**. No assumed global bound is hidden as a
new axiom. The source H8 local lemmas are reused unchanged.

H7 and H8 remain open. This does not decide a lowest carrier, a quantum
excitation gap, a fluctuation determinant, a tunnelling prefactor or the
continuum theory. Larger groups may have other modes and other wells;
the theorem answers one precise endpoint-to-orbit question in their
unchanged reduced model.
