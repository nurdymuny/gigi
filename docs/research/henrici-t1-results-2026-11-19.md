# T1: Henrici's departure from normality of the linearised SU(2) forces (19 November 2026)

**Results note** for test T1 of [`adjacent-ideas-grid-2026-11-19.md`](adjacent-ideas-grid-2026-11-19.md).

- Script: `scripts/ricci_su2_henrici_t1.py`.
- Receipt: `research/henrici-t1-20261119/t1.json`.
- Operators: the same K matrices as the S2b–S2d spectra, read from the S3b cache. Each is whitened to
  G = L⁻¹KL⁻ᵀ in the native mass norm.

## What T1 predicted

My Crouzeix curvature flow is Yang–Mills gradient descent (grid note §2.1). Along it, the energy dissipated is
½·dep², where dep² = ‖G‖²_F − Σ|λ|² is Henrici's departure from normality. The prediction was that dep should
track the growth of the two-ring force's max |Im λ| (104, 150, 418 at n = 1, 2, 3).

## Result: the prediction fails

| Operator | dof | dep | dep / ‖G‖_F | max \|Im λ\| | ‖skew‖ / ‖sym‖ |
| --- | --- | --- | --- | --- | --- |
| native n = 1 | 426 | 65.5 | 5.1 × 10⁻³ | 0.80 | 3.6 × 10⁻³ |
| native n = 2 | 3,372 | 104 | 7.4 × 10⁻⁴ | 0.63 | 5.3 × 10⁻⁴ |
| native n = 3 | 11,358 | 132 | 2.3 × 10⁻⁴ | 0.64 | 1.6 × 10⁻⁴ |
| two-ring n = 1 | 426 | 1,956 | 0.224 | 104 | 0.166 |
| two-ring n = 2 | 3,372 | 19,030 | 0.168 | 150 | 0.120 |
| two-ring n = 3 | 11,358 | 68,920 | 0.149 | 418 | 0.106 |
| one-ring n = 3 | 11,358 | 145,800 | 0.259 | 1,306 | 0.189 |

- **The absolute departure does not track max |Im λ|.** From n = 1 to 2, dep grows 9.7× while max |Im λ| grows
  1.45×. From n = 2 to 3, dep grows 3.6× while max |Im λ| grows 2.8×. The departure follows the operator's
  norm, which grows with refinement (stiffness).
- **The relative departure falls while max |Im λ| rises.** For the two-ring force, dep/‖G‖_F goes 0.224, 0.168,
  0.149. So the growth in max |Im λ| is not growing non-normality. It is the spectrum spreading as the mesh
  refines, which a normal operator could do just as well.
- **Retraction.** I had cited the 104/150/418 series as "non-normality grows with refinement" (R7). That reading
  is wrong. Measured by Henrici's departure, the two-ring force becomes slightly *more* normal relative to its
  size.

## What the departure does show

- **The native force tends to normal, fast.** Its relative departure falls 6.9× from n = 1 to 2 and 3.3× from
  n = 2 to 3. With h ∝ 1/n, that is about h^2.8 and h^2.9: roughly third order.
- **The two-ring force keeps a persistent non-normal part.** Its relative departure falls only like
  h^0.4 to h^0.3. It may approach a nonzero limit near 0.14; three levels cannot tell. This is the
  Petrov–Galerkin, non-variational part of the force (R3) made visible as a matrix property. A variational force
  would be symmetric in the mass norm, with departure zero.
- **The one-ring force is the least normal** (0.259 at n = 3): exact determination (R2) shows up here too.

## Checks

- **Eigenvalues.** Σ|λ|² from the real Schur form agrees with the committed S2d spectra to 6 × 10⁻¹⁵ or
  better in every case.
- **Henrici's upper bound** dep² ≤ √((N³ − N)/12)·‖κ‖_F holds everywhere (ratio 6 × 10⁻⁸ to 9 × 10⁻⁴).
  The bound is far from tight, as expected at this size.
- **The flow identity on real operators** (n = 1, starting from A₀ = G/‖G‖_F, RK45, rtol 10⁻⁹):
  - ∫₀ᵗ‖κ‖² + ½·dep²(A(t)) = ½·dep²(A₀) holds at every checkpoint, to 1.0 × 10⁻⁹ (two-ring) and
    3.5 × 10⁻⁸ (native).
  - The spectrum is preserved to 2.3 × 10⁻¹¹.
  - By t = 400 the flow has dissipated 79% (two-ring) and 46% (native) of the energy ½·dep². Convergence to
    normal is slow; the identity does not need it.

## Consequences

- R7's wording in the grid note is corrected: max |Im λ| measures spectral spread, not non-normality.
- The numerical-range results stand. The left edge of W(G) bounds transients whatever the departure.
- **A cleaner diagnostic for the variational programme:** dep/‖G‖_F of the mass-normalised Jacobian. For a
  force that is the gradient of an energy it is exactly zero, so M0 and its successors can be scored on it
  directly.
