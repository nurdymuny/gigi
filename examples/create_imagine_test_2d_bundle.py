"""
Create a purpose-built 2D test bundle for IMAGINE_COHERENCE end-to-end
verification.

================================================================================
WHY THIS EXISTS

IMAGINE_COHERENCE Phase 1 supports 2D substrates only (see geodesic.rs
DimNotSupported). Marcella's production bundles are 384-dim (BGE-v2
embeddings). Even when the request seed is `[0.0, 0.0]` (2D), the
endpoint reads the substrate's mean Gaussian curvature from
`bundle.curvature_stats().mean()` -- which for a 384-dim bundle is
large enough to make the 2D conformal integrator diverge at step 1.

This script creates a bundle whose:
  - fiber is exactly 2D (so DimNotSupported never fires)
  - records are distributed so the mean K is small (so the integrator
    doesn't diverge)
  - distribution covers enough of the chart to make IMAGINE trajectories
    interesting (not flat-line)

It's the synthetic test bundle Marcella asked for in her 2026-06-03 note
so her wired `imagine_coherence()` consumer in `brain_primitives.py` can
verify end-to-end against gigi-stream BEFORE Phase 2 dim lift lands.

USAGE
    python examples/create_imagine_test_2d_bundle.py \
        --base-url https://gigi-stream.fly.dev \
        --api-key $GIGI_API_KEY
    # Defaults to localhost:3142 with no key.

================================================================================

WHAT GETS CREATED

Bundle name:     imagine_test_2d
Records:         100 points on a noisy 2D ring (radius ~0.5)
Schema:
    base:  [{name: "pk", type: "int"}]
    fiber: [{name: "x", type: "float"},
            {name: "y", type: "float"}]

The ring is a "tame" curvature distribution: spread points around a
small circle in the chart, so the curvature signal is bounded and
non-degenerate. Expected mean Gaussian K is in [0.05, 0.5] -- well
within the conformal integrator's stable range.

PROBE EXAMPLE

After running this script, call:

    curl -X POST \
      https://gigi-stream.fly.dev/v1/bundles/imagine_test_2d/imagine_coherence \
      -H "Content-Type: application/json" \
      -H "X-API-Key: $GIGI_API_KEY" \
      -d '{
        "starting_from": [0.1, 0.1],
        "along": [0.5, 0.0],
        "steps": 5,
        "metric_curvature": 0.3,
        "query_grounding_normalized": 0.2
      }'

Expected response (200): trajectory of 6 points, endpoint_coherence
near 0.95, refused=false, routing_advisory present (because we passed
query_grounding_normalized).
"""

from __future__ import annotations
import argparse
import json
import math
import sys
from typing import Optional
from urllib import request as urlrequest
from urllib import error as urlerror


BUNDLE_NAME = "imagine_test_2d"
N_RECORDS = 100
RING_RADIUS = 0.5
RING_NOISE = 0.05


def make_records() -> list[dict]:
    """Generate 100 points on a noisy 2D ring (deterministic seed)."""
    import random
    rng = random.Random(20260603)
    records = []
    for i in range(N_RECORDS):
        theta = 2.0 * math.pi * (i / N_RECORDS)
        # Ring at radius 0.5 with small Gaussian noise.
        r = RING_RADIUS + rng.gauss(0.0, RING_NOISE)
        x = r * math.cos(theta)
        y = r * math.sin(theta)
        records.append({"pk": i, "x": x, "y": y})
    return records


def http_post(base_url: str, path: str, body: dict, api_key: Optional[str]) -> dict:
    """POST JSON, return parsed response or raise."""
    url = f"{base_url.rstrip('/')}{path}"
    data = json.dumps(body).encode("utf-8")
    headers = {"Content-Type": "application/json"}
    if api_key:
        headers["X-API-Key"] = api_key
    req = urlrequest.Request(url, data=data, headers=headers, method="POST")
    try:
        with urlrequest.urlopen(req, timeout=30) as resp:
            return json.loads(resp.read().decode("utf-8"))
    except urlerror.HTTPError as e:
        body_str = e.read().decode("utf-8", errors="replace")
        raise SystemExit(f"HTTP {e.code} on POST {path}: {body_str}")
    except urlerror.URLError as e:
        raise SystemExit(f"Network error on POST {path}: {e}")


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Create the imagine_test_2d bundle on a gigi-stream instance.",
    )
    parser.add_argument(
        "--base-url",
        default="http://localhost:3142",
        help="gigi-stream base URL (default: http://localhost:3142).",
    )
    parser.add_argument(
        "--api-key",
        default=None,
        help="API key (if the instance is auth-gated). Reads from GIGI_API_KEY env if unset.",
    )
    args = parser.parse_args()

    import os
    api_key = args.api_key or os.environ.get("GIGI_API_KEY")

    print(f"Creating bundle '{BUNDLE_NAME}' on {args.base_url}...")
    create_body = {
        "name": BUNDLE_NAME,
        "base": [{"name": "pk", "type": "int"}],
        "fiber": [
            {"name": "x", "type": "float"},
            {"name": "y", "type": "float"},
        ],
    }
    resp = http_post(args.base_url, "/v1/bundles", create_body, api_key)
    print(f"  bundle create response: {resp}")

    records = make_records()
    print(f"Inserting {len(records)} records onto a noisy 2D ring...")
    insert_body = {"records": records}
    resp = http_post(
        args.base_url,
        f"/v1/bundles/{BUNDLE_NAME}/insert",
        insert_body,
        api_key,
    )
    print(f"  insert response: {resp}")

    print()
    print(f"Bundle '{BUNDLE_NAME}' ready. Probe IMAGINE_COHERENCE with:")
    print()
    print(f"  curl -X POST {args.base_url}/v1/bundles/{BUNDLE_NAME}/imagine_coherence \\")
    print(f"    -H 'Content-Type: application/json' \\")
    if api_key:
        print(f"    -H 'X-API-Key: $GIGI_API_KEY' \\")
    print(f"    -d '{{\"starting_from\":[0.1,0.1],\"along\":[0.5,0.0],\"steps\":5,\"metric_curvature\":0.3,\"query_grounding_normalized\":0.2}}'")
    print()
    print("Expected: 200 OK with trajectory[6], endpoint_coherence ~0.95,")
    print("          refused=false, routing_advisory present.")

    return 0


if __name__ == "__main__":
    sys.exit(main())
