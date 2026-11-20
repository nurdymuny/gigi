"""Pre-check for M1c (MAGNET_M1_SPEC Amendment 2): leftover moments (G^T)^k z after k rounds of the M1b correction, and the top eigenvalues of G = I1 R1. usage: python scripts/ricci_magnet_m1_neumann_precheck.py (levels 0, 1)"""
import sys, json, numpy as np, multiprocessing as mp, os
sys.path[:0]=['scripts','e2e/probes']
import ricci_magnet_m1b as B, ricci_magnet_m0 as M
def main(level):
    L=M.pg.load_level(level,M.ORDER); cells=L['cells']; ne=len(L['a'])
    hmax=json.loads((M.V._DATA/"output/ricci-variational-20261117/abelian/diagnostic.json").read_text())["cases"][f"level{level}_q6"]["h_max"]
    G=np.zeros((ne,ne)); cnt=np.zeros(ne)
    with mp.get_context('spawn').Pool(6, initializer=B._init, initargs=(level,hmax,None,None)) as pool:
        for re,nv,own,edges,Lm in pool.imap_unordered(B._w_pass1, range(len(cells)), chunksize=2):
            for k,g in enumerate(own): G[g,edges]+=Lm[k]; cnt[g]+=1
    G/=cnt[:,None]
    mo=np.load(f'research/magnet-m1-20261119/moments_L{level}.npz'); z,l1=mo['z'],mo['l1']
    v=[z]
    for k in range(6): v.append(G.T@v[-1])
    med=[float(np.median(np.linalg.norm(x,axis=1)/l1)) for x in v]; nrm=[float(np.linalg.norm(x)) for x in v]
    print(f"n={level+1}: median relative leftover after k corrections (k=0 is M0, k=1 is M1b):", [round(m,4) for m in med])
    print(f"      norm ratios ||(G^T)^(k+1) z|| / ||(G^T)^k z||:", [round(nrm[k+1]/nrm[k],3) for k in range(6)])
    ev=np.sort(np.abs(np.linalg.eigvals(G)))[::-1]; print("      largest |eig(G)|:", np.round(ev[:6],4), " count >0.9:", int((ev>0.9).sum()), "of", ne)
if __name__=='__main__':
    for lv in (0,1): main(lv)
