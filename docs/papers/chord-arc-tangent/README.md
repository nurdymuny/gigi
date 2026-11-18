# Three Ways Across a Turn: Chord, Arc and Tangent

**Paper:** [`paper.pdf`](paper.pdf) (8 pages, working draft). **Author:** Bee Rosa Davis, Davis Geometric.

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

A companion research note, `docs/research/unit-curvature-2026-11-18.md`, follows ±1
into SU(2), its curvature and the mass gap. It keeps the open link tagged as open.

## Where it came from

The picture is Bee's (2026-11-18). It came up while working on the SU(2) force of the
two-ring curvature recovery (`docs/papers/two-ring-pg/`), at the point where that work
turns to time integration.

## What is claimed

Every identity is checked by [`verify.py`](verify.py) (39 checks, all passing; output in
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
pdflatex paper.tex && pdflatex paper.tex
```
