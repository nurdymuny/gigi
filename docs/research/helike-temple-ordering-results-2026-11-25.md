# Helike below the competitor: Kato–Temple results at the controls

25 November 2026. The registered evaluation of
[the Kato–Temple ordering registration](helike-temple-ordering-registration-2026-11-25.md).
**ORDERED at three of four controls, conditional on (A) and S_C. UNRESOLVED at Mq = 6, g² = 0.004. Every verdict
and margin matches the prediction recorded before computation. Full grid held.**

## Result in plain English

At weak coupling, Helike's excitation sits below the competitor's. That is a statement about the exact eigenvalues
of the physical operator, not about the trial functions. It holds:
- **with no hypothesis on Helike's side.** An upper bound on her level follows from min–max.
- **under one stated counting hypothesis on the competitor's side, S_C.** It says the competitor's charge sector has
  no third level below a frozen threshold, about 0.8 above the competitor. That is far below where the trial
  spectrum puts its next level.

The proof of (A), discrete spectrum, is still only sketched. Everything here is "conditional on (A) and S_C".

At the fourth control (Mq = 6, g² = 0.004), the smaller basis leaves a residual of 0.08 against a separation of
0.008. The rule therefore says UNRESOLVED. That outcome was predicted, so it is not a defect.

## Results

| g² | Mq | Helike upper U_H | competitor lower L_C | margin ≥ | verdict | predicted |
| --- | --- | --- | --- | --- | --- | --- |
| 0.004 | 6 | −41.032384 | −41.033439 | −0.001055 | UNRESOLVED | −0.0011, UNRESOLVED |
| 0.004 | 8 | −41.033021 | −41.025842 | +0.007179 | ORDERED | +0.0072, ORDERED |
| 0.002 | 6 | −88.556320 | −88.546277 | +0.010042 | ORDERED | +0.0100, ORDERED |
| 0.002 | 8 | −88.556472 | −88.544410 | +0.012061 | ORDERED | +0.0121, ORDERED |

Absolute energies include the background shift; only differences matter.

**The new input, the linear moment s = ⟨ψ, (H − E)ψ⟩.** For every state its enclosure is within ±1.6 × 10⁻¹¹,
consistent with zero, as expected for Ritz vectors. The independent polarization route, which uses two residual
enclosures with centres shifted by an exact 1/8, agrees, with enclosures of about ±10⁻⁸. The predictions set s = 0,
which is why they match to the digits shown.

**Lemma 1's μ.** The Rayleigh-quotient offset is dominated by the charge term c(r + ℓ):
- ±4 × 10⁻⁴ for Helike at Mq = 6, g² = 0.004;
- ±1 × 10⁻⁵ at Mq = 8, g² = 0.004;
- ±1 × 10⁻⁶ at Mq = 8, g² = 0.002.

**Lemma 3** holds at every control: the ground and competitor windows are disjoint and below ρ_C.

**Gap values** (reported, not decisive; conditional on (A), S_0, S_H and S_C). At g² = 0.004, Mq = 8:
- Helike's exact excitation gap lies in [1.08786, 1.08804];
- the competitor's is at least 1.09516.

At g² = 0.002, Mq = 8: Helike's lies in [1.08903, 1.08905], and the competitor's is at least 1.10110. The Ritz
gaps lie inside these.

## Order of events and evidence

1. Registration with predictions: 571621f.
2. Implementation, before any evaluation: feat/helike-lab ec0f079.
   - RED: the missing-module log.
   - GREEN: 7 tests.
   - All 9 registered faults caught by assertion, plus the linear moment's own two.
3. Evaluation with an explicit `--assume S_C`: receipt feat/helike-lab 59b123f, copied here as
   [cone_temple_c31aab8e.json](../../research/temple-20261125/cone_temple_c31aab8e.json). It replays with no
   differences.

## What this does and does not establish

**It establishes**, conditional on (A) and S_C: at three weak-coupling controls, the exact charge-±2 ground level
lies strictly below the exact second charge-0 level. In words, Helike's excitation is below the competitor's.

**It does not establish:**
- S_C, or (A) as a written proof;
- anything at the registered grid couplings, or the location of the flux-edge switch;
- the registered gap accuracy at the grid;
- any continuum statement.

The method amendment and the grid remain the next gates. `certified: false` for the grid, `full_grid_released: false`.
