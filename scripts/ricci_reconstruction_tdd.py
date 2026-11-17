"""Run focused RF2E tests and retain the behavioral result, including RED."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
p = argparse.ArgumentParser(); p.add_argument('receipt', type=Path)
p.add_argument('--module', default='test_ricci_curvature_reconstruction')
args = p.parse_args()
if args.receipt.exists(): raise ValueError('retain prior receipt; choose a new filename')
run = subprocess.run([sys.executable, '-m', 'unittest', '-v',
    args.module], cwd=ROOT/'e2e/probes', capture_output=True, text=True)
files = ['docs/RICCI_CURVATURE_RECONSTRUCTION_SPEC.md',
         'e2e/probes/ricci_curvature_reconstruction.py',
         'e2e/probes/test_ricci_curvature_reconstruction.py', 'scripts/ricci_reconstruction_tdd.py']
if args.module == 'test_ricci_reconstruction_guards':
    files += ['e2e/probes/ricci_reconstruction_evidence.py',
              'e2e/probes/test_ricci_reconstruction_guards.py']
if args.module == 'test_ricci_variational_math':
    files += ['docs/RICCI_VARIATIONAL_RECOVERY_SPEC.md',
              'e2e/probes/ricci_variational_math.py', 'e2e/probes/test_ricci_variational_math.py']
if args.module == 'test_ricci_variational_evidence':
    files += ['docs/RICCI_VARIATIONAL_RECOVERY_SPEC.md',
              'e2e/probes/ricci_variational_evidence.py', 'e2e/probes/test_ricci_variational_evidence.py']
receipt = {'exit_code': run.returncode, 'stdout': run.stdout, 'stderr': run.stderr,
    'sources': {f: hashlib.sha256((ROOT/f).read_bytes()).hexdigest() for f in files},
    'scope': 'focused behavioral tests; not a convergence or release gate'}
args.receipt.parent.mkdir(parents=True, exist_ok=True)
args.receipt.write_text(json.dumps(receipt, indent=2)+'\n', encoding='utf-8')
print(run.stdout+run.stderr)
sys.exit(run.returncode)
