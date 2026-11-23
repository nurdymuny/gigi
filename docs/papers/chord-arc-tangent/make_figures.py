"""Figures for "Three Ways Across a Turn": computed from the paper's formulas and the frozen meshes.

  fig_sphere_3d.pdf   the three paths on a sphere in 3-D: arc on the surface, chord through it,
                      outside path through the tangent corner at sec(theta/2)
  fig_lengths.pdf     the misses A-C and T-A against theta (1/24 and 1/12), their ratio -> 2,
                      and the Snellius-Huygens residual theta^5/320
  fig_energy.pdf      chord, tangent and 2:1 blended energy errors (theta^4/12, theta^4/6, 7 theta^6/720),
                      with the face-angle range of the real meshes marked
  fig_cayley_3d.pdf   the Cayley vector as stereographic projection from -1 (S^2 drawn for S^3),
                      and the three coordinates of a turn, 2 sin(a/2), a, 2 tan(a/2)
  fig_time.pdf        explicit Euler, implicit Euler and Crank-Nicolson on y' = i y; phase lag s^3/12
  fig_face_angles.pdf face-angle distributions at n = 1, 2, 3, 5 and the chord-arc energy gap per face

Colour = path identity, fixed across the paper: chord blue, arc aqua, outside/tangent orange,
2:1 blend violet (palette checked with the dataviz validator; every series is also direct-labelled).
Magnitude (mesh level, shell radius) uses one blue ramp, light to dark.

usage: GIGI_RICCI_DATA_ROOT=<gigi checkout with output/> python docs/papers/chord-arc-tangent/make_figures.py [names...]
(only fig_face_angles needs the frozen meshes)
"""
import json
import os
import sys
from pathlib import Path

import numpy as np
import matplotlib
matplotlib.use('pdf')
import matplotlib.pyplot as plt
from matplotlib.colors import LinearSegmentedColormap

HERE = Path(__file__).resolve().parent
LANG = os.environ.get('FIG_LANG', 'en')            # FIG_LANG=ru writes the Russian edition's figures
FIG = HERE / 'figures' if LANG == 'en' else HERE / LANG / 'figures'

# Russian edition: every on-figure string passes through this table (exact match; others unchanged)
RU = {
    ' corner': ' вершина', 'chord': 'хорда', 'arc': 'дуга', 'outside': 'внешний',
    r'at $180^\circ$ the corner is at infinity': r'при $180^\circ$ вершина уходит в бесконечность',
    r'outside $T-A\approx\theta^3/12$': r'внешний $T-A\approx\theta^3/12$',
    r'chord $A-C\approx\theta^3/24$': r'хорда $A-C\approx\theta^3/24$',
    r'turn $\theta$ (radians)': r'поворот $\theta$ (радианы)',
    'miss against the arc $A=\\theta$': 'промах относительно дуги $A=\\theta$',
    r'limit $2$ as $\theta\to0$': r'предел $2$ при $\theta\to0$',
    r'turn $\theta$': r'поворот $\theta$',
    'overshoot / short-fall': 'избыток / недостача',
    'faces on the\nBerger meshes': 'грани\nсеток Берже',
    r'outside $\approx\theta^2/6$': r'внешний $\approx\theta^2/6$',
    r'chord (Wilson) $\approx\theta^2/12$': r'хорда (Вильсон) $\approx\theta^2/12$',
    r'2:1 blend $\approx7\theta^4/720$': r'смесь 2:1 $\approx7\theta^4/720$',
    r'relative error against $A^2=\theta^2$': r'относительная погрешность к $A^2=\theta^2$',
    r'chord $|U-1|^2$': r'хорда $|U-1|^2$', r'arc $\theta^2$': r'дуга $\theta^2$',
    r'outside $T^2$': r'внешний $T^2$', 'face energy': 'энергия грани',
    r'chord $|U-1|$': r'хорда $|U-1|$', r'arc $|\log U|$': r'дуга $|\log U|$',
    r'Cayley $2|v|$': r'Кэли $2|v|$',
    r'geodesic angle $a$ of $U$ on $S^3$': r'геодезический угол $a$ элемента $U$ на $S^3$',
    'coordinate length': 'координатная длина', 'exact': 'точное',
    r'explicit gains energy, $\approx s^2$': r'явный: энергия растёт, $\approx s^2$',
    r'implicit loses it, $\approx s^2$': r'неявный: энергия убывает, $\approx s^2$',
    'Crank–Nicolson:\nno energy error,\nphase lag $\\approx s^3/12$':
        'Кранк — Николсон:\nбез ошибки энергии,\nфазовое запаздывание $\\approx s^3/12$',
    'step $s$': 'шаг $s$', 'error per step': 'ошибка за шаг',
    r'face holonomy angle $a$ (radians)': r'угол голономии грани $a$ (радианы)',
    'fraction of faces with angle $\\leq a$': 'доля граней с углом $\\leq a$',
    'measured force errors, 4.7%–36%': 'измеренные ошибки силы, 4.7%–36%',
    r'chord–arc energy gap $a^2/12$': r'разрыв энергий хорды и дуги $a^2/12$',
    'explicit: out': 'явный: наружу', 'Crank–Nicolson: on': 'Кранк — Николсон:\nна окружности',
    'implicit: in': 'неявный: внутрь',
}


