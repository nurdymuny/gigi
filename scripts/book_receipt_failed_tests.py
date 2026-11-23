"""Receipt for the Helike book, section 4.1: "That one sentence explains what took four failed tests to find."

The four tests are Test 2 (squashed meron, symmetry alone), Test 4 (alignment, A = s theta), Test 5 (the stretch test,
curvature singling her out) and Test 6 (the fitted edge curve). Each one's pre-registered reading failed. This script
opens each test's own receipt and prints the verdict field recorded there, so the count is read from the receipts, not
from the notes.
usage (repository root): python scripts/book_receipt_failed_tests.py research/book-receipts-20261122/failed_tests.json
"""
import json
import sys
from pathlib import Path

R = Path("research/su2-floor-family-20261119")
out = {}

d = json.loads((R / "test2_meron.json").read_text())
out["Test 2 (squashed meron): symmetry makes her the floor"] = dict(
    receipt=str(R / "test2_meron.json"), field="summary.class", value=d["summary"]["class"],
    failed=d["summary"]["class"] == "N", note="docs/research/skeleton-test2-family1-results-2026-11-20.md")

d = json.loads((R / "alignment_test_4b.json").read_text())
out["Test 4 (alignment, A = s theta): alignment makes her the floor"] = dict(
    receipt=str(R / "alignment_test_4b.json"), field="summary.class", value=d["summary"]["class"],
    failed=d["summary"]["class"] == "N", note="docs/research/alignment-test-results-2026-11-21.md")

d = json.loads((R / "stretch_test.json").read_text())
s = d["summary"]
out["Test 5 (stretch): being singled out by curvature is sufficient"] = dict(
    receipt=str(R / "stretch_test.json"), field="summary.P_suf", value=s["P_suf"],
    floor_among_singled_out=s["agreement_table"]["L1"], failed=s["P_suf"] is False,
    note="docs/research/stretch-test-results-2026-11-21.md")

d = json.loads((R / "edge_test_results.json").read_text())
out["Test 6 (edge): the fitted curve x* = c1/t + c2/t^2 and its Berger crossing"] = dict(
    receipt=str(R / "edge_test_results.json"), field="E1_pass, E2.pass", value=[d["E1_pass"], d["E2"]["pass"]],
    failed=(d["E1_pass"] is False) or (d["E2"]["pass"] is False), note="docs/research/edge-test-results-2026-11-21.md")

n_failed = sum(v["failed"] for v in out.values())
summary = dict(tests=len(out), failed=n_failed, verdict=f"{n_failed} of {len(out)} pre-registered readings failed")
for k, v in out.items():
    print(f"{k}: {v['field']} = {v['value']}  -> failed: {v['failed']}")
print(summary["verdict"])
Path(sys.argv[1]).parent.mkdir(parents=True, exist_ok=True)
Path(sys.argv[1]).write_text(json.dumps(dict(tests=out, summary=summary), indent=1))
