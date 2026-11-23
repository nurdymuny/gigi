"""Promotion and retraction-aware retrieval.

Promotion is the decision the registry exists to get right. Altitude is earned
from evidence records and is never implied by the relation's name, so
`promote()` inspects the evidence and refuses with a named reason when the
requirement is not met.

Retrieval is the other half. Finding the relations that rest on a retracted
source is useless if a reader is still shown those relations as though they were
supported, so `view()` carries the flags a reader must see.
"""

from registry_bootstrap import eq, query, write_verified

ALTITUDES = ["NONE", "INCIDENCE", "RESONANCE", "STRUCTURAL"]
VALID_SOURCE_STATES = {"available", "unavailable", "restricted"}


def nonblank(value):
    return isinstance(value, str) and bool(value.strip())


def relation(rid):
    rows = query("research_relations", [eq("id", rid)])
    return rows[0] if rows else None


def evidence_for(rid):
    return query("research_relation_evidence", [eq("relation_id", rid)])


def checks_for(rid):
    return query("research_checks", [eq("relation_id", rid)])


def source(sid):
    rows = query("research_sources", [eq("id", sid)])
    return rows[0] if rows else None


# ── Promotion ────────────────────────────────────────────────────────────────

def requirement_met(rid, target):
    """Does the stored evidence earn `target`? Returns (bool, reason)."""
    rel = relation(rid)
    if rel is None:
        return False, f"no such relation {rid}"
    if target not in ALTITUDES:
        return False, f"{target} is not an altitude"

    ev = evidence_for(rid)
    # Access and validity are different. Missing/unknown sources and absent
    # locators cannot establish support; unavailable sources are not retracted.
    live = [e for e in ev
            if (source(e["source_id"]) or {}).get("availability") in VALID_SOURCE_STATES
            and nonblank(e.get("locator"))]

    if target == "NONE":
        return True, "no requirement"

    if target == "INCIDENCE":
        if not any(e["locator"] for e in live):
            return False, "no existing non-retracted source with a nonblank locator"
        return True, "a live source locates the claim"

    if target == "RESONANCE":
        sides = {e["locates_mechanism_for"] for e in live if e["locates_mechanism_for"]}
        if "subject" not in sides or "object" not in sides:
            have = sorted(sides) or ["neither side"]
            return False, ("the shared mechanism must be located on BOTH sides; "
                           f"evidence locates it for {', '.join(have)}")
        return True, "mechanism located on both sides"

    if target == "STRUCTURAL":
        if not nonblank(rel["altitude_scope"]):
            return False, "STRUCTURAL requires an explicit altitude_scope"
        if not live:
            return False, "STRUCTURAL requires an existing non-retracted source with a nonblank locator"
        good = [c for c in checks_for(rid)
                if c["result"] == "pass" and c["control_ran"] == "true"
                and c["control_failed_as_required"] == "true"]
        if not good:
            return False, ("STRUCTURAL requires a check whose falsifying control "
                           "was run and did fail; none found")
        return True, "check with a working negative control"

    return False, "unreachable"


def promote(rid, target):
    """Attempt a promotion. Refuses rather than raising; returns (ok, reason)."""
    ok, reason = requirement_met(rid, target)
    if not ok:
        return False, reason
    rel = dict(relation(rid))
    rel["established_altitude"] = target
    try:
        write_verified("research_relations", rel)
    except (RuntimeError, OSError) as error:
        return False, str(error)
    return True, reason


# ── Retraction-aware retrieval ───────────────────────────────────────────────

def view(rid):
    """What a reader must be shown, including why it may not be trustworthy."""
    rel = relation(rid)
    if rel is None:
        raise ValueError(f"no such relation {rid}")
    flags, sources = [], []
    needs_review = False
    for e in evidence_for(rid):
        s = source(e["source_id"])
        if s is None:
            flags.append(f"evidence {e['source_id']} is missing from the registry")
            needs_review = True
        availability = s.get("availability") if s else "missing"
        sources.append({"id": e["source_id"], "title": s.get("title", "") if s else "",
                        "availability": availability,
                        "locator": e["locator"], "kind": e["evidence_kind"]})
        if not nonblank(e.get("locator")):
            flags.append(f"evidence {e['id']} has no nonblank locator")
            needs_review = True
        if availability == "retracted":
            needs_review = True
            flags.append(f"supporting source {s['id']} was RETRACTED; "
                         "this relation needs review")
        elif availability == "unavailable":
            flags.append(f"supporting source {s['id']} is temporarily unavailable; "
                         "altitude is unchanged")
        elif availability == "restricted":
            flags.append(f"supporting source {s['id']} is marked restricted; "
                         "this metadata API does not enforce access permissions")
        elif availability not in VALID_SOURCE_STATES and availability != "missing":
            flags.append(f"supporting source {e['source_id']} has unknown availability {availability!r}")
            needs_review = True

    still_earned, why = requirement_met(rid, rel["established_altitude"])
    if not still_earned:
        needs_review = True
        flags.append(f"stored altitude {rel['established_altitude']} is no longer "
                     f"earned: {why}")

    return {
        "relation": rel,
        "sources": sources,
        "flags": flags,
        "needs_review": needs_review,
    }
