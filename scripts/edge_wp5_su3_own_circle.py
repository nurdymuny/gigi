"""Pre-registration D (research/edge-theorem-20261122-d/wp5_prereg_D_su3_own_circle.md): SU(3) stretched along the
twist's own charge circle H = -i diag(1,0,-1), with the field x along it free.

Same machinery as Test C (scripts/edge_wp4_su3_free_fibre.py: evaluate / phi / floor_set / competitor, block-diagonal
in the eigenbasis of Q), with the basis rotated so that Y_2 = e_H, Y_7 = e_K (scripts/edge_wp5_design_checks.py), the
metric g = t on e_H, and the 5-parameter sub-family M = x e_H, w e_K, y12 I, y23 I, y13 I on the root planes.
Copies removed: b and her mirror (charge -(p+q+4)) when tied; any other tie is a competitor (reported).
usage: OMP_NUM_THREADS=1 python scripts/edge_wp5_su3_own_circle.py OUT.json
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su3_test2_family as T2        # noqa: E402
import edge_wp4_su3_free_fibre as C        # noqa: E402

SECTORS = T2.SECTORS
H_IDX, K_IDX = 2, 7
ROOTS = {"12": (0, 1), "13": (3, 4), "23": (5, 6)}


def rotated():
    X, Cx = T2.su3_data()
    XH = -1j * np.diag([1.0, 0, -1.0])
    h = np.array([float((-2 * np.trace(XH @ X[a])).real) for a in range(8)])
    eH = h / np.linalg.norm(h); eK = np.zeros(8); eK[2], eK[7] = eH[7], -eH[2]
    R = np.eye(8); R[2], R[7] = eH, eK
    Cy = np.einsum("ai,bj,ijk,ck->abc", R, R, Cx, R)
    return X, Cx, R, h, Cy


class GroupH(C.Group):
    def __init__(self):
        self.name = "su3H"
        self.X, self.Cx, self.R, self.h, Cy = rotated()
        self.C = self.f = Cy; self.n = self.r = 8
        self.sectors = SECTORS
        self.repsX = {pq: T2.su3_irrep(*pq, self.X) for pq in SECTORS}
        self.reps = {pq: [sum(self.R[a, b] * r_ for b, r_ in enumerate(self.repsX[pq])) for a in range(8)] for pq in SECTORS}
        self.vidx = H_IDX * 8 + H_IDX
        self.base = (0, 0)
        self.adH = np.einsum("c,cab->ba", self.R @ self.h, Cy)
        self.Q, self.top, self.U = {}, {}, {}

    def metric(self, t):
        g = np.ones(8); g[H_IDX] = t; return g

    def field(self, x, shape):
        w, ys, y13 = shape
        M = np.zeros((8, 8)); M[H_IDX, H_IDX] = x; M[K_IDX, K_IDX] = w
        for k, v in (("12", ys), ("23", ys), ("13", y13)):
            for i in ROOTS[k]:
                M[i, i] = v
        return [M[i] for i in range(8)]

    def topcharge(self, s):
        return s[0] + s[1] + 4

    def parts(self, s):
        return (1j * self.adH, 1j * self.adH, 1j * sum(self.h[a] * self.repsX[s][a] for a in range(8)))


def mirror_tied(grp, x, shape, t, s):
    """Is the state of charge -(p+q+4) at her value? (the mirror copy)"""
    H, G = grp.HG(x, shape, t, s)
    if s not in grp.Q:
        grp.fit(s, H, G)
    q, U = np.linalg.eigh(grp.Q[s]); top = grp.topcharge(s)
    lo = np.abs(q + top) < 1e-8
    if lo.sum() != 1:
        return None
    m = U[:, lo][:, 0]; b = grp.top[s]
    lm, lb = float((m.conj() @ H @ m).real), float((b.conj() @ H @ b).real)
    res = float(np.linalg.norm(H @ m - lm * m) / np.linalg.norm(H))     # Frobenius: stricter threshold below
    return abs(lm - lb) <= 1e-9 * max(1.0, abs(lb)) and res <= 1e-12


def main():
    out_path = Path(sys.argv[1]); res = {"protocol": "wp5_prereg_D_su3_own_circle.md (revised after review)"}

    def save():
        out_path.write_text(json.dumps(res, indent=1, default=str) + "\n")

    def stop(msg):
        res["stopped"] = msg; save(); raise SystemExit("stop: " + msg)
    # D-K1..K5: Test C's controls, same code
    k1 = T2.control_K1(); k3 = T2.control_K3(); k2 = T2.control_K2()
    res["D_K1_K3"] = {"K1": k1["pass"], "K2": k2["pass"], "K3": k3["pass"]}
    g2 = C.Group("su2"); rows = json.loads((C.R_ / "exact_edge.json").read_text())["edges"]; k4 = []
    for tv, hh in ((4.0, "1.5"), (8.0, "sqrt_t"), (3.0, "0.5")):
        r = [rr for rr in rows if rr["t"] == tv and rr["height"] == hh][0]
        st, nb = C.structure(g2, (r["y"],), tv)
        runs = C.floor_set(g2, (r["y"],), tv, {s: c[0] for s, c in nb.items()}, st["r_lo"], st["r_hi"])
        k4.append(abs(max(b_ for _, b_ in runs) - r["edge_S"]) if runs else np.inf)
    res["D_K4_max_dev"] = max(k4)
    g3 = C.Group("su3"); ref = {p["t"]: p for p in json.loads((C.R_ / "su3_test2_family2b.json").read_text())["points"]}
    k5 = 0.0
    for tv in (2.0, 4.0, 8.0):
        for s in SECTORS:
            e = C.evaluate(g3, 1 / (2 * np.sqrt(tv)), (0.5, 0.5), tv, s); rr = ref[tv]["sectors"][str(s)]
            k5 = max(k5, abs(e["lam"] - rr["extremal"]) / max(1, abs(rr["extremal"])), abs(e["mu"][0] - rr["slice_floor"]) / max(1, abs(rr["slice_floor"])))
    res["D_K5_max_rel"] = k5
    # D-K6: rotated basis at t = 1, Maurer-Cartan member, vs the unrotated builder
    gh = GroupH(); k6 = 0.0
    for s in SECTORS:
        e = C.evaluate(gh, 0.5, (0.5, 0.5, 0.5), 1.0, s)
        H0, G0 = T2.hessian_and_gauge(g3.C, np.ones(8), g3.C, T2.su3_background(np.ones(8)), g3.reps[s])
        mu0 = np.sort(T2.slice_spectrum(H0, G0))
        k6 = max(k6, float(np.abs(mu0 - e["mu"]).max() / max(1, np.abs(mu0).max())))
    res["D_K6_max_rel"] = k6
    print("D-K1..K3", res["D_K1_K3"], "K4", max(k4), "K5", k5, "K6", k6, flush=True)
    if not (all(res["D_K1_K3"].values()) and max(k4) <= 1e-6 and k5 <= 1e-9 and k6 <= 1e-12):
        stop("D-K controls")
    # D-K7: the same rotated code with the stretch on e_K (Weyl-conjugate to the lambda_8 stretch) at Maurer-Cartan
    gk = GroupH(); gk.name = "su3K"; gk.metric = lambda t: (lambda g: (g.__setitem__(K_IDX, t), g)[1])(np.ones(8))
    k7 = 0.0
    for tv in (2.0, 4.0, 8.0):
        for s in SECTORS:
            e = C.evaluate(gk, 0.5, (1 / (2 * np.sqrt(tv)), 0.5, 0.5), tv, s); rr = ref[tv]["sectors"][str(s)]
            # spectra only: the Weyl reflection taking lambda_8 to e_K also moves her charge circle, so b differs
            k7 = max(k7, abs(e["mu"][0] - rr["slice_floor"]) / max(1, abs(rr["slice_floor"])), abs(e["mu"][1] - rr["slice_second"]) / max(1, abs(rr["slice_second"])))
    res["D_K7_max_rel"] = k7; print("D-K7", k7, flush=True)
    if k7 > 1e-9:
        stop("D-K7")
    C.SYM.update(comm=0.0, res=0.0, gstar=0.0)
    gh.repsX.update({pq: T2.su3_irrep(*pq, gh.X) for pq in ((5, 0), (6, 0))})
    gh.reps.update({pq: [sum(gh.R[a, b] * r_ for b, r_ in enumerate(gh.repsX[pq])) for a in range(8)] for pq in ((5, 0), (6, 0))})
    g2 = C.Group("su2"); g2.name = "su2j4"; g2.sectors = list(range(5))
    shapes = [(tv, w, ys, y13) for tv in (2.0, 3.0, 4.0, 6.0, 8.0) for (w, ys, y13) in ((0.5, 0.5, 0.25), (0.5, 0.5, 0.75), (0.5, 1.0, 1.0), (1.0, 1.0, 1.5))]
    pts, kap_dev, quad_dev = [], 0.0, 0.0
    for (tv, w, ys, y13) in shapes:
        sh = (w, ys, y13); o = {"t": tv, "shape": sh}
        lam = {s: [C.lam_at(gh, xv, sh, tv, s) for xv in (-1.0, 0.0, 1.0)] for s in SECTORS}
        quad = max(abs(v[0] - 2 * v[1] + v[2] - 2.0) for v in lam.values()); quad_dev = max(quad_dev, quad)
        if quad > 1e-9:
            stop(f"D-quad at {(tv, sh)}: {quad}")
        for s in SECTORS:
            if s != (0, 0):
                kap = (lam[s][2] - lam[(0, 0)][2]) - (lam[s][1] - lam[(0, 0)][1])
                kap_dev = max(kap_dev, abs(kap - (s[0] + s[1]) / np.sqrt(tv)))
        if kap_dev > 1e-9:
            stop(f"D-kappa at {(tv, sh)}: {kap_dev}")
        vv, gs = [], 0.0
        for xv in (-1.0, 0.0, 1.0):
            Hm, G = gh.HG(xv, sh, tv, (0, 0)); e_ = np.zeros(Hm.shape[0], complex); e_[gh.vidx] = 1
            vv.append(float((e_ @ Hm @ e_).real)); gs = max(gs, float(np.linalg.norm(G.conj().T @ e_)))
        if abs(vv[0] - 2 * vv[1] + vv[2]) > 1e-9 or gs > 1e-12:
            stop(f"D-lin at {(tv, sh)}")
        o["v_values"] = vv; lb = lam[(0, 0)]
        A_, B_, C_ = (lb[0] - 2 * lb[1] + lb[2]) / 2, (lb[2] - lb[0]) / 2 - (vv[2] - vv[0]) / 2, lb[1] - vv[1]
        disc = B_ * B_ - 4 * A_ * C_
        if disc <= 0:
            o.update(I=[], I_reason="no real root: never the floor of sector (0,0) (revision item 2)", sigma=1)
            pts.append(o); res["points"] = pts; save(); print("point", tv, sh, "no real root", flush=True); continue
        r_lo, r_hi = sorted([(-B_ - np.sqrt(disc)) / (2 * A_), (-B_ + np.sqrt(disc)) / (2 * A_)])
        o.update(r_lo=r_lo, r_hi=r_hi, sigma=1)
        xs3 = (r_lo, 0.5 * (r_lo + r_hi), r_hi) if r_hi - r_lo < 0.6 else (r_lo + 0.3, 0.5 * (r_lo + r_hi), r_hi - 0.3)
        copies, ncount = {}, {}
        for s in SECTORS:
            for xv in xs3:
                if mirror_tied(gh, xv, sh, tv, s) is not True:
                    stop(f"D-mult at {(tv, sh)} sector {s} x={xv}")
            copies[s] = 2
            cts = []
            for xv in xs3:
                e = C.evaluate(gh, xv, sh, tv, s); cts.append(int((np.abs(e["mu"] - e["lam"]) <= 1e-9 * max(1.0, abs(e["lam"]))).sum()))
            ncount[str(s)] = cts
        o["n_b_counts"] = ncount
        o["I"] = C.floor_set(gh, sh, tv, copies, r_lo, r_hi)
        o["I_HGG"] = C.floor_set(gh, sh, tv, copies, r_lo, r_hi, which="full", refine=False)
        if not o["I"]:                                   # revision item 6: golden-section on the best bracket
            xs = np.arange(r_lo - 0.5, r_hi + 0.5 + 1e-9, 0.25)
            vals = [C.phi(gh, xv, sh, tv, copies) for xv in xs]; k = int(np.argmax(vals))
            a_, b_ = xs[max(k - 1, 0)], xs[min(k + 1, len(xs) - 1)]; gr = (np.sqrt(5) - 1) / 2
            c_, d_ = b_ - gr * (b_ - a_), a_ + gr * (b_ - a_); fc, fd = C.phi(gh, c_, sh, tv, copies), C.phi(gh, d_, sh, tv, copies)
            while b_ - a_ > 1e-6:
                if fc > fd:
                    b_, d_, fd = d_, c_, fc; c_ = b_ - gr * (b_ - a_); fc = C.phi(gh, c_, sh, tv, copies)
                else:
                    a_, c_, fc = c_, d_, fd; d_ = a_ + gr * (b_ - a_); fd = C.phi(gh, d_, sh, tv, copies)
            xm = 0.5 * (a_ + b_); fm = C.phi(gh, xm, sh, tv, copies, arg=True)
            o["golden"] = {"x": xm, "phi": fm[0], "blocking_sector": fm[1], "grid_best": float(vals[k])}
            if fm[0] >= 0:
                o["I"] = [[xm, xm]]; o["I_from_golden"] = True
        o["bound_ok"] = all(r_lo - 1e-9 <= a and b2 <= r_hi + 1e-9 for a, b2 in o["I"])
        if o["I"]:
            lo, hi = min(a for a, _ in o["I"]), max(b2 for _, b2 in o["I"])
            o["x_mid"], o["upper_edge"] = 0.5 * (lo + hi), hi
            if hi > lo:
                comp = C.competitor(gh, hi + 1e-4, sh, tv, copies)
                comp["slope_b"] = (C.lam_at(gh, hi + 1e-5, sh, tv, comp["sector"]) - C.lam_at(gh, hi - 1e-5, sh, tv, comp["sector"])) / 2e-5
                comp["rho_s_HF"] = abs(comp["slope_HF"]) / abs(comp["slope_b"]); comp["rho_s_direct"] = abs(comp["slope_direct"]) / abs(comp["slope_b"])
                o["competitor"] = comp
            # D-ext: sectors (5,0), (6,0) at every grid x in I
            ext = []
            for xv in np.arange(r_lo - 0.5, r_hi + 0.5 + 1e-9, 0.25):
                if any(a - 1e-12 <= xv <= b2 + 1e-12 for a, b2 in o["I"]):
                    m = min((lambda e: C.drop(e["mu"], e["lam"], 2)[0] - e["lam"])(C.evaluate(gh, xv, sh, tv, s)) for s in ((5, 0), (6, 0)))
                    ext.append((float(xv), float(m)))
            o["D_ext"] = ext; o["I_ext_gridpoints"] = [xv for xv, m in ext if m >= -1e-9]
        # SU(2) calibration (revision item 1): j <= 4 at y = 2 y13, same t
        st2, nb2 = C.structure(g2, (2 * y13,), tv)
        runs2 = C.floor_set(g2, (2 * y13,), tv, {s: c[0] for s, c in nb2.items()}, st2["r_lo"], st2["r_hi"])
        o["su2_calibration_x_hi_mapped"] = (max(b2 for _, b2 in runs2) / 2) if runs2 else None
        pts.append(o); res["points"] = pts; save()
        print("point", tv, sh, "r", (round(r_lo, 3), round(r_hi, 3)), "I", o["I"], "I_HGG", o["I_HGG"], "golden", o.get("golden"),
              "ext", o.get("I_ext_gridpoints"), "calib", o["su2_calibration_x_hi_mapped"], "comp", o.get("competitor"), flush=True)
        if not o["bound_ok"]:
            stop(f"D-bound at {(tv, sh)}")
    res["D_quad_max_dev"] = quad_dev; res["D_kappa_max_dev"] = kap_dev
    res["D_sym"] = dict(C.SYM); res["D_sym_pass"] = C.SYM["comm"] <= 1e-10 and C.SYM["res"] <= 1e-9 and C.SYM["gstar"] <= 1e-9
    ne = [o for o in pts if o["I"]]
    res["D_P1_weak"] = bool(ne)
    res["D_P1_strict_failures"] = [(o["t"], o["shape"]) for o in pts if o["t"] >= 3 and not o["I"]]
    res["D_P1_strict"] = not res["D_P1_strict_failures"]
    res["D_P1_truncation_sensitive"] = bool(ne) and all(not o.get("I_ext_gridpoints") for o in ne)
    scorable = [o for o in ne if o.get("r_hi", -1) > 0 and o.get("su2_calibration_x_hi_mapped") is not None and o["su2_calibration_x_hi_mapped"] <= 0]
    res["D_P2_scorable"] = len(scorable)
    res["D_P2_failures"] = [(o["t"], o["shape"], o["upper_edge"]) for o in scorable if not o["upper_edge"] <= 0]
    res["D_P2"] = "not informative" if not scorable else ("pass" if not res["D_P2_failures"] else "fail")
    res["D_P3_failures"] = [(o["t"], o["shape"], o["I"]) for o in ne if len(o["I"]) != 1]; res["D_P3"] = not res["D_P3_failures"]
    withc = [o for o in ne if "competitor" in o]
    res["D_P4_S_failures"] = [(o["t"], o["shape"], o["competitor"]["rho_s_HF"]) for o in withc if not o["competitor"]["rho_s_HF"] < 0.5]
    res["D_P4_S"] = "vacuous" if not withc else (not res["D_P4_S_failures"])
    save()
    print(json.dumps({k: v for k, v in res.items() if k not in ("points",)}, indent=1, default=str))


if __name__ == "__main__":
    main()
