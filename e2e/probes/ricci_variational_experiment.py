"""RF2F same-mesh Abelian experiment, with a full unprojected error gate."""
import argparse
from collections import Counter
import json
from pathlib import Path
import time
import numpy as np
from scipy.sparse import save_npz
from scipy.sparse.linalg import splu
from ricci_consistency_diagnostic import ROOT, BASE, frozen, scalar_control
from ricci_consistency_math import base_log, residual_split
from ricci_lattice_evidence import read, same
from ricci_shape_evidence import numerical_gate
from ricci_shape_experiment import write
from monodromy_evidence import sha
from ricci_variational_mesh import topology, recover_patches, deck
from ricci_variational_assembly import assemble
from ricci_variational_math import recovered_energy, solve
from ricci_lattice_mesh import plus

PRIOR = ROOT/'output/ricci-consistency-20261117/diagnostic'
SOURCES = [ROOT/'docs/RICCI_VARIATIONAL_RECOVERY_SPEC.md', Path(__file__),
    *[ROOT/'e2e/probes'/name for name in ['ricci_variational_math.py',
    'ricci_variational_mesh.py', 'ricci_variational_assembly.py',
    'test_ricci_variational_math.py', 'test_ricci_variational_geometry.py']]]


def manifest():
    if read(PRIOR/'manifest.json') != frozen(): raise ValueError('frozen RF2D source/data drift')
    verified = read(PRIOR/'VERIFIED_DIAGNOSTIC.json')
    if not verified['evidence_valid'] or verified['diagnostic_sha256'] != sha(PRIOR/'diagnostic.json'):
        raise ValueError('verified original baseline required')
    return {'schema': 'rf2f_frozen_v1', 'sources': {str(p.resolve()): sha(p) for p in SOURCES},
        'prior': {str(PRIOR/name): sha(PRIOR/name) for name in
            ['manifest.json', 'diagnostic.json', 'VERIFIED_DIAGNOSTIC.json']},
        'degree': 1, 'patch_radii': [1, 2, 3], 'condition_cap': 1e6,
        'rank_rtol': 1e-11, 'resolutions': [1, 2, 3, 5], 'orders': [4, 6],
        'mapping': 'UNCHANGED', 'clock': 1, 'mass': 'UNCHANGED', 'thresholds': 'BW05_UNCHANGED',
        'regularization': False, 'reference_fields_used_in_recovery': False}


def linear_correction(h, q, m, j, z, d, f, result, velocity):
    s = (z.T@h@z).tocsr(); coupling = (z.T@(q-h@j)).tocsr()
    correct = lambda mat, rhs: solve(mat, rhs, atol=1e-25)[0]
    ez = correct(s, s@result['free']-coupling@f)
    ew = correct(s, s@result['adjoint']-z.T@(h@result['moments']))
    dy = h@(z@ez)
    dg = j.T@dy+coupling.T@correct(s, z.T@dy)+coupling.T@ew
    return correct(m, m@velocity+d.T@result['gradient'])-correct(m, d.T@dg)


def measure(h, q, m, j, z, d, a, volume, load, direct=False):
    flux = d@a; result = recovered_energy(h, q, j, z, flux)
    gradient = d.T@result['gradient']; velocity, receipt = solve(m, -gradient)
    norm = lambda v: float(np.sqrt(max(0., v@(m@v))/volume))
    correction = linear_correction(h, q, m, j, z, d, flux, result, velocity)
    reference = -4*a
    split = residual_split(m, gradient, reference, load, volume)
    error = norm(velocity-reference)
    if abs(split['total_error']-error) > 2e-8: raise ValueError('independent residual split mismatch')
    recovered_flux = result['moments'].reshape(-1, 3).sum(axis=1)
    preservation = float(np.max(abs(recovered_flux-flux)))
    if preservation > 1e-10: raise ValueError('original face fluxes were changed')
    row = {'full_mass_error': error, 'relative_error': error/norm(reference),
        'reference_mass_norm': norm(reference), 'split': split,
        'energy_density': result['energy']/volume, 'flux_max_error': preservation,
        'linear_correction_mass_estimate': norm(correction), 'solves': result['solves']+[receipt]}
    if direct:
        s = (z.T@h@z).tocsc(); factor = splu(s); coupling = z.T@(q-h@j)
        free = factor.solve(coupling@flux); moments = j@flux+z@free
        adjoint = factor.solve(z.T@(h@moments))
        grad = d.T@(j.T@(h@moments)+coupling.T@adjoint)
        exact = splu(m.tocsc()).solve(-grad)
        row['direct_velocity_mass_difference'] = norm(exact-velocity)
        if row['direct_velocity_mass_difference'] > max(1e-9, 10*norm(correction)):
            raise ValueError('independent direct solver disagrees with correction estimate')
    return row, velocity


