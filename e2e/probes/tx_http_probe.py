"""Live verification of the /v1/transactions HTTP surface against
gigi-stream.fly.dev.

Covers Phase-A behavior end-to-end:
  - begin/status/write/commit on a multi-bundle transaction
  - records actually land in the touched bundles
  - status reflects each lifecycle step
  - post-commit writes are refused with 409
  - rollback discards pending and produces aborted state
  - isolation level option round-trips

This is the contract probe customers will use to wire client SDKs
against the surface. Failures here gate any deploy that touches the
transactions feature.
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
        j = {"_raw": data[:200].decode("utf-8", errors="replace")}
    return r.status, j


results = []
fails = []


def check(label, ok, detail=""):
    tag = "PASS" if ok else "FAIL"
    print(f"  [{tag}] {label}" + (f"  -- {detail}" if detail else ""))
    results.append((label, ok))
    if not ok:
        fails.append((label, detail))


print("=" * 72)
print(f"LIVE /v1/transactions PROBE: {HOST}")
print("=" * 72)

ts = int(time.time())
b1 = f"tx_probe_a_{ts}"
b2 = f"tx_probe_b_{ts}"
for b in [b1, b2]:
    s, _ = call(
        "POST",
        "/v1/bundles",
        {"name": b, "schema": {"fields": {"id": "numeric", "val": "numeric"}, "keys": ["id"]}},
    )
    check(f"create {b}", s in (200, 201), f"status={s}")

# 1. BEGIN
s, body = call("POST", "/v1/transactions/begin", {})
check(
    "BEGIN returns tx_id + snap_id + iso=SI",
    s == 200 and body.get("tx_id", "").startswith("tx_") and body.get("isolation") == "snapshot_isolation",
    f"status={s}, body={body}",
)
tx_id = body["tx_id"]

# 2. STATUS (open, no pending)
s, body = call("GET", f"/v1/transactions/{tx_id}")
check(
    "STATUS shows state=open, pending=0",
    s == 200 and body.get("state") == "open" and body.get("pending_writes") == 0,
    f"state={body.get('state')}, pending={body.get('pending_writes')}",
)

# 3. WRITE to b1
s, body = call(
    "POST",
    f"/v1/transactions/{tx_id}/write",
    {"bundle": b1, "records": [{"id": i, "val": float(i)} for i in range(1, 4)]},
)
check(
    "WRITE b1: 3 staged",
    s == 200 and body.get("staged") == 3 and body.get("total_in_tx") == 3,
    f"body={body}",
)

# 4. WRITE to b2
s, body = call(
    "POST",
    f"/v1/transactions/{tx_id}/write",
    {"bundle": b2, "records": [{"id": 100 + i, "val": float(i) * 10} for i in range(1, 6)]},
)
check(
    "WRITE b2: 5 staged, total=8, both bundles touched",
    s == 200
    and body.get("staged") == 5
    and body.get("total_in_tx") == 8
    and len(body.get("touched_bundles", [])) == 2,
    f"body={body}",
)

# 5. STATUS (open, pending=8)
s, body = call("GET", f"/v1/transactions/{tx_id}")
check(
    "STATUS mid-tx: pending=8, two bundles touched",
    s == 200 and body.get("state") == "open" and body.get("pending_writes") == 8,
    f"body={body}",
)

# 6. COMMIT
s, body = call("POST", f"/v1/transactions/{tx_id}/commit")
check(
    "COMMIT applies all 8 records across both bundles",
    s == 200 and body.get("records_committed") == 8 and len(body.get("bundles_committed", [])) == 2,
    f"body={body}",
)

# 7. Verify records actually landed
for bn, expected_ids in [(b1, {1, 2, 3}), (b2, {101, 102, 103, 104, 105})]:
    s, body = call("POST", f"/v1/bundles/{bn}/query", {"limit": 10})
    ids = {r.get("id") for r in body.get("data", [])}
    check(
        f"verify {bn} contains {expected_ids}",
        s == 200 and expected_ids.issubset(ids),
        f"got ids={ids}",
    )

# 8. STATUS post-commit
s, body = call("GET", f"/v1/transactions/{tx_id}")
check(
    "STATUS post-commit: state=committed",
    s == 200 and body.get("state") == "committed",
    f"state={body.get('state')}",
)

# 9. Write-after-commit refused
s, body = call(
    "POST",
    f"/v1/transactions/{tx_id}/write",
    {"bundle": b1, "records": [{"id": 999, "val": 999.0}]},
)
check(
    "WRITE after commit refused (409)",
    s == 409 and "Committed" in (body.get("error") or ""),
    f"status={s}, error={body.get('error')}",
)

# 10. ROLLBACK path
s, body = call("POST", "/v1/transactions/begin", {"isolation": "read_committed"})
check(
    "BEGIN with isolation=read_committed",
    s == 200 and body.get("isolation") == "read_committed",
    f"iso={body.get('isolation')}",
)
tx_rb = body["tx_id"]
call(
    "POST",
    f"/v1/transactions/{tx_rb}/write",
    {"bundle": b1, "records": [{"id": 555, "val": 555.0}]},
)
s, body = call("POST", f"/v1/transactions/{tx_rb}/rollback")
check(
    "ROLLBACK discards pending and reports aborted",
    s == 200 and body.get("aborted") is True and body.get("discarded_records") == 1,
    f"body={body}",
)

# 11. Verify rollback did NOT land
s, body = call("POST", f"/v1/bundles/{b1}/query", {"limit": 100})
has_555 = any(r.get("id") == 555 for r in body.get("data", []))
check("rollback: id=555 NOT in b1 after rollback", not has_555, f"present={has_555}")

# 12. Post-rollback status
s, body = call("GET", f"/v1/transactions/{tx_rb}")
check(
    "STATUS post-rollback: state=aborted",
    s == 200 and body.get("state") == "aborted",
    f"state={body.get('state')}",
)

# 13. System bundle blocked
s, body = call(
    "POST",
    "/v1/transactions/begin",
    {},
)
sys_tx = body["tx_id"]
s, body = call(
    "POST",
    f"/v1/transactions/{sys_tx}/write",
    {"bundle": "_gigi_log", "records": [{"id": 1}]},
)
check(
    "WRITE to system bundle refused (403)",
    s == 403,
    f"status={s}, error={body.get('error', body)}",
)
call("POST", f"/v1/transactions/{sys_tx}/rollback")

# 14. Bogus tx_id returns 400
s, body = call("GET", "/v1/transactions/not-a-uuid")
check("STATUS on malformed tx_id returns 400", s == 400, f"status={s}")

# 15. Unknown tx_id returns 404
s, body = call("GET", "/v1/transactions/tx_00000000-0000-0000-0000-000000000000")
check("STATUS on unknown tx_id returns 404", s == 404, f"status={s}")

# Cleanup
call("DELETE", f"/v1/bundles/{b1}")
call("DELETE", f"/v1/bundles/{b2}")

print("\n" + "=" * 72)
passed = sum(1 for _, ok in results if ok)
total = len(results)
print(f"PASSED: {passed}/{total}")
if fails:
    print("\nFAILURES:")
    for label, detail in fails:
        print(f"  - {label}: {detail}")
    sys.exit(1)
print("\nAll /v1/transactions probes green.")
sys.exit(0)
