"""Prediction G of the Test 2 pre-registration (family 2 add-on): the field-strength share
s_F = <beta, -2[F, .] beta> and the Ricci share <beta, Ric beta> of the extremal frame state beta = E_theta (x) E_theta,
for A = theta/2 on the SU(3) families 2a and 2b. Computed from the background alone.

Validation first, on SU(2) with the general data of scripts/ricci_su3_test2_family.py: the Berger field must give
Ricci 4 - 2t and field strength 8 - 6t (scripts/ricci_su2_gyromagnetic_split.py), and the beta built here must be the
top-charge state of the frame (x) gauge charge.
Ricci of a left-invariant metric on a unimodular group, orthonormal frame:
  Ric(X, X) = -1/2 sum_i |[X, e_i]|^2 - 1/2 B(X, X) + 1/4 sum_ij <[e_i, e_j], X>^2,  B the Killing form.
usage: python scripts/ricci_su3_gyromagnetic_share.py OUT.json [FAMILY_RESULTS_2a.json FAMILY_RESULTS_2b.json]
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su3_test2_family as T          # noqa: E402


def ortho_c(C, g):
    s = np.sqrt(np.asarray(g, float))
    return C * s[None, None, :] / (s[:, None, None] * s[None, :, None])


def ricci(C, g):
    c = ortho_c(C, g); n = len(g)
    ad = [c[a].T for a in range(n)]                       # ad(e_a)[k, b] = c[a, b, k]
    B = np.array([[np.trace(ad[a] @ ad[b]) for b in range(n)] for a in range(n)])
    R = np.zeros((n, n))
    for a in range(n):
        for b in range(n):
            t1 = -0.5 * sum(ad[a][:, i] @ ad[b][:, i] for i in range(n))
            t3 = 0.25 * sum(c[i, j, a] * c[i, j, b] for i in range(n) for j in range(n))
            R[a, b] = t1 - 0.5 * B[a, b] + t3
    return R


def field_strength_op(C, g, f, M):
    """T[(i, alpha), (j, beta)] = ad(F_ij)[alpha, beta], so ([F, .] b)_i = sum_j [F_ij, b_j]."""
    c = ortho_c(C, g); n = len(g); r = f.shape[0]
    Top = np.zeros((n * r, n * r))
    for i in range(n):
        for j in range(n):
            Fij = T.adm(M[i], f) @ M[j] - sum(c[i, j, k] * M[k] for k in range(n))
            Top[i * r:(i + 1) * r, j * r:(j + 1) * r] = T.adm(Fij, f)
    return Top


def shares(C, g, f, M, beta):
    n, r = len(g), f.shape[0]
    Ric = np.kron(ricci(C, g), np.eye(r))
    Fop = field_strength_op(C, g, f, M)
    return float((beta.conj() @ Ric @ beta).real), float((beta.conj() @ (-2 * Fop) @ beta).real)


def top_frame_state(Af, Ag, n, r):
    Q = np.kron(Af, np.eye(r)) + np.kron(np.eye(n), Ag)
    q, U = np.linalg.eigh(0.5 * (Q + Q.conj().T))
    return q, U[:, -1]


def main():
    out = {}
    # --- validation on SU(2): Berger field
    import ricci_su2_soft_mode_flow as F
    S, C2, eps = T.su2_data()
    val = []
    for t in (1.0, 2.0, 3.0, 4.0, 6.0):
        m = [np.asarray(F.lc_background(t))[i] for i in range(3)]
        Af = 1j * C2[0].T; Ag = 1j * 2 * T.adm(np.eye(3)[0], eps)
        q, beta = top_frame_state(Af, Ag, 3, 3)
        ric, sf = shares(C2, [t, 1, 1], eps, m, beta)
        val.append({"t": t, "top_charge": float(q[-1]), "ricci": ric, "field": sf,
                    "ricci_expected": 4 - 2 * t, "field_expected": 8 - 6 * t})
    out["su2_validation"] = val
    out["su2_validation_pass"] = all(abs(v["ricci"] - v["ricci_expected"]) < 1e-10 and abs(v["field"] - v["field_expected"]) < 1e-10
                                     for v in val)
    print("SU(2) validation:", out["su2_validation_pass"], [(v["t"], round(v["ricci"], 6), round(v["field"], 6)) for v in val], flush=True)
    # --- SU(3) families
    X, C = T.su3_data()
    rho0 = T.su3_irrep(0, 0, X)
    Af, Ag, _ = T.su3_charge(rho0, X, C)
    q, beta = top_frame_state(Af, Ag, 8, 8)
    assert abs(q[-1] - 4) < 1e-10 and abs(q[-2] - 4) > 1e-6, q[-3:]
    out["su3_top_charge"] = float(q[-1])
    for fam in ("2a", "2b"):
        rows = []
        for t in map(float, T.GRID):
            g = T.su3_metric(fam, t); M = T.su3_background(g)
            ric, sf = shares(C, g, C, M, beta)
            rows.append({"t": t, "ricci": ric, "field": sf, "total": ric + sf})
        out[fam] = rows
    # --- verdict against the family results, if given
    if len(sys.argv) > 3:
        verdict = {}
        for fam, path in (("2a", sys.argv[2]), ("2b", sys.argv[3])):
            res = json.loads(Path(path).read_text())
            secs = [str(pq) for pq in T.SECTORS]
            t_all = {p["t"] for p in res["points"] if all(p["sectors"][k].get("is_floor") for k in secs)}
            sf = {r["t"]: r["field"] for r in out[fam]}
            bad = sorted(t for t in t_all if sf[t] >= 0)
            verdict[fam] = {"n_T_all": len(t_all), "T_all_points_with_s_F_nonneg": bad,
                            "G": ("untested (T_all empty)" if not t_all else ("holds" if not bad else "fails")),
                            "s_F_negative_but_not_floor_t": sorted(t for t in sf if sf[t] < 0 and t not in t_all)[:400]}
        out["prediction_G"] = verdict
        print(json.dumps(verdict, indent=1)[:2000])
    Path(sys.argv[1]).write_text(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
