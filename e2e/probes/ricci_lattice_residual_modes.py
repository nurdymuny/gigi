"""Explain a failed full-norm gate without replacing it by a projection gate."""
import argparse
import json
from pathlib import Path
import numpy as np
from scipy.linalg import expm_frechet,logm
from ricci_lattice_demo import matrix,SIGMA
from ricci_lattice_operator import orbit_fixture
from ricci_lattice_convergence import reference_velocity,solve
from ricci_wilson_oracle import phi
from monodromy_evidence import sha


def homogeneous_tangents(points,g):
    x=phi(g);out=np.empty((3*len(points),9))
    for edge,(p,q) in enumerate(np.asarray(points)):
        base=matrix(np.r_[p[0],-p[1:]]).conj().T@matrix(np.r_[q[0],-q[1:]])
        logarithm=logm(base)
        xi=np.array([(0.5j*np.trace(s@logarithm)).real for s in SIGMA])
        a=-.5j*np.einsum('i,ijk->jk',x*xi,SIGMA)
        for direction in range(3):
            for axis in range(3):
                u,du=expm_frechet(a,-.5j*xi[direction]*SIGMA[axis])
                out[3*edge:3*edge+3,3*direction+axis]=[(-.5j*np.trace(s@u.conj().T@du)).real for s in SIGMA]
    return out


def run(directory,levels):
    result={'scope':'post-hoc residual diagnosis, not an alternate acceptance gate','cases':[]}
    for level in levels:
        stem=f'level{level}_berger_q6';path=directory/(stem+'.output.json');raw=json.loads(path.read_text())
        fixture=orbit_fixture(level);g=[3.,1.,1.];b,w,_=solve(raw)
        residual=w-reference_velocity(fixture['edge_points'],g)
        basis=homogeneous_tangents(fixture['edge_points'],g)
        gram=basis.T@(b@basis);coefficients=np.linalg.solve(gram,basis.T@(b@residual))
        resolved=basis@coefficients;unresolved=residual-resolved
        norm=lambda r:float(np.sqrt(r@(b@r)/raw['volume']))
        total,homogeneous,transverse=map(norm,[residual,resolved,unresolved])
        np.testing.assert_allclose(total**2,homogeneous**2+transverse**2,rtol=3e-12,atol=1e-12)
        row={'level':level,'full_mass_error':total,'nine_mode_component':homogeneous,
            'transverse_component':transverse,'gram_condition':float(np.linalg.cond(gram)),
            'native_output_sha256':sha(path)}
        result['cases'].append(row);print(row,flush=True)
    (directory/'residual_modes.json').write_text(json.dumps(result,indent=2,allow_nan=False))


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('directory',type=Path);p.add_argument('--levels',nargs='+',type=int,default=[0,1,2,3])
    a=p.parse_args();run(a.directory,a.levels)
