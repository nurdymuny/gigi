# The cubic interaction cannot escape its bracket geometry

27 November 2026. **CLOSED BY DEDUCTION:** Sub-conjecture A_g in H7, at
alpha = 3/4, for each fixed compact simple real Lie algebra with a fixed
positive Ad-invariant inner product. **H7 remains open.**

I wanted to know whether the small room remains a room when I change the
colour group. For SU(2), the determinant and cofactor make that visible.
For another group those particular matrices disappear, but the brackets do
not. This is the replacement: the signed cubic interaction is controlled by
the total squared bracket size, even near commuting directions.

The difficult global estimate is already in Austing and Wheater. I use it
as a cited theorem, not as a new discovery. The deduction below translates
it into my conventions and closes the specific missing inequality.

## 1. Objects and the exact statement

Fix a compact simple real Lie algebra $\mathfrak g$, a positive Ad-invariant
inner product, and $q=(X_1,X_2,X_3)\in\mathfrak g^3$. Put

$$\mathcal C(q)=\langle X_1,[X_2,X_3]\rangle,\qquad
Q(q)=\|[X_1,X_2]\|^2+\|[X_2,X_3]\|^2+\|[X_3,X_1]\|^2.$$

With $Y_k=\tfrac12\sum_{l,m}\varepsilon_{klm}[X_l,X_m]$, the three $Y_k$
are exactly $[X_2,X_3],[X_3,X_1],[X_1,X_2]$, and $Q=\sum_k\|Y_k\|^2$.
Ad-invariance gives $\langle X_k,Y_k\rangle=\mathcal C$ for every k.

**Proposition.** There is a finite constant $c_{\mathfrak g}>0$, depending
on the fixed algebra and inner product but not on q, such that

$$|\mathcal C(q)|\le c_{\mathfrak g} Q(q)^{3/4}\quad\text{for every }q.$$

This proves the old sub-conjecture with $\alpha=3/4$; its factor
$\|q\|^{3-4\alpha}$ is no longer needed. No uniform constant across groups
or optimal value of $c_{\mathfrak g}$ is asserted.
This cubic-bound constant is distinct from the gap constant also denoted
c_g in the full H7 conjecture; no identification between them is made.

## 2. The cited input and the convention check

