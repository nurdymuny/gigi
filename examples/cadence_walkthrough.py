"""CADENCE — an honest worked example against a live engine.

Everything below runs against a real gigi-stream. Nothing is mocked. Every
stream is generated with a KNOWN answer so you can check the verb against truth
rather than take a number on faith, and one section deliberately triggers
refusals so you can see the engine decline rather than guess.

The data is a web service's request log — arrival times of HTTP requests. No
market vocabulary anywhere, because CADENCE measures the shape of any timestamped
event stream: log lines, sensor pings, seismic events, neuron spikes, trades.

    cargo build --release --bin gigi-stream
    PORT=3155 GIGI_DATA_DIR=<scratch> ./target/release/gigi-stream.exe
    python examples/cadence_walkthrough.py

Use 127.0.0.1, never `localhost` — on Windows the latter resolves ::1 first and
costs about 2 seconds per request against 11 ms on the loopback address.
"""
from __future__ import annotations

import json
import math
import random
import urllib.error
import urllib.request

GIGI = "http://127.0.0.1:3155"


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


def make(name):
    call("POST", "/v1/gql", {"query": f"DROP BUNDLE {name};"})
    code, out = call("POST", "/v1/gql", {
        "query": f"CREATE BUNDLE {name} (row_id INT BASE, "
                 f"ts NUMERIC FIBER, endpoint TEXT FIBER);"})
    assert code == 200, out


def load(name, stamps):
    rows = [{"row_id": i, "ts": round(t, 9), "endpoint": "/api/v1/search"}
            for i, t in enumerate(stamps)]
    nd = "\n".join(json.dumps(r) for r in rows).encode() + b"\n"
    code, out = call("POST", f"/v1/bundles/{name}/ingest", nd, ndjson=True)
    assert code == 200, out


def measure(name, stamps, verbose=True):
    make(name)
    load(name, stamps)
    code, out = call("POST", f"/v1/bundles/{name}/cadence", {"time": "ts"})
    if verbose:
        print(f"    HTTP {code}")
        if code != 200:
            print(f"    error: {out.get('error')}")
            return out
        print(f"    index   {out['index']:.4f}   ({out['index_z']:+.1f} sd)"
              f"   -> {out['verdict']}")
        print(f"    memory  {out['memory']:+.4f}   ({out['memory_z']:+.1f} sd)"
              f"   -> {out['memory_verdict']}")
    return out


rng = random.Random(20260809)
def expo(mean):
    return -math.log(max(rng.random(), 1e-12)) * mean


print("=" * 76)
print("  CADENCE — worked example, live engine, planted answers")
print("  Data: arrival times of HTTP requests hitting a web service.")
print("=" * 76)

N = 4096

# ─────────────────────────────────────────────────────────────────────────
# 1 · The null
# ─────────────────────────────────────────────────────────────────────────
print("\n" + "-" * 76)
print("1 · THE NULL — traffic with no structure at all")
print("-" * 76)
print("  Memoryless arrivals: each request independent of the last. The")
print("  boring, healthy baseline. index should sit at 1.")

t, steady = 0.0, []
for _ in range(N):
    t += expo(0.1)
    steady.append(t)
measure("cad_steady", steady)
print("    PLANTED TRUTH: memoryless -> expect STEADY / MEMORYLESS")

# ─────────────────────────────────────────────────────────────────────────
# 2 · Three streams index cannot tell apart
# ─────────────────────────────────────────────────────────────────────────
print("\n" + "-" * 76)
print("2 · WHY THIS VERB RETURNS TWO NUMBERS")
print("-" * 76)
print("  Three services. All three are BURSTY — `index` calls them the same.")
print("  They are not the same, and `memory` is what says so.")

# (a) Heavy-tailed but independent: some requests are slow to arrive, but
#     nothing about one gap tells you about the next.
t, heavy = 0.0, []
for _ in range(N):
    z = math.sqrt(-2 * math.log(max(rng.random(), 1e-12))) * \
        math.cos(2 * math.pi * rng.random())
    t += math.exp(z)
    heavy.append(t)
print("\n  (a) Long-tailed traffic — occasional very quiet stretches, but each")
print("      gap independent of the last. Nothing is triggering anything.")
measure("cad_heavy", heavy)

# (b) A cron job: a tight burst on a fixed rhythm.
t, cron = 0.0, []
while len(cron) < N:
    for _ in range(8):
        t += expo(0.02)
        cron.append(t)
    t += expo(4.0)
cron = cron[:N]
print("\n  (b) A batch job firing on a rhythm — eight requests, a pause, repeat.")
measure("cad_cron", cron)

# (c) Genuine mode switching: the service is busy for a while, then quiet
#     for a while, and it STAYS in whichever mode it is in.
t, moded, fast = 0.0, [], True
for _ in range(N):
    if rng.random() < 0.02:
        fast = not fast
    t += expo(0.1 if fast else 2.0)
    moded.append(t)
print("\n  (c) Real load waves — the service gets busy and STAYS busy, then")
print("      goes quiet and stays quiet. Traffic has a state.")
measure("cad_moded", moded)

print("\n  All three read BURSTY. Read `index` alone and they are one thing.")
print("  `memory` splits them: no memory (a), alternating (b), persistent (c).")
print("  Only (c) is a stream where being busy now predicts being busy next.")

# ─────────────────────────────────────────────────────────────────────────
# 3 · The property being sold
# ─────────────────────────────────────────────────────────────────────────
print("\n" + "-" * 76)
print("3 · THE CLAIM — `index` provably cannot see order; `memory` only sees order")
print("-" * 76)
print("  Take the load-wave stream and SHUFFLE its gaps. Same gaps, same total")
print("  span, every trace of the load waves destroyed.")