def exact_gradient_control(edges, d, h, q, j, z):
    values, _, _, _, _ = deck()
    def potential(points):
        real = plus(values[:,None,:], points[None,:,:])[...,0]
        return np.sum(real**12, axis=0)
    a = potential(edges[:,1])-potential(edges[:,0])
    f = d@a
    if np.ptp(potential(edges[:,0])) < 1e-4: raise ValueError('planted scalar potential unexpectedly constant')
    result = recovered_energy(h, q, j, z, f)
    if np.max(abs(f)) > 1e-11 or abs(result['energy']) > 1e-18:
        raise ValueError('exact scalar gradient acquired curvature')
    return {'potential_range': float(np.ptp(potential(edges[:,0]))),
            'maximum_face_flux': float(np.max(abs(f))), 'energy': result['energy']}


def run(directory):
    directory = Path(directory); directory.mkdir(parents=True, exist_ok=True)
    if (directory/'diagnostic.json').exists(): raise ValueError('preserve evidence; use a new run directory')
    frozen_manifest = manifest(); write(directory/'manifest.json', frozen_manifest)
    receipt = {'schema': 'rf2f_abelian_v1', 'status': 'IN_PROGRESS', 'release_gate': 'OPEN',
        'berger': 'NOT_RUN', 'full_regressions': 'DEFERRED', 'cases': {}, 'patches': {},
        'manifest_sha256': sha(directory/'manifest.json')}
    start = time.perf_counter(); old = read(BASE/'diagnostic.json'); prior = read(PRIOR/'diagnostic.json')
    hs = []; quadrature = []; numerical = []
    try:
        for level in range(4):
            payload = read(BASE/f'level{level}_berger_q6.input.json')
            topo = topology(payload); patches, diagnostics = recover_patches(payload, topo)
            patch_summary = {'radii': dict(Counter(str(row['radius']) for row in diagnostics)),
                'max_central_condition': max(row['central_condition'] for row in diagnostics),
                'max_free_condition': max(row['free_condition'] for row in diagnostics),
                'max_coefficient_amplification': max(row['coefficient_amplification'] for row in diagnostics),
                'faces': len(topo['faces']), 'cells': len(payload['cells'])}
            receipt['patches'][str(level)] = patch_summary
            print('patches '+str(level)+' '+json.dumps(patch_summary), flush=True)
            hmax = old['geometry'][str(level)]['h_max']; hs.append(hmax)
            velocities = {}; estimates = []
            for order in [4, 6]:
                h, q, m, j, z, volume, loads = assemble(payload, topo, patches, order)
                d = topo['incidence']; edges = np.array(payload['edge_points'])
                aa = base_log(edges[:,0], edges[:,1]); key = f'level{level}_q{order}'
                baseline_k = d.T@j.T@h@j@d
                baseline, _ = scalar_control(m, baseline_k, aa[:,0], volume)
                same(baseline, prior['cases'][key]['abelian_control'])
                rows = {}; vectors = []
                for axis in range(3):
                    row, velocity = measure(h, q, m, j, z, d, aa[:,axis], volume, loads[:,axis], direct=level==0)
                    rows[str(axis)] = row; vectors.append(velocity)
                control = exact_gradient_control(edges, d, h, q, j, z)
                np.savez_compressed(directory/(key+'.arrays.npz'), a=aa, velocity=np.column_stack(vectors), load=loads)
                for name, matrix in [('h',h), ('q',q), ('m',m), ('d',d)]:
                    save_npz(directory/(key+'.'+name+'.npz'), matrix)
                receipt['cases'][key] = {'level': level, 'order': order, 'h_max': hmax,
                    'volume': volume, 'controls': rows, 'exact_gradient': control,
                    'artifacts': {key+'.'+suffix: sha(directory/(key+'.'+suffix))
                        for suffix in ['arrays.npz', 'h.npz', 'q.npz', 'm.npz', 'd.npz']}}
                velocities[order] = vectors[0]; estimates.append(rows['0']['linear_correction_mass_estimate'])
                write(directory/'diagnostic.json', receipt)
                print(key+' '+json.dumps({'relative': rows['0']['relative_error'],
                    'full': rows['0']['full_mass_error'], 'action': rows['0']['split']['action_error'],
                    'mass': rows['0']['split']['mass_error']}), flush=True)
            delta = velocities[6]-velocities[4]
            quadrature.append(float(np.sqrt(max(0., delta@(m@delta))/volume))); numerical.append(max(estimates))
        receipt['numerical_gate'] = numerical_gate([receipt['cases'][f'level{i}_q6']['controls']['0']
            for i in range(4)], hs, quadrature, numerical)
        if frozen_manifest != manifest(): raise ValueError('source drift during experiment')
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
