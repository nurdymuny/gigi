# The curved path reaches the other well

27 November 2026. **PROVED: the inter-well path sub-conjecture in H8,
Step 6, for the existing unsourced reduced potential. H8 remains open.**

I had a lower bound and a straight-line upper bound. The missing question
was whether a path actually followed the gradient all the way between
the wells. It does. The shape of the permitted motion gives the answer:
there is a rectangle the inward branch cannot leave, and inside that
rectangle the repeated coordinates keep decreasing.

This is a statement about my reduced configuration space. It does not
turn its large-gauge copies into distinct physical Yang--Mills vacua.

## 1. The objects, unchanged

Fix t > 0, let $u=t^{-1/2}>0$, and use scaled coordinates $p=gq$, g > 0.
The space is $\mathbb R^{3\times3}$ with its Frobenius inner product. Put

$$W_1(p)=\frac12\sum_{k=1}^3\omega_k\|p_k\|^2-\det p,
\qquad \omega=(1/u,u,u),\qquad V_1(p)=\tfrac12\|\nabla W_1(p)\|^2.$$

Let $P=\operatorname{diag}(u,1,1)$. Both 0 and P are critical points of
W_1 and zeroes of V_1. The second well is the orbit P SO(3). Define

$$d=\inf_{\gamma:0\to P SO(3)}\int_0^1
\sqrt{2V_1(\gamma(r))}\,\|\gamma'(r)\|\,dr,$$

over continuous piecewise C1 paths. This is the zero-energy Agmon
normalization for $-\hbar^2\Delta/2+V_1$, not the convention for
$-\hbar^2\Delta+V_1$. Here $\hbar=g^2$ in the scaled reduced operator.

**Proposition.** For every t > 0 there is a negative-gradient trajectory
$p(\tau)$, defined for all real flow times, such that

$$\lim_{\tau\to-\infty}p(\tau)=P,\qquad
\lim_{\tau\to+\infty}p(\tau)=0.$$

It lies in the plane $p=\operatorname{diag}(x,y,y)$ and, at every finite
time, in $0<x<u$, $0<y<1$. Consequently

$$\boxed{d=\frac u2=1/(2\sqrt t).}$$

The path has infinite flow time, but finite geometric action. No claim
of a finite-time arrival at an equilibrium is made.

## 2. Keep the metric when reducing the coordinates

Restricting the Frobenius metric to this plane gives $dx^2+2dy^2$.
The factor two counts the two diagonal entries changed by y. The restricted
superpotential is

$$w(x,y)=\frac{x^2}{2u}+uy^2-xy^2.$$

The full gradient is tangent to the plane:

$$\nabla W_1(\operatorname{diag}(x,y,y))=
\operatorname{diag}(x/u-y^2,(u-x)y,(u-x)y).$$

Thus its negative-gradient equations, with the inherited metric, are

$$\dot x=f(x,y)=y^2-x/u,\qquad
\dot y=h(x,y)=(x-u)y.$$

The second equation is $-\tfrac12\partial_yw$, not $-\partial_yw$.
Along this flow,

$$\frac{dw}{d\tau}=-(f^2+2h^2)=-\|\nabla W_1\|^2.$$

## 3. The branch enters the rectangle

At (u,1) the flow Jacobian is

$$J=\begin{pmatrix}-1/u&2\\1&0\end{pmatrix},\qquad
\det J=-2.$$

It is hyperbolic, with exactly one positive eigenvalue

$$\lambda_+=\frac{\sqrt{u^{-2}+8}-u^{-1}}2>0.$$

An eigenvector is $(\lambda_+,1)$, since
$\lambda_+^2+\lambda_+/u-2=0$. The inward branch has tangent
$(-\lambda_+,-1)$: both entries decrease. By the unstable-manifold
theorem there is a nonconstant local orbit approaching (u,1) as time
tends to minus infinity, tangent to that direction. Sufficiently close
to the saddle it is strictly inside the rectangle.

