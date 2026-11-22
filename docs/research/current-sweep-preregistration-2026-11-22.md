# Pre-registration S6: the current sweep (22 November 2026)

> *How much motion does she need?*

**Why this test.** Stress test S1b failed. At the flat field A = 0 she is undercut in families 1 to 5, yet at the cut
point she is the floor of every family. The cut point has y = 0, but the field along her circle, x, is still at its
Berger value. My reading: the only thing that changed between the two is that the motion along her circle drained
away. Her advantage is the walkway, riding the belt harder than anything beside her. With no belt there is no race,
and nothing picks her out. The perfect resting field is the one place the race is not run. This test turns that
reading into a number: for each family, how much current she needs to stay the floor.

This note is committed and pushed **before** the sweep script runs. The results go in a separate note.

**Instrument.**
- The circle-symmetric family at y = 0 (no cross field).
- The Floor paper's charge-block builder (H + GG*), families j ≤ 40.
- Her price is λ_j = (x + (j+2)/√t)² + 2√t·x.
- For each stretch t ∈ {3, 3.55, 5, 7, 10}, sweep x from the Berger value (2 − t)/√t up to 0 on a grid of 400 steps.
  For each family j, find every x where she stops being the floor of that family, refined by root-finding to 10⁻¹⁰.
- **The threshold x\*_j** is the value of the field along her circle below which (more negative, more current) she
  is the floor of family j, and above which she is not.

| id | prediction | what would kill it |
| --- | --- | --- |
| S6a | Along the whole sweep she is the floor of every family j ≤ 40 at the Berger end, and she loses a family only as x approaches 0. Every loss happens at a single threshold x\*_j < 0, and beyond it, toward 0, she stays undercut. | a family she loses and then regains before x = 0, or a loss at the Berger end |
| S6b | The families she has lost by x = 0 are exactly the S1b families: 1–5 at t = 3, then fewer as t grows, family 2 alone by t = 7 and t = 10. | any other set at x = 0 |
| S6c | Exploratory, no prediction: the thresholds x\*_j as numbers; which family she loses first as the current drains; and whether the thresholds have a closed form. | — |

**Reading.** If S6a holds, "she needs motion to be singled out" becomes a measured statement, with the exact amount of
motion each family needs.
