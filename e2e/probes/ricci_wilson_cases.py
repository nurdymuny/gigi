"""Frozen RF2A query fixtures. No fitted clocks or learned parameters."""
import itertools
import numpy as np

def audit_query(g,normalized=False):
    coords=','.join(format(float(x),'.17f').rstrip('0').rstrip('.') for x in g)
    return f"RICCI_WILSON_AUDIT SU2 METRIC ({coords})"+(" NORMALIZED" if normalized else "")+";"

def audits():
    gs=list(itertools.product([.5,1.,2.,5.],repeat=3))
    gs+=np.exp(np.random.default_rng(20261118).uniform(-1,1,(12,3))).tolist()
    result={f"audit_{i}_{int(n)}":(list(g),n) for i,g in enumerate(gs) for n in [False,True]}
    result.update(scaled_berger=([21.,7.,7.],False),sphere=([3.,1.,1.],False),poincare=([3.,1.,1.],False))
    return result

def flows():
    return {f"{name}_{int(n)}":(g,n) for name,g in {
        "round":[1.,1.,1.],"berger":[3.,1.,1.],"triaxial":[.5,1.,2.],"negative":[5.,1.,1.]}.items() for n in [False,True]}

def flow_query(g,n):
    return "RICCI_FLOW SU2 METRIC ("+','.join(str(x) for x in g)+") TIME 0.02 STEPS 256"+(" NORMALIZED" if n else "")+";"

REFUSALS={
    "bad_metric":("RICCI_WILSON_AUDIT SU2 METRIC (0,1,1);","METRIC"),
    "bad_anisotropy":("RICCI_WILSON_AUDIT SU2 METRIC (0.000001,1000000,1);","METRIC"),
    "wrong_manifold":("RICCI_WILSON_AUDIT TORUS METRIC (1,1,1);","SU2"),
    "fitted_clock":("RICCI_WILSON_AUDIT SU2 METRIC (1,1,1) FIT_CLOCK;","NORMALIZED"),
}
