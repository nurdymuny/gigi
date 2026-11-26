"""M8 carrier tracking and verdicts (docs/research/helike-M8-weighted-geometry-registration-2026-11-26.md).

Pure logic: no physics. It implements the current contract, which is amendment 3 together with amendment 2's shares
and refinement budget.
- Carriers are followed by physical overlap, checked through midpoints: A3.1.
- Each branch is assigned one-to-one.
- A crossing is verified only by subdivision-consistent labels, resolved opposite energy signs, and an exchange of
  lowest-level identities.
- Each channel runs a HELD / HANDED / DISSOLVED state machine with a resolved-crossing priority: A3.2.
- The continuity description decides nothing: A2.2(b).

The physics enters only through a *family*, which supplies:
  family.sample(s)            -> Sample (energies ascending, uncertainties, clusters of level indices, sector)
  family.overlap(A, B)        -> ndarray O[i, j] = |<a_i | b_j>|^2 over the levels of samples A and B

Budget (A3.1 as clarified): the finest sample spacing is 1/64 of the grid step. The deepest acceptable step is depth
5 (1/32), checked through its midpoint.
"""
from dataclasses import dataclass, field
import math

import numpy as np

MATCH_MIN = 0.5            # the largest overlap must be at least this
MATCH_MARGIN = 0.1         # and exceed the second largest by more than this
INSPECT = 1e-4             # energy closeness: a reason to inspect, never evidence
OWN_SHARE_MIN = 0.5        # the decided own-block threshold
JUMP = 0.2                 # continuity description: total-variation jump that triggers refinement
MAX_DEPTH = 5              # deepest acceptable step: 2**-5 of the grid step, midpoint at 2**-6


@dataclass
class Sample:
    s: float
    energies: np.ndarray                 # ascending, all candidate levels
    uncert: object                       # ndarray of numerical energy uncertainties, or None (unknown, not zero)
    clusters: list                       # tuples of level indices: the branches (degenerate subspaces kept whole)
    sector: list                         # cluster positions (indices into clusters) that belong to the channel's sector
    payload: object = None               # family-private data (vectors, ...)

    def energy(self, c):
        return float(min(self.energies[i] for i in self.clusters[c]))

    def u(self, c):
        if self.uncert is None:
            return None
        return float(max(self.uncert[i] for i in self.clusters[c]))


# ---------------------------------------------------------------- matching
def cluster_overlap(O, A, B, ca, cb):
    """Mean squared singular value of the overlap block: ||M||_F^2 / dim(source)."""
    ia, ib = A.clusters[ca], B.clusters[cb]
    return float(sum(O[i, j] for i in ia for j in ib)) / len(ia)


def match(family, A, B, branches):
    """Propose a one-to-one match of the tracked branches (cluster positions at A) into B's sector.
    Returns (assignment dict name -> cluster position at B, unambiguous bool, table)."""
    O = family.overlap(A, B)
    out, ok, table = {}, True, {}
    for name, ca in branches.items():
        ov = sorted(((cluster_overlap(O, A, B, ca, cb), cb) for cb in B.sector), reverse=True)
        table[name] = ov
        best = ov[0]
        second = ov[1][0] if len(ov) > 1 else 0.0
        if not (best[0] >= MATCH_MIN and best[0] - second > MATCH_MARGIN):
            ok = False
        out[name] = best[1]
    if len(set(out.values())) != len(out):          # one-to-one: two branches may not share a later state
        ok = False
    return out, ok, table


@dataclass
class Step:
    status: str                          # 'RESOLVED' or 'OPEN'
    path: list                           # [(Sample, {name: cluster position})] at every accepted node
    rejected: list = field(default_factory=list)
    coarse: dict = None                  # the direct coarse endpoint match (kept for the receipt only)
    reason: str = ""
    max_depth_used: int = 0


def track_step(family, A, B, branches, depth=0, cache=None):
    """Follow branches from sample A to sample B (A3.1). Every step is checked through its midpoint."""
    cache = {} if cache is None else cache
    smp = lambda s: cache.setdefault(s, family.sample(s))
    direct, ok_d, _ = match(family, A, B, branches)
    if depth == 0:
        coarse = dict(direct)
    else:
        coarse = None
    M = smp(0.5 * (A.s + B.s))
    m1, ok1, _ = match(family, A, M, branches)
    m2, ok2, _ = match(family, M, B, m1)
    if ok_d and ok1 and ok2 and m2 == direct:
        return Step("RESOLVED", [(A, dict(branches)), (M, m1), (B, direct)], coarse=coarse, max_depth_used=depth)
    rej = [dict(s0=A.s, s1=B.s, depth=depth, direct=direct, composed=m2, unambiguous=(ok_d, ok1, ok2))]
    if depth >= MAX_DEPTH:
        return Step("OPEN", [], rej, coarse, f"unresolved matching at depth {depth} on [{A.s}, {B.s}]", depth)
    left = track_step(family, A, M, branches, depth + 1, cache)
    if left.status != "RESOLVED":
        return Step("OPEN", [], rej + left.rejected, coarse, left.reason, left.max_depth_used)
    right = track_step(family, M, B, left.path[-1][1], depth + 1, cache)
    if right.status != "RESOLVED":
        return Step("OPEN", [], rej + left.rejected + right.rejected, coarse, right.reason, right.max_depth_used)
    return Step("RESOLVED", left.path + right.path[1:], rej + left.rejected + right.rejected, coarse,
                max_depth_used=max(left.max_depth_used, right.max_depth_used))


