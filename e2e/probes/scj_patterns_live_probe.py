"""Live probe for the SCJ Pattern Hunt HTTP surface on gigi-stream.fly.dev.

Runs after `flyctl deploy` to verify the v0.1 patterns surface:
  1. POST /v1/patterns             — DEFINE
  2. GET  /v1/patterns             — SHOW
  3. POST /v1/bundles/{name}/hunt  — HUNT (with min/max in WEIGHT)
  4. _score pinned as the LAST key in HUNT row JSON (SCJ §5(a))
  5. EXCLUDING IN as left-anti-join by base PK
  6. TOP-N + PROJECT
  7. DELETE /v1/patterns/{name}    — DROP
  8. DEFINE OR REPLACE             — collision behavior

Exits non-zero on any FAIL. Designed to be a green/red gate, not a demo.

Auth: requires GIGI_API_KEY in the environment.
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
    print("[fatal] GIGI_API_KEY not set; cannot exercise auth'd endpoints.")
    sys.exit(2)

HEADERS = {
    "Content-Type": "application/json",
    "X-API-Key": API_KEY,
    "Connection": "keep-alive",
}
CTX = ssl.create_default_context()
CONN = http.client.HTTPSConnection(HOST, 443, timeout=60, context=CTX)


def call(method, path, body=None, raw=False):
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
    if raw:
        return r.status, data.decode("utf-8", errors="replace")
    try:
        j = json.loads(data) if data else {}
    except Exception:
        j = {"raw": data[:200].decode("utf-8", errors="replace")}
    return r.status, j


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
print("SCJ PATTERNS LIVE PROBE: gigi-stream.fly.dev")
print("=" * 72)

ts = int(time.time())
bn = f"scj_patterns_probe_{ts}"
en = f"scj_patterns_probe_excl_{ts}"
pname = f"probe_p_{ts}"
created_bundles = []
created_patterns = []

try:
    # ─── Setup ──────────────────────────────────────────────────────────
    s, _ = call("POST", "/v1/bundles", {
        "name": bn,
        "schema": {"fields": {"id": "numeric", "a": "numeric", "b": "numeric"},
                   "keys": ["id"]}
    })
    check(f"create bundle {bn}", s in (200, 201), f"status={s}")
    if s in (200, 201):
        created_bundles.append(bn)

    # 5 rows; a + b = 11 always so min(a*5 + b*5, 10) clips every row.
    records = [{"id": i, "a": i, "b": 11 - i} for i in range(1, 6)]
    s, _ = call("POST", f"/v1/bundles/{bn}/insert", {"records": records})
    check("insert 5 rows", s == 200, f"status={s}")

    # Exclusion bundle with overlap on ids {2, 4}.
    s, _ = call("POST", "/v1/bundles", {
        "name": en,
        "schema": {"fields": {"id": "numeric", "x": "numeric"}, "keys": ["id"]}
    })
    check(f"create exclusion bundle {en}", s in (200, 201), f"status={s}")
    if s in (200, 201):
        created_bundles.append(en)

    excl_records = [{"id": i, "x": 0} for i in (2, 4)]
    s, _ = call("POST", f"/v1/bundles/{en}/insert", {"records": excl_records})
    check("insert 2 exclusion rows", s == 200, f"status={s}")

    # ─── §1: DEFINE / SHOW / HUNT happy path ────────────────────────────
    s, body = call("POST", "/v1/patterns", {
        "name": pname,
        "predicate": "a >= 0",
        "weight": "min(a * 5 + b * 5, 10)",
        "using": ["a", "b"],
    })
    check("POST /v1/patterns (DEFINE w/ min)", s in (200, 201), f"status={s}, body={body}")
    if s in (200, 201):
        created_patterns.append(pname)

    s, body = call("GET", "/v1/patterns")
    names = [p.get("name") for p in body] if isinstance(body, list) else []
    check("GET /v1/patterns includes new pattern",
          pname in names, f"status={s}, count={len(names)}")

    s, raw = call("POST", f"/v1/bundles/{bn}/hunt", {"pattern": pname}, raw=True)
    check("POST /v1/bundles/{bn}/hunt returns 200", s == 200, f"status={s}")
    try:
        rows = json.loads(raw)
    except Exception:
        rows = []
    check("HUNT returns 5 rows",
          isinstance(rows, list) and len(rows) == 5,
          f"got {len(rows) if isinstance(rows, list) else '?'}")

    if isinstance(rows, list) and rows:
        scores = [r.get("_score") for r in rows]
        check("min() clip: every _score == 10.0",
              all(s == 10.0 or s == 10 for s in scores),
              f"scores={scores}")

        # _score must appear AFTER every other key in the raw JSON.
        first_raw = json.dumps(rows[0])
        score_pos = first_raw.rfind('"_score"')
        other_max = max(
            first_raw.find(f'"{k}"') for k in rows[0].keys() if k != "_score"
        )
        check("_score is LAST key in wire JSON (SCJ §5(a))",
              score_pos > other_max,
              f"score@{score_pos} other_max@{other_max} raw={first_raw}")

    # ─── §2: EXCLUDING IN excludes {2, 4} ───────────────────────────────
    s, body = call("POST", f"/v1/bundles/{bn}/hunt", {
        "pattern": pname,
        "excluding": [en],
    })
    check("HUNT EXCLUDING IN returns 200", s == 200, f"status={s}")
    surviving_ids = {r.get("id") for r in body} if isinstance(body, list) else set()
    check("EXCLUDING IN drops {2,4}, keeps {1,3,5}",
          surviving_ids == {1, 3, 5},
          f"got={sorted(surviving_ids)}")

    # ─── §3: TOP-N + PROJECT ────────────────────────────────────────────
    s, body = call("POST", f"/v1/bundles/{bn}/hunt", {
        "pattern": pname,
        "top": 2,
        "project": ["id", "_score"],
    })
    check("HUNT TOP 2 PROJECT (id, _score) returns 200", s == 200, f"status={s}")
    if isinstance(body, list):
        check("TOP 2 returns exactly 2 rows", len(body) == 2, f"got {len(body)}")
        if body:
            keys = set(body[0].keys())
            check("PROJECT (id, _score) returns only id + _score",
                  keys == {"id", "_score"},
                  f"got keys {sorted(keys)}")

    # ─── §4: DEFINE OR REPLACE collision behavior ───────────────────────
    s, body = call("POST", "/v1/patterns", {
        "name": pname,
        "predicate": "a >= 0",
        "weight": "max(a, b)",
        "using": ["a", "b"],
        "replace": False,
    })
    check("POST /v1/patterns w/ replace=false errors on collision",
          s >= 400,
          f"status={s}, body={body}")

    s, body = call("POST", "/v1/patterns", {
        "name": pname,
        "predicate": "a >= 0",
        "weight": "max(a, b)",
        "using": ["a", "b"],
        "replace": True,
    })
    check("POST /v1/patterns w/ replace=true overwrites",
          s in (200, 201),
          f"status={s}, body={body}")

    # Re-run HUNT to confirm new max() weight is in effect.
    s, body = call("POST", f"/v1/bundles/{bn}/hunt", {"pattern": pname, "top": 1})
    check("HUNT after REPLACE returns 200", s == 200, f"status={s}")
    if isinstance(body, list) and body:
        top_score = body[0].get("_score")
        # max(a, b) for row id=1 → max(1, 10) = 10; for id=5 → max(5, 6) = 6.
        # Top row by max(a, b) DESC is id=1 with _score=10.
        check("max() weight reorders results (top _score=10 not 5)",
              top_score == 10 or top_score == 10.0,
              f"top _score={top_score}")

finally:
    # ─── Cleanup ────────────────────────────────────────────────────────
    for p in created_patterns:
        s, _ = call("DELETE", f"/v1/patterns/{p}")
        check(f"DELETE pattern {p}", s == 200, f"status={s}")

    for b in created_bundles:
        s, _ = call("DELETE", f"/v1/bundles/{b}")
        check(f"DELETE bundle {b}", s == 200, f"status={s}")

print("=" * 72)
print(f"PASSED: {PASSED}  FAILED: {FAILED}")
if FAILED:
    print("SCJ PATTERNS LIVE PROBE: FAILED.")
    sys.exit(1)
print("SCJ PATTERNS LIVE PROBE: all checks passed.")
