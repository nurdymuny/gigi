"""Pictures from retained RF2D/F fields only. No new flow solve or repair."""
import json
from pathlib import Path
import sys
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.collections import LineCollection
from matplotlib.patches import Polygon
from scipy.spatial import cKDTree

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'e2e/probes'))
from ricci_lattice_evidence import read
from ricci_lattice_mesh import plus
from ricci_variational_mesh import deck,inverse,topology
from ricci_variational_math import EDGES,GRADS
from monodromy_evidence import sha

NATIVE=ROOT/'output/ricci-shape-regular-20261117/berger'
OLD=ROOT/'output/ricci-consistency-20261117/diagnostic'
RECOVERED=ROOT/'output/ricci-variational-20261117/abelian'
OUT=ROOT/'output/ricci-visual-review-20261117'
SLICE=.23


def slice_cells(payload,chart):
    topo=topology(payload);elements,tree,product,inverses,identity=deck()
    points=np.array([c['vertices'] for c in payload['cells']])
    centers=points.mean(axis=1);centers/=np.linalg.norm(centers,axis=1)[:,None]
    center=chart.mean(axis=0);center/=np.linalg.norm(center)
    distance,ids=cKDTree(centers).query(plus(inverse(elements),center))
    gi=int(np.argmin(distance));start=(int(ids[gi]),gi)
    pending=[start];seen=set();accepted=[];invchart=np.linalg.inv(chart.T)
    while pending:
        state=pending.pop()
        if state in seen:continue
        seen.add(state);ci,g=state
        p=plus(elements[g],points[ci]);hom=p@invchart.T;denom=hom.sum(axis=1)
        if np.any(denom<=0):continue
        bary=hom/denom[:,None]
        if np.any(bary.max(axis=0)<-1e-9):continue
        if min(bary[:,3])<=SLICE+1e-10 and max(bary[:,3])>=SLICE-1e-10:
            accepted.append((state,p,bary[:,1:]))
        for fi,own in topo['cell_faces'][ci]:
            image=product[g,own]
            for neighbor,_,other in topo['owners'][fi]:
                pending.append((neighbor,int(product[image,inverses[other]])))
        if len(seen)>20000:raise ValueError('visual patch unexpectedly unbounded')
    return sorted(accepted,key=lambda x:x[0]),len(seen)


def field(points,cell,cochain,positions):
    homogeneous=positions@np.linalg.inv(points.T).T
    lam=homogeneous/homogeneous.sum(axis=1)[:,None]
    b=(points[1:]-points[0]).T
    y=lam@points;r=np.linalg.norm(y,axis=1);unit=y/r[:,None]
    dx=(b[None]-unit[:,:,None]*np.einsum('qi,ia->qa',unit,b)[:,None,:])/r[:,None,None]
    one=np.array([lam[:,i,None]*GRADS[j]-lam[:,j,None]*GRADS[i] for i,j in EDGES]).transpose(1,0,2)
    local=cochain[np.array(cell['edges'])]*np.where(cell['reverse'],-1.,1.)
    covector=np.einsum('e,qea->qa',local,one)
    g=np.einsum('qia,qib->qab',dx,dx)
    tangent=np.einsum('qia,qa->qi',dx,np.linalg.solve(g,covector[...,None])[...,0])
    # Components in the natural local compass, not the displayed slice axes.
    e1=np.column_stack([-unit[:,1],unit[:,0],unit[:,3],-unit[:,2]])
    e2=np.column_stack([-unit[:,2],-unit[:,3],unit[:,0],unit[:,1]])
    e3=np.column_stack([-unit[:,3],unit[:,2],-unit[:,1],unit[:,0]])
    return np.column_stack([-np.sum(tangent*e1,axis=1),np.sum(tangent*e2,axis=1),np.sum(tangent*e3,axis=1)])/4


def section(payload,cells,chart,cochains,xy):
    raw=np.column_stack([1-xy.sum(axis=1)-SLICE,xy,np.full(len(xy),SLICE)])@chart
    unit=raw/np.linalg.norm(raw,axis=1)[:,None]
    owners=np.full(len(xy),-1,dtype=int);result=np.full((len(cochains),len(xy),3),np.nan)
    boundaries=[]
    for k,((ci,g),p,xyz) in enumerate(cells):
        h=unit@np.linalg.inv(p.T).T;l=h/h.sum(axis=1)[:,None]
        inside=(owners<0)&(l.min(axis=1)>=-2e-10)&(h.sum(axis=1)>0)
        if inside.any():
            owners[inside]=k
            for j,c in enumerate(cochains):result[j,inside]=field(p,payload['cells'][ci],c,unit[inside])
        cuts=[]
        for a,b in EDGES:
            za,zb=xyz[a,2],xyz[b,2]
            if (za-SLICE)*(zb-SLICE)<0:
                t=(SLICE-za)/(zb-za);cuts.append((xyz[a]+t*(xyz[b]-xyz[a]))[:2])
        if len(cuts)>=3:
            cuts=np.array(cuts);center=cuts.mean(axis=0)
            cuts=cuts[np.argsort(np.arctan2(cuts[:,1]-center[1],cuts[:,0]-center[0]))]
            boundaries.extend([[cuts[i],cuts[(i+1)%len(cuts)]] for i in range(len(cuts))])
    if np.any(owners<0):raise ValueError('visual slice has uncovered points')
    return result,owners,boundaries


