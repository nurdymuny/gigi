"""S_C stage R2 (amendment 1): interval Lehmann steps with the exact Robin eigenfunction trial states.

Per control, along a given step list [(s, N), ...]:
1. the floating VSCF trial states (vscf_helike_sc_lehmann, folded base, R1 enclosed v_J and face-wise beta) give
   coefficient tensors C (N x n_u x n_v) over the base block eigenfunctions; any coefficients are admissible;
2. the EXACT base eigenfunctions replace the Galerkin ones (same index, sign fixed): R1.4 eigenvalue balls,
   Kummer-series radial moments and exact angular ladders (helike_sc_r2_moments);
3. D = P - W0 from the R1 Arb polynomial P (not the floating fit), D^2 exactly;
4. A0 = C^T C, A1 = C^T E C + s C^T D C, A2 = C^T E^2 C + s C^T (E D + D E) C + s^2 C^T D^2 C, all in
   midpoint-radius arithmetic (exact restriction: every trial state lies in the representation span);
5. the pencil (A1 - rho A0, A2 - 2 rho A1 + rho^2 A0) in Arb: congruence by the floating eigenvectors, Weyl bounds,
   then l_{N+1-i} >= rho + 1/tau_i^upper for tau_i^upper < 0.
The first rho is the enclosed base level lambda_{N+1}(H_0) (lower end); each later rho is the previous step's
interval l_{N'+1}. Report: interval l_5 against rho_C, and l_11 against rho_b.
usage: python scripts/helike_sc_r2_lehmann.py --g2 0.002 --fold 0.36,0.17 --vj X --beta-u X --beta-v X --steps 0.385:23,0.98625:13
"""
import argparse
import json
import math
import sys
import time
from pathlib import Path
from collections import defaultdict

import numpy as np
from flint import arb, arb_mat, ctx

import helike_sc_r1 as r1
from helike_sc_r1 import Poly
from helike_sc_r1_spectrum import RadialBlock
import helike_sc_r2_moments as rm

EPS = 2.0 ** -53


# ------------------------------------------------------------------ midpoint-radius helpers
def mr_mul(Am, Ar, Bm, Br):
    """(Am +- Ar)(Bm +- Br) for matrices (last two axes), with an a priori rounding bound."""
    n = Am.shape[-1]
    Cm = Am @ Bm
    aAm, aBm = np.abs(Am), np.abs(Bm)
    Cr = aAm @ Br + Ar @ (aBm + Br) + (2 * n + 4) * EPS * (aAm @ aBm)
    return Cm, Cr * (1 + 4 * (n + 2) * EPS) + 1e-300


def to_mr(x):
    """arb -> (mid, rad) floats enclosing it."""
    m = float(x.mid())
    return m, float(x.rad()) + abs(m) * 2 * EPS + 1e-300


