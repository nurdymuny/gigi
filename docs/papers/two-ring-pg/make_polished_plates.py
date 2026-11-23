"""Non-destructive, data-driven editorial drawings for paper Figures 1--3.

Run from any directory: python path/to/make_polished_plates.py
Reads the saved geometry/measurement receipts only; no research calculation runs.
"""
from __future__ import annotations

from collections import Counter
from itertools import combinations
import hashlib
import json
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.backends.backend_pdf import PdfPages
from matplotlib.patches import Polygon
import numpy as np
from transparent_tetra_renderer import COLORS, render as render_volume

HERE = Path(__file__).resolve().parent
OUT = HERE / "figures" / "polished"
DATA = HERE / "data"
INK, MUTED, RULE = "#20272d", "#616c74", "#d8dfe3"
BLUE, COPPER, TEAL, GREY = (COLORS[k] for k in ("K", "ring1", "chosen", "ring2"))
FACES = list(combinations(range(4), 3))
EDGES = list(combinations(range(4), 2))
plt.rcParams.update({
    "font.family": "sans-serif", "font.sans-serif": ["Arial", "DejaVu Sans"],
    "font.size": 9, "text.color": INK, "mathtext.fontset": "stix",
    "pdf.fonttype": 42, "svg.fonttype": "none", "savefig.facecolor": "white",
})


def norm(x):
    return x / np.linalg.norm(x)


def basis(eye):
    eye = norm(np.asarray(eye, dtype=float))
    up = np.array([0., 0., 1.])
    if abs(eye @ up) > .95:
        up = np.array([0., 1., 0.])
    right = norm(np.cross(up, eye))
    return np.column_stack([right, np.cross(eye, right), eye])


def project(v, cam):
    return np.asarray(v) @ cam


def normal(v, f):
    n = norm(np.cross(v[f[1]]-v[f[0]], v[f[2]]-v[f[0]]))
    other = next(i for i in range(4) if i not in f)
    return -n if n @ (v[other]-v[f[0]]) > 0 else n


def axes(fig, rect, half=.25):
    ax = fig.add_axes(rect)
    ax.set_aspect("equal")
    ax.set_xlim(-half, half)
    ax.set_ylim(-half, half)
    ax.axis("off")
    return ax


def frame(ax, points, pad=1.10):
    p=np.asarray(points)[:,:2]
    low,high=p.min(axis=0),p.max(axis=0)
    mid=(low+high)/2
    width,height=(high-low)*pad
    r=ax.get_position()
    fw,fh=ax.figure.get_size_inches()
    aspect=r.width*fw/(r.height*fh)
    width=max(width,height*aspect)
    height=max(height,width/aspect)
    ax.set_xlim(mid[0]-width/2,mid[0]+width/2)
    ax.set_ylim(mid[1]-height/2,mid[1]+height/2)


def technical_camera(k):
    """Maximize the weakest visible facet, requiring three visible faces."""
    rng=np.random.default_rng(34)
    directions=rng.normal(size=(2400,3))
    directions/=np.linalg.norm(directions,axis=1)[:,None]
    normals=np.array([normal(k,f) for f in FACES])
    areas=np.array([np.linalg.norm(np.cross(k[f[1]]-k[f[0]],k[f[2]]-k[f[0]]))/2 for f in FACES])
    dots=directions@normals.T
    visible=dots>0
    good=np.sum(visible,axis=1)==3
    projected=np.maximum(dots,0)*areas
    worst=np.min(np.where(visible,projected,np.inf),axis=1)
    score=np.where(good,worst,-1)
    eye=directions[int(np.argmax(score))]
    cam=basis(eye)
    # A camera roll, not a geometry deformation: place the long edge diagonally.
    p=project(k,cam)
    i,j=max(EDGES,key=lambda e:np.linalg.norm(p[e[1],:2]-p[e[0],:2]))
    theta=np.arctan2(*(p[j,:2]-p[i,:2])[::-1])
    turn=np.radians(-24)-theta
    c,s=np.cos(turn),np.sin(turn)
    return cam@np.array([[c,s,0],[-s,c,0],[0,0,1]])


