"""Exact O(g^2) origin-splitting algebra and bounded Lean checks; no spectral runs.

Ground-state picture: H - nS/2 acts on e^(-W) phi as L phi, L = L0 + g L1,
L0 = -Lap/2 + sum_k omega_k X_k . d_k,  L1 = -grad C . grad,  C = <X1,[X2,X3]>.
"""
import argparse
import hashlib
import io
import itertools
import json
from pathlib import Path
import re
import subprocess
import sys
import unittest

import sympy as s
from sympy.combinatorics import Permutation

from check_h7_lean import ROOT, PROJECT, ALLOWED_AXIOMS, run

NOTE = ROOT/'docs/research/h8-origin-splitting-proof-2026-11-27.md'
LEAN = ('H8OriginSplitting.lean', 'H8OriginDeclarations.lean', 'H8OriginVerification.lean')
EXPECTED = ('e2_closed_form', 'e2_negative', 'charge_two_annihilated', 'charge_two_cross_annihilated',
            'charge_zero_image', 'ordering_of_quasimodes')

t = s.symbols('t', positive=True)
U = 1/s.sqrt(t)
OMEGA = (s.sqrt(t), U, U)


def require_zero(expression):
    if s.simplify(s.expand(expression)) != 0:
        raise ValueError('Exact identity failed')


def su2():
    """Cross-product bracket on R^3: structure constants epsilon, kappa = 2."""
    f = {}
    for a, b, c in itertools.permutations(range(3)):
        f[a, b, c] = s.LeviCivita(a, b, c)
    return 3, f


def su3():
    """Gell-Mann structure constants f_abc (Tr t_a t_b = delta/2), kappa = C_A = 3."""
    half, r3 = s.Rational(1, 2), s.sqrt(3)/2
    base = {(0, 1, 2): 1, (0, 3, 6): half, (0, 4, 5): -half, (1, 3, 5): half, (1, 4, 6): half,
            (2, 3, 4): half, (2, 5, 6): -half, (3, 4, 7): r3, (5, 6, 7): r3}
    f = {}
    for (a, b, c), v in base.items():
        for p in itertools.permutations((0, 1, 2)):
            idx = tuple((a, b, c)[i] for i in p)
            f[idx] = v*Permutation(list(p)).signature()
    return 8, f


