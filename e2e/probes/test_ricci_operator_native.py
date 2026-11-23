"""Native assembly gates. Requires an explicitly pinned diagnostic executable."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
import numpy as np
from scipy.linalg import expm
from scipy.sparse import coo_matrix
from ricci_lattice_operator import orbit_fixture,cover_fixture
from ricci_lattice_demo import matrix,SIGMA,matrix_link
from ricci_lattice_forms import masses


def invoke(fixture,metric=(3.,1.,1.),order=4):
    with tempfile.TemporaryDirectory(prefix='gigi-rf2b-') as folder:
        a=Path(folder)/'input.json';b=Path(folder)/'output.json'
        a.write_text(json.dumps(dict(metric=list(metric),order=order,edge_points=fixture['edge_points'],cells=fixture['cells'])))
        subprocess.run([os.environ['GIGI_RF2B_OPERATOR_BINARY'],str(a),str(b)],check=True,timeout=600)
        return json.loads(b.read_text())


def sparse_matrix(result):
    rows=[];cols=[];values=[]
    for block in result['blocks']:
        for i in range(3):
            for j in range(3):
                rows.append(3*block['row']+i);cols.append(3*block['col']+j);values.append(block['value'][i][j])
    size=3*len(result['links'])
    return coo_matrix((values,(rows,cols)),shape=(size,size)).tocsr()


def rotation(u):
    return np.array([[np.trace(s@u@t@u.conj().T).real/2 for t in SIGMA] for s in SIGMA])


def oracle_energy(links,mass):
    faces=[(0,3,1),(0,4,2),(1,5,2),(3,5,4)]
    defects=[links[a]@links[b]@links[c].conj().T-np.eye(2) for a,b,c in faces]
    defects[3]=links[0]@defects[3]@links[0].conj().T
    return sum(mass[i,j]*np.trace(defects[i].conj().T@defects[j]).real for i in range(4) for j in range(4))


class NativeTests(unittest.TestCase):
    def test_independent_cell_action_mass_and_every_derivative(self):
        fixture=cover_fixture(0);c=fixture['cells'][17]
        points=np.array(c['vertices']);pairs=[(0,1),(0,2),(0,3),(1,2),(1,3),(2,3)]
        cell={'vertices':points.tolist(),'edges':list(range(6)),'reverse':[False]*6,'weight':1.}
        f={'cells':[cell],'edge_points':[[points[a].tolist(),points[b].tolist()] for a,b in pairs]}
        result=invoke(f); self.assertGreater(result['energy'],0.01)
        reference=masses(points,[3.,1.,1.],4)
        links=np.array([matrix_link([3.,1.,1.],a,b) for a,b in f['edge_points']])
        np.testing.assert_allclose(matrix(result['links']),links,atol=2e-13,rtol=0)
        self.assertAlmostEqual(result['energy'],oracle_energy(links,reference['face_mass']),places=11)
        rotations=[rotation(links[i]) for i in [0,1,2,1,2,2]]
        expected=np.block([[4*reference['edge_mass'][i,j]*rotations[i].T@rotations[j] for j in range(6)] for i in range(6)])
        np.testing.assert_allclose(sparse_matrix(result).toarray(),expected,atol=2e-12,rtol=2e-12)
        for e in range(6):
            for axis in range(3):
                h=1e-5;values=[]
                for sign in [1,-1]:
                    changed=links.copy();changed[e]=links[e]@expm(sign*h*1j*SIGMA[axis])
                    values.append(oracle_energy(changed,reference['face_mass']))
                np.testing.assert_allclose(result['gradient'][e][axis],(values[0]-values[1])/(2*h),atol=3e-9,rtol=0)

    def test_orbit_assembly_matches_entire_cover(self):
        f=orbit_fixture(0);reduced=invoke(f);full=invoke(cover_fixture(0))
        self.assertGreater(reduced['energy'],1.)
        from scipy.sparse import lil_matrix
        p=lil_matrix((3*len(full['links']),3*len(reduced['links'])))
        for e,(orbit,reverse) in enumerate(zip(f['cover_edge_orbit'],f['cover_edge_reverse'])):
            block=-rotation(matrix(reduced['links'][orbit])) if reverse else np.eye(3)
            p[3*e:3*e+3,3*orbit:3*orbit+3]=block
        p=p.tocsr()
        np.testing.assert_allclose(reduced['energy'],full['energy']/120,atol=1e-11,rtol=1e-11)
        np.testing.assert_allclose(reduced['volume'],full['volume']/120,atol=1e-13,rtol=1e-12)
        np.testing.assert_allclose(np.ravel(reduced['gradient']),p.T@np.ravel(full['gradient'])/120,atol=1e-11,rtol=1e-11)
        np.testing.assert_allclose(sparse_matrix(reduced).toarray(),(p.T@sparse_matrix(full)@p).toarray()/120,atol=2e-12,rtol=2e-12)

        # Explicit group average on the entire cover, including components not
        # chosen as independent quotient coordinates. No projected residual pass.
        from ricci_lattice_convergence import solve
        from ricci_lattice_mesh import mesh,plus,qfloat
        from monodromy_exact_oracle import exact_closure,S,T
        from scipy.spatial import cKDTree
        _,velocity,_=solve(reduced)
        residual=(sparse_matrix(full)@(p@velocity)+np.ravel(full['gradient'])).reshape(-1,3)
        m=mesh(0);tree=cKDTree(m['vertices']);index={tuple(e):i for i,e in enumerate(m['edges'])}
        average=np.zeros_like(residual)
        for group_element in exact_closure([S,T]):
            distance,permutation=tree.query(plus(qfloat(group_element),m['vertices']))
            self.assertLess(distance.max(),2e-12)
            for e,(a,b) in enumerate(permutation[m['edges']]):
                rotated=-rotation(matrix(full['links'][e]))@residual[e] if a>b else residual[e]
                average[index[min(a,b),max(a,b)]]+=rotated/120
        np.testing.assert_allclose(average.ravel(),p@(p.T@residual.ravel())/120,atol=4e-13,rtol=0)
        self.assertLess(np.linalg.norm(average),1e-9)
        self.assertGreater(np.linalg.norm(residual),0.01)


if __name__=='__main__':unittest.main()
