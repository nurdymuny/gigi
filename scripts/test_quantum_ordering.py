"""Focused, planted review gates. No held-out coupling is evaluated."""
import json
import math
import unittest
from pathlib import Path

import numpy as np
import scipy.linalg as la

import quantum_ordering as o
import ym_helike_quantum as q


def branch(low, residual=0., next_low=6., next_residual=0., high=None, spin_error=0.):
    return dict(low=low, high=low if high is None else high, residual=residual,
                next_low=next_low, next_residual=next_residual, spin_error=spin_error)


class BoundTests(unittest.TestCase):
    def test_known_separation_formula(self):
        rho, eps2 = 2.03, .0891
        r = o.lower_estimate(rho, math.sqrt(eps2), 5.)
        self.assertTrue(r.get('usable'))
        self.assertAlmostEqual(r['lower'], 2., places=12)

    def test_separation_guard_is_strict(self):
        self.assertFalse(o.lower_estimate(2., .5, 2.5)['usable'])
        self.assertEqual(o.lower_estimate(2., .5, 2.5)['reason'], 'SEPARATION_NOT_RESOLVED')

    def test_both_conditional_directions(self):
        self.assertEqual(o.ordering(branch(2.), branch(3.))['conditional_order'], 'SCALAR_LOWER')
        self.assertEqual(o.ordering(branch(3.), branch(2.))['conditional_order'], 'TENSOR_LOWER')

    def test_equal_levels_are_unresolved(self):
        self.assertEqual(o.ordering(branch(2.), branch(2.))['conditional_order'], 'ORDER_UNRESOLVED')

    def test_missing_exact_level_never_becomes_verified(self):
        # Exact scalar [0,1,2,4], captured [0,2,4]; tensor begins at 1.5.
        r = o.ordering(branch(2., next_low=4.), branch(1.5, next_low=5.))
        self.assertEqual(r['conditional_order'], 'TENSOR_LOWER')
        self.assertEqual(r['unconditional_order'], 'ORDER_UNRESOLVED')
        self.assertIs(r['separation_verified'], False)
        self.assertIs(r['certified'], False)
        self.assertEqual(r['basis'], 'ASSUMED_FROM_RITZ')

    def test_unused_unavailable_bound_does_not_block_order(self):
        r = o.ordering(branch(2., 1., 2.1), branch(3.))
        self.assertEqual(r['conditional_order'], 'SCALAR_LOWER')

    def test_half_separation_can_change_the_order(self):
        r = o.ordering(branch(2., .3, 3.), branch(1.85, 1., 3.))
        self.assertEqual(r['conditional_order'], 'ORDER_UNRESOLVED')
        self.assertEqual(r['reason'], 'SEPARATION_SENSITIVE')

    def test_half_separation_guard_cannot_be_skipped(self):
        r = o.ordering(branch(1., next_low=6.), branch(2., .6, 3.))
        self.assertEqual(r['conditional_order'], 'ORDER_UNRESOLVED')
        self.assertEqual(r['reason'], 'SEPARATION_SENSITIVE')

    def test_spin_failure_refuses_instead_of_padding_energy(self):
        with self.assertRaisesRegex(ValueError, 'spin'):
            o.ordering(branch(2., spin_error=1e-3), branch(3.))

    def test_spin_tolerance_does_not_change_energy_residual(self):
        r = o.ordering(branch(2., .1, 5., spin_error=5e-7), branch(3.))
        self.assertIn('scalar', r)
        self.assertEqual(r['scalar']['residual'], .1)
        self.assertAlmostEqual(r['scalar']['base']['lower'], 2.-.01/3., places=14)

    def test_next_residual_is_in_denominator(self):
        r = o.ordering(branch(2., .2, 3., .5), branch(4.))
        self.assertIn('scalar', r)
        self.assertEqual(r['scalar']['beta'], 2.5)
        self.assertAlmostEqual(r['scalar']['base']['lower'], 1.92)

    def test_invalid_numbers_refused(self):
        for value in (-1., float('nan'), float('inf')):
            with self.subTest(value=value), self.assertRaises(ValueError):
                o.lower_estimate(2., value, 5.)


class SectorTests(unittest.TestCase):
    @staticmethod
    def fixture():
        e = np.array([0., 1.]+[1.]*5+[2.]+[3.]*5)
        j = np.diag([0., 0.]+[6.]*5+[0.]+[6.]*5)
        return e, np.eye(13), np.eye(13), j

    def test_next_means_distinct_complete_multiplet(self):
        levels = o.select_levels(*self.fixture())
        self.assertEqual(set(levels), {'vacuum','scalar','scalar_next','tensor','tensor_next'})
        self.assertEqual(levels['tensor']['rank'], 5)
        self.assertEqual(levels['tensor_next']['low'], 3.)
        self.assertEqual(levels['scalar_next']['low'], 2.)

    def test_mixed_block_complex_rotation_and_metric(self):
        e, v, _, j = self.fixture()
        rng = np.random.default_rng(51)
        v = v.astype(complex)
        v[1:7, 1:7] = la.qr(rng.normal(size=(6,6))+1j*rng.normal(size=(6,6)))[0]
        scales = np.linspace(1., 2., 13)
        s = np.diag(scales**2)
        cov_j = scales[:,None]*j*scales[None,:]
        levels = o.select_levels(e, v/scales[:,None], s, cov_j)
        self.assertIn('tensor', levels)
        for name, level in levels.items():
            w = level['vectors']
            np.testing.assert_allclose(w.conj().T@s@w, np.eye(level['rank']), atol=1e-12)
            target = level['spin']*(level['spin']+1)
            np.testing.assert_allclose(w.conj().T@cov_j@w, target*np.eye(level['rank']), atol=1e-12)

    def test_incomplete_multiplet_refused(self):
        e, v, s, j = self.fixture()
        with self.assertRaisesRegex(ValueError, 'multiplet'):
            o.select_levels(e[:-1],v[:-1,:-1],s[:-1,:-1],j[:-1,:-1])

    def test_missing_neighbor_refused(self):
        e, v, s, j = self.fixture()
        with self.assertRaisesRegex(ValueError, 'next'):
            o.select_levels(e[:8],v[:8,:8],s[:8,:8],j[:8,:8])

    def test_repeated_same_spin_copies_refused(self):
        e,v,s,j=self.fixture(); e[8:]=1.
        order=np.argsort(e)
        with self.assertRaisesRegex(ValueError,'repeated'):
            o.select_levels(e[order],v[:,order],s,j)

    def test_invalid_spin_refused(self):
        e,v,s,j=self.fixture(); j[2,2]=6.01
        with self.assertRaisesRegex(ValueError,'Casimir'):
            o.select_levels(e,v,s,j)


