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
            rejected.append({"name": name, "rejected": True, "test_output": log.getvalue()})
        with fitz.open(report.OUT/"report.pdf") as pdf:
            if pdf.metadata.get("author"):
                raise ValueError("report has an author metadata field")
            if len(pdf) != 7:
                raise ValueError(f"unexpected pagination: {len(pdf)}")
            text = "\n".join(page.get_text() for page in pdf)
            if "UNRESOLVED" not in text or "99.154" not in text:
                raise ValueError("result or qualification missing from PDF")
            if any(f"Figure {n}:" not in text for n in range(1, 6)):
                raise ValueError("a required figure caption is missing")
            for page in pdf:
                for word in page.get_text("words"):
                    if not page.rect.contains(fitz.Rect(*word[:4])):
                        raise ValueError("text outside the page")
        result.update(status="PASS", scientific_status="UNRESOLVED", tested_cases=tested.testsRun,
                      rejected_mutations=rejected, pages=7, figures=5, empty_author_metadata=True,
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
