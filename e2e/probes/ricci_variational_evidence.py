"""Fail-closed RF2F evidence replay. Never converts an Abelian result to Berger."""
import argparse
from collections import Counter
import json
from pathlib import Path
import numpy as np
from scipy.sparse import load_npz
from ricci_variational_experiment import (manifest, BASE, PRIOR, topology, recover_patches,
    assemble, base_log, scalar_control, measure, exact_gradient_control, numerical_gate,
    sha, read, same, write)


def validate_structure(receipt):
    if (receipt.get('schema') != 'rf2f_abelian_v1' or receipt.get('status') != 'COMPLETE_UNVERIFIED'
        or receipt.get('release_gate') != 'OPEN' or receipt.get('berger') != 'NOT_RUN'
        or receipt.get('full_regressions') != 'DEFERRED'
        or set(receipt.get('cases', {})) != {f'level{i}_q{q}' for i in range(4) for q in [4,6]}
        or set(receipt.get('patches', {})) != {str(i) for i in range(4)}
        or set(receipt) != {'schema','status','release_gate','berger','full_regressions','cases',
            'patches','manifest_sha256','elapsed_seconds','numerical_gate'}):
        raise ValueError('incomplete or altered variational Abelian-only scope')


def verify(directory):
    directory = Path(directory)
    for name in ['gate.PASS.json', 'VERIFIED_DIAGNOSTIC.json']:
        (directory/name).unlink(missing_ok=True)
    receipt = read(directory/'diagnostic.json'); validate_structure(receipt)
    if sha(directory/'manifest.json') != receipt['manifest_sha256']: raise ValueError('manifest digest differs')
    frozen = manifest()
    if read(directory/'manifest.json') != frozen: raise ValueError('frozen source/mapping/policy drift')
    declared = receipt['numerical_gate']
    same(declared, numerical_gate([receipt['cases'][f'level{i}_q6']['controls']['0'] for i in range(4)],
        [receipt['cases'][f'level{i}_q6']['h_max'] for i in range(4)],
        declared['quadrature_mass_differences'], declared['linear_correction_mass_estimates']))
    for key, row in receipt['cases'].items():
        expected = {key+'.'+suffix for suffix in ['arrays.npz','h.npz','q.npz','m.npz','d.npz']}
        if set(row.get('artifacts', {})) != expected: raise ValueError('artifact set differs')
        for name, digest in row['artifacts'].items():
            if sha(directory/name) != digest: raise ValueError('artifact digest differs')
    prior = read(PRIOR/'diagnostic.json'); old = read(BASE/'diagnostic.json')
    hs = []; quadrature = []; numerical = []; checked = {}
    for level in range(4):
        payload = read(BASE/f'level{level}_berger_q6.input.json')
        topo = topology(payload); patches, diagnostics = recover_patches(payload, topo)
        summary = {'radii': dict(Counter(str(row['radius']) for row in diagnostics)),
            'max_central_condition': max(row['central_condition'] for row in diagnostics),
            'max_free_condition': max(row['free_condition'] for row in diagnostics),
            'max_coefficient_amplification': max(row['coefficient_amplification'] for row in diagnostics),
            'faces': len(topo['faces']), 'cells': len(payload['cells'])}
        same(receipt['patches'][str(level)], summary)
        hmax = old['geometry'][str(level)]['h_max']; hs.append(hmax)
        velocities = {}; estimates = []
        for order in [4,6]:
            key = f'level{level}_q{order}'
            h,q,m,j,z,volume,loads = assemble(payload, topo, patches, order); d = topo['incidence']
            for name, matrix in [('h',h),('q',q),('m',m),('d',d)]:
                saved = load_npz(directory/(key+'.'+name+'.npz'))
                if saved.shape != matrix.shape: raise ValueError('matrix shape differs')
                for field in ['indices','indptr','data']:
                    np.testing.assert_array_equal(getattr(saved, field), getattr(matrix, field))
            edges = np.array(payload['edge_points']); aa = base_log(edges[:,0], edges[:,1])
            baseline, _ = scalar_control(m, d.T@j.T@h@j@d, aa[:,0], volume)
            same(baseline, prior['cases'][key]['abelian_control'])
            rows = {}; vectors = []
            for axis in range(3):
                row, velocity = measure(h,q,m,j,z,d,aa[:,axis],volume,loads[:,axis],direct=level==0)
                rows[str(axis)] = row; vectors.append(velocity)
            expected = {'level':level,'order':order,'h_max':hmax,'volume':volume,'controls':rows,
                        'exact_gradient':exact_gradient_control(edges,d,h,q,j,z)}
            same({k:v for k,v in receipt['cases'][key].items() if k!='artifacts'}, expected)
            with np.load(directory/(key+'.arrays.npz'), allow_pickle=False) as saved:
                if set(saved.files) != {'a','velocity','load'}: raise ValueError('array keys differ')
                for name, array in [('a',aa),('velocity',np.column_stack(vectors)),('load',loads)]:
                    np.testing.assert_array_equal(saved[name], array)
            checked[key] = rows['0']; velocities[order] = vectors[0]
            estimates.append(rows['0']['linear_correction_mass_estimate'])
            print('verified '+key, flush=True)
        delta = velocities[6]-velocities[4]
        quadrature.append(float(np.sqrt(max(0.,delta@(m@delta))/volume))); numerical.append(max(estimates))
    gate = numerical_gate([checked[f'level{i}_q6'] for i in range(4)], hs, quadrature, numerical)
    same(declared, gate)
    if manifest() != frozen: raise ValueError('source drift during replay')
    report = {'evidence_valid':True,'recomputed_cases':8,'smooth_field_checks':24,'exact_gradient_checks':8,
        'abelian_numeric_gate':'PASS' if gate['pass'] else 'FAIL','summary':gate,'berger':'NOT_RUN',
        'correspondence':'NOT_ESTABLISHED','release_gate':'OPEN','full_regressions':'DEFERRED',
        'diagnostic_sha256':sha(directory/'diagnostic.json'),'verifier_sha256':sha(Path(__file__))}
    write(directory/'VERIFIED_DIAGNOSTIC.json', report)
    return report


if __name__ == '__main__':
    parser = argparse.ArgumentParser(); parser.add_argument('directory',type=Path)
    print(json.dumps(verify(parser.parse_args().directory),indent=2))
