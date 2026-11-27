import math
import unittest
import copy
import json
from pathlib import Path
import tempfile
from unittest.mock import patch
import numpy as np
import helike_m8_geometry as gm
import helike_carrier_geometry_probe as probe


def squeezed():
    x,w=gm.product_grid(32)
    i=np.array(np.unravel_index(np.arange(32**2),(32,32))).T
    nodes=x[i]; W=np.prod(w[i],axis=1)
    core=gm.Core(W,nodes,np.ones(2),lambda z:np.broadcast_to(np.eye(2),(len(z),2,2)),np.zeros(2),
                 lambda z: .5*(4*z[:,0]**2+z[:,1]**2),lambda z:z)
    p0=2**.25*np.exp(-nodes[:,0]**2/2)
    dp0=np.column_stack([-nodes[:,0]*p0,np.zeros(len(p0))])
    return core,p0,dp0


class GeometryProbe(unittest.TestCase):
    def test_decision_keeps_old_gate_and_refuses_bad_evidence(self):
        row=dict(shares={'H':.05,'C':.9,'S':.025,'R':.025},identity_rel_error=1e-8,alignment=.974,
                 norm=1.,nonpositive_weight=0.,raw_sum=1.,share_sum=1.)
        row['shares_by_cut']={k:row['shares'].copy() for k in ('1e-12','1e-14')}
        good={'14':copy.deepcopy(row),'16':copy.deepcopy(row)}
        result=probe.decide(good,.974)
        self.assertEqual(result['status'],'RESOLVED_ALIGNED')
        self.assertEqual(result['old_export_gate'],'FAIL')
        self.assertIs(result['full_grid_released'],False)
        for key,value in [('nonpositive_weight',1e-9),('identity_rel_error',.01),('alignment',.7),('norm',.98)]:
            bad=copy.deepcopy(good); bad['16'][key]=value
            self.assertEqual(probe.decide(bad,.974)['status'],'OPEN')
        bad=copy.deepcopy(good); bad['14']['shares']['C']=float('nan')
        with self.assertRaises(ValueError): probe.decide(bad,.974)
        with self.assertRaises(ValueError): probe.decide({'16':row},.974)
        mixed=copy.deepcopy(good)
        for row in mixed.values():
            row['shares']={'H':.05,'C':.4,'S':.525,'R':.025}
            row['shares_by_cut']={k:row['shares'].copy() for k in ('1e-12','1e-14')}
        self.assertEqual(probe.decide(mixed,.974)['status'],'RESOLVED_NOT_ALIGNED')

    def test_changed_input_cannot_leave_pass(self):
        with tempfile.TemporaryDirectory() as root:
            out=Path(root)/'out'
            with patch('sys.argv',['probe',str(out)]),patch.object(probe.grid,'sha',return_value='changed'):
                self.assertEqual(probe.main(),2)
                self.assertEqual(json.loads((out/'receipt.json').read_text())['status'],'OPEN')
                with self.assertRaises(FileExistsError): probe.main()

    def test_saved_chart_is_reproduced(self):
        X=probe.load_source()
        P=gm.Point(X,14,nodes=(np.ones(1),np.zeros((1,6))))
        np.testing.assert_allclose(P.metric(np.zeros((1,6)))[0],np.eye(6),rtol=0,atol=1e-10)

    def test_exact_deformation_is_not_mixing(self):
        core,p0,dp0=squeezed(); x=core.z
        p=2*x[:,0]*p0; dp=2*x[:,0,None]*dp0; dp[:,0]+=2*p0
        r=core.analyse(p0,dp0,p,dp,1.5,{'C':(0,),'S':(1,)},np.eye(2),total_vacuum_norm=1.)
        z=float(np.sum(core.W*p0))**2
        bare=float(np.sum(core.W*p*np.sqrt(2)*x[:,0]))**2
        self.assertAlmostEqual(bare/z,8/9,places=10)
        self.assertAlmostEqual(r['identity'],2.,places=10)
        self.assertAlmostEqual(r['R'],0.,places=10)
        self.assertAlmostEqual(r['shares']['C'],1.,places=12)
        self.assertAlmostEqual(core.alignment(p0,p,[0]),1.,places=10)

    def test_genuine_mixture_remains_mixed(self):
        core,p0,dp0=squeezed(); x=core.z
        f=x[:,0]+math.sqrt(1.5)*x[:,1]
        p=f*p0; dp=f[:,None]*dp0+np.array([1.,math.sqrt(1.5)])[None,:]*p0[:,None]
        r=core.analyse(p0,dp0,p,dp,1.5,{'C':(0,),'S':(1,)},np.eye(2),total_vacuum_norm=1.)
        self.assertAlmostEqual(r['shares']['C'],.4,places=10)
        self.assertAlmostEqual(core.alignment(p0,p,[0]),.25,places=10)
        self.assertAlmostEqual(r['identity'],1.25,places=10)

    def test_global_cut_is_independent_of_partition(self):
        core,p0,dp0=squeezed(); x=core.z
        p=2*x[:,0]*p0; dp=2*x[:,0,None]*dp0; dp[:,0]+=2*p0
        blocks={'C':(0,),'S':(1,)}
        full=core.analyse(p0,dp0,p,dp,1.5,blocks,np.eye(2),cuts=(.01,.001),total_vacuum_norm=1.)
        parts=[]
        for rows in np.array_split(np.arange(len(p0)),17):
            c=gm.Core(core.W[rows],core.x[rows],core.w,core.G_fn,core.ell,core.V_fn,core.y_fn)
            parts.append(c.analyse(p0[rows],dp0[rows],p[rows],dp[rows],1.5,blocks,np.eye(2),cuts=(.01,.001),total_vacuum_norm=1.))
        got=probe.combine(parts)
        for key in ('D','R','norm','kinetic','potential'):
            self.assertAlmostEqual(got[key],full[key],places=10)
        for cut in ('.01','.001'):
            c=str(float(cut))
            self.assertAlmostEqual(got['D_by_cut'][c],full['D_by_cut'][c],places=12)

    def test_slabs_match_full_values_and_derivatives(self):
        import itertools
        basis=[n for n in itertools.product(range(3),repeat=6) if sum(n)<=2]
        coeff=np.random.default_rng(3).normal(size=(len(basis),2))
        expected,dexpected=gm.grid_values(basis,coeff,6,2)
        blocks=list(probe.slabs(basis,coeff,6,2))
        got=np.concatenate([x[2] for x in blocks]); dg=np.concatenate([x[3] for x in blocks])
        np.testing.assert_allclose(got,expected,rtol=1e-12,atol=1e-12)
        np.testing.assert_allclose(dg,dexpected,rtol=1e-12,atol=1e-12)
        W,x=gm.grid_nodes(6,np.ones(6))
        np.testing.assert_allclose(np.concatenate([v[0] for v in blocks]),W,rtol=1e-14)
        np.testing.assert_array_equal(np.concatenate([v[1] for v in blocks]),x)


if __name__=='__main__': unittest.main()
