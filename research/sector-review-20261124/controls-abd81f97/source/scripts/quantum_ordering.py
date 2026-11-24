"""Conditional ordering; these routines never verify an assumed separation."""
import math

import numpy as np
import scipy.linalg as la

import ym_helike_quantum as q


def lower_estimate(rho, residual, beta):
    if not np.isfinite([rho, residual, beta]).all() or residual < 0:
        raise ValueError('finite energies and nonnegative measured residual required')
    if beta <= rho + residual:
        return {'usable': False, 'reason': 'SEPARATION_NOT_RESOLVED', 'lower': None, 'tau': None}
    tau = residual**2/(beta-rho)
    if not np.isfinite(tau):
        raise ValueError('nonfinite Temple calculation')
    return {'usable': True, 'reason': 'ASSUMED_FROM_RITZ', 'lower': rho-tau, 'tau': tau}


def _branch(data):
    if not np.isfinite(list(data.values())).all() or data['residual'] < 0 or data['next_residual'] < 0:
        raise ValueError('finite branch values and nonnegative residuals required')
    if data['spin_error'] < 0 or data['spin_error'] > 1e-6:
        raise ValueError('spin validation failed')
    if data['high'] < data['low'] or data['next_low'] <= data['high']:
        raise ValueError('ordered, distinct complete multiplets required')
    beta = data['next_low']-data['next_residual']
    half = data['low']+(beta-data['low'])/2
    return dict(data, beta=beta, beta_half=half,
                base=lower_estimate(data['low'], data['residual'], beta),
                half=lower_estimate(data['low'], data['residual'], half))


def ordering(scalar, tensor):
    s, t = _branch(scalar), _branch(tensor)
    eta = 1e-8*max(1., abs(s['high']), abs(t['high']))

    def decide(key):
        if t[key]['usable'] and s['high']+eta < t[key]['lower']:
            return 'SCALAR_LOWER'
        if s[key]['usable'] and t['high']+eta < s[key]['lower']:
            return 'TENSOR_LOWER'
        return 'ORDER_UNRESOLVED'

    base, half = decide('base'), decide('half')
    conditional = base if base == half else 'ORDER_UNRESOLVED'
    if base != half:
        reason = 'SEPARATION_SENSITIVE'
    elif conditional != 'ORDER_UNRESOLVED':
        reason = 'CONDITIONAL_INEQUALITY'
    elif not s['base']['usable'] or not t['base']['usable']:
        reason = 'SEPARATION_NOT_RESOLVED'
    else:
        reason = 'BOUNDS_OVERLAP'
    radius = s['residual']+t['residual']+eta
    weinstein = 'ORDER_UNRESOLVED'
    if s['high']+radius < t['low']:
        weinstein = 'SCALAR_LOWER'
    elif t['high']+radius < s['low']:
        weinstein = 'TENSOR_LOWER'
    return {'conditional_order': conditional, 'reason': reason, 'base_order': base, 'half_order': half,
            'unconditional_order': 'ORDER_UNRESOLVED', 'basis': 'ASSUMED_FROM_RITZ',
            'separation_verified': False, 'certified': False, 'scientific_status': 'UNRESOLVED',
            'scalar': s, 'tensor': t, 'eta': eta, 'weinstein_order': weinstein}


