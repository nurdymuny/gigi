"""Tests and mutants for the dressed chart (flux-method amendment §5.10). Exact Gaussians on dense grids.
usage: python scripts/test_helike_dressed_chart.py [--mutants]"""
import math
import sys
import unittest

import numpy as np

import helike_dressed_chart as dc


def grid2(n=241, L=7.0):
    x = np.linspace(-L, L, n)
    X, Y = np.meshgrid(x, x, indexing="ij")
    z = np.column_stack((X.ravel(), Y.ravel()))
    W = np.full(len(z), (x[1] - x[0]) ** 2)
    return W, z


def gauss(z, a, Om):
    d = z - a
    return np.exp(-0.5 * np.einsum("ni,ij,nj->n", d, Om, d))


class Dressing(unittest.TestCase):
    def test_shifted_and_deformed_gaussian_is_recovered(self):
        """psi0 = exp(-1/2 (z-a)^T Omega (z-a)) with a sheared constant kinetic form G: the dressed centre is a, the
        dressed widths are the true frequencies eig(G Omega), the vacuum covariance is harmonic and the kinetic form is
        the identity in the dressed frame."""
        W, z = grid2()
        a = np.array([0.4, -0.25])
        Om = np.array([[1.8, 0.3], [0.3, 1.1]])
        G = np.array([[1.2, 0.2], [0.2, 0.9]])
        zbar, C = dc.vacuum_moments(W, z, gauss(z, a, Om))
        zb, T, w = dc.dress(zbar, C, G)
        self.assertTrue(np.allclose(zb, a, atol=1e-10))
        self.assertTrue(np.allclose(np.sort(w), np.sort(np.linalg.eigvals(G @ Om).real), rtol=1e-9))
        Ti = np.linalg.inv(T)
        self.assertTrue(np.allclose(Ti @ C @ Ti.T, np.diag(1 / (2 * w)), atol=1e-10))
        self.assertTrue(np.allclose(Ti @ G @ Ti.T, np.eye(2), atol=1e-12))

    def test_planted_width_deformation_reads_one_in_the_dressed_frame(self):
        """Bee's control: the true vacuum is twice as stiff along mode 0 (Omega = 2 vs bare w = 1). Its first excitation
        along mode 0 reads w/Z = 8/9 in the bare frame and exactly 1 in the dressed frame."""
        W, z = grid2()
        wb = np.array([1.0, 1.5])
        Om = np.diag([2.0, 1.5])
        psi0 = gauss(z, 0, Om)
        psi1 = z[:, 0] * psi0
        bare = dc.reference_weight(W, z, psi0, psi1, np.zeros(2), np.eye(2), wb, [0])
        self.assertAlmostEqual(bare, 8 / 9, places=9)
        zbar, C = dc.vacuum_moments(W, z, psi0)
        zb, T, w = dc.dress(zbar, C, np.eye(2))
        k = int(np.argmax(np.abs(T[0])))                     # the dressed mode along coordinate 0
        dressed = dc.reference_weight(W, z, psi0, psi1, zb, T, w, [k])
        self.assertAlmostEqual(dressed, 1.0, places=9)

    def test_genuine_mixing_stays_mixed(self):
        """A carrier that really mixes two modes keeps its mixing in the dressed frame: w/Z = cos^2(theta)."""
        W, z = grid2()
        Om = np.diag([2.0, 1.5])
        psi0 = gauss(z, 0, Om)
        th = 0.5
        e0 = math.sqrt(2 * 2.0) * z[:, 0] * psi0
        e1 = math.sqrt(2 * 1.5) * z[:, 1] * psi0
        psi = math.cos(th) * e0 + math.sin(th) * e1
        zbar, C = dc.vacuum_moments(W, z, psi0)
        zb, T, w = dc.dress(zbar, C, np.eye(2))
        k = int(np.argmax(np.abs(T[0])))
        self.assertAlmostEqual(dc.reference_weight(W, z, psi0, psi, zb, T, w, [k]), math.cos(th) ** 2, places=9)

    def test_roles_by_overlap_under_permutation_and_rotation(self):
        """Dressed modes permuted, rotated inside the doublet, and slightly perturbed: roles are recovered by overlap.
        A mode rotated halfway between two roles is refused, and so is a split doublet."""
        rng = np.random.default_rng(2)
        bare = {"H": (0, 1), "C": (2,), "S": (3,)}
        R = np.eye(4)
        c, s = math.cos(0.7), math.sin(0.7)
        R[:2, :2] = [[c, -s], [s, c]]
        T = R[:, [2, 0, 1, 3]] + 1e-3 * rng.normal(size=(4, 4))   # dressed order: C, H, H, S
        w = np.array([0.8, 1.3, 1.3, 2.0])
        roles = dc.assign_roles(T, np.eye(4), bare, w)
        self.assertEqual(roles, {"H": (1, 2), "C": (0,), "S": (3,)})
        bad = T.copy()
        bad[:, [0, 3]] = (T[:, [0, 3]] @ np.array([[1, 1], [-1, 1]]) / math.sqrt(2))
        with self.assertRaises(ValueError):
            dc.assign_roles(bad, np.eye(4), bare, w)
        with self.assertRaises(ValueError):
            dc.assign_roles(T, np.eye(4), bare, np.array([0.8, 1.3, 1.31, 2.0]))

    def test_symmetrization_restores_the_rotation(self):
        """§5.10 A1: a planted rotation-breaking perturbation of the vacuum moments is removed: doublet means vanish,
        doublet blocks become (trace/2) I, cross terms with doublets vanish; singlet-singlet terms are kept."""
        rng = np.random.default_rng(4)
        C = np.diag([0.5, 0.5, 0.3, 0.2, 0.25, 0.25])
        C[2, 3] = C[3, 2] = 0.05                               # charge-0 singlets may correlate
        A = rng.normal(size=(6, 6)) * 1e-3
        Cp = C + 0.5 * (A + A.T)
        m = np.array([1e-3, -2e-3, 0.1, 0.02, 5e-4, 0.0])
        ms, Cs, asym = dc.symmetrize(m, Cp, [(0, 1), (4, 5)], [(2,), (3,)])
        self.assertEqual((ms[0], ms[1], ms[4], ms[5]), (0.0, 0.0, 0.0, 0.0))
        self.assertAlmostEqual(ms[2], 0.1); self.assertAlmostEqual(Cs[2, 3], Cp[2, 3])
        self.assertAlmostEqual(Cs[0, 0], Cs[1, 1]); self.assertEqual(Cs[0, 1], 0.0)
        self.assertEqual(Cs[0, 4], 0.0); self.assertEqual(Cs[1, 2], 0.0)
        self.assertGreater(asym, 0.0)
        w = dc.dress(ms, Cs, np.eye(6))[2]
        self.assertEqual([len(c) for c in dc.degenerate_clusters(w) if len(c) > 1], [2, 2])

    def test_chart_update_and_settling(self):
        u0 = np.arange(6.0); S = np.diag(np.arange(1.0, 7.0)); zb = np.full(6, 0.1); T = 2 * np.eye(6)
        u1, S1 = dc.chart_update(u0, S, zb, T)
        self.assertTrue(np.allclose(u1, u0 + S @ zb) and np.allclose(S1, 2 * S))
        self.assertTrue(dc.settled((u0, np.ones(6)), (u0 + 1e-8, np.ones(6) * (1 + 1e-8))))
        self.assertFalse(dc.settled((u0, np.ones(6)), (u0 + 1e-4, np.ones(6))))


