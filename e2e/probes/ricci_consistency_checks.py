"""Independent Stokes check on radial S3 triangles."""
import itertools
import numpy as np


def triangle_flux(points,order=8):
    p=np.asarray(points);b=(p[1:]-p[0]).T
    nodes,weights=np.polynomial.legendre.leggauss(order);nodes=(nodes+1)/2;weights/=2
    value=0.
    for i,j in itertools.product(range(order),repeat=2):
        u,v=nodes[i],nodes[j];local=np.array([u,(1-u)*v]);lam=np.r_[1-local.sum(),local]
        y=lam@p;length=np.linalg.norm(y);x=y/length;dx=(np.eye(4)-np.outer(x,x))@b/length
        a,bb,c=x[1:];cross=np.array([[0,-c,bb],[c,0,-a],[-bb,a,0]])
        frame=np.vstack([-x[1:],x[0]*np.eye(3)+cross]);theta=frame.T@dx
        value+=weights[i]*weights[j]*(1-u)*(-2)*(theta[1,0]*theta[2,1]-theta[1,1]*theta[2,0])
    return float(value)
