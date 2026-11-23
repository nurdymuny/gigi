"""RF2E four-level Abelian gate, with untouched RF2C/D as data sources."""
import argparse
import json
from pathlib import Path
import time
import numpy as np
from scipy.sparse import save_npz, load_npz
from scipy.sparse.linalg import splu
from ricci_consistency_diagnostic import BASE, ROOT, frozen, assemble, scalar_control
from ricci_consistency_math import base_log
from ricci_curvature_reconstruction import evaluate, geometry_scale
from ricci_lattice_evidence import read, same
from ricci_shape_evidence import numerical_gate
from ricci_shape_experiment import write
from monodromy_evidence import sha

PRIOR = ROOT/'output/ricci-consistency-20261117/diagnostic'
SOURCES = [Path(__file__), ROOT/'e2e/probes/ricci_curvature_reconstruction.py',
           ROOT/'docs/RICCI_CURVATURE_RECONSTRUCTION_SPEC.md']


def manifest():
    if read(PRIOR/'manifest.json') != frozen(): raise ValueError('RF2D source drift')
    receipt = read(PRIOR/'VERIFIED_DIAGNOSTIC.json')
    if not receipt['evidence_valid'] or receipt['recomputed_cases'] != 8:
        raise ValueError('complete verified RF2D evidence required')
    if receipt['diagnostic_sha256'] != sha(PRIOR/'diagnostic.json'):
        raise ValueError('RF2D diagnostic drift')
    return {'prior': {str(PRIOR/name): sha(PRIOR/name) for name in
        ['manifest.json', 'diagnostic.json', 'VERIFIED_DIAGNOSTIC.json']},
        'sources': {str(path.resolve()): sha(path) for path in SOURCES},
        'scale': 'L^2*(h/L)^(3/2)', 'reference_length': 1.,
        'resolutions': [1, 2, 3, 5], 'orders': [4, 6],
        'mapping': 'UNCHANGED', 'time_scale': 1, 'mass': 'UNCHANGED',
        'thresholds': 'BW05_UNCHANGED', 'candidate': 'two_sided_bias_corrected_resolvent'}


def direct_reference(m, k, a, epsilon):
    s = m+epsilon*k
    shifted = splu(s.tocsc()); mass = splu(m.tocsc()); residuals = []

    def solve(matrix, factor, rhs):
        value = factor.solve(rhs)
        residual = float(np.linalg.norm(matrix@value-rhs))
        limit = 3*max(1e-12*float(np.linalg.norm(rhs)), 1e-14)
        if not np.isfinite(value).all() or residual > limit:
            raise ValueError('independent LU residual refused')
        residuals.append({'residual_norm': residual, 'residual_limit': limit})
        return value

    def r(x):
        y = solve(s, shifted, m@x)
        return 2*y-solve(s, shifted, m@y)

    first = r(a); second = r(first)
    return solve(m, mass, -k@second), residuals


def measure(m, k, a, volume, h):
    epsilon = geometry_scale(h)
    result = evaluate(m, k, a, epsilon)
    direct, direct_receipts = direct_reference(m, k, a, epsilon)
    norm = lambda v: float(np.sqrt(max(0., v@(m@v))/volume))
    reference = -4*a  # Oracle only, after the target-blind operator returned.
    error = norm(result['velocity']-reference); reference_norm = norm(reference)
    row = {'full_mass_error': error, 'reference_mass_norm': reference_norm,
        'relative_error': error/reference_norm, 'epsilon': epsilon,
        'energy_density': result['energy']/volume, 'volume': volume,
        'solver_mass_difference': norm(direct-result['velocity']),
        'cg_solves': result['solves'], 'lu_solves': direct_receipts,
        'degrees_of_freedom': len(a)}
    return row, result['velocity'], direct


def run(directory):
    directory = Path(directory); directory.mkdir(parents=True, exist_ok=True)
    if (directory/'diagnostic.json').exists(): raise ValueError('preserve prior run; use fresh directory')
    frozen_manifest = manifest(); write(directory/'manifest.json', frozen_manifest)
    receipt = {'schema': 'rf2e_abelian_v1', 'status': 'IN_PROGRESS',
        'manifest_sha256': sha(directory/'manifest.json'), 'cases': {},
        'berger': 'NOT_RUN', 'release_gate': 'OPEN', 'full_regressions': 'DEFERRED'}
    start = time.perf_counter(); baseline = read(BASE/'diagnostic.json')
    prior = read(PRIOR/'diagnostic.json'); hs = []; quadrature = []; linear = []
    try:
        for level in range(4):
            h = baseline['geometry'][str(level)]['h_max']; hs.append(h)
            velocities = {}; estimates = []
            for order in [4, 6]:
                key = f'level{level}_q{order}'; oldkey = f'level{level}_berger_q{order}'
                payload = read(BASE/(oldkey+'.input.json')); raw = read(BASE/(oldkey+'.output.json'))
                _, m, k, volume, _ = assemble(payload, raw, order)
                edges = np.array(payload['edge_points']); a = base_log(edges[:,0], edges[:,1])[:,0]
                old_numbers, _ = scalar_control(m, k, a, volume)
                same(old_numbers, prior['cases'][key]['abelian_control'])
                row, velocity, direct = measure(m, k, a, volume, h)
                row.update({'level': level, 'order': order, 'h_max': h,
                            'baseline_relative_error': old_numbers['relative_error']})
                np.savez_compressed(directory/(key+'.arrays.npz'), a=a, velocity=velocity,
                                    direct_velocity=direct)
                save_npz(directory/(key+'.mass.npz'), m); save_npz(directory/(key+'.stiffness.npz'), k)
                row['artifacts'] = {key+'.'+suffix: sha(directory/(key+'.'+suffix))
                    for suffix in ['arrays.npz', 'mass.npz', 'stiffness.npz']}
                receipt['cases'][key] = row; velocities[order] = velocity
                estimates.append(row['solver_mass_difference'])
                write(directory/'diagnostic.json', receipt)
                print(json.dumps({'case': key, 'relative_error': row['relative_error'],
                    'full_mass_error': row['full_mass_error'], 'solver_difference': estimates[-1]}), flush=True)
            delta = velocities[6]-velocities[4]
            quadrature.append(float(np.sqrt(max(0., delta@(m@delta))/volume)))
            linear.append(max(estimates))
        receipt['numerical_gate'] = numerical_gate([receipt['cases'][f'level{i}_q6']
            for i in range(4)], hs, quadrature, linear)
        if manifest() != frozen_manifest: raise ValueError('source drift during experiment')
        receipt['status'] = 'COMPLETE_UNVERIFIED'
    except BaseException as error:
        receipt['status'] = 'FAILED'; receipt['error'] = str(error)
        raise
    finally:
        receipt['elapsed_seconds'] = time.perf_counter()-start
        write(directory/'diagnostic.json', receipt)
    print(json.dumps(receipt['numerical_gate'], indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(); parser.add_argument('directory', type=Path)
    run(parser.parse_args().directory.resolve())
