"""The fall on the Poincare homology sphere S^3/2I (22 November 2026).

Setting: the binary icosahedral group 2I (order 120) acts on S^3 = SU(2) by LEFT multiplication. That preserves the
left-invariant Berger metric (t,1,1), the left-invariant fields of the circle-symmetric family (M0 = x e0, M1 = y e1,
M2 = y e2), and the rotation about the long circle. So the family, its energy and its gradient flow descend unchanged
(the energy is divided by 120), and the fall from the Berger field is the same path (x(tau), y(tau)) as on S^3,
landing on A = theta (x = 2/sqrt t, y = 2) for t != 1 (landing_scan.py).

What is new on the quotient:
  1. Holonomy. On S^3, A = theta = g^{-1}dg is gauge-equivalent to A = 0 through u(g) = g, which wraps S^3 once.
     u is not 2I-invariant (u(gamma g) = gamma u(g)), so it does not descend. On S^3/2I the parallel transport of
     A = theta around the loop of gamma is Ad(gamma^{-1}) (checked numerically below by integrating the transport
     equation with the family's own matrices): the holonomy is the defining 2-dimensional representation of 2I,
     which is irreducible. A = 0 has trivial holonomy. So the two flat fields are two different basins.
  2. Sectors. Peter-Weyl sector j survives with multiplicity dim V_j^{2I} (Molien series of 2I).
  3. The spectrum at the bottom of each basin, in the surviving sectors, with the Floor paper's own builders:
     H + GG^* (charge blocks, ricci_su2_exact_edge.Blocks) and the gauge-orthogonal slice (ricci_su3_test2_family).
usage: python docs/research/basin-2026-11-22/poincare_fall.py   (from the repository root)
"""
import itertools
import sys
from pathlib import Path

import numpy as np
from scipy.integrate import solve_ivp
from scipy.linalg import expm

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "scripts"))
_argv, sys.argv = sys.argv, [sys.argv[0]]
import ricci_su3_test2_family as T2          # noqa: E402
import ricci_su2_exact_edge as EE            # noqa: E402
sys.argv = _argv

PHI = (1 + 5 ** 0.5) / 2


# ---------------------------------------------------------------- the group 2I as unit quaternions
def icosians():
    els = set()
    def add(q):
        els.add(tuple(np.round(q, 12)))
    for i in range(4):
        for s in (1, -1):
            q = np.zeros(4); q[i] = s; add(q)
    for signs in itertools.product((1, -1), repeat=4):
        add(0.5 * np.array(signs))
    base = [PHI, 1, 1 / PHI, 0]
    even = [p for p in itertools.permutations(range(4))
            if sum(1 for a in range(4) for b in range(a + 1, 4) if p[a] > p[b]) % 2 == 0]
    for p in even:
        for signs in itertools.product((1, -1), repeat=3):
            v = np.array(base, float) * np.array(list(signs) + [1])
            q = np.zeros(4)
            for k in range(4):
                q[p[k]] = v[k]
            add(0.5 * q)
    return [np.array(e) for e in els]


def quat_to_su2(q):
    a, b, c, d = q
    return np.array([[a + 1j * b, c + 1j * d], [-c + 1j * d, a - 1j * b]])


def dim_invariants(G, j):
    tot = 0.0
    for q in G:
        th = np.arccos(np.clip(q[0], -1, 1))
        if abs(np.sin(th)) < 1e-12:
            tot += (j + 1) * (1 if q[0] > 0 else (-1) ** j)      # gamma = +1 or -1
        else:
            tot += np.sin((j + 1) * th) / np.sin(th)
    return tot / len(G)


# ---------------------------------------------------------------- holonomy of A = theta around gamma
TAU = [-0.5j * np.array(s) for s in ([[0, 1], [1, 0]], [[0, -1j], [1j, 0]], [[1, 0], [0, -1]])]   # [tau_a, tau_b] = eps tau_c


def su2_log(U):
    """Y in su(2) with expm(Y) = U, as coefficients in TAU."""
    w, V = np.linalg.eig(U)
    L = V @ np.diag(np.log(w)) @ np.linalg.inv(V)
    return np.array([np.real(-2 * np.trace(L @ T)) for T in TAU])  # <tau_a, tau_b> = -tr/2 ... coefficient extraction


def adjoint(U):
    return np.array([[np.real(-2 * np.trace(TAU[a] @ U @ TAU[b] @ U.conj().T)) for b in range(3)] for a in range(3)])


