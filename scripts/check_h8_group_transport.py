"""Exact embedded-path fixtures and bounded Lean audit; no simulation."""
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

NOTE = ROOT/'docs/research/h8-group-transport-proof-2026-11-27.md'
EXPECTED = ('gradient_intertwines', 'cubic_transfer', 'superpotential_transfer',
            'plane_pairing', 'plane_dissipation', 'scaled_saddle_height',
            'action_density_scale', 'distance_squeeze', 'coupling_action')


def pair(a, b):
    return s.expand(-2*s.trace(a*b))


def bracket(a, b):
    return a*b-b*a


def fundamental():
    return (s.Matrix([[0, -s.I/2], [-s.I/2, 0]]),
            s.Matrix([[0, -s.Rational(1, 2)], [s.Rational(1, 2), 0]]),
            s.diag(-s.I/2, s.I/2))


def fixtures():
    base = fundamental()
    block = tuple(s.diag(t, 0) for t in base)
    spin = (s.Matrix([[0, 1, 0], [1, 0, 1], [0, 1, 0]])*(-s.I/s.sqrt(2)),
            s.Matrix([[0, -s.I, 0], [s.I, 0, -s.I], [0, s.I, 0]])*(-s.I/s.sqrt(2)),
            s.diag(-s.I, 0, s.I))
    rotation = s.Matrix([[s.Rational(3, 5), 0, s.Rational(4, 5)],
                         [0, 1, 0], [-s.Rational(4, 5), 0, s.Rational(3, 5)]])
    return {'su3_block': (block, s.Integer(1)), 'su3_spin1': (spin, s.Integer(4)),
            'su4_two_blocks': (tuple(s.diag(t, t) for t in base), s.Integer(2)),
            'su3_conjugated': (tuple(rotation*t*rotation.T for t in block), s.Integer(1))}


def zero(matrix):
    return all(s.simplify(v) == 0 for v in matrix)


def certify_frame(frame, declared):
    if declared <= 0:
        raise ValueError('Nonpositive metric factor')
    for t in frame:
        if not zero(t+t.conjugate().T) or s.trace(t) != 0:
            raise ValueError('Not a traceless anti-Hermitian generator')
    for k in range(3):
        if not zero(bracket(frame[(k+1)%3], frame[(k+2)%3])-frame[k]):
            raise ValueError('Bracket preservation failed')
    gram = s.Matrix(3, 3, lambda a, b: pair(frame[a], frame[b])).applyfunc(s.simplify)
    if gram != declared*s.eye(3):
        raise ValueError('Declared metric factor does not match the full Gram matrix')


def W(X, omega):
    return s.expand(sum(omega[k]*pair(X[k], X[k]) for k in range(3))/2
                    - pair(X[0], bracket(X[1], X[2])))


def gradient(X, omega):
    return tuple(omega[k]*X[k]-bracket(X[(k+1)%3], X[(k+2)%3]) for k in range(3))


def image(p, frame):
    return tuple(sum((p[k, a]*frame[a] for a in range(3)), s.zeros(frame[0].rows))
                 for k in range(3))


def su3_basis():
    basis = []
    for a, b in ((0, 1), (0, 2), (1, 2)):
        t, v = s.zeros(3), s.zeros(3)
        t[a, b] = t[b, a] = -s.I/2
        v[a, b], v[b, a] = -s.Rational(1, 2), s.Rational(1, 2)
        basis.extend((t, v))
    return basis+[s.diag(-s.I/2, s.I/2, 0), s.diag(-s.I, -s.I, 2*s.I)/(2*s.sqrt(3))]


def certify_plane_velocity(factor, speed=1):
    u, x, y = s.symbols('u x y', positive=True)
    w = x*x/(2*u)+u*y*y-x*y*y
    measured = -s.diag(factor, 2*factor).inv()*s.Matrix([s.diff(factor*w, x), s.diff(factor*w, y)])
    proposed = speed*s.Matrix([y*y-x/u, (x-u)*y])
    if not zero(measured-proposed):
        raise ValueError('Proposed velocity is not the inherited-metric gradient')


