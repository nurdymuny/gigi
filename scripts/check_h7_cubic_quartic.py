"""Exact supporting algebra and scope checks; not a replacement for the cited proof."""
import argparse
from copy import deepcopy
import hashlib
import io
import json
from pathlib import Path
import subprocess
import sys
import unittest

import sympy as sp

ROOT = Path(__file__).resolve().parents[1]
NOTE = ROOT / 'docs/research/h7-cubic-quartic-proof-2026-11-27.md'
H7 = ROOT / 'docs/research/closed-set-20261127/H7-every-group.md'
PAPER = ROOT / 'docs/papers/ym-theory-statement'


def bracket(a, b):
    return a*b-b*a


def fixtures():
    a = sp.Matrix([[0, 1, 0], [1, 0, 0], [0, 0, 0]])/2
    b = sp.Matrix([[0, -sp.I, 0], [sp.I, 0, 0], [0, 0, 0]])/2
    c = sp.diag(1, -1, 0)/2
    return a, b, c


def hermitian_invariants(matrices):
    a, b, c = matrices
    cubic = sp.simplify(-sp.I*sp.trace(a*bracket(b, c)))
    quartic = sp.simplify(sum(sp.trace(bracket(x, y).H*bracket(x, y))
                              for x, y in ((a, b), (b, c), (c, a))))
    return cubic, quartic


def validate_contract(data):
    expected = dict(cubic=sp.Rational(1, 2), quartic=sp.Rational(3, 2),
                    exponent=sp.Rational(3, 4), remainder=sp.Rational(27, 32),
                    h7_status='CONJECTURE')
    if data != expected:
        raise ValueError('Changed convention, exponent, remainder or scope')


class Algebra(unittest.TestCase):
    def test_trace_conventions_on_exact_su3_embedding(self):
        matrices = fixtures()
        self.assertEqual(hermitian_invariants(matrices), (sp.Rational(1, 2), sp.Rational(3, 2)))
        for a in matrices:
            self.assertEqual(a.H, a)
            self.assertEqual(sp.trace(a), 0)
        anti = tuple(-sp.I*a for a in matrices)
        self.assertEqual(-sp.trace(anti[0]*bracket(anti[1], anti[2])), sp.Rational(1, 2))
        self.assertEqual(sum(-sp.trace(bracket(a, b)**2) for a, b in
                             ((anti[0], anti[1]), (anti[1], anti[2]), (anti[2], anti[0]))),
                         sp.Rational(3, 2))

    def test_cyclic_pairings_and_epsilon_half(self):
        x = tuple(-sp.I*a for a in fixtures())
        expected = (bracket(x[1], x[2]), bracket(x[2], x[0]), bracket(x[0], x[1]))
        for k in range(3):
            y = sum((sp.LeviCivita(k, l, m)*bracket(x[l], x[m])/2
                     for l in range(3) for m in range(3)), sp.zeros(3))
            self.assertEqual(y, expected[k])
            self.assertEqual(-sp.trace(x[k]*y), sp.Rational(1, 2))
            self.assertNotEqual(2*y, expected[k])

    def test_homogeneity_and_rescaling(self):
        r, q, a, b = sp.symbols('r q a b', positive=True)
        c0, q0 = hermitian_invariants(fixtures())
        c1, q1 = hermitian_invariants(tuple(r*x for x in fixtures()))
        self.assertEqual(c1, r**3*c0)
        self.assertEqual(q1, r**4*q0)
        result = sp.simplify((a*r*q+b/r**3).subs(r, q**(-sp.Rational(1, 4))))
        self.assertEqual(result, (a+b)*q**sp.Rational(3, 4))
        self.assertNotEqual(sp.simplify(result-(a+b)*q**sp.Rational(2, 3)), 0)

    def test_fixed_invariant_norm_rescaling(self):
        scale, c, q = sp.symbols('scale c q', positive=True)
        self.assertEqual(sp.simplify(scale*c*q**sp.Rational(3, 4)
                                    -scale**sp.Rational(1, 4)*c*(scale*q)**sp.Rational(3, 4)), 0)

    def test_zero_quartic_commuting_case(self):
        matrices = (sp.diag(1, 1, -2), sp.diag(2, -3, 1), sp.diag(-1, 2, -1))
        self.assertEqual(hermitian_invariants(matrices), (0, 0))

    def test_nonregular_commuting_limit_exact_identity(self):
        e = sp.symbols('e', real=True)
        a, b, c = fixtures()
        h = sp.diag(1, 1, -2)
        self.assertEqual(bracket(h, a), sp.zeros(3))
        self.assertEqual(bracket(h, b), sp.zeros(3))
        self.assertEqual(hermitian_invariants((h+e*c, e*a, e*b)), (e**3/2, 3*e**4/2))

    def test_coercivity_identity_and_negative_control(self):
        z = sp.symbols('z', nonnegative=True)
        polynomial = z**4/2-z**3+sp.Rational(27, 32)
        factors = (2*z-3)**2*(4*z**2+4*z+3)/32
        self.assertEqual(sp.expand(polynomial-factors), 0)
        self.assertEqual(polynomial.subs(z, sp.Rational(3, 2)), 0)
        self.assertLess((polynomial-sp.Rational(27, 64)).subs(z, sp.Rational(3, 2)), 0)

    def test_coercivity_coupling_and_constant(self):
        z, g, S, c = sp.symbols('z g S c', positive=True)
        q = (S*c*z/g)**4
        expression = g**2*q/2-g*S*c*q**sp.Rational(3, 4)+27*S**4*c**4/(32*g**2)
        self.assertEqual(sp.simplify(expression-S**4*c**4/g**2
                                    *(2*z-3)**2*(4*z**2+4*z+3)/32), 0)

    def test_same_positive_kappa_window(self):
        t, s = sp.symbols('t s', positive=True)
        a1 = t-(1-s)*(t-2*s)
        a2 = 1/t-(1-s)*((1-s)/t-s)
        kappa = s*(t+2-s*(t+1))/t
        self.assertEqual(sp.simplify(a2-kappa), 0)
        self.assertEqual(sp.simplify(a1-a2-s*(t-1)*(t+2-s)/t), 0)
        self.assertEqual(kappa.subs(s, (t+2)/(t+1)), 0)
        self.assertEqual(kappa.subs({t: 3, s: sp.Rational(23, 20)}), sp.Rational(23, 150))

    def test_bad_evidence_is_rejected(self):
        c, q = hermitian_invariants(fixtures())
        good = dict(cubic=c, quartic=q, exponent=sp.Rational(3, 4),
                    remainder=sp.Rational(27, 32), h7_status='CONJECTURE')
        validate_contract(good)
        for key, value in [('cubic', -c), ('quartic', 1), ('exponent', sp.Rational(2, 3)),
                           ('remainder', sp.Rational(27, 64)), ('h7_status', 'PROVED')]:
            bad = deepcopy(good)
            bad[key] = value
            with self.subTest(mutation=key), self.assertRaises(ValueError):
                validate_contract(bad)


