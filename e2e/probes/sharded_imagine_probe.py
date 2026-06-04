"""Live verification of the sharded + imagine HTTP surfaces against
gigi-stream.fly.dev.

Same shape as tx_http_probe.py — contract-level probes that exercise
the deployed endpoints end-to-end:

  Sharded:
    POST /v1/bundles/{name}/sharded/spectral_gap
    POST /v1/bundles/{name}/sharded/curvature
    POST /v1/bundles/{name}/sharded/holonomy_loop

  Imagine:
    POST /v1/bundles/{name}/imagine_coherence

The probes use a fresh bundle seeded with deterministic numeric +
vector data so the responses are reproducible. Each check carries
exactly what was sent and got back; misses surface the raw error
body for fast triage.
"""
import http.client
import json
import os
import ssl
import sys
import time

HOST = "gigi-stream.fly.dev"
API_KEY = os.environ.get("GIGI_API_KEY", "")
if not API_KEY:
    print("[fatal] GIGI_API_KEY must be set", file=sys.stderr)
    sys.exit(2)
HEADERS = {"Content-Type": "application/json", "X-API-Key": API_KEY}
ctx = ssl.create_default_context()


def call(method, path, body=None):
    c = http.client.HTTPSConnection(HOST, 443, timeout=60, context=ctx)
    c.request(method, path, json.dumps(body) if body is not None else "", HEADERS)
    r = c.getresponse()
    data = r.read()
    try:
        j = json.loads(data) if data else {}
    except Exception:
        j = {"_raw": data[:300].decode("utf-8", errors="replace")}
    return r.status, j


results = []
fails = []


def check(label, ok, detail=""):
    tag = "PASS" if ok else "MISS"
    print(f"  [{tag}] {label}" + (f"  -- {detail}" if detail else ""))
    results.append((label, ok))
    if not ok:
        fails.append((label, detail))


def section(title):
    print(f"\n[{title}]")


print("=" * 72)
print(f"LIVE SHARDED + IMAGINE PROBE: {HOST}")
print("=" * 72)

ts = int(time.time())
bundle = f"si_probe_{ts}"

# Setup: bundle with vector(4) fiber + scalar fields, populated with
# enough records for spectral_gap (>=2) and curvature stats to be
# meaningful (~150 records).
section("setup")
s, _ = call(
    "POST",
    "/v1/bundles",
    {
        "name": bundle,
        "schema": {
            "fields": {
                "id": "numeric",
                "color": "categorical",
                "price": "numeric",
                "qty": "numeric",
                "emb": "vector(4)",
            },
            "keys": ["id"],
        },
    },
)
check(f"create {bundle}", s in (200, 201), f"status={s}")

records = [
    {
        "id": i,
        "color": ["red", "blue", "green"][i % 3],
        "price": 100.0 + i * 5.0,
        "qty": float(i),
        "emb": [float(i % 4) + 0.1, float(i % 3) + 0.2, float(i % 5) + 0.3, float(i % 7) + 0.4],
    }
    for i in range(1, 151)
]
s, body = call("POST", f"/v1/bundles/{bundle}/insert", {"records": records})
check(
    f"insert 150 records (K={body.get('curvature'):.4f})",
    s == 200 and body.get("count") == 150,
    f"status={s}",
)

# ─────────────────────────────────────────────────────────────────────────────
# Sharded endpoints
# ─────────────────────────────────────────────────────────────────────────────
section("S1 sharded/spectral_gap default (k=8, k_max=120)")
s, body = call("POST", f"/v1/bundles/{bundle}/sharded/spectral_gap", {})
check(
    "spectral_gap returns lambda_1 >= 0",
    s == 200 and isinstance(body.get("lambda_1"), (int, float)) and body["lambda_1"] >= 0,
    f"status={s}, body={json.dumps(body)[:300]}",
)

section("S2 sharded/spectral_gap with k_neighbors=4")
s, body = call(
    "POST",
    f"/v1/bundles/{bundle}/sharded/spectral_gap",
    {"k_neighbors": 4, "k_max": 60},
)
check(
    "spectral_gap accepts k_neighbors override",
    s == 200 and body.get("k_neighbors") == 4,
    f"status={s}, body={json.dumps(body)[:300]}",
)

