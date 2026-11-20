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
