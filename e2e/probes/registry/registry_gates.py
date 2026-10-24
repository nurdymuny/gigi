"""Acceptance gates for the research relationship registry, run live.

Covers the gates that are properties of storage and retrieval. Gates needing
path traversal (9), structured contradiction detection (10), altitude
recomputation (3) and the executable map layer (8) are NOT covered here and are
reported as deferred rather than silently omitted.
"""

import sys

from registry_bootstrap import query, eq, call

FAILS = []


def check(gate, ok, detail):
    print(f"  [{'PASS' if ok else 'FAIL'}] {gate}: {detail}")
    if not ok:
        FAILS.append(gate)


def rel(rid):
    rows = query("research_relations", [eq("id", rid)])
    return rows[0] if rows else None


def gate1():
    """Retrieve a relation with its source and scope visible."""
    r = rel("DG-REL-DAVISMANIFOLD-PULLBACK-001")
    ev = query("research_relation_evidence", [eq("relation_id", r["id"])])
    srcs = []
    for e in ev:
        s = query("research_sources", [eq("id", e["source_id"])])
        srcs.extend(s)
    ok = (r["scope"] and r["altitude_scope"] and len(srcs) == 2
          and all(s["uri"] for s in srcs))
    check("G1 evidence + scope visible", ok,
          f"{len(srcs)} sources resolved, scope and altitude_scope non-empty")


def gate2():
    """The homonym fixture must not be promoted, and the naive method must fuse it."""
    r = rel("DG-REL-PULLBACK-HOMONYM-001")
    ok = r["established_altitude"] == "NONE" and r["lifecycle"] == "proposed"
    check("G2a homonym stays unpromoted", ok,
          f"altitude={r['established_altitude']} lifecycle={r['lifecycle']}")

    ev = query("research_relation_evidence", [eq("relation_id", r["id"])])
    check("G2b homonym has no evidence", len(ev) == 0, f"{len(ev)} evidence rows")

    # The control. If a shared token were enough, these two concepts would fuse.
    # Showing the naive method DOES fire is what makes G2a mean anything: without
    # it, G2a would pass on a corpus where nothing could have been confused.
    cs = query("research_concepts")
    names = {c["id"]: c["name"].lower() for c in cs}
    a, b = names["gigi-pullback-join"], names["differential-geometric-pullback-metric"]
    shared = set(a.split()) & set(b.split())
    check("G2c naive token overlap WOULD fuse them", "pullback" in shared,
          f"shared tokens {sorted(shared)} -- the registry refuses anyway")


def gate11():
    """Agent-asserted and unreviewed must be visible without reading history."""
    rows = query("research_relations")
    visible = all(r.get("asserted_by") and r.get("reviewer_decision") for r in rows)
    agent_unreviewed = [r["id"] for r in rows
                        if r["asserted_by_is_agent"] == "true"
                        and r["reviewer_decision"] == "unreviewed"]
    check("G11 agent attribution at retrieval", visible and len(agent_unreviewed) == 3,
          f"{len(agent_unreviewed)}/3 marked agent-asserted and unreviewed")


def gate13():
    """Direction: readable from both ends, with no stored converse."""
    rid = "DG-REL-DAVISMANIFOLD-PULLBACK-001"
    r = rel(rid)
    from_subj = query("research_relations", [eq("subject", r["subject"])])
    from_obj = query("research_relations", [eq("object", r["object"])])
    same = any(x["id"] == rid for x in from_subj) and any(x["id"] == rid for x in from_obj)
    converse = query("research_relations", [eq("subject", r["object"])])
    has_converse = any(x["object"] == r["subject"] for x in converse)
    check("G13a reachable from both ends", same,
          "found from subject and from object")
    check("G13b no stored converse row", not has_converse,
          "no row asserts the reverse direction")

    sym = rel("DG-REL-PULLBACK-HOMONYM-001")
    check("G13c symmetric type marked", sym["direction"] == "symmetric",
          f"direction={sym['direction']}")


def gate3_storage_half():
    """Retracting a source must be able to find every dependent relation."""
    sid = "DG-REF-COORDINATE-SAMENESS-001"
    call(f"/v1/bundles/research_sources/insert",
         {"records": [dict(query("research_sources", [eq("id", sid)])[0],
                           availability="retracted")]})
    s = query("research_sources", [eq("id", sid)])[0]
    deps = query("research_relation_evidence", [eq("source_id", sid)])
    dependent_relations = sorted({d["relation_id"] for d in deps})
    check("G3 retracted source finds its dependents",
          s["availability"] == "retracted" and len(dependent_relations) == 2,
          f"availability={s['availability']}, dependents={dependent_relations}")


def main():
    print("=" * 78)
    print("Registry acceptance gates (live engine)")
    print("=" * 78)
    gate1()
    gate2()
    gate11()
    gate13()
    gate3_storage_half()
    print()
    print("DEFERRED, not covered by this run:")
    print("  G9  composition state      -- needs path traversal")
    print("  G10 structured contradiction -- needs conflict detection")
    print("  G3  altitude recomputation -- storage half above only")
    print("  G8  executable map layer   -- later phase by design")
    print("  G5  restart persistence    -- run restart check separately")
    print()
    if FAILS:
        print("FAILED:", ", ".join(FAILS))
        return 1
    print("All implemented gates pass.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
