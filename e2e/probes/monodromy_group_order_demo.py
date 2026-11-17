"""Own one isolated server; GIGI computes every closure, not this client.

Run: python e2e/probes/monodromy_group_order_demo.py --binary PATH --output DIR
No third-party Python dependencies. Mathematical fixtures, not measured data.
"""
import argparse
import http.client
import json
import math
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time

from monodromy_evidence import sha, strict_json, verify
from monodromy_exact_oracle import oracle

ROOT = Path(__file__).resolve().parents[2]
ID = [1., 0., 0., 0.]
S = [0.5] * 4
PHI = (1 + math.sqrt(5)) / 2
T = [PHI / 2, 0.5, (PHI - 1) / 2, 0.]


def product(a, b):
    """Fixture conjugation only. No closure or order calculation here."""
    a0, a1, a2, a3 = a
    b0, b1, b2, b3 = b
    return [a0*b0-a1*b1-a2*b2-a3*b3,
            a0*b1+a1*b0-a2*b3+a3*b2,
            a0*b2+a2*b0-a3*b1+a1*b3,
            a0*b3+a3*b0-a1*b2+a2*b1]


def inverse(a):
    return [a[0], -a[1], -a[2], -a[3]]


def source_manifest():
    paths = list((ROOT / "src").rglob("*.rs"))
    paths += [ROOT / "Cargo.toml", ROOT / "Cargo.lock"]
    paths += list((ROOT / "e2e/probes").glob("*monodromy*.py"))
    paths += [ROOT / "tests/monodromy_group_order.rs", ROOT / "docs/MONODROMY_GROUP_ORDER_SPEC.md",
              ROOT / "GQL_REFERENCE.md", ROOT / "scripts/monodromy_mutations.py", ROOT / "scripts/monodromy_gate.py"]
    if (ROOT / "build.rs").exists():
        paths.append(ROOT / "build.rs")
    return {p.as_posix(): sha(p) for p in sorted(paths)}


