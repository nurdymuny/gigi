"""Fresh isolated engine, planted decisions, separate processes, read-only restart.

Run from any directory with --engine pointing at a freshly built gigi-stream.
Evidence/logs are retained in an automatically created directory under outputs/.
Only the child process created here is stopped; existing engines are untouched.
"""

import argparse
from contextlib import redirect_stdout
import hashlib
import io
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import time
from unittest.mock import patch

import registry_bootstrap as db
import registry_gates as gates
import registry_paths as paths
import registry_review as review

GOOD = "DG-REL-DAVISMANIFOLD-PULLBACK-001"
SID = "DG-REF-COORDINATE-SAMENESS-001"
START = "davis-manifold-metric-construction"
SIG = GOOD + ">DG-REL-PULLBACK-JACOBIAN-001"
ROOT = Path(__file__).resolve().parents[3]


def require(condition, message):
    if not condition:
        raise AssertionError(message)
    print("[PASS]", message, flush=True)


def snapshot():
    return {name: sorted(db.query(name), key=lambda r: r["id"])
            for name in db.SCHEMAS}


def live_adversaries():
    rel = review.relation(GOOD)
    db.write_verified("research_relations", dict(rel, established_altitude="NONE"))
    with patch.object(gates, "promote", review.requirement_met), redirect_stdout(io.StringIO()):
        gates.FAILS.clear()
        gates.gate2()
    require(gates.FAILS == ["G2b promotion changes stored NONE to RESONANCE"],
            "live promotion gate fails when the write mechanism is removed")
    gates.FAILS.clear()

    original_call = db.call
    for status in (422, 200):
        def no_write(path, payload=None, method=None):
            if path.endswith("/insert"):
                return status, {"error": "planted refusal"} if status == 422 else {"status": "ok"}
            return original_call(path, payload, method)
        with patch.object(db, "call", no_write):
            ok, reason = review.promote(GOOD, "RESONANCE")
            require(not ok, f"HTTP {status} without a stored write cannot report promotion success: {reason}")
        require(review.relation(GOOD)["established_altitude"] == "NONE", "failed write left altitude unchanged")

    originals = review.evidence_for(GOOD)
    try:
        for e in originals:
            db.write_verified("research_relation_evidence", dict(e, source_id="missing-source"))
        require(not review.promote(GOOD, "RESONANCE")[0] and review.view(GOOD)["needs_review"],
                "missing sources refuse promotion and flag retrieval")
        for e in originals:
            db.write_verified("research_relation_evidence", dict(e, locator="   "))
        require(not review.promote(GOOD, "RESONANCE")[0] and review.view(GOOD)["needs_review"],
                "blank locators refuse promotion and flag retrieval")
    finally:
        for e in originals:
            db.write_verified("research_relation_evidence", e)
    require(review.promote(GOOD, "RESONANCE")[0]
            and review.relation(GOOD)["established_altitude"] == "RESONANCE",
            "restored evidence permits a real NONE to RESONANCE transition")

    before = db.query("research_path_reviews")
    invalid = [(SIG, "anything_goes", "reason", "", "reviewer", True),
               ("nonexistent", "incompatible", "reason", "", "reviewer", True),
               (SIG, "compatible_under", "reason", "", "reviewer", True),
               (SIG, "incompatible", "", "", "", True)]
    for args in invalid:
        try:
            paths.record_composition_review(*args)
        except ValueError:
            pass
        else:
            raise AssertionError(f"invalid ruling accepted: {args}")
    require(db.query("research_path_reviews") == before, "invalid rulings create no database records")

    claims = [dict(db.CLAIMS[0], id="CHECK-A", value="r", value_kind="expression"),
              dict(db.CLAIMS[0], id="CHECK-B", value="1*r", value_kind="expression")]
    for claim in claims:
        db.write_verified("research_claims", claim)
    require(not any({"CHECK-A", "CHECK-B"} <= set(c["claim_ids"]) for c in paths.contradictions()),
            "r and 1*r are not labelled a contradiction")
    require(any(c["claim_ids"] == ["CHECK-A", "CHECK-B"] for c in paths.possible_conflicts()),
            "unparsed symbolic comparison appears in the review queue")
    for claim, value in zip(claims, ("1", "1.0")):
        db.write_verified("research_claims", dict(claim, value=value, value_kind="numeric"))
    require(not any(c["claim_ids"] == ["CHECK-A", "CHECK-B"] for c in paths.contradictions()),
            "equal exact numbers with different spellings do not conflict")


