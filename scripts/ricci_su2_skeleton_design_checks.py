"""Design checks for docs/research/skeleton-tests-preregistration-2026-11-20.md, section 1. Made at eps = 0 or on the
perturbation itself; no splitting or floor away from the Berger family is measured.
usage: python scripts/ricci_su2_skeleton_design_checks.py"""
import sys, numpy as np
from pathlib import Path; sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_sector_gauge_check as C, ricci_su2_spin_sector as S, ricci_su2_chiral_bound as CB
def Qop(j):
    d=j+1; rho=S.spin_rep(j); I3=np.eye(3)
    Q=(np.kron(np.eye(9),0.5j*rho[0])+np.kron(np.kron(CB.rot0(),I3),np.eye(d))+np.kron(np.kron(I3,CB.rot0()),np.eye(d)))
    return 0.5*(Q+Q.conj().T)
for t in (3.0,4.0):
  for j in (2,3,4):
    H,G=C.hessian_and_gauge([t,1,1],j); A=H+G@G.conj().T; Q=Qop(j)
    q,U=np.linalg.eigh(Q)
    # gauge parameter charge: Q_xi = J0 + R_Lie on V_j (x) su(2)
    d=j+1; rho=S.spin_rep(j); Qx=np.kron(np.eye(3),0.5j*rho[0])+np.kron(CB.rot0(),np.eye(d)); Qx=0.5*(Qx+Qx.conj().T)
    print(f"t={t} j={j}: |[Q,A]|={np.linalg.norm(Q@A-A@Q):.1e} |[Q,H]|={np.linalg.norm(Q@H-H@Q):.1e} |QG-GQx|={np.linalg.norm(Q@G-G@Qx):.1e}"
          f" maxq field={q.max():.2f} (count {int(np.sum(np.abs(q-q.max())<1e-8))}) maxq gauge={np.linalg.eigvalsh(Qx).max():.2f}")
    # charge content of the circle-breaking perturbation, by |dq|
    for eps in (1e-3,):
        Hp,Gp=C.hessian_and_gauge([t,1+eps,1],j); Ap=Hp+Gp@Gp.conj().T
        Hm,Gm=C.hessian_and_gauge([t,1-eps,1],j); Am=Hm+Gm@Gm.conj().T
        A1=(Ap-Am)/(2*eps); A2=(Ap+Am-2*A)/(eps**2)
        B1=U.conj().T@A1@U; B2=U.conj().T@A2@U; dq=np.abs(q[:,None]-q[None,:])
        c1={k:float(np.abs(B1[np.abs(dq-k)<1e-6]).max()) for k in (0,1,2,3,4)}
        c2={k:float(np.abs(B2[np.abs(dq-k)<1e-6]).max()) for k in (0,1,2,3,4,5,6)}
        print("   A1 by |dq|:",{k:f"{v:.1e}" for k,v in c1.items()}," A2 by |dq|:",{k:f"{v:.1e}" for k,v in c2.items()})
    # Kramers check at eps=0 for odd j: all eigenvalues doubled
    w=np.linalg.eigvalsh(A); print("   min spacing of sorted eigenvalue pairs (odd j => ~0):", f"{np.max(np.abs(w[0::2]-w[1::2])):.1e}" if j%2 else "-")
