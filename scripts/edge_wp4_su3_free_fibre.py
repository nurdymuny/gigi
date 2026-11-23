"""Pre-registration C (research/edge-theorem-20261121/wp4_prereg_C_su3.md, registered in commit 232ab8d, before any
SU(3) spectrum of this family was computed): an SU(3) family whose fibre field x is free.

Family (metric 2b, g = t on lambda_8): M_a = u X_a (a = 1,2,3), M_8 = x X_8, M_a = y X_a (a = 4..7), components in the
orthonormal coframe (as T2.su3_background). Nine sectors of Test 2. Her state b = top-charge eigenvector of Q (circle of
H = -i diag(1,0,-1), parts from T2.su3_charge, signs fitted once per sector by T2.fit_signs; Q is field-independent,
and [Q, H] is re-checked at every evaluated point).
Gates (stop before any prediction is read): C-K1..K3 (T2 controls rerun), C-K4 (SU(2) analogue with the same scan
code), C-K5 (Maurer-Cartan member vs su3_test2_family2b.json), C-sym, C-quad, C-lin, C-sigma (C-P2 untestable if it
fails), C-mult, C-bound. Predictions C-P1 (strict/weak), C-P2 (informative points), C-P3, C-P4 (S); descriptive (N), C-P5.
Slopes: the registered Hellmann-Feynman value w^H (dH/dx) w (central difference, h = 1e-5, checked against h = 1e-4) and,
reported alongside, the direct derivative of the competitor's slice eigenvalue (the slice moves with x, so HF on H alone
is not exact for slice eigenvalues; both are reported, (S) is scored with the registered HF value).
usage: OMP_NUM_THREADS=1 python scripts/edge_wp4_su3_free_fibre.py OUT.json
"""
import json
import sys
import time
from pathlib import Path

import numpy as np
from scipy.optimize import brentq

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su3_test2_family as T2        # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
R_ = ROOT / "research/su2-floor-family-20261119"
SECTORS = T2.SECTORS
TOLB = 1e-9


class Group:
    """Group data for the scan code: frame brackets C, gauge f, metric(t), field(x, shape), sectors, reps, Q parts."""

    def __init__(self, name):
        self.name = name
        if name == "su3":
            X, C = T2.su3_data(); self.X, self.C, self.f = X, C, C; self.n = self.r = 8
            self.sectors = SECTORS; self.reps = {pq: T2.su3_irrep(*pq, X) for pq in SECTORS}
            self.vidx = 7 * 8 + 7                      # theta^8 (x) X_8 in sector (0, 0)
            self.base = (0, 0)
        else:
            S, C, eps = T2.su2_data(); self.C, self.f = C, eps; self.n = self.r = 3
            self.sectors = list(range(13)); self.reps = {j: S.spin_rep(j) for j in self.sectors}
            self.vidx = 0                              # theta^0 (x) tau_0 in sector 0
            self.base = 0
        self.Q, self.top, self.U = {}, {}, {}

    def metric(self, t):
        return T2.su3_metric("2b", t) if self.name == "su3" else np.array([t, 1.0, 1.0])

    def field(self, x, shape):
        if self.name == "su3":
            u, y = shape; c = np.array([u, u, u, y, y, y, y, x])
            return [c[a] * np.eye(8)[a] for a in range(8)]
        (y,) = shape
        return [x * np.eye(3)[0], y * np.eye(3)[1], y * np.eye(3)[2]]

    def topcharge(self, s):
        return s[0] + s[1] + 4 if self.name == "su3" else s + 4

    def parts(self, s):
        rho = self.reps[s]
        if self.name == "su3":
            return T2.su3_charge(rho, self.X, self.C)
        return (1j * self.C[0].T, 1j * 2 * T2.adm(np.eye(3)[0], self.f), 1j * rho[0])

    def HG(self, x, shape, t, s):
        return T2.hessian_and_gauge(self.C, self.metric(t), self.f, self.field(x, shape), self.reps[s])

    def fit(self, s, H, G):
        d = self.reps[s][0].shape[0]
        signs = T2.fit_signs(self.parts(s), H, G @ G.conj().T, self.n, self.r, d)
        if len(signs) != 1:
            raise SystemExit(f"stop: C-sym sign fit {s}: {signs}")
        Q = T2.assemble_Q(self.parts(s), signs[0], self.n, self.r, d)
        tc = self.topcharge(s)
        if abs(np.linalg.eigvalsh(Q).max() - tc) > 1e-8:
            Q = -Q
        q, U = np.linalg.eigh(Q); top = np.abs(q - tc) < 1e-8
        if top.sum() != 1 or q.max() > tc + 1e-8:
            raise SystemExit(f"stop: C-sym top eigenspace {s}: {int(top.sum())}")
        self.Q[s], self.top[s] = Q, U[:, top][:, 0]


