"""Manufactured mathematical gates; never fit a physical control in this suite."""
import math
import copy
import unittest
from fractions import Fraction
from flint import arb, ctx
import mpmath as mp
import numpy as np
import helike_cutoff_bounds as cb


class Bounds(unittest.TestCase):
    def setUp(self):
        ctx.prec=128
        mp.mp.dps=90

    def call(self,name,*args,**kwargs):
        try: return getattr(cb,name)(*args,**kwargs)
        except NotImplementedError: self.fail(name+' missing behavioral contract')

    def test_radial_tail_independent_gamma(self):
        for k,rho,decay in ((0,0,1),(0,2,1),(3,4,1),(8,12,15/16)):
            actual=self.call('radial_tail',k,arb(rho),arb(decay))
            expected=mp.gammainc(k+3,mp.mpf(decay)*rho*rho,mp.inf)/(2*mp.mpf(decay)**(k+3))
            self.assertTrue(actual.contains(arb(str(expected))))

    def test_infinite_tail_does_not_underflow(self):
        actual=self.call('radial_tail',0,arb(40))
        self.assertTrue(actual>0)
        expected=mp.gammainc(3,1600,mp.inf)/2
        self.assertTrue(actual.contains(arb(str(expected))))

    def test_outward_endpoint_serialization(self):
        x=arb(1)/3
        for side,relation in (('upper',lambda x:x>=Fraction(1,3)),('lower',lambda x:x<=Fraction(1,3))):
            record=self.call('endpoint',x,side)
            f=Fraction(int(record['mantissa']))*Fraction(2)**record['exponent']
            self.assertTrue(relation(f))
        tiny=self.call('endpoint',(-arb(1600)).exp(),'upper')
        self.assertGreater(int(tiny['mantissa']),0)

    def test_complex_polynomial_majorant_and_normalization(self):
        basis=[(0,)*6,(2,0,0,0,0,0)]
        a,b,norm=self.call('polynomial_bounds',basis,np.array([3.,4j]),arb(2))
        # |h2(x)| <= sqrt(2)*(1+1/(2rho^2))*r^2; h2'=2h1.
        expected=arb(3)/4+4*arb(2).sqrt()*arb(9)/8
        self.assertTrue(a.contains(expected))
        self.assertTrue(b[0].contains(8*arb(2).sqrt()))
        self.assertTrue(norm.contains(25))
        self.assertTrue(all(v==0 for v in b[1:]))

    def test_majorant_bounds_explicit_polynomial(self):
        basis=[(0,)*6,(1,1,0,0,0,0),(3,0,0,0,0,0)]
        a,_,_=self.call('polynomial_bounds',basis,np.array([1+2j,3-1j,-2j]),arb(2))
        for x,y in ((2.,1.),(-3.,4.),(8.,-1.)):
            r=math.hypot(x,y)
            P=1+2j+(3-1j)*2*x*y-2j*(8*x**3-12*x)/math.sqrt(48)
            self.assertTrue(arb(abs(P))<a*arb(r)**3)

    def test_plateau_whole_ball_and_refusal(self):
        p=self.call('plateau',np.zeros(6),np.eye(6)*.01,np.ones(6),np.eye(3))
        self.assertEqual(p['rho'],arb(16))
        # Exact point inside the first radius has D below the cutoff.
        with self.assertRaises(ValueError): cb.plateau(np.zeros(6),np.eye(6)*100,np.ones(6),np.eye(3))

    def test_derivative_density_gaussian_and_inverse(self):
        result=self.call('derivative_bounds',arb(2),[arb(3)]+[arb(0)]*5,np.eye(6)*2,np.ones(6),arb(2))
        self.assertTrue(result[0].contains(arb('2.875')))
        self.assertTrue(result[1].contains(2))
        self.assertTrue(result[2].contains(arb('1.5')))
        self.assertTrue(result[3].contains(1))

    def test_cutoff_constants_keep_mixed_term(self):
        g,l=self.call('cutoff_constants',arb(1),arb(4))
        ga=arb(512).sqrt(); gb=arb(2).sqrt(); la=arb(128); lb=14*arb(3).sqrt()/4
        self.assertTrue(g.contains(8*(ga+gb)))
        self.assertTrue(l.contains(128*ga**2+8*la+128*gb**2+8*lb+128*ga*gb))

    def test_metric_tail_keeps_ordered_terms_and_exponent(self):
        metric=[[{} for j in range(6)] for i in range(6)]
        key=(2,0,0,1,0,0)
        metric[0][0]={key:2}; metric[0][1]={key:1}; metric[1][0]={key:1}
        result=self.call('metric_tail',np.zeros(6),np.eye(6),np.ones(6),[arb(1)]*6,0,arb(2),metric)
        # Sum coefficients=4, exp bound=e^(4/(4/16))=e^16; odd r^3 <= r^4/2.
        expected=2*arb(16).exp()*cb.radial_tail(2,arb(2),arb(15)/16)
        self.assertTrue(result.contains(expected))

    def test_combination_keeps_both_terms_and_norm(self):
        result=self.call('combine',arb(4),arb(9),arb(2),arb(6),arb(100))
        self.assertTrue(result['tail_sq'].contains(arb(4)/100))
        self.assertTrue(result['leakage_sq'].contains(arb(144)/100))
        self.assertTrue(result['cut_norm_sq_lower']<=arb(96)/100)

    def test_status_is_only_partial_and_thresholds_fixed(self):
        yes=self.call('classify',arb('1e-30'),arb('1e-22'),arb(1))
        self.assertTrue(yes['cutoff_tail_gate_passed'])
        self.assertEqual(yes['status'],'UNRESOLVED_INTEGRATION')
        self.assertFalse(yes['certified']); self.assertFalse(yes['full_grid_released'])
        no=cb.classify(arb('1e-30'),arb('1.01e-20'),arb(1))
        self.assertFalse(no['cutoff_tail_gate_passed'])
        self.assertFalse(cb.classify(arb('1.01e-12'),arb('1e-22'),arb(1))['cutoff_tail_gate_passed'])
        self.assertFalse(cb.classify(arb('1e-30'),arb('1e-22'),arb(1,'1e-9'))['cutoff_tail_gate_passed'])

    def test_bad_inputs_refuse(self):
        self.call('radial_tail',0,arb(1))
        for args in ((-1,arb(1),arb(1)),(1,arb(-1),arb(1)),(1,arb(1),arb(0))):
            with self.assertRaises(ValueError): cb.radial_tail(*args)
        for coeff in ([0.],[np.nan],[np.inf]):
            with self.assertRaises(ValueError): cb.polynomial_bounds([(0,)*6],np.array(coeff),arb(2))
        with self.assertRaises(ValueError): cb.polynomial_bounds([(0,)*6]*2,np.ones(2),arb(2))
        with self.assertRaises(ValueError): cb.combine(arb(1),arb(1),arb(1),arb(1),arb(0))

    def test_precision_cannot_silently_change(self):
        with ctx.workprec(53),self.assertRaises(ValueError): cb.radial_tail(0,arb(2))

    def test_symbolic_metric_matches_cartesian_gram_map(self):
        import sympy as sp
        import helike_positive_cone as pc
        import helike_condensate_pt as cp
        q=sp.Matrix(3,3,sp.symbols('q0:9',real=True)); G=q*q.T
        x=sp.Matrix([G[i,j] for i,j in cp.GKEYS]); J=x.jacobian(list(q))
        expected=J*J.T
        actual=sp.Matrix(6,6,lambda i,j:sum(int(cp.C_A[i,j,k])*x[k] for k in range(6)))
        self.assertEqual((actual-expected).applyfunc(sp.expand),sp.zeros(6))
        variables,L,terms=pc._symbolic_metric(); G=L*L.T
        x=sp.Matrix([G[i,j] for i,j in cp.GKEYS])
        J=x.jacobian(variables)*sp.diag(*variables[:3],1,1,1)
        B=sp.zeros(6)
        for i in range(6):
            for j in range(6):
                for key,value in terms[i][j].items():
                    self.assertTrue(float(value).is_integer())
                    B[i,j]+=int(value)*sp.prod(v**p for v,p in zip(variables,key))
        actual=sp.Matrix(6,6,lambda i,j:sum(int(cp.C_A[i,j,k])*x[k] for k in range(6)))
        self.assertEqual((J*B*J.T-actual).applyfunc(sp.cancel),sp.zeros(6))

    def test_derivative_majorants_cover_nonzero_shells(self):
        import torch
        import helike_positive_cone as pc
        import helike_quasimode as qm
        from scipy.optimize import brentq
        chart=pc.ConeChart(np.diag([2.,3.,4.]),np.eye(6))
        p=cb.plateau(chart.u0,chart.S,np.ones(6),chart.G0)
        g,l=cb.cutoff_constants(p['Ds'],p['R'])
        N=6*np.linalg.norm(chart.G0); D=.015*np.linalg.det(chart.G0)
        b=brentq(lambda b:np.sqrt(N*N-2*b*b)*b*b-D,.001,1.,xtol=1e-14)
        overlap=np.diag(np.sqrt([np.sqrt(N*N-2*b*b),b,b]))
        for q in (np.linalg.cholesky(chart.G0)*(.015**(1/6)),np.linalg.cholesky(chart.G0)*np.sqrt(6.),overlap):
            qt=torch.tensor(q,requires_grad=True); f=qm.cutoff_q(qt,chart)
            grad=torch.autograd.grad(f,qt,create_graph=True)[0]
            lap=sum(torch.autograd.grad(grad.ravel()[j],qt,retain_graph=True)[0].ravel()[j] for j in range(9))
            self.assertGreater(float(torch.linalg.vector_norm(grad)),0)
            self.assertTrue(g>arb(float(torch.linalg.vector_norm(grad))))
            self.assertTrue(l>arb(abs(float(lap))))

    def test_complete_bound_is_scale_invariant(self):
        args=(np.zeros(6),np.eye(6)*.01,np.ones(6),np.eye(3),[(0,)*6])
        one=cb.bound_state(*args,np.array([1j])); three=cb.bound_state(*args,np.array([3j]))
        def upper(item):
            e=item['upper']; return arb((int(e['mantissa']),e['exponent']))
        exact_tail=cb.radial_tail(0,arb(16))
        self.assertTrue(upper(one['tail_sq'])>=exact_tail)
        self.assertTrue(upper(one['tail_sq'])<exact_tail*arb('1.000000000000001'))
        self.assertTrue(abs(upper(one['tail_sq'])-upper(three['tail_sq']))<arb('1e-120'))
        self.assertTrue(abs(upper(one['leakage_sq'])-upper(three['leakage_sq']))<arb('1e-90'))
        self.assertFalse(one['certified']); self.assertFalse(one['full_grid_released'])

    def fixture_report(self):
        row=cb.bound_state(np.zeros(6),np.eye(6)*.01,np.ones(6),np.eye(3),[(0,)*6],np.array([1.]))
        return dict(execution='RUNNING',cutoff_tail_gate_passed=False,status='UNRESOLVED_INTEGRATION',
            certified=False,full_grid_released=False,controls=[dict(g2=g,Mq=m,states={
                n:copy.deepcopy(row) for n in ('ground','helike_plus','helike_minus','competitor')})
                for g in (.004,.002) for m in (6,8)])

    def test_finalization_requires_all_controls_and_verified_flags(self):
        import run_helike_cutoff_bounds as run
        good=self.fixture_report()
        try: run.finalize(good)
        except NotImplementedError: self.fail('coverage gate not implemented')
        self.assertTrue(good['cutoff_tail_gate_passed'])
        self.assertEqual(good['execution'],'COMPLETE_CUTOFF_TAIL_BOUNDS')
        self.assertFalse(good['certified']); self.assertFalse(good['full_grid_released'])
        bad=self.fixture_report(); bad['controls'].pop()
        with self.assertRaises(ValueError): run.finalize(bad)
        bad=self.fixture_report(); bad['controls'][0]['states'].pop('competitor')
        with self.assertRaises(ValueError): run.finalize(bad)
        bad=self.fixture_report(); state=bad['controls'][0]['states']['ground']
        state['leakage_sq']=cb.zero_to(arb(1))
        with self.assertRaises(ValueError): run.finalize(bad)

    def test_failure_clears_stale_pass(self):
        import run_helike_cutoff_bounds as run
        report=self.fixture_report(); report.update(cutoff_tail_gate_passed=True,certified=True,full_grid_released=True)
        try: run.failure(report,ValueError('planted invalid evidence'))
        except NotImplementedError: self.fail('failure reset not implemented')
        self.assertFalse(report['cutoff_tail_gate_passed']); self.assertFalse(report['certified'])
        self.assertFalse(report['full_grid_released']); self.assertEqual(report['execution'],'FAILED')
        self.assertIn('planted invalid evidence',report['error'])


if __name__=='__main__': unittest.main()
