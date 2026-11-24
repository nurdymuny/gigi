"""Gates for the sourced-condensate solver (mixed widths, orbit-envelope functions)."""
import math
import unittest

import mpmath as mp
import numpy as np

import helike_berger_condensate as bc
import helike_berger_model as bm
import helike_berger_sector as bs


def _solve(tc, *a, **k):
    try:
        return bc.solve_items(*a, **k)
    except (ArithmeticError, ValueError, ZeroDivisionError) as exc:
        tc.fail(f"condensate solver refused: {exc}")


def evaluate(p, q):
    G, d = q @ q.T, np.linalg.det(q)
    tot = 0.
    for e, c in p.items():
        v = float(c)
        for i, (k, l) in enumerate(bs.GKEYS):
            v *= G[k, l] ** e[i]
        tot += v * d ** e[6]
    return tot


class Condensate(unittest.TestCase):
    def test_single_width_items_reproduce_sector_solver(self):
        t = 2.
        r1 = bs.solve(6, .5, t, 0, 1, dps=40, nlev=4)
        r2 = _solve(self, bc.origin_items(6, 0, 1, list(bm.omega(t))), .5, t, dps=40, nlev=4)
        np.testing.assert_allclose(r1["energies"], r2["energies"], atol=1e-12)

    def test_mixed_width_overlap(self):
        """Normalised overlap of exp(-|q|^2 a/2) and exp(-|q|^2 b/2) per row is (2 sqrt(ab)/(a+b))^{3/2} per row."""
        Mo = bc.Moments()
        with mp.workdps(30):
            one = {bs.ONE: mp.mpf(1)}
            a, b = [mp.mpf(1.2), mp.mpf(.7), mp.mpf(.7)], [mp.mpf(.5), mp.mpf(1.5), mp.mpf(1.5)]
            ov = Mo.inner(one, a, one, b) / mp.sqrt(Mo.inner(one, a, one, a) * Mo.inner(one, b, one, b))
            want = 1
            for k in range(3):
                want *= (2 * mp.sqrt(a[k] * b[k]) / (a[k] + b[k])) ** (mp.mpf(3) / 2)
            self.assertLess(abs(ov - want), mp.mpf(10) ** -20)

    def test_orbit_envelope_is_the_haar_average(self):
        from scipy.spatial.transform import Rotation
        lam = [1.2 * 1.3, .8 * .9, .8 * .9]
        with mp.workdps(30):
            F = bc.dome_poly(24, [mp.mpf(x) for x in lam])
        q = np.random.default_rng(2).normal(size=(3, 3)) * .7
        R = Rotation.random(60000, random_state=3).as_matrix()
        e = np.exp(np.einsum("nij,ij->n", R, np.diag(lam) @ q))
        self.assertLess(abs(evaluate(F, q) - e.mean()), 5 * e.std() / math.sqrt(len(e)))

    def test_sources_enter_exactly(self):
        """Single Gaussian: switching on the sources shifts the energy by -3/4 (a/alpha_0 + 2 b/alpha_perp)."""
        alpha, a, b = [1.3, .8, .8], -.18, .11
        item = [({bs.ONE: mp.mpf(1)}, alpha)]
        e0 = _solve(self, item, .3, 4., dps=40, nlev=1)["energies"][0]
        e1 = _solve(self, item, .3, 4., a, b, dps=40, nlev=1)["energies"][0]
        self.assertAlmostEqual(e1 - e0, -.75 * (a / alpha[0] + 2 * b / alpha[1]), places=10)

    def test_orbit_envelope_never_raises_ritz_values(self):
        t, s, g2 = 3., 1.2, .2
        _, _, qb = bm.mc_background(s, t, math.sqrt(g2))
        a, b = bm.mc_sources(s, t)
        base = _solve(self, bc.origin_items(4, 0, 1, [1., 1., 1.]), g2, t, a, b, dps=40, nlev=3)
        aug = _solve(self, bc.origin_items(4, 0, 1, [1., 1., 1.]) + bc.dome_items(0, 0, 1, 8, np.diag(qb), [1., 1., 1.]),
                     g2, t, a, b, dps=40, nlev=3)
        for e1, e2 in zip(base["energies"], aug["energies"]):
            self.assertLessEqual(e2, e1 + 1e-10)
        self.assertLess(aug["energies"][0], base["energies"][0] - 1e-3)


if __name__ == "__main__":
    unittest.main()
