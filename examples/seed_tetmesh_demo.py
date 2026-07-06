#!/usr/bin/env python3
"""Materialize `tetmesh_demo` on a live gigi-stream engine.

Replays the tetmesh harness's exact deterministic generation path —
Kuhn/Freudenthal n=4 triangulation (384 tets) + 3 levels of naive
longest-edge bisection = 5,760 fiber records — and loads it under the
bundle name the site's public pages query (`tetmesh_demo`) instead of
the harness's local-run name (`tets`).

Record construction is `load_and_query`'s record() closure verbatim
(same first-seen classifier-cell label registry), so the corpus is
field-for-field identical to `examples/tetmesh_fiber_harness.py
--server ...` output. Receipt: 5,760 records, classifier state
{C0: 3456, C1: 768, C2: 1536} — the spec's Maubach three-class cycle
(GIGI_TETMESH_SPEC_v0.6.md, validation appendix).

Only differences from the harness's own live run: the bundle name, and
an X-API-Key header (the harness's plain http() has no auth support).
The refine-events sidecar bundle is not loaded — the site queries only
`tetmesh_demo`, and only `tetmesh_demo` is on the public allowlist.

Idempotent-ish: drops and recreates the bundle each run.

Usage:
    # the public instance (needs the write key)
    GIGI_API_KEY=... python3 examples/seed_tetmesh_demo.py \\
        --endpoint https://gigi-stream.fly.dev

    # local engine (no key)
    python3 examples/seed_tetmesh_demo.py --endpoint http://localhost:3142
"""
from __future__ import annotations

import argparse
import importlib.util
import json
import os
import urllib.request

HARNESS = os.path.join(os.path.dirname(__file__), "tetmesh_fiber_harness.py")
_spec = importlib.util.spec_from_file_location("tetmesh_fiber_harness", HARNESS)
tmh = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(tmh)

BUNDLE = "tetmesh_demo"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--endpoint", default="http://localhost:3142")
    ap.add_argument("--key", default=os.environ.get("GIGI_API_KEY"))
    args = ap.parse_args()

    def http(method, path, payload=None):
        headers = {"Content-Type": "application/json"}
        if args.key:
            headers["X-API-Key"] = args.key
        req = urllib.request.Request(
            args.endpoint.rstrip("/") + path,
            data=json.dumps(payload).encode() if payload is not None else None,
            headers=headers,
            method=method,
        )
        with urllib.request.urlopen(req) as r:
            return json.loads(r.read())

    tets = tmh.kuhn_cube_mesh(tmh.N)
    _rows, snapshots, _events, ids, roots = tmh.refinement_telemetry(tets, tmh.LEVELS)

    # -- record(), verbatim from the harness's load_and_query ------------
    cells = {}

    def record(tet, tid, lvl, root):
        f = tmh.tet_fiber(tet)
        key = tmh.classifier_key(f["alpha"], digits=5)
        label = cells.setdefault(key, f"C{len(cells)}")
        return {
            "id": tid, "level": float(lvl), "root": root,
            "classifier_cell": label,
            "min_dihedral": float(f["alpha"].min()),
            "max_dihedral": float(f["alpha"].max()),
            "q_vol": float(f["q_vol"]), "d_bad": float(f["d_bad"]),
            "volume": float(f["vol"]),
            "dihedrals": [float(x) for x in f["alpha"]],
            "edge_lengths": [float(x) for x in f["lengths"]],
            "face_normal_gram": [float(x) for x in f["A"].ravel()],
            "edge_metric_gram": [float(x) for x in f["G_edge"].ravel()],
            "psd_mode": "numeric",
            "psd_numeric_rank": float((f["eigvals"] > 1e-9).sum()),
            "psd_min_eig": float(f["eigvals"][0]),
        }

    recs = [record(t, ids[id(t)], 0, roots[id(t)]) for t in tets]
    recs += [record(t, ids[id(t)], lvl, roots[id(t)]) for t, lvl in snapshots]
    print(f"records built: {len(recs)}")
    print(f"target: {args.endpoint}")

    try:
        http("DELETE", f"/v1/bundles/{BUNDLE}")
        print(f"dropped old '{BUNDLE}'")
    except Exception:
        pass

    http("POST", "/v1/bundles", {
        "name": BUNDLE,
        "schema": {
            "fields": {
                "id": "text", "level": "float", "root": "text",
                "classifier_cell": "text", "min_dihedral": "float",
                "max_dihedral": "float", "q_vol": "float", "d_bad": "float",
                "volume": "float", "dihedrals": "vector(6)",
                "edge_lengths": "vector(6)", "face_normal_gram": "vector(16)",
                "edge_metric_gram": "vector(9)", "psd_mode": "text",
                "psd_numeric_rank": "float", "psd_min_eig": "float",
            },
            "keys": ["id"],
            "indexed": ["classifier_cell", "level", "root"],
        },
    })
    for i in range(0, len(recs), 500):
        http("POST", f"/v1/bundles/{BUNDLE}/insert", {"records": recs[i:i + 500]})
    print(f"inserted {len(recs)} tet fibers into {BUNDLE}")
    print("HEALTH:", json.dumps(http("POST", "/v1/gql", {"query": f"HEALTH {BUNDLE};"})))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
