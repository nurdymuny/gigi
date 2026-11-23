"""RF2G minimum physical energy at fixed original face fluxes."""
import numpy as np
from ricci_variational_math import solve


def recover(h, j, z, f):
    f=np.asarray(f,dtype=float)
    if f.ndim!=1 or len(f)!=j.shape[1] or not np.isfinite(f).all():
        raise ValueError('finite face cochain with matching dimensions required')
    if h.shape!=(j.shape[0],j.shape[0]) or z.shape[0]!=j.shape[0]:
        raise ValueError('inconsistent constrained-space dimensions')
    if not all(np.isfinite(matrix.data).all() for matrix in [h,j,z]):
        raise ValueError('finite verified matrices required')
    s=(z.T@h@z).tocsr();t=(z.T@h@j).tocsr()
    u,first=solve(s,t@f)
    c = j@f-z@u
    y=h@c
    adjoint,second=solve(s,z.T@y)
    return {'moments':c,'gradient':j.T@y-t.T@adjoint,'energy':float(.5*c@y),
            'solves':[first,second]}
