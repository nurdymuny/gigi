"""RF2B group-equivariant spherical tetrahedra; not a topology-label fixture."""
import numpy as np
import itertools
from functools import lru_cache
from scipy.spatial import ConvexHull, cKDTree
from monodromy_exact_oracle import exact_closure,S,T


def qfloat(q):
    return np.array([float(a)+float(b)*np.sqrt(5) for a,b in q])


def plus(a,b):
    a=np.asarray(a); b=np.asarray(b)
    return np.concatenate([(a[...,0]*b[...,0]-np.sum(a[...,1:]*b[...,1:],axis=-1))[...,None],
        a[...,0,None]*b[...,1:]+b[...,0,None]*a[...,1:]+np.cross(a[...,1:],b[...,1:])],axis=-1)


def unit(x):
    return x/np.linalg.norm(x,axis=-1,keepdims=True)


def orient(vertices,cells):
    cells=cells.copy()
    reverse=np.linalg.det(vertices[cells])<0
    cells[reverse,2],cells[reverse,3]=cells[reverse,3].copy(),cells[reverse,2].copy()
    return cells


@lru_cache(maxsize=4)
def mesh(level=0):
    if type(level) is not int or not 0<=level<=3:
        raise ValueError("mesh level must be an integer in [0,3]")
    if level==0:
        vertices=np.array([qfloat(q) for q in exact_closure([S,T])])
        cells=ConvexHull(vertices).simplices
    else:
        parent=mesh(level-1)
        vertices=parent["vertices"]; old=parent["tetrahedra"]
        pairs=list(itertools.combinations(range(4),2))
        edges,indices=np.unique(np.sort(old[:,pairs].reshape(-1,2),axis=1),axis=0,return_inverse=True)
        mids=indices.reshape(len(old),6)+len(vertices)
        centers=np.arange(len(old))+len(vertices)+len(edges)
        vertices=np.vstack([vertices,unit(vertices[edges].sum(axis=1)),unit(vertices[old].sum(axis=1))])
        mid={pair:mids[:,i] for i,pair in enumerate(pairs)}
        def m(i,j): return mid[tuple(sorted((i,j)))]
        children=[]
        for i in range(4):
            neighbors=[j for j in range(4) if j!=i]
            triangle=[m(i,j) for j in neighbors]
            children.append(np.column_stack([old[:,i],*triangle]))
            children.append(np.column_stack([centers,*triangle]))
        for i,j,k in itertools.combinations(range(4),3):
            children.append(np.column_stack([centers,m(i,j),m(j,k),m(k,i)]))
        cells=np.vstack(children)
    cells=orient(vertices,cells)
    edges=np.unique(np.sort(cells[:,list(itertools.combinations(range(4),2))].reshape(-1,2),axis=1),axis=0)
    faces=np.unique(np.sort(cells[:,list(itertools.combinations(range(4),3))].reshape(-1,3),axis=1),axis=0)
    return {"vertices":vertices,"tetrahedra":cells,"edges":edges,"faces":faces}


class Union:
    def __init__(self,n): self.parents=list(range(n))
    def root(self,i):
        while self.parents[i]!=i:
            self.parents[i]=self.parents[self.parents[i]]; i=self.parents[i]
        return i
    def join(self,a,b):
        a,b=self.root(a),self.root(b)
        if a!=b:self.parents[b]=a


