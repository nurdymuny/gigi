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
    todo = sys.argv[1:] or ['neighbourhood', 'chip', 'amplification', 'su2_convergence', 'su2_spectra']
    summary = {}
    for t in todo:
        summary[t] = globals()[t]()
        print(t, json.dumps(summary[t]), flush=True)
    path = DATA/'figures_3d_summary.json'
    old = json.loads(path.read_text()) if path.exists() else {}
    old.update(summary); path.write_text(json.dumps(old, indent=1))
