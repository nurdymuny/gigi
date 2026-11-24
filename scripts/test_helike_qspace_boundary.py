"""Local boundary gates from the original Cartesian SO(3)-singlet model."""
import unittest
import json
from pathlib import Path
import numpy as np
import sympy as sp
from scipy.spatial.transform import Rotation

import helike_berger_model as bm
import helike_condensate_pt as cp
import helike_positive_cone as pc
import helike_qspace_boundary as qb


POINT = np.array([np.log(1.2),np.log(.9),0.,.25,-.35,.4])


def probes(widths=None):
    """Independent Cartesian differentiation and chart jets for regular test states."""
    qv=sp.symbols('q0:9',real=True)
    q=sp.Matrix(3,3,qv); G=q*q.T; d=q.det()
    x,L,_,_,_=qb.symbolic_chart()
    substitution=dict(zip(qv,list(L)))
    def du(expr,i):
        return sp.diff(expr,x[i])*(x[i] if i<2 else 1)
    out=[]
    widths=np.full(3,.1) if widths is None else np.asarray(widths)
    envelope=sp.exp(-sum(sp.Float(widths[i])*q[i,j]**2 for i in range(3) for j in range(3))/2)
    for polynomial in (1,d,G[0,1],G[1,1]-G[2,2],(1+d)*G[0,0]):
        f=polynomial*envelope
        cart=sp.lambdify(qv,(f,sum(sp.diff(f,v,2) for v in qv)),'numpy')
        fu=f.subs(substitution,simultaneous=True)
        grad=[du(fu,i) for i in range(6)]
        hess=[[du(grad[i],j) for j in range(6)] for i in range(6)]
        jets=sp.lambdify(x,(fu,grad,hess),'numpy')
        out.append((cart,jets))
    return out


class Coordinates(unittest.TestCase):
    def test_both_orientations_and_regular_rank_two(self):
        for w in (-.3,0.,.3):
            u=POINT.copy(); u[2]=w
            q=qb.factor(u)
            np.testing.assert_allclose(qb.coordinates(q),u,atol=1e-12,rtol=0)
            R=Rotation.from_rotvec([.4,-.7,.2]).as_matrix()
            np.testing.assert_allclose(qb.coordinates(q@R.T),u,atol=1e-12,rtol=0)
            self.assertAlmostEqual(np.linalg.det(q),1.2*.9*w,delta=1e-12)
        p=POINT.copy(); p[2]=.3
        m=p.copy(); m[2]=-.3
        np.testing.assert_allclose(qb.factor(p)@qb.factor(p).T,qb.factor(m)@qb.factor(m).T,atol=1e-12,rtol=0)

    def test_metric_from_independent_cartesian_derivatives(self):
        for w in (-.3,0.,.3):
            u=POINT.copy(); u[2]=w; q=qb.factor(u)
            grad=np.zeros((6,9)); step=1e-6
            for k in range(9):
                delta=np.eye(9)[k].reshape(3,3)*step
                grad[:,k]=(qb.coordinates(q+delta)-qb.coordinates(q-delta))/(2*step)
            np.testing.assert_allclose(qb.metric(u),grad@grad.T,atol=1e-8,rtol=1e-8)
            self.assertGreater(np.linalg.eigvalsh(qb.metric(u)).min(),0.)

    def test_interface_density_and_normal_metric(self):
        r0,r1,w,a,b,c=qb.chart_values(POINT)
        self.assertAlmostEqual(qb.density(POINT),8*r0**3*r1**2,delta=1e-10)
        B=qb.metric(POINT)
        self.assertAlmostEqual(B[2,2],1+c*c/r1**2+(a*c-b*r1)**2/(r0*r1)**2,delta=1e-10)
        np.testing.assert_allclose(np.delete(B[2],2),0.,atol=1e-10,rtol=0)

    def test_agreement_with_existing_positive_chart(self):
        G0=np.diag([2.,3.,4.]); M=np.linalg.cholesky(cp.A_at(np.array([2.,3.,4.,0.,0.,0.])))
        chart=pc.ConeChart(G0,M)
        for z in np.random.default_rng(9).normal(scale=.2,size=(5,6)):
            L=chart.factor(z); u=qb.coordinates(L)
            J=np.diag([1.,1.,L[2,2],1.,1.,1.])@chart.S
            np.testing.assert_allclose(qb.metric(u),J@chart.metric(z)@J.T,atol=1e-10,rtol=1e-10)
            self.assertAlmostEqual(qb.density(u),np.exp(chart.log_measure(z))/abs(np.linalg.det(J)),delta=1e-10)

    def test_bad_patch_and_nonfinite_refuse(self):
        for q in (np.zeros((3,3)),np.diag([1.,0.,1.]),np.full((3,3),np.nan),np.ones((2,2))):
            with self.subTest(q=q),self.assertRaises(ValueError): qb.coordinates(q)
        for u in (np.full(6,np.nan),np.full(6,np.inf),np.ones(5),np.array([1000.,0.,0.,0.,0.,0.])):
            with self.subTest(u=u),self.assertRaises(ValueError): qb.factor(u)


