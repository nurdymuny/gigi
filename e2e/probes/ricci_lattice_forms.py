"""Independent mass oracle: ambient R4 metric, not native Maurer-Cartan code."""
import itertools
import numpy as np


def masses(points,g,order):
    points=np.asarray(points); g=np.asarray(g)
    roots,weights=np.polynomial.legendre.leggauss(order)
    roots=(roots+1)/2;weights=weights/2
    grads=np.vstack([-np.ones(3),np.eye(3)])
    edges=list(itertools.combinations(range(4),2));faces=list(itertools.combinations(range(4),3))
    m1=np.zeros((6,6));m2=np.zeros((4,4));volume=0.
    b=(points[1:]-points[0]).T
    for i,j,k in itertools.product(range(order),repeat=3):
        u,v,w=roots[[i,j,k]]
        local=np.array([u,(1-u)*v,(1-u)*(1-v)*w])
        lam=np.r_[1-local.sum(),local]
        y=lam@points; length=np.linalg.norm(y);x=y/length
        dx=(np.eye(4)-np.outer(x,x))@b/length
        a,bv,c=x[1:]
        cross=np.array([[0,-c,bv],[c,0,-a],[-bv,a,0]])
        frame=np.vstack([-x[1:],x[0]*np.eye(3)+cross])
        ambient=frame@np.diag(g)@frame.T
        metric=dx.T@ambient@dx
        det=np.linalg.det(metric)
        weight=weights[i]*weights[j]*weights[k]*(1-u)**2*(1-v)*np.sqrt(det)
        one=np.array([lam[a]*grads[b]-lam[b]*grads[a] for a,b in edges])
        two=np.array([2*(lam[a]*np.cross(grads[b],grads[c])-lam[b]*np.cross(grads[a],grads[c])+lam[c]*np.cross(grads[a],grads[b])) for a,b,c in faces])
        m1+=weight*(one@np.linalg.inv(metric)@one.T)
        m2+=weight*(two@metric@two.T)/det
        volume+=weight
    return {"edge_mass":m1,"face_mass":m2,"volume":volume}
