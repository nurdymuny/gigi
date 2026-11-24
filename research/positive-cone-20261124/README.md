# Positive-cone basis controls

Baseline `9c17cc33`. Spec written before implementation. Preserve the
negative-kinetic RED witness, focused control logs and mechanism removals
here. No larger registered run is authorized by these receipts.

## Before the Mq=6 refinement

The first two direct domain-witness tests failed on the legacy linear map,
then passed on the log-Cholesky chart. All six geometry and four moment/
reference-transform tests pass. Mq=4 does NOT pass the unchanged control
gate: the extrapolated competitor coefficient error is 0.041367 > 0.03.

The initial logarithmic-coordinate weights were not the original reference
observable. The corrected fixed linear-Gram projectors give competitor
weights 0.9812470 and 0.9907386, which pass 0.95; basis-coordinate weights
are retained separately. The 8/10 overlap rule refused at 1.36e-6; 10/12
resolves the same observable to 1.70e-7 and 2.37e-8 without changing the
1e-6 requirement. All earlier failures remain recorded.

This is an experimental source checkpoint, not a closed physical gate.
The Mq=6 refinement of the SAME two weak-coupling controls is specified
before its first run. Commit and push this source first. The full physical
grid stays held; the singular-boundary realization is unresolved.

Earlier development receipts retain matrices and source hashes, but not
complete source snapshots. Subsequent clean-commit runs retain both.
