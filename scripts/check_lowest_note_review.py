"""Bounded algebra, receipt replay and document checks. Not a Yang-Mills proof verifier."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess
import unittest

import numpy as np
import sympy as sp

ROOT = Path(__file__).resolve().parents[1]
PAPER = ROOT / 'docs/papers/ym-theory-statement'


def section(name):
    return (PAPER / 'sections' / name).read_text(encoding='utf-8')


def receipt(name):
    return json.loads((ROOT / name).read_text(encoding='utf-8'))


def threshold_replay():
    data = receipt('research/gigi-m3-20261127/result.json')['background']
    h, c = data['helike_mode'], data['competitor_mode']
    d0 = c['omega'] - h['omega']
    potential = c['H_zeta']/c['omega'] - h['H_zeta']/h['omega']
    x = sp.symbols('s', real=True)
    a = 5*x
    b = -sp.sqrt(2)*x*(sp.sqrt(3)+2*(1-x)/sp.sqrt(3))
    c0 = 2*x*x
    lowest_c = (a+c0-sp.sqrt((a-c0)**2+4*b*b))/2
    h0 = 2*x*(sp.Rational(5,3)-x)
    separation = sp.sqrt(lowest_c)-sp.sqrt(h0)
    root = 5*(sp.sqrt(11)-sp.sqrt(3))/(4*sp.sqrt(3))
    slope = float(sp.diff(separation,x).subs(x,root))
    return dict(harmonic_separation=d0, potential_coefficient=potential,
                anharmonic_coefficient=-2.208, harmonic_root=float(root), harmonic_slope=slope,
                kinetic_thresholds={str(g):d0/g-2.208+potential for g in (.002,.004)},
                frozen_control_switches=[dict(g2=g, estimate=float(root)+2.208*g/slope,
                                             registered=registered,
                                             difference=float(root)+2.208*g/slope-registered)
                                        for g,registered in ((.03,1.1674),(.05,1.1809),(.07,1.1931))])


class Algebra(unittest.TestCase):
    def test_kinetic_inverse_metric_sign_and_factor(self):
        eps, mass, omega = sp.symbols('eps mass omega', positive=True)
        metric = 1/(1-2*eps*mass)
        frequency = omega*sp.sqrt(1-2*eps*mass)
        self.assertEqual(sp.diff(metric,eps).subs(eps,0),2*mass)
        self.assertEqual(sp.diff(frequency,eps).subs(eps,0),-omega*mass)

    def test_linear_ordering_includes_negative_zero_and_positive_denominator(self):
        d0, eps = sp.Rational(1655,100000), sp.Rational(2,1000)
        loss = sp.Rational(950,1000)
        threshold = d0/eps-loss
        for kinetic in (-2,-loss,0,threshold-sp.Rational(1,100),threshold,threshold+sp.Rational(1,100)):
            with self.subTest(kinetic=kinetic):
                d = d0-eps*(loss+kinetic)
                self.assertEqual(bool(d>0),bool(kinetic<threshold))
                self.assertEqual(bool(d<0),bool(kinetic>threshold))
        # The old quotient condition reverses the answer when its denominator is negative.
        self.assertGreater(d0-eps*(loss-2),0)
        self.assertFalse(eps<d0/(loss-2))
        self.assertEqual(d0-eps*(loss+threshold),0)

    def test_born_huang_local_power_count_and_common_offset(self):
        g,q,m = sp.symbols('g q m', real=True)
        phi = sp.Function('phi')
        scalar = g**2*phi(2*g*q)
        self.assertEqual(sp.diff(scalar,q,2),4*g**4*sp.diff(phi(m),m,2).subs(m,2*g*q))
        local = sp.diag(0,2,3)+g**2*sp.eye(3)
        self.assertEqual(sp.expand(local[1,1]-local[0,0]),2)
        # Distinct background offsets need not cancel in a comparison between wells.
        self.assertEqual(sp.expand((2+3*g**2)-g**2),2+2*g**2)

    def test_switch_derivative_uses_coefficients_at_the_root(self):
        eps = sp.symbols('eps')
        root, reference = sp.Rational(1), sp.Rational(115,100)
        # D0=2(s-1), a(s)+p(s)-kappa(s)=3s. This has an exact perturbed root.
        exact_root = 2/(2+3*eps)
        derivative = sp.diff(exact_root,eps).subs(eps,0)
        self.assertEqual(derivative,-3*root/2)
        self.assertNotEqual(derivative,-3*reference/2)

    def test_equilibrium_motion_can_contribute_at_the_same_order(self):
        x,g = sp.symbols('x g', real=True)
        potential = x*x/2+g*x**3/3+g*x
        # The fast linear force moves the minimum by -g at leading order.
        moved_stiffness = sp.diff(potential,x,2).subs(x,-g)
        self.assertEqual(moved_stiffness,1-2*g**2)
        self.assertEqual(sp.diff(potential,x,2).subs(x,0),1)

    def test_wilson_cost_is_half_squared_quaternion_chord(self):
        for angle in (0., .2, np.pi / 2, np.pi):
            u = np.array([np.cos(angle), np.sin(angle), 0., 0.])
            self.assertAlmostEqual(1 - u[0], .5 * np.sum((u - [1, 0, 0, 0]) ** 2))

    def test_path_maximum_is_not_full_berger_saddle(self):
        x, y, z = sp.symbols('x y z', real=True)
        for t, expected in [(sp.Integer(1), sp.zeros(3, 1)),
                            (sp.Integer(3), sp.Matrix([sp.sqrt(3)/6, -sp.Rational(1, 12), -sp.Rational(1, 12)]))]:
            f = sp.Matrix([sp.sqrt(t)*x-y*z, y/sp.sqrt(t)-x*z, z/sp.sqrt(t)-x*y])
            v = (f.dot(f))/2
            point = {x: 1/(2*sp.sqrt(t)), y: sp.Rational(1, 2), z: sp.Rational(1, 2)}
            grad = sp.Matrix([sp.diff(v, q).subs(point) for q in (x, y, z)])
            self.assertEqual(sp.simplify(grad-expected), sp.zeros(3, 1))
            self.assertEqual(sp.simplify(grad.dot(sp.Matrix([1/sp.sqrt(t), 1, 1]))), 0)

    def test_factorization_and_mc_hessian(self):
        variables = sp.symbols('q0:9', real=True)
        q = sp.Matrix(3, 3, variables)
        w = [sp.sqrt(3), 1/sp.sqrt(3), 1/sp.sqrt(3)]
        W = sum(w[k]*sum(q[k,c]**2 for c in range(3))/2 for k in range(3))-q.det()
        self.assertEqual(sp.simplify(sum(sp.diff(W, v, 2) for v in variables)-3*sum(w)), 0)
        subs = dict(zip(variables, list(sp.diag(1/sp.sqrt(3), 1, 1))))
        J = sp.hessian(W, variables).subs(subs)
        lam = sp.Symbol('lam')
        expected = {sp.Integer(0): 3, 2/sp.sqrt(3): 2, 4/sp.sqrt(3): 2,
                    (sp.sqrt(3)-sp.sqrt(11))/2: 1, (sp.sqrt(3)+sp.sqrt(11))/2: 1}
        # Compare polynomials, avoiding unsimplified cubic radicals in eigenvals().
        expected_polynomial = sp.prod((lam-value)**count for value,count in expected.items())
        self.assertEqual(sp.simplify(J.charpoly(lam).as_expr()-sp.expand(expected_polynomial)), 0)

    def test_negative_direction_is_not_straight_mc_tangent(self):
        t = 3.
        J = np.array([[np.sqrt(t),-1,-1],[-1,1/np.sqrt(t),-1/np.sqrt(t)],[-1,-1/np.sqrt(t),1/np.sqrt(t)]])
        values, vectors = np.linalg.eigh(J)
        tangent = np.array([1/np.sqrt(t),1,1]); tangent /= np.linalg.norm(tangent)
        self.assertLess(values[0], 0.)
        self.assertLess(abs(tangent@vectors[:,0]), 1.-1e-6)

    def test_harmonic_switch_and_window(self):
        for t in (1.1, 3., 10.):
            s = (t+2)*(np.sqrt(t+8)-np.sqrt(t))/(4*np.sqrt(t))
            c = np.array([[s*(t+2), -np.sqrt(2)*s*(np.sqrt(t)+2*(1-s)/np.sqrt(t))],
                          [-np.sqrt(2)*s*(np.sqrt(t)+2*(1-s)/np.sqrt(t)), 2*s*s]])
            h = 2*s*(1+2/t-s)
            self.assertAlmostEqual(np.linalg.eigvalsh(c)[0], h, places=11)
            self.assertGreater(s, 1.)
            self.assertLess(s, (t+2)/(t+1))

    def test_frequency_uses_kinetic_metric_and_is_coordinate_invariant(self):
        lam = sp.Symbol('lam')
        K = sp.diag(1, 4)
        G = sp.diag(4, 1)
        self.assertEqual((G.inv()*K).eigenvals(), {sp.Rational(1,4): 1, sp.Integer(4): 1})
        self.assertNotEqual((G.inv()*K).eigenvals(), K.eigenvals())
        S = sp.Matrix([[1,1],[0,1]])
        transformed = (S.T*G*S).inv()*(S.T*K*S)
        self.assertEqual(transformed.charpoly(lam).as_expr(), (G.inv()*K).charpoly(lam).as_expr())

    def test_potential_hessian_is_superpotential_hessian_squared_at_wells(self):
        variables = sp.symbols('q0:9', real=True)
        q = sp.Matrix(3,3,variables)
        weights = [sp.sqrt(3), 1/sp.sqrt(3), 1/sp.sqrt(3)]
        W = sum(weights[k]*sum(q[k,c]**2 for c in range(3))/2 for k in range(3))-q.det()
        gradient = sp.Matrix([sp.diff(W,v) for v in variables])
        J = sp.hessian(W,variables)
        K = sp.hessian(gradient.dot(gradient)/2,variables)
        for point in (sp.zeros(3), sp.diag(1/sp.sqrt(3),1,1)):
            subs = dict(zip(variables,list(point)))
            self.assertEqual(gradient.subs(subs),sp.zeros(9,1))
            self.assertEqual(sp.simplify(K.subs(subs)-J.subs(subs)**2),sp.zeros(9))
        # Away from a critical point, the gradient/third-derivative term cannot be dropped.
        midpoint = dict(zip(variables,list(sp.diag(1/(2*sp.sqrt(3)),sp.Rational(1,2),sp.Rational(1,2)))))
        self.assertNotEqual(sp.simplify(K.subs(midpoint)-J.subs(midpoint)**2),sp.zeros(9))

    def test_frequency_shift_includes_potential_and_kinetic_terms(self):
        eps, lam = sp.symbols('eps lam', real=True)
        K = sp.diag(1,4)
        G = sp.diag(4,1)
        dK = sp.Matrix([[3,1],[1,2]])
        dG = sp.Matrix([[1,sp.Rational(1,2)],[sp.Rational(1,2),2]])
        characteristic = (K+eps*dK-lam*(G+eps*dG)).det()
        v = sp.Matrix([1,0]); omega = sp.Rational(1,2)
        point = {eps:0, lam:omega**2}
        derivative_from_root = -sp.diff(characteristic,eps).subs(point)/sp.diff(characteristic,lam).subs(point)/(2*omega)
        potential = (v.T*dK*v)[0]/(2*omega*(v.T*G*v)[0])
        kinetic = -(v.T*dG*v)[0]*omega/(2*(v.T*G*v)[0])
        self.assertEqual(sp.simplify(derivative_from_root-potential-kinetic),0)
        self.assertEqual(derivative_from_root,sp.Rational(11,16))
        self.assertNotEqual(derivative_from_root,potential)
        self.assertNotEqual(derivative_from_root,kinetic)

    def test_gap_variational_problem_uses_vacuum_orthogonal_states(self):
        H = sp.diag(7,9,12)
        excited = sp.Matrix([[0,0],[1,0],[0,1]])
        restricted = excited.T*(H-7*sp.eye(3))*excited
        self.assertEqual(min(restricted.eigenvals()),2)
        self.assertEqual(min((H-7*sp.eye(3)).eigenvals()),0)
        change = sp.Matrix([[0,1,0],[0,0,1],[1,0,0]])
        rotated = (change*excited).T*(change*H*change.T-7*sp.eye(3))*(change*excited)
        self.assertEqual(rotated,restricted)
        degenerate = sp.diag(7,7,9)
        self.assertEqual(min((excited.T*(degenerate-7*sp.eye(3))*excited).eigenvals()),0)
        above_vacuum_space = sp.Matrix([0,0,1])
        self.assertEqual((above_vacuum_space.T*(degenerate-7*sp.eye(3))*above_vacuum_space)[0],2)

    def test_ordered_projection_can_jump_at_crossing(self):
        a = np.diag([-1., 1.]); b = -a
        ua = np.linalg.eigh(a)[1][:, 0]; ub = np.linalg.eigh(b)[1][:, 0]
        self.assertEqual(np.linalg.norm(np.outer(ua,ua)-np.outer(ub,ub), 2), 1.)

    def test_pointwise_positive_does_not_give_uniform_gap(self):
        x = sp.symbols('x', positive=True)
        delta = x/(1+x)
        self.assertTrue((delta/x).is_positive)
        self.assertEqual(sp.limit(delta/x, x, sp.oo), 0)

    def test_vacuum_superposition_is_not_gap_counterexample(self):
        H = np.diag([0., 2.]); eps = .001
        psi = np.array([np.sqrt(1-eps), np.sqrt(eps)])
        self.assertAlmostEqual(psi@H@psi, 2*eps)
        self.assertLess(psi@H@psi, 2.)

    def test_anomaly_normalization_and_sign(self):
        b, g = sp.symbols('b g', positive=True)
        beta = -b*g**3/(16*sp.pi**2)
        plaquette_coefficient = b/(2*sp.pi**2)*g**2/16
        self.assertEqual(sp.simplify(plaquette_coefficient+beta/(2*g)), 0)

    def test_gaussian_composite_has_nonzero_third_cumulant(self):
        # For X standard Gaussian, Y=X^2-1: E(Y^3)=15-3*3+3*1-1.
        self.assertEqual(15-3*3+3-1, 8)


class Evidence(unittest.TestCase):
    def test_threshold_numbers_and_root_replay(self):
        data = threshold_replay()
        text = section('s6-fast.tex')
        pattern = (r'\\label\{eq:threshold\}\s*D_\{\\rm lin\}\(g\^2\)='
                   r'([0-9.]+)\+g\^2\\bigl\((-[0-9.]+)\+([0-9.]+)-\\kappa\\bigr\)')
        def agrees(source):
            match = re.search(pattern,source)
            if match is None:
                return False
            d0,anharmonic,potential = map(float,match.groups())
            return (abs(d0-data['harmonic_separation'])<5e-7
                    and anharmonic==data['anharmonic_coefficient']
                    and abs(potential-data['potential_coefficient'])<5e-4)
        self.assertTrue(agrees(text))
        self.assertFalse(agrees(text.replace('0.01655+','0.02655+',1)))
        self.assertFalse(agrees(text.replace('-2.208+1.258', '-2.208+0.629',1)))
        self.assertFalse(agrees(text.replace(r'-\kappa\bigr',r'+\kappa\bigr',1)))
        self.assertAlmostEqual(data['kinetic_thresholds']['0.002'],7.33,delta=.005)
        self.assertAlmostEqual(data['kinetic_thresholds']['0.004'],3.19,delta=.005)
        self.assertAlmostEqual(data['harmonic_root'],1.143568,delta=5e-7)
        self.assertAlmostEqual(data['harmonic_slope'],2.569,delta=5e-4)
        for g in (.002,.004):
            saved = receipt('research/gigi-m3-20261127/result.json')['background'][f'g2_{g}']
            self.assertAlmostEqual(data['potential_coefficient']*g,saved['fast_fibre_shift_zeta'],places=14)
        # An opposite kinetic sign or a missing potential factor changes the threshold.
        self.assertNotAlmostEqual(data['harmonic_separation']/.002-2.208,
                                  data['kinetic_thresholds']['0.002'],places=2)

    def test_frozen_control_slope_is_not_registered_switch_reproduction(self):
        data = threshold_replay()
        for row,expected in zip(data['frozen_control_switches'],(.001950,.005638,.010626)):
            self.assertAlmostEqual(row['difference'],expected,delta=5e-7)
            self.assertGreater(row['difference'],.0001)
        proof = (ROOT/'docs/papers/helike-quantum-probe/proof/part5.tex').read_text(encoding='utf-8')
        for value in ('1.1674','1.1809','1.1931','c_C=-2.738','c_H=-0.530'):
            self.assertIn(value,proof)

    def test_running_coupling_ranges(self):
        data = receipt('research/gigi-m3-20261127/result.json')
        group = next(v for v in data.values() if isinstance(v, dict) and 'dC_over_prediction' in v)
        ratios = group['dC_over_prediction']
        errors = [abs(v-1) for v in ratios.values()]
        central = [abs(v-1) for k,v in ratios.items() if .25 <= float(k) <= .75]
        self.assertEqual(len(errors), 7); self.assertEqual(len(central), 5)
        self.assertGreater(max(errors), 1e-4)
        self.assertLess(max(errors), 7.95e-4)
        self.assertLess(max(central), 6.66e-5)

    def test_loop_receipt_and_potential_shift(self):
        loops = receipt('research/gigi-lattice-gauge-20261127/result.json')
        self.assertLess(loops['gauge_invariance_max_loop_change'], 1.23e-15)
        self.assertGreater(loops['gauge_invariance_max_loop_change'], 1e-15)
        data = receipt('research/gigi-m3-20261127/result.json')['background']
        for key in ('g2_0.002','g2_0.004'):
            row = data[key]
            self.assertAlmostEqual(row['harmonic_separation']+row['fast_fibre_shift_zeta'], row['separation_with_fibre_zeta'])


class Document(unittest.TestCase):
    def test_threshold_keeps_local_estimate_and_root_distinct(self):
        text = section('s6-fast.tex')
        self.assertNotIn('Helike stays lowest exactly',text)
        self.assertNotIn('which reproduces the registered',text)
        self.assertNotIn('the physical switch is lower',text)
        for required in (r'D_{\rm lin}', 'same held background', 'no remainder bound',
                         'at the harmonic root', 'equilibrium', 'same-well'):
            self.assertIn(required,text)

    def test_labels_citations_and_statuses(self):
        files = [PAPER/'statement.tex', *sorted((PAPER/'sections').glob('*.tex'))]
        text = '\n'.join(line for p in files for line in p.read_text(encoding='utf-8').splitlines()
                         if not line.lstrip().startswith('%'))
        labels = re.findall(r'\\label\{([^}]+)\}', text)
        self.assertEqual(len(labels),len(set(labels)))
        self.assertFalse(set(re.findall(r'\\ref\{([^}]+)\}',text))-set(labels))
        keys = set(re.findall(r'\\bibitem\{([^}]+)\}',(PAPER/'refs.tex').read_text()))
        cited = {k for group in re.findall(r'\\cite\{([^}]+)\}',text) for k in group.split(',')}
        self.assertFalse(cited-keys); self.assertFalse(keys-cited)
        allowed = {'PROVED','PROVED (CITED)','COMPUTED','CONDITIONAL','CONJECTURE','OPEN'}
        statuses = re.findall(r'\\begin\{claim\}\{([^}]+)\}',text)
        self.assertFalse(set(statuses)-allowed)
        self.assertEqual(len(statuses),text.count(r'\end{claim}'))

    def test_conclusion_does_not_restore_corrected_overclaims(self):
        text = section('s10-what-it-is.tex')
        self.assertNotIn('fast modes only run the coupling',text)
        self.assertNotIn('Topology decides who is lightest}: proved in the reduced model',text)
        self.assertNotIn('then H8 fails',text)


def main():
    ap = argparse.ArgumentParser(); ap.add_argument('out',type=Path); args=ap.parse_args()
    if args.out.exists(): raise FileExistsError(args.out)
    args.out.mkdir(parents=True)
    stream = io.StringIO()
    suite = unittest.defaultTestLoader.loadTestsFromModule(__import__(__name__))
    result = unittest.TextTestRunner(stream=stream,verbosity=2).run(suite)
    log = stream.getvalue(); print(log,end='')
    (args.out/'checks.log').write_text(log,encoding='utf-8')
    sources = [Path(__file__), *sorted(PAPER.rglob('*.tex')), PAPER/'HANDOFF.md', PAPER/'REVIEW-2026-11-27.md',
               ROOT/'docs/research/closed-set-20261127/CLOSED-SET.md',
               PAPER/'THRESHOLD-REVIEW-2026-11-27.md',
               ROOT/'docs/papers/helike-quantum-probe/proof/part5.tex',
               ROOT/'research/gigi-m3-20261127/result.json',ROOT/'research/gigi-lattice-gauge-20261127/result.json']
    hashes = {str(p.relative_to(ROOT)).replace('\\','/'):hashlib.sha256(p.read_bytes().replace(b'\r\n',b'\n')).hexdigest() for p in sources}
    out = dict(scope='Finite algebra, saved receipt replay and document consistency; not a proof certificate',
               tests=result.testsRun,failures=len(result.failures),errors=len(result.errors),passed=result.wasSuccessful(),
               source_revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
               dirty=bool(subprocess.check_output(['git','status','--porcelain'],cwd=ROOT,text=True).strip()),
               numpy=np.__version__,sympy=sp.__version__,sha256_lf=hashes,
               threshold_replay=threshold_replay(),
               pdf_sha256=hashlib.sha256((PAPER/'statement.pdf').read_bytes()).hexdigest())
    (args.out/'receipt.json').write_text(json.dumps(out,indent=2)+'\n',encoding='utf-8')
    return 0 if result.wasSuccessful() else 1


if __name__=='__main__': raise SystemExit(main())
