"""Pre-registration G (research/edge-theorem-20261122-g/wp8_prereg_G_two_new_groups.md, revised after review): does she
follow her own circle in G2 and Sp(3)? Test F's method on two groups never computed before, plus the far walls
(G2's arc end; Sp(3)'s short-root circle sS, the face s4 and the edge s5).
Main sectors: block code (edge_wp8_fast.FastGroupK). Check sectors: matrix-free slice Lanczos (edge_wp8_fast.check_margin).
usage: python scripts/edge_wp8_two_new_groups.py OUT.json [g2|sp3]     (one group per process; gates run in each)
       python scripts/edge_wp8_two_new_groups.py --merge OUT.json IN_g2.json IN_sp3.json   (score both together)
"""
import json
import sys
import time
from pathlib import Path

import numpy as np
from scipy.optimize import brentq

sys.path.insert(0, str(Path(__file__).resolve().parent))
import edge_wp8_fast as FA                 # noqa: E402  (also routes g2 / sp3 to the survey builders)
import edge_wp7_principal_circle as F      # noqa: E402

SHAPES = F.SHAPES
F_RECEIPT = Path(__file__).resolve().parent.parent / "research/edge-theorem-20261122-f/wp7_principal_circle.json"
ARC = {"g2": ["g0", "g1", "g2", "g3", "g4"], "sp3": ["s0", "s1", "s2", "s3", "s4"]}

# registration §1: (zero roots, tie pattern sorted, y_theta root length, parallel-to-coroot pattern)
EXPECT = {"g0": (1, "L", "L", "L"), "g1": (0, "L", "L", ""), "g2": (0, "L", "L", ""), "g3": (0, "L", "L", ""),
          "g4": (1, "LL", "L", "S"), "s0": (4, "L", "L", "L"), "s1": (0, "L", "L", ""), "s2": (0, "L", "L", ""),
          "s3": (0, "L", "L", ""), "s4": (1, "LLS", "S", ""), "sS": (2, "LLS", "S", "S"), "s5": (3, "LLLSSS", "S", "")}
# registration §2 (G-K2): top charge of each sector = (coefficients on the top defining weights mu_1, mu_2, mu_3) . e_K
HW = {"g2": {"1": (), "7": (1,), "14": (1, 1), "27": (2,), "77s": (3,), "64": (2, 1), "77p": (2, 2)},
      "sp3": {"1": (), "6": (1,), "14": (1, 1), "21": (2,), "14p": (1, 1, 1), "56s": (3,), "64": (2, 1), "90": (2, 2), "126s": (4,)}}
LARGEST_MAIN = {"g2": "27", "sp3": "14"}
ADJ = {"g2": "14", "sp3": "21"}


def geometry(gname):
    g = FA.FastGroupK(gname, 0.0, sectors={})
    wts = np.array([w for w, _ in g.rts]); key = np.arange(1, g.rank + 1.0)
    pos = [w for w in wts if w @ key > 1e-9]; L2 = max(w @ w for w in pos)
    short = [w for w in pos if w @ w < L2 - 1e-9]; ths = max(short, key=lambda w: w @ g.rhov); thsv = 2 * ths / (ths @ ths)
    E = lambda a: np.cos(np.radians(a)) * g.u0 + np.sin(np.radians(a)) * g.perp          # noqa: E731
    rho_ang = float(np.degrees(np.arccos(g.rhov @ g.u0 / np.linalg.norm(g.rhov))))
    wall = 30.0 if gname == "g2" else float(brentq(lambda a: min(w @ E(a) for w in pos), rho_ang + 1, 60.0, xtol=1e-12))
    if gname == "g2":
        assert abs(min(w @ E(30.0) for w in pos)) < 1e-9 and min(w @ E(29.9) for w in pos) > 0
    out = {"pos": pos, "L2": L2, "thsv": thsv, "rho_ang": rho_ang, "wall": wall, "u0": g.u0, "perp": g.perp}
    if gname == "sp3":                                        # omega_3-vee: pairs equally with the positive defining weights
        w, V, fv = FA.defining_weights(g); pw = w[fv > 0]
        e = np.linalg.lstsq(pw, np.ones(len(pw)), rcond=None)[0]; out["om3v"] = e / np.linalg.norm(e)
    return out


