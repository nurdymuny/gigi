"""TEXTURE and PRECEDENCE — an honest worked example against a live engine.

Everything below runs against a real gigi-stream. Nothing is mocked. Two of the
four sections use data with a PLANTED answer so you can check the verbs against
truth rather than take a number on faith, and one section deliberately triggers
a refusal so you can see the engine decline rather than guess.

    cargo build --release --bin gigi-stream
    PORT=3153 GIGI_DATA_DIR=<scratch> ./target/release/gigi-stream.exe
    python examples/texture_precedence_walkthrough.py

Use 127.0.0.1, never `localhost` — on Windows the latter resolves ::1 first and
costs about 2 seconds per request against 11 ms on the loopback address.
"""
from __future__ import annotations

import json
import math
import random
import urllib.error
import urllib.request

GIGI = "http://127.0.0.1:3153"


def call(method, path, body=None, ndjson=False):
    data = body if isinstance(body, bytes) else (
        json.dumps(body).encode() if body is not None else None)
    hdr = {}
    if data is not None:
        hdr["Content-Type"] = "application/x-ndjson" if ndjson else "application/json"
    req = urllib.request.Request(GIGI + path, data=data, headers=hdr, method=method)
    try:
        with urllib.request.urlopen(req, timeout=300) as r:
            return r.status, json.loads(r.read().decode("utf-8", "replace"))
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read().decode("utf-8", "replace"))


def make(name, fields):
    call("POST", "/v1/gql", {"query": f"DROP BUNDLE {name};"})
    cols = ", ".join(fields)
    code, out = call("POST", "/v1/gql",
                     {"query": f"CREATE BUNDLE {name} (row_id INT BASE, {cols});"})
    assert code == 200, out


def load(name, rows):
    nd = "\n".join(json.dumps(r) for r in rows).encode() + b"\n"
    code, out = call("POST", f"/v1/bundles/{name}/ingest", nd, ndjson=True)
    assert code == 200, out
    return out


def show(title, code, body, keys):
    print(f"\n  {title}")
    print(f"    HTTP {code}")
    if code != 200:
        print(f"    error: {body.get('error')}")
        return
    for k in keys:
        v = body.get(k)
        print(f"    {k:<12} {v:.4f}" if isinstance(v, float) else f"    {k:<12} {v}")
    for line in body.get("reads", []):
        print(f"      > {line}")


rng = random.Random(20260809)
def gauss():
    return rng.gauss(0.0, 1.0)


print("=" * 74)
print("  TEXTURE + PRECEDENCE — worked example, live engine, planted answers")
print("=" * 74)

# ─────────────────────────────────────────────────────────────────────────
# 1 · TEXTURE, with a planted roughness ordering
# ─────────────────────────────────────────────────────────────────────────
print("\n" + "-" * 74)
print("1 · TEXTURE — three signals whose roughness we CHOSE in advance")
print("-" * 74)
print("  Increments built as an MA(1): d_t = e_t + phi * e_{t-1}.")
print("    phi < 0  moves reverse   -> should read ROUGH")
print("    phi = 0  random walk     -> should read RANDOM_WALK")
print("    phi > 0  moves continue  -> should read SMOOTH")

for label, phi in (("jagged", -0.9), ("random_walk", 0.0), ("trending", 0.9)):
    prev, acc, rows = gauss(), 0.0, []
    for i in range(4096):
        e = gauss()
        acc += e + phi * prev
        prev = e
        rows.append({"row_id": i, "seq": float(i), "value": round(acc, 8)})
    nm = f"tex_{label}"
    make(nm, ["seq NUMERIC FIBER", "value NUMERIC FIBER"])
    load(nm, rows)
    code, out = call("POST", f"/v1/bundles/{nm}/texture",
                     {"field": "value", "order": "seq"})
    show(f"{label}  (phi = {phi:+.1f})", code, out,
         ["exponent", "verdict", "r_squared", "n_lags", "n"])

# ─────────────────────────────────────────────────────────────────────────
# 2 · PRECEDENCE, with a planted lead
# ─────────────────────────────────────────────────────────────────────────
print("\n" + "-" * 74)
print("2 · PRECEDENCE — a NON-FINANCIAL pair with a lead we planted")
print("-" * 74)
print("  A web service: `requests` rises, and `latency_ms` follows 6 records")
print("  later because the queue takes time to back up. Requests LEADS latency.")

lag = 6
src = [gauss() for _ in range(2048 + lag + 2)]
req, lat, a, b, rows = 0.0, 0.0, 0.0, 0.0, []
for i in range(2048):
    a += src[i + lag]      # requests: the source, advanced
    b += src[i]            # latency:  the same source, delayed
    rows.append({"row_id": i, "seq": float(i),
                 "requests": round(a, 8), "latency_ms": round(b, 8)})
