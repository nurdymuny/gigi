# Flux-grid repair: one bounded basis smoke test

Status: registered before the new numerical run. The full grid stays held.

## The question I am testing

The completed gate57 grid is unresolved for two different reasons. The competitor gap still changes by more
than 0.003 between Mq 8 and 10, and its harmonic-reference weight divided by vacuum weight does not meet the
0.95 export-selection threshold. More basis functions might fix the first problem without fixing the second.
I will measure them separately, not relabel the higher dressed-overlap cross-check as the registered weight.

## Frozen experiment

- Source baseline: `d3e03ad13d99755f9559900e2ec6d6e48ff795c1`; the numerical source is unchanged.
- One point: s = 1.18, t = 3, g^2 = 0.03. This is the first registered point above the predicted crossing at
  1.1674 that passes the grid's 0.9 identification rule. It avoids the almost degenerate point at s = 1.165.
  This is a deliberately selected diagnostic, not a representative statistical sample of the entire grid.
- Rerun Mq 10 before Mq 12. Stop if Mq 10 does not reproduce the saved gaps within 1e-8, or the saved
  reference weights within 1e-6. The historical receipt must match its registered LF-normalized SHA256
  `f1c880bb1bb7f025dc8c29a60fbc346746cdbe279ba655c5b2d2a9e96df9081d`.
- Same Hamiltonian, positive-cone chart, measure, source, exact-moment operator and harmonic reference.
  Same default Lanczos settings: 12 levels, block 4, tolerance 1e-10, maximum dimension 1600.
  Same independent grid cross-check and reference-overlap tolerance 1e-6. No extrapolation.
- Change only total occupation cutoff: 10 -> 12, increasing trial dimension from 8,008 to 18,564.
- At most 8 Rayon threads and 900 seconds per solve. Capture stdout, stderr, commands, exit status,
  binary digest, source revision, elapsed time and raw receipts. No automatic Mq 14, no full grid.

The finite-space residual check is the existing solver's requested tolerance:
each returned residual must be at most 1e-10 * max(1, absolute Ritz value).
It is not the full physical-operator residual. A failure is reported, never hidden by a stable gap.

## Distinct outcomes

1. **Reproduction**: the fresh Mq 10 control agrees with the saved result.
2. **Basis-step check**: both absolute gap changes from 10 to 12 are at most 0.003. Also report the ratio
   of the competitor's 10 -> 12 step to its historical 8 -> 10 step. A smaller step alone is not a pass.
3. **Grid identification**: both final reference weights w/Z are at least 0.9.
4. **Export obstruction**: report whether the competitor alone is below 0.95. If it is, the unchanged
   exporter necessarily refuses; if it is not, this does NOT prove the two Helike partners pass selection.
5. **Physical domain**: NOT RUN. The current rigorous cutoff implementation explicitly caps polynomial
   degree at 10. This smoke test does not amend that resource/numerical contract or test a boundary certificate.

Even if 1-3 pass, the overall grid remains UNRESOLVED, uncertified and unreleased. A single basis step is not
a convergence proof. It cannot locate a certified crossing or determine the continuum Yang-Mills gap.

## Focused gates before the run

- Preserve a subprocess refusal's actual stdout/stderr and exit code, while retaining OPEN rather than PASS.
  Demonstrate this fails on the old wrapper, then passes after the diagnostic-only repair.
- Test the smoke assessor with planted passing data and independent bad fixtures: wrong point or cutoff,
  model/settings mismatch, failed reproduction, NaN, broken weight normalization, unconverged finite-space
  residual, too-large basis step, identification failure, and dressed overlap substituted for w/Z.
- No result directory reuse: a timeout or error cannot inherit an old receipt or PASS. Write RUNNING first,
  persist each completed rung, write errors explicitly, and never generate a grid-release decision.
- Mechanism-removal checks must defeat the safety assertions. These test reporting/acceptance machinery,
  not the physics. No full test suite for this bounded diagnostic.

## Stop rule and next decision

After the two solves, stop. If the gap stabilizes but w/Z does not improve, increasing resolution is not a
demonstrated repair of identification. Any proposed alternative state identification must be a separately
registered experiment with independent geometry/charge checks, not a post-hoc replacement of this gate.
If Mq 12 fails to resolve the basis step or the eigensolver, report that and do not escalate automatically.