def direct_ladder_residual(b, v, B, g2, alpha):
    """Independent tuple/ladder action; no production transition routine."""
    fock = b['C']@v
    pol = q.operator_polynomials()
    terms = [(float(c)*(1/(2*alpha)-alpha/2), axes) for c, axes in q.terms(pol['trace'])]
    terms += [(-3*math.sqrt(g2)*alpha**-1.5*c, axes) for c, axes in q.terms(pol['D'])]
    terms += [(g2*alpha**-2*c, axes) for c, axes in q.terms(pol['V4'])]
    result = {}
    for state, row in zip(b['states'], fock):
        state = tuple(map(int, state))
        result[state] = result.get(state, 0) + alpha*(sum(state)+4.5)*row-row@B
        for c, axes in terms:
            active = {state:c}
            for axis in reversed(axes):
                nxt = {}
                for st, value in active.items():
                    n = st[axis]
                    for sign in (-1,1):
                        if sign == -1 and n == 0:
                            continue
                        out = list(st); out[axis] += sign; out = tuple(out)
                        nxt[out] = nxt.get(out, 0) + value*math.sqrt((n+int(sign>0))/2)
                active = nxt
            for st, value in active.items():
                result[st] = result.get(st, 0)+value*row
    action = np.array(list(result.values()))
    return action.conj().T@action


class ResidualTests(unittest.TestCase):
    def test_existing_residuals_agree_on_identical_subspaces(self):
        b=q.build_basis(4); ops=q.build_operators(b)
        e,v,_=q.solve_generalized(q.hamiltonian(ops,1.,1.7),b['S'],'cpu')
        old=q.exterior_diagnostics_scaled(b,ops,e,v,1.,1.7)
        groups=q.reported_groups(e,v,[ops['tensor'],[ops['scalar']]])
        for indices,expected in zip(groups,old['level_worst_residuals']):
            actual=o.full_residual(b,v[:,indices],np.diag(e[indices]),1.,1.7)
            self.assertAlmostEqual(actual['norm'],expected,places=10)

    def test_full_action_matches_independent_ladders(self):
        b = q.build_basis(2)
        ops = q.build_operators(b)
        g2, alpha = .7, 1.7
        e, v, _ = q.solve_generalized(q.hamiltonian(ops,g2,alpha), b['S'], 'cpu')
        v = v[:, :3].astype(complex)@np.diag([1.,1j,-1j])
        B = np.diag(e[:3])
        expected = direct_ladder_residual(b,v,B,g2,alpha)
        actual = o.full_residual(b,v,B,g2,alpha)
        np.testing.assert_allclose(actual['gram'], expected, atol=1e-10, rtol=1e-10)
        self.assertAlmostEqual(actual['norm'], math.sqrt(la.eigvalsh(expected)[-1]), places=10)

    def test_operator_norm_and_batch_invariance(self):
        b = q.build_basis(2); ops = q.build_operators(b)
        e,v,_ = q.solve_generalized(q.hamiltonian(ops,1.,1.3),b['S'],'cpu')
        all_r = o.full_residual(b,v[:,:3],np.diag(e[:3]),1.,1.3)
        self.assertGreater(all_r['norm'], .01)
        for i in range(3):
            one = o.full_residual(b,v[:,i:i+1],np.diag(e[i:i+1]),1.,1.3)
            self.assertAlmostEqual(one['gram'][0,0].real, all_r['gram'][i,i].real, places=10)


class WidthTests(unittest.TestCase):
    def test_saved_width_baselines_are_distinct(self):
        root = Path(__file__).resolve().parents[1]
        rows = json.loads((root/'research/helike-quantum-probe-20261123/exploration-edge-20261124/width_residuals_N10.json').read_text())['rows']
        r = o.width_summary(rows)
        self.assertAlmostEqual(r['expanded_max_gain'], 1.1089676170903249, places=12)
        self.assertAlmostEqual(r['minimum_excited_residual'], .3470552370314853, places=12)
        self.assertGreater(r['legacy_ground_gain'], 1.3)
        self.assertEqual(r['legacy_from_alpha'], 2.)
        self.assertEqual(r['legacy_to_alpha'], 2.3)


if __name__ == '__main__':
    unittest.main(verbosity=2)