def text(fig, x, y, s, size=9, color=INK, **kw):
    return fig.text(x, y, s, fontsize=size, color=color, va="top", **kw)


def rule(fig, y, x0=.045, x1=.955):
    fig.add_artist(plt.Line2D([x0, x1], [y, y], transform=fig.transFigure,
                              color=RULE, lw=.6))


def heading(fig, number, title, subtitle):
    text(fig, .045, .967, "DAVIS GEOMETRIC  /  CURVATURE RECOVERY", 7, MUTED)
    text(fig, .955, .967, f"PLATE {number:02d}", 7, MUTED, ha="right")
    text(fig, .045, .917, title, 20, fontfamily="Palatino Linotype")
    text(fig, .045, .858, subtitle, 8.5, MUTED)
    rule(fig, .827)


def panel(fig, x, y, letter, title, sub=None):
    text(fig, x, y, letter.upper(), 8, BLUE, weight="bold")
    text(fig, x+.028, y+.002, title, 11, weight="bold")
    if sub:
        text(fig, x+.028, y-.031, sub, 8, MUTED)


def legend(fig, y, folded=False):
    items = [(BLUE, "Cell K"), (COPPER, "First ring"), (TEAL, "Second ring" if not folded else "Second ring: retained")]
    if folded:
        items.append((GREY, "Second ring: omitted"))
    xs = [.11, .31, .53] if not folded else [.055, .21, .40, .70]
    for x, (c, label) in zip(xs, items):
        fig.add_artist(plt.Line2D([x, x+.022], [y-.009, y-.009],
                       transform=fig.transFigure, color=c, lw=3))
        text(fig, x+.032, y, label, 8)


def label(ax, p, s, offset=(0, 0), color=INK, size=9, leader=False, **kw):
    return ax.annotate(s, xy=np.asarray(p)[:2], xytext=offset,
        textcoords="offset points", fontsize=size, color=color,
        ha="center", va="center", zorder=200,
        bbox=dict(facecolor="white", edgecolor="none", pad=1.4, alpha=.96),
        arrowprops=dict(arrowstyle="-", lw=.6, color=color, shrinkA=3, shrinkB=3) if leader else None,
        **kw)


def centre_tet(cell):
    return np.array(next(t["v"] for t in cell["tets"] if t["ring"] == "K"))


def assembly(ax, cell, cam, mode="two", centre_label=True):
    tets=[]
    for t in cell["tets"]:
        if mode=="one" and t["ring"] not in ("K","ring1"):
            continue
        ring="chosen" if mode=="two" and t["ring"]=="ring2" else t["ring"]
        tets.append(dict(t,ring=ring))
    limits=(*ax.get_xlim(),*ax.get_ylim())
    rgb=render_volume(tets,cam,limits)
    ax.imshow(rgb,extent=limits,origin="lower",interpolation="lanczos",zorder=1)
    if centre_label:
        p=project(centre_tet(cell).mean(axis=0),cam)
        label(ax,p,"$K$",(-42,30),BLUE,size=12,leader=True)


def exploded_points(cell,cam,factor=1.15):
    return np.vstack([project(np.array(t["v"])+factor*np.mean(t["v"],axis=0)
                     if t["ring"]!="K" else t["v"],cam) for t in cell["tets"]])


def exploded(ax, cell, cam, factor=1.15):
    limits=(*ax.get_xlim(),*ax.get_ylim())
    rgb=render_volume(cell["tets"],cam,limits,explode=factor)
    ax.imshow(rgb,extent=limits,origin="lower",interpolation="lanczos",zorder=1)