def circles():
    out = []
    for gname, labs in (("g2", "g"), ("sp3", "s")):
        geo = geometry(gname); r, w = geo["rho_ang"], geo["wall"]
        for i, a in enumerate((0.0, r / 2, r, (r + w) / 2, w)):
            out.append({"group": gname, "label": f"{labs}{i}", "angle": float(a), "eK": None})
        if gname == "sp3":
            for lab, v in (("sS", geo["thsv"]), ("s5", geo["om3v"])):
                e = v / np.linalg.norm(v)
                out.append({"group": gname, "label": lab, "angle": float(np.degrees(np.arccos(e @ geo["u0"]))), "eK": e})
    return out


def build(c):
    g = FA.CheckGroupK(c["group"], c["angle"], eK_torus=c["eK"], sectors={})
    g.angle = c["angle"]
    main, extra = FA.sector_specs(g, c["group"])
    FA.add_sectors(g, {**main, **extra}); g.check = tuple(extra)
    return g, main, extra


def geometry_record(g, geo):
    pos = geo["pos"]; e = g.eK; Lr = lambda w: "L" if w @ w > geo["L2"] - 1e-9 else "S"     # noqa: E731
    pairs = sorted(((abs(float(w @ e)), w) for w in pos), key=lambda t: -t[0])
    top = pairs[0][0]; tied = [w for p, w in pairs if abs(p - top) < 1e-9]
    cor = [2 * w / (w @ w) for w in pos]
    par = [i for i, cv in enumerate(cor) if abs(abs(cv @ e) / np.linalg.norm(cv) - 1) < 1e-9]
    yth = g.plane_roots[g.top_plane]
    return {"zero_roots": int(sum(abs(w @ e) < 1e-9 for w in pos)), "ties": "".join(sorted(Lr(w) for w in tied)),
            "tied_top": [[float(v) for v in w] for w in tied], "y_theta_len": Lr(yth), "y_theta_root": [float(v) for v in yth],
            "parallel": "".join(sorted(Lr(pos[i]) for i in par)), "q_K": g.qK, "regular": bool(min(abs(w @ e) for w in pos) > 1e-9)}


def twist_content(g, shape, x=-1.0):
    ev = g.evaluate(x, shape, "1", want_vec=True)
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
                tw, cr = (tw + w_, cr) if a == b else (tw, cr + w_)
    return {"x": x, "twist_share": tw, "cross_share": cr, "parts": parts}


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
            worst = max(worst, np.max(np.abs(np.sort(a["mu"]) - b["mu"])) / max(1, np.abs(a["mu"]).max()),
                        abs(a["lam"] - b["lam"]) / max(1, abs(a["lam"])), float(a["ncopies"] != b["ncopies"]))
        pa, pb = F.phi(gd, x, sh, list(main)), F.phi(gf, x, sh, list(main))
        worst = max(worst, abs(pa[0] - pb[0]) / max(1, abs(pa[0])), float(pa[1] != pb[1]))
        rows.append([gname, ang, list(sh), x, float(pa[0]), float(pb[0])])
    return float(worst), rows


def gate_K1():
    main, extra = F.sectors_for("sp2")
    g = FA.FastGroupK("sp2", 0.0, sectors=main, extra=extra)
    o = F.scan(g, SHAPES[0], list(main), list(main) + list(extra))
    ref = next(p for p in json.loads(F_RECEIPT.read_text())["points"] if p["group"] == "sp2" and p["angle"] == 0.0 and tuple(p["shape"]) == SHAPES[0])
    dev = max(abs(o["phi_grid_best"] - ref["phi_grid_best"]),
              max(abs(a - b) for ra, rb in zip(o["I"], ref["I"]) for a, b in zip(ra, rb)),
              float(len(o["I"]) != len(ref["I"])), float(o["survives"] != ref["survives"]))
    return float(dev), {"I": o["I"], "phi": o["phi_grid_best"], "survives": o["survives"], "ref_I": ref["I"], "ref_phi": ref["phi_grid_best"]}