The precise external input is Simon Brendle, *Ordinary Differential
Equations*, [Theorem 4.10, applied with time reversed](https://web.stanford.edu/class/math63cm/ode.pdf).
Its hypotheses hold: the vector field is polynomial and the equilibrium
is hyperbolic. The full nine-dimensional equilibrium has gauge-zero
directions; the theorem is applied to the invariant two-dimensional
plane, where there are none. Plane invariance makes this an ambient
trajectory, not a new constrained model.

## 4. It cannot escape, and it must reach the origin

On the four faces of $[0,u]\times[0,1]$,

$$f(0,y)=y^2\ge0,\quad f(u,y)=y^2-1\le0,
\quad h(x,0)=0,\quad h(x,1)=x-u\le0.$$

Here is the strict-interior argument, so tangency is not mistaken for a
complete proof. For positive y initially,
$y(\tau)=y(0)\exp\int_0^\tau(x(r)-u)dr$ cannot reach zero in finite time.
The integrating-factor formula for $x'+x/u=y^2$ gives x > 0. At the right
and top faces the vector field points strictly inward except at (u,1).
A first contact there is excluded by uniqueness, because it is an
equilibrium. Thus an interior trajectory remains interior. Compact
containment and smoothness extend it for all positive time; the usual
continuation theorem applies (Brendle, section 3.3).

Boundedness alone is not convergence. Choose any finite starting point
$(x_0,y_0)$ of this branch and reset its time to zero. Since x < u,
y is decreasing and $0<y(\tau)\le y_0<1$. Define

$$M=\max(x_0,uy_0^2),\qquad M<u,\qquad \delta=u-M>0.$$

The equation for x gives

$$x(\tau)\le e^{-\tau/u}x_0+uy_0^2(1-e^{-\tau/u})\le M.$$

Therefore $y'\le-\delta y$ and
$0<y(\tau)\le y_0e^{-\delta\tau}$. Finally

$$x(\tau)=e^{-\tau/u}x_0+
\int_0^\tau e^{-(\tau-r)/u}y(r)^2dr\longrightarrow0.$$

For an explicit check without a nonresonance assumption, split that
integral at $\tau/2$. It is at most
$uy_0^2(e^{-\tau/(2u)}+e^{-\delta\tau})$, which tends to zero.
Hence the orbit reaches 0 asymptotically. This also rules out another
limit set without invoking a numerical orbit or a phase-portrait guess.

## 5. The action is the endpoint difference, in the full space

For every ambient piecewise C1 path, Cauchy--Schwarz and the chain rule give

$$\int\sqrt{2V_1}\,\|dp\|=\int\|\nabla W_1\|\,\|dp\|
\ge |W_1(\text{end})-W_1(\text{start})|.$$

Right multiplication by SO(3) preserves W_1, so every endpoint on the
orbit has the same value $W_1(P)=u/2$; $W_1(0)=0$. Consequently d >= u/2,
including for paths that leave the diagonal plane.

On the constructed negative-gradient orbit equality holds pointwise:

$$\sqrt{2V_1(p)}\,\|p'\|=\|\nabla W_1(p)\|^2=-\frac{d}{d\tau}W_1(p).$$

Its action on any finite time interval is exactly the decrease of W_1.
Letting the endpoints tend to minus and plus infinity gives total action
u/2. To connect this infinite-time orbit to the definition using finite
paths, truncate it and add straight endpoint **connectors** to P and 0.
Near either critical point c, boundedness of the Hessian gives
$\|\nabla W_1(z)\|\le L\|z-c\|$. A connector of Euclidean length r
therefore has action at most $Lr^2/2$. Both connector actions tend to zero.
Reverse the resulting path to run from 0 to P. These are admissible
finite paths with actions tending to u/2, proving d <= u/2 and the result.

## 6. What the curve improves

The old straight segment $p=\ell P$, $0\le\ell\le1$, still supplies a
valid comparison. Its action is

$$L_{\rm straight}=\frac{\sqrt{(u^2+2)(1+2u^2)}}6,
\qquad L_{\rm straight}^2-d^2=\frac{(u^2-1)^2}{18}.$$

It is strictly larger unless t = 1. At the round sphere the exact orbit
is $x=y=(1+e^{\tau-\tau_0})^{-1}$ and follows the straight segment. Away
from it, the gradient has the wrong tangent to stay on that segment:
$f(u\ell,\ell)-u h(u\ell,\ell)=\ell(\ell-1)(1-u^2)$.
The bending is necessary to saturate the bound; it is not decoration.

At t = 3,
$d=1/(2\sqrt3)=0.2886751345948129\ldots$ while
$L_{\rm straight}=\sqrt{35}/18=0.3286710990610898\ldots$.
The original model is unchanged. Returning to q gives
$W_g(q)=g^{-2}W_1(gq)$ and action $d/g^2=1/(2\sqrt t\,g^2)$.

This establishes an exact geometric action, not an exact quantum level
splitting, a prefactor, the finite-g carrier ordering or a continuum mass
gap. The two wells here retain the reduced model's large-gauge copies.
H8 remains open.

## 7. Verification and what Lean covers

The [contract](h8-interwell-contract-2026-11-27.md) precedes the proof.
`scripts/check_h8_interwell.py` independently differentiates the full
nine-variable polynomial and checks the restriction, metric, local flow,
comparison identities, straight path, round orbit and scaling in exact
SymPy arithmetic. Deliberately discarding the repeated coordinate,
using the ordinary coordinate gradient, or reversing flow is rejected.

`formal/h7-cubic-quartic/H8Interwell.lean` formalizes the local real
algebra, a genuine chain-rule derivative along an ODE solution, rectangle
inequalities, equilibrium classification, unstable eigenvalue/direction,
decay comparison, weighted Cauchy inequality, action normalization and
straight-path excess. The global orbit theorem and the improper integral
and path-infimum construction above are **not end-to-end Lean verified**.
They use the cited ODE theorem and the written continuation, comparison
and connector arguments. There is no formal theorem pretending to
establish orbit existence from the mere signs on four faces.

Receipts are in `research/h8-interwell-20261127/`; the release gate also
checks the earlier H7 declarations and their foundational dependencies.
No simulation, eigensolve, GPU run or engine test suite is required.
