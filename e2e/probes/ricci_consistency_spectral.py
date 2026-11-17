"""Mass-orthogonal spectral accounting, not a filtered acceptance gate."""
import argparse
from pathlib import Path
import numpy as np
from scipy.linalg import eigh
from scipy.sparse import load_npz
from monodromy_evidence import sha
from ricci_shape_experiment import write


def accounting(m,k,a,reference=4.,cut=8.):
    lam,v=eigh(k,m);c=v.T@m@a
    mass=c*c;residual=(lam-reference)**2*mass
    direct=np.linalg.solve(m,k@a)-reference*a;direct_norm=float(direct@m@direct)
    return {'reference_eigenvalue':reference,'cut':cut,'mass_squared':float(mass.sum()),
        'residual_squared':float(residual.sum()),'direct_residual_squared':direct_norm,
        'identity_error':abs(direct_norm-float(residual.sum())),
        'high_field_mass_fraction':float(mass[lam>cut].sum()/mass.sum()),
        'high_residual_fraction':float(residual[lam>cut].sum()/residual.sum()),
        'maximum_eigenvalue':float(lam.max()),'eigenvalues':lam.tolist(),
        'mass_coefficients':c.tolist(),'scope':'coarse scalar control only; no mode removed'}


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('directory',type=Path);d=p.parse_args().directory.resolve()
    paths=[d/'coarse_q6_mass.npz',d/'coarse_q6_stiffness.npz',d/'level0_q6.arrays.npz']
    m=load_npz(paths[0]).toarray();k=load_npz(paths[1]).toarray()
    with np.load(paths[2]) as arrays:a=arrays['abelian_edge_integrals']
    result=accounting(m,k,a)
    if result['identity_error']>1e-10:raise ValueError('spectral and direct residuals disagree')
    result['input_sha256']={str(path):sha(path) for path in paths};result['source_sha256']=sha(Path(__file__))
    result['spec_sha256']=sha(Path(__file__).resolve().parents[2]/'docs/RICCI_OPERATOR_SPECTRAL_CONTROL_ADDENDUM.md')
    write(d/'SPECTRAL_ACCOUNTING.json',result)
    print({key:value for key,value in result.items() if key not in ['eigenvalues','mass_coefficients','input_sha256']})
