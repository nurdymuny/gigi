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

### 5.5 Gluing-invariance test (REGISTERED before any grid value is seen; report-only qualifier; Bee's principle, 26 November)

**The principle.** The gluing must not become part of the reading. Every reading the grid reports is stitched together
from choices: a truncation, a reference vacuum, a quadrature, an interpolation between grid points, a cutoff for the
certificate. A reading that belongs to the field does not move when the stitching changes. A reading that moves
belongs to the seams. This section registers which seams are varied, what must not move, and what is reported. It is
fixed now, while the registered grid of §5.4 runs (`research/flux-grid-20261126/`, commit 93a6de65), and before any
gap, weight or verdict from that run has been seen.

**Relation to the registered grid.** The test is a **qualifier**. It never changes a registered point verdict, case
verdict or the overall verdict. Every published statement of a grid verdict must carry the qualifier beside it, for
example "SUPPORTED; gluing-invariant", or "SUPPORTED; gluing-sensitive at the interpolation seam". It runs after the
registered grid completes. It uses the grid's receipts, plus re-solves of the final rung (Mq = 10) at each point that
the grid did not refuse.

**The seams varied, and what must not move.**

| seam | variation | must not move (tolerance) |
|---|---|---|
| S1. Identification reference | Select Helike and the competitor by (a) the bare weight w, (b) w̃ = w/Z (registered), (c) the dressed overlap of §5.3, computed for **every** candidate level, not only the selected one | The selected level indices: identical under (a), (b) and (c) |
| S2. Quadrature | The resolved order pair (o, o+2) against the next pair (o+2, o+4) | Selected indices identical; w̃ changes ≤ 10⁻⁵ |
| S3. Truncation | The registered Mq ladder 6 → 8 → 10 (already registered as convergence) | Restated as a seam: the last-step change ≤ 0.003, unchanged from §3 |
| S4. Switch interpolation | Registered linear bracket crossing; quadratic through the three points nearest the bracket; natural cubic spline through all points of the case | The measured switch s*: max pairwise difference ≤ 0.005 (one third of the registered s-tolerance 0.015) |
| S5. Certificate cutoff | The domain gate at its registered plateau radius against the next registered radius that also proves a complete plateau ball (if none exists, S5 is reported as not testable at that point) | The relative-gate verdict (PASS / FAIL / OPEN) for Helike and the competitor: identical |

Notes on the seams:
- S1 also asks whether the identity of the lowest Helike-channel and competitor levels depends on the vacuum it is
  measured against. Under (c), selection is by the largest dressed overlap among levels 1 onward, the same argmax
  rule as the registered selection.
- S4 tests whether a switch point is a property of the gap curves or of how the curves are drawn between points.
- S5 tests that the certificate's verdict does not depend on where the cut is made.

**Verdicts.**
- **Per point:** GLUING-INVARIANT if S1, S2, S3 and S5 all hold. Otherwise GLUING-SENSITIVE, naming each seam that
  moved, with the size of the move.
- **Per case:** the switch is GLUING-INVARIANT if S4 holds and every point within one grid spacing of the bracket is
  GLUING-INVARIANT. Otherwise GLUING-SENSITIVE.
- **Overall:** the number of invariant points and cases, reported next to the registered overall verdict.

There is no "fail" outcome that overturns physics. A seam-sensitive reading is reported as a property of the method
at that point.

**Code and tests, before any variation is computed.** The solver gains an option that writes the dressed overlap of
every level (an extension of §5.3, report-only), and the runner gains a `gluing_check` stage that reads the
completed grid receipt. Tests:
- the qualifier cannot change `assess_point`, `assess_case` or `overall`;
- each seam's tolerance is enforced as registered;
- S4 computes all three interpolants on synthetic cases with known crossings.

Mutants that must be caught:
- S1 compares only the selected level instead of all candidates;
- S4 tolerance loosened;
- the qualifier allowed to flip a verdict;
- S5 reusing the same radius twice.

**Controls first.** The test runs at the four g² = 0.002 and 0.004 controls before the grid points. Expectation,
stated now: invariant at all four. If a control is seam-sensitive, the test is reported as not yet meaningful and
does not run on the grid until the cause is understood and disclosed.

**Known context, disclosed.** At the unregistered exposure point (§5.1, §5.3), the dressed overlaps of the
**selected** levels were 0.990 (Helike) and 0.971 (competitor). Whether the dressed overlap selects the same levels
there is unknown; it was not computed. Van den Heuvel (hep-lat/9604017; Leiden thesis 1996) reported a 0⁺/2⁺
crossing on S³ that he read as a breakdown of the effective Hamiltonian, not as physics. That is the kind of reading
this test is built to separate from a real switch.

### 5.6 Gluing test, first stage at the controls: one definition fixed; one real finding about the gate (26 November)