Austing--Wheater, *Adding a Myers Term to the IIB Matrix Model*, JHEP 11
(2003) 009, [arXiv:hep-th/0310170v2](https://arxiv.org/pdf/hep-th/0310170v2),
equation (2.2), supplies constants A,B > 0 bounding the absolute cubic trace
by A times the sum of the three squared commutator norms plus B, for a fixed
compact gauge algebra. This is the algebraic result used here. I do not
need convergence of their D=3 partition integral.

Choose a faithful unitary Lie-algebra representation $\rho$ (the adjoint
representation suffices) and set $A_k=i\rho(X_k)$, which is Hermitian.
Simplicity implies that for some fixed $\eta>0$,

$$\langle X,Y\rangle=-\eta\operatorname{Tr}(\rho(X)\rho(Y)).$$

Define $C_H=-i\operatorname{Tr}(A_1[A_2,A_3])$ and
$Q_H=\sum_{i<j}\operatorname{Tr}([A_i,A_j]^\dagger[A_i,A_j])$. Directly,

$$\mathcal C=\eta C_H,\qquad Q=\eta Q_H.$$

Indeed $\rho([X_i,X_j])=-[A_i,A_j]$. Cyclicity identifies the cubic trace
in (2.2), up to a sign inside an absolute value. Every Hermitian commutator
is anti-Hermitian, so the absolute trace of the sum of its three squares
in that equation equals $Q_H$. There is no factor of two from ordered pairs.
Thus its bound reads $|C_H|\le A Q_H+B$, with A and B independent of q.

## 3. Homogeneity closes the inequality

First work in the Hermitian convention. If $Q_H>0$, apply the cited bound
to $(rA_1,rA_2,rA_3)$ with $r=Q_H^{-1/4}$. The cubic scales by $r^3$ and
the quartic by $r^4$, giving

$$r^3|C_H|\le A r^4Q_H+B=A+B,
\qquad |C_H|\le(A+B)Q_H^{3/4}.$$

Consequently $|\mathcal C|\le\eta^{1/4}(A+B)Q^{3/4}$, so take
$c_{\mathfrak g}=\eta^{1/4}(A+B)$. Equivalently one can translate the
inhomogeneous bound first and rescale by $Q^{-1/4}$.
When $Q=0$, positivity forces all three brackets to vanish and
$\mathcal C=0$. This treats that case without division. QED.

**Why the geometry matters.** The set Q = 1 is not compact: large commuting
components can remain. Homogeneity alone does not bound the cubic there.
The global cited estimate is what controls those directions; the scaling
step uses that control rather than assuming it. The non-regular commuting
strata therefore do not remain an extra unproved step in this corollary.

## 4. The small-room consequence, with an explicit constant

Keep the original source and frequencies, with $t\ge1$, $g>0$ and
$0<s<(t+2)/(t+1)$. Write $S=\sqrt t+2/\sqrt t$ and

$$a_1=\omega_1^2-\mathrm{src}_1=s(t+2-2s),\qquad
a_2=a_3=\frac{s[(t+2)-s(t+1)]}{t}=\kappa>0.$$

Since $a_1-a_2=s(t-1)(t+2-s)/t\ge0$ on this window, the same kappa is the
minimum quadratic coefficient for every group. Expanding the existing
potential and using the proposition yields

$$V(q)=\frac12\sum_k a_k\|X_k\|^2-gS\mathcal C(q)+\frac{g^2}{2}Q(q)
\ \ge\ \frac\kappa2\|q\|^2+\frac{g^2}{2}Q-gS c_{\mathfrak g}Q^{3/4}.$$

The remaining one-variable minimization is exact. For $z\ge0$,

$$\frac{z^4}{2}-z^3+\frac{27}{32}
=\frac{(2z-3)^2(4z^2+4z+3)}{32}\ge0.$$

Set $z=gQ^{1/4}/(S c_{\mathfrak g})$. It follows that

$$\boxed{V(q)\ge\frac\kappa2\|q\|^2-
\frac{27S^4c_{\mathfrak g}^4}{32g^2}.}$$

The scalar minimum is attained at z = 3/2, explaining the coefficient
27/32. This does not assert that a Lie-algebra triple saturates the whole
bound. The previous general-alpha Young weights would have been alpha,
(3-4 alpha)/2 and (2 alpha-1)/2; the older (4 alpha-1)/2 was incorrect.
The factorization removes the need for that intermediate estimate here.

## 5. Operator and sectors

Take the Friedrichs realization of $-\tfrac12\Delta+V$ on
$L^2(\mathfrak g^3,dq)$ with this positive Euclidean inner product. V is a
smooth polynomial bounded below. After shifting the quadratic form by a
constant, its bounded sets control both the H1 norm and
$\int\|q\|^2|u(q)|^2\,dq$. Outside a ball of radius L their squared L2
mass is at most a constant times $L^{-2}$. Inside the ball Rellich
compactness applies. Combining the two gives a compact form embedding,
hence compact resolvent and discrete spectrum with finite multiplicities.

Constant colour rotations, and the remaining spatial circle symmetry,
preserve this form. Their closed invariant sectors are reducing subspaces;
the resolvent restricted to each is still compact. This includes the
colour-singlet charge sectors. This is the natural full-space reduction,
not a newly imposed boundary condition on a singular quotient chart.

The spectral implication is the standard compact-resolvent argument; see
Reed--Simon, *Methods of Modern Mathematical Physics IV: Analysis of
Operators* (1978), already cited for Theorem A. This extends its reduced
discreteness conclusion, with a group-dependent additive constant. It does
not compute a gap, identify its carrier, remove fast modes, or take a
continuum or infinite-volume limit.

## 6. Adversarial audit and lightweight verification

| Possible shortcut or mistake | Resolution |
| --- | --- |
| Quartic degree alone dominates cubic | Not used; the global (2.2) bound is essential. |
| Q = 1 is compact | False; explicitly not used. |
| A near-commuting or singular stratum escapes | The cited global estimate covers every triple; no regular-stratum restriction is introduced. |
| Hermitian and anti-Hermitian signs or pair counts differ | The trace dictionary above retains eta, i and three unordered pairs. |
| Q = 0 makes the normalization undefined | Treated separately by vanishing brackets. |
| Matrix integral convergence is needed | Not used; only its algebraic bound is invoked. |
| An arbitrary group-uniform constant is inferred | Not claimed; each group and norm are fixed. |
| A quotient boundary condition changes the model | The argument uses the full-space Friedrichs form and invariant restrictions. |
| Discreteness closes the full H7 conjecture | It closes only A_g and the reduced Theorem A extension. |

`scripts/check_h7_cubic_quartic.py` checks exact trace and scaling identities,
the factorization, the unchanged kappa window, a non-regular commuting
limit, five deliberately wrong contracts, and the document scope. It is
not a proof assistant and does not re-prove the cited global inequality.
The Python calculation is supporting algebra, not numerical evidence for
all triples. There are no simulations or numerical eigensolves.

The contract was written first in
[`h7-cubic-quartic-contract-2026-11-27.md`](h7-cubic-quartic-contract-2026-11-27.md).
Before the proof edit, ten algebra checks passed and both document checks
failed. Receipts are retained in
[`research/h7-cubic-quartic-20261127/`](../../research/h7-cubic-quartic-20261127/).
Run from the repository root, with a new output directory:

```text
python scripts/check_h7_cubic_quartic.py <new-output-directory>
```

**What changes in my theory.** The small room has a confining-at-infinity
potential and discrete spectrum for every fixed compact simple colour
algebra, on the same source window. The embedded Helike doublet remains
what it was; this result does not make it the lowest extra-colour mode.
H7 remains open, but this particular obstruction is removed by an exact
argument rather than a larger computation.

## 7. A second check in Lean

I have now separated the central real-algebraic argument into a small
[Lean project](../../formal/h7-cubic-quartic/README.md). Lean checks the
3/4 deduction for every configuration satisfying the stated global bound
and homogeneity, including Q = 0. It also checks the exact 27/32
factorization, the scaled coercivity estimate, the potential lower bound,
and the same kappa identities and positive source window.

The global Austing--Wheater bound is an explicit argument to the theorem,
not an added axiom. This is the formal verification of the deduction from
that bound. I have not formalized its Lie-algebra theorem, my trace
dictionary, or the compact-resolvent argument in Lean here. Those remain
the cited and written parts of this proof. H7 remains open.

All eight target declarations compiled with only Lean's standard
foundational dependencies: `propext`, `Classical.choice`, and `Quot.sound`.
There are no admitted proof steps. Changing the exponent, removing a
needed hypothesis, or halving the scalar remainder makes the deliberately
broken source fail to compile. Exact examples check equality at the scalar
minimum and zero kappa at a source-window boundary. The
[contract](h7-lean-contract-2026-11-27.md) and
[compiler receipts](../../research/h7-lean-20261127/) keep the scope and
the checks together. This adds verification without changing the model,
the source window, or the claim about which sub-conjecture is closed.
