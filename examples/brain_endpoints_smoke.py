#!/usr/bin/env python3
"""Live HTTP smoke test for the L13 brain-primitive endpoints.

Hits the 5 new endpoints on a running gigi-stream (defaults to
production at https://gigi-stream.fly.dev). Creates a small
synthetic bundle, inserts a few records, then exercises each
brain primitive and prints what comes back.

Usage:
    export GIGI_API_KEY=<key>
    python brain_endpoints_smoke.py                  # → production
    python brain_endpoints_smoke.py http://localhost:3142   # → local

The 5 endpoints exercised:
    POST /v1/bundles/{name}/brain/sample
    POST /v1/bundles/{name}/brain/confidence
    POST /v1/bundles/{name}/brain/attend
    POST /v1/bundles/{name}/brain/episodic
    GET  /v1/bundles/{name}/brain/semantic

Catalog: theory/brain_primitives/catalog.md §2, §8, §9, §10, §11, §12.
"""

from __future__ import annotations

import json
import os
import sys
import urllib.request
import urllib.error


def call(method: str, base: str, path: str, body=None, api_key: str | None = None):
    url = base.rstrip("/") + path
    data = None
    headers = {"Accept": "application/json"}
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"
    if body is not None:
        data = json.dumps(body).encode("utf-8")
        headers["Content-Type"] = "application/json"
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            return r.status, json.loads(r.read().decode("utf-8"))
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read().decode("utf-8") or "{}")
    except Exception as e:
        return 0, {"error": repr(e)}