class Model:
    def __init__(self, n, f, omega=OMEGA, scale=1, cubic=None):
        self.n, self.omega = n, omega
        self.X = [[s.Symbol(f'x{k}_{a}', real=True) for a in range(n)] for k in range(3)]
        self.vars = [v for row in self.X for v in row]
        self.row = {v: k for k, row in enumerate(self.X) for v in row}
        # metric <e_a,e_b> = scale * delta; bracket [e_a,e_b] = sum f_abc e_c
        self.scale = scale

        def br(x, y):
            return [sum(f.get((a, b, c), 0)*x[a]*y[b] for a in range(n) for b in range(n)) for c in range(n)]

        self.br = br
        self.ip = lambda x, y: scale*sum(p*q for p, q in zip(x, y))
        self.C = s.expand(self.ip(self.X[0], br(self.X[1], self.X[2]))) if cubic is None else cubic(self)

    def norm2(self, k):
        return self.ip(self.X[k], self.X[k])

    def lap(self, h):
        # the flat Laplacian of the Euclidean coordinates; with metric scale, coordinates are scale-weighted
        return sum(s.diff(h, v, 2) for v in self.vars)/self.scale

    def L0(self, h):
        return s.expand(-self.lap(h)/2 + sum(self.omega[self.row[v]]*v*s.diff(h, v) for v in self.vars))

    def L1(self, h):
        return s.expand(-sum(s.diff(self.C, v)*s.diff(h, v) for v in self.vars)/self.scale)

    def sigma2(self, v):
        # Gaussian weight exp(-sum omega_k ||X_k||^2): coordinate variance 1/(2 omega_k scale)
        return 1/(2*self.omega[self.row[v]]*self.scale)

    def gauss(self, expr):
        p = s.Poly(s.expand(expr), *self.vars)
        total = 0
        for mon, co in p.terms():
            if any(e % 2 for e in mon):
                continue
            val = co
            for v, e in zip(self.vars, mon):
                if e:
                    val *= s.factorial2(e - 1)*self.sigma2(v)**(e//2)
            total += val
        return s.simplify(total)

    def r(self):
        # charge-0 two-quantum Hermite polynomial at 2u: centred so that it is an L0 eigenfunction
        h = self.norm2(1) + self.norm2(2)
        return s.expand(h - self.gauss(h))

    def e2(self, resolvent_sign=1):
        r = self.r()
        image = self.L1(r)
        energy = self.omega[0] + 2*U  # L0 eigenvalue of C
        f = self.L1(resolvent_sign*image/(2*U - energy))
        return s.simplify(self.gauss(r*f)/self.gauss(r*r))


def energy(model, mon):
    return sum(model.omega[model.row[v]]*e for v, e in zip(model.vars, mon))


def clean(model, expr):
    p = s.Poly(s.expand(expr), *model.vars)
    return s.Add(*[s.simplify(co)*s.prod(v**e for v, e in zip(model.vars, mon)) for mon, co in p.terms()])


def solve_shifted(model, rhs, target):
    """Solve (L0 - target) phi = rhs for a polynomial phi; fail if rhs touches the target eigenspace."""
    phi, residual = 0, clean(model, rhs)
    for _ in range(12):
        if residual == 0:
            return phi
        p = s.Poly(residual, *model.vars)
        top = max(sum(m) for m in p.monoms())
        step = 0
        for mon, co in p.terms():
            if sum(mon) != top:
                continue
            gap = s.simplify(energy(model, mon) - target)
            if gap == 0:
                raise ValueError('Right-hand side has a component at the target level')
            step += co/gap*s.prod(v**e for v, e in zip(model.vars, mon))
        phi = clean(model, phi + step)
        residual = clean(model, rhs - (model.L0(phi) - target*phi))
    raise ValueError('Solver did not terminate')


class Algebra(unittest.TestCase):
    def test_su2_mechanism_and_closed_form(self):
        m = Model(*su2())
        X = m.X
        P = s.expand(m.norm2(1) - m.norm2(2))
        Q = s.expand(m.ip(X[1], X[2]))
        self.assertEqual(m.L1(P), 0)
        self.assertEqual(m.L1(Q), 0)
        require_zero(m.L0(P) - 2*U*P)
        require_zero(m.L0(Q) - 2*U*Q)
        r = m.r()
        require_zero(m.L0(r) - 2*U*r)
        require_zero(m.L1(r) + 4*m.C)
        require_zero(m.L0(m.C) - (s.sqrt(t) + 2*U)*m.C)
        require_zero(m.e2() + 4*(1 + 1/t))

    def test_su2_second_order_quasimode_solves_exactly(self):
        m = Model(*su2())
        r = m.r()
        e2 = -4*(1 + 1/t)
        phi1 = solve_shifted(m, -m.L1(r), 2*U)
        require_zero(phi1 - 4*m.C/s.sqrt(t))
        phi2 = solve_shifted(m, e2*r - m.L1(phi1), 2*U)
        # the three order-by-order equations hold exactly
        require_zero(m.L0(phi2) - 2*U*phi2 + m.L1(phi1) - e2*r)
        # any other e2 leaves a component at 2u and the solver must refuse
        with self.assertRaises(ValueError):
            solve_shifted(m, (e2 + 1)*r - m.L1(phi1), 2*U)

    def test_su3_every_group_formula(self):
        m = Model(*su3())
        P = s.expand(m.norm2(1) - m.norm2(2))
        self.assertEqual(m.L1(P), 0)
        self.assertEqual(m.L1(s.expand(m.ip(m.X[1], m.X[2]))), 0)
        require_zero(m.L1(m.r()) + 4*m.C)
        require_zero(m.e2() + 2*3*(1 + 1/t))

    def test_casimir_values(self):
        for (n, f), kappa in ((su2(), 2), (su3(), 3)):
            for a in range(n):
                for b in range(n):
                    val = sum(f.get((a, c, d), 0)*f.get((b, c, d), 0) for c in range(n) for d in range(n))
                    self.assertEqual(s.nsimplify(val), kappa if a == b else 0)

    def test_rescaled_metric_rescales_kappa(self):
        # metric scale lam: orthonormal basis e/sqrt(lam), kappa -> kappa/lam with the same structure constants
        lam = s.Rational(3)
        m = Model(*su2(), scale=lam)
        require_zero(m.e2() + 2*(2/lam)*(1 + 1/t))

    def test_nondegeneracy_at_2u(self):
        # Hermite energies d1 sqrt(t) + d23 u equal 2u at t > 1 only for (d1, d23) = (0, 2),
        # or (1, 0) at t = 2 (linear in X1: no singlet, since the adjoint has no invariant vector)
        tt = s.symbols('tt', positive=True)
        for d1 in range(5):
            for d23 in range(5):
                sol = [v for v in s.solve(s.Eq(d1*s.sqrt(tt) + d23/s.sqrt(tt), 2/s.sqrt(tt)), tt) if v > 1]
                if (d1, d23) == (0, 2):
                    continue
                self.assertTrue(sol == [] or (d1, d23, sol) == (1, 0, [2]), (d1, d23, sol))
        # Schur: invariant symmetric bilinear forms on su(2), su(3) are one-dimensional
        for n, f in (su2(), su3()):
            B = s.Matrix(n, n, lambda a, b: s.Symbol(f'B{min(a, b)}_{max(a, b)}'))
            eqs = []
            for c in range(n):
                ad = s.Matrix(n, n, lambda i, j: f.get((c, j, i), 0))
                eqs += list(ad.T*B + B*ad)
            sol = s.solve(eqs, list(B.free_symbols), dict=True)[0]
            self.assertEqual(len(B.subs(sol).free_symbols), 1)


class Mutations(unittest.TestCase):
    def test_non_invariant_cubic_breaks_charge_two(self):
        # ||X2||^2 - ||X3||^2 is killed by every row-trilinear cubic (Euler homogeneity);
        # <X2,X3> needs ad-invariance, so a non-invariant trilinear cubic must break it
        def bad(m):
            x, y, z = m.X
            return s.expand(m.ip(x, m.br(y, z)) + x[0]*y[0]*z[1])
        m = Model(*su2(), cubic=bad)
        self.assertEqual(m.L1(s.expand(m.norm2(1) - m.norm2(2))), 0)
        self.assertNotEqual(m.L1(s.expand(m.ip(m.X[1], m.X[2]))), 0)

    def test_unequal_frequencies_break_the_eigenfunction(self):
        m = Model(*su2(), omega=(s.sqrt(t), U, 2*U))
        P = s.expand(m.norm2(1) - m.norm2(2))
        with self.assertRaises(ValueError):
            require_zero(m.L0(P) - 2*U*P)

    def test_no_cubic_no_splitting(self):
        m = Model(*su2(), cubic=lambda m: s.Integer(0))
        self.assertEqual(s.simplify(m.e2()), 0)

    def test_reversed_resolvent_flips_the_sign(self):
        m = Model(*su2())
        require_zero(m.e2(resolvent_sign=-1) - 4*(1 + 1/t))


class Document(unittest.TestCase):
    def test_written_proof_and_scope(self):
        self.assertTrue(NOTE.exists(), 'Missing origin-splitting proof')
        text = ' '.join(NOTE.read_text(encoding='utf-8').split())
        for marker in ('−2κ(1 + 1/t)', 'quasimode', 'Schur', 'H2b', 'not end-to-end Lean',
                       'H7 and H8 remain open', 'Simon', 't > 1'):
            self.assertIn(marker, text)


def audit(output):
    found = {name: sorted(v.strip() for v in deps.split(',') if v.strip())
             for name, deps in re.findall(r"'(H8Origin\.\w+)' depends on axioms: \[([^\]]*)\]", output, re.S)}
    for name in re.findall(r"'(H8Origin\.\w+)' does not depend on any axioms", output):
        found[name] = []
    for suffix in EXPECTED:
        key = 'H8Origin.'+suffix
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
                   scope='Exact polynomial algebra and conditional Lean deduction; localization cited, not formalized',
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
            if result.errors or len(result.failures) != 1 or 'Missing origin-splitting proof' not in stream.getvalue():
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
        entry, output = run(['lake', 'env', 'lean', 'H8OriginDeclarations.lean'], out/'declarations.log')
        receipt['runs'].append(entry)
        if args.expect_red:
            if not entry['exit_code'] or any('H8Origin.'+n not in output for n in EXPECTED):
                raise ValueError('Missing declarations did not fail as expected')
            receipt['expected_red_observed'] = True
            return 0
        if entry['exit_code']:
            raise ValueError('Declaration audit failed')
        entry, output = run(['lake', 'env', 'lean', 'H8OriginVerification.lean'], out/'verification.log')
        receipt['runs'].append(entry)
        if entry['exit_code']:
            raise ValueError('Lean verification failed')
        receipt['axioms'] = audit(output)
        for poison in ('sorryAx', 'UnverifiedLocalization'):
            try:
                audit(output.replace('propext', poison))
            except ValueError:
                continue
            raise ValueError('Axiom auditor accepted '+poison)
        receipt['audit_rejections'] = ['sorryAx', 'UnverifiedLocalization']
        source = (PROJECT/'H8OriginSplitting.lean').read_text(encoding='utf-8')
        mutations = {
            'missing_invariance': ('(hinv : ∀ x y z, B (br x y) z = B x (br y z)) (hsym : ∀ x y, B x y = B y x)\n    (hanti',
                                   '(hinv : True) (hsym : ∀ x y, B x y = B y x)\n    (hanti'),
            'wrong_e2_factor': ('-(2 * κ / w1) * (1 / u + 1 / w1) = -2 * κ * (1 + 1 / t)',
                                '-(2 * κ / w1) * (1 / u + 1 / w1) = -4 * κ * (1 + 1 / t)'),
            'missing_quasimode_bound': ('(h0 : |E0 - (c + e2 * g^2)| ≤ K * g^3)', '(h0 : True)'),
            'wrong_window': ('(hg : K * g < -e2 / 2)', '(hg : K * g < -e2 * 2)'),
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
                 ROOT/'docs/research/h8-origin-splitting-contract-2026-11-27.md',
                 ROOT/'docs/research/h8-vacuum-floor-proof-2026-11-27.md',
                 ROOT/'docs/research/closed-set-20261127/H7-every-group.md',
                 ROOT/'docs/research/closed-set-20261127/H8-s1-carrier.md',
                 ROOT/'docs/research/closed-set-20261127/CLOSED-SET.md',
                 PROJECT/'H8Origin-README.md',
                 *PROJECT.glob('*.lean'), PROJECT/'lakefile.toml', PROJECT/'lake-manifest.json', PROJECT/'lean-toolchain']
        receipt['sha256_lf'] = {p.relative_to(ROOT).as_posix(): hashlib.sha256(p.read_bytes().replace(b'\r\n', b'\n')).hexdigest()
                                for p in files if p.exists()}
        (out/'receipt.json').write_text(json.dumps(receipt, indent=2)+'\n', encoding='utf-8')
        print(json.dumps({k: receipt[k] for k in ('mode', 'passed', 'failures')}, indent=2))


if __name__ == '__main__':
    raise SystemExit(main())
