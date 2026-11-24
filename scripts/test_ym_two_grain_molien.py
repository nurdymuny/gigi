"""Phase 3 budget gates: Molien-Weyl counts, independent null-space counts, and the proven (0,0) basis."""
import unittest

import numpy as np

import ym_helike_j0 as j0
import ym_helike_j2 as j2
import ym_two_grain_molien as mo

NS = (10, 12, 14, 16, 18, 20)


def _hilbert(tc, D, sector=(0, 0)):
    """A non-integral (refused) Molien computation is a failed assertion, not an error."""
    try:
        return mo.hilbert_bigraded(D, sector)
    except ArithmeticError as exc:
        tc.fail(f"Molien refused: {exc}")


class MolienCounts(unittest.TestCase):
    def setUp(self):
        self.H00 = _hilbert(self, 20)
        self.H20 = _hilbert(self, 20, (2, 0))

    def test_integral_and_grain_symmetric(self):
        np.testing.assert_array_equal(self.H00, self.H00.T)
        self.assertEqual(self.H00[0, 0], 1)

    def test_one_grain_marginals_match_verified_solvers(self):
        self.assertEqual(mo.one_grain_counts(self.H00, NS), {N: len(j0.basis(N)) for N in NS})
        self.assertEqual(mo.one_grain_counts(self.H20, NS), {N: len(j2.basis(N)) for N in NS})

    def test_independent_nullspace_counts(self):
        for k, l in [(1, 1), (2, 1), (3, 0), (4, 0), (2, 2), (3, 1)]:
            self.assertEqual(mo.nullspace_count(k, l), self.H00[k, l], (k, l))

    def test_mixed_sector_starts_at_degree_two(self):
        H11 = _hilbert(self, 6, (1, 1))
        self.assertEqual(H11[1, 1], 1)
        self.assertEqual(H11[0, 0] + H11[1, 0] + H11[0, 1] + H11[2, 0] + H11[0, 2], 0)


class ProvenBasis(unittest.TestCase):
    def test_generators_are_homogeneous_of_declared_bidegree(self):
        rng = np.random.default_rng(81)
        q = rng.integers(1, 1000, size=(3, 3)).astype(np.int64)
        p = rng.integers(1, 1000, size=(3, 3)).astype(np.int64)
        lam, mu = 7, 11
        base, scaled = mo.generators_mod(q, p), mo.generators_mod(lam * q % mo.PRIME, mu * p % mo.PRIME)
        for name, ((k, l), v) in base.items():
            want = v * pow(lam, k, mo.PRIME) * pow(mu, l, mo.PRIME) % mo.PRIME
            self.assertEqual(scaled[name][1], want, name)

    def test_ten_generators_form_a_basis_through_degree_16(self):
        H = _hilbert(self, 16)
        orig = mo.generators_mod
        mo.generators_mod = lambda q, p: {k: v for k, v in orig(q, p).items() if k != "Y"}
        try:
            res = mo.verify_generators(16, H)
        finally:
            mo.generators_mod = orig
        for kl, v in res.items():
            self.assertTrue(v["spans"], kl)
            self.assertEqual(v["monomials"], v["molien"], kl)   # count = rank = Molien: a basis, no relations

    def test_rank_mod_p_detects_dependence(self):
        M = np.array([[1, 2, 3], [2, 4, 6], [1, 0, 1]], dtype=np.int64)
        self.assertEqual(mo.rank_mod_p(M), 2)


if __name__ == "__main__":
    unittest.main()
