"""Evaluate the relative cutoff-leakage gate on the 16 saved control states.

usage: python scripts/run_helike_leakage_gate.py OUT_DIR

Inputs (both pinned by SHA-256):
- the Arb cutoff/tail receipt research/cutoff-tail-bounds-20261124/controls-c31aab8e/receipt.json
  (authoritative dyadic endpoints: tail_sq, norm_sq, leakage_norm upper);
- the helike-lab interval receipt research/leakage-gate-20261125/inputs/cone_sector_bounds_c31aab8e.json
  (feat/helike-lab 3746a9b; r_lower and r_upper for the same hulled trial vector as the Arb bounds).

Exit 0: gate PASS on all 16 states. Exit 3: completed, gate FAIL or OPEN. Exit 2: refusal (inputs, provenance).
Nothing here is a spectral certificate; certified and full_grid_released stay false.
"""
import hashlib
import json
import math
from pathlib import Path
import subprocess
import sys
import time

import helike_leakage_gate as lg

ROOT = Path(__file__).resolve().parents[1]
ARB = ROOT / 'research/cutoff-tail-bounds-20261124/controls-c31aab8e/receipt.json'
RUST = ROOT / 'research/leakage-gate-20261125/inputs/cone_sector_bounds_c31aab8e.json'
RUST_SHA256 = '152f1f7ceec781c367b3571879045277e9c6592a918987147fe89291335f5a51'
RUST_ORIGIN = {'branch': 'feat/helike-lab', 'commit': '3746a9b', 'path': 'helike-lab/receipts/cone_sector_bounds_c31aab8e.json'}
SOURCES = ['scripts/helike_leakage_gate.py', 'scripts/run_helike_leakage_gate.py']
CONTROLS = [(0.004, 6), (0.004, 8), (0.002, 6), (0.002, 8)]
STATES = ['ground', 'helike_plus', 'helike_minus', 'competitor']


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def r_lower_value(rust_state):
    return rust_state['r_lower']


def historical_gate(arb_receipt):
    """The historical receipt's own verdict, reported unchanged (never relabelled)."""
    return arb_receipt['cutoff_tail_gate_passed']


def check_pairing(arb_state, rust_state):
    """The Rust receipt carries the Arb leakage endpoint rounded up to binary64: it must not undercut it,
    and must be within one binary64 step of it."""
    a = lg.dyadic(arb_state['leakage_norm']['upper'])
    b = lg.exact(rust_state['leakage_norm_upper'])
    if b < a or b > lg.exact(math.nextafter(float(a), math.inf)):
        raise ValueError(f'leakage endpoints do not pair: Arb {float(a)!r}, Rust {float(b)!r}')


def evaluate(arb, rust, rust_sha):
    rows = []
    rust_by = {(c['g2'], c['Mq']): {s['name']: s for s in c['states']} for c in rust['results']['controls']}
    arb_by = {(c['g2'], c['Mq']): c['states'] for c in arb['controls']}
    for key in CONTROLS:
        for name in STATES:
            a, r = arb_by[key][name], rust_by[key][name]
            check_pairing(a, r)
            leak = lg.dyadic(a['leakage_norm']['upper'])
            rl = lg.RigorousLower(r_lower_value(r), source=str(RUST.relative_to(ROOT)), sha256=rust_sha,
                                  field='r_lower', status=rust['status'])
            s = lg.classify_state(tail_sq_upper=lg.dyadic(a['tail_sq']['upper']),
                                  norm_sq_lower=lg.dyadic(a['norm_sq']['lower']),
                                  norm_sq_upper=lg.dyadic(a['norm_sq']['upper']),
                                  leakage_upper=leak, r_lower=rl)
            sector = r['sector'].get('Ok')
            if sector:
                q = sector['q_lower']
                s['projected_residual_bound_upper'] = math.nextafter(float(lg.residual_bound(r['r_upper'], leak, q)), math.inf)
                s['temple_r2_enlargement_upper'] = math.nextafter(float(lg.temple_enlargement(r['r_upper'], leak, q)), math.inf)
            s.update(g2=key[0], Mq=key[1], state=name, leakage_norm_upper=float(leak), r_lower=r['r_lower'], r_upper=r['r_upper'])
            rows.append(s)
    return rows


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    out = Path(sys.argv[1])
    if out.exists() and any(out.iterdir()):
        print('refusing to write into a non-empty directory'); return 2
    rev = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    dirty = bool(subprocess.check_output(['git', 'status', '--porcelain', '--', *SOURCES], cwd=ROOT, text=True).strip())
    if dirty:
        print('refusing: registered sources are dirty'); return 2
    arb, rust = json.loads(ARB.read_text()), json.loads(RUST.read_text())
    rust_sha, arb_sha = sha(RUST), sha(ARB)
    if rust_sha != RUST_SHA256:
        print('refusing: the residual receipt does not match its pinned hash'); return 2
    if rust['status'] != 'INTERVAL_ENCLOSURE':
        print('refusing: the residual receipt is not an interval enclosure'); return 2
    try:
        rows = evaluate(arb, rust, rust_sha)
        verdict = lg.overall(rows, expected=16)
    except (KeyError, ValueError) as e:
        print(f'refusing: {e}'); return 2
    rec = {
        'execution': 'COMPLETE_RELATIVE_LEAKAGE_GATE',
        'rule': 'leakage_norm_upper <= kappa * r_lower (rigorous), kappa = 1/1000; tail_sq <= 1e-12; norm_sq width <= 1e-10',
        'registration': 'docs/research/helike-leakage-gate-amendment-2026-11-24.md',
        'relative_leakage_gate': verdict,
        'historical_cutoff_gate_passed_unchanged': historical_gate(arb),
        'status': 'UNRESOLVED_INTEGRATION' if verdict != 'PASS' else 'CUTOFF_LEAKAGE_GATE_PASSED',
        'certified': False,
        'full_grid_released': False,
        'inputs': {'arb_receipt': str(ARB.relative_to(ROOT)), 'arb_sha256': arb_sha,
                   'residual_receipt': str(RUST.relative_to(ROOT)), 'residual_sha256': rust_sha,
                   'residual_origin': RUST_ORIGIN, 'residual_provenance': rust.get('provenance')},
        'source_revision': rev, 'source_dirty': dirty,
        'sources_sha256': {s: sha(ROOT / s) for s in SOURCES},
        'states': rows,
        'max_leakage_over_r_lower': max(s['leakage_over_r_lower_upper'] for s in rows),
        'finished_unix': time.time(),
    }
    out.mkdir(parents=True, exist_ok=True)
    (out / 'receipt.json').write_text(json.dumps(rec, indent=1))
    print(verdict, 'max l/r_lower', rec['max_leakage_over_r_lower'])
    return 0 if verdict == 'PASS' else 3


if __name__ == '__main__':
    sys.exit(main())
