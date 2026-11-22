"""Pre-registration B (research/edge-theorem-20261121/wp4_prereg_B_general_fields.md, registered in commit 232ab8d,
before any B point was computed): random circle-symmetric fields, and a random (rho, t) scan of the reduced family.

B-I (code control, all gates stop the run):
  200 draws, numpy default_rng(1122): k in {+1, -1}, x ~ U[-10, 10], y ~ U[0, 6], z ~ U[-6, 6], t ~ U[0.5, 15], a random
  constant gauge frame R (QR of a Gaussian, det +1). Field (prereg table): k = +1: m0 = x e0, m1 = y e1 - z e2,
  m2 = z e1 + y e2; k = -1: m1 = -y e1 + z e2, m2 = z e1 + y e2; every Lie vector rotated by R. Sectors j <= 8, the
  general builder T2.hessian_and_gauge, no reduction. Q fitted by T2.fit_signs from the parts
  (i C[0]^T, i 2 adm(R e0), i rho_0) (the convention of T2.control_K2); exactly one sign choice must pass.
  B-sym, B-exact, B-red, B-spec, B-even as registered.
B-II (the part that can fail): for each draw's (rho, t), the upper edge on x in [-10, 10] from Blocks.phi_A (j <= 12),
  last >= 0 -> < 0 sign change on a 0.01 grid, brentq to 1e-13; competitor (j, q) at x* + 1e-4; slopes by
  Hellmann-Feynman (dB/dx exact: B is quadratic in x, so the central difference with step 1 is exact up to rounding);
  twist slope 2(x + (j+2)/sqrt t) + 2 sqrt t in the competitor's sector. Gates B-II-code, B-count; scored B-S, B-half;
  descriptive B-N, rho_s distribution, q_comp on rho < 1.1, lower edge.
usage: OMP_NUM_THREADS=1 python scripts/edge_wp4_general_fields.py OUT.json
"""
import json
import sys
from pathlib import Path

import numpy as np
from scipy.optimize import brentq

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_exact_edge as EE          # noqa: E402
import ricci_su3_test2_family as T2        # noqa: E402
from edge_wp4_heldout import r_bounds      # noqa: E402  (closed-form lower/upper roots, prereg A section 2)

ROOT = Path(__file__).resolve().parent.parent
R_ = ROOT / "research/su2-floor-family-20261119"
JB = 8
L = np.array([[0, 0, 0], [0, 0, -1.0], [0, 1.0, 0]])


def field(k, x, y, z):
    M = np.array([[x, 0, 0], [0, y, -z], [0, z, y]], float) if k == 1 else \
        np.array([[x, 0, 0], [0, -y, z], [0, z, y]], float)
    assert np.abs(L @ M - k * M @ L).max() < 1e-14, "field not invariant"
    return M


def gauge_frame(rng):
    Qm, Rm = np.linalg.qr(rng.standard_normal((3, 3)))
    Qm = Qm @ np.diag(np.sign(np.diag(Rm)))
    if np.linalg.det(Qm) < 0:
        Qm[:, 0] *= -1
    return Qm


def rel(a, b):
    return float(np.abs(np.asarray(a) - np.asarray(b)).max() / max(1.0, np.abs(a).max()))


