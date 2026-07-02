#!/usr/bin/env python3
"""Seed the GIGI Builds demo bundles — stations + chembl — into a live engine.

The site's web extras (the-weather-on-a-sphere-that-notices, chemical-
space-has-a-shape) compute their worlds in-browser from a seeded PRNG.
This script regenerates the SAME worlds — bit-for-bit, via an exact
port of the demos' mulberry32 — and loads them into an engine, so the
GQL drawers on those pages query the very dots the visitor is looking
at.

Usage:
    # local engine (no key)
    python3 examples/seed_demo_bundles.py --endpoint http://localhost:3142

    # the public instance (needs the write key; run once, from Bee's side)
    GIGI_API_KEY=... python3 examples/seed_demo_bundles.py \\
        --endpoint https://gigi-stream.fly.dev

    # look, don't touch
    python3 examples/seed_demo_bundles.py --dry-run

Idempotent-ish: drops and recreates the two bundles each run.
"""
from __future__ import annotations

import argparse
import math
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "sdk", "python"))
from gigi.client import GigiClient  # noqa: E402


# ── mulberry32, exactly as the demos run it ─────────────────────────────
# JS semantics reproduced with 32-bit masks: Math.imul is a 32-bit
# multiply, |0 / >>>0 are truncations, and the float division at the end
# is exact. tests: `first_outputs_match_js` below, cross-checked against
# node during development.
M32 = 0xFFFFFFFF


def mulberry32(seed: int):
    a = seed & M32

    def rng() -> float:
        nonlocal a
        a = (a + 0x6D2B79F5) & M32
        t = a
        t = ((t ^ (t >> 15)) * (t | 1)) & M32
        t = (t ^ ((t + (((t ^ (t >> 7)) * (t | 61)) & M32)) & M32)) & M32
        return ((t ^ (t >> 14)) & M32) / 4294967296

    return rng


# ── the globe's 480 stations (seed 20260702) ────────────────────────────
def gen_stations():
    rng = mulberry32(20260702)
    centers = []
    for _ in range(14):
        centers.append(
            {"lat": (rng() * 150 - 75) * math.pi / 180, "lon": rng() * 2 * math.pi}
        )
    out = []
    for i in range(480):
        if i % 5 == 4:  # dust
            lat = math.asin(rng() * 2 - 1) * 0.9
            lon = rng() * 2 * math.pi
        else:
            ct = centers[i % 14]
            lat = ct["lat"] + (rng() - 0.5) * 0.35
            lon = ct["lon"] + (rng() - 0.5) * 0.5
            lat = max(-1.45, min(1.45, lat))
        lat_deg = lat * 180 / math.pi
        band = (
            "60N-90N" if lat_deg > 60 else
            "30N-60N" if lat_deg > 30 else
            "0-30N" if lat_deg > 0 else
            "0-30S" if lat_deg > -30 else
            "30S-60S" if lat_deg > -60 else
            "60S-90S"
        )
        base = 28 * math.cos(lat) - 2 + (rng() - 0.5) * 4  # climate normal
        _phase = rng()  # demo uses this for its live wiggle; consumed to stay in sync
        out.append(
            {
                "station_id": f"st-{i:03d}",
                "band": band,
                "lat": round(lat_deg, 4),
                "lon": round(lon * 180 / math.pi, 4),
                "temp": round(base, 4),
            }
        )
    return out


# ── the chem demo's 2,200 molecules (seed 42) ───────────────────────────
def gen_molecules():
    rng = mulberry32(42)

    def gauss() -> float:
        u = max(rng(), 1e-12)
        v = rng()
        return math.sqrt(-2 * math.log(u)) * math.cos(6.2832 * v)

    n = 2200
    mols = []

    def mk(i, mw, logp, tpsa):
        mols.append(
            {
                "chembl_id": f"CHX-{i:04d}",
                "mw": round(mw, 4),
                "logp": round(logp, 4),
                "tpsa": round(tpsa, 4),
            }
        )

    i = 0
    while i < n * 0.55:  # drug-like core
        mk(i, 380 + gauss() * 60, 2.5 + gauss() * 1.0, 75 + gauss() * 18)
        i += 1
    while i < n * 0.80:  # fragment corner
        mk(i, 230 + gauss() * 30, 1.2 + gauss() * 0.8, 45 + gauss() * 12)
        i += 1
    while i < n * 0.95:  # greasy tail
        mk(i, 480 + gauss() * 60, 5.5 + gauss() * 0.8, 40 + gauss() * 12)
        i += 1
    while i < n - 12:  # loose dust
        mk(i, 180 + rng() * 420, -2 + rng() * 9, 10 + rng() * 140)
        i += 1
    while i < n:  # the twelve planted outliers (JS arg order: mw, logp, branch, value)
        mw = 150 + rng() * 550
        logp = -2.8 + rng() * 11
        tpsa = (150 + rng() * 30) if rng() < 0.5 else rng() * 8
        mk(i, mw, logp, tpsa)
        i += 1
    return mols


def first_outputs_match_js():
    """Guard: first three draws of mulberry32(42), computed once with
    node during development. If this port ever drifts, refuse to seed."""
    rng = mulberry32(42)
    got = [rng() for _ in range(3)]
    want = [0.60110375192016363, 0.44829055899754167, 0.85246579349040985]
    for g, w in zip(got, want):
        assert abs(g - w) < 1e-15, f"mulberry32 port drifted: {got} != {want}"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--endpoint", default="http://localhost:3142")
    ap.add_argument("--key", default=os.environ.get("GIGI_API_KEY"))
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()

    first_outputs_match_js()
    stations = gen_stations()
    mols = gen_molecules()
    print(f"stations: {len(stations)} rows (bands: "
          f"{sorted(set(s['band'] for s in stations))})")
    print(f"chembl:   {len(mols)} rows, e.g. {mols[0]}")
    if args.dry_run:
        print("dry run — nothing sent")
        return 0

    client = GigiClient(args.endpoint, api_key=args.key)
    print(f"target: {args.endpoint}")

    for name in ("stations", "chembl"):
        try:
            client.drop_bundle(name)
            print(f"dropped old '{name}'")
        except Exception:
            pass

    client.create_bundle(
        "stations",
        fields={"station_id": "categorical", "band": "categorical",
                "lat": "numeric", "lon": "numeric", "temp": "numeric"},
        keys=["station_id"],
        indexed=["band"],
    )
    for i in range(0, len(stations), 200):
        client.insert("stations", stations[i : i + 200])
    print("stations seeded")

    client.create_bundle(
        "chembl",
        fields={"chembl_id": "categorical", "mw": "numeric",
                "logp": "numeric", "tpsa": "numeric"},
        keys=["chembl_id"],
    )
    for i in range(0, len(mols), 200):
        client.insert("chembl", mols[i : i + 200])
    print("chembl seeded")

    for name in ("stations", "chembl"):
        import requests

        h = requests.post(
            f"{args.endpoint.rstrip('/')}/v1/gql",
            json={"query": f"HEALTH {name};"},
            headers={"X-Api-Key": args.key} if args.key else {},
            timeout=15,
        ).json()
        print(f"HEALTH {name}: {h}")
    print("done — the site's GQL drawers for these bundles are now live")
    return 0


if __name__ == "__main__":
    sys.exit(main())
