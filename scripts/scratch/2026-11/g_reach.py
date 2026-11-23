import sys, numpy as np
sys.path.insert(0,'scripts')
import edge_wp8_fast as FA, edge_wp8_two_new_groups as G
import edge_wp7_principal_circle as F
def specs(g, gname):
    wts, V, fv = FA.defining_weights(g); v=[V[:,i] for i in range(V.shape[1])]
    a2 = FA.antisym(v[:2])
    if gname=="g2":
        return {"7":(1,0,v[0],7),"27":(2,0,F.tens(v[0],v[0]),27),"77s":(3,0,F.tens(v[0],v[0],v[0]),77),
                "64":(3,0,F.tens(v[0],a2),64),"77p":(4,0,np.kron(a2,a2),77),"182s":(4,0,F.tens(v[0],v[0],v[0],v[0]),182)}
    return {"6":(1,0,v[0],6),"14":(2,0,a2,14),"14p":(3,0,FA.antisym(v[:3]),14),"56s":(3,0,F.tens(v[0],v[0],v[0]),56),
            "64":(3,0,F.tens(v[0],a2),64),"90":(4,0,np.kron(a2,a2),90),"126s":(4,0,F.tens(v[0],v[0],v[0],v[0]),126)}
circ = G.circles() + [{"group":"sp3","label":"s5","angle":None,"eK":None}]
for c in circ:
    if c["label"]=="s5":
        g0 = FA.FastGroupK("sp3", 0.0, sectors={})
        # omega3-vee: direction where all three e_i equal -> find torus coords via weights of defining rep
        w, V, fv = FA.defining_weights(g0)
        # defining weights are +-e_i; omega3v pairs equally (=1) with e1,e2,e3: solve least squares
        pos = w[fv>0]
        e = np.linalg.lstsq(pos, np.ones(len(pos)), rcond=None)[0]; e/=np.linalg.norm(e)
        ang = float(np.degrees(np.arccos(abs(e@g0.u0))))
        c.update(eK=e, angle=ang)
    g = FA.FastGroupK(c["group"], c["angle"], eK_torus=c["eK"], sectors={})
    sp_ = specs(g, c["group"]); out=[]
    for k,(p,q,vec,dim) in sp_.items():
        RX = FA.GR.irrep_from(g.basis, [E for _,E in g.rts], np.asarray(vec,complex), FA.GR.rep_tensor(None,p,q), dim_expected=dim)
        RK = sum(c_*R_ for c_,R_ in zip(np.linalg.solve(np.array([[FA.GR.inner(Y,B) for B in g.basis] for Y in [g.Y[0]]]).T if False else [0],[0]), [0])) if False else None
        Rk = sum(g.R[0,a]*RX[a] for a in range(len(g.basis)))
        top = np.linalg.eigvalsh(1j*Rk).max()
        out.append(f"{k}:{top/g.qK:.2f}")
    print(c["label"], round(c["angle"],3), "qK", round(g.qK,4), " ".join(out), flush=True)
