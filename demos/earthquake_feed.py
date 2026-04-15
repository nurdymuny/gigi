#!/usr/bin/env python3
"""
USGS Live Earthquake Feed — GIGI Demo
=======================================
Fetches the past 30 days of M1.0+ earthquake events from the USGS GeoJSON
feed and ingests them into GIGI. Demonstrates curvature analysis of a
real-time geospatial dataset.

Usage:
    python demos/earthquake_feed.py [--gigi http://localhost:3142] [--feed month]

Feed options: hour | day | week | month  (default: month)
No API keys required.
"""
import sys
import time
import argparse
from datetime import datetime, timezone

try:
    import requests
except ImportError:
    print("Install requests first: pip install requests")
    sys.exit(1)


USGS_FEEDS = {
    "hour":  "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/1.0_hour.geojson",
    "day":   "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/1.0_day.geojson",
    "week":  "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/1.0_week.geojson",
    "month": "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/1.0_month.geojson",
}

ALERT_MAP = {              # PAGER alert level → ordinal for numeric field
    "green":  1,
    "yellow": 2,
    "orange": 3,
    "red":    4,
    None:     0,
    "":       0,
}


def gql(gigi_url: str, query: str) -> dict:
    r = requests.post(f"{gigi_url}/v1/gql", json={"query": query}, timeout=30)
    r.raise_for_status()
    return r.json()


