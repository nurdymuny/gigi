"""Focused behavior gates: no new physical eigensolve."""
import functools
import unittest
import tempfile
import shutil
import json
from pathlib import Path
import numpy as np
from scipy.special import roots_hermite
import helike_residual_moments as rm
import helike_positive_cone as pc
import helike_condensate_pt as cp
import helike_quasimode as qm
import run_helike_residual_moments as runner


def behavior(test):
    @functools.wraps(test)
    def run(self):
        try: test(self)
        except (NotImplementedError,AttributeError) as exc:
            self.fail(f'missing required behavior: {exc}')
    return run


class Moments(unittest.TestCase):
    def one(self,space,occupation=0,amplitude=1.):
        C=np.zeros((len(space.basis),1),complex)
        C[space.index[(occupation,)+(0,)*(len(space.w)-1)],0]=amplitude
        return C

    @behavior
    def test_high_degree_alias_is_recovered(self):
        sp=rm.Space([1.],10); C=self.one(sp,10)
        x,w=roots_hermite(10)
        old=np.sum(w*pc.normalized_hermites(x,10)[10]**2)/np.sqrt(np.pi)
        self.assertLess(old,1e-20)
        self.assertAlmostEqual(rm.norm_squared(sp,[(np.zeros(1),C)])['value'][0],1.,places=12)

    @behavior
    def test_tensor_replay_on_known_alias(self):
        sp=rm.Space([1.],10); groups=[(np.zeros(1),self.one(sp,10))]
        try: old=rm.tensor_norm(sp,groups,10); fine=rm.tensor_norm(sp,groups,12)
        except ValueError as exc: self.fail(f'tensor polynomial replay missing: {exc}')
        self.assertLess(old[0],1e-20)
        self.assertAlmostEqual(fine[0],1.,places=12)

    @behavior
    def test_translation_coefficients(self):
        sp=rm.Space([2.],3); C=self.one(sp,2); mu=np.array([.3])
        expected=C.copy(); expected[sp.index[(1,)],0]=2*np.sqrt(2)*.3
        expected[sp.index[(0,)],0]=np.sqrt(2)*2*.3**2
        np.testing.assert_allclose(sp.shift(C,mu),expected,rtol=1e-13,atol=1e-13)
        np.testing.assert_allclose(sp.shift(C,mu,oracle=True),expected,rtol=1e-13,atol=1e-13)

    @behavior
    def test_completed_square_prefactor(self):
        sp=rm.Space([1.4],0); C=self.one(sp)
        self.assertAlmostEqual(rm.norm_squared(sp,[(np.array([.7]),C)])['value'][0],np.exp(.7**2/1.4),places=12)

    @behavior
    def test_complex_norm_and_cross_terms(self):
        sp=rm.Space([1.],1)
        C=self.one(sp,0,1j); D=self.one(sp,0,2+1j)
        value=rm.norm_squared(sp,[(np.zeros(1),C),(np.zeros(1),D)])['value'][0]
        self.assertAlmostEqual(value,8.,places=12)

    @behavior
    def test_independent_unshifted_quadrature(self):
        sp=rm.Space([1.4],5); C=self.one(sp,5,.2+.3j)+self.one(sp,0)
        D=self.one(sp,2,-.4j)
        groups=[(np.array([.7]),C),(np.array([-.4]),D)]
        x,weights=roots_hermite(80); z=x[:,None]/np.sqrt(sp.w)
        value=rm.evaluate(sp,groups,z)[:,0]
        expected=(weights@abs(value)**2)/np.sqrt(np.pi)
        a=rm.norm_squared(sp,groups); b=rm.norm_squared(sp,groups,oracle=True)
        np.testing.assert_allclose(a['value'],[expected],rtol=1e-12,atol=1e-12)
        np.testing.assert_allclose(b['value'],[expected],rtol=1e-12,atol=1e-12)

    @behavior
    def test_operator_matches_cartesian_and_reduced(self):
        G=np.diag([2.,3.,4.]); M=np.linalg.cholesky(cp.A_at(np.array([2.,3.,4.,0.,0.,0.])))
        chart=pc.ConeChart(G,M); w=np.arange(1.,7.)/2; basis=pc.states_up_to(6,2)
        C=np.zeros((len(basis),1),complex); C[0]=.8; C[basis.index((0,0,0,1,0,0))]=.6j
        E=np.array([.37])
        space,groups=rm.build_residual(chart,basis,w,C,E,1.15,3.,np.sqrt(.004))
        for z in (np.zeros(6),np.array([.1,-.1,.05,.07,-.09,.04])):
            q=chart.factor(z)
            psi=complex(qm.physical_z(__import__('torch').tensor(z),chart,basis,w,C[:,0]).item())
            f=np.exp(.25*np.log(w/np.pi).sum()-.5*(w*z*z).sum()-.5*chart.log_measure(z))
            actual=rm.evaluate(space,groups,z[None,:])[0,0]*f
            oracle=qm.cartesian_action(q,chart,basis,w,C[:,0],1.15,3.,np.sqrt(.004),False)-E[0]*psi
            reduced=qm.point_action(q,chart,basis,w,C[:,0],1.15,3.,np.sqrt(.004),False)-E[0]*psi
            np.testing.assert_allclose(actual,oracle,rtol=1e-9,atol=1e-10)
            np.testing.assert_allclose(actual,reduced,rtol=1e-9,atol=1e-10)

    @behavior
    def test_derivative_keeps_raising_part(self):
        sp=rm.Space([2.],2); C=self.one(sp)
        expected=self.one(sp,1,-1.)
        np.testing.assert_allclose(sp.op(C,0,'physical'),expected,atol=1e-14)

    @behavior
    def test_nonzero_truncation_refuses(self):
        sp=rm.Space([1.],2); C=self.one(sp,2,1e-25)
        with self.assertRaises(ValueError): sp.op(C,0,'coordinate')

    @behavior
    def test_invalid_inputs_refuse(self):
        with self.assertRaises(ValueError): rm.Space([0.],2)
        with self.assertRaises(ValueError): rm.Space([np.nan],2)
        sp=rm.Space([1.],2); C=self.one(sp)
        with self.assertRaises(ValueError): rm.norm_squared(sp,[(np.array([np.inf]),C)])
        with self.assertRaises(ValueError): rm.norm_squared(sp,[(np.array([100.]),C)])
        C[0]=np.nan
        with self.assertRaises(ValueError): rm.norm_squared(sp,[(np.zeros(1),C)])

    @behavior
    def test_numerical_moments_never_certify(self):
        result=rm.numerical_result([.1],[.1])
        self.assertTrue(result['agreement'])
        self.assertFalse(result['certified'])
        self.assertFalse(result['full_grid_released'])
        self.assertEqual(result['status'],'UNRESOLVED_INTEGRATION')
        self.assertFalse(rm.numerical_result([.1],[.11])['agreement'])

    @behavior
    def test_saved_control_provenance(self):
        self.assertEqual(len(runner.load_controls(runner.OLD)['controls']),4)

    @behavior
    def test_bad_state_cannot_replay(self):
        with tempfile.TemporaryDirectory() as tmp:
            dest=Path(tmp)/'states'; shutil.copytree(runner.OLD,dest)
            state=next(dest.glob('*.npz')); state.write_bytes(state.read_bytes()+b'corrupt')
            with self.assertRaises(ValueError): runner.load_controls(dest)

    @behavior
    def test_missing_control_cannot_replay(self):
        with tempfile.TemporaryDirectory() as tmp:
            dest=Path(tmp)/'states'; shutil.copytree(runner.OLD,dest)
            path=dest/'receipt.json'; receipt=json.loads(path.read_text()); receipt['controls'].pop()
            path.write_text(json.dumps(receipt))
            with self.assertRaises(ValueError): runner.load_controls(dest)


if __name__=='__main__': unittest.main()