def gate_B(g, sh, s, rng):
    """Block code vs U^*(H U) from the matrix-free H, on three random blocks and her top columns, at three x."""
    pr = g.prepare(sh, s); bi = pr["bi"]; worst = 0.0
    cvs = [cv for cv in pr["blocks"] if pr["blocks"][cv]["idx"].size <= 400]
    pick = [pr["ctop"]] + list(rng.choice(cvs, size=min(3, len(cvs)), replace=False))
    for xv in (-1.3, -0.4, 0.7):
        Hm = FA.MatFreeH(g.ops(xv, sh, s))
        for cv in pick:
            b = pr["blocks"][cv]; U = g.U_columns(bi, b["idx"])
            HU = np.column_stack([Hm(U[:, i]) for i in range(U.shape[1])])
            ref = U.conj().T @ HU; mine = b["H0"] + xv * b["H1"] + xv * xv * b["H2"]
            worst = max(worst, float(np.abs(ref - mine).max() / max(np.abs(ref).max(), 1e-300)))
    return worst


def fast_margin(g, x, sh, s):
    ev = FA.FastGroupK.evaluate(g, x, sh, s)
    rest = ev["mu"][np.argsort(np.abs(ev["mu"] - ev["lam"]))[ev["ncopies"]:]]
    return float(np.sort(rest)[0] - ev["lam"])


def gate_X(g, sh, s, x):
    a = fast_margin(g, x, sh, s); b = FA.check_margin(g, x, sh, s)
    return abs((b["mu"][0] - b["lam"]) - a) / max(1.0, abs(a))


def gate_S5(g, sh, s):
    """Quadratic pieces (main sector, top block) against a direct matrix-free H at three x."""
    pr = g.prepare(sh, s); bi = pr["bi"]; worst = 0.0
    b = pr["blocks"][pr["ctop"]]; U = g.U_columns(bi, b["idx"])
    for xv in (-0.7, 0.4, 1.9):
        Hm = FA.MatFreeH(g.ops(xv, sh, s))
        ref = U.conj().T @ np.column_stack([Hm(U[:, i]) for i in range(U.shape[1])])
        mine = b["H0"] + xv * b["H1"] + xv * xv * b["H2"]
        worst = max(worst, float(np.abs(ref - mine).max() / max(np.abs(ref).max(), 1e-300)))
    return worst


def gate_S5c(g, sh, s, rng):
    """Check-sector x-pieces against direct builds on a probe vector, at x = 1.9."""
    cp = FA.check_pieces(g, sh, s); xv = 1.9
    direct = FA.MatFreeH(g.ops(xv, sh, s))
    piece = FA.MatFreeH(D=cp["D"][0] + xv * cp["D"][1], Cv=cp["C"][0] + xv * cp["C"][1] + xv * xv * cp["C"][2], G=cp["G"][0] + xv * cp["G"][1])
    v = rng.standard_normal(direct.D.shape[1]) + 1j * rng.standard_normal(direct.D.shape[1])
    a, b = direct(v), piece(v); c_, d_ = direct.GG(v), piece.GG(v)
    return float(max(np.linalg.norm(a - b) / np.linalg.norm(a), np.linalg.norm(c_ - d_) / np.linalg.norm(c_)))


