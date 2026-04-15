#!/usr/bin/env python3
"""
Bitcoin Crash Detector — GIGI Demo
===================================
Fetches 365 days of BTC/USD price data from CoinGecko and ingests it into
a GIGI fiber bundle. Classifies each day as crash / pump / normal based on
daily return. Runs GQL queries to show curvature, crash detection, and
point lookups.

Usage:
    python demos/btc_crash.py [--gigi http://localhost:3142]

No API keys required. CoinGecko free tier is rate-limited to ~10 req/min.
"""
import sys
import json
import time
import argparse
from datetime import datetime, timezone

try:
    import requests
except ImportError:
    print("Install requests first: pip install requests")
    sys.exit(1)


def gql(gigi_url: str, query: str) -> dict:
    r = requests.post(f"{gigi_url}/v1/gql", json={"query": query}, timeout=30)
    r.raise_for_status()
    return r.json()


def main():
    parser = argparse.ArgumentParser(description="BTC Crash Detector — GIGI Demo")
    parser.add_argument("--gigi", default="https://gigi-stream.fly.dev",
                        help="GIGI server URL (default: gigi-stream.fly.dev)")
    parser.add_argument("--days", type=int, default=365,
                        help="Days of BTC history to fetch (default: 365)")
    parser.add_argument("--keep", action="store_true",
                        help="Keep the GIGI bundle after demo (default: delete it)")
    args = parser.parse_args()

    GIGI = args.gigi.rstrip("/")
    BUNDLE = f"btc_demo_{int(time.time())}"
    CRASH_THRESHOLD = -0.05   # >5% drop = crash
    PUMP_THRESHOLD  =  0.05   # >5% rise = pump

    print(f"\n{'='*60}")
    print("  GIGI Demo — Bitcoin Crash Detector")
    print(f"  Server : {GIGI}")
    print(f"  Bundle : {BUNDLE}")
    print(f"{'='*60}\n")

    # ── 1. Fetch from CoinGecko ───────────────────────────────────
    print(f"[1/5] Fetching {args.days} days BTC/USD from CoinGecko...")
    resp = requests.get(
        "https://api.coingecko.com/api/v3/coins/bitcoin/market_chart",
        params={"vs_currency": "usd", "days": str(args.days)},
        timeout=30,
    )
    if resp.status_code == 429:
        print("Rate-limited by CoinGecko — wait 60s and retry")
        sys.exit(1)
    resp.raise_for_status()
    raw = resp.json()

    prices  = raw["prices"]          # [[ts_ms, price], ...]
    volumes = raw["total_volumes"]   # [[ts_ms, vol], ...]
    print(f"    Got {len(prices)} data points\n")

    # ── 2. Build GIGI records ──────────────────────────────────────
    print("[2/5] Building records with crash classification...")
    records = []
    for i, (ts_ms, price) in enumerate(prices):
        prev_price = prices[i - 1][1] if i > 0 else price
        dr = (price - prev_price) / prev_price if prev_price and i > 0 else 0.0
        vol = volumes[i][1] if i < len(volumes) else 0.0
        records.append({
            "day_id":       i,
            "timestamp_ns": int(ts_ms * 1_000_000),
            "close_price":  round(price, 2),
            "volume":       int(vol),
            "daily_return": round(dr, 4),
            "crash_signal": (
                "crash" if dr < CRASH_THRESHOLD else
                "pump"  if dr > PUMP_THRESHOLD  else
                "normal"
            ),
        })

    n_crash = sum(1 for r in records if r["crash_signal"] == "crash")
    n_pump  = sum(1 for r in records if r["crash_signal"] == "pump")
    print(f"    {len(records)} records  |  {n_crash} crash days  |  {n_pump} pump days\n")

    # ── 3. Create bundle ───────────────────────────────────────────
    print(f"[3/5] Creating GIGI bundle '{BUNDLE}'...")
    r = requests.post(f"{GIGI}/v1/bundles", json={
        "name": BUNDLE,
        "schema": {
            "fields": {
                "day_id":       "numeric",
                "timestamp_ns": "numeric",
                "close_price":  "numeric",
                "volume":       "numeric",
                "daily_return": "numeric",
                "crash_signal": "categorical",
            },
            "keys": ["day_id"],
        },
    }, timeout=30)
    if r.status_code not in (200, 201):
        print(f"    Bundle create failed: {r.status_code} {r.text[:200]}")
        sys.exit(1)
    print(f"    Created OK\n")

    # ── 4. Ingest ──────────────────────────────────────────────────
    print(f"[4/5] Ingesting {len(records)} records...")
    CHUNK = 200
    t0 = time.time()
    for start in range(0, len(records), CHUNK):
        batch = records[start : start + CHUNK]
        r = requests.post(f"{GIGI}/v1/bundles/{BUNDLE}/insert",
                          json={"records": batch}, timeout=30)
        r.raise_for_status()
    elapsed = time.time() - t0
    print(f"    Done in {elapsed:.2f}s  ({len(records)/elapsed:.0f} rec/s)\n")

    # ── 5. GQL queries ─────────────────────────────────────────────
    print("[5/5] Running GQL queries on GIGI...\n")

    # Curvature
    q = f"CURVATURE {BUNDLE}"
    print(f"    GQL: {q}")
    curv = gql(GIGI, q)
    k    = curv.get("value", 0)
    conf = curv.get("confidence", 0)
    print(f"    K = {k:.6f}   confidence = {conf:.4f}\n")

    # Crash days
    q = f"COVER {BUNDLE} ON crash_signal = 'crash'"
    print(f"    GQL: {q}")
    crashes = gql(GIGI, q)
    crash_records = crashes.get("records", [])
    print(f"    Found {crashes.get('count', len(crash_records))} crash days")
    if crash_records:
        worst = sorted(crash_records, key=lambda x: x.get("daily_return", 0))[:5]
        for rec in worst:
            ts_s  = rec.get("timestamp_ns", 0) // 1_000_000_000
            dt    = datetime.fromtimestamp(ts_s, tz=timezone.utc).strftime("%Y-%m-%d")
            price = rec.get("close_price", 0)
            ret   = rec.get("daily_return", 0)
            print(f"      {dt}  return={ret:+.1%}  BTC=${price:,.0f}")
    print()

    # First point query
    q = f"SECTION {BUNDLE} AT day_id=0"
    print(f"    GQL: {q}")
    sec = gql(GIGI, q)
    day0 = sec.get("section") or sec
    print(f"    Day 0 BTC: ${day0.get('close_price', '?'):,}\n")

    # ── Summary ────────────────────────────────────────────────────
    print("=" * 60)
    print(f"  Bundle : {BUNDLE}")
    print(f"  Records: {len(records)}")
    print(f"  K      : {k:.6f}  (high K = volatile price manifold)")
    print(f"  Crashes: {n_crash} days with >5% single-day drop")
    print()
    print("  curl commands:")
    print(f'    curl -s {GIGI}/v1/gql -H "Content-Type: application/json" \\')
    print(f"         -d '{{\"query\": \"CURVATURE {BUNDLE}\"}}' | python -m json.tool")
    print()
    print(f'    curl -s {GIGI}/v1/gql -H "Content-Type: application/json" \\')
    print(f"         -d '{{\"query\": \"COVER {BUNDLE} ON crash_signal = \'crash\'\"}}' | python -m json.tool")
    print("=" * 60)

    if not args.keep:
        print(f"\nCleaning up bundle '{BUNDLE}'...")
        requests.delete(f"{GIGI}/v1/bundles/{BUNDLE}", timeout=10)
        print("Done.")
    else:
        print(f"\nBundle '{BUNDLE}' kept on {GIGI}")


if __name__ == "__main__":
    main()
