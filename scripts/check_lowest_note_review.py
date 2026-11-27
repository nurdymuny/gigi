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


class Algebra(unittest.TestCase):
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
               ROOT/'research/gigi-m3-20261127/result.json',ROOT/'research/gigi-lattice-gauge-20261127/result.json']
    hashes = {str(p.relative_to(ROOT)).replace('\\','/'):hashlib.sha256(p.read_bytes().replace(b'\r\n',b'\n')).hexdigest() for p in sources}
    out = dict(scope='Finite algebra, saved receipt replay and document consistency; not a proof certificate',
               tests=result.testsRun,failures=len(result.failures),errors=len(result.errors),passed=result.wasSuccessful(),
               source_revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
               dirty=bool(subprocess.check_output(['git','status','--porcelain'],cwd=ROOT,text=True).strip()),
               numpy=np.__version__,sympy=sp.__version__,sha256_lf=hashes,
               pdf_sha256=hashlib.sha256((PAPER/'statement.pdf').read_bytes()).hexdigest())
    (args.out/'receipt.json').write_text(json.dumps(out,indent=2)+'\n',encoding='utf-8')
    return 0 if result.wasSuccessful() else 1


if __name__=='__main__': raise SystemExit(main())
