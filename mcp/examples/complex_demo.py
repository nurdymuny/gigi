"""Show what a complex GQL query looks like — raw response, no LLM postprocessing.

The point: each result here would be 'coherent' to someone who reads tables and
knows what curvature/spectral-gap mean. The structure IS the answer.
"""

from __future__ import annotations

import io
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "sdk" / "python"))
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")

from gigi.client import GigiClient, GigiError

client = GigiClient("http://localhost:3142", api_key="dev-local")


def heading(s: str) -> None:
    print()
    print("━" * 70)
    print(f"  {s}")
    print("━" * 70)


def show_query(label: str, query: str) -> None:
    print(f"\n  Q: {label}")
    print(f"  ┌─ GQL ─────────────────────────────────────────────")
    print(f"  │  {query}")
    print(f"  └───────────────────────────────────────────────────")
    try:
        result = client.gql(query)
        # Print a structured response
        if isinstance(result, dict):
            count = result.get("count")
            rows = result.get("rows", [])
            if rows:
                print(f"  ┌─ FIBER RESPONSE ── count: {count}")
                # Print as a table
                for r in rows[:12]:
                    if isinstance(r, dict):
                        subj = str(r.get("subject", ""))[:22].ljust(22)
                        pred = str(r.get("predicate", ""))[:14].ljust(14)
                        obj = str(r.get("object", ""))[:36].ljust(36)
                        dom = str(r.get("domain", ""))[:10]
                        print(f"  │ {subj} {pred} {obj} ({dom})")
                    else:
                        print(f"  │ {r}")
                if len(rows) > 12:
                    print(f"  │ ... and {len(rows) - 12} more")
                print(f"  └───────────────────────────────────────────────────")
            else:
                # Non-row result — print as JSON
                print(f"  ┌─ FIBER RESPONSE")
                print(f"  │ {json.dumps(result, indent=2, default=str)[:400]}")
                print(f"  └───────────────────────────────────────────────────")
        else:
            print(f"  ┌─ FIBER RESPONSE")
            print(f"  │ {result}")
            print(f"  └───────────────────────────────────────────────────")
    except GigiError as e:
        print(f"  ✗ {e}")


# ─── 1. Compound WHERE ──────────────────────────────────────────────────

heading("1. COMPOUND WHERE — 'math theorems proved after 1900'")
print("""
  Multi-condition filter — domain AND predicate AND time range. Same shape
  as SQL. The response is a table; no LLM needed to format it.""")

# 1900 in ms since epoch (UTC) = -2208988800000
show_query(
    "All math results proved after 1900",
    "COVER worldfacts WHERE domain = 'math' AND predicate = 'proved' AND time > -2208988800000",
)


# ─── 2. DISTINCT — what's possible ──────────────────────────────────────

heading("2. DISTINCT — 'what predicates exist in this corpus?'")
print("""
  Schema-level introspection: what *kinds* of relationships are stored?
  This is what an LLM (or a human, or a downstream tool) would ask before
  composing more sophisticated queries.""")

try:
    result = client.distinct("worldfacts", "predicate")
    print(f"\n  → {len(result)} distinct predicates:")
    for p in sorted(result):
        print(f"      • {p}")
except GigiError as e:
    print(f"  ✗ {e}")


# ─── 3. Count via aggregate ──────────────────────────────────────────────

heading("3. AGGREGATE — 'how many facts per domain?'")
print("""
  GIGI's aggregate() method groups records by a base/fiber field and counts
  (or other reduction). This is the kind of summary that's *already* coherent
  as a number table — Claude would just read it back, not synthesize anything.""")

try:
    result = client.aggregate("worldfacts", group_by="domain", field="confidence")
    groups = result.get("groups", {})
    print(f"\n  → {len(groups)} group(s):")
    for dom, stats in sorted(groups.items(), key=lambda kv: -kv[1].get("count", 0)):
        c = stats.get("count", "?")
        avg = stats.get("avg", 0)
        print(f"      {dom:14} count={c:>3}   avg_confidence={avg:.3f}")
except GigiError as e:
    print(f"  ✗ {e}")


# ─── 4. The geometric verbs — fiber properties of the BUNDLE itself ─────

heading("4. GEOMETRIC PROPERTIES — properties of the corpus as a fiber bundle")
print("""
  Here's where GIGI gets weird in a beautiful way. Beyond record-level
  queries, GIGI exposes properties of the bundle *itself* — curvature,
  spectral gap, Betti numbers, entropy. These are not records; they're
  scalars describing how the bundle is shaped.

  For worldfacts specifically: curvature ≈ how 'tense' the SPO graph is,
  spectral gap ≈ how connected its clustering is, entropy ≈ how varied
  the values are. These are coherent answers on their own — no LLM
  needed to interpret a single scalar.""")

for name, method in [
    ("Curvature K",  client.curvature),
    ("Spectral",     client.spectral),
    ("Bundle stats", client.stats),
]:
    try:
        r = method("worldfacts")
        print(f"\n  Q: {name} of the worldfacts bundle")
        print(f"  → {json.dumps(r, indent=2, default=str)[:500]}")
    except GigiError as e:
        print(f"  ✗ {name}: {e}")


# ─── 5. The killer: 'what facts CITE Riemann?' (cross-reference) ────────

heading("5. CROSS-REFERENCE — facts whose object references Riemann's name")
print("""
  The interesting thing about SPO triples: the same name can appear as
  a *subject* (Riemann did X) or inside an *object* (Y is a Riemannian
  manifold). The query below catches both, by filtering on substring.""")

# Note: depends on whether GIGI's parser supports LIKE/contains.
# Try a few syntaxes.
for q in [
    "COVER worldfacts WHERE object CONTAINS 'Riemann'",
    "COVER worldfacts WHERE object LIKE '%Riemann%'",
    "COVER worldfacts WHERE subject = 'Riemann'",  # fallback to exact match
]:
    show_query(q.split()[0] + " variant", q)


print()
print("━" * 70)
print("  done")
print("━" * 70)