def main(base: str) -> int:
    api_key = os.environ.get("GIGI_API_KEY")
    if not api_key:
        print("set GIGI_API_KEY env var")
        return 1

    bundle = "brain_smoke_demo"
    print(f"base: {base}")
    print(f"bundle: {bundle}")
    print()

    # ── Create the bundle (delete first if it exists). ───────────
    print("1. Setup: create bundle with 2 fiber fields (x, y)")
    call("POST", base, f"/v1/bundles/{bundle}/truncate", body={}, api_key=api_key)
    code, resp = call(
        "POST", base, "/v1/bundles",
        body={
            "name": bundle,
            "schema": {
                "base_fields": [{"name": "id", "field_type": "Numeric"}],
                "fiber_fields": [
                    {"name": "x", "field_type": "Numeric"},
                    {"name": "y", "field_type": "Numeric"},
                ],
            },
        },
        api_key=api_key,
    )
    print(f"   create_bundle -> {code}")
    if code >= 400 and "already exists" not in str(resp):
        print(f"   error: {resp}")

    # ── Insert 30 records: a clear 2-cluster pattern. ───────────
    print()
    print("2. Insert 30 records (cluster around (0,0) + cluster around (5,5))")
    records = []
    for i in range(15):
        records.append({"id": i, "x": 0.1 * (i % 5), "y": 0.1 * (i // 5)})
    for i in range(15):
        records.append({"id": 100 + i, "x": 5.0 + 0.1 * (i % 5), "y": 5.0 + 0.1 * (i // 5)})
    code, resp = call(
        "POST", base, f"/v1/bundles/{bundle}/insert",
        body={"records": records},
        api_key=api_key,
    )
    print(f"   insert -> {code} ({resp.get('inserted', resp)})")

    # ── §2 SAMPLE ──────────────────────────────────────────
    print()
    print("3. POST /brain/sample - 5 Langevin draws (seed=42)")
    code, resp = call(
        "POST", base, f"/v1/bundles/{bundle}/brain/sample",
        body={
            "fields": ["x", "y"],
            "n_samples": 5,
            "temperature": 1.0,
            "burn_in": 500,
            "seed": 42,
        },
        api_key=api_key,
    )
    print(f"   sample -> {code}")
    if code == 200:
        print(f"   fit_mean: {resp['fit_mean']}")
        print(f"   fit_sigma_sq: {resp['fit_sigma_sq']:.4f}")
        for i, s in enumerate(resp["samples"]):
            print(f"   draw[{i}]: ({s[0]:.3f}, {s[1]:.3f})")
    else:
        print(f"   {resp}")

    # ── §12 SELF-MONITOR at a known point vs a far point ──
    print()
    print("4. POST /brain/confidence - known (2.5, 2.5 between clusters) vs far (100, 100)")
    for q, label in [([2.5, 2.5], "between clusters"), ([100.0, 100.0], "far outlier")]:
        code, resp = call(
            "POST", base, f"/v1/bundles/{bundle}/brain/confidence",
            body={"fields": ["x", "y"], "query": q, "bandwidth": 1.0},
            api_key=api_key,
        )
        print(f"   query={q} ({label}) -> {code}")
        if code == 200:
            print(f"     raw={resp['raw']:.3e}  normalized={resp['normalized']:.3e}  n={resp['n_samples']}")

    # ── §8 ATTEND - query near cluster A, expect cluster A wins ──
    print()
    print("5. POST /brain/attend - query (0.2, 0.2) near cluster A; top_k=5")
    code, resp = call(
        "POST", base, f"/v1/bundles/{bundle}/brain/attend",
        body={
            "fields": ["x", "y"],
            "query": [0.2, 0.2],
            "bandwidth": 0.3,
            "top_k": 5,
        },
        api_key=api_key,
    )
    print(f"   attend -> {code}")
    if code == 200:
        for idx, w in zip(resp["indices"], resp["weights"]):
            print(f"     idx={idx:3d}  weight={w:.4f}")

    # ── §10 EPISODIC - x field, expect 1 change-point ─────
    print()
    print("6. POST /brain/episodic - x-field change-point (0->5 jump between clusters)")
    code, resp = call(
        "POST", base, f"/v1/bundles/{bundle}/brain/episodic",
        body={"field": "x", "min_persistence_ratio": 20.0},
        api_key=api_key,
    )
    print(f"   episodic -> {code}")
    if code == 200:
        print(f"   n_records: {resp['n_records']}, events: {len(resp['events'])}")
        for e in resp["events"]:
            print(f"     boundary_idx={e['boundary_idx']}  gap={e['gap']:.3f}  ratio={e['persistence_ratio']:.1f}x")

    # ── §10 EPISODIC w/ filter (L13.5) - per-cohort change-point ──
    print()
    print("6a. POST /brain/episodic with where_field filter (L13.5)")
    code, resp = call(
        "POST", base, f"/v1/bundles/{bundle}/brain/episodic",
        body={
            "field": "y",
            "min_persistence_ratio": 20.0,
            "where_field": "x",          # filter records where x ≈ 0.1
            "where_value": 0.1,
        },
        api_key=api_key,
    )
    print(f"   episodic filtered -> {code}")
    if code == 200:
        print(f"   n_records (post-filter): {resp['n_records']}, events: {len(resp['events'])}")
        if resp.get('filter_applied'):
            print(f"   filter_applied: {resp['filter_applied']}")

    # ── §11 SEMANTIC ───────────────────────────────────────
    print()
    print("7. GET /brain/semantic - Morse-compressed gist")
    code, resp = call("GET", base, f"/v1/bundles/{bundle}/brain/semantic", api_key=api_key)
    print(f"   semantic -> {code}")
    if code == 200:
        print(f"   Betti (b0, b1, b2) = ({resp['betti_b0']}, {resp['betti_b1']}, {resp['betti_b2']})")
        print(f"   critical/original = {resp['n_critical']}/{resp['n_original']}  ratio={resp['compression_ratio']:.2f}x")
        print(f"   cohomology_preserved: {resp['cohomology_preserved']}")
    elif code == 404:
        print(f"   (bundle has no Morse-compressible structure; expected for small bundles)")

    # ──────────────────────────────────────────────────────
    # PR window 3 endpoints (L13.2)
    # ──────────────────────────────────────────────────────

    # ── §4 DREAM (trajectory) ──────────────────────────────
    print()
    print("8. POST /brain/dream - high-T trajectory (T=4.0, 200 steps)")
    code, resp = call(
        "POST", base, f"/v1/bundles/{bundle}/brain/dream",
        body={
            "fields": ["x", "y"],
            "n_steps": 200,
            "temperature": 4.0,
            "seed": 42,
        },
        api_key=api_key,
    )
    print(f"   dream -> {code}")
    if code == 200:
        print(f"   trajectory length: {len(resp['trajectory'])} (= n_steps + 1)")
        print(f"   fit_mean: {resp['fit_mean']}")
        print(f"   mean dist from mean: {resp['mean_dist_from_mean']:.3f}")
        print(f"   max  dist from mean: {resp['max_dist_from_mean']:.3f}  (wandering)")

    # ── §3 FORECAST (Hamilton trajectory, no noise) ────────
    print()
    print("9. POST /brain/forecast - Hamilton flow from (3, 2)")
    code, resp = call(
        "POST", base, f"/v1/bundles/{bundle}/brain/forecast",
        body={
            "fields": ["x", "y"],
            "initial": [3.0, 2.0],
            "n_steps": 300,
        },
        api_key=api_key,
    )
    print(f"   forecast -> {code}")
    if code == 200:
        path = resp["trajectory"]
        print(f"   trajectory length: {len(path)}")
        print(f"   start: ({path[0][0]:.3f}, {path[0][1]:.3f})")
        print(f"   mid:   ({path[len(path)//2][0]:.3f}, {path[len(path)//2][1]:.3f})")
        print(f"   end:   ({path[-1][0]:.3f}, {path[-1][1]:.3f})")

    # ── §5 RECONSTRUCT (T=0 descent to MAP) ────────────────
    print()
    print("10. POST /brain/reconstruct - descent from (10, 10) toward MAP")
    code, resp = call(
        "POST", base, f"/v1/bundles/{bundle}/brain/reconstruct",
        body={
            "fields": ["x", "y"],
            "noisy_initial": [10.0, 10.0],
            "n_steps": 500,
        },
        api_key=api_key,
    )
    print(f"   reconstruct -> {code}")
    if code == 200:
        print(f"   result (= MAP): {resp['result']}")
        print(f"   fit_mean:       {resp['fit_mean']}")
        print(f"   descent_distance: {resp['descent_distance']:.3f}")

    # ── §6 INPAINT (lock x=8, sample y) ────────────────────
    print()
    print("11. POST /brain/inpaint - lock x=8, sample y from conditional")
    code, resp = call(
        "POST", base, f"/v1/bundles/{bundle}/brain/inpaint",
        body={
            "fields": ["x", "y"],
            "partial_state": [8.0, 0.0],
            "locked_indices": [0],
            "burn_in": 2000,
            "seed": 42,
        },
        api_key=api_key,
    )
    print(f"   inpaint -> {code}")
    if code == 200:
        print(f"   result: ({resp['result'][0]:.3f}, {resp['result'][1]:.3f})  (x locked at 8.0)")
        print(f"   locked_indices: {resp['locked_indices']}")

    # ── §7 PREDICT (single-step natural-gradient) ──────────
    print()
    print("12. POST /brain/predict - one step from (5, 5)")
    code, resp = call(
        "POST", base, f"/v1/bundles/{bundle}/brain/predict",
        body={
            "fields": ["x", "y"],
            "state": [5.0, 5.0],
            "lr": 0.2,
        },
        api_key=api_key,
    )
    print(f"   predict -> {code}")
    if code == 200:
        print(f"   next_state: {resp['next_state']}")
        print(f"   step_size:  {resp['step_size']:.4f}")

    # ── Cleanup ────────────────────────────────────────────
    print()
    print("13. Cleanup: drop bundle")
    code, _ = call("POST", base, f"/v1/bundles/{bundle}/truncate", body={}, api_key=api_key)
    print(f"   truncate -> {code}")

    print()
    print("Done.")
    return 0


if __name__ == "__main__":
    base = sys.argv[1] if len(sys.argv) > 1 else "https://gigi-stream.fly.dev"
    sys.exit(main(base))