def checked_survival(g, sh, xs_with_phi, check_keys, ext_log):
    """Registration §3: checked x in order of decreasing main margin; stop at the first x that survives every check
    sector; within an x stop at the first sector that blocks her."""
    for xv, _ in sorted(xs_with_phi, key=lambda t: -t[1]):
        ok = True
        for s in check_keys:
            ev = g.evaluate(xv, sh, s); m = float(ev["mu"][0] - ev["lam"] + 1e-9 * max(1.0, abs(ev["lam"])))
            ext_log.append({"x": xv, "sector": s, "margin": m, "lam": ev["lam"], "gstar": ev["gstar"], "resid": ev["resid"],
                            "copies_removed": ev["ncopies_removed"]})
            if ev["gstar"] > 1e-9 * max(1.0, abs(ev["lam"])):
                raise SystemExit(f"stop: G-gauge (check) {g.angle} {sh} {s}: {ev['gstar']}")
            if m < 0:
                ok = False; break
        if ok:
            return True
    return False


def merge():
    out_path, a, b = Path(sys.argv[2]), Path(sys.argv[3]), Path(sys.argv[4])
    A, B = json.loads(a.read_text()), json.loads(b.read_text())
    res = {"protocol": A["protocol"], "merged_from": [str(a), str(b)]}
    for k in ("G_K0", "G_K1"):
        res[k] = {"g2_process": A.get(k), "sp3_process": B.get(k)}
    res["geometry"] = {**A.get("geometry", {}), **B.get("geometry", {})}
    res["circle_gates"] = {**A.get("circle_gates", {}), **B.get("circle_gates", {})}
    rows = A.get("points", []) + B.get("points", [])
    res["points"] = rows
    score_all(res, rows)
    out_path.write_text(json.dumps(res, indent=1, default=str) + "\n")
    print(json.dumps({k: v_ for k, v_ in res.items() if k not in ("points", "geometry")}, indent=1, default=str))


