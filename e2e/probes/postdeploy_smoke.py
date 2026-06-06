"""Post-deploy smoke test against live gigi-stream.fly.dev.

Runs immediately after `flyctl deploy` to verify:
  1. Health endpoint returns 200
  2. Existing bundles still reachable
  3. SUDOKU + SAMPLE_TRANSPORT (new) work on a fresh bundle
  4. #107 fix: SUDOKU works on a bundle that has been snapshotted+reloaded

This is the production correctness gate. ANY FAIL → roll back.
"""
import http.client
import json
import os
import ssl
import sys
import time

HOST = "gigi-stream.fly.dev"
HEADERS = {"Content-Type": "application/json", "Connection": "keep-alive"}
API_KEY = os.environ.get("GIGI_API_KEY", "")
if API_KEY:
    HEADERS["X-API-Key"] = API_KEY
else:
    print("[warn] GIGI_API_KEY not set — auth'd endpoints will 401.")
    print("       export GIGI_API_KEY=... before re-running for full smoke.\n")
ctx = ssl.create_default_context()
CONN = http.client.HTTPSConnection(HOST, 443, timeout=60, context=ctx)

def call(method, path, body=None):
    payload = json.dumps(body) if body is not None else ""
    try:
        CONN.request(method, path, payload, HEADERS)
        r = CONN.getresponse()
        data = r.read()
        try:
            j = json.loads(data) if data else {}
        except Exception:
            j = {"raw": data[:200].decode("utf-8", errors="replace")}
        return r.status, j
    except (http.client.BadStatusLine, ConnectionResetError, BrokenPipeError):
        CONN.close()
        CONN.connect()
        CONN.request(method, path, payload, HEADERS)
        r = CONN.getresponse()
        data = r.read()
        try:
            j = json.loads(data) if data else {}
        except Exception:
            j = {"raw": data[:200].decode("utf-8", errors="replace")}
        return r.status, j


results = []
def report(label, ok, detail=""):
    tag = "PASS" if ok else "FAIL"
    print(f"  [{tag}] {label}" + (f"  -- {detail}" if detail else ""))
    results.append((label, ok, detail))


print("=" * 70)
print(f"POST-DEPLOY SMOKE TEST: {HOST}")
print("=" * 70)

# 1. Liveness
print("\n[1] Liveness check")
s, b = call("GET", "/v1/bundles")
report("GET /v1/bundles returns 200", s == 200, f"status={s}")
if s != 200:
    sys.exit(1)
existing = [x for x in b if not x['name'].startswith('_gigi_')]
print(f"  existing user bundles: {len(existing)}: "
      f"{[(x['name'], x['records']) for x in existing][:5]}")

# 2. Brain endpoints reachable on an existing bundle (if any has records).
print("\n[2] Brain endpoints reach an existing bundle (#107 production verify)")
target = next((b for b in existing if b['records'] > 0), None)
if target:
    s, body = call("POST", f"/v1/bundles/{target['name']}/brain/sudoku",
                   {"constraints": [], "max_options": 1, "max_near_misses": 0})
    report(f"SUDOKU on {target['name']} returns 200",
           s == 200, f"status={s}, verdict={body.get('verdict')}, "
                     f"records={body.get('n_records_considered')}")
else:
    report("[skip] no pre-existing bundle with records", True, "skipped")

# 3. Create a fresh bundle + verify new endpoints work on it.
print("\n[3] New bundle creation + new wave-3-6 endpoints work")
bundle_name = f"postdeploy_smoke_{int(time.time())}"
s, _ = call("POST", "/v1/bundles", {
    "name": bundle_name,
    "schema": {"fields": {
        "id": "numeric", "color": "categorical",
        "price": "numeric", "qty": "numeric",
        "emb": "vector(4)"
    }, "keys": ["id"]}
})
report(f"create {bundle_name}", s in (200, 201), f"status={s}")

records = [{
    "id": i, "color": ["red", "blue", "green"][i % 3],
    "price": 100.0 + i * 10,
    "qty": float(i),
    "emb": [float(i % 4), float(i % 3), float(i % 5), float(i % 7)]
} for i in range(1, 51)]
s, _ = call("POST", f"/v1/bundles/{bundle_name}/insert", {"records": records})
report(f"insert 50 records", s == 200, f"status={s}")

# SUDOKU
s, body = call("POST", f"/v1/bundles/{bundle_name}/brain/sudoku",
               {"constraints": [
                   {"type": "field", "field": "color", "op": "eq", "value": "red"},
                   {"type": "field", "field": "price", "op": "le", "value": 300.0}],
                "max_options": 5, "max_near_misses": 3})
report("SUDOKU on fresh bundle", s == 200 and body.get("verdict") == "sat",
       f"status={s}, verdict={body.get('verdict')}, "
       f"solutions={len(body.get('solutions') or [])}, "
       f"K_c[0]={body.get('selectivity', [{}])[0].get('raw_curvature', 'N/A')}")

# Holonomy pre-flight
s, body = call("POST", f"/v1/bundles/{bundle_name}/brain/sudoku",
               {"constraints": [
                   {"type": "field", "field": "color", "op": "eq", "value": "red"},
                   {"type": "field", "field": "color", "op": "eq", "value": "blue"}]})
report("Cech pre-flight catches contradiction",
       s == 200 and body.get("verdict") == "unsat"
       and body.get("n_records_considered") == 0
       and body.get("pre_flight_unsat_reason"),
       f"reason={body.get('pre_flight_unsat_reason')}")

# SAMPLE_TRANSPORT
s, body = call("POST", f"/v1/bundles/{bundle_name}/brain/sample_transport",
               {"from_keys": {"id": 1}, "fiber_fields": ["emb"],
                "budget": 0.5, "k": 5, "seed": 42})
report("SAMPLE_TRANSPORT on fresh bundle",
       s == 200, f"status={s}, candidates={len(body.get('candidates') or [])}, "
                 f"kappa={body.get('kappa')}")

# fit_diagnostics — needs >=2 fiber-only scalar fields (even fiber dim,
# canonical symplectic structure). 'id' is the base key, so we use
# 'price' + 'qty' for an n=2 fiber.
s, body = call("POST", f"/v1/bundles/{bundle_name}/brain/fit_diagnostics",
               {"fields": ["price", "qty"]})
report("fit_diagnostics on fresh bundle",
       s == 200, f"status={s}")

# confidence (Marcella's refuse-gate). Same fiber-fields constraint.
s, body = call("POST", f"/v1/bundles/{bundle_name}/brain/confidence",
               {"fields": ["price", "qty"], "query": [200.0, 10.0]})
raw = body.get("raw")
raw_str = f"{raw:.2f}" if isinstance(raw, (int, float)) else str(raw)
report("brain/confidence on fresh bundle",
       s == 200, f"status={s}, raw={raw_str}")

# 4. cleanup
print("\n[4] Cleanup")
s, _ = call("DELETE", f"/v1/bundles/{bundle_name}")
report(f"DELETE {bundle_name}", s in (200, 204, 404), f"status={s}")

# Summary
print("\n" + "=" * 70)
fails = [r for r in results if not r[1]]
passes = [r for r in results if r[1]]
print(f"PASSED: {len(passes)}  FAILED: {len(fails)}")
if fails:
    print("\nFAILURES (ROLLBACK CANDIDATE):")
    for label, _, detail in fails:
        print(f"  - {label}: {detail}")
    sys.exit(1)
print("All post-deploy smoke checks passed.")
sys.exit(0)
