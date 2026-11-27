# The triangle cancels; four insertions do not

27 November 2026. **RESOLVED / REFUTED:** the prediction in H6, Step 6,
that the leading free-field Wick triangle T_O is nonzero. It is identically
zero at pairwise distinct points in four-dimensional Euclidean space.
**H6 remains open.** The seven top-level hypotheses are unchanged.

I wrote down a specific expectation: the free triangle would give me the
first higher connected signal. It does not. The reason is geometric, not
a small number that disappeared under rounding. A covariance exchanges
the two Hodge halves of a two-form. Three exchanges cannot close inside
one half. The trace cancels exactly.

The four-insertion alternative was already in my note. Four exchanges
can return to their starting half. The exact calculation below gives a
nonzero connected four-point function, so that alternative is available
without a spectral run. This settles the finite free-field question; it
does not establish the continuum hypothesis that uses it.

## 1. Which quantity is being settled

Use canonically normalized free gauge fields on oriented Euclidean R^4:

$$\langle A_\mu^a(x)A_\nu^b(y)\rangle
=\delta^{ab}\delta_{\mu\nu}G(x-y),\qquad
G(r)=\frac1{4\pi^2|r|^2}.$$

This is Feynman gauge; longitudinal gauge terms disappear after the two
antisymmetrized derivatives, because partial derivatives commute. At this
free order $F^a=dA^a$. There are $n=\dim\mathfrak g$ independent colour
copies, not interacting nonabelian fields. Keep the paper's observable

$$O(x)=\sum_{a,\mu,\nu}:F^a_{\mu\nu}(x)F^a_{\mu\nu}(x):
=2\sum_{a,A}:(F_A^a(x))^2:,$$

where $A=(\mu,\nu)$ with $\mu<\nu$ runs through
$(01,02,03,12,13,23)$. The factor two is essential. Normal ordering
removes self-contractions; at distinct points the connected function is
unchanged by vacuum subtraction. The Lagrangian normalization is O/4.

The question is $T_O(x_1,x_2,x_3)=\langle O(x_1)O(x_2)O(x_3)\rangle_c$.
This is not a stress-tensor three-point function, not a three-point
function of F itself, and not the full interacting trace-anomaly
correlator. Contact terms are not included or claimed to vanish.

## 2. The covariance is a reflected two-form

The second insertion contributes a minus sign on differentiating G.
Writing $D_{\mu\rho}=-\partial_\mu\partial_\rho G$, the one-colour
covariance is

$$C_{\mu\nu,\rho\sigma}(r)
=\delta_{\nu\sigma}D_{\mu\rho}-\delta_{\nu\rho}D_{\mu\sigma}
-\delta_{\mu\sigma}D_{\nu\rho}+\delta_{\mu\rho}D_{\nu\sigma}.$$

Direct differentiation, for r != 0, gives

$$C_{\mu\nu,\rho\sigma}(r)=\frac{
I_{\mu\rho}(r)I_{\nu\sigma}(r)-I_{\mu\sigma}(r)I_{\nu\rho}(r)}{\pi^2|r|^4},
\qquad I(r)=1-2\frac{rr^T}{|r|^2}.$$

I reverses the radial direction and fixes its orthogonal complement: an
orthogonal reflection with determinant -1. Consequently C is
$\Lambda^2 I/(\pi^2|r|^4)$ on the six-dimensional space of two-forms.

Let J be Euclidean Hodge star there. Then $J^2=1$. For an orthogonal map
R, the defining wedge/volume identity for Hodge star gives
$J\Lambda^2R=(\det R)\Lambda^2R J$. The metric is preserved while an
orientation-reversing map changes the sign of the volume form. Hence

$$JC(r)=-C(r)J.$$

The covariance exchanges the self-dual and anti-self-dual eigenspaces of
J. Equivalently, with $P_\pm=(1\pm J)/2$, $P_\pm C P_\pm=0$.
The exact code verifies all 36 covariance entries and this identity
symbolically, with arbitrary real displacement components and r != 0.

## 3. Eight Wick pairings, one vanishing trace

At three quadratic insertions, a connected contraction has one edge
between each pair of points. There are eight labelled pairings of the six
legs. Summing components gives a trace; the two orientations have equal
traces because reversed covariance blocks are transposes. Each insertion
contributes its factor two, and connected colour contractions force one
shared colour around the cycle. Therefore

$$T_O=64n\,\operatorname{tr}(C_{12}C_{23}C_{31}).$$

Put $M=C_{12}C_{23}C_{31}$. Three applications of the exchange identity
give $JM=-MJ$, so $JMJ=-M$. But cyclicity and $J^2=1$ give

$$\operatorname{tr}(JMJ)=\operatorname{tr}(M),\qquad
\operatorname{tr}(JMJ)=-\operatorname{tr}(M).$$

Over the reals this forces the trace to vanish. Thus

$$\boxed{T_O(x_1,x_2,x_3)=0\quad
\text{for every pairwise distinct triple in free Euclidean R^4}.}$$

