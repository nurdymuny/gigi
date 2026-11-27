# The formal vacuum does not fit in the room

27 November 2026. **CLOSED BY DEDUCTION:** strict separation of the
unsourced reduced ground energy from its factorization floor, for every
fixed compact simple metric Lie algebra and every t > 0, g > 0.
**H7 and H8 remain open.**

I already had a floor under the energy. The question was whether the
vacuum could actually sit on it. The factorization gives an exact candidate
for doing that, but I have to look at the shape of the candidate across
the whole configuration space, not just near its central well.

Near that well it looks like a Gaussian. Farther out, the cubic opens a
whole cone where the candidate grows instead of decays. It cannot be
normalized. Since the room is confining and its lowest energy is attained,
the actual vacuum must sit strictly above the floor. This is a deduction
from the geometry we have already established, not another spectral run.

## 1. The same operator, with all parameters visible

Fix a compact simple real Lie algebra $\mathfrak g$ of dimension n with
a positive Ad-invariant inner product. The configuration is
$q=(X_1,X_2,X_3)\in\mathfrak g^3$, of real dimension D = 3n. Keep the
original product metric and Lebesgue measure. Write

$$u=t^{-1/2},\quad \omega=(1/u,u,u),\quad S=1/u+2u,$$
$$\mathcal C(q)=\langle X_1,[X_2,X_3]\rangle,\qquad
W_g(q)=\frac12\sum_k\omega_k\|X_k\|^2-g\mathcal C(q).$$

At s = 1 the sources vanish. The original reduced operator is

$$H_g=-\frac12\Delta+\frac12\|\nabla W_g\|^2,$$

with its full-space Friedrichs realization. These are the reduced energy
units, not a newly rescaled Yang--Mills operator. Let
$E_0=\inf\operatorname{spec}H_g$ and $E_*=nS/2$.

**Proposition.** For every fixed algebra and inner product above, t > 0
and g > 0,

$$\boxed{E_0(g,t)>\frac n2\left(\sqrt t+\frac2{\sqrt t}\right)=E_*.}$$

The same strict lower bound holds on every nonzero reducing symmetry
sector, including the colour-singlet sector. For SU(2), n = 3. No
quantitative lower bound for $E_0-E_*$ is asserted.

## 2. First dependency: the bottom is attained

The [cubic-quartic deduction](h7-cubic-quartic-proof-2026-11-27.md)
establishes, for this fixed metric algebra,

$$|\mathcal C|\le c_{\mathfrak g}Q^{3/4},\qquad
Q=\sum_{i<j}\|[X_i,X_j]\|^2.$$

