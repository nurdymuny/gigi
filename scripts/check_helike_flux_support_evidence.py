"""Recheck saved evidence and explore enrichment directions; never solves an eigenproblem."""
import argparse
import copy
import json
from pathlib import Path

import numpy as np
import run_helike_flux_grid as grid
import run_helike_flux_support as probe


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('run_dir',type=Path)
    a = ap.parse_args()
    d = a.run_dir
    digest = json.loads((d/'receipt-integrity.json').read_text())['sha256']
    if grid.sha(d/'receipt.json') != digest or grid.sha(probe.BASE) != probe.BASE_SHA:
        raise ValueError('receipt or baseline digest mismatch')
    receipt = json.loads((d/'receipt.json').read_text())
    if receipt['status'] != 'DIAGNOSTIC_COMPLETE' or grid.sha(d/'support.json') != receipt['support_sha256_lf']:
        raise ValueError('incomplete or altered diagnostic')
    x = json.loads((d/'support.json').read_text())
    baseline = json.loads(probe.BASE.read_text())['rungs'][-1]['result']['result']
    analysis = probe.inspect(x,baseline)
    b = np.asarray(x['probe_basis'])
    outside = b.sum(axis=1) > 12
    c_axis = x['roles'][2]
    R = np.column_stack([probe.decode(st['probe_residual_bits']) for st in x['states']])
    R[~outside] = 0
    shares = []
    for j, state in enumerate(x['states']):
        w = R[:,j]**2
        total = float(w.sum())
        joint = sorted(state['omitted']['joint'],key=lambda k:k['mass'],reverse=True)
        pure = np.delete(b,c_axis,axis=1).sum(axis=1)==0
        shares.append({'state':state['name'],'pure_c_axis_share':float(w[pure].sum()/total),
                       'c_at_least_7_share':float(w[b[:,c_axis]>=7].sum()/total),
                       'top_joint_bin_share':joint[0]['mass']/total,
                       'top_10_joint_bins_share':sum(k['mass'] for k in joint[:10])/total})
    # SVD only on omitted rows, so the proposed directions have identically zero support in V_12.
    U, singular, _ = np.linalg.svd(R[outside],full_matrices=False)
    span = {'singular_values':singular.tolist(),
            'orthonormality_error':float(np.max(np.abs(U.T@U-np.eye(4)))),
            'projection_error_by_state':np.linalg.norm(R[outside]-U@(U.T@R[outside]),axis=0).tolist(),
            'old_dimension':len(x['basis']),'proposed_dimension':len(x['basis'])+4,
            'full_probe_dimension':len(x['probe_basis']),
            'new_eigensolve_performed':False}
    rejections = []
    for fault in ['wrong_point','false_certificate','corrupt_summary','corrupt_coefficients','missing_shell','wrong_energy']:
        z = copy.deepcopy(x)
        if fault=='wrong_point': z['s']=1.19
        elif fault=='false_certificate': z['certified']=True
        elif fault=='corrupt_summary': z['states'][3]['omitted']['shells'][13]*=1.1
        elif fault=='corrupt_coefficients': z['states'][3]['coeff_bits']=z['states'][0]['coeff_bits']
        elif fault=='missing_shell': z['probe_basis']=z['probe_basis'][:-1]
        else: z['states'][3]['energy']+=.1
        try:
            probe.inspect(z,baseline)
            raise AssertionError(f'bad evidence accepted: {fault}')
        except (ValueError,KeyError,IndexError) as exc:
            rejections.append({'fault':fault,'rejected':True,'reason':str(exc)})
    print(json.dumps({'scope':'post-diagnostic exploration, not a registered acceptance gate',
                      'receipt_sha256_lf':digest,'joint_support_checks':shares,'candidate_span':span,
                      'corruptions':rejections,'full_grid_released':False,'certified':False},indent=2,allow_nan=False))


if __name__=='__main__':
    main()