section("S3 sharded/curvature (trivial atlas, n_charts=1)")
s, body = call("POST", f"/v1/bundles/{bundle}/sharded/curvature", {"n_charts": 1})
check(
    "sharded curvature n=1 matches unsharded shape",
    s == 200
    and body.get("n_charts") == 1
    and body.get("n_records") == 150
    and isinstance(body.get("mean_k"), (int, float)),
    f"status={s}, body={json.dumps(body)[:300]}",
)

section("S4 sharded/curvature with n_charts=4")
s, body = call("POST", f"/v1/bundles/{bundle}/sharded/curvature", {"n_charts": 4})
check(
    "sharded curvature n=4 splits the bundle",
    s == 200 and body.get("n_charts") == 4 and body.get("n_records") == 150,
    f"status={s}, body={json.dumps(body)[:300]}",
)

section("S5 sharded/holonomy_loop trivial (identity transitions, expect H=I)")
# Path: triangle on the trivial single-chart atlas (ChartId 0).
trivial_loop = {
    "path": [
        [0, [0.0, 0.0]],
        [0, [1.0, 0.0]],
        [0, [0.5, 1.0]],
    ],
    "transitions": [],
}
s, body = call("POST", f"/v1/bundles/{bundle}/sharded/holonomy_loop", trivial_loop)
check(
    "trivial loop H is the 2x2 identity (det=1, no flip)",
    s == 200 and abs(body.get("det", 0) - 1.0) < 1e-9 and body.get("orientation_flipped") is False,
    f"status={s}, body={json.dumps(body)[:300]}",
)

section("S6 sharded/holonomy_loop Möbius (det = -1, orientation flip)")
# Single transition on the loop closure that reflects: [-1, 0, 0, 1] = det -1
mobius_loop = {
    "path": [
        [0, [0.0, 0.0]],
        [1, [1.0, 0.0]],
    ],
    "transitions": [
        [0, 1, [1.0, 0.0, 0.0, 1.0]],
        [1, 0, [-1.0, 0.0, 0.0, 1.0]],
    ],
}
s, body = call("POST", f"/v1/bundles/{bundle}/sharded/holonomy_loop", mobius_loop)
check(
    "Möbius loop H has det=-1 and orientation_flipped=true",
    s == 200 and abs(body.get("det", 0) - (-1.0)) < 1e-9 and body.get("orientation_flipped") is True,
    f"status={s}, body={json.dumps(body)[:300]}",
)

section("S7 sharded/holonomy_loop empty path returns 400")
s, body = call("POST", f"/v1/bundles/{bundle}/sharded/holonomy_loop", {"path": []})
check("empty path refused (400)", s == 400, f"status={s}, error={body.get('error')}")

# ─────────────────────────────────────────────────────────────────────────────
# Imagine endpoint
# ─────────────────────────────────────────────────────────────────────────────
section("I1 imagine_coherence happy path (low-K override, default budget)")
# The bundle's K is high enough (~0.5+) that the default holonomy budget
# of 0.5 trips at step 1. Override metric_curvature to 0.01 so the
# integrator stays well inside the safety envelope and returns a full
# trajectory — the canonical "I want to project forward in a calm
# region" call.
imagine_req = {
    "starting_from": [0.0, 0.0],
    "along": [1.0, 0.0],
    "steps": 3,
    "metric_curvature": 0.01,
}
s, body = call("POST", f"/v1/bundles/{bundle}/imagine_coherence", imagine_req)
check(
    "imagine_coherence happy path returns a trajectory[]",
    s == 200 and isinstance(body.get("trajectory"), list) and len(body["trajectory"]) == 4,
    f"status={s}, len(trajectory)={len(body.get('trajectory', []))}",
)
if s == 200:
    print(f"      keys: {sorted(body.keys())}")
    print(f"      step[0]: {body['trajectory'][0]}")

