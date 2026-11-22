# Pre-registration S: stress tests (22 November 2026)

> *Break something on purpose, and watch what she does in that moment.*

The question is mine, from this morning: when a field is destroyed, torn, cut or overloaded, **what is she doing in
that moment?** These are the lab versions. This note is committed and pushed **before any of the S scripts run**. The
results will go in a separate note, and nothing in this one will be edited after the run.

## The instrument

- **The flow.** The two-number family flow from *the basin* (`docs/research/basin-2026-11-22/family_flow.py`):
  - E = ½(B² + 2c²), with B = y² − 2√t·x and c = y(x − 2/√t);
  - metric x² + 2y²;
  - it reproduced the Floor paper's flow to 0.729 vs 0.73.

  Every break below keeps the circle symmetry, so the broken field stays in her family and the flow is exact.
- **Her price.** At every instant, λ_j = (x + (j+2)/√t)² + 2√t·x − y² (proved across the family).
- **Floor checks.** The Floor paper's charge-block builder (H + GG*), sectors j ≤ 40. That is a finite window, and every
  floor claim below is scoped to it.
- **Landing.** The run ends on A = 0 if (x, y) comes within 10⁻³ of (0, 0), on A = θ if within 10⁻³ of (2/√t, 2), and
  is called "other" otherwise. Flow time is 200.

## S1. Cut: switch the cross field off (y → 0)

Start from the Berger field at t = 3, 3.55, 5 and 10, and from 20 points of the certified box
D = [−3, −1.5] × [0.5, 3] × [3, 10] (a fixed grid, listed in the script). At τ = 0, set y = 0 and keep x.

Derived **before** the run, from the flow equations: ∂E/∂y = 2y[B + (x − 2/√t)²], which vanishes at y = 0. So y stays 0
forever, E = 2t·x², and x decays to 0.

| id | prediction | what would kill it |
| --- | --- | --- |
| S1a | Every cut run stays at y = 0 to machine precision and lands on **A = 0**, the trivial basin. | any run with \|y\| > 10⁻¹² or landing elsewhere |
| S1b | At the landing (A = 0, t ≥ 3) she is the floor of every sector j ≤ 40. | any sector j ≤ 40 with a state below her at A = 0 |
| S1c | Along the cut flow her price only rises: the cut never makes her cheaper. | any run where her lowest price (j ≤ 40) decreases at some step |

## S2. Flip: reverse the field along her circle (x → −x)

Same 20 points of D. At τ = 0, set x → −x and keep y.

| id | prediction | what would kill it |
| --- | --- | --- |
| S2a | **Control.** At the flipped point she is not the floor in some sector j ≤ 40, wherever the edge paper's x > 0 theorem puts the losing sector inside j ≤ 40 (that is, l* < 20). | any such point where she is still the floor of every j ≤ 40 |
| S2b | Exploratory, no prediction: where the flipped field lands (A = 0, A = θ or other), and when her price crosses zero. | — |

## S3. Overload: scale the whole field up (x, y) → s·(x, y)

From the Berger field at t = 3, 3.55 and 5, with s = 1.5, 2, 5 and 10.

| id | prediction | what would kill it |
| --- | --- | --- |
| S3a | Her lowest price falls like −s²: the ratio λ_min(s)/s² tends to x² − y² = (4 − 4t)/t, within 5% at s = 10. | ratio off by more than 5% at s = 10 |
| S3b | Exploratory: whether she stays the floor of every j ≤ 40 at the overloaded point, and where it lands. | — |

## S4. Crush and stretch: push t to the extremes (at the Berger field)

t ∈ {0.25, 0.5, 1, 1.5, 2, 2.5, 2.75, 12, 15, 20, 30, 50}.

| id | prediction | what would kill it |
| --- | --- | --- |
| S4a | She is the floor of every sector j ≤ 40 for every t ≥ 2.75 in the list, extending the proved [2.75, 10] to 50. | any t ≥ 2.75 in the list with a state below her in some j ≤ 40 |
| S4b | She is **not** the floor of every sector for t ≤ 2.5 (the H + GG* threshold is t_A ≈ 2.731). | any t ≤ 2.5 where she is the floor of every j ≤ 40 |

## S5. Which basin: the map of the fall at t = 3

A 41 × 41 grid of starting fields (x, y) ∈ [−3, 3] × [0, 3] at t = 3, each flowed to its landing.

| id | prediction | what would kill it |
| --- | --- | --- |
| S5a | The plane splits into exactly two landing regions, A = 0 and A = θ. "Other" occurs only on the boundary between them. | an open patch of "other", or a third flat landing |
| S5b | The **squashed meron**, the resting field that is not flat (computed as the non-flat zero of ∇E with x > 0), lies **on the boundary** between the two regions. Its single unstable direction points one way into each basin. | the meron more than one grid step from the boundary, or both sides of its unstable direction landing in the same basin |

## What I am asking of each result

For each break: what her price is at the moment of the break, whether she is the floor there, how long until her price
crosses zero (if it does), where the field lands, and what she is at the bottom. In one line: **door, wall, or
shadow** at every moment.
