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

### 5.1 First registered attempt: void (every point refused before any gap existed)

The first run under §5 (`research/flux-grid-20261126-VOID-quadrature/`) ended UNRESOLVED in about 2 minutes, but
the run was void: every one of the 31 points was refused by the solver before producing a gap or weight. Reproduced
at (t = 3, g² = 0.03, s = 1.12, Mq = 6), the refusal was "reference weights unresolved at frozen overlap tolerance:
7.6·10⁻⁵". No gap, weight or level was computed or seen. The runner had also discarded the solver's message; it now
records it.

**Cause and fix (a numerical-method error, not physics).**
- The identification weights are Gauss–Hermite overlaps with the linear-Gram harmonic projectors. The integrand is
  not polynomial in the chart, and the solver fixed the orders at (10, 12) with a 10⁻⁶ agreement tolerance. Those
  orders resolve the g² = 0.002 and 0.004 controls, but not the registered couplings.
- The order is now raised in steps of 2, up to 28, until consecutive orders agree to the **unchanged** 10⁻⁶. The
  first resolved pair is used, so the controls reproduce exactly. The orders used are written to the receipt.
- The state export (domain gate) picked Helike's pair with a hard-coded order 12. It now uses the identification's
  resolved order.
- Checked at unregistered couplings only. The orders converge by (16, 18) or (18, 20) (error ≤ 7.9·10⁻⁷) at:
  - t = 3, g² = 0.04 and 0.08;
  - t = 4, g² = 0.06.
- Rust tests all pass, and the Python grid tests pass 23/23.

**Exposure, disclosed.** Checking the full chain at the unregistered point (t = 3, g² = 0.04, s = 1.12, Mq = 10)
showed its gaps and weights:
- Helike gap 1.0899, weight 0.858;
- competitor gap 0.9599, weight 0.757.

So identification fails there: both weights are below the registered 0.9. The domain-gate export, which applies the
quasimode selection rule's 0.95, refuses with "unresolved Helike partner identification". No registered point has
been computed. The registered grid is not rerun until the identification question is settled; see §5.2 when written.

### 5.2 Identification relative to the dressed vacuum (REGISTERED before code; Bee's decision, 26 November)

**The defect.** The registered identification measures a level's weight on the *bare* harmonic single-quantum state
a†_m|0⟩. Suppose a single-quantum level is the dressed vacuum with one quantum added: |m⟩ ≈ U a†_m|0⟩, with U the
coupling's dressing. Its bare weight then carries the vacuum's own overlap factor, Z = |⟨0|U|0⟩|², at leading order.
That factor is shared by every level, and it falls as g² grows. The 0.9 threshold was set where Z ≈ 1 (g² ≈ 0.002,
weights ≥ 0.98). It was meant to test that a level is *its channel's* single quantum, not to test how far the vacuum
is dressed. At the registered couplings the shared factor alone can push a correctly identified level below 0.9. At
the unregistered g² = 0.04 the bare weights were 0.858 and 0.757 (§5.1).

**The amended rule.**
- Let Z be the ground state's weight on the harmonic vacuum |0⟩. It uses the same linear-Gram harmonic reference, the
  same Gauss–Hermite quadrature, and the same resolved order as the single-quantum weights.
- For each level and channel, the **dressed weight** is w̃ = w / Z.
- **Selection is unchanged.** Helike is the level of maximal weight on her doublet; the competitor is the level of
  maximal weight on the low Q = 0 mode. Dividing by a common Z does not change either argmax.
- **Thresholds are unchanged and now apply to w̃:**
  - the registered identification needs w̃ ≥ 0.9 for both identified levels at the final rung;
  - the domain gate's quasimode selection needs w̃ ≥ 0.95 for both Helike partners and for the competitor.
- **Numerics.** Z enters the order-agreement check, with the same 10⁻⁶ tolerance as the weights.
- **Receipt.** The bare w, Z and w̃ are all written to the receipt.

**What this does and does not do.**
- It changes neither the gaps, nor the switch, nor the convergence rule, nor any threshold.
- It removes the leading common factor and nothing else. A level whose single-quantum character is genuinely mixed
  (for example, with a two-quantum state of another channel) still has w̃ well below 1.
- w̃ can exceed 1 at higher order. That is reported, and it does not change the rule.

**Checks before the grid.**
- (a) The g² = 0.002 and 0.004 controls: Z ≥ 0.98, and every identification there is unchanged.
- (b) Tests: w̃ = w/Z in the solver output; the runner uses w̃; and a mutant that drops the normalisation (uses w) is
  caught.
- (c) The unregistered exposure point of §5.1 (g² = 0.04, s = 1.12) is recomputed, and its w̃ reported, whatever it
  is. The rule and thresholds above are fixed by this commit and are not adjusted after seeing it.