def tr(s):
    return RU.get(s, s) if LANG == 'ru' and isinstance(s, str) else s


if LANG == 'ru':                                   # route every text call through tr()
    import matplotlib.axes as _maxes
    from mpl_toolkits.mplot3d import axes3d as _a3d
    _text, _xl, _yl, _ti = _maxes.Axes.text, _maxes.Axes.set_xlabel, _maxes.Axes.set_ylabel, _maxes.Axes.set_title
    _maxes.Axes.text = lambda self, x, y, s, *a, **k: _text(self, x, y, tr(s), *a, **k)
    _maxes.Axes.set_xlabel = lambda self, s, *a, **k: _xl(self, tr(s), *a, **k)
    _maxes.Axes.set_ylabel = lambda self, s, *a, **k: _yl(self, tr(s), *a, **k)
    _maxes.Axes.set_title = lambda self, s, *a, **k: _ti(self, tr(s), *a, **k)
    _t3, _t2d = _a3d.Axes3D.text, _a3d.Axes3D.text2D
    _a3d.Axes3D.text = lambda self, x, y, z, s, *a, **k: _t3(self, x, y, z, tr(s), *a, **k)
    _a3d.Axes3D.text2D = lambda self, x, y, s, *a, **k: _t2d(self, x, y, tr(s), *a, **k)
INK, INK2, GRID, SURF = '#0b0b0b', '#52514e', '#e4e3df', '#fcfcfb'
CHORD, ARC, TAN, BLEND = '#2a78d6', '#1baf7a', '#eb6834', '#6250d6'
RAMP = LinearSegmentedColormap.from_list('blue', ['#86b6ef', '#3987e5', '#256abf', '#184f95', '#0d366b'])
plt.rcParams.update({
    'font.family': 'serif', 'font.size': 8.5, 'axes.labelsize': 8.5, 'axes.titlesize': 9,
    'axes.edgecolor': INK2, 'axes.labelcolor': INK, 'xtick.color': INK2, 'ytick.color': INK2,
    'axes.linewidth': 0.6, 'axes.grid': True, 'grid.color': GRID, 'grid.linewidth': 0.5,
    'axes.axisbelow': True, 'legend.frameon': False, 'legend.fontsize': 7.3, 'mathtext.fontset': 'cm',
    'axes.spines.top': False, 'axes.spines.right': False, 'savefig.bbox': 'tight', 'pdf.fonttype': 42})
W2 = 6.9
PRED = dict(lw=0.8, ls=(0, (4, 2)), color=INK2)


def C(t): return 2*np.sin(t/2)
def T(t): return 2*np.tan(t/2)


def label(ax, x, y, s, color=INK, **kw):
    kw.setdefault("fontsize", 7.6)
    ax.text(x, y, s, color=color, **kw)


