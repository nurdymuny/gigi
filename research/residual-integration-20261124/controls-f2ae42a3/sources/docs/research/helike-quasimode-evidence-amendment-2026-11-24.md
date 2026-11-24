# Quasimode evidence repair: frozen focused gates

24 November 2026. SPEC BEFORE REPAIR. No physical model, operator, coupling, grid or numerical threshold changes.
The current scope is evidence validation and the quasimode/orientation contracts, not a new spectrum run.

## Fixed contract

The convergence runner assesses exactly s=1.15, t=3, g^2={0.004,0.002}, Mq={6,8}.
Both gaps must change by at most 0.003 at both couplings, and all four pairs of weights must be at least 0.95.
All recorded diagnostics must be finite and properly typed; bool is not a measurement. Reference-weight error
must remain at most 1e-6. Missing, extra, malformed or failed controls are UNRESOLVED, never CONVERGED.
Complete valid evidence above the gap tolerance is NOT_CONVERGED, not a software error.

Verification means **receipt consistency only, not matrix replay, authenticity or physical certification**.
It checks exact parameters and case coverage, current-source/saved-source hashes, revision/dirty provenance,
integrity, all recorded decisions and changes. Checksums detect changes, not a malicious author who fabricates
all numbers consistently. Pin the recorded source checkout for replay; do not weaken source checks to reuse
old receipts. No validation is allowed to call the solver.

Write RUNNING before source collection or the first solver call; checkpoint each completed rung atomically.
On an exception or interruption, save FAILED/UNRESOLVED with prior rows retained. Serialization forbids NaN.
At verification start replace any old PASS by RUNNING, then PASS/FAIL bound to the receipt hash. A crash after
invalidation cannot leave PASS. If that first invalidation write fails, raise and return nonzero; an old file is
not authority. Consumers must revalidate, never trust verification.json on its own. No concurrent writers to
the same output directory are supported. Atomic replacement is not a claim of transactional crash durability.

The release helper must validate convergence evidence, require CONVERGED and clean recorded sources, and
never accept a certificate status string. Certificate validation is not implemented: its condition remains unmet.
The LocalFock method-amendment hold remains unconditional. All bigger runs stay held.

## RED, GREEN, then mechanism removal

1. Demonstrate assertion failures against the old checker: empty/missing/extra controls, nonfinite and invalid
   measurements, resealed forged changes/decisions/thresholds, partial receipts, and stale PASS.
2. Accept a complete planted CONVERGED fixture and, separately, a planted NOT_CONVERGED fixture; both verify
   as consistent while only the first can meet the convergence condition. These fixtures are software controls,
   not new Mq=8 measurements.
3. Reject malformed JSON, duplicate keys, bool-as-number, missing/changed sources, forged revision/dirty flag,
   missing integrity and mismatched scope. Test threshold boundaries with adjacent floating-point values.
4. Prove initial/partial logging on first-call and later failures, including KeyboardInterrupt and NaN output.
5. Test release with forged statuses and verify no solver is invoked.
6. Remove coverage, finiteness, threshold, provenance, hash, decision-recomputation, stale-PASS invalidation,
   and release-validation mechanisms one at a time. Each targeted test must fail an assertion, not crash.
7. Small independent mathematical controls: complex absolute squares, N/2N Fourier aliasing, exact charged
   covariants, Temple's positive denominator and separation, reflection-even truncation error, and the block
   elimination identity. These demonstrate why the spec changes are necessary; they do not implement the
   physical quasimode integrator.
8. Run only focused research tests, retain commands/logs/results, commit and push before any larger run.

The quasimode spec keeps interval certification distinct from numerical agreement, adds charge control rather
than replacing the existing 0.9 identification gate, and leaves unregistered integration choices visibly held.
