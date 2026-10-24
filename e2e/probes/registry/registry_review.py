"""Promotion and retraction-aware retrieval.

Promotion is the decision the registry exists to get right. Altitude is earned
from evidence records and is never implied by the relation's name, so
`promote()` inspects the evidence and refuses with a named reason when the
requirement is not met.

Retrieval is the other half. Finding the relations that rest on a retracted
source is useless if a reader is still shown those relations as though they were
supported, so `view()` carries the flags a reader must see.
"""

from registry_bootstrap import call, eq, query

ALTITUDES = ["NONE", "INCIDENCE", "RESONANCE", "STRUCTURAL"]


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
    # Retracted evidence does not support anything.
    live = [e for e in ev
            if (source(e["source_id"]) or {}).get("availability") != "retracted"]

    if target == "NONE":
        return True, "no requirement"

    if target == "INCIDENCE":
        if not any(e["locator"] for e in live):
            return False, "no live source with a precise locator"
        return True, "a live source locates the claim"

    if target == "RESONANCE":
        sides = {e["locates_mechanism_for"] for e in live if e["locates_mechanism_for"]}
        if "subject" not in sides or "object" not in sides:
            have = sorted(sides) or ["neither side"]
            return False, ("the shared mechanism must be located on BOTH sides; "
                           f"evidence locates it for {', '.join(have)}")
        return True, "mechanism located on both sides"

    if target == "STRUCTURAL":
        if not rel["altitude_scope"]:
            return False, "STRUCTURAL requires an explicit altitude_scope"
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
    call("/v1/bundles/research_relations/insert", {"records": [rel]})
    return True, reason


# ── Retraction-aware retrieval ───────────────────────────────────────────────

def view(rid):
    """What a reader must be shown, including why it may not be trustworthy."""
    rel = relation(rid)
    flags, sources = [], []
    for e in evidence_for(rid):
        s = source(e["source_id"])
        if s is None:
            flags.append(f"evidence {e['source_id']} is missing from the registry")
            continue
        sources.append({"id": s["id"], "title": s["title"],
                        "availability": s["availability"],
                        "locator": e["locator"], "kind": e["evidence_kind"]})
        if s["availability"] == "retracted":
            flags.append(f"supporting source {s['id']} was RETRACTED; "
                         "this relation needs review")
        elif s["availability"] == "unavailable":
            flags.append(f"supporting source {s['id']} is temporarily unavailable; "
                         "altitude is unchanged")
        elif s["availability"] == "restricted":
            flags.append(f"supporting source {s['id']} is not readable by you; "
                         "its content is withheld")

    still_earned, why = requirement_met(rid, rel["established_altitude"])
    if not still_earned:
        flags.append(f"stored altitude {rel['established_altitude']} is no longer "
                     f"earned: {why}")

    return {
        "relation": rel,
        "sources": sources,
        "flags": flags,
        "needs_review": any("needs review" in f or "no longer earned" in f
                            for f in flags),
    }