class Operator(unittest.TestCase):
    @classmethod
    def setUpClass(cls): cls.probes=probes()

    def test_reduced_laplacian_matches_cartesian_at_and_across_interface(self):
        for w in (-.3,0.,.3):
            u=POINT.copy(); u[2]=w; q=qb.factor(u)
            for cart,jets in self.probes:
                value,lap=cart(*q.ravel()); fu,grad,hess=jets(*qb.chart_values(u))
                self.assertAlmostEqual(value,fu,delta=1e-10)
                np.testing.assert_allclose(qb.laplacian(u,grad,hess),lap,atol=1e-9,rtol=1e-9)
                for g2 in (.004,.002):
                    a,b=bm.mc_sources(1.15,3.)
                    want=-.5*lap+bm.V_sourced(q,np.sqrt(g2),3.,a,b)*value
                    got=-.5*qb.laplacian(u,grad,hess)+qb.potential(u,1.15,3.,np.sqrt(g2))*fu
                    np.testing.assert_allclose(got,want,atol=1e-9,rtol=1e-9)

    def test_signed_flux_cubic_at_same_control_backgrounds(self):
        for g2 in (.004,.002):
            g=np.sqrt(g2); _,_,q0=bm.mc_background(1.15,3.,g)
            up=qb.coordinates(q0); up[2]=.2
            um=up.copy(); um[2]=-.2
            r0,r1,w,*_=qb.chart_values(up)
            want=-2*g*bm.omega(3.).sum()*r0*r1*w
            self.assertNotEqual(want,0.)
            self.assertAlmostEqual(qb.potential(up,1.15,3.,g)-qb.potential(um,1.15,3.,g),want,delta=1e-10)

    def test_gram_square_root_does_not_force_zero_derivative(self):
        r0,r1,*_=qb.chart_values(POINT)
        slopes=[]
        for h in (1e-4,1e-8):
            step=h*1e-4
            def determinant(hh):
                u=POINT.copy(); u[2]=np.sqrt(hh)
                return np.linalg.det(qb.factor(u))
            derivative=(determinant(h+step)-determinant(h-step))/(2*step)
            slopes.append(derivative)
            self.assertAlmostEqual(2*np.sqrt(h)*derivative,r0*r1,delta=1e-8)
        self.assertGreater(slopes[1],90*slopes[0])

    def test_operator_bad_jets_refuse(self):
        with self.assertRaises(ValueError): qb.laplacian(POINT,np.full(6,np.nan),np.eye(6))
        with self.assertRaises(ValueError): qb.laplacian(POINT,np.ones(5),np.eye(6))
        with self.assertRaises(ValueError): qb.laplacian(POINT,np.ones(6),np.ones((5,5)))


