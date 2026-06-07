"""Live probe for the Patterns v0.2 verdict envelope on gigi-stream.fly.dev.

Tests the new HUNT response shape when v0.2 flags are present:
  - verdict: sat | near_miss | unsat
  - n_matches / near_miss_count / reason
  - _explain attached when explain=true
  - _repair_menu attached when include_repair_menu=true
  - v0.1 backwards-compatible array shape when no v0.2 flags

Auth: requires GIGI_API_KEY env var.
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
    print("[fatal] GIGI_API_KEY not set")
    sys.exit(2)

HEADERS = {
    "Content-Type": "application/json",
    "X-API-Key": API_KEY,
    "Connection": "keep-alive",
}
CTX = ssl.create_default_context()
CONN = http.client.HTTPSConnection(HOST, 443, timeout=60, context=CTX)


def call(method, path, body=None):
    payload = json.dumps(body) if body is not None else ""
    try:
        CONN.request(method, path, payload, HEADERS)
        r = CONN.getresponse()
        data = r.read()
    except (http.client.BadStatusLine, ConnectionResetError, BrokenPipeError):
        CONN.close()
        CONN.connect()
        CONN.request(method, path, payload, HEADERS)
        r = CONN.getresponse()
        data = r.read()
    try:
        return r.status, json.loads(data) if data else {}
    except Exception:
        return r.status, {"raw": data[:200].decode("utf-8", errors="replace")}


PASSED = 0
FAILED = 0


def check(label, ok, detail=""):
    global PASSED, FAILED
    tag = "PASS" if ok else "FAIL"
    print(f"  [{tag}] {label}" + (f"  -- {detail}" if detail else ""))
    if ok:
        PASSED += 1
    else:
        FAILED += 1


print("=" * 72)
print("PATTERNS v0.2 LIVE PROBE: gigi-stream.fly.dev")
print("=" * 72)

ts = int(time.time())
bn = f"scj_v02_probe_{ts}"
pname = f"v02_p_{ts}"
created_b = []
created_p = []

try:
    # ─── Setup ──────────────────────────────────────────────────────────
    s, _ = call("POST", "/v1/bundles", {
        "name": bn,
        "schema": {
            "fields": {"id": "numeric", "a": "numeric", "x": "numeric"},
            "keys": ["id"],
        },
    })
    check(f"create bundle {bn}", s in (200, 201), f"status={s}")
    if s in (200, 201):
        created_b.append(bn)

    # 5 rows, all with (a=1, x=1) — perfect for near-miss tests against (a=1, x=0)
    records = [{"id": i, "a": 1, "x": 1} for i in range(1, 6)]
    s, _ = call("POST", f"/v1/bundles/{bn}/insert", {"records": records})
    check("insert 5 rows (a=1, x=1)", s == 200, f"status={s}")

    # Pattern with multi-field predicate: a=1 AND x=0. All 5 rows are
    # 1 flip from matching — perfect near-miss bait.
    s, body = call("POST", "/v1/patterns", {
        "name": pname,
        "predicate": "a = 1 AND x = 0",
        "weight": "a + x",
        "using": ["a", "x"],
    })
    check("DEFINE PATTERN multi-field", s in (200, 201), f"status={s}, body={body}")
    if s in (200, 201):
        created_p.append(pname)

    # ─── §1: v0.1 backwards compat — no v0.2 flags → bare array ─────────
    s, body = call("POST", f"/v1/bundles/{bn}/hunt", {"pattern": pname})
    check("v0.1 backcompat — bare array when no v0.2 flags",
          s == 200 and isinstance(body, list),
          f"status={s}, body type={type(body).__name__}")
    check("v0.1 backcompat — empty (no rows match strictly)",
          isinstance(body, list) and len(body) == 0,
          f"got {len(body) if isinstance(body, list) else '?'} rows")

    # ─── §2: v0.2 envelope — near_miss verdict ───────────────────────────
    s, body = call("POST", f"/v1/bundles/{bn}/hunt", {
        "pattern": pname,
        "near_miss_budget": 1,
    })
    check("v0.2 envelope returns 200", s == 200, f"status={s}")
    check("envelope carries verdict field",
          isinstance(body, dict) and "verdict" in body,
          f"keys: {list(body.keys()) if isinstance(body, dict) else '?'}")
    if isinstance(body, dict):
        check("verdict == 'near_miss'", body.get("verdict") == "near_miss",
              f"verdict={body.get('verdict')}")
        check("near_miss_count == 5", body.get("near_miss_count") == 5,
              f"count={body.get('near_miss_count')}")
        check("near_miss_rows is an array of 5",
              isinstance(body.get("near_miss_rows"), list)
              and len(body.get("near_miss_rows", [])) == 5,
              f"got {len(body.get('near_miss_rows', []))}")

    # ─── §3: v0.2 envelope — sat verdict (pattern that matches) ──────────
    # Define a sat pattern: a >= 0 (every row matches)
    sat_pname = f"v02_sat_{ts}"
    s, _ = call("POST", "/v1/patterns", {
        "name": sat_pname,
        "predicate": "a >= 0",
        "weight": "a * 3 + x * 2",
        "using": ["a", "x"],
    })
    check("DEFINE sat pattern", s in (200, 201), f"status={s}")
    if s in (200, 201):
        created_p.append(sat_pname)

    s, body = call("POST", f"/v1/bundles/{bn}/hunt", {
        "pattern": sat_pname,
        "near_miss_budget": 1,
        "explain": True,
    })
    check("sat envelope returns 200", s == 200, f"status={s}")
    if isinstance(body, dict):
        check("verdict == 'sat'", body.get("verdict") == "sat",
              f"verdict={body.get('verdict')}")
        check("n_matches == 5", body.get("n_matches") == 5,
              f"got {body.get('n_matches')}")
        check("rows is an array of 5",
              isinstance(body.get("rows"), list) and len(body.get("rows", [])) == 5,
              f"got {len(body.get('rows', []))}")
        if isinstance(body.get("rows"), list) and body.get("rows"):
            check("each sat row carries _explain",
                  all("_explain" in row for row in body["rows"]),
                  f"rows: {[list(r.keys()) for r in body['rows'][:1]]}")

    # ─── §4: v0.2 envelope — unsat verdict at budget=0 ───────────────────
    unsat_pname = f"v02_unsat_{ts}"
    s, _ = call("POST", "/v1/patterns", {
        "name": unsat_pname,
        "predicate": "a >= 999",
        "weight": "a",
        "using": ["a"],
    })
    check("DEFINE unsat pattern", s in (200, 201))
    if s in (200, 201):
        created_p.append(unsat_pname)

    s, body = call("POST", f"/v1/bundles/{bn}/hunt", {
        "pattern": unsat_pname,
        "near_miss_budget": 0,
    })
    check("unsat envelope returns 200", s == 200, f"status={s}")
    if isinstance(body, dict):
        check("verdict == 'unsat'", body.get("verdict") == "unsat",
              f"verdict={body.get('verdict')}")
        check("preflight_caught == true", body.get("preflight_caught") is True,
              f"preflight_caught={body.get('preflight_caught')}")
        check("reason field present",
              isinstance(body.get("reason"), str),
              f"reason={body.get('reason')}")

    # ─── §5: repair_menu attaches to near-miss rows ──────────────────────
    s, body = call("POST", f"/v1/bundles/{bn}/hunt", {
        "pattern": pname,
        "near_miss_budget": 1,
        "include_repair_menu": True,
    })
    if isinstance(body, dict) and isinstance(body.get("near_miss_rows"), list):
        rows = body["near_miss_rows"]
        check("near-miss rows carry _repair_menu",
              rows and all("_repair_menu" in r for r in rows),
              f"keys per row: {[list(r.keys()) for r in rows[:1]]}")

finally:
    for p in created_p:
        s, _ = call("DELETE", f"/v1/patterns/{p}")
        check(f"DELETE pattern {p}", s == 200, f"status={s}")
    for b in created_b:
        s, _ = call("DELETE", f"/v1/bundles/{b}")
        check(f"DELETE bundle {b}", s == 200, f"status={s}")

print("=" * 72)
print(f"PASSED: {PASSED}  FAILED: {FAILED}")
if FAILED:
    print("PATTERNS v0.2 LIVE PROBE: FAILED.")
    sys.exit(1)
print("PATTERNS v0.2 LIVE PROBE: all checks passed.")
