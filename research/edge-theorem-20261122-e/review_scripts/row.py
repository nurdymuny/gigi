import sys, numpy as np
sys.path.insert(0,'/Users/bdavis/.claude/jobs/c8f0f39d/tmp/rtc/scripts')
import ricci_su2_exact_edge as E
bl=E.Blocks(); ch=bl.ch
for (xv,yv,tv,j) in [(0.33,1.0,4.4,400),(0.33,3.0,4.4,400),(0.33,1.0,4.4,40),(1.3,2.0,2.0,400)]:
    d=bl.data(xv,yv,tv); l=j/2
    st,B=bl.block(d,j,l,tv)
    k=[i for i,(m,a) in enumerate(st) if abs(m-l)<1e-9 and ch[a]==(1,-1)][0]
    print((xv,yv,tv,j))
    for i,(m,a) in enumerate(st):
        if i!=k: print('   ',(m,ch[a]),'|B_ks|=%.3g'%abs(B[i,k]),'gap D_i-D_s=%.4g'%(B[i,i].real-B[k,k].real))