SYM = {"comm": 0.0, "res": 0.0, "gstar": 0.0}


def evaluate(grp, x, shape, t, s, want_vec=False):
    """lambda_b, slice spectrum (and eigenvectors), full H+GG* spectrum, with C-sym checks.
    Computed in the eigenbasis U of Q (field-independent, fixed once per sector): H, GG* and ker G* commute with Q, so
    every quantity splits into charge blocks. This is a unitary change of basis; the C-sym commutator is evaluated
    exactly as ||[Q, H]||_F^2 = sum |H'_ij|^2 (q_i - q_j)^2 with H' = U^H H U."""
    H, G = grp.HG(x, shape, t, s)
    if s not in grp.Q:
        grp.fit(s, H, G)
    if s not in grp.U:
        q, U = np.linalg.eigh(grp.Q[s]); q = np.round(q, 8)
        grp.U[s] = (q, U, {c: np.where(q == c)[0] for c in np.unique(q)}, int(np.argmax(q)))
    q, U, blocks, ib = grp.U[s]
    Hp = U.conj().T @ H @ U
    Gs = G.conj().T @ U                                   # G^* in the Q-eigenbasis (columns = basis states)
    nHF = np.linalg.norm(Hp)
    SYM["comm"] = max(SYM["comm"], float(np.sqrt(np.sum(np.abs(Hp) ** 2 * (q[:, None] - q[None, :]) ** 2)) / nHF))
    lam = float(Hp[ib, ib].real)
    col = Hp[:, ib].copy(); col[ib] -= lam
    svs, hev, mu, full, vecs = {}, [], [], [], []
    for c, idx in blocks.items():
        svs[c] = np.linalg.svd(Gs[:, idx], compute_uv=False) if Gs.shape[0] else np.zeros(0)
    gmax = max((v.max() for v in svs.values() if v.size), default=0.0)
    for c, idx in blocks.items():
        Hc = Hp[np.ix_(idx, idx)]; Hc = 0.5 * (Hc + Hc.conj().T)
        hev.append(np.linalg.eigvalsh(Hc))
        Gc = Gs[:, idx]
        if Gc.shape[0] and gmax > 0:
            _, sv, Vh = np.linalg.svd(Gc, full_matrices=True)
            rank = int((sv > 1e-10 * gmax).sum()); V = Vh.conj().T[:, rank:]
        else:
            V = np.eye(len(idx), dtype=complex)
        if V.shape[1]:
            A = V.conj().T @ Hc @ V; A = 0.5 * (A + A.conj().T)
            if want_vec:
                w, Wc = np.linalg.eigh(A); mu.extend(w); vecs.append(U[:, idx] @ V @ Wc)
            else:
                mu.extend(np.linalg.eigvalsh(A))
        F = Hc + Gc.conj().T @ Gc
        full.extend(np.linalg.eigvalsh(0.5 * (F + F.conj().T)))
    nH2 = max(np.abs(np.concatenate(hev)).max(), 1e-300)
    SYM["res"] = max(SYM["res"], float(np.linalg.norm(col) / nH2))
    SYM["gstar"] = max(SYM["gstar"], float(np.linalg.norm(Gs[:, ib]) / max(gmax, 1.0)))
    mu = np.array(mu); out = {"lam": lam, "full": np.sort(full)}
    if want_vec:
        W = np.hstack(vecs); o = np.argsort(mu); out.update(mu=mu[o], W=W[:, o])
    else:
        out["mu"] = np.sort(mu)
    return out


