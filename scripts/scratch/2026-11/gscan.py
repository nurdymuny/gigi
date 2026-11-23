import numpy as np
types=[(a,b) for a in (-1,0,1) for b in (-1,0,1)]
def ing(x,y,t):
    st=np.sqrt(t); w={(-1,-1):-st+x+2/st,(-1,0):(2-t)/st,(-1,1):-st-x+2/st,(0,-1):x,(0,0):0,(0,1):-x,(1,-1):st+x-2/st,(1,0):(t-2)/st,(1,1):st-x-2/st}
    Caa={(-1,-1):4*st*x-t-y*y+4,(-1,0):-t+2*y*y+4,(-1,1):-4*st*x-t+3*y*y+4,(0,-1):4*t+y*y,(0,0):4*t+2*y*y,(0,1):4*t+y*y,(1,-1):-4*st*x-t+3*y*y+4,(1,0):-t+2*y*y+4,(1,1):4*st*x-t-y*y+4}
    co=abs(2*y*(st*x-t-2)/st)
    R={(-1,-1):0,(-1,0):co,(-1,1):co,(0,-1):co,(0,0):2*co,(0,1):co,(1,-1):co,(1,0):co,(1,1):0}
    r=np.sqrt(2)
    Pp={(-1,-1):r*(y+st),(-1,0):r*(y+st),(-1,1):r*st,(0,-1):r*(y+st),(0,0):r*(y+st),(0,1):r*st,(1,-1):r*y,(1,0):r*y,(1,1):0}
    Pm={(-1,-1):0,(-1,0):r*y,(-1,1):r*y,(0,-1):r*st,(0,0):r*(st+y),(0,1):r*(st+y),(1,-1):r*st,(1,0):r*(st+y),(1,1):r*(st+y)}
    return w,Caa,R,Pp,Pm
def Gmin(l,x,y,t):
    w,Caa,R,Pp,Pm=ing(x,y,t); st=np.sqrt(t); lam=(x+(2*l+2)/st)**2+2*st*x-y*y; best=(np.inf,None)
    for a in types:
        for xi in range(0,int(2*l)+1):
            if (a==(1,1) and xi==0) or (a==(-1,-1) and xi==int(2*l)): continue
            m=l-xi
            D=(w[a]-2*m/st)**2+4*(l*(l+1)-m*m)+Caa[a]
            G=D-R[a]-lam-2*np.sqrt(max(xi*(2*l-xi+1),0))*Pm[a]-2*np.sqrt(max((2*l-xi)*(xi+1),0))*Pp[a]
            if G<best[0]: best=(G,(a,xi))
    return best
for dom,(xs,ys,ts) in {"x in [-2,-0.5]":([-2,-1,-0.5],[0.5,1.5,3],[3,6,10]),"x in [-2,-1]":([-2,-1.5,-1],[0.5,1.5,3],[3,6,10]),"x in [-3,-1.5]":([-3,-2,-1.5],[0.5,1.5,3],[3,6,10])}.items():
    L0=None; worst=None
    for l in [5,10,20,40,80,160,320,640,1280,2560]:
        vals=[Gmin(l,x,y,t) for x in xs for y in ys for t in ts]
        if all(v[0]>0 for v in vals):
            L0=l; break
        worst=min(vals,key=lambda v:v[0])
    print(dom,"plain Gershgorin positive from l =",L0, "| last worst row", worst)
