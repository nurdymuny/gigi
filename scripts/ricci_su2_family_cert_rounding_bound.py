"""Rounding of the non-directed final operations of scripts/ricci_su2_family_block_cert.py (review finding S6-5,
22 November 2026). Does NOT rerun the certificate.

Part 1 (rigorous, interval arithmetic): an upper bound M* on mu_t = eta - eps over every certified box, where
  mu_t <= lambda_min(B~(c)) <= min_k B~_kk(c),  B~ = B_{j,q} - lambda_j I,
because the directed part of certify_point proves lambda_min(B~(c)) >= eta - eps exactly. So
  M* = max_{j < 80, q non-twist} min_k sup_{(x, y, s) in D} B~_kk,
each sup an upper endpoint of an outward interval evaluation of the shifted diagonal (the certificate's own dshift
expressions) over the whole domain D = [-3, -1.5] x [0.5, 3] x [sqrt 3, sqrt 10].
Part 2: the error constant kappa and the comparison with the minimum certified margin of the receipt.
Part 3 (demonstration, not a re-check of the receipt): on a random sample of sub-boxes, certify_mv as written versus a
directed version in which hw, var and mu - var are computed in interval arithmetic and the lower endpoint is tested.
usage: python scripts/ricci_su2_family_cert_rounding_bound.py OUT.json [NSAMPLE]
"""
import json
import sys
from pathlib import Path

import mpmath
import numpy as np
import sympy as sp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_family_block_cert as FC      # noqa: E402

iv = mpmath.iv
ROOT = Path(__file__).resolve().parent.parent


def states(j, q):
    S = FC.symbolic(); l = sp.Rational(j, 2)
    return [(q - a - b, k) for k, (a, b) in enumerate(S["ch"]) if abs(q - a - b) <= l]


def diag_sup_min(j, q, X, Y, Sv):
    S = FC.symbolic(); l = sp.Rational(j, 2); best = None
    for (mm, a) in states(j, q):
        e = FC.ivv(S["dshift"][a](X, Y, Sv, iv.mpf(float(mm)), iv.mpf(float(l)), iv.mpf(j)))
        ub = mpmath.mpf(e.b)
        best = ub if best is None else min(best, ub)
    return best


