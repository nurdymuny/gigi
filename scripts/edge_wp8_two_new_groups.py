"""Pre-registration G (research/edge-theorem-20261122-g/wp8_prereg_G_two_new_groups.md): does she follow her own circle
in G2 and Sp(3)? Test F's method, unchanged, on two groups never computed before, plus the short-root circle and a
control wall. Fast block code: scripts/edge_wp8_fast.py (gate G-K0 checks it against Test F's dense code).
usage: OMP_NUM_THREADS=<n> python scripts/edge_wp8_two_new_groups.py OUT.json
"""
import json
import sys
import time
from pathlib import Path

import numpy as np
import scipy.sparse as sp
from scipy.optimize import brentq

sys.path.insert(0, str(Path(__file__).resolve().parent))
import edge_wp8_fast as FA                 # noqa: E402  (also routes g2 / sp3 to the survey builders)
import edge_wp7_principal_circle as F      # noqa: E402
import edge_wp7_groups as GR               # noqa: E402

SHAPES = F.SHAPES
F_RECEIPT = Path(__file__).resolve().parent.parent / "research/edge-theorem-20261122-f/wp7_principal_circle.json"


def geometry(gname):
    """Arc angles (registration §1) and theta_s-vee, from the root data."""
    g = FA.FastGroupK(gname, 0.0, sectors={})
    wts = np.array([w for w, _ in g.rts]); key = np.arange(1, g.rank + 1.0)
    pos = [w for w in wts if w @ key > 1e-9]; L2 = max(w @ w for w in pos)
    short = [w for w in pos if w @ w < L2 - 1e-9]; ths = max(short, key=lambda w: w @ g.rhov); thsv = 2 * ths / (ths @ ths)
    E = lambda a: np.cos(np.radians(a)) * g.u0 + np.sin(np.radians(a)) * g.perp          # noqa: E731
    rho_ang = float(np.degrees(np.arccos(g.rhov @ g.u0 / np.linalg.norm(g.rhov))))
    wall = 30.0 if gname == "g2" else float(brentq(lambda a: min(w @ E(a) for w in pos), rho_ang + 1, 60.0, xtol=1e-12))
    if gname == "g2":                                                                      # check the wall is at 30
        assert abs(min(w @ E(30.0) for w in pos)) < 1e-9 and min(w @ E(29.9) for w in pos) > 0
    return {"pos": pos, "L2": L2, "thsv": thsv, "rho_ang": rho_ang, "wall": wall, "u0": g.u0, "perp": g.perp}


def circles():
    out = []
    for gname, labs in (("g2", "g"), ("sp3", "s")):
        geo = geometry(gname); r, w = geo["rho_ang"], geo["wall"]
        for i, a in enumerate((0.0, r / 2, r, (r + w) / 2, w)):
            out.append({"group": gname, "label": f"{labs}{i}", "angle": float(a), "eK": None})
        if gname == "sp3":
            e = geo["thsv"] / np.linalg.norm(geo["thsv"])
            ang = float(np.degrees(np.arccos(e @ geo["u0"])))
            out.append({"group": gname, "label": "sS", "angle": ang, "eK": e})
    return out


def build(c, with_sectors=True):
    g = FA.FastGroupK(c["group"], c["angle"], eK_torus=c["eK"], sectors={})
    g.angle = c["angle"]                         # the actual angle from theta-vee (used by the SU(2) calibration)
    main, extra = FA.sector_specs(g, c["group"])
    if with_sectors:
        FA.add_sectors(g, {**main, **extra})
    return g, main, extra


def geometry_record(g, gname, geo):
    pos = geo["pos"]; e = g.eK
    pairs = sorted(((abs(float(w @ e)), w) for w in pos), key=lambda t: -t[0])
    top = pairs[0][0]; tied = [w for p, w in pairs if abs(p - top) < 1e-9]
    cor = [2 * w / (w @ w) for w in pos]
    par = [i for i, cv in enumerate(cor) if abs(abs(cv @ e) / np.linalg.norm(cv) - 1) < 1e-9]
    return {"regular": bool(min(abs(w @ e) for w in pos) > 1e-9), "top_root_unique": len(tied) == 1,
            "tied_top": [[float(v) for v in w] + ["L" if w @ w > geo["L2"] - 1e-9 else "S"] for w in tied],
            "parallel_to_coroot_of": [[float(v) for v in pos[i]] + ["L" if pos[i] @ pos[i] > geo["L2"] - 1e-9 else "S"] for i in par],
            "q_K": g.qK, "y_theta_plane": int(g.top_plane), "y_theta_root": [float(v) for v in g.plane_roots[g.top_plane]]}


