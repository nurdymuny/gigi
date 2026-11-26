"""Gluing-invariance qualifier for the flux grid (docs/research/helike-flux-method-amendment-2026-11-25.md, §5.5).
Report-only: it reads the completed registered grid receipt and never changes a point, case or overall verdict.

Seams (tolerances as registered):
  S1 identification reference: selected levels by bare w, by w/Z, and by the dressed overlap over ALL candidate levels
  S2 quadrature: resolved order pair vs the next (coarse vs fine in the cone-heat receipt); indices equal, |dw~| <= 1e-5
  S3 truncation: the registered Mq ladder last-step change <= 0.003 (restated)
  S4 switch interpolation: linear (registered) / quadratic (3 nearest the bracket) / natural cubic spline; spread <= 0.005
  S5 certificate cutoff: the §5.7 cut-gate verdicts at the registered plateau radius and at the next smaller proving radius

usage:
  python scripts/run_helike_gluing_check.py --controls OUT_DIR                 (first: g2 = 0.002, 0.004; Mq = 8, 10)
  python scripts/run_helike_gluing_check.py --grid GRID_RECEIPT OUT_DIR        (only after the registered grid completes)
"""
import argparse
import json
import math
from pathlib import Path

import numpy as np
from scipy.interpolate import CubicSpline

import helike_heat_signal as hs

QUAD_W_TOL = 1e-5
GAP_TOL = 0.003
SWITCH_TOL = 0.005


# ------------------------------------------------------------------ S1, S2 (from a cone-heat receipt)
def selections(rec, order="fine"):
    """Selected (helike, competitor) level indices by (a) bare w, (b) w/Z, (c) dressed overlap; first max over 1..k."""
    k = len(rec["levels"])
    F = rec[order]
    amp = np.array(F["reference_amplitudes"]).reshape(7, k)
    roles = rec["roles"]
    Z = amp[6, 0] ** 2
    bare_h = np.array([amp[roles[0], c] ** 2 + amp[roles[1], c] ** 2 for c in range(k)])
    bare_c = np.array([amp[roles[2], c] ** 2 for c in range(k)])
    dress_h = hs.level_weights(rec, "H", order=order)[0] * 2          # projection weight on the 2-d space
    dress_c = hs.level_weights(rec, "C", order=order)[0]
    first_max = lambda w: int(1 + np.argmax(w[1:]))
    return {"bare": (first_max(bare_h), first_max(bare_c)),
            "w_over_Z": (first_max(bare_h / Z), first_max(bare_c / Z)),
            "dressed": (first_max(dress_h), first_max(dress_c)),
            "w_tilde": (bare_h / Z, bare_c / Z)}


def cluster_of(rec, n):
    """The degenerate cluster of level n: levels within 1e-6 in energy (§5.6: Helike's partners count together)."""
    E = np.asarray(rec["levels"])
    return tuple(m for m in range(1, len(E)) if abs(E[m] - E[n]) <= 1e-6)


def s1(rec):
    sel = selections(rec)
    a, b, c = sel["bare"], sel["w_over_Z"], sel["dressed"]
    cl = {k: tuple(cluster_of(rec, i) for i in v) for k, v in (("bare", a), ("w_over_Z", b), ("dressed", c))}
    return {"selected": {"bare": a, "w_over_Z": b, "dressed": c}, "clusters": cl,
            "holds": cl["bare"] == cl["w_over_Z"] == cl["dressed"]}


def s2(rec):
    f, c = selections(rec, "fine"), selections(rec, "coarse")
    cl = lambda v: tuple(cluster_of(rec, i) for i in v)
    same = cl(f["w_over_Z"]) == cl(c["w_over_Z"]) and cl(f["dressed"]) == cl(c["dressed"])
    dw = max(float(np.max(np.abs(f["w_tilde"][i] - c["w_tilde"][i]))) for i in (0, 1))
    return {"max_w_tilde_change": dw, "indices_equal": same, "holds": bool(same and dw <= QUAD_W_TOL)}


# ------------------------------------------------------------------ S3 (from the ladder)
def s3(ladder):
    rows = sorted(ladder, key=lambda r: r["Mq"])
    step = max(abs(rows[-1][n]["gap"] - rows[-2][n]["gap"]) for n in ("helike", "competitor"))
    return {"last_step_change": step, "holds": bool(step <= GAP_TOL)}


