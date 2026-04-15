#!/usr/bin/env python3
"""
Music DNA — GIGI Demo
======================
Ingests a curated dataset of 50 well-known artists x 8 genre dimensions
into a GIGI fiber bundle. Demonstrates curvature analysis on a cultural
manifold. No external API required — data is embedded in this script.

Usage:
    python demos/music_dna.py [--gigi http://localhost:3142] [--seed ARTIST]

The --seed artist controls which artist is used as the probe for SECTION
and COVER queries (default: Radiohead).
"""
import sys
import time
import argparse

try:
    import requests
except ImportError:
    print("Install requests first: pip install requests")
    sys.exit(1)


# ── Artist dataset ─────────────────────────────────────────────────────────────
# Each artist has 8 genre dimension scores (0.0–1.0) and a primary_genre label.
# Scores are approximate and intended for demo / illustrative purposes.
ARTISTS = [
    {"artist": "Radiohead",              "rock": 0.85, "electronic": 0.70, "pop": 0.35, "hip_hop": 0.05, "jazz": 0.15, "classical": 0.20, "metal": 0.10, "folk": 0.05, "primary_genre": "art_rock"},
    {"artist": "Pink Floyd",             "rock": 0.90, "electronic": 0.50, "pop": 0.30, "hip_hop": 0.00, "jazz": 0.10, "classical": 0.25, "metal": 0.05, "folk": 0.10, "primary_genre": "prog_rock"},
    {"artist": "Aphex Twin",             "rock": 0.05, "electronic": 0.98, "pop": 0.10, "hip_hop": 0.05, "jazz": 0.20, "classical": 0.30, "metal": 0.05, "folk": 0.00, "primary_genre": "idm"},
    {"artist": "Boards of Canada",       "rock": 0.10, "electronic": 0.92, "pop": 0.15, "hip_hop": 0.05, "jazz": 0.15, "classical": 0.20, "metal": 0.00, "folk": 0.15, "primary_genre": "idm"},
    {"artist": "Kendrick Lamar",         "rock": 0.05, "electronic": 0.30, "pop": 0.35, "hip_hop": 0.98, "jazz": 0.25, "classical": 0.05, "metal": 0.00, "folk": 0.00, "primary_genre": "hip_hop"},
    {"artist": "Tyler the Creator",      "rock": 0.10, "electronic": 0.40, "pop": 0.40, "hip_hop": 0.92, "jazz": 0.20, "classical": 0.05, "metal": 0.00, "folk": 0.05, "primary_genre": "hip_hop"},
    {"artist": "Massive Attack",         "rock": 0.20, "electronic": 0.85, "pop": 0.30, "hip_hop": 0.20, "jazz": 0.25, "classical": 0.15, "metal": 0.00, "folk": 0.05, "primary_genre": "trip_hop"},
    {"artist": "Portishead",             "rock": 0.25, "electronic": 0.75, "pop": 0.25, "hip_hop": 0.15, "jazz": 0.30, "classical": 0.15, "metal": 0.00, "folk": 0.05, "primary_genre": "trip_hop"},
    {"artist": "Miles Davis",            "rock": 0.05, "electronic": 0.15, "pop": 0.05, "hip_hop": 0.05, "jazz": 0.99, "classical": 0.20, "metal": 0.00, "folk": 0.00, "primary_genre": "jazz"},
    {"artist": "John Coltrane",          "rock": 0.00, "electronic": 0.00, "pop": 0.00, "hip_hop": 0.00, "jazz": 0.99, "classical": 0.25, "metal": 0.00, "folk": 0.00, "primary_genre": "jazz"},
    {"artist": "Beethoven",              "rock": 0.00, "electronic": 0.00, "pop": 0.05, "hip_hop": 0.00, "jazz": 0.05, "classical": 0.99, "metal": 0.10, "folk": 0.10, "primary_genre": "classical"},
    {"artist": "Bach",                   "rock": 0.00, "electronic": 0.00, "pop": 0.00, "hip_hop": 0.00, "jazz": 0.05, "classical": 0.99, "metal": 0.00, "folk": 0.05, "primary_genre": "classical"},
    {"artist": "The Beatles",            "rock": 0.95, "electronic": 0.10, "pop": 0.80, "hip_hop": 0.00, "jazz": 0.15, "classical": 0.10, "metal": 0.05, "folk": 0.30, "primary_genre": "rock"},
    {"artist": "Led Zeppelin",           "rock": 0.95, "electronic": 0.05, "pop": 0.20, "hip_hop": 0.00, "jazz": 0.10, "classical": 0.05, "metal": 0.40, "folk": 0.30, "primary_genre": "rock"},
    {"artist": "Black Sabbath",          "rock": 0.75, "electronic": 0.05, "pop": 0.05, "hip_hop": 0.00, "jazz": 0.00, "classical": 0.05, "metal": 0.97, "folk": 0.00, "primary_genre": "metal"},
    {"artist": "Metallica",              "rock": 0.60, "electronic": 0.05, "pop": 0.05, "hip_hop": 0.00, "jazz": 0.00, "classical": 0.05, "metal": 0.99, "folk": 0.00, "primary_genre": "metal"},
    {"artist": "Bob Dylan",              "rock": 0.45, "electronic": 0.00, "pop": 0.30, "hip_hop": 0.00, "jazz": 0.10, "classical": 0.05, "metal": 0.00, "folk": 0.95, "primary_genre": "folk"},
    {"artist": "Nick Drake",             "rock": 0.20, "electronic": 0.00, "pop": 0.20, "hip_hop": 0.00, "jazz": 0.15, "classical": 0.20, "metal": 0.00, "folk": 0.95, "primary_genre": "folk"},
    {"artist": "Daft Punk",              "rock": 0.10, "electronic": 0.95, "pop": 0.65, "hip_hop": 0.10, "jazz": 0.05, "classical": 0.00, "metal": 0.00, "folk": 0.00, "primary_genre": "electronic"},
    {"artist": "The Chemical Brothers",  "rock": 0.20, "electronic": 0.90, "pop": 0.25, "hip_hop": 0.10, "jazz": 0.00, "classical": 0.00, "metal": 0.05, "folk": 0.00, "primary_genre": "electronic"},
    {"artist": "Coltrane",               "rock": 0.00, "electronic": 0.00, "pop": 0.00, "hip_hop": 0.00, "jazz": 0.99, "classical": 0.30, "metal": 0.00, "folk": 0.00, "primary_genre": "jazz"},
    {"artist": "Bjork",                  "rock": 0.30, "electronic": 0.80, "pop": 0.45, "hip_hop": 0.05, "jazz": 0.10, "classical": 0.30, "metal": 0.10, "folk": 0.15, "primary_genre": "art_pop"},
    {"artist": "Kate Bush",              "rock": 0.35, "electronic": 0.40, "pop": 0.60, "hip_hop": 0.00, "jazz": 0.10, "classical": 0.25, "metal": 0.00, "folk": 0.20, "primary_genre": "art_pop"},
    {"artist": "David Bowie",            "rock": 0.75, "electronic": 0.35, "pop": 0.65, "hip_hop": 0.00, "jazz": 0.10, "classical": 0.10, "metal": 0.05, "folk": 0.10, "primary_genre": "art_rock"},
    {"artist": "Brian Eno",              "rock": 0.20, "electronic": 0.90, "pop": 0.15, "hip_hop": 0.00, "jazz": 0.10, "classical": 0.35, "metal": 0.00, "folk": 0.05, "primary_genre": "idm"},
    {"artist": "Kanye West",             "rock": 0.10, "electronic": 0.55, "pop": 0.55, "hip_hop": 0.95, "jazz": 0.15, "classical": 0.05, "metal": 0.00, "folk": 0.00, "primary_genre": "hip_hop"},
    {"artist": "Jay-Z",                  "rock": 0.00, "electronic": 0.25, "pop": 0.40, "hip_hop": 0.98, "jazz": 0.10, "classical": 0.00, "metal": 0.00, "folk": 0.00, "primary_genre": "hip_hop"},
    {"artist": "Burial",                 "rock": 0.05, "electronic": 0.95, "pop": 0.05, "hip_hop": 0.10, "jazz": 0.05, "classical": 0.05, "metal": 0.00, "folk": 0.00, "primary_genre": "electronic"},
    {"artist": "Four Tet",               "rock": 0.05, "electronic": 0.90, "pop": 0.20, "hip_hop": 0.10, "jazz": 0.20, "classical": 0.10, "metal": 0.00, "folk": 0.10, "primary_genre": "idm"},
    {"artist": "Sigur Ros",              "rock": 0.35, "electronic": 0.30, "pop": 0.25, "hip_hop": 0.00, "jazz": 0.05, "classical": 0.45, "metal": 0.05, "folk": 0.20, "primary_genre": "post_rock"},
    {"artist": "Godspeed You Black Emperor","rock": 0.55,"electronic": 0.20,"pop": 0.05,"hip_hop": 0.00,"jazz": 0.15,"classical": 0.35,"metal": 0.25,"folk": 0.10,"primary_genre": "post_rock"},
    {"artist": "Sufjan Stevens",         "rock": 0.30, "electronic": 0.25, "pop": 0.40, "hip_hop": 0.00, "jazz": 0.10, "classical": 0.50, "metal": 0.00, "folk": 0.70, "primary_genre": "folk"},
    {"artist": "Frank Ocean",            "rock": 0.05, "electronic": 0.40, "pop": 0.70, "hip_hop": 0.50, "jazz": 0.20, "classical": 0.10, "metal": 0.00, "folk": 0.10, "primary_genre": "r_and_b"},
    {"artist": "Prince",                 "rock": 0.40, "electronic": 0.35, "pop": 0.75, "hip_hop": 0.15, "jazz": 0.30, "classical": 0.05, "metal": 0.05, "folk": 0.05, "primary_genre": "r_and_b"},
    {"artist": "Tool",                   "rock": 0.60, "electronic": 0.15, "pop": 0.00, "hip_hop": 0.00, "jazz": 0.10, "classical": 0.20, "metal": 0.95, "folk": 0.00, "primary_genre": "metal"},
    {"artist": "Portishead",             "rock": 0.25, "electronic": 0.75, "pop": 0.25, "hip_hop": 0.15, "jazz": 0.30, "classical": 0.15, "metal": 0.00, "folk": 0.05, "primary_genre": "trip_hop"},
    {"artist": "Thom Yorke",             "rock": 0.50, "electronic": 0.80, "pop": 0.25, "hip_hop": 0.05, "jazz": 0.10, "classical": 0.15, "metal": 0.00, "folk": 0.05, "primary_genre": "art_rock"},
    {"artist": "Autechre",               "rock": 0.00, "electronic": 0.99, "pop": 0.00, "hip_hop": 0.05, "jazz": 0.10, "classical": 0.10, "metal": 0.05, "folk": 0.00, "primary_genre": "idm"},
    {"artist": "The xx",                 "rock": 0.45, "electronic": 0.60, "pop": 0.55, "hip_hop": 0.05, "jazz": 0.05, "classical": 0.05, "metal": 0.00, "folk": 0.10, "primary_genre": "indie_pop"},
    {"artist": "LCD Soundsystem",        "rock": 0.55, "electronic": 0.80, "pop": 0.40, "hip_hop": 0.05, "jazz": 0.05, "classical": 0.00, "metal": 0.00, "folk": 0.00, "primary_genre": "electronic"},
    {"artist": "Talking Heads",          "rock": 0.60, "electronic": 0.40, "pop": 0.45, "hip_hop": 0.05, "jazz": 0.10, "classical": 0.05, "metal": 0.00, "folk": 0.10, "primary_genre": "art_rock"},
    {"artist": "Scott Walker",           "rock": 0.15, "electronic": 0.10, "pop": 0.30, "hip_hop": 0.00, "jazz": 0.20, "classical": 0.50, "metal": 0.10, "folk": 0.10, "primary_genre": "art_pop"},
    {"artist": "Arca",                   "rock": 0.00, "electronic": 0.95, "pop": 0.10, "hip_hop": 0.20, "jazz": 0.05, "classical": 0.20, "metal": 0.15, "folk": 0.00, "primary_genre": "electronic"},
]