def main():
    if sys.argv[1] == "--merge":
        return merge()
    only = sys.argv[2] if len(sys.argv) > 2 else None
    out_path = Path(sys.argv[1]); res = {"protocol": "wp8_prereg_G_two_new_groups.md (revised after review)", "group": only}

    def save():
        out_path.write_text(json.dumps(res, indent=1, default=str) + "\n")

    def stop(msg):
        res["stopped"] = msg; save(); raise SystemExit("stop: " + msg)
    prev = json.loads(out_path.read_text()) if out_path.exists() else {}
    res["resumed_after"] = prev.get("stopped")
    res["G_K0"], res["G_K0_rows"] = gate_K0(); print("G-K0", res["G_K0"], flush=True); save()
    if res["G_K0"] > 1e-9:
        stop("G-K0")
    res["G_K1"], res["G_K1_detail"] = gate_K1(); print("G-K1", res["G_K1"], flush=True); save()
    if res["G_K1"] > 1e-8:
        stop("G-K1")
    rows = list(prev.get("points", [])); done = {(o["label"], tuple(o["shape"])) for o in rows}
    res["geometry"] = dict(prev.get("geometry", {})); res["circle_gates"] = dict(prev.get("circle_gates", {}))
    geos = {gname: geometry(gname) for gname in ("g2", "sp3")}
    rng = np.random.default_rng(2026)
    for c in circles():
        gname = c["group"]; geo = geos[gname]
        if only and gname != only:
            continue
        if all((c["label"], tuple(sh)) in done for sh in SHAPES):
            continue
        g, main_s, extra_s = build(c)
        # G-K2
        brk = 0.0
        for a in range(g.d):
            for b in range(g.d):
                Z = g.Y[a] @ g.Y[b] - g.Y[b] @ g.Y[a]
                brk = max(brk, float(np.abs(Z - sum(g.Cy[a, b, k] * g.Y[k] for k in range(g.d))).max()))
        reps = {k: FA.rep_check(g, k) for k in g.reps}
        wts, V, fv = FA.defining_weights(g)
        topq = {}
        for k in g.reps:
            got = float(np.linalg.eigvalsh(1j * g.reps[k][0]).max()) if g.reps[k][0].shape[0] > 1 else 0.0
            exp = float(sum(cf * wts[i] for i, cf in enumerate(HW[gname][k])) @ g.eK) if HW[gname][k] else 0.0
            topq[k] = (got, exp)
        dims = {k: g.reps[k][0].shape[0] for k in g.reps}
        gr = geometry_record(g, geo); gr.update(label=c["label"], angle=c["angle"], dims=dims, brackets=brk, reps=reps, top_charge=topq)
        res["geometry"][c["label"]] = gr; save()
        if brk > 1e-13 or max(reps.values()) > 1e-12 or any(abs(a_ - b_) > 1e-9 for a_, b_ in topq.values()):
            stop(f"G-K2 {c['label']}")
        # G-K3
        z, ties, ylen, par = EXPECT[c["label"]]
        bad = (gr["zero_roots"], gr["ties"], gr["y_theta_len"], gr["parallel"]) != (z, ties, ylen, par)
        if c["label"] in ("g4", "sS"):
            bad |= abs(abs(g.eK @ geo["thsv"]) / np.linalg.norm(geo["thsv"]) - 1) > 1e-9
        if c["label"] == "sS":
            s4 = res["geometry"].get("s4")
            bad |= s4 is None or sorted(map(tuple, np.round(s4["tied_top"], 9))) != sorted(map(tuple, np.round(gr["tied_top"], 9)))
        if bad:
            stop(f"G-K3 geometry {c['label']}: {gr}")
        # G-S5, G-S5c, G-B (circle-level gates, first shape)
        sh0 = SHAPES[0]; cg = dict(res["circle_gates"].get(c["label"], {}))
        cg["G_S5"] = gate_S5(g, sh0, LARGEST_MAIN[gname])
        cg["G_S5c"] = max(gate_S5c(g, sh0, s, rng) for s in extra_s)
        cg["G_B"] = max(gate_B(g, sh0, LARGEST_MAIN[gname], rng), gate_B(g, sh0, ADJ[gname], rng))
        res["circle_gates"][c["label"]] = cg; save()
        print(c["label"], "circle gates", cg, flush=True)
        if cg["G_S5"] > 1e-12 or cg["G_S5c"] > 1e-12 or cg["G_B"] > 1e-10:
            stop(f"G-S5/G-B {c['label']}: {cg}")
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
                    if ev["gstar"] > 1e-9 * max(1.0, abs(ev["lam"])):
                        stop(f"G-gauge {c['label']} {sh} {s}: {ev['gstar']}")
                mult[s] = {"top": int(len(pr["top"])), "bottom": int(len(pr["bot"])), "mirror_tied": g.evaluate(0.0, sh, s)["mirror_tied"]}
            if max(quad) > 1e-9:
                stop(f"G-quad {c['label']} {sh}: {max(quad)}")
            if sym > 1e-10:
                stop(f"G-sym {c['label']} {sh}: {sym}")
            keys = list(main_s)
            o = F.scan(g, sh, keys, keys)                     # floor set over the main sectors (F's scan)
            if abs(o["lin_second_diff"]) > 1e-9 or o["lin_Gstar"] > 1e-12:
                stop(f"G-lin {c['label']} {sh}")
            if o.get("I") and not o["bound_ok"]:
                stop(f"G-bound {c['label']} {sh}")
            ext_log = []
            xg = o["ext"][0][0] if o["I"] else -1.2
            if "G_X" not in res["circle_gates"][c["label"]]:
                gx = gate_X(g, sh, LARGEST_MAIN[gname], xg)
                res["circle_gates"][c["label"]]["G_X"] = gx; save()
                if gx > 1e-9:
                    stop(f"G-X {c['label']} {sh}: {gx}")
            if o["I"]:
                o["survives"] = checked_survival(g, sh, [(e_[0], e_[1]) for e_ in o["ext"]], list(extra_s), ext_log)
            else:
                o["survives"] = False
            o["check_log"] = ext_log
            o.update(group=gname, label=c["label"], angle=c["angle"], shape=list(sh), mult=mult, quad_dev=max(quad), sym=sym)
            theta_carries_yth = gr["y_theta_len"] == "L"
            o["calibration_edge"] = F.su2_calibration(g, sh if theta_carries_yth else (sh[0], sh[1], sh[1]))
            if gr["ties"] != "L":
                o["twist_content"] = twist_content(g, sh)
            o["seconds"] = time.time() - t0
            o["lives_point"] = bool(o["I"]) and o["survives"]
            rows.append(o); res["points"] = rows; save()
            print(c["label"], f"{c['angle']:.2f}", sh, "I", o["I"], "survives", o["survives"], "phi*", round(o.get("phi_grid_best", float("nan")), 4),
                  "golden", o.get("golden"), "rival", o.get("rival_above"), f"{o['seconds']:.0f}s", flush=True)
        g.cache.clear()
    if only:
        res["note"] = "one group only; score with --merge"; save(); print("done", only, flush=True); return
    score_all(res, rows); save()
    print(json.dumps({k: v_ for k, v_ in res.items() if k not in ("points", "geometry", "G_K0_rows")}, indent=1, default=str))


