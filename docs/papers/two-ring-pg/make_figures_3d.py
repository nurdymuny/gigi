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
def _proj_basis(d, up=np.array([0, 0, 1.0])):
    d = d/np.linalg.norm(d)
    if abs(up@d) > 0.95:
        up = np.array([1.0, 0, 0])
    u = np.cross(up, d); u /= np.linalg.norm(u)
    return u, np.cross(d, u), d


def _angle(p, q, r):
    """angle at q between q->p and q->r, in degrees"""
    a, b = p - q, r - q
    return float(np.degrees(np.arccos(np.clip(a@b/np.linalg.norm(a)/np.linalg.norm(b), -1, 1))))


def cell_views(ci=0):
    """A drafting sheet for cell K of Figure 1: one isometric view with every true edge length,
    and three views straight down an edge of K, where the two faces at that edge appear as lines
    and the angle between them is the true dihedral angle. The neighbourhood is drawn only as two
    flat silhouettes (first ring, second ring) behind K, for context."""
    from matplotlib.collections import PolyCollection
    cells, faces, _ = mesh()
    nb = tr.neighbourhood_rings(cells, faces, ci, rings=2)
    ring1 = set(tr.neighbourhood_rings(cells, faces, ci, rings=1)) - {ci}
    ring2 = set(nb) - ring1 - {ci}
    to_orth, _, _ = pg.chart(nb[ci])
    O = {k: to_orth(v) for k, v in nb.items()}
    V4 = np.asarray(nb[ci], float)
    K = O[ci]
    scale = np.linalg.norm(V4[0]-V4[1])/np.linalg.norm(K[0]-K[1])
    K = K*scale                                       # chart -> chordal units; K is exact up to this scale
    O = {k: v*scale for k, v in O.items()}
    EDG = [(i, j) for i in range(4) for j in range(i+1, 4)]
    L4 = {e: float(np.linalg.norm(V4[e[0]]-V4[e[1]])) for e in EDG}
    assert max(abs(L4[e] - np.linalg.norm(K[e[0]]-K[e[1]])) for e in EDG) < 1e-12
    cen = K.mean(axis=0)

    def dihedral(i, j):
        k, l = [m for m in range(4) if m not in (i, j)]
        e = K[j]-K[i]; e /= np.linalg.norm(e)
        pk = K[k]-K[i]; pk -= pk@e*e
        pl = K[l]-K[i]; pl -= pl@e*e
        return float(np.degrees(np.arccos(pk@pl/np.linalg.norm(pk)/np.linalg.norm(pl))))
    dih = {e: dihedral(*e) for e in EDG}
    order = sorted(dih, key=dih.get)
    edge_views = [order[0], order[2], order[-1]]

    def outward_normal(f):
        opp = [m for m in range(4) if m not in f][0]
        n = np.cross(K[f[1]]-K[f[0]], K[f[2]]-K[f[0]])
        return -n if n@(K[opp]-K[f[0]]) > 0 else n

    LIGHT1, LIGHT2, KFILL = '#e3e1dc', '#f1f0ec', '#dbe7f6'
    fig, axs = plt.subplots(2, 2, figsize=(W2, 6.9))
    fig.subplots_adjust(left=0.01, right=0.99, top=0.95, bottom=0.06, wspace=0.04, hspace=0.16)
    def clarity(d):                                   # how well a view separates K's vertices and edges
        u, w, _ = _proj_basis(d)
        P = np.c_[K@u, K@w]
        worst = np.inf
        for m in range(4):
            for (i, j) in EDG:
                if m in (i, j):
                    continue
                t = np.clip((P[m]-P[i])@(P[j]-P[i])/((P[j]-P[i])@(P[j]-P[i])), 0, 1)
                worst = min(worst, np.linalg.norm(P[m]-P[i]-t*(P[j]-P[i])))
        return worst
    rng = np.random.default_rng(0)
    cand = rng.normal(size=(4000, 3)); cand /= np.linalg.norm(cand, axis=1)[:, None]
    cand = cand[cand[:, 2] < -0.15]                   # look slightly from above, as a drawing would
    iso = cen - max(cand, key=clarity)
    views = [('iso', None)] + [('edge', e) for e in edge_views]
    bases = {}
    for kind, item in views:
        d = (K[item[1]]-K[item[0]]) if kind == 'edge' else (cen - iso)
        bases[(kind, item)] = _proj_basis(d)
    R = 1.32*max(np.abs(np.c_[K@b[0], K@b[1]] - np.c_[K@b[0], K@b[1]].mean(axis=0)).max() for b in bases.values())

    def boxed(ax, xy, text, color=INK, size=7.6, weight='normal', ha='center'):
        ax.text(*xy, text, color=color, fontsize=size, fontweight=weight, ha=ha, va='center', zorder=12,
                bbox=dict(boxstyle='round,pad=0.16', fc='white', ec='none', alpha=0.95))

    for p_, (ax, (kind, item)) in enumerate(zip(axs.ravel(), views)):
        u, w, d = bases[(kind, item)]
        pr = lambda X: np.c_[X@u, X@w]
        P2 = pr(K); c2 = P2.mean(axis=0)
        # context: two flat silhouettes, no outlines
        for ids, col in ((ring2, LIGHT2), (ring1, LIGHT1)):
            ax.add_collection(PolyCollection([pr(O[k][list(f)]) for k in ids for f in FACES4],
                                             facecolors=col, edgecolors=col, linewidths=0.6, zorder=1))
        # K: front faces filled, edges visible/hidden by face orientation (orthographic)
        front = [f for f in FACES4 if outward_normal(f)@d < 0]
        ax.add_collection(PolyCollection([P2[list(f)] for f in front], facecolors=KFILL, edgecolors='none', zorder=2))
        for (i, j) in EDG:
            vis = any(i in f and j in f for f in front) or kind == 'edge'
            ax.plot(*P2[[i, j]].T, color=INK, lw=1.6 if vis else 0.9, ls='-' if vis else (0, (4, 2.5)),
                    solid_capstyle='round', zorder=4 if vis else 3)
        # vertex labels, pushed away from K's centre; coincident ends share one label
        done = set()
        for m in range(4):
            if m in done:
                continue
            same = [q for q in range(4) if np.linalg.norm(P2[q]-P2[m]) < 1e-9]
            done.update(same)
            off = P2[m]-c2; off = off/np.linalg.norm(off)*0.075*R
            ax.scatter(*P2[m], s=18, color=INK, zorder=6)
            boxed(ax, P2[m]+off, '$' + ',\\,'.join(f'v_{q}' for q in same) + '$', size=8.2)
        if kind == 'iso':
            for (i, j) in EDG:                       # true lengths, set off the edge on K's outside
                mid = (P2[i]+P2[j])/2; t = P2[j]-P2[i]; nrm = np.array([-t[1], t[0]])/np.linalg.norm(t)
                if nrm@(mid-c2) < 0:
                    nrm = -nrm
                vis = any(i in f and j in f for f in front)
                boxed(ax, mid + nrm*0.07*R*(1 if vis else -0.2), f'{L4[(i, j)]:.3f}',
                      color=INK if vis else INK2, size=7.2)
            title = '(a) overview: true edge lengths'
        else:
            i, j = item
            k, l = [m for m in range(4) if m not in (i, j)]
            c0 = P2[i]
            angs = [np.arctan2(*(P2[m]-c0)[::-1]) for m in (k, l)]
            lo, hi = sorted(angs)
            if hi - lo > np.pi:
                lo, hi = hi, lo + 2*np.pi
            r = 0.48*min(np.linalg.norm(P2[k]-c0), np.linalg.norm(P2[l]-c0))
            tt = np.linspace(lo, hi, 80)
            ax.plot(c0[0]+r*np.cos(tt), c0[1]+r*np.sin(tt), color=BLUE, lw=1.8, zorder=7)
            mid = (lo+hi)/2
            boxed(ax, c0 + 1.6*r*np.array([np.cos(mid), np.sin(mid)]), f'{dih[item]:.1f}°', color=BLUE,
                  size=9.5, weight='bold')
            title = f'({"bcd"[p_-1]}) looking down edge $v_{i}v_{j}$: dihedral angle'
        ax.set_title(title, fontsize=8.6, loc='left', color=INK)
        ax.set_aspect('equal'); ax.axis('off')
        ax.set_xlim(c2[0]-R, c2[0]+R); ax.set_ylim(c2[1]-R, c2[1]+R)
        x0, y0 = c2[0]-0.93*R, c2[1]-0.93*R           # scale bar, 0.05 chordal
        ax.plot([x0, x0+0.05], [y0, y0], color=INK, lw=1.3, zorder=9)
        for x in (x0, x0+0.05):
            ax.plot([x, x], [y0-0.012*R, y0+0.012*R], color=INK, lw=1.0, zorder=9)
        ax.text(x0+0.025, y0+0.045*R, '0.05', fontsize=7, ha='center', color=INK)
    handles = [plt.Rectangle((0, 0), 1, 1, fc=KFILL, ec=INK, lw=0.8), plt.Rectangle((0, 0), 1, 1, fc=LIGHT1),
               plt.Rectangle((0, 0), 1, 1, fc=LIGHT2)]
    fig.legend(handles, ['cell $K$ (hidden edges dashed)', 'first ring', 'second ring'], loc='lower center',
               ncol=3, bbox_to_anchor=(0.5, 0.0), handlelength=1.4)
    fig.savefig(FIG/'fig_cell_views.pdf'); plt.close(fig)
    meas = {'cell': ci, 'units': 'chordal length on the unit three-sphere in R^4',
            'chordal_edge_lengths': {f'v{i}v{j}': round(L4[(i, j)], 6) for (i, j) in EDG},
            'dihedral_angles_deg': {f'v{i}v{j}': round(dih[(i, j)], 3) for (i, j) in EDG},
            'face_angles_deg': {''.join(f'v{m}' for m in f): {f'v{f[q]}': round(_angle(K[f[q-1]], K[f[q]], K[f[(q+1) % 3]]), 3)
                                                              for q in range(3)} for f in FACES4},
            'edge_views': [f'v{i}v{j}' for (i, j) in edge_views],
            'volume_chordal': float(abs(np.linalg.det((K[1:]-K[0]).T))/6),
            'chart_scale_check_max_abs': float(max(abs(L4[e] - np.linalg.norm(K[e[0]]-K[e[1]])) for e in EDG))}
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
    # higher-fidelity layout: two large 3-D panels at one arrow scale, plus the 300-chip statistic
    fig = plt.figure(figsize=(W2, 3.35))
    gs = fig.add_gridspec(1, 3, width_ratios=[1.0, 1.0, 0.82], wspace=0.22)
    nb2 = tr.neighbourhood_rings(cells, faces, ci, rings=1)
    Vn = to_orth(nb2[nbr])
    li = cells[nbr]['edges'].index(edge); u, v = pg.EDGES[li]
    L = 0.42*np.ptp(VK, axis=0).max()/scale
    for p, (r, col, name) in enumerate([(1, ORANGE, 'one ring'), (2, AQUA, 'two rings')]):
        ax = fig.add_subplot(gs[0, p], projection='3d')
        ax.computed_zorder = False
        ax.add_collection3d(Poly3DCollection(tetra_polys(Vn), facecolor=ORANGE, alpha=0.07, edgecolor=INK2,
                                             linewidths=0.35, linestyles=':'))
        ax.add_collection3d(Poly3DCollection(tetra_polys(VK), facecolor=BLUE, alpha=0.10, edgecolor=INK,
                                             linewidths=0.9))
        ax.scatter(VK[:, 0], VK[:, 1], VK[:, 2], color=INK, s=9, depthshade=False)
        ax.add_collection3d(Line3DCollection([[Vn[u], Vn[v]]], colors=MAGENTA, linewidths=3.2))
        mid = (Vn[u]+Vn[v])/2
        ax.text(mid[0], mid[1], mid[2], '  chipped edge', color=INK, fontsize=7.4)
        ax.quiver(X[:, 0], X[:, 1], X[:, 2], dP[r][:, 0]*L, dP[r][:, 1]*L, dP[r][:, 2]*L, color=col, linewidth=1.35,
                  arrow_length_ratio=0.3, normalize=False)
        rel = np.linalg.norm(dP[r], axis=1).mean()/np.linalg.norm(P0, axis=1).mean()
        equalise(ax, np.vstack([VK, Vn])); style3d(ax, 24, -46, zoom=1.85)
        ax.set_title(f'({"ab"[p]}) {name}: curvature in $K$\nmoves {100*rel:.1f}%', color=INK, fontsize=8.5, pad=-6)
    axh = fig.add_subplot(gs[0, 2])
    bins = np.logspace(np.log10(max(ratios.min(), 0.5)), np.log10(ratios.max()), 26)
    axh.hist(ratios, bins=bins, color=BLUE, alpha=0.85, edgecolor='white', linewidth=0.4)
    axh.set_xscale('log')
    axh.axvline(np.median(ratios), color=INK, lw=1.0, ls='--')
    axh.text(np.median(ratios)*1.08, axh.get_ylim()[1]*0.92, f'median {np.median(ratios):.1f}x', fontsize=7.4, color=INK, va='top')
    axh.axvline(1.0, color=INK2, lw=0.6)
    axh.set_xlabel('one ring / two rings')
    axh.set_ylabel('chips')
    axh.set_title(f'(c) response ratio,\n{n_stat} random chips', color=INK, fontsize=8.5)
    fig.subplots_adjust(left=0, right=0.97, top=0.86, bottom=0.17)
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
