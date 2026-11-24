"""Focused construction gates. No control spectrum is built in this suite."""
import unittest
from types import SimpleNamespace
from unittest.mock import patch
import numpy as np
import torch
from scipy.optimize import brentq
import helike_positive_cone as pc
import helike_condensate_pt as cp
import helike_quasimode as qm


class Construction(unittest.TestCase):
    def call(self, name, *args, **kwargs):
        try:
            return getattr(qm, name)(*args, **kwargs)
        except NotImplementedError:
            self.fail(f'{name} has not implemented its behavioral contract')

    def chart(self):
        G=np.diag([2.,3.,4.])
        M=np.linalg.cholesky(cp.A_at(np.array([2.,3.,4.,0.,0.,0.])))
        return pc.ConeChart(G,M)

    def test_step_endpoints_and_derivatives(self):
        h,d,dd=self.call('smooth_step',np.array([0.,1.,1.5,2.,3.]))
        np.testing.assert_array_equal(h,[0.,0.,.5,1.,1.])
        self.assertEqual(d[2],2.)
        self.assertEqual(dd[2],0.)
        np.testing.assert_array_equal(d[[0,1,3,4]],0.)
        with self.assertRaises(ValueError): qm.smooth_step(np.array([np.nan]))

    def test_cutoff_is_invariant_and_shells_are_nontrivial(self):
        chart=self.chart()
        q=np.linalg.cholesky(chart.G0)*(.015**(1/6))
        value=self.call('cutoff_q',torch.tensor(q),chart).item()
        self.assertAlmostEqual(value,.5,places=12)
        R=np.array([[1.,0.,0.],[0.,.8,-.6],[0.,.6,.8]])
        self.assertAlmostEqual(qm.cutoff_q(torch.tensor(R@q),chart).item(),value,places=12)
        q=np.linalg.cholesky(chart.G0)*np.sqrt(6.)
        self.assertAlmostEqual(qm.cutoff_q(torch.tensor(q),chart).item(),.5,places=12)

    def test_cutoff_derivatives_match_cartesian_in_both_shells(self):
        chart=self.chart()
        for scale in (.015**(1/6),np.sqrt(6.)):
            q=np.linalg.cholesky(chart.G0)*scale
            z=chart.inverse(q@q.T)
            geom=self.call('geometry',chart,z[None,:],1.15,3.,np.sqrt(.004))
            qt=torch.tensor(q,requires_grad=True)
            value=qm.cutoff_q(qt,chart)
            grad=torch.autograd.grad(value,qt,create_graph=True)[0]
            lap=sum(torch.autograd.grad(grad.ravel()[j],qt,retain_graph=True)[0].ravel()[j] for j in range(9))
            self.assertGreater(abs(lap.item()),1e-6)
            self.assertAlmostEqual(geom['eta'][0],value.item(),places=11)
            np.testing.assert_allclose(geom['lap_eta'][0],lap.item(),rtol=1e-10,atol=1e-10)

    def test_overlapping_shells_keep_mixed_laplacian_term(self):
        chart=self.chart(); N=6*np.linalg.norm(chart.G0); D=.015*np.linalg.det(chart.G0)
        b=brentq(lambda b:np.sqrt(N*N-2*b*b)*b*b-D,.001,1.,xtol=1e-14)
        q=np.diag(np.sqrt([np.sqrt(N*N-2*b*b),b,b]))
        geo=self.call('geometry',chart,chart.inverse(q@q.T)[None,:],1.15,3.,np.sqrt(.004))
        qt=torch.tensor(q,requires_grad=True); f=qm.cutoff_q(qt,chart)
        grad=torch.autograd.grad(f,qt,create_graph=True)[0]
        lap=sum(torch.autograd.grad(grad.ravel()[j],qt,retain_graph=True)[0].ravel()[j] for j in range(9))
        self.assertAlmostEqual(f.item(),.25,places=10)
        np.testing.assert_allclose(geo['lap_eta'][0],lap.item(),rtol=1e-10,atol=1e-10)

    def test_full_operator_matches_independent_cartesian_differentiation(self):
        chart=self.chart(); basis=pc.states_up_to(6,2); w=np.arange(1.,7.)/2
        coeff=np.zeros(len(basis),dtype=complex); coeff[0]=1/np.sqrt(2)
        coeff[basis.index((0,0,0,1,0,0))]=1j/np.sqrt(2)
        for scale in (1.,.015**(1/6),np.sqrt(6.)):
            q=np.linalg.cholesky(chart.G0)*scale
            for cutoff in (False,True):
                actual=self.call('point_action',q,chart,basis,w,coeff,1.15,3.,np.sqrt(.004),cutoff)
                oracle=qm.cartesian_action(q,chart,basis,w,coeff,1.15,3.,np.sqrt(.004),cutoff)
                np.testing.assert_allclose(actual,oracle,atol=1e-10,rtol=1e-9)

    def test_tensor_operator_matches_pointwise_action(self):
        chart=self.chart(); basis=pc.states_up_to(6,2); w=np.arange(1.,7.)/2
        coeff=np.zeros((len(basis),1)); coeff[0,0]=1.
        solver=SimpleNamespace(chart=chart,basis=basis,w=w,s=1.15,t=3.,g=np.sqrt(.004))
        with patch.object(qm,'ORDERS',(3,4)):
            samples=qm.sample_grid(solver,coeff,3)
        z,weights=qm.grid_coordinates(w,3)
        for i in (1,364,700):
            q=chart.factor(z[i]); value=qm.point_action(q,chart,basis,w,coeff[:,0],solver.s,solver.t,solver.g,False)
            f=np.exp(.25*np.log(w/np.pi).sum()-.5*np.sum(w*z[i]**2)-.5*chart.log_measure(z[i]))
            np.testing.assert_allclose(samples['HP'][i,0]*f,value,atol=1e-10,rtol=1e-9)

    def test_cutoff_state_has_zero_action_off_positive_support(self):
        chart=self.chart(); basis=pc.states_up_to(6,1); coeff=np.zeros(len(basis)); coeff[0]=1.
        q=np.linalg.cholesky(chart.G0); q[:,2]*=-1
        for point in (q,np.zeros((3,3)),np.diag([1.,1.,0.])):
            for action in (qm.point_action,qm.cartesian_action):
                try: value=action(point,chart,basis,np.ones(6),coeff,1.15,3.,np.sqrt(.004),True)
                except Exception as exc: self.fail(f'zero extension must not enter singular chart: {exc}')
                self.assertEqual(value,0j)

    def test_generator_sign_and_metric_on_known_covariant(self):
        chart=self.chart(); z=np.array([[.1,-.1,.05,.07,-.09,.04]])
        geom=self.call('geometry',chart,z,1.15,3.,np.sqrt(.004))
        G=chart.gram(z[0]); J=chart.jacobian(z[0])
        gradient=(J[1]-J[2])/2+1j*J[5]
        w=(G[1,1]-G[2,2])/2+1j*G[1,2]
        self.assertAlmostEqual(abs(-1j*(geom['velocity'][0]@gradient)-2*w),0.,places=11)
        np.testing.assert_allclose(geom['B'][0],chart.metric(z[0]),atol=1e-11)

    def test_hermite_grid_norm_and_lowering(self):
        basis=pc.states_up_to(6,2); coeff=np.zeros((len(basis),1))
        coeff[basis.index((2,0,0,0,0,0)),0]=1.
        w=np.ones(6)
        values=self.call('polynomial_grid',coeff,basis,w,4)
        deriv=qm.polynomial_grid(coeff,basis,w,4,(0,))
        _,weights=qm.grid_coordinates(w,4)
        self.assertAlmostEqual(float(weights@(values[:,0]**2)),1.,places=12)
        self.assertAlmostEqual(float(weights@(deriv[:,0]**2)),4.,places=12)

    def test_projector_bound_and_refusals(self):
        # Unit vector sqrt(.99)|2> + .1|3>: exact retained norm sqrt(.99).
        bound=self.call('projection_bound',0.,.1,.02,.001)
        self.assertGreaterEqual(bound,.021/np.sqrt(.99))
        for values in ((1.,0.,1.,0.),(0.,1.,1.,0.),(-.1,0.,1.,0.),(0.,0.,np.nan,0.)):
            with self.subTest(values=values),self.assertRaises(ValueError): qm.projection_bound(*values)

    def test_commutator_keeps_both_contributions(self):
        out=self.call('cutoff_action',np.array([2.+1j]),np.array([3.-2j]),np.array([.5]),
                      np.array([4.+2j]),np.array([6.]))
        np.testing.assert_allclose(out,.5*(3.-2j)-(4.+2j)-3*(2.+1j))

    def test_complex_norm_and_nonfinite_refusals(self):
        result=self.call('square_integral',np.array([[1.],[1j]]),np.array([.5,.5]))
        np.testing.assert_allclose(result,[1.])
        for x in (np.array([[np.nan]]),np.array([[np.inf]])):
            with self.assertRaises(ValueError): qm.square_integral(x,np.array([1.]))

    def test_numerical_agreement_never_certifies(self):
        status=self.call('integration_status',{'norm':1.,'residual_sq':.01},{'norm':1.,'residual_sq':.01})
        self.assertEqual(status['agreement'],'NUMERICAL_AGREEMENT')
        self.assertEqual(status['status'],'UNRESOLVED_INTEGRATION')
        self.assertFalse(status['certified'])
        with self.assertRaises(ValueError): qm.integration_status({'norm':np.nan},{'norm':np.nan})
        with self.assertRaises(ValueError): qm.integration_status({'norm':1.},{})


if __name__=='__main__': unittest.main()
