"""SU(2) flow figures (Section 9.3): data gathered from the committed receipts into
data/su2_flow.json and data/su2_cost.json, then plotted to figures/.

Sources (repo-relative):
  research/su2-flow-20261118/L{0,1}_{native,pg,pg_fixed}_N{32,64}_q6.json   RKMK4, n = 1, 2
  research/su2-cayley-20261118/L2_{native,pg,pg_fixed}_N8.json             Cayley, n = 3
  research/su2-batched-20261119/timing.jsonl                                loop vs batched
  research/su2-batched-20261119/amendment1/run_L*_N8.json                   batched + metric cache,
                                                                            averaged over a Cayley run
"""
import json
from pathlib import Path

import numpy as np
import matplotlib
matplotlib.use('pdf')
import matplotlib.pyplot as plt

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
DATA, FIG = HERE / 'data', HERE / 'figures'

INK, INK2, GRID, SURF = '#0b0b0b', '#52514e', '#e4e3df', '#fcfcfb'
NATIVE = ('#2a78d6', 's', 'Native SU(2) Whitney')
TWO = ('#1baf7a', 'o', 'Two-ring PG')
FROZEN = ('#8a8984', 'v', 'Two-ring, metric frozen')
plt.rcParams.update({
    'font.family': 'serif', 'font.size': 8.5, 'axes.labelsize': 8.5, 'axes.titlesize': 9,
    'axes.edgecolor': INK2, 'axes.labelcolor': INK, 'xtick.color': INK2, 'ytick.color': INK2,
    'axes.grid': True, 'grid.color': GRID, 'grid.linewidth': 0.5, 'axes.axisbelow': True,
    'legend.frameon': False, 'legend.fontsize': 7.5, 'figure.dpi': 200,
    'axes.spines.top': False, 'axes.spines.right': False, 'savefig.bbox': 'tight',
    'pdf.fonttype': 42,
})

FLOW = REPO / 'research' / 'su2-flow-20261118'
CAY = REPO / 'research' / 'su2-cayley-20261118'
BAT = REPO / 'research' / 'su2-batched-20261119'
SRC = {1: (FLOW, '{t}', '_N32_q6'), 2: (FLOW, '{t}', '_N64_q6'), 3: (CAY, '{t}', '_N8')}
LEVEL = {1: 0, 2: 1, 3: 2}


def gather():
    flow = {}
    for n, (d, _, suf) in SRC.items():
        for t in ('native', 'pg', 'pg_fixed'):
            r = json.loads((d / f'L{LEVEL[n]}_{t}{suf}.json').read_text())
            flow[f'{t}_n{n}'] = {'stepper': 'RKMK4' if n < 3 else 'Cayley', 'N': r['N'],
                                 't': [x['t'] for x in r['rows']],
                                 'R': [x['R'] for x in r['rows']],
                                 'E': [x['E'] for x in r['rows']]}
    cost = {'loop_and_batched': [json.loads(x) for x in (BAT / 'timing.jsonl').read_text().split('\n') if x.strip()],
            'batched_metric_cache': [json.loads(x) for x in (BAT / 'amendment1' / 'timing.jsonl').read_text().split('\n') if x.strip()],
            'cayley_run_average': {f'{l}_{t}': json.loads((BAT / 'amendment1' / f'run_L{l}_{t}_N8.json').read_text())['seconds_per_evaluation']
                                   for l in (0, 1, 2) for t in ('native', 'pg')}}
    DATA.mkdir(exist_ok=True)
    (DATA / 'su2_flow.json').write_text(json.dumps(flow, indent=1) + '\n')
    (DATA / 'su2_cost.json').write_text(json.dumps(cost, indent=1) + '\n')
    return flow, cost


