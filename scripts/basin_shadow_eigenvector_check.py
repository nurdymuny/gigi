"""The shadow, eigenvector by eigenvector (22 November 2026), for docs/research/the-basin-2026-11-22.md, item 4.

The gauge transformation u(g) = g (degree one) maps the flat field A = 0 to A = theta and a perturbation b to
Ad(g^-1) b. In the builder's Peter-Weyl language a sector-j state is a right vector w in V_j (with a left copy v): the
component function is v^dagger pi_j(g) w. Multiplying by the matrix coefficients of Ad (spin 1, j = 2 in our units)
sends it into V_J for J in {j-2, j, j+2}: with isometric intertwiners Phi_J: V_J -> V_j (x) R^3,
  b'_{i,b}(g) = sum_a Ad(g)_{a,b} b_{i,a}(g) = sum_J sum_a (Phi_J^dag (v (x) e_a))^dag pi_J(g) Phi_J^dag (w_{ia} (x) e_b),
and expanding the left vectors in a basis l_k of V_J gives right-vector states Y_{J,k} in sector J (builder layout:
frame i, gauge b, V_J). Steps:
  1. intertwiners Phi_J by solving T_a Phi = Phi rho_J[a] (null space), T_a = rho_j[a] (x) I + I (x) R_a;
  2. covariance gate, with NO eigenvectors: for random sector-j states b, U(H_0 b) = H_theta(U b) in every target sector,
     for H + GG^*; this fixes the sign of R_a and the target field (the gate selects among the conventions tried and
     records which one passed);
  3. her A = 0 eigenvector in sector s (top charge of the family's Q, value (s+2)^2/t), pushed to sector s+2: residual
     against H_theta, its value, and whether the floor eigenspace of sector s+2 at theta is spanned by the images of her
     (and her mirror) alone, by principal angles.
usage: OMP_NUM_THREADS=1 python scripts/basin_shadow_eigenvector_check.py OUT.json
"""
import json
import sys
from pathlib import Path

import numpy as np
from scipy.linalg import null_space, subspace_angles, orth

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su3_test2_family as T2        # noqa: E402

S, C2, EPS = T2.su2_data()
REP = {}


def rep(j):
    if j not in REP:
        REP[j] = S.spin_rep(j)
    return REP[j]


def HG(x, y, t, j):
    H, G = T2.hessian_and_gauge(C2, [t, 1, 1], EPS, [x * np.eye(3)[0], y * np.eye(3)[1], y * np.eye(3)[2]], rep(j))
    return H, G, H + G @ G.conj().T


def intertwiners(j, sgn):
    """Phi_J (isometries V_J -> V_j (x) C^3) for J in {j-2, j, j+2}, J >= 0."""
    d = j + 1
    R = [sgn * 2 * T2.adm(np.eye(3)[a], EPS) for a in range(3)]
    T = [np.kron(rep(j)[a], np.eye(3)) + np.kron(np.eye(d), R[a]) for a in range(3)]
    out = {}
    for J in (j - 2, j, j + 2):
        if J < 0:
            continue
        dJ = J + 1; rJ = rep(J)
        # unknown Phi (3d x dJ), equations T_a Phi - Phi rJ_a = 0
        Mx = np.vstack([np.kron(np.eye(dJ), T[a]) - np.kron(rJ[a].T, np.eye(3 * d)) for a in range(3)])
        N = null_space(Mx)
        if N.shape[1] == 0:
            continue
        Phi = N[:, 0].reshape(dJ, 3 * d).T                     # column-major vec
        Phi = Phi / np.sqrt(np.real(np.trace(Phi.conj().T @ Phi)) / dJ)
        out[J] = Phi
    return out


def push(bvec, j, Phis, v):
    """Images Y_{J,k} of the sector-j state bvec (layout frame i, gauge a, V_j) with left copy v."""
    d = j + 1; b = bvec.reshape(3, 3, d)
    imgs = {}
    for J, Phi in Phis.items():
        dJ = J + 1
        L = np.array([Phi.conj().T @ np.kron(v, np.eye(3)[a]) for a in range(3)])        # L_{J,a} in V_J
        Rr = np.zeros((3, 3, 3, dJ), complex)                                             # [i, a, b, :]
        for i in range(3):
            for a in range(3):
                for bb in range(3):
                    Rr[i, a, bb] = Phi.conj().T @ np.kron(b[i, a], np.eye(3)[bb])
        Ys = []
        for k in range(dJ):
            c = L[:, k]                                                                   # c_{k a} = <l_k, L_a>
            Y = np.einsum("a,iabm->ibm", np.conj(c), Rr).reshape(-1)
            Ys.append(Y)
        imgs[J] = Ys
    return imgs


