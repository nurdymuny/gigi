# The count: why V1 had to fail, and what the variational recovery needs (19 November 2026)

This is the method that got *Reading the Curve* its result: count the clues, count the unknowns, and
look at what the recovery is being asked to believe. I applied it to V1 before trying any new
objective. **No new measurement:** this note counts V1's existing construction and its neighbourhoods.
Scripts: `scripts/ricci_variational_count.py`, `scripts/ricci_variational_quadcount.py`. Receipts:
`research/variational-count-20261119/`. Abelian control, q6.

## The counts

A cell's **clues** are the independent face fluxes its neighbourhood reads: the rank of the flux map
D, which is fewer than the number of faces, because every cell's fluxes sum to zero.

| Neighbourhood | faces read | **independent clues** | n |
| --- | ---: | ---: | --- |
| one ring | 16 | **11** | 1 and 2 |
| two rings | 40 | **26** | 1 |
| two rings | 44 | **29** | 2 |
| three rings | 80 | **50** | 1 |
| three rings | 100 | **64** | 2 |

Every sampled cell gave the same numbers.

- **One ring is exactly 11.** That equals the eleven coefficients of a closed linear field: the
  paper's "as many clues as unknowns", now counted.
- **Two rings give the paper's vote of 26 to 29.**

The constraint-row totals reproduce the V1 receipts exactly: 2,698 at n = 1 and 21,356 at n = 2.

## What the spare clues are

How many unknowns does a smooth field need? Closed linear fields have 11. Closed quadratic fields add
15 more: divergence-free homogeneous quadratic vector fields, 18 coefficients less 3 conditions. That
makes **26**.

Then the decisive check. On every sampled cell I computed the flux map of all 26 smooth fields
exactly (quadrature exact for quadratics). Its rank is 26, and it lies entirely inside the clue space.

**At n = 1, the two-ring clue space is exactly the closed linear and closed quadratic fields.** The 15
"spare" clues are not noise directions at all. They are precisely the second-order part of a smooth
field. At n = 2 there are 29 clues, so 3 directions per cell lie beyond second order.

## Why V1 failed, in one line

V1 changes the recovery only in the spare directions. It does so by construction: M′ = M + Z C Nᵀ,
with Nᵀ killing the closed linear fields. At two rings and n = 1, those directions *are* the
quadratic part of the true field. So V1 had no way to obey the column condition except by reading the
field's own second-order part, with weights of about 288 (‖C‖), and calling it signal.

That is the reconstruction term: 4.7% to 432%. It is the one-ring disease again by another route.
The recovery is made to believe directions it had no business believing.

This also explains two further results:
- **V1b could not help** (485%). The trouble was never which objective it minimised; there is
  nowhere else for the change to go.
- **The error falls from n = 1 to n = 2** (432% to 61.6%). At n = 2 the three directions beyond
  second order appear.

## What the count says to build

| Design | unknowns | clues | spare | vote |
| --- | ---: | ---: | ---: | --- |
| two-ring fit, linear-exact (today, published) | 11 | 26–29 | 15–18 | 2.4–2.6× |
| two rings, linear + quadratic exact | 26 | 26–29 | **0–3** | **1.0–1.1×: the one-ring trap** |
| **three rings, linear + quadratic exact** | 26 | 50–64 | **24–38** | **1.9–2.5×** |

The quadratic-exact rows proposed in `where-we-are-2026-11-19.md` are the right idea. But at two
rings they would use up the entire vote: 26 unknowns from 26 clues at n = 1. **They need a third
ring.** With three rings, the change C would live in 24–38 directions that carry no smooth field to
second order. The column condition could then be met without reading the true field's own curvature
as signal. The vote that remains, 1.9–2.5×, is the same order as the published two-ring fit's.

## What this does not show

- That the three-ring construction meets the column condition with a *small* change. That is the
  experiment, and it needs its own spec with its gates fixed before it runs.
- How large the three-ring recovery's amplification is. Bigger neighbourhoods also bring in
  farther, more distorted cells.
- Anything about SU(2), or about meshes finer than n = 2.

## Proposed next step

1. A spec for V2: three rings, rows exact on closed linear **and** quadratic fields, and V1's column
   condition, at n = 1 and n = 2.
2. Fix this prediction in the spec, before anything runs: the reconstruction term stays at the
   published fit's level, a few percent, instead of exploding. If it still explodes when the change
   cannot reach the smooth part, then the count is not the whole story, and that is the finding.