class Geometry(unittest.TestCase):
    def test_frames_and_measured_indices(self):
        for name, (frame, factor) in fixtures().items():
            with self.subTest(name=name):
                certify_frame(frame, factor)

    def test_bad_frames_and_wrong_index_rejected(self):
        frame, _ = fixtures()['su3_block']
        bad = [(tuple(2*t for t in frame), 4), ((-frame[0], frame[1], frame[2]), 1),
               (frame, 4), (frame, 0)]
        for frame, factor in bad:
            with self.subTest(factor=factor), self.assertRaises(ValueError):
                certify_frame(frame, factor)

    def test_arbitrary_nine_coordinate_restriction(self):
        u = s.symbols('u', positive=True)
        p = s.Matrix(3, 3, s.symbols('p0:9', real=True))
        omega = (1/u, u, u)
        w = sum(omega[k]*sum(p[k, a]**2 for a in range(3)) for k in range(3))/2-p.det()
        grad = s.Matrix(3, 3, [s.diff(w, v) for v in p])
        for name, (frame, factor) in fixtures().items():
            with self.subTest(name=name):
                X = image(p, frame)
                self.assertEqual(s.simplify(W(X, omega)-factor*w), 0)
                actual, expected = gradient(X, omega), image(grad, frame)
                self.assertTrue(all(zero(a-b) for a, b in zip(actual, expected)))
                self.assertEqual(s.simplify(sum(pair(v, v) for v in actual)
                                          -factor*sum(v*v for v in grad)), 0)

    def test_full_su3_ambient_differential(self):
        u, x, y, eps = s.symbols('u x y eps', real=True, nonzero=True)
        basis = su3_basis()
        self.assertEqual(s.Matrix(8, 8, lambda a, b: pair(basis[a], basis[b])).applyfunc(s.simplify), s.eye(8))
        for name in ('su3_block', 'su3_spin1', 'su3_conjugated'):
            frame, _ = fixtures()[name]
            X, omega = image(s.diag(x, y, y), frame), (1/u, u, u)
            grad = gradient(X, omega)
            for k in range(3):
                for a, direction in enumerate(basis):
                    with self.subTest(embedding=name, row=k, colour=a):
                        perturbed = list(X)
                        perturbed[k] += eps*direction
                        derivative = s.expand(W(perturbed, omega)).coeff(eps, 1)
                        self.assertEqual(s.simplify(derivative-pair(grad[k], direction)), 0)

    def test_inherited_plane_metric(self):
        x, y = s.symbols('x y', real=True)
        for frame, factor in fixtures().values():
            X = image(s.diag(x, y, y), frame)
            metric = s.Matrix(2, 2, lambda a, b: sum(pair(v.diff((x, y)[a]), v.diff((x, y)[b])) for v in X))
            self.assertEqual(metric.applyfunc(s.simplify), factor*s.diag(1, 2))

    def test_factor_cancels_from_velocity_not_action(self):
        for _, factor in fixtures().values():
            certify_plane_velocity(factor)
        with self.assertRaisesRegex(ValueError, 'inherited-metric gradient'):
            certify_plane_velocity(4, s.Rational(1, 4))

    def test_endpoints_are_ambient_critical(self):
        u = s.symbols('u', positive=True)
        for frame, factor in fixtures().values():
            omega = (1/u, u, u)
            X = image(s.diag(u, 1, 1), frame)
            self.assertTrue(all(zero(v) for v in gradient(X, omega)))
            self.assertEqual(s.simplify(W(X, omega)), factor*u/2)

    def test_conjugation_preserves_W_and_metric(self):
        u, x, y = s.symbols('u x y', positive=True)
        a = image(s.diag(x, y, y), fixtures()['su3_block'][0])
        b = image(s.diag(x, y, y), fixtures()['su3_conjugated'][0])
        self.assertEqual(s.simplify(W(a, (1/u, u, u))-W(b, (1/u, u, u))), 0)
        self.assertEqual(s.simplify(sum(pair(v, v) for v in a)-sum(pair(v, v) for v in b)), 0)

    def test_action_density_has_full_index_not_square_root(self):
        scale, a, b = s.symbols('scale a b', positive=True)
        self.assertEqual(s.simplify(s.sqrt(scale*a)*s.sqrt(scale*b)-scale*s.sqrt(a*b)), 0)
        self.assertNotEqual(s.sqrt(4)*s.sqrt(4), s.sqrt(4))

    def test_q_coupling_scaling(self):
        g, u, x, y, scale = s.symbols('g u x y scale', positive=True)
        wg = scale*(x*x/(2*u)+u*y*y-g*x*y*y)
        expected = scale*(x*x/(2*u)+u*y*y-x*y*y)/g**2
        self.assertEqual(s.simplify(wg.subs({x:x/g, y:y/g}, simultaneous=True)-expected), 0)
        self.assertEqual(s.simplify(expected.subs({x:u, y:1})), scale*u/(2*g**2))


class Document(unittest.TestCase):
    def test_written_proof_and_scope(self):
        self.assertTrue(NOTE.exists(), 'Missing group-transport proof')
        text = NOTE.read_text(encoding='utf-8')
        for marker in ('I>0', 'Ad-invariant', 'connectors', 'every ambient path',
                       'not end-to-end Lean', 'H7 and H8 remain open', 'Iu/2'):
            self.assertIn(marker, text)


