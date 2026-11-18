"""Figures for the paper, from data/*.json|npz (see make_data.py). Vector PDFs in figures/.

Colour = method identity, fixed across every figure (validated categorical order, light mode);
each series also carries its own marker and a direct label (contrast relief for the 3 slots
below 3:1), and every plotted number is also tabulated in the paper.
"""
import json
from pathlib import Path
import numpy as np
import matplotlib
matplotlib.use('pdf')
import matplotlib.pyplot as plt

HERE = Path(__file__).resolve().parent
DATA, FIG = HERE/'data', HERE/'figures'
FIG.mkdir(exist_ok=True)

INK, INK2, GRID, SURF = '#0b0b0b', '#52514e', '#e4e3df', '#fcfcfb'
METHOD = {   # identity -> (colour, marker, label)
    'whitney':              ('#2a78d6', 's', 'Whitney (original)'),
    'pg_one_ring':          ('#eb6834', '^', 'PG, one-ring fit'),
    'pg_two_ring':          ('#1baf7a', 'o', 'PG, two-ring fit'),
    'variational_one_ring': ('#eda100', 'v', 'Variational, one-ring'),
    'variational_two_ring': ('#e87ba4', 'D', 'Variational, two-ring'),
}
plt.rcParams.update({
    'font.family': 'serif', 'font.size': 8.5, 'axes.labelsize': 8.5, 'axes.titlesize': 9,
    'axes.edgecolor': INK2, 'axes.labelcolor': INK, 'xtick.color': INK2, 'ytick.color': INK2,
    'axes.linewidth': 0.6, 'xtick.major.width': 0.6, 'ytick.major.width': 0.6,
    'axes.grid': True, 'grid.color': GRID, 'grid.linewidth': 0.5, 'axes.axisbelow': True,
    'legend.frameon': False, 'legend.fontsize': 7.5, 'figure.dpi': 200,
    'axes.spines.top': False, 'axes.spines.right': False, 'savefig.bbox': 'tight',
    'pdf.fonttype': 42,
})
W1, W2 = 3.35, 6.9          # single- and double-column widths (in)

E = json.loads((DATA/'errors.json').read_text())
H = np.array(E['h_max'])


def series(name, key):
    rows = E['series'][name]
    return np.array([H[r['level']] for r in rows]), np.array([r[key] for r in rows])*100


def slope_guide(ax, x0, y0, p, length=1.6, label=None):
    x1 = x0*length; y1 = y0*(x1/x0)**p
    ax.plot([x0, x1], [y0, y1], color=INK2, lw=0.6, ls=(0, (3, 2)))
    ax.text(x1*1.03, y1, label or f'slope {p}', color=INK2, fontsize=7, va='center')


# ---- Figure 2: force error vs h -------------------------------------------------------------
fig, ax = plt.subplots(figsize=(W1, 2.75))
for name in ['whitney', 'pg_one_ring', 'pg_two_ring', 'variational_one_ring', 'variational_two_ring']:
    c, m, lab = METHOD[name]
    x, y = series(name, 'relative_error')
    ax.plot(x, y, color=c, lw=1.6 if name == 'pg_two_ring' else 1.1, marker=m, ms=4.5,
            mec=SURF, mew=0.8, label=lab, zorder=3 if name == 'pg_two_ring' else 2)
ax.axhline(5, color=INK2, lw=0.7, ls=(0, (1, 1.5)))
ax.text(0.0835, 4.35, 'BW05 threshold 5%', color=INK2, fontsize=6.8, ha='left', va='top')
ax.set_xscale('log'); ax.set_yscale('log')
ax.set_xlim(0.395, 0.074)
ax.set_xticks(H); ax.set_xticklabels([f'{h:.3f}\n(n={n})' for h, n in zip(H, E['n'])], fontsize=6.8)
ax.minorticks_off()
ax.set_xlabel('mesh size $h_{\\max}$ (refinement $\\rightarrow$)')
ax.set_ylabel('relative force error (%)')
ax.legend(loc='upper left', ncol=1, handlelength=1.6, bbox_to_anchor=(1.0, 1.0))
fig.savefig(FIG/'fig_force_error.pdf'); plt.close(fig)

# ---- Figure 3: curvature error vs h ---------------------------------------------------------
fig, ax = plt.subplots(figsize=(W1, 2.5))
for name in ['whitney', 'pg_one_ring', 'pg_two_ring']:
    c, m, lab = METHOD[name]
    x, y = series(name, 'curvature_relative_error')
    ax.plot(x, y, color=c, lw=1.6 if name == 'pg_two_ring' else 1.1, marker=m, ms=4.5, mec=SURF, mew=0.8, label=lab)
    ax.annotate(lab.split(',')[-1].strip() if 'PG' in lab else 'Whitney', (x[-1], y[-1]),
                xytext=(4, 0), textcoords='offset points', fontsize=7, color=INK, va='center')
ax.set_xscale('log'); ax.set_yscale('log'); ax.set_xlim(0.395, 0.06)
ax.set_xticks(H); ax.set_xticklabels([f'{h:.3f}' for h in H], fontsize=6.8); ax.minorticks_off()
slope_guide(ax, 0.30, 0.55, 2, 1.0/2.2, 'order 2')
slope_guide(ax, 0.30, 9.0, 1, 1.0/2.2, 'order 1')
ax.set_xlabel('mesh size $h_{\\max}$'); ax.set_ylabel('relative curvature error (%)')
fig.savefig(FIG/'fig_curvature_error.pdf'); plt.close(fig)

