"""Fail-closed RF2E replay. An Abelian pass cannot close the Berger gate."""
import argparse
import json
from pathlib import Path
import numpy as np
from scipy.sparse import load_npz
from ricci_reconstruction_experiment import (manifest, measure, BASE, PRIOR,
    assemble, scalar_control, base_log, numerical_gate, sha, read, same, write)


def validate_structure(receipt):
    keys = {f'level{i}_q{q}' for i in range(4) for q in [4, 6]}
    if (receipt.get('schema') != 'rf2e_abelian_v1'
        or receipt.get('status') != 'COMPLETE_UNVERIFIED'
        or receipt.get('berger') != 'NOT_RUN' or receipt.get('release_gate') != 'OPEN'
        or receipt.get('full_regressions') != 'DEFERRED'
        or set(receipt.get('cases', {})) != keys
        or set(receipt) != {'schema', 'status', 'cases', 'berger', 'release_gate',
            'full_regressions', 'manifest_sha256', 'numerical_gate', 'elapsed_seconds'}):
        raise ValueError('incomplete/altered Abelian-only evidence contract')


def verify(directory):
    directory = Path(directory)
    for name in ['gate.PASS.json', 'VERIFIED_DIAGNOSTIC.json']:
        (directory/name).unlink(missing_ok=True)
    receipt = read(directory/'diagnostic.json'); validate_structure(receipt)
    if sha(directory/'manifest.json') != receipt['manifest_sha256']:
        raise ValueError('manifest digest differs')
    expected_manifest = manifest()
    if read(directory/'manifest.json') != expected_manifest:
        raise ValueError('source, mapping, scale, or prior evidence drift')
    for key, row in receipt['cases'].items():
        names = {key+'.'+suffix for suffix in ['arrays.npz', 'mass.npz', 'stiffness.npz']}
        if set(row.get('artifacts', {})) != names: raise ValueError('incomplete array/matrix artifacts')
        for name, digest in row['artifacts'].items():
            if sha(directory/name) != digest: raise ValueError('artifact digest differs')
    baseline = read(BASE/'diagnostic.json'); prior = read(PRIOR/'diagnostic.json')
    checked = {}; hs = []; quadrature = []; linear = []
    for level in range(4):
        h = baseline['geometry'][str(level)]['h_max']; hs.append(h)
        velocities = {}; estimates = []
        for order in [4, 6]:
            key = f'level{level}_q{order}'; oldkey = f'level{level}_berger_q{order}'
            payload = read(BASE/(oldkey+'.input.json')); raw = read(BASE/(oldkey+'.output.json'))
            _, m, k, volume, _ = assemble(payload, raw, order)
            for suffix, matrix in [('mass', m), ('stiffness', k)]:
                saved = load_npz(directory/(key+'.'+suffix+'.npz'))
                if saved.shape != matrix.shape: raise ValueError('matrix shape differs')
                for field in ['indptr', 'indices', 'data']:
                    np.testing.assert_array_equal(getattr(saved, field), getattr(matrix, field))
            edges = np.array(payload['edge_points']); a = base_log(edges[:,0], edges[:,1])[:,0]
            old_numbers, _ = scalar_control(m, k, a, volume)
            same(old_numbers, prior['cases'][key]['abelian_control'])
            row, velocity, direct = measure(m, k, a, volume, h)
            row.update({'level': level, 'order': order, 'h_max': h,
                        'baseline_relative_error': old_numbers['relative_error']})
            same({name: value for name, value in receipt['cases'][key].items()
                  if name != 'artifacts'}, row)
            with np.load(directory/(key+'.arrays.npz'), allow_pickle=False) as saved:
                if set(saved.files) != {'a', 'velocity', 'direct_velocity'}:
                    raise ValueError('array keys differ')
                for name, value in [('a', a), ('velocity', velocity), ('direct_velocity', direct)]:
                    np.testing.assert_array_equal(saved[name], value)
            velocities[order] = velocity; estimates.append(row['solver_mass_difference'])
            checked[key] = row
            print('verified '+key, flush=True)
        delta = velocities[6]-velocities[4]
        quadrature.append(float(np.sqrt(max(0., delta@(m@delta))/volume)))
        linear.append(max(estimates))
    gate = numerical_gate([checked[f'level{i}_q6'] for i in range(4)], hs, quadrature, linear)
    same(receipt['numerical_gate'], gate)
    if manifest() != expected_manifest: raise ValueError('source drift during replay')
    report = {'evidence_valid': True, 'recomputed_cases': 8,
        'abelian_numeric_gate': 'PASS' if gate['pass'] else 'FAIL', 'summary': gate,
        'berger': 'NOT_RUN', 'correspondence': 'NOT_ESTABLISHED', 'release_gate': 'OPEN',
        'full_regressions': 'DEFERRED', 'diagnostic_sha256': sha(directory/'diagnostic.json'),
        'verifier_sha256': sha(Path(__file__))}
    write(directory/'VERIFIED_DIAGNOSTIC.json', report)
    return report


if __name__ == '__main__':
    parser = argparse.ArgumentParser(); parser.add_argument('directory', type=Path)
    print(json.dumps(verify(parser.parse_args().directory), indent=2))
