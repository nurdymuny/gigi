"""Same retained binary, new mesh, full cover and physical scale checks."""
import unittest
import numpy as np
from scipy.sparse import coo_matrix
from ricci_shape_regular_mesh import mesh
from ricci_mesh_orbits import orbit_fixture_for_mesh,cover_fixture_for_mesh
from test_ricci_operator_native import invoke,sparse_matrix,rotation,matrix
from ricci_lattice_convergence import solve,reference_velocity


def lift(f, links, omit_adjoint=False):
    rows=[];cols=[];data=[]
    for e,(o,rev) in enumerate(zip(f['cover_edge_orbit'],f['cover_edge_reverse'])):
        b=-rotation(matrix(links[o])) if rev and not omit_adjoint else np.eye(3)
        for i in range(3):
            for j in range(3):rows.append(3*e+i);cols.append(3*o+j);data.append(b[i,j])
    return coo_matrix((data,(rows,cols)),shape=(3*len(f['cover_edge_orbit']),3*len(links))).tocsr()


class NativeShapeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.f=orbit_fixture_for_mesh(mesh(0));cls.reduced=invoke(cls.f)

    def test_whole_cover_and_reversal_mutant(self):
        # Relabel only: identical geometric cells, planted reversed deck edges.
        source=mesh(0);n=len(source['vertices']);permutation=np.random.default_rng(812).permutation(n)
        inverse=np.argsort(permutation)
        m={'vertices':source['vertices'][permutation],
            'tetrahedra':inverse[source['tetrahedra']],
            'edges':np.unique(np.sort(inverse[source['edges']],axis=1),axis=0),
            'faces':np.unique(np.sort(inverse[source['faces']],axis=1),axis=0)}
        f=orbit_fixture_for_mesh(m);r=invoke(f);full=invoke(cover_fixture_for_mesh(m))
        self.assertGreater(sum(f['cover_edge_reverse']),1000)
        np.testing.assert_allclose(r['energy'],self.reduced['energy'],rtol=2e-11,atol=1e-11)
        p=lift(f,r['links']);b=sparse_matrix(full);br=sparse_matrix(r)
        np.testing.assert_allclose(r['energy'],full['energy']/120,rtol=2e-11,atol=1e-11)
        np.testing.assert_allclose(r['volume'],full['volume']/120,rtol=2e-11,atol=1e-13)
        grad=np.ravel(full['gradient']);gr=np.ravel(r['gradient'])
        np.testing.assert_allclose(gr,p.T@grad/120,rtol=2e-11,atol=1e-11)
        np.testing.assert_allclose(br.toarray(),(p.T@b@p).toarray()/120,rtol=2e-11,atol=2e-12)
        _,w,_=solve(r);residual=b@(p@w)+grad
        self.assertLess(np.linalg.norm(p@(p.T@residual)/120),1e-9)
        wrong=lift(f,r['links'],omit_adjoint=True)
        self.assertGreater(np.linalg.norm(gr-wrong.T@grad/120),.01)
        self.assertGreater(np.linalg.norm(br.toarray()-(wrong.T@b@wrong).toarray()/120),.01)

    def test_metric_scaling_with_no_fitted_clock(self):
        a=self.reduced;b=invoke(self.f,(21.,7.,7.));ba,wa,_=solve(a);bb,wb,_=solve(b)
        np.testing.assert_allclose(b['links'],a['links'],atol=2e-14,rtol=0)
        np.testing.assert_allclose(b['volume'],a['volume']*7**1.5,rtol=2e-13)
        np.testing.assert_allclose(b['energy'],a['energy']/np.sqrt(7),rtol=2e-13)
        np.testing.assert_allclose(b['gradient'],np.array(a['gradient'])/np.sqrt(7),atol=1e-12,rtol=2e-12)
        np.testing.assert_allclose(bb.toarray(),ba.toarray()*np.sqrt(7),atol=1e-12,rtol=2e-12)
        np.testing.assert_allclose(wb,wa/7,atol=1e-11,rtol=2e-11)
        np.testing.assert_allclose(reference_velocity(self.f['edge_points'],[21.,7.,7.]),
            reference_velocity(self.f['edge_points'],[3.,1.,1.])/7,atol=1e-12,rtol=2e-12)


if __name__=='__main__':unittest.main()