def style3d(ax, elev, azim, zoom=1.3):
    ax.view_init(elev=elev, azim=azim)
    ax.set_axis_off()
    ax.set_box_aspect((1, 1, 1), zoom=zoom)


def sphere_wire(ax, r=1.0, n=13, color=GRID, lw=0.45, alpha=1.0):
    u = np.linspace(0, 2*np.pi, 200)
    for lat in np.linspace(-np.pi/2, np.pi/2, n)[1:-1]:
        ax.plot(r*np.cos(lat)*np.cos(u), r*np.cos(lat)*np.sin(u), r*np.sin(lat)*np.ones_like(u),
                color=color, lw=lw, alpha=alpha)
    v = np.linspace(-np.pi/2, np.pi/2, 120)
    for lon in np.linspace(0, np.pi, n)[:-1]:
        ax.plot(r*np.cos(v)*np.cos(lon), r*np.cos(v)*np.sin(lon), r*np.sin(v), color=color, lw=lw, alpha=alpha)
        ax.plot(-r*np.cos(v)*np.cos(lon), -r*np.cos(v)*np.sin(lon), r*np.sin(v), color=color, lw=lw, alpha=alpha)


def sphere_surface(ax, r=1.0, color='#dfe9f6', alpha=0.18):
    u, v = np.mgrid[0:2*np.pi:60j, 0:np.pi:30j]
    ax.plot_surface(r*np.cos(u)*np.sin(v), r*np.sin(u)*np.sin(v), r*np.cos(v),
                    color=color, alpha=alpha, linewidth=0, shade=False, zorder=0)


def turn_frame(elev, azim, tilt=0.55):
    """A plane through the centre, tilted towards the camera, and an in-plane orthonormal pair."""
    el, az = np.radians(elev), np.radians(azim)
    cam = np.array([np.cos(el)*np.cos(az), np.cos(el)*np.sin(az), np.sin(el)])
    nrm = cam + tilt*np.array([0.0, 0.0, 1.0]) - 0.35*np.cross(cam, [0, 0, 1.0]); nrm /= np.linalg.norm(nrm)
    e1 = np.cross([0, 0, 1.0], nrm); e1 /= np.linalg.norm(e1)
    e2 = np.cross(nrm, e1)
    return e1, e2, nrm