# ------------------------------------------------------------------ S4 (from a case)
def switch_linear(s, d):
    for i in range(len(s) - 1):
        if d[i] == 0:
            return s[i]
        if d[i] * d[i + 1] < 0:
            return s[i] - d[i] * (s[i + 1] - s[i]) / (d[i + 1] - d[i])
    return None


def switch_quadratic(s, d):
    lin = switch_linear(s, d)
    if lin is None:
        return None
    i = int(np.argmin(np.abs(np.asarray(s) - lin)))
    idx = sorted(sorted(range(len(s)), key=lambda j: abs(s[j] - lin))[:3])
    c = np.polyfit([s[j] for j in idx], [d[j] for j in idx], 2)
    roots = [r.real for r in np.roots(c) if abs(r.imag) < 1e-12]
    lo, hi = min(s[j] for j in idx), max(s[j] for j in idx)
    roots = [r for r in roots if lo - 1e-12 <= r <= hi + 1e-12]
    return min(roots, key=lambda r: abs(r - lin)) if roots else None


def switch_spline(s, d):
    lin = switch_linear(s, d)
    if lin is None:
        return None
    cs = CubicSpline(s, d, bc_type="natural")
    roots = [r for r in cs.roots(extrapolate=False) if min(s) <= r <= max(s)]
    return min(roots, key=lambda r: abs(r - lin)) if roots else None


def s4(svals, diffs):
    vals = {"linear": switch_linear(svals, diffs), "quadratic": switch_quadratic(svals, diffs),
            "spline": switch_spline(svals, diffs)}
    if any(v is None for v in vals.values()):
        return {"switch": vals, "spread": None, "holds": False if vals["linear"] is not None else None}
    spread = max(vals.values()) - min(vals.values())
    return {"switch": vals, "spread": float(spread), "holds": bool(spread <= SWITCH_TOL)}


# ------------------------------------------------------------------ S5 (domain gate at two radii)
def plateau_at(u0, S, w, G0, rho):
    """The registered plateau computation restricted to one radius; raises ValueError if it does not prove."""
    import helike_cutoff_bounds as cb
    from flint import arb
    u, matrix, freqs, A, norms = cb.scaled_chart(u0, S, w)
    G0 = np.asarray(G0)
    G = cb.arb_mat([[cb.exact(v) for v in row] for row in G0])
    D = G.det() / 100; R = 4 * sum((G[i, j] ** 2 for i in range(3) for j in range(3)), arb(0)).sqrt()
    Ds = D.union(cb.exact(.01 * np.linalg.det(G0))); R = R.union(cb.exact(4 * np.linalg.norm(G0)))
    slope = sum(((2 * sum((A[i][j] for i in range(3)), arb(0))) ** 2 for j in range(6)), arb(0)).sqrt()
    r = arb(rho)
    rs = [(u[i] + r * norms[i]).exp() for i in range(3)]
    a, b, c = [abs(u[i]) + r * norms[i] for i in range(3, 6)]
    g00 = rs[0] ** 2; g11 = rs[1] ** 2 + a * a; g22 = rs[2] ** 2 + b * b + c * c
    nmax = (g00 ** 2 + g11 ** 2 + g22 ** 2 + 2 * ((rs[0] * a) ** 2 + (rs[0] * b) ** 2 + (a * b + rs[1] * c) ** 2)).sqrt()
    dmin = (2 * sum(u[:3], arb(0)) - r * slope).exp()
    if not (dmin >= 2 * Ds and nmax <= R):
        raise ValueError("radius does not prove a complete plateau ball")
    return dict(rho=r, Ds=Ds, R=R, det_lower=dmin.lower(), norm_upper=nmax.upper())