def main():
    parser = argparse.ArgumentParser(description="USGS Earthquake Feed — GIGI Demo")
    parser.add_argument("--gigi", default="https://gigi-stream.fly.dev",
                        help="GIGI server URL (default: gigi-stream.fly.dev)")
    parser.add_argument("--feed", choices=list(USGS_FEEDS.keys()), default="month",
                        help="USGS feed window (default: month)")
    parser.add_argument("--keep", action="store_true",
                        help="Keep the GIGI bundle after demo")
    args = parser.parse_args()

    GIGI   = args.gigi.rstrip("/")
    BUNDLE = f"quake_demo_{int(time.time())}"
    URL    = USGS_FEEDS[args.feed]

    print(f"\n{'='*60}")
    print("  GIGI Demo — USGS Live Earthquake Feed")
    print(f"  Server : {GIGI}")
    print(f"  Bundle : {BUNDLE}")
    print(f"  Feed   : {URL}")
    print(f"{'='*60}\n")

    # ── 1. Fetch from USGS ────────────────────────────────────────
    print(f"[1/5] Fetching USGS earthquake feed ({args.feed})...")
    resp = requests.get(URL, timeout=30)
    resp.raise_for_status()
    geo = resp.json()

    features = geo["features"]
    meta     = geo.get("metadata", {})
    print(f"    Feed title : {meta.get('title', 'USGS earthquakes')}")
    print(f"    Count      : {len(features)} events")
    print(f"    Generated  : {datetime.fromtimestamp(meta.get('generated', 0)//1000, tz=timezone.utc).strftime('%Y-%m-%d %H:%M UTC')}\n")

    # ── 2. Build GIGI records ──────────────────────────────────────
    print("[2/5] Building records with categorical classification...")
    records = []
    for i, feat in enumerate(features):
        props = feat["properties"]
        coords = feat["geometry"]["coordinates"]  # [lon, lat, depth]

        mag   = props.get("mag") or 0.0
        depth = coords[2] if len(coords) > 2 else 0.0
        alert = props.get("alert") or ""

        # Magnitude tier — categorical for COVER queries
        if mag >= 7.0:
            mag_tier = "major"
        elif mag >= 5.0:
            mag_tier = "strong"
        elif mag >= 3.0:
            mag_tier = "moderate"
        else:
            mag_tier = "minor"

        # Hazard flag — "true" for M>=5
        significant = "true" if mag >= 5.0 else "false"

        # Depth class — categorical
        depth_class = "shallow" if depth < 70 else ("intermediate" if depth < 300 else "deep")

        records.append({
            "quake_id":    i,
            "time_ns":     int(props.get("time", 0) * 1_000_000),
            "magnitude":   round(mag, 2),
            "depth_km":    round(depth, 1),
            "lat":         round(coords[1], 4),
            "lon":         round(coords[0], 4),
            "mag_tier":    mag_tier,
            "significant": significant,
            "depth_class": depth_class,
            "alert_level": alert if alert in ("green", "yellow", "orange", "red") else "none",
        })

    by_tier = {t: sum(1 for r in records if r["mag_tier"] == t) for t in ("major","strong","moderate","minor")}
    print(f"    major:{by_tier['major']}  strong:{by_tier['strong']}  moderate:{by_tier['moderate']}  minor:{by_tier['minor']}\n")

    # ── 3. Create bundle ───────────────────────────────────────────
    print(f"[3/5] Creating GIGI bundle '{BUNDLE}'...")
    r = requests.post(f"{GIGI}/v1/bundles", json={
        "name": BUNDLE,
        "schema": {
            "fields": {
                "quake_id":    "numeric",
                "time_ns":     "numeric",
                "magnitude":   "numeric",
                "depth_km":    "numeric",
                "lat":         "numeric",
                "lon":         "numeric",
                "mag_tier":    "categorical",
                "significant": "categorical",
                "depth_class": "categorical",
                "alert_level": "categorical",
            },
            "keys": ["quake_id"],
        },
    }, timeout=30)
    if r.status_code not in (200, 201):
        print(f"    Bundle create failed: {r.status_code} {r.text[:200]}")
        sys.exit(1)
    print("    Created OK\n")

    # ── 4. Ingest ──────────────────────────────────────────────────
    print(f"[4/5] Ingesting {len(records)} records...")
    CHUNK = 500
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

    # Curvature of the seismic manifold
    q = f"CURVATURE {BUNDLE}"
    print(f"    GQL: {q}")
    curv = gql(GIGI, q)
    k    = curv.get("value", 0)
    conf = curv.get("confidence", 0)
    print(f"    K = {k:.6f}   confidence = {conf:.4f}\n")

    # Significant (M>=5) quakes
    q = f"COVER {BUNDLE} ON significant = 'true'"
    print(f"    GQL: {q}")
    sig = gql(GIGI, q)
    sig_recs = sig.get("records", [])
    print(f"    Found {sig.get('count', len(sig_recs))} significant M≥5 events")
    if sig_recs:
        biggest = sorted(sig_recs, key=lambda x: -x.get("magnitude", 0))[:5]
        for rec in biggest:
            ts = rec.get("time_ns", 0) // 1_000_000_000
            dt = datetime.fromtimestamp(ts, tz=timezone.utc).strftime("%Y-%m-%d %H:%M")
            lat = rec.get("lat", 0)
            lon = rec.get("lon", 0)
            mag = rec.get("magnitude", 0)
            print(f"      M{mag:.1f}  {dt}  ({lat:+.2f}, {lon:+.2f})")
    print()

    # Deep vs shallow
    q = f"COVER {BUNDLE} ON depth_class = 'deep'"
    print(f"    GQL: {q}")
    deep = gql(GIGI, q)
    print(f"    Deep events (>300 km): {deep.get('count', len(deep.get('records', [])))}\n")

    # Point query for the largest event
    if records:
        largest = max(records, key=lambda r: r["magnitude"])
        q = f"SECTION {BUNDLE} AT quake_id={largest['quake_id']}"
        print(f"    GQL: {q}  (largest event)")
        sec = gql(GIGI, q)
        ev  = sec.get("section") or sec
        print(f"    M{ev.get('magnitude', '?')}  lat={ev.get('lat', '?')}  lon={ev.get('lon', '?')}\n")

    # ── Summary ────────────────────────────────────────────────────
    n_sig = sum(1 for r in records if r["significant"] == "true")
    print("=" * 60)
    print(f"  Bundle   : {BUNDLE}")
    print(f"  Records  : {len(records)}")
    print(f"  K        : {k:.6f}  (curvature of seismic fiber bundle)")
    print(f"  M≥5      : {n_sig} events  ({100*n_sig/max(len(records),1):.1f}%)")
    print()
    print("  To query live:")
    print(f'    curl -s {GIGI}/v1/gql -H "Content-Type: application/json" \\')
    print(f"         -d '{{\"query\": \"CURVATURE {BUNDLE}\"}}' | python -m json.tool")
    print("=" * 60)

    if not args.keep:
        print(f"\nCleaning up bundle '{BUNDLE}'...")
        requests.delete(f"{GIGI}/v1/bundles/{BUNDLE}", timeout=10)
        print("Done.")
    else:
        print(f"\nBundle '{BUNDLE}' kept on {GIGI}")


if __name__ == "__main__":
    main()
