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
import sys
from pathlib import Path

import numpy as np
import matplotlib
matplotlib.use('pdf')
import matplotlib.pyplot as plt
from matplotlib.colors import LinearSegmentedColormap

HERE = Path(__file__).resolve().parent
FIG = HERE / 'figures'
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


