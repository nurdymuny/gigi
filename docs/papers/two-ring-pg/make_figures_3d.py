"""Higher-fidelity and 3-D figures, from the real meshes and the committed receipts.

  fig_neighbourhood_3d.pdf  an actual n = 2 cell K with its one-ring and two-ring cells, in K's chart
  fig_chip_3d.pdf           how much the recovered curvature in K moves when one neighbouring edge
                            value is perturbed ("a chip"), one ring vs two rings (+ statistics)
  fig_amplification_3d.pdf  every n = 2 cell carried onto one sheet, coloured by fit amplification
  fig_su2_convergence.pdf   SU(2): covariant recovery and the force, from the receipts
  fig_su2_spectra.pdf       SU(2) linearised force at n = 2: native, two-ring, one-ring

Colour = method identity, fixed across the paper (validated categorical order); every series also
carries a marker and a direct label. Sequential magnitude uses one hue (blue ramp), light to dark.
Needs the checkpoint archives extracted under output/ (see README).
usage: python docs/papers/two-ring-pg/make_figures_3d.py [names...]
"""
import json, sys
from pathlib import Path
import numpy as np
import matplotlib
matplotlib.use('pdf')
import matplotlib.pyplot as plt
from matplotlib.colors import LogNorm, LinearSegmentedColormap
from mpl_toolkits.mplot3d.art3d import Poly3DCollection, Line3DCollection

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
sys.path[:0] = [str(REPO/'scripts'), str(REPO/'e2e/probes')]
import ricci_pg_curvature as pg          # noqa: E402
import ricci_pg_tworing as tr            # noqa: E402

FIG, DATA = HERE/'figures', HERE/'data'
INK, INK2, GRID, SURF = '#0b0b0b', '#52514e', '#e4e3df', '#fcfcfb'
BLUE, ORANGE, AQUA, YELLOW, MAGENTA = '#2a78d6', '#eb6834', '#1baf7a', '#eda100', '#e87ba4'
RAMP = LinearSegmentedColormap.from_list('blue', ['#86b6ef', '#3987e5', '#256abf', '#184f95', '#0d366b'])
plt.rcParams.update({
    'font.family': 'serif', 'font.size': 8.5, 'axes.labelsize': 8.5, 'axes.titlesize': 9,
    'axes.edgecolor': INK2, 'axes.labelcolor': INK, 'xtick.color': INK2, 'ytick.color': INK2,
    'axes.linewidth': 0.6, 'axes.grid': True, 'grid.color': GRID, 'grid.linewidth': 0.5,
    'axes.axisbelow': True, 'legend.frameon': False, 'legend.fontsize': 7.3,
    'axes.spines.top': False, 'axes.spines.right': False, 'savefig.bbox': 'tight', 'pdf.fonttype': 42})
W2 = 6.9
LEVEL = 1                                   # n = 2
FACES4 = [(0, 1, 2), (0, 1, 3), (0, 2, 3), (1, 2, 3)]


def mesh(level=LEVEL):
    L = pg.load_cells(level)
    return L['cells'], pg.face_index(L['cells']), L['a']


def tetra_polys(V):
    return [V[list(f)] for f in FACES4]


def style3d(ax, elev=18, azim=-58, zoom=1.45):
    ax.view_init(elev=elev, azim=azim)
    ax.set_axis_off()
    ax.set_box_aspect((1, 1, 1), zoom=zoom)


def equalise(ax, P):
    c = P.mean(axis=0); r = np.max(np.abs(P-c))*1.02
    ax.set_xlim(c[0]-r, c[0]+r); ax.set_ylim(c[1]-r, c[1]+r); ax.set_zlim(c[2]-r, c[2]+r)