def b_one(S, C2, eps, reps, k, x, y, z, t, Rg):
    """B-I gates for one draw. Returns a dict of worst values and a list of failures."""
    M = field(k, x, y, z) @ Rg.T                 # rows m_i, each Lie vector rotated by R
    rho_ = float(np.hypot(y, z)); xr = x if k == 1 else -x
    Mred = [xr * np.eye(3)[0], rho_ * np.eye(3)[1], rho_ * np.eye(3)[2]]
    axis = Rg @ np.eye(3)[0]
    g = [t, 1.0, 1.0]; out = {"sym": 0.0, "exact": 0.0, "gstar": 0.0, "red": 0.0, "spec_full": 0.0, "spec_slice": 0.0}
    fails = []
    for j in range(JB + 1):
        rho = reps[j]; d = j + 1
        H, G = T2.hessian_and_gauge(C2, g, eps, [M[i] for i in range(3)], rho)
        GG = G @ G.conj().T
        parts = (1j * C2[0].T, 1j * 2 * T2.adm(axis, eps), 1j * rho[0])
        signs = T2.fit_signs(parts, H, GG, 3, 3, d)
        if len(signs) != 1:
            fails.append(("B-sym signs", j, signs)); continue
        Q = T2.assemble_Q(parts, signs[0], 3, 3, d)
        if abs(np.linalg.eigvalsh(Q).max() - (j + 4)) > 1e-8:
            Q = -Q
        out["sym"] = max(out["sym"], float(np.linalg.norm(Q @ H - H @ Q) / np.linalg.norm(H)))
        q, U = np.linalg.eigh(Q)
        top = np.abs(q - (j + 4)) < 1e-8
        if top.sum() != 1 or q.max() > j + 4 + 1e-8:
            fails.append(("B-sym top dim", j, int(top.sum()))); continue
        b = U[:, top][:, 0]
        lam = float((b.conj() @ H @ b).real)
        res = float(np.linalg.norm(H @ b - lam * b) / np.linalg.norm(H, 2))
        gs = float(np.linalg.norm(G.conj().T @ b) / np.linalg.norm(G, 2))
        out["exact"] = max(out["exact"], res); out["gstar"] = max(out["gstar"], gs)
        lr = EE.lam_tw(xr, rho_, t, j)
        out["red"] = max(out["red"], abs(lam - lr) / max(1.0, abs(lr)))
        Hr, Gr = T2.hessian_and_gauge(C2, g, eps, Mred, rho)
        out["spec_full"] = max(out["spec_full"], rel(np.linalg.eigvalsh(Hr + Gr @ Gr.conj().T), np.linalg.eigvalsh(H + GG)))
        out["spec_slice"] = max(out["spec_slice"], rel(T2.slice_spectrum(Hr, Gr), T2.slice_spectrum(H, G)))
    if out["sym"] > 1e-10: fails.append(("B-sym", out["sym"]))
    if out["exact"] > 1e-9 or out["gstar"] > 1e-9: fails.append(("B-exact", out["exact"], out["gstar"]))
    if out["red"] > 1e-10: fails.append(("B-red", out["red"]))
    if max(out["spec_full"], out["spec_slice"]) > 1e-12: fails.append(("B-spec", out["spec_full"], out["spec_slice"]))
    return out, fails


def upper_edge(bl, yv, tv, lo=-10.0, hi=10.0, step=0.01):
    xs = np.round(np.arange(lo, hi + 1e-9, step), 10)
    vals = np.array([bl.phi_A(xv, yv, tv) for xv in xs])
    k = None
    for i in range(len(xs) - 1):
        if vals[i] >= 0 > vals[i + 1]:
            k = i
    if k is None:
        return None
    return brentq(lambda xv: bl.phi_A(xv, yv, tv), xs[k], xs[k + 1], xtol=1e-13)


def comp_slope(bl, xv, yv, tv, j, q):
    _, B = bl.block(bl.data(xv, yv, tv), j, q, tv)
    _, Bp = bl.block(bl.data(xv + 1.0, yv, tv), j, q, tv)
    _, Bm = bl.block(bl.data(xv - 1.0, yv, tv), j, q, tv)
    w, V = np.linalg.eigh(B); v = V[:, 0]
    return float((v.conj() @ ((Bp - Bm) / 2.0) @ v).real)


def b2_point(bl, rho_, tv):
    o = {"rho": rho_, "t": tv}
    xs_ = upper_edge(bl, rho_, tv)
    o["x_star"] = xs_
    if xs_ is None:
        return o
    j, q = bl.phi_A(xs_ + 1e-4, rho_, tv, True)[1]
    o["competitor"] = (j, q)
    sc = comp_slope(bl, xs_, rho_, tv, j, q)
    st = 2 * (xs_ + (j + 2) / np.sqrt(tv)) + 2 * np.sqrt(tv)
    o["slope_comp"], o["slope_twist"] = sc, st
    o["rho_s"] = abs(sc) / abs(st) if st != 0 else np.inf
    r_lo, r_hi, adev = r_bounds(rho_, tv)
    o["r_lo"], o["r_hi"], o["affine_dev"] = r_lo, r_hi, adev
    xs = np.round(np.arange(r_lo - 0.5, xs_ + 0.5 + 1e-9, 0.01), 10)
    sg = np.array([bl.phi_A(xv, rho_, tv) >= 0 for xv in xs])
    changes = [(i, "up" if sg[i + 1] else "down") for i in range(len(sg) - 1) if sg[i] != sg[i + 1]]
    o["half_changes"] = [c[1] for c in changes]
    o["B_half"] = [c[1] for c in changes] == ["up", "down"]
    if changes and changes[0][1] == "up":
        i = changes[0][0]
        o["x_lo"] = brentq(lambda xv: bl.phi_A(xv, rho_, tv), xs[i], xs[i + 1], xtol=1e-12)
        o["x_lo_block"] = bl.phi_A(o["x_lo"] - 1e-4, rho_, tv, True)[1]
    return o


