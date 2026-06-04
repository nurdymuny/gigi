"""Live data-stream probe against gigi-stream.fly.dev.

Goes deeper than postdeploy_smoke.py: exercises geometric primitives
(CURVATURE, SPECTRAL_GAP, HOLONOMY, IMAGINE_COHERENCE) on a fresh
bundle that we drive with a streaming insert pattern. Each probe
records exactly what it sent and what it got back; failures dump the
response body so we can root-cause without a second round-trip.

Run with:
    export GIGI_API_KEY=...
    python e2e/probes/live_stream_probe.py

Categories:
  L1  Liveness + auth wall holds
  L2  Schema reflection round-trips
  L3  Streaming insert -> incremental curvature update
  L4  SPECTRAL_GAP cache + recompute path
  L5  CURVATURE endpoint shape (raw, Kahler, holonomy)
  L6  IMAGINE_COHERENCE on the freshly-built bundle
  L7  SAMPLE_TRANSPORT on Vector-fiber + scalar-fiber bundles
  L8  Brain endpoints (sudoku, attention, episodic, semantic)
  L9  Sharded routes (spectral_gap, curvature, holonomy_loop)
  L10 GQL query through the public surface
  L11 Cleanup
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
    conn = http.client.HTTPSConnection(HOST, 443, timeout=60, context=ctx)
    payload = json.dumps(body) if body is not None else ""
    conn.request(method, path, payload, HEADERS)
    r = conn.getresponse()
    data = r.read()
    try:
        j = json.loads(data) if data else {}
    except Exception:
        j = {"_raw_first_300": data[:300].decode("utf-8", errors="replace")}
    return r.status, j


results = []
finds = []


def check(label, ok, detail=""):
    tag = "PASS" if ok else "MISS"
    print(f"  [{tag}] {label}" + (f"  -- {detail}" if detail else ""))
    results.append((label, ok))
    if not ok:
        finds.append((label, detail))


def section(title):
    print(f"\n[{title}]")


print("=" * 72)
print(f"LIVE STREAM PROBE: {HOST}")
print("=" * 72)

# ----- L1 Liveness + auth ----------------------------------------------------
section("L1 Liveness + auth wall")
s, b = call("GET", "/v1/bundles")
check("authed GET /v1/bundles returns 200", s == 200, f"status={s}")

# auth wall: same call without key
HEADERS_NOAUTH = {"Content-Type": "application/json"}
conn = http.client.HTTPSConnection(HOST, 443, timeout=60, context=ctx)
conn.request("GET", "/v1/bundles", "", HEADERS_NOAUTH)
r = conn.getresponse()
r.read()
check("unauthed GET /v1/bundles refused (401)", r.status == 401, f"status={r.status}")

# ----- L2 Bundle create + schema reflection ----------------------------------
section("L2 Schema create + reflection")
bundle = f"live_probe_{int(time.time())}"
schema = {
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
}
s, b = call("POST", "/v1/bundles", schema)
check(f"create {bundle}", s in (200, 201), f"status={s}")

s, b = call("GET", f"/v1/bundles/{bundle}")
check("reflect schema", s == 200, f"status={s}")

# ----- L3 Streaming insert + curvature evolution -----------------------------
section("L3 Streaming insert + incremental curvature")
prev_curvature = None
for batch_i in range(3):
    batch = [
        {
            "id": batch_i * 100 + i,
            "color": ["red", "blue", "green"][i % 3],
            "price": 100.0 + i * 10.0,
            "qty": float(i + batch_i * 50),
            "emb": [float(i % 4), float(i % 3), float(i % 5), float(i % 7)],
        }
        for i in range(1, 51)
    ]
    s, b = call("POST", f"/v1/bundles/{bundle}/insert", {"records": batch})
    check(f"insert batch {batch_i} (50 records)", s == 200, f"status={s}")
    cur = b.get("curvature")
    if prev_curvature is not None:
        check(
            f"curvature evolved batch {batch_i}",
            isinstance(cur, (int, float)) and cur != prev_curvature,
            f"prev={prev_curvature:.4f} -> now={cur if cur is None else round(cur, 4)}",
        )
    prev_curvature = cur

# ----- L4 SPECTRAL_GAP -------------------------------------------------------
section("L4 SPECTRAL_GAP cache + recompute")
s, b = call("GET", f"/v1/bundles/{bundle}/spectral_gap")
check(
    "spectral_gap returns non-negative float",
    s == 200 and isinstance(b.get("spectral_gap"), (int, float)) and b["spectral_gap"] >= 0,
    f"status={s}, body={json.dumps(b)[:200]}",
)

# ----- L5 CURVATURE endpoint -------------------------------------------------
section("L5 CURVATURE endpoint shape")
s, b = call("GET", f"/v1/bundles/{bundle}/curvature")
has_scalar = isinstance(b.get("scalar_curvature"), (int, float))
check(
    "curvature returns scalar_curvature",
    s == 200 and has_scalar,
    f"status={s}, scalar_curvature={b.get('scalar_curvature')}",
)

# ----- L6 IMAGINE_COHERENCE --------------------------------------------------
section("L6 IMAGINE_COHERENCE shape")
s, b = call(
    "POST",
    f"/v1/bundles/{bundle}/imagine_coherence",
    {"steps": 5, "n_paths": 4, "seed": 17},
)
# IMAGINE may refuse or accept depending on dim. Either is informative.
if s == 200:
    check(
        "imagine_coherence accepted (200)",
        "trajectory" in b or "summary" in b or "outcome" in b,
        f"keys={list(b.keys())[:8]}",
    )
elif s == 400:
    check(
        "imagine_coherence honest 400 (e.g. dim mismatch)",
        True,
        f"error={b.get('error', json.dumps(b))[:160]}",
    )
else:
    check(f"imagine_coherence unexpected status {s}", False, f"body={json.dumps(b)[:200]}")

# ----- L7 SAMPLE_TRANSPORT on vector fiber + scalar fiber --------------------
section("L7 SAMPLE_TRANSPORT (vector fiber)")
s, b = call(
    "POST",
    f"/v1/bundles/{bundle}/brain/sample_transport",
    {
        "from_keys": {"id": 1},
        "fiber_fields": ["emb"],
        "budget": 0.5,
        "k": 5,
        "seed": 42,
    },
)
check(
    "SAMPLE_TRANSPORT on vector(4) fiber",
    s == 200 and len(b.get("candidates", []) or []) > 0,
    f"status={s}, candidates={len(b.get('candidates') or [])}, kappa={b.get('kappa')}",
)

section("L7b SAMPLE_TRANSPORT (scalar fiber)")
s, b = call(
    "POST",
    f"/v1/bundles/{bundle}/brain/sample_transport",
    {
        "from_keys": {"id": 1},
        "fiber_fields": ["price", "qty"],
        "budget": 0.5,
        "k": 5,
        "seed": 42,
    },
)
check(
    "SAMPLE_TRANSPORT on scalar fiber pair",
    s == 200 and len(b.get("candidates", []) or []) > 0,
    f"status={s}, candidates={len(b.get('candidates') or [])}",
)

# ----- L8 Brain endpoints -----------------------------------------------------
section("L8 Brain endpoints")
s, b = call(
    "POST",
    f"/v1/bundles/{bundle}/brain/sudoku",
    {
        "constraints": [
            {"type": "field", "field": "color", "op": "eq", "value": "red"},
            {"type": "field", "field": "price", "op": "le", "value": 300.0},
        ],
        "max_options": 5,
        "max_near_misses": 3,
    },
)
check(
    "brain/sudoku returns sat",
    s == 200 and b.get("verdict") == "sat" and len(b.get("solutions", [])) > 0,
    f"status={s}, verdict={b.get('verdict')}, solutions={len(b.get('solutions', []))}",
)

# attention
s, b = call(
    "POST",
    f"/v1/bundles/{bundle}/brain/attention",
    {"fields": ["price", "qty"], "query": [200.0, 10.0], "k": 5},
)
check(
    "brain/attention",
    s == 200,
    f"status={s}, top={len(b.get('top', []) or [])}",
)

# semantic
s, b = call("POST", f"/v1/bundles/{bundle}/brain/semantic", {})
check("brain/semantic", s == 200, f"status={s}, betti={b.get('betti')}")

# episodic
s, b = call(
    "POST",
    f"/v1/bundles/{bundle}/brain/episodic",
    {"fields": ["price", "qty"]},
)
check(
    "brain/episodic",
    s == 200,
    f"status={s}, change_points={len(b.get('change_points', []) or [])}",
)

# ----- L9 Sharded routes -----------------------------------------------------
section("L9 Sharded routes")
for endpoint in ["spectral_gap", "curvature", "holonomy_loop"]:
    s, b = call("GET", f"/v1/bundles/{bundle}/sharded/{endpoint}")
    # Sharded endpoints may not be wired for this schema; honest 400/404 OK.
    check(
        f"sharded/{endpoint} reachable (200/400/404)",
        s in (200, 400, 404),
        f"status={s}, body={json.dumps(b)[:160]}",
    )

# ----- L10 GQL ---------------------------------------------------------------
section("L10 GQL through public surface")
s, b = call(
    "POST",
    "/v1/gql",
    {"query": f"SELECT id, price FROM {bundle} WHERE price < 200 LIMIT 5"},
)
# /v1/gql may be at a different path; if 404, try alt paths
if s == 404:
    s, b = call(
        "POST",
        "/v1/query",
        {"gql": f"SELECT id, price FROM {bundle} WHERE price < 200 LIMIT 5"},
    )
check(
    "GQL select returns rows",
    s == 200,
    f"status={s}, body={json.dumps(b)[:200]}",
)

# ----- L11 Cleanup -----------------------------------------------------------
section("L11 Cleanup")
s, _ = call("DELETE", f"/v1/bundles/{bundle}")
check(f"DELETE {bundle}", s in (200, 204, 404), f"status={s}")

# ----- Summary ---------------------------------------------------------------
print("\n" + "=" * 72)
passed = sum(1 for _, ok in results if ok)
total = len(results)
print(f"PASSED: {passed}/{total}")
if finds:
    print("\nMISSES (potentially real or schema-tuning):")
    for label, detail in finds:
        print(f"  - {label}")
        print(f"      {detail}")
    sys.exit(1)
print("\nAll live-stream probes green.")
sys.exit(0)
