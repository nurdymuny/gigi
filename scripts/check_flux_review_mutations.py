"""Remove review safeguards in memory; every mutant must fail an assertion, not crash."""
import contextlib
import io
import json
from pathlib import Path
import sys
import unittest

import helike_condensate_fock as lf
import run_helike_flux_background as run
import test_flux_review as tests
import ym_two_grain_molien as mo


MUTANTS = [
    ("newline_portability_removed", run,
     [('data = Path(p).read_bytes().replace(b"\\r\\n", b"\\n").replace(b"\\r", b"\\n")',
       'data = Path(p).read_bytes()')],
     "test_line_endings_do_not_break_receipt_replay"),
    ("molien_alias_guard_removed", mo,
     [("if isinstance(M, bool) or not isinstance(M, (int, np.integer)) or M <= max_frequency:", "if False:")],
     "test_molien_rejects_aliasing_even_when_answers_are_integer"),
    ("basis_drift_ignored", run,
     [("all(v <= D[\"convergence_tol\"] for ds in deltas.values() for v in ds)",
       "all(v <= D[\"convergence_tol\"] for v in deltas['taylor'])")],
     "test_basis_refinement_cannot_hide_behind_stable_taylor_step"),
    ("domain_hold_removed", run, [("if not args.smoke:", "if False:")],
     "test_full_grid_is_held_before_any_solver"),
    ("point_saved_too_late", run,
     [("                    case[\"points\"].append(point)\n", ""),
      ("                    point.update(assess_point(point, D))", "                    case['points'].append(point)\n                    point.update(assess_point(point, D))")],
     "test_partial_rung_is_retained_on_failure"),
    ("multiple_crossings_accepted", run, [("if len(changes) > 1:", "if False:")],
     "test_multiple_crossings_are_not_silently_selected"),
    ("smoke_misreported_as_support", run,
     [("rec[\"decision\"] = \"SOFTWARE_SMOKE_ONLY\"", "rec[\"decision\"] = \"SUPPORTED\"")],
     "test_smoke_cannot_emit_scientific_verdict"),
    ("snapshot_check_removed", run,
     [("if sha(out / \"sources\" / s) != h or sha(ROOT / s) != h:", "if False:")],
     "test_changed_snapshot_cannot_pass"),
    ("coverage_check_removed", run, [("if len(rec[\"cases\"]) != len(expected):", "if False:")],
     "test_resealed_incomplete_evidence_cannot_pass"),
    ("failed_replay_leaves_old_pass", run,
     [("    write(report_path, report)\n    try:", "    # mutant omits RUNNING invalidation\n    try:"),
      ("    write(report_path, report)\n    print(report", "    if report['status'] == 'PASS':\n        write(report_path, report)\n    print(report")],
     "test_changed_receipt_clears_stale_pass"),
    ("small_spectrum_bounds_removed", lf, [("range(1, len(e))", "range(1, nlev)")],
     "test_selection_uses_available_eigenpairs"),
    ("ambiguous_label_guard_removed", lf, [("if len(hel) != 2:", "if False:")],
     "test_accidental_harmonic_degeneracy_refuses_labels"),
    ("symmetry_guard_removed", lf, [("if np.max(np.abs(H - H.T)) > 1e-10 * scale:", "if False:")],
     "test_materially_asymmetric_spectrum_refuses"),
]


def main():
    report = {}
    for name, module, replacements, test in MUTANTS:
        saved = module.__dict__.copy()
        source = Path(module.__file__).read_text(encoding="utf-8")
        for old, new in replacements:
            if source.count(old) != 1:
                raise AssertionError(f"mutation anchor drift: {name}: {old}")
            source = source.replace(old, new, 1)
        sink = io.StringIO()
        try:
            exec(compile(source, module.__file__, "exec"), module.__dict__)
            with contextlib.redirect_stdout(sink), contextlib.redirect_stderr(sink):
                r = unittest.TextTestRunner(stream=sink).run(unittest.TestSuite([tests.FluxReview(test)]))
            report[name] = dict(caught_by_assertion=bool(r.failures) and not r.errors,
                                failures=len(r.failures), errors=len(r.errors), log=sink.getvalue())
        finally:
            module.__dict__.clear()
            module.__dict__.update(saved)
        print(name, {k: v for k, v in report[name].items() if k != "log"}, flush=True)
    report["all_caught"] = all(v["caught_by_assertion"] for v in report.values())
    if len(sys.argv) > 1:
        Path(sys.argv[1]).write_text(json.dumps(report, indent=2), encoding="utf-8")
    return 0 if report["all_caught"] else 1


if __name__ == "__main__":
    sys.exit(main())