# --------------------------------------------------------------------------- the three paths in 3-D
def sphere_3d():
    from mpl_toolkits.mplot3d.art3d import Poly3DCollection
    th = np.radians(70)
    ELEV, AZIM = 22, -60
    e1, e2, nrm = turn_frame(ELEV, AZIM)
    P = lambda a, r=1.0: r*(np.cos(a)[..., None]*e1 + np.sin(a)[..., None]*e2)
    A0, B0 = P(np.array(0.0)), P(np.array(th))
    Q = P(np.array(th/2), 1/np.cos(th/2))
    O = np.zeros(3)
    fig = plt.figure(figsize=(W2, 3.3))
    fig.subplots_adjust(left=0, right=1, top=0.97, bottom=0.0, wspace=0.0)

    ax = fig.add_subplot(1, 2, 1, projection='3d'); ax.set_proj_type('ortho')
    sphere_surface(ax, alpha=0.16); sphere_wire(ax, n=11, lw=0.4)
    circ = P(np.linspace(0, 2*np.pi, 300))
    ax.plot(*circ.T, color=INK2, lw=0.5, ls=(0, (1, 2)))
    wedge = np.vstack([O, P(np.linspace(0, th, 60))])
    ax.add_collection3d(Poly3DCollection([wedge], facecolor=ARC, alpha=0.08, edgecolor='none'))
    for X in (A0, B0):
        ax.plot(*np.c_[O, X], color=INK, lw=0.9)
    ax.plot(*np.c_[O, Q], color=INK2, lw=0.6, ls=(0, (2, 2)))
    ax.plot(*P(np.linspace(0, th, 120)).T, color=ARC, lw=2.8, solid_capstyle='round')
    ax.plot(*np.c_[A0, B0], color=CHORD, lw=2.2, solid_capstyle='round')
    ax.plot(*np.c_[A0, Q, B0], color=TAN, lw=2.2, solid_joinstyle='round')
    ax.plot(*P(np.linspace(0, th, 30), 0.22).T, color=INK, lw=0.7)
    ax.text(*P(np.array(th/2), 0.31), r'$\theta$', fontsize=9, ha='center', va='center')
    ax.scatter(*np.c_[O, A0, B0], color=INK, s=12, depthshade=False)
    ax.scatter(*Q, color=TAN, s=16, depthshade=False)
    ax.text(*(A0*1.1), '$1$', fontsize=9, va='top')
    ax.text(*(B0*1.1), '$U$', fontsize=9, ha='right')
    ax.text(*(Q*1.06), r' corner', fontsize=7.4, color=INK2, ha='left')
    ax.text(*((A0+B0)/2*0.82), 'chord', color=CHORD, fontsize=8, ha='center', va='center', fontweight='bold')
    ax.text(*(P(np.array(th*0.62))*1.10), 'arc', color=ARC, fontsize=8, ha='left', fontweight='bold')
    ax.text(*(P(np.array(th*0.12), 1.2)), 'outside', color=TAN, fontsize=8, ha='left', fontweight='bold')
    style3d(ax, ELEV, AZIM, 1.25 if LANG == 'en' else 1.1)   # Cyrillic labels run wider
    for f in (ax.set_xlim, ax.set_ylim, ax.set_zlim): f(-1.0, 1.0)
    ax.text2D(0.03, 0.95, '(a)', transform=ax.transAxes, fontsize=9)

    ax = fig.add_subplot(1, 2, 2, projection='3d'); ax.set_proj_type('ortho')
    sphere_surface(ax, alpha=0.16); sphere_wire(ax, n=11, lw=0.4)
    ax.plot(*circ.T, color=INK2, lw=0.5, ls=(0, (1, 2)))
    angles = np.radians([30, 60, 90, 110])
    for k, a in enumerate(angles):
        sh = 0.4 + 0.6*k/(len(angles)-1)
        Bk, Qk = P(np.array(a)), P(np.array(a/2), 1/np.cos(a/2))
        ax.plot(*P(np.linspace(0, a, 100)).T, color=ARC, lw=1.8, alpha=sh)
        ax.plot(*np.c_[A0, Bk], color=CHORD, lw=1.3, alpha=sh)
        ax.plot(*np.c_[A0, Qk, Bk], color=TAN, lw=1.3, alpha=sh)
        ax.scatter(*Qk, color=TAN, s=10, depthshade=False)
        ax.scatter(*Bk, color=INK, s=6, depthshade=False)
        ax.text(*(Qk*1.07), f' {np.degrees(a):.0f}°', fontsize=7, color=INK2, ha='left')
    ax.scatter(*A0, color=INK, s=12, depthshade=False)
    ax.text(*(A0*1.1), '$1$', fontsize=9, va='top')
    style3d(ax, ELEV, AZIM, 1.3)
    for f in (ax.set_xlim, ax.set_ylim, ax.set_zlim): f(-1.5, 1.5)
    ax.text2D(0.97, 0.06, r'at $180^\circ$ the corner is at infinity', transform=ax.transAxes, fontsize=7,
              color=INK2, ha='right')
    ax.text2D(0.03, 0.95, '(b)', transform=ax.transAxes, fontsize=9)
    fig.savefig(FIG / 'fig_sphere_3d.pdf'); plt.close(fig)


