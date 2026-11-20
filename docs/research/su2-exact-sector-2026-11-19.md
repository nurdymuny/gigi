# An exact answer for the SU(2) spectrum at the Berger field, and what it means for the mass-gap observable (19 November 2026)

A mathematical exercise prompted by the method that keeps working: *have an exact answer to measure against
before trusting a number*. The Yang–Mills strategy note (`yang-mills-strategy-2026-11-19.md`, step 2) asks for a
mass-gap observable that separates what the curved space contributes from what the field contributes. Until now
there was no exact answer to measure that against.

Script: `scripts/ricci_su2_left_invariant_sector.py` (sympy, exact). Receipt:
`research/su2-left-invariant-sector-20261119/sector.json`. The comparison reads only committed eigenvalue
receipts: S2b at n = 1, 2 and S2d at n = 3. No new lattice run. This is a research script, not an engine run.

## The idea

The Berger field is the Levi-Civita (spin) connection of the left-invariant metric g = (3, 1, 1) on S³. The field,
the metric and the energy are all invariant under left translations. So the linearised Yang–Mills operator (the
Hessian of E = ½|F|²) splits by representation. Its smallest piece is the **left-invariant sector**: the
su(2)-valued 1-forms that are constant in the left-invariant frame. That is 3 directions × 3 Lie-algebra
directions, so **9 numbers**. On that sector the Hessian is a 9 × 9 matrix, which can be computed exactly.

