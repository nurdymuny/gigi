"""Exact inter-well geometry checks and bounded Lean proof audit; no simulation."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess
import sys
import unittest

import sympy as sp

from check_h7_lean import PROJECT, ROOT, ALLOWED_AXIOMS, audit_axioms as audit_h7, run

NOTE = ROOT/'docs/research/h8-interwell-proof-2026-11-27.md'
EXPECTED = ('metric_pairing', 'dissipation', 'path_derivative', 'rectangle_boundaries',
            'interior_descent', 'equilibria', 'saddle_height', 'unstable_root',
            'unstable_direction', 'rectangle_ceiling', 'decay_direction',
            'weighted_cauchy', 'straight_excess', 'coupling_scaling')


def plane():
    u, x, y = sp.symbols('u x y', positive=True)
    w = x**2/(2*u)+u*y**2-x*y**2
    f, h = y**2-x/u, (x-u)*y
    return u, x, y, w, f, h


def metric_flow_check(metric_weight=2, coordinate_factor=sp.Rational(1, 2), sign=-1):
    u, x, y, w, f, h = plane()
    p = sp.diag(x, y, y)
    metric = sp.Matrix([[sum(a*b for a, b in zip(p.diff(i), p.diff(j)))
                         for j in (x, y)] for i in (x, y)])
    if metric != sp.diag(1, metric_weight):
        raise ValueError('The inherited matrix metric is not the proposed coordinate metric')
    flow = sp.Matrix([sign*sp.diff(w, x), sign*coordinate_factor*sp.diff(w, y)])
    if any(sp.simplify(v) != 0 for v in flow-sp.Matrix([f, h])):
        raise ValueError('The proposed flow is not the ambient negative gradient')


class Geometry(unittest.TestCase):
    def test_full_nine_dimensional_gradient_restricts(self):
        u, x, y, w, f, h = plane()
        entries = sp.symbols('p0:9')
        p = sp.Matrix(3, 3, entries)
        full = sum((1/u, u, u)[k]*sum(p[k, c]**2 for c in range(3))
                   for k in range(3))/2-p.det()
        substitution = dict(zip(entries, sp.diag(x, y, y)))
        self.assertEqual(sp.simplify(full.subs(substitution)-w), 0)
        grad = sp.Matrix(3, 3, [sp.diff(full, e).subs(substitution) for e in entries])
        self.assertEqual((grad-sp.diag(-f, -h, -h)).applyfunc(sp.simplify), sp.zeros(3))

    def test_inherited_metric_and_flow(self):
        metric_flow_check()

    def test_metric_and_flow_mechanism_removal(self):
        for kwargs in ({'metric_weight': 1}, {'coordinate_factor': 1}, {'sign': 1}):
            with self.subTest(kwargs=kwargs), self.assertRaises(ValueError):
                metric_flow_check(**kwargs)

    def test_dissipation_and_action_density(self):
        u, x, y, w, f, h = plane()
        self.assertEqual(sp.expand(sp.diff(w, x)*f+sp.diff(w, y)*h+f*f+2*h*h), 0)
        self.assertNotEqual(sp.expand(sp.diff(w, x)*f+sp.diff(w, y)*h+f*f+h*h), 0)

    def test_equilibria_and_saddle(self):
        u, x, y, w, f, h = plane()
        roots = sp.solve([f, h], [x, y], dict=True)
        # Positive-symbol assumptions hide the zero solution from solve; test it separately.
        self.assertEqual(f.subs({x: 0, y: 0}), 0)
        self.assertEqual(h.subs({x: 0, y: 0}), 0)
        self.assertEqual(roots, [{x: u, y: 1}])
        jac = sp.Matrix([f, h]).jacobian([x, y]).subs({x: u, y: 1})
        self.assertEqual(jac, sp.Matrix([[-1/u, 2], [1, 0]]))
        self.assertEqual(jac.det(), -2)

    def test_unstable_direction(self):
        u, x, y, w, f, h = plane()
        lam = (sp.sqrt(1/u**2+8)-1/u)/2
        self.assertEqual(sp.simplify(lam**2+lam/u-2), 0)
        self.assertEqual(sp.simplify(-lam/u+2-lam**2), 0)
        self.assertEqual(sp.simplify(sp.sqrt(1/u**2+8)**2-(1/u)**2), 8)

    def test_rectangle_faces(self):
        u, x, y, w, f, h = plane()
        self.assertEqual([f.subs(x, 0), f.subs(x, u), h.subs(y, 0), h.subs(y, 1)],
                         [y**2, y**2-1, 0, x-u])

    def test_decay_comparison_identity(self):
        u, x, y, w, f, h = plane()
        M = sp.symbols('M', real=True)
        self.assertEqual(sp.expand(f+x/u), y**2)
        self.assertEqual(sp.expand(h+(u-M)*y), x*y-M*y)
        self.assertEqual(sp.simplify(w.subs({x: u, y: 1})), u/2)

    def test_global_path_cauchy_identity(self):
        a, b, c, d = sp.symbols('a b c d', real=True)
        self.assertEqual(sp.expand((a*a+2*b*b)*(c*c+2*d*d)
                                  -(a*c+2*b*d)**2-2*(a*d-b*c)**2), 0)

    def test_straight_path_action_and_excess(self):
        u, x, y, w, f, h = plane()
        ell = sp.symbols('ell', real=True)
        grad_sq = (f*f+2*h*h).subs({x: u*ell, y: ell})
        self.assertEqual(sp.factor(grad_sq), ell**2*(ell-1)**2*(2*u*u+1))
        self.assertEqual(sp.integrate(ell*(1-ell), (ell, 0, 1)), sp.Rational(1, 6))
        length_sq = (u*u+2)*(1+2*u*u)/36
        self.assertEqual(sp.factor(length_sq-u*u/4), (u-1)**2*(u+1)**2/18)
        self.assertNotEqual(sp.simplify(length_sq.subs(u, 1/sp.sqrt(3))-sp.Rational(1, 12)), 0)
        self.assertEqual(sp.factor((f-u*h).subs({x: u*ell, y: ell})),
                         -ell*(ell-1)*(u-1)*(u+1))

    def test_round_sphere_exact_orbit(self):
        tau = sp.symbols('tau', real=True)
        z = 1/(1+sp.exp(tau))
        self.assertEqual(sp.simplify(sp.diff(z, tau)-(z*z-z)), 0)
        self.assertEqual(sp.limit(z, tau, -sp.oo), 1)
        self.assertEqual(sp.limit(z, tau, sp.oo), 0)

    def test_action_normalization_and_coupling(self):
        u, x, y, w, f, h = plane()
        g = sp.symbols('g', positive=True)
        wg = x*x/(2*u)+u*y*y-g*x*y*y
        self.assertEqual(sp.simplify(wg.subs({x: x/g, y: y/g}, simultaneous=True)-w/g**2), 0)
        self.assertEqual(sp.simplify(w.subs({x: u, y: 1})-w.subs({x: 0, y: 0})), u/2)
        self.assertNotEqual(u/2, u/4)
        self.assertNotEqual(u/(2*g*g), u/(2*g))


class Document(unittest.TestCase):
    def test_proof_scope_and_endpoint_argument(self):
        self.assertTrue(NOTE.exists(), 'Missing complete written proof')
        text = NOTE.read_text(encoding='utf-8')
        for marker in ('dx^2+2dy^2', 'Theorem 4.10', 'M<u', 'connectors',
                       'H8 remains open', 'not end-to-end Lean', '1/(2\\sqrt t)', 'p=gq'):
            self.assertIn(marker, text)

    def test_only_interwell_subclaim_is_closed(self):
        h8 = (ROOT/'docs/research/closed-set-20261127/H8-s1-carrier.md').read_text(encoding='utf-8')
        self.assertTrue('h8-interwell-proof-2026-11-27.md' in h8, 'H8 has not integrated the inter-well proof')
        self.assertIn('H8 remains open', h8)
        self.assertNotIn('I expect it does [guess]', h8)
        register = (ROOT/'docs/research/closed-set-20261127/CLOSED-SET.md').read_text(encoding='utf-8')
        self.assertIn('18. [X;', register)
        self.assertIn('inter-well action closure', register)


def audit(text):
    found = {name: sorted(d.strip() for d in deps.split(',') if d.strip())
             for name, deps in re.findall(r"'(H8\.\w+)' depends on axioms: \[([^\]]*)\]", text, re.S)}
    for name in re.findall(r"'(H8\.\w+)' does not depend on any axioms", text):
        found[name] = []
    for name in EXPECTED:
        if 'H8.'+name not in found or set(found['H8.'+name])-ALLOWED_AXIOMS:
            raise ValueError('Missing or unexpected dependencies for H8.'+name)
    return found


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('out', type=Path)
    parser.add_argument('--expect-red', action='store_true')
    args = parser.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    sys.stdout.reconfigure(encoding='utf-8')
    receipt = dict(passed=False, mode='red' if args.expect_red else 'release',
                   scope='Exact geometry and local Lean lemmas; global ODE/action proof is written, not formalized',
                   source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
                   runs=[], failures=[])
    try:
        stream = io.StringIO()
        result = unittest.TextTestRunner(stream=stream, verbosity=2).run(
            unittest.defaultTestLoader.loadTestsFromModule(sys.modules[__name__]))
        (out/'algebra.log').write_text(stream.getvalue(), encoding='utf-8')
        receipt['algebra'] = dict(tests=result.testsRun, failures=len(result.failures), errors=len(result.errors))
        receipt['sympy'] = sp.__version__
        if not args.expect_red and not result.wasSuccessful():
            raise ValueError('Exact geometry or document checks failed')
        if args.expect_red and (result.errors or len(result.failures) != 1
                                or 'Missing complete written proof' not in stream.getvalue()):
            raise ValueError('Expected only the absent written-proof failure')
        manifest = json.loads((PROJECT/'lake-manifest.json').read_text())
        receipt['dependency_revisions'] = {p['name']: p['rev'] for p in manifest['packages']}
        receipt['toolchain'] = (PROJECT/'lean-toolchain').read_text().strip()
        if receipt['dependency_revisions']['mathlib'] != 'fc4f59e577feb1a4956671bc04ac18aaaf8cd842' or receipt['toolchain'] != 'leanprover/lean4:v4.36.0':
            raise ValueError('Toolchain or Mathlib pin changed')
        for file in ('H8Interwell.lean', 'H8Declarations.lean', 'H8Verification.lean'):
            if not (PROJECT/file).exists() and args.expect_red:
                continue
            if re.search(r'\b(sorry|admit|sorryAx|axiom|native_decide|unsafe)\b', (PROJECT/file).read_text(encoding='utf-8')):
                raise ValueError('Forbidden proof shortcut: '+file)
        entry, _ = run(['lake', 'build'], out/'build.log')
        receipt['runs'].append(entry)
        if entry['exit_code']:
            raise ValueError('Lean build failed')
        if not args.expect_red:
            entry, _ = run(['lake', 'env', 'lean', 'H8Declarations.lean'], out/'declarations.log')
            receipt['runs'].append(entry)
            if entry['exit_code']:
                raise ValueError('Declared theorem signatures failed')
        target = 'H8Declarations.lean' if args.expect_red else 'H8Verification.lean'
        entry, output = run(['lake', 'env', 'lean', target], out/'verification.log')
        receipt['runs'].append(entry)
        if args.expect_red:
            if not entry['exit_code'] or any('Unknown identifier `H8.'+n+'`' not in output for n in EXPECTED):
                raise ValueError('Absent declarations were not all rejected')
            receipt['expected_red_observed'] = True
            return 0
        if entry['exit_code']:
            raise ValueError('Lean verification failed')
        receipt['axioms'] = audit(output)
        for poison in ('sorryAx', 'UnverifiedOrbit'):
            try:
                audit(output.replace('propext', poison))
            except ValueError:
                continue
            raise ValueError('Poisoned dependency audit passed')
        receipt['auditor_rejections'] = ['sorryAx', 'UnverifiedOrbit']
        entry, old_output = run(['lake', 'env', 'lean', 'Verification.lean'], out/'h7-regression.log')
        receipt['runs'].append(entry)
        if entry['exit_code']:
            raise ValueError('H7 regression failed')
        receipt['h7_axioms'] = audit_h7(old_output)
        source = (PROJECT/'H8Interwell.lean').read_text(encoding='utf-8')
        mutations = {
            'wrong_metric': ('a * c + 2 * b * d', 'a * c + 3 * b * d'),
            'reversed_flow': ('(x - u) * y', '(u - x) * y'),
            'wrong_action': ('W u u 1 - W u 0 0 = u / 2', 'W u u 1 - W u 0 0 = u / 4'),
            'missing_domain': ('(hy1 : y < 1)', '(hy1 : True)'),
        }
        for name, (old, new) in mutations.items():
            if old not in source:
                raise ValueError('Missing mutation anchor: '+name)
            path = out/(name+'.lean')
            path.write_text(source.replace(old, new, 1), encoding='utf-8')
            entry, diagnostic = run(['lake', 'env', 'lean', str(path)], out/(name+'.log'))
            receipt['runs'].append(entry)
            if not entry['exit_code'] or not re.search(r'error[:(]', diagnostic):
                raise ValueError('Bad proof accepted: '+name)
            if re.search(r'unknown module|object file .* does not exist', diagnostic):
                raise ValueError('Import failure is not a valid mutation gate')
        receipt['mutations_rejected'] = list(mutations)
        receipt['passed'] = True
        return 0
    except Exception as exc:
        receipt['failures'].append(str(exc))
        return 1
    finally:
        files = [Path(__file__), ROOT/'scripts/check_h7_lean.py', NOTE,
                 ROOT/'docs/research/h8-interwell-contract-2026-11-27.md',
                 ROOT/'docs/research/closed-set-20261127/H8-s1-carrier.md',
                 ROOT/'docs/research/closed-set-20261127/CLOSED-SET.md',
                 *PROJECT.glob('*.lean'), PROJECT/'lakefile.toml', PROJECT/'lake-manifest.json', PROJECT/'lean-toolchain']
        receipt['sha256_lf'] = {p.relative_to(ROOT).as_posix(): hashlib.sha256(
            p.read_bytes().replace(b'\r\n', b'\n')).hexdigest() for p in files if p.exists()}
        (out/'receipt.json').write_text(json.dumps(receipt, indent=2)+'\n', encoding='utf-8')
        print(json.dumps({k: receipt[k] for k in ('mode', 'passed', 'failures')}, indent=2))


if __name__ == '__main__':
    raise SystemExit(main())
