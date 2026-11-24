# Floor Theorem page: copy for the 24 November update

Drafted for davisgeometric.com/research/papers/floor-theorem. Three pieces: a replacement for the *By Enough* passage,
a new "Since v3.3" box, and a related-work line. The page's theorem statement, formula and proved range
(2.75 ≤ t ≤ 10) are unchanged.

---

## 1. Replace the *By Enough* passage and link

**Link:** *By Enough, But Not Too Much* (v1.1): https://doi.org/10.5281/zenodo.24472820

**Copy:**

> The Floor Theorem asks for the lowest mode at one field, the Berger field. *By Enough, But Not Too Much* asks where
> else Helike wins. Every left-invariant field with the sphere's circle symmetry comes down to two numbers: a strength
> x along the long circle and a strength y across it. At every such field Helike is still an exact eigenvector, with
> eigenvalue (x + (j+2)/√t)² + 2√t·x − y².
>
> Where she is the floor:
> - **Proved:** the region is bounded on both sides. Below, an explicit cubic from the lowest sector cuts it off.
>   Above, high sectors undercut her as soon as the field along her circle turns with her (x > 0). That is proved
>   for H + GG\*; for the gauge-orthogonal spectrum it is computed.
> - **Proved:** on the domain x ∈ [−3, −1.5], y ∈ [0.5, 3], t ∈ [3, 10], interval certificates show that she is the
>   floor of every sector, twice (with her mirror).
> - **Proved, an exact identity:** where the main competitor is the charge-0 block of sector 4, the edge (computed in the
>   sectors j ≤ 12) is a root of an explicit polynomial. On the Berger path it contains the polynomial of the Floor paper's threshold. Its
>   identities are also checked in Lean 4.
> - **Computed:** on every grid I ran, the region is a single interval.
> - **Tested, and failed as stated:** three simpler explanations of why she wins (symmetry, field alignment, and the
>   curvature term), each written down before its data. Six further tests were pre-registered, and their failures
>   are reported with their passes.

---

## 2. New box: "Since v3.3 (draft, not yet posted)"

> **Since v3.3: why she wins (draft).** This is new work, written up as Section 4 of *By Enough* version 2, which I
> have not posted yet. It does not change the Floor Theorem or its range.
>
> **The Helike Ladder Theorem (proved by hand).** On a three-manifold the Yang–Mills Hessian splits into the square
> of the curl plus a flux term. Helike's value is exactly *helicity squared minus the flux along her circle*. She
> takes the whole flux discount and pays only her own twist.
>
> The flux-aligned modes form a ladder, one rung per weight. Every rung but the top leaks sideways, and that leak is
> exactly its gauge part: a Cauchy–Riemann identity. The top rung has nowhere higher to go, so it cannot leak. That
> rung is Helike: the one flux-aligned mode that neither leaks nor has a gauge part.
>
> **A sufficient condition for the floor (proved by hand).** She is the floor of a sector whenever the flux gap (how
> much better her lane is paid than anything outside it) covers her helicity, with a gauge correction. At the Berger
> field this proves her the floor of **sector 0 for every t ≥ 2√5 − 2 ≈ 2.472, with no upper limit**. That is
> sector 0 only; the all-sector range stays 2.75 ≤ t ≤ 10.
>
> **What it explains.** The edge in x is a contest of helicity inside the flux-favourable region, not the end of flux
> alignment. On the round sphere the flux gap is exactly zero, which matches the failure of the "alignment" reading.
>
> **Conjecture (computed, not proved).** At the Berger field there is no upper edge in the stretch. She is the floor
> of every sector j ≤ 12 at t = 10, 12, 15 and 20. The flux gap grows with t, and her helicity can be made small in
> the sector j ≈ t − 4. The upper limit t = 10 is where the certificates stop, not an observed edge.
>
> **Checked against the literature.** At zero field my curl operator reproduces the known curl spectrum of the
> Berger sphere exactly (Gibbons 1980; Capoferri and Vassiliev 2026).

---

## 3. Related work (add one line)

> On the round sphere, Helike's eigenvalue also appears in Kumar, Lechtenfeld and Picanço Costa, *Instability of
> cosmic Yang–Mills fields* (Nucl. Phys. B 973, 2021), as the frequency of an extremal total-spin sector. They reach
> it by adding orbital, colour and vector spin into one total spin, which needs the round sphere's full symmetry. My
> charge is the one component of that total spin that survives stretching the sphere, which is how the formula
> reaches every Berger sphere.

---

## Not for the page yet

- The quantum probe and its flux-edge test: on hold until its domain question is settled.
- The orientation-partner observation: a pattern in my notes, not a result.
