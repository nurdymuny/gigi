"""Acceptance gates for the research relationship registry, run live.

Every gate here exercises a decision that could have gone the other way. A check
that a record inserted as `proposed` is still `proposed` is not a gate -- nothing
tried to change it -- and an earlier version of gate 2 made exactly that mistake.
"""

import sys

from registry_bootstrap import eq, query, write_verified
from registry_paths import (UNCHECKED, contradictions, possible_conflicts,
                            record_composition_review, traverse)
from registry_review import promote, relation, requirement_met, view

FAILS = []


def check(gate, ok, detail):
    print(f"  [{'PASS' if ok else 'FAIL'}] {gate}: {detail}")
    if not ok:
        FAILS.append(gate)


# ── Gate 1 ───────────────────────────────────────────────────────────────────

def gate1():
    v = view("DG-REL-DAVISMANIFOLD-PULLBACK-001")
    ok = (len(v["sources"]) == 2 and all(s["locator"] for s in v["sources"])
          and v["relation"]["scope"] and v["relation"]["altitude_scope"])
    check("G1 evidence, locators and scope visible", ok,
          f"{len(v['sources'])} sources, each with a locator")


# ── Gate 2: promotion must actually refuse ───────────────────────────────────

def gate2():
    rid = "DG-REL-PULLBACK-HOMONYM-001"
    before = relation(rid)["established_altitude"]
    ok, reason = promote(rid, "RESONANCE")
    after = relation(rid)["established_altitude"]
    check("G2a promotion of the homonym is REFUSED", (not ok) and after == before,
          f"refused: {reason}")

    # The control. If promote() simply always refused, G2a would pass while
    # testing nothing. A relation that HAS the evidence must go through.
    good = "DG-REL-DAVISMANIFOLD-PULLBACK-001"
    before_good = relation(good)["established_altitude"]
    ok2, reason2 = promote(good, "RESONANCE")
    after_good = relation(good)["established_altitude"]
    check("G2b promotion changes stored NONE to RESONANCE",
          before_good == "NONE" and ok2 and after_good == "RESONANCE",
          f"accepted: {reason2}")

    # And the next rung up must still refuse, so success is not unconditional.
    ok3, reason3 = promote(good, "STRUCTURAL")
    check("G2c the same relation is refused STRUCTURAL", not ok3, reason3)

    # Scope note: this demonstrates a LEXICAL matcher fires on the shared token.
    # It does not test an embedding matcher, and does not claim to.
    cs = {c["id"]: c["name"].lower() for c in query("research_concepts")}
    shared = set(cs["gigi-pullback-join"].split()) & \
        set(cs["differential-geometric-pullback-metric"].split())
    check("G2d a lexical matcher WOULD fuse them", "pullback" in shared,
          f"shared token {sorted(shared)}; registry refused anyway (embeddings untested)")


# ── Gate 4 / 12: retraction must change what a reader sees ───────────────────

def gate4():
    sid = "DG-REF-COORDINATE-SAMENESS-001"
    rid = "DG-REL-POLAR-SAMENESS-001"

    before = view(rid)
    check("G4a supported before retraction", not before["needs_review"],
          "no flags raised")

    src = dict(query("research_sources", [eq("id", sid)])[0])
    write_verified("research_sources", dict(src, availability="retracted"))

    after = view(rid)
    named = any("RETRACTED" in f for f in after["flags"])
    check("G4b reader sees the retraction", after["needs_review"] and named,
          after["flags"][0] if after["flags"] else "no flags")
    paths = traverse("classical-polar-coordinate-map", 1)
    check("G4d path readers see the same retraction", len(paths) == 1
          and paths[0]["needs_review"] and paths[0]["edges"][0]["needs_review"]
          and paths[0]["edges"][0]["flags"] == after["flags"],
          "path and direct lookup carry identical warnings")

    # Unavailable is not retracted: access changed, validity did not.
    write_verified("research_sources", dict(src, availability="unavailable"))
    unav = view(rid)
    check("G4c unavailable does NOT trigger review",
          not unav["needs_review"] and any("unavailable" in f for f in unav["flags"]),
          "flagged as unreachable, altitude unchanged")

    write_verified("research_sources", src)