def score_all(res, rows):
    labels = [c["label"] for c in circles()]
    score = {}
    for lab in labels:
        n = sum(1 for o in rows if o["label"] == lab and o["lives_point"])
        score[lab] = {"n": n, "verdict": "lives" if n >= 2 else ("does not live" if n == 0 else "marginal")}
    res["score"] = score
    v = lambda lab: score[lab]["verdict"]                                                  # noqa: E731

    def decide(conds):
        """Supported if every cell has the needed verdict; refuted if some cell has the opposite one; else not decided."""
        if all(v(l) == w for l, w in conds):
            return "supported"
        if any(v(l) != "marginal" and v(l) != w for l, w in conds):
            return "refuted"
        return "not decided"

    def phistar(o):
        return o.get("phi_grid_best", o.get("golden", {}).get("phi", -np.inf))
    out = {}
    for gname, own in (("g2", "g0"), ("sp3", "s0")):
        arc = ARC[gname]; H_theta = decide([(own, "lives")])
        H_notP = "supported" if H_theta == "supported" and v(arc[2]) != "lives" else ("not decided" if H_theta == "not decided" else "refuted")
        cnt = 0; per = []
        for sh in SHAPES:
            vals = [phistar(next(o for o in rows if o["label"] == l and tuple(o["shape"]) == tuple(sh))) for l in arc]
            fin = [x for x in vals if np.isfinite(x)]
            srt = sorted(fin, reverse=True)
            okshape = bool(fin) and (len(srt) < 2 or srt[0] - srt[1] > 1e-6 * max(1.0, abs(srt[0])))
            k = int(np.argmax([x if np.isfinite(x) else -np.inf for x in vals]))
            near = bool(okshape and k <= 1); cnt += near
            per.append({"shape": list(sh), "phi_star": vals, "argmax": arc[k], "counts": near})
        H_near = "supported" if cnt >= 2 else "refuted"
        everywhere = all(v(l) == "lives" for l in arc)
        out[gname] = {"H_theta": H_theta, "H_notP": H_notP, "H_near": H_near, "H_near_detail": per,
                      "lives_everywhere_on_arc": everywhere,
                      "F_pattern": (H_theta == H_notP == H_near == "supported") and not everywhere}
    res["replication"] = out
    res["H_wall2"] = decide([("g4", "lives")])
    res["H_s"] = decide([("sS", "lives"), ("s4", "does not live"), ("s5", "does not live")])
    res["H_edge"] = decide([("sS", "lives"), ("s5", "lives"), ("s4", "does not live")])
    res["H_tie"] = decide([("sS", "lives"), ("s4", "lives"), ("s5", "lives"), ("g4", "does not live")])
    side = []
    for o in rows:
        if o["I"] and o.get("r_hi", -1) > 0 and o["calibration_edge"] is not None and o["calibration_edge"] <= 0:
            side.append((o["label"], o["shape"], o["x_hi"], o["x_hi"] <= 0))
    res["G_side_scored"] = side; res["G_side"] = "not informative" if not side else all(s_[-1] for s_ in side)


if __name__ == "__main__":
    main()
