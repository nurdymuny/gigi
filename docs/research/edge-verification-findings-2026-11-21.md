# What the edge verification found, and what it corrected (21 November 2026)

I asked for the edge to become a theorem. That needs a project of its own: its own tests and its own paper. I also
asked that nothing about the literature be made up. So the first step was a ten-part adversarial verification round:
- a literature search allowed to report only sources it actually fetched, with a verbatim quote, and a second pass
  that re-fetched every citation;
- four work packages, each followed by a verifier told to **refute** it.

Every report and verdict is in `research/edge-theorem-20261121/workflow_reports.json`.

## What held (and was reproduced by a verifier)

- **The competitors (WP1).** On sectors j ≤ 12, eight blocks set the edge across t ∈ [2, 12], y ∈ [0.25, 3].
  - Each has an exact polynomial (factored over ℚ), and every computed edge is a root of the right factor to
    2 × 10⁻¹³.
  - The sector-4 charge-0 factor is exactly the P₂ found earlier.
- **The "iff" at the edge (WP3).** Proved exactly on two slices (t = 3, y = √3 and y = 3/2), for j ≤ 12:
  - root isolation with rational endpoints;
  - simple roots;
  - an interval Cholesky base point and a Rellich branch argument;
  - interval certificates for every other block near the edge.

  It is conditional on the closed-form blocks being the H + GG* blocks, which the earlier checks support.
- **A sharper large-sector bound (WP2).** Every sector j ≥ 40 is proved on boxes that reach to within about 0.1 of the
  edge (0.02 in one slab), using interval LDLᵀ of the near-end blocks plus Gershgorin for the rest.
- **The family is general (WP4).** Every left-invariant, circle-symmetric SU(2) field on (t, 1, 1) is
  constant-gauge-equivalent to the (x, y) form. Proved.
- **The literature (literature search, every citation re-checked).** All 22 sources exist; a few metadata fixes.
  - The mechanism behind her is the textbook one: a lowest Landau level plus a gyromagnetic-ratio-2 Zeeman term
    (Nielsen–Olesen 1978; stated explicitly in Bolognesi et al., JHEP 2014).
  - The closest prior art is Avramidi–Collopy (2012): negative modes of covariantly constant backgrounds on the
    **round** S³.
  - Nothing found contains a squashed-sphere family with a floor that switches at a polynomial wall. That is not proof
    that nobody has done it; it is what a careful, sourced search found.

## What it corrected (I said these; they were too strong)

- **The edge never goes positive.** I had reported that on strongly stretched spheres she wins even when the field
  along her circle turns slightly with her. That came from checking only j ≤ 12. **Proved:** for x > 0, a high sector
  always undercuts her. Direction is not just a rich vector; its sign is necessary.
- **Her region is an interval.** **Proved:** too strong a turn against her also loses, because her value grows like
  x² and a sector-0 competitor only like x.
- **The all-sector edge is at most min(x*₁₂, 0).** Equality is a conjecture for t ≥ 3 only. Near t = 2 another family
  undercuts her first, in sector j = 16 (refuted there by the verifier).
- **Not every "proved" label held.** The verifiers demoted several labels to "computed" (sampled cutoffs, a ratio
  checked at four points, the conjecture above). The record now says so.

## What is not ready

The three pre-registrations (held-out points, general fields, SU(3)) are **drafts**. The reviewer found fixes that
must go in before any of them is registered. For example, she has multiplicity two in the full spectrum, not one; one
gate would wrongly fail at the newly found lower edge; and some predictions are nearly automatic. They will not be run
until those fixes are made and committed.

## Where this leaves the sentence

> She is the law that a field gives way first along the twist that spins with it, and does so exactly when the field
> along her own circle turns against her charge — by enough, but not too much, as the shape of space sets.