def validate(m):
    v,c,e,f=(m[k] for k in ["vertices","tetrahedra","edges","faces"])
    if not np.isfinite(v).all() or np.max(np.abs(np.linalg.norm(v,axis=1)-1))>1e-12:
        raise ValueError("finite unit vertices required")
    if c.min()<0 or c.max()>=len(v) or (np.linalg.det(v[c])<=1e-16).any():
        raise ValueError("invalid or degenerate oriented tetrahedron")
    all_faces=np.concatenate([np.delete(c,k,axis=1) for k in range(4)])
    parity=np.sum(np.stack([all_faces[:,i]>all_faces[:,j] for i,j in [(0,1),(0,2),(1,2)]]),axis=0)
    signs=np.tile(np.repeat([1,-1,1,-1],len(c)),1)*(-1.)**parity
    unique,inv,counts=np.unique(np.sort(all_faces,axis=1),axis=0,return_inverse=True,return_counts=True)
    if not np.array_equal(unique,f) or not (counts==2).all() or np.any(np.bincount(inv,weights=signs)):
        raise ValueError("closed oriented tetrahedral face incidence required")
    expected_edges=np.unique(np.sort(c[:,list(itertools.combinations(range(4),2))].reshape(-1,2),axis=1),axis=0)
    if not np.array_equal(expected_edges,e):raise ValueError("edge table mismatch")
    if len(v)-len(e)+len(f)-len(c)!=0:raise ValueError("3D Euler characteristic")
    link_euler=np.bincount(e.ravel(),minlength=len(v))-np.bincount(f.ravel(),minlength=len(v))+np.bincount(c.ravel(),minlength=len(v))
    if not (link_euler==2).all():raise ValueError("vertex link must have Euler characteristic two")
    link=Union(2*len(e)); global_components=Union(len(v)); directed={}
    for i,(a,b) in enumerate(e):
        directed[int(a),int(b)]=2*i; directed[int(b),int(a)]=2*i+1
        global_components.join(int(a),int(b))
    for a,b,d in f:
        link.join(directed[a,b],directed[a,d]);link.join(directed[b,a],directed[b,d]);link.join(directed[d,a],directed[d,b])
    roots={}
    for (a,b),idx in directed.items():roots.setdefault(a,set()).add(link.root(idx))
    if len(roots)!=len(v) or any(len(r)!=1 for r in roots.values()):raise ValueError("disconnected vertex link")
    if len({global_components.root(i) for i in range(len(v))})!=1:raise ValueError("disconnected cover")
    h=np.arccos(np.clip(np.sum(v[e[:,0]]*v[e[:,1]],axis=1),-1,1))
    return {"vertices":len(v),"edges":len(e),"faces":len(f),"tetrahedra":len(c),
        "h_max":float(h.max()),"h_min":float(h.min()),"vertex_links_checked":len(v),"euler_3d":0}


def deck_check(m):
    v=m["vertices"]; tree=cKDTree(v)
    tables=[np.arange(len(v))[:,None],m["edges"],m["faces"],np.sort(m["tetrahedra"],axis=1)]
    indices=[{tuple(row):i for i,row in enumerate(t)} for t in tables]
    orbits=[Union(len(t)) for t in tables]
    for q in [qfloat(S),qfloat(T)]:
        distance,permutation=tree.query(plus(q,v))
        if distance.max()>2e-12 or len(np.unique(permutation))!=len(v):raise ValueError("deck vertex action does not preserve mesh")
        for table,index,orbit in zip(tables,indices,orbits):
            for i,row in enumerate(np.sort(permutation[table],axis=1)):
                if tuple(row) not in index:raise ValueError("deck action does not preserve cells")
                orbit.join(i,index[tuple(row)])
    counts=[]
    for table,orbit in zip(tables,orbits):
        _,sizes=np.unique([orbit.root(i) for i in range(len(table))],return_counts=True)
        if not (sizes==120).all():raise ValueError("nonfree cell orbit")
        counts.append(len(sizes))
    return {"deck_order":120,"quotient_cell_counts":counts,"cover_divisor":120}


def volume(m,g=(1.,1.,1.)):
    points=m["vertices"][m["tetrahedra"]]
    b=(points[:,1:]-points[:,0,None]).transpose(0,2,1)
    total=0.
    for i in range(4):
        weights=np.full(4,(5-np.sqrt(5))/20);weights[i]=(5+3*np.sqrt(5))/20
        y=np.einsum('a,tai->ti',weights,points); length=np.linalg.norm(y,axis=1)
        x=y/length[:,None]
        dx=(b-x[:,:,None]*np.einsum('ti,tij->tj',x,b)[:,None,:])/length[:,None,None]
        conjugate=x.copy();conjugate[:,1:]*=-1
        maurer=plus(conjugate[:,None,:],dx.transpose(0,2,1))[:,:,1:].transpose(0,2,1)
        metric=np.einsum('tia,i,tib->tab',maurer,np.asarray(g),maurer)
        total+=np.sqrt(np.linalg.det(metric)).sum()/24
    return float(total)