The registered first stage of §5.5 ran on the four controls: g² = 0.002 and 0.004 at s = 1.15, t = 3, with Mq = 8 and
10 (`research/gluing-check-20261126/controls/`). As §5.5 requires, the result is reported, and the test does not run on
the grid until the causes are understood and disclosed. No grid value has been seen.

**S1 and S2, a definition fixed (cause understood).**
- At Mq = 10, the dressed reference selected level 2 where the registered rule selects level 1. Levels 1 and 2 are
  Helike's two degenerate partners: energies equal to well under 10⁻⁶, dressed overlaps both about 0.999. The argmax
  flipped on a negligible difference between the partners.
- S1 compared raw level indices. It should compare **degenerate clusters**, levels within 10⁻⁶ in energy, as the
  heat-signal amendment 2 does. S1 and S2 now compare clusters (`cluster_of` in `run_helike_gluing_check.py`). A new
  test covers the partner case, so 10 tests pass and the registered mutants stay 4/4.
- Re-evaluated on the same receipts, S1 and S2 hold at all four controls: clusters (1, 2) and (3).

**S5 at g² = 0.004, Mq = 10: a real finding.**
- At the registered plateau radius (ρ = 9.75), the relative leakage gate **fails** for both Helike partners:
  ℓ/r = 5.58·10⁻⁶ / 4.03·10⁻³ = 1.39·10⁻³, against the registered limit of 10⁻³. The competitor passes, with ratio
  4.1·10⁻⁴.
- At the next proving radius (ρ = 9.5), the competitor fails too.

**Why.** At the same control, Mq = 8 passes with room to spare: Helike's ℓ/r is 3·10⁻⁵, with r = 8.8·10⁻³ and
ℓ = 2.7·10⁻⁷. From Mq = 8 to 10, the residual halves because the basis improves, while the leakage grows about twenty
times because the richer states reach further toward the cutoff. **The registered relative gate becomes stricter
exactly as the solution converges.** At g² = 0.002 the plateau radius is larger (13.75) and the gate passes trivially,
with ℓ/r ≈ 10⁻²¹.

**Consequence for the registered grid (stated before any grid value is seen).**
- §3 requires a domain-gate PASS at the final rung, Mq = 10, for a point to be converged.
- The gate already fails at the g² = 0.004 control. The grid couplings (0.03 to 0.07) are stronger, with smaller
  plateau radii.
- The gate may therefore fail at many grid points. Such points would be "not converged", and their cases UNRESOLVED,
  for a reason that belongs to the cut, not to the field. This is the §5.5 principle applied to the gate itself.

**Decision: Bee's.** The registered grid is running at 93a6de65, and its rule is not changed here. Options for her
decision, to be recorded as a separate amendment, with the timing disclosed:
1. Let the registered run stand, and read UNRESOLVED points as gate-limited.
2. Register a gate that does not tighten with convergence, before any grid value is seen. For example, apply it at the
   rung where the gap change is measured, or use an absolute leakage bound tied to the gap tolerance. Then rerun.
3. Run the registered grid to completion, and separately rerun under option 2 as a disclosed second analysis.

### 5.7 A domain gate that does not tighten as the basis converges (REGISTERED before code and before any grid value; Bee's decision, 26 November)

**Decision.** Following §5.6, Bee chose to **stop the registered run at 93a6de65 and re-register.** The stopped run's
partial receipt is archived **unread**, as in §5.4. No gap, weight or verdict from it is looked at.

**What the gate is for.** The grid reports gaps to a registered tolerance of 0.003 (§3). The domain gate must
guarantee that cutting a trial state to the physical domain cannot move a reported energy by an amount that matters.
The relative rule ℓ ≤ 10⁻³·r answered a different question, the certificate's error budget. It also tightens as r
shrinks with the basis, which §5.6 showed.

**The registered gate.**
- Inputs are the same Arb and interval enclosures as before: for Helike's two partners and the competitor at the final
  rung (Mq = 10), the upper bounds on leakage ℓ, tail t and residual r.
- **Energy shift of the cut:** e_cut = ℓ + 2·t·r, the cut-dependent part of Lemma 1's e = 2tr + ℓ + c(r + ℓ). The
  charge term c belongs to sector projection, which the grid does not use.
- **PASS** if, for all three states: e_cut ≤ 3·10⁻⁴ (one tenth of the gap tolerance), the tail budget t² ≤ 10⁻¹²
  holds, and the normalisation-width budget ≤ 10⁻¹⁰ holds. These budgets are unchanged.
- **FAIL** if any of these fails with the enclosures resolved. **OPEN** if an enclosure does not resolve.
- The old relative ratio ℓ/r stays in the receipt as `relative_ratio_reported`. It is reported, not decided on.

**Where it replaces the old gate.** In `assess_point`, a point is converged when the last-step change is ≤ 0.003
**and** this gate passes. Nothing else in the registered grid rule changes: ladder, identification (§5.2), switch,
tolerance, overall rule. The gluing qualifier (§5.5, §5.6) applies, with S5 now comparing this gate's verdict at the
two radii.

