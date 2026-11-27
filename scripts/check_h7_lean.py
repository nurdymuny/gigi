"""Compile H7 Lean proofs, audit dependencies, and reject broken source variants."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
PROJECT = ROOT/'formal/h7-cubic-quartic'
EXPECTED = (
    'zero_of_scaled_bound', 'cubic_quartic_bound', 'positive_constant_bound',
    'coercivity_factorization', 'scaled_coercivity', 'potential_lower_bound',
    'kappa_pos', 'quadratic_coefficients',
)
ALLOWED_AXIOMS = {'propext', 'Classical.choice', 'Quot.sound'}


def run(args, destination):
    result = subprocess.run(args, cwd=PROJECT, text=True, encoding='utf-8',
                            errors='replace', stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                            timeout=300)
    destination.write_text(result.stdout, encoding='utf-8')
    return dict(command=args, exit_code=result.returncode, log=destination.name), result.stdout


def audit_axioms(text):
    found = {}
    for name, deps in re.findall(r"'(H7\.\w+)' depends on axioms: \[([^\]]*)\]", text, re.S):
        found[name] = sorted(d.strip() for d in deps.split(',') if d.strip())
    for name in re.findall(r"'(H7\.\w+)' does not depend on any axioms", text):
        found[name] = []
    for suffix in EXPECTED:
        name = 'H7.'+suffix
        if name not in found:
            raise ValueError('Missing axiom audit: '+name)
        if set(found[name])-ALLOWED_AXIOMS:
            raise ValueError('Unexpected axiom in '+name+': '+str(found[name]))
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
                   scope='Kernel-checked real-algebraic implications; global Lie-algebra bound is a hypothesis',
                   source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
                   runs=[], failures=[])
    try:
        for filename in ('H7CubicQuartic.lean', 'Declarations.lean', 'Verification.lean'):
            text = (PROJECT/filename).read_text(encoding='utf-8')
            if re.search(r'\b(sorry|admit|sorryAx|axiom|native_decide|unsafe)\b', text):
                raise ValueError('Forbidden declaration or admission in '+filename)
        manifest = json.loads((PROJECT/'lake-manifest.json').read_text())
        receipt['dependency_revisions'] = {p['name']: p['rev'] for p in manifest['packages']}
        if receipt['dependency_revisions']['mathlib'] != 'fc4f59e577feb1a4956671bc04ac18aaaf8cd842':
            raise ValueError('Unexpected Mathlib revision')
        receipt['toolchain'] = (PROJECT/'lean-toolchain').read_text().strip()
        if receipt['toolchain'] != 'leanprover/lean4:v4.36.0':
            raise ValueError('Unexpected Lean toolchain')
        build, _ = run(['lake', 'build'], out/'build.log')
        receipt['runs'].append(build)
        if build['exit_code'] != 0:
            raise ValueError('Project build failed; see build.log')
        contract = 'Declarations.lean' if args.expect_red else 'Verification.lean'
        verification, output = run(['lake', 'env', 'lean', contract], out/'verification.log')
        receipt['runs'].append(verification)
        if args.expect_red:
            if verification['exit_code'] == 0:
                raise ValueError('The absent-theorem contract unexpectedly passed')
            for suffix in EXPECTED:
                if 'H7.'+suffix not in output:
                    raise ValueError('Red contract did not name the missing theorem '+suffix)
            receipt['expected_red_observed'] = True
            return 0
        if verification['exit_code'] != 0:
            raise ValueError('Signature, counterexample or axiom check failed')
        receipt['axioms'] = audit_axioms(output)
        for forbidden in ('sorryAx', 'UnverifiedGlobalBound'):
            altered = output.replace('propext', forbidden)
            try:
                audit_axioms(altered)
            except ValueError:
                pass
            else:
                raise ValueError('Axiom auditor accepted '+forbidden)
        receipt['axiom_auditor_negative_controls'] = ['sorryAx', 'UnverifiedGlobalBound']
        source = (PROJECT/'H7CubicQuartic.lean').read_text(encoding='utf-8')
        changes = {
            'wrong_exponent': ('(A + B) * Q q ^ (3 / 4 : ℝ)', '(A + B) * Q q ^ (2 / 3 : ℝ)'),
            'missing_global_bound': ('hglobal : ∀ q, |C q| ≤ A * Q q + B', 'hglobal : True'),
            'missing_cubic_homogeneity': ('hCscale : ∀ r, 0 < r → ∀ q, C (r • q) = r ^ 3 * C q', 'hCscale : True'),
            'half_additive_constant': ('z ^ 4 / 2 - z ^ 3 + 27 / 32 =', 'z ^ 4 / 2 - z ^ 3 + 27 / 64 ='),
        }
        for name, (old, new) in changes.items():
            if old not in source:
                raise ValueError('Mutation anchor missing: '+name)
            mutated = source.replace(old, new, 1)
            path = out/(name+'.lean')
            path.write_text(mutated, encoding='utf-8')
            check, diagnostics = run(['lake', 'env', 'lean', str(path)], out/(name+'.log'))
            receipt['runs'].append(check)
            if check['exit_code'] == 0 or not re.search(r'error[:(]', diagnostics):
                raise ValueError('Broken proof was not rejected: '+name)
            if re.search(r'unknown module|object file .* does not exist', diagnostics):
                raise ValueError('Mutation failed for an import problem, not its proof: '+name)
        receipt['mutations_rejected'] = list(changes)
        receipt['passed'] = True
    except Exception as exc:
        receipt['failures'].append(str(exc))
        return 1
    finally:
        sources = [Path(__file__), *PROJECT.glob('*.lean'), PROJECT/'lakefile.toml',
                   PROJECT/'lake-manifest.json', PROJECT/'lean-toolchain',
                   ROOT/'docs/research/h7-lean-contract-2026-11-27.md']
        receipt['sha256_lf'] = {p.relative_to(ROOT).as_posix(): hashlib.sha256(
            p.read_bytes().replace(b'\r\n', b'\n')).hexdigest() for p in sources if p.exists()}
        (out/'receipt.json').write_text(json.dumps(receipt, indent=2)+'\n', encoding='utf-8')
        print(json.dumps({k: receipt[k] for k in ('mode', 'passed', 'failures')}, indent=2))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