gaps = [moded[i + 1] - moded[i] for i in range(len(moded) - 1)]
rng.shuffle(gaps)
t, shuffled = moded[0], [moded[0]]
for g in gaps:
    t += g
    shuffled.append(t)

a = measure("cad_moded2", moded, verbose=False)
b = measure("cad_shuf", shuffled, verbose=False)
print(f"\n    load waves   : index = {a['index']:.12f}   memory = {a['memory']:+.4f}"
      f"  ({a['memory_verdict']})")
print(f"    gaps shuffled: index = {b['index']:.12f}   memory = {b['memory']:+.4f}"
      f"  ({b['memory_verdict']})")
delta = abs(a["index"] - b["index"])
print(f"    index moved by {delta:.2e}   memory moved by "
      f"{abs(a['memory'] - b['memory']):.4f}")
print("\n    `index` is a sum over the gaps, and addition does not care about")
print("    order — so it CANNOT distinguish these two, ever. That is not a")
print("    finite-sample weakness, it is arithmetic. Which is exactly why the")
print("    second number exists, and why the two never measure the same thing.")
print("\n    Being precise about that 1e-12, because it is not the statistic")
print("    moving: to feed shuffled gaps back in, this script rebuilds the")
print("    timestamps by cumulative sum, which adds the same floats in a")
print("    different order and shifts the last bit or two. The gaps GIGI then")
print("    re-derives differ in their final ulp. Gate CAD-6 does the same")
print("    reconstruction and holds the difference under 1e-9. Feed the")
print("    identical gap multiset and the answer is bit-for-bit identical;")
print("    what you are seeing here is float reassociation, not sensitivity.")

# ─────────────────────────────────────────────────────────────────────────
# 4 · Refusals
# ─────────────────────────────────────────────────────────────────────────
print("\n" + "-" * 76)
print("4 · THE FEED-QUALITY ALARM — index BELOW 1")
print("-" * 76)
print("  Everything above was bursty or memoryless. The other direction is the")
print("  one worth wiring to a pager: a stream MORE REGULAR than random.")
print("  Real sources are not that even. If you see this, something is metering")
print("  the stream between the source and you — a poll interval, a rate")
print("  limiter, a fixed flush — and every number computed downstream is")
print("  describing your own plumbing rather than the thing you meant to watch.")

t, polled = 0.0, []
for _ in range(N):
    t += 0.1 * (0.95 + 0.10 * rng.random())     # 100ms poll, +-5% jitter
    polled.append(t)
r = measure("cad_polled", polled)
print("    PLANTED TRUTH: a 100 ms poller. The clock is full f64 resolution —")
print("    nothing is rounded. The stream is regular; the CLOCK is fine.")
print("    That distinction is the whole gate: a verb that refused this as")
print("    'coarsely stamped' would have no way to raise the alarm at all.")

print("\n" + "-" * 76)
print("5 · REFUSALS — what it does when it cannot honestly answer")
print("-" * 76)

print("\n    a row index passed where a clock belongs")
print("       (the easy mistake: TEXTURE takes `order`, this takes `time`)")
r = measure("cad_counter", [float(i) for i in range(2048)], verbose=False)
print(f"      {r.get('error')}")

print("\n    a clock too coarse to measure (timestamps rounded to whole seconds)")
t, coarse = 0.0, []
for _ in range(2048):
    t += expo(2.0)
    coarse.append(round(t))
r = measure("cad_coarse", coarse, verbose=False)
print(f"      {r.get('error')}")

print("\n    not enough events")
r = measure("cad_thin", steady[:20], verbose=False)
print(f"      {r.get('error')}")

make("cad_ghost")
load("cad_ghost", steady)
for title, body in (("a field that does not exist", {"time": "not_a_column"}),
                    ("a text field", {"time": "endpoint"})):
    code, out = call("POST", "/v1/bundles/cad_ghost/cadence", body)
    print(f"\n    {title}")
    print(f"      HTTP {code}: {out.get('error')}")

code, out = call("POST", "/v1/bundles/no_such_bundle/cadence", {"time": "ts"})
print(f"\n    a bundle that does not exist")
print(f"      HTTP {code}: {out.get('error')}")

print("\n    No verdicts here. No index of 0.0 on the counter, no STEADY on the")
print("    rounded clock. The failure mode being avoided is a confident number")
print("    computed from something that was never measured.")

# ─────────────────────────────────────────────────────────────────────────
print("\n" + "=" * 76)
print("  THE EXACT CALLS")
print("=" * 76)
print("""
  curl -s -XPOST 127.0.0.1:3155/v1/bundles/cad_moded/cadence \\
       -H 'Content-Type: application/json' \\
       -d '{"time":"ts"}'

  One required parameter: `time`, a NUMERIC timestamp field.
  `block` is optional (default 64 gaps per block).

  It is `time`, not `order`, on purpose. TEXTURE and PRECEDENCE take an
  ordinal sort key and ignore its values; CADENCE needs the real clock,
  because the spacing IS the measurement.
""")
for nm in ("cad_steady", "cad_heavy", "cad_cron", "cad_moded", "cad_moded2", "cad_polled",
           "cad_shuf", "cad_counter", "cad_coarse", "cad_thin", "cad_ghost"):
    call("POST", "/v1/gql", {"query": f"DROP BUNDLE {nm};"})
print("  (example bundles dropped)")