def audit(output):
    found = {name: sorted(v.strip() for v in deps.split(',') if v.strip())
             for name, deps in re.findall(r"'(H8Group\.\w+)' depends on axioms: \[([^\]]*)\]", output, re.S)}
    for name in re.findall(r"'(H8Group\.\w+)' does not depend on any axioms", output):
        found[name] = []
    for suffix in EXPECTED:
        key = 'H8Group.'+suffix
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
    sys.stdout.reconfigure(encoding='utf-8')
    receipt = dict(passed=False, mode='red' if args.expect_red else 'release',
                   scope='Exact matrix examples and conditional Lean algebra; general global theorem written separately',
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
            if result.errors or len(result.failures) != 1 or 'Missing group-transport proof' not in stream.getvalue():
                raise ValueError('Expected only the missing-proof red test')
        elif not result.wasSuccessful():
            raise ValueError('Exact or written-scope gate failed')
        receipt['metric_factors'] = {name: str(factor) for name, (_, factor) in fixtures().items()}
        manifest = json.loads((PROJECT/'lake-manifest.json').read_text())
        receipt['dependency_revisions'] = {p['name']: p['rev'] for p in manifest['packages']}
        receipt['toolchain'] = (PROJECT/'lean-toolchain').read_text().strip()
        if receipt['dependency_revisions']['mathlib'] != 'fc4f59e577feb1a4956671bc04ac18aaaf8cd842' or receipt['toolchain'] != 'leanprover/lean4:v4.36.0':
            raise ValueError('Changed dependency pin')
        for file in ('H8GroupTransport.lean', 'H8GroupDeclarations.lean', 'H8GroupVerification.lean'):
            path = PROJECT/file
            if not path.exists() and args.expect_red:
                continue
            if re.search(r'\b(sorry|admit|sorryAx|axiom|native_decide|unsafe)\b', path.read_text(encoding='utf-8')):
                raise ValueError('Forbidden shortcut: '+file)
        entry, _ = run(['lake', 'build'], out/'build.log')
        receipt['runs'].append(entry)
        if entry['exit_code']:
            raise ValueError('Lean build failed')
        entry, output = run(['lake', 'env', 'lean', 'H8GroupDeclarations.lean'], out/'declarations.log')
        receipt['runs'].append(entry)
        if args.expect_red:
            if not entry['exit_code'] or any('Unknown identifier `H8Group.'+n+'`' not in output for n in EXPECTED):
                raise ValueError('Missing declarations did not fail as expected')
            receipt['expected_red_observed'] = True
            return 0
        if entry['exit_code']:
            raise ValueError('Declaration audit failed')
        entry, output = run(['lake', 'env', 'lean', 'H8GroupVerification.lean'], out/'verification.log')
        receipt['runs'].append(entry)
        if entry['exit_code']:
            raise ValueError('Lean verification failed')
        receipt['axioms'] = audit(output)
        for poison in ('sorryAx', 'UnverifiedEmbedding'):
            try:
                audit(output.replace('propext', poison))
            except ValueError:
                continue
            raise ValueError('Axiom auditor accepted '+poison)
        receipt['audit_rejections'] = ['sorryAx', 'UnverifiedEmbedding']
        source = (PROJECT/'H8GroupTransport.lean').read_text(encoding='utf-8')
        mutations = {
            'missing_bracket': ('hbr : L (b x y) = c (L x) (L y)', 'hbr : True'),
            'missing_index': ('= I * superpotential B b w1 w2 w3 x y z', '= superpotential B b w1 w2 w3 x y z'),
            'wrong_plane_metric': ('-(I * fx u x y * vx + 2 * I * fy u x y * vy)', '-(I * fx u x y * vx + I * fy u x y * vy)'),
            'half_action': ('I * (W u u 1 - W u 0 0) = I * u / 2', 'I * (W u u 1 - W u 0 0) = I * u / 4'),
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
                 ROOT/'docs/research/h8-group-transport-contract-2026-11-27.md',
                 ROOT/'docs/research/h8-interwell-proof-2026-11-27.md',
                 *PROJECT.glob('*.lean'), PROJECT/'lakefile.toml', PROJECT/'lake-manifest.json', PROJECT/'lean-toolchain']
        receipt['sha256_lf'] = {p.relative_to(ROOT).as_posix():hashlib.sha256(p.read_bytes().replace(b'\r\n', b'\n')).hexdigest()
                                for p in files if p.exists()}
        (out/'receipt.json').write_text(json.dumps(receipt, indent=2)+'\n', encoding='utf-8')
        print(json.dumps({k:receipt[k] for k in ('mode', 'passed', 'failures')}, indent=2))


if __name__ == '__main__':
    raise SystemExit(main())