**Checks before the rerun.**
1. The four controls, g² = 0.002 and 0.004 at Mq = 8 and 10: expected PASS. Stated now: at g² = 0.004, Mq = 10,
   e_cut ≈ 5.6·10⁻⁶ from the enclosures already seen in §5.6.
2. The §5.5 gluing check reruns at the controls with this gate.
3. Tests: the gate's arithmetic and boundaries; the relative ratio can never decide a verdict; assess_point uses the new
   gate.
4. Mutants that must be caught:
   - the old relative gate restored as decisive;
   - the tail term dropped from e_cut;
   - the threshold set to the full gap tolerance;
   - OPEN read as PASS.

**Rerun.** The registered grid is then run again on Bee's machine, from a fresh commit, with a new receipt directory.

#### 5.7 control results (check 1)

Commit fa69d91. Report: `research/flux-grid-20261126/gate57-controls/report.json`. s = 1.15, t = 3. Each entry is the
largest e_cut upper bound over the three gated states, against the budget 3·10⁻⁴. The l/r ratio is reported, not decided on.

| g², Mq | verdict | e_cut (Helike partners / competitor) | l/r reported |
|---|---|---|---|
| 0.002, 8 | PASS | 1.8·10⁻²⁵ / 1.1·10⁻²⁵ | 6.4·10⁻²³ |
| 0.002, 10 | PASS | 6.5·10⁻²⁴ / 4.4·10⁻²⁴ | 6.5·10⁻²¹ |
| 0.004, 8 | PASS | 2.7·10⁻⁷ / 1.7·10⁻⁷ | 3.1·10⁻⁵ |
| 0.004, 10 | PASS | 5.58·10⁻⁶ / 3.66·10⁻⁶ | 1.39·10⁻³ (above the old 10⁻³ line) |

The prediction stated in the registration, e_cut ≈ 5.6·10⁻⁶ at g² = 0.004 and Mq = 10, is met: 5.58·10⁻⁶. That point
is the one §5.6 showed failing the old relative rule, and its cut moves Helike's energy by less than 1/50 of the
budget. All four controls pass, as expected. The gluing rerun (check 2) follows.

### 5.8 Two corrections to the §5.7 code before the rerun (26 November, before check 2 and before any grid value)

A review of the §5.7 code before the rerun found two places where the code differed from the registered text. Neither
one can change a convergence decision, because only PASS makes a point converged.

1. **Point verdict, OPEN versus FAIL** (`helike_leakage_gate.overall_cut`). §5.7 says FAIL "if any of these fails with
   the enclosures resolved". The code returned OPEN whenever a state's residual enclosure was unresolved, even when its
   tail or normalisation budget had failed with that enclosure resolved. Such a point now reads FAIL. New test:
   `test_resolved_budget_failure_is_fail_even_when_r_is_open`.
2. **Gluing seam S5** (`run_helike_gluing_check.s5`). §5.7 compares "this gate's verdict at the two radii". The code
   compared only each state's cut-gate label, so a tail or normalisation budget that changed with the radius went
   unseen. S5 now compares, at each radius, each state's (cut gate, state passed) and the point verdict.

After both fixes, the runner and gluing tests pass and the grid mutants stay 15/15 caught. Then check 2 (the §5.5
gluing check at the controls under the §5.7 gate) runs, and the registered grid reruns from this commit into
`research/flux-grid-20261126-gate57/`.

#### 5.8 check 2: the gluing rerun at the controls

This run uses the code at c839dab, with §5.7 and both §5.8 corrections. The scripts are unchanged since that commit.
Report: `research/flux-grid-20261126/gluing-controls-gate57/report.json`.

**All four controls are GLUING-INVARIANT; no seam moved.**
- **S1 and S2 hold everywhere.**
- **S3 holds at Mq = 10.** At Mq = 8 it is not applicable, because the ladder ends at 10.
- **S5 holds at all four.** The points were checked at two radii:
  - g² = 0.002 at ρ = 13.75 and 13.5;
  - g² = 0.004 at ρ = 9.75 and 9.5.

  At both radii, every gated state reads (cut gate PASS, state passed). Both point verdicts are PASS.
- **The §5.6 finding does not recur.** That S5 failure at g² = 0.004, Mq = 10 came from the old relative rule, and
  the cut-energy gate removes it.

An earlier pass of this check ran on fa69d91, before the §5.8 S5 correction, and gave the same verdicts. It was set
aside unrecorded, because §5.8 put check 2 after the corrections.

Checks 1 to 4 of §5.7 are complete. The registered grid reruns from c839dab or later into
`research/flux-grid-20261126-gate57/`.

### 5.9 The stopped c839dab4 run is void; rerun from scratch (26 November)

The run launched from c839dab4 (§5.8) was stopped by Bee before it completed. It had two parts: check 2, the gluing check at the controls, and then the registered grid.

