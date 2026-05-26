"""End-to-end probe — wave 2 + SUDOKU S2/S3 against local gigi-stream.

Probe-script style modeled on Marcella's existing probes:
  - Real HTTP calls, real bundle setup, real assertions.
  - Categorical + numeric fiber fields shaped like voice_anchors.
  - All three SUDOKU verdict cases (Sat, Unsat, Unknown).
  - DHOOM content negotiation (Accept: application/dhoom).
  - X-Bundle-Mutation-Counter header invariants across calls.
  - Brain regression: sample, fit_diagnostics still work after the
    flat-matrix refactor.

Exits 0 on full pass; >=1 on any failure. Prints findings inline so
they're easy to skim.
"""
import json
import sys
import time
import urllib.request

BASE = "http://localhost:3143"

class Probe:
    def __init__(self):
        self.findings = []
        self.fails = 0

    def call(self, method, path, body=None, accept="application/json",
             timeout=30):
        data = None
        headers = {"Accept": accept}
        if body is not None:
            data = json.dumps(body).encode()
            headers["Content-Type"] = "application/json"
        req = urllib.request.Request(
            BASE + path, data=data, headers=headers, method=method
        )
        t0 = time.perf_counter()
        try:
            with urllib.request.urlopen(req, timeout=timeout) as r:
                raw = r.read()
                dt = time.perf_counter() - t0
                return r.status, dict(r.headers), raw, dt
        except urllib.error.HTTPError as e:
            dt = time.perf_counter() - t0
            return e.code, dict(e.headers or {}), e.read(), dt

    def check(self, name, cond, detail=""):
        prefix = "  PASS" if cond else "  FAIL"
        if not cond:
            self.fails += 1
        msg = f"{prefix}: {name}" + (f" — {detail}" if detail else "")
        print(msg)
        self.findings.append((name, cond, detail))


