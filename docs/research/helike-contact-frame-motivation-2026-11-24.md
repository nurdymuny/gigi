# Contact sets the frame: the Bevel razor observation

24 November 2026. **Research motivation and a proposed geometric translation.**
This fuller account was written after the registered dome experiment. It does
not amend that experiment's preregistration, thresholds or results. No new
numerical experiment is reported here.

## Where the idea came from

This idea came from the razor I use. I want to preserve that part of the story
because the experience is not decoration added after the mathematics. It is
where I noticed the mechanism.

As a person of color, I have had to pay attention to what shaving does to my
skin. The multiple-blade razors I have tried give me razor bumps. I use a
single-blade Bevel razor because it works better for me. I want a close cut,
but not a cut that is too close for my skin. That is my experience and the
practical constraint behind this observation, not a medical claim about every
person or every razor.

The part that caught my attention was the rounded head and how it meets the
curvature of my face. There are two shapes in contact, with the blade held in
a definite relationship to the head. That contact gives my hand a reference
for the blade's approach. I do not have to see the exact angle at every point
to make use of that relationship.

I know where my hair is. I know which way it grows in different places. I
choose where to move and in which direction. But that knowledge is not the
same thing as knowing the blade's local angle at every instant. The contact
helps with a part of the operation that my map of the hair does not settle.

That is the distinction I was trying to carry into the calculation:

**I supply knowledge of the direction. The contact supplies a local reference
for how the operation meets the surface.**

The razor does not know my face the way I do. It does not need to. Its shape
can do useful work through contact while I do a different part of the work.
I am interested in that division of responsibility, not just in the fact that
the head happens to be curved.

## Contact is a relationship, not just a location