# --------------------------------------------------------------------------- lengths
def lengths():
    t = np.geomspace(0.02, 2.0, 400)
    c, a, tt = C(t), t, T(t)
    fig, axs = plt.subplots(1, 2, figsize=(W2, 2.7), gridspec_kw=dict(width_ratios=[1.25, 1]))
    fig.subplots_adjust(wspace=0.3)
    ax = axs[0]
    ok = t > 0.06
    ax.loglog(t, a - c, color=CHORD, lw=2)
    ax.loglog(t, tt - a, color=TAN, lw=2)
    ax.loglog(t[ok], ((2*c + tt)/3 - a)[ok], color=BLEND, lw=2)
    ax.loglog(t, t**3/24, **PRED); ax.loglog(t, t**3/12, **PRED); ax.loglog(t[ok], t[ok]**5/320, **PRED)
    label(ax, 0.05, 1.2e-4, r'outside $T-A\approx\theta^3/12$', TAN, fontweight='bold', rotation=27)
    label(ax, 0.07, 3.5e-6, r'chord $A-C\approx\theta^3/24$', CHORD, fontweight='bold', rotation=27)
    label(ax, 0.17, 1.3e-8, r'$\frac{2C+T}{3}-A\approx\theta^5/320$', BLEND, fontweight='bold', rotation=40)
    ax.set_xlabel(r'turn $\theta$ (radians)'); ax.set_ylabel('miss against the arc $A=\\theta$')
    ax.set_ylim(1e-9, 1); ax.set_title('(a)', loc='left')
    ax = axs[1]
    tl = np.linspace(0.01, 3.0, 400)
    ax.plot(tl, (T(tl) - tl)/(tl - C(tl)), color=INK, lw=2)
    ax.axhline(2, **PRED)
    ax.set_xlim(0, 3.1); ax.set_ylim(1.5, 6)
    ax.set_xticks([0, np.pi/4, np.pi/2, 3*np.pi/4, np.pi])
    ax.set_xticklabels(['0', r'$\pi/4$', r'$\pi/2$', r'$3\pi/4$', r'$\pi$'])
    label(ax, 0.1, 1.72, r'limit $2$ as $\theta\to0$', INK2)
    label(ax, 0.35, 4.6, r'$\dfrac{T-A}{A-C}$', INK, fontsize=9)
    ax.set_xlabel(r'turn $\theta$'); ax.set_ylabel('overshoot / short-fall'); ax.set_title('(b)', loc='left')
    fig.savefig(FIG / 'fig_lengths.pdf'); plt.close(fig)


# --------------------------------------------------------------------------- energy
def energy():
    rows = json.loads((HERE / 'face_angles.json').read_text())
    t = np.geomspace(1e-3, 2.0, 500)
    c2, t2 = C(t)**2, T(t)**2
    fig, axs = plt.subplots(1, 2, figsize=(W2, 2.7), gridspec_kw=dict(width_ratios=[1.25, 1]))
    fig.subplots_adjust(wspace=0.3)
    ax = axs[0]
    lo = min(r['median_a'] for r in rows); hi = max(r['max_a'] for r in rows)
    ax.axvspan(lo, hi, color='#e6eef9', lw=0, zorder=0)
    ax.text(np.sqrt(lo*hi), 2e-9*3, 'faces on the\nBerger meshes', fontsize=7, color=INK2, ha='center', va='bottom')
    ok = t > 0.02
    ax.loglog(t, (t**2 - c2)/t**2, color=CHORD, lw=2)
    ax.loglog(t, (t2 - t**2)/t**2, color=TAN, lw=2)
    ax.loglog(t[ok], (((2*c2 + t2)/3 - t**2)/t**2)[ok], color=BLEND, lw=2)
    ax.loglog(t, t**2/12, **PRED); ax.loglog(t, t**2/6, **PRED); ax.loglog(t[ok], 7*t[ok]**4/720, **PRED)
    label(ax, 1.3e-3, 1.5e-6, r'outside $\approx\theta^2/6$', TAN, fontweight='bold', rotation=24)
    label(ax, 2.2e-3, 1.3e-7, r'chord (Wilson) $\approx\theta^2/12$', CHORD, fontweight='bold', rotation=24)
    label(ax, 0.14, 1.5e-8, r'2:1 blend $\approx7\theta^4/720$', BLEND, fontweight='bold', rotation=40)
    ax.set_ylim(1e-9, 1)
    ax.set_xlabel(r'turn $\theta$ (radians)'); ax.set_ylabel(r'relative error against $A^2=\theta^2$')
    ax.set_title('(a)', loc='left')
    ax = axs[1]
    tl = np.linspace(0, 3.05, 400)
    ax.plot(tl, C(tl)**2, color=CHORD, lw=2); ax.plot(tl, tl**2, color=ARC, lw=2); ax.plot(tl, T(tl)**2, color=TAN, lw=2)
    ax.axvline(np.pi, color=INK2, lw=0.6, ls=(0, (1, 2)))
    ax.set_xlim(0, 3.3); ax.set_ylim(0, 14)
    ax.set_xticks([0, np.pi/2, np.pi]); ax.set_xticklabels(['0', r'$\pi/2$', r'$\pi$'])
    label(ax, 3.0, 2.6, r'chord $|U-1|^2$', CHORD, ha='right', fontweight='bold')
    label(ax, 2.95, 10.0, r'arc $\theta^2$', ARC, ha='right', fontweight='bold')
    label(ax, 2.2, 12.5, r'outside $T^2$', TAN, ha='right', fontweight='bold')
    label(ax, 3.1, 13.2, r'$U=-1$', INK2, ha='right', fontsize=7)
    ax.set_xlabel(r'turn $\theta$'); ax.set_ylabel('face energy'); ax.set_title('(b)', loc='left')
    fig.savefig(FIG / 'fig_energy.pdf'); plt.close(fig)