def run():
    OUT.mkdir(parents=True,exist_ok=True)
    if (OUT/'receipt.json').exists():raise ValueError('preserve previous visual review')
    coarse=read(NATIVE/'level0_berger_q6.input.json');chart=np.array(coarse['cells'][0]['vertices'])
    xx=np.linspace(.003,.767,161);x,y=np.meshgrid(xx,xx);mask=x+y<1-SLICE-.003
    xy=np.column_stack([x[mask],y[mask]])
    line=np.column_stack([np.linspace(.008,.532,1200),np.full(1200,.23)])
    fig,axes=plt.subplots(2,2,figsize=(11,10),layout='constrained')
    profile,pax=plt.subplots(2,1,figsize=(12,7),sharex=True,layout='constrained')
    receipt={'scope':'visual inspection only; no operator changes, new solves, or gate claim',
        'selection':'first original flag tetrahedron; fixed interior slice; no error-based selection',
        'sources':{},'script_sha256':sha(Path(__file__)),'levels':{}}
    plt.rcParams.update({'font.family':'DejaVu Sans','font.size':12})
    for row,level in enumerate([1,3]):
        key=f'level{level}_q6';source=NATIVE/f'level{level}_berger_q6.input.json'
        native_meta=read(NATIVE/'diagnostic.json')['cases'][f'level{level}_berger_q6']
        if sha(source)!=native_meta['input_sha256']:raise ValueError('geometry source changed')
        oldpath=OLD/(key+'.arrays.npz');newpath=RECOVERED/(key+'.arrays.npz')
        if sha(oldpath)!=read(OLD/'diagnostic.json')['cases'][key]['arrays_sha256']:raise ValueError('original data changed')
        if sha(newpath)!=read(RECOVERED/'diagnostic.json')['cases'][key]['artifacts'][newpath.name]:raise ValueError('recovery data changed')
        for path in [source,oldpath,newpath]:receipt['sources'][str(path)]=sha(path)
        payload=read(source)
        with np.load(oldpath,allow_pickle=False) as data:old=data['abelian_velocity'].copy()
        with np.load(newpath,allow_pickle=False) as data:new=data['velocity'][:,0].copy()
        cells,visited=slice_cells(payload,chart)
        values,owners,boundaries=section(payload,cells,chart,[old,new],xy)
        along,lineowners,_=section(payload,cells,chart,[old,new],line)
        errors=np.linalg.norm(values-np.array([1.,0.,0.]),axis=2)
        for col,title in enumerate(['Original lattice','Fitted curvature recovery']):
            ax=axes[row,col];canvas=np.full(x.shape,np.nan);canvas[mask]=errors[col]
            im=ax.pcolormesh(x,y,canvas,cmap='magma',vmin=0,vmax=1.5,shading='nearest',rasterized=True)
            triangle=Polygon([[0,0],[1-SLICE,0],[0,1-SLICE]],fill=False,edgecolor='#7b7b7b',linewidth=.7)
            ax.add_patch(triangle)
            collection=LineCollection(boundaries,colors='#acacac',linewidths=.45,alpha=.65)
            collection.set_clip_path(triangle);ax.add_collection(collection)
            start=np.array([line[0],line[-1]])
            ax.plot(start[:,0],start[:,1],color='#54cfac',linewidth=1.5)
            ax.set_aspect('equal');ax.set_xlim(-.015,.79);ax.set_ylim(-.015,.79);ax.axis('off')
            ax.set_title(title+' | '+('coarser' if row==0 else 'finer'),fontsize=15,pad=8)
        ax=pax[row]
        ax.axhline(1,color='#128260',linewidth=2,label='Desired motion')
        ax.plot(line[:,0],along[0,:,0],color='#1479b8',linewidth=1.2,label='Original lattice')
        ax.plot(line[:,0],along[1,:,0],color='#c34251',linewidth=1.1,label='Fitted recovery')
        for position in line[np.r_[False,np.diff(lineowners)!=0],0]:
            ax.axvline(position,color='#999999',linewidth=.55,alpha=.45)
        ax.set_title('Same crossing, '+('coarser mesh' if row==0 else 'finer mesh'),loc='left',fontsize=14)
        ax.set_ylim(-1.8,3.8);ax.set_yticks([-1,0,1,2,3],['backward','stopped','desired','twice desired','three times'])
        ax.spines[['top','right']].set_visible(False);ax.tick_params(axis='x',labelbottom=False,bottom=False)
        receipt['levels'][str(level)]={'slice_cells':len(cells),'visited_cover_cells':visited,
            'coverage':'complete','sample_count':len(xy),'line_samples':len(line),
            'original_line_range':[float(along[0,:,0].min()),float(along[0,:,0].max())],
            'recovery_line_range':[float(along[1,:,0].min()),float(along[1,:,0].max())]}
        np.savez_compressed(OUT/(key+'.visual.npz'),xy=xy,values=values,owners=owners,
                            line=line,line_values=along,line_owners=lineowners)
    fig.suptitle('Where the motion disagrees: the same physical patch',fontsize=19)
    cb=fig.colorbar(im,ax=axes,shrink=.65,pad=.02);cb.set_ticks([0,.75,1.5],labels=['small','moderate','large'])
    cb.set_label('Local motion mismatch (all directions)',fontsize=12)
    fig.supxlabel('Thin lines: cell boundaries. Green line: the crossing inspected below.\nImages are local slices, not the whole-manifold acceptance score.',fontsize=11)
    fig.savefig(OUT/'spatial-mismatch.png',dpi=160)
    profile.suptitle('A smooth instruction becomes a jagged response',fontsize=19)
    pax[0].legend(loc='upper right',fontsize=11,ncol=3)
    profile.supxlabel('Position across the same patch; vertical markers are crossed cell boundaries.\nOnly the intended-direction component is plotted here.',fontsize=11)
    profile.savefig(OUT/'boundary-crossing.png',dpi=160)
    receipt['images']={name:sha(OUT/name) for name in ['spatial-mismatch.png','boundary-crossing.png']}
    (OUT/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(receipt['levels'],indent=2))


if __name__=='__main__':run()