# --------------------------------------------------------------------------- neighbourhood
def neighbourhood(ci=0):
    cells, faces, _ = mesh()
    nb = tr.neighbourhood_rings(cells, faces, ci, rings=2)
    to_orth, _, _ = pg.chart(nb[ci])
    ring1 = set(tr.neighbourhood_rings(cells, faces, ci, rings=1)) - {ci}
    ring2 = set(nb) - ring1 - {ci}
    f1 = tr.fit_cell_rings(cells, np.zeros(len(pg.load_cells(LEVEL)['a'])), faces, ci, rings=1)
    f2 = tr.fit_cell_rings(cells, np.zeros(len(pg.load_cells(LEVEL)['a'])), faces, ci, rings=2)
    O = {k: to_orth(v) for k, v in nb.items()}
    fig = plt.figure(figsize=(W2, 2.9))
    fig.subplots_adjust(left=0, right=1, top=0.95, bottom=0.08, wspace=0.0)
    for p, (title, show2) in enumerate([(f'one ring: {1+len(ring1)} cells, {f1["faces"]} faces', False),
                                       (f'two rings: {len(nb)} cells, {f2["faces"]} faces', True)]):
        ax = fig.add_subplot(1, 2, p+1, projection='3d')
        groups = [([ci], INK, 0.85, 0.7), (sorted(ring1), ORANGE, 0.42, 0.5)]
        if show2:
            groups.append((sorted(ring2), AQUA, 0.10, 0.35))
        for ids, col, alpha, lw in groups[::-1]:
            polys = [poly for k in ids for poly in tetra_polys(O[k])]
            ax.add_collection3d(Poly3DCollection(polys, facecolor=col, alpha=alpha, edgecolor=INK2 if col != INK else SURF,
                                                 linewidths=lw*0.6))
        P = np.vstack([O[k] for k in (nb if show2 else [ci, *ring1])])
        equalise(ax, np.vstack([O[k] for k in nb])); style3d(ax)
        ax.set_title(title, color=INK, fontsize=8.5, pad=-4)
    handles = [plt.Rectangle((0, 0), 1, 1, fc=c, alpha=a) for c, a in [(INK, .85), (ORANGE, .5), (AQUA, .3)]]
    fig.legend(handles, ['cell $K$', 'first ring (face neighbours)', 'second ring'], loc='lower center', ncol=3,
               bbox_to_anchor=(0.5, -0.02))
    fig.savefig(FIG/'fig_neighbourhood_3d.pdf'); plt.close(fig)
    return {'cell': ci, 'ring1_cells': len(ring1), 'two_ring_cells': len(nb), 'faces_one_ring': f1['faces'],
            'faces_two_ring': f2['faces']}


# --------------------------------------------------------------------------- four direct views of the same cells
def _proj_basis(d):
    d = d/np.linalg.norm(d)
    a = np.array([0, 0, 1.0]) if abs(d[2]) < 0.9 else np.array([1.0, 0, 0])
    u = np.cross(a, d); u /= np.linalg.norm(u)
    return u, np.cross(d, u), d


def _angle(p, q, r):
    """angle at q between q->p and q->r, in degrees"""
    a, b = p - q, r - q
    return float(np.degrees(np.arccos(np.clip(a@b/np.linalg.norm(a)/np.linalg.norm(b), -1, 1))))


