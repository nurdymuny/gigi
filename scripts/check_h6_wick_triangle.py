"""Exact, adversarial gates for the documented free-field Wick triangle."""
import argparse
import hashlib
import io
import itertools
import json
from pathlib import Path
import re
import subprocess
import sys
import types
import unittest
from unittest.mock import patch

import sympy as s

import helike_wick_triangle as engine
from check_h7_lean import ROOT, PROJECT, ALLOWED_AXIOMS, run

NOTE = ROOT/'docs/research/h6-wick-triangle-proof-2026-11-27.md'
EXPECTED = ('odd_product', 'odd_conjugation', 'trace_conjugation',
            'odd_trace_zero', 'triangle_zero')
PAIRS = list(itertools.combinations(range(4), 2))
X = s.symbols('x0:4', real=True)


def zero(value):
    values = list(value) if isinstance(value, s.MatrixBase) else [value]
    return all(s.cancel(x) == 0 for x in values)


def reflection_covariance(r):
    r = s.Matrix(r)
    r2 = r.dot(r)
    R = s.eye(4)-2*r*r.T/r2
    return s.Matrix(6, 6, lambda a, b:
                    (R[PAIRS[a][0], PAIRS[b][0]]*R[PAIRS[a][1], PAIRS[b][1]]
                     -R[PAIRS[a][0], PAIRS[b][1]]*R[PAIRS[a][1], PAIRS[b][0]])/(s.pi**2*r2**2))


def blocks(points, covariance=None):
    covariance = covariance or engine.field_covariance
    return {(i, j): covariance(s.Matrix(points[i])-s.Matrix(points[j]))
            for i in range(len(points)) for j in range(i+1, len(points))}


def collinear(n):
    return [(i, 0, 0, 0) for i in range(n)]


def assert_nonzero_controls(calculate):
    two = calculate(blocks(collinear(2)))
    four = calculate(blocks(collinear(4)))
    if not zero(two-48/s.pi**4) or not zero(four-s.Rational(676, 27)/s.pi**8):
        raise ValueError('Nonzero Maxwell control failed')


