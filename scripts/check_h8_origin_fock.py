"""Independent cross-check of the O(g^2) origin splitting (docs/research/h8-origin-splitting-proof-2026-11-27.md).

Method differs from the proof note (which conjugates by e^{-W}): plain second-order Rayleigh-Schrodinger in a Fock basis.
Reduced model, s = 1, SU(2), the q = 0 well. With W = 1/2 sum w_k |q_k|^2 - g det q and sum_c q_kc cof_kc = det q,
    V = 1/2 |grad W|^2 = 1/2 |w q|^2 - g S det q + g^2/2 |cof q|^2,
    E2(phi) = g^2 [ 1/2 <phi|cof^2|phi> - S^2 <phi| det (H0 - E_phi)^-1 det |phi> ].
Prediction: charge-0 two-quantum excitation moves by -2 kappa (1 + 1/t) g^2 (kappa = 2 for SU(2)); the charge-+-2 pair by 0.
usage: python scripts/check_h8_origin_fock.py OUT.json"""
import itertools, math, sys
from collections import defaultdict
w=[1.0,1.0,1.0]; S=3.0
modes=[(k,c) for k in range(3) for c in range(3)]; idx={m:i for i,m in enumerate(modes)}
def apply_q(state,i):  # q_i = (a+a^dag)/sqrt(2 w)
    out=defaultdict(float); f=1/math.sqrt(2*w[modes[i][0]])
    for occ,amp in state.items():
        n=occ[i]
        if n>0:
            o=list(occ); o[i]-=1; out[tuple(o)]+=amp*f*math.sqrt(n)
        o=list(occ); o[i]+=1; out[tuple(o)]+=amp*f*math.sqrt(n+1)
    return out
def add(a,b,s=1.0):
    r=defaultdict(float,a)
    for k,v in b.items(): r[k]+=s*v
    return r
def mono(state, ops):  # product of q's
    for i in reversed(ops): state=apply_q(state,i)
    return state
def det_op(state):
    out=defaultdict(float)
    for p in itertools.permutations(range(3)):
        sgn=round(math.copysign(1,_sign(p)))
        out=add(out, mono(state,[idx[(0,p[0])],idx[(1,p[1])],idx[(2,p[2])]]), sgn)
    return out
def _sign(p):
    s=1
    for i in range(3):
        for j in range(i+1,3):
            if p[i]>p[j]: s=-s
    return s
def cof_ops():  # cof_kc as list of (coef, [i,j]) quadratic monomials
    res={}
    for k in range(3):
        for c in range(3):
            terms=[]
            r=[x for x in range(3) if x!=k]; cs=[x for x in range(3) if x!=c]
            sgn=(-1)**(k+c)
            terms.append((sgn,[idx[(r[0],cs[0])],idx[(r[1],cs[1])]]))
            terms.append((-sgn,[idx[(r[0],cs[1])],idx[(r[1],cs[0])]]))
            res[(k,c)]=terms
    return res
COF=cof_ops()
def cof_apply(state,kc):
    out=defaultdict(float)
    for co,ops in COF[kc]: out=add(out,mono(state,ops),co)
    return out
def energy(occ): return sum(occ[i]*w[modes[i][0]] for i in range(9))
def inner(a,b): return sum(v*b.get(k,0.0) for k,v in a.items())
vac=tuple([0]*9)
def create(state,i):
    out=defaultdict(float)
    for occ,amp in state.items():
        o=list(occ); n=o[i]; o[i]+=1; out[tuple(o)]+=amp*math.sqrt(n+1)
    return out
def two(row_a,row_b,sign=1.0):  # sum_c a^dag_{row_a c} a^dag_{row_b c} |0>
    s=defaultdict(float)
    for c in range(3): s=add(s, create(create({vac:1.0},idx[(row_b,c)]),idx[(row_a,c)]))
    return s
def norm(s):
    n=math.sqrt(inner(s,s)); return {k:v/n for k,v in s.items()}
phi0=norm(add(two(1,1),two(2,2)))          # charge 0 (rows 2,3 = indices 1,2)
phiA=norm(add(two(1,1),two(2,2),-1.0))     # charge +-2
phiB=norm(two(1,2))                        # charge +-2
def E2(phi,Ephi):
    C2=sum(inner(cof_apply(phi,kc),cof_apply(phi,kc)) for kc in COF)   # <phi|cof^2|phi> = sum |cof_kc phi|^2
    d=det_op(phi)
    second=sum(v*v/(energy(k)-Ephi) for k,v in d.items() if abs(v)>1e-14)
    return 0.5*C2 - S*S*second

def run(tv):
    global w, S
    w=[math.sqrt(tv),1/math.sqrt(tv),1/math.sqrt(tv)]; S=sum(w)
    u=w[1]; e0=E2({vac:1.0},0.0)
    return {"t": tv, "charge0": E2(phi0,2*u)-e0, "charge2_real": E2(phiA,2*u)-e0, "charge2_imag": E2(phiB,2*u)-e0,
            "predicted_charge0": -4*(1+1/tv)}

if __name__=="__main__":
    import json
    rec={"results":[run(tv) for tv in (1.5,2.0,3.0,5.0)]}
    for r in rec["results"]:
        assert abs(r["charge0"]-r["predicted_charge0"])<1e-9 and abs(r["charge2_real"])<1e-9 and abs(r["charge2_imag"])<1e-9, r
    rec["pass"]=True
    open(sys.argv[1],"w").write(json.dumps(rec,indent=1)+"\n"); print(json.dumps(rec,indent=1))
