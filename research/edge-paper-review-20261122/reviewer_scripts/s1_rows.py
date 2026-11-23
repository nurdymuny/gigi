import sys
sys.path.insert(0,'./scripts')
import sympy as sp
import ricci_su2_exact_edge as E
x,y,t=E.x,E.y,E.t
GC=E.GC
s,c,_,_,gmat=GC.background(sp.sqrt, sp.Integer(1), t)
print("c sample", {k:v for k,v in c.items() if v!=0})
print("gmat",gmat)
m=[sp.Matrix([x,0,0]),sp.Matrix([0,y,0]),sp.Matrix([0,0,y])]
I3=sp.eye(3); cross=E.cross
Om=[sp.kronecker_product(I3,cross(m[i]))+sp.kronecker_product(-sp.Matrix(gmat[i]),I3) for i in range(3)]
Ric=sp.diag(2*t,4-2*t,4-2*t)
T=sp.zeros(9,9); F={}
for i in range(3):
  for k in range(3):
    Fik=cross(m[i])*m[k]
    for l in range(3): Fik=Fik-c[(i,k,l)]*m[l]
    F[(i,k)]=sp.simplify(Fik)
    T[3*i:3*i+3,3*k:3*k+3]=cross(Fik)
print("F12",F[(1,2)].T,"F01",F[(0,1)].T,"F02",F[(0,2)].T)
R0=sp.kronecker_product(Ric,I3)-2*T
r2=sp.sqrt(2)
U3=sp.Matrix([[0,1,0],[1/r2,0,1/r2],[-sp.I/r2,0,sp.I/r2]])
U=sp.kronecker_product(U3,U3)
tr=lambda M:(U.H*M*U).applyfunc(sp.simplify)
Omc=[tr(O) for O in Om]
w=[sp.simplify(Omc[0][a,a]/sp.I) for a in range(9)]
Op=(Omc[1]+sp.I*Omc[2]).applyfunc(sp.simplify); On=(Omc[1]-sp.I*Omc[2]).applyfunc(sp.simplify)
R0c=tr(R0)
C=(-(Omc[1]*Omc[1]+Omc[2]*Omc[2])+R0c).applyfunc(sp.simplify)
ch=[(cf,cl) for cf in (-1,0,1) for cl in (-1,0,1)]
for a in range(9): print(ch[a], "w=",w[a], " w-formula diff", sp.simplify(w[a]-(ch[a][0]*(t-2)/sp.sqrt(t)-ch[a][1]*x)))
# antihermitian check
for i in range(3): print("Omega",i,"antiherm:", (Omc[i]+Omc[i].H).applyfunc(sp.simplify)==sp.zeros(9,9))
# charge selection
bad=[]
for a in range(9):
  for b in range(9):
    ca=sum(ch[a]); cb=sum(ch[b])
    if Op[a,b]!=0 and ca!=cb+1: bad.append(('Op',a,b))
    if On[a,b]!=0 and ca!=cb-1: bad.append(('On',a,b))
    if C[a,b]!=0 and ca!=cb: bad.append(('C',a,b))
print("selection violations",bad)
print("Op/On involve x:", any(e.has(x) for e in list(Op)+list(On)))
print("max deg x in C:", max(sp.degree(sp.expand(e),x) if e!=0 else 0 for e in C))
tw=8
print("C_twist", sp.simplify(C[tw,tw]), "diff", sp.simplify(C[tw,tw]-(4*sp.sqrt(t)*x-t-y**2+4)))
print("R0 twist col", [sp.simplify(R0c[a,tw]) for a in range(9)])
print("R0tw - (4-2t-2B)", sp.simplify(R0c[tw,tw]-(4-2*t-2*(y**2-2*sp.sqrt(t)*x))))
print("-(O1^2+O2^2)_tw", sp.simplify(C[tw,tw]-R0c[tw,tw]))
print("Op beta col", [sp.simplify(Op[a,tw]) for a in range(9)])
j=sp.symbols('j',nonnegative=True)
D=(w[tw]-j/sp.sqrt(t))**2+2*j+C[tw,tw]
lam=(x+(j+2)/sp.sqrt(t))**2+2*sp.sqrt(t)*x-y**2
print("D-lam", sp.simplify(sp.expand(D-lam)))
# Berger: Omega_pm moduli
bx=(2-t)/sp.sqrt(t)
mods=set()
for M_ in (Op,On):
  for e in M_:
    if e!=0: mods.add(sp.simplify(sp.Abs(e.subs({x:bx,y:sp.sqrt(t)}))**2))
print("Berger |Om_pm|^2 entries", mods)
# Is Berger field the LC connection? compute LC Gamma in orthonormal frame
import itertools
