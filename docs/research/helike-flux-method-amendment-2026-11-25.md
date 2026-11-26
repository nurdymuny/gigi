# Method amendment for the registered flux-edge grid

25 November 2026. **AMENDMENT, before any registered grid point is computed.** It replaces the retired LocalFock
method of the [flux-background registration](helike-flux-background-registration-2026-11-24.md) with the certified
cone method developed on the controls. The question, model, grid, predictions, tolerance and decision rule do not
change. No grid point has been evaluated under any method since that registration. The only exposure remains the
disclosed low-order smoke run at g² = 0.05, s ∈ {1.15, 1.20}, whose gaps were not inspected.

## 1. Why the method must change

The registered method, local Fock diagonalisation with a Taylor-expanded operator, is not bounded below: its
continuation reaches non-Gram matrices (`B = −I` at an explicit point). That is a defect of the method, not a
finding about the model, and the runner has held the grid on `HOLD_DOMAIN` since. The replacement has been
developed and checked on the four controls:
- the positive-cone chart, so every trial point is a genuine Gram matrix;
- exact moments, so there is no Taylor order P and no quadrature error;
- cutoff quasimodes in the physical operator's domain;
- rigorous residual, charge, tail and leakage bounds;
- Theorem A (discrete spectrum on the whole window, every g > 0);
- a Kato–Temple ordering under S_C.

## 2. What is unchanged

- **The model:** H_src, the sources a, b, the Maurer–Cartan background, colour singlets.
- **The grid:**
  - primary block t = 3, g² ∈ {0.03, 0.05, 0.07}, s ∈ {1.12, 1.15, 1.165, 1.18, 1.195, 1.22, 1.24};
  - secondary block t = 4, g² ∈ {0.03, 0.05}, s ∈ {1.09, 1.12, 1.14, 1.16, 1.18}.
- **The predictions** for the switch location, and the tolerance |measured − predicted| ≤ 0.015 in s.
- **The switch definition:** where the competitor's gap rises above Helike's, by linear interpolation across the
  s grid.
- **The decision rule,** per (t, g²) and overall, verbatim.
- **The identification rule:** Helike is the state of maximal weight on her |Q| = 2 single-quantum doublet, the
  competitor the state of maximal weight on the low Q = 0 mode, and both weights must be ≥ 0.9.
- **The secondary prediction** on |c_H| against |c_C| stays reported and not decisive.

## 3. What changes

### 3.1 Solver

The positive-cone Ritz solver with exact Kronecker moments (`helike-lab`, `helike cone`). The Python reference
(`helike_positive_cone.ConeRitz`) remains the parity oracle at Mq ≤ 6, and the four control parities are rerun
before the grid.

### 3.2 Convergence ladder

The old ladder (P, Mq) = (6, 6), (6, 8), (8, 8) refined a Taylor order that no longer exists. It is replaced by
**Mq = 6, 8, 10**, with the same rule: a point is converged if both identified gaps change by at most 0.003 on the
last step (Mq 8 → 10). The extra basis step takes the place of the retired Taylor step. The threshold is unchanged.

### 3.3 Domain validity (new, per point)

At the final rung, the Helike and competitor states of every point must pass the registered relative leakage gate,
ℓ_upper ≤ 10⁻³·r_lower, with the same rigorous machinery:
- the Arb cutoff and tail bounds, with the registered cutoff (D_s = 0.01 det G₀, R = 4‖G₀‖_F) and its geometry-only
  plateau rule;
- the interval residual enclosure.

A point that fails or refuses the gate is not converged, and its (t, g²) is UNRESOLVED under the unchanged decision
rule. This makes the "tunnelling to the origin region is neglected" approximation of the original registration a
measured condition at each point instead of an assumption.

### 3.4 Certificates (new, reported, not decisive)