- **Archive.** Its partial output is kept **unread** as `research/flux-grid-20261126-VOID_partial-c839dab4/`. Nobody opened the receipt. It holds:
  - the partial grid receipt;
  - the partial gluing check from that run.
- **Check 2.** It is taken from the committed `research/flux-grid-20261126/gluing-controls-gate57/report.json`.
- **Rerun.** The registered grid reruns from scratch, from the commit that records this section, into `research/flux-grid-20261126-gate57/`. No rule, threshold or prediction changes.

**The 93a6de65 run is void too.** Its partial output is archived **unread** as
`research/flux-grid-20261126-VOID_partial-93a6de65/`, the same way. Nobody opened the receipt. This is the §5.4 run,
stopped for the §5.7 re-registration.


### 5.10 The dressed chart: measure the competitor in the frame the vacuum occupies (REGISTERED 26 November, approved by Bee; before any code or value of the dressed chart)

**Where the grid stands.** The gate57 rerun is UNRESOLVED on all five cases. It was not stopped and no rule was
changed (`research/flux-grid-20261126-gate57/RESULTS.md`). Two numbers fail for the competitor:
- **its gap is not converged at Mq = 10;**
- **its identity is weak:** w/Z = 0.70–0.94, below 0.9 at 26 of 31 points.

Bee's bounded experiments (branch `feat/helike-carrier-geometry-probe`) localise both failures:
- **Mq = 12 does not repair the gap.** At s = 1.18, g² = 0.03, the competitor moves 0.0063 from Mq 10 to 12, against
  a tolerance of 0.003, and its w/Z stays at 0.913.
- **The missing room is coupled.** Only 5% of the competitor's leaked residual lies along its own axis.
- **Residual-driven enrichment works numerically.** Four directions cut the leaked residual by 69%. But the competitor's
  w/Z stays at 0.912.
- **The Ritz ground state crosses zero inside the physical cone,** so the M8 ratio geometry is undefined there.
- **A planted width-deformed excitation** has bare overlap 8/9 while remaining fully aligned in the ground-weighted
  geometry.

**The diagnosis this section acts on.** The basis and the identity test share one ruler: the harmonic frame of the
bare background, meaning the Gaussian vacuum at G0 and the single quanta at the bare frequencies w. As g² grows, the
vacuum shifts and changes width. The two failures follow from that one mismatch:
- A polynomial basis centred on the wrong Gaussian converges slowly. Adding shells, or enriching, patches the wrong
  coordinates.
- An identity test against the bare shape reads a deformed carrier as a mixed one. The planted control shows this
  happens.

The geometric repair is to build the trial space, and the identity test, in the frame the vacuum actually occupies.

**The dressed chart (self-consistent harmonic geometry).** At each point and rung:
1. **Solve in the current chart.** It starts as the bare one.
2. **Measure the vacuum's shape.** From the ground state, compute the mean z̄ and the covariance C of the chart
   coordinates in the ψ₀² measure, by the same exact-moment quadrature, and the kinetic form G(z̄) at the mean.