def run(binary, output):
    output.mkdir(parents=True, exist_ok=True)
    evidence = output / "live.json"
    evidence.with_suffix(".PASS.json").unlink(missing_ok=True)
    manifest = source_manifest()
    binary_receipt = {"path": binary.as_posix(), "sha256": sha(binary),
                      "mtime_ns": binary.stat().st_mtime_ns}
    exact = oracle()
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        port = sock.getsockname()[1]
    requests = []

    def call(label, path, body):
        connection = http.client.HTTPConnection("127.0.0.1", port, timeout=60)
        try:
            connection.request("POST", path, json.dumps(body, allow_nan=False),
                               {"Content-Type": "application/json", "Accept": "application/json"})
            response = connection.getresponse()
            status, raw = response.status, response.read().decode("utf-8")
        finally:
            connection.close()
        parsed = strict_json(raw)
        requests.append({"label": label, "path": path, "request": body,
                         "status": status, "response": parsed, "raw_response": raw})
        return status, parsed

    def gql(label, query, refusal=False):
        status, body = call(label, "/v1/gql", {"query": query})
        if not refusal and status != 200:
            raise AssertionError(f"{label}: HTTP {status}: {body}")
        return body

    def field(name, qs):
        name = "mg_" + name
        gql(name + "_create", f"CREATE BUNDLE {name}_data (config_id INT BASE, edge_id INT BASE, vertex_a INT BASE, vertex_b INT BASE, q0 NUMERIC FIBER, q1 NUMERIC FIBER, q2 NUMERIC FIBER, q3 NUMERIC FIBER);")
        records = []
        for edge in range(4):
            q = qs[edge] if edge < len(qs) else ID
            records.append(dict(config_id=0, edge_id=edge, vertex_a=2*edge, vertex_b=2*edge+1,
                                **{f"q{i}": q[i] for i in range(4)}))
        status, body = call(name + "_insert", f"/v1/bundles/{name}_data/insert", {"records": records})
        if status != 200:
            raise AssertionError(f"insertion failed: {status}: {body}")
        gql(name + "_load", f"GAUGE_FIELD {name} GROUP SU(2) INIT FROM BUNDLE {name}_data ON LATTICE mg_lat;")

    h = [0.6, 0., 0.8, 0.]
    conjugate = lambda q: product(product(h, q), inverse(h))
    angle = math.tau / 5
    fixtures = {
        "icosahedral": [S, T], "identity": [ID], "central": [[-1., 0., 0., 0.]],
        "cyclic5": [[math.cos(angle), 0., 0., math.sin(angle)]],
        "quaternion": [[0., 1., 0., 0.], [0., 0., 1., 0.]],
        "tetrahedral": [[0., 1., 0., 0.], S], "s_only": [S], "t_only": [T],
        "reordered": [T, S], "conjugated": [conjugate(S), conjugate(T)],
        "redundant": [S, T, S, ID], "inverse": [inverse(S), T],
        "cap119": [S, T], "irrational": [[math.cos(math.sqrt(2)), 0., 0., math.sin(math.sqrt(2))]],
    }
    with tempfile.TemporaryDirectory(prefix="gigi-monodromy-") as data_dir, (output / "server.log").open("w", encoding="utf-8") as log:
        env = os.environ.copy()
        for key in ["GIGI_JWT_SECRET", "GIGI_API_KEY"]:
            env.pop(key, None)
        env.update(PORT=str(port), GIGI_DATA_DIR=data_dir, RUST_LOG="warn")
        process = subprocess.Popen([str(binary)], env=env, cwd=ROOT, stdout=log, stderr=log,
                                   creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
        try:
            for _ in range(300):
                if process.poll() is not None:
                    raise RuntimeError("isolated engine exited; see server.log")
                try:
                    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=1)
                    conn.request("GET", "/v1/health")
                    response = conn.getresponse()
                    ready = response.status == 200
                    response.read()
                    conn.close()
                    if ready:
                        break
                except OSError:
                    pass
                time.sleep(0.1)
            else:
                raise RuntimeError("isolated engine never became ready")
            gql("lattice", "LATTICE mg_lat VERTICES 8 EDGES ((0,1),(2,3),(4,5),(6,7)) FACES ();")
            for name, qs in fixtures.items():
                field(name, qs)
                cap = {"icosahedral": 120, "identity": 1, "cyclic5": 5, "cap119": 119, "irrational": 32}.get(name, 256)
                edges = ",".join(map(str, range(len(qs))))
                gql(name, f"MONODROMY_GROUP_ORDER mg_{name} FROM EDGES ({edges}) COMMUTATOR CAP {cap} TOLERANCE 0.00000001;")
            gql("u1", "GAUGE_FIELD mg_u1 GROUP U(1) INIT IDENTITY ON LATTICE mg_lat;")
            for name, target, edge in [("missing", "mg_not_declared", 0), ("bad_edge", "mg_identity", 999), ("wrong_group", "mg_u1", 0)]:
                gql(name, f"MONODROMY_GROUP_ORDER {target} FROM EDGES ({edge}) COMMUTATOR CAP 256 TOLERANCE 0.00000001;", refusal=True)
        finally:
            process.terminate()
            try:
                process.wait(timeout=15)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            # Retain requests even on a failed demonstration, never a stale PASS.
            evidence.write_text(json.dumps({"schema": "monodromy_live_v1",
                "head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
                "sources": manifest, "binary": binary_receipt, "oracle": exact,
                "address": f"http://127.0.0.1:{port}", "requests": requests}, indent=2, allow_nan=False), encoding="utf-8")
    result = verify(evidence)
    print(json.dumps(result, indent=2))
    for item in requests:
        if item["label"] in fixtures:
            r = item["response"]["rows"][0]
            print(f"{item['label']:15s} {r['status']:20s} order={r['group_order']} derived={r['commutator_subgroup_order']}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    run(args.binary.resolve(), args.output.resolve())