# ------------------------------------------------------------------ exact blocks matched to the floating ones
def exact_block_mats(fblock, idx, w2, L, beta, c, dmax=8, log=print):
    """(mid, rad) matrices <phi_i, x^alpha phi_j> for the floating block's functions idx, built from the exact
    eigenfunctions (same l, same order within l, sign of the floating radial at small r), and the E enclosures."""
    used = sorted({fblock.funcs[i][0] for i in idx})
    lmax = max(fblock.rad[r][0] for r in used)
    t0 = time.time()
    ecut = max(fblock.rad[r][1] for r in used) + 0.5
    RB = RadialBlock(arb(w2), arb(L), arb(beta), arb(c))
    # the enclosures take hours: cache them on disk (arb balls round-trip exactly through their string form)
    import hashlib, json as _json, os
    key = hashlib.sha256(repr((w2, L, beta, c, ecut, ctx.prec)).encode()).hexdigest()[:16]
    cache = Path(os.environ.get("R2_CACHE", "/tmp")) / f"r2_radial_spectrum_{key}.json"
    if cache.exists():
        spec = [(l, arb(e)) for l, e in _json.loads(cache.read_text())]
        log(f"    radial enclosures from cache {cache.name}")
    else:
        spec, _ = RB.spectrum(ecut)
        cache.write_text(_json.dumps([(l, E.str(radius=True, more=True)) for l, E in spec]))
    enc = defaultdict(list)
    for l, E in sorted(spec, key=lambda t: (t[0], float(t[1].mid()))):
        enc[l].append(E)
    order = defaultdict(list)
    for r, (l, E, _) in enumerate(fblock.rad):
        order[l].append(r)
    levels, rpos, sign = [], {}, {}
    for r in used:
        l, E, vals = fblock.rad[r]
        k = order[l].index(r)
        Ee = enc[l][k]
        # identity matching, not accuracy: the floating radial solver is only ~1e-6 accurate at higher l, so the test is
        # that the enclosed level matched by (l, order) is the nearest enclosed level of that l to the floating one, by a
        # margin of at least 100x (the exact functions then carry the identity; their energies are the enclosures)
        d = [abs(float(x.mid()) - E) for x in enc[l]]
        near = int(np.argmin(d)); second = sorted(d)[1] if len(d) > 1 else float("inf")
        if near != k or d[k] > 1e-4 or second < 100 * d[k]:
            raise ValueError(f"level matching failed l={l}, order {k}: distances {d[:5]}")
        rpos[r] = len(levels); levels.append((l, Ee))
        nz = np.argmax(np.abs(vals) > 1e-8 * np.abs(vals).max()); sign[r] = float(np.sign(vals[nz]))
    XB = rm.ExactBlock(arb(w2), L, arb(beta), arb(c), levels)
    log(f"    exact block: {len(levels)} radial functions (lmax {lmax}), enclosures in {time.time() - t0:.0f}s")
    t0 = time.time()
    nr = len(levels)
    radm = np.zeros((dmax + 1, nr, nr)); radr = np.zeros_like(radm)
    for k in range(dmax + 1):
        for a in range(nr):
            for b in range(a, nr):
                if (levels[a][0] + levels[b][0] + k) % 2:
                    continue
                m, rr = to_mr(XB.radial(a, b, k))
                radm[k, a, b] = radm[k, b, a] = m; radr[k, a, b] = radr[k, b, a] = rr
    log(f"    radial moments in {time.time() - t0:.0f}s")
    t0 = time.time()
    ang, nang = rm.angular_matrices(lmax, dmax)
    log(f"    angular matrices in {time.time() - t0:.0f}s")
    ri = np.array([rpos[fblock.funcs[i][0]] for i in idx])
    sg = np.array([sign[fblock.funcs[i][0]] for i in idx])
    ai = np.array([fblock.funcs[i][1] ** 2 + fblock.funcs[i][1] + fblock.funcs[i][2] for i in idx])
    E = [levels[ri[p]][1] for p in range(len(idx))]
    mats = {}
    for a, Mang in ang.items():
        am = np.empty((len(ai), len(ai))); ar = np.empty_like(am)
        for p, x in enumerate(ai):
            for q, y in enumerate(ai):
                v = Mang[int(x), int(y)].real
                am[p, q] = float(v.mid()); ar[p, q] = float(v.rad()) + abs(am[p, q]) * 2 * EPS
        k = sum(a)
        Rm = radm[k][np.ix_(ri, ri)] * np.outer(sg, sg); Rr = radr[k][np.ix_(ri, ri)]
        mats[a] = (Rm * am, np.abs(Rm) * ar + Rr * (np.abs(am) + ar) + 4 * EPS * np.abs(Rm * am))
    return mats, E


# ------------------------------------------------------------------ exact D and D^2
def exact_D(M, w2s, w2t):
    """D = P - W0 as a dict exponent -> arb, W0 = 1/2 w2s |u|^2 + 1/2 w2t |v|^2 (the exact base used)."""
    t = dict(M.P.t)
    for k in range(6):
        e = tuple(2 if i == k else 0 for i in range(6))
        t[e] = t.get(e, arb(0)) - (arb(w2s) if k < 3 else arb(w2t)) / 2
    return Poly(6, t)


def groups_mr(D, Vmats):
    """Group D by u-exponent: list of (alpha_u, (Vt_mid, Vt_rad)) with Vt = sum_beta d_{alpha beta} V^beta."""
    g = defaultdict(dict)
    for e, c in D.t.items():
        g[e[:3]][e[3:]] = c
    out = []
    for a, inner in g.items():
        Vm = 0; Vr = 0
        for b, c in inner.items():
            cm, cr = to_mr(c)
            Bm, Br = Vmats[b]
            Vm = Vm + cm * Bm; Vr = Vr + abs(cm) * Br + cr * (np.abs(Bm) + Br)
        out.append((a, (Vm, Vr * (1 + 8 * EPS) + 4 * EPS * np.abs(Vm))))
    return out