def select_levels(energies, vectors, metric, casimir):
    np.testing.assert_allclose(vectors.conj().T@metric@vectors, np.eye(len(energies)), atol=1e-10, rtol=1e-10)
    spins = q.level_spins(energies, vectors, casimir)
    if spins[0]['J'] != 0 or spins[0]['multiplicity'] != 1:
        raise ValueError('nondegenerate spin-zero vacuum required')
    by_spin = {0: [], 2: []}
    for index, group in enumerate(spins):
        G = vectors[:, group['indices']]
        cj = G.conj().T@casimir@G
        values, axes = la.eigh((cj+cj.conj().T)/2)
        labels = np.rint((np.sqrt(1+4*np.maximum(values,0))-1)/2).astype(int)
        for J, count in ((0, 3), (2, 2)):
            if len(by_spin[J]) >= count or not np.any(labels == J):
                continue
            W = axes[:, labels == J]
            if W.shape[1] != 2*J+1:
                raise ValueError('incomplete multiplet or repeated unresolved same-spin copies')
            B = W.conj().T@(energies[group['indices'],None]*W)
            ev, rot = la.eigh((B+B.conj().T)/2)
            selected = G@W@rot
            coeff = vectors.conj().T@metric@selected
            spin_action = vectors.conj().T@casimir@selected
            error = float(la.norm(spin_action-J*(J+1)*coeff)/max(1.,la.norm(coeff)))
            if error > 1e-6:
                raise ValueError('spin validation failed')
            by_spin[J].append({'spin': J, 'rank': W.shape[1], 'group_index': index,
                               'low': float(ev[0]), 'high': float(ev[-1]), 'spin_error': error,
                               'vectors': selected, 'energy_block': np.diag(ev)})
        if len(by_spin[0]) == 3 and len(by_spin[2]) == 2:
            break
    if len(by_spin[0]) != 3 or len(by_spin[2]) != 2:
        raise ValueError('next distinct same-spin multiplet missing')
    return dict(zip(('vacuum','scalar','scalar_next','tensor','tensor_next'), by_spin[0]+by_spin[2]))


def full_residual(basis, vectors, energy_block, g2, alpha, output_codes=None):
    if not np.isfinite([g2,alpha]).all() or g2 < 0 or alpha <= 0:
        raise ValueError('valid coupling and width required')
    if not np.isfinite(vectors).all() or not np.isfinite(energy_block).all():
        raise ValueError('finite states and energies required')
    fock = basis['C']@vectors
    np.testing.assert_allclose(fock.conj().T@fock, np.eye(vectors.shape[1]), atol=1e-10, rtol=1e-10)
    np.testing.assert_allclose(energy_block, energy_block.conj().T, atol=1e-10, rtol=1e-10)
    codes = q.oscillator_codes(basis['cutoff']+4) if output_codes is None else output_codes
    pos = np.searchsorted(codes, basis['codes'])
    p = q.operator_polynomials()
    terms = [(-3*math.sqrt(g2)*alpha**-1.5*c,a) for c,a in q.terms(p['D'])]
    terms += [(g2*alpha**-2*c,a) for c,a in q.terms(p['V4'])]
    terms += [((1/(2*alpha)-alpha/2)*c,a) for c,a in q.terms(p['trace'])]
    action = q.expanded_action(basis,vectors,terms,codes)
    action[pos] += alpha*(basis['states'].sum(axis=1)+4.5)[:,None]*fock-fock@energy_block
    gram = action.conj().T@action
    norm = float(math.sqrt(max(0.,la.eigvalsh(gram)[-1])))
    if not np.isfinite(gram).all() or not math.isfinite(norm):
        raise ValueError('nonfinite residual')
    return {'norm': norm, 'gram': gram}


def width_summary(rows):
    gains = []
    for g, alpha in ((1.,1.7),(2.,2.3)):
        for level in (0,1,2):
            group = [r for r in rows if r['g2']==g and r['level']==level]
            base = next(r for r in group if r['alpha']==alpha)
            best = min(group,key=lambda r:r['full_residual'])
            gains.append(base['full_residual']/best['full_residual'])
    ground = {r['alpha']:r['full_residual'] for r in rows if r['g2']==2. and r['level']==0}
    return {'expanded_max_gain': max(gains),
            'minimum_excited_residual': min(r['full_residual'] for r in rows if r['level']>0),
            'legacy_ground_gain': ground[2.]/ground[2.3], 'legacy_from_alpha': 2., 'legacy_to_alpha': 2.3}
