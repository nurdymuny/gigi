#!/usr/bin/env python
"""Seed the `worldfacts` bundle for the gigi-mcp demo.

Curated public-knowledge SPO triples covering math, physics, chemistry,
biology, computer science, history, astronomy, and geometry/topology.
Schema deliberately mirrors `marcella_state_beliefs` so the demo shows
the same fiber shape Marcella uses — but with safe, public data.

Usage:

    # Default: seed to whatever GIGI_URL points at (env or arg), reading
    # GIGI_API_KEY from env.
    python gigi/mcp/examples/seed_worldfacts.py

    # Override endpoint / key:
    python gigi/mcp/examples/seed_worldfacts.py --url http://localhost:8080 --key abc123

    # Drop existing worldfacts bundle first (destructive):
    python gigi/mcp/examples/seed_worldfacts.py --reset

The bundle name is hardcoded to `worldfacts`. All ~150 triples are inline
below so the data is reviewable in a single file — no external CSV, no
mystery JSON. Edit the TRIPLES list directly to add/fix entries.
"""

from __future__ import annotations

import argparse
import io
import os
import sys
from datetime import datetime, timezone
from pathlib import Path

# ─── Path setup (so we can run from a fresh checkout without pip install) ───
HERE = Path(__file__).resolve().parent
MCP_ROOT = HERE.parent
GIGI_ROOT = MCP_ROOT.parent
SDK_PATH = GIGI_ROOT / "sdk" / "python"
if str(SDK_PATH) not in sys.path:
    sys.path.insert(0, str(SDK_PATH))

# UTF-8 stdout for Windows consoles (city names, accented authors)
if hasattr(sys.stdout, "buffer"):
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")

from gigi.client import GigiClient, GigiError  # noqa: E402


BUNDLE_NAME = "worldfacts"

CREATE_STMT = (
    f"CREATE BUNDLE {BUNDLE_NAME} ("
    "  idx NUMERIC BASE,"
    "  domain CATEGORICAL BASE,"
    "  confidence NUMERIC FIBER,"
    "  subject CATEGORICAL FIBER,"
    "  predicate CATEGORICAL FIBER,"
    "  object CATEGORICAL FIBER,"
    "  time TIMESTAMP FIBER"
    ")"
)


def year_to_ms(year: int) -> int:
    """Convert an AD year to milliseconds since epoch (UTC, January 1)."""
    return int(datetime(year, 1, 1, tzinfo=timezone.utc).timestamp() * 1000)


# ─── The curated triples ────────────────────────────────────────────────────
# Each tuple is: (subject, predicate, object, year, confidence, domain)
#
# Confidence scale:
#   1.00       canonically attested, no controversy
#   0.95–0.99  well-attested, minor nuance (date approximation, joint claim)
#   0.85–0.94  notable controversy or uncertainty (priority disputes, independent
#              discovery)
#   below 0.85 traditional/legendary attribution — used sparingly
#
# Predicates used (small, consistent set):
#   co-invented, proved, discovered, formulated, introduced, published,
#   founded, developed, named, predicted, measured, described, won,
#   wrote, built, signed, started, ended, led, popularized
# ────────────────────────────────────────────────────────────────────────────