Its cited input is Austing--Wheater, [equation (2.2)](https://arxiv.org/pdf/hep-th/0310170v2),
translated and homogenized in that note. Only the cubic bound is used;
no matrix-integral convergence assumption is introduced here.

At s = 1, for **all** t > 0, take $\kappa=\min(t,1/t)>0$. The previous
scalar factorization gives directly

$$\frac12\|\nabla W_g(q)\|^2
\ge\frac\kappa2\|q\|^2-
\frac{27S^4c_{\mathfrak g}^4}{32g^2}.$$

The earlier sourced-window argument used t >= 1 to order two coefficients.
Here the unsourced coefficients are exactly t, 1/t, 1/t, so taking their
minimum needs no such ordering. The same compact form-embedding proof
applies: local Rellich compactness, with the quadratic lower bound
controlling the mass outside a large ball. Thus H has compact resolvent
and E0 is an eigenvalue with a normalized eigenfunction.

This step must come first. Excluding a zero eigenvector of a nonnegative
operator does not, by itself, exclude zero from its spectrum.

## 3. The constant shift and the equality condition

Choose orthonormal coordinates on the algebra in each row. The cubic is
linear in each row and therefore in each individual coordinate. All its
pure second derivatives vanish, including for groups other than SU(2).
Consequently

$$\Delta W_g=nS.$$

For each coordinate j define $A_j=\partial_j+\partial_jW_g$. On compactly
supported smooth functions, integration by parts gives

$$\mathfrak h[\psi]-E_*\|\psi\|^2
=\frac12\sum_{j=1}^D\|A_j\psi\|^2\ge0.$$

The sign is important: $A_j^*A_j=-\partial_j^2+(\partial_jW_g)^2-
\partial_j^2W_g$, so the term added back to H is **plus** $nS/2$.

This is also an identity on the Friedrichs form domain. Approximate in the
form norm by compactly supported smooth functions. Since the potential
is $\|\nabla W_g\|^2/2\ge0$, both the weak gradient and multiplication by
$\nabla W_g$ converge in L2. Hence each $A_j\psi$ converges in L2, and
the identity passes to the limit. No boundary integration is being assumed
for an unknown eigenfunction at infinity.

If E0 = E*, the normalized ground eigenfunction therefore satisfies
$A_j\psi=0$ for every j. Locally, multiplication by the smooth positive
function $e^{W_g}$ is legitimate and the weak product rule yields

$$\partial_j(e^{W_g}\psi)=0\quad\hbox{for all }j.$$

All weak derivatives vanish on the connected full space. A locally
integrable function with this property is constant: mollification makes
it constant on each smaller ball, and overlapping balls identify the
constants. Thus

$$\psi=c\,e^{-W_g}\quad\hbox{almost everywhere}.$$

The normalization requires c != 0. The remaining question is entirely
geometric: does that shape have finite squared norm?

## 4. A nonzero bracket makes a positive-measure cone

There is no need to select an embedded SU(2) representation. Nonabelianity
gives Y,Z with $B=[Y,Z]\ne0$. At $q_*=(B,Y,Z)$,

$$\mathcal C(q_*)=\langle B,[Y,Z]\rangle=\|B\|^2>0.$$

Normalize $\theta_*=q_*/\|q_*\|$. By continuity, some open spherical cap
U about $\theta_*$ and some c0 > 0 satisfy $\mathcal C(\theta)\ge c_0$
throughout U. Its surface measure is positive. For unit theta the
quadratic coefficient is at most $a=\max_k\omega_k/2$. On this cone,

$$W_g(r\theta)\le ar^2-gc_0r^3.$$

For $r\ge R=\max(1,2a/(gc_0))$, the right side is at most
$-gc_0r^3/2$. Therefore

$$e^{-2W_g(r\theta)}\ge e^{gc_0r^3}\ge1,$$
$$\int_{\mathfrak g^3}e^{-2W_g(q)}\,dq
\ge |U|\int_R^\infty r^{D-1}\,dr=\infty.$$

One ray would not suffice: a ray has zero D-dimensional measure. The
positive-measure cap is what turns the pointwise geometric observation
into a failure of normalization. Hence no nonzero multiple of exp(-W)
is in L2. Equality E0 = E* is impossible. Together with E0 >= E* this
proves the proposition. QED.

The lower bound for a nonzero reducing sector follows by restriction of
the same form: its spectral bottom is at least the full-space E0. No
new quotient boundary condition or assumption about the ground state's
charge is needed for this corollary.

## 5. What this closes, and what it does not

For this reduced model, the formal vacuum annihilated by every A_j is
not a physical normalized state at any positive coupling. Its failure is
global even though it looks correct near the central well. The actual
ground energy is strictly above the factorization floor.

This is **not the excitation gap** $E_1-E_0$. It does not determine the
lowest excited carrier, its charge, a tunnelling prefactor, or the size
of the shift. The earlier SU(2) cutoff argument for an exponentially
small upper shift is separate; the present proof neither depends on nor
re-proves that estimate. Strict positivity does not conflict with a shift
smaller than every power of the coupling.

At g = 0 the answer changes: W is positive quadratic, exp(-W) is a
normalizable Gaussian, and E0 = E* exactly. The same equality holds if
the cubic interaction is identically zero. Those are required controls,
not exceptions hidden in the statement.

For comparison, the free operator $-\partial_x^2/2$ on the line has no
nonzero L2 zero eigenfunction but has spectral bottom zero. The normalized
Gaussians $\pi^{-1/4}L^{-1/2}e^{-x^2/(2L^2)}$ have energy $1/(4L^2)$,
approaching zero. That is exactly why confinement/attainment cannot be
discarded from the proof above.

## 6. Verification and the next dependency

The [frozen contract](h8-vacuum-floor-contract-2026-11-27.md) precedes the
proof. The exact Python checks test the nine-coordinate SU(2) polynomial,
the general row-trilinearity mechanism, the factorization sign, Gaussian
controls, the cone-tail algebra and the free-particle dilation. None is a
numerical eigensolve. The spherical-cap argument and weak-kernel argument
are written proofs, not inferred from sampled configurations.

Seven Lean declarations verify the scalar jet identities, a finite
sum-of-squares implication, the cone-tail inequality, and the conditional
deduction from an attained bottom and excluded kernel. This is **not
end-to-end Lean** verification of the global theorem. The Lie-algebra
estimate, compact resolvent, weak-kernel characterization and spherical
integration are the cited/written parts. See the
[formal boundary](../../formal/h7-cubic-quartic/H8Vacuum-README.md).

Reproduce from the repository root with a fresh output directory:

```text
python scripts/check_h8_vacuum_floor.py <new-output-directory>
```

The [receipts](../../research/h8-vacuum-floor-20261127/) retain the initial
red gate and later checks. Bad factorization signs, missing attainment,
missing kernel exclusion and an overstrong tail inequality must not pass.

**Dependency order:** cubic-quartic confinement -> attained ground level;
factorization + cone geometry -> excluded equality; together -> strict
floor. The embedded inter-well path is not a prerequisite. H7 and H8
remain open, and there are still seven top-level hypotheses. The next
carrier-ordering question requires spectral information beyond this
ground-energy statement; no large run is authorized by this closure.