def plate_one(geo):
    cell = geo["cells"][0]
    cam = technical_camera(centre_tet(cell))
    fig = plt.figure(figsize=(8.6, 5.8))
    heading(fig, 1, "More neighbourhood. The same measurements.",
            "One cell of the real mesh, in a common chart. Same camera and scale in both views.")
    panel(fig,.055,.788,"a","One ring","Central cell and its face-neighbours")
    panel(fig,.535,.788,"b","Two rings","Neighbours of those neighbours")
    allpoints=np.vstack([project(t["v"],cam) for t in cell["tets"]])
    for x, mode in [(.035,"one"),(.515,"two")]:
        ax = axes(fig,[x,.295,.45,.425])
        frame(ax,allpoints,pad=1.12)
        assembly(ax,cell,cam,mode)
    for x, counts, name in [(.065, "5 cells   /   16 faces   /   11 coefficients", "Exactly determined"),
                              (.545,"15 cells   /   44 faces   /   11 coefficients", "Overdetermined")]:
        text(fig,x,.255,name,12,BLUE if x>.5 else COPPER,weight="bold")
        text(fig,x,.211,counts,8)
    rule(fig,.165)
    legend(fig,.139)
    text(fig,.045,.071,"Translucent faces are composited in their actual depth order. Shared faces are counted once. No displacement.",7.3,MUTED)
    return fig


def plate_two(geo,measure):
    k=centre_tet(geo["cells"][0])
    views=[technical_camera(k)] + [basis(k[int(e[3])]-k[int(e[1])]) for e in measure["edge_views"]]
    half=max(np.ptp(project(k,c)[:,:2],axis=0).max()/2 for c in views)*1.26
    fig=plt.figure(figsize=(8.6,8.1))
    heading(fig,2,"The cell, measured.","Four orthographic views. One scale. Measurements refer to the chordal tetrahedron.")
    positions=[(.055,.443),(.535,.443),(.055,.084),(.535,.084)]
    titles=[("Oblique view","Six true edge lengths"),
            ("Along edge 01","The two endpoints coincide"),
            ("Along edge 23","The two endpoints coincide"),
            ("Along edge 13","The two endpoints coincide")]
    for idx,((x,y),cam,(title,sub)) in enumerate(zip(positions,views,titles)):
        panel(fig,x,y+.354,"abcd"[idx],title,sub)
        ax=axes(fig,[x,y+.079,.41,.235],half)
        p=project(k,cam)
        view_mid=(p[:,:2].min(axis=0)+p[:,:2].max(axis=0))/2
        ax.set_xlim(view_mid[0]-half,view_mid[0]+half)
        ax.set_ylim(view_mid[1]-half,view_mid[1]+half)
        if idx==0:
            assembly(ax,{"tets":[{"ring":"K","v":k.tolist()}]},cam,"one",centre_label=False)
            for n,(i,j) in enumerate(EDGES):
                column,row=n//3,n%3
                tx=x+.045+column*.20; ty=y+.062-row*.021
                text(fig,tx,ty,f"$v_{i}v_{j}$",8.5,MUTED)
                text(fig,tx+.055,ty,f'{measure["chordal_edge_lengths"][f"v{i}v{j}"]:.3f}',8.5)
        else:
            edge=measure["edge_views"][idx-1]
            i,j=int(edge[1]),int(edge[3]); u,v=[a for a in range(4) if a not in (i,j)]
            tri=p[[i,u,v],:2]
            ax.add_patch(Polygon(tri,fc="#e9f0f7",ec="none",zorder=2))
            for a,b in [(i,u),(i,v),(u,v)]:
                ax.plot(p[[a,b],0],p[[a,b],1],color=INK,lw=1.3,zorder=4)
            center=p[i,:2]
            angles=[np.arctan2(*(p[m,:2]-center)[::-1]) for m in (u,v)]
            lo,hi=sorted(angles)
            if hi-lo>np.pi: lo,hi=hi,lo+2*np.pi
            radius=.38*min(np.linalg.norm(p[m,:2]-center) for m in (u,v))
            a=np.linspace(lo,hi,100)
            ax.plot(center[0]+radius*np.cos(a),center[1]+radius*np.sin(a),color=BLUE,lw=1.5,zorder=8)
            mid=(lo+hi)/2
            label(ax,center+radius*1.75*np.array([np.cos(mid),np.sin(mid)]),
                  f'{measure["dihedral_angles_deg"][edge]:.1f}'+r"$^\circ$",size=12,color=BLUE,weight="bold")
        done=set()
        for m in range(4):
            if m in done: continue
            same=[q for q in range(4) if np.linalg.norm(p[q,:2]-p[m,:2])<1e-6]
            done.update(same)
            delta=p[m,:2]-p[:,:2].mean(axis=0)
            off=norm(delta)*12 if np.linalg.norm(delta)>1e-10 else np.array([12.,12.])
            label(ax,p[m],"$"+", ".join(f"v_{s}" for s in same)+"$",off,size=9)
            ax.scatter(*p[m,:2],s=10,color=INK,zorder=80)
        # A true geometric scale bar, independent of camera direction.
        start=view_mid+np.array([-half*.87,-half*.86]); end=start+np.array([.05,0.])
        ax.plot([start[0],end[0]],[start[1],end[1]],color=INK,lw=1)
        for x0 in [start[0],end[0]]:
            ax.plot([x0,x0],[start[1]-.002,start[1]+.002],color=INK,lw=.8)
        label(ax,(start+end)/2,"0.05",(0,-9),size=7.5)
    rule(fig,.064)
    text(fig,.045,.045,"Lengths: chordal units on the unit three-sphere. Rear edges remain visible through the translucent faces.",7.4,MUTED)
    return fig