def main():
    out_path = Path(sys.argv[1]); res = {}
    t = 3.0; th = (2 / np.sqrt(t), 2.0)
    rng = np.random.default_rng(7)
    # 2. covariance gate over conventions
    chosen = None; gate = []
    for sgn in (1, -1):
        for target in ((th[0], th[1]), (-th[0], -th[1]), (-th[0], th[1]), (th[0], -th[1])):
            worst = 0.0
            for j in (1, 2, 3):
                _, _, A0 = HG(0.0, 0.0, t, j)
                Phis = intertwiners(j, sgn)
                if not Phis:
                    worst = np.inf; break
                AT = {J: HG(target[0], target[1], t, J)[2] for J in Phis}
                for _ in range(2):
                    b = rng.standard_normal(9 * (j + 1)) + 1j * rng.standard_normal(9 * (j + 1))
                    v = rng.standard_normal(j + 1) + 1j * rng.standard_normal(j + 1); v /= np.linalg.norm(v)
                    lhs = push(A0 @ b, j, Phis, v); rhs = push(b, j, Phis, v)
                    for J in Phis:
                        for Yl, Yr in zip(lhs[J], rhs[J]):
                            worst = max(worst, float(np.linalg.norm(Yl - AT[J] @ Yr) / max(1e-12, np.linalg.norm(Yl))))
            gate.append({"sign": sgn, "target_xy": list(target), "max_rel_violation": worst})
            if worst < 1e-10 and chosen is None:
                chosen = (sgn, target)
    res["covariance_gate"] = gate; res["chosen"] = chosen
    print("covariance gate:", gate, flush=True)
    if chosen is None:
        out_path.write_text(json.dumps(res, indent=1, default=str) + "\n"); raise SystemExit("stop: no convention passes")
    sgn, target = chosen
    # 3. her A = 0 eigenvector, pushed two sectors up
    rows = []
    for tv in (1.5, 3.0, 8.0):
        for s_ in range(0, 9):
            H0, G0, A0 = HG(0.0, 0.0, tv, s_)
            Hf, Gf, _ = HG(0.7, 1.3, tv, s_)                     # Q fitted at a generic family member
            parts = (1j * C2[0].T, 1j * 2 * T2.adm(np.eye(3)[0], EPS), 1j * rep(s_)[0])
            sg = T2.fit_signs(parts, Hf, Gf @ Gf.conj().T, 3, 3, s_ + 1)[0]
            Q = T2.assemble_Q(parts, sg, 3, 3, s_ + 1)
            if abs(np.linalg.eigvalsh(Q).max() - (s_ + 4)) > 1e-8:
                Q = -Q
            q, U = np.linalg.eigh(Q); her = U[:, np.argmax(q)]; mir = U[:, np.argmin(q)]
            lam = float((her.conj() @ A0 @ her).real)
            target = (2 / np.sqrt(tv), 2.0) if chosen[1][0] > 0 else (-2 / np.sqrt(tv), chosen[1][1])
            J = s_ + 2; Phis = intertwiners(s_, sgn)
            _, _, AT = HG(target[0], target[1], tv, J)
            # images of her and her mirror, over a basis of left copies
            imgs = []
            for vec in (her, mir):
                for k in range(s_ + 1):
                    v = np.eye(s_ + 1)[k].astype(complex)
                    imgs += [Y for Y in push(vec, s_, Phis, v)[J] if np.linalg.norm(Y) > 1e-10]
            Ymat = np.array(imgs).T
            Bimg = orth(Ymat, rcond=1e-9)
            resid = float(np.linalg.norm(AT @ Bimg - lam * Bimg) / max(1, abs(lam)))
            # theta floor eigenspace of sector J (H + GG^*), and theta's slice floor
            w, V = np.linalg.eigh(AT)
            HT, GT, _ = HG(target[0], target[1], tv, J); mu = np.sort(T2.slice_spectrum(HT, GT))
            Ef = V[:, np.abs(w - lam) <= 1e-9 * max(1, abs(lam))]
            angles = subspace_angles(Ef, Bimg) if Ef.shape[1] and Bimg.shape[1] else np.array([np.pi / 2])
            row = {"t": tv, "her_sector_A0": s_, "her_value": lam, "expected_(s+2)^2/t": (s_ + 2) ** 2 / tv,
                   "image_sector_theta": J, "image_dim": int(Bimg.shape[1]), "image_residual": resid,
                   "theta_eigenspace_at_her_value_dim": int(Ef.shape[1]), "max_principal_angle": float(np.max(angles)),
                   "image_spans_eigenspace": bool(Bimg.shape[1] == Ef.shape[1] and np.max(angles) < 1e-7),
                   "theta_slice_floor_sector_J": float(mu[0]), "her_value_is_theta_slice_floor": bool(abs(mu[0] - lam) <= 1e-9 * max(1, lam))}
            rows.append(row); print(row, flush=True)
    res["rows"] = rows
    out_path.write_text(json.dumps(res, indent=1, default=str) + "\n")


if __name__ == "__main__":
    main()
