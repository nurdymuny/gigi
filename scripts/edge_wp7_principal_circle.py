"""Pre-registration F (research/edge-theorem-20261122-f/wp7_prereg_F_principal_circle.md, revised after review):
which circle does she live on? A sweep of the stretched circle e_K across the Weyl chamber of SU(4) and Sp(2).

Fast evaluation: H(x) = H0 + x H1 + x^2 H2 and G(x) = G0 + x G1 exactly (gate F-S5), so per (sector, shape) the pieces
are built once, rotated into the eigenbasis of the field-independent charge Q_K, and each x needs only small block
eigensolves. Removal of her copies: the whole tied top space of Q_K plus the bottom (mirror) space when tied.
usage: OMP_NUM_THREADS=1 python scripts/edge_wp7_principal_circle.py OUT.json
"""
import json
import sys
import time
from pathlib import Path

import numpy as np
from scipy.optimize import brentq

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su3_test2_family as T2        # noqa: E402
import edge_wp7_groups as GR               # noqa: E402
import edge_wp4_su3_free_fibre as C        # noqa: E402

ANGLES = (0.0, 9.22, 18.43, 27.65, 36.87, 45.0)
SHAPES = ((0.5, 0.5, 0.25), (0.5, 0.5, 0.75), (0.5, 1.0, 1.0))
T = 3.0


def tens(*vs):
    v = np.array([1.0 + 0j])
    for x in vs:
        v = np.kron(v, x)
    return v


