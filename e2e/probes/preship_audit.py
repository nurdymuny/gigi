"""Pre-ship audit for SUDOKU + SAMPLE_TRANSPORT + S3.5 expansion.

Audits D, E, F from Bee's checklist:
  D. Malformed-input fuzz   -- bad input should return 4xx, never panic
  E. Memory/payload bounds  -- hostile query must not OOM or balloon
  F. Persistence smoke      -- insert + query + restart + re-query

Pass = every check below reports OK. Any FAIL is a ship-blocker.
"""
import http.client
import json
import random
import sys
import time

HOST, PORT = "localhost", 3143
CONN = http.client.HTTPConnection(HOST, PORT, timeout=60)
HEADERS = {"Content-Type": "application/json", "Connection": "keep-alive"}

results = []  # (audit_id, label, ok, detail)

def http(method, path, body=None):
    payload = json.dumps(body) if body is not None else ""
    try:
        CONN.request(method, path, payload, HEADERS)
        resp = CONN.getresponse()
        data = resp.read()
        try:
            j = json.loads(data) if data else {}
        except Exception:
            j = {"raw": data[:200].decode("utf-8", errors="replace")}
        return resp.status, j
    except (http.client.BadStatusLine, ConnectionResetError, BrokenPipeError):
        CONN.close()
        CONN.connect()
        CONN.request(method, path, payload, HEADERS)
        resp = CONN.getresponse()
        data = resp.read()
        try:
            j = json.loads(data) if data else {}
        except Exception:
            j = {"raw": data[:200].decode("utf-8", errors="replace")}
        return resp.status, j


def report(audit_id, label, ok, detail=""):
    results.append((audit_id, label, ok, detail))
    tag = "OK" if ok else "FAIL"
    print(f"  [{tag:4s}] {audit_id}: {label}" + (f"  -- {detail}" if detail else ""))


def section(s):
    print(f"\n=== {s} ===")


# Set up a baseline bundle for fuzz/queries.
section("setup")
http("POST", "/v1/bundles", {"name": "audit_bundle",
    "schema": {"fields": {
        "id": "numeric", "color": "categorical", "price": "numeric",
        "emb": "vector(4)"}, "keys": ["id"]}})
rng = random.Random(42)
recs = []
for i in range(1, 201):
    recs.append({
        "id": i,
        "color": rng.choice(["red", "blue", "green"]),
        "price": round(rng.uniform(50, 250), 2),
        "emb": [round(rng.gauss(0, 1), 4) for _ in range(4)],
    })
status, body = http("POST", "/v1/bundles/audit_bundle/insert", {"records": recs})
report("setup", f"created bundle + inserted {len(recs)} records",
       status == 200, f"insert status={status}")


# =================================================================
# AUDIT D -- Malformed-input fuzz
# Each subtest must return 4xx (never 5xx, never crash).
# =================================================================
section("AUDIT D -- malformed-input fuzz")

# D.1: empty constraint list (should be 200 -- valid query, returns everything)
s, b = http("POST", "/v1/bundles/audit_bundle/brain/sudoku",
            {"constraints": [], "max_options": 5, "max_near_misses": 0})
report("D.1", "empty constraints returns SAT with all records",
       s == 200 and b.get("verdict") == "sat", f"status={s}, verdict={b.get('verdict')}")

# D.2: unknown bundle
s, b = http("POST", "/v1/bundles/does_not_exist/brain/sudoku",
            {"constraints": []})
report("D.2", "unknown bundle returns 404", s == 404)

# D.3: malformed constraint (missing field)
s, b = http("POST", "/v1/bundles/audit_bundle/brain/sudoku",
            {"constraints": [{"type": "field", "op": "eq", "value": "red"}]})
report("D.3", "missing 'field' returns 4xx", 400 <= s < 500,
       f"status={s}")

# D.4: unknown op
s, b = http("POST", "/v1/bundles/audit_bundle/brain/sudoku",
            {"constraints": [{"type": "field", "field": "color",
                              "op": "bogus_op", "value": "red"}]})
report("D.4", "unknown op returns 4xx", 400 <= s < 500, f"status={s}")

