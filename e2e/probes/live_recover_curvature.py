"""RECOVER_CURVATURE against a LIVE gigi-stream (AGENTS.md section 2).

Ingests the paper's level-0 mesh (n = 1, 120 cells, 142 edges) and its Abelian
edge data over HTTP. Then it calls the verb through both surfaces and compares
every cell with the paper's Python fit (scripts/ricci_pg_tworing.py), whose
values are the answer fixed in advance. It also checks that the refusals fire.

usage: python e2e/probes/live_recover_curvature.py BASE_URL API_KEY RECEIPT.json
Refuses to overwrite RECEIPT.json. Never point it at a production engine: it
creates bundles.
"""
import json
import sys
import urllib.error
import urllib.request
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import test_recover_curvature_parity as par   # noqa: E402  (Python fit + canonical chart)

BASE, KEY, RECEIPT = sys.argv[1].rstrip("/"), sys.argv[2], Path(sys.argv[3])
if RECEIPT.exists():
    raise SystemExit(f"refusing to overwrite {RECEIPT}")


def call(method, path, body=None):
    req = urllib.request.Request(BASE + path, method=method,
                                 data=None if body is None else json.dumps(body).encode(),
                                 headers={"Content-Type": "application/json", "X-API-Key": KEY})
    try:
        with urllib.request.urlopen(req, timeout=120) as r:
            return r.status, json.loads(r.read() or b"{}")
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read() or b"{}")


level = par.pg.load_cells(0)
cells, a = level["cells"], np.asarray(level["a"], float)

cell_fields = {"cell": "integer"}
cell_fields.update({f"v{i}_{k}": "float" for i in range(4) for k in range(4)})
cell_fields.update({f"e{e}": "integer" for e in range(6)})
cell_fields.update({f"r{e}": "integer" for e in range(6)})
s1 = call("POST", "/v1/bundles", {"name": "rc_cells", "schema": {"fields": cell_fields, "keys": ["cell"]}})
s2 = call("POST", "/v1/bundles", {"name": "rc_edges",
                                  "schema": {"fields": {"edge": "integer", "a": "float"}, "keys": ["edge"]}})
print("create", s1, s2)

recs = []
for ci, c in enumerate(cells):
    r = {"cell": ci}
    for i in range(4):
        for k in range(4):
            r[f"v{i}_{k}"] = float(c["vertices"][i][k])
    for e in range(6):
        r[f"e{e}"] = int(c["edges"][e])
        r[f"r{e}"] = 1 if c["reverse"][e] else 0
    recs.append(r)
print("insert cells", call("POST", "/v1/bundles/rc_cells/insert", {"records": recs})[0])
print("insert edges", call("POST", "/v1/bundles/rc_edges/insert",
                           {"records": [{"edge": g, "a": float(v)} for g, v in enumerate(a)]})[0])

checks = {}
py = par.run_python(cells, a, 2, par.canonical_chart)

st, rest = call("POST", "/v1/bundles/rc_cells/recover_curvature", {"edges": "rc_edges", "field": "a"})
assert st == 200, (st, rest)
coef = max(float(np.max(np.abs(np.asarray(e["coefficients"]) - np.asarray(p["x"]))))
           / max(1.0, float(np.max(np.abs(p["x"])))) for e, p in zip(rest["cells"], py))
amp = max(abs(e["amplification"] - float(np.linalg.norm(p["map"], 2))) / float(np.linalg.norm(p["map"], 2))
          for e, p in zip(rest["cells"], py))
checks["rest_two_ring"] = {"status": st, "n_cells": rest["n_cells"], "coef_rel_max": coef,
                           "amplification_rel_max": amp, "faces": [rest["faces_min"], rest["faces_max"]],
                           "amplification_median": rest["amplification_median"]}
assert rest["n_cells"] == len(cells) and coef < 1e-9 and amp < 1e-9

st, gql = call("POST", "/v1/gql", {"query": "RECOVER_CURVATURE rc_cells FROM rc_edges ON a RINGS 1;"})
assert st == 200 and "rows" in gql, (st, str(gql)[:300])
faces = sorted({int(r["faces"]) for r in gql["rows"]})
checks["gql_one_ring"] = {"status": st, "rows": len(gql["rows"]), "faces": faces}
assert len(gql["rows"]) == len(cells) and faces == [16]

st, gql2 = call("POST", "/v1/gql", {"query": "RECOVER_CURVATURE rc_cells FROM rc_edges ON a;"})
by_cell = {int(r["cell"]): r for r in gql2["rows"]}
gql_vs_rest = max(abs(by_cell[e["cell"]]["amplification"] - e["amplification"]) for e in rest["cells"])
checks["gql_default_matches_rest"] = {"status": st, "max_amplification_diff": gql_vs_rest}
assert st == 200 and gql_vs_rest == 0.0

refusals = {
    "unknown_key": call("POST", "/v1/bundles/rc_cells/recover_curvature",
                        {"edges": "rc_edges", "field": "a", "ring": 2}),
    "missing_bundle": call("POST", "/v1/bundles/no_such/recover_curvature", {"edges": "rc_edges", "field": "a"}),
    "missing_field": call("POST", "/v1/bundles/rc_cells/recover_curvature", {"edges": "rc_edges", "field": "zz"}),
    "rings_out_of_range": call("POST", "/v1/bundles/rc_cells/recover_curvature",
                               {"edges": "rc_edges", "field": "a", "rings": 9}),
}
checks["refusals"] = {k: {"status": s, "body": str(b)[:200]} for k, (s, b) in refusals.items()}
assert refusals["unknown_key"][0] == 422
assert refusals["missing_bundle"][0] == 404
assert refusals["missing_field"][0] == 422 and "zz" in str(refusals["missing_field"][1])
assert refusals["rings_out_of_range"][0] == 422 and "RINGS" in str(refusals["rings_out_of_range"][1])

RECEIPT.write_text(json.dumps({"kind": "live gigi-stream run of RECOVER_CURVATURE",
                               "base_url": BASE, "mesh": "paper level 0 (n = 1)", "checks": checks},
                              indent=2) + "\n")
print(json.dumps(checks, indent=2))
print("ALL LIVE CHECKS PASSED")