section("I2 imagine_coherence provenance carries is_imagined audit")
# Marcella round-3 feedback #1: every imagined trajectory point must
# carry a provenance string identifying it as imagined, not measured.
s, body = call(
    "POST",
    f"/v1/bundles/{bundle}/imagine_coherence",
    {
        "starting_from": [0.0, 0.0],
        "along": [1.0, 0.0],
        "steps": 3,
        "metric_curvature": 0.01,
    },
)
all_provenances = [p.get("provenance", "") for p in body.get("trajectory", [])]
all_imagined = all("imagined" in p.lower() for p in all_provenances)
check(
    "every trajectory step carries 'imagined' provenance",
    s == 200 and all_imagined and len(all_provenances) > 0,
    f"provenances={all_provenances}",
)

section("I3 imagine_coherence honest refusal (default budget on high-K bundle)")
# Default holonomy budget is 0.5; the bundle's K_global is high enough
# that even 1 step bursts it. The endpoint MUST refuse with 422 and an
# error string identifying which step burst what budget. This is the
# Marcella refuse-gate per round-3 feedback #2.
s, body = call(
    "POST",
    f"/v1/bundles/{bundle}/imagine_coherence",
    {"starting_from": [0.0, 0.0], "along": [1.0, 0.0], "steps": 3},
)
err_str = body.get("error", "") if isinstance(body, dict) else ""
check(
    "default-budget call refuses 422 with 'accumulated holonomy' in error",
    s == 422 and "holonomy" in err_str.lower() and "budget" in err_str.lower(),
    f"status={s}, error={err_str[:160]}",
)

section("I4 imagine_coherence relaxed budget on same bundle -> trajectory")
# Bump the budget high enough to clear the cap; verify trajectory comes
# back populated.
s, body = call(
    "POST",
    f"/v1/bundles/{bundle}/imagine_coherence",
    {
        "starting_from": [0.0, 0.0],
        "along": [1.0, 0.0],
        "steps": 5,
        "max_accumulated_holonomy": 10.0,
    },
)
check(
    "relaxed budget -> 5+1 trajectory points returned",
    s == 200
    and len(body.get("trajectory", [])) == 6
    and body.get("max_accumulated_holonomy") == 10.0,
    f"status={s}, len={len(body.get('trajectory', []))}, max_h={body.get('max_accumulated_holonomy')}",
)

section("I5 imagine_coherence routing advisory (grounding=0.3, low K)")
# Marcella round-3 feedback #3: when query_grounding_normalized is in
# the body, the response carries a routing_advisory block. Use the
# low-K override so we get to a 200 instead of a refusal.
s, body = call(
    "POST",
    f"/v1/bundles/{bundle}/imagine_coherence",
    {
        "starting_from": [0.0, 0.0],
        "along": [0.7071, 0.7071],
        "steps": 3,
        "metric_curvature": 0.01,
        "query_grounding_normalized": 0.3,
    },
)
adv = body.get("routing_advisory") if isinstance(body, dict) else None
check(
    "grounding=0.3 -> routing_advisory present",
    s == 200 and adv is not None,
    f"status={s}, advisory={adv}",
)

section("I6 imagine_coherence dim mismatch -> 400")
# The endpoint's Phase-1 supports dim=2 only; sending dim=1 must return
# a clean 400 with an explicit message, not a 500.
s, body = call(
    "POST",
    f"/v1/bundles/{bundle}/imagine_coherence",
    {"starting_from": [0.0], "along": [1.0]},
)
err = body.get("error", "") if isinstance(body, dict) else ""
check(
    "dim=1 input -> 400 with explicit 'dim' message",
    s == 400 and "dim" in err.lower(),
    f"status={s}, error={err[:120]}",
)

# ─────────────────────────────────────────────────────────────────────────────
# Cleanup
# ─────────────────────────────────────────────────────────────────────────────
section("cleanup")
s, _ = call("DELETE", f"/v1/bundles/{bundle}")
check(f"DELETE {bundle}", s in (200, 204, 404), f"status={s}")

# ─────────────────────────────────────────────────────────────────────────────
print("\n" + "=" * 72)
passed = sum(1 for _, ok in results if ok)
total = len(results)
print(f"PASSED: {passed}/{total}")
if fails:
    print("\nMISSES:")
    for label, detail in fails:
        print(f"  - {label}")
        print(f"      {detail}")
    sys.exit(1)
print("\nAll sharded + imagine probes green.")
sys.exit(0)
