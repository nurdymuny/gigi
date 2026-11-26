# Flux grid, gate57 rerun: results

Registered run of the 31-point flux grid at commit `9b36f670` (clean tree), executed from scratch under the method
amendment through §5.9. Receipt: `receipt.json` (sha256 in `receipt-integrity.json`), `status: COMPLETE`,
`certified: false`, **overall: UNRESOLVED**. No rule was adjusted after the run.

## Verdict per case

| block | t | g² | status | measured switch | predicted s* |
|---|---|---|---|---|---|
| primary | 3 | 0.03 | UNRESOLVED | none | 1.1674 |
| primary | 3 | 0.05 | UNRESOLVED | none | 1.1809 |
| primary | 3 | 0.07 | UNRESOLVED | none | 1.1931 |
| secondary | 4 | 0.03 | UNRESOLVED | none | 1.1260 |
| secondary | 4 | 0.05 | UNRESOLVED | none | 1.1413 |

## Why: the competitor gap has not converged at Mq 10

Convergence requires the Mq 8 → Mq 10 step to be ≤ 0.003. It is not met at any of the 31 points. The level that
moves is the competitor. Helike moves much less.

| t | g² | Helike step, max | competitor step, range |
|---|---|---|---|
| 3 | 0.03 | 0.0024 | 0.0058 – 0.0147 |
| 3 | 0.05 | 0.0073 | 0.0191 – 0.0328 |
| 3 | 0.07 | 0.0116 | 0.0320 – 0.0467 |
| 4 | 0.03 | 0.0043 | 0.0131 – 0.0232 |
| 4 | 0.05 | 0.0096 | 0.0297 – 0.0363 |

The steps grow with g². At t = 3, g² = 0.03, Helike passes the 0.003 rule at every point. There the competitor
alone keeps the case open.

## Identification (w̃ ≥ 0.9, §5.2)

Helike's dressed weight is 0.905 – 0.966 everywhere. The competitor's is lower, and it falls as g² grows. Its
maximum per case is 0.940, 0.892, 0.852, 0.903 and 0.848, in table order. Identification holds at 5 of the 31
points: s ≥ 1.18 at t = 3, g² = 0.03, and s = 1.18 at t = 4, g² = 0.03.

## Domain gate (§5.7)

The gate is OPEN at all 31 points, and it was never evaluated. The domain-gate export applies the registered
quasimode selection rule (amendment line 174), which needs w̃ ≥ 0.95 for both Helike partners and for the
competitor. The competitor's w̃ never reaches 0.95 on this grid, so the export refuses every point. I reproduced
this at s = 1.12, t = 3, g² = 0.03, Mq 10 with the same binary (sha256 `210293c9…`). It exits with code 2 and the
message `error: ambiguous state selection` (`helike-quantum/src/export.rs:157`).

This is registered behaviour, not a crash. But the receipt records it as a raw `CalledProcessError` and does not
keep the export's message. A later runner should capture stderr so the refusal reason is in the receipt.

No e_cut, tail², normalisation width or ℓ/r values exist for this run.

## Dressed-overlap cross-check

Present at every rung of every point. The overlap error is ≤ 1.3·10⁻¹³ and orthonormality ≤ 1.3·10⁻¹³. The
reference-weight error falls with Mq, from 8.7·10⁻⁷ to 2.6·10⁻⁷ at the first point.

## Observation only: where the Mq 10 difference changes sign

These numbers do not count as a measured switch. They come from unconverged levels, and the rules give them no
standing. Linear interpolation of (competitor − Helike) between neighbouring s points at Mq 10:

| t | g² | sign change at | predicted | difference |
|---|---|---|---|---|
| 3 | 0.03 | 1.1659 | 1.1674 | −0.0015 |
| 3 | 0.05 | 1.1677 | 1.1809 | −0.0132 |
| 3 | 0.07 | 1.1558 | 1.1931 | −0.0373 |
| 4 | 0.03 | 1.1187 | 1.1260 | −0.0073 |
| 4 | 0.05 | 1.1060 | 1.1413 | −0.0353 |

The competitor gap is still falling as Mq grows. A converged crossing would therefore sit at smaller s still.

## What would unblock it

- A larger ladder rung for the competitor sector: Mq 12, and possibly Mq 14 at g² ≥ 0.05.
- Or a registered convergence rule that accounts for the dressed competitor.

Either one is a new amendment. It cannot be applied to this receipt.