class Document(unittest.TestCase):
    def test_proof_dependency_and_scope_are_explicit(self):
        self.assertTrue(NOTE.exists(), 'Missing written proof')
        text = NOTE.read_text(encoding='utf-8')
        for required in ('(2.2)', '0310170v2', 'Q=0', 'Q^{-1/4}', '27/32',
                         'not compact', 'H7 remains open', 'Friedrichs', 'not a proof assistant'):
            self.assertIn(required, text)

    def test_only_subconjecture_is_closed(self):
        note = H7.read_text(encoding='utf-8')
        self.assertTrue('CLOSED BY DEDUCTION' in note, 'The sub-conjecture is still pending')
        self.assertNotIn('**Need a new lemma.** Line 9', note)
        text = (PAPER/'sections/s9-content.tex').read_text(encoding='utf-8')
        self.assertIn(r'\label{b:cubicquartic}', text)
        self.assertIn(r'\begin{claim}{CONJECTURE}{H7:', text)
        self.assertNotIn('have not proved', text)
        register = (ROOT/'docs/research/closed-set-20261127/CLOSED-SET.md').read_text(encoding='utf-8')
        self.assertIn('24. [X = H7]', register)
        self.assertIn('cubic-quartic closure', register)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('out', type=Path)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=False)
    stream = io.StringIO()
    result = unittest.TextTestRunner(stream=stream, verbosity=2).run(
        unittest.defaultTestLoader.loadTestsFromModule(__import__(__name__)))
    log = stream.getvalue()
    (args.out/'checks.log').write_text(log, encoding='utf-8')
    sys.stdout.reconfigure(encoding='utf-8')
    print(log, end='')
    files = [Path(__file__), NOTE, H7,
             ROOT/'docs/research/h7-cubic-quartic-contract-2026-11-27.md',
             ROOT/'docs/research/closed-set-20261127/CLOSED-SET.md', *PAPER.rglob('*.tex')]
    receipt = dict(scope='Exact supporting algebra and document checks, not a universal-inequality proof checker',
                   dependency='Austing--Wheater hep-th/0310170v2 eq. (2.2)',
                   source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
                   sympy=sp.__version__, tests=result.testsRun, failures=len(result.failures),
                   errors=len(result.errors), passed=result.wasSuccessful(),
                   sha256_lf={p.relative_to(ROOT).as_posix(): hashlib.sha256(
                       p.read_bytes().replace(b'\r\n', b'\n')).hexdigest() for p in files if p.exists()},
                   missing=[p.relative_to(ROOT).as_posix() for p in files if not p.exists()],
                   pdf_sha256=hashlib.sha256((PAPER/'statement.pdf').read_bytes()).hexdigest())
    (args.out/'receipt.json').write_text(json.dumps(receipt, indent=2)+'\n', encoding='utf-8')
    return 0 if result.wasSuccessful() else 1


if __name__ == '__main__':
    raise SystemExit(main())