class GroupK:
    """A compact group stretched along e_K (unit), with the diagonal sub-family (x on e_K, w on the Cartan complement,
    y_theta on the plane of the root highest for K, y on the other planes)."""

    def __init__(self, gname, angle_deg=None, eK_torus=None, sectors=None, extra=()):
        self.gname = gname
        n, basis, tor = GR.algebra(gname)
        self.n_fund, self.basis, self.tor = n, basis, tor
        rts = GR.roots(basis, tor); self.rts = rts
        wts = np.array([w for w, _ in rts]); key = np.arange(1, len(tor) + 1, dtype=float)
        pos = [w for w in wts if w @ key > 1e-9]
        cor = [2 * w / (w @ w) for w in pos]; rhov = 0.5 * sum(cor)
        theta = max(pos, key=lambda w: w @ rhov); thv = 2 * theta / (theta @ theta)
        u0 = thv / np.linalg.norm(thv); r = rhov / np.linalg.norm(rhov); perp = r - (r @ u0) * u0; perp /= np.linalg.norm(perp)
        self.u0, self.perp, self.rhov, self.thv = u0, perp, rhov, thv
        if eK_torus is None:
            a = np.radians(angle_deg); eK = np.cos(a) * u0 + np.sin(a) * perp
        else:
            eK = np.asarray(eK_torus, float); eK = eK / np.linalg.norm(eK)
        self.eK = eK; self.angle = angle_deg
        Y, R, plane_roots = GR.rotated(basis, tor, eK, rts)
        self.Y, self.R, self.plane_roots = Y, R, plane_roots
        self.d = len(Y); self.rank = len(tor)
        self.Cy = GR.structure(Y)
        pairs = [abs(w @ eK) for w in plane_roots]
        self.top_plane = int(np.argmax(pairs)); self.qK = float(max(pairs))
        self.top_root_unique = sum(abs(p - self.qK) < 1e-9 for p in pairs) == 1
        self.regular = min(abs(w @ eK) for w in pos) > 1e-9
        self.adK = np.einsum("c,cab->ba", np.eye(self.d)[0], self.Cy)
        # sectors
        e = np.eye(n); ops = [E for _, E in rts]
        self.reps = {}; self.repsK = {}
        for key_, (p, q, v) in (sectors or {}).items():
            self.add_sector(key_, p, q, v, ops)
        for key_, (p, q, v) in dict(extra).items():
            self.add_sector(key_, p, q, v, ops)
        self.cache = {}

    def add_sector(self, key_, p, q, v, ops):
        if p + q == 0:
            RX = [np.zeros((1, 1), complex) for _ in self.basis]
        else:
            RX = GR.irrep_from(self.basis, ops, np.asarray(v, complex), GR.rep_tensor(None, p, q))
        RY = [sum(self.R[k, a] * RX[a] for a in range(len(self.basis))) for k in range(self.d)]
        self.reps[key_] = RY

    def field(self, x, shape):
        w, y, yth = shape
        diag = np.full(self.d, y, float); diag[0] = x; diag[1:self.rank] = w
        for k in range(len(self.plane_roots)):
            if k == self.top_plane:
                diag[self.rank + 2 * k: self.rank + 2 * k + 2] = yth
        return [diag[i] * np.eye(self.d)[i] for i in range(self.d)]

    def HG(self, x, shape, s):
        g = np.ones(self.d); g[0] = T
        return T2.hessian_and_gauge(self.Cy, g, self.Cy, self.field(x, shape), self.reps[s])

    def prepare(self, shape, s):
        """Quadratic pieces in the Q_K eigenbasis, blocks, her copy indices."""
        key = (tuple(shape), s)
        if key in self.cache:
            return self.cache[key]
        H0, G0 = self.HG(0.0, shape, s); Hp, Gp = self.HG(1.0, shape, s); Hm, Gm = self.HG(-1.0, shape, s)
        H1 = (Hp - Hm) / 2; H2 = (Hp + Hm) / 2 - H0; G1 = (Gp - Gm) / 2
        dV = self.reps[s][0].shape[0]
        parts = (1j * self.adK, 1j * self.adK, 1j * self.reps[s][0])
        signs = T2.fit_signs(parts, Hp, Gp @ Gp.conj().T, self.d, self.d, dV)
        if len(signs) != 1:
            raise SystemExit(f"stop: F-sym sign fit {self.gname} {self.angle} {s}: {signs}")
        Q = T2.assemble_Q(parts, signs[0], self.d, self.d, dV)
        q, U = np.linalg.eigh(Q)
        if abs(q.min() + q.max()) > 1e-8 and q.max() < -q.min():
            Q = -Q; q, U = np.linalg.eigh(Q)
        qr = np.round(q, 7)
        blocks = {c: np.where(qr == c)[0] for c in np.unique(qr)}
        P = {k: U.conj().T @ M @ U for k, M in (("H0", H0), ("H1", H1), ("H2", H2))}
        P.update(Gs0=G0.conj().T @ U, Gs1=G1.conj().T @ U)
        top = np.where(np.abs(q - q.max()) < 1e-7)[0]; bot = np.where(np.abs(q - q.min()) < 1e-7)[0]
        out = {"q": q, "U": U, "blocks": blocks, "P": P, "top": top, "bot": bot, "Q": Q}
        self.cache[key] = out
        return out

    def evaluate(self, x, shape, s, want_vec=False):
        pr = self.prepare(shape, s); P = pr["P"]
        Hp = P["H0"] + x * P["H1"] + x * x * P["H2"]; Gs = P["Gs0"] + x * P["Gs1"]
        top, bot = pr["top"], pr["bot"]
        # Amendment 2: her state is the cheapest exact state of top charge (identical to the registered rule when the
        # top-charge space is one-dimensional); the mirror likewise in the bottom-charge space
        Ht = Hp[np.ix_(top, top)]; Ht = 0.5 * (Ht + Ht.conj().T); wt, vt = np.linalg.eigh(Ht)
        Hb = Hp[np.ix_(bot, bot)]; Hb = 0.5 * (Hb + Hb.conj().T); wb = np.linalg.eigvalsh(Hb)
        lam = float(wt[0]); top_spread = float(np.ptp(wt))
        mirror_tied = bool(abs(wb[0] - lam) <= 1e-9 * max(1.0, abs(lam)))
        others = np.setdiff1d(np.arange(Hp.shape[0]), top)
        resid = float(np.linalg.norm(Hp[np.ix_(others, top)]))
        gstar = float(np.linalg.norm(Gs[:, top] @ vt[:, 0]))
        svs = {c: np.linalg.svd(Gs[:, idx], compute_uv=False) for c, idx in pr["blocks"].items()}
        gmax = max((v.max() for v in svs.values() if v.size), default=0.0)
        mu, vecs = [], []
        for c, idx in pr["blocks"].items():
            Hc = Hp[np.ix_(idx, idx)]; Hc = 0.5 * (Hc + Hc.conj().T); Gc = Gs[:, idx]
            if Gc.shape[0] and gmax > 0:
                _, sv, Vh = np.linalg.svd(Gc, full_matrices=True)
                V = Vh.conj().T[:, int((sv > 1e-10 * gmax).sum()):]
            else:
                V = np.eye(len(idx), dtype=complex)
            if V.shape[1]:
                A = V.conj().T @ Hc @ V; A = 0.5 * (A + A.conj().T)
                if want_vec:
                    w_, W = np.linalg.eigh(A); mu.extend(w_); vecs.append(pr["U"][:, idx] @ V @ W)
                else:
                    mu.extend(np.linalg.eigvalsh(A))
        mu = np.array(mu); o = np.argsort(mu)
        ncopies = 1 + (1 if mirror_tied else 0)
        out = {"lam": lam, "lam_top_spread": 0.0, "top_block_spread": top_spread, "top_dim": int(len(top)),
               "mirror_tied": mirror_tied, "resid": resid,
               "gstar": gstar, "mu": mu[o], "ncopies": ncopies}
        if want_vec:
            out["W"] = np.hstack(vecs)[:, o]
        return out