At each converged point, the receipt also reports:
- the rigorous residual and charge bounds;
- the projected residual bound R;
- the Kato–Temple verdict (ORDERED or UNRESOLVED) under the frozen hypothesis S_C with the same ρ_C formula.

These numbers are not inputs to the registered decision, which is about the computed switch location. A
SUPPORTED verdict will be reported together with whether the certificates order the two levels on each side of the
switch, conditional on S_C. That is a strictly stronger, separately labelled statement.

### 3.5 Runner and release

- **A new runner,** `scripts/run_helike_flux_background_cone.py`, implements §3.1–3.4. It must be committed with
  tests before any grid point is computed. The tests cover:
  - parity on the controls;
  - refusal on a missing rung;
  - refusal when the leakage gate fails;
  - that the decision rule is unchanged (the original tests, ported);
  - a mutant that relaxes 0.003, 0.9 or 0.015 must be caught.
- **The old runner** stays on HOLD_DOMAIN permanently. It is never released.
- **`unmet_release_conditions`** gains the reading that this amendment satisfies `method_amendment` once the new
  runner is committed. `convergence` and `certificate` are satisfied by the control receipts already archived.
- **The grid runs on the big machine.** Mq = 10 is 8,008 trial functions; it is CPU work for the matrix-free
  solver, not a GPU requirement.

## 4. What this amendment does not do

It computes nothing and inspects no grid point. It does not change the threshold of any test, the predictions, or
the definition of the switch. It does not make S_C a theorem. Its certificates are reported, conditional on S_C, as
before.

## 5. Runner note, 26 November: fixes before the grid (no grid point computed)

These are written and committed before any registered point is computed. They change no question, model, grid,
prediction, tolerance, identification or decision threshold. Before them, the runner could not complete the grid at
all on this machine.

1. **Domain gate at the final rung was impossible.**
   - `helike_cutoff_bounds.polynomial_bounds` refused total degree above 8, the scope of the Mq = 6, 8 controls it was
     registered on. §3.3 puts the gate at the final rung, Mq = 10, so every point would have been OPEN and every case
     UNRESOLVED by construction.
   - The inequalities of §3 of the cutoff registration (Hermite majorants k_n, the exact six-dimensional radial tail
     T_k) hold for any m. The cap is now 10 (`MAX_DEGREE`).
   - Checked: cutoff tests 19/19 and cutoff mutants all caught. At the control (t = 3, g² = 0.002, s = 1.15; not a
     grid point), the Mq = 10 gate PASSES all four states, with leakage ≤ 7·10⁻²⁴ against r_lower ≥ 2.7·10⁻⁴.
2. **Pinned receipts read as changed on a CRLF checkout.** The release check hashed raw bytes, while the pins are LF
   digests. It now uses the original runner's `sha256-universal-newline-v1`, as the domain gate already did.
3. **The binary path missed `.exe` on Windows**, in both the runner and `helike_domain_gate`.
4. **One point could crash the grid.**
   - A rung failing validation in `assess_point` is now a refused point, so its case is UNRESOLVED under the
     registered rule.
   - Any exception inside the domain stage is OPEN, as before, but now for every exception type.
5. **An ambiguous crossing could crash the grid.** `switch_point` refuses multiple crossings, or a zero difference
   without a unique interior bracket. The registered rule has no outcome for this. The case is reported **UNRESOLVED
   with reason "ambiguous crossing"**: fail-closed, claiming neither SUPPORTED nor NOT_SUPPORTED.
6. **Provenance.** The receipt now records:
   - the commit and dirty flag, the binary's SHA-256, LF digests of the runner and gate sources and of the
     registration and amendment, and the python-flint version;
   - atomic writes with an integrity file. Non-finite numbers are written as strings, not dropped.

Tests: `test_helike_flux_grid.py` passes 23/23; six were added for the items above. The grid-runner mutants are all
caught. The grid runs on this machine (Mq = 10 at the control: about 75 s for the solve and 75 s for the gate).
