"""M8 layer 3: physical overlaps between states at different points (registration A3.1), and the tracking family.

The configuration space (the cone of 3x3 Gram matrices) and its measure do not depend on s, and the Cholesky
coordinates u = (log r, a, b, c) are global. Each point's chart is affine, u = u0 + S z. With chi = sqrt(rho) Psi and
rho dz = dmu, the ratio rho_A / rho_B is the constant |det S_A / det S_B|, so

  <Psi_a(A), Psi_b(B)> = sqrt|det S_B / det S_A| * int chi_a(z_A(z_B)) chi_b(z_B) dz_B,
  z_A = S_A^{-1} (u0_B - u0_A + S_B z_B).

On B's Gauss–Hermite grid (x_B = sqrt(w_B) z_B, weights for e^{-|x_B|^2}):
  int chi_a chi_b dz_B = sum wts p_a(x_A) p_b(x_B) e^{(|x_B|^2 - |x_A|^2)/2} N_A N_B / prod sqrt(w_B),
  N = prod (w / pi)^{1/4}.
p_a at the scattered points x_A is evaluated directly from the Hermite basis.
"""
import math
from pathlib import Path
import subprocess
import sys

import numpy as np

import helike_m8_geometry as gm
import helike_m8_tracking as tr

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / ("helike-lab/target/release/helike" + (".exe" if sys.platform == "win32" else ""))
SEAM = 1e-4
CHUNK = 1024


def scattered_values(X, cols, xs):
    """p for coefficient columns at arbitrary Hermite points xs (n, 6)."""
    mq = X["mq"]
    basis = np.array(X["basis"], dtype=np.int64)
    C = X["coeff"][:, cols]
    out = np.empty((len(xs), len(cols)))
    for a in range(0, len(xs), CHUNK):
        b = min(a + CHUNK, len(xs))
        tabs = [gm.normalized_hermites(xs[a:b, j], mq).T for j in range(6)]      # (chunk, mq+1)
        val = tabs[0][:, basis[:, 0]]
        for j in range(1, 6):
            val *= tabs[j][:, basis[:, j]]
        out[a:b] = val @ C
    return out


def overlap_matrix(XA, colsA, XB, colsB, order):
    """O[i, j] = <Psi_a_i(A), Psi_b_j(B)> (signed), on B's grid at the given order."""
    W, xB = gm.grid_nodes(order, XB["w"])
    zB = xB / np.sqrt(XB["w"])
    zA = np.linalg.solve(XA["S"], (XB["u0"] - XA["u0"])[:, None] + XB["S"] @ zB.T).T
    xA = zA * np.sqrt(XA["w"])
    pB, _ = gm.grid_values(XB["basis"], XB["coeff"][:, colsB], order, XB["mq"])
    pA = scattered_values(XA, colsA, xA)
    NA = np.prod((XA["w"] / math.pi) ** 0.25)
    NB = np.prod((XB["w"] / math.pi) ** 0.25)
    gauss = np.exp(0.5 * (np.sum(xB * xB, axis=1) - np.sum(xA * xA, axis=1)))
    jac = math.sqrt(abs(np.linalg.det(XB["S"]) / np.linalg.det(XA["S"])))
    fac = (W * math.pi ** 3) * gauss * NA * NB / np.prod(np.sqrt(XB["w"])) * jac
    return pA.T @ (fac[:, None] * pB)


def resolved_overlap(XA, colsA, XB, colsB, orders=(10, 12)):
    """Squared overlaps at the fine order, with the order-10 vs order-12 seam (A3.1: within 1e-4)."""
    Oc = overlap_matrix(XA, colsA, XB, colsB, orders[0]) ** 2
    Of = overlap_matrix(XA, colsA, XB, colsB, orders[1]) ** 2
    seam = float(np.max(np.abs(Of - Oc)))
    if seam > SEAM:
        raise ArithmeticError(f"overlap quadrature seam {seam:.2e} exceeds {SEAM}")
    return Of, seam


class PhysicalFamily:
    """The tracking family for one channel at fixed (t, g2, Mq), parametrised by s. sector: 'helike' (charge-2
    branches) or 'competitor' (charge-0 single levels above the ground state)."""

    def __init__(self, t, g2, mq, sector, workdir, orders=None):
        self.t, self.g2, self.mq, self.sector_name = t, g2, mq, sector
        self.dir = Path(workdir); self.dir.mkdir(parents=True, exist_ok=True)
        n = max(10, mq + 1); n += n % 2
        self.orders = orders or (n, n + 2)                 # amendment 4: seam pair by basis degree
        self.seams = []

    def export(self, s):
        path = self.dir / f"levels_s{s!r}_t{self.t}_g{self.g2}_Mq{self.mq}.json"
        if not path.exists():
            subprocess.run([str(BIN), "--out", str(path.with_suffix(".receipt.json")), "cone-levels", "--s", repr(s),
                            "--t", str(self.t), "--g2", str(self.g2), "--mq", str(self.mq), "--levels-out", str(path)],
                           check=True, capture_output=True, text=True)
        return gm.load_levels(path)

    def sample(self, s):
        X = self.export(s)
        br = gm.branches(X)
        want = 2 if self.sector_name == "helike" else 0
        clusters = [lev for lev, q in br if q == want and lev != (0,)]
        E = X["levels"]
        unc = X["resid"].copy()
        # a symmetric pair's numerical splitting is a truncation artifact; half of it joins the uncertainty
        for lev in clusters:
            if len(lev) == 2:
                half = 0.5 * abs(E[lev[1]] - E[lev[0]])
                for l in lev:
                    unc[l] = unc[l] + half
        return tr.Sample(s, E, unc, clusters, list(range(len(clusters))), payload=X)

    def overlap(self, A, B):
        XA, XB = A.payload, B.payload
        k = len(XA["levels"])
        O, seam = resolved_overlap(XA, list(range(k)), XB, list(range(len(XB["levels"]))), self.orders)
        self.seams.append(seam)
        return O