def apply_mr(groups, Umats, C):
    """(mid, rad) of the coefficient tensors of D psi_n restricted to the span: sum_a U_a C_n Vt_a^T."""
    Xm = np.zeros_like(C); Xr = np.zeros_like(C)
    Cr0 = np.zeros_like(C)
    for a, (Vm, Vr) in groups:
        Um, Ur = Umats[a]
        Ym, Yr = mr_mul(Um[None], Ur[None], C, Cr0)                       # U C
        Zm, Zr = mr_mul(Ym, Yr, Vm.T[None], Vr.T[None])                   # (U C) Vt^T
        Xm += Zm; Xr += Zr
    return Xm, Xr * (1 + 4 * len(groups) * EPS)


def gram_mr(Am, Ar, Bm, Br):
    """(mid, rad) of the N x N matrix <A_m, B_n> (sum over the two block indices)."""
    N = Am.shape[0]
    a = Am.reshape(N, -1); b = Bm.reshape(N, -1); ar = Ar.reshape(N, -1); br = Br.reshape(N, -1)
    m = a @ b.T
    r = np.abs(a) @ br.T + ar @ (np.abs(b) + br).T + (2 * a.shape[1] + 4) * EPS * (np.abs(a) @ np.abs(b).T)
    return m, r * (1 + 4 * a.shape[1] * EPS) + 1e-300


# ------------------------------------------------------------------ the verified pencil
def arbmat(m, r):
    n = m.shape[0]
    return arb_mat([[arb(float(m[i, j]), float(r[i, j])) for j in range(n)] for i in range(n)])


def verified_lehmann(A0, A1, A2, rho):
    """A* are (mid, rad) N x N. Returns lower bounds l_k (index k-1), -inf where none."""
    N = A0[0].shape[0]
    a0, a1, a2 = arbmat(*A0), arbmat(*A1), arbmat(*A2)
    rho = arb(rho)
    K = a1 - a0 * rho
    B = a2 - a1 * (2 * rho) + a0 * (rho * rho)
    Km = np.array([[float(K[i, j].mid()) for j in range(N)] for i in range(N)])
    Bm = np.array([[float(B[i, j].mid()) for j in range(N)] for i in range(N)])
    from scipy.linalg import eigh
    lam, X = eigh((Km + Km.T) / 2, (Bm + Bm.T) / 2)
    Xa = arb_mat([[arb(float(X[i, j])) for j in range(N)] for i in range(N)])
    XtB = Xa.transpose() * B * Xa
    XtK = Xa.transpose() * K * Xa
    E1 = arb(0); E2 = arb(0)
    for i in range(N):
        for j in range(N):
            d1 = XtB[i, j] - (1 if i == j else 0)
            d2 = XtK[i, j] - (arb(float(lam[i])) if i == j else 0)
            E1 += d1 * d1; E2 += d2 * d2
    eta = E1.sqrt(); e2 = E2.sqrt()
    if float(eta.upper()) >= 0.5:
        raise ArithmeticError("congruence too far from B-orthonormal")
    # B positive definite: X^T B X = I + E1 with ||E1|| < 1, and X nonsingular (its float smallest singular value)
    if np.linalg.svd(X, compute_uv=False).min() < 1e-8:
        raise ArithmeticError("eigenvector matrix numerically singular")
    lmax = arb(float(np.abs(lam).max()))
    delta = 1 / (1 - eta).sqrt() - 1
    shift = lmax * (2 * delta + delta * delta) + e2 / (1 - eta)
    lo = [-math.inf] * N
    taus = []
    for i, t in enumerate(sorted(lam), start=1):
        tu = arb(float(t)) + shift                                 # upper end of tau_i (Weyl)
        taus.append((float(t), float(shift.upper())))
        if float(tu.upper()) < 0:
            lo[N - i] = float((rho + 1 / arb(tu.upper())).lower())
    return lo, dict(eta=float(eta.upper()), e2=float(e2.upper()), weyl_shift=float(shift.upper()))


