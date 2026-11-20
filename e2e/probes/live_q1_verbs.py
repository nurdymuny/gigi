"""CORRELATOR / EFFECTIVE_MASS through a live gigi-stream (Q1 registration): GQL and REST against the
probe on the same series. Own port (never 3142 / 3147), own data dir, own key.

  L1  the Q2 fresh heat-kernel beta = 8 Polyakov series (first 2,000 configurations, block 20):
      CORRELATOR rows and the EFFECTIVE_MASS plateau, GQL and REST, equal the probe to 1e-12 relative;
  L2  EFFECTIVE_MASS ... GEVP 1, 2 over (Polyakov, plaquette row) equals the probe's GEVP mode;
  L3  the fresh Halcyon L = 16 ensemble (block 1): GQL errors and REST answers 422, both naming t,
      as the probe refuses.

usage: python live_q1_verbs.py GIGI_STREAM_EXE PROBE_EXE RECEIPT.json
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

EXE, PROBE, RECEIPT = Path(sys.argv[1]), sys.argv[2], Path(sys.argv[3])
if RECEIPT.exists():
    raise SystemExit(f"refusing to overwrite {RECEIPT}")
KEY = "q1-verbs-test-key"
s = socket.socket()
s.bind(("127.0.0.1", 0))
PORT = s.getsockname()[1]
s.close()
assert PORT not in (3142, 3147)
BASE = f"http://127.0.0.1:{PORT}"


def call(method, path, body=None, timeout=600):
    req = urllib.request.Request(BASE + path, method=method,
                                 data=None if body is None else json.dumps(body).encode(),
                                 headers={"Content-Type": "application/json", "X-API-Key": KEY})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return r.status, json.loads(r.read() or b"{}")
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read() or b"{}")


def probe(payload):
    with tempfile.TemporaryDirectory() as d:
        i, o = Path(d) / "i.json", Path(d) / "o.json"
        i.write_text(json.dumps(payload))
        subprocess.run([PROBE, str(i), str(o)], check=True)
        return json.loads(o.read_text())


def load(name, fields):
    """fields: {field: array [n][L]}"""
    n, L = next(iter(fields.values())).shape
    sch = {"id": "integer", "cfg": "integer", "t": "integer"}
    sch.update({f: "float" for f in fields})
    assert call("POST", "/v1/bundles", {"name": name, "schema": {"fields": sch, "keys": ["id"]}})[0] in (200, 201)
    recs = [{"id": c * L + t, "cfg": c, "t": t, **{f: float(a[c, t]) for f, a in fields.items()}}
            for c in range(n) for t in range(L)]
    for k in range(0, len(recs), 5000):
        assert call("POST", f"/v1/bundles/{name}/insert", {"records": recs[k:k + 5000]})[0] in (200, 201)


def rel(a, b):
    return abs(a - b) / max(abs(b), 1e-300)


root = Path(__file__).resolve().parents[2]
q2 = json.loads((root / "research/q2-exact-2d-fresh-20261119/heat_kernel_b8.ensemble.json").read_text())
pol = np.array(q2["polyakov"])[:2000]
plq = np.array(q2["plaquette_rows"])[:2000]
hal = np.load(root / "research/q1-halcyon-m1-fresh-20261119/m1_su2_L16_beta2p3_seed20261119.npy")

data_dir = tempfile.mkdtemp(prefix="gigi-q1verbs-")
env = dict(os.environ, PORT=str(PORT), GIGI_DATA_DIR=data_dir, GIGI_API_KEY=KEY)
server = subprocess.Popen([str(EXE)], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.STDOUT)
rec = {"kind": "Q1 verbs through a live gigi-stream", "port": PORT,
       "binary": {"path": str(EXE), "sha256": hashlib.sha256(EXE.read_bytes()).hexdigest()}}
gates = {}
try:
    for _ in range(120):
        try:
            if call("GET", "/v1/health", timeout=2)[0] in (200, 404):
                break
        except Exception:
            time.sleep(1)
    load("q2pol", {"p": pol, "w": plq})
    load("hal16", {"p": hal})

    # L1
    pr = probe({"p": pol.tolist(), "block": 20})
    st, g = call("POST", "/v1/gql", {"query": "CORRELATOR q2pol ON p BLOCK 20;"})
    gr = sorted(g.get("rows", []), key=lambda r: r["t"])
    c_gql = max(rel(r["C"], pr["C_t"][int(r["t"])]) for r in gr) if gr else float("inf")
    st2, rr = call("POST", "/v1/bundles/q2pol/correlator", {"fields": ["p"], "block": 20})
    rrows = sorted(rr.get("rows", []), key=lambda r: r["t"])
    c_rest = max(rel(r["C"], pr["C_t"][int(r["t"])]) for r in rrows) if rrows else float("inf")
    st3, ge = call("POST", "/v1/gql", {"query": "EFFECTIVE_MASS q2pol ON p BLOCK 20;"})
    st4, re_ = call("POST", "/v1/bundles/q2pol/effective_mass", {"fields": ["p"], "block": 20})
    m_gql = rel(ge["rows"][0]["mass"], pr["plateau"]["mass"]) if st3 == 200 else float("inf")
    m_rest = rel(re_["rows"][0]["mass"], pr["plateau"]["mass"]) if st4 == 200 else float("inf")
    e_rest = rel(re_["rows"][0]["error"], pr["plateau"]["error"]) if st4 == 200 else float("inf")
    rec["L1"] = {"status": [st, st2, st3, st4], "C_gql": c_gql, "C_rest": c_rest, "mass_gql": m_gql,
                 "mass_rest": m_rest, "error_rest": e_rest, "probe_plateau": pr["plateau"]}
    gates["L1"] = max(c_gql, c_rest, m_gql, m_rest, e_rest) <= 1e-12

    # L2
    pg = probe({"ops": [pol.tolist(), plq.tolist()], "t0": 1, "t1": 2, "block": 20})
    st, gg = call("POST", "/v1/gql", {"query": "EFFECTIVE_MASS q2pol ON p, w BLOCK 20 GEVP 1, 2;"})
    st2, rg = call("POST", "/v1/bundles/q2pol/effective_mass", {"fields": ["p", "w"], "block": 20, "gevp": [1, 2]})
    rec["L2"] = {"status": [st, st2], "probe": pg.get("plateau"),
                 "gql": gg.get("rows", [{}])[0].get("mass") if st == 200 else gg,
                 "rest": rg.get("rows", [{}])[0].get("mass") if st2 == 200 else rg}
    gates["L2"] = st == 200 and st2 == 200 and "mass" in pg["plateau"] \
        and rel(gg["rows"][0]["mass"], pg["plateau"]["mass"]) <= 1e-12 \
        and rel(rg["rows"][0]["mass"], pg["plateau"]["mass"]) <= 1e-12

    # L3
    ph = probe({"p": hal.tolist(), "block": 1})
    st, gh = call("POST", "/v1/gql", {"query": "EFFECTIVE_MASS hal16 ON p;"})
    st2, rh = call("POST", "/v1/bundles/hal16/effective_mass", {"fields": ["p"]})
    rec["L3"] = {"probe": ph["plateau"], "gql": [st, gh], "rest": [st2, rh]}
    gates["L3"] = "refused" in ph["plateau"] and st2 == 422 and "at t =" in json.dumps(rh) \
        and "at t =" in json.dumps(gh)
finally:
    server.terminate()
    server.wait(timeout=30)
rec["gates"] = gates
rec["all_pass"] = all(gates.values())
RECEIPT.parent.mkdir(parents=True, exist_ok=True)
RECEIPT.write_text(json.dumps(rec, indent=1) + "\n")
print(json.dumps({"gates": gates, "L1": {k: rec["L1"][k] for k in ("status", "C_gql", "C_rest", "mass_gql", "mass_rest")},
                  "L3": {"gql": rec["L3"]["gql"], "rest": rec["L3"]["rest"]}}, indent=1)[:3000])