TRIPLES = [

    # ─── MATH (30) ────────────────────────────────────────────────────────
    ("Pythagoras",  "described",   "Pythagorean theorem",                530, 0.70, "math"),
    ("Euclid",      "wrote",       "Elements",                           300, 1.00, "math"),
    ("Archimedes",  "described",   "method of exhaustion",               250, 0.90, "math"),
    ("Khwarizmi",   "founded",     "algebra",                            820, 0.95, "math"),
    ("Fibonacci",   "introduced",  "Fibonacci sequence to Europe",       1202, 0.95, "math"),
    ("Descartes",   "introduced",  "Cartesian coordinates",              1637, 1.00, "math"),
    ("Fermat",      "proved",      "Fermat's little theorem",            1640, 0.98, "math"),
    ("Pascal",      "developed",   "Pascal's triangle",                  1653, 0.95, "math"),
    ("Newton",      "co-invented", "calculus",                           1666, 0.99, "math"),
    ("Leibniz",     "co-invented", "calculus",                           1675, 0.99, "math"),
    ("Euler",       "founded",     "graph theory",                       1736, 1.00, "math"),
    ("Euler",       "proved",      "Euler's identity",                   1748, 1.00, "math"),
    ("Gauss",       "proved",      "fundamental theorem of algebra",     1799, 0.95, "math"),
    ("Galois",      "founded",     "group theory",                       1832, 0.95, "math"),
    ("Boole",       "introduced",  "Boolean algebra",                    1854, 1.00, "math"),
    ("Cantor",      "founded",     "set theory",                         1874, 1.00, "math"),
    ("Cantor",      "proved",      "different sizes of infinity",        1891, 1.00, "math"),
    ("Hilbert",     "presented",   "Hilbert's 23 problems",              1900, 1.00, "math"),
    ("Russell",     "discovered",  "Russell's paradox",                  1901, 1.00, "math"),
    ("Noether",     "proved",      "Noether's theorem",                  1915, 1.00, "math"),
    ("Banach",      "introduced",  "Banach spaces",                      1932, 1.00, "math"),
    ("Gödel",       "proved",      "incompleteness theorem",             1931, 1.00, "math"),
    ("Turing",      "described",   "Turing machine",                     1936, 1.00, "math"),
    ("Turing",      "proved",      "halting problem is undecidable",     1936, 1.00, "math"),
    ("Shannon",     "founded",     "information theory",                 1948, 1.00, "math"),
    ("Erdős",       "founded",     "modern combinatorics",               1950, 0.95, "math"),
    ("Mandelbrot",  "introduced",  "fractal geometry",                   1975, 0.98, "math"),
    ("Mandelbrot",  "named",       "Mandelbrot set",                     1980, 0.99, "math"),
    ("Wiles",       "proved",      "Fermat's last theorem",              1994, 1.00, "math"),
    ("Perelman",    "proved",      "Poincaré conjecture",                2003, 1.00, "math"),

    # ─── GEOMETRY / TOPOLOGY (15) — relevant to Davis Geometric framework ─
    ("Euclid",      "stated",      "parallel postulate",                 300, 1.00, "geometry"),
    ("Gauss",       "proved",      "Theorema Egregium",                  1827, 1.00, "geometry"),
    ("Lobachevsky", "introduced",  "hyperbolic geometry",                1829, 1.00, "geometry"),
    ("Bolyai",      "introduced",  "hyperbolic geometry",                1832, 1.00, "geometry"),
    ("Möbius",      "described",   "Möbius strip",                       1858, 1.00, "geometry"),
    ("Riemann",     "introduced",  "Riemannian manifold",                1854, 1.00, "geometry"),
    ("Klein",       "described",   "Klein bottle",                       1882, 1.00, "geometry"),
    ("Klein",       "introduced",  "Erlangen program",                   1872, 1.00, "geometry"),
    ("Poincaré",    "founded",     "algebraic topology",                 1895, 1.00, "geometry"),
    ("Brouwer",     "proved",      "fixed point theorem",                1910, 1.00, "geometry"),
    ("Hopf",        "introduced",  "Hopf fibration",                     1931, 1.00, "geometry"),
    ("Chern",       "introduced",  "Chern classes",                      1946, 1.00, "geometry"),
    ("Atiyah",      "proved",      "Atiyah-Singer index theorem",        1963, 1.00, "geometry"),
    ("Berry",       "discovered",  "Berry phase",                        1984, 1.00, "geometry"),
    ("Donaldson",   "discovered",  "exotic 4-manifolds",                 1983, 1.00, "geometry"),

    # ─── PHYSICS (25) ─────────────────────────────────────────────────────
    ("Galileo",     "described",   "law of inertia",                     1632, 0.95, "physics"),
    ("Kepler",      "formulated",  "laws of planetary motion",           1609, 1.00, "physics"),
    ("Newton",      "formulated",  "law of universal gravitation",       1687, 1.00, "physics"),
    ("Newton",      "formulated",  "three laws of motion",               1687, 1.00, "physics"),
    ("Carnot",      "founded",     "thermodynamics",                     1824, 1.00, "physics"),
    ("Faraday",     "discovered",  "electromagnetic induction",          1831, 1.00, "physics"),
    ("Joule",       "demonstrated","conservation of energy",             1843, 0.97, "physics"),
    ("Maxwell",     "formulated",  "Maxwell's equations",                1865, 1.00, "physics"),
    ("Boltzmann",   "founded",     "statistical mechanics",              1872, 1.00, "physics"),
    ("Planck",      "introduced",  "quantum hypothesis",                 1900, 1.00, "physics"),
    ("Einstein",    "published",   "special relativity",                 1905, 1.00, "physics"),
    ("Einstein",    "published",   "general relativity",                 1915, 1.00, "physics"),
    ("Rutherford",  "discovered",  "atomic nucleus",                     1911, 1.00, "physics"),
    ("Bohr",        "proposed",    "Bohr atomic model",                  1913, 1.00, "physics"),
    ("Heisenberg",  "formulated",  "uncertainty principle",              1927, 1.00, "physics"),
    ("Schrödinger", "formulated",  "Schrödinger equation",               1926, 1.00, "physics"),
    ("Dirac",       "formulated",  "Dirac equation",                     1928, 1.00, "physics"),
    ("Pauli",       "formulated",  "exclusion principle",                1925, 1.00, "physics"),
    ("Fermi",       "built",       "first nuclear reactor",              1942, 1.00, "physics"),
    ("Feynman",     "introduced",  "Feynman diagrams",                   1948, 1.00, "physics"),
    ("Higgs",       "predicted",   "Higgs boson",                        1964, 1.00, "physics"),
    ("Hawking",     "predicted",   "Hawking radiation",                  1974, 1.00, "physics"),
    ("Penrose",     "described",   "Penrose tiling",                     1974, 1.00, "physics"),
    ("Witten",      "developed",   "string theory unification",          1995, 0.97, "physics"),
    ("LIGO",        "detected",    "gravitational waves",                2015, 1.00, "physics"),

    # ─── CHEMISTRY (12) ───────────────────────────────────────────────────
    ("Lavoisier",   "formulated",  "conservation of mass",               1789, 1.00, "chemistry"),
    ("Dalton",      "introduced",  "atomic theory",                      1808, 0.98, "chemistry"),
    ("Mendeleev",   "introduced",  "periodic table",                     1869, 1.00, "chemistry"),
    ("Curie",       "discovered",  "radium",                             1898, 1.00, "chemistry"),
    ("Curie",       "discovered",  "polonium",                           1898, 1.00, "chemistry"),
    ("Rutherford",  "discovered",  "alpha and beta radiation",           1899, 0.97, "chemistry"),
    ("Haber",       "developed",   "Haber process",                      1909, 1.00, "chemistry"),
    ("Pauling",     "described",   "nature of the chemical bond",        1939, 1.00, "chemistry"),
    ("Hodgkin",     "determined",  "structure of insulin",               1969, 1.00, "chemistry"),
    ("Khorana",     "synthesized", "first artificial gene",              1972, 0.97, "chemistry"),
    ("Mullis",      "invented",    "polymerase chain reaction",          1983, 1.00, "chemistry"),
    ("Sharpless",   "developed",   "click chemistry",                    2001, 0.97, "chemistry"),

    # ─── BIOLOGY (13) ─────────────────────────────────────────────────────
    ("Linnaeus",    "founded",     "biological classification",          1735, 1.00, "biology"),
    ("Darwin",      "published",   "On the Origin of Species",           1859, 1.00, "biology"),
    ("Wallace",     "co-formulated","natural selection",                 1858, 0.97, "biology"),
    ("Mendel",      "founded",     "genetics",                           1866, 1.00, "biology"),
    ("Pasteur",     "developed",   "germ theory of disease",             1862, 1.00, "biology"),
    ("Fleming",     "discovered",  "penicillin",                         1928, 1.00, "biology"),
    ("McClintock",  "discovered",  "genetic transposition",              1948, 1.00, "biology"),
    ("Avery",       "demonstrated","DNA carries genetic information",    1944, 0.97, "biology"),
    ("Franklin",    "imaged",      "DNA double helix structure",         1952, 0.97, "biology"),
    ("Watson",      "co-described","DNA double helix",                   1953, 1.00, "biology"),
    ("Crick",       "co-described","DNA double helix",                   1953, 1.00, "biology"),
    ("Salk",        "developed",   "polio vaccine",                      1955, 1.00, "biology"),
    ("Tu Youyou",   "discovered",  "artemisinin",                        1972, 1.00, "biology"),

    # ─── COMPUTER SCIENCE (20) ────────────────────────────────────────────
    ("Babbage",     "designed",    "Difference Engine",                  1822, 1.00, "cs"),
    ("Lovelace",    "wrote",       "first computer algorithm",           1843, 1.00, "cs"),
    ("Boole",       "founded",     "logical algebra of computing",       1854, 0.95, "cs"),
    ("von Neumann", "described",   "stored-program architecture",        1945, 1.00, "cs"),
    ("Hopper",      "developed",   "first compiler",                     1952, 1.00, "cs"),
    ("McCarthy",    "coined",      "artificial intelligence",            1956, 1.00, "cs"),
    ("McCarthy",    "designed",    "Lisp programming language",          1958, 1.00, "cs"),
    ("Backus",      "designed",    "FORTRAN",                            1957, 1.00, "cs"),
    ("Dijkstra",    "described",   "Dijkstra's shortest path algorithm", 1959, 1.00, "cs"),
    ("Engelbart",   "demonstrated","mouse and hypertext",                1968, 1.00, "cs"),
    ("Ritchie",     "designed",    "C programming language",             1972, 1.00, "cs"),
    ("Thompson",    "co-designed", "Unix",                               1969, 1.00, "cs"),
    ("Knuth",       "wrote",       "The Art of Computer Programming",    1968, 1.00, "cs"),
    ("Cerf",        "co-designed", "TCP/IP",                             1974, 1.00, "cs"),
    ("Berners-Lee", "invented",    "World Wide Web",                     1989, 1.00, "cs"),
    ("Stroustrup",  "designed",    "C++",                                1985, 1.00, "cs"),
    ("Torvalds",    "released",    "Linux",                              1991, 1.00, "cs"),
    ("Page",        "co-developed","PageRank",                           1996, 1.00, "cs"),
    ("LeCun",       "developed",   "convolutional neural networks",      1989, 0.97, "cs"),
    ("Hassabis",    "led",         "AlphaFold protein folding",          2020, 1.00, "cs"),

    # ─── ASTRONOMY (12) ───────────────────────────────────────────────────
    ("Copernicus",  "published",   "heliocentric model",                 1543, 1.00, "astronomy"),
    ("Galileo",     "discovered",  "moons of Jupiter",                   1610, 1.00, "astronomy"),
    ("Cassini",     "discovered",  "Saturn ring division",               1675, 1.00, "astronomy"),
    ("Halley",      "predicted",   "return of Halley's Comet",           1705, 1.00, "astronomy"),
    ("Herschel",    "discovered",  "Uranus",                             1781, 1.00, "astronomy"),
    ("Tombaugh",    "discovered",  "Pluto",                              1930, 1.00, "astronomy"),
    ("Lemaître",    "proposed",    "Big Bang theory",                    1927, 1.00, "astronomy"),
    ("Hubble",      "measured",    "galactic redshift",                  1929, 1.00, "astronomy"),
    ("Penzias",     "co-detected", "cosmic microwave background",        1965, 1.00, "astronomy"),
    ("Bell Burnell","discovered",  "pulsars",                            1967, 1.00, "astronomy"),
    ("Rubin",       "demonstrated","galaxy rotation curves",             1970, 1.00, "astronomy"),
    ("Sagan",       "popularized", "astronomy",                          1980, 0.97, "astronomy"),

    # ─── HISTORY (25, AD only) ────────────────────────────────────────────
    ("Constantine", "legalized",   "Christianity in Roman Empire",       313, 0.98, "history"),
    ("Charlemagne", "founded",     "Carolingian Empire",                 800, 1.00, "history"),
    ("Genghis Khan","founded",     "Mongol Empire",                      1206, 1.00, "history"),
    ("King John",   "signed",      "Magna Carta",                        1215, 1.00, "history"),
    ("Gutenberg",   "invented",    "movable-type printing press",        1440, 1.00, "history"),
    ("Constantinople","fell to",   "Ottoman Empire",                     1453, 1.00, "history"),
    ("Columbus",    "led",         "voyage to the Americas",             1492, 1.00, "history"),
    ("Vespucci",    "led",         "voyages naming the Americas",        1499, 0.95, "history"),
    ("Magellan",    "led",         "first circumnavigation expedition",  1519, 1.00, "history"),
    ("Luther",      "published",   "Ninety-five Theses",                 1517, 1.00, "history"),
    ("Continental Congress","signed","Declaration of Independence",     1776, 1.00, "history"),
    ("French people","started",    "French Revolution",                  1789, 1.00, "history"),
    ("Industrial Revolution","began","in Britain",                       1760, 0.97, "history"),
    ("Napoleon",    "crowned",     "emperor of France",                  1804, 1.00, "history"),
    ("Lincoln",     "signed",      "Emancipation Proclamation",          1863, 1.00, "history"),
    ("Allied powers","ended",      "World War I",                        1918, 1.00, "history"),
    ("Gandhi",      "led",         "Salt March",                         1930, 1.00, "history"),
    ("Allied powers","ended",      "World War II",                       1945, 1.00, "history"),
    ("UN founders", "founded",     "United Nations",                     1945, 1.00, "history"),
    ("MLK",         "delivered",   "I Have a Dream speech",              1963, 1.00, "history"),
    ("Apollo 11",   "landed",      "first humans on the Moon",           1969, 1.00, "history"),
    ("Berlin",      "fell with",   "Berlin Wall",                        1989, 1.00, "history"),
    ("Soviet Union","dissolved",   "into independent republics",         1991, 1.00, "history"),
    ("Mandela",     "became",      "president of South Africa",          1994, 1.00, "history"),
    ("UK women",    "won",         "right to vote",                      1918, 0.97, "history"),
]


