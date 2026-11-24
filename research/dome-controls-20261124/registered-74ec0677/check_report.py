"""Replay displayed measurement tables against the retained numerical receipt."""
import hashlib
import json
import math
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPORT = HERE.parents[2] / "docs/research/helike-dome-envelope-results-2026-11-24.md"


def check(text, receipt):
    points = {(int(c["g2"]), p["cutoff"]): p for c in receipt["cases"] for p in c["points"]}

    def table(header, count):
        tail = text.split(header + "\n", 1)[1].splitlines()[1:]
        rows = []
        for line in tail:
            if not line.startswith("|"):
                break
            rows.append([v.strip() for v in line.strip("|").split("|")])
        assert len(rows) == count
        return rows

    def close(a, b):
        assert math.isclose(float(a), b, rel_tol=1e-10, abs_tol=1e-16), (a, b)

    for g, n, e0, ed, en, pct in table("| g^2 | N | Origin residual | +5 dome residual | N+2 origin residual | Reduction with +5 domes |", 8):
        a = points[int(g), int(n)]["dome"]["4"]["comparison"]["scalar"]
        for actual, key in zip((e0, ed, en), ("eps_origin", "eps_dome", "eps_next_cutoff")):
            close(actual, a[key])
        assert abs(float(pct.rstrip("%")) - 100 * (1 - a["eps_dome"] / a["eps_origin"])) <= 5e-7

    for g, n, rd, ro, ha, hb in table("| g^2 | N | Scalar r_dome | Scalar r_orig | H1a | H1b |", 8):
        a = points[int(g), int(n)]["dome"]["4"]["comparison"]
        close(rd, a["scalar"]["r_dome"])
        close(ro, a["scalar"]["r_orig"])
        assert (ha, hb) == (str(a["H1a"]).lower(), str(a["H1b"]).lower())

    for g, n, count, ev, es, en, ha, hb in table("| g^2 | N | Functions | Vacuum residual | Scalar residual | Next scalar residual | H1a | H1b |", 24):
        a = points[int(g), int(n)]["dome"][{1: "0", 3: "3", 5: "4"}[int(count)]]["comparison"]
        for val, key in zip((ev, es, en), ("vacuum", "scalar", "scalar_next")):
            close(val, a[key]["eps_dome"])
        assert (ha, hb) == (str(a["H1a"]).lower(), str(a["H1b"]).lower())

    for g, n, fv, fs, fn in table("| g^2 | N | Vacuum outside-origin fraction | Scalar fraction | Next scalar fraction |", 8):
        a = points[int(g), int(n)]["dome"]["4"]["comparison"]
        for val, key in zip((fv, fs, fn), ("vacuum", "scalar", "scalar_next")):
            close(val, a[key]["outside_origin_fraction"])


if __name__ == "__main__":
    receipt_path = HERE / "receipt.json"
    output = HERE / "report-check.json"
    output.write_text(json.dumps({"status": "RUNNING"}) + "\n")
    try:
        receipt = json.loads(receipt_path.read_text())
        text = REPORT.read_text()
        assert receipt["status"] in ("COMPLETE", "INVALID")
        check(text, receipt)
        for value in ("0.158879704627", "0.0544374438191", "0.0384785460321", "1.32838599277e-6"):
            assert value in text
            try:
                check(text.replace(value, "0.0", 1), receipt)
            except AssertionError:
                continue
            raise AssertionError("altered report number was accepted: " + value)
        result = dict(status="PASS", measurement_rows=48, altered_number_checks=4,
                      report_sha256=hashlib.sha256(REPORT.read_bytes()).hexdigest(),
                      receipt_sha256=hashlib.sha256(receipt_path.read_bytes()).hexdigest(),
                      scope="four measurement tables; not a scientific acceptance gate")
    except BaseException as exc:
        output.write_text(json.dumps(dict(status="FAIL", error=str(exc))) + "\n")
        raise
    output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result))