# ---------------------------------------------------------------- crossing evidence
def lowest_status(S, c):
    """True if cluster c is the sector's lowest, resolved; False if resolved not lowest; None if tied or unknown."""
    others = [d for d in S.sector if d != c]
    if not others:
        return True
    if S.uncert is None:
        return None
    e = S.energy(c)
    for d in others:
        gap = S.energy(d) - e
        if abs(gap) <= S.u(c) + S.u(d):
            return None                  # an unresolved tie with another sector level
    return all(S.energy(d) > e for d in others)


def signed_sep(S, c, b):
    """(E_c - E_b, resolved sign or 0). A missing uncertainty is not zero: unresolved."""
    d = S.energy(c) - S.energy(b)
    uc, ub = S.u(c), S.u(b)
    if uc is None or ub is None:
        return d, 0
    tol = max(INSPECT, uc + ub)
    return d, (1 if d > tol else -1 if d < -tol else 0)


def verify_crossing(step, carrier="carrier"):
    """A3.1 verified crossing on a resolved step path. Returns (kind, new_carrier_name, record).
    kind: 'NONE' (no crossing), 'VERIFIED', or 'POSSIBLE' (a possible crossing left unresolved -> OPEN)."""
    path = step.path
    S0, a0 = path[0]
    S1, a1 = path[-1]
    st0 = lowest_status(S0, a0[carrier])
    st1 = lowest_status(S1, a1[carrier])
    others = sorted((n for n in a0 if n != carrier), key=lambda n: S0.energy(a0[n]))
    record = dict(carrier_lowest=(st0, st1), pairs={})
    if st0 is True and st1 is True:
        return "NONE", None, record
    for b in others:
        signs = [signed_sep(S, a[carrier], a[b])[1] for S, a in path]
        record["pairs"][b] = signs
        neg = [i for i, x in enumerate(signs) if x < 0]
        pos = [i for i, x in enumerate(signs) if x > 0]
        bracket = bool(neg and pos and neg[0] == 0 and max(neg) < min(pos) and pos[-1] == len(signs) - 1)
        b_low = lowest_status(S1, a1[b])
        if bracket and st0 is True and st1 is False and b_low is True:
            record["bracket"] = (path[max(neg)][0].s, path[min(pos)][0].s)
            return "VERIFIED", b, record
    return "POSSIBLE", None, record


# ---------------------------------------------------------------- continuity description (decides nothing)
def tv(p, q):
    return 0.5 * float(np.sum(np.abs(np.asarray(p) - np.asarray(q))))


def continuity(family, step, pi_fn, carrier="carrier", depth_left=None):
    """Max total-variation jump of the carrier's positive shares between accepted nodes, refining jumps > JUMP by
    bisection within the remaining budget (A2.2(b)). Returns dict(status, max_jump, width)."""
    if pi_fn is None:
        return {"status": "not computed"}
    nodes = [(S, a[carrier]) for S, a in step.path]
    worst, wide = 0.0, []

    def refine(SA, ca, SB, cb, budget):
        nonlocal worst
        j = tv(pi_fn(SA, ca), pi_fn(SB, cb))
        if j <= JUMP:
            worst = max(worst, j)
            return True
        if budget <= 0:
            worst = max(worst, j)
            return False
        SM = family.sample(0.5 * (SA.s + SB.s))
        m, ok, _ = match(family, SA, SM, {"c": ca})
        if not ok:
            return False
        wide.append((SA.s, SB.s))
        return refine(SA, ca, SM, m["c"], budget - 1) and refine(SM, m["c"], SB, cb, budget - 1)

    budget = MAX_DEPTH + 1 - step.max_depth_used if depth_left is None else depth_left
    ok = all(refine(nodes[i][0], nodes[i][1], nodes[i + 1][0], nodes[i + 1][1], budget) for i in range(len(nodes) - 1))
    return {"status": "resolved" if ok else "unresolved", "max_jump": worst,
            "width": (min(w[0] for w in wide), max(w[1] for w in wide)) if wide else None}


# ---------------------------------------------------------------- the state machine (A3.2)
def apply_rules(state, crossing, share):
    """One step of a HELD channel, with the A3.2 priority: a verified crossing first, then the own-share rule."""
    if crossing == "VERIFIED":
        return "HANDED"
    if share >= OWN_SHARE_MIN:
        return "HELD"
    return "DISSOLVED"


