# LinkedIn post (draft for Bee)

Images, in this order: `cards/01_cover_one_vs_two_rings.png`, `cards/02_fold_keep_what_matters.png`,
`cards/03_results.png`, `cards/04_three_ways_across_a_turn.png`. Use
`cards/05_link_preview_1200x627.png` when sharing a link.

---

The standard way of reading curvature on a mesh has a wall in it. On the Poincaré homology
sphere, its force error stalls at 26% and never comes down, however fine the mesh.

I think of it the way an archaeologist reads a broken vessel from a single rim sherd. With
one ring of neighbours, a cell has exactly as many clues as unknowns, so it has to believe
every one of them, noise included, and it amplifies that noise about 250 times. Give it a
second ring and the noise is outvoted instead of obeyed. The error falls to 1.45%, the force
is stable, and nothing is thrown away: every number the standard method had is kept, and
only the curve between them is refined.

Two things surprised me.

You don't need the whole second ring. Two to four well-chosen cells out of about ten keep
the force accurate and stable. Pick the same number at random and it blows up, with 48
runaway modes. Keep the cells that change the answer.

And the same reading carries to a non-commuting SU(2) field in our research code: 25.5%
down to 2.51%, with no runaway modes on the meshes we tested.

The companion paper, *Three Ways Across a Turn*, is about the smallest piece of all: one turn,
and the three ways to get across it (the chord, the arc and the detour). A lattice pays for
the chord, the continuum is made of arcs, and the gap between them is curvature.

Everything runs in GIGI, which is open source (PolyForm). If you don't agree, run the code.

---

**Do not change without checking the receipts:** 26%, 1.45%, about 250× (cell 0: 253.5; the
median is 125–252 by mesh), 2–4 of about 10, 48 runaway modes, 25.5% and 2.51%. "Research
code" stays: the SU(2) results are not engine runs.