# ------------------------------------------------------------------------------------------------- sectors
def sectors_for(gname):
    n = 4; e = np.eye(n)
    if gname == "su4":
        main = {"1": (0, 0, None), "4": (1, 0, e[0]), "4b": (0, 1, e[3]), "6": (2, 0, tens(e[0], e[1]) - tens(e[1], e[0])),
                "10": (2, 0, tens(e[0], e[0])), "10b": (0, 2, tens(e[3], e[3]))}
        extra = {"20s": (3, 0, tens(e[0], e[0], e[0]))}
    else:
        a = tens(e[0], e[1]) - tens(e[1], e[0])
        main = {"1": (0, 0, None), "4": (1, 0, e[0]), "5": (2, 0, a), "10": (2, 0, tens(e[0], e[0])),
                "14": (4, 0, np.kron(a, a)), "16": (3, 0, np.kron(e[0], a))}
        extra = {"20s": (3, 0, tens(e[0], e[0], e[0])), "35s": (4, 0, tens(e[0], e[0], e[0], e[0]))}
    return main, extra


def phi(g, x, shape, keys):
    best = (np.inf, None)
    for s in keys:
        ev = g.evaluate(x, shape, s)
        rest = ev["mu"][np.argsort(np.abs(ev["mu"] - ev["lam"]))[ev["ncopies"]:]]
        m = np.sort(rest)[0] - ev["lam"] + 1e-9 * max(1.0, abs(ev["lam"]))
        if m < best[0]:
            best = (m, s)
    return best


def bounding(g, shape):
    lam = [g.evaluate(xv, shape, "1")["lam"] for xv in (-1.0, 0.0, 1.0)]
    vv, gs = [], 0.0
    for xv in (-1.0, 0.0, 1.0):
        H, G = g.HG(xv, shape, "1"); e_ = np.zeros(H.shape[0], complex); e_[0] = 1
        vv.append(float((e_ @ H @ e_).real)); gs = max(gs, float(np.linalg.norm(G.conj().T @ e_)))
    A_ = (lam[0] - 2 * lam[1] + lam[2]) / 2; B_ = (lam[2] - lam[0]) / 2 - (vv[2] - vv[0]) / 2; C_ = lam[1] - vv[1]
    disc = B_ * B_ - 4 * A_ * C_
    roots = None if disc <= 0 else sorted([(-B_ - np.sqrt(disc)) / (2 * A_), (-B_ + np.sqrt(disc)) / (2 * A_)])
    return roots, lam, vv, gs