def cross_process_reviews():
    ids = []
    for seed, state in (("1", "incompatible"), ("2", "compatible_under")):
        code = ("import registry_paths as p; print(p.record_composition_review("
                + repr(SIG) + ", " + repr(state)
                + ", 'replacement from process " + seed
                + "', 'full rank J', 'live test reviewer', True))")
        rid = subprocess.check_output([sys.executable, "-c", code], text=True,
               cwd=Path(__file__).parent,
               env=dict(os.environ, GIGI_REGISTRY_URL=db.BASE, PYTHONHASHSEED=seed)).strip()
        ids.append(rid)
    rows = db.query("research_path_reviews", [db.eq("path_signature", SIG)])
    result = paths._review(SIG)
    require(ids[0] == ids[1] and len(rows) == 1, "two Python hash seeds update one stable review record")
    require(result["composition_state"] == "compatible_under"
            and result["reason"] == "replacement from process 2",
            "the explicit replacement ruling and reason are returned")


def check_retraction():
    direct = review.view(GOOD)
    path = next(p for p in paths.traverse(START) if p["path_signature"] == SIG)
    require(direct["needs_review"] and path["needs_review"]
            and path["edges"][0]["flags"] == direct["flags"]
            and any("RETRACTED" in f for f in direct["flags"]),
            "direct and path readers see the named retracted source")
    require(path["edges"][0]["established_altitude"] == "RESONANCE",
            "stored altitude remains visible with an explicit invalidation warning")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", type=Path, required=True)
    args = parser.parse_args()
    engine = args.engine.resolve(strict=True)
    output_parent = ROOT / "outputs"
    output_parent.mkdir(exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix="registry-verification-", dir=output_parent))
    print("Evidence directory:", output, flush=True)
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        port = probe.getsockname()[1]
    db.BASE = f"http://127.0.0.1:{port}"
    env = dict(os.environ, PORT=str(port), GIGI_DATA_DIR=str(output / "engine-data"))
    process = None
    logs = []

    def start(label):
        nonlocal process
        stdout = open(output / f"{label}.stdout.log", "wb")
        stderr = open(output / f"{label}.stderr.log", "wb")
        logs.extend([stdout, stderr])
        process = subprocess.Popen([str(engine)], cwd=output, env=env, stdout=stdout, stderr=stderr,
                    creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
        deadline = time.monotonic() + 45
        while time.monotonic() < deadline:
            if process.poll() is not None:
                raise RuntimeError(f"engine exited: see {output}")
            try:
                status, body = db.call("/v1/health")
                if status == 200:
                    return
            except OSError:
                pass
            time.sleep(0.2)
        raise RuntimeError("engine did not become ready within 45 seconds")

    def stop():
        if process is not None and process.poll() is None:
            process.kill()
            process.wait(timeout=20)

    receipt = dict(engine=str(engine), engine_sha256=hashlib.sha256(engine.read_bytes()).hexdigest(),
                   engine_mtime_ns=engine.stat().st_mtime_ns, base_url=db.BASE,
                   python=sys.version, passed=False)
    try:
        start("initial")
        db.main()
        require(gates.main() == 0, "all fresh-fixture acceptance gates pass against live HTTP")
        live_adversaries()
        cross_process_reviews()
        src = review.source(SID)
        db.write_verified("research_sources", dict(src, availability="retracted"))
        check_retraction()
        before = snapshot()
        (output / "before-restart.json").write_text(json.dumps(before, indent=2), encoding="utf-8")
        stop()
        start("restart")
        # Every operation after restart is read-only. Do not rerun mutating gates.
        after = snapshot()
        (output / "after-restart.json").write_text(json.dumps(after, indent=2), encoding="utf-8")
        require(after == before, f"all {len(before)} bundles match exactly after forced-stop WAL replay")
        require(review.relation(GOOD)["established_altitude"] == "RESONANCE", "real promotion survives restart")
        require(paths._review(SIG)["reason"] == "replacement from process 2", "replacement review survives restart")
        check_retraction()
        receipt["passed"] = True
    finally:
        stop()
        for log in logs:
            log.close()
        (output / "receipt.json").write_text(json.dumps(receipt, indent=2), encoding="utf-8")
    print("Live verification complete; engine stopped.", flush=True)


if __name__ == "__main__":
    main()