def holonomy_theta(U, tv):
    """Transport of the Lie index along s -> exp(s Y), U = exp(Y), for the family field at A = theta.
    In the Floor paper's frame the left-invariant fields X_a satisfy [X_a, X_b] = 2 eps X_c, so X_a = 2 tau_a, and the
    orthonormal frame is e_a = X_a / sqrt(g_aa). A path with velocity sum Y^a tau_a = sum (Y^a/2) X_a has
    A(velocity) = sum (Y^a/2) sqrt(g_aa) M_a, acting on the Lie index by the cross product."""
    Y = su2_log(U)
    g = np.array([tv, 1.0, 1.0])
    M = [np.array([2 / np.sqrt(tv), 0, 0]), np.array([0, 2.0, 0]), np.array([0, 0, 2.0])]
    a = sum((Y[k] / 2) * np.sqrt(g[k]) * M[k] for k in range(3))
    Acr = EE.cross(a)
    Acr = np.array(Acr.tolist(), float)
    sol = solve_ivp(lambda s, v: (-Acr @ v.reshape(3, 3)).ravel(), [0, 1], np.eye(3).ravel(), rtol=1e-12, atol=1e-13)
    return sol.y[:, -1].reshape(3, 3)


def main():
    G = icosians()
    print(f"|2I| = {len(G)}")
    surv = [j for j in range(0, 41) if round(dim_invariants(G, j)) > 0]
    print("surviving sectors j <= 40 (dim V_j^2I):", [(j, int(round(dim_invariants(G, j)))) for j in surv])

    # holonomy check
    print("\nholonomy of A = theta around gamma, against Ad(gamma^{-1}); trace of the adjoint holonomy vs tr(gamma)^2 - 1")
    worst = 0.0
    for q in G[:40]:
        U = quat_to_su2(q)
        if np.allclose(U, np.eye(2)) or np.allclose(U, -np.eye(2)):
            continue
        for tv in (3.0, 3.55):
            Hol = holonomy_theta(U, tv)
            target = adjoint(U.conj().T)
            worst = max(worst, np.abs(Hol - target).max())
    print(f"  max |Hol - Ad(gamma^-1)| over 2I elements and t = 3, 3.55: {worst:.2e}")
    tr = sorted({round(float(np.real(np.trace(quat_to_su2(q)))), 6) for q in G})
    print("  fundamental traces of 2I classes:", tr, " -> not all equal to 2: the holonomy is non-trivial (irreducible)")

    # energy cross-check of the family formula
    S, C2, eps = T2.su2_data()
    for (xv, yv, tv) in [(-0.5, 1.2, 3.0), ((2 - 3.55) / np.sqrt(3.55), np.sqrt(3.55), 3.55)]:
        M = [xv * np.eye(3)[0], yv * np.eye(3)[1], yv * np.eye(3)[2]]
        e_builder = T2.energy(C2, [tv, 1, 1], eps, M)
        B = yv ** 2 - 2 * np.sqrt(tv) * xv; c = yv * (xv - 2 / np.sqrt(tv))
        print(f"\nenergy check at (x,y,t)=({xv:.4f},{yv:.4f},{tv}): builder {e_builder:.6f}  family formula {0.5*(B*B+2*c*c):.6f}")

    # spectra at the bottoms
    blocks = EE.Blocks()
    rho = {}
    print("\nspectrum at the bottom of each basin, surviving sectors only")
    for tv in (1.0, 2.75, 3.0, 3.55, 5.0, 10.0):
        for name, (xv, yv) in {"A=0 (trivial holonomy)": (0.0, 0.0),
                               "A=theta (holonomy = 2I itself)": (2 / np.sqrt(tv), 2.0)}.items():
            rows = []
            for j in surv[:5]:
                ev = blocks.sector_spectrum(xv, yv, tv, j)
                if j not in rho:
                    rho[j] = S.spin_rep(j)
                M = [xv * np.eye(3)[0], yv * np.eye(3)[1], yv * np.eye(3)[2]]
                H, Gm = T2.hessian_and_gauge(C2, [tv, 1, 1], eps, M, rho[j])
                mu = T2.slice_spectrum(H, Gm)
                lam = EE.lam_tw(xv, yv, tv, j)
                below = int((mu < lam - 1e-8).sum())
                rows.append((j, ev.min(), mu.min(), lam, below, int((np.abs(mu) < 1e-8).sum())))
            print(f"  t={tv:5.2f} {name}")
            for (j, a, b, lam, below, nz) in rows:
                print(f"     j={j:2d}: min(H+GG*)={a:9.4f}  floor (gauge-orth)={b:9.4f}  her price={lam:9.4f}  "
                      f"slice states below her={below:4d}  slice zero modes={nz}")


if __name__ == "__main__":
    main()