class Matching(unittest.TestCase):
    def test_regular_cartesian_probes_match_at_same_controls(self):
        for g2 in (.004,.002):
            _,_,background=bm.mc_background(1.15,3.,np.sqrt(g2))
            u=qb.coordinates(background); u[2]=0.
            saw_nonzero_pair=False
            for cart,jets in probes(1/np.diag(background)**2):
                value,grad,hess=jets(*qb.chart_values(u))
                grad=np.array(grad); minus=grad.copy(); minus[2]*=-1
                self.assertTrue(qb.matching_report(u,value,value,grad,minus)['matches'])
                saw_nonzero_pair |= abs(value)>1e-8 and abs(grad[2])>1e-8
                direct_value,direct_lap=cart(*qb.factor(u).ravel())
                np.testing.assert_allclose(value,direct_value,atol=1e-9,rtol=1e-9)
                np.testing.assert_allclose(qb.laplacian(u,grad,hess),direct_lap,atol=1e-9,rtol=1e-9)
            self.assertTrue(saw_nonzero_pair)

    def test_nonzero_value_and_derivative_are_allowed(self):
        plus=np.array([.1,.2,1.3,.3,.4,.5]); minus=plus.copy(); minus[2]*=-1
        r=qb.matching_report(POINT,2.,2.,plus,minus)
        self.assertTrue(r['matches'])
        self.assertEqual(r['value_jump'],0.)
        self.assertLess(r['conormal_sum'],1e-10)
        self.assertEqual(r['scope'],'local_rank_two_matching_only')
        self.assertFalse(r['spectral_certified'])

    def test_complex_current_crosses_not_reflects(self):
        # psi=(1+i*d) exp(-tr(G)/20) is a smooth global Cartesian singlet.
        r0,r1,_,a,b,c=qb.chart_values(POINT)
        envelope=np.exp(-(r0*r0+r1*r1+a*a+b*b+c*c)/20)
        value=envelope; plus=np.zeros(6,complex); plus[2]=1j*r0*r1*envelope
        minus=plus.copy(); minus[2]*=-1
        r=qb.matching_report(POINT,value,value,plus,minus)
        self.assertTrue(r['matches'])
        current=qb.density(POINT)*np.imag(np.conjugate(value)*(qb.metric(POINT)[2]@plus))
        self.assertGreater(current,0.)

    def test_jump_rejected_even_when_fluxes_match(self):
        r=qb.matching_report(POINT,1.,1.1,np.zeros(6),np.zeros(6))
        self.assertFalse(r['matches'])

    def test_cusp_and_wrong_half_normal_sign_rejected(self):
        slope=np.eye(6)[2]
        r=qb.matching_report(POINT,1.,1.,slope,slope)
        self.assertFalse(r['matches'])
        self.assertGreater(r['conormal_sum'],1.)

    def test_finite_input_and_exact_interface_required(self):
        for value in (np.nan,np.inf,1j*np.inf):
            with self.subTest(value=value),self.assertRaises(ValueError):
                qb.matching_report(POINT,value,value,np.zeros(6),np.zeros(6))
        for grad in (np.zeros(5),np.full(6,np.nan)):
            with self.subTest(grad=grad),self.assertRaises(ValueError):
                qb.matching_report(POINT,1.,1.,grad,grad)
        p=POINT.copy(); p[2]=1e-8
        with self.assertRaises(ValueError): qb.matching_report(p,1.,1.,np.zeros(6),np.zeros(6))

    def test_no_universal_dirichlet_neumann_or_robin_wall(self):
        # Independent jets A and d*C of regular Cartesian functions.
        reports=[]
        for value,derivative in ((1.,0.),(0.,1.),(1.,2.)):
            plus=np.eye(6)[2]*derivative; minus=-plus
            reports.append(qb.matching_report(POINT,value,value,plus,minus))
            self.assertTrue(reports[-1]['matches'])
        self.assertEqual(reports[0]['positive_conormal'],[0.,0.])
        self.assertGreater(reports[2]['positive_conormal'][0],1.)


class SavedBoundary(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.folder=Path(__file__).resolve().parent.parent/'research/qspace-boundary-20261124/controls-ffc54419'
        cls.receipt=json.loads((cls.folder/'receipt.json').read_text())

    def test_source_snapshots_and_local_only_status(self):
        from run_helike_flux_background import sha
        r=self.receipt
        self.assertEqual(r['status'],'PASS_LOCAL_MATCHING')
        self.assertFalse(r['source_dirty'])
        self.assertFalse(r['physical_spectrum_certified'])
        self.assertFalse(r['registered_grid_released'])
        self.assertEqual([row['g2'] for row in r['controls']],[.004,.002])
        for path,digest in r['sources'].items():
            self.assertEqual(sha(self.folder/'sources'/path),digest)
        for row in r['controls']:
            self.assertEqual(row['s'],1.15); self.assertEqual(row['t'],3.)
            self.assertEqual(len(row['checks']),15)
            for check in row['checks']:
                a,b=check['cartesian_H'],check['reduced_H']
                self.assertTrue(np.isfinite([a,b]).all())
                self.assertLessEqual(abs(a-b),1e-9*(1+abs(a)))
                self.assertAlmostEqual(abs(a-b),check['H_error'],delta=1e-12)

    def test_replay_against_independent_cartesian_oracle(self):
        from run_helike_qspace_boundary import measure_control
        for row in self.receipt['controls']:
            replay=measure_control(row['g2'])
            self.assertEqual(len(replay['checks']),len(row['checks']))
            for actual,saved in zip(replay['checks'],row['checks']):
                self.assertEqual((actual['w'],actual['probe']),(saved['w'],saved['probe']))
                for key in ('value','cartesian_laplacian','reduced_laplacian','cartesian_H','reduced_H'):
                    np.testing.assert_allclose(actual[key],saved[key],atol=1e-9,rtol=1e-9)
                if actual['w']==0.:
                    self.assertTrue(actual['matching']['matches'])
                    self.assertTrue(saved['matching']['matches'])


if __name__=='__main__': unittest.main(verbosity=2)
