# Pre-registration S7: the tear (22 November 2026)

> *Tear the space, and some of the families are simply gone. Who is left standing?*

**The break.** Quotient the three-sphere by a finite group Γ ⊂ SU(2) acting on the left. That keeps:
- the Berger metric;
- every field in her family;
- the fall, so it is the same path, still landing on A = θ.

What the tear changes is **which families exist**: family j survives only if V_j has Γ-invariant vectors. The
Poincaré sphere (Γ = 2I) was the first tear. Here are eight:

| Γ | order | the torn space |
| --- | --- | --- |
| Z₂ = {±1} | 2 | RP³ |
| Z₃ | 3 | lens space L(3, 1) |
| Z₅ | 5 | lens space L(5, 1) |
| Q₈ (binary dihedral 2D₂) | 8 | quaternionic space |
| 2D₃ (binary dihedral) | 12 | prism manifold |
| 2T (binary tetrahedral) | 24 | |
| 2O (binary octahedral) | 48 | |
| 2I (binary icosahedral) | 120 | Poincaré sphere |

This note is committed and pushed **before** the tear script runs.

**Instrument.**
- Surviving families: dim V_j^Γ from the character sum over the group's unit quaternions, j ≤ 40.
- Spectra: the Floor paper's charge blocks (H + GG\*). The torn space's spectrum is the union of the surviving
  families' spectra.
- Stretches t ∈ {3, 5, 10}.
- Holonomy at A = θ: parallel transport around each γ ∈ Γ, compared with Ad(γ⁻¹), as in `poincare_fall.py`.
- **L(t)**, the families she loses at A = 0 (from S1b and S6): {1, 2, 3, 4, 5} at t = 3, {2, 3} at t = 5, {2} at
  t = 10.

| id | prediction | what would kill it |
| --- | --- | --- |
| T1 | **At the top she survives every tear.** At the Berger field she is the floor of every surviving family j ≤ 40, for every Γ and t, and her cheapest surviving price is negative: she is still a door. | any Γ, t, surviving family with a state below her; or her cheapest surviving price ≥ 0 |
| T2 | **Every torn basin is gapped.** At A = θ and at A = 0 there are no zero modes in any surviving family, for every Γ and t. The holonomy at θ is Ad(γ⁻¹) to 10⁻¹⁰: irreducible for the five non-abelian groups, abelian for Z₂, Z₃, Z₅. | a zero mode anywhere; or a holonomy mismatch above 10⁻¹⁰ |
| T3 | **The shadow law, family by family.** At A = θ, in every surviving family j ≥ 2, the lowest wall is exactly j²/t (her shadow from family j − 2) when j − 2 ∉ L(t), and strictly below j²/t when j − 2 ∈ L(t). | any surviving family j ≥ 2 that breaks either half |
| T4 | **The tear decides whether her shadow holds the whole basin.** Where family 2 survives the tear, the lowest wall of the whole θ basin is 4/t, her shadow from family 0. Where family 2 is torn away, the lowest wall of the θ basin is strictly above 4/t. | a Γ where the outcome is the other way round |
| T5 | **The trivial basin always keeps her.** At A = 0 the lowest wall of the torn space is 4/t, and her family-0 state holds it, for every Γ. Family 0 always survives. | any Γ, t where the lowest wall at A = 0 differs from 4/t, or she does not attain it |
| T6 | Exploratory, no prediction: in each torn θ basin where family 2 is gone, which family holds the lowest wall, and at what price. | — |
