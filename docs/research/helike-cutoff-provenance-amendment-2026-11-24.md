# Cutoff bound provenance: retain the reviewed zero-extension guard

24 November 2026. Before any physical cutoff/tail bound was measured, the runner
at 517ab5ad refused because helike_quasimode.py differed from the original saved
state source. The refusal and empty-control receipt are retained unchanged.

The entire difference was inspected against the source snapshot: commit
8dd9a873071bcf77a908a5b2ccba8e656a050e90 added outside_support and two early returns
in point_action/cartesian_action. They implement the already specified zero
extension off the positive cutoff support. They do not change cutoff_q, geometry,
the Hamiltonian, any numerical threshold, coefficients, or any saved state.

The four model/chart modules still must match their original hashes exactly.
For helike_quasimode.py require the already accepted zero-extension revision,
whose normalized SHA-256 is

    cfd39191a61ad0c41c90ac4bf2fc0fa30ddf379d7ea1019f32bba2a8c1b41d7e

This hash also appears in the previous centered-moment receipt, source
d02970726220fd161fbdf98047610fae18a0cb47. No wildcard or source-guard bypass is
permitted. A behavioral test first reproduces rejection of the reviewed version;
after correction it must accept that version and reject a changed hash for each
of the five guarded source files. A guard-removal mutant must fail by assertion.

No analytical bound, radius rule, precision, cutoff, model, state or threshold is
amended. The complete physical grid remains held.
