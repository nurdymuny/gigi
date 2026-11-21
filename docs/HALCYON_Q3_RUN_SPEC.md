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

## Amendment 2 (adopted by Bee 2026-11-19, after a peek at k0 and k2, before k1, k3–k6 are read)

What prompted it (recorded in the run log): at k0 and k2 the 0⁺⁺ signal stays above 3σ only to
t = 3. m_eff(t) uses C(t+1), so under Q1-1 only t = 2 is admissible and the rule refuses, although
m_eff(2) and m_eff(3) agree with each other and with LTW. Separately, at k0 the flux-loop GEVP chose a
noise-dominated vector (mostly smearing level 0, whose t = 1 signal is ~10σ against 50–70σ for the
other levels); the condition number (~4.5e3) was far below the refusal threshold.

- **Q1-4, two-slice plateau.** If the three-slice rule (Q1-1, Q1-2, MIN_LEN 3) finds no window, a
  window of two consecutive slices (t, t+1) is accepted when t passes Q1-1 (C(t−1), C(t), C(t+1) each
  ≥ 3σ), m_eff(t+1) is defined in every jackknife sample with a finite positive error (it need not pass
  the correlator floor; its jackknife error carries that noise), the two agree within 2σ (Q1-2), and the
  constant fit has χ²/dof ≤ 2. For two points χ²/dof ≤ 2 binds first: the two must agree within √2
  combined σ. The earliest such window is taken. The error is the jackknife of the fit (Q1-3). This
  becomes the verb's rule; the as-fixed rule is still returned beside it. The null protection is the
  floor on the first slice, unchanged.
- **Q3-B, operator selection before the GEVP.** For each operator, the t = 1 signal-to-noise
  C_aa(1)/σ_aa(1) (blocked jackknife). Operators below half of the best are dropped before the solve;
  the kept indices are reported and the vector is reported over all operators with zeros for the
  dropped ones. After projection, if the projected C(1) is less than 3σ from zero, the GEVP refuses
  ("noise-level mode at t = 1"). Applies to the verb (EFFECTIVE_MASS ... GEVP) and the probe alike.
- **Where it is judged.** Only on k1, k3, k4, k5, k6, none of which has been read. k0 and k2 are
  scored by the harness like the others but the amendment's success is not claimed from them. Q3-n
  (nulls refuse) and Q3-r (removal) stand unchanged and test the amendment against its failure mode.

## Amendment 3 and the extension run (directed by Bee 2026-11-20, after the Q3 results; written before any extension data)

Bee chose all three follow-ups in `docs/HALCYON_Q3_RESULTS.md`, the GEVP fix first. The details below are
fixed before any extension configuration is generated and before the fix is coded.

**Q3-C, truncate the directions C(t₁) cannot see.** After Q3-B selection, each kept operator is scaled so
C_aa(t₀) = 1. C(t₀) is diagonalised, C(t₀) = U Λ Uᵀ. For each eigendirection u_j the projected series
Σ_a (u_j)_a O_a has a correlator at t₁ with a blocked-jackknife error; the direction is kept only if that
C(t₁) is at least `MIN_SIGNAL_SIGMA` (3) errors above zero. The GEVP is solved in the kept subspace,
Ĉ(t) = U_keptᵀ C(t) U_kept, and the vector is mapped back to the operators (zeros for operators Q3-B
dropped). If no direction is kept, the GEVP refuses ("no direction of C(t0) has a signal at t1"). The
Q3-B noise-mode refusal after projection stays. No new threshold: the 3σ is the one Q1-1 and Q3-B
already use. The kept directions are reported.

Why this criterion: in a near-null direction of C(t₀) the true C(t₁) is smaller still, so what the
GEVP sees there at t₁ is noise divided by a small number, which can win the largest eigenvalue. That is
the failure diagnosed at k1 (after scoring). The criterion drops exactly the directions whose t₁ signal is
not distinguishable from noise.

**Tests before data (unit, synthetic).** A two-state matrix correlator with a third operator nearly
collinear with the first (independent noise added): (i) without Q3-C the GEVP's mode is wrong or refused;
(ii) with Q3-C it recovers the ground mass within 3σ; (iii) on clean, well-separated operators Q3-C keeps
every direction and the vector equals the untruncated one. (i) is the removal arm: with the
truncation off the failure must return.

**Extension points (seed = 20261193 + 1000·k, as before; parallel sampler, THREADS ≥ 1, which is a
different ensemble class from k0–k6 and bit-identical for any thread count):**

| k | β | L | role |
|---|---|---|---|
| 7 | 2.2986 | 12 | second volume at β = 2.2986, replacing k1 in Q3-v (k1 stays reported as failed) |
| 8 | 2.2986 | 14 | fresh replicate of k1: the Q3-C test point |
| 9 | 2.5115 | 20 | LTW Table 5 calibration |
| 10 | 2.5115 | 24 | second volume at β = 2.5115 |

