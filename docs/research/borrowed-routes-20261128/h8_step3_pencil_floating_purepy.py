# Floating (NOT certified) cross-check of the projected-Gaussian 2x2 pencil, pure Python (no numpy).
# Independent of h8_step3_projected_gaussian_floating.py: analytic |J|, Isserlis moments, full pencil mu_2.
# Usage: python3 h8_step3_pencil_floating_purepy.py g2 alpha nq cutsig
import math, sys, itertools, random
s3=math.sqrt(3.0); u=1/s3; om=[s3,u,u]
def zeros(n,m): return [[0.0]*m for _ in range(n)]
def mm(X,Y): return [[sum(X[i][k]*Y[k][j] for k in range(len(Y))) for j in range(len(Y[0]))] for i in range(len(X))]
def tr(X): return [list(r) for r in zip(*X)]
def mv(X,v): return [sum(X[i][k]*v[k] for k in range(len(v))) for i in range(len(X))]
def dot(a,b): return sum(x*y for x,y in zip(a,b))
def inv_det(X):
    n=len(X); A=[list(r)+[1.0 if i==j else 0.0 for j in range(n)] for i,r in enumerate(X)]; d=1.0
    for c in range(n):
        p=max(range(c,n),key=lambda r:abs(A[r][c])); 
        if p!=c: A[c],A[p]=A[p],A[c]; d=-d
        d*=A[c][c]; pv=A[c][c]; A[c]=[x/pv for x in A[c]]
        for r in range(n):
            if r!=c and A[r][c]!=0:
                f=A[r][c]; A[r]=[x-f*y for x,y in zip(A[r],A[c])]
    return [r[n:] for r in A], d
idx=lambda k,c:3*k+c
# --- J = Hess W(a), analytic, g-free; check by finite differences ---
ap=[u,1.0,1.0]
J=zeros(9,9)
for k in range(3):
    for c in range(3): J[idx(k,c)][idx(k,c)]=om[k]
for k in range(3):
    for l in range(3):
        if k==l: continue
        m=3-k-l
        J[idx(k,k)][idx(l,l)]=-ap[m]; J[idx(k,l)][idx(l,k)]=+ap[m]
def cof(q):
    return [[q[(i+1)%3][(j+1)%3]*q[(i+2)%3][(j+2)%3]-q[(i+1)%3][(j+2)%3]*q[(i+2)%3][(j+1)%3] for j in range(3)] for i in range(3)]
def gradW(v,g):
    q=[v[0:3],v[3:6],v[6:9]]; C=cof(q); return [om[k]*q[k][c]-g*C[k][c] for k in range(3) for c in range(3)]
g0=0.37; a0=[ap[k]/g0 if k==c else 0.0 for k in range(3) for c in range(3)]
print("max|gradW(a)| =",max(abs(x) for x in gradW(a0,g0)))
e=1e-6; Jfd=zeros(9,9)
for i in range(9):
    dp=list(a0); dm=list(a0); dp[i]+=e; dm[i]-=e
    gp,gm=gradW(dp,g0),gradW(dm,g0)
    for j in range(9): Jfd[j][i]=(gp[j]-gm[j])/(2*e)
print("max|J-Jfd| =",max(abs(J[i][j]-Jfd[i][j]) for i in range(9) for j in range(9)))
# |J| + alpha P_ker, analytic
wC=(math.sqrt(11)-s3)/2; r=(s3+math.sqrt(11))/4
vC=[0.0]*9; nrm=math.sqrt(1+2*r*r); vC[idx(0,0)]=1/nrm; vC[idx(1,1)]=r/nrm; vC[idx(2,2)]=r/nrm
Jv=mv(J,vC); print("J vC + wC vC residual:",max(abs(Jv[i]+wC*vC[i]) for i in range(9)))
kers=[]
for (i,j) in [(0,1),(0,2),(1,2)]:
    v=[0.0]*9; v[idx(i,j)]=ap[i]; v[idx(j,i)]=-ap[j]   # a' X with X=E_ij-E_ji : (a'X)_ij=a'_i, (a'X)_ji=-a'_j
    n=math.sqrt(dot(v,v)); kers.append([x/n for x in v])
print("|J k| for gauge k:",[max(abs(x) for x in mv(J,k)) for k in kers])
OmD=[om[k] for k in range(3) for c in range(3)]
print("competitor: e^T|J|e =",wC," e^T Omega e =",sum(OmD[i]*vC[i]**2 for i in range(9)),
      " exact (19+sqrt33)/(sqrt3(11+sqrt33)) =",(19+math.sqrt(33))/(s3*(11+math.sqrt(33))))
def Amat(alpha):
    A=[row[:] for row in J]
    for i in range(9):
        for j in range(9): A[i][j]+=2*wC*vC[i]*vC[j]+sum(alpha*k[i]*k[j] for k in kers)
    return A
# --- Gaussian moments (Isserlis, noncentral) ---
def E2(m,S,i,j): return m[i]*m[j]+S[i][j]
def E4(m,S,i,j,k,l):
    return (m[i]*m[j]*m[k]*m[l]+m[i]*m[j]*S[k][l]+m[i]*m[k]*S[j][l]+m[i]*m[l]*S[j][k]+m[j]*m[k]*S[i][l]
            +m[j]*m[l]*S[i][k]+m[k]*m[l]*S[i][j]+S[i][j]*S[k][l]+S[i][k]*S[j][l]+S[i][l]*S[j][k])
