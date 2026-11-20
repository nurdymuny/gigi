"""YM5 = BW08 (docs/YANG_MILLS_ENGINE_SPEC.md): the SU(2) verbs through a LIVE gigi-stream.

Starts its own gigi-stream (the built binary, a free port, a throwaway data directory, its own
API key; never a production engine), ingests the paper's mesh and stored Berger links over HTTP,
and calls the verbs through both surfaces (REST and GQL):

  * RECOVER_CURVATURE ... GROUP SU2: equal to the parity probe (YM1) cell by cell;
  * LINK_FORCE, two-ring and native: equal to the probe (YM2), and BW05 from the server's force
    (independent measure) equals the paper's Table 6;
  * GQL and REST give identical numbers;
  * refusals: a missing METRIC (GQL parse error, REST 422), an unknown key, a wrong group/method.

The binary's SHA-256 goes in the receipt.

usage: python e2e/probes/live_ym5.py GIGI_STREAM_EXE RECEIPT.json [LEVELS, default 0,1]
Requires GIGI_RICCI_DATA_ROOT (frozen inputs), GIGI_RECOVER_CURVATURE_PROBE, GIGI_LINK_FORCE_PROBE.
"""
import hashlib
import json
import os
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parents[1] / "scripts"))
import test_link_force_parity as LF                  # noqa: E402  (probe runner, Table 6, measures)
import test_recover_curvature_su2_parity as RC       # noqa: E402  (probe runner)

EXE, RECEIPT = Path(sys.argv[1]), Path(sys.argv[2])
LEVELS = [int(x) for x in (sys.argv[3] if len(sys.argv) > 3 else "0,1").split(",")]
if RECEIPT.exists():
    raise SystemExit(f"refusing to overwrite {RECEIPT}")
KEY = "ym5-test-key"


def free_port():
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    p = s.getsockname()[1]
    s.close()
    return p


PORT = free_port()
assert PORT not in (3142, 3147)
BASE = f"http://127.0.0.1:{PORT}"


def call(method, path, body=None, timeout=1800):
    req = urllib.request.Request(BASE + path, method=method,
                                 data=None if body is None else json.dumps(body).encode(),
                                 headers={"Content-Type": "application/json", "X-API-Key": KEY})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return r.status, json.loads(r.read() or b"{}")
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read() or b"{}")


def gql(q):
    return call("POST", "/v1/gql", {"query": q})


data_dir = tempfile.mkdtemp(prefix="gigi-ym5-")
env = dict(os.environ, PORT=str(PORT), GIGI_DATA_DIR=data_dir, GIGI_API_KEY=KEY)
server = subprocess.Popen([str(EXE)], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.STDOUT)
receipt = {"kind": "YM5 / BW08: SU(2) verbs through a live gigi-stream", "port": PORT,
           "binary": {"path": str(EXE), "sha256": hashlib.sha256(EXE.read_bytes()).hexdigest()},
           "levels": {}}
