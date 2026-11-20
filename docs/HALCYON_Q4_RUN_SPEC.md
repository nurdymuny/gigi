# Q4 run spec: candidate lower bounds, fixed before the data they are tested on (exploratory)

2026-11-19. Written while the Q3 ensembles are running and before any Q3 correlator is computed.
Branch `feat/ym-engine`. Parent: `docs/HALCYON_QUANTUM_GAP_SPEC.md` §Q4.

**Labelled exploratory.** Q4 proves nothing. A candidate that survives every measured point is a
conjecture worth trying to prove, and nothing more. The parent spec orders Q4 after Q1–Q3 pass. This
spec only fixes the candidates now, so the Q3 points are new data for them. Testing waits for Q3, and
uses only the Q3 points that pass Q3-a and Q3-v. On record: Q2-a missed at β = 4 in the fresh run
(pull −2.04; Q2 results).

## The shape being hunted

In the Floor Theorem an argument covered every sector beyond j = 30 and the computer covered four. The
quantum analogue in the parent spec: a bound on the gap in physical units, m₀₊₊ ≥ c √σ with c > 0, for
every spacing below some a₀, so that only finitely many spacings above a₀ are left to compute.

Published SU(2) data (LTW, hep-lat/0404008, Table 5; ratios computed here from am₀₊₊ and a√σ):

| β | a²σ | m₀₊₊/√σ | −4 ln u, u = I₂(β)/I₁(β) | am₀₊₊ |
|---|---|---|---|---|
| 2.1768 | 0.2651 | 3.158(120) | 3.098 | 1.626 |
| 2.2986 | 0.1345 | 3.305(62) | 2.944 | 1.212 |
| 2.3715 | 0.0829 | 3.453(64) | 2.857 | 0.994 |
| 2.3726 | 0.0829 | 3.526(47) | 2.856 | 1.015 |
| 2.4265 | 0.0570 | 3.492(56) | 2.795 | 0.834 |
| 2.5115 | 0.0313 | 3.722(59) | 2.703 | 0.658 |
| continuum | 0 | 3.78(7) | | |

The ratio rises toward the continuum value at every step: the approach is from below. If that held at
every spacing, the coarsest spacing would set a floor uniform in a, which is the shape wanted.

## Candidates (constants from the published data above, never from ours)

- **Q4-C1, monotone approach from below.** Among the points tested, m₀₊₊/√σ never decreases as a²σ
  decreases: for every pair with a²σ_i > a²σ_j, ratio_i ≤ ratio_j + 2σ_comb. A violation is any pair
  where the coarser point is higher by more than 2 combined σ.
- **Q4-C2, a floor uniform in the spacing.** m₀₊₊/√σ ≥ 3.158 at every point tested, within 2σ.
  3.158 is LTW's coarsest point (a√σ = 0.515). Together with C1 this is the conjecture: for
  a√σ ≤ 0.52, the lattice ratio stays above about 3.16.
- **Q4-K, a control that must fail.** am₀₊₊ ≥ −4 ln(I₂(β)/I₁(β)), the leading strong-coupling form of
  the gap (a glueball as the smallest closed surface of four plaquettes). Expected: false at every
  point in this window. LTW's data are below it by 0.9–2.0 in lattice units. If it holds anywhere in
  our data, the measurement or the harness is suspect, and Q4 stops until that is understood.

Ruled out before testing, and recorded so it is not reintroduced: the straight line from LTW's
coarsest point to the continuum (ratio ≥ 3.78 − 2.35·a²σ). LTW's own β = 2.2986 point already
violates it by 2.6σ.

## Data

The Q3 points that pass Q3-a and Q3-v (ratio from the joint jackknife), and β = 2.5115 if it is run.
Any candidate suggested by those results is tested only on later, new data, under a new spec.

## Gates (procedural)

- Every result names, for each candidate, the β and L of every point where it fails.
- If C1 and C2 survive, they are stated as a conjecture with the window they were tested in, and the
  statement says the window is three or four spacings.
- If K holds anywhere, the run is stopped and the reason found before anything else is reported.

## What a proof would need (recorded, not attempted here)

C1 is a statement about the sign of the lattice artefact in m₀₊₊/√σ at every spacing, not only the
leading O(a²) term. No argument for it exists here yet. The finite-check structure needs C1 for all
a < a₀ from an argument, and C2 at finitely many a ≥ a₀ from measurement with errors. That second part
is the only one the engine can supply.
