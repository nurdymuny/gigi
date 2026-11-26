"""Check report evidence, artifacts, focused gates and deliberate mechanism removal."""

import hashlib
import inspect
import io
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import fitz
import build_ym_helike_report as report

PAGES = 23
import test_ym_helike_report as tests


def main():
    output = report.OUT / "report-verification.json"
    output.write_text(json.dumps({"status": "RUNNING", "source_run": report.RUN_ID}), encoding="utf-8")
    result = {"status": "FAILED", "source_run": report.RUN_ID}
    try:
        manifest = json.loads((report.OUT/"artifact-manifest.json").read_text())
        if manifest["source_receipt_sha256"] != report.digest(report.RUN/"receipt.json"):
            raise ValueError("source receipt changed")
        if manifest["source_verification_sha256"] != report.digest(report.RUN/"verification.json"):
            raise ValueError("source verification changed")
        if manifest["sector_receipt_sha256"] != report.digest(report.SECTOR_RUN/"receipt.json"):
            raise ValueError("sector receipt changed")
        if manifest["sector_verification_sha256"] != report.digest(report.SECTOR_RUN/"verification.json"):
            raise ValueError("sector verification changed")
        if manifest["builder_sha256"] != report.digest(Path(report.__file__)):
            raise ValueError("builder changed after artifact generation")
        if manifest["latex_sha256"] != report.digest(report.OUT/"report.tex"):
            raise ValueError("LaTeX changed after artifact generation")
        for name, expected in manifest["outputs"].items():
            if report.digest(report.OUT/name) != expected:
                raise ValueError("artifact changed: " + name)
        _, payload = report.load_evidence()
        if json.loads((report.OUT/"plot-data.json").read_text()) != payload:
            raise ValueError("plot data differs from the authoritative receipt")
        for name, (path, expected) in report.FLUX_RECEIPTS.items():
            if manifest["flux_receipts_sha256"][name] != expected or report.digest(report.ROOT/path) != expected:
                raise ValueError("flux receipt changed: " + name)
        if json.loads((report.OUT/"flux-plot-data.json").read_text()) != json.loads(json.dumps(report.load_flux_evidence())):
            raise ValueError("flux plot data differs from the authoritative receipts")
        if manifest["sc_receipt_sha256"] != report.SC_RECEIPT[1] or report.digest(report.ROOT/report.SC_RECEIPT[0]) != report.SC_RECEIPT[1]:
            raise ValueError("S_C status receipt changed")
        if json.loads((report.OUT/"sc-plot-data.json").read_text()) != json.loads(json.dumps(report.load_sc_evidence())):
            raise ValueError("S_C plot data differs from the authoritative receipt")
        _, sectors = report.load_sector_evidence()
        if json.loads((report.OUT/"sector-plot-data.json").read_text()) != json.loads(json.dumps(sectors)):
            raise ValueError("sector plot data differs from authoritative receipt")
        stream = io.StringIO()
        suite = unittest.defaultTestLoader.loadTestsFromModule(tests)
        tested = unittest.TextTestRunner(stream=stream, verbosity=2).run(suite)
        if not tested.wasSuccessful():
            raise ValueError(stream.getvalue())
        mutations = [
            ("stale_verification", "make_payload", 'verification.get("run_id") != receipt["run_id"] or',
             'False or', "ReportTests.test_stale_verification_and_forged_pass_are_refused"),
            ("lost_weight", "make_payload", 'abs(sum(weights)-1) > 1e-10 or',
             'False or', "ReportTests.test_missing_case_and_changed_level_weight_are_refused"),
            ("flipped_quadrature", "quadratures", 'return bc, bs',
             'return bc, -bs', "ReportTests.test_geometry_is_the_declared_tensor_not_artwork"),
            ("sector_gate_removed", "make_sector_payload",
             'm["residual_gate"] != gates or m["conservative_residual_gate"] != strict',
             'False', "SectorReportTests.test_rejects_loose_allowance_and_forged_residual_pass"),
            ("sector_certification_promoted", "make_sector_payload",
             'verification.get("certified") is not False or', 'False or',
             "SectorReportTests.test_rejects_false_certification_and_failed_precision"),
            ("flux_reverse_order_accepted", "make_flux_payload",
             'if verdict not in ("ORDERED", "UNRESOLVED"):', 'if False:',
             "FluxReportTests.test_rejects_reversed_or_promoted_claims"),
            ("flux_grid_release_accepted", "make_flux_payload",
             'leak.get("full_grid_released") is not False or', 'False or',
             "FluxReportTests.test_rejects_released_grid_and_missing_hypothesis"),
            ("sc_promotion_accepted", "make_sc_payload",
             'if status.get("S_C") != "STATED_HYPOTHESIS":', 'if False:',
             "SCReportTests.test_rejects_promoted_sc"),
            ("sc_floating_listed_rigorous", "make_sc_payload",
             'if any("homotopy" in item for item in status.get("proved_rigorous", [])):', 'if False:',
             "SCReportTests.test_rejects_floating_homotopy_listed_as_rigorous"),
            ("sector_arithmetic_unchecked", "make_sector_payload",
             'not math.isclose(m[key], value, rel_tol=0, abs_tol=1e-13)', 'False',
             "SectorReportTests.test_rejects_nonfinite_residual_and_changed_gap"),
        ]
        rejected = []
        for name, function, old, new, gate in mutations:
            source = inspect.getsource(getattr(report, function))
            if old not in source:
                raise ValueError("mutation target changed")
            namespace = dict(report.__dict__)
            exec(compile(source.replace(old, new), f"<report-mutation:{name}>", "exec"), namespace)
            log = io.StringIO()
            with patch.object(report, function, namespace[function]):
                trial = unittest.TextTestRunner(stream=log).run(unittest.defaultTestLoader.loadTestsFromName(gate, tests))
            if trial.wasSuccessful():
                raise ValueError("removed mechanism was accepted: " + name)
            if trial.errors or not trial.failures:
                raise ValueError("mutation must fail by assertion, not an unexpected error: " + name)
            rejected.append({"name": name, "rejected": True, "test_output": log.getvalue()})
        with fitz.open(report.OUT/"report.pdf") as pdf:
            if pdf.metadata.get("author"):
                raise ValueError("report has an author metadata field")
            if len(pdf) != PAGES:
                raise ValueError(f"unexpected pagination: {len(pdf)}")
            text = "\n".join(page.get_text() for page in pdf)
            if "UNRESOLVED" not in text or "99.154" not in text:
                raise ValueError("result or qualification missing from PDF")
            if any(f"Figure {n}:" not in text for n in range(1, 16)):
                raise ValueError("a required figure caption is missing")
            if "0.002774" not in text or "0.004552" not in text or "FULL-OPERATOR ERROR" not in text:
                raise ValueError("new numerical result or mathematical derivation missing")
            if "FLUX BACKGROUND" not in text or "+0.007179" not in text or "THE ORDERING" not in text:
                raise ValueError("flux-background pages or their ordering result missing")
            if "TOWARD S" not in text or "still a hypothesis" not in text or "Floating point only" not in text:
                raise ValueError("S_C status page or its qualification missing")
            for page in pdf:
                for word in page.get_text("words"):
                    if not page.rect.contains(fitz.Rect(*word[:4])):
                        raise ValueError("text outside the page")
        result.update(status="PASS", scientific_status="UNRESOLVED", tested_cases=tested.testsRun,
                      rejected_mutations=rejected, pages=PAGES, figures=15, sector_cases=16, empty_author_metadata=True,
                      artifact_manifest_sha256=report.digest(report.OUT/"artifact-manifest.json"),
                      verifier_sha256=report.digest(Path(__file__)), test_output=stream.getvalue(),
                      disclosure="Report integrity PASS is not scientific convergence PASS; visual review is separate.")
    except BaseException as exc:
        result["error"] = f"{type(exc).__name__}: {exc}"
        raise
    finally:
        output.write_text(json.dumps(result, indent=2, allow_nan=False)+"\n", encoding="utf-8")
    print(json.dumps({k: result[k] for k in ("status", "scientific_status", "tested_cases", "pages", "figures")}, indent=2))


if __name__ == "__main__":
    main()
