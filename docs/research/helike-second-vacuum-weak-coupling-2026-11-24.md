# A second-vacuum candidate for the weak-coupling scalar: an exact local count and a prediction

> **SCOPE CORRECTION (same day, two-grain Phase 1).** The prediction below is confined to the one-grain model.
> Adding the other grain stiffens nine modes at the second vacuum (ω² = 7/3), changing the local harmonic offset
> to 4.5√(7/3) - 3.5, approximately 3.37. That is not a measured global excitation gap, nor does it refute the
> prediction within its original one-grain Hamiltonian. It does prevent using the one-grain local offset as a
> prediction for the enlarged model. No full-theory zero-point difference was computed. Revise the intended model
> and global spectral acceptance criteria before a weak-coupling run. The one-grain superpotential and Hessian
> identities remain unchanged. See
> [two-grain Phase 1](helike-two-grain-phase1-2026-11-24.md).

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

The one downward direction of W is the colour-space singlet. It supplies the extra unit in the local count below,
but it is not an unstable direction of V: at a critical point the Hessian of V squares that eigenvalue. The stiffest
directions are the spin-2 shape modes. Sharing a spin label with Helike is not yet an identification of the two
operators or their states. Test: `scripts/test_ym_helike_second_vacuum.py`. It checks the finite-difference Hessian,
the Casimir labels, explicit geometric subspaces and their transformation at rotated points of the vacuum orbit.

I want to keep the double-cover connection precise:
1. **In the computation.** Unit quaternions give S³ = SU(2) → SO(3). The Haar integrand is unchanged by u → −u.
   Squared components of a uniform real unit quaternion have Dirichlet parameters ½. That distribution follows
   from spherical measure, not just from the two-to-one map. Normalized Haar pushforward adds no factor of two.
2. **In the vacuum.** The spherical field construction relates the second vacuum to a Maurer-Cartan form and a
   large gauge transformation. In this finite model its orbit is SO(3), since the central sign acts trivially on
   adjoint colour. The value ΔW = 1/(2g²) follows from the stated W and endpoint normalization; winding alone does
   not set that number.
3. **In the local geometry.** The verified result is the spin-1 tensor-product decomposition and colour-space
   locking identity. Calling it a double-cover phase transition, or identifying it with a particular historical
   spin-from-isospin mechanism, would require additional evidence.

