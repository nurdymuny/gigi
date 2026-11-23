# Yang-Mills Requirements as Geometric Objects

Bee Rosa Davis | Davis Geometric

23 November 2026 | Research note

## The change of attention

I am not looking for an analogy for Yang-Mills. I am looking for the object that
the mathematics is already describing.

When I ask to see the geometry, I do not mean replacing a calculation with a
picture. I mean keeping the calculation and asking what it measured. Where does
the object live? What are its directions? What can move, and what must remain
unchanged? Which distinctions belong to the object, and which belong only to the
coordinates I chose?

An eigenvalue is a number attached to a mode. The eigenfield is the thing that
has that number. I want both. I want to know how the field turns, where it lives,
how its spatial and internal directions are related, and what happens to those
relationships when I change the geometry.

This is the attention I brought to Helike and to *Reading the Curve, Not Counting
the Points*. I am bringing it here without asking the earlier results to answer
a different question on my behalf.

The central question of this note is:

**What are the actual geometric objects behind the Yang-Mills requirements, and
what would I have to establish about those objects?**

I use three labels:

- **EXACT:** a definition or equivalence under the stated hypotheses.
- **CONDITIONAL:** an exact construction once additional objects or assumptions
  have been supplied; supplying them may be the difficult part.
- **OPEN:** a proposed connection that has not been established.

These refine the structural/open distinction in my earlier notes. They are not
three levels of confidence in the same assertion. They identify different kinds
of assertion. This note gives geometric reformulations and a research direction,
not a construction of four-dimensional quantum Yang-Mills.

## 1. What my existing work contributes

### The data stays and the reading changes

In the Abelian control in *Reading the Curve*, edge integrals determine face
fluxes. The recovery reads those fluxes across a neighborhood and reconstructs
a closed linear curvature field in a specified chart. It does not invent extra
observations. The non-Abelian extension instead uses face holonomies and their
logarithms, with transport into a common internal frame; it cannot obtain that
curvature by simply adding edge logarithms.

Write the local vector proxy as

\[
P_K(x)=c+Bx,\qquad \operatorname{tr}B=0.
\]

Read this as "P sub K of x equals c plus B x; the trace of B is zero." Here
\(K\) is a cell, \(x\) a chart position, \(c\) a constant vector, and \(B\) a
three-by-three matrix. The trace condition makes the proxy divergence-free,
equivalently a closed two-form in this chart representation. There are eleven
independent coefficients.

The flux map \(X_K\), pronounced "X sub K," sends these coefficients to measured
face fluxes. The constrained recovery \(\mathcal R_K\), "script R sub K," preserves
the cell's own face fluxes and fits the neighboring ones. Under full column rank,

\[
\mathcal R_K X_K=I_{11}.
\]

That says "recovery followed after measurement returns the original eleven
coefficients." It is exact on the declared model space. It does not say that a
finite list of fluxes uniquely determines every smooth curvature field.

There is also an actual shape here. With the coefficient and flux inner products
fixed, the image of the coefficient unit ball under \(X_K\) is an ellipsoid in
its image subspace. Its singular values are its semiaxis lengths. A short axis
identifies a field direction that produces a small change in the measured fluxes.
Recovering that direction is sensitive to errors in those measurements.

That is not an illustration of conditioning. It is conditioning expressed as
geometry. Changing the coefficient scaling changes this ellipsoid unless the
metric is transformed with it. The paper's diameter-normalized chart and physical
mass pairing are part of the claim, not incidental implementation details.

The paper also distinguishes the *direction* of a force error from its total
magnitude. Error in rapidly damped directions can affect a trajectory differently
from error in slowly damped directions. Reading only the norm loses that
distinction. The reconstruction's full convergence argument still needs the
uniform constants and chart-distortion bounds identified in the paper's proof
sketch. [Reading the Curve, recovery, conditioning, and flow analysis](../papers/two-ring-pg/paper.tex).

### The mode is a field, not just an entry in a spectrum

In the pinned Floor manuscript, the twist has the form

\[
b=f\,(\hat\theta^1+i\hat\theta^2)\otimes(\tau_1+i\tau_2).
\]

Read "b equals f times theta-hat one plus i theta-hat two, tensor tau one plus
i tau two." The symbol \(i\) is the imaginary unit. The theta-hat terms are
spatial covectors; the tau terms are internal Lie-algebra directions. The
function \(f\) supplies the orbital dependence.

At a point, this field takes a spatial tangent direction as input and returns a
complexified internal direction. Across the manifold, it is a section of the
cotangent bundle tensored with the complexified adjoint bundle. A displayed
matrix represents that map in chosen frames; the matrix alone is not its full
identity. The background connection, metric, symmetry charge, and quotient
conditions all matter.

