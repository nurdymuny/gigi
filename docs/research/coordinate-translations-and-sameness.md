# Coordinate translations, Jacobians, and the Geometry of Sameness

Reference ID: `DG-REF-COORDINATE-SAMENESS-001`  
Recorded: 2026-10-24  
Purpose: shared research memory for Bee and collaborators; not a public post or an originality claim.

## The connection to keep

Classical geometry explains how different coordinate descriptions refer to the same object and how measurements translate between them. Bee's Geometry of Sameness asks when translator-based and manifold-based systems realize the same semantic sameness structure, with controlled error in transferring detection guarantees.

The polar-coordinate example is a classical teaching example of the translation/geometry relationship. It is not by itself a proof of the manuscript's full equivalence claim.

This splits into two records of different strength rather than one, so the stronger claim is not diluted by the weaker:

- **Davis Manifold** — `uses_definition`, altitude `RESONANCE`. The manuscript explicitly uses J^T J, so the mechanism is named and located on both sides.
- **Geometry of Sameness** — `illustrates`, altitude `INCIDENCE`. The example illustrates the question; no mechanism is located on the manuscript side.

Neither is `equivalent_to` or `proves`. Altitude is earned from the evidence record, never from the relation's name — see the [registry proposal](../RESEARCH_RELATIONSHIP_REGISTRY_PROPOSAL.md).

## Source and attribution

- Mathematical reference: [MIT, Jacobians and curvilinear coordinates](https://web.mit.edu/8.Math/www/lectures/lec14/lec14.pdf). The Jacobian of a polar-to-Cartesian map is standard material and this note claims no originality for it.
- Bee's manuscript: *The Geometry of Sameness: An epsilon-Equivalence of Translation and Distance*, abstract and sections on semantic sameness structures and smooth chartability. Local source: `C:/Users/nurdm/OneDrive/Documents/curvature aware gpu/the_geometry_of _sameness.tex`.
- Bee's manuscript: *The Davis Manifold: Geometry-First Detection with Compositional Error Budgets*, metric definition and pullback construction. Local source: `C:/Users/nurdm/OneDrive/Documents/curvature aware gpu/the_davis_manifold.tex`, near line 445 at review time.

This note is repository-relative and portable. The manuscript locations above are local discovery pointers, not shared publication URLs. Attach approved versions and immutable hashes before treating those manuscripts as portable registry evidence — and note that a hash fixes which version is meant without making it readable by a collaborator.

## A worked mathematical anchor

Let F(r,theta) = (r cos(theta), r sin(theta)). Work with r > 0 and a local angular interval on which the map is one-to-one.

Its Jacobian is

```text
J = [ cos(theta)   -r sin(theta) ]
    [ sin(theta)    r cos(theta) ]

det J = r
g = J^T J = diag(1, r^2)
ds^2 = dr^2 + r^2 dtheta^2
dA = r dr dtheta
```

A small angular step travels farther at a larger radius. The induced metric accounts for that fact. For a non-Euclidean target metric h, the general formula is g = J^T h(F) J. Full column rank is needed for positive definiteness.

At r = 0 the angular direction collapses. A zero determinant means rank loss; it does not imply every such singularity is a fold. A negative determinant means orientation reversal, not universally an invalid map. The plane remains intrinsically flat: curved coordinate lines do not establish intrinsic curvature.

## What this supports in Bee's work

1. The Jacobian is the local linear behavior of a differentiable translator.
2. A pullback metric converts translated displacements into a consistent local length rule; the Davis Manifold manuscript explicitly uses J^T J.
3. In Geometry of Sameness, translation-first and geometry-first systems are families of realizations of a fixed semantic sameness structure S. They are not merely two arrows drawn toward the same target.
4. Establishing the broader proposed correspondence requires the manuscript's chartability, compatibility, and error hypotheses. A coordinate example supplies an explanatory foundation, not independent validation of all those hypotheses.

Suggested retrieval questions: "What familiar mathematics explains the manifold approach?"; "How do translators determine a metric?"; "Where is the Jacobian connection to Geometry of Sameness?"

## GIGI follow-up

See [Typed research relationship registry proposal](../RESEARCH_RELATIONSHIP_REGISTRY_PROPOSAL.md). Keep the distinction between documentary relationships among ideas and executable numerical maps between charts.
