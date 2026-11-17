"""Independent shape-regularity diagnostic; positivity alone is insufficient."""
import argparse
import itertools
import json
from pathlib import Path
import numpy as np
from ricci_lattice_mesh import mesh


def quality(vertices,cells):
    p=np.asarray(vertices)[cells];b=p[:,1:]-p[:,0,None]
    gram=b@b.transpose(0,2,1);volume=np.sqrt(np.linalg.det(gram))/6
    area=np.zeros(len(cells))
    for i,j,k in itertools.combinations(range(4),3):
        u=p[:,j]-p[:,i];v=p[:,k]-p[:,i]
        area+=np.sqrt(np.sum(u*u,axis=1)*np.sum(v*v,axis=1)-np.sum(u*v,axis=1)**2)/2
    radius=3*volume/area
    diameter=np.max(np.stack([np.linalg.norm(p[:,i]-p[:,j],axis=1) for i,j in itertools.combinations(range(4),2)]),axis=0)
    ratio=radius/diameter
    if not np.isfinite(ratio).all() or np.any(ratio<=0):raise ValueError('degenerate cell shape')
    return {'min_inradius_over_diameter':float(ratio.min()),'median_inradius_over_diameter':float(np.median(ratio)),
        'max_diameter_over_inradius':float(np.max(1/ratio)),'cells':len(cells)}


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('output',type=Path);a=p.parse_args()
    rows=[]
    for level in range(4):
        m=mesh(level);row=dict(level=level,**quality(m['vertices'],m['tetrahedra']));rows.append(row);print(row,flush=True)
    a.output.write_text(json.dumps({'scope':'observed shape quality, not a uniform-in-level bound','levels':rows},indent=2))