class Exact(unittest.TestCase):
    def test_covariance_all_entries_and_normalization(self):
        actual = engine.field_covariance(X)
        self.assertTrue(zero(actual-reflection_covariance(X)))
        self.assertTrue(zero(actual-actual.T))
        self.assertEqual(engine.field_covariance((1, 0, 0, 0)),
                         s.diag(-1, -1, -1, 1, 1, 1)/s.pi**2)

    def test_hodge_involution_and_symbolic_exchange(self):
        J, C = engine.hodge_star(), engine.field_covariance(X)
        self.assertEqual(J*J, s.eye(6))
        self.assertTrue(zero(J*C+C*J))
        for sign in (-1, 1):
            P = (s.eye(6)+sign*J)/2
            self.assertTrue(zero(P*C*P))

    def test_pairing_counts(self):
        for n, count in ((2, 2), (3, 8), (4, 48)):
            pairings = engine.connected_pairings(n)
            self.assertEqual(len(pairings), count)
            self.assertEqual(len(set(pairings)), count)
            for pairing in pairings:
                self.assertEqual(sorted(v for pair in pairing for v in pair), list(range(2*n)))
                self.assertTrue(all(a//2 != b//2 for a, b in pairing))

    def test_triangle_direct_wick_and_trace_agree(self):
        fixtures = [((0, 0, 0, 0), (1, 2, 0, 1), (-1, 0, 3, 2)),
                    ((2, -1, 0, 3), (0, 1, 2, -1), (4, 2, 1, 0))]
        for points in fixtures:
            cov = blocks(points)
            direct = engine.connected_quadratic(cov, colours=3)
            trace = 64*3*s.trace(cov[0, 1]*cov[1, 2]*cov[0, 2].T)
            self.assertTrue(zero(direct-trace))
            self.assertEqual(direct, 0)

    def test_trace_factor_on_noncancelling_synthetic_covariances(self):
        # Not Maxwell: this prevents a zero/incorrect-prefactor trace oracle.
        A, B, C = s.diag(1, 2), s.Matrix([[3, 1], [1, 4]]), s.diag(5, 6)
        cov = {(0, 1): A, (1, 2): B, (0, 2): C}
        self.assertEqual(engine.connected_quadratic(cov, colours=2), 128*s.trace(A*B*C))
        self.assertNotEqual(s.trace(A*B*C), 0)

    def test_nonzero_two_and_four_point_controls(self):
        assert_nonzero_controls(engine.connected_quadratic)
        self.assertEqual(engine.connected_quadratic(blocks(collinear(2)), colours=3), 144/s.pi**4)

    def test_scalar_triangle_is_nonzero(self):
        points = (0, 1, 3)
        cov = {(i, j): s.Matrix([[1/(4*s.pi**2*(points[i]-points[j])**2)]])
               for i in range(3) for j in range(i+1, 3)}
        self.assertEqual(engine.connected_quadratic(cov, scale=1), 1/(288*s.pi**6))

    def test_bad_evidence_and_mechanisms_rejected(self):
        J, C = engine.hodge_star(), engine.field_covariance(X)
        broken_J = J.copy()
        broken_J[0, 5] = -broken_J[0, 5]
        self.assertNotEqual(broken_J*broken_J, s.eye(6))
        broken_C = C.copy()
        broken_C[0, 0] += 1
        self.assertFalse(zero(broken_J*C+C*broken_J))
        self.assertFalse(zero(J*broken_C+broken_C*J))
        with self.assertRaisesRegex(ValueError, 'control failed'):
            assert_nonzero_controls(lambda *args, **kwargs: s.Integer(0))
        with self.assertRaisesRegex(ValueError, 'control failed'):
            assert_nonzero_controls(lambda cov: engine.connected_quadratic(cov, scale=1))
        original = engine.connected_pairings
        with patch.object(engine, 'connected_pairings', lambda n: original(n)[:-1]):
            with self.assertRaisesRegex(ValueError, 'control failed'):
                assert_nonzero_controls(engine.connected_quadratic)

    def test_input_refusals(self):
        with self.assertRaises(ValueError):
            engine.field_covariance((0, 0, 0, 0))
        with self.assertRaises(ValueError):
            engine.field_covariance((1, 0, 0))
        with self.assertRaises(ValueError):
            engine.field_covariance((1.0, 0, 0, 0))
        with self.assertRaises(ValueError):
            engine.field_covariance((s.oo, 0, 0, 0))
        with self.assertRaises(ValueError):
            engine.connected_quadratic({(0, 1): s.eye(2), (1, 2): s.eye(3), (0, 2): s.eye(2)})
        with self.assertRaises(ValueError):
            engine.connected_quadratic({(0, 1): s.eye(2), (1, 2): s.eye(2)})
        with self.assertRaises(ValueError):
            engine.connected_pairings(5)


class Document(unittest.TestCase):
    def test_original_conjecture_is_resolved_with_scope(self):
        self.assertTrue(NOTE.exists(), 'Missing Wick-triangle proof')
        text = ' '.join(NOTE.read_text(encoding='utf-8').split())
        for marker in ('REFUTED', 'pairwise distinct', '64', '676', 'contact terms',
                       'not end-to-end', 'H6 remains open', 'additive'):
            self.assertIn(marker.lower(), text.lower())


def audit(output):
    found = {name: sorted(v.strip() for v in deps.split(',') if v.strip())
             for name, deps in re.findall(r"'(H6Wick\.\w+)' depends on axioms: \[([^\]]*)\]", output, re.S)}
    for name in re.findall(r"'(H6Wick\.\w+)' does not depend on any axioms", output):
        found[name] = []
    for suffix in EXPECTED:
        key = 'H6Wick.'+suffix
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
                   scope='Exact free separated-point contractions and conditional finite-matrix Lean theorem',
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
            if len(result.failures) != 1 or len(result.errors) != 9 or any(
                    'NotImplementedError: Wick triangle engine not implemented' not in log for _, log in result.errors):
                raise ValueError('Expected nine missing-engine errors and one missing-proof failure')
        elif not result.wasSuccessful():
            raise ValueError('Exact or written-scope gate failed')
        if not args.expect_red:
            core = (ROOT/'scripts/helike_wick_triangle.py').read_text(encoding='utf-8')
            core_mutations = {
                'drop_derivative_term': ('+s.KroneckerDelta(PAIRS[a][0], PAIRS[b][0])*D[PAIRS[a][1], PAIRS[b][1]]',
                                         '+0*s.KroneckerDelta(PAIRS[a][0], PAIRS[b][0])*D[PAIRS[a][1], PAIRS[b][1]]'),
                'always_zero': ('return s.factor(colours*scale**n*answer)', 'return s.Integer(0)'),
                'wrong_observable_scale': ('colours*scale**n*answer', 'colours*scale*answer'),
                'missing_pairing': ('return tuple(result)', 'return tuple(result[:-1])'),
            }
            receipt['engine_mutations'] = {}
            for name, (old, new) in core_mutations.items():
                if core.count(old) != 1:
                    raise ValueError('Missing engine mutation anchor: '+name)
                path = out/(name+'.py')
                path.write_text(core.replace(old, new, 1), encoding='utf-8')
                mutated = types.ModuleType(name)
                exec(compile(path.read_text(encoding='utf-8'), str(path), 'exec'), mutated.__dict__)
                log = io.StringIO()
                with patch.object(sys.modules[__name__], 'engine', mutated):
                    bad = unittest.TextTestRunner(log, verbosity=2).run(
                        unittest.defaultTestLoader.loadTestsFromTestCase(Exact))
                (out/(name+'.log')).write_text(log.getvalue(), encoding='utf-8')
                if not bad.failures:
                    raise ValueError('Broken engine did not produce assertion failures: '+name)
                receipt['engine_mutations'][name] = dict(assertion_failures=len(bad.failures), errors=len(bad.errors))
        manifest = json.loads((PROJECT/'lake-manifest.json').read_text())
        receipt['dependency_revisions'] = {p['name']: p['rev'] for p in manifest['packages']}
        receipt['toolchain'] = (PROJECT/'lean-toolchain').read_text().strip()
        if receipt['dependency_revisions']['mathlib'] != 'fc4f59e577feb1a4956671bc04ac18aaaf8cd842' or receipt['toolchain'] != 'leanprover/lean4:v4.36.0':
            raise ValueError('Changed dependency pin')
        for file in ('H6WickTriangle.lean', 'H6WickDeclarations.lean', 'H6WickVerification.lean'):
            path = PROJECT/file
            if not path.exists() and args.expect_red:
                continue
            if re.search(r'\b(sorry|admit|sorryAx|axiom|native_decide|unsafe)\b', path.read_text(encoding='utf-8')):
                raise ValueError('Forbidden shortcut: '+file)
        entry, _ = run(['lake', 'build'], out/'build.log')
        receipt['runs'].append(entry)
        if entry['exit_code']:
            raise ValueError('Lean build failed')
        entry, output = run(['lake', 'env', 'lean', 'H6WickDeclarations.lean'], out/'declarations.log')
        receipt['runs'].append(entry)
        if args.expect_red:
            if not entry['exit_code'] or any('Unknown identifier `H6Wick.'+n+'`' not in output for n in EXPECTED):
                raise ValueError('Missing declarations did not fail as expected')
            receipt['expected_red_observed'] = True
            return 0
        if entry['exit_code']:
            raise ValueError('Declaration audit failed')
        entry, output = run(['lake', 'env', 'lean', 'H6WickVerification.lean'], out/'verification.log')
        receipt['runs'].append(entry)
        if entry['exit_code']:
            raise ValueError('Lean verification failed')
        receipt['axioms'] = audit(output)
        for poison in ('sorryAx', 'UnprovedHodge'):
            try:
                audit(output.replace('propext', poison))
            except ValueError:
                continue
            raise ValueError('Axiom auditor accepted '+poison)
        receipt['audit_rejections'] = ['sorryAx', 'UnprovedHodge']
        source = (PROJECT/'H6WickTriangle.lean').read_text(encoding='utf-8')
        mutations = {
            'missing_third_exchange': ('hC : S * C = -(C * S)', 'hC : True'),
            'missing_involution': ('hSS : S * S = 1', 'hSS : True'),
            'wrong_conjugation_sign': ('S * A * S = -A', 'S * A * S = A'),
        }
        for name, (old, new) in mutations.items():
            if old not in source:
                raise ValueError('Missing mutation anchor: '+name)
            path = out/(name+'.lean')
            path.write_text(source.replace(old, new), encoding='utf-8')
            entry, output = run(['lake', 'env', 'lean', str(path)], out/(name+'.log'))
            receipt['runs'].append(entry)
            if not entry['exit_code'] or not re.search(r'error[:(]', output):
                raise ValueError('Bad mechanism accepted: '+name)
            if re.search(r'unknown module|object file .* does not exist', output):
                raise ValueError('Import failure is not a valid negative gate')
        receipt['mutations_rejected'] = list(mutations)
        receipt['values'] = dict(T_O='0 for every pairwise-distinct triple in free R4',
                                 O_two_point_unit_separation='48 n / pi^4',
                                 O_four_point_collinear_0_1_2_3='676 n / (27 pi^8)',
                                 scalar_triangle_0_1_3='1 / (288 pi^6)')
        receipt['passed'] = True
        return 0
    except Exception as exc:
        receipt['failures'].append(str(exc))
        return 1
    finally:
        files = [Path(__file__), ROOT/'scripts/helike_wick_triangle.py', ROOT/'scripts/check_h7_lean.py', NOTE,
                 ROOT/'docs/research/h6-wick-triangle-contract-2026-11-27.md',
                 *PROJECT.glob('H6Wick*'), PROJECT/'lakefile.toml', PROJECT/'lake-manifest.json', PROJECT/'lean-toolchain']
        receipt['sha256_lf'] = {p.relative_to(ROOT).as_posix():hashlib.sha256(p.read_bytes().replace(b'\r\n', b'\n')).hexdigest()
                                for p in files if p.is_file()}
        (out/'receipt.json').write_text(json.dumps(receipt, indent=2)+'\n', encoding='utf-8')
        print(json.dumps({k:receipt[k] for k in ('mode', 'passed', 'failures')}, indent=2))


if __name__ == '__main__':
    raise SystemExit(main())