EXPECT = {  # registration §1: (regular, top unique)
    "g0": (False, True), "g1": (True, True), "g2": (True, True), "g3": (True, True), "g4": (False, False),
    "s0": (False, True), "s1": (True, True), "s2": (True, True), "s3": (True, True), "s4": (False, False), "sS": (False, False)}


def twist_content(g, shape, x=-1.0):
    """Descriptive: her state in the trivial sector split over (form plane, algebra plane) pairs."""
    pr = g.prepare(shape, "1"); ev = g.evaluate(x, shape, "1", want_vec=True)
    M = ev["her_vec"].reshape(g.d, g.d); r = g.rank
    idx = {"cartan": list(range(r))}
    for k in range(len(g.plane_roots)):
        idx[f"plane{k}"] = [r + 2 * k, r + 2 * k + 1]
    tw = cr = 0.0; parts = {}
    for a, ia in idx.items():
        for b, ib in idx.items():
            w_ = float(np.sum(np.abs(M[np.ix_(ia, ib)]) ** 2))
            if w_ > 1e-6:
                parts[f"{a}|{b}"] = round(w_, 6)
            if a.startswith("plane") and b.startswith("plane"):
                if a == b:
                    tw += w_
                else:
                    cr += w_
    return {"x": x, "twist_share": tw, "cross_share": cr, "parts": parts, "top_dim": int(len(pr["top"]))}


def gate_K0():
    worst = 0.0; rows = []
    for gname, ang, sh, x in (("sp2", 0.0, (0.5, 0.5, 0.25), -1.3), ("sp2", 45.0, (0.5, 0.5, 0.75), -1.2),
                              ("su4", 0.0, (0.5, 0.5, 0.25), -1.4)):
        main, _ = F.sectors_for(gname)
        if gname == "su4":
            main = {k: main[k] for k in ("1", "4", "6")}
        gd = F.GroupK(gname, ang, sectors=main); gf = FA.FastGroupK(gname, ang, sectors=main)
        for s in main:
            a = gd.evaluate(x, sh, s); b = gf.evaluate(x, sh, s)
            dv = max(np.max(np.abs(np.sort(a["mu"]) - b["mu"])) / max(1, np.abs(a["mu"]).max()),
                     abs(a["lam"] - b["lam"]) / max(1, abs(a["lam"])), float(a["ncopies"] != b["ncopies"]))
            worst = max(worst, dv)
        pa, pb = F.phi(gd, x, sh, list(main)), F.phi(gf, x, sh, list(main))
        worst = max(worst, abs(pa[0] - pb[0]) / max(1, abs(pa[0])), float(pa[1] != pb[1]))
        rows.append([gname, ang, list(sh), x, float(pa[0]), float(pb[0])])
    return worst, rows


def gate_K1():
    main, extra = F.sectors_for("sp2")
    g = FA.FastGroupK("sp2", 0.0, sectors=main, extra=extra)
    o = F.scan(g, SHAPES[0], list(main), list(main) + list(extra))
    ref = next(p for p in json.loads(F_RECEIPT.read_text())["points"] if p["group"] == "sp2" and p["angle"] == 0.0 and tuple(p["shape"]) == SHAPES[0])
    dev = max(abs(o["phi_grid_best"] - ref["phi_grid_best"]),
              max(abs(a - b) for ra, rb in zip(o["I"], ref["I"]) for a, b in zip(ra, rb)),
              float(len(o["I"]) != len(ref["I"])), float(o["survives"] != ref["survives"]))
    return dev, {"I": o["I"], "phi": o["phi_grid_best"], "survives": o["survives"], "ref_I": ref["I"], "ref_phi": ref["phi_grid_best"]}