def scan(g, shape, keys, extra_keys):
    roots, lam, vv, gs = bounding(g, shape)
    o = {"quad_second_diff_sector1": lam[0] - 2 * lam[1] + lam[2], "lin_second_diff": vv[0] - 2 * vv[1] + vv[2], "lin_Gstar": gs}
    if roots is None:
        o.update(I=[], reason="no real root (never the floor of the trivial sector)", survives=False)
        return o
    r_lo, r_hi = roots; o.update(r_lo=r_lo, r_hi=r_hi)
    f = lambda xv: phi(g, xv, shape, keys)[0]                                            # noqa: E731
    xs = np.arange(r_lo - 0.5, r_hi + 0.5 + 1e-9, 0.25); vals = [f(xv) for xv in xs]; sg = [v >= 0 for v in vals]
    runs, i = [], 0
    while i < len(xs):
        if sg[i]:
            j = i
            while j + 1 < len(xs) and sg[j + 1]:
                j += 1
            lo = brentq(f, xs[i - 1], xs[i], xtol=1e-8) if i > 0 else xs[i]
            hi = brentq(f, xs[j], xs[j + 1], xtol=1e-8) if j + 1 < len(xs) else xs[j]
            runs.append([float(lo), float(hi)]); i = j + 1
        else:
            i += 1
    o["I"] = runs; o["phi_grid_best"] = float(max(vals)); check = []
    if not runs:
        k = int(np.argmax(vals)); a_, b_ = xs[max(k - 1, 0)], xs[min(k + 1, len(xs) - 1)]; gr = (np.sqrt(5) - 1) / 2
        c_, d_ = b_ - gr * (b_ - a_), a_ + gr * (b_ - a_); fc, fd = f(c_), f(d_)
        while b_ - a_ > 1e-6:
            if fc > fd:
                b_, d_, fd = d_, c_, fc; c_ = b_ - gr * (b_ - a_); fc = f(c_)
            else:
                a_, c_, fc = c_, d_, fd; d_ = a_ + gr * (b_ - a_); fd = f(d_)
        xm = 0.5 * (a_ + b_); pm = phi(g, xm, shape, keys)
        o["golden"] = {"x": xm, "phi": pm[0], "blocking": pm[1]}
        if pm[0] >= 0:
            o["I"] = [[xm, xm]]; check = [xm]
    else:
        check = [float(xv) for xv in xs if any(a - 1e-12 <= xv <= b + 1e-12 for a, b in runs)] or [0.5 * (runs[0][0] + runs[0][1])]
    o["bound_ok"] = all(r_lo - 1e-9 <= a and b <= r_hi + 1e-9 for a, b in o["I"])
    ext = [(xv, phi(g, xv, shape, extra_keys)) for xv in check]
    o["ext"] = [(xv, float(p[0]), p[1]) for xv, p in ext]
    o["survives"] = any(p[0] >= 0 for _, p in ext)
    if o["I"]:
        hi = max(b for _, b in o["I"]); o["x_hi"] = hi
        o["rival_above"] = phi(g, hi + 1e-4, shape, keys)[1]
    return o


def su2_calibration(g, shape):
    """D's map: her SU(2) at x_SU2 = 2(x cos phi + w sin phi), y_SU2 = 2 y_theta, t_SU2 = t cos^2 + sin^2; j <= 4."""
    phi_ = np.radians(g.angle); w, y, yth = shape
    g2 = C.Group("su2"); g2.name = "su2cal"; g2.sectors = list(range(5))
    t2 = T * np.cos(phi_) ** 2 + np.sin(phi_) ** 2
    st, nb = C.structure(g2, (2 * yth,), t2)
    runs = C.floor_set(g2, (2 * yth,), t2, {s: c[0] for s, c in nb.items()}, st["r_lo"], st["r_hi"])
    if not runs:
        return None
    xs2 = max(b for _, b in runs)
    return (xs2 / 2 - w * np.sin(phi_)) / np.cos(phi_)


def known_answers():
    """F-K0 (SU(3), K = H, against D's code) and F-K1 (SU(3), K = hypercharge, against C's code)."""
    import edge_wp5_su3_own_circle as D
    e = np.eye(3)
    secs = {}
    for (p, q) in T2.SECTORS:
        v = tens(*([e[0]] * p + [e[2]] * q)) if p + q else None
        secs[str((p, q))] = (p, q, v)
    out = {}
    # F-K0: K = H = theta-vee = rho-vee in SU(3), at D's first shape, t = 2, x = -1.5
    global T
    T_save = T; T = 2.0
    gF = GroupK("su3", angle_deg=0.0, sectors=secs)
    gD = D.GroupH(); best = {}
    for sgn in (1, -1):                                   # e_K orientation: e_K -> -e_K is x -> -x
        worst = 0.0
        for (p, q) in T2.SECTORS:
            a = gF.evaluate(-1.5 * sgn, (0.5, 0.5, 0.25), str((p, q)))
            b = C.evaluate(gD, -1.5, (0.5, 0.5, 0.25), 2.0, (p, q))
            worst = max(worst, abs(a["lam"] - b["lam"]) / max(1, abs(b["lam"])), abs(a["mu"][0] - b["mu"][0]) / max(1, abs(b["mu"][0])))
        best[sgn] = worst
    out["F_K0_by_orientation"] = best; out["F_K0"] = min(best.values())
    # F-K1: K = hypercharge (lambda_8) in SU(3), field u = y = 1 (matches C's family), t = 2, x = 0.3
    lam8 = np.array([0.0, 1.0])                                                         # torus coords: tor[1] ~ lambda_8
    gC1 = GroupK("su3", eK_torus=lam8, sectors=secs); gC = C.Group("su3"); best = {}
    for sgn in (1, -1):
        worst = 0.0
        for (p, q) in T2.SECTORS:
            a = gC1.evaluate(0.3 * sgn, (1.0, 1.0, 1.0), str((p, q)))
            b = C.evaluate(gC, 0.3, (1.0, 1.0), 2.0, (p, q))
            worst = max(worst, abs(np.sort(a["mu"])[0] - b["mu"][0]) / max(1, abs(b["mu"][0])))
        best[sgn] = worst
    out["F_K1_by_orientation"] = best; out["F_K1"] = min(best.values())
    T = T_save
    return out