GENRE_DIMS = ["rock", "electronic", "pop", "hip_hop", "jazz", "classical", "metal", "folk"]


def gql(gigi_url: str, query: str) -> dict:
    r = requests.post(f"{gigi_url}/v1/gql", json={"query": query}, timeout=30)
    r.raise_for_status()
    return r.json()


def main():
    parser = argparse.ArgumentParser(description="Music DNA — GIGI Demo")
    parser.add_argument("--gigi", default="https://gigi-stream.fly.dev",
                        help="GIGI server URL (default: gigi-stream.fly.dev)")
    parser.add_argument("--seed", default="Radiohead",
                        help="Seed artist for SECTION + COVER queries (default: Radiohead)")
    parser.add_argument("--keep", action="store_true",
                        help="Keep the GIGI bundle after demo")
    args = parser.parse_args()

    GIGI   = args.gigi.rstrip("/")
    BUNDLE = f"music_dna_{int(time.time())}"
    SEED   = args.seed

    # Validate seed
    artist_names = [a["artist"] for a in ARTISTS]
    if SEED not in artist_names:
        close = [n for n in artist_names if SEED.lower() in n.lower()]
        if close:
            SEED = close[0]
            print(f"Using closest match: '{SEED}'")
        else:
            print(f"Unknown artist '{SEED}'. Available: {', '.join(sorted(artist_names))}")
            sys.exit(1)

    print(f"\n{'='*60}")
    print("  GIGI Demo — Music DNA")
    print(f"  Server : {GIGI}")
    print(f"  Bundle : {BUNDLE}")
    print(f"  Seed   : {SEED}")
    print(f"  Dataset: {len(ARTISTS)} artists × {len(GENRE_DIMS)} dimensions")
    print(f"{'='*60}\n")

    # ── 1. Build records ───────────────────────────────────────────
    print("[1/5] Building artist genre-vector records...")
    records = [dict(artist_id=i, **a) for i, a in enumerate(ARTISTS)]
    genres_seen = sorted({a["primary_genre"] for a in ARTISTS})
    print(f"    {len(records)} artists across {len(genres_seen)} primary genres")
    print(f"    Genres: {', '.join(genres_seen)}\n")

    # ── 2. Create bundle ───────────────────────────────────────────
    print(f"[2/5] Creating GIGI bundle '{BUNDLE}'...")
    fields = {"artist_id": "numeric", "artist": "categorical", "primary_genre": "categorical"}
    for dim in GENRE_DIMS:
        fields[dim] = "numeric"

    r = requests.post(f"{GIGI}/v1/bundles", json={
        "name": BUNDLE,
        "schema": {"fields": fields, "keys": ["artist_id"]},
    }, timeout=30)
    if r.status_code not in (200, 201):
        print(f"    Bundle create failed: {r.status_code} {r.text[:200]}")
        sys.exit(1)
    print("    Created OK\n")

    # ── 3. Ingest ──────────────────────────────────────────────────
    print(f"[3/5] Ingesting {len(records)} records...")
    t0 = time.time()
    r = requests.post(f"{GIGI}/v1/bundles/{BUNDLE}/insert",
                      json={"records": records}, timeout=30)
    r.raise_for_status()
    elapsed = time.time() - t0
    print(f"    Done in {elapsed:.3f}s\n")

    # ── 4. GQL queries ─────────────────────────────────────────────
    print("[4/5] Running GQL queries on the music manifold...\n")

    # Overall curvature of music space
    q = f"CURVATURE {BUNDLE}"
    print(f"    GQL: {q}")
    curv = gql(GIGI, q)
    k    = curv.get("value", 0)
    conf = curv.get("confidence", 0)
    print(f"    K = {k:.6f}   confidence = {conf:.4f}")
    print(f"    (K close to 0 = flat uniform space; high K = curved diverse musical manifold)\n")

    # Point query: seed artist's genre DNA
    seed_id = next(i for i, a in enumerate(ARTISTS) if a["artist"] == SEED)
    q = f"SECTION {BUNDLE} AT artist_id={seed_id}"
    print(f"    GQL: {q}")
    sec  = gql(GIGI, q)
    data = sec.get("section") or sec
    print(f"    {SEED}'s genre DNA:")
    for dim in GENRE_DIMS:
        score = data.get(dim, 0)
        bar   = "█" * int(score * 20) + "░" * (20 - int(score * 20))
        print(f"      {dim:12s} {bar} {score:.2f}")
    print()

    # COVER: find artists of the same primary genre
    seed_genre = next(a["primary_genre"] for a in ARTISTS if a["artist"] == SEED)
    q = f"COVER {BUNDLE} ON primary_genre = '{seed_genre}'"
    print(f"    GQL: {q}  ({SEED}'s genre = {seed_genre})")
    cover = gql(GIGI, q)
    cover_recs = cover.get("records", [])
    same_genre = [r.get("artist") or f"id={r.get('artist_id', '?')}" for r in cover_recs]
    print(f"    Found {cover.get('count', len(cover_recs))} artists in genre '{seed_genre}':")
    for name in same_genre[:10]:
        print(f"      - {name}")
    if len(same_genre) > 10:
        print(f"      ... and {len(same_genre) - 10} more")
    print()

    # IDM cluster
    q = f"COVER {BUNDLE} ON primary_genre = 'idm'"
    print(f"    GQL: {q}")
    idm = gql(GIGI, q)
    idm_recs = idm.get("records", [])
    idm_names = [r.get("artist") or f"id={r.get('artist_id','?')}" for r in idm_recs]
    print(f"    IDM artists: {', '.join(idm_names)}\n")

    # ── Summary ────────────────────────────────────────────────────
    print("=" * 60)
    print(f"  Bundle   : {BUNDLE}")
    print(f"  Artists  : {len(records)}")
    print(f"  K        : {k:.6f}")
    print(f"  Genres   : {len(genres_seen)}")
    print(f"  Seed     : {SEED}  ({seed_genre})")
    print()
    print("  To explore more:")
    print(f'    curl -s {GIGI}/v1/gql -H "Content-Type: application/json" \\')
    print(f"         -d '{{\"query\": \"COVER {BUNDLE} ON primary_genre = \'hip_hop\'\"}}' | python -m json.tool")
    print("=" * 60)

    if not args.keep:
        print(f"\nCleaning up bundle '{BUNDLE}'...")
        requests.delete(f"{GIGI}/v1/bundles/{BUNDLE}", timeout=10)
        print("Done.")
    else:
        print(f"\nBundle '{BUNDLE}' kept on {GIGI}")


if __name__ == "__main__":
    main()