def plate_three(geo):
    fig=plt.figure(figsize=(8.6,8.3))
    heading(fig,3,"What the second ring contributes.","The full neighbourhood and the smaller, greedily selected stencil. All geometry comes from the same mesh.")
    c0,c2,c7=geo["cells"]
    cam=technical_camera(centre_tet(c0))
    allpoints=np.vstack([project(t["v"],cam) for t in c0["tets"]])
    for i,(mode,title,key) in enumerate([("one","One ring","amp_one"),("two","Two rings","amp_two"),("fold","Folded stencil","amp_fold")]):
        x=.045+i*.315
        panel(fig,x,.792,"abc"[i],title)
        ax=axes(fig,[x-.005,.548,.30,.205])
        frame(ax,allpoints,pad=1.08)
        assembly(ax,c0,cam,mode,centre_label=False)
        count=5 if mode=="one" else (len(c0["tets"]) if mode=="two" else 5+c0["k"])
        text(fig,x+.01,.532,f"{count} cells",9,MUTED)
        text(fig,x+.01,.501,f'{c0[key]:.1f}',21,BLUE,fontfamily="Palatino Linotype")
        text(fig,x+.115,.488,"amplification",7.5,MUTED)
    rule(fig,.45)
    text(fig,.045,.428,"SEPARATED VIEWS",8,INK,weight="bold")
    text(fig,.045,.402,"Cells translated along centroid rays; shapes and orientations are unchanged.",8,MUTED)
    cam2=cam
    all_exploded=np.vstack([exploded_points(c,cam2) for c in geo["cells"]])
    for i,cell in enumerate(geo["cells"]):
        x=.045+i*.315
        panel(fig,x,.359,"def"[i],f'Cell {cell["cell"]}')
        ax=axes(fig,[x-.005,.14,.30,.19])
        frame(ax,all_exploded,pad=1.05)
        exploded(ax,cell,cam2)
        text(fig,x+.01,.129,f'{cell["k"]} of {cell["candidates"]} second-ring cells retained',7.7)
    rule(fig,.093)
    legend(fig,.075,folded=True)
    text(fig,.045,.031,"All overlaps respect depth. Top: assembled. Bottom: display gaps are illustrative, not physical distances.",7.3,MUTED)
    return fig