**§5.2 checks, run after the code and before the grid.** Log: `research/flux-grid-amendment-5.2-checks.txt`.
- **(a) Controls** (t = 3, s = 1.15, Mq = 6, 8, 10): **PASS.**
  - Z = 0.9965 at g² = 0.002 and 0.9929 at g² = 0.004.
  - Every identification is unchanged. Dressed weights are 0.9941–0.9979 at g² = 0.002 and 0.9878–0.9957 at
    g² = 0.004. Orders are (10, 12), except (12, 14) at g² = 0.004, Mq = 10.
- **(b) Tests:** 26/26 pass, including three new ones for w̃. Grid-runner mutants: 10/10 caught, including the new
  `bare_weight_identification`. Rust tests: all pass.
- **(c) The exposure point** (t = 3, g² = 0.04, s = 1.12, Mq = 10), reported as found:
  - Z = 0.9055;
  - Helike: w = 0.8575, w̃ = **0.9471**;
  - competitor: w = 0.7574, w̃ = **0.8365**.

  The dressing removes Helike's shortfall, but not the competitor's. The competitor's mixing at that point is specific
  to its state, and the rule fails it, as registered. The domain-gate export refuses there as well (w̃ < 0.95).
  Nothing is adjusted. The registered grid now runs under §5.2.

### 5.3 Dressed-overlap cross-check (report-only; Bee's request, 26 November)

**What is added.** A second identification number is written next to w̃ = w/Z and never used by the decision rule. With
G the computed ground state and y_m the same linear-Gram harmonic coordinates as the registered reference, set
b_m = y_m G − ⟨G|y_m G⟩G. The **dressed overlap** of a level ψ on a channel is the squared norm of its projection onto
span{b_m : m in the channel}: d = vᵀB⁻¹v, with v_m = ⟨b_m|ψ⟩ and B the Gram matrix of the b_m (2×2 for Helike's
doublet).
- It is the registered reference a†|0⟩ = √(2w)·y|0⟩ with the bare vacuum replaced by the actual ground state.
- When G is Gaussian, it equals |⟨ψ|a_m†|G⟩|²/‖a_m†G‖².
- It needs no derivatives of the chart functions: every term is a node sum on the same Gauss–Hermite grid at the
  resolved orders.

**Receipt.** Per rung:
- `cross_check.dressed_overlap` for Helike and the competitor;
- `cross_check.dressed_overlap_error`, the |fine − coarse| at the two resolved orders;
- `cross_check.dressed_overlap_orthonormality`, the max |⟨G|ψ_c⟩ − δ_{0c}| on the grid.

A failure writes null. It never refuses a point.

**Code.** `dressed_overlap_terms` and `dressed_overlap_weight` (`ritz.rs`), and the runner field.
- Tests: the Rust suites pass (the 5.2 tests unchanged), and the runner passes 27/27, two of them new. One new test
  checks that the cross-check cannot change `assess_point`; the other checks that the runner records it.
- The amendment 5.2 code path is untouched.

**Values seen (logs in `research/flux-grid-crosscheck-20261126/`):**

| point | Z | Helike w / w̃ / dressed overlap | competitor w / w̃ / dressed overlap |
|---|---|---|---|
| control g² = 0.004, Mq = 6 | 0.9929 | 0.9885 / 0.9956 / 0.9990 | 0.9808 / 0.9878 / 0.9964 |
| control g² = 0.002, Mq = 6 | 0.9965 | 0.9943 / 0.9979 / 0.9995 | 0.9906 / 0.9941 / 0.9982 |
| exposure g² = 0.04, s = 1.12, Mq = 10 (unregistered) | 0.9055 | 0.8575 / 0.9471 / **0.9897** | 0.7574 / 0.8365 / **0.9705** |

Fine and coarse orders agree to ≤ 10⁻¹². The grid orthonormality is ≤ 1.2·10⁻¹³.

**Reading, stated plainly.** At the exposure point the competitor is a clean single quantum on the dressed ground
state (0.971). Its w̃ = 0.837 falls below 0.9 because of higher-order dressing, not because of mixing with another
channel. (I had predicted mixing; that was wrong.) Under the registered rule (5.2), points like this would be
UNRESOLVED for an identification reason, not a physical one. **The rule is not changed here.** Any change would be a
new amendment, disclosed as made after seeing these values.

### 5.4 Grid restart for the §5.3 receipt fields (26 November)

- A registered run under §5.2 was started at commit 9e422f61. It was stopped after 6 of 31 points so that the
  registered receipt carries the §5.3 cross-check at every rung.
- Its partial receipt is archived **unread** in `research/flux-grid-20261126-SUPERSEDED-9e422f61/`. No gap, weight
  or verdict from it was looked at before the restart.
- The rule did not change between the two commits: §5.3 is report-only, and a test shows that `assess_point` cannot
  see it. The solver is deterministic, so the restart can change no decision.
- The registered run is `research/flux-grid-20261126/`, at 93a6de65. Before it was launched:
  - runner tests passed 28/28;
  - grid mutants were 10/10 caught;
  - no release conditions were unmet.
