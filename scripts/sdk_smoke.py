#!/usr/bin/env python3
"""SDK smoke test — the python client, end to end, against a live engine.

Run by scripts/sdk_smoke.sh (which boots gigi-stream first), or by hand:

    GIGI_URL=http://localhost:3142 python3 scripts/sdk_smoke.py

Exercises the surface a first-day user touches: health, create_bundle,
insert, get, query, and a GQL statement — and asserts the geometric
ride-alongs (curvature, confidence) actually arrive. Exits nonzero on
the first broken promise, so CI catches SDK/server drift.
"""
from __future__ import annotations

import os
import sys
import time

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "sdk", "python"))
from gigi.client import GigiClient  # noqa: E402

URL = os.environ.get("GIGI_URL", "http://localhost:3142")
BUNDLE = "smoke_sensors"


def main() -> int:
    client = GigiClient(URL)

    # 1. health answers
    h = client.health()
    assert h.get("status") == "ok", f"health not ok: {h}"

    # 2. create a bundle (drop leftovers from a previous run first)
    try:
        client.drop_bundle(BUNDLE)
    except Exception:
        pass
    client.create_bundle(
        BUNDLE,
        fields={"id": "categorical", "city": "categorical", "temp": "numeric"},
        keys=["id"],
        indexed=["city"],
    )

    # 3. insert sections — the response carries the geometry
    r = client.insert(
        BUNDLE,
        [
            {"id": "s1", "city": "Moscow", "temp": -3.0},
            {"id": "s2", "city": "Moscow", "temp": -25.5},
            {"id": "s3", "city": "Lagos", "temp": 31.0},
            {"id": "s4", "city": "Lagos", "temp": 29.5},
        ],
    )
    inserted = r.get("inserted", r.get("count"))
    assert inserted == 4, f"expected 4 inserted, got: {r}"

    # 4. point read comes back whole
    rec = client.get(BUNDLE, id="s2")
    assert rec is not None and rec.get("city") == "Moscow", f"get(s2): {rec}"

    # 5. filtered query
    rows = client.query(
        BUNDLE, filters=[{"field": "temp", "op": "lt", "value": 0}]
    )
    assert len(rows) == 2, f"expected 2 sub-zero rows, got {len(rows)}: {rows}"

    # 6. GQL: aggregation over the indexed field
    import requests

    headers = {"Content-Type": "application/json"}
    gql = requests.post(
        f"{URL}/v1/gql",
        json={"query": f"INTEGRATE {BUNDLE} OVER city MEASURE count(*), avg(temp);"},
        headers=headers,
        timeout=10,
    )
    gql.raise_for_status()
    body = gql.json()
    rows = body.get("rows", [])
    assert len(rows) >= 2, f"INTEGRATE OVER city should give >=2 groups: {body}"

    # 7. geometric ride-alongs exist and are sane
    stats = requests.post(
        f"{URL}/v1/gql",
        json={"query": f"HEALTH {BUNDLE};"},
        headers=headers,
        timeout=10,
    ).json()
    assert "curvature" in stats and "confidence" in stats, f"HEALTH shape: {stats}"
    assert 0.0 <= stats["confidence"] <= 1.0, f"confidence out of range: {stats}"

    # 8. cleanup
    client.drop_bundle(BUNDLE)
    print("SDK smoke: all 8 checks passed")
    return 0


if __name__ == "__main__":
    for attempt in range(3):
        try:
            sys.exit(main())
        except AssertionError:
            raise
        except Exception as e:  # transient boot-order noise: retry
            if attempt == 2:
                raise
            print(f"retry after: {e}", file=sys.stderr)
            time.sleep(2)
