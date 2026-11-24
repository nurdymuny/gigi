"""Same-model controls for a domain-preserving geometric trial basis."""
import unittest
import numpy as np
from scipy.special import roots_hermite, eval_hermite, factorial
import helike_berger_model as bm
import helike_condensate_pt as cp
import helike_positive_cone as pc


class Geometry(unittest.TestCase):
    def setUp(self):
        self.G0 = np.diag([2., 3., 4.])
        self.x0 = np.array([2., 3., 4., 0., 0., 0.])
        self.M = np.linalg.cholesky(cp.A_at(self.x0))
        self.chart = pc.ConeChart(self.G0, self.M)

    def test_old_negative_continuation_point_stays_inside(self):
        z = -2*np.linalg.solve(self.M, self.x0)
        self.assertGreater(np.linalg.eigvalsh(self.chart.gram(z)).min(), 0.)
        self.assertGreater(np.diag(self.chart.factor(z)).min(),0.)

    def test_old_negative_kinetic_directions_are_positive(self):
        z = -2*np.linalg.solve(self.M, self.x0)
        self.assertGreater(np.linalg.eigvalsh(self.chart.metric(z)).min(), 0.)

    def test_roundtrip_metric_and_measure(self):
        for z in np.random.default_rng(17).normal(size=(12, 6)):
            G = self.chart.gram(z)
            np.testing.assert_allclose(self.chart.inverse(G), z, atol=1e-10)
            J = self.chart.jacobian(z)
            x = np.array([G[i,j] for i,j in cp.GKEYS])
            inv = np.linalg.inv(J)
            np.testing.assert_allclose(self.chart.metric(z), inv @ cp.A_at(x) @ inv.T, rtol=1e-10, atol=1e-10)
            want = np.linalg.slogdet(J)[1] - .5*np.linalg.slogdet(G)[1]
            self.assertAlmostEqual(self.chart.log_measure(z), want, delta=1e-10)
            self.assertGreater(np.linalg.eigvalsh(self.chart.metric(z)).min(), 0.)

    def test_jacobian_and_measure_gradient_finite_difference(self):
        z = np.array([.12, -.23, .05, .18, -.17, .09])
        eps = 1e-6
        J = np.zeros((6,6)); ell = np.zeros(6)
        for j in range(6):
            d = np.eye(6)[j]*eps
            diff = (self.chart.gram(z+d)-self.chart.gram(z-d))/(2*eps)
            J[:,j] = [diff[a,b] for a,b in cp.GKEYS]
            ell[j] = (self.chart.log_measure(z+d)-self.chart.log_measure(z-d))/(2*eps)
        np.testing.assert_allclose(self.chart.jacobian(z), J, rtol=1e-8, atol=1e-8)
        np.testing.assert_allclose(self.chart.ell, ell, atol=1e-8)

    def test_refuses_bad_geometry_instead_of_projecting(self):
        for G in (-self.G0, np.zeros((3,3)), np.array([[1., 2., 0.],[0.,1.,0.],[0.,0.,1.]])):
            with self.subTest(G=G), self.assertRaises(ValueError): self.chart.inverse(G)
        with self.assertRaises(ValueError): self.chart.gram(np.full(6, np.nan))
        with self.assertRaises(ValueError): self.chart.gram(np.full(6, 1e6))

    def test_metric_and_potential_match_independent_q_space(self):
        s,t,g = 1.15,3.,np.sqrt(.004)
        a,b = bm.mc_sources(s,t)
        metric, potential = pc.laurent_model(s,t,g)
        for z in np.random.default_rng(12).normal(size=(8,6)):
            L = self.chart.factor(z)
            coords = [L[0,0],L[1,1],L[2,2],L[1,0],L[2,0],L[2,1]]
            V = pc.evaluate_terms(potential, coords)
            self.assertAlmostEqual(V, bm.V_sourced(L,g,t,a,b), delta=1e-10)
            Ju = self.chart.jacobian_u(L)
            dq = np.zeros((6,9)); eps = 1e-6
            for j in range(9):
                e = np.eye(9)[j].reshape(3,3)*eps
                dg = ((L+e)@(L+e).T - (L-e)@(L-e).T)/(2*eps)
                dq[:,j] = [dg[i,k] for i,k in cp.GKEYS]
            want = np.linalg.solve(Ju,dq) @ np.linalg.solve(Ju,dq).T
            got = np.array([[pc.evaluate_terms(metric[i][j], coords) for j in range(6)] for i in range(6)])
            np.testing.assert_allclose(got,want,rtol=1e-8,atol=1e-8)