try:
    for _ in range(120):
        try:
            if call("GET", "/v1/health", timeout=2)[0] in (200, 404):
                break
        except Exception:
            time.sleep(1)
    for level in LEVELS:
        payload = json.loads((LF.F.BERGER / f"level{level}_berger_q6.input.json").read_text())
        raw = json.loads((LF.F.BERGER / f"level{level}_berger_q6.output.json").read_text())
        cells, links = payload["cells"], np.asarray(raw["links"], float)
        cb, lb = f"ym5_cells_{level}", f"ym5_links_{level}"
        cf = {"cell": "integer", "weight": "float"}
        cf.update({f"v{i}_{k}": "float" for i in range(4) for k in range(4)})
        cf.update({f"e{e}": "integer" for e in range(6)})
        cf.update({f"r{e}": "integer" for e in range(6)})
        assert call("POST", "/v1/bundles", {"name": cb, "schema": {"fields": cf, "keys": ["cell"]}})[0] in (200, 201)
        lf = {"edge": "integer", "q0": "float", "q1": "float", "q2": "float", "q3": "float"}
        assert call("POST", "/v1/bundles", {"name": lb, "schema": {"fields": lf, "keys": ["edge"]}})[0] in (200, 201)
        recs = []
        for ci, c in enumerate(cells):
            r = {"cell": ci, "weight": float(c["weight"])}
            for i in range(4):
                for k in range(4):
                    r[f"v{i}_{k}"] = float(c["vertices"][i][k])
            for e in range(6):
                r[f"e{e}"] = int(c["edges"][e])
                r[f"r{e}"] = 1 if c["reverse"][e] else 0
            recs.append(r)
        assert call("POST", f"/v1/bundles/{cb}/insert", {"records": recs})[0] in (200, 201)
        assert call("POST", f"/v1/bundles/{lb}/insert",
                    {"records": [{"edge": g, **{f"q{k}": float(q[k]) for k in range(4)}}
                                 for g, q in enumerate(links)]})[0] in (200, 201)
        out = {}

        # YM1 through the server
        st, rest = call("POST", f"/v1/bundles/{cb}/recover_curvature", {"edges": lb, "field": "q", "group": "su2"})
        assert st == 200, (st, str(rest)[:300])
        probe = RC.run_engine(cells, links, 2, True)
        rc_rest = max(float(np.max(np.abs(np.asarray(a["coefficients"]) - np.asarray(b["coefficients"]))))
                      for a, b in zip(rest["cells"], probe["cells"]))
        st, g = gql(f"RECOVER_CURVATURE {cb} FROM {lb} ON q GROUP SU2;")
        assert st == 200 and "rows" in g, (st, str(g)[:300])
        byc = {int(r["cell"]): r for r in g["rows"]}
        rc_gql = max(abs(byc[c["cell"]][f"c{a}_{k}"] - c["coefficients"][a][k])
                     for c in rest["cells"] for a in range(3) for k in range(11))
        out["recover_su2"] = {"rest_vs_probe_max_abs": rc_rest, "gql_vs_rest_max_abs": rc_gql}
        assert rc_rest == 0.0 and rc_gql == 0.0

        # YM2 through the server
        for method in ("pg2", "native"):
            st, rest = call("POST", f"/v1/bundles/{cb}/link_force",
                            {"links": lb, "q": "q", "metric": [3.0, 1.0, 1.0], "method": method})
            assert st == 200, (st, str(rest)[:300])
            fe = np.asarray(rest["force"], float)
            fp = np.asarray(LF.run_engine(payload, links, method)["gradient"], float)
            q = f"LINK_FORCE {cb} LINKS {lb} ON q METRIC 3, 1, 1 METHOD {method.upper()};"
            st2, g = gql(q)
            assert st2 == 200 and "rows" in g, (st2, str(g)[:300])
            fg = np.array([[r["f0"], r["f1"], r["f2"]] for r in sorted(g["rows"], key=lambda r: r["edge"])])
            bw = LF.measures(payload, LF.METRIC, dict(raw, gradient=fe.tolist()))["relative_error"] * 100
            out[f"force_{method}"] = {"rest_vs_probe_max_abs": float(np.abs(fe - fp).max()),
                                      "gql_vs_rest_max_abs": float(np.abs(fg - fe).max()),
                                      "variational": rest["variational"], "bw05_percent": bw,
                                      "table6_percent": LF.TABLE6[method][level]}
            assert np.abs(fe - fp).max() == 0.0 and np.abs(fg - fe).max() == 0.0
            assert round(bw, 2) == LF.TABLE6[method][level]
            assert rest["variational"] == (method == "native")

        # refusals
        ref = {
            "gql_missing_metric": gql(f"LINK_FORCE {cb} LINKS {lb} ON q;"),
            "rest_missing_metric": call("POST", f"/v1/bundles/{cb}/link_force", {"links": lb, "q": "q"}),
            "rest_unknown_key": call("POST", f"/v1/bundles/{cb}/link_force",
                                     {"links": lb, "q": "q", "metric": [3, 1, 1], "metrik": 1}),
            "rest_bad_method": call("POST", f"/v1/bundles/{cb}/link_force",
                                    {"links": lb, "q": "q", "metric": [3, 1, 1], "method": "whitney"}),
            "rest_bad_group": call("POST", f"/v1/bundles/{cb}/recover_curvature",
                                   {"edges": lb, "field": "q", "group": "su3"}),
            "rest_missing_links": call("POST", f"/v1/bundles/{cb}/link_force",
                                       {"links": "no_such", "q": "q", "metric": [3, 1, 1]}),
        }
        out["refusals"] = {k: {"status": s, "body": str(b)[:160]} for k, (s, b) in ref.items()}
        assert ref["gql_missing_metric"][0] != 200 or "error" in ref["gql_missing_metric"][1]
        assert "METRIC" in str(ref["gql_missing_metric"][1])
        assert ref["rest_missing_metric"][0] == 422 and "metric" in str(ref["rest_missing_metric"][1]).lower()
        assert ref["rest_unknown_key"][0] == 422
        assert ref["rest_bad_method"][0] == 422
        assert ref["rest_bad_group"][0] == 422
        assert ref["rest_missing_links"][0] == 404
        receipt["levels"][str(level)] = out
        print(level, json.dumps(out)[:600], flush=True)
    RECEIPT.parent.mkdir(parents=True, exist_ok=True)
    RECEIPT.write_text(json.dumps(receipt, indent=2) + "\n")
    print("ALL LIVE YM5 CHECKS PASSED")
finally:
    server.terminate()
    try:
        server.wait(timeout=20)
    except subprocess.TimeoutExpired:
        server.kill()
