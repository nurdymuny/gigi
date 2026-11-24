# Mq 6 to 8 controls: the registered refinement check passes

24 November 2026. COMPUTED: **CONVERGED under the fixed finite-refinement rule.** Not a physical-spectrum
certificate, an infinite-basis convergence proof, or permission to release the full grid.

## What ran

The unchanged runner at clean source revision `59534df9c8dcb6de640c78e8993cb5e08bac5d80` evaluated
the two registered controls, s=1.15, t=3, g^2=0.004 and 0.002. Each was calculated independently at
Mq=6 (924 trial functions) and Mq=8 (3,003 trial functions). No model, domain, grid or threshold changed.

The job ran from 21:44:14 to 21:45:51 UTC (13:44 to 13:45 Pacific), exited zero, and checkpointed each
completed rung before continuing. Per-rung times were 6.390, 42.437, 6.157 and 42.156 seconds.
The CPU/library/memory environment, resource samples and live output are retained with the receipt.

## Measured refinement

The allowed absolute gap change is **at most 0.003**, at both couplings and for both channels.

| g^2 | Channel | Mq=6 gap | Mq=8 gap | Absolute change | Decision |
| --- | --- | --- | --- | --- | --- |
| 0.004 | Helike | 1.0881506933025147 | 1.0879701666872634 | 0.0001805266152513 | PASS |
| 0.004 | Competitor | 1.0962441489593235 | 1.0955779453125754 | 0.0006662036467481 | PASS |
| 0.002 | Helike | 1.0890870744498322 | 1.0890424648723310 | 0.0000446095775012 | PASS |
| 0.002 | Competitor | 1.1013023579665457 | 1.1011455727505188 | 0.0001567852160269 | PASS |

The table differences are decimal subtractions of the serialized gap values. The runner's binary-float
differences agree to rounding; both calculations independently pass the same 0.003 rule.

The identification requirement is **weight at least 0.95 on every rung**, using the original linear-Gram
harmonic projectors, not the log-coordinate basis weights.

| g^2 | Mq | Helike weight | Competitor weight | Reference-weight error |
| --- | --- | --- | --- | --- |
| 0.004 | 6 | 0.9885333264400942 | 0.9808081919195198 | 2.7980580152853207e-7 |
| 0.004 | 8 | 0.9886875768437073 | 0.9811986591438617 | 8.3631986103061e-7 |
| 0.002 | 6 | 0.9943433989747622 | 0.9905908650892777 | 3.16359253238474e-8 |
| 0.002 | 8 | 0.9943831154865386 | 0.9907002546154084 | 8.940704054882787e-8 |

All eight weights pass. The maximum reference-weight error, 8.3632e-7, is below the unchanged 1e-6 limit,
although it is closer to that limit than the other controls. No integration order or tolerance was adjusted.
The four newly computed Mq=6 gaps reproduce the earlier archived control gaps within 3.5e-15.

## What this establishes

The basis can grow from 924 to 3,003 trial functions without moving either identified gap beyond the
registered tolerance at either control. This closes the **Mq=6 to 8 control-refinement check**, which had
been pending. It does not erase the older Mq=4 to 6 competitor change of 0.0034559546, which failed 0.003.

Receipt replay passes completeness, finite values, the frozen parameters, source/snapshot hashes, provenance,
and recomputed decisions. A separate decimal-arithmetic check agrees with all four gap comparisons, all
eight weight checks and all four reference-error checks. These checks are of recorded scalars, not an
independent replay of the new 3,003-dimensional matrices. The current runner does not save those matrices or
eigenvectors; do not cite this receipt as a full-operator residual or sector certificate.

## The full grid is still held

Evaluating `unmet_release_conditions` at the computation revision returned exactly:

```json
["certificate", "method_amendment"]
```

The convergence condition is now met for this clean-source run. The physical quasimode certificate is not
implemented or measured, and the retired LocalFock grid cannot be released by this receipt. The method
amendment remains required, including the still-open charge, cutoff, integration and separation obligations
in the corrected spec. The full grid was **not started**. No full engine suite or deployment was run.

## Evidence

- [Run folder and reproduction instructions](../../research/cone-convergence-20261124/README.md)
- [Full measured receipt](../../research/cone-convergence-20261124/controls-59534df9/receipt.json)
- [Live output](../../research/cone-convergence-20261124/run.log)
- [Independent arithmetic](../../research/cone-convergence-20261124/independent-arithmetic.json)
- [Remaining release gates](../../research/cone-convergence-20261124/release-gates.json)
- [Corrected quasimode spec](helike-quasimode-certification-spec-2026-11-24.md)

Verification is intentionally pinned to the computation checkout. The evidence commit comes later and does
not change the computation's revision. See the run README before replaying from another checkout.
