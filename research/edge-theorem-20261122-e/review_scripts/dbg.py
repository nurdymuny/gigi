import sys, numpy as np
sys.path.insert(0,'/Users/bdavis/.claude/jobs/c8f0f39d/tmp/rtc/scripts')
import ricci_su2_exact_edge as E
bl=E.Blocks(); ch=bl.ch
X,Y,T=0.05,1.5,4.0; d=bl.data(X,Y,T)
for l in (2,10,40,150):
    st,B=bl.block(d,2*l,l,T); k=[i for i,(m,a) in enumerate(st) if abs(m-l)<1e-9 and ch[a]==(1,-1)][0]
    Ds=B[k,k].real
    print(l,'Ds-lam',Ds-E.lam_tw(X,Y,T,2*l),[((m,ch[a]),round(abs(B[i,k]),3),round(B[i,i].real-Ds,2)) for i,(m,a) in enumerate(st) if i!=k])