def drop(ev, lam, n):
    ev = np.array(ev); idx = np.argsort(np.abs(ev - lam))[:n]
    return np.delete(ev, idx)


_CACHE = {}


def phi(grp, x, shape, t, nb, which="mu", arg=False):
    key = (grp.name, float(x), tuple(shape), float(t))
    if key not in _CACHE:
        best = {"mu": (np.inf, None), "full": (np.inf, None)}
        for s in grp.sectors:
            e = evaluate(grp, x, shape, t, s)
            for w in ("mu", "full"):
                rest = drop(e[w], e["lam"], nb[s])
                v = rest[0] - e["lam"] + TOLB * max(1.0, abs(rest[0]))     # >= 0 iff she is the floor of this sector
                if v < best[w][0]:
                    best[w] = (v, s)
        _CACHE[key] = best
    b = _CACHE[key][which]
    return b if arg else b[0]


def lam_at(grp, x, shape, t, s):
    H, _ = grp.HG(x, shape, t, s)
    if s not in grp.Q:
        grp.fit(s, *grp.HG(x, shape, t, s))
    b = grp.top[s]
    return float((b.conj() @ H @ b).real)


def structure(grp, shape, t):
    """C-quad, C-lin, C-sigma, the bounding interval, C-mult."""
    o = {}
    lam = {s: [lam_at(grp, xv, shape, t, s) for xv in (-1.0, 0.0, 1.0)] for s in grp.sectors}
    o["quad_second_diff"] = {str(s): v[0] - 2 * v[1] + v[2] for s, v in lam.items()}
    vv, gs = [], 0.0
    for xv in (-1.0, 0.0, 1.0):
        H, G = grp.HG(xv, shape, t, grp.base)
        e = np.zeros(H.shape[0], complex); e[grp.vidx] = 1
        vv.append(float((e @ H @ e).real)); gs = max(gs, float(np.linalg.norm(G.conj().T @ e)))
    o["lin_second_diff"] = vv[0] - 2 * vv[1] + vv[2]; o["lin_Gstar_v"] = gs
    kap = {str(s): (lam[s][2] - lam[grp.base][2]) - (lam[s][1] - lam[grp.base][1]) for s in grp.sectors if s != grp.base}
    o["kappa"] = kap
    signs = {np.sign(v) for v in kap.values()}
    o["sigma"] = int(signs.pop()) if len(signs) == 1 else None
    lb = lam[grp.base]
    a2 = (lb[0] - 2 * lb[1] + lb[2]) / 2; a1 = (lb[2] - lb[0]) / 2; a0 = lb[1]
    c1 = (vv[2] - vv[0]) / 2; c0 = vv[1]
    A, B, Cc = a2, a1 - c1, a0 - c0
    disc = B * B - 4 * A * Cc
    if disc <= 0:
        raise SystemExit(f"stop: no real root of lambda_b - <v,Hv> at {shape}, t={t} (contradicts F2)")
    o["r_lo"], o["r_hi"] = sorted([(-B - np.sqrt(disc)) / (2 * A), (-B + np.sqrt(disc)) / (2 * A)])
    nb = {}
    for s in grp.sectors:
        counts = []
        for xv in (o["r_lo"] + 0.3, 0.5 * (o["r_lo"] + o["r_hi"]), o["r_hi"] - 0.3):
            e = evaluate(grp, xv, shape, t, s)
            counts.append(int((np.abs(e["mu"] - e["lam"]) <= TOLB * max(1.0, abs(e["lam"]))).sum()))
        nb[s] = counts
    o["n_b_counts"] = {str(s): c for s, c in nb.items()}
    return o, nb