# --------------------------------------------------------------------------- Cayley as stereographic projection
def cayley_3d():
    fig = plt.figure(figsize=(W2, 3.0))
    gs = fig.add_gridspec(1, 2, width_ratios=[1.3, 1], wspace=0.15, left=0.0, right=0.98, top=0.95, bottom=0.14)
    ax = fig.add_subplot(gs[0], projection='3d'); ax.set_proj_type('ortho')
    sphere_surface(ax, alpha=0.12); sphere_wire(ax, n=9, lw=0.35)
    u = np.linspace(0, 2*np.pi, 240)
    shells = np.radians([30, 60, 90, 120])
    cols = [RAMP(x) for x in np.linspace(0.1, 1, len(shells))]
    Rp = 1.85
    plane = np.array([[np.cos(w)*Rp*1.05, np.sin(w)*Rp*1.05, 0] for w in np.linspace(0, 2*np.pi, 120)])
    from mpl_toolkits.mplot3d.art3d import Poly3DCollection
    ax.add_collection3d(Poly3DCollection([plane], facecolor='#e6eef9', alpha=0.45, edgecolor=GRID, lw=0.5))
    phi = np.radians(-30)
    S = np.array([0, 0, -1.0])
    for a, col in zip(shells, cols):
        ax.plot(np.sin(a)*np.cos(u), np.sin(a)*np.sin(u), np.cos(a)*np.ones_like(u), color=col, lw=1.7)
        r = np.tan(a/2)
        ax.plot(r*np.cos(u), r*np.sin(u), 0*u, color=col, lw=1.1, ls=(0, (3, 1.5)))
        X = np.array([np.sin(a)*np.cos(phi), np.sin(a)*np.sin(phi), np.cos(a)])
        Y = np.array([r*np.cos(phi), r*np.sin(phi), 0])
        far = X if X[2] > 0 else Y
        ax.plot(*np.c_[S, far], color=INK2, lw=0.45)
        ax.scatter(*np.c_[X, Y], color=col, s=9, depthshade=False)
        ax.text(-r*np.cos(phi)*1.0, -r*np.sin(phi)*1.0 - 0.05, 0.02, f'{np.degrees(a):.0f}°', fontsize=6.8,
                color=INK2, ha='center', va='bottom')
    ax.scatter(0, 0, 1, color=INK, s=16, depthshade=False); ax.text(0.06, 0, 1.1, r'$1$', fontsize=9)
    ax.scatter(0, 0, -1, color=INK, s=16, depthshade=False); ax.text(0.1, 0, -1.22, r'$-1$', fontsize=9)
    style3d(ax, 16, -62, 1.55)
    for f in (ax.set_xlim, ax.set_ylim, ax.set_zlim): f(-1.5, 1.5)
    ax.text2D(0.03, 0.95, '(a)', transform=ax.transAxes, fontsize=9)
    ax = fig.add_subplot(gs[1])
    a = np.linspace(0, 3.08, 400)
    ax.plot(a, C(a), color=CHORD, lw=2); ax.plot(a, a, color=ARC, lw=2); ax.plot(a, T(a), color=TAN, lw=2)
    ax.axvline(np.pi, color=INK2, lw=0.6, ls=(0, (1, 2)))
    ax.set_xlim(0, 3.3); ax.set_ylim(0, 8)
    ax.set_xticks([0, np.pi/2, np.pi]); ax.set_xticklabels(['0', r'$\pi/2$', r'$\pi$'])
    label(ax, 3.1, 1.35, r'chord $|U-1|$', CHORD, ha='right', fontweight='bold')
    label(ax, 3.1, 3.55, r'arc $|\log U|$', ARC, ha='right', fontweight='bold')
    label(ax, 2.35, 6.8, r'Cayley $2|v|$', TAN, ha='right', fontweight='bold')
    ax.set_xlabel(r'geodesic angle $a$ of $U$ on $S^3$'); ax.set_ylabel('coordinate length')
    ax.set_title('(b)', loc='left')
    fig.savefig(FIG / 'fig_cayley_3d.pdf'); plt.close(fig)