def cell_views(ci=0):
    """Orthographic views of cell K and its two rings: three looking straight down an edge of K
    (the two faces at that edge then appear as lines, so their angle is the true dihedral angle)
    and one looking straight down a face normal (the face appears in true shape)."""
    from matplotlib.collections import PolyCollection
    cells, faces, _ = mesh()
    nb = tr.neighbourhood_rings(cells, faces, ci, rings=2)
    ring1 = set(tr.neighbourhood_rings(cells, faces, ci, rings=1)) - {ci}
    ring2 = set(nb) - ring1 - {ci}
    to_orth, _, _ = pg.chart(nb[ci])
    O = {k: to_orth(v) for k, v in nb.items()}
    V4 = np.asarray(nb[ci], float)                    # K's vertices in R^4 (unit S^3)
    K = O[ci]
    scale = np.linalg.norm(V4[0]-V4[1])/np.linalg.norm(K[0]-K[1])   # chart unit -> chordal length
    # K sits in its own chart exactly (a similarity): check it
    L4 = {(i, j): float(np.linalg.norm(V4[i]-V4[j])) for i in range(4) for j in range(i+1, 4)}
    Lc = {e: float(np.linalg.norm(K[e[0]]-K[e[1]]))*scale for e in L4}
    assert max(abs(L4[e]-Lc[e]) for e in L4) < 1e-12
    def dihedral(i, j):
        k, l = [m for m in range(4) if m not in (i, j)]
        e = K[j]-K[i]; e /= np.linalg.norm(e)
        pk = K[k]-K[i]; pk -= pk@e*e
        pl = K[l]-K[i]; pl -= pl@e*e
        return float(np.degrees(np.arccos(pk@pl/np.linalg.norm(pk)/np.linalg.norm(pl))))
    dih = {e: dihedral(*e) for e in L4}
    order = sorted(dih, key=dih.get)
    edge_views = [order[0], order[len(order)//2], order[-1]]          # smallest, middle, largest dihedral
    # face view: the face whose hidden opposite vertex projects farthest from the face's corners
    def face_normal(fv):
        opp = [m for m in range(4) if m not in fv][0]
        fn = np.cross(K[fv[1]]-K[fv[0]], K[fv[2]]-K[fv[0]])
        return (-fn if fn@(K[opp]-K[fv[0]]) > 0 else fn)/np.linalg.norm(fn), opp
    def clearance(fv):
        fn, opp = face_normal(fv)
        q = K[opp] - (K[opp]-K[fv[0]])@fn*fn
        return min(np.linalg.norm(q-K[m]) for m in fv)
    fv = max(FACES4, key=clearance)
    fn, opp = face_normal(fv)
    face_angles = {fv[m]: _angle(K[fv[m-1]], K[fv[m]], K[fv[(m+1) % 3]]) for m in range(3)}

    fig, axs = plt.subplots(2, 2, figsize=(W2, 6.6))
    fig.subplots_adjust(left=0.01, right=0.99, top=0.96, bottom=0.07, wspace=0.03, hspace=0.12)
    views = [('edge', e) for e in edge_views] + [('face', fv)]
    # one scale for all four panels: the largest projected half-extent of K over the views
    def basis_for(kind, item):
        d = (K[item[1]]-K[item[0]]) if kind == 'edge' else -fn
        return _proj_basis(d)
    REXT = max(np.abs(np.c_[K@b[0], K@b[1]] - np.c_[K@b[0], K@b[1]].mean(axis=0)).max()
               for b in (basis_for(*v) for v in views))
    for ax, (kind, item) in zip(axs.ravel(), views):
        u, w, d = basis_for(kind, item)
        pr = lambda X: np.c_[X@u, X@w]
        polys, cols, alph, depth, ecs, lws = [], [], [], [], [], []
        for ids, col, a_, lw in [(sorted(ring2), AQUA, 0.0, 0.35), (sorted(ring1), ORANGE, 0.10, 0.4)]:
            for k in ids:
                for f in FACES4:
                    P = O[k][list(f)]
                    polys.append(pr(P)); cols.append(col); alph.append(a_); depth.append(float((P@d).mean()))
                    ecs.append('#0f8a5f' if col == AQUA else '#c9874f'); lws.append(lw)
        idx = np.argsort(depth)[::-1]                                  # far first
        for i in idx:
            from matplotlib.colors import to_rgba
            ax.add_collection(PolyCollection([polys[i]], facecolors=[to_rgba(cols[i], alph[i])],
                                             edgecolors=[to_rgba(ecs[i], 0.3 if cols[i] == AQUA else 0.55)], linewidths=lws[i]))
        # K on top: faces filled lightly, every edge drawn, hidden edges dashed
        P2 = pr(K)
        for f in FACES4:
            ax.add_collection(PolyCollection([P2[list(f)]], facecolors=[INK], alpha=0.10, edgecolors='none'))
        for (i, j) in L4:
            hidden = kind == 'face' and opp in (i, j)          # edges to the vertex behind the face
            ax.plot(*P2[[i, j]].T, color=INK, lw=1.0 if hidden else 1.5, ls=(0, (3, 2)) if hidden else '-',
                    solid_capstyle='round', zorder=5)
        ext = REXT
        done = set()
        for m in range(4):
            if m in done:
                continue
            same = [q for q in range(4) if np.linalg.norm(P2[q]-P2[m]) < 1e-9*ext + 1e-12]
            done.update(same)
            off = P2[m] - P2.mean(axis=0); off = off/(np.linalg.norm(off)+1e-12)*0.16*ext
            if len([q for q in range(4) if np.linalg.norm(P2[q]-P2[m]) < 1e-9*ext + 1e-12]) > 1:
                off = off*1.7
            ax.scatter(*P2[m], s=16, color=INK, zorder=6)
            name = '$' + '{=}'.join(f'v_{q}' for q in same) + '$'
            if len(same) > 1:
                name += ' (end-on)'
            ax.text(*(P2[m]+off), name, fontsize=8, ha='center', va='center', zorder=7,
                    bbox=dict(boxstyle='round,pad=0.12', fc='white', ec='none', alpha=0.85))
        if kind == 'edge':
            i, j = item
            k, l = [m for m in range(4) if m not in (i, j)]
            c0 = P2[i]
            a1 = np.degrees(np.arctan2(*(P2[k]-c0)[::-1])); a2 = np.degrees(np.arctan2(*(P2[l]-c0)[::-1]))
            lo, hi = sorted([a1, a2])
            if hi - lo > 180:
                lo, hi = hi, lo + 360
            r = 0.4*min(np.linalg.norm(P2[k]-c0), np.linalg.norm(P2[l]-c0))
            tt = np.radians(np.linspace(lo, hi, 60))
            ax.plot(c0[0]+r*np.cos(tt), c0[1]+r*np.sin(tt), color=BLUE, lw=1.6, zorder=7)
            mid = np.radians((lo+hi)/2)
            ax.text(c0[0]+2.05*r*np.cos(mid), c0[1]+2.05*r*np.sin(mid), f'{dih[item]:.1f}°', color=INK, fontsize=8.5,
                    ha='center', va='center', zorder=8, fontweight='bold',
                    bbox=dict(boxstyle='round,pad=0.15', fc='white', ec=BLUE, lw=0.6))
            title = (f'({"abc"[edge_views.index(item)]}) down edge $v_{i}v_{j}$ (length {L4[item]:.3f}): '
                     f'dihedral angle {dih[item]:.1f}°')
        else:
            for m in range(3):
                a_, b_ = fv[m], fv[(m+1) % 3]
                mid = (P2[a_]+P2[b_])/2; out = mid - P2[list(fv)].mean(axis=0); out /= np.linalg.norm(out)
                ax.text(*(mid+out*0.09*ext), f'{L4[tuple(sorted((a_, b_)))]:.3f}', fontsize=7.4, color=INK,
                        ha='center', va='center', zorder=8,
                        bbox=dict(boxstyle='round,pad=0.12', fc='white', ec='none', alpha=0.9))
                v = fv[m]; c0 = P2[v]
                inward = P2[list(fv)].mean(axis=0) - c0; inward /= np.linalg.norm(inward)
                ax.text(*(c0+inward*0.24*ext), f'{face_angles[v]:.1f}°', fontsize=7.4, color=BLUE, ha='center',
                        va='center', zorder=8, fontweight='bold',
                        bbox=dict(boxstyle='round,pad=0.1', fc='white', ec='none', alpha=0.85))
            title = f'(d) face-on to $v_{fv[0]}v_{fv[1]}v_{fv[2]}$: true shape ($v_{opp}$ behind, dashed)'
        ax.set_title(title, fontsize=8, loc='left', color=INK)
        ax.set_aspect('equal'); ax.axis('off')
        c = P2.mean(axis=0); R = REXT*1.3                              # K large; the rings give context
        ax.set_xlim(c[0]-R, c[0]+R); ax.set_ylim(c[1]-R, c[1]+R)
        # scale bar: 0.1 chordal units
        bar = 0.1/scale
        x0, y0 = c[0]-R*0.95, c[1]-R*0.95
        ax.plot([x0, x0+bar], [y0, y0], color=INK, lw=1.4, zorder=9)
        ax.text(x0+bar/2, y0+R*0.03, '0.1 (chordal)', fontsize=7, ha='center', va='bottom', color=INK)
    handles = [plt.Rectangle((0, 0), 1, 1, fc='#c7a99a', ec=INK), plt.Rectangle((0, 0), 1, 1, fc=ORANGE, alpha=0.3),
               plt.Rectangle((0, 0), 1, 1, fc='none', ec='#0f8a5f')]
    fig.legend(handles, ['cell $K$', 'first ring', 'second ring (outlines)'], loc='lower center', ncol=3,
               bbox_to_anchor=(0.5, 0.0))
    fig.savefig(FIG/'fig_cell_views.pdf'); plt.close(fig)
    meas = {'cell': ci, 'chordal_edge_lengths': {f'v{i}v{j}': round(L4[(i, j)], 6) for (i, j) in L4},
            'dihedral_angles_deg': {f'v{i}v{j}': round(dih[(i, j)], 3) for (i, j) in dih},
            'face_view': 'v%dv%dv%d' % fv, 'face_view_angles_deg': {f'v{m}': round(face_angles[m], 3) for m in face_angles},
            'edge_views': [f'v{i}v{j}' for (i, j) in edge_views],
            'volume_chordal': float(abs(np.linalg.det((K[1:]-K[0]).T))/6*scale**3),
            'chart_scale_check_max_abs': float(max(abs(L4[e]-Lc[e]) for e in L4))}
    (DATA/'cell_views.json').write_text(json.dumps(meas, indent=1) + '\n')
    return meas


# --------------------------------------------------------------------------- chip test
def field(fit, X):
    return fit['c'][None, :] + X@fit['B'].T                              # chart-frame proxy at chart points X


def interior_points(V, n=4):
    pts = []
    for i in range(n+1):
        for j in range(n+1-i):
            for k in range(n+1-i-j):
                l = n-i-j-k
                if min(i, j, k, l) >= 1 or n < 4:
                    pts.append((i*V[0]+j*V[1]+k*V[2]+l*V[3])/n)
    return np.array(pts)


def chip_response(cells, faces, a, ci, edge, delta):
    out = {}
    for r in (1, 2):
        f0 = tr.fit_cell_rings(cells, a, faces, ci, rings=r)
        a1 = a.copy(); a1[edge] += delta
        f1 = tr.fit_cell_rings(cells, a1, faces, ci, rings=r)
        out[r] = (f0, f1)
    return out


def chip(ci=0, n_stat=300, seed=0):
    cells, faces, a = mesh()
    nb1 = tr.neighbourhood_rings(cells, faces, ci, rings=1)
    to_orth, _, _ = pg.chart(nb1[ci])
    own = set(cells[ci]['edges'])
    nbr = next(k for k in nb1 if k != ci)
    edge = next(e for e in cells[nbr]['edges'] if e not in own)
    delta = 0.05*float(np.median(abs(a)))
    resp = chip_response(cells, faces, a, ci, edge, delta)
    VK = to_orth(nb1[ci]); X = interior_points(VK, 8)
    dP = {r: field(resp[r][1], X)-field(resp[r][0], X) for r in (1, 2)}
    P0 = field(resp[2][0], X)
    scale = max(np.abs(dP[1]).max(), 1e-30)
    # statistics over random cells and chips
    rng = np.random.default_rng(seed); ratios = []
    for cj in rng.choice(len(cells), n_stat, replace=False):
        nbj = tr.neighbourhood_rings(cells, faces, int(cj), rings=1)
        ownj = set(cells[cj]['edges'])
        candidates = [e for k in nbj if k != cj for e in cells[k]['edges'] if e not in ownj]
        e = int(rng.choice(candidates))
        rr = chip_response(cells, faces, a, int(cj), e, delta)
        to_j, _, _ = pg.chart(nbj[cj]); Xj = interior_points(to_j(nbj[cj]), 4)
        d1 = np.linalg.norm(field(rr[1][1], Xj)-field(rr[1][0], Xj), axis=1).mean()
        d2 = np.linalg.norm(field(rr[2][1], Xj)-field(rr[2][0], Xj), axis=1).mean()
        ratios.append(d1/max(d2, 1e-300))
    ratios = np.array(ratios)
    fig = plt.figure(figsize=(W2, 2.6))
    nb2 = tr.neighbourhood_rings(cells, faces, ci, rings=1)
    for p, (r, col, name) in enumerate([(1, ORANGE, 'one ring'), (2, AQUA, 'two rings')]):
        ax = fig.add_subplot(1, 2, p+1, projection='3d')
        ax.add_collection3d(Poly3DCollection(tetra_polys(VK), facecolor=INK, alpha=0.06, edgecolor=INK2, linewidths=0.6))
        Vn = to_orth(nb2[nbr])
        ax.add_collection3d(Poly3DCollection(tetra_polys(Vn), facecolor=ORANGE, alpha=0.08, edgecolor=GRID, linewidths=0.4))
        ends = [Vn[i] for i, e in enumerate(cells[nbr]['edges']) for i in [0]][:0]
        li = cells[nbr]['edges'].index(edge); u, v = pg.EDGES[li]
        ax.add_collection3d(Line3DCollection([[Vn[u], Vn[v]]], colors=INK, linewidths=2.6))
        mid = (Vn[u]+Vn[v])/2
        ax.text(mid[0], mid[1], mid[2], '  chip', color=INK, fontsize=7.5)
        L = 0.42*np.ptp(VK, axis=0).max()/scale
        ax.quiver(X[:, 0], X[:, 1], X[:, 2], dP[r][:, 0]*L, dP[r][:, 1]*L, dP[r][:, 2]*L, color=col, linewidth=1.0,
                  arrow_length_ratio=0.25, normalize=False)
        rel = np.linalg.norm(dP[r], axis=1).mean()/np.linalg.norm(P0, axis=1).mean()
        equalise(ax, np.vstack([VK, Vn])); style3d(ax, 22, -50, zoom=1.7)
        ax.set_title(f'{name}: curvature in $K$ moves {100*rel:.1f}%', color=INK, fontsize=8.5, pad=-2)
    fig.subplots_adjust(left=0, right=1, top=0.93, bottom=0.0, wspace=0.0)
    fig.savefig(FIG/'fig_chip_3d.pdf'); plt.close(fig)
    return {'cell': ci, 'edge': int(edge), 'delta': delta, 'n_stat': n_stat,
            'ratio_median': float(np.median(ratios)), 'ratio_q25': float(np.quantile(ratios, .25)),
            'ratio_q75': float(np.quantile(ratios, .75)), 'ratio_min': float(ratios.min())}


# --------------------------------------------------------------------------- amplification map
def amplification():
    cells, faces, _ = mesh()
    C = json.loads((DATA/'conditioning.json').read_text())
    a1 = np.array(C[f'L{LEVEL}_r1']['amplification']); a2 = np.array(C[f'L{LEVEL}_r2']['amplification'])
    allc = tr.neighbourhood_rings(cells, faces, 0, rings=60)
    if len(allc) != len(cells):
        raise ValueError('not every cell reached on one sheet')
    to_orth, _, _ = pg.chart(allc[0])
    cent = np.array([to_orth(allc[k]).mean(axis=0) for k in range(len(cells))])
    norm = LogNorm(vmin=min(a1.min(), a2.min()), vmax=max(a1.max(), a2.max()))
    fig = plt.figure(figsize=(W2, 3.1))
    for p, (amp, name) in enumerate([(a1, 'one ring'), (a2, 'two rings')]):
        ax = fig.add_subplot(1, 2, p+1, projection='3d')
        sc = ax.scatter(cent[:, 0], cent[:, 1], cent[:, 2], c=amp, cmap=RAMP, norm=norm, s=9, depthshade=False,
                        edgecolors='none')
        equalise(ax, cent); style3d(ax, 16, -40, zoom=1.35)
        ax.set_title(f'{name}: median {np.median(amp):.0f}, max {amp.max():.0f}', color=INK, fontsize=8.5, pad=-2)
    cb = fig.colorbar(sc, ax=fig.axes, shrink=0.65, pad=0.02, location='right')
    cb.set_ticks([30, 50, 100, 200]); cb.set_ticklabels(['30', '50', '100', '200'])
    fig.text(0.42, 0.04, 'each dot is one cell of the $n=2$ mesh, all 960 carried onto one sheet of the cover', ha='center',
             fontsize=7.2, color=INK2)
    cb.set_label('fit amplification $\\|\\mathcal{R}_K\\|_2$ (log scale)', color=INK)
    cb.outline.set_linewidth(0.4)
    fig.savefig(FIG/'fig_amplification_3d.pdf'); plt.close(fig)
    return {'cells': len(cells), 'one_ring_median': float(np.median(a1)), 'two_ring_median': float(np.median(a2))}


# --------------------------------------------------------------------------- SU(2)
def su2_convergence():
    geo = json.loads((REPO/'output/ricci-shape-regular-20261117/berger/diagnostic.json').read_text())
    H = np.array([geo['geometry'][str(l)]['h_max'] for l in range(4)])
    native = np.array([geo['cases'][f'level{l}_berger_q6']['measures']['relative_error'] for l in range(4)])*100
    rows = lambda f: np.array([r['relative_error'] for r in json.loads((REPO/f'research/su2-pg-force-20261117/{f}').read_text())['rows']])*100
    rec = {m: json.loads((REPO/f'research/recover-curvature-su2-20261117/g2_{m}.json').read_text())['variants'] for m in ('berger', 'round')}
    rv = lambda m, v: np.array([r['relative_error'] for r in rec[m][v]['rows']])*100
    fig, axes = plt.subplots(1, 2, figsize=(W2, 2.75))
    ax = axes[0]
    for m, v, c, mk, ls, lab in [('berger', 'full', AQUA, 'o', '-', 'Berger, covariant'),
                                 ('round', 'full', AQUA, 's', (0, (3, 1.5)), 'round, covariant'),
                                 ('berger', 'M1_no_register', YELLOW, 'v', '-', 'Berger, no register'),
                                 ('berger', 'M2_abelianised', MAGENTA, 'D', '-', 'Berger, abelianised')]:
        ax.plot(H, rv(m, v), color=c, marker=mk, ls=ls, lw=1.3, ms=4.2, mec=SURF, mew=0.7, label=lab)
    ax.set_title('(a) recovered curvature', color=INK)
    ax.set_ylabel('relative error (%)')
    ax = axes[1]
    for y, c, mk, lab, lw in [(native, BLUE, 's', 'native $SU(2)$ Whitney', 1.1), (rows('pg_q6.json'), AQUA, 'o', 'two-ring PG', 1.7),
                              (rows('pg_onering_q6.json'), ORANGE, '^', 'one-ring PG', 1.1),
                              (rows('pg_noregister_q6.json'), YELLOW, 'v', 'two-ring, no register', 1.1)]:
        ax.plot(H, y, color=c, marker=mk, lw=lw, ms=4.2, mec=SURF, mew=0.7, label=lab)
    ax.axhline(5, color=INK2, lw=0.7, ls=(0, (1, 1.5)))
    ax.text(H[-1]*0.985, 4.2, 'BW05 5%', color=INK2, fontsize=6.8, ha='left', va='top')
    ax.set_title('(b) force on the Berger field', color=INK)
    for ax in axes:
        ax.set_xscale('log'); ax.set_yscale('log'); ax.set_xlim(0.41, 0.075)
        ax.set_xticks(H); ax.set_xticklabels([f'{h:.3f}\n(n={n})' for h, n in zip(H, [1, 2, 3, 5])], fontsize=6.6)
        ax.minorticks_off()
        ax.set_xlabel('mesh size $h_{\\max}$')
        ax.legend(loc='upper center', bbox_to_anchor=(0.5, -0.33), ncol=2, handlelength=1.6)
    axes[1].set_yticks([2, 5, 10, 20, 40]); axes[1].set_yticklabels(['2', '5', '10', '20', '40'])
    axes[0].set_yticks([0.1, 0.3, 1, 3, 10, 30]); axes[0].set_yticklabels(['0.1', '0.3', '1', '3', '10', '30'])
    for ax in axes:
        ax.minorticks_off()
    fig.savefig(FIG/'fig_su2_convergence.pdf'); plt.close(fig)
    return {'native_n5': float(native[-1]), 'two_ring_n5': float(rows('pg_q6.json')[-1])}


def su2_spectra():
    base = REPO/'research/su2-pg-stability-20261117'
    fig, axes = plt.subplots(1, 3, figsize=(W2, 2.55), sharey=True)
    out = {}
    for ax, (key, c, mk, lab) in zip(axes, [('native', BLUE, 's', 'native $SU(2)$ Whitney'), ('pg', AQUA, 'o', 'two-ring PG'),
                                            ('pg_onering', ORANGE, '^', 'one-ring PG')]):
        lam = np.load(base/f'L1_{key}.eigs.npy'); lam = lam[abs(lam) >= 1]
        rough = int((lam.real < -10).sum())
        x = np.sign(lam.real)*np.log10(1+abs(lam.real))
        ax.axvline(0, color=INK, lw=0.6); ax.axvspan(-5.2, -np.log10(11), color='#d03b3b', alpha=0.05, lw=0)
        ax.scatter(x, lam.imag, s=5, color=c, marker=mk, edgecolors='none', alpha=0.8)
        ax.set_title(f'{lab}\n{rough} rough of {lam.size} physical', color=INK, fontsize=8)
        ax.set_xlim(-5.2, 4.4); ax.set_xlabel('sign(Re $\\lambda$)$\\cdot\\log_{10}(1+|$Re $\\lambda|)$', fontsize=7.5)
        out[key] = rough
    axes[0].set_ylabel('Im $\\lambda$')
    axes[0].text(-5.0, axes[0].get_ylim()[1]*0.9, 'rough:\nRe $\\lambda<-10$', fontsize=6.5, color='#d03b3b', va='top')
    fig.savefig(FIG/'fig_su2_spectra.pdf'); plt.close(fig)
    return out


if __name__ == '__main__':
    todo = sys.argv[1:] or ['neighbourhood', 'cell_views', 'chip', 'amplification', 'su2_convergence', 'su2_spectra']
    summary = {}
    for t in todo:
        summary[t] = globals()[t]()
        print(t, json.dumps(summary[t]), flush=True)
    path = DATA/'figures_3d_summary.json'
    old = json.loads(path.read_text()) if path.exists() else {}
    old.update(summary); path.write_text(json.dumps(old, indent=1))
