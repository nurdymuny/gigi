"""Exact checks for the H8 charge tail (horizontal barrier, zero-point charge floor) and bounded Lean checks.

No spectral run, no eigenvalue, no numerical diagonalization. Every identity is exact polynomial algebra.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path
import random
import re
import subprocess
import sys
import unittest

import sympy as s

from check_h7_lean import ROOT, PROJECT, ALLOWED_AXIOMS, run
from check_h8_origin_splitting import su2, su3

NOTE = ROOT/'docs/research/h8-charge-tail-proof-2026-11-27.md'
CONTRACT = ROOT/'docs/research/h8-charge-tail-contract-2026-11-27.md'
LEAN = ('H8ChargeTail.lean', 'H8ChargeTailDeclarations.lean', 'H8ChargeTailVerification.lean')
EXPECTED = ('gauge_rows', 'ruler', 'zero_point_floor', 'large_charge_floor', 't3_floor_closed_form')

t, g = s.symbols('t g', positive=True)
U = 1/s.sqrt(t)
OMEGA = (s.sqrt(t), U, U)
S_SUM = s.sqrt(t) + 2*U


def require_zero(expression):
    if s.simplify(s.expand(expression)) != 0:
        raise ValueError('Exact identity failed')


def is_zero(expression):
    return s.simplify(s.expand(expression)) == 0


class Rows:
    """q = (X1, X2, X3), X_k in g; W = 1/2 sum omega_k |X_k|^2 - g C, gradient by differentiation."""

    def __init__(self, n, f, omega=OMEGA, cubic=None):
        self.n, self.f, self.omega = n, f, omega
        self.X = [[s.Symbol(f'x{k}_{a}') for a in range(n)] for k in range(3)]
        self.flat = [x for row in self.X for x in row]
        self.C = cubic(self.X) if cubic else self.dot(self.X[0], self.br(self.X[1], self.X[2]))
        self.W = sum(w*self.dot(x, x) for w, x in zip(omega, self.X))/2 - g*self.C
        self.grad = [[s.diff(self.W, x) for x in row] for row in self.X]

    def br(self, x, y):
        out = [0]*self.n
        for (a, b, c), v in self.f.items():
            out[c] += v*x[a]*y[b]
        return out

    @staticmethod
    def dot(x, y):
        return sum(a*b for a, b in zip(x, y))

    def norm2(self, rows):
        return sum(self.dot(r, r) for r in rows)

    def circle(self):
        return [[0]*self.n, [-x for x in self.X[2]], list(self.X[1])]

    def gauge(self, Y):
        return [self.br(Y, x) for x in self.X]

    def V(self):
        return self.norm2(self.grad)/2

    def lap(self, h):
        return sum(s.diff(h, x, 2) for x in self.flat)

    def barrier_residual(self):
        Y = [-g*s.sqrt(t)*x for x in self.X[0]]
        diff = [[a - b for a, b in zip(r, q)] for r, q in zip(self.circle(), self.gauge(Y))]
        tail = [a - g/s.sqrt(t)*b for a, b in zip(self.X[0], self.br(self.X[1], self.X[2]))]
        return t*self.norm2(self.grad) - self.norm2(diff) - t**2*self.dot(tail, tail)

    def hor2(self, values):
        """Exact squared distance from the circle vector to the colour-orbit tangent at a point."""
        Y = s.symbols(f'y0:{self.n}')
        circ = [[s.sympify(c).subs(values) for c in r] for r in self.circle()]
        gau = [[s.sympify(c).subs(values) for c in r] for r in self.gauge(Y)]
        F = sum((a - b)**2 for r, q in zip(circ, gau) for a, b in zip(r, q))
        sol = s.solve([s.diff(F, y) for y in Y], Y, dict=True)
        return s.nsimplify(s.simplify(F.subs(sol[0]).subs({y: 0 for y in Y})))


def rational_point(model, seed):
    rng = random.Random(seed)
    return {x: s.Rational(rng.randint(-9, 9), rng.randint(1, 4)) for x in model.flat}


def floor(m, tt, n=3):
    sigma = n*(s.sqrt(tt) + 2/s.sqrt(tt))/2
    c = s.Rational(m*m, 4)/tt
    return sigma + c/sigma if c <= sigma**2 else 2*s.sqrt(c)


class Algebra(unittest.TestCase):
    def test_step1_frozen_object(self):
        m = Rows(*su2())
        q = s.Matrix(m.X)
        cof = q.adjugate().T
        for k in range(3):
            for c in range(3):
                require_zero(m.grad[k][c] - (OMEGA[k]*q[k, c] - g*cof[k, c]))
        R = s.Matrix([[1, 0, 0], [0, s.Rational(3, 5), -s.Rational(4, 5)], [0, s.Rational(4, 5), s.Rational(3, 5)]])
        O = s.Matrix([[s.Rational(5, 13), -s.Rational(12, 13), 0], [s.Rational(12, 13), s.Rational(5, 13), 0], [0, 0, 1]])
        V = m.V()
        for M in (R*q, q*O.T):
            require_zero(V - V.subs(dict(zip(m.flat, list(M))), simultaneous=True))
        G = q*q.T
        gram = (t*G[0, 0] + (G[1, 1] + G[2, 2])/t)/2 - g*S_SUM*q.det() + g**2/4*(G.trace()**2 - (G*G).trace())
        require_zero(V - gram)
        require_zero(m.lap(m.W) - 3*S_SUM)

    def test_gram_horizontal_formula_at_full_rank(self):
        m = Rows(*su2())
        J = s.Matrix([[0, 0, 0], [0, 0, -1], [0, 1, 0]])
        ratios = []
        for seed in range(4):
            val = rational_point(m, seed)
            q = s.Matrix(m.X).subs(val)
            G = q*q.T
            self.assertNotEqual(q.det(), 0)
            Gdot = J*G - G*J
            Y = s.Matrix(3, 3, s.symbols('z0:9'))
            sol = s.solve(list(G*Y + Y*G - Gdot), list(Y), dict=True)[0]
            formula = (Gdot*Y.subs(sol)).trace()/2
            ratios.append(s.simplify(formula/m.hor2(val)))
        self.assertEqual(set(ratios), {1})

    def test_barrier_identity_su2(self):
        require_zero(Rows(*su2()).barrier_residual())

    def test_barrier_identity_su3(self):
        require_zero(Rows(*su3()).barrier_residual())

    def test_barrier_against_direct_projection(self):
        m = Rows(*su2())
        for seed in range(4):
            val = rational_point(m, 100 + seed)
            val.update({t: 3, g: s.Rational(1, 7)})
            lhs, rhs = m.hor2(val), (t*m.norm2(m.grad)).subs(val)
            self.assertTrue(bool(lhs <= rhs))

    def test_sharpness(self):
        m = Rows(*su2())
        sv, mv = s.symbols('s_ m_', positive=True)
        point = {x: 0 for x in m.flat}
        point[m.X[1][0]] = sv
        self.assertEqual(s.simplify(m.hor2(point) - sv**2), 0)
        require_zero(m.norm2(m.grad).subs(point) - sv**2/t)
        ruler = (sv**2/t)/2 + mv**2/(2*sv**2)
        best = s.solve(s.diff(ruler, sv), sv)
        self.assertEqual([s.simplify(ruler.subs(sv, b) - mv/s.sqrt(t)) for b in best], [0])

    def test_twisted_ingredients(self):
        for n, f in (su2(), su3()):
            m = Rows(n, f)
            Y = s.symbols(f'y0:{n}')
            for rows in (m.circle(), m.gauge(list(Y))):
                require_zero(sum(a*b for r, q in zip(rows, m.grad) for a, b in zip(r, q)))
            require_zero(m.lap(m.W) - n*S_SUM)
        m = Rows(*su2())
        F = s.Function('f')(*m.flat)
        lhs = 0
        for x in m.flat:
            dW = s.diff(m.W, x)
            a_f = s.diff(F, x) + dW*F
            lhs += -s.diff(a_f, x) + dW*a_f
        rhs = -m.lap(F) + 2*m.V()*F - m.lap(m.W)*F
        require_zero(lhs - rhs)

    def test_zero_point_floor_closed_form(self):
        sig, c = s.symbols('sigma c', positive=True)
        E, P = sig + c/sig, sig
        require_zero(P*(E - P) - c)
        require_zero(P*(E - sig) - c)
        for mm in range(1, 16):
            require_zero(floor(mm, 3) - s.Rational(225 + mm*mm, 30)/s.sqrt(3))
        self.assertEqual(floor(16, 3), 16/s.sqrt(3))
        harmonic = 3*(s.sqrt(3) + 2/s.sqrt(3))/2 + (s.sqrt(11) - s.sqrt(3))/2
        cut = [mm for mm in range(1, 30) if floor(mm, 3) > harmonic]
        self.assertEqual(cut[0], 7)
        self.assertEqual(cut, list(range(7, 30)))
        self.assertEqual([round(float(floor(mm, 3)), 4) for mm in (7, 8, 9)], [5.2731, 5.5618, 5.889])


class Mutations(unittest.TestCase):
    def test_unequal_frequencies_break_the_circle(self):
        m = Rows(*su2(), omega=(s.sqrt(t), U, 2*U))
        self.assertFalse(is_zero(sum(a*b for r, q in zip(m.circle(), m.grad) for a, b in zip(r, q))))
        self.assertFalse(is_zero(m.barrier_residual()))

    def test_non_invariant_cubic_breaks_both(self):
        m = Rows(*su2(), cubic=lambda X: X[0][0]*X[1][0]*X[2][1])
        self.assertFalse(is_zero(m.barrier_residual()))
        Y = s.symbols('y0:3')
        self.assertFalse(is_zero(sum(a*b for r, q in zip(m.gauge(list(Y)), m.grad) for a, b in zip(r, q))))

    def test_unprojected_circle_fails_at_background(self):
        m = Rows(*su2())
        point = {x: 0 for x in m.flat}
        point.update({m.X[0][0]: U/g, m.X[1][1]: 1/g, m.X[2][2]: 1/g})
        require_zero(m.norm2(m.grad).subs(point))
        self.assertFalse(is_zero(m.norm2(m.circle()).subs(point)))
        require_zero(m.hor2(point))

    def test_zero_point_removed_leaves_seven_uncut(self):
        harmonic = 3*(s.sqrt(3) + 2/s.sqrt(3))/2 + (s.sqrt(11) - s.sqrt(3))/2
        self.assertTrue(bool(7/s.sqrt(3) < harmonic))
        self.assertTrue(bool(floor(7, 3) > harmonic))

    def test_inflated_constant_fails_at_equality(self):
        m = Rows(*su2())
        point = {x: 0 for x in m.flat}
        point[m.X[1][0]] = 1
        lhs = m.norm2(m.grad).subs(point)
        self.assertTrue(bool((1 + s.Rational(1, 100))*m.hor2(point)/t > lhs))


class Document(unittest.TestCase):
    def test_written_proof_and_scope(self):
        self.assertTrue(NOTE.exists(), 'Missing charge-tail proof')
        text = ' '.join(NOTE.read_text(encoding='utf-8').split())
        for marker in ('|∇W|² ≥ |X_hor|²/t', 'every rank stratum', 'σ + m²/(4tσ)', '(225 + m²)/(30√3)',
                       'Step 3', 'not end-to-end Lean', 'H8 remains open', 'H2b'):
            self.assertIn(marker, text)


def audit(output):
    found = {name: sorted(v.strip() for v in deps.split(',') if v.strip())
             for name, deps in re.findall(r"'(H8ChargeTail\.\w+)' depends on axioms: \[([^\]]*)\]", output, re.S)}
    for name in re.findall(r"'(H8ChargeTail\.\w+)' does not depend on any axioms", output):
        found[name] = []
    for suffix in EXPECTED:
        key = 'H8ChargeTail.'+suffix
        if key not in found or set(found[key])-ALLOWED_AXIOMS:
            raise ValueError('Missing or untrusted proof dependency: '+key)
    return found


MUTATIONS = {
    'missing_antisymmetry': ('(hanti : ∀ x y, br x y = -br y x)', '(hanti : True)'),
    'missing_small_charge': ('(hsmall : c ≤ σ ^ 2)', '(hsmall : True)'),
    'missing_twisted_bound': ('(hB : c ≤ P * (E - σ))', '(hB : True)'),
    'doubled_ruler': ('|m| / Real.sqrt t ≤ s ^ 2 / (2 * t) + m ^ 2 / (2 * s ^ 2)',
                      '2 * |m| / Real.sqrt t ≤ s ^ 2 / (2 * t) + m ^ 2 / (2 * s ^ 2)'),
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('out', type=Path)
    parser.add_argument('--expect-red', action='store_true')
    args = parser.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    receipt = dict(passed=False, mode='red' if args.expect_red else 'release',
                   scope='Exact polynomial algebra and bounded Lean deduction; no eigenvalue is computed',
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
            if result.errors or len(result.failures) != 1 or 'Missing charge-tail proof' not in stream.getvalue():
                raise ValueError('Expected only the missing-proof red test')
        elif not result.wasSuccessful():
            raise ValueError('Exact or written-scope gate failed')
        manifest = json.loads((PROJECT/'lake-manifest.json').read_text())
        receipt['dependency_revisions'] = {p['name']: p['rev'] for p in manifest['packages']}
        receipt['toolchain'] = (PROJECT/'lean-toolchain').read_text().strip()
        if receipt['dependency_revisions']['mathlib'] != 'fc4f59e577feb1a4956671bc04ac18aaaf8cd842' or receipt['toolchain'] != 'leanprover/lean4:v4.36.0':
            raise ValueError('Changed dependency pin')
        for file in LEAN:
            path = PROJECT/file
            if not path.exists() and args.expect_red:
                continue
            if re.search(r'\b(sorry|admit|sorryAx|axiom|native_decide|unsafe)\b', path.read_text(encoding='utf-8')):
                raise ValueError('Forbidden shortcut: '+file)
        entry, _ = run(['lake', 'build'], out/'build.log')
        receipt['runs'].append(entry)
        if entry['exit_code']:
            raise ValueError('Lean build failed')
        if args.expect_red:
            decl = out/'H8ChargeTailDeclarations.lean'
            decl.write_text('import H7CubicQuartic\n\n' + ''.join(f'#check H8ChargeTail.{n}\n' for n in EXPECTED),
                            encoding='utf-8')
            entry, output = run(['lake', 'env', 'lean', str(decl)], out/'declarations.log')
        else:
            entry, output = run(['lake', 'env', 'lean', 'H8ChargeTailDeclarations.lean'], out/'declarations.log')
        receipt['runs'].append(entry)
        if args.expect_red:
            if not entry['exit_code'] or any('H8ChargeTail.'+n not in output for n in EXPECTED):
                raise ValueError('Missing declarations did not fail as expected')
            receipt['expected_red_observed'] = True
            return 0
        if entry['exit_code']:
            raise ValueError('Declaration audit failed')
        entry, output = run(['lake', 'env', 'lean', 'H8ChargeTailVerification.lean'], out/'verification.log')
        receipt['runs'].append(entry)
        if entry['exit_code']:
            raise ValueError('Lean verification failed')
        receipt['axioms'] = audit(output)
        for poison in ('sorryAx', 'UnverifiedProjection'):
            try:
                audit(output.replace('propext', poison))
            except ValueError:
                continue
            raise ValueError('Axiom auditor accepted '+poison)
        receipt['audit_rejections'] = ['sorryAx', 'UnverifiedProjection']
        source = (PROJECT/'H8ChargeTail.lean').read_text(encoding='utf-8')
        for name, (old, new) in MUTATIONS.items():
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
        receipt['mutations_rejected'] = list(MUTATIONS)
        receipt['passed'] = True
        return 0
    except Exception as exc:
        receipt['failures'].append(str(exc))
        return 1
    finally:
        files = [Path(__file__), ROOT/'scripts/check_h7_lean.py', ROOT/'scripts/check_h8_origin_splitting.py',
                 NOTE, CONTRACT, ROOT/'docs/research/h8-rest-of-the-route-2026-11-27.md',
                 ROOT/'docs/research/closed-set-20261127/H8-s1-carrier.md',
                 ROOT/'docs/research/closed-set-20261127/CLOSED-SET.md',
                 PROJECT/'H8ChargeTail-README.md',
                 *PROJECT.glob('*.lean'), PROJECT/'lakefile.toml', PROJECT/'lake-manifest.json', PROJECT/'lean-toolchain']
        receipt['sha256_lf'] = {p.relative_to(ROOT).as_posix(): hashlib.sha256(p.read_bytes().replace(b'\r\n', b'\n')).hexdigest()
                                for p in files if p.exists()}
        (out/'receipt.json').write_text(json.dumps(receipt, indent=2)+'\n', encoding='utf-8')
        print(json.dumps({k: receipt[k] for k in ('mode', 'passed', 'failures')}, indent=2))


if __name__ == '__main__':
    raise SystemExit(main())