# --------------------------------------------------------------------------- time stepping
def time_stepping():
    s, K = 0.3, 21
    k = np.arange(K + 1)
    ex, im = (1 + 1j*s)**k, (1/(1 - 1j*s))**k
    cn, exact = ((1 + 0.5j*s)/(1 - 0.5j*s))**k, np.exp(1j*s*k)
    fig, axs = plt.subplots(1, 2, figsize=(W2, 2.95), gridspec_kw=dict(width_ratios=[1, 1.15]))
    fig.subplots_adjust(wspace=0.42)
    ax = axs[0]
    u = np.linspace(0, 2*np.pi, 300)
    ax.plot(np.cos(u), np.sin(u), color=INK2, lw=0.6)
    for z, col in ((ex, TAN), (im, CHORD), (cn, ARC)):
        ax.plot(z.real, z.imag, color=col, lw=1.5, marker='o', ms=2.8, mec='white', mew=0.4)
    ax.plot(exact[-1].real, exact[-1].imag, 'o', ms=6, mfc='none', mec=INK, mew=0.8)
    for k_, (txt, col) in enumerate((('explicit: out', TAN), ('Crank–Nicolson: on', ARC), ('implicit: in', CHORD))):
        label(ax, 3.45 if LANG == 'en' else 4.6, 1.55 - (0.32 if LANG == 'en' else 0.5)*k_, txt, col, fontweight='bold', ha='right', va='center', fontsize=7.4)
    label(ax, 1.12, -0.17, 'exact', INK2, fontsize=7, va='center')
    ax.set_aspect('equal'); ax.set_xlim(-2.0, 3.5 if LANG == 'en' else 4.7); ax.set_ylim(-1.75, 1.75)
    ax.set_xlabel(r'Re $y$'); ax.set_ylabel(r'Im $y$'); ax.set_title('(a)', loc='left')
    ax = axs[1]
    ss = np.geomspace(1e-2, 1.5, 300)
    ax.loglog(ss, np.abs(1 + 1j*ss)**2 - 1, color=TAN, lw=2)
    ax.loglog(ss, 1 - np.abs(1/(1 - 1j*ss))**2, color=CHORD, lw=2, ls=(0, (5, 1.5)))
    ax.loglog(ss, ss - 2*np.arctan(ss/2), color=ARC, lw=2)
    ax.loglog(ss, ss**3/12, **PRED)
    label(ax, 0.011, 1.2, r'explicit gains energy, $\approx s^2$', TAN, fontweight='bold', fontsize=7.2)
    label(ax, 0.011, 0.25, r'implicit loses it, $\approx s^2$', CHORD, fontweight='bold', fontsize=7.2)
    label(ax, 0.06, 2e-7, 'Crank–Nicolson:\nno energy error,\n' r'phase lag $\approx s^3/12$', ARC,
          fontweight='bold', fontsize=7.2, va='center')
    ax.set_ylim(1e-8, 5)
    ax.set_xlabel('step $s$'); ax.set_ylabel('error per step'); ax.set_title('(b)', loc='left')
    fig.savefig(FIG / 'fig_time.pdf'); plt.close(fig)


