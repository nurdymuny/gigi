# Three Ways Across a Turn: Chord, Arc and Tangent

**Paper:** [`paper.pdf`](paper.pdf) (16 pages, working draft). **Author:** Bee Rosa Davis, Davis Geometric.

## The idea

Turn a line of length one through an angle. Its tip moves along a circle. Afterwards,
the two tip positions can reach each other three ways:

- the **chord**, straight through the circle (the shortcut);
- the **arc**, the path the turn itself travels;
- the **outside path**, along the two tangent lines that meet outside the circle (the detour).

The shortcut is always shortest and the detour always longest. The true path lies
between them.

The paper reads the three as one object: three measurements of the same turn that miss
in fixed proportions. The detour overshoots by exactly twice what the shortcut falls short,
so a two-to-one blend cancels the main error. The same proportion appears again:

- **in length:** the Snellius–Huygens estimate;
- **in energy:** the shortcut's energy is Wilson's lattice action, and the true path's is
  the continuum one. The blend cancels the θ⁴ term;
- **in algebra:** the detour's leg is the Cayley parameter that rebuilds the turn with
  plain arithmetic;
- **in time stepping:** explicit Euler steps out along the tangent, implicit Euler falls
  inside, and Crank–Nicolson stays on the circle, lagging by the same 1/12.

Half the shortcut is the distance *d* of the double-cover identity S + d² = 1.

Every miss has a twin on the other side (Snellius' lower estimate, nine times smaller),
and the same chord has a twin turn the long way round. Crossing the circle's rim makes
the turn imaginary while S + d² = 1 survives. Bertrand's paradox is the question of
which quantity is uniform: the arc (1/3), √S (1/2) or S (1/4).

**The ticket game** (Bee's, 2026-11-18) puts it all in one room. A ticket buys one straight
walk of at most one step. Two friends each walk one ticket out from the middle. Getting
back costs one ticket along the curve up to one radian (57.3°), straight across up to 60°,
and, counted as energy the way the lattice counts it, exactly one ticket at 90°
(Pythagoras). With whole tickets it is always two trips, turning either at the middle or at
its twin corner across the crossing. At 90° that twin corner is where the edge lines meet.

Filling the board forces six one-ticket angles: in length, the detour at 53.13° (the 3-4-5
triangle), the arc at 57.30° and the chord at 60°; in energy, 70.53° (cos θ = 1/3, the
tetrahedron), 81.03° (√2 radians) and 90°. The energy of crossing to a friend and to the
friend's opposite point always sum to two tickets (Thales). They are 2d² and 2S, so the
90° break-even is where S = d² = ½.

A companion research note, `docs/research/unit-curvature-2026-11-18.md`, follows ±1
into SU(2), its curvature and the mass gap. It keeps the open link tagged as open.

## Where it came from

The picture is Bee's (2026-11-18). It came up while working on the SU(2) force of the
two-ring curvature recovery (`docs/papers/two-ring-pg/`), at the point where that work
turns to time integration.

## What is claimed

Every identity is checked by [`verify.py`](verify.py) (all checks passing; output in
[`verify_output.txt`](verify_output.txt)). The classical identities are cited, and no
priority is claimed for them. The energy blend is the one statement not found in the
sources checked, and a proper prior-art search is listed as open.

On the real SU(2) meshes, face angles are at most 0.094, so shortcut and true path differ
in energy by under 0.08% ([`face_angles.py`](face_angles.py),
[`face_angles.json`](face_angles.json)). That is far below the errors measured there,
and the paper says so: the idea does not explain those errors. It matters for the time
integrator and for strong fields.

## Rebuild

```bash
python docs/papers/chord-arc-tangent/verify.py
GIGI_RICCI_DATA_ROOT=<gigi checkout with output/> python docs/papers/chord-arc-tangent/face_angles.py
GIGI_RICCI_DATA_ROOT=<gigi checkout with output/> python docs/papers/chord-arc-tangent/make_figures.py   # figures/*.pdf
cd docs/papers/chord-arc-tangent && pdflatex paper.tex && pdflatex paper.tex
```

Only `fig_face_angles` needs the frozen meshes; it asserts that its numbers match Table 2.
Typesetting uses the shared house style [`../dgstyle.tex`](../dgstyle.tex) (Palatino via
`newpx`, `titlesec`, `caption`, `fancyhdr`, `enumitem`). On a minimal TeX Live install:
`tlmgr --usermode install titlesec newpx newtx enumitem fontaxes xstring kastrup`.
