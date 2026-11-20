"""YM5 flow (docs/YANG_MILLS_ENGINE_SPEC.md, Amendment 1): LINK_FLOW through a LIVE gigi-stream.

Starts its own gigi-stream (free port, throwaway data directory, own API key; never a production engine),
ingests the mesh, the stored Berger links and the edge points over HTTP, and checks:

  F1  route = probe (same engine-built K0): rows and final links identical (0.0)
  F2  engine-K0 flow vs the S3b receipt: every E to 1e-6 relative; final R to the paper's printed digits
      (two-ring 13.8% / 1.77% / 0.87%; frozen control 8.13% at n = 3); Newton counts reported
  F3  preconditioner effect (probe): same flow with the research K0 and with the engine's: largest E gap
  F4  refusals: missing metric sample (422, names the time), unknown key (422), missing bundle (404)

usage: python e2e/probes/live_ym5_flow.py GIGI_STREAM_EXE RECEIPT.json [CASES, default "0:native,0:pg,1:native,1:pg"]
Requires GIGI_RICCI_DATA_ROOT and GIGI_LINK_FLOW_PROBE (built from the same tree).
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
import test_link_flow_parity as FLW              # noqa: E402

EXE, RECEIPT = Path(sys.argv[1]), Path(sys.argv[2])
CASES = [(int(c.split(":")[0]), c.split(":")[1])
         for c in (sys.argv[3] if len(sys.argv) > 3 else "0:native,0:pg,1:native,1:pg").split(",")]
if RECEIPT.exists():
    raise SystemExit(f"refusing to overwrite {RECEIPT}")
KEY = "ym5-flow-key"
PAPER_R = {(0, "pg"): "13.8", (1, "pg"): "1.77", (2, "pg"): "0.87", (2, "pg_fixed"): "8.13"}


def free_port():
    s = socket.socket(); s.bind(("127.0.0.1", 0)); p = s.getsockname()[1]; s.close()
    return p


PORT = free_port()
assert PORT not in (3142, 3147)
BASE = f"http://127.0.0.1:{PORT}"


def call(method, path, body=None, timeout=36000):
    req = urllib.request.Request(BASE + path, method=method,
                                 data=None if body is None else json.dumps(body).encode(),
                                 headers={"Content-Type": "application/json", "X-API-Key": KEY})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return r.status, json.loads(r.read() or b"{}")
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read() or b"{}")


def inputs(level, trial):
    lat = FLW.FL.Lattice(level, "pg" if trial == "pg_fixed" else trial)
    met_step, met_true = FLW.FL.Metric(fixed=(trial == "pg_fixed")), FLW.FL.Metric()
    h = FLW.T_END / FLW.N
    step = [[t, [float(x) for x in met_step(t)]] for t in [0.0] + [(k + 0.5) * h for k in range(FLW.N)]]
    ref = [[t, [float(x) for x in met_true(t)]] for t in [(k + 1) * h for k in range(FLW.N)]]
    return lat, step, ref


def probe(lat, trial, step, ref, k0_research):
    with tempfile.TemporaryDirectory(prefix="gigi-ym5f-") as d:
        d = Path(d)
        body = {"order": 6, "method": "native" if trial == "native" else "pg2", "steps": FLW.N, "t_end": FLW.T_END,
                "metric_step": step, "metric_ref": ref, "links": lat.raw["links"], "edge_points": lat.edge_points,
                "cells": [{"vertices": c["vertices"], "edges": c["edges"], "reverse": [bool(r) for r in c["reverse"]],
                           "weight": float(c["weight"])} for c in lat.cells]}
        if k0_research:
            K = FLW.CAY.load_K(lat.level, trial).tocoo()
            K.row.astype("<u4").tofile(d / "r.bin"); K.col.astype("<u4").tofile(d / "c.bin")
            K.data.astype("<f8").tofile(d / "v.bin")
            body["k0"] = {"rows": str(d / "r.bin"), "cols": str(d / "c.bin"), "vals": str(d / "v.bin")}
        (d / "in.json").write_text(json.dumps(body))
        subprocess.run([os.environ["GIGI_LINK_FLOW_PROBE"], str(d / "in.json"), str(d / "out.json")],
                       check=True, timeout=36000)
        res = json.loads((d / "out.json").read_text())
        res["_final"] = np.fromfile(d / "out.final.bin", dtype="<f8").reshape(-1, 4)
    return res


data_dir = tempfile.mkdtemp(prefix="gigi-ym5f-")
env = dict(os.environ, PORT=str(PORT), GIGI_DATA_DIR=data_dir, GIGI_API_KEY=KEY)
server = subprocess.Popen([str(EXE)], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.STDOUT)
receipt = {"kind": "YM5 flow (Amendment 1): LINK_FLOW through a live gigi-stream", "port": PORT,
           "binary": {"path": str(EXE), "sha256": hashlib.sha256(EXE.read_bytes()).hexdigest()}, "cases": {}}
try:
    for _ in range(120):
        try:
            if call("GET", "/v1/health", timeout=2)[0] in (200, 404):
                break
        except Exception:
            time.sleep(1)
    loaded = set()
    for level, trial in CASES:
        lat, step, ref = inputs(level, trial)
        cb, lb, eb = f"f_cells_{level}", f"f_links_{level}", f"f_points_{level}"
        if level not in loaded:
            cf = {"cell": "integer", "weight": "float"}
            cf.update({f"v{i}_{k}": "float" for i in range(4) for k in range(4)})
            cf.update({f"e{e}": "integer" for e in range(6)}); cf.update({f"r{e}": "integer" for e in range(6)})
            assert call("POST", "/v1/bundles", {"name": cb, "schema": {"fields": cf, "keys": ["cell"]}})[0] in (200, 201)
            assert call("POST", "/v1/bundles", {"name": lb, "schema": {"fields": {"edge": "integer", **{f"q{k}": "float" for k in range(4)}},
                                                                     "keys": ["edge"]}})[0] in (200, 201)
            pf = {"edge": "integer", **{f"p{a}_{k}": "float" for a in range(2) for k in range(4)}}
            assert call("POST", "/v1/bundles", {"name": eb, "schema": {"fields": pf, "keys": ["edge"]}})[0] in (200, 201)
            recs = []
            for ci, c in enumerate(lat.cells):
                r = {"cell": ci, "weight": float(c["weight"])}
                for i in range(4):
                    for k in range(4):
                        r[f"v{i}_{k}"] = float(c["vertices"][i][k])
                for e in range(6):
                    r[f"e{e}"] = int(c["edges"][e]); r[f"r{e}"] = 1 if c["reverse"][e] else 0
                recs.append(r)
            assert call("POST", f"/v1/bundles/{cb}/insert", {"records": recs})[0] in (200, 201)
            assert call("POST", f"/v1/bundles/{lb}/insert", {"records": [
                {"edge": g, **{f"q{k}": float(q[k]) for k in range(4)}} for g, q in enumerate(lat.raw["links"])]})[0] in (200, 201)
            assert call("POST", f"/v1/bundles/{eb}/insert", {"records": [
                {"edge": g, **{f"p{a}_{k}": float(p[a][k]) for a in range(2) for k in range(4)}}
                for g, p in enumerate(lat.edge_points)]})[0] in (200, 201)
            loaded.add(level)
        body = {"links": lb, "q": "q", "method": "native" if trial == "native" else "pg2", "steps": FLW.N,
                "t_end": FLW.T_END, "metric_step": step, "reference": {"edge_points": eb, "metric_ref": ref}}
        st, route = call("POST", f"/v1/bundles/{cb}/link_flow", body)
        assert st == 200 and route["status"] == "ok", (st, str(route)[:400])
        p_eng = probe(lat, trial, step, ref, k0_research=False)
        p_res = probe(lat, trial, step, ref, k0_research=True)
        s3b = json.loads((FLW.RECEIPTS / f"L{level}_{trial}_N{FLW.N}.json").read_text())
        f1_rows = max(abs(a["E"] - b["E"]) for a, b in zip(route["rows"], p_eng["rows"]))
        f1_links = float(np.abs(np.asarray(route["final_links"]) - p_eng["_final"]).max())
        f2 = max(abs(a["E"] - b["E"]) / abs(b["E"]) for a, b in zip(route["rows"], s3b["rows"]))
        f3 = max(abs(a["E"] - b["E"]) / abs(b["E"]) for a, b in zip(p_eng["rows"], p_res["rows"]))
        R = route["rows"][-1]["R"]
        row = {"F1_rows_max_abs": f1_rows, "F1_final_links_max_abs": f1_links, "F2_E_rel_vs_S3b": f2,
               "F3_engineK0_vs_researchK0_E_rel": f3, "R_final": R, "R_percent": 100 * R,
               "paper_R_percent": PAPER_R.get((level, trial)),
               "newton_route": route["newton_iterations"], "newton_S3b": s3b["newton_iterations"],
               "k0_seconds": route["k0_seconds"], "seconds": route["seconds"], "variational": route["variational"]}
        receipt["cases"][f"level{level}_{trial}"] = row
        print(level, trial, json.dumps(row)[:500], flush=True)
        assert f1_rows == 0.0 and f1_links == 0.0
        assert f2 <= 1e-6, f"F2 {f2:.2e}"
        assert f3 <= 1e-6, f"F3 {f3:.2e}"
        if (level, trial) in PAPER_R:
            want = PAPER_R[(level, trial)]
            assert f"{100 * R:.{len(want.split('.')[1])}f}" == want, (100 * R, want)

    # F4 refusals (on the first case's bundles)
    level, trial = CASES[0]
    lat, step, ref = inputs(level, trial)
    cb, lb = f"f_cells_{level}", f"f_links_{level}"
    base = {"links": lb, "q": "q", "method": "pg2", "steps": FLW.N, "t_end": FLW.T_END, "metric_step": step}
    missing = dict(base, metric_step=[s for s in step if abs(s[0] - 0.5 * FLW.T_END / FLW.N) > 1e-15])
    refusals = {"missing_metric_sample": call("POST", f"/v1/bundles/{cb}/link_flow", missing),
                "no_metric": call("POST", f"/v1/bundles/{cb}/link_flow", dict(base, metric_step=[])),
                "unknown_key": call("POST", f"/v1/bundles/{cb}/link_flow", dict(base, stepz=8)),
                "missing_bundle": call("POST", "/v1/bundles/no_such/link_flow", base)}
    receipt["F4_refusals"] = {k: {"status": s, "body": str(b)[:200]} for k, (s, b) in refusals.items()}
    assert refusals["missing_metric_sample"][0] == 422 and "t = 0.00125" in str(refusals["missing_metric_sample"][1])
    assert refusals["no_metric"][0] == 422
    assert refusals["unknown_key"][0] == 422
    assert refusals["missing_bundle"][0] == 404
    RECEIPT.parent.mkdir(parents=True, exist_ok=True)
    RECEIPT.write_text(json.dumps(receipt, indent=2) + "\n")
    print("ALL LIVE YM5 FLOW CHECKS PASSED")
finally:
    server.terminate()
    try:
        server.wait(timeout=20)
    except subprocess.TimeoutExpired:
        server.kill()