def run_case(family, grid, share_fn, selector_fn=None, pi_fn=None, inter_sector=(), carrier0=None):
    """One channel along one grid case.
    share_fn(sample, cluster) -> own-block share of that branch; selector_fn(sample) -> the grid selector's cluster;
    inter_sector: indices k of steps k -> k+1 that contain the registered inter-sector switch (reported only)."""
    cache = {}
    smp = lambda s: cache.setdefault(s, family.sample(s))
    S = smp(grid[0])
    c = S.sector[0] if carrier0 is None else carrier0
    rep = {"points": [], "steps": [], "selection_changed": [], "gap_handover": [], "verdict": None}
    if lowest_status(S, c) is not True:
        rep["verdict"] = "OPEN"; rep["reason"] = "starting identity unresolved"
        return rep
    sh = share_fn(S, c)
    state = "HELD" if sh >= OWN_SHARE_MIN else "DISSOLVED"
    rep["points"].append(dict(s=S.s, carrier=c, share=sh, state=state))
    if selector_fn is not None and selector_fn(S) != c:
        rep["selection_changed"].append(S.s)
    for k in range(len(grid) - 1):
        if state == "DISSOLVED":
            break
        A, B = smp(grid[k]), smp(grid[k + 1])
        branches = {"carrier": c}
        for i, d in enumerate(sorted((d for d in A.sector if d != c), key=A.energy)[:2]):
            branches[f"other{i}"] = d                       # the neighbours that could take part in a crossing
        step = track_step(family, A, B, branches, cache=cache)
        srec = dict(s0=A.s, s1=B.s, status=step.status, rejected=step.rejected, coarse=step.coarse,
                    depth=step.max_depth_used)
        rep["steps"].append(srec)
        if k in inter_sector:
            rep["gap_handover"].append((A.s, B.s))          # the gap hands over between channels; no channel event
        if step.status != "RESOLVED":
            rep["verdict"] = "OPEN"; rep["reason"] = step.reason
            return rep
        kind, newb, crec = verify_crossing(step)
        srec["crossing"] = crec | {"kind": kind}
        srec["continuity"] = continuity(family, step, pi_fn)
        if kind == "POSSIBLE":
            rep["verdict"] = "OPEN"; rep["reason"] = "possible crossing unresolved"
            return rep
        end = step.path[-1][1]
        c = end[newb] if kind == "VERIFIED" else end["carrier"]
        sh = share_fn(B, c)
        if state == "HELD":
            state = apply_rules(state, kind, sh)
        rep["points"].append(dict(s=B.s, carrier=c, share=sh, state=state))
        if selector_fn is not None and selector_fn(B) != c:
            rep["selection_changed"].append(B.s)
    rep["verdict"] = {"HELD": "PERSISTS", "HANDED": "HANDS OVER", "DISSOLVED": "DISSOLVES"}[state]
    return rep


# ---------------------------------------------------------------- matrix families (fixtures and small models)
class MatrixFamily:
    """H(s) a real symmetric matrix function. Optional fixed symmetry labels per basis vector: H must commute with
    them, and eigenvectors are taken block by block, so a degeneracy between labels is resolved by the symmetry and
    not by the eigensolver. uncert: per-level numerical uncertainty (a number, applied to all levels), or None."""

    def __init__(self, H, labels=None, uncert=0.0, degen_tol=1e-12):
        self.H, self.labels, self.uncert_value, self.tol = H, labels, uncert, degen_tol

    def sample(self, s):
        H = np.asarray(self.H(s), dtype=float)
        n = len(H)
        if self.labels is None:
            e, v = np.linalg.eigh(H)
            lab = [None] * n
        else:
            lab_arr = np.asarray(self.labels)
            es, vs, ls = [], [], []
            for L in sorted(set(self.labels), key=str):
                idx = np.flatnonzero(lab_arr == L)
                if np.max(np.abs(H[np.ix_(idx, np.setdiff1d(np.arange(n), idx))]), initial=0.0) > 1e-12:
                    raise ValueError("H does not commute with the supplied symmetry labels")
                eb, vb = np.linalg.eigh(H[np.ix_(idx, idx)])
                for j in range(len(eb)):
                    full = np.zeros(n); full[idx] = vb[:, j]
                    es.append(eb[j]); vs.append(full); ls.append(L)
            order = np.argsort(es, kind="stable")
            e = np.array(es)[order]; v = np.array(vs).T[:, order]; lab = [ls[i] for i in order]
        clusters, used = [], set()
        for i in range(n):
            if i in used:
                continue
            cl = [j for j in range(n) if j not in used and abs(e[j] - e[i]) <= self.tol and lab[j] == lab[i]]
            used.update(cl); clusters.append(tuple(cl))
        unc = None if self.uncert_value is None else np.full(n, float(self.uncert_value))
        return Sample(s, e, unc, clusters, list(range(len(clusters))), payload=(v, lab))

    def overlap(self, A, B):
        va, vb = A.payload[0], B.payload[0]
        return (va.T @ vb) ** 2
