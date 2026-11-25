"""Export what the Rust charge-projection bound needs from the frozen Python/Arb record (bit-exact).

Reads
  * the saved states already exported for the residual enclosure (quasimode_states_e21087a1.json);
  * the Arb cutoff-tail receipt (research/cutoff-tail-bounds-20261124/controls-c31aab8e/receipt.json, on
    feat/leakage-gate-amendment) and its four trial NPZ files, which freeze the binary64 product coeff @ mixing;
  * the construction receipt (controls-e21087a1/receipt.json) for the reference charge defects (quadrature, orders
    10 and 12; diagnostics, not enclosures).

Writes, per control: the trial coefficients as IEEE-754 bit patterns, the rigorous Arb upper endpoints of the tail
norm t >= ||(1 - eta) psi|| and the cutoff leakage l >= ||[H, eta] psi|| (exact mantissa/exponent, and the binary64
value rounded upward from it), and the reference charge_sq values. Every hash is checked; nothing is recomputed.

usage: python export_projection_inputs.py <quasimode_states json> <cutoff receipt.json> <construction receipt.json> <out.json>
       (the four trial-g2-*-Mq-*.npz files sit next to the cutoff receipt)
"""
import hashlib
import json
import math
import sys
from fractions import Fraction
from pathlib import Path
import numpy as np

EXPECTED_PARENT = 'e922d6bfffea50907d1c28d7cf7cdae622bb8eda0da5e4f97fdc1c13807045ab'
EXPECTED_CUTOFF_REVISION = 'c31aab8ed623761c42ef6ad62af85dae120fa7c0'
NAMES = ('ground', 'helike_plus', 'helike_minus', 'competitor')
CHARGES = (0, 2, -2, 0)


def text_sha(path):
    return hashlib.sha256(Path(path).read_bytes().replace(b'\r\n', b'\n').replace(b'\r', b'\n')).hexdigest()


def bits(x):
    a = np.ascontiguousarray(np.asarray(x, dtype='<f8'))
    return [int(v) for v in a.view('<u8').ravel()]


def dyadic(d):
    return Fraction(int(d['mantissa'])) * Fraction(2) ** int(d['exponent'])


def upward(q):
    """The least binary64 >= q (q a nonnegative exact rational)."""
    if q < 0:
        raise ValueError('nonnegative bound expected')
    x = float(q)
    if Fraction(x) < q:
        x = math.nextafter(x, math.inf)
    assert Fraction(x) >= q
    return x


def main():
    states_path, cutoff_path, construction_path, out = map(Path, sys.argv[1:5])
    states = json.loads(states_path.read_text())
    cutoff = json.loads(cutoff_path.read_text())
    construction = json.loads(construction_path.read_text())
    if text_sha(construction_path) != EXPECTED_PARENT or states['parent_receipt_sha256'] != EXPECTED_PARENT:
        raise ValueError('construction receipt does not match the frozen measurement')
    if cutoff['parent_receipt_sha256'] != EXPECTED_PARENT:
        raise ValueError('cutoff receipt is not built on the frozen measurement')
    if cutoff['source_revision'] != EXPECTED_CUTOFF_REVISION or cutoff['source_dirty']:
        raise ValueError('cutoff receipt is not the reviewed clean revision')
    if cutoff['certified'] or cutoff['full_grid_released']:
        raise ValueError('unexpected cutoff status')
    by_key = {(c['g2'], c['Mq']): c for c in cutoff['controls']}
    charge_ref = {(c['g2'], c['Mq']): c for c in construction['controls']}
    controls = []
    for saved in states['controls']:
        key = (saved['g2'], saved['Mq'])
        cc = by_key[key]
        trial_path = cutoff_path.parent / cc['trial_file']
        if hashlib.sha256(trial_path.read_bytes()).hexdigest() != cc['trial_sha256']:
            raise ValueError(f'{trial_path.name}: hash differs from the cutoff receipt')
        if cc['state_sha256'] != saved['state_sha256']:
            raise ValueError('cutoff receipt used a different saved state')
        z = np.load(trial_path)
        for name, field in (('u0', 'u0'), ('S', 'S'), ('w', 'w'), ('centers', 'centers')):
            if bits(z[name]) != saved[field]:
                raise ValueError(f'{trial_path.name}: {name} differs from the saved state bits')
        if [list(map(int, b)) for b in z['basis']] != saved['basis']:
            raise ValueError('trial basis differs from the saved basis')
        coeff = np.asarray(z['coeff'], dtype=complex)
        if coeff.shape != tuple(saved['coeff_shape']):
            raise ValueError('trial coefficient shape differs')
        per_state = []
        ref = charge_ref[key]['integrals']
        for j, (name, m) in enumerate(zip(NAMES, CHARGES)):
            st = cc['states'][name]
            t = dyadic(st['tail_norm']['upper'])
            ell = dyadic(st['leakage_norm']['upper'])
            per_state.append(dict(
                name=name, charge=m,
                tail_norm_upper=upward(t), leakage_norm_upper=upward(ell),
                tail_norm_upper_dyadic=st['tail_norm']['upper'],
                leakage_norm_upper_dyadic=st['leakage_norm']['upper'],
                reference_charge_sq={o: ref[o][name]['charge_sq'] for o in ('10', '12')},
            ))
        controls.append(dict(
            g2=saved['g2'], Mq=saved['Mq'], trial_file=cc['trial_file'], trial_sha256=cc['trial_sha256'],
            trial_coeff_re=bits(coeff.real), trial_coeff_im=bits(coeff.imag), states=per_state,
        ))
    doc = dict(
        schema='helike-lab/projection-inputs/v1',
        states_file_sha256=text_sha(states_path),
        cutoff_receipt_sha256=text_sha(cutoff_path),
        cutoff_source_revision=cutoff['source_revision'],
        construction_receipt_sha256=EXPECTED_PARENT,
        note=('tail and leakage upper bounds are the Arb receipt endpoints, rounded upward to binary64; '
              'reference_charge_sq is tensor Gauss-Hermite quadrature (a diagnostic, not an enclosure)'),
        controls=controls,
    )
    out.write_text(json.dumps(doc, indent=1) + '\n', newline='\n')


if __name__ == '__main__':
    main()