def floor_set(grp, shape, t, nb_fixed, r_lo, r_hi, which="mu", step=0.25, refine=True):
    xs = np.arange(r_lo - 0.5, r_hi + 0.5 + 1e-9, step)
    f = lambda xv: phi(grp, xv, shape, t, nb_fixed, which)          # noqa: E731
    vals = [f(xv) for xv in xs]; sg = [v >= 0 for v in vals]
    runs, i = [], 0
    while i < len(xs):
        if sg[i]:
            j = i
            while j + 1 < len(xs) and sg[j + 1]:
                j += 1
            lo = brentq(f, xs[i - 1], xs[i], xtol=1e-8) if (refine and i > 0) else xs[i]
            hi = brentq(f, xs[j], xs[j + 1], xtol=1e-8) if (refine and j + 1 < len(xs)) else xs[j]
            runs.append([float(lo), float(hi)]); i = j + 1
        else:
            i += 1
    return runs


def competitor(grp, x, shape, t, nb):
    """Lowest non-b slice eigenvector over sectors at x: sector, Q-charge, HF slope (h=1e-5 vs 1e-4), direct slope."""
    best = None
    for s in grp.sectors:
        e = evaluate(grp, x, shape, t, s, want_vec=True)
        order = np.argsort(np.abs(e["mu"] - e["lam"]))[:nb[s]]
        keep = [k for k in range(len(e["mu"])) if k not in set(order)]
        k0 = keep[0]
        if best is None or e["mu"][k0] - e["lam"] < best[0]:
            best = (e["mu"][k0] - e["lam"], s, e["W"][:, k0], e["mu"][k0])
    _, s, w, mu0 = best
    Q = grp.Q[s]; qc = float((w.conj() @ Q @ w).real); qev = np.linalg.eigvalsh(Q)
    out = {"sector": s, "charge": qc, "charge_is_eigen": bool(np.abs(qev - qc).min() <= 1e-6)}
    hf = {}
    for h in (1e-5, 1e-4):
        Hp, _ = grp.HG(x + h, shape, t, s); Hm, _ = grp.HG(x - h, shape, t, s)
        hf[h] = float((w.conj() @ ((Hp - Hm) / (2 * h)) @ w).real)
    out["slope_HF"] = hf[1e-5]; out["slope_HF_h_check"] = abs(hf[1e-5] - hf[1e-4])

    def mu_s(xv):
        e = evaluate(grp, xv, shape, t, s)
        return drop(e["mu"], e["lam"], nb[s])[0]
    out["slope_direct"] = (mu_s(x + 1e-5) - mu_s(x - 1e-5)) / 2e-5
    out["slope_b"] = (lam_at(grp, x + 1e-5, shape, t, s) - lam_at(grp, x - 1e-5, shape, t, s)) / 2e-5
    out["slope_b_base"] = None
    return out