# ------------------------------------------------------------------ driver
def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--g2", type=float, required=True)
    ap.add_argument("--fold", required=True)
    for k in ("vj", "beta-u", "beta-v"):
        ap.add_argument(f"--{k}", required=True)
    ap.add_argument("--steps", required=True)
    ap.add_argument("--rho-C", type=float, default=None)
    ap.add_argument("--out", default=None)
    ap.add_argument("--soft-cut", type=float, default=None, help="keep soft base functions with E_u below this")
    ap.add_argument("--stiff-cut", type=float, default=None, help="keep stiff base functions with E_v below this")
    ap.add_argument("--probe-cuts", action="store_true", help="only report the trial norm lost at candidate cuts")
    a = ap.parse_args()
    log = lambda s: print(s, flush=True)
    import dryrun_helike_sc_lehmann_homotopy as dr
    import estimate_helike_sc_gbase as gb
    import plan_helike_sc_lehmann_path as pl
    import vscf_helike_sc_lehmann as vs
    fa, fb = (float(x) for x in a.fold.split(","))
    M = r1.Model(a.g2)
    w2s = float(arb((min(M.w2[:3], key=lambda x: float(x.mid())) - arb(a.fold.split(",")[0])).lower()))
    w2t = float(arb((min(M.w2[3:], key=lambda x: float(x.mid())) - arb(a.fold.split(",")[1])).lower()))
    # floating VSCF with the SAME base frequencies as the exact blocks
    vs.FOLD = (fa, fb)                                  # the VSCF operator uses the same folded base
    gb.curvature = lambda m, L, rng, n=0: (float(a.vj), float(a.vj), float(a.beta_u))
    _Block = dr.Block

    class FaceBlock(_Block):
        def __init__(self, w2r, L, beta, shift, ecut, K=dr.K_RAD):
            w2r = w2s if L == dr.LU else w2t
            super().__init__(w2r, L, float(a.beta_u) if L == dr.LU else float(a.beta_v), shift, ecut, K)
    dr.Block = FaceBlock
    E0 = dict(gb.CONTROLS)[a.g2]
    t0 = time.time(); V = vs.VSCF(a.g2, E0, log); log(f"  VSCF built in {time.time() - t0:.0f}s")
    rho_b = E0 + gb.RHO_B_REL
    rho_C = a.rho_C if a.rho_C is not None else E0 + 1.75 * math.sqrt(float(min(M.w2, key=lambda x: float(x.mid())).mid()))
    steps = [(float(x.split(":")[0]), int(x.split(":")[1])) for x in a.steps.split(",")]
    if a.probe_cuts:
        for s_, N_ in steps:
            C = V.trial(s_, N_)
            for cu in (8, 9, 10, 11, 12, 13, 14.7):
                for cv in (8, 10, 12, 20.1):
                    Pu = V.Eu < cu; Pv = V.Ev < cv
                    kept = np.einsum("nij,nij->n", C[:, Pu][:, :, Pv], C[:, Pu][:, :, Pv])
                    log(f"  probe s={s_} N={N_} soft<{cu} ({Pu.sum()}) stiff<{cv} ({Pv.sum()}): max lost norm^2 {1 - kept.min():.2e}")
        return
    Pu = np.arange(len(V.ui)) if a.soft_cut is None else np.flatnonzero(V.Eu < a.soft_cut)
    Pv = np.arange(len(V.vi)) if a.stiff_cut is None else np.flatnonzero(V.Ev < a.stiff_cut)
    log(f"  restricted span: soft {len(Pu)}/{len(V.ui)}, stiff {len(Pv)}/{len(V.vi)} (cuts {a.soft_cut}, {a.stiff_cut})")
    log(f"  exact blocks (soft w2 {w2s:.9f}, stiff w2 {w2t:.9f}):")
    Umats, Eu = exact_block_mats(V.ublock, [V.ui[p] for p in Pu], w2s, dr.LU, a.beta_u, a.vj, log=log)
    Vmats, Ev = exact_block_mats(V.vblock, [V.vi[p] for p in Pv], w2t, dr.LV, a.beta_v, 0, log=log)
    for name, mats, fl, P in (("soft", Umats, V.U, Pu), ("stiff", Vmats, V.V, Pv)):
        worst = max(np.abs(mats[k][0] - fl[k][np.ix_(P, P)]).max() for k in fl if k in mats)
        log(f"    check {name}: max |exact - floating| over the monomial matrices = {worst:.2e}")
    D = exact_D(M, w2s, w2t); D2 = D * D
    gD, gD2 = groups_mr(D, Vmats), groups_mr(D2, Vmats)
    Esum = [[eu + ev for ev in Ev] for eu in Eu]
    Em = np.array([[float(x.mid()) for x in row] for row in Esum]); Er = np.array([[float(x.rad()) + abs(float(x.mid())) * 2 * EPS for x in row] for row in Esum])
    # base product levels (enclosed) for the first rho
    base = sorted((x for row in Esum for x in row), key=lambda x: float(x.mid()))
    record = dict(g2=a.g2, fold=a.fold, rho_C=rho_C, rho_b=rho_b, steps=[])
    rho = None
    for k, spec in enumerate(a.steps.split(",")):
        s, N = spec.split(":"); s, N = float(s), int(N)
        if rho is None:
            rho = float(base[N].lower())                                  # lambda_{N+1}(H_0), lower end
        t0 = time.time()
        Cfull = V.trial(s, N)
        C = Cfull[:, Pu][:, :, Pv]
        kept = np.sqrt(np.einsum("nij,nij->n", C, C))
        C = C / kept[:, None, None]                                   # any trial functions are admissible
        log(f"    trial norm lost to the restriction: max {1 - kept.min() ** 2:.2e}")
        Z = np.zeros_like(C)
        DCm, DCr = apply_mr(gD, Umats, C)
        D2Cm, D2Cr = apply_mr(gD2, Umats, C)
        ECm, ECr = Em[None] * C, Er[None] * np.abs(C)
        A0 = gram_mr(C, Z, C, Z)
        CEC = gram_mr(C, Z, ECm, ECr); CDC = gram_mr(C, Z, DCm, DCr)
        A1 = (CEC[0] + s * CDC[0], CEC[1] + s * CDC[1] + 4 * EPS * np.abs(CEC[0] + s * CDC[0]))
        EE = gram_mr(ECm, ECr, ECm, ECr); ED = gram_mr(ECm, ECr, DCm, DCr); CD2C = gram_mr(C, Z, D2Cm, D2Cr)
        A2m = EE[0] + s * (ED[0] + ED[0].T) + s * s * CD2C[0]
        A2r = EE[1] + s * (ED[1] + ED[1].T) + s * s * CD2C[1] + 8 * EPS * np.abs(A2m)
        lo, diag = verified_lehmann(A0, A1, (A2m, A2r), rho)
        rec = dict(step=k + 1, s=s, N=N, rho=rho, l5=lo[4] if N >= 5 else None, l11=lo[10] if N >= 11 else None,
                   lower=lo, max_radius=dict(A0=float(A0[1].max()), A1=float(A1[1].max()), A2=float(A2r.max())), **diag,
                   seconds=time.time() - t0)
        record["steps"].append(rec)
        log(f"  step {k + 1}: s = {s}, N = {N}, rho = {rho:.6f}: interval l_5 = {rec['l5']}, l_11 = {rec['l11']};"
            f" radii A0/A1/A2 {rec['max_radius']['A0']:.1e}/{rec['max_radius']['A1']:.1e}/{rec['max_radius']['A2']:.1e};"
            f" congruence eta {diag['eta']:.1e}, Weyl shift {diag['weyl_shift']:.1e}; {rec['seconds']:.0f}s")
        nxt = a.steps.split(",")[k + 1] if k + 1 < len(a.steps.split(",")) else None
        if nxt:
            N2 = int(nxt.split(":")[1])
            rho = lo[N2] if N2 < N else -math.inf                        # interval l_{N2+1} of this step
            if not math.isfinite(rho):
                log("  chain broken: no interval bound for the next step's rho"); break
    last = record["steps"][-1]
    ok = last.get("l5") is not None and last["l5"] >= rho_C
    record["verdict_floating_rhoC"] = "l5 >= rho_C" if ok else "not shown"
    log(f"  RESULT: last interval l_5 = {last.get('l5')} vs rho_C {rho_C:.6f} (recomputed, not frozen): "
        f"{record['verdict_floating_rhoC']}")
    if a.out:
        with open(a.out, "w") as f:
            json.dump(record, f, indent=1)


if __name__ == "__main__":
    main()