def main():
    out_path = Path(sys.argv[1]); res = {"protocol": "wp8_prereg_G_two_new_groups.md"}

    def save():
        out_path.write_text(json.dumps(res, indent=1, default=str) + "\n")

    def stop(msg):
        res["stopped"] = msg; save(); raise SystemExit("stop: " + msg)
    prev = json.loads(out_path.read_text()) if out_path.exists() else {}
    if "G_K0" in prev and "G_K1" in prev:
        for k in ("G_K0", "G_K0_rows", "G_K1", "G_K1_detail"):
            res[k] = prev[k]
        res["resumed_after"] = prev.get("stopped")
    else:
        res["G_K0"], res["G_K0_rows"] = gate_K0(); print("G-K0", res["G_K0"], flush=True); save()
        if res["G_K0"] > 1e-9:
            stop("G-K0")
        res["G_K1"], res["G_K1_detail"] = gate_K1(); print("G-K1", res["G_K1"], flush=True); save()
        if res["G_K1"] > 1e-8:
            stop("G-K1")
    rows = list(prev.get("points", [])); done = {(o["label"], tuple(o["shape"])) for o in rows}
    geos = {gname: geometry(gname) for gname in ("g2", "sp3")}
    res["geometry"] = {}
    for c in circles():
        gname = c["group"]; geo = geos[gname]
        g, main_s, extra_s = build(c)
        # G-K2: group data
        brk = 0.0
        for a in range(g.d):
            for b in range(g.d):
                Z = g.Y[a] @ g.Y[b] - g.Y[b] @ g.Y[a]
                brk = max(brk, float(np.abs(Z - sum(g.Cy[a, b, k] * g.Y[k] for k in range(g.d))).max()))
        reps = {k: FA.rep_check(g, k) for k in g.reps}
        dims = {k: g.reps[k][0].shape[0] for k in g.reps}
        if brk > 1e-13 or max(reps.values()) > 1e-12:
            stop(f"G-K2 {c['label']}: brackets {brk}, reps {reps}")
        # G-K3: geometry
        gr = geometry_record(g, gname, geo); gr.update(label=c["label"], angle=c["angle"], dims=dims, brackets=brk, reps=reps)
        exp = EXPECT[c["label"]]
        bad = (gr["regular"], gr["top_root_unique"]) != exp
        if c["label"] == "g4":
            bad |= abs(abs(g.eK @ geo["thsv"]) / np.linalg.norm(geo["thsv"]) - 1) > 1e-9 or abs(c["angle"] - 30.0) > 1e-9
        if c["label"] == "sS":
            bad |= abs(c["angle"] - 45.0) > 1e-9
        if c["label"] == "s4":
            bad |= bool(gr["parallel_to_coroot_of"])
        res["geometry"][c["label"]] = gr; save()
        if bad:
            stop(f"G-K3 geometry {c['label']}: {gr}")
        # G-S5 on one sector: quadratic decomposition against direct sparse builds
        sh = SHAPES[1]; s0 = list(main_s)[2]; pr = g.prepare(sh, s0); worst = 0.0
        for xv in (-0.7, 0.4, 1.9):
            Hb, _, _ = g.build_blocks(xv, sh, s0, pr["bi"], pr["top"])
            num = max(float(np.abs(Hb[cv] - (b_["H0"] + xv * b_["H1"] + xv * xv * b_["H2"])).max()) for cv, b_ in pr["blocks"].items())
            den = max(float(np.abs(Hb[cv]).max()) for cv in Hb)
            worst = max(worst, num / den)
        gr["G_S5"] = worst
        if worst > 1e-12:
            stop(f"G-S5 {c['label']}: {worst}")
        for sh in SHAPES:
            if (c["label"], tuple(sh)) in done:
                continue
            t0 = time.time(); quad = []; sym = 0.0; mult = {}
            for s in main_s:
                pr = g.prepare(sh, s)
                if len(pr["top"]) == 1:
                    lam = [g.evaluate(xv, sh, s)["lam"] for xv in (-1.0, 0.0, 1.0)]
                    quad.append(abs(lam[0] - 2 * lam[1] + lam[2] - 2 * g.qK ** 2))
                else:
                    H2t = pr["blocks"][pr["ctop"]]["H2"]
                    quad.append(float(np.abs(H2t - g.qK ** 2 * np.eye(H2t.shape[0])).max()))
                for xv in (-1.0, 0.0, 1.0):
                    ev = g.evaluate(xv, sh, s)
                    sym = max(sym, g.sym_probe(xv, sh, s, pr["signs"]))
                    bt = pr["blocks"][pr["ctop"]]; hscale = float(np.abs(bt["H0"] + xv * bt["H1"] + xv * xv * bt["H2"]).max())
                    if ev["resid"] > 1e-9 * max(1.0, hscale):
                        stop(f"G-mult {c['label']} {sh} {s}: resid {ev['resid']}")
                mult[s] = {"top": int(len(pr["top"])), "bottom": int(len(pr["bot"])), "mirror_tied": g.evaluate(0.0, sh, s)["mirror_tied"]}
            if max(quad) > 1e-9:
                stop(f"G-quad {c['label']} {sh}: {max(quad)}")
            if sym > 1e-10:
                stop(f"G-sym {c['label']} {sh}: {sym}")
            o = F.scan(g, sh, list(main_s), list(main_s) + list(extra_s))
            if abs(o["lin_second_diff"]) > 1e-9 or o["lin_Gstar"] > 1e-12:
                stop(f"G-lin {c['label']} {sh}")
            if o.get("I") and not o["bound_ok"]:
                stop(f"G-bound {c['label']} {sh}")
            o.update(group=gname, label=c["label"], angle=c["angle"], shape=list(sh), mult=mult, quad_dev=max(quad), sym=sym)
            o["calibration_edge"] = F.su2_calibration(g, sh)
            if not gr["top_root_unique"]:
                o["twist_content"] = twist_content(g, sh)
            o["seconds"] = time.time() - t0
            o["lives_point"] = bool(o["I"]) and o["survives"]
            rows.append(o); res["points"] = rows; save()
            print(c["label"], f"{c['angle']:.2f}", sh, "I", o["I"], "survives", o["survives"], "phi*", round(o["phi_grid_best"], 4) if "phi_grid_best" in o else None,
                  "golden", o.get("golden"), "rival", o.get("rival_above"), f"{o['seconds']:.0f}s", flush=True)
        g.cache.clear()
    # ------------------------------------------------------------------ scoring (registration §4)
    labels = [c["label"] for c in circles()]
    score = {}
    for lab in labels:
        n = sum(1 for o in rows if o["label"] == lab and o["lives_point"])
        score[lab] = {"n": n, "verdict": "lives" if n >= 2 else ("does not live" if n == 0 else "marginal")}
    res["score"] = score
    v = lambda lab: score[lab]["verdict"]                                                  # noqa: E731

    def decide(conds):
        """conds: list of (label, wanted verdict). True / False / 'not decided' (a marginal cell where one is needed)."""
        if any(v(l) == "marginal" for l, _ in conds):
            return "not decided" if all(v(l) == w for l, w in conds if v(l) != "marginal") else False
        return all(v(l) == w for l, w in conds)
    res["H_theta"] = {"g2": decide([("g0", "lives")]), "sp3": decide([("s0", "lives")])}
    res["H_notP"] = {"g2": v("g2") != "lives", "sp3": v("s2") != "lives"}
    res["H_s"] = decide([("g4", "lives"), ("sS", "lives")])
    res["H_s_control"] = decide([("s4", "does not live")])
    res["H_tie"] = decide([("sS", "lives"), ("s4", "lives")])
    res["strict_guess"] = (res["H_s"] is True and res["H_s_control"] is True)

    def phistar(o):
        return o.get("phi_grid_best", o.get("golden", {}).get("phi", -np.inf))
    near = {}
    for gname, arc in (("g2", ["g0", "g1", "g2", "g3", "g4"]), ("sp3", ["s0", "s1", "s2", "s3", "s4"])):
        cnt = 0; per = []
        for sh in SHAPES:
            vals = [phistar(next(o for o in rows if o["label"] == l and tuple(o["shape"]) == tuple(sh))) for l in arc]
            k = int(np.argmax(vals)); per.append({"shape": list(sh), "phi_star": vals, "argmax": arc[k]}); cnt += k <= 1
        near[gname] = {"shapes_near": cnt, "holds": cnt >= 2, "detail": per}
    res["H_near"] = near
    res["H_theta"]["both"] = all(res["H_theta"][k] is True for k in ("g2", "sp3"))
    res["H_notP"]["both"] = all(res["H_notP"][k] for k in ("g2", "sp3"))
    res["H_near"]["both"] = all(near[k]["holds"] for k in ("g2", "sp3"))
    side = []
    for o in rows:
        if o["I"] and o.get("r_hi", -1) > 0 and o["calibration_edge"] is not None and o["calibration_edge"] <= 0:
            side.append((o["label"], o["shape"], o["x_hi"], o["x_hi"] <= 0))
    res["G_side_scored"] = side; res["G_side"] = "not informative" if not side else all(s_[-1] for s_ in side)
    save()
    print(json.dumps({k: v_ for k, v_ in res.items() if k not in ("points", "geometry")}, indent=1, default=str))


if __name__ == "__main__":
    main()