MUTANTS = {}


def mutant(name):
    def deco(f):
        MUTANTS[name] = f
        return f
    return deco


@mutant("the dressed centre dropped")
def _m1():
    orig = dc.dress
    dc.dress = lambda zbar, C, G: (np.zeros_like(np.asarray(zbar, float)),) + orig(zbar, C, G)[1:]
    return lambda: setattr(dc, "dress", orig)


@mutant("the dressed widths dropped (bare widths kept)")
def _m2():
    orig = dc.dress
    def f(zbar, C, G):
        zb, T, w = orig(zbar, C, G)
        return zb, T, np.ones_like(w)
    dc.dress = f
    return lambda: setattr(dc, "dress", orig)


@mutant("the kinetic whitening dropped")
def _m3():
    orig = dc.sym_sqrt
    dc.sym_sqrt = lambda G: np.eye(len(G))
    return lambda: setattr(dc, "sym_sqrt", orig)


@mutant("roles assigned by frequency order")
def _m4():
    orig = dc.assign_roles
    def f(T, G_bare, bare_roles, w=None):
        out, k = {}, 0
        for role, idx in sorted(bare_roles.items(), key=lambda kv: min(kv[1])):
            out[role] = tuple(range(k, k + len(idx))); k += len(idx)
        return out
    dc.assign_roles = f
    return lambda: setattr(dc, "assign_roles", orig)


@mutant("the rotation symmetrization dropped (A1)")
def _m5():
    orig = dc.symmetrize
    dc.symmetrize = lambda mean, C, doublets, singlets: (np.array(mean, float), np.array(C, float), 0.0)
    return lambda: setattr(dc, "symmetrize", orig)


def run():
    return unittest.TextTestRunner(verbosity=0).run(unittest.defaultTestLoader.loadTestsFromTestCase(Dressing))


if __name__ == "__main__":
    if "--mutants" in sys.argv:
        caught = 0
        for name, apply in MUTANTS.items():
            undo = apply()
            try:
                ok = not run().wasSuccessful()
            finally:
                undo()
            caught += ok
            print(("CAUGHT  " if ok else "MISSED  ") + name, flush=True)
        print(f"{caught}/{len(MUTANTS)} mutants caught")
        sys.exit(0 if caught == len(MUTANTS) else 1)
    sys.exit(0 if run().wasSuccessful() else 1)