class Moments(unittest.TestCase):
    def test_completed_square_against_unshifted_quadrature(self):
        x,wt = roots_hermite(80)
        for w,a,p in ((.7,.4,3),(1.8,-.6,2),(.9,0.,0)):
            h=np.array([eval_hermite(n,x)/np.sqrt(2.**n*factorial(n)) for n in range(4)])
            want=(h*(wt*np.exp(a*x/np.sqrt(w))*(x/np.sqrt(w))**p)/np.sqrt(np.pi))@h.T
            np.testing.assert_allclose(pc.one_dimensional_moments(w,a,p,3),want,rtol=1e-11,atol=1e-11)

    def test_harmonic_single_and_double_controls(self):
        w=np.array([.7,.9,1.1,1.3,1.7,2.1])
        H=pc.harmonic_matrix(w,2)
        e=np.linalg.eigvalsh(H)
        want=np.sort([np.dot(n,w)+.5*w.sum() for n in pc.states_up_to(6,2)])
        np.testing.assert_allclose(e,want,atol=1e-6,rtol=0)

    def test_nonfinite_and_bad_order_refuse(self):
        for args in ((0.,.1,0,2),(1.,np.nan,0,2),(1.,.1,-1,2)):
            with self.subTest(args=args), self.assertRaises(ValueError): pc.one_dimensional_moments(*args)

    def test_reference_amplitudes_against_direct_pullback(self):
        import itertools
        L=pc.ConeRitz(1.15,3.,np.sqrt(.004),Mq=2)
        rng=np.random.default_rng(21)
        vectors=np.linalg.qr(rng.normal(size=(len(L.basis),2)))[0]
        x,weights=roots_hermite(3)
        want=np.zeros((6,2))
        for ix in itertools.product(range(3),repeat=6):
            z=x[list(ix)]/np.sqrt(L.w)
            G=L.chart.gram(z)
            y=np.linalg.solve(L.chart.M,np.array([G[i,j]-L.chart.G0[i,j] for i,j in cp.GKEYS]))
            determinant=abs(np.linalg.det(np.linalg.solve(L.chart.M,L.chart.jacobian(z))))
            ratio=np.sqrt(determinant)*np.exp(-.5*np.sum(L.w*(y*y-z*z)))
            polynomial=np.array([np.prod([eval_hermite(n[j],x[ix[j]])/np.sqrt(2.**n[j]*factorial(n[j])) for j in range(6)])
                                 for n in L.basis])
            weight=np.prod(weights[list(ix)]/np.sqrt(np.pi))
            want += weight*ratio*np.outer(np.sqrt(2*L.w)*y,polynomial@vectors)
        np.testing.assert_allclose(L.reference_amplitudes(vectors,3),want,rtol=1e-10,atol=1e-10)


class PhysicalControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.controls={g2:pc.ConeRitz(1.15,3.,np.sqrt(g2),Mq=6) for g2 in (.004,.002)}

    def test_same_weak_coupling_coefficients(self):
        vals={}
        for g2,L in self.controls.items():
            r=L.helike_and_competitor()
            wh=np.sqrt(bm.mc_helike_freq2(1.15,3.))
            wc=min(w for w in L.w if np.count_nonzero(abs(L.w-w)<1e-6)==1)
            vals[g2]=[(r['helike']['gap']-wh)/g2,(r['competitor']['gap']-wc)/g2]
            self.assertGreater(min(r[n]['weight'] for n in ('helike','competitor')), .95)
        for k,want in ((0,-.5300),(1,-2.7379)):
            self.assertAlmostEqual(2*vals[.002][k]-vals[.004][k],want,delta=.03)

    def test_mass_and_kinetic_form(self):
        for L in self.controls.values():
            np.testing.assert_allclose(L.mass,np.eye(len(L.basis)),atol=1e-10)
            self.assertGreaterEqual(np.linalg.eigvalsh(L.T).min(),-1e-10*max(1,np.linalg.norm(L.T,2)))
            self.assertLess(np.max(abs(L.H-L.H.T)),1e-10)
            self.assertTrue(np.isfinite(L.H).all())

    def test_nested_ritz_values(self):
        big=self.controls[.004]
        small=pc.ConeRitz(1.15,3.,np.sqrt(.004),Mq=2)
        np.testing.assert_allclose(big.H[:len(small.basis),:len(small.basis)],small.H,atol=1e-10)
        self.assertTrue(np.all(np.linalg.eigvalsh(big.H)[:7] <= np.linalg.eigvalsh(small.H)[:7]+1e-10))

    def test_scope_and_resource_refusal(self):
        self.assertEqual(self.controls[.004].boundary_status,'UNRESOLVED_PHYSICAL_REALIZATION')
        with self.assertRaises(ValueError): pc.ConeRitz(1.15,3.,.1,Mq=7)


if __name__ == '__main__':
    unittest.main(verbosity=2)