def certify_mv_directed(j, q, box):
    """certify_mv with hw, var and mu - var in interval arithmetic (lower endpoint tested)."""
    (x0, x1), (y0, y1), (t0, t1) = box
    s0, s1 = float(np.sqrt(t0)), float(np.sqrt(t1))
    xc, yc, sc = 0.5 * (x0 + x1), 0.5 * (y0 + y1), 0.5 * (s0 + s1)
    mpmath.mp.prec = 53; iv.prec = 53
    _, Bf = FC.block_iv(j, q, iv.mpf(xc), iv.mpf(yc), iv.mpf(sc), shifted=True)
    Bm = np.array([[float(mpmath.mpf(e.mid)) for e in row] for row in Bf])
    eta = 0.5 * float(np.linalg.eigvalsh(0.5 * (Bm + Bm.T))[0])
    if eta <= 0:
        return None
    # reproduce certify_point's eps (directed part unchanged), then combine in iv
    mpmath.mp.prec = FC.PREC; iv.prec = FC.PREC
    mu_nearest = FC.certify_point(j, q, xc, yc, sc)
    O = FC.O
    _, B = FC.block_iv(j, q, iv.mpf(xc), iv.mpf(yc), iv.mpf(sc), shifted=True)
    n = len(B); mid = [[None] * n for _ in range(n)]; bud = O.Budget()
    for r in range(n):
        for c_ in range(r + 1):
            v = B[r][c_] - (iv.mpf(eta) if r == c_ else 0)
            m_, rr = O.mid_rad(v); mid[r][c_] = mid[c_][r] = m_; bud.add(r, c_, rr)
    rho = bud.rho()
    L = [[mpmath.mpf(0)] * n for _ in range(n)]; d = []
    for k in range(n):
        dk = mid[k][k] - mpmath.fsum(L[k][s_] ** 2 * d[s_] for s_ in range(k))
        if dk <= 0:
            return None
        d.append(dk)
        for i2 in range(k + 1, n):
            L[i2][k] = (mid[i2][k] - mpmath.fsum(L[i2][s_] * L[k][s_] * d[s_] for s_ in range(k))) / dk
        L[k][k] = mpmath.mpf(1)
    Lrows = [{s_: L[i2][s_] for s_ in range(i2 + 1) if L[i2][s_] != 0} for i2 in range(n)]
    eps_up, _ = O.eps_residual(rho, O.residual_frob(n, lambda i2, k2: mid[i2][k2], Lrows, d))
    mu_iv = iv.mpf(eta) - iv.mpf(eps_up)
    X, Y = iv.mpf([mpmath.mpf(x0), mpmath.mpf(x1)]), iv.mpf([mpmath.mpf(y0), mpmath.mpf(y1)])
    Sv = iv.sqrt(iv.mpf([mpmath.mpf(t0), mpmath.mpf(t1)]))
    sl, sh = iv.mpf(Sv.a), iv.mpf(Sv.b)
    hx = (iv.mpf(x1) - iv.mpf(x0)) / 2; hy = (iv.mpf(y1) - iv.mpf(y0)) / 2
    d1, d2 = sh - iv.mpf(sc), iv.mpf(sc) - sl
    hs = iv.mpf(max(mpmath.mpf(d1.b), mpmath.mpf(d2.b)))
    var = sum((iv.mpf(FC.dblock_upper(j, q, X, Y, Sv, i)) * h for i, h in enumerate((hx, hy, hs))), iv.mpf(0))
    low_iv = mu_iv - var
    ok_n, low_n = FC.certify_mv(j, q, box)
    # the same combination exactly as certify_mv writes it (round-to-nearest, 64 bits), kept as an mpf
    mpmath.mp.prec = FC.PREC; iv.prec = FC.PREC
    hw = [mpmath.mpf(x1 - x0) / 2, mpmath.mpf(y1 - y0) / 2, max(mpmath.mpf(Sv.b) - mpmath.mpf(sc), mpmath.mpf(sc) - mpmath.mpf(Sv.a))]
    var_n = mpmath.fsum(FC.dblock_upper(j, q, X, Y, Sv, i) * hw[i] for i in range(3))
    low_n_mpf = mu_nearest - var_n
    dl = mpmath.mpf(low_iv.a)
    return {"j": j, "q": q, "box": box, "nearest_lower_float_returned": low_n, "nearest_ok": ok_n,
            "nearest_lower_mpf": mpmath.nstr(low_n_mpf, 25), "directed_lower_endpoint": mpmath.nstr(dl, 25),
            "directed_ok": bool(dl > 0), "diff": float(dl - low_n_mpf),
            "rel_diff": float(abs(dl - low_n_mpf) / abs(low_n_mpf)),
            "mu_nearest": float(mu_nearest), "diff_over_u_mu": float(abs(dl - low_n_mpf) / (mpmath.mpf(2) ** -FC.PREC * mu_nearest)),
            "mu_nearest_minus_mu_directed_lower": float(mu_nearest - mpmath.mpf(mu_iv.a))}