On S³/2I (Poincaré's sphere, a quotient by left translations), the left-invariant sector descends intact. The
icosahedral selection rule then forbids every other sector up to spin 6. On the round metric that is the known
first scalar eigenvalue, 168. On the Berger metric the squashing pulls the spin-6 sector down, but it still sits
well above this one (see below).

## The exact sector (Berger, g = (3, 1, 1))

The eigenvalues against the L² mass, with multiplicities. Negative means a direction of growth: the field is not
at rest.

| exact | value | multiplicity | measured n = 1 | n = 2 | n = 3 (native / two-ring) |
| --- | ---: | :-: | ---: | ---: | ---: |
| −14/3 | −4.6667 | 2 | −4.475 | −4.651 | −4.661 / −4.651 |
| (29 − √1345)/3 | −2.5581 | 2 | −2.191 | −2.504 | −2.529 / −2.529 |
| 16 − 2√73 | −1.0880 | 1 | −0.536 | −1.041 | −1.0875 / −1.0836 |
| 8 | 8 | 1 | 7.083 | 7.869 | 7.954 / 7.931 |
| (29 + √1345)/3 | 21.8914 | 2 | 20.71 | 21.77 | 21.86 / 21.80 |
| 16 + 2√73 | 33.0880 | 1 | 32.64 | 33.44 | 33.29 / 33.16 |

**All nine are matched by both lattice forces (native SU(2) Whitney and two-ring), and the real parts converge
towards the exact values from n = 1 to n = 3.**

- The measured doubles carry small imaginary parts, ±0.05 to ±0.09, which do not shrink. That is expected: the
  lattice linearises in right-trivialised link coordinates at a field that is not at rest. There the Jacobian of
  the force is the Hessian plus a term proportional to the gradient, and that extra term is not symmetric.
- The real parts are what converge.

**Checks on the conventions.**
- The round metric gives the background gradient exactly 0 (the field is at rest), with sector eigenvalues
  −2 (×1), 0 (×3, gauge) and 10 (×5).
- The Berger background gradient is nonzero, as the S2b notes say ("the field is not at rest").
- With the commutator dropped, the round metric gives 4 (×9). That is the Abelian control's eigenvalue 4, which the
  paper measures as 4.001, 3.994 and 3.997.

## Space against field, exactly

The same sector with the non-Abelian commutator dropped, i.e. geometry only:

| | eigenvalues |
| --- | --- |
| Berger, abelianised | 4/3 (×6), 12 (×3): all positive |
| Berger, full SU(2) | −14/3 (×2), −2.558 (×2), −1.088, 8, 21.89 (×2), 33.09 |

**Every growth direction of the Berger field comes from the field acting on itself (the commutator).** None
comes from the curved space. That is the space/field separation the strategy note asks for, exact on this
sector.

## Three consequences

1. **A spectral gate is available, and it is cheap.**
   - Like BW05 for the force, the lattice operator's low spectrum now has an exact target. On the existing
     receipts it converges for both forces.
   - A gate written before the next measurement could ask for the sector's real parts within a fixed tolerance
     at n = 5, or for an observed order.
   - That needs only the n = 5 spectrum near these nine values, a shift-invert solve of the size B6 already ran.
2. **The S2b statement "the few negative eigenvalues belong to the field" becomes exact.**
   - The physical growth directions are **exactly five**: −14/3 (×2), −2.558 (×2) and −1.088.
   - The other negative-real eigenvalues the lattice shows (20 at n = 2, about 33 at n = 3, growing with n) are
     complex pairs. They are gauge-type directions: zero modes that the non-rest background tilts. Their number
     grows with the number of vertices. The paper can now say so, with the exact count.
3. **For the mass-gap observable (strategy step 2).**
   - On S³/2I with a left-invariant background, the entire low physical spectrum is this 9 × 9 algebra. It is
     fixed by the metric, the Lie algebra and the icosahedral selection rule.
   - An observable that only reads "the lowest excitation" here would be measuring that algebra, not a dynamical
     gap. On this manifold it is an exactly solvable control, not a test.
   - That is useful in itself: it is the falsifier the strategy note asked for. Any proposed mass-gap observable
     must reproduce these exact sector values on this background before it is trusted anywhere else.
   - A field that is *not* left-invariant, or the quantum theory, is where a gap would have to show something this
     sector does not already fix.

## What is not shown

- That nothing else lies below the spin-6 sector at finer meshes. Between 10 and 40, only the three sector values
  appear at n = 2 and 3. The next measured band starts near 48–53, but the exact spin-6 Berger values were not
  computed. That is a next exercise: one finite matrix per allowed spin.
- The rate. The n = 1 → 3 real parts approach the exact values, but no order is claimed from three levels with
  imaginary parts present.
- Anything about the quantum theory or four dimensions.

## Proposed next exercises (small, each with an exact answer)

- **The spin-6 sector on Berger:** a finite matrix, to confirm the gap above 33.09 exactly and to match the band
  near 50.
- **The same sector along the Ricci flow g(t):** the sector eigenvalues as exact functions of t. That gives the
  flow experiment an exact spectral trajectory to compare with, alongside the exact links.
- **Altitude:** STRUCTURAL. This is the operator the lattice discretises, restricted to an invariant subspace.
  It is not an analogy.

## The spin-6 sector (added the same evening)

Script: `scripts/ricci_su2_spin_sector.py`. Receipt: `research/su2-left-invariant-sector-20261119/spin_sectors.json`.

**How.** A sector of spin j/2 consists of su(2)-valued 1-forms whose coefficient functions lie in the V_j
component of L²(S³). Left-invariant vector fields act on it through the spin-j/2 representation matrices,
normalised to the frame's bracket [e_i, e_j] = 2e_k. So the Hessian, now with its derivative term
ê_i b_j − ê_j b_i, is an exact finite matrix of size 9(j+1). On S³/2I, sector j appears with the multiplicity of
icosahedral invariants in V_j. That multiplicity comes from the Molien series (1 + t³⁰)/((1 − t¹²)(1 − t²⁰)):
it is 1 for j = 0, 12, 20, 24, 30, … and 0 for j = 1 to 11.

**Controls, both passed.**
- At j = 0 the construction reproduces the 9 × 9 sector to 8 × 10⁻¹⁵.
- The round metric without the commutator gives exactly the coexact 1-form spectrum of the unit S³. Sector j
  has j² (×3(j−1)), (j+2)² (×3(j+3)) and gauge zeros (×3(j+1)). At j = 12 that is 144 (×33), 196 (×45) and
  0 (×39), and summed over copies it reproduces the known multiplicity 2k(k+2) for each Lie-algebra direction.
- Further checks: the round metric with the commutator has exact gauge zeros (its field is at rest), and the
  Berger metric without the commutator has no negative eigenvalue.

**The physical spectrum above the left-invariant sector.** Berger, full SU(2), j = 12 (spin 6), against the
measured lattice:

| exact (spin 6) | n = 2 (native) | n = 3 (native) | n = 3 (two-ring) |
| --- | ---: | ---: | ---: |
| 51.33 (×2) | 53.41 | 52.27 | 49.27 |
| 59.52 (×2) | 61.92 / 62.03 | 60.69 / 60.75 | 57.50 |
| 67.85 (×2) | 67.47 / 68.60 | 67.88 / 68.16 | 65.04 |
| 69.73 (×2) | 69.57 / 71.12 | 69.76 / 70.41 | 67.35 / 67.64 |
| 71.31 (×2) | 73.51 / 74.20 | 72.69 / 72.91 | 69.13 / 69.25 |

- The native force approaches these values from above, and the two-ring force sits below them at n = 3.
  Its rate is not measured.
- The next allowed sectors start far higher: spin 10 at **142**, spin 12 at **203.3**, spin 15 at **315.3**.

So the exact physical spectrum of the Berger field on Poincaré's sphere is:
- the left-invariant sector, from −14/3 up to 33.09;
- **a gap to 51.33**, where the spin-6 sector begins;
- nothing else below 142.

**The gauge-type modes.**
- Each sector j > 0 carries **j + 3** negative eigenvalues and no exact zeros. These are gauge directions tilted by
  the background, which is not at rest. Their size shrinks like 1/j: the most negative is −2.35 at spin 6, −1.73 at
  spin 10, −1.52 at spin 12 and −1.28 at spin 15.
- So the continuum Hessian has infinitely many negative eigenvalues, accumulating at 0 from below. All but five are
  gauge-type.
- Below −1.0 the exact count is **41**: the 5 physical, plus 36 from spins 6 to 20. The lattice shows 9 at n = 2
  and 13 at n = 3. That is consistent with it resolving the sectors one by one as the mesh refines.
- **Caveat.** On gauge-type directions the lattice linearises the force's Jacobian in link coordinates. That
  differs from the Hessian by a term proportional to the nonzero gradient, and gauge-type directions are exactly
  where that term matters. So these counts are a prediction for the Hessian, and a strong *hint* for the lattice:
  measured −1.86, −1.45, −1.14 and −1.02 against exact −1.81, −1.47, −1.11 and −1.03 at spin 6. They are not a gate.

**For the mass-gap observable.** The exact Berger spectrum on S³/2I now has a computable gap at every level of
detail: −14/3 … 33.09 | 51.33 … | 142 …. Any proposed observable must reproduce this exactly solvable structure
on this background first. A dynamical claim would have to be about something this representation-theoretic
structure does not already fix.
