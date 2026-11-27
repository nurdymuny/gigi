"""Compile isolated copies of the new module; never edits its production source."""
import argparse
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
LAB = ROOT/'helike-lab'
TESTS = LAB/'crates/helike-quantum/tests'
SOURCE = LAB/'crates/helike-quantum/src/support.rs'


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('out_dir', type=Path)
    args = ap.parse_args()
    args.out_dir.mkdir(parents=True, exist_ok=True)
    source = '\n'.join(line for line in SOURCE.read_text().splitlines() if not line.startswith('//!'))
    tests = (TESTS/'support_diagnostic.rs').read_text().replace('use helike_quantum::{basis, support};', 'use crate::{basis, support};')
    variants = {
        'baseline': (None,None),
        'drop_squaring': ('v[i] * v[i]', 'v[i].abs()'),
        'ignore_occupation_identity': ('out[j] = v[i]', 'out[i] = v[i]'),
        'erase_omitted_residual': ('let outer_sq =', 'outer.fill(0.0);\n    let outer_sq ='),
        'drop_mass_normalization': ('Some(means.map(|x| x / mass))', 'Some(means)'),
    }
    generated = (TESTS/'support_mutation_probe.rs').resolve()
    if generated.parent != TESTS.resolve() or generated.exists():
        raise RuntimeError('refusing to replace an existing generated test')
    results = []
    for name, (old,new) in variants.items():
        if old and source.count(old) != 1:
            raise RuntimeError(f'mutation target not unique: {name}')
        body = source if old is None else source.replace(old,new)
        text = 'pub use helike_quantum::{basis,model,operator,ritz};\nmod support {\n'+body+'\n}\nmod checks {\n'+tests+'\n}\n'
        created = False
        try:
            # This is a mechanically generated test crate, not an edit of the production module.
            with generated.open('x') as f:
                created = True
                f.write(text)
            run = subprocess.run(['cargo','test','--release','-p','helike-quantum','--test','support_mutation_probe','--','--nocapture'],
                                 cwd=LAB,env={**os.environ,'RAYON_NUM_THREADS':'8'},capture_output=True,text=True,timeout=180)
            (args.out_dir/(name+'.log')).write_text(run.stdout+'\n'+run.stderr)
            compiled_and_ran = 'running 8 tests' in run.stdout
            accepted = compiled_and_ran and (run.returncode == 0 if name == 'baseline' else run.returncode != 0 and 'FAILED' in run.stdout)
            results.append({'variant':name,'compiled_and_ran':compiled_and_ran,'gate_pass':accepted,'returncode':run.returncode})
            print(json.dumps(results[-1]),flush=True)
        finally:
            if created:
                generated.unlink(missing_ok=True)
        if not accepted:
            break
    (args.out_dir/'mutation-results.json').write_text(json.dumps(results,indent=2)+'\n')
    return 0 if len(results)==len(variants) and all(x['gate_pass'] for x in results) else 1


if __name__ == '__main__':
    raise SystemExit(main())