Bevel's own guide describes establishing the approach by placing the head
against the skin and then tilting until the blade makes contact. It also
distinguishes that positioning from choosing the stroke direction according
to hair growth. This supports the distinction in my observation; it is not
a measurement of the head's engineering dimensions or a proof of an optimal
angle. [Bevel shave guide](https://bevel-assets.s3.amazonaws.com/how-to-shave/shaveguide-download.pdf)

I should not describe this as an automatic angle optimizer. My hand, pressure,
motion, the head and the blade position still matter. What the geometry gives
me is a contact reference and restrictions on the relative position of the
parts. It does not replace all judgment.

The surfaces also do not have to possess identical curvature to make contact.
Two smooth surfaces can share a tangent plane at a contact point while bending
differently away from it. The original note's phrase about matching curvature
should not be read as a demonstrated equality of the razor's curvature and
the skin's curvature.

A minimal geometric description separates the pieces:

| Piece | Geometric role |
| --- | --- |
| The skin near a contact point | A local tangent plane and a normal direction; both can change as the contact moves |
| The razor head | A shaped support that constrains the blade's pose relative to contact |
| The blade | An edge and a blade plane fixed relative to the head; these are not the same object |
| Hair direction | A locally varying preferred direction for the stroke, supplied by my knowledge of the region |
| My motion | A path and a changing pose that must maintain the relevant contact relationship |
| A useful result | Cutting sufficiently closely for the task without treating maximum closeness as the only objective |

For an idealized smooth, nonpenetrating contact between two bodies, their
outward unit normals at the common contact point oppose one another:

\[
\mathbf n_{\rm head}=-\mathbf n_{\rm skin}.
\]

Here each n, read "normal," points perpendicularly out of its body's surface.
This is a local idealization, not a full model of deformable skin, force or
friction. One contact condition alone does not determine the entire pose.
The head's other geometry, additional contacts and the chosen motion matter.

To distinguish the approach from the blade's sharpening bevel, let
\(\mathbf n_{\rm blade}\) be a unit normal to the blade's plane at its current
pose. The smaller unoriented angle between that plane and the skin's local tangent plane is

\[
\alpha=\arccos\!\left(
  \left|\mathbf n_{\rm blade}\cdot\mathbf n_{\rm skin}\right|
\right).
\]

Alpha is the angle; arccos, read "arc cosine," converts the absolute dot product
of the unit normals into the angle between the planes. This is a geometric
definition, not a numerical model of the Bevel razor. It deliberately forgets
orientation signs; it does not specify which way to move the blade or predict
cutting depth. A complete physical description would need those quantities too.

My screenshot titled "Geometry of a Bevel Razor" discusses generic
straight-razor sharpening geometry. The angle forming an edge is not the angle
at which a mounted blade meets the skin. I am not using that screenshot's
formula as an engineering specification for the Bevel safety razor.

## What reached the first experiment

The [original dome note](helike-dome-envelope-preregistration-2026-11-24.md)
preserved the observation that contact sets the frame locally. It then narrowed
the computational question to placement: our existing trial functions were
centered at the origin, and the model has another classical vacuum orbit.
Would adding functions seated around that orbit improve the scalar state?

That was a concrete, testable question. It was not yet the full contact question.

The implemented envelope was a Haar average of a Gaussian with one width in
all directions, with a finite series used for the average. The orbit average
itself has nontrivial structure; calling the Gaussian isotropic does not mean
that the resulting function is simply radial about the origin. But the
construction did not independently fit directional widths to local curvature,
or impose a contact-angle rule between an operator-defined frame and the trial
functions.

**Getting the center right is not the same as getting the relationship right.**

The [recorded results](helike-dome-envelope-results-2026-11-24.md) stand:
five added functions improved the scalar residual at all eight primary points,
by about 1.2% to 33.1%, and beat ordinary expansion per added function at three
of eight points. A secondary one-function comparison changed its efficiency
verdict when the series order changed from 30 to 36. The registered scientific
verdict remains `UNRESOLVED_TRUNCATION`.

This clarification does not explain that outcome by itself. In particular,
it does not establish that an omitted contact rule caused the mixed gains, or
that a more directionally shaped envelope would fix them. I am preserving the
origin of the next question, not changing the score of the previous test.

## The two kinds of geometry I want to keep together

The [sector construction](sector-review-results-2026-11-24.md) uses the exact
symmetry of the states to organize the calculation. It tells us which kinds of
state are being represented and which transformations their representation must
respect. That is established mathematical structure, not information supplied
by the razor analogy.

The contact observation asks a complementary question: within that structure,
can the geometry of the problem determine how the representation is locally
oriented and shaped?

The scalar dome experiment already combined the invariant sector basis with
additional orbit-centered functions. The next conceptual step is not to blend
two answers. It is to construct one admissible trial space in which both
sources of geometric information are used, with all overlaps and Hamiltonian
cross terms accounted for.

I do not want to give the word "contact" a mathematical meaning by declaration.
The following are candidate counterparts whose definitions would have to be
fixed before testing:

| Role in the observation | Candidate computational counterpart | Current status |
| --- | --- | --- |
| Knowing the appropriate kind of motion | Choosing and preserving the exact symmetry sector | Implemented in the sector solvers; not literally a hair-direction field |
| Having a local surface reference | A local frame derived from explicitly identified geometry of the model | Proposed; the source of the frame must be specified |
| Holding the blade in a useful relationship to that reference | Aligning trial-function directions and widths with that frame | Proposed; an angle or alignment error must be defined using a stated metric |
| Maintaining contact while moving | Transforming the frame and trial functions consistently as the configuration changes | Proposed; derivative and symmetry terms cannot be omitted |
| Testing whether the engagement actually works | Measuring the full-operator residual and comparison cost | Existing measurements can assess candidates; they do not guarantee a good construction |

One possible starting point is the Hessian of the potential: the matrix
describing its second-order change near the vacuum orbit. Its principal
directions could supply a directional frame, and a positive quadratic
approximation in directions normal to the orbit could suggest different
Gaussian widths. That is a candidate construction, not a demonstrated answer.

The mathematical objects must stay distinct. The Hessian of the potential is
not automatically the Hessian of the superpotential W, a Riemann curvature
tensor, or one of GIGI's statistical curvature quantities. A physical change
of configuration is not the same thing as a gauge transformation. Directions
along a symmetry orbit cannot simply be assigned a positive confinement width
because that would make the Gaussian convenient.

Nor does a local normal-frame formula automatically define a globally valid
trial function. An extension must be smooth enough for the Hamiltonian,
normalizable, and preserve the required symmetry. If the frame changes with
position, the kinetic operator differentiates that change as well. A contact
analogy is not permission to leave those terms out.

## The question this leaves us

Can the geometry of the approximation regulate how it engages the problem,
instead of relying only on increasing the number of functions?

To make that question operational, a future specification must identify:

1. The actual mathematical object supplying the local reference, including
   its metric, domain and symmetry behavior.
2. The measurable relationship corresponding to the approach angle, and
   what changes when that relationship is imposed versus removed.
3. A globally admissible construction, including overlap, derivative and
   Hamiltonian terms, rather than only a locally appealing shape.
4. Independent approximation controls: origin cutoff, envelope resolution
   and arithmetic precision must not be conflated.
5. A frozen comparison against the successful sector baseline, reporting
   residuals, conditioning, time and memory, with the mechanism deliberately
   removed in a control.

These are requirements for a future experiment, not a new preregistration or
an authorization to run one. The existing truncation question remains open.
There is no claim here of an optimal angle in the quantum model, a demonstrated
tunnelling mechanism, or a continuum mass-gap result.

What I want preserved is the observation itself. I was not simply thinking
that a curved tool might work on a curved surface. I was noticing that the
relationship between them does part of the work. I know where to move. The
contact gives me a local reference for how to meet what is there.

That is what I want the mathematics to capture.

## Addendum: the model already supplies the local frame

*Added after review, the same day. Analytic; no new experiment.*

The first open requirement above asks what object supplies the local reference. At the second vacuum the model
answers it exactly. At any critical point of W the gradient vanishes, so the Hessian of the potential is the
square of the Hessian of W:

  Hess V = (Hess W)².

So in this model the two Hessians I was careful to keep apart are tied together at the vacua. The eigenspaces of
Hess W give the local decomposition, and the absolute eigenvalues give harmonic inverse-width coefficients.
Degenerate eigenspaces do not select a unique basis. At q₀ = I/g:
- the breathing direction (δ ∝ I) has μ = −1;
- the five symmetric traceless shape directions have μ = +2;
- the three orbit directions have μ = 0.

None of this depends on g.

The orbit directions are exactly the ones my note warned should not get a confinement width for convenience, and
the model agrees: μ = 0 there. Haar averaging can preserve the orbit symmetry, as it does for the existing dome.
But zero tangent stiffness is not permission to use a nonnormalizable degenerate Gaussian in ambient coordinates.
A contact-matched candidate needs a global extension with the six normal harmonic inverse-width coefficients,
finite norm, correct symmetry and every overlap and frame-dependent kinetic term retained. The local calculation
supplies the normal geometry; it does not complete that global construction.

The same Hessian gives a local energy count. Because the Euclidean Laplacian of W is 9, H = H_W + 9/2, where H_W is the 0-form part of
Witten's supersymmetric Hamiltonian. Its harmonic ground energy at a critical point is ½Σ(|μ| − μ): 0 at the origin
and 1 at the second vacuum. That leads to a weak-coupling prediction and a candidate mechanism for the crossing,
recorded in [the weak-coupling note](helike-second-vacuum-weak-coupling-2026-11-24.md). It is a candidate, not a
result.

## Addendum: the direction of the sweep

*Added the same day. Analytic, with a test; no new experiment.*

One thing I left out of the original observation is the direction of the sweep. In a given patch, going against
the grain gives a much closer shave than going with it, close enough that in some sections the Bevel no longer
protects me. The difference is night and day. My angle α = arccos|n_blade·n_skin| deliberately forgot that sign. The
model does not forget it.

**The protection has a direction.** The same Hamiltonian factors two ways:

  H = ½A†A + 9/2,  A = ∇ + ∇W,
  H = ½A′†A′ − 9/2,  A′ = ∇ − ∇W.

Both are exact on the stated flat-coordinate operator domain. Factoring with W exposes the floor 9/2.
Factoring with -W gives only -9/2, weaker than H >= 0. The stronger bound remains true of the same Hamiltonian;
changing the factorization hides it rather than physically removing it. This is the precise part of the sweep
observation that the algebra captures.

**S³ supplies two canonical Lie-group framings.** These are the left-invariant and right-invariant framings,
not the only possible global frames or direction fields on S³. Choosing a fixed unit Lie-algebra direction in
each gives opposite-handed Hopf circles. A face patch is not a closed two-sphere, so the hairy-ball theorem
does not force facial-hair whorls. The framings'
Maurer–Cartan equations differ by a sign: dθ + θ∧θ = 0 for θ = g⁻¹dg, and dθ̄ − θ̄∧θ̄ = 0 for θ̄ = dg g⁻¹. That
sign reaches the cubic term. The opposite grain alone is the same model with g → −g, its second vacuum on the
det < 0 orbit: V(−p; g) = V(p; −g).

**Our model shaves with one grain.** The nine homogeneous modes use the left-invariant frame only. Its two local
vacuum Hessians differ, and the truncation is not closed under parity exchanging the mode families. Implementing large-gauge
equivalence requires a quantum domain and boundary conditions, not merely adding the other family. The experiential
contrast motivates testing the larger model; it does not identify its mechanism in advance.

**Patches on a grid.** Each patch has its own local frame. Neighbouring frames are related by how the grain turns
between them, and the cut depends on the sweep relative to the local frame. That is a bundle with a connection, and
on a grid it suggests frames at sites, transport on links and curvature around plaquettes. A literal lattice gauge
model additionally needs a specified group, link variables, action and transformation rules. Cutting too close
does not establish that a frame turns fastest there; that causal step is not measured by the analogy.

Tests: `scripts/test_ym_helike_second_vacuum.py` (`SweepDirection`) checks both factorizations against finite
differences and the grain-reversal identity. The model that keeps both grains is scoped in
[helike-two-grain-scope](helike-two-grain-scope-2026-11-24.md).