make("svc", ["seq NUMERIC FIBER", "requests NUMERIC FIBER", "latency_ms NUMERIC FIBER"])
load("svc", rows)

code, out = call("POST", "/v1/bundles/svc/precedence",
                 {"x": "requests", "y": "latency_ms", "order": "seq"})
show("requests vs latency_ms", code, out, ["area", "leads", "magnitude", "n"])
print(f"    PLANTED TRUTH: requests leads by {lag} records -> expect leads = 'x'")

code, out = call("POST", "/v1/bundles/svc/precedence",
                 {"x": "latency_ms", "y": "requests", "order": "seq"})
show("roles swapped", code, out, ["area", "leads"])
print("    Swapping the arguments must negate the area EXACTLY. Same data,")
print("    same verdict, stated the other way round.")

# ─────────────────────────────────────────────────────────────────────────
# 3 · The property being sold: the clock does not matter
# ─────────────────────────────────────────────────────────────────────────
print("\n" + "-" * 74)
print("3 · THE CLAIM — same events, same order, a wildly different clock")
print("-" * 74)
print("  Identical rows. Only the `seq` VALUES change: uniform 0,1,2,... versus")
print("  a cubic clock i^3+i. Any method that bins by time changes its answer.")

warped = [dict(r, seq=float(i ** 3 + i)) for i, r in enumerate(rows)]
make("svc_warped", ["seq NUMERIC FIBER", "requests NUMERIC FIBER", "latency_ms NUMERIC FIBER"])
load("svc_warped", warped)
_, a1 = call("POST", "/v1/bundles/svc/precedence",
             {"x": "requests", "y": "latency_ms", "order": "seq"})
_, a2 = call("POST", "/v1/bundles/svc_warped/precedence",
             {"x": "requests", "y": "latency_ms", "order": "seq"})
print(f"\n    uniform clock : area = {a1['area']:.12f}   leads = {a1['leads']}")
print(f"    cubic   clock : area = {a2['area']:.12f}   leads = {a2['leads']}")
same = a1["area"] == a2["area"]
print(f"    identical: {'YES — bit for bit' if same else 'NO'}")
print("    PRECEDENCE reads record ORDER. There is no bin width to choose and")
print("    therefore none to get wrong.")

# ─────────────────────────────────────────────────────────────────────────
# 4 · Refusals — the engine declining rather than guessing
# ─────────────────────────────────────────────────────────────────────────
print("\n" + "-" * 74)
print("4 · REFUSALS — what it does when it cannot honestly answer")
print("-" * 74)

flat = [{"row_id": i, "seq": float(i), "value": 7.0, "other": float(i)}
        for i in range(512)]
make("flatline", ["seq NUMERIC FIBER", "value NUMERIC FIBER", "other NUMERIC FIBER"])
load("flatline", flat)

for title, path, body in (
    ("a field that never moves", "/v1/bundles/flatline/texture",
     {"field": "value", "order": "seq"}),
    ("a bundle that does not exist", "/v1/bundles/ghost_bundle/texture",
     {"field": "value"}),
    ("a field that does not exist", "/v1/bundles/flatline/texture",
     {"field": "not_a_column"}),
    ("a signal against itself", "/v1/bundles/flatline/precedence",
     {"x": "other", "y": "other", "order": "seq"}),
):
    code, out = call("POST", path, body)
    print(f"\n    {title}")
    print(f"      HTTP {code}: {out.get('error')}")

print("\n    No verdicts here. No exponent of 0.0, no 'neither leads' fallback.")
print("    The failure mode we are avoiding is a confident number computed from")
print("    nothing.")

# ─────────────────────────────────────────────────────────────────────────
print("\n" + "=" * 74)
print("  THE EXACT CALLS")
print("=" * 74)
print("""
  curl -s -XPOST 127.0.0.1:3153/v1/bundles/svc/texture \\
       -H 'Content-Type: application/json' \\
       -d '{"field":"latency_ms","order":"seq"}'

  curl -s -XPOST 127.0.0.1:3153/v1/bundles/svc/precedence \\
       -H 'Content-Type: application/json' \\
       -d '{"x":"requests","y":"latency_ms","order":"seq"}'

  TEXTURE     one required parameter: `field`.
  PRECEDENCE  two: `x` and `y`.
  `order` is optional everywhere — omit it and record order is used.
""")
for nm in ("tex_jagged", "tex_random_walk", "tex_trending", "svc", "svc_warped", "flatline"):
    call("POST", "/v1/gql", {"query": f"DROP BUNDLE {nm};"})
print("  (example bundles dropped)")