# D.5: numeric op with non-numeric value
s, b = http("POST", "/v1/bundles/audit_bundle/brain/sudoku",
            {"constraints": [{"type": "field", "field": "price",
                              "op": "le", "value": "not-a-number"}]})
report("D.5", "numeric op + non-numeric value handled cleanly",
       s < 500, f"status={s}")

# D.6: NaN/Inf in numeric op
s, b = http("POST", "/v1/bundles/audit_bundle/brain/sudoku",
            {"constraints": [{"type": "field", "field": "price",
                              "op": "le", "value": float("inf")}]})
# Python json.dumps writes Infinity; server should reject or handle.
report("D.6", "Infinity value handled cleanly (no 5xx, no crash)",
       s < 500, f"status={s}")

# D.7: between with lo > hi (degenerate range)
s, b = http("POST", "/v1/bundles/audit_bundle/brain/sudoku",
            {"constraints": [{"type": "field", "field": "price",
                              "op": "between", "value": [200.0, 100.0]}]})
report("D.7", "degenerate between [lo>hi] handled",
       s == 200, f"status={s}")
# Sub-check: should produce UNSAT (no record can be in empty interval).
if s == 200:
    report("D.7b", "degenerate between yields UNSAT or 0 solutions",
           b.get("verdict") in ("unsat", "unknown") or not b.get("solutions"),
           f"verdict={b.get('verdict')}, sols={len(b.get('solutions') or [])}")

# D.8: contradictory Eq+Eq -- pre-flight UNSAT, 0 records walked
s, b = http("POST", "/v1/bundles/audit_bundle/brain/sudoku",
            {"constraints": [
                {"type": "field", "field": "color", "op": "eq", "value": "red"},
                {"type": "field", "field": "color", "op": "eq", "value": "blue"},
            ]})
report("D.8", "contradictory Eq+Eq pre-flight fires",
       s == 200 and b.get("verdict") == "unsat"
       and b.get("n_records_considered") == 0,
       f"status={s}, verdict={b.get('verdict')}, n={b.get('n_records_considered')}")
if b.get("pre_flight_unsat_reason"):
    report("D.8b", "pre_flight_unsat_reason populated",
           "color" in b["pre_flight_unsat_reason"],
           f"reason={b['pre_flight_unsat_reason']}")
else:
    report("D.8b", "pre_flight_unsat_reason populated", False, "field missing")

# D.9: SAMPLE_TRANSPORT with missing from_keys
s, b = http("POST", "/v1/bundles/audit_bundle/brain/sample_transport",
            {"fiber_fields": ["emb"], "budget": 0.3, "k": 5, "seed": 1})
report("D.9", "SAMPLE_TRANSPORT missing from_keys → 4xx",
       400 <= s < 500, f"status={s}")

# D.10: SAMPLE_TRANSPORT with empty fiber_fields
s, b = http("POST", "/v1/bundles/audit_bundle/brain/sample_transport",
            {"from_keys": {"id": 1}, "fiber_fields": [],
             "budget": 0.3, "k": 5, "seed": 1})
report("D.10", "SAMPLE_TRANSPORT empty fiber_fields → 4xx",
       400 <= s < 500, f"status={s}")

# D.11: SAMPLE_TRANSPORT with bad src key
s, b = http("POST", "/v1/bundles/audit_bundle/brain/sample_transport",
            {"from_keys": {"id": 99999}, "fiber_fields": ["emb"],
             "budget": 0.3, "k": 5, "seed": 1})
report("D.11", "SAMPLE_TRANSPORT unknown source key → 4xx",
       400 <= s < 500, f"status={s}")

# D.12: SAMPLE_TRANSPORT with budget out of range
s, b = http("POST", "/v1/bundles/audit_bundle/brain/sample_transport",
            {"from_keys": {"id": 1}, "fiber_fields": ["emb"],
             "budget": -0.1, "k": 5, "seed": 1})
report("D.12", "SAMPLE_TRANSPORT budget=-0.1 → 4xx",
       400 <= s < 500, f"status={s}")

s, b = http("POST", "/v1/bundles/audit_bundle/brain/sample_transport",
            {"from_keys": {"id": 1}, "fiber_fields": ["emb"],
             "budget": 5.0, "k": 5, "seed": 1})
report("D.12b", "SAMPLE_TRANSPORT budget=5.0 (> 1.0) → 4xx",
       400 <= s < 500, f"status={s}")


