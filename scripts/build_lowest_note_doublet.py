"""Draw the homogeneous Helike doublet from its actual matrices, not a surface analogy."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'docs/papers/ym-theory-statement/figures'
INK, TEAL, GOLD, MUTED, RULE = '#25343D', '#176F78', '#A16623', '#52636B', '#CED7DA'


def doublet():
    return np.diag([0., 1., -1.]), np.array([[0., 0., 0.], [0., 0., 1.], [0., 1., 0.]])


def rotation(theta):
    c, s = np.cos(theta), np.sin(theta)
    return np.array([[1., 0., 0.], [0., c, -s], [0., s, c]])


def figure_data():
    basis = doublet()
    states = []
    for degrees in (0, 45, 90, 135, 180):
        r = rotation(np.deg2rad(degrees))
        transformed = r @ basis[0] @ r.T
        coefficients = [float(np.sum(transformed*d)/np.sum(d*d)) for d in basis]
        states.append(dict(theta_degrees=degrees, coefficients=coefficients,
                           matrix=transformed.tolist()))
    return dict(convention='rows: spatial frame; columns: internal colour; b(v)=D.T @ v',
                action='R(theta) D R(theta).T; positive rotation e2 toward e3',
                scope='homogeneous real doublet at a point in a fixed orthonormal frame, not a spatial trajectory',
                normalization='matrices as written; each has Frobenius norm sqrt(2), not unit L2 normalization',
                matrices=[d.tolist() for d in basis],
                images=[[list(d.T @ e) for e in np.eye(3)] for d in basis],
                orbit=states)


def style():
    import matplotlib.pyplot as plt
    from matplotlib import font_manager
    for face in ('regular', 'italic', 'bold', 'bolditalic'):
        result = subprocess.run(['kpsewhich', f'texgyrepagella-{face}.otf'],
                                capture_output=True, text=True, check=True)
        path = Path(result.stdout.strip())
        if not path.is_file():
            raise FileNotFoundError('TeX Gyre Pagella is required to match the paper')
        font_manager.fontManager.addfont(str(path))
    plt.rcParams.update({'font.family':'TeX Gyre Pagella', 'font.size':9,
                         'mathtext.fontset':'custom', 'mathtext.rm':'TeX Gyre Pagella',
                         'mathtext.it':'TeX Gyre Pagella:italic', 'mathtext.bf':'TeX Gyre Pagella:bold',
                         'mathtext.fallback':'stix', 'text.color':INK,
                         'pdf.fonttype':3, 'svg.fonttype':'path',
                         'svg.hashsalt':'lowest-note-homogeneous-doublet-v1'})


def render(out):
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    from matplotlib.lines import Line2D
    from matplotlib.patches import Rectangle, Circle, FancyArrowPatch
    style()
    data = figure_data()
    fig = plt.figure(figsize=(6.8, 4.65), facecolor='white')
    titles = (r'a   $D_1=E_{22}-E_{33}$', r'b   $D_2=E_{23}+E_{32}$')
    for i, (matrix, images) in enumerate(zip(data['matrices'], data['images'])):
        left = .015 + .505*i
        fig.text(left, .963, titles[i], fontsize=11)
        ax = fig.add_axes([left, .515, .227, .385])
        ax.set(xlim=(-.95, 3.2), ylim=(-.2, 4.0), aspect='equal')
        ax.axis('off')
        ax.text(1.5, 3.85, 'colour index', ha='center', fontsize=8, color=MUTED)
        ax.text(-.89, 1.5, 'space index', va='center', rotation=90, fontsize=8, color=MUTED)
        for k in range(3):
            ax.text(k+.5, 3.3, str(k+1), ha='center', fontsize=8, color=MUTED)
            ax.text(-.25, 2.5-k, str(k+1), va='center', ha='center', fontsize=8, color=MUTED)
        for row in range(3):
            for col in range(3):
                value = matrix[row][col]
                colour = TEAL if row==1 else GOLD
                if value:
                    ax.add_patch(Rectangle((col,2-row),1,1,fc=colour,alpha=.10,lw=0))
                ax.text(col+.5,2.5-row, f'{int(value):d}', ha='center',va='center',
                        fontsize=12, color=colour if value else MUTED)
        for x in (0,3):
            ax.plot([x+.1 if x==0 else x-.1,x,x,x+.1 if x==0 else x-.1],
                    [0,0,3,3],color=INK,lw=.8)
        plane = fig.add_axes([left+.245, .555, .225, .315])
        plane.set(xlim=(-1.35,1.45),ylim=(-1.35,1.45),aspect='equal')
        plane.axis('off')
        plane.plot([-1.15,1.15],[0,0],color=RULE,lw=.7)
        plane.plot([0,0],[-1.15,1.15],color=RULE,lw=.7)
        plane.text(1.2,-.20,r'$T_2$',fontsize=10)
        plane.text(-.27,1.22,r'$T_3$',fontsize=10)
        for row, colour in ((1,TEAL),(2,GOLD)):
            v = np.asarray(images[row])[1:]
            plane.add_patch(FancyArrowPatch((0,0),v,arrowstyle='-|>',mutation_scale=12,
                                           color=colour,lw=1.8,shrinkA=0,shrinkB=0))
        plane.plot(0,0,'o',color=INK,ms=2.4)
        fig.text(left+.355,.883,'internal colour plane',ha='center',fontsize=8,color=MUTED)
        fig.text(left+.24,.515,r'input $e_2$',color=TEAL,fontsize=8.5)
        fig.text(left+.36,.515,r'input $e_3$',color=GOLD,fontsize=8.5)
    fig.text(.5,.467,r'Both maps send $e_1$ to zero.  Arrows are colour outputs, not spatial displacements.',
             ha='center',fontsize=8.5,color=MUTED)
    fig.add_artist(Line2D([.015,.985],[.438,.438],transform=fig.transFigure,color=RULE,lw=.7))
    fig.text(.015,.401,'c   Charge 2: a half-turn of the symmetry makes a full turn in the doublet',fontsize=9.5)
    labels = (r'$D_1$',r'$D_2$',r'$-D_1$',r'$-D_2$',r'$D_1$')
    for i,state in enumerate(data['orbit']):
        center = .09+.205*i
        ax = fig.add_axes([center-.062,.115,.124,.203])
        ax.set(xlim=(-1.32,1.32),ylim=(-1.32,1.32),aspect='equal')
        ax.axis('off')
        ax.add_patch(Circle((0,0),1,fc='none',ec=RULE,lw=.8))
        ax.plot([-1.1,1.1],[0,0],color=RULE,lw=.55)
        ax.plot([0,0],[-1.1,1.1],color=RULE,lw=.55)
        ax.add_patch(FancyArrowPatch((0,0),state['coefficients'],arrowstyle='-|>',
                                    color=TEAL,lw=1.6,mutation_scale=10,shrinkA=0,shrinkB=0))
        fig.text(center,.328,labels[i],ha='center',fontsize=10)
        fig.text(center,.087,rf'$\theta={state["theta_degrees"]}^\circ$',ha='center',fontsize=9)
    fig.text(.5,.025,r'Coefficient plane: horizontal $D_1$, vertical $D_2$.  The parameter is a symmetry angle, not time.',
             ha='center',fontsize=8.4,color=MUTED)
    out.mkdir(parents=True,exist_ok=True)
    for ext,metadata in (('pdf',{'Author':'','Creator':'','CreationDate':None,'ModDate':None}),
                         ('svg',{'Creator':'','Date':None}),('png',{})):
        fig.savefig(out/f'helike-doublet.{ext}',dpi=220,metadata=metadata)
    plt.close(fig)
    # Keep generated text byte-stable across Windows and Unix checkouts.
    svg = out/'helike-doublet.svg'
    svg.write_text('\n'.join(line.rstrip() for line in svg.read_text(encoding='utf-8').splitlines())+'\n',
                   encoding='utf-8',newline='\n')
    data['generator_sha256_lf'] = hashlib.sha256(Path(__file__).read_bytes().replace(b'\r\n',b'\n')).hexdigest()
    data['artifacts_sha256'] = {f'helike-doublet.{ext}':hashlib.sha256((out/f'helike-doublet.{ext}').read_bytes()).hexdigest()
                                 for ext in ('pdf','svg','png')}
    (out/'helike-doublet.json').write_text(json.dumps(data,indent=2)+'\n',encoding='utf-8',newline='\n')
    print(json.dumps(dict(output=str(out),artifacts=data['artifacts_sha256']),indent=2))


if __name__=='__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out',type=Path,default=OUT)
    render(parser.parse_args().out)
