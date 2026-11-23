"""Orthographic, per-pixel depth-sorted transparency for small tetrahedral scenes.

All faces and edges contribute at their actual depth. Shared faces/edges are
deduplicated, and no object receives a display-priority override. Tiling bounds
memory independently of image resolution. Labels remain vector artwork outside
this renderer.
"""
from itertools import combinations

import numpy as np
from matplotlib.colors import to_rgb

FACES = tuple(combinations(range(4), 3))
EDGES = tuple(combinations(range(4), 2))
COLORS = {"K": "#2363a0", "ring1": "#bc693b", "chosen": "#168378", "ring2": "#9daeb7"}
PRIORITY = {"K": 0, "ring1": 1, "chosen": 2, "ring2": 3}
OPACITY = {"K": .40, "ring1": .19, "chosen": .15, "ring2": .035}
EDGE_ALPHA = {"K": .95, "ring1": .82, "chosen": .75, "ring2": .28}


def composite(depths, alphas, colors):
    """Straight-alpha over, far to near, for each pixel independently."""
    order = np.argsort(depths, axis=0, kind="stable")
    sorted_alpha = np.take_along_axis(alphas, order, axis=0)
    result = np.ones((*depths.shape[1:], 3), dtype=np.float32)
    for i in range(len(depths)):
        a = sorted_alpha[i][..., None]
        rgb = colors[order[i]]
        result = a*rgb + (1-a)*result
    return result


def geometry_items(tets, cam, explode=0.):
    face_map, edge_map = {}, {}
    light = cam @ np.array([-.4, .6, 1.])
    light /= np.linalg.norm(light)
    for t in tets:
        ring = t["ring"]
        v = np.array(t["v"], float)
        if ring != "K":
            v += explode*v.mean(axis=0)
        p = v @ cam
        for f in FACES:
            key = tuple(sorted(tuple(x) for x in v[list(f)]))
            if key in face_map and PRIORITY[face_map[key][3]] <= PRIORITY[ring]:
                continue
            n = np.cross(v[f[1]]-v[f[0]], v[f[2]]-v[f[0]])
            n /= np.linalg.norm(n)
            other = next(i for i in range(4) if i not in f)
            if n @ (v[other]-v[f[0]]) > 0:
                n = -n
            # Two-sided diffuse lighting changes facet tone, not geometry/depth.
            diffuse = .48 + .52*abs(float(n @ light))
            rgb = np.array(to_rgb(COLORS[ring]))*diffuse
            face_map[key] = ("face", p[list(f)], rgb, ring)
        for e in EDGES:
            key = tuple(sorted(tuple(x) for x in v[list(e)]))
            if key in edge_map and PRIORITY[edge_map[key][3]] <= PRIORITY[ring]:
                continue
            rgb = np.array(to_rgb(COLORS[ring]))*.67
            edge_map[key] = ("edge", p[list(e)], rgb, ring)
    return list(face_map.values()) + list(edge_map.values())


def render(tets, cam, limits, width=1050, explode=0.):
    x0,x1,y0,y1 = limits
    height = max(1, round(width*(y1-y0)/(x1-x0)))
    dx,dy = (x1-x0)/width,(y1-y0)/height
    items = geometry_items(tets, cam, explode)
    canvas = np.ones((height,width,3),dtype=np.float32)
    tile = 112
    for iy in range(0,height,tile):
        for ix in range(0,width,tile):
            th,tw = min(tile,height-iy),min(tile,width-ix)
            xx,yy = np.meshgrid(x0+(np.arange(ix,ix+tw)+.5)*dx,
                                y0+(np.arange(iy,iy+th)+.5)*dy)
            layers_z,layers_a,colors = [],[],[]
            for kind,p,rgb,ring in items:
                radius = 1.9 if ring=="K" else 1.25
                margin = 3*max(dx,dy)
                if (p[:,0].max()+margin < xx.min() or p[:,0].min()-margin > xx.max()
                    or p[:,1].max()+margin < yy.min() or p[:,1].min()-margin > yy.max()):
                    continue
                if kind == "face":
                    a,b,c = p
                    den = (b[1]-c[1])*(a[0]-c[0])+(c[0]-b[0])*(a[1]-c[1])
                    if abs(den)<1e-15:
                        continue
                    u = ((b[1]-c[1])*(xx-c[0])+(c[0]-b[0])*(yy-c[1]))/den
                    v = ((c[1]-a[1])*(xx-c[0])+(a[0]-c[0])*(yy-c[1]))/den
                    w = 1-u-v
                    mask = (u>=0)&(v>=0)&(w>=0)
                    z = u*a[2]+v*b[2]+w*c[2]
                    alpha = np.where(mask,OPACITY[ring],0.).astype(np.float32)
                else:
                    a,b=p
                    ab=(b[:2]-a[:2])/np.array([dx,dy])
                    length=float(ab @ ab)
                    if length < 1e-10:
                        continue
                    px,py=(xx-a[0])/dx,(yy-a[1])/dy
                    t=np.clip((px*ab[0]+py*ab[1])/length,0,1)
                    distance=np.sqrt((px-t*ab[0])**2+(py-t*ab[1])**2)
                    alpha=(np.clip(radius+.5-distance,0,1)*EDGE_ALPHA[ring]).astype(np.float32)
                    # Coplanar boundary lines sit infinitesimally above their faces.
                    z=a[2]+t*(b[2]-a[2])+1e-9
                    mask=alpha>0
                if not mask.any():
                    continue
                layers_z.append(np.where(mask,z,-np.inf).astype(np.float32))
                layers_a.append(alpha)
                colors.append(rgb)
            if layers_z:
                canvas[iy:iy+th,ix:ix+tw]=composite(np.stack(layers_z),np.stack(layers_a),np.array(colors,dtype=np.float32))
    return np.round(np.clip(canvas,0,1)*255).astype(np.uint8)
