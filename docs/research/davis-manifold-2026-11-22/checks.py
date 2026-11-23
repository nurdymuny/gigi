import numpy as np
from math import exp, sin
# 1. toy horizon: argmax of (1-e^{-L/6}) (1-0.05L)^2/((1-0.05L)^2+1)
L=np.linspace(0.01,19.99,200001); f=(1-np.exp(-L/6))*(1-0.05*L)**2/((1-0.05*L)**2+1)
i=f.argmax(); print("toy argmax L=%.4f  LB=%.5f"%(L[i],f[i]))
for l in [3,6,10,15]:
    v=(1-exp(-l/6))*(1-0.05*l)**2/((1-0.05*l)**2+1); print(" L=%d LB=%.4f"%(l,v))
# 2. vacuity corollary: four equal errors summing to just over 0.7
e=0.7/4+1e-9; print("E_sum=0.70+, product (1-e)^4 = %.4f, union 1-sum = %.4f"%((1-e)**4,1-4*e))
# 3. Lemma 1 counterexample: delta+ = 1 always; delta- in {0,2} w.p. 1/2; D=2, tau=1
tau=1.0; D=2.0
m_eff = (0.5*0**2+0.5*2**2 - 1**2)/(2*tau**2)   # E[s(d+)] - E[s(d-)], s=-d^2/2tau^2
Delta = (0.5*0+0.5*2) - 1
print("m_eff=%.3f  Delta_cfg=%.3f  claimed lower bound m_eff/L_eff=%.3f"%(m_eff,Delta,m_eff/(D/tau**2)))
# 4. chord vs arc on a circle of curvature k: 1 - chord/arc vs kL
for kL in [0.1,0.3,0.5,1.0]:
    rel=1-2*sin(kL/2)/kL; print(" kL=%.1f  1-chord/arc=%.5f  (kL)^2/24=%.5f"%(kL,rel,kL**2/24))
