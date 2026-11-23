# Settling diagnosis receipt

Evidence checkpoint before diagnosis: `633a77a`. No RC engine changes, release,
deployment or hardware-gate closure are included in this investigation.

`report.json` records the engine source manifest, binary, input and diagnostic
protocol hashes and all six traces, including unfavorable controls. `tests.json`
records ten passing helper tests and two assertion failures after deliberate
mechanism removal. `initial-stub-red.log` retains the preliminary runtime-stub
failures; these are not accepted as strict behavioral RED gates.

`local-artifacts.json` hashes the detailed local evidence under
`output/conduct-settle/diagnosis-20261113/`: exact HTTP requests/responses, six
pointwise comparisons, and build/server logs. These include third-party recorded
values and are not redistributed here without an established license. They
remain available in this workspace. The source CSV hashes and download URLs are
in the earlier measured report. The tested engine still has earlier uncommitted
prerequisites; this Git commit is not a standalone engine release.

The results and limitations are in
`docs/research/settling-diagnosis-results-2026-11-13.md`. The smallest proposed
application fix is specified in `docs/SETTLING_APPLICABILITY_SPEC.md`; its
SA00-SA12 gates are not run. CS08 remains OPEN.