def s5(s, t, g2, mq):
    import helike_cutoff_bounds as cb
    import helike_domain_gate as dg
    orig = cb.plateau
    seen = {}

    def capture(u0, S, w, G0):
        best = orig(u0, S, w, G0)
        seen["args"] = (u0, S, w, G0); seen["rho"] = float(best["rho"].mid())
        return best
    cb.plateau = capture
    try:
        registered = dg.domain_gate_point(s, t, g2, mq)
    finally:
        cb.plateau = orig
    reg_rho = seen["rho"]
    u0, S, w, G0 = seen["args"]
    alt = None
    q = round(reg_rho * 4) - 1
    while q >= 1:
        try:
            alt = plateau_at(u0, S, w, G0, q / 4)
            break
        except ValueError:
            q -= 1
    if alt is None:
        return {"registered_rho": reg_rho, "alternate_rho": None, "holds": None, "note": "not testable at this point"}
    alt_rho = float(alt["rho"].mid())
    if alt_rho == reg_rho:
        raise AssertionError("S5 must use two different radii")
    cb.plateau = lambda *a_, **k_: alt
    try:
        other = dg.domain_gate_point(s, t, g2, mq)
    finally:
        cb.plateau = orig
    # §5.7: the gate's verdict per state is the cut gate together with the tail and normalisation budgets
    pick = lambda r: {n: (r["states"][n]["cut_gate"], bool(r["states"][n]["state_passed"]))
                      for n in ("helike_plus", "helike_minus", "competitor")}
    a, b = pick(registered), pick(other)
    return {"registered_rho": reg_rho, "alternate_rho": alt_rho, "registered": a, "alternate": b,
            "verdicts": (registered["verdict"], other["verdict"]),
            "holds": a == b and registered["verdict"] == other["verdict"]}


# ------------------------------------------------------------------ verdicts
def point_verdict(seams):
    moved = [k for k in ("S1", "S2", "S3", "S5") if seams.get(k, {}).get("holds") is False]
    return ("GLUING-INVARIANT" if not moved else "GLUING-SENSITIVE"), moved


def qualify(grid_verdict, qualifier):
    """The qualifier is attached beside the registered verdict; it never replaces or alters it."""
    return {"registered_verdict": grid_verdict, "gluing_qualifier": qualifier}


def controls(out_dir):
    out = Path(out_dir); out.mkdir(parents=True, exist_ok=True)
    rep = {"stage": "controls (registered first stage of §5.5)", "points": []}
    import helike_progress as hp                 # viewer progress lines only (Controls scene spec section 5)
    for g2 in (0.002, 0.004):
        ladder = []
        recs = {}
        for mq in (6, 8, 10):
            if mq in (8, 10):
                hp.emit(out, "gluing", g2, mq, "cone-heat solve")
            r = hs.cone_heat(1.15, 3.0, g2, mq)
            recs[mq] = r
            ident = r["identification"]
            ladder.append({"Mq": mq, "helike": {"gap": ident["helike"]["gap"]}, "competitor": {"gap": ident["competitor"]["gap"]}})
            (out / f"cone_heat_g{g2}_Mq{mq}.json").write_text(json.dumps(r) + "\n")
        for mq in (8, 10):
            hp.emit(out, "gluing", g2, mq, "seams S1-S5 (S5: domain gate at two radii)")
            seams = {"S1": s1(recs[mq]), "S2": s2(recs[mq]),
                     "S3": s3([x for x in ladder if x["Mq"] <= mq]) if mq == 10 else {"holds": None, "note": "ladder ends at 10"},
                     "S5": s5(1.15, 3.0, g2, mq)}
            v, moved = point_verdict(seams)
            rep["points"].append({"g2": g2, "Mq": mq, "seams": seams, "verdict": v, "moved": moved})
            hp.emit(out, "gluing", g2, mq, "done")
            (out / "report.json").write_text(json.dumps(rep, indent=1, default=str) + "\n")
    rep["controls_invariant"] = all(p["verdict"] == "GLUING-INVARIANT" for p in rep["points"])
    (out / "report.json").write_text(json.dumps(rep, indent=1, default=str) + "\n")
    return rep


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--controls", metavar="OUT_DIR")
    a = ap.parse_args()
    if a.controls:
        rep = controls(a.controls)
        print(json.dumps({"controls_invariant": rep["controls_invariant"],
                          "points": [{k: p[k] for k in ("g2", "Mq", "verdict", "moved")} for p in rep["points"]]},
                         indent=1))
