# The second vacuum carries the scalar at weak coupling: an exact local count and a prediction

24 November 2026. **ANALYTIC NOTE AND PROPOSED PREDICTION. No new numerical experiment was run for this note.**
The numbers quoted from data come from the saved exploration scan
(`research/helike-quantum-probe-20261123/exploration-edge-20261124/edge_scan.json`, cutoff 12). That data is
exploratory, not a registered measurement.

## The exact local geometry at the second vacuum

W = T/2 − g det q has two critical-point sets: q = 0, and the orbit {R/g : R ∈ SO(3)}. At q₀ = I/g, write
q = q₀ + δ. Then det(q₀ + δ) = g⁻³ det(I + gδ), so the quadratic part of W is

  ½|δ|² − e₂(δ),  e₂(δ) = ½[(tr δ)² − tr(δ²)].

The Hessian of W splits the nine directions exactly:

| Directions | Count | Hess W eigenvalue μ |
| --- | --- | --- |
| Breathing, δ ∝ I | 1 | −1 |
| Shape, δ symmetric traceless | 5 | +2 |
| Orbit, δ antisymmetric | 3 | 0 |

At the origin every eigenvalue is +1. None of these numbers depends on g. Only the position (|q₀| = √3/g) and the
barrier between the vacua depend on the coupling. Along the gradient-flow path q = aR/g, V = (3/2)a²(1 − a)²/g²,
so the barrier is 3/(32g²), and the action difference is ΔW = 1/(2g²).

**Why these numbers: colour–space locking, and the double cover.** The table is one identity:

  Hess W(I/g) = I + L_colour · L_space,  while Hess W(0) = I.

The fluctuation δ carries colour spin 1 (row index) and spatial spin 1 (column index). At the second vacuum, q ∝ R
identifies the colour frame with the spatial frame, so δ splits by total spin J = 0, 1, 2. With
L_c · L_s = [J(J+1) − 4]/2,

  μ_J = J(J+1)/2 − 1:  J = 0 → −1 (breathing),  J = 1 → 0 (orbit),  J = 2 → +2 (shape).

The one downward direction, which supplies the extra unit of energy below, is the colour–space singlet, the
state in which the two spins are fully anti-aligned. The stiffest directions are the spin-2 shape modes, the same
quadrupole channel as Helike's |m| = 2. Test: `scripts/test_ym_helike_second_vacuum.py`. It checks the identity
against a finite-difference Hessian, and the J-labels against the Casimir.

I read this as another instance of the double cover, at three levels:
1. **In the computation.** The dome's Haar series came from unit quaternions, S³ = SU(2) → SO(3). The quadratic form
   uᵀBu is blind to u → −u, and that is why the Dirichlet parameters are ½.
2. **In the vacuum.** The space is the three-sphere, which is SU(2) itself. The second vacuum is essentially the
   Maurer–Cartan form of the identity map S³ → SU(2): the vacuum in which space is the group. Its winding fixes
   ΔW = 1/(2g²). Its orbit is SO(3) = SU(2)/{±1}, because −1 acts trivially on adjoint colour, so states along it
   carry integer spin only.
3. **In the local geometry.** Locking turns colour into spin. That is the identity above, the mechanism known as
   "spin from isospin".

Caveat. In the full field theory the second vacuum is a gauge copy of the first, related by a large gauge
transformation, and the two cannot be told apart. This model keeps only the homogeneous left-invariant modes, and
that truncation breaks the equivalence, which is why the Hessians differ (all 1 against −1, 0, 2). Everything in
this note is a statement about this model. Whether it survives with the other modes restored must be checked
against analyses that keep both left- and right-invariant modes (van Baal and collaborators).

**One object gives both frame and widths.** At any critical point of W, ∇W = 0, so

  Hess V = (Hess W)².

The eigenvectors of Hess W give the local frame. Their absolute values |μ| are the oscillator frequencies of V.
So the frame and the widths of a matched local envelope come from the same object, as the magnet rule asks. The
orbit directions have μ = 0. They get no width at all: the state is spread uniformly along the orbit, which is
exactly what Haar averaging does.

## The local energy count

Because ΔW = 9 (det q is harmonic), H = H_W + 9/2, where H_W = ½(−Δ + |∇W|² − ΔW) is the 0-form part of Witten's
supersymmetric Hamiltonian for W. In the harmonic approximation at a nondegenerate critical point (orbit
directions excluded), H_W has ground energy ½Σ(|μᵢ| − μᵢ):