def main():
    out_path = Path(sys.argv[1]); res = {"protocol": "wp7_prereg_F_principal_circle.md (revised after review)"}

    def save():
        out_path.write_text(json.dumps(res, indent=1, default=str) + "\n")

    def stop(msg):
        res["stopped"] = msg; save(); raise SystemExit("stop: " + msg)
    prev = json.loads(out_path.read_text()) if out_path.exists() else {}
    if "F_K0" in prev:                                      # resume (Amendment 2): keep gates and computed points
        ka = {k: prev[k] for k in ("F_K0", "F_K1", "F_K0_by_orientation", "F_K1_by_orientation") if k in prev}
        res["resumed_after"] = prev.get("stopped"); res["amendment"] = "wp7_prereg_F_amendment2_wall.md"
    else:
        ka = known_answers()
    res.update(ka); print("known answers", {k: ka[k] for k in ("F_K0", "F_K1")}, flush=True)
    if ka["F_K0"] > 1e-9 or ka["F_K1"] > 1e-9:
        stop("F-K0/F-K1")
    rows = list(prev.get("points", []))
    done = {(o["group"], o["angle"], tuple(o["shape"])) for o in rows}
    for gname in ("sp2", "su4"):
        main_s, extra_s = sectors_for(gname)
        for ang in ANGLES:
            g = GroupK(gname, angle_deg=ang, sectors=main_s, extra=extra_s)
            geo = {"group": gname, "angle": ang, "regular": g.regular, "top_root_unique": g.top_root_unique, "q_K": g.qK}
            exp_q = {0.0: 1.0, 9.22: 0.987, 18.43: 0.949, 27.65: 0.886, 36.87: 0.800, 45.0: 0.707}[ang]
            if (g.regular != (ang not in (0.0, 45.0)) or g.top_root_unique != (ang != 45.0) or abs(g.qK - exp_q) > 2e-3):
                stop(f"F-K3 geometry {gname} {ang}: {geo}")
            # F-S5: quadratic decomposition against direct builds
            sh = SHAPES[1]; s0 = list(main_s)[2]; pr = g.prepare(sh, s0); worst = 0.0
            for xv in (-0.7, 0.4, 1.9):
                H, G = g.HG(xv, sh, s0)
                Hp = pr["U"].conj().T @ H @ pr["U"]; Hq = pr["P"]["H0"] + xv * pr["P"]["H1"] + xv * xv * pr["P"]["H2"]
                worst = max(worst, float(np.abs(Hp - Hq).max() / np.abs(Hp).max()))
            geo["F_S5"] = worst
            if worst > 1e-12:
                stop(f"F-S5 {gname} {ang}: {worst}")
            for sh in SHAPES:
                if (gname, ang, tuple(sh)) in done:
                    continue
                t0 = time.time()
                quad = []; sym = 0.0; mult = {}
                for s in main_s:
                    pr = g.prepare(sh, s)
                    if len(pr["top"]) == 1:
                        lam = [g.evaluate(xv, sh, s)["lam"] for xv in (-1.0, 0.0, 1.0)]
                        quad.append(abs(lam[0] - 2 * lam[1] + lam[2] - 2 * g.qK ** 2))
                    else:                                  # Amendment 2: quadratic part of the top block is q_K^2 I
                        H2t = pr["P"]["H2"][np.ix_(pr["top"], pr["top"])]
                        quad.append(float(np.abs(H2t - g.qK ** 2 * np.eye(len(pr["top"]))).max()))
                    for xv in (-1.0, 0.0, 1.0):
                        ev = g.evaluate(xv, sh, s)
                        Hx = pr["P"]["H0"] + xv * pr["P"]["H1"] + xv * xv * pr["P"]["H2"]
                        qd = pr["q"][:, None] - pr["q"][None, :]
                        sym = max(sym, float(np.sqrt(np.sum(np.abs(Hx) ** 2 * qd ** 2)) / np.linalg.norm(Hx)))
                        if ev["resid"] > 1e-9 * max(1.0, np.abs(Hx).max()) or ev["lam_top_spread"] > 1e-9 * max(1, abs(ev["lam"])):
                            stop(f"F-mult {gname} {ang} {sh} {s}: resid {ev['resid']}, spread {ev['lam_top_spread']}")
                    mult[s] = {"top": int(len(pr["top"])), "bottom": int(len(pr["bot"])), "mirror_tied": g.evaluate(0.0, sh, s)["mirror_tied"]}
                if max(quad) > 1e-9:
                    stop(f"F-quad {gname} {ang} {sh}: {max(quad)}")
                if sym > 1e-10:
                    stop(f"F-sym {gname} {ang} {sh}: {sym}")
                o = scan(g, sh, list(main_s), list(main_s) + list(extra_s))
                if abs(o["lin_second_diff"]) > 1e-9 or o["lin_Gstar"] > 1e-12:
                    stop(f"F-lin {gname} {ang} {sh}")
                if o.get("I") and not o["bound_ok"]:
                    stop(f"F-bound {gname} {ang} {sh}")
                o.update(geo); o["shape"] = sh; o["mult"] = mult; o["quad_dev"] = max(quad); o["sym"] = sym
                o["calibration_edge"] = su2_calibration(g, sh); o["seconds"] = time.time() - t0
                o["lives_point"] = bool(o["I"]) and o["survives"]
                rows.append(o); res["points"] = rows; save()
                print(gname, ang, sh, "I", o["I"], "survives", o["survives"], "golden", o.get("golden"), "rival", o.get("rival_above"),
                      "cal", o["calibration_edge"], f"{o['seconds']:.0f}s", flush=True)
    # scoring
    score = {}
    for gname in ("sp2", "su4"):
        for ang in ANGLES:
            n = sum(1 for o in rows if o["group"] == gname and o["angle"] == ang and o["lives_point"])
            score[f"{gname}@{ang}"] = {"n": n, "verdict": "lives" if n >= 2 else ("does not live" if n == 0 else "marginal")}
    res["score"] = score
    def v(g_, a):
        return score[f"{g_}@{a}"]["verdict"]
    interior = (9.22, 18.43, 27.65, 36.87)
    res["H_P"], res["H_theta"], res["H_reg"], res["H_C"] = {}, {}, {}, {}
    for g_ in ("sp2", "su4"):
        lv = [v(g_, a) for a in ANGLES]
        living = [a for a in ANGLES if v(g_, a) == "lives"]
        contiguous = bool(living) and all(v(g_, a) == "lives" for a in ANGLES if min(living) <= a <= max(living))
        res["H_P"][g_] = (v(g_, 18.43) == "lives" and contiguous and v(g_, 0.0) == "does not live" and v(g_, 45.0) == "does not live")
        res["H_theta"][g_] = v(g_, 0.0) == "lives"
        res["H_reg"][g_] = all(v(g_, a) == "lives" for a in interior) and v(g_, 0.0) == "does not live" and v(g_, 45.0) == "does not live"
        res["H_C"][g_] = v(g_, 45.0) == "does not live"
        res.setdefault("marginal_cells", {})[g_] = [a for a in ANGLES if v(g_, a) == "marginal"]
    for h in ("H_P", "H_theta", "H_reg", "H_C"):
        res[h]["supported_both"] = all(res[h][g_] for g_ in ("sp2", "su4"))
    side = []
    for o in rows:
        if o["I"] and o.get("r_hi", -1) > 0 and o["calibration_edge"] is not None and o["calibration_edge"] <= 0:
            side.append((o["group"], o["angle"], o["shape"], o["x_hi"], o["x_hi"] <= 0))
    res["F_side_scored"] = side; res["F_side"] = "not informative" if not side else all(s[-1] for s in side)
    save()
    print(json.dumps({k: v_ for k, v_ in res.items() if k != "points"}, indent=1, default=str))


if __name__ == "__main__":
    main()
