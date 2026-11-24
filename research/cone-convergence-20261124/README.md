# Mq 6 to 8 control evidence

Source: `59534df9c8dcb6de640c78e8993cb5e08bac5d80`, clean for every recorded source file.
Branch: `feat/helike-quantum-convergence-20261123`.
Result: **CONVERGED** under the unchanged finite-refinement rule. Full grid still held.

The original command was run from that checkout:

```powershell
python -u scripts/run_helike_cone_convergence.py research/cone-convergence-20261124/controls-59534df9
python -u scripts/run_helike_cone_convergence.py research/cone-convergence-20261124/controls-59534df9 --verify
```

The output directory must be empty for a new calculation. Use a different new folder rather than overwriting
these results. Verification does not launch the solver. For replay after the evidence commit, use a separate
clean checkout at `59534df9` and pass the **absolute path** to this retained `controls-59534df9` folder. The
verifier compares the computation revision, source tree and saved snapshots; an unrelated later HEAD will
correctly refuse. Do not edit the receipt's revision or weaken that check.

## Contents

| File | Meaning |
| --- | --- |
| `launch.json` | Command, source revision, host resources and explicit no-full-grid authorization |
| `environment.json` | Python, NumPy, SciPy, SymPy and BLAS versions/thread settings |
| `run.log` | Unbuffered START/DONE messages as all four results landed, plus final decision |
| `resources.jsonl` | Timestamped process/memory samples; samples, not a continuous peak measurement |
| `process-exit.json` | Exit status and completion time |
| `controls-59534df9/receipt.json` | All four measured rows, parameters, differences, decision and source manifest |
| `controls-59534df9/receipt-integrity.json` | Receipt digest |
| `controls-59534df9/sources/` | Source and contract snapshots from the run |
| `controls-59534df9/verification.json` and `verification.log` | Successful receipt-consistency replay |
| `independent-arithmetic.json` | Decimal differences, all weight/error checks and agreement with archived Mq=6 gaps |
| `release-gates.json` | Convergence accepted; certificate and method amendment remain unmet |

## Independent arithmetic

For each coupling and channel, the check reads the serialized gaps as `decimal.Decimal`, subtracts Mq=6
from Mq=8, takes the absolute value, and compares with Decimal("0.003"). All eight identification weights
are compared with Decimal("0.95"); the four recorded reference errors with Decimal("0.000001"). These
calculations do not call the runner's decision function. The binary-float reported differences agree within
1e-14 (an arithmetic cross-check, not a change to the scientific acceptance threshold).

The Mq=6 baseline comparison uses `research/positive-cone-20261124/controls-ad4536e4/receipt.json`.
The maximum discrepancy in those four gap values is 3.5e-15. No archived value was substituted into this run.

No new matrices, eigenvectors or full-operator residuals are saved by this runner. A successful consistency
replay is not matrix replay, proof of authenticity, exact-sector certification or physical-spectrum certification.
The corrected quasimode gates remain mandatory before any full-grid release.