This is an all-configurations proof. Collinear or specially symmetric
triangles are not being used to infer the result. The prediction
$T_O\not\equiv0$ in my note is false in its stated free-field setting.

## 4. Nonzero controls, including the documented alternative

These controls are evaluated by the same connected-pairing code, which
enumerates contractions instead of hard-coding a zero or a trace formula.

**Two Maxwell insertions.** Two labelled pairings give

$$\langle O(x)O(y)\rangle_c
=8n\operatorname{tr}(C_{xy}C_{yx})
=\frac{48n}{\pi^4|x-y|^8}.$$

**An ordinary free scalar triangle.** With the same scalar propagator G
and $Q=: \phi^2:$, Wick's theorem instead gives $8G_{12}G_{23}G_{31}$.
At positions 0, 1 and 3 on a unit coordinate line this is
$1/(288\pi^6)>0$. There is no two-form Hodge exchange here.

**Four Maxwell insertions.** The 48 connected labelled pairings fall into
three unoriented cycles, 16 pairings each. The observable factors make

$$T_O^{(4)}=256n\sum_{\text{three cycles}}
\operatorname{tr}(C_{i_1i_2}C_{i_2i_3}C_{i_3i_4}C_{i_4i_1}).$$

For $x_j=jL e_0$, j = 0,1,2,3 and L > 0, every reflection is the same,
its fourth power is identity, and each trace has dimension factor six.
The three distance products contribute $1/81$, $1/256$ and $1/20736$.
Thus the connected, not full, function is

$$\boxed{T_O^{(4)}(0,Le_0,2Le_0,3Le_0)
=\frac{1536n}{\pi^8L^{16}}
\left(\frac1{81}+\frac1{256}+\frac1{20736}\right)
=\frac{676n}{27\pi^8L^{16}}>0.}$$

This one separated configuration proves the four-point function is not
identically zero. It is the alternative already named in H6, not a
replacement invented to declare the failed three-point prediction true.

## 5. The correction to the theory

Let $a(g)=b_0g^2/(32\pi^2)$ denote the paper's leading trace-anomaly
coefficient. The proposed relative expansion
$a(g)^3 T_O[1+O(g^2)]$ cannot describe the first nonzero term when T_O
vanishes. Under H6's perturbative matching the safe statement is additive:

$$\langle\Theta\Theta\Theta\rangle_c
=a(g)^3\bigl[T_O+O(g^2)\bigr]=O(g^8),$$

at a fixed separated configuration and common short-distance scaling,
with the appropriate dimensionful position factors understood. This does
not assert that the higher-order coefficient is nonzero, nor that the
interacting three-point function vanishes identically.

The corresponding four-point expansion has the nonzero coefficient just
computed:

$$\langle\Theta^4\rangle_c=a(g)^4\bigl[T_O^{(4)}+O(g^2)\bigr].$$

Conditional on H6, it is nonzero for sufficiently weak running coupling
at the stated fixed shape. That excludes **Theta itself** being a
generalized free (Gaussian) field, whose connected functions of order
greater than two vanish. It does not exclude Theta being a non-Gaussian
composite of underlying Gaussian fields. The logarithmic two-point test
keeps its existing restricted comparison class. Neither test alone
classifies the complete observable algebra or closes H6.

## 6. Sources and verification boundary

The duality structure is standard. Cordova, Maldacena and Turiaci,
*Bounds on OPE coefficients from interference effects in the conformal
collider*, JHEP 11 (2017) 032,
[section 3.2](https://arxiv.org/abs/1710.03199), discuss the opposite
duality charges of the self-dual and anti-self-dual free field strengths.
Their discussion there concerns a stress-tensor selection rule. The
observable, Wick factors and cancellation above are derived explicitly
here; I do not attribute this exact calculation or its normalization to
that different correlator. No novelty claim is made for the duality rule.

The [contract](h6-wick-triangle-contract-2026-11-27.md) was frozen before
implementation. `scripts/helike_wick_triangle.py` differentiates the
propagator and enumerates connected pairings for two to four quadratic
insertions. The checker compares the reflection formula independently,
checks generic Hodge exchange, and uses direct component contractions at
two noncollinear rational triples. Noncancelling synthetic covariances
test the factor 64n, which cannot be validated by a zero result alone.

Five Lean declarations prove the matrix trace argument from an involution
and three anticommutation hypotheses, for arbitrary finite dimension.
This is **not end-to-end** QFT formalization: the propagator dictionary,
Hodge instantiation and Wick counting have the written proofs and exact
symbolic checks, not a formal construction of the field theory. See
[the formal boundary](../../formal/h7-cubic-quartic/H6Wick-README.md).

Fresh-output reproduction:

```text
python scripts/check_h6_wick_triangle.py <new-output-directory>
```

[Receipts](../../research/h6-wick-triangle-20261127/) retain the initial
red gate, compiler output and bad-mechanism rejections. No numerical
spectra or simulation are involved. This closes one documented conjecture
by refutation and verifies its documented four-point alternative; it
does not change the count of seven open top-level hypotheses.
