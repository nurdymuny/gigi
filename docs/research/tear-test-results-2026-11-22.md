# Pre-registration S7: the tear: results (22 November 2026)

> *Tear the space so it can no longer be untwisted, and the floor of the basin goes to the one who beat her at rest.*

**Registered:** [tear-test-preregistration-2026-11-22.md](tear-test-preregistration-2026-11-22.md), commit a5be2123,
pushed before the script existed.
**Script and receipt:** [`stress-2026-11-22/tear_test.py`](stress-2026-11-22/tear_test.py) and `tear_receipt.json`.

The torn spaces are S³/Γ for eight groups Γ acting on the left:
- the cyclic groups Z₂, Z₃, Z₅;
- the binary dihedral groups Q₈ and 2D₃;
- the binary tetrahedral 2T, binary octahedral 2O and binary icosahedral 2I.

Before the run the script checked that each set is a group of the stated order. These are my scripts, using the Floor
paper's charge blocks (H + GG\*): families j ≤ 40, at t = 3, 5 and 10.

## Scorecard: all five held

| id | prediction | verdict |
| --- | --- | --- |
| T1 | at the top she survives every tear: the floor of every surviving family, and still a door | **held**, 8 groups × 3 stretches |
| T2 | every torn basin is gapped; holonomy at θ is Ad(γ⁻¹) | **held**: no zero modes anywhere; holonomy to 1.0 × 10⁻¹² or better |
| T3 | the shadow law, family by family: j²/t where j − 2 ∉ L(t), strictly below where j − 2 ∈ L(t) | **held**, every surviving family j ≥ 2 |
| T4 | the θ basin's lowest wall is 4/t (her shadow) exactly when family 2 survives the tear | **held**, every group and stretch |
| T5 | the trivial basin's lowest wall is 4/t and hers, in every torn space | **held** |

## What the tear does

**Which families survive.**

| Γ | order | abelian? | family 2 survives? | first surviving families |
| --- | --- | --- | --- | --- |
| Z₂ | 2 | yes | yes | 0, 2, 4, 6, … (even only) |
| Z₃ | 3 | yes | yes | 0, 2, 3, 4, 5, 6, … |
| Z₅ | 5 | yes | yes | 0, 2, 4, 5, 6, 7, … |
| Q₈ | 8 | no | **no** | 0, 4, 6, 8, … |
| 2D₃ | 12 | no | **no** | 0, 4, 6, 8, … |
| 2T | 24 | no | **no** | 0, 6, 8, 12, … |
| 2O | 48 | no | **no** | 0, 8, 12, 16, … |
| 2I | 120 | no | **no** | 0, 12, 20, 24, 30, … |

Family 2 survives exactly for the three cyclic groups, and this is no accident.
- Family 2 is the adjoint representation, the rotations of a 3-dimensional space.
- It has an invariant vector only if every element of Γ fixes a common axis, which happens exactly when Γ is cyclic.
  (STRUCTURAL: standard representation theory, not proved in this note.)
- So **the tear removes family 2 exactly when it makes the holonomy irreducible**: when no loop can be untwisted by
  turning about a single axis.

**What she is in each torn space.**
- **At the top**, at the Berger field, nothing changes. In every torn space she is the floor of every surviving family
  and still a door: family 0 always survives, and her price there is negative. **No tear removes her.**
- **At the bottom of the trivial basin (A = 0)**, nothing changes either. The lowest wall is 4/t and it is hers, in
  every torn space.
- **At the bottom of the θ basin**, the tear decides.
  - **Cyclic tears** keep family 2. The lowest wall of the basin is 4/t, her shadow from family 0. She settles last,
    as herself carried by the wrap.
  - **Non-abelian tears** remove family 2. The lowest wall rises above 4/t: 2.5109 at t = 3, 1.8755 at t = 5,
    1.1672 at t = 10.

**Who takes it (T6, exploratory).** In every non-abelian torn space, the new lowest wall of the θ basin is in family 0,
and in family 4 as well where family 4 survives (Q₈, 2D₃). Its price is **exactly** the lowest state of family 2 at
A = 0, to twelve digits, at all three stretches. That state is the rival that undercuts her in family 2 at perfect
rest, from stress test S1b and the current sweep.

So, in words:

> On the round sphere, and in any space torn only cyclically, the last thing settling in the basin the fall reaches is
> her shadow. Tear the space so that it can no longer be untwisted about one axis, and her shadow's family is gone.
> The floor of that basin passes to the one state that beat her when nothing was moving.

That connects the tear to S1b and S6. The rival who wins at rest, where the race is not run, is the one who inherits
the basin when the tear deletes her shadow.

## Ledger

| claim | tag |
| --- | --- |
| at the Berger field she survives every tear, the floor of every surviving family and still a door | COMPUTED (8 groups, 3 stretches, j ≤ 40) |
| every torn basin gapped; holonomy at θ = Ad(γ⁻¹) | COMPUTED (≤ 1.0 × 10⁻¹²) |
| the shadow law holds family by family, with its exceptions exactly at L(t) | COMPUTED |
| family 2 survives iff Γ is cyclic | COMPUTED here; STRUCTURAL (the adjoint representation's fixed vectors) |
| her shadow holds the θ basin iff the tear is cyclic | COMPUTED |
| otherwise the basin's floor is her rival at rest (A = 0, family 2), to 12 digits | COMPUTED |
| these eight groups are the ADE family of McKay's correspondence (see *the arc*) | INCIDENCE: chosen that way, not a finding |