# ---- Figure 4: spectra (n = 2), one-ring vs two-ring ----------------------------------------
S = np.load(DATA/'spectra.npz')
fig, axes = plt.subplots(1, 2, figsize=(W2, 2.95), sharey=True)
for ax, rings, name in [(axes[0], 1, 'pg_one_ring'), (axes[1], 2, 'pg_two_ring')]:
    lam = S[f'pg_r{rings}_L1']; lam = lam[abs(lam) >= 1]
    c, m, lab = METHOD[name]
    neg = lam.real < 0
    ax.axvline(0, color=INK, lw=0.7)
    ax.scatter(np.sign(lam.real)*np.log10(1+abs(lam.real)), lam.imag, s=7, color=c, marker=m,
               edgecolors='none', alpha=0.85, label=f'{lab}: {neg.sum()} of {lam.size} with Re $<0$')
    w = S['whitney_L1']; w = w[abs(w) >= 1]
    ax.scatter(np.log10(1+w), np.zeros_like(w)-0.0, s=5, color=METHOD['whitney'][0], marker='|',
               linewidths=0.6, label='Whitney (symmetric, real)')
    ax.set_xlabel('sign(Re $\\lambda$) $\\cdot\\log_{10}(1+|$Re $\\lambda|)$')
    ax.set_title(f'{lab}, n = 2', color=INK)
    ax.legend(loc='upper center', bbox_to_anchor=(0.5, -0.24), fontsize=6.8, handletextpad=0.3, ncol=1)
    ax.set_xlim(-5, 4.3)
axes[0].set_ylabel('Im $\\lambda$')
fig.savefig(FIG/'fig_spectra.pdf'); plt.close(fig)

# ---- Figure 5: per-cell fit amplification ---------------------------------------------------
C = json.loads((DATA/'conditioning.json').read_text())
fig, axes = plt.subplots(1, 3, figsize=(W2, 2.45), sharey=False)
for ax, level, n in zip(axes, (0, 1, 2), (1, 2, 3)):
    bins = np.logspace(np.log10(20), np.log10(300), 40)
    for rings, name in [(1, 'pg_one_ring'), (2, 'pg_two_ring')]:
        amp = np.array(C[f'L{level}_r{rings}']['amplification'])
        c, m, lab = METHOD[name]
        ax.hist(amp, bins=bins, color=c, alpha=0.85, label=f'{lab.split(", ")[1]}: median {np.median(amp):.0f}',
                edgecolor=SURF, linewidth=0.4)
    ax.set_xscale('log'); ax.set_title(f'n = {n}', color=INK)
    ax.set_xticks([30, 50, 100, 300]); ax.set_xticklabels(['30', '50', '100', '300']); ax.minorticks_off()
    ax.set_xlabel('fit amplification $\\|\\mathcal{R}_K\\|_2$')
    ax.legend(loc='upper center', bbox_to_anchor=(0.5, -0.32), fontsize=6.6)
axes[0].set_ylabel('cells')
fig.savefig(FIG/'fig_conditioning.pdf'); plt.close(fig)
# ---- Figure 1: neighbourhood schematic (exact edge adjacency, 2-D analogue) --------------------
tris = []
for i in range(-5, 6):
    for j in range(-4, 5):
        p = lambda a, b: (a + 0.5*b, b*np.sqrt(3)/2)
        tris.append((p(i, j), p(i+1, j), p(i, j+1)))          # up
        tris.append((p(i+1, j), p(i+1, j+1), p(i, j+1)))      # down
key = lambda v: (round(v[0], 6), round(v[1], 6))
edges = [{frozenset((key(t[a]), key(t[b]))) for a, b in ((0, 1), (1, 2), (0, 2))} for t in tris]
cent = np.array([np.mean(t, axis=0) for t in tris])
k = int(np.argmin(np.linalg.norm(cent-np.array([0.5, np.sqrt(3)/6]), axis=1)))
ring = {k: 0}; frontier = [k]
for r in (1, 2):
    nxt = []
    for a in frontier:
        for b in range(len(tris)):
            if b not in ring and edges[a] & edges[b]:
                ring[b] = r; nxt.append(b)
    frontier = nxt
fill = {0: INK, 1: METHOD['pg_one_ring'][0], 2: METHOD['pg_two_ring'][0]}
alpha = {0: 0.85, 1: 0.75, 2: 0.45}
fig, ax = plt.subplots(figsize=(W2, 2.3))
for i, t in enumerate(tris):
    c = np.mean(t, axis=0)
    if abs(c[0]) > 3.4 or abs(c[1]) > 2.2:
        continue
    poly = plt.Polygon(t, closed=True, facecolor=fill.get(ring.get(i), 'none'), alpha=alpha.get(ring.get(i), 1),
                       edgecolor='#9a9994', lw=0.5)
    ax.add_patch(poly)
ax.set_xlim(-3.4, 3.4); ax.set_ylim(-2.25, 2.35); ax.set_aspect('equal'); ax.axis('off')
n1 = sum(1 for v in ring.values() if v == 1); n2 = sum(1 for v in ring.values() if v == 2)
for y, r, txt in [(1.6, 0, 'cell $K$'), (0.9, 1, f'one ring: {n1} edge-neighbours (3-D: exactly determined fit)'),
                  (0.2, 2, f'two rings: +{n2} cells (3-D: overdetermined least squares)')]:
    ax.add_patch(plt.Rectangle((3.65, y-0.13), 0.26, 0.26, facecolor=fill[r], alpha=alpha[r], edgecolor='none', clip_on=False))
    ax.text(4.05, y, txt, va='center', fontsize=7.5, color=INK, clip_on=False)
fig.savefig(FIG/'fig_schematic.pdf'); plt.close(fig)
print('figures written to', FIG)