| Critical set | ½Σ(\|μ\| − μ) | Local ground energy of H |
| --- | --- | --- |
| Origin | ½(9 − 9) = 0 | 9/2 |
| Second-vacuum orbit | ½(11 − 9) = 1 | 9/2 + 1 |

The one unit comes entirely from the breathing mode, the one direction in which W curves downward.

**Prediction (this model, weak coupling).** As g → 0 the barrier 3/(32g²) separates the two vacua. The lowest
excited colour-singlet scalar is then the state seated on the second vacuum, and its gap tends to 1. The
oscillator's own scalar excitation at the origin has gap 2. The lowest spin-2 state stays at the origin with
gap 2, because a shape excitation on the second vacuum costs 1 + 2 = 3. So at weak coupling the scalar is lowest
by a margin that tends to 1. As g grows, the vacua approach and overlap, and the anharmonic terms grow. The saved
scan shows the scalar rising faster than the tensor and crossing it near g² ≈ 1.76. This gives a mechanism for
the crossing: the scalar is the second-vacuum state, and the tensor is the origin's quadrupole.

The count above is a leading-order statement. Corrections from anharmonic terms, from the orbit's embedding, and
from tunnelling (suppressed roughly like e^(−2ΔW) = e^(−1/g²)) are not computed here.

## What the saved data already show (exploratory, cutoff 12)

| g² | Scalar gap | Scalar ⟨det q⟩ | Orbit g⁻³ | Scalar ⟨T⟩ | Orbit 3/g² | Ground ⟨T⟩ |
| --- | --- | --- | --- | --- | --- | --- |
| 0.5 | 1.561 | 2.390 | 2.828 | 7.014 | 6.000 | 4.457 |
| 0.75 | 1.958 | 1.631 | 1.540 | 5.807 | 4.000 | 3.951 |
| 1 | 2.271 | 1.235 | 1.000 | 5.124 | 3.000 | 3.619 |
| 2 | 3.097 | 0.593 | 0.354 | 3.925 | 1.500 | 2.921 |

At g² = 0.5 the scalar's signed volume is 85% of the orbit value. Its size is the orbit's 3/g² plus spread. In the
harmonic picture that spread is Σ1/(2|μ|) over the six normal directions, about 1.75, before the orbit's own
contribution. Meanwhile the ground stays at the origin, where the free oscillator gives ⟨T⟩ = 9/2. The scalar gap
falls as g² falls, from 2.27 at g² = 1 to 1.56 at 0.5. The direction is right for a limit of 1, but three
unconverged points are not a test of the limit.

## Why the registered dome test was the hard case

The dome test ran at g² = 1 and 2. There the second vacuum sits at |q| = √3 and √1.5. That is inside the region the
origin basis already covers well, and the barrier is small (3/32 and 3/64). The dome's new content shrank to 10⁻⁸
of a state's norm by cutoff 18. A seated envelope should matter most where the vacua are far apart, which is weak
coupling.

## Proposed next test (for registration, not authorized by this note)

1. **Couplings.** g² ∈ {0.15, 0.25, 0.35}. These are new couplings, not yet inspected.
2. **Observables.** Vacuum and scalar energies and residuals, and the scalar's ⟨T⟩ g²/3 and ⟨det q⟩ g³, whose
   predicted limits are 1.
3. **Prediction.** The scalar gap decreases with g² across the three couplings and lies below 2 at each. Its distance
   from 1 shrinks as g² falls. ⟨T⟩ g²/3 and ⟨det q⟩ g³ for the scalar approach 1, while the vacuum's ⟨T⟩
   approaches 9/2. Thresholds and uncertainty rules must be fixed before any data is seen.
4. **Basis.** Origin functions plus envelopes seated on the orbit with widths matched to |μ|: 1 for breathing, 2 for
   shape, none along the orbit. The isotropic dome is the fallback. The series order K must be chosen by the
   outside-origin truncation criterion (see the review of the dome result), not by a total-norm criterion. At weak
   coupling the required K grows roughly like 3κ/g, so the per-term bound must be computed before freezing K.
5. **Control.** The same couplings with the origin basis alone, to show what the seated envelope adds.
6. **Literature.** Finite-volume glueball calculations on the three-sphere with a sphaleron barrier, including van
   Baal and collaborators, must be checked for this mechanism before any novelty is claimed. None is claimed here.
