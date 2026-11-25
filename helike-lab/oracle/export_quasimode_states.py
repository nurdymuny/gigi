"""Export the four saved quasimode control states for the Rust residual enclosure (bit-exact).

Reads the frozen construction folder (research/quasimode-construction-20261124/controls-e21087a1, on
feat/leakage-gate-amendment) and the centred residual-moment receipt (research/residual-integration-20261124/
controls-d0297072/receipt.json). Writes every float as its IEEE-754 bit pattern (an unsigned 64-bit integer), so the
Rust side reads exactly the binary64 values the Python reference used. Nothing is recomputed except the chart, which
is rebuilt with the reference `ConeChart` exactly as `run_helike_residual_moments.py` does.

usage: python export_quasimode_states.py <controls-e21087a1 folder> <residual receipt.json> <out.json>
       (run with the reference scripts/ directory of that branch on PYTHONPATH)
"""
import hashlib
import json
import sys
from pathlib import Path
import numpy as np
import helike_positive_cone as pc

EXPECTED_PARENT = 'e922d6bfffea50907d1c28d7cf7cdae622bb8eda0da5e4f97fdc1c13807045ab'
NAMES = ('ground', 'helike_plus', 'helike_minus', 'competitor')


def text_sha(path):
    """The repository's text digest: LF-normalised SHA-256 (independent of the checkout's line endings)."""
    return hashlib.sha256(Path(path).read_bytes().replace(b'\r\n', b'\n').replace(b'\r', b'\n')).hexdigest()


def bits(x):
    a = np.ascontiguousarray(np.asarray(x, dtype='<f8'))
    return [int(v) for v in a.view('<u8').ravel()]


def main():
    folder, residual, out = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
    parent = folder / 'receipt.json'
    if text_sha(parent) != EXPECTED_PARENT:
        raise ValueError('parent receipt does not match the frozen measurement')
    receipt = json.loads(parent.read_text())
    moments = {(c['g2'], c['Mq']): c for c in json.loads(residual.read_text())['controls']}
    controls = []
    for row in receipt['controls']:
        path = folder / row['state_file']
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if digest != row['state_sha256']:
            raise ValueError('saved state hash mismatch')
        with np.load(path, allow_pickle=False) as z:
            chart = pc.ConeChart(z['G0'], z['M'])
            coeff, mixing = z['coeff'], z['mixing']
            if coeff.dtype != np.float64 or mixing.dtype != np.complex128:
                raise ValueError('unexpected saved dtypes')
            m = moments[(row['g2'], row['Mq'])]
            metric, V = pc.laurent_model(row['s'], row['t'], np.sqrt(row['g2']))
            controls.append(dict(
                g2=row['g2'], Mq=row['Mq'], s=row['s'], t=row['t'],
                state_file=row['state_file'], state_sha256=digest,
                s_bits=bits(row['s'])[0], t_bits=bits(row['t'])[0], g2_bits=bits(row['g2'])[0],
                g_bits=bits(np.sqrt(row['g2']))[0],
                u0=bits(chart.u0), S=bits(chart.S), saved_S_equals_chart_S=bool(np.array_equal(z['S'], chart.S)),
                w=bits(z['w']), basis=z['basis'].astype(int).tolist(),
                coeff=bits(coeff), coeff_shape=list(coeff.shape),
                mixing_re=bits(mixing.real), mixing_im=bits(mixing.imag),
                centers=bits(z['centers']), names=list(NAMES),
                python_potential=[dict(key=list(map(int, k)), c=bits(float(c))[0]) for k, c in V.items()],
                python_metric=[dict(i=i, j=j, key=list(map(int, k)), c=float(c))
                               for i in range(6) for j in range(6) for k, c in metric[i][j].items()],
                reference_centered_moments=dict(value=m['moments']['value'],
                                                independent=m['independent_moments']['value'],
                                                group_count=m['group_count'], workspace_degree=m['workspace_degree']),
            ))
    doc = dict(schema='helike-lab/quasimode-states/v1', parent_receipt_sha256=EXPECTED_PARENT,
               residual_receipt_sha256=text_sha(residual),
               float_encoding='IEEE-754 binary64 bit patterns as unsigned integers, little-endian row-major',
               controls=controls)
    out.write_text(json.dumps(doc) + '\n')
    print(out, len(controls), 'controls', [c['saved_S_equals_chart_S'] for c in controls])


if __name__ == '__main__':
    main()