def main():
    out_path = Path(sys.argv[1]); res = {"protocol": "wp4_prereg_B_general_fields.md @ 232ab8d"}
    S, C2, eps = T2.su2_data(); reps = {j: S.spin_rep(j) for j in range(JB + 1)}
    rng = np.random.default_rng(1122)
    draws = []
    for _ in range(200):
        k = int(rng.choice([1, -1])); x = rng.uniform(-10, 10); y = rng.uniform(0, 6); z = rng.uniform(-6, 6)
        t = rng.uniform(0.5, 15); Rg = gauge_frame(rng)
        draws.append((k, x, y, z, t, Rg))
    worst = {}; fails = []
    for n, (k, x, y, z, t, Rg) in enumerate(draws):
        w, f = b_one(S, C2, eps, reps, k, x, y, z, t, Rg)
        for kk, v in w.items():
            worst[kk] = max(worst.get(kk, 0.0), v)
        fails += [(n,) + tuple(ff) for ff in f]
    # B-even: 10 extra draws, y = z = 0: spectrum at x equals spectrum at -x
    even = 0.0
    for _ in range(10):
        x = rng.uniform(-10, 10); t = rng.uniform(0.5, 15); Rg = gauge_frame(rng)
        for j in range(JB + 1):
            sp_ = []
            for xv in (x, -x):
                Mv = np.array([[xv, 0, 0], [0, 0, 0], [0, 0, 0.]]) @ Rg.T
                H, G = T2.hessian_and_gauge(C2, [t, 1, 1], eps, [Mv[i] for i in range(3)], reps[j])
                sp_.append(np.linalg.eigvalsh(H + G @ G.conj().T))
            even = max(even, rel(sp_[0], sp_[1]))
    if even > 1e-12:
        fails.append(("B-even", even))
    res["B_I_worst"] = worst; res["B_even_max_rel"] = even; res["B_I_failures"] = [list(map(str, f)) for f in fails[:50]]
    res["B_I_pass"] = not fails
    print("B-I worst:", worst, "even", even, "failures", len(fails), flush=True)
    if fails:
        out_path.write_text(json.dumps(res, indent=1, default=str) + "\n"); raise SystemExit("stop: B-I gate")
    # B-II-code: the 27 rows of exact_edge.json
    bl = EE.Blocks()
    rows = json.loads((R_ / "exact_edge.json").read_text())["edges"]
    code = []
    for r in rows:
        e = upper_edge(bl, r["y"], r["t"])
        comp = list(bl.phi_A(e + 1e-4, r["y"], r["t"], True)[1]) if e is not None else None
        ok = (e is not None and r["edge_A"] is not None and abs(e - r["edge_A"]) <= 1e-9
              and comp == list(r["competitor_block_A"])) or (e is None and r["edge_A"] is None)
        code.append({"t": r["t"], "height": r["height"], "edge": e, "edge_A": r["edge_A"], "comp": comp,
                     "comp_ref": r["competitor_block_A"], "ok": bool(ok)})
    res["B_II_code_rows"] = code; res["B_II_code"] = all(c["ok"] for c in code)
    print("B-II-code:", res["B_II_code"], [c for c in code if not c["ok"]], flush=True)
    if not res["B_II_code"]:
        out_path.write_text(json.dumps(res, indent=1, default=str) + "\n"); raise SystemExit("stop: B-II-code")
    pts = []
    for n, (k, x, y, z, t, Rg) in enumerate(draws):
        o = b2_point(bl, float(np.hypot(y, z)), t); o["draw"] = n; pts.append(o)
        print(n, round(o["rho"], 4), round(o["t"], 4), "x*", o["x_star"], "comp", o.get("competitor"),
              "rho_s", o.get("rho_s"), "half", o.get("half_changes"), flush=True)
        Path(str(out_path) + ".partial").write_text(json.dumps(pts, default=str))
    res["points"] = pts
    sc = [o for o in pts if o["x_star"] is not None]
    res["B_count"] = len(sc); res["B_count_met"] = len(sc) >= 60
    res["B_N_fraction_neutral"] = sum(1 for o in sc if o["competitor"][1] == 0) / max(1, len(sc))
    res["rho_s_quantiles"] = list(np.quantile([o["rho_s"] for o in sc], [0, 0.25, 0.5, 0.75, 1])) if sc else None
    res["B_S_failures"] = [(o["draw"], o["rho"], o["t"], o["rho_s"], o["competitor"]) for o in sc if not o["rho_s"] < 0.5]
    res["B_half_failures"] = [(o["draw"], o["rho"], o["t"], o["half_changes"]) for o in sc if not o["B_half"]]
    res["q_comp_rho_below_1.1"] = [(o["rho"], o["t"], o["competitor"]) for o in sc if o["rho"] < 1.1]
    if res["B_count_met"]:
        res["B_S"] = "pass" if not res["B_S_failures"] else "fail"
        res["B_half"] = "pass" if not res["B_half_failures"] else "fail"
    else:
        res["B_S"] = res["B_half"] = "inconclusive"
    out_path.write_text(json.dumps(res, indent=1, default=str) + "\n")
    print(json.dumps({k: v for k, v in res.items() if k not in ("points", "B_II_code_rows")}, indent=1, default=str))


if __name__ == "__main__":
    main()