# ── Gate 9: paths stay paths ─────────────────────────────────────────────────

def gate9():
    paths = [p for p in traverse("oriented-heading-S1", 3) if p["length"] == 2]
    check("G9a the two-edge chain is found", len(paths) == 1,
          f"{len(paths)} two-edge path(s)")
    if not paths:
        return
    p = paths[0]

    check("G9b every edge returned intact", len(p["edges"]) == 2 and
          all(e["assumptions"] is not None for e in p["edges"]),
          "per-edge assumptions preserved")

    # The structural assertion: nothing was summarised into a new claim.
    forbidden = {"derived_relation", "merged_assumptions", "aggregate_altitude",
                 "combined_scope"}
    check("G9c no derived relation or merged assumptions",
          not (forbidden & set(p.keys())),
          "path carries edges only")

    check("G9d machine assigns composition_unchecked",
          p["composition_state"] == UNCHECKED, p["composition_state"])

    # A reviewer rules, and the ruling must be found again.
    record_composition_review(
        p["path_signature"], "incompatible",
        "RP^1 -> S^1 has no continuous section; the orientation the second edge "
        "requires is absent, not merely degraded.",
        "", "registry probe", True)
    again = [q for q in traverse("oriented-heading-S1", 3)
             if q["path_signature"] == p["path_signature"]][0]
    check("G9e reviewer ruling persists", again["composition_state"] == "incompatible",
          again["composition_reason"][:58] + "...")

    # Control. Ruling on one path must not touch any other, and the graph has a
    # second, genuinely composable two-edge chain so this is not vacuous --
    # all() over an empty list would have been true and tested nothing.
    others = [q for q in traverse("davis-manifold-metric-construction", 3)
              if q["length"] > 1 and q["path_signature"] != p["path_signature"]]
    check("G9f ruling does not leak to other paths",
          len(others) >= 1 and all(q["composition_state"] == UNCHECKED for q in others),
          f"{len(others)} other multi-edge path(s), still unchecked")

    # The rule stated directly: a machine may only ever assign unchecked.
    try:
        record_composition_review(p["path_signature"], UNCHECKED, "", "", "probe", True)
        refused = False
    except ValueError:
        refused = True
    check("G9g 'unchecked' cannot be recorded as a ruling", refused,
          "it is the machine default, not a reviewer decision")


# ── Gate 10: structured contradiction, both halves ───────────────────────────

def gate10():
    found = contradictions()
    ids = {tuple(c["claim_ids"]) for c in found}

    check("G10a incompatible typed assertions ARE flagged",
          ids == {("CLM-007", "CLM-008"), ("CLM-004", "CLM-005")},
          f"{len(found)} conflict group(s): {sorted(ids)}")

    flagged_claims = {c for grp in ids for c in grp}
    check("G10b different DOMAIN is not a contradiction",
          "CLM-003" not in flagged_claims,
          "det J = 0 at r = 0 coexists with det J = r on r > 0")
    check("G10c different CONDITIONS is not a contradiction",
          "CLM-006" not in flagged_claims,
          "a dilation changing arc length does not conflict with rotation invariance")

    # The control: those non-contradictions must still be visible as something,
    # or the detector could pass by ignoring them entirely.
    pc = {c["quantity"] for c in possible_conflicts()}
    check("G10d non-contradictions surface for review instead",
          {"det J", "induced arc length"} <= pc,
          f"routed for review: {sorted(pc)}")
    pending = {tuple(c["claim_ids"]) for c in possible_conflicts()}
    check("G10e symbolic expressions require review",
          ("CLM-001", "CLM-002") in pending and "CLM-001" not in flagged_claims,
          "r versus 1 is not proved contradictory by comparing text")


def main():
    FAILS.clear()
    print("=" * 78)
    print("Registry acceptance gates (live engine)")
    print("=" * 78)
    for g in (gate1, gate2, gate4, gate9, gate10):
        g()
    print()
    print("NOT IMPLEMENTED, stated rather than omitted:")
    print("  G8  executable map layer -- later phase by design")
    print("Restart persistence is checked read-only by verify_registry_live.py.")
    print()
    if FAILS:
        print("FAILED:", ", ".join(FAILS))
        return 1
    print("All implemented gates pass.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