Same statistics as k0–k6: 1,000 thermalisation updates, 10,000 measurements, one heat bath + four
overrelaxation per update, levels 0, 4, 8, 16, α = 0.5, blocks of 100, GEVP at (1, 2).

β = 2.5115 overrides the rule in "The run" (first three β must pass Q3-a and Q3-v, which β = 2.2986
did not). The override is Bee's decision, recorded here before any of its data.

**Gates for Q3-C, judged only on k7–k10:**
- **C-harm.** At every extension point and channel where the Amendment-2 GEVP (no Q3-C) returns a
  plateau, the Q3-C value differs from it by at most 1 σ of the Amendment-2 value.
- **C-job.** At k8 the Q3-C 0⁺⁺ plateau exists and agrees with LTW's am₀₊₊ = 1.212(22) (β = 2.2986,
  aL√σ > 5, so volume effects are below the errors) within 2 combined σ. Whether the Amendment-2 GEVP
  refused at k8 is reported; if it did not, C-job tests correctness only, and that is said.
- **C-null.** Every channel refuses on its shuffled series at k7–k10.
- The unit tests above.

**Q3 gates on the extension.** Q3-a at β = 2.5115 (k9) against Table 5; Q3-v at β = 2.2986 from k0 and
k7, at β = 2.5115 from k9 and k10; Q3-s refitted over every β that passes Q3-a and Q3-v (up to four
points, so the fit has degrees of freedom and the falsifier can trigger). All with the Q3-C verb rule.
k0–k6 are rescored with Q3-C and reported; k1 is not evidence for Q3-C (it prompted it). Q4 is rerun on
the new receipt, exploratory as before.

## Amendment 4 (DRAFT, requested by Bee 2026-11-20 after the extension results; not adopted; before any of its code or data)

**Status.** Draft for Bee. Nothing below is coded. The decisions marked **[Bee]** are open. Once adopted, the
text is fixed and committed before any code for it, and before any of its fresh configurations are generated.

**What prompted it.** Recorded in `docs/HALCYON_Q3_EXTENSION_RESULTS.md`, after scoring:

- Amendment 3 failed C-harm. The k10 flux loop moved 2.0σ, on the same window.
- Rescoring k0–k6 under Q3-C lost β = 2.3715 (k3 flux loop refused) and β = 2.4265 (k4 0⁺⁺ 2.95σ high). The
  effective-mass curves barely moved there; the plateau windows did.
- The larger volume's a√σ is high wherever it can be read: k1, k8 and k10 at 6.6–7.4σ. Those flux-loop windows start
  at t = 1 (k1, k8) or t = 2 with three slices of signal (k10), before excited states have decayed. Amendment 2 shows
  the same excess at k10, so this predates Q3-C.
- A4-2 below addresses the windows that start at t = 1. It does not touch k10, whose window already starts at t = 2.
  If the fresh replicate of k10 (k18) shows the same excess, that is reported as unaddressed by this amendment.

Because these rules are chosen after seeing that data, k0–k10 are rescored under Amendment 4 and reported, but they
are **not evidence for it**. It is judged only on fresh seeds (below).

**The changes, two only.** One change per failure, so a result can be traced to its cause.

- **A4-1, Q3-C becomes a fallback.** The GEVP first runs as in Amendment 2 (Q3-B selection, untruncated solve). Q3-C
  is used only if that path fails: the solve refuses, or its plateau refuses. The row reports which path produced the
  value (`gevp_path: "a2"` or `"q3c_fallback"`).
  - *Why.* Q3-C was written for one failure (a near-null C(t₀) direction, k1 and k8), and it fixed that failure. It
    also moved values where nothing had failed. As a fallback it cannot move a value the untruncated path already
    returns.
  - *Cost, stated.* Two chances at a plateau instead of one is a mild look-elsewhere effect. The protection is the
    null: every shuffled series must refuse under both paths (A4-null).
- **A4-2, no plateau slice before t₁.** For a GEVP-projected correlator, every slice in a plateau window satisfies
  t ≥ t₁ = 2. A window that would start at t = 1 is not taken; the rule looks for the earliest window from t = 2 on,
  and refuses if there is none.
  - *Why.* The GEVP eigenvalue is solved at (t₀, t₁) = (1, 2). Before t₁ the projected correlator has had the least
    time for excited states to decay, and the flux loops at aL√σ > 5 (heavy, am ≈ 2) are exactly where windows
    started at t = 1. Choosing t₁ ties the bound to the solve already fixed, so no new number is introduced.
  - *Expected consequence, stated now.* At the larger volumes the flux loop may then refuse rather than return a
    biased σ. A refusal is a correct answer there. Q3-v at a β then fails by refusal, and that is reported as such.
  - It applies only to GEVP-projected correlators. EFFECTIVE_MASS on a single series is unchanged.

