"""Exact vacuum-floor algebra and bounded Lean checks; no spectral runs."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess
import sys
import unittest

import sympy as s

from check_h7_lean import ROOT, PROJECT, ALLOWED_AXIOMS, run

NOTE = ROOT/'docs/research/h8-vacuum-floor-proof-2026-11-27.md'
EXPECTED = ('factorization_jet', 'squares_zero', 'cone_tail_identity',
            'cone_tail_bound', 'strict_floor_of_attained', 'sector_floor',
            'gaussian_jet')


def require_zero(expression):
    if s.simplify(expression) != 0:
        raise ValueError('Exact identity failed')


class Algebra(unittest.TestCase):
    def test_su2_gradient_laplacian_and_potential(self):
        u, g = s.symbols('u g', positive=True)
        q = s.Matrix(3, 3, s.symbols('q0:9', real=True))
        omega = (1/u, u, u)
        w = sum(omega[k]*sum(q[k, a]**2 for a in range(3)) for k in range(3))/2-g*q.det()
        cofactor = q.cofactor_matrix()
        grad = [s.diff(w, x) for x in q]
        for k in range(3):
            for a in range(3):
                require_zero(grad[3*k+a]-(omega[k]*q[k, a]-g*cofactor[k, a]))
        lap = sum(s.diff(w, x, 2) for x in q)
        require_zero(lap-3*sum(omega))
        expanded = (sum(omega[k]**2*sum(q[k, a]**2 for a in range(3)) for k in range(3))/2
                    -g*sum(omega)*q.det()+g**2*sum(x*x for x in cofactor)/2)
        require_zero(sum(x*x for x in grad)/2-expanded)
        with self.assertRaises(ValueError):
            require_zero(lap/2-3*sum(omega))

    def test_row_trilinearity_in_arbitrary_colour_examples(self):
        omega = s.symbols('w0:3', positive=True)
        g = s.symbols('g', positive=True)
        for n in (2, 3, 8):
            x, y, z = (s.symbols(f'{row}0:{n}', real=True) for row in 'xyz')
            coeff = s.symbols(f'c0:{n**3}', real=True)
            cubic = sum(coeff[(a*n+b)*n+c]*x[a]*y[b]*z[c]
                        for a in range(n) for b in range(n) for c in range(n))
            w = sum(wk*sum(v*v for v in row)/2 for wk, row in zip(omega, (x, y, z)))-g*cubic
            require_zero(sum(s.diff(w, v, 2) for v in (*x, *y, *z))-n*sum(omega))

    def test_factorization_sign_and_formal_zero_mode(self):
        x = s.symbols('x', real=True)
        w, f = s.Function('W')(x), s.Function('f')(x)
        af = s.diff(f, x)+s.diff(w, x)*f
        astaraf = -s.diff(af, x)+s.diff(w, x)*af
        h = -s.diff(f, x, 2)/2+s.diff(w, x)**2*f/2
        require_zero(astaraf/2+s.diff(w, x, 2)*f/2-h)
        with self.assertRaises(ValueError):
            require_zero(astaraf/2-s.diff(w, x, 2)*f/2-h)
        require_zero(s.diff(s.exp(-w), x)+s.diff(w, x)*s.exp(-w))

    def test_gaussian_equality_at_zero_coupling_or_zero_cubic(self):
        x = s.symbols('x', real=True)
        omega = s.symbols('omega', positive=True)
        f = s.exp(-omega*x*x/2)
        require_zero(-s.diff(f, x, 2)/2+omega**2*x*x*f/2-omega*f/2)
        self.assertEqual(s.integrate(f*f, (x, -s.oo, s.oo)), s.sqrt(s.pi/omega))

    def test_open_cone_and_positive_cubic_witness(self):
        # SU(2) cross-product convention, exact witness [e2,e3]=e1.
        e2, e3 = s.Matrix([0, 1, 0]), s.Matrix([0, 0, 1])
        b = e2.cross(e3)
        self.assertEqual(b.dot(e2.cross(e3)), 1)
        r, a, g, c, v = s.symbols('r a g c v', positive=True)
        tail = a*r*r-g*c*r**3
        require_zero(-g*c*r**3/2-tail-r*r*(g*c*r-2*a)/2)
        at_threshold = tail.subs(r, 2*a/(g*c))
        self.assertLess(at_threshold.subs({a:1, g:1, c:1}), 0)
        with self.assertRaises(ValueError):
            require_zero(-g*c*r**3-tail-r*r*(g*c*r-2*a)/2)
        # Positive cap area is essential to the polar lower bound.
        for dimension in (3, 9, 24):
            radial = s.integrate(r**(dimension-1), (r, 1, v))
            self.assertEqual(s.limit(radial, v, s.oo), s.oo)
            self.assertEqual(s.simplify(0*radial), 0)  # a ray has zero area

    def test_no_kernel_is_not_a_positive_spectral_bottom(self):
        # Normalized Gaussian dilates for -d^2/dx^2 / 2 on the line.
        x, length = s.symbols('x L', real=True, positive=True)
        f = s.pi**(-s.Rational(1, 4))*length**(-s.Rational(1, 2))*s.exp(-x*x/(2*length**2))
        require_zero(s.integrate(f*f, (x, -s.oo, s.oo))-1)
        energy = s.integrate(s.diff(f, x)**2/2, (x, -s.oo, s.oo))
        require_zero(energy-1/(4*length**2))
        self.assertEqual(s.limit(energy, length, s.oo), 0)
        # The only square-integrable affine zero-energy solution is zero.
        a, b = s.symbols('a b', real=True)
        require_zero(s.diff(a*x+b, x, 2))


class Document(unittest.TestCase):
    def test_written_proof_and_scope(self):
        self.assertTrue(NOTE.exists(), 'Missing strict-floor proof')
        text = ' '.join(NOTE.read_text(encoding='utf-8').split())
        for marker in ('positive-measure', 'Friedrichs', 'compact resolvent',
                       'weak derivatives', 'not end-to-end Lean', 'H7 and H8 remain open',
                       'nS/2', 'not the excitation gap'):
            self.assertIn(marker, text)


def audit(output):
    found = {name: sorted(v.strip() for v in deps.split(',') if v.strip())
             for name, deps in re.findall(r"'(H8Vacuum\.\w+)' depends on axioms: \[([^\]]*)\]", output, re.S)}
    for name in re.findall(r"'(H8Vacuum\.\w+)' does not depend on any axioms", output):
        found[name] = []
    for suffix in EXPECTED:
        key = 'H8Vacuum.'+suffix
        if key not in found or set(found[key])-ALLOWED_AXIOMS:
            raise ValueError('Missing or untrusted proof dependency: '+key)
    return found


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('out', type=Path)
    parser.add_argument('--expect-red', action='store_true')
    args = parser.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    receipt = dict(passed=False, mode='red' if args.expect_red else 'release',
                   scope='Exact algebra and conditional Lean deduction; global analysis written separately',
                   source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
                   runs=[], failures=[])
    try:
        stream = io.StringIO()
        result = unittest.TextTestRunner(stream=stream, verbosity=2).run(
            unittest.defaultTestLoader.loadTestsFromModule(sys.modules[__name__]))
        (out/'exact.log').write_text(stream.getvalue(), encoding='utf-8')
        receipt['exact'] = dict(tests=result.testsRun, failures=len(result.failures), errors=len(result.errors))
        receipt['sympy'] = s.__version__
        if args.expect_red:
            if result.errors or len(result.failures) != 1 or 'Missing strict-floor proof' not in stream.getvalue():
                raise ValueError('Expected only the missing-proof red test')
        elif not result.wasSuccessful():
            raise ValueError('Exact or written-scope gate failed')
        manifest = json.loads((PROJECT/'lake-manifest.json').read_text())
        receipt['dependency_revisions'] = {p['name']: p['rev'] for p in manifest['packages']}
        receipt['toolchain'] = (PROJECT/'lean-toolchain').read_text().strip()
        if receipt['dependency_revisions']['mathlib'] != 'fc4f59e577feb1a4956671bc04ac18aaaf8cd842' or receipt['toolchain'] != 'leanprover/lean4:v4.36.0':
            raise ValueError('Changed dependency pin')
        for file in ('H8VacuumFloor.lean', 'H8VacuumDeclarations.lean', 'H8VacuumVerification.lean'):
            path = PROJECT/file
            if not path.exists() and args.expect_red:
                continue
            if re.search(r'\b(sorry|admit|sorryAx|axiom|native_decide|unsafe)\b', path.read_text(encoding='utf-8')):
                raise ValueError('Forbidden shortcut: '+file)
        entry, _ = run(['lake', 'build'], out/'build.log')
        receipt['runs'].append(entry)
        if entry['exit_code']:
            raise ValueError('Lean build failed')
        entry, output = run(['lake', 'env', 'lean', 'H8VacuumDeclarations.lean'], out/'declarations.log')
        receipt['runs'].append(entry)
        if args.expect_red:
            if not entry['exit_code'] or any('Unknown identifier `H8Vacuum.'+n+'`' not in output for n in EXPECTED):
                raise ValueError('Missing declarations did not fail as expected')
            receipt['expected_red_observed'] = True
            return 0
        if entry['exit_code']:
            raise ValueError('Declaration audit failed')
        entry, output = run(['lake', 'env', 'lean', 'H8VacuumVerification.lean'], out/'verification.log')
        receipt['runs'].append(entry)
        if entry['exit_code']:
            raise ValueError('Lean verification failed')
        receipt['axioms'] = audit(output)
        for poison in ('sorryAx', 'UnverifiedCompactness'):
            try:
                audit(output.replace('propext', poison))
            except ValueError:
                continue
            raise ValueError('Axiom auditor accepted '+poison)
        receipt['audit_rejections'] = ['sorryAx', 'UnverifiedCompactness']
        source = (PROJECT/'H8VacuumFloor.lean').read_text(encoding='utf-8')
        mutations = {
            'wrong_floor_shift': ('+ wpp * f / 2 =', '- wpp * f / 2 ='),
            'missing_attainment': ('hatt : ∃ x, admissible x ∧ energy x = bottom', 'hatt : True'),
            'missing_kernel_exclusion': ('hno : ∀ x, admissible x → ¬ kernel x', 'hno : True'),
            'too_strong_cone_bound': ('a * r^2 - g * c * r^3 ≤ -(g * c * r^3) / 2', 'a * r^2 - g * c * r^3 ≤ -(g * c * r^3)'),
        }
        for name, (old, new) in mutations.items():
            if source.count(old) != 1:
                raise ValueError('Missing or ambiguous mutation anchor: '+name)
            path = out/(name+'.lean')
            path.write_text(source.replace(old, new, 1), encoding='utf-8')
            entry, output = run(['lake', 'env', 'lean', str(path)], out/(name+'.log'))
            receipt['runs'].append(entry)
            if not entry['exit_code'] or not re.search(r'error[:(]', output):
                raise ValueError('Bad mechanism accepted: '+name)
            if re.search(r'unknown module|object file .* does not exist', output):
                raise ValueError('Import failure is not a valid negative gate')
        receipt['mutations_rejected'] = list(mutations)
        receipt['passed'] = True
        return 0
    except Exception as exc:
        receipt['failures'].append(str(exc))
        return 1
    finally:
        files = [Path(__file__), ROOT/'scripts/check_h7_lean.py', NOTE,
                 ROOT/'docs/research/h8-vacuum-floor-contract-2026-11-27.md',
                 ROOT/'docs/research/h7-cubic-quartic-proof-2026-11-27.md',
                 ROOT/'docs/research/closed-set-20261127/H8-s1-carrier.md',
                 ROOT/'docs/research/closed-set-20261127/CLOSED-SET.md',
                 PROJECT/'H8Vacuum-README.md',
                 *PROJECT.glob('*.lean'), PROJECT/'lakefile.toml', PROJECT/'lake-manifest.json', PROJECT/'lean-toolchain']
        receipt['sha256_lf'] = {p.relative_to(ROOT).as_posix():hashlib.sha256(p.read_bytes().replace(b'\r\n', b'\n')).hexdigest()
                                for p in files if p.exists()}
        (out/'receipt.json').write_text(json.dumps(receipt, indent=2)+'\n', encoding='utf-8')
        print(json.dumps({k:receipt[k] for k in ('mode', 'passed', 'failures')}, indent=2))


if __name__ == '__main__':
    raise SystemExit(main())
