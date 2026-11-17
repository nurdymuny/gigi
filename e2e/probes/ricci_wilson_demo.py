"""RF2A native queries, captured raw, against a private rebuilt GIGI engine."""
import argparse
import http.client
import json
import math
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import time
import numpy
import scipy
import sympy
from monodromy_evidence import sha, strict_json
from monodromy_exact_oracle import oracle
from ricci_evidence import ROOT, setup_queries, group_query
from ricci_wilson_cases import audits, flows, audit_query, flow_query, REFUSALS
from ricci_wilson_oracle import symbolic_certificate, operator_checks, experiment, acceleration_check, quotient_checks
from ricci_wilson_evidence import manifest, verify


def run(binary, output):
    output.mkdir(parents=True, exist_ok=True)
    path = output / "live.json"
    path.with_suffix(".PASS.json").unlink(missing_ok=True)
    data = {"schema": "ricci_wilson_live_v1",
        "head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "sources": manifest(), "binary": {"path": str(binary), "sha256": sha(binary)},
        "python_version": sys.version, "numpy_version": numpy.__version__,
        "scipy_version": scipy.__version__, "sympy_version": sympy.__version__,
        "symbolic": symbolic_certificate(), "operator_checks": operator_checks(),
        "acceleration_check": acceleration_check(), "quotient_checks": quotient_checks(), "group_oracle": oracle(),
        "reference_experiments": {}, "requests": []}
    for name, (g, normalized) in flows().items():
        data["reference_experiments"][name] = experiment(g, .02, normalized, rtol=2e-12)
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        port = s.getsockname()[1]

    def call(name, body, route="/v1/gql"):
        conn = http.client.HTTPConnection("127.0.0.1", port, timeout=60)
        try:
            conn.request("POST", route, json.dumps(body, allow_nan=False),
                         {"Content-Type": "application/json", "Accept": "application/json"})
            response = conn.getresponse()
            raw = response.read().decode("utf-8")
            parsed = strict_json(raw)
            data["requests"].append({"label": name, "path": route, "request": body,
                "status": response.status, "raw_response": raw, "response": parsed})
            return parsed
        finally:
            conn.close()

    with tempfile.TemporaryDirectory(prefix="gigi-ricci-wilson-") as scratch, (output / "server.log").open("w") as log:
        env = os.environ.copy()
        for key in ["GIGI_JWT_SECRET", "GIGI_API_KEY"]:
            env.pop(key, None)
        env.update(PORT=str(port), GIGI_DATA_DIR=scratch, RUST_LOG="warn")
        process = subprocess.Popen([str(binary)], cwd=ROOT, env=env, stdout=log, stderr=log,
                                  creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
        try:
            for _ in range(300):
                if process.poll() is not None:
                    raise RuntimeError("private server exited")
                try:
                    with socket.create_connection(("127.0.0.1", port), timeout=1):
                        break
                except OSError:
                    time.sleep(.1)
            else:
                raise RuntimeError("private server did not start")
            for name, (g, normalized) in audits().items():
                call(name, {"query": audit_query(g, normalized)})
            for name, (g, normalized) in flows().items():
                result = call("flow_" + name, {"query": flow_query(g, normalized)})
                rows = result.get("rows", [])
                if len(rows) != 257:
                    raise AssertionError(f"native flow {name} did not execute: {result}")
                for i in range(0, 257, 32):
                    metric = [rows[i][k] for k in ["a", "b", "c"]]
                    call(f"checkpoint_{name}_{i}", {"query": audit_query(metric, normalized)})
            for name, (query, _) in REFUSALS.items():
                call(name, {"query": query})
            setup = setup_queries()
            for name in ["lattice", "group_create"]:
                call(name, {"query": setup[name]})
            phi5 = (1 + math.sqrt(5)) / 2
            qs = [[.5] * 4, [phi5 / 2, .5, (phi5 - 1) / 2, 0.], [1., 0., 0., 0.], [1., 0., 0., 0.]]
            records = [dict(config_id=0, edge_id=e, vertex_a=2*e, vertex_b=2*e+1,
                       **{f"q{i}": q[i] for i in range(4)}) for e, q in enumerate(qs)]
            call("group_insert", {"records": records}, "/v1/bundles/rf_group_data/insert")
            call("group_load", {"query": setup["group_load"]})
            call("group120", {"query": group_query("0,1")})
            call("group1", {"query": group_query("2")})
        finally:
            process.terminate()
            try:
                process.wait(timeout=15)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            path.write_text(json.dumps(data, indent=2, allow_nan=False), encoding="utf-8")
    print(json.dumps(verify(path), indent=2))


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("--binary", type=Path, required=True)
    p.add_argument("--output", type=Path, required=True)
    a = p.parse_args()
    run(a.binary.resolve(), a.output.resolve())
