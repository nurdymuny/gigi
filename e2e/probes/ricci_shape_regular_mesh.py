"""RF2C: one flag subdivision, then a fixed edgewise family. RF2B is frozen."""
import numpy as np
import itertools
from functools import lru_cache
from ricci_lattice_mesh import mesh as baseline_mesh,orient,unit

RESOLUTIONS=(1,2,3,5)


@lru_cache(maxsize=8)
def reference_subdivision(n):
    if type(n) is not int or not 1<=n<=16:raise ValueError('integer subdivision n in [1,16] required')
    coordinates=[(i,j,k) for i in range(n+1) for j in range(i+1) for k in range(j+1)]
    index={p:i for i,p in enumerate(coordinates)};cells=[]
    for origin in itertools.product(range(n),repeat=3):
        for permutation in itertools.permutations(range(3)):
            p=list(origin);vertices=[tuple(p)]
            for axis in permutation:
                p=p.copy();p[axis]+=1;vertices.append(tuple(p))
            if all(v in index for v in vertices):cells.append([index[v] for v in vertices])
    xyz=np.array(coordinates,dtype=np.int64)
    bary=np.column_stack([n-xyz[:,0],xyz[:,0]-xyz[:,1],xyz[:,1]-xyz[:,2],xyz[:,2]])
    cells=np.array(cells,dtype=np.int64)
    p=bary[cells,1:];negative=np.linalg.det(p[:,1:]-p[:,0,None])<0
    cells[negative,2],cells[negative,3]=cells[negative,3].copy(),cells[negative,2].copy()
    return bary,cells


@lru_cache(maxsize=4)
def mesh(level=0):
    if type(level) is not int or not 0<=level<len(RESOLUTIONS):raise ValueError('mesh level in [0,3] required')
    n=RESOLUTIONS[level];base=baseline_mesh(0);bary,template=reference_subdivision(n)
    flags=base['tetrahedra'][:,list(itertools.permutations(range(4)))].reshape(-1,4)
    # Exact sparse barycentric keys on the original vertices, denominator 12*n.
    weights=np.column_stack([12*bary[:,0]+6*bary[:,1]+4*bary[:,2]+3*bary[:,3],
        6*bary[:,1]+4*bary[:,2]+3*bary[:,3],4*bary[:,2]+3*bary[:,3],3*bary[:,3]])
    identifiers=np.broadcast_to(flags[:,None,:],(len(flags),len(weights),4)).copy()
    numerators=np.broadcast_to(weights,(len(flags),len(weights),4)).copy()
    identifiers[numerators==0]=-1
    order=np.argsort(identifiers,axis=2)
    identifiers=np.take_along_axis(identifiers,order,axis=2)
    numerators=np.take_along_axis(numerators,order,axis=2)
    keys=np.concatenate([identifiers,numerators],axis=2).reshape(-1,8)
    keys,inverse=np.unique(keys,axis=0,return_inverse=True)
    ids=keys[:,:4];weights_unique=keys[:,4:]
    affine=np.einsum('vki,vk->vi',base['vertices'][np.maximum(ids,0)],weights_unique)/(12*n)
    vertices=unit(affine)
    local=inverse.reshape(len(flags),len(weights))
    cells=orient(vertices,local[:,template].reshape(-1,4))
    edges=np.unique(np.sort(cells[:,list(itertools.combinations(range(4),2))].reshape(-1,2),axis=1),axis=0)
    faces=np.unique(np.sort(cells[:,list(itertools.combinations(range(4),3))].reshape(-1,3),axis=1),axis=0)
    return {'vertices':vertices,'tetrahedra':cells,'edges':edges,'faces':faces,
        'affine_vertices':affine,'node_keys':keys,'denominator':12*n,'subdivision_n':n}