def validate(geo,measure):
    k=centre_tet(geo["cells"][0])
    assert geo["n"]==2 and len(geo["cells"])==3
    assert [c["cell"] for c in geo["cells"]]==[0,1,7]
    for c in geo["cells"]:
        counts=Counter(t["ring"] for t in c["tets"])
        assert counts["K"]==1 and counts["ring1"]==4
        assert counts["chosen"]==c["k"]
        assert counts["chosen"]+counts["ring2"]==c["candidates"]
        for t in c["tets"]:
            v=np.asarray(t["v"],float)
            assert v.shape==(4,3) and np.isfinite(v).all()
            translated=v+1.15*v.mean(axis=0)
            for i,j in EDGES:
                assert abs(np.linalg.norm(v[j]-v[i])-np.linalg.norm(translated[j]-translated[i]))<1e-14
    errors=[]
    for i,j in EDGES:
        errors.append(abs(np.linalg.norm(k[j]-k[i])-measure["chordal_edge_lengths"][f"v{i}v{j}"]))
    assert max(errors)<2e-6, errors
    angle_errors=[]
    for edge in measure["edge_views"]:
        i,j=int(edge[1]),int(edge[3]); others=[a for a in range(4) if a not in (i,j)]
        e=norm(k[j]-k[i]); a,b=[norm(k[q]-k[i]-((k[q]-k[i])@e)*e) for q in others]
        angle=float(np.degrees(np.arccos(np.clip(a@b,-1,1))))
        angle_errors.append(abs(angle-measure["dihedral_angles_deg"][edge]))
    assert max(angle_errors)<.002, angle_errors
    return {"cell_ids":[c["cell"] for c in geo["cells"]],
            "max_edge_difference_from_receipt":max(errors),
            "max_dihedral_difference_deg":max(angle_errors),
            "geometry_precision":"Saved vertex coordinates: six decimals; measurement labels use original receipt.",
            "explosion":"Centroid translation only; all six edge lengths invariant to 1e-14.",
            "transparency":"Every pixel sorts faces and edges by actual depth before alpha composition; shared faces deduplicated.",
            "research_calculations_rerun":False}


def check_layout(fig):
    fig.canvas.draw()
    renderer=fig.canvas.get_renderer()
    alltext=list(fig.texts)+[t for ax in fig.axes for t in ax.texts]
    boxes=[]
    for t in alltext:
        b=t.get_window_extent(renderer)
        assert b.x0>=0 and b.y0>=0 and b.x1<=fig.bbox.width and b.y1<=fig.bbox.height, t.get_text()
        boxes.append((b,t.get_text()))
    for i,(a,s) in enumerate(boxes):
        for b,t in boxes[i+1:]:
            dx=min(a.x1,b.x1)-max(a.x0,b.x0)
            dy=min(a.y1,b.y1)-max(a.y0,b.y0)
            assert dx<1 or dy<1, f"Text overlap: {s!r} and {t!r}"
    for ax in fig.axes:
        xmin,xmax=ax.get_xlim(); ymin,ymax=ax.get_ylim()
        for line in ax.lines:
            x,y=np.array(line.get_xdata()),np.array(line.get_ydata())
            assert x.min()>=xmin and x.max()<=xmax and y.min()>=ymin and y.max()<=ymax, "Clipped drawing line"
    return {"text_elements_checked":len(boxes),"text_overlap":False,"out_of_bounds":False}


def main():
    geo=json.loads((DATA/"rings_3d.json").read_text())
    measure=json.loads((DATA/"cell_views.json").read_text())
    receipt=validate(geo,measure)
    receipt["inputs"]={name:hashlib.sha256((DATA/name).read_bytes()).hexdigest()
                       for name in ["rings_3d.json","cell_views.json"]}
    OUT.mkdir(exist_ok=True)
    figures=[plate_one(geo),plate_two(geo,measure),plate_three(geo)]
    receipt["layout"]=[check_layout(fig) for fig in figures]
    names=["fig_neighbourhood_polished","fig_cell_views_polished","fig_rings_polished"]
    with PdfPages(OUT/"three_plates_preview.pdf") as pdf:
        for fig,name in zip(figures,names):
            fig.savefig(OUT/(name+".pdf"),dpi=400)
            fig.savefig(OUT/(name+".svg"),dpi=400)
            fig.savefig(OUT/(name+".png"),dpi=190)
            pdf.savefig(fig,dpi=400)
            plt.close(fig)
    (OUT/"verification.json").write_text(json.dumps(receipt,indent=2)+"\n",encoding="utf-8")
    print(json.dumps(receipt,indent=2))
    print(OUT/"three_plates_preview.pdf")


if __name__=="__main__":
    main()
