# LinkedIn post (my draft): version 2 of Reading the Curve

Images, in this order: `cards/01_cover_one_vs_two_rings.png`, `cards/06_letting_the_lattice_move.png`,
`cards/03_results.png`, `cards/02_fold_keep_what_matters.png`, `cards/04_three_ways_across_a_turn.png`.
Use `cards/05_link_preview_1200x627.png` when sharing a link.

---

The standard way of reading curvature on a mesh has a wall in it. On the Poincaré homology
sphere, its force error stalls at 26% and never comes down, however fine the mesh.

I think of it the way an archaeologist reads a broken vessel from a single rim sherd. With
one ring of neighbours, a cell has exactly as many clues as unknowns, so it has to believe
every one of them, noise included, and it amplifies that noise about 250 times. Give it a
second ring and the noise is outvoted instead of obeyed. The error falls to 1.45%, the force
is stable, and nothing is thrown away.

The same reading carries to a non-commuting SU(2) field in my research code: 25.5% down to
2.51%, with no runaway modes on the meshes I tested.

New in version 2: I let the lattice move. The space underneath flows (Ricci flow of the
Berger metric), the links move under the force, and at the end I compare them with where the
true field went. The two-ring force ends 13.8%, 1.76%, then 0.87% off the true path on three
meshes. Freeze the background and the error stays stuck near 8%, which is what it should do
if the test really feels the moving space.

Two honest notes, because they matter more than the headline. Most of the field's motion
happens even with the space frozen; against only the part the moving space causes, the
two-ring error is 10.6% on the finest mesh. Still falling, not solved. And one of my own
forecasts missed: I expected two rings to pull further ahead on the finest mesh. Both errors
halved instead.

Because so many people now assume everyone else is faking results, I had this work reviewed
for exactly that. Nothing was found that makes the code flatter the result; the review did
catch wording and three roundings that leaned my way, and they are fixed.

The companion paper, Three Ways Across a Turn, is now published: one turn, and the three ways
across it (the chord, the arc and the detour). It supplied the time step the moving lattice
needed at its finest level.

Reading the Curve: https://doi.org/10.5281/zenodo.24328085
Three Ways Across a Turn: https://doi.org/10.5281/zenodo.24375338

Everything runs on GIGI, which is open source (PolyForm), and the code and data behind every
number are in the record. If you don't agree, run the code.

---

**Do not change without checking the receipts:** 26%, 1.45%, about 250× (cell 0: 253.5; the
median is 125–252 by mesh), 25.5% and 2.51%, 13.8% / 1.76% / 0.87%, about 8% (8.07% and 8.13%),
10.6%. "Research code" stays: the SU(2) results are not engine runs. Do not call the review
"independent" or "peer review": it was my own adversarial review.