**Not changed.**

- The window choice among admissible windows: earliest; three slices before two (Q1-4); Q1-1 to Q1-3.
- MIN_SIGNAL_SIGMA, Q3-B, the null rule.

Changing the window choice as well would make a pass impossible to attribute. **[Bee]** Confirm that it stays out.

**Tests before data** (unit, synthetic; each with a removal arm):

1. **Fallback is invisible when not needed.** On clean two-state operators the A4 result is bit-identical to the
   Amendment-2 result, and `gevp_path = "a2"`.
2. **Fallback engages when needed.** On the near-collinear fixture of Amendment 3 (the same 8 noise seeds), the
   untruncated path fails 8/8 and A4 recovers the ground mass within 3σ through `q3c_fallback` 8/8. Removal arm:
   with the fallback disabled the failure returns.
3. **The start bound does its job.** Use a two-state correlator with a heavy excited state, strong enough that a
   window starting at t = 1 sits more than 3σ above the ground mass.
   - With A4-2, the plateau agrees with the ground mass within 2σ, or refuses.
   - Removal arm: without A4-2, the earliest window starts at t = 1 and is more than 3σ high. If this arm does not
     fail, the fixture does not test A4-2 and is replaced before any data.
4. **Nulls.** A shuffled series refuses under both paths.

These apply to the verb (EFFECTIVE_MASS … GEVP) and the probe alike, and I-parity is rerun on the stored bundles
after the change.

**Fresh points.** Seeds are 20261193 + 1000·k, as before. Same statistics, sampler and frozen-binary procedure as the
extension.

| k | β | L | role |
| --- | --- | --- | --- |
| 11 | 2.2986 | 10 | Q3-a |
| 12 | 2.2986 | 12 | Q3-v |
| 13 | 2.3715 | 12 | Q3-a |
| 14 | 2.3715 | 16 | Q3-v |
| 15 | 2.4265 | 16 | Q3-a |
| 16 | 2.4265 | 20 | Q3-v |
| 17 | 2.5115 | 20 | Q3-a; also a direct replicate of k9, whose 0⁺⁺ was 3.2σ high under both GEVPs |
| 18 | 2.5115 | 24 | Q3-v |
| 19 | 2.2986 | 14 | the A4-2 test point (added by Bee 2026-11-21) |

k19 is the volume where the flux-loop excess was largest (k1, k8). It does not enter Q3-v or Q3-s. It is scored
against LTW's a√σ = 0.3667(18) and am₀₊₊ = 1.212(22) at β = 2.2986 (aL√σ > 5, so volume effects are below the
errors), and it is reported under A4-start: whether A4-2 moved the flux-loop window, and whether a√σ now agrees with
LTW within 2σ or the flux loop refuses. Neither outcome is a pass or a fail of the amendment.

**Gates, judged only on k11–k19.**

| gate | what must hold | if it fails |
| --- | --- | --- |
| **A4-path** | every value reports its path. A `q3c_fallback` value is listed by point and channel | report |
| **A4-null** | every channel's shuffled series refuses, under both paths | stop |
| **A4-start** | report every channel where A4-2 changed the window or produced a refusal, with the value the rule without A4-2 would have given | report. This is the mechanism record, not a pass/fail |
| **Q3-a** | at each of the four β, against LTW Table 5, as fixed | name the β |
| **Q3-v** | at each of the four β, as fixed. A refusal is a failure, not a pass | name the β and say whether it was a refusal |
| **Q3-s** | over every β passing both, as fixed, falsifier included | — |
| **k17 replicate** | report whether k17's am₀₊₊ reproduces k9's excess (> 2σ above LTW) | if it does, the excess at (2.5115, 20) is a property of the point, not a fluctuation. It is reported, and no rule is written from it inside this amendment |

**Success and failure, fixed now.**

- **Success:** Q3-a and Q3-v pass at three or more β, so Q3-s has a fit with at least one degree of freedom, and
  A4-null holds.
- **Partial:** two β. Q3-s is a two-point line, reported without a falsifier.
- **Failure:** fewer than two β. Then the estimator is not ready for step 3. Before any further amendment, the next
  move is a longer-statistics or anisotropic run, under its own spec, not a further plateau rule.

Q3-r (k6) is not rerun, because there is no fresh β = 2.3 point; it is rescored and reported. Q4 (C1, C2, K) is
rerun on the A4 receipt, exploratory.

## Order of work

1. Sampler and its Q3-0 tests.
2. Smearing, operators, GEVP, with tests (gauge invariance; GEVP on a synthetic two-state correlator
   recovers both masses).
3. Timing run, then statistics fixed in the run log.
4. The production points, in the order above.

## Not in this run

- SU(3). Q4. Any claim beyond "the lattice ratio at these spacings, against LTW's continuum value".
