# Dome-envelope experiment evidence

24 November 2026. Registration:
`docs/research/helike-dome-envelope-preregistration-2026-11-24.md`.

## Before measurement

The upstream 11 mathematical tests passed. Five deliberate mathematical defects
were rejected by assertions, with zero unexpected errors. Runner tests exposed
three behavioral failures: unreachable truncation status, incomplete primary
coverage accepted, and repeats covering only the primary dome. A further RED
test demonstrated that an empty COMPLETE receipt could pass the first draft
of evidence verification. These are corrected with recorded RED/GREEN outputs.

The final focused gates are 11 math tests plus seven execution tests. All pass,
with no skips. Four runner mechanism removals are rejected by assertions. The
matrix/Haar/moment calculations and registered physical design are unchanged.
The solver additionally returns its overlap matrix for evidence replay.

The `smoke/` folder is a tiny, explicitly unregistered software test. Its 14
saved solves replay successfully. Its decision is not scientific evidence for
the registered hypothesis. It retains the precommit source bytes used by that
check; the final source list additionally includes the runner mutation script.

## Registered measurement

The registered run is launched only after committing and pushing the reviewed
source. Each solve is archived and logged immediately. A source snapshot,
resource log, terminal receipt and saved-matrix verification accompany it.
The resource ceiling is 45 minutes and 8 GiB, one BLAS thread, CPU only.
Do not infer a scientific conclusion from a partial run or a width-grid refusal.

The completed run is `registered-74ec0677/`, pinned to source
`74ec06775770aaea5e08b8c9eaf71a3c16f27fa3`. All 80 solves are retained and
replay successfully. The scientific verdict is **UNRESOLVED_TRUNCATION**:
D00, D01, D02, D03 and D05 pass; D04 fails in the one-function N=18, g^2=1
comparison. The terminal status INVALID and nonzero process exit reflect that
registered refusal, not a crash. The separate verification PASS means saved
evidence is consistent; it does not close the scientific gate.

See [the complete measured-results note](../../docs/research/helike-dome-envelope-results-2026-11-24.md)
for all eight primary cases, all 24 dome configurations, the truncation
reversal, projection geometry, limits and replay commands. No follow-up physical
experiment was launched. `check_report.py` rechecks the four main numerical
tables and rejects four deliberately altered displayed values; it is not a
physical-model verifier.