3. **The new frame.** Whiten by the kinetic form, L = G(z̄)^{1/2}, and diagonalise L⁻¹CL⁻¹ = Q diag(c) Qᵀ. The new
   chart has:
   - centre u₀' = u₀ + S z̄;
   - frame S' = S L Q;
   - widths w'_k = 1/(2c_k).

   In it the vacuum's covariance is the harmonic one, and the kinetic form at the centre is the identity, as in the
   bare normal frame. It is passed to the unchanged operator as (G0', M', w') = (G(u₀'), J_u(u₀') S', w').
4. **Iterate** until the centre and the widths change by less than 10⁻⁶. Stop after 6 iterations, and if it has not
   settled, the point is OPEN.

The Hamiltonian, the measure, the exact moments and the Lanczos settings are unchanged. Only the trial space moves. At
weak coupling the dressed chart tends to the bare one.

**Mode roles in the dressed chart.** The charge rotation is a symmetry of the vacuum, so C commutes with it and the
dressed frame keeps the doublets.
- Roles (Helike's doublet, the competitor, the stiff doublet, the other singlet) are assigned by the largest overlap of
  each dressed mode with the bare modes, never by matching a frequency.
- A dressed doublet must be degenerate to 10⁻⁸, relative. An assignment whose largest overlap is below 0.9, or that is
  not one-to-one, makes the point OPEN.
- The identification that uses Helike's bare frequency ω_H is replaced by this role map in the dressed chart.

**What stays registered.**
- **The ladder:** Mq ∈ {6, 8, 10}.
- **Convergence:** the last-step change is ≤ 0.003, now in the dressed chart.
- **Identification:** w̃ = w/Z ≥ 0.9, now measured against the dressed single quanta, in the same w/Z form. The
  identification quadrature ladder is unchanged.
- **Unchanged:** the switch definition, the predictions s*, the s-tolerance 0.015 and the overall rule.

**The selection fix, folded in.**
- **The states.** The domain-gate export now gates exactly the states the grid identified: the identified Helike
  level, its charge partner found from the charge compression (as in `helike cone-levels`, not by a second weight
  ranking), and the identified competitor.
- **One identity rule.** It requires only the grid's own identification, w̃ ≥ 0.9. The quasimode certificate's 0.95
  selection is not part of the grid's identity rule, and it stays where it belongs, in the certificate path.
- **Refusals are recorded.** A refusal records its exit code, stdout and stderr in the receipt. Bee's wrapper repair
  is kept.
- **The degree cap.** The domain gate still runs at the final rung, Mq = 10, inside the rigorous cutoff code's
  degree cap of 10, in the dressed chart.

**Enrichment stays a diagnostic.** At the final rung of each point, the leaked residual into shells 11 and 12 is
reported for the ground state, the Helike pair and the competitor, with Bee's support probe. It decides nothing. If
the dressed chart is the right geometry, the competitor's leaked residual should fall toward Helike's.

**Order of work.**
1. **Tests and mutants first.** They cover:
   - an exactly solvable shifted and width-deformed Gaussian, where the dressed chart must recover the shift and the
     width;
   - Bee's planted width-deformed excitation, where the dressed w/Z must be 1 while the bare w/Z is 8/9;
   - a genuinely mixed control, which must stay mixed;
   - role assignment under a permuted and rotated frame;
   - the selection fix.

   Mutants that must be caught:
   - the dressed centre dropped;
   - the widths dropped;
   - the kinetic whitening dropped;
   - roles assigned by frequency;
   - the export's 0.95 rule restored.
2. **The four controls** (g² = 0.002, 0.004; Mq = 8, 10). The dressed and bare levels must agree to ≤ 10⁻⁴ at
   Mq = 10, where dressing is O(g²). The dressed w/Z must be ≥ the bare w/Z.
3. **Bee's smoke point**, s = 1.18, t = 3, g² = 0.03, the full ladder in the dressed chart, before any grid run.
4. **Only then** a fresh registered grid run, from a new commit, into a new directory.

**Predictions, stated now** (at the smoke point; each can fail):
- **The gap converges.** The competitor's dressed step from Mq 8 to 10 is ≤ 0.003. The bare step was 0.0091.
- **The identity was deformation.** The competitor's dressed w/Z is ≥ 0.95. The bare value was 0.914.
  - If it rises above 0.9 but stays below 0.95, most of the deficit was deformation.
  - If it stays near 0.91, the competitor is genuinely mixed at this coupling. That is a physics finding about the
    carrier, and M8 would be the instrument for it.
- **The leaked residuals even out.** The competitor's leaked residual is ≤ 1.5× Helike's. Bare, it was 2.1×.

**What this section is not.** It does not change what counts as converged, identified or switched. It changes the
coordinates the trial space and the reference are written in, to the ones the vacuum selects. Whether that was the
missing geometry is what the controls and the smoke point decide.

#### 5.10 A1: dress in the coordinates where the symmetry acts linearly (26 November; during layer-2 coding, before any control value)

**What happened.** A plumbing check of the dressing loop at s = 1.15, t = 3, g² = 0.004 and Mq = 6 was refused by the
registered doublet check: "a dressed doublet must be degenerate to 10⁻⁸". It is not a control, since the controls are
at Mq = 8 and 10.
- **What was seen:** Helike's two dressed widths came out as 1.08624 and 1.08638. The vacuum's variances along her
  two doublet coordinates were 0.4649 and 0.4596.
- **The cause** is in the construction as written, not in the physics. §5.10 took the vacuum's moments in the
  solver's chart coordinates z, which are affine in the log-Cholesky variables. The charge rotation does not act
  linearly on those, because a Cholesky factor is not rotation-equivariant. So an exactly symmetric vacuum looks
  lopsided in them. The check caught a geometric error.

**The correction.**
- **Coordinates.** The dressing step works in the Gram-linear normal coordinates y = M⁻¹(x − x₀), where x is the six
  Gram entries. These are the coordinates in which the bare normal frame and the reference single quanta are defined,
  and in which the rotation acts linearly: each doublet rotates and each singlet is fixed. The mean ȳ and the
  covariance C_y are taken in the ψ₀² measure. The kinetic form is the cometric A(x) written in y at the mean,
  A_y = M⁻¹ A(x̄) M⁻ᵀ.
- **Symmetrization.** The physical vacuum is exactly rotation-invariant, and the truncated trial space breaks that only
  slightly. The moments are averaged over the rotation:
  - the doublet components of the mean are set to zero;
  - each doublet's covariance block becomes (trace/2)·I;
  - doublet–singlet and doublet–doublet covariances are set to zero.

  The size of what is removed is reported as the asymmetry. It must be ≤ 10⁻⁴, or the point is OPEN.
- **The new frame is fed to the solver exactly as the bare one is.**
  - The bare chart is (G0 = the classical minimum, M = the normal frame, w = the harmonic frequencies).
  - The dressed chart is (G0' = the vacuum's mean Gram matrix x̄ = x₀ + Mȳ, M' = M·T, w' = the dressed widths), with
    T and w' from the kinetic-whitened covariance as in §5.10.
  - The solver derives the log-Cholesky chart from (G0', M') in the same way.
  - A mean Gram matrix off the positive cone is refused.
- **Roles** are assigned by subspace overlap, as before, in y with the current frame's kinetic form.
- **Unchanged:** the self-consistency rule (10⁻⁶, at most 6 iterations), every grid rule and threshold, the selection
  fix, and the predictions.

With the correction, the same plumbing point settles in 3 iterations. The centre change goes 2.6·10⁻³ → 2·10⁻⁶ →
2.5·10⁻⁹, the asymmetry is 1.1·10⁻⁵, and the roles equal the bare roles. These values are recorded as plumbing
evidence, and they are not control results.

**Tests added for A1:**
- a planted asymmetric covariance, which symmetrization must restore;
- a mutant with symmetrization dropped, which must be caught;
- the bare chart passed through a chart file, which must reproduce the bare solver bit for bit (it does: zero
  difference in E₀, the gaps and w̃);
- the loop settling at the plumbing point.

#### 5.10 A2: the asymmetry guard tells truncation from geometry along the ladder (26 November; after an outside review, before any control result or smoke-point value is read)

**Disclosure.** An adversarial review, run as a workflow with every finding independently verified, found that A1's
fixed guard of 10⁻⁴ would refuse Bee's registered smoke point at its first rung. It also measured the asymmetry this
section is about:
- at s = 1.18, t = 3, g² = 0.03, the vacuum breaks the rotation by 1.8·10⁻³ at Mq = 6 and by 6.8·10⁻³ at Mq = 4;
- the result is the same at quadrature orders 12, 14 and 16;
- the breaking sits on the doublet component h₁.

The log-Cholesky trial space is not rotation-invariant, and its breaking falls by about 3.7× per two rungs. A1's
10⁻⁴ had been calibrated only at the weak g² = 0.004 plumbing point, where the asymmetry is 1.1·10⁻⁵. These
asymmetries are intermediate quantities of the construction. No control verdict, w̃ or gap at the smoke point was
looked at in making this change.

**What the guard is for.** Symmetrization removes the breaking in any case. The guard exists to catch a geometric
error, such as wrong coordinates or a wrong role map (A1's log-Cholesky error), hiding behind it. The two kinds of
breaking behave differently along the ladder:
- a geometric error is fixed by the coordinates and does not shrink as the basis grows;
- truncation breaking shrinks.

**The rule (replaces A1's fixed 10⁻⁴).**
- **A gross bound at every rung:** the asymmetry is ≤ 10⁻². Above it the point is OPEN.
- **Along the ladder:** the asymmetry of the final dressed iterate strictly decreases from rung to rung. For a point,
  that means Mq 6 → 8 → 10. For the controls, which run Mq 8 and 10, it means Mq 8 → 10. At the final rung it must be
  ≤ 10⁻³. Otherwise the point is OPEN.
- **Reporting:** the asymmetry at every rung and iteration is reported.

A1's log-Cholesky error would still be refused: it persists with Mq, and its size in the y-measure is several 10⁻³.

**Also from the review:**
- **The solver must honour the chart.** The plumbing tests could not tell whether the solver actually uses `--chart`.
  A test now checks that a dressed chart changes the solve: the competitor's w̃ moves by more than 10⁻³ and the energy
  shift moves, while the gaps stay within 10⁻³ at weak coupling. A mutant that never passes the chart must be caught.
- **Only gaps compare across charts.** The stored E₀ and levels are Ritz values without the chart's energy shift V(G0),
  so they cannot be compared across charts. The shift is now recorded, and only gaps are compared.
- **A refuted finding.** A claim that the dressed trial space is not centred on the vacuum was refuted by its verifier.
  It is not acted on.

#### 5.10 control results (step 2; 26 November)

Report: `research/dressed-chart-20261126/controls/report.json`. The point is s = 1.15, t = 3, the dressed chart is
self-consistent at each point, and the bare solve is alongside.

| g², Mq | iterations | asymmetry | gap change, Helike / competitor | Helike w̃, bare → dressed | competitor w̃, bare → dressed |
|---|---|---|---|---|---|
| 0.002, 8 | 2 | 2·10⁻⁷ | 7·10⁻⁸ / 6·10⁻⁷ | 0.997891 → 0.997860 | 0.994195 → 0.997960 |
| 0.002, 10 | 2 | 7·10⁻⁹ | 1·10⁻⁸ / 1·10⁻⁷ | 0.997891 → 0.997860 | 0.994196 → 0.997962 |
| 0.004, 8 | 3 | 1.6·10⁻⁶ | 1·10⁻⁶ / 8·10⁻⁶ | 0.995747 → 0.995686 | 0.988205 → 0.995802 |
| 0.004, 10 | 3 | 1.1·10⁻⁷ | 2·10⁻⁷ / 2·10⁻⁶ | 0.995747 → 0.995687 | 0.988207 → 0.995813 |

**The registered checks.**
- **Levels agree to ≤ 10⁻⁴ at Mq = 10:** PASS at both couplings. The largest change is 1.8·10⁻⁶.
- **A2 asymmetry ladder:** PASS. The asymmetry decreases from Mq 8 to 10 and is far below 10⁻³ at the final rung.
- **Dressed w̃ ≥ bare w̃:**
  - competitor: PASS at all four points, with gains of +0.0038 at g² = 0.002 and +0.0076 at g² = 0.004;
  - Helike: **FAIL at all four points**, by −3.1·10⁻⁵ at g² = 0.002 and −6.1·10⁻⁵ at g² = 0.004.

  As registered, the controls do not pass (`controls_pass: false`).

**Reading.**
- **The trial space moved; the physics did not.** The gaps change by at most 8·10⁻⁶, and by at most 2·10⁻⁶ at
  Mq = 10.
- **The competitor's bare deficit was mostly the ruler.** In the vacuum's own frame its w̃ rises to 0.9958, equal to
  Helike's 0.9957: in the right frame both carriers are equally single-quantum. Its §5.3 dressed overlap does not
  change with the frame (0.99648 against 0.99647), as it should not.
- **Helike's small loss is of order g² and converged in Mq** (identical at Mq 8 and 10). The dressed frame is the
  vacuum's, and her excitation deforms slightly differently from the vacuum. So "dressed ≥ bare" was a prediction
  about her, not a law, and it missed. She stays far above the 0.9 rule.

**What follows.**
- Step 3, the smoke point, runs as registered, as a diagnostic. Its predictions concern the competitor.
- **Step 4, the fresh grid run, is held** until Bee decides how to treat the failed Helike check. The options include
  keeping it as a blocker, or registering that check as a report-only diagnostic, with this miss on record.

#### 5.10 smoke-point results (step 3; 26 November)

Report: `research/dressed-chart-20261126/smoke/report.json`. The point is s = 1.18, t = 3, g² = 0.03, and the dressed
chart is self-consistent at each rung.

| Mq | iterations | asymmetry | Helike gap, bare → dressed | competitor gap, bare → dressed | Helike w̃ | competitor w̃, bare → dressed |
|---|---|---|---|---|---|---|
| 6 | 6 | 1.2·10⁻³ | 1.066537 → 1.064894 | 1.135779 → 1.125594 | 0.9620 → 0.9592 | 0.9157 → 0.9449 |
| 8 | 5 | 7.1·10⁻⁴ | 1.058627 → 1.057299 | 1.105835 → 1.098023 | 0.9654 → 0.9635 | 0.9190 → 0.9569 |
| 10 | 4 | 2.8·10⁻⁴ | 1.057131 → 1.056569 | 1.096694 → 1.092682 | 0.9650 → 0.9637 | 0.9136 → **0.9570** |

The identified levels are the same as in the bare frame: Helike is level 1 and the competitor level 3 at every rung.

**Against the predictions.**
- **The identity was deformation: MET.** The competitor's dressed w̃ at Mq = 10 is **0.9570 ≥ 0.95**; the bare value
  was 0.9136. In the vacuum's own frame, the competitor's apparent mixing at grid coupling is gone.
- **The gap converges: MISSED.** The competitor's dressed step from Mq 8 to 10 is 0.00534, against the prediction of
  ≤ 0.003. The bare step was 0.00914, so the dressed chart improves it by 42%, but not enough. Helike's dressed step
  is 0.00073.
- **The leaked residuals even out: NOT RUN.** The enrichment diagnostic was not implemented for this run.
- **A2 asymmetry ladder: PASS.** 1.2·10⁻³ → 7.1·10⁻⁴ → 2.8·10⁻⁴, strictly decreasing, and ≤ 10⁻³ at Mq = 10.

**Reading.** The dressed chart fixes the ruler, not the rate. It adapts a Gaussian's centre and width to the vacuum.
What remains is shape that a Gaussian cannot carry. The design memo
(`helike-M8-positive-ground-design-DRAFT-2026-11-26.md`) gives the mechanism: the cometric A grows with G, so the
vacuum is skewed, and its log is cubic. The next step therefore asks where that skew lives, and whether a chart that
does not put an exponential wall on the outer side removes it.

**Grid run (step 4):** still held, both for Bee's decision on the Helike check and because convergence is not met.

### 5.11 The unfolded chart (REGISTERED 27 November, approved by Bee)

**Registered after exploratory values were seen, and disclosed.** The prototype's numbers below (control parity, and
the smoke-point step of 0.0004) were seen before registration. They motivate the section, and they decide nothing.
Every check listed under "Checks before any grid value" is rerun by the registered code, the Rust port.

**One physics detail, fixed now, before the port runs.**
- **The potential is the physical one.** It is evaluated as V(q) at q = P. That is the signed potential on both sheets
  of the fold.
- **How the prototype differed.** The prototype evaluated V through a Cholesky factor of G = P², which is always on the
  det q > 0 sheet. The two agree wherever det P > 0, which is everywhere the vacuum carries weight.
- **The parity test against the prototype** uses only nodes with det P > 0. Elsewhere it reports the difference.

**Why.**
- §5.10 fixed the ruler. In the vacuum's own frame the competitor's identity is clean at grid coupling: w̃ = 0.957
  against 0.914 bare.
- It did not fix the rate: the dressed step was 0.0053, against a rule of 0.003.
- The vacuum-shape diagnostic (`helike-vacuum-shape-diagnostic-2026-11-26.md`) locates the cause. The vacuum is
  nearly Gaussian across the tube around the colour ring, in P = √G. It is skewed in the log-Cholesky chart that the
  solver's basis is built in: Mardia skewness 0.06 against 0.59.
- An exploratory √G Rayleigh–Ritz prototype (`research/dressed-chart-20261126/vacuum-shape/sqrtg_ritz.py`) shows:
  - **Parity at the controls.** At g² = 0.004, Mq = 6 reproduces the log chart's converged gaps to 2.5·10⁻⁶.
  - **Convergence at the smoke point.** The competitor's step from Mq 6 to 8 is 0.0004.
  - **Symmetry.** Helike's pair is exactly degenerate without symmetrization.
  - **Quadrature.** The integrands are smooth on the vacuum's sheet: ρ·g = N₆/w with w the orbit volume, nearly
    constant there. So quadrature converges very fast, with orders 10 and 12 agreeing to 10⁻¹⁵, but it is not exact
    (corrected by a symbolic check).

**The proposal.**
- **Chart.** P is a symmetric 3×3 matrix, and G = P². Every symmetric P gives an admissible G. P with flipped
  eigenvalue signs is the other sheet of the fold, so the trial space covers the fold, as the physical operator does
  (report page 12: a fold, not a wall).
- **Frame.** P = μ + Aζ, made self-consistent from the ground state's mean and covariance of P, by the same
  procedure as §5.10. The rotation acts linearly on P, so no symmetrization is needed. The asymmetry is still reported.
- **Basis and operator.** Hermite functions in ζ, of total degree ≤ Mq.
  - The measure is the colour-orbit volume, ∏|λᵢ+λⱼ|. It enters as a weight, so the problem is a generalized
    eigenproblem.
  - The cometric is J A(x) Jᵀ, with J = dP/dx.
  - The potential is the same V.
  - The quadrature is Gauss–Hermite of order n_q = Mq + 6, with a seam at n_q + 2 that must agree to 10⁻⁸ on the gaps.
    The integrands are rational (denominator w), not polynomial, so the seam is decisive.
- **The face.** The quadrature weight on nodes where an eigenvalue of P changes sign is reported, and it must be
  ≤ 10⁻¹². This is where the polynomial structure and the near-vacuum picture stop holding.
- **Ladder.** Mq ∈ {4, 6, 8}. Convergence is the Mq 6 → 8 change ≤ 0.003, the unchanged tolerance.
- **Unchanged:**
  - identification: dressed w̃ ≥ 0.9 against the √G frame's single quanta;
  - the selection fix and the domain gate, recomputed for √G states;
  - the switch rule, the predictions s*, the s-tolerance and the overall rule.

**Checks before any grid value.**
1. **Tests and mutants.** Closed-form cases: a harmonic vacuum written in P, and a rotated frame. Mutants, each of
   which must be caught:
   - the orbit-volume weight dropped;
   - J taken as dx/dP instead of dP/dx;
   - the frame left unconverged;
   - the face weight not checked.
2. **The four controls.** The √G gaps at Mq = 8 must agree with the log chart's converged Mq = 10 gaps to ≤ 10⁻⁴.
3. **The smoke point, fresh,** with the predictions below.
4. **Then the grid,** as a fresh registered run. That needs a Rust port of the operator (`SlabGrid` streaming with an
   pointwise kernel at the seam-checked order), because the Python prototype takes about 80 minutes per Mq = 8
   iteration.

**Predictions** (from the prototype, so they should be re-measured by the registered code):
- the smoke point's competitor step from Mq 6 to 8 is ≤ 0.001;
- every control's gaps agree with the log chart's converged values to ≤ 10⁻⁵ at Mq = 8;
- the competitor's dressed w̃ at the smoke point is ≥ 0.95.

**What this would and would not change.**
- **It would change the coordinates of the trial space.** The tube's cross-section replaces the log-Cholesky chart.
  The Hamiltonian, the measure and every acceptance rule stay as they are.
- **It would not, by itself, release the grid.** The domain gate's rigorous cutoff machinery (degree cap 10) is
  written for the log chart. Porting it is a separate step.