def main():
    out_path = Path(sys.argv[1]); res = {"protocol": "wp4_prereg_C_su3.md @ 232ab8d + Amendment C1 @ 4287efc + Amendment C2"}

    def save():
        out_path.write_text(json.dumps(res, indent=1, default=str) + "\n")

    def stop(msg):
        res["stopped"] = msg; save(); raise SystemExit("stop: " + msg)
    # C-K1..K3
    k1 = T2.control_K1(); k3 = T2.control_K3(); k2 = T2.control_K2()
    res["C_K1_K3"] = {"K1": k1["pass"], "K2": k2["pass"], "K3": k3["pass"]}
    print("C-K1..K3:", res["C_K1_K3"], flush=True)
    if not all(res["C_K1_K3"].values()):
        stop("C-K1..K3")
    # C-K4: SU(2) analogue with the same scan code
    g2 = Group("su2"); rows = json.loads((R_ / "exact_edge.json").read_text())["edges"]
    k4 = {"second_diff": [], "edges": []}
    for tv, h in ((4.0, "1.5"), (8.0, "sqrt_t"), (3.0, "0.5")):
        r = [rr for rr in rows if rr["t"] == tv and rr["height"] == h][0]
        st, nb = structure(g2, (r["y"],), tv)
        k4["second_diff"].append(max(abs(v - 2) for v in st["quad_second_diff"].values()))
        nbf = {s: c[0] for s, c in nb.items()}
        runs = floor_set(g2, (r["y"],), tv, nbf, st["r_lo"], st["r_hi"])
        upper = max(rr_[1] for rr_ in runs) if runs else None
        k4["edges"].append({"t": tv, "height": h, "edge": upper, "edge_S": r["edge_S"], "runs": runs, "n_b": nb})
    res["C_K4"] = k4
    k4ok = max(k4["second_diff"]) <= 1e-9 and all(e["edge"] is not None and abs(e["edge"] - e["edge_S"]) <= 1e-6 for e in k4["edges"])
    res["C_K4_pass"] = k4ok; print("C-K4:", k4ok, k4["second_diff"], [(e["edge"], e["edge_S"]) for e in k4["edges"]], flush=True)
    if not k4ok:
        stop("C-K4")
    # C-K5: Maurer-Cartan member vs Test 2b
    g3 = Group("su3")
    ref = {p["t"]: p for p in json.loads((R_ / "su3_test2_family2b.json").read_text())["points"]}
    k5 = 0.0
    for tv in (2.0, 4.0, 8.0):
        for s in SECTORS:
            e = evaluate(g3, 1 / (2 * np.sqrt(tv)), (0.5, 0.5), tv, s)
            rr = ref[tv]["sectors"][str(s)]
            k5 = max(k5, abs(e["lam"] - rr["extremal"]) / max(1, abs(rr["extremal"])),
                     abs(e["mu"][0] - rr["slice_floor"]) / max(1, abs(rr["slice_floor"])))
    res["C_K5_max_rel"] = k5; print("C-K5:", k5, flush=True)
    if k5 > 1e-9:
        stop("C-K5")
    # timing on one point, reported before the scan
    t0 = time.time(); phi(g3, 0.0, (1.0, 2.0), 8.0, {s: 1 for s in SECTORS}); res["timing_one_x_point_s"] = time.time() - t0
    print("timing, one x point (9 sectors):", res["timing_one_x_point_s"], flush=True); save()
    # structure gates at every shape point, before any scan
    shapes = [(tv, u, y) for tv in (2.0, 3.0, 4.0, 6.0, 8.0) for (u, y) in ((0.5, 0.5), (0.5, 1.0), (1.0, 1.0), (1.0, 2.0))]
    st_all = {}
    for (tv, u, y) in shapes:
        st, nb = structure(g3, (u, y), tv)
        quad = max(abs(v - 1.5) for v in st["quad_second_diff"].values())
        selfconj = {s for s in SECTORS if s[0] == s[1]}
        nmul = {s_: (s_[0] + 3) * (2 if s_ in selfconj else 1) for s_ in SECTORS}
        mult_ok = all(c[0] == c[1] == c[2] and c[0] >= nmul[s] for s, c in nb.items())   # Amendment C2
        st["extra_ties"] = {str(s): c[0] - nmul[s] for s, c in nb.items() if c[0] > nmul[s]}
        nb = {s: [nmul[s]] * 3 for s in nb}                                            # remove exactly her multiplet
        st.update(quad_dev=quad, mult_ok=mult_ok)
        st_all[(tv, u, y)] = (st, nb)
        print("shape", (tv, u, y), "quad", quad, "lin", st["lin_second_diff"], st["lin_Gstar_v"], "sigma", st["sigma"],
              "r", (st["r_lo"], st["r_hi"]), "n_b", st["n_b_counts"], flush=True)
        if quad > 1e-9:
            res["structure"] = {str(k): v[0] for k, v in st_all.items()}; stop(f"C-quad at {(tv, u, y)}")
        if abs(st["lin_second_diff"]) > 1e-9 or st["lin_Gstar_v"] > 1e-12:
            res["structure"] = {str(k): v[0] for k, v in st_all.items()}; stop(f"C-lin at {(tv, u, y)}")
        if not mult_ok:
            res["structure"] = {str(k): v[0] for k, v in st_all.items()}; stop(f"C-mult at {(tv, u, y)}")
    res["structure"] = {str(k): v[0] for k, v in st_all.items()}
    sig_ok = all(v[0]["sigma"] is not None for v in st_all.values()) and len({v[0]["sigma"] for v in st_all.values()}) == 1
    res["C_sigma"] = sig_ok; save()
    # scans
    pts = []
    for (tv, u, y) in shapes:
        st, nb = st_all[(tv, u, y)]; nbf = {s: c[0] for s, c in nb.items()}
        o = {"t": tv, "u": u, "y": y, "r_lo": st["r_lo"], "r_hi": st["r_hi"], "sigma": st["sigma"]}
        o["I"] = floor_set(g3, (u, y), tv, nbf, st["r_lo"], st["r_hi"])
        o["I_HGG"] = floor_set(g3, (u, y), tv, nbf, st["r_lo"], st["r_hi"], which="full", refine=False)
        o["bound_ok"] = all(st["r_lo"] - 1e-9 <= a and b_ <= st["r_hi"] + 1e-9 for a, b_ in o["I"])
        if o["I"]:
            lo, hi = min(a for a, _ in o["I"]), max(b_ for _, b_ in o["I"])
            o["x_mid"] = 0.5 * (lo + hi); sg = st["sigma"] or 1
            o["upper_edge"] = hi if sg > 0 else lo
            o["informative"] = not (sg * 0.5 * (st["r_lo"] + st["r_hi"]) < 0)
            xc = o["upper_edge"] + (1e-4 if sg > 0 else -1e-4)
            comp = competitor(g3, xc, (u, y), tv, nbf)
            comp["slope_b"] = (lam_at(g3, o["upper_edge"] + 1e-5, (u, y), tv, comp["sector"])
                               - lam_at(g3, o["upper_edge"] - 1e-5, (u, y), tv, comp["sector"])) / 2e-5
            comp["rho_s_HF"] = abs(comp["slope_HF"]) / abs(comp["slope_b"])
            comp["rho_s_direct"] = abs(comp["slope_direct"]) / abs(comp["slope_b"])
            o["competitor"] = comp
        pts.append(o); res["points"] = pts; save()
        print("point", (tv, u, y), "I", o["I"], "I_HGG", o["I_HGG"], "comp", o.get("competitor"), flush=True)
        if not o["bound_ok"]:
            stop(f"C-bound at {(tv, u, y)}")
    res["C_sym"] = dict(SYM); res["C_sym_pass"] = SYM["comm"] <= 1e-10 and SYM["res"] <= 1e-9 and SYM["gstar"] <= 1e-9
    nonempty = [o for o in pts if o["I"]]
    res["C_P1_weak"] = bool(nonempty)
    res["C_P1_strict_failures"] = [(o["t"], o["u"], o["y"]) for o in pts if o["t"] >= 3 and not o["I"]]
    res["C_P1_strict"] = not res["C_P1_strict_failures"]
    if not sig_ok:
        res["C_P2"] = "untestable"
    else:
        inf = [o for o in nonempty if o["informative"]]
        res["C_P2_informative_points"] = len(inf)
        res["C_P2_failures"] = [(o["t"], o["u"], o["y"], o["x_mid"]) for o in inf if not o["sigma"] * o["x_mid"] < 0]
        res["C_P2"] = "not informative" if not inf else ("pass" if not res["C_P2_failures"] else "fail")
    res["C_P3_failures"] = [(o["t"], o["u"], o["y"], o["I"]) for o in nonempty if len(o["I"]) != 1]
    res["C_P3"] = not res["C_P3_failures"]
    res["C_P4_S_failures"] = [(o["t"], o["u"], o["y"], o["competitor"]["rho_s_HF"]) for o in nonempty if not o["competitor"]["rho_s_HF"] < 0.5]
    res["C_P4_S"] = not res["C_P4_S_failures"]
    res["C_P4_S_direct_failures"] = [(o["t"], o["u"], o["y"], o["competitor"]["rho_s_direct"]) for o in nonempty if not o["competitor"]["rho_s_direct"] < 0.5]
    res["C_P5_upper_edge_vs_t"] = {f"{u},{y}": [(o["t"], o.get("upper_edge")) for o in pts if (o["u"], o["y"]) == (u, y)]
                                   for (u, y) in ((0.5, 0.5), (0.5, 1.0), (1.0, 1.0), (1.0, 2.0))}
    save()
    print(json.dumps({k: v for k, v in res.items() if k not in ("points", "structure", "C_K4")}, indent=1, default=str))


if __name__ == "__main__":
    main()