# --------------------------------------------------------------------------- face angles on the real meshes
def face_angles():
    sys.path.insert(0, str(HERE.parents[2] / 'scripts'))
    import ricci_su2_pg_force as F
    data = {}
    per_face = HERE / 'face_angles_per_face.json'      # committed receipt: every face angle, from the meshes below
    if (F.BERGER / "level0_berger_q6.input.json").exists():
        for level, n in zip(range(4), (1, 2, 3, 5)):
            payload = json.loads((F.BERGER / f"level{level}_berger_q6.input.json").read_text())
            raw = json.loads((F.BERGER / f"level{level}_berger_q6.output.json").read_text())
            links = np.asarray(raw["links"], float)
            D = np.array([F.defects(F.local_links(c, links)) + F.ONE for c in payload["cells"]]).reshape(-1, 4)
            data[n] = np.arctan2(np.linalg.norm(D[:, 1:], axis=1), D[:, 0])
        per_face.write_text(json.dumps({str(n): [float(v) for v in a] for n, a in data.items()}))
    else:                                               # meshes not in this checkout: use the receipt
        data = {int(n): np.asarray(a) for n, a in json.loads(per_face.read_text()).items()}
    ref = {r['n']: r for r in json.loads((HERE / 'face_angles.json').read_text())}
    for n, a in data.items():           # the figure and Table 2 come from the same numbers
        assert len(a) == ref[n]['faces'] and abs(np.median(a) - ref[n]['median_a']) < 1e-12
        assert abs(a.max() - ref[n]['max_a']) < 1e-12
    fig, axs = plt.subplots(1, 2, figsize=(W2, 2.7), gridspec_kw=dict(width_ratios=[1.25, 1]))
    fig.subplots_adjust(wspace=0.32)
    cols = {n: RAMP(x) for n, x in zip(data, np.linspace(0.1, 1, 4))}
    ax = axs[0]
    for n, a in data.items():
        x = np.sort(a); y = np.arange(1, len(x) + 1)/len(x)
        ax.step(np.r_[x[0], x], np.r_[0, y], where='post', color=cols[n], lw=1.8)
        ax.text(x[len(x)//2]*0.93, 0.5, f'$n={n}$', ha='right', va='center', fontsize=7.6, color=INK)
    ax.axvline(0.1, color=INK2, lw=0.6, ls=(0, (1, 2)))
    ax.set_xscale('log'); ax.set_xlim(1e-3, 0.15); ax.set_ylim(0, 1.02)
    ax.set_xlabel(r'face holonomy angle $a$ (radians)'); ax.set_ylabel('fraction of faces with angle $\\leq a$')
    ax.set_title('(a)', loc='left')
    ax = axs[1]
    for p, (n, a) in enumerate(data.items()):
        g = a**2/12
        ax.plot([p, p], [g.min(), g.max()], color=cols[n], lw=6, solid_capstyle='butt')
        ax.plot([p - 0.17, p + 0.17], [np.median(g)]*2, color=INK, lw=1)
    ax.axhspan(0.025, 0.36, color='#fbe5d9', lw=0)
    ax.text(1.5, 0.095, 'measured force errors, 4.7%–36%', ha='center', va='center', fontsize=7.2, color=INK)
    ax.set_yscale('log'); ax.set_ylim(3e-8, 1); ax.set_xlim(-0.6, 3.6)
    ax.set_xticks(range(4)); ax.set_xticklabels([f'$n={n}$' for n in data])
    ax.set_ylabel(r'chord–arc energy gap $a^2/12$'); ax.set_title('(b)', loc='left')
    fig.savefig(FIG / 'fig_face_angles.pdf'); plt.close(fig)


ALL = dict(sphere=sphere_3d, lengths=lengths, energy=energy, cayley=cayley_3d, time=time_stepping,
           faces=face_angles)

if __name__ == '__main__':
    FIG.mkdir(parents=True, exist_ok=True)
    for name in (sys.argv[1:] or ALL):
        ALL[name]()
        print('wrote', name)
