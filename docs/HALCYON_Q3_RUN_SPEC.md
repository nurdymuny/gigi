# Q3 run spec: scaling of m₀₊₊/√σ in 4D SU(2), measured in the engine

2026-11-19. Written before any Q3 code or data. Branch `feat/ym-engine`. Parent:
`docs/HALCYON_QUANTUM_GAP_SPEC.md` §Q3. Estimator: the Q1 verb with Amendment 1 (3σ correlator floor,
2σ consecutive agreement).

## The reference, fixed now

B. Lucini, M. Teper, U. Wenger, *Glueballs and k-strings in SU(N) gauge theories: calculations with
improved operators*, arXiv:hep-lat/0404008 (2004). Read for this spec from the arXiv PDF.

- **Table 14, continuum limit, SU(2):** m₀₊₊/√σ = **3.78(7)**. (Also m₀₊₊⋆/√σ = 6.46(14),
  m₂₊₊/√σ = 5.45(11).)
- **Table 5, SU(2) on L⁴ lattices** (the calibration points):

  | β | L | a√σ | am₀₊₊ | am₀₊₊⋆ |
  |---|---|---|---|---|
  | 2.2986 | 10 | 0.3667(18) | 1.212(22) | 2.00(11) |
  | 2.3715 | 12 | 0.2879(13) | 0.994(18) | 1.64(7) |
  | 2.4265 | 16 | 0.2388(9) | 0.834(13) | 1.47(4) |
  | 2.5115 | 20 | 0.1768(8) | 0.658(10) | 1.100(17) |

- **String tension from the flux loop, their eq. (14):** the lightest flux loop winding a spatial
  direction of length L has am(L) = a²σL − π/(3L), stated to be accurate for k = 1 once aL√σ ≥ 3.

**A note that shapes the run.** Halcyon's M1 values at β = 2.3 (2.24, 2.05, 1.98, 1.95 at
L = 6, 8, 12, 16) are about 60% above LTW's am₀₊₊ = 1.212(22) at β = 2.2986, and close to their
excited 0⁺⁺⋆ = 2.00(11). Q1 found that the unsmeared plaquette correlator has one significant time
step at 100 configurations. The likely reading is excited-state contamination of a raw operator read
at t = 1. Q3 tests that directly (mechanism removal below), not by assumption.

## What the engine needs (none of it exists yet)

1. **A 4D hypercubic SU(2) sampler**, self-contained like `lattice2d`: Kennedy–Pendleton heat bath
   (the engine's `sample_su2_link`) plus overrelaxation, own seeded generator, Wilson action. It is
   a new verb, so it does not inherit `GIBBS_SAMPLE`'s sequential-order bit-identity contract.
2. **Spatial APE smearing**: U_i → Proj_SU(2)[U_i + α Σ (spatial staples)], α = 0.5, at levels
   n ∈ {0, 4, 8, 16}. Time links are never smeared.
3. **Operators, zero momentum, per time slice:**
   - 0⁺⁺: the sum of ½ tr of all spatial plaquettes at smearing level n;
   - the flux loop: the sum over transverse positions of ½ tr of the smeared Polyakov line winding
     each spatial direction (three directions averaged).
4. **A variational step (GEVP)** over the four smearing levels, with t₀ = 1 fixed now. The
   effective mass of the lowest GEVP eigenvalue goes into EFFECTIVE_MASS. This is the standard way to
   suppress excited states and is fixed before data, so no smearing level is chosen after looking.

## The run

- β ∈ {2.2986, 2.3715, 2.4265} at LTW's L (10, 12, 16), and a second volume at each:
  L = 14, 16, 20. β = 2.5115 (L = 20, 24) only if the first three pass and the cost allows. That is
  decided before its data is read.
- β = 2.3, L = 16, with and without smearing: the direct comparison with Halcyon's M1.
- Each sweep is one heat bath plus four overrelaxation sweeps; thermalisation 1,000 sweeps; one
  measurement per sweep. The number of measurements is set from a timing run (which reads no
  spectroscopy) and recorded in the run log before any correlator is computed. Target: at least
  10,000 measurements per point, binned jackknife with blocks of 100.
- Seeds: 20261193 + 1000·k for point k, in the order listed above.

## Gates (fixed now)

- **Q3-0, sampler.** (a) At β = 0.5 the mean plaquette agrees with the strong-coupling value
  u = I₂(β)/I₁(β) within 4σ plus 5u⁵, the size of the first correction. (b) On a 4⁴ lattice at
  β = 2.3, heat bath and plain Metropolis give the same ⟨P⟩ within 4σ. (c) Gauge invariance of
  every operator to 1e-12.
- **Q3-a, calibration.** At each of LTW's (β, L), a√σ and am₀₊₊ agree with Table 5 within 2 combined
  σ. A refusal at a point is a failure of that point, not a pass.
- **Q3-v, volume.** At each β the two volumes agree on am₀₊₊ and a√σ within 2 combined σ. A point
  with aL√σ < 3 is refused for σ by the verb, with the reason.
- **Q3-s, scaling.** Over the β points that pass Q3-a and Q3-v, m₀₊₊/√σ fitted as c + d·a²σ gives c
  within 2σ of 3.78(7). Falsifier, fixed now: if m₀₊₊/√σ drifts monotonically by more than its errors
  and the fit has χ²/dof > 2, the window is not in scaling and no structural conclusion is drawn.
- **Q3-r, mechanism removal.** At β = 2.3, L = 16: with smearing and GEVP off (the raw plaquette, as
  Halcyon's M1), EFFECTIVE_MASS either refuses or returns a value more than 3σ above the smeared GEVP
  value. If the two agree, smearing is not doing what it is claimed to do here.
- **Q3-n, nulls.** Every channel refuses on its configuration-and-time shuffled series.

## Amendment 1 (adopted by Bee 2026-11-19, before any Q3 code beyond the sampler or any Q3 data)

Plateau-rule Amendment Q1-3: the verb's plateau error is the blocked jackknife of the plateau fit
itself (window and weights fixed from the full sample). The fit's 1/√Σw was about 25% too small on
long windows in the Q2 fresh run. Every Q3 gate reads the jackknifed error. The GEVP projection
vector is fixed from the full sample and not re-solved per jackknife sample (standard; recorded as
such).

## Order of work

1. Sampler and its Q3-0 tests.
2. Smearing, operators, GEVP, with tests (gauge invariance; GEVP on a synthetic two-state correlator
   recovers both masses).
3. Timing run, then statistics fixed in the run log.
4. The production points, in the order above.

## Not in this run

- SU(3). Q4. Any claim beyond "the lattice ratio at these spacings, against LTW's continuum value".