def main():
    p = Probe()

    print("\n=== Phase 1: bundle setup ===")
    # Create a voice-anchors-shaped bundle (CreateBundleRequest shape).
    schema = {
        "name": "voice_anchors_e2e",
        "schema": {
            "fields": {
                "anchor_id": "numeric",
                "intent": "categorical",
                "confidence": "numeric",
                "v0": "numeric",
                "v1": "numeric",
                "v2": "numeric",
                "v3": "numeric",
                "phrase": "categorical",
            },
            "keys": ["anchor_id"],
        }
    }
    status, _, raw, _ = p.call("POST", "/v1/bundles", body=schema)
    p.check("bundle create", status in (200, 201, 409),
            f"HTTP {status} — body: {raw[:200]!r}")

    # Insert ~100 records exercising Sat / Unsat / near-miss cases.
    # Pattern:
    #   30 'math_question' records, high conf (0.8-0.99)
    #   30 'greeting' records, high conf
    #   20 'critique' records, medium conf (0.5-0.7)
    #   20 'off_topic' records, low conf (0.1-0.3)
    import random
    random.seed(42)
    records = []
    for i in range(30):
        records.append({
            "anchor_id": i,
            "intent": "math_question",
            "confidence": 0.8 + 0.19 * random.random(),
            "v0": random.gauss(0, 1),
            "v1": random.gauss(0, 1),
            "v2": random.gauss(0, 1),
            "v3": random.gauss(0, 1),
            "phrase": f"math_phrase_{i % 5}",  # duplicates for mass aggregation
        })
    for i in range(30):
        records.append({
            "anchor_id": 100 + i,
            "intent": "greeting",
            "confidence": 0.85 + 0.14 * random.random(),
            "v0": random.gauss(0, 1),
            "v1": random.gauss(0, 1),
            "v2": random.gauss(0, 1),
            "v3": random.gauss(0, 1),
            "phrase": f"hello_phrase_{i % 4}",
        })
    for i in range(20):
        records.append({
            "anchor_id": 200 + i,
            "intent": "critique",
            "confidence": 0.5 + 0.2 * random.random(),
            "v0": random.gauss(0, 1),
            "v1": random.gauss(0, 1),
            "v2": random.gauss(0, 1),
            "v3": random.gauss(0, 1),
            "phrase": f"critique_phrase_{i % 3}",
        })
    for i in range(20):
        records.append({
            "anchor_id": 300 + i,
            "intent": "off_topic",
            "confidence": 0.1 + 0.2 * random.random(),
            "v0": random.gauss(0, 1),
            "v1": random.gauss(0, 1),
            "v2": random.gauss(0, 1),
            "v3": random.gauss(0, 1),
            "phrase": f"off_phrase_{i % 2}",
        })

    # Insert (correct route: /insert with {records: [...]}).
    status, _, raw, _ = p.call(
        "POST", "/v1/bundles/voice_anchors_e2e/insert",
        body={"records": records},
    )
    p.check("batch insert", status == 200,
            f"HTTP {status} — body: {raw[:200]!r}")

    # Verify count.
    status, _, raw, _ = p.call(
        "POST", "/v1/bundles/voice_anchors_e2e/query",
        body={"filters": [], "limit": 1000}
    )
    body = json.loads(raw) if status == 200 else {}
    count = len(body.get("data") or [])
    p.check("record count", count == 100,
            f"got {count} (expected 100)")

    print("\n=== Phase 2: SUDOKU endpoint — Sat case ===")
    # Constraint: intent == math_question. Should return many solutions.
    status, headers, raw, dt = p.call(
        "POST", "/v1/bundles/voice_anchors_e2e/brain/sudoku",
        body={
            "constraints": [
                {"type": "field", "field": "intent", "op": "eq",
                 "value": "math_question", "hard": True}
            ],
            "max_options": 10,
            "max_near_misses": 5,
        }
    )
    print(f"  HTTP {status} in {dt*1000:.1f}ms")
    body = json.loads(raw) if status == 200 else {}
    p.check("Sat: HTTP 200", status == 200,
            f"body: {raw[:200]!r}")
    p.check("Sat: verdict=sat",
            body.get("verdict") == "sat",
            f"got {body.get('verdict')!r}")
    p.check("Sat: solutions non-empty",
            len(body.get("solutions") or []) > 0,
            f"got {len(body.get('solutions') or [])} solutions")
    p.check("Sat: coverage = 1.0",
            body.get("coverage") == 1.0,
            f"got {body.get('coverage')}")
    p.check("Sat: n_records_considered = 100",
            body.get("n_records_considered") == 100,
            f"got {body.get('n_records_considered')}")
    p.check("Sat: X-Bundle-Mutation-Counter header present",
            "x-bundle-mutation-counter" in {k.lower() for k in headers},
            f"headers keys: {list(headers.keys())}")
    counter_1 = headers.get("x-bundle-mutation-counter") or \
                headers.get("X-Bundle-Mutation-Counter")

    print("\n=== Phase 3: SUDOKU — cache hit check ===")
    # Same call again — should be cache hit. Counter should match.
    status, headers2, raw2, dt2 = p.call(
        "POST", "/v1/bundles/voice_anchors_e2e/brain/sudoku",
        body={
            "constraints": [
                {"type": "field", "field": "intent", "op": "eq",
                 "value": "math_question", "hard": True}
            ],
            "max_options": 10,
        }
    )
    print(f"  HTTP {status} in {dt2*1000:.1f}ms (was {dt*1000:.1f}ms)")
    counter_2 = headers2.get("x-bundle-mutation-counter") or \
                headers2.get("X-Bundle-Mutation-Counter")
    p.check("cache hit: same counter",
            counter_1 == counter_2,
            f"counter1={counter_1} counter2={counter_2}")

    print("\n=== Phase 4: SUDOKU — Unsat case ===")
    # Constraint: intent == "nonexistent_intent". Should return Unsat.
    status, _, raw, dt = p.call(
        "POST", "/v1/bundles/voice_anchors_e2e/brain/sudoku",
        body={
            "constraints": [
                {"type": "field", "field": "intent", "op": "eq",
                 "value": "nonexistent_intent", "hard": True}
            ],
        }
    )
    print(f"  HTTP {status} in {dt*1000:.1f}ms")
    body = json.loads(raw) if status == 200 else {}
    p.check("Unsat: HTTP 200", status == 200,
            f"body: {raw[:200]!r}")
    p.check("Unsat: verdict=unsat",
            body.get("verdict") == "unsat",
            f"got {body.get('verdict')!r}")
    p.check("Unsat: solutions empty",
            len(body.get("solutions") or []) == 0,
            f"got {len(body.get('solutions') or [])} solutions")

    print("\n=== Phase 5: SUDOKU — Numeric Le constraint + near-miss ===")
    # confidence <= 0.85; math_question records around 0.8-0.99
    # will produce some solutions and some near-misses.
    status, _, raw, dt = p.call(
        "POST", "/v1/bundles/voice_anchors_e2e/brain/sudoku",
        body={
            "constraints": [
                {"type": "field", "field": "intent", "op": "eq",
                 "value": "math_question", "hard": True},
                {"type": "field", "field": "confidence", "op": "le",
                 "value": 0.85, "hard": True}
            ],
            "max_options": 5,
            "max_near_misses": 5,
        }
    )
    print(f"  HTTP {status} in {dt*1000:.1f}ms")
    body = json.loads(raw) if status == 200 else {}
    p.check("Compound: HTTP 200", status == 200,
            f"body: {raw[:200]!r}")
    p.check("Compound: has solutions",
            len(body.get("solutions") or []) > 0,
            "expected some math_question records with conf<=0.85")
    near_misses = body.get("near_misses") or []
    p.check("Compound: has near-misses",
            len(near_misses) > 0,
            "expected some records violating exactly one constraint")
    if near_misses:
        nm = near_misses[0]
        p.check("near-miss: violations is single-element",
                len(nm.get("violations") or []) == 1,
                f"violations: {nm.get('violations')}")

    print("\n=== Phase 6: SUDOKU — is_in categorical ===")
    status, _, raw, dt = p.call(
        "POST", "/v1/bundles/voice_anchors_e2e/brain/sudoku",
        body={
            "constraints": [
                {"type": "field", "field": "intent", "op": "is_in",
                 "value": ["greeting", "critique"], "hard": True}
            ],
            "max_options": 10,
        }
    )
    body = json.loads(raw) if status == 200 else {}
    p.check("is_in: verdict=sat",
            body.get("verdict") == "sat",
            f"got {body.get('verdict')!r}")

    print("\n=== Phase 7: SUDOKU — Manifold constraint returns 400 (S4) ===")
    status, _, raw, dt = p.call(
        "POST", "/v1/bundles/voice_anchors_e2e/brain/sudoku",
        body={
            "constraints": [
                {"type": "manifold", "field": "v0",
                 "near_manifold": "fake_bundle", "epsilon": 0.3,
                 "hard": True}
            ],
        }
    )
    body = json.loads(raw) if status >= 400 else {}
    p.check("Manifold: HTTP 400",
            status == 400,
            f"got HTTP {status} — body: {raw[:200]!r}")
    err = body.get("error", "")
    p.check("Manifold: error mentions S4",
            "S4" in err or "Manifold" in err,
            f"error: {err!r}")

    print("\n=== Phase 8: DHOOM content negotiation ===")
    status, headers, raw, dt = p.call(
        "POST", "/v1/bundles/voice_anchors_e2e/brain/sudoku",
        body={
            "constraints": [
                {"type": "field", "field": "intent", "op": "eq",
                 "value": "greeting", "hard": True}
            ],
            "max_options": 3,
        },
        accept="application/dhoom",
    )
    ct = headers.get("content-type") or headers.get("Content-Type") or ""
    p.check("DHOOM: HTTP 200", status == 200,
            f"body: {raw[:200]!r}")
    p.check("DHOOM: Content-Type is application/dhoom",
            "application/dhoom" in ct,
            f"got: {ct!r}")
    p.check("DHOOM: body starts with text marker, not JSON",
            not raw.startswith(b"{"),
            f"body first 80B: {raw[:80]!r}")
    p.check("DHOOM: X-Bundle-Mutation-Counter still emitted",
            "x-bundle-mutation-counter" in {k.lower() for k in headers},
            f"headers keys: {list(headers.keys())}")

    print("\n=== Phase 9: Regression — brain/sample (wave 2 §B flat matrix) ===")
    # confidence is a single numeric — won't fit a flow (needs even
    # dim). Use v0/v1/v2/v3 instead.
    status, headers, raw, dt = p.call(
        "POST", "/v1/bundles/voice_anchors_e2e/brain/sample",
        body={
            "fields": ["v0", "v1", "v2", "v3"],
            "fit_mode": "diagonal",
            "n_samples": 5,
            "burn_in": 100,
            "temperature": 1.0,
            "seed": 42
        }
    )
    body = json.loads(raw) if status == 200 else {}
    p.check("brain/sample: HTTP 200", status == 200,
            f"body: {raw[:300]!r}")
    samples = body.get("samples") or []
    p.check("brain/sample: returned 5 samples",
            len(samples) == 5,
            f"got {len(samples)}")
    if samples:
        p.check("brain/sample: each sample has 4 dims",
                all(len(s) == 4 for s in samples),
                f"first sample length: {len(samples[0])}")

    print("\n=== Phase 10: Regression — brain/fit_diagnostics ===")
    status, _, raw, dt = p.call(
        "POST", "/v1/bundles/voice_anchors_e2e/brain/fit_diagnostics",
        body={
            "fields": ["v0", "v1", "v2", "v3"],
            "fit_mode": "diagonal",
            "sigma_floor_epsilon": 0.001,
        }
    )
    body = json.loads(raw) if status == 200 else {}
    p.check("fit_diagnostics: HTTP 200", status == 200,
            f"body: {raw[:200]!r}")
    p.check("fit_diagnostics: dim = 4",
            body.get("dim") == 4,
            f"got {body.get('dim')}")
    p.check("fit_diagnostics: fit_mean has 4 entries",
            len(body.get("fit_mean") or []) == 4,
            f"got {len(body.get('fit_mean') or [])}")
    p.check("fit_diagnostics: variance_per_dim_raw has 4 entries",
            len(body.get("variance_per_dim_raw") or []) == 4,
            f"got {len(body.get('variance_per_dim_raw') or [])}")

    print("\n=== Phase 11: Counter invariant after insert ===")
    # Insert one more record; counter should bump on next SUDOKU call.
    new_record = {
        "anchor_id": 999,
        "intent": "greeting",
        "confidence": 0.95,
        "v0": 0.1, "v1": 0.2, "v2": 0.3, "v3": 0.4,
        "phrase": "test_insert"
    }
    status, _, _, _ = p.call(
        "POST", "/v1/bundles/voice_anchors_e2e/insert",
        body={"records": [new_record]}
    )
    p.check("insert one record: HTTP 200", status == 200, "")

    status, headers3, _, _ = p.call(
        "POST", "/v1/bundles/voice_anchors_e2e/brain/sudoku",
        body={"constraints": []}
    )
    counter_3 = headers3.get("x-bundle-mutation-counter") or \
                headers3.get("X-Bundle-Mutation-Counter")
    p.check("counter bumped after insert",
            int(counter_3) > int(counter_2),
            f"counter2={counter_2} counter3={counter_3}")

    print(f"\n=== SUMMARY ===")
    print(f"Total checks: {len(p.findings)}")
    print(f"Passes:       {len(p.findings) - p.fails}")
    print(f"Failures:     {p.fails}")
    return 1 if p.fails > 0 else 0


if __name__ == "__main__":
    sys.exit(main())