def fig_flow(flow):
    fig, (a, b) = plt.subplots(1, 2, figsize=(6.6, 2.55), gridspec_kw={'width_ratios': [1.25, 1]})
    styles = {1: (0, (1, 1.6)), 2: (0, (4, 2)), 3: '-'}
    for t, (c, m, lab) in (('native', NATIVE), ('pg', TWO)):
        for n in (1, 2, 3):
            s = flow[f'{t}_n{n}']
            a.plot(s['t'], 100 * np.array(s['R']), color=c, ls=styles[n], marker=m, ms=3.2,
                   mec=SURF, mew=0.6, lw=1.4 if t == 'pg' else 1.0)
    for n in (1, 2, 3):
        a.plot([], [], color=INK2, ls=styles[n], lw=1.0, label=f'n = {n}')
    a.plot([], [], color=NATIVE[0], marker='s', ls='none', ms=3.5, label='native')
    a.plot([], [], color=TWO[0], marker='o', ls='none', ms=3.5, label='two-ring')
    a.set_yscale('log')
    a.set_yticks([1, 2, 5, 10, 20]); a.set_yticklabels(['1', '2', '5', '10', '20']); a.minorticks_off()
    a.set_xlabel('flow time $t$'); a.set_ylabel('$E(t)/D(t)$  (%)')
    a.set_title('(a) error over distance travelled, during the flow', color=INK)
    a.legend(ncol=5, loc='upper center', bbox_to_anchor=(0.5, -0.3), fontsize=6.6, handlelength=2.2,
             columnspacing=0.8)
    a.set_xticks([0.005, 0.01, 0.015, 0.02])

    h = np.array([1.0, 0.5, 1 / 3])
    for t, (c, m, lab) in (('native', NATIVE), ('pg', TWO), ('pg_fixed', FROZEN)):
        R = np.array([100 * flow[f'{t}_n{n}']['R'][-1] for n in (1, 2, 3)])
        b.plot(h, R, color=c, marker=m, ms=4.5, mec=SURF, mew=0.8, lw=1.6 if t == 'pg' else 1.1, label=lab)
        offs = {'native': (6, 2), 'pg': (-26, -10), 'pg_fixed': (0, 5)}[t]
        for k, (x, y) in enumerate(zip(h, R)):
            if t == 'pg_fixed' and k == 0:
                continue                       # 14.0 sits on the two-ring 13.8; both are in Table 9
            xy = (-24, -3) if (t == 'pg' and k == 0) else (-24, -3) if (t == 'native' and k == 0) else offs
            b.annotate(f'{y:.2f}' if y < 10 else f'{y:.1f}', (x, y), xytext=xy,
                       textcoords='offset points', fontsize=6.4, color=c,
                       ha='center' if t == 'pg_fixed' else 'left')
    b.axhline(5, color=INK2, lw=0.7, ls=(0, (1, 1.5)))
    b.text(1.0, 4.3, 'gate 5%', color=INK2, fontsize=6.6, va='top')
    b.set_xscale('log'); b.set_yscale('log')
    b.set_xticks(h); b.set_xticklabels(['n = 1', 'n = 2', 'n = 3']); b.minorticks_off()
    b.set_yticks([0.5, 1, 2, 5, 10, 20]); b.set_yticklabels(['0.5', '1', '2', '5', '10', '20'])
    b.set_ylim(0.45, 22); b.set_xlim(1.45, 0.29)
    b.set_ylabel('$R=E/D$ at $t=0.02$  (%)')
    b.set_title('(b) final error as the mesh refines', color=INK)
    b.legend(loc='upper center', bbox_to_anchor=(0.5, -0.16), fontsize=6.6, ncol=1)
    fig.tight_layout(w_pad=1.4)
    fig.savefig(FIG / 'fig_su2_flow.pdf'); plt.close(fig)


def fig_cost(cost):
    loop = {(r['level'], r['trial']): r for r in cost['loop_and_batched']}
    a1 = {(r['level'], r['trial']): r for r in cost['batched_metric_cache']}
    fig, axes = plt.subplots(1, 2, figsize=(6.6, 2.4), sharey=True)
    x = np.arange(3)
    wbar = 0.26
    for ax, (trial, (c, _, lab)) in zip(axes, (('native', NATIVE), ('pg', TWO))):
        vals = [[loop[(l, trial)]['loop_s_per_eval'] for l in range(3)],
                [loop[(l, trial)]['batched_s_per_eval'] for l in range(3)],
                [cost['cayley_run_average'][f'{l}_{trial}'] for l in range(3)]]
        names = ['per-cell loop', 'batched', 'batched + metric cache,\naveraged over a Cayley run']
        alphas = [0.35, 0.7, 1.0]
        for k in range(3):
            bars = ax.bar(x + (k - 1) * wbar, vals[k], wbar, color=c, alpha=alphas[k], edgecolor=SURF,
                          lw=0.5, label=names[k])
            for xi, v in zip(x + (k - 1) * wbar, vals[k]):
                ax.text(xi, v * 1.12, f'{v:.2g}', ha='center', fontsize=5.8, color=INK2)
        ax.set_yscale('log')
        ax.set_xticks(x); ax.set_xticklabels(['n = 1', 'n = 2', 'n = 3'])
        ax.set_title(f'{lab}', color=INK)
        ax.grid(axis='x', visible=False)
    axes[0].set_ylabel('seconds per force evaluation')
    fig.tight_layout(w_pad=1.0)
    h, l = axes[1].get_legend_handles_labels()
    fig.legend(h, l, loc='upper center', bbox_to_anchor=(0.5, 0.02), ncol=3, fontsize=6.4)
    fig.savefig(FIG / 'fig_su2_cost.pdf'); plt.close(fig)


if __name__ == '__main__':
    flow, cost = gather()
    fig_flow(flow)
    fig_cost(cost)
    print('wrote', FIG / 'fig_su2_flow.pdf', FIG / 'fig_su2_cost.pdf')