When an eigenvalue is repeated, an eigensolver can rotate its basis within the
eigenspace. I should compare the spectral subspace or its orthogonal projector,
or use a specified commuting symmetry to select the mode. A changing plotted
eigenvector is not automatically a changing physical field.

The Floor manuscript studies a **classical Hessian**, including a chosen
gauge-orthogonal compression at a nonstationary background. That operator is not
the physical quantum Hamiltonian introduced below. I will call them different
names even though the letter \(H\) is often used for either.

## 2. Fix the target before identifying its pieces

The official target is a nontrivial quantum Yang-Mills theory on four-dimensional
flat spacetime for each compact simple gauge group, with an appropriate rigorous
quantum-field framework and a positive, finite first excitation threshold. Its
local observables must have the prescribed Yang-Mills short-distance behavior,
including the renormalized curvature observables, stress tensor, and operator
product structure. Confinement and an isolated one-particle state are additional
questions, not substitutes for this target.
[Jaffe and Witten, sections 3-5](https://www.claymath.org/wp-content/uploads/2022/06/yangmills.pdf).

I separate that target into the following records. The details below explain
what the geometric descriptions mean; the table is not an alternative axiom set.

| Requirement | Kind of piece | Actual object or relationship to inspect |
| --- | --- | --- |
| Four-dimensional spacetime | Object and domain | Minkowski affine space with its causal cones; Euclidean space in a reconstruction route |
| Gauge group and field | Objects and actions | Principal bundle, connection, parallel transport, curvature, gauge transformations |
| Physical identification | Equivalence relation | Gauge orbits; not an arbitrary identification of similar-looking arrays |
| Quantum existence and positivity | Construction and test | Correlations defining a positive inner product, a completed state space, and operators on controlled domains |
| Vacuum and nontriviality | State and tests | An invariant vacuum ray, its generated states, and nonzero physical directions beyond it |
| Relativistic covariance and positive energy | Actions and support condition | Unitary spacetime symmetries and joint energy-momentum spectral support |
| Locality | Algebraic relation over geometry | Commuting observable operations in spacelike-separated regions |
| Yang-Mills short-distance identity | Compatibility under change of scale | Renormalized local observables and their coincident-point singularities |
| Removal of regulators, when used | Limit and uniformity tests | A compatible family of correlations, forms, and physical units |
| Mass gap | Property of a quadratic form | Uniform energy coercivity transverse to the vacuum |

My working record for each piece is:

**Object -> space -> metric or pairing -> allowed transformations -> measurable
geometry -> exact connection to the requirement.**

Not every space here is a smooth manifold. Some are affine spaces, singular
quotients, spaces of distributions, Hilbert spaces, or algebras indexed by regions.
Calling all of them Riemannian would lose information rather than recover it.

## 3. The base space and the internal space

**EXACT: different spaces have different jobs.**

Let \(M\), "M," be spacetime. In the Lorentzian description it is
\(\mathbb R^{1,3}\), "R one-three": one time and three space directions with the
Minkowski metric. Its light cones decide which regions can communicate causally.
Its metric is not a positive distance rule. In the Euclidean description the
base is \(\mathbb R^4\), "R four," with a positive metric. Passing between these
descriptions requires a reconstruction theorem, not merely changing a sign in
a numerical array.

Let \(G\), "G," be the compact gauge group; "simple" here concerns its
non-Abelian Lie algebra, not the absence of a finite center. The group gives
internal transformations, not additional spacetime directions. Its global form
must be specified. The requirement covers each permitted group; an SU(2)
construction alone does not cover them all.

The principal bundle \(P\to M\), "P over M," carries the internal frames. A
connection \(A\), "A," specifies parallel transport between fibers along paths.
Composing paths composes the transport operations. This is an actual map from
paths to transformations, not a resemblance between a curve and a matrix.
[Baez and Huerta, ordinary connections in section 2](https://math.ucr.edu/home/baez/invitation.pdf).

In an anti-Hermitian matrix convention the curvature is

\[
F_A=dA+A\wedge A.
\]

Read "F sub A equals d A plus A wedge A." The first term measures spatial
variation of the connection; the second retains the noncommuting internal
operations. Curvature is an adjoint-valued two-form: it accepts an oriented
tangent plane element and returns a Lie-algebra element. Infinitesimal loop
transport detects it; finite loop transport contains more than a pointwise
curvature value. [Tong, Yang-Mills theory, section 2.1](https://davidtong.org/pdfs/teaching/gauge-theory/gauge2.pdf).

This distinction is load-bearing for my programme. Curvature of the spacetime
metric, curvature of the connection, and curvature of a metric on a space of
configurations are three different objects. An identity between particular
constructions has to be established, with its map and hypotheses.

## 4. The field and the space of fields

**EXACT for the classical construction; additional choices are needed globally.**

The space of connections on a fixed bundle is affine. A difference between two
connections is an adjoint-valued one-form. That gives an actual tangent space;
Helike's complex field lives in its complexification.

A gauge transformation changes the internal frame. It transforms both the
connection and curvature according to their rules. Gauge-equivalent
descriptions belong to one orbit. The configuration quotient
\(\mathcal A/\mathcal G\), "script A modulo script G," identifies these orbits.
It can have singular strata because different connections can have different
stabilizers. I cannot assume it is one globally smooth coordinate chart.

On a spatial slice \(\Sigma\), "Sigma," a Riemannian metric and an invariant
internal inner product give the classical pairing

\[
\langle a,b\rangle_{L^2}
=\int_\Sigma \langle a(x),b(x)\rangle\,d\mathrm{vol}_\Sigma.
\]

Read "the L-two inner product of a and b is the integral of their pointwise
inner product against spatial volume." Infinitesimal gauge directions have the
form \(d_A\xi\), "d sub A xi," where xi is an infinitesimal frame change. Their
orthogonal complement obeys \(d_A^*b=0\), "d sub A adjoint b equals zero."
This is a metric-dependent local slice condition, not a global elimination of
every gauge ambiguity. For a non-Abelian group no continuous global slice exists
on \(S^3\) or \(S^4\) (Singer, the Gribov ambiguity), so the Coulomb slice of the
Floor paper is local by necessity, not by convenience.
[Singer 1978](https://doi.org/10.1007/BF01609471). Here xi is pronounced "ksee."

A double cover also has an exact job. The map from SU(2) to SO(3) identifies
the two group elements \(U\) and \(-U\), pronounced "U and minus U." Their
adjoint actions agree, while their fundamental matrices differ by sign. I must
specify which representation and observables retain that distinction. A
two-to-one group map does not by itself imply a twofold energy threshold.

In a canonical classical description, adding the conjugate electric field gives
a constrained phase space. Gauss' law is a constraint before quotienting; in a
quantum construction it must also be respected in the physical states or
observable formulation. A quotient of sampled field arrays is not itself a
quantization.

The quantum state space is another object. A wavefunctional, where such a
representation is justified, assigns amplitudes to configurations. It is not
one connection drawn with a different color. Nor is there a ready-made
infinite-dimensional Lebesgue measure that turns a formal expression into a
constructed theory.

## 5. Correlations can define geometry

**EXACT construction from a positive state; CONDITIONAL as a Yang-Mills route.**

Suppose there is a unital observable star-algebra and a normalized positive state
\(\omega\), "omega," assigning expectations. Positivity means
\(\omega(B^*B)\ge0\) for every algebra element \(B\); the star means adjoint.
Then set

\[
\langle[B],[C]\rangle=\omega(B^*C).
\]

Square brackets here denote equivalence classes after removing zero-norm
elements. Positivity supplies the Cauchy-Schwarz inequality, making the
quotient inner product well-defined. Complete the quotient in that norm.
Left multiplication gives the observable representation on its natural dense
domain, and the class of the identity gives a cyclic vector. This is the
Gelfand-Naimark-Segal construction.
[Fewster and Rejzner, section 2.3, Theorem 10](https://arxiv.org/pdf/1904.04051).

For a finite list of preparations, their correlation matrix is consequently a
Gram matrix. Its eigenvalues measure squared lengths in suitable combinations
of those preparations. A negative eigenvalue cannot be interpreted as an
ordinary Hilbert-space squared length. This is a geometric obstruction, not
just an inconvenient sign.

The abstract positive-state construction does not supply the missing
Yang-Mills correlations or their locality, covariance, and regularity. Its
usefulness is that it tells me exactly which data would define the geometry.

### Reflection positivity has a specific geometric role

In a Euclidean construction, take preparations \(B_i\) supported at positive
Euclidean times. Reflect time and take the adjoint on one side of the
correlation. Writing that combined operation as \(\Theta\), "Theta," define

\[
K_{ij}=\langle (\Theta B_i)B_j\rangle_E,
\qquad
\sum_{i,j}\overline{z_i}K_{ij}z_j\ge0.
\]

Read "K sub i j is the reflected Euclidean correlation; every complex linear
combination has nonnegative squared length." The bar means complex conjugation.
The test is for **every** admissible finite family, not only one chosen matrix.
Zero-norm combinations are quotiented out; completion gives the physical
Hilbert space in the reconstruction. Reflection positivity is therefore a
test that the reflected correlations can be an inner product.
[Jaffe, sections I-II](https://www.arthurjaffe.com/Assets/pdf/SPDE-QFT-v3.pdf).

A positive probability distribution is not enough: its ordinary covariance and
its time-reflected correlation form are different pairings. A finite positive
Gram matrix is evidence only for that family of preparations. Reconstruction
also needs the relevant Euclidean symmetry, permutation, and growth/regularity
conditions; the vacuum sector needs the appropriate clustering condition.
[Osterwalder and Schrader, reconstruction framework](https://doi.org/10.1007/BF01608978).

## 6. What the reconstructed objects must do

### A vacuum is a ray, not the zero vector

Let \(\mathcal H\), "script H," be the physical Hilbert space, and let
\(\Omega\), "Omega," be a normalized vacuum vector. Multiplying it by a
constant complex phase describes the same ray. The zero vector is not a state.

The vacuum is invariant under spacetime symmetries. In the chosen vacuum
representation it is unique up to phase, and applying the observable fields to
it generates a dense set of states. Nontriviality requires more than this one
ray. There must be physical states and correlations beyond constant multiples
of the vacuum. These are properties of the representation, not features one
can establish from the appearance of a single classical field.

### Symmetries are actual actions on the state space

On the separable physical Hilbert space, the proper orthochronous spacetime
symmetries (or their covering group) have a strongly continuous unitary
representation. They preserve the inner product and transform local fields
covariantly. Strong continuity means a small symmetry change moves each state
by a small Hilbert-norm amount. Their translation
generators have joint spectral support in the closed future light cone. That
cone is a literal geometric support condition in energy-momentum space.
Positive energy alone does not replace the full support condition.
[Etingof, sections 11.7-11.9](https://ocw.mit.edu/courses/18-238-geometry-and-quantum-field-theory-spring-2023/mit18_238_s23_lec_full.pdf).

### Locality is geometry together with an algebraic relation

Associate observables with spacetime regions. For bosonic observables supported
in spacelike-separated regions, the two orders of operation agree:

\[
[\mathcal O_1,\mathcal O_2]=0.
\]

Read "the commutator of O one and O two is zero." Here brackets mean
\(\mathcal O_1\mathcal O_2-\mathcal O_2\mathcal O_1\), not an equivalence class.
The causal geometry identifies which region pairs qualify; the algebra says
what must commute. Entangled correlations may still be present. Locality is
not statistical independence and not a distance cutoff.

### Fields are not necessarily values at points

The local quantum fields are operator-valued distributions. They are smeared
against smooth test functions before being applied on a common invariant dense
domain. Temperedness controls their growth and singularity in the distributional
sense; adjoints and products require compatible domains. The Wightman framework
contains these analytic conditions as well as symmetry and locality.
[Schlegelmilch, section 1.1.1](https://www.physik.uni-hamburg.de/th2/ag-fredenhagen/dokumente/desy-thesis-05-036.pdf).

I can geometrically locate a distribution's support and track how it transforms.
I cannot honestly turn all domain and continuity conditions into a finite
surface. Here the faithful object includes analysis, not only a picture.

## 7. The gap as the shape of fixed-energy sets

**EXACT equivalence, conditional on having the physical quantum operator.**

This is where the change of attention gives me a particularly precise object.
Instead of asking only for the lowest energy number, I ask how far a
vacuum-orthogonal vector can extend while its energy form stays bounded.

### Objects and hypotheses

Let \(H_{\mathrm{phys}}\), "H physical," be a nonnegative self-adjoint Hamiltonian
on \(\mathcal H\). Suppose its kernel is exactly the vacuum line. Let

\[
\mathcal H_0=\Omega^\perp,\qquad
q(\psi)=\|H_{\mathrm{phys}}^{1/2}\psi\|^2.
\]

Read "script H zero is Omega perpendicular; q of psi is the squared norm of
H physical to the one-half applied to psi." Psi is pronounced "sigh." The space \(\mathcal H_0\) consists
of vectors orthogonal to the vacuum. Assume it is nonzero. The form domain
\(D(H_{\mathrm{phys}}^{1/2})\), "domain of H physical to the one-half," is the set
where this expression is defined and finite.

I use the quadratic form, not an unrestricted expression involving
\(H_{\mathrm{phys}}\psi\): the operator itself can require a smaller domain.
Define

\[
\Delta_*=
\inf_{\substack{\psi\in D(H_{\mathrm{phys}}^{1/2})\cap\mathcal H_0\\
                 \|\psi\|=1}}q(\psi).
\]

Read "Delta star is the infimum of q over unit, vacuum-orthogonal vectors in
the form domain." The spectral theorem identifies it with the bottom of the
spectrum restricted to \(\mathcal H_0\). The gap condition is
\(\Delta_*>0\). Nonzero form-domain vectors give a finite upper bound on this
infimum, so the nontrivial sector prevents a vacuous infinite threshold.

Choose a positive energy budget \(E_*\), "E star," in fixed physical units, and
form the set

\[
\mathcal C_{E_*}=
\{\psi\in D(H_{\mathrm{phys}}^{1/2})\cap\mathcal H_0:q(\psi)\le E_*\}.
\]

Read "script C sub E star is the set of vacuum-orthogonal vectors whose energy
form is at most E star." **These vectors are not normalized.** On the unit
sphere, boundedness would be automatic and would tell me nothing.

### The exact statement

The following conditions are equivalent:

1. The restricted spectrum has a positive lower bound \(\Delta_*\).
2. There is a positive \(\Delta\), "Delta," such that
   \(q(\psi)\ge\Delta\|\psi\|^2\) throughout the vacuum-orthogonal form domain.
3. For one, and therefore every, positive \(E_*\), the set
   \(\mathcal C_{E_*}\) is bounded in the Hilbert norm.

Moreover, its radius is exactly

\[
R_{E_*}:=\sup_{\psi\in\mathcal C_{E_*}}\|\psi\|
=\sqrt{\frac{E_*}{\Delta_*}}.
\]

Read "R sub E star is the square root of E star divided by Delta star." The
supremum need not be attained. A lowest isolated eigenvector is not required.

**Argument.** The spectral lower bound gives the form inequality. The inequality
gives \(\|\psi\|^2\le E_*/\Delta\) on the sublevel set. Conversely, suppose that
set has finite radius \(R\). Every nonzero vector in the restricted form domain
has positive form value, since the vacuum kernel has been removed. Scale it by
\(\sqrt{E_*/q(\psi)}\). The scaled vector lies in the set, so
\(q(\psi)\ge(E_*/R^2)\|\psi\|^2\). Taking an infimizing sequence of unit vectors
gives the radius equality. This is a standard quadratic-form consequence of
the spectral theorem, written out here to expose its geometry.

This is the same kind of object as the flux ellipsoid of Section 1, read the
other way round. There a short axis is a direction the measurements barely
see; here a long axis would be a direction the energy barely charges for.

In finite dimension, diagonalization makes the sublevel set an ellipsoid with
semiaxes \(\sqrt{E_*/\lambda_j}\), "the square root of E star over lambda sub j."
The quantities \(\lambda_j\) are the positive energy eigenvalues. In infinite
dimension, use the spectral form, not an assumed list of eigenvectors.

**In my plain words:** after removing the vacuum direction, a fixed energy
budget cannot support vectors of arbitrarily large norm. If it can, there are
directions whose energy cost per squared norm approaches zero. The mass gap is
the absence of that escape in the physical energy form.

The center of this set is the zero vector, not the vacuum. These unnormalized
vectors are a device for measuring the form, not a claim that physical states
have variable normalization. Nor does the gap give a minimum positive average
energy for every normalized superposition containing a small nonvacuum component;
the orthogonality condition matters.

### Two further readings of the same form

The semigroup identity gives another exact test:

\[
\|e^{-sH_{\mathrm{phys}}}|_{\mathcal H_0}\|
=e^{-s\Delta_*},\qquad s>0.
\]

Read "the operator norm of exponential minus s H physical on script H zero
equals exponential minus s Delta star." This is uniform contraction under
Euclidean-time evolution, not loss of norm under real-time quantum evolution,
which is unitary. Auxiliary optimization or Ricci-flow time is neither of these
merely because it is called time.

For a particular vector \(v\), "v," in \(\mathcal H_0\),

\[
\langle v,e^{-sH_{\mathrm{phys}}}v\rangle
=\int e^{-s\lambda}\,d\mu_v(\lambda).
\]

Read "the correlation is the integral of exponential minus s lambda against
mu sub v." The measure \(\mu_v\) is the energy spectral measure seen by that
vector. A probe can miss the slowest sector because it has no overlap with it.
One rapidly decaying correlation is therefore not a uniform gap certificate.

### Counterexamples that keep this conversion honest

| Object | Exact result | What it rules out |
| --- | --- | --- |
| On square-summable sequences, energy diagonal \(0,1,1,1,\ldots\) | Gap one; the fixed-energy unit sublevel contains infinitely many orthonormal vectors | Bounded does not mean compact |
| Energy diagonal \(0,1,1/2,1/3,\ldots\) | No positive gap; the vector \(\sqrt n\,e_n\) has energy one and norm \(\sqrt n\), for excitation index \(n\ge1\) | Every finite truncation can have a positive gap while the limiting gap is zero |
| A vacuum line plus \(L^2([1,2])\), with excitation energy multiplication by the coordinate | Gap one, but no eigenvector at energy one | A gap does not require a discrete lowest particle state |
| A Hilbert space consisting only of the vacuum line | No nonvacuum directions to test | An empty excitation sector cannot establish a nontrivial theory |

Here \(e_n\), "e sub n," denotes an orthonormal basis vector, and \(L^2\),
"L two," denotes square-integrable functions. These are analytic countermodels,
not Yang-Mills simulations.

## 8. When the gap becomes a weighted geometric inequality

**CONDITIONAL: a precise route, not an assumed continuum representation.**

There is a second construction worth keeping because it places the geometry on
a configuration space rather than only on a Hilbert-space sublevel set.

Suppose a regulated model has a Riemannian configuration space \(Q\), "Q," a
Schrodinger operator

\[
H=-\kappa\Delta_g+V,
\qquad H\psi_0=E_0\psi_0,
\qquad \psi_0>0,
\]

and a normalized ground state. Read "H equals minus kappa Laplace sub g plus V;
psi zero is a strictly positive ground state with energy E zero." The constant
\(\kappa>0\) fixes the kinetic normalization, \(g\) is the metric, and \(V\) is
the potential. The Laplacian convention makes \(-\Delta_g\) nonnegative. Assume
self-adjointness and boundary/domain conditions that justify integration by
parts, first on a core and then by form closure.

Define \(d\nu=\psi_0^2d\mathrm{vol}_g\), "d nu equals psi zero squared times
metric volume," and write a state as \(\psi=\psi_0 f\). Direct expansion,
integration by parts, and the ground-state equation give

\[
\|\psi_0f\|^2=\int_Q|f|^2\,d\nu,
\qquad
q_{H-E_0}(\psi_0f)=\kappa\int_Q|\nabla_g f|^2\,d\nu.
\]

Read "the norm becomes the weighted integral of f squared; energy above the
ground state becomes kappa times the weighted squared gradient." The cancellation
can be seen before integration: expanding the kinetic term gives
\(\psi_0^2|\nabla_g f|^2\) plus
\(\nabla_g\psi_0\cdot\nabla_g(\psi_0|f|^2)\), read "gradient psi zero dot
gradient of psi zero times the squared magnitude of f." After integration by
parts, this second term combines with the potential into the ground-state
equation and vanishes. Vacuum
orthogonality becomes \(\int_Q f\,d\nu=0\), zero weighted mean.

The gap inequality is now the weighted Poincare inequality:

\[
\int_Q\left|f-\int_Q f\,d\nu\right|^2d\nu
\le\frac{\kappa}{\Delta}\int_Q|\nabla_g f|^2d\nu.
\]

"Poincare" is pronounced "pwahn-kah-RAY." The left side is weighted variance.
The right side measures variation across the actual configuration geometry.

This is not a claim that any curved space has a mass gap. The metric, measure,
domain, and kinetic normalization all participate. A function separating two
regions with substantial measure, while changing mainly through a region of
very little measure, is a candidate for small gradient cost relative to its
variance. Such functions reveal where the inequality may weaken.

For Yang-Mills I would have to construct and justify the physical version of
these objects, including gauge constraints and the regulator limit. A smooth
quotient and a positive ground-state function cannot simply be assumed in the
continuum. This is a concrete question I can investigate; it is not yet a map
from Helike's classical eigenspace to quantum states.

## 9. Scale changes must preserve the object being claimed

**CONDITIONAL on a regulated route; the continuum target is mandatory.**

A lattice supplies genuine finite objects: group elements on oriented edges,
products around faces, a specified action, and a measure. A finite lattice
integral is not yet the continuum quantum field. In a lattice route, the
correlations and required structures must converge as the short-distance
cutoff is removed and the physical volume grows. A direct construction need
not use a lattice. [Douglas, pages 1-2](https://www.claymath.org/library/annual_report/douglas_quantum_yang_mills.pdf).

Different resolutions do not automatically inhabit one common vector space.
I need explicit embeddings, coarse-graining maps, or a shared observable
algebra with which to compare them. I also need a declared topology of
convergence. A sequence of attractive plots supplies neither.

In a transfer-operator setting, if a dimensionless lattice Hamiltonian obeys
\(H_{\mathrm{lat}}=a_t H_{\mathrm{phys}}\), then

\[
\Delta_{\mathrm{phys}}=\Delta_{\mathrm{lat}}/a_t.
\]

Read "the physical gap is the lattice gap divided by the time spacing." This
uses units with Planck's reduced constant and the speed of light set to one,
and \(a_t\), "a sub t," is the Euclidean-time spacing. A lattice gap going to
zero can therefore be consistent with a finite physical gap. Units must be
fixed before deciding what a curve means.

My geometric version of the uniformity question is whether the energy-sublevel
radii, in the correctly identified physical norm and units, remain uniformly
bounded along the limiting construction. To transfer that bound to a limiting
Hamiltonian still requires suitable form/operator convergence, compatible
vacuum projections, and control of all physical directions. Newly appearing
low-energy directions cannot be omitted by a finite-rank comparison map.

Short-distance Yang-Mills identity is a separate requirement. Renormalized
curvature observables must have the prescribed asymptotically free behavior;
the correspondence with classical curvature polynomials includes operator
mixing and is not a naive one-to-one substitution. The stress tensor and
operator product expansion test the local singular structure as observation
points approach one another.
[Jaffe and Witten, section 4 and its footnote](https://www.claymath.org/wp-content/uploads/2022/06/yangmills.pdf).

The geometric place to look is the family of local observables under rescaling,
including its behavior near coincident-point diagonals in \(M^n\), pronounced
"M to the n": the space of n spacetime observation points. This is not the
configuration space of whole gauge fields. Choosing a metric on a space of
effective actions could be useful, but
there is no unique such metric supplied by the phrase "renormalization group."
I will not substitute an invented distance for the required compatibility.

## 10. What this asks of Helike and of GIGI

The useful inheritance is a method and a set of tools, not an identification of
two operators because both have spectra.

The Berger and Poincare geometries are controlled compact backgrounds for my
classical work. Their finite spatial size and geometry can themselves create
spectral separations. Those separations cannot be carried over to flat,
infinite-volume quantum Yang-Mills without the intervening construction.

For a classical mode, I can retain its field reconstruction, mass normalization,
gauge content, symmetry sector, spatial support, and response to covariant
transport. Where degeneracy is present I can retain a subspace projector. Where
an operator is non-normal I can inspect its numerical range and singular
directions in the physical pairing rather than trusting eigenvalues alone.

These last diagnostics matter for the Petrov-Galerkin force and its numerical
evolution. The physical quantum Hamiltonian, by contrast, must be self-adjoint.
A Crouzeix bound for a non-normal discrete force does not establish positivity
or a gap for that Hamiltonian. Holomorphic functional calculus may help control
an approximation after the operator correspondence is established; it cannot
supply that correspondence by itself.

For a candidate quantum construction I would need different records:

- Gauge-invariant observable preparations and their regions of support.
- Ordinary and reflected correlations, with the two pairings distinguished.
- Their null directions, positive Gram subspaces, and error enclosures.
- A justified time-translation or transfer operator, with units and domain.
- Vacuum projections and the physical energy form on their complements.
- Comparison maps across resolutions, volumes, and symmetry sectors.
- The measurements and uniform estimates that support any claimed limit.

These are proposed research records, not a claim that new GIGI verbs already
implement a quantum construction. The next implementation spec must map them
to actual engine contracts and identify what is missing.

Two clarifications belong beside my earlier notes. Curvature of the group does
give something exact: on a compact manifold whose Ricci curvature is at least
\((n-1)k>0\), the first nonzero eigenvalue of the Laplacian is at least \(nk\)
(Lichnerowicz). A Hamiltonian lattice gauge theory contains such Laplacians: its
electric term is a sum of group Laplacians, one per link (Kogut and Susskind).
But the full Hamiltonian adds a nonnegative magnetic term, and its vacuum is not
the electric term's vacuum, so a gap for the electric term is not a gap for the
whole. The electric term's scale is also set by the bare coupling, which
asymptotic freedom sends to zero in lattice units as the spacing shrinks. The
curvature bound exists; it is not, by itself, a lower bound on the physical gap.
[Kogut and Susskind 1975](https://doi.org/10.1103/PhysRevD.11.395). Also, the suggestion of using flow time as a fourth coordinate in
the November 19 strategy note needs a separate construction: dissipative
classical evolution is not automatically physical time or a Euclidean quantum
field. I retain the earlier notes as research history; these distinctions
govern the present one.

## 11. The tests I would require before advancing this route

These are proposed acceptance tests, not reported engine results. They test
the conversion first so that a later experiment cannot pass by using the wrong
object.

| Test | Required distinction | Deliberate failure it must detect |
| --- | --- | --- |
| Metric fidelity | A basis change with the transported metric preserves norms and generalized spectra | Treating raw coefficient length as physical length |
| Gauge fidelity | Gauge-equivalent data give equivalent observables and transported field descriptions | A frame rotation reported as a physical change |
| Mode fidelity | A degenerate spectral projector is unchanged by rotating its eigenbasis | An arbitrary eigenvector drawing treated as unique |
| Reconstruction fidelity | Model-space recovery reproduces known fields and preserves the specified measurements | Fitting a desired answer by changing the source data |
| Positivity | The reflected pairing is nonnegative for the tested family, with certified error bounds | A negative direction hidden by rounding, clipping, or using ordinary covariance instead |
| Gap geometry | A prescribed positive operator gives the predicted sublevel radius | Testing only normalized vectors or retaining the vacuum direction |
| Limit sensitivity | The diagonal sequence with energies \(1/n\) is identified as lacking a uniform lower bound | Reporting a positive gap from every finite truncation |
| Probe coverage | An observable orthogonal to a planted low-energy sector cannot certify the full gap | Mistaking its fast decay for a universal bound |
| Continuum identity | Units, comparison maps, local observables, and vacuum sector remain consistent | Comparing different operators merely because their curves look similar |

Removing the mechanism should make its corresponding gate fail. Finite tests
can reject a construction and validate a numerical implementation. They do not
replace the universal estimates or axioms in the continuum requirement.

## 12. The question I want to carry forward

**OPEN: identify and construct the quantum objects to which the exact
reformulations would apply.**

The most useful question is not yet "is Helike the mass gap?" That jumps over
the object I need to construct.

It is:

**Can I identify the physical configuration or state-space geometry in which
the Yang-Mills energy form has a uniform positive cost in every direction
orthogonal to the vacuum, and show that this geometry survives the quantum
construction and its limits?**

Helike gives me a reason to inspect the directions rather than only their
eigenvalues. *Reading the Curve* gives me a reason to examine how measurements,
reconstruction, and pairing determine which directions I can see faithfully.
Neither tells me in advance what the quantum answer must be.

The missing pieces are now specific: which physical measure or correlation
functional, which energy form, which identification across scales, and which
estimate controls every remaining direction. If I cannot name one of these,
that is where I need to stop and work, not where I need a more persuasive
analogy.

I am keeping the data. I am keeping the equations. I am changing what I attend
to inside them.

## Sources and development record

### My source material

- [Reading the Curve, Not Counting the Points](../papers/two-ring-pg/paper.tex),
  especially the recovery, conditioning, force-error decomposition, and
  limitations. Version 3 as posted (doi:10.5281/zenodo.24386756, all versions
  doi:10.5281/zenodo.24328085): tag `reading-the-curve-v3`, source SHA-256
  `fce414f64244236785b5fd5d7ad18fa5c3e02b397a9b8b1511ead52470571158`.
- *The Floor Theorem for the Yang-Mills Hessian at the Berger Field*, version
  3.3 source, tag `floor-theorem-v3.3`, Git object
  `8f51dfe17e6ff6b4a3ef5d60f66417e2adc3f90b:docs/papers/berger-spectrum/paper.tex`
  (posted as version 3, doi:10.5281/zenodo.24471504).
  The field definition, slice convention, charge structure, and scope were the
  parts used here; this note does not rerun the manuscript's certificates.
- *The Potter's Toolkit*, local research note
  `docs/research/potters-toolkit-2026-11-17.md` (not included in this publication).
- [Small Edges](small-edges-2026-11-18.md),
  [The Curvature of One Unit](unit-curvature-2026-11-18.md), and
  [Yang-Mills Inside GIGI](yang-mills-strategy-2026-11-19.md).
- [Coordinate Translations and Sameness](coordinate-translations-and-sameness.md),
  for the distinction between a chosen coordinate representation and a
  metric-consistent geometric identification.

### External mathematical references

The linked passages above identify the source of the problem statement,
standard gauge geometry, and quantum-field frameworks. The quadratic-form
radius equivalence and the conditional ground-state transformation are derived
in the text; they are standard mathematics, not originality claims. Historical
status discussions in older references are not used as present-day status
reports.

### Authorship and verification scope

The change-of-attention method, the requirement map, the research motivation,
and the connections to Helike and *Reading the Curve* are mine. Each
reformulation was checked against the primary references cited beside it.

The scope here is a mathematical and source-level research note. No new
Yang-Mills computation, certificate, engine feature, or continuum-limit result
is claimed. Existing source files were left unchanged. The sources are
identified by their tags and hashes above.