The finite nine-coordinate Hamiltonian does not by itself implement all large-gauge identifications or their
quantum boundary conditions. I therefore do not infer field-theory equivalence, or a continuum gap, from its two
local Hessians. Relevant prior work is van Baal and Hari Dass, *The theta dependence beyond steepest descent*,
[Nuclear Physics B 385 (1992), 185-226](https://www.ilorentz.org/research/vanbaal/DECEASED/HOME/JOUR/NPB385_185.pdf),
especially its spherical-mode and configuration-space boundary treatment. That work includes modes and boundary
conditions absent from this nine-coordinate calculation; it does not validate the proposed gap limit here.

**One object gives both frame and widths.** At any critical point of W, ∇W = 0, so

  Hess V = (Hess W)².

The eigenspaces of Hess W give the local decomposition; degeneracy leaves the basis within each space nonunique.
The absolute eigenvalues |μ| are the normal harmonic frequencies of V and the Gaussian inverse-width coefficients,
not standard deviations. For a normalized mode, ψ(x) is proportional to exp(−|μ|x²/2) and its probability variance
is 1/(2|μ|). The orbit directions have zero local stiffness. Haar averaging preserves the orbit symmetry, but does
not on its own make a degenerate ambient Gaussian normalizable. A global envelope must specify its extension,
normalization, symmetry and all frame-dependent kinetic terms before it is used in the physical solver.

## The local energy count

Because Lap W = 9 (det q is harmonic), H = H_W + 9/2, where H_W = ½(−Lap + |∇W|² − Lap W) is the 0-form part of Witten's
supersymmetric Hamiltonian for W. In the harmonic approximation at a nondegenerate critical point (orbit
directions excluded), H_W has ground energy ½Σ(|μᵢ| − μᵢ):

Here Lap denotes the nine-coordinate Euclidean Laplacian; it is not the endpoint difference ΔW used above.

| Critical set | ½Σ(\|μ\| − μ) | Local ground energy of H |
| --- | --- | --- |
| Origin | ½(9 − 9) = 0 | 9/2 |
| Second-vacuum orbit | ½(11 − 9) = 1 | 9/2 + 1 |

The one unit comes entirely from the breathing mode, the one direction in which W curves downward.

**Conditional prediction (this model, weak coupling).** As g → 0 the barrier 3/(32g²) separates the two vacua.
If the second-orbit local ground state becomes the lowest excited colour-singlet scalar of the global operator,
and no lower competing states intervene, its gap should tend to 1. The
oscillator's own scalar excitation at the origin has gap 2. The lowest spin-2 state stays at the origin with
gap 2, because a shape excitation on the second vacuum costs 1 + 2 = 3. So at weak coupling the scalar is lowest
by a margin that tends to 1. As g grows, the vacua approach and overlap, and the anharmonic terms grow. The saved
scan shows the scalar rising faster than the tensor and crossing it near g² ≈ 1.76. This gives a mechanism for
the crossing as a hypothesis: a second-vacuum scalar and an origin quadrupole. The scan and local count do not yet
prove those global state identifications.

The count above is local and leading order. Anharmonic, orbit-embedding and tunnelling corrections are not
computed here. The action difference suggests exp(−ΔW) as a candidate semiclassical transition-amplitude scale;
it does not establish a universal energy correction exp(−2ΔW), especially for unequal local levels.

## What the saved data already show (exploratory, cutoff 12)

| g² | Scalar gap | Scalar ⟨det q⟩ | Orbit g⁻³ | Scalar ⟨T⟩ | Orbit 3/g² | Ground ⟨T⟩ |
| --- | --- | --- | --- | --- | --- | --- |
| 0.5 | 1.561 | 2.390 | 2.828 | 7.014 | 6.000 | 4.457 |
| 0.75 | 1.958 | 1.631 | 1.540 | 5.807 | 4.000 | 3.951 |
| 1 | 2.271 | 1.235 | 1.000 | 5.124 | 3.000 | 3.619 |
| 2 | 3.097 | 0.593 | 0.354 | 3.925 | 1.500 | 2.921 |

At g² = 0.5 the scalar's signed-volume expectation is about 85% of the orbit value, not an 85% localization
probability. The six normal harmonic variances sum to 7/4. This is not the complete constant-order correction to
⟨T⟩: a normal mean shift and the embedded orbit's geometry can contribute at the same order. The ground's moment
is consistent with the origin oscillator's ⟨T⟩ = 9/2; these moments alone do not establish localization. The scalar gap
falls as g² falls, from 2.27 at g² = 1 to 1.56 at 0.5. The direction is right for a limit of 1, but three
unconverged points are not a test of the limit.

## Why the registered dome test was the hard case

The dome test ran at g² = 1 and 2. There the second vacuum sits at |q| = √3 and √1.5. That is inside the region the
origin basis already covers well, and the barrier is small (3/32 and 3/64). The registered state's outside-origin
fraction and the exterior fraction of an individual dome function are different diagnostics and must not be
substituted for one another. A seated envelope may matter most where the vacua are far apart, which is weak
coupling.

## Proposed next test (for registration, not authorized by this note)

1. **Couplings.** g² ∈ {0.15, 0.25, 0.35}. These are new couplings, not yet inspected.
2. **Observables.** Vacuum and scalar energies and residuals, and the scalar's ⟨T⟩ g²/3 and ⟨det q⟩ g³, whose
   predicted limits are 1.
3. **Prediction.** The scalar gap decreases as g² decreases across the three couplings and lies below 2 at each. Its distance
   from 1 shrinks as g² falls. ⟨T⟩ g²/3 and ⟨det q⟩ g³ for the scalar approach 1, while the vacuum's ⟨T⟩
   approaches 9/2. Thresholds and uncertainty rules must be fixed before any data is seen.
4. **Basis.** Origin functions plus globally admissible envelopes whose normal inverse-width coefficients match
   |μ|: 1 for breathing, 2 for shape. Specify the global extension and symmetry before implementation. The isotropic
   dome remains a control. Freeze K using exterior, joint-span and Hamiltonian graph-norm controls plus a certified
   infinite-tail bound (see the dome review). A guessed growth rule for K is not that certificate.
5. **Control.** The same couplings with the origin basis alone, to show what the seated envelope adds.
6. **Literature.** Finite-volume glueball calculations on the three-sphere with a sphaleron barrier, including van
   Baal and collaborators, must be checked for this mechanism before any novelty is claimed. None is claimed here.