# ─── Runner ─────────────────────────────────────────────────────────────────


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument(
        "--url",
        default=os.environ.get("GIGI_URL", "https://gigi-stream.fly.dev"),
        help="GIGI server URL (default: $GIGI_URL or https://gigi-stream.fly.dev)",
    )
    parser.add_argument(
        "--key",
        default=os.environ.get("GIGI_API_KEY"),
        help="API key (default: $GIGI_API_KEY). Required for write ops.",
    )
    parser.add_argument(
        "--reset",
        action="store_true",
        help=f"Drop bundle '{BUNDLE_NAME}' first if it exists. DESTRUCTIVE.",
    )
    args = parser.parse_args()

    if not args.key:
        print("✗ No API key. Set GIGI_API_KEY env var or pass --key.", file=sys.stderr)
        return 2

    print("=" * 64)
    print(f"  seed_worldfacts.py")
    print(f"  endpoint: {args.url}")
    print(f"  bundle:   {BUNDLE_NAME}")
    print(f"  triples:  {len(TRIPLES)}")
    print(f"  domains:  {sorted(set(t[5] for t in TRIPLES))}")
    print(f"  reset:    {args.reset}")
    print("=" * 64)

    client = GigiClient(url=args.url, api_key=args.key)

    # Step 1: optional reset
    if args.reset:
        print(f"\nDropping existing '{BUNDLE_NAME}' (if present)...")
        try:
            client.drop_bundle(BUNDLE_NAME)
            print("  → dropped")
        except GigiError as e:
            print(f"  → drop failed (likely not present): {e}")

    # Step 2: create the bundle
    print(f"\nCreating bundle...")
    try:
        client.gql(CREATE_STMT)
        print("  → created")
    except GigiError as e:
        msg = str(e).lower()
        if "exist" in msg or "already" in msg:
            print(f"  → already exists (continuing): {e}")
        else:
            print(f"  ✗ create failed: {e}", file=sys.stderr)
            return 3

    # Step 3: assemble records
    records = []
    for idx, (subject, predicate, obj, year, confidence, domain) in enumerate(TRIPLES):
        records.append({
            "idx": idx,
            "domain": domain,
            "confidence": float(confidence),
            "subject": subject,
            "predicate": predicate,
            "object": obj,
            "time": year_to_ms(year),
        })

    # Step 4: bulk insert
    print(f"\nInserting {len(records)} records...")
    try:
        result = client.insert(BUNDLE_NAME, records)
        print(f"  → insert returned: {result}")
    except GigiError as e:
        print(f"  ✗ insert failed: {e}", file=sys.stderr)
        return 4

    # Step 5: verify
    print(f"\nVerifying...")
    try:
        count = client.count(BUNDLE_NAME)
        print(f"  → bundle '{BUNDLE_NAME}' now has {count} records")
    except GigiError as e:
        print(f"  ✗ count failed: {e}", file=sys.stderr)
        return 5

    # Step 6: sample query
    print(f"\nSample query: COVER {BUNDLE_NAME} WHERE subject = 'Newton'")
    try:
        sample = client.gql(f"COVER {BUNDLE_NAME} WHERE subject = 'Newton'")
        print(f"  → {sample}")
    except GigiError as e:
        print(f"  (sample query failed: {e})")

    print("\n" + "=" * 64)
    print(f"  done. {BUNDLE_NAME} ready for the demo.")
    print("=" * 64)
    return 0


if __name__ == "__main__":
    sys.exit(main())
