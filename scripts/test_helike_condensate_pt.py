"""Gates for the O(g^2) condensate perturbation theory (scripts/helike_condensate_pt.py)."""
import unittest

import numpy as np

import helike_berger_model as bm
import helike_condensate_pt as cp


def one_dim_shift(eps, lam, mu, w=1.3):
    F = cp.Fock([w])
    z, d = (lambda st: F.z(st, 0)), (lambda st: F.dz(st, 0))
    res = []
    for n in (0, 1):
        st = {(n,): 1.}
        img = {}
        cp.addto(img, z(z(z(st))), lam)
        cp.addto(img, z(d(d(st))), -.5 * eps)
        cp.addto(img, d(st), -.5 * eps)
        v4 = {}
        cp.addto(v4, z(z(z(z(st)))), mu)
        e0 = F.energy((n,))
        res.append(v4.get((n,), 0.) + sum(a * a / (e0 - F.energy(o)) for o, a in img.items() if o != (n,)))
    return res[1] - res[0]


def one_dim_exact(eps, lam, mu, w=1.3, L=9., Nx=2400):
    x = np.linspace(-L, L, Nx)
    h = x[1] - x[0]
    xm = x[:-1] + h / 2
    a = 1 + eps * xm
    V = .5 * w * w * x ** 2 + lam * x ** 3 + mu * x ** 4
    H = np.diag((a[:-1] + a[1:]) / (2 * h * h) + V[1:-1]) + np.diag(-a[1:-1] / (2 * h * h), 1) + np.diag(-a[1:-1] / (2 * h * h), -1)
    e = np.linalg.eigvalsh(H)[:2]
    return e[1] - e[0] - w


class Reduction(unittest.TestCase):
    def test_measure_identity(self):
        q = np.random.default_rng(1).normal(size=(3, 3))
        G = q @ q.T
        Gv = np.array([G[k, l] for k, l in cp.GKEYS])
        A = cp.A_at(Gv)
        divA = np.array([sum(cp.C_A[i, j, j] for j in range(6)) for i in range(6)])
        Gi = np.linalg.inv(G)
        dlogdet = np.array([Gi[k, l] * (1 if k == l else 2) for k, l in cp.GKEYS])
        np.testing.assert_allclose(divA + A @ (-.5 * dlogdet), cp.B_VEC, atol=1e-10)

    def test_harmonic_frequencies_equal_q_space_vibrations(self):
        for s, t, g in ((1.2, 3., .3), (1.1, 4., .2)):
            pt = cp.CondensatePT(s, t, g)
            self.assertLess(pt.grad_norm, 1e-10)
            vib = bm.mc_vibrations(s, t, g)
            q = np.sort(np.sqrt([x for v in vib.values() for x in v]))
            np.testing.assert_allclose(np.sort(pt.w), q, atol=2e-5)


class Engine(unittest.TestCase):
    def test_one_dimensional_validation(self):
        off = one_dim_exact(0, 0, 0)
        for k in (.5, .25):
            eps, lam, mu = .06 * k, .05 * k, .02 * k * k
            p, e = one_dim_shift(eps, lam, mu), one_dim_exact(eps, lam, mu) - off
            self.assertLess(abs(p - e), .01 * abs(e), k)


class Corrections(unittest.TestCase):
    def test_g_squared_scaling_and_degeneracy(self):
        out = {}
        for g in (.2, .1):
            gaps = cp.CondensatePT(1.15, 3., g).gaps()
            wH = np.sqrt(bm.mc_helike_freq2(1.15, 3.))
            H = next(o for o in gaps if o["multiplicity"] == 2 and abs(o["omega"] - wH) < 1e-6)
            C = min((o for o in gaps if o["multiplicity"] == 1), key=lambda o: o["omega"])
            self.assertAlmostEqual(H["dE2"][0], H["dE2"][1], places=10)
            out[g] = (H["dE2"][0] / g ** 2, C["dE2"][0] / g ** 2)
        np.testing.assert_allclose(out[.2], out[.1], rtol=1e-8)
        np.testing.assert_allclose(out[.1], (-.5300, -2.7379), atol=2e-3)


if __name__ == "__main__":
    unittest.main()