def main():
    out = Path(sys.argv[1]); nsample = int(sys.argv[2]) if len(sys.argv) > 2 else 60
    rec_path = ROOT / "research/su2-floor-family-20261119/family_block_cert_j80.json"
    R = json.loads(rec_path.read_text())
    D = R["domain"]
    mpmath.mp.prec = FC.PREC; iv.prec = FC.PREC
    X, Y = iv.mpf(D[0]), iv.mpf(D[1]); Sv = iv.sqrt(iv.mpf(D[2]))
    Mstar, arg = mpmath.mpf("-inf"), None
    per_j = []
    for j in range(R["jmin"], R["jmax"]):
        l = j / 2
        qs = [float(q) for q in np.arange(-l - 2, l + 2 + 1e-9, 1.0) if abs(abs(q) - (l + 2)) > 1e-9]
        mj = max(diag_sup_min(j, q, X, Y, Sv) for q in qs)
        per_j.append(float(mj))
        if mj > Mstar:
            Mstar, arg = mj, j
    u = mpmath.mpf(2) ** -FC.PREC
    g4 = 4 * u / (1 - 4 * u)
    kappa = u + g4 * (1 + u) / (1 - g4)
    mmin = min(r["min_margin"] for r in R["rows"])
    jmin_row = min(R["rows"], key=lambda r: r["min_margin"])["j"]
    lower_hat_min = mpmath.mpf(mmin) * (1 - mpmath.mpf(2) ** -53)          # float(lower) -> lower: half-ulp of double
    err_bound = kappa * Mstar * (1 + u)
    rec = {"receipt": str(rec_path.relative_to(ROOT)), "domain": D, "receipt_all_certified": R["all_certified"],
           "total_certified_boxes": sum(r["certified_boxes"] for r in R["rows"]),
           "u": mpmath.nstr(u, 6), "gamma4": mpmath.nstr(g4, 6), "kappa": mpmath.nstr(kappa, 8),
           "Mstar_upper_bound_on_mu": mpmath.nstr(Mstar, 10), "Mstar_attained_at_j": arg,
           "Mstar_per_j": per_j, "min_certified_margin": mmin, "min_margin_j": jmin_row,
           "rounding_error_bound_kappa_Mstar_(1+u)": mpmath.nstr(err_bound, 6),
           "ratio_margin_over_bound": mpmath.nstr(lower_hat_min / err_bound, 6),
           "conclusion_rigorous": bool(lower_hat_min > err_bound)}
    print({k: v for k, v in rec.items() if k != "Mstar_per_j"}, flush=True)
    # Part 3: sample
    rng = np.random.default_rng(20261122); samples = []
    widths = [D[i][1] - D[i][0] for i in range(3)]
    while len(samples) < nsample:
        j = int(rng.integers(0, 80)); l = j / 2
        qs = [float(q) for q in np.arange(-l - 2, l + 2 + 1e-9, 1.0) if abs(abs(q) - (l + 2)) > 1e-9]
        q = qs[int(rng.integers(0, len(qs)))]
        depth = int(rng.integers(6, 15))
        box = []
        for i in range(3):
            k = depth // 3 + (1 if i < depth % 3 else 0); nseg = 2 ** k
            a = int(rng.integers(0, nseg)); lo = D[i][0] + widths[i] * a / nseg; hi = D[i][0] + widths[i] * (a + 1) / nseg
            box.append((lo, hi))
        r = certify_mv_directed(j, q, box)
        if r is not None:
            samples.append(r)
    rec["sample_directed_vs_nearest"] = {
        "n": len(samples), "max_abs_diff": max(abs(r["diff"]) for r in samples),
        "max_rel_diff": max(r["rel_diff"] for r in samples),
        "max_diff_over_u_mu": max(r["diff_over_u_mu"] for r in samples), "directed_never_above_nearest": all(r["diff"] <= 0 for r in samples),
        "verdicts_agree": all(r["nearest_ok"] == r["directed_ok"] for r in samples),
        "n_certified": sum(r["directed_ok"] for r in samples), "rows": samples,
        "status": "demonstration on random sub-boxes; not the receipt's boxes"}
    out.write_text(json.dumps(rec, indent=1, default=str) + "\n")
    s = rec["sample_directed_vs_nearest"]
    print("sample:", s["n"], "max |diff|", s["max_abs_diff"], "max rel", s["max_rel_diff"],
          "max |diff|/(u mu)", s["max_diff_over_u_mu"], "agree", s["verdicts_agree"], "certified", s["n_certified"])


if __name__ == "__main__":
    import _receipt_status                     # exit 1 if the receipt is missing, stale or records a failed check
    _receipt_status.begin()
    main()
    _receipt_status.exit_on_failure(sys.argv[1])
