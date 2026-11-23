"""Explicit-mesh version of the frozen RF2B orbit recipe. No physics here."""
import itertools
from collections import Counter
import numpy as np
from scipy.spatial import cKDTree
from ricci_lattice_mesh import plus,qfloat,Union
from monodromy_exact_oracle import exact_closure,S,T

EDGES=list(itertools.combinations(range(4),2))


def orbit_fixture_for_mesh(m):
    v=m['vertices'];edges=m['edges'];cells=m['tetrahedra'];tree=cKDTree(v)
    edge_index={tuple(e):i for i,e in enumerate(edges)}
    cell_index={tuple(sorted(c)):i for i,c in enumerate(cells)}
    groups=[Union(len(edges)),Union(len(cells))]
    for generator in [S,T]:
        distance,permutation=tree.query(plus(qfloat(generator),v))
        if distance.max()>2e-12:raise ValueError('deck map missing vertex')
        for table,index,group in zip([edges,cells],[edge_index,cell_index],groups):
            for i,row in enumerate(np.sort(permutation[table],axis=1)):
                group.join(i,index[tuple(row)])
    representatives=[]
    for group in groups:
        _,indices,counts=np.unique([group.root(i) for i in range(len(group.parents))],return_index=True,return_counts=True)
        if not (counts==120).all():raise ValueError('incomplete deck orbit')
        representatives.append(np.sort(indices))
    ereps,creps=representatives
    edge_orbit=np.full(len(edges),-1,dtype=int);reverse=np.zeros(len(edges),dtype=bool)
    orders=[Counter() for _ in creps];visited=np.zeros(len(cells),dtype=int)
    for element in exact_closure([S,T]):
        q=qfloat(element)
        dist,images=tree.query(plus(q,v[edges[ereps]]).reshape(-1,4))
        if dist.max()>2e-12:raise ValueError('edge deck image absent')
        for orbit,(a,b) in enumerate(images.reshape(-1,2)):
            eid=edge_index[min(a,b),max(a,b)]
            if edge_orbit[eid]!=-1:raise ValueError('edge orbit repeated')
            edge_orbit[eid]=orbit;reverse[eid]=a>b
        dist,images=tree.query(plus(q,v[cells[creps]]).reshape(-1,4))
        if dist.max()>2e-12:raise ValueError('cell deck image absent')
        for orbit,ordered in enumerate(images.reshape(-1,4)):
            cid=cell_index[tuple(sorted(ordered))];visited[cid]+=1
            local={vertex:i for i,vertex in enumerate(ordered)}
            orders[orbit][tuple(local[a] for a in cells[cid])]+=1
    if (edge_orbit<0).any() or not (visited==1).all():raise ValueError('incomplete cover accounting')
    reduced=[]
    for representative,order_counts in zip(creps,orders):
        if sum(order_counts.values())!=120:raise ValueError('orbit weight incomplete')
        for permutation,count in sorted(order_counts.items()):
            ordered=cells[representative][list(permutation)]
            ids=[];rev=[]
            for i,j in EDGES:
                a,b=ordered[i],ordered[j];eid=edge_index[min(a,b),max(a,b)]
                ids.append(int(edge_orbit[eid]));rev.append(bool((a>b)^reverse[eid]))
            reduced.append({'vertices':v[ordered].tolist(),'edges':ids,'reverse':rev,'weight':count/120})
    return {'cells':reduced,'edge_points':v[edges[ereps]].tolist(),
        'cover_edge_orbit':edge_orbit.tolist(),'cover_edge_reverse':reverse.tolist(),
        'cell_order_histograms':[{'representative':int(rep),'orders':[[list(p),n] for p,n in sorted(h.items())]} for rep,h in zip(creps,orders)]}


def cover_fixture_for_mesh(m):
    index={tuple(e):i for i,e in enumerate(m['edges'])};cells=[]
    for cell in m['tetrahedra']:
        pairs=[(int(cell[i]),int(cell[j])) for i,j in EDGES]
        cells.append({'vertices':m['vertices'][cell].tolist(),
            'edges':[index[min(a,b),max(a,b)] for a,b in pairs],
            'reverse':[a>b for a,b in pairs],'weight':1.})
    return {'edge_points':m['vertices'][m['edges']].tolist(),'cells':cells}