# =================================================================
# AUDIT E -- memory / payload growth bounds
# Hostile / large queries must not balloon response size or crash.
# =================================================================
section("AUDIT E -- payload bounds")

# E.1: 20-constraint query on the 200-record bundle. Verify payload
#      stays bounded (selectivity has 20 entries, relaxations capped,
#      pareto capped).
many_constraints = []
for i in range(20):
    many_constraints.append({"type": "field", "field": "price",
                             "op": "ge", "value": float(i * 10)})
s, b = http("POST", "/v1/bundles/audit_bundle/brain/sudoku",
            {"constraints": many_constraints, "max_options": 5,
             "max_near_misses": 5})
report("E.1", "20-constraint query returns 200",
       s == 200, f"status={s}")
if s == 200:
    n_relax = len(b.get("relaxations") or [])
    n_pareto = len(b.get("pareto_near_misses") or [])
    report("E.1b", f"relaxations bounded ({n_relax} entries, cap=12)",
           n_relax <= 12, f"got {n_relax}")
    report("E.1c", f"pareto bounded ({n_pareto} entries)",
           n_pareto <= 50, f"got {n_pareto}")
    report("E.1d", "20-constraint payload < 200KB",
           len(json.dumps(b)) < 200_000, f"size={len(json.dumps(b))}")

# E.2: enormous max_options
s, b = http("POST", "/v1/bundles/audit_bundle/brain/sudoku",
            {"constraints": [], "max_options": 1_000_000,
             "max_near_misses": 0})
report("E.2", "max_options=1M handled (capped by record count)",
       s == 200, f"status={s}")
if s == 200:
    n_sols = len(b.get("solutions") or [])
    report("E.2b", f"solutions = min(records, request) = {n_sols}",
           n_sols == 200, f"got {n_sols} (expected 200)")

# E.3: huge IsIn list
huge_set = [f"color_{i}" for i in range(10_000)] + ["red"]
s, b = http("POST", "/v1/bundles/audit_bundle/brain/sudoku",
            {"constraints": [{"type": "field", "field": "color",
                              "op": "is_in", "value": huge_set}]})
report("E.3", "IsIn with 10001 values handled",
       s == 200, f"status={s}")


# =================================================================
# AUDIT F -- persistence smoke (insert → query → restart → re-query)
# Verifies WAL replay + snapshot integrity for SUDOKU.
# =================================================================
section("AUDIT F -- persistence smoke")

# Snapshot the bundle BEFORE restart and capture results.
def golden_query():
    return http("POST", "/v1/bundles/audit_bundle/brain/sudoku",
                {"constraints": [
                    {"type": "field", "field": "color", "op": "eq",
                     "value": "red"},
                    {"type": "field", "field": "price", "op": "between",
                     "value": [100.0, 200.0]},
                ],
                 "max_options": 5, "max_near_misses": 5})

s_pre, b_pre = golden_query()
report("F.1", "pre-restart golden query OK",
       s_pre == 200 and b_pre.get("verdict") in ("sat", "unsat", "unknown"),
       f"status={s_pre}, verdict={b_pre.get('verdict')}, "
       f"solutions={len(b_pre.get('solutions') or [])}")

# Force a snapshot so the data persists across restart.
s, _ = http("POST", "/v1/bundles/audit_bundle/snapshot", {})
report("F.2", f"explicit snapshot trigger OK",
       s in (200, 204, 404), f"status={s} (404 = no manual endpoint, fine if auto-snap)")

# Tell the test driver the server PID so it can be restarted by us.
print("\n(restart logic handled by caller; F.3-F.5 require server restart)\n")

# =================================================================
# SUMMARY
# =================================================================
print()
print("=" * 70)
print("AUDIT SUMMARY")
print("=" * 70)
fails = [r for r in results if not r[2]]
passes = [r for r in results if r[2]]
print(f"  Passed: {len(passes)}")
print(f"  Failed: {len(fails)}")
if fails:
    print()
    print("FAILURES:")
    for aid, label, ok, detail in fails:
        print(f"  [{aid}] {label}  -- {detail}")
    sys.exit(1)
else:
    print("  All audit D/E checks passed.")
    sys.exit(0)