def E3(m,S,i,j,k): return m[i]*m[j]*m[k]+m[i]*S[j][k]+m[j]*S[i][k]+m[k]*S[i][j]
cofterms={}
for k in range(3):
    for c in range(3):
        k1,k2=(k+1)%3,(k+2)%3; c1,c2=(c+1)%3,(c+2)%3
        cofterms[(k,c)]=[(1.0,idx(k1,c1),idx(k2,c2)),(-1.0,idx(k1,c2),idx(k2,c1))]
def EV0(m,S,g):
    tot=0.0
    for k in range(3):
        for c in range(3):
            i=idx(k,c); T=cofterms[(k,c)]
            e_qq=E2(m,S,i,i); e_qc=sum(s*E3(m,S,i,p,q) for s,p,q in T)
            e_cc=sum(s1*s2*E4(m,S,p1,q1,p2,q2) for s1,p1,q1 in T for s2,p2,q2 in T)
            tot+=om[k]**2*e_qq-2*om[k]*g*e_qc+g*g*e_cc
    return 0.5*tot
def pair(A1,a1,A2,a2,g):
    C=[[A1[i][j]+A2[i][j] for j in range(9)] for i in range(9)]; Sg,dC=inv_det(C)
    c=mv(Sg,[x+y for x,y in zip(mv(A1,a1),mv(A2,a2))])
    logN=0.5*(dot(c,mv(C,c))-dot(a1,mv(A1,a1))-dot(a2,mv(A2,a2)))+4.5*math.log(2*math.pi)-0.5*math.log(dC)
    A12=mm(A1,A2); kin=0.5*(sum(A12[i][j]*Sg[j][i] for i in range(9) for j in range(9))
                           +dot([x-y for x,y in zip(c,a1)],mv(A12,[x-y for x,y in zip(c,a2)])))
    return logN, kin+EV0(c,Sg,g)
def rot(th):
    a=math.sqrt(dot(th,th))
    if a<1e-15: return [[1.0 if i==j else 0.0 for j in range(3)] for i in range(3)]
    k=[x/a for x in th]; K=[[0,-k[2],k[1]],[k[2],0,-k[0]],[-k[1],k[0],0]]; KK=mm(K,K)
    return [[(1.0 if i==j else 0.0)+math.sin(a)*K[i][j]+(1-math.cos(a))*KK[i][j] for j in range(3)] for i in range(3)]
def MO(O): return [[(O[c][d] if k==l else 0.0) for l in range(3) for d in range(3)] for k in range(3) for c in range(3)]
def run(g2,alpha,nq,cutsig):
    g=math.sqrt(g2); A=Amat(alpha); a=[ap[k]/g if k==c else 0.0 for k in range(3) for c in range(3)]
    # circle-invariance check of A: L_R = kron(R,R)
    ph=0.7; R=[[1,0,0],[0,math.cos(ph),-math.sin(ph)],[0,math.sin(ph),math.cos(ph)]]
    LR=[[R[k][l]*R[c][d] for l in range(3) for d in range(3)] for k in range(3) for c in range(3)]
    X=mm(LR,A); Y=mm(A,LR); print("  [A,L_R] max:",max(abs(X[i][j]-Y[i][j]) for i in range(9) for j in range(9)),
                                 " |L_R a - a|:",max(abs(x-y) for x,y in zip(mv(LR,a),a)))
    Om=[[OmD[i] if i==j else 0.0 for j in range(9)] for i in range(9)]; z=[0.0]*9
    l00,h00=pair(Om,z,Om,z,g); l01,h01=pair(Om,z,A,a,g)
    def NO(O):
        M=MO(O); B=mm(tr(M),mm(A,M)); b=mv(tr(M),a); return pair(A,a,B,b,g)
    # width estimate in theta
    hh=1e-2; L0=NO(rot([0,0,0]))[0]; sig=max(math.sqrt(hh*hh/(-2*(NO(rot([hh*x for x in ev]))[0]-L0))) for ev in ([1,0,0],[0,1,0],[0,0,1]))
    Rr=min(cutsig*sig,math.pi); gr=[-Rr+2*Rr*i/(nq-1) for i in range(nq)]; dv=(gr[1]-gr[0])**3
    S1=SH=0.0
    for th in itertools.product(gr,repeat=3):
        n=math.sqrt(dot(th,th))
        if n>Rr: continue
        jac=(2-2*math.cos(n))/n**2 if n>1e-12 else 1.0
        lN,h=NO(rot(list(th))); w=math.exp(lN)*jac*dv/(8*math.pi**2); S1+=w; SH+=w*h
    # pencil, scaled by m00 and m11 to avoid under/overflow
    m00=1.0; m11=1.0; m01=math.exp(l01-0.5*(l00+math.log(S1))); k00=h00; k11=SH/S1; k01=m01*h01
    # det(K - mu M)=0
    aa=m00*m11-m01*m01; bb=-(k00*m11+k11*m00-2*k01*m01); cc=k00*k11-k01*k01
    disc=math.sqrt(max(bb*bb-4*aa*cc,0)); mu2=(-bb+disc)/(2*aa); mu1=(-bb-disc)/(2*aa)
    return dict(R_phi0=h00,R_psi1=k11,rel_overlap=m01,mu1=mu1,mu2=mu2,sig=sig,margin=5.2731-mu2,
                unaveraged=NO(rot([0,0,0]))[1])
g2=float(sys.argv[1]); alpha=float(sys.argv[2]); nq=int(sys.argv[3]); cs=float(sys.argv[4])
res=run(g2,alpha,nq,cs); print(f"g2={g2} alpha={alpha} nq={nq} cut={cs}:",{k:round(v,6) if abs(v)>1e-4 else v for k,v in res.items()})
