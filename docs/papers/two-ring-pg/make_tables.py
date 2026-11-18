"""Generate tables/*.tex from data/ and the gate receipt. No number is typed by hand."""
import json
from pathlib import Path
import numpy as np

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
DATA, TAB = HERE/'data', HERE/'tables'
TAB.mkdir(exist_ok=True)
E = json.loads((DATA/'errors.json').read_text())
G = json.loads((REPO/'research/ricci-pg-tworing-20261117/diagnostic.json').read_text())
NR = json.loads((DATA/'numerical_range.json').read_text())
S = np.load(DATA/'spectra.npz')
H, N = E['h_max'], E['n']


def pct(x, d=2):
    return '---' if x is None else f'{100*x:.{d}f}'


# BW05 orders use the ABSOLUTE mass-norm error. The reference norm ||4a|| depends only on the level,
# so absolute = relative * ref_l, with ref_l read from the gate receipt.
REF = [G['cases'][f'level{l}_q6']['full_mass_error']/G['cases'][f'level{l}_q6']['relative_error'] for l in range(4)]


def orders(rel):
    errs = [None if r is None else r*REF[i] for i, r in enumerate(rel)]
    return [None]+[None if errs[i] is None or errs[i+1] is None else
                   float(np.log(errs[i]/errs[i+1])/np.log(H[i]/H[i+1])) for i in range(len(errs)-1)]


def col(name, key='relative_error'):
    rows = {r['level']: r[key] for r in E['series'][name]}
    return [rows.get(l) for l in range(4)]


# ---- force table -----------------------------------------------------------------------
W, P1, P2 = col('whitney'), col('pg_one_ring'), col('pg_two_ring')
V1, V2 = col('variational_one_ring'), col('variational_two_ring')
o2 = orders(P2); o1 = orders(P1)
gate = G['numerical_gate']
lines = [r'\begin{table}[t]\centering\small',
         r'\caption{Relative instantaneous force error $\varepsilon_n$ (\%, $q6$). Orders are observed orders between consecutive levels, computed as in \BW{} from the absolute mass-norm error. The two-ring Petrov--Galerkin column passes \BW{} as evaluated by the unchanged gate function.}',
         r'\label{tab:force}',
         r'\begin{tabular}{rrrrrrrrr}\toprule',
         r'$n$ & $h_{\max}$ & Whitney & PG 1-ring & order & \textbf{PG 2-ring} & \textbf{order} & Var.\ 1-ring & Var.\ 2-ring \\ \midrule']
for l in range(4):
    s1 = '' if o1[l] is None else f'{o1[l]:.2f}'
    s2 = '' if o2[l] is None else r'\textbf{' + f'{o2[l]:.2f}' + '}'
    p2 = r'\textbf{' + pct(P2[l]) + '}'
    lines.append(f"{N[l]} & {H[l]:.4f} & {pct(W[l])} & {pct(P1[l])} & {s1} & {p2} & {s2} & {pct(V1[l],1)} & {pct(V2[l],1)} " + r'\\')
q = gate['quadrature_mass_differences']; li = gate['linear_correction_mass_estimates']
lines += [r'\midrule',
          r'\multicolumn{9}{l}{Two-ring gate: \texttt{pass}='+str(gate['pass']).lower()+
          f"; $q4/q6$ differences {q[0]:.1e}--{q[-1]:.1e}; solver corrections $\\le$ {max(li):.1e}." + r'}\\',
          r'\bottomrule\end{tabular}\end{table}']
(TAB/'force.tex').write_text('\n'.join(lines)+'\n')

# ---- stability table -------------------------------------------------------------------
def s3_one(level):
    lam = S[f'pg_r1_L{level}']; w = S[f'whitney_L{level}']
    phys = lam[abs(lam) >= 1]
    return int((abs(w) < 1).sum()), int((abs(lam) < 1).sum()), float(phys.real.min()), int((phys.real < 0).sum())

lines = [r'\begin{table}[t]\centering\small',
         r"\caption{Stability. S3: generalized spectrum of $(K_{\mathrm{PG}},B_h)$; ``gauge'' counts eigenvalues of modulus below one (Whitney / PG). $\mu$: smallest eigenvalue of the symmetric part against $B_h$ on the physical space (Proposition~\ref{prop:diss}). Dashes: not decomposed.}",
         r'\label{tab:stability}',
         r'\begin{tabular}{rr rr rr rr}\toprule',
         r' & & \multicolumn{2}{c}{min Re $\lambda$} & \multicolumn{2}{c}{\# Re $\lambda<0$} & \multicolumn{2}{c}{$\mu$} \\',
         r'\cmidrule(lr){3-4}\cmidrule(lr){5-6}\cmidrule(lr){7-8}',
         r'$n$ & gauge & 1-ring & \textbf{2-ring} & 1-ring & \textbf{2-ring} & 1-ring & \textbf{2-ring} \\ \midrule']
for l in range(3):
    s2 = G['s3'][str(l)]
    if l < 2:
        gw, g1, m1, n1 = s3_one(l)
        mu1 = f"{NR[f'L{l}_r1']['min_symmetric_part']:.0f}"; mu2 = f"\\textbf{{{NR[f'L{l}_r2']['min_symmetric_part']:.3f}}}"
        r1 = (f'{m1:.0f}', f'{n1}')
    else:
        r1 = ('---', '---'); mu1 = mu2 = '---'
    lines.append(f"{N[l]} & {s2['gauge_dimension_whitney']}/{s2['small_eigenvalues_pg']} & {r1[0]} & "
                 f"\\textbf{{{s2['min_real_part_physical']:.3f}}} & {r1[1]} & \\textbf{{{s2['negative_real_part_count']}}} & {mu1} & {mu2} \\\\")
lines += [r'\bottomrule\end{tabular}\end{table}']
(TAB/'stability.tex').write_text('\n'.join(lines)+'\n')

# ---- removals table --------------------------------------------------------------------
sym = E.get('symmetrised', {})
def sym_cell(r, l, k):
    rows = sym.get(str(r), [])
    return pct(rows[l][k], 1) if l < len(rows) else '---'
lines = [r'\begin{table}[t]\centering\small',
         r"\caption{Mechanism removals: relative force error (\%) at $n=1,2$ (and $n=5$ where measured). ``Transpose term'' is $D^\top(\cR-j)^\top Hj\,Da$, measured in the same norm.}",
         r'\label{tab:removals}',
         r'\begin{tabular}{l rrr l}\toprule',
         r'Variant & $n=1$ & $n=2$ & $n=5$ & Fails \\ \midrule',
         f"Two-ring Petrov--Galerkin (this work) & {pct(P2[0])} & {pct(P2[1])} & {pct(P2[3])} & --- \\\\",
         f"$-$ second ring (one-ring PG) & {pct(P1[0])} & {pct(P1[1])} & {pct(P1[3])} & S3 \\\\",
         f"$-$ recovered trial (Whitney) & {pct(W[0])} & {pct(W[1])} & {pct(W[3])} & \\BW{{}} floor \\\\",
         f"$-$ Whitney test (variational, 2-ring) & {pct(V2[0],1)} & {pct(V2[1],1)} & --- & \\BW{{}}, diverges \\\\",
         f"$-$ Whitney test (variational, 1-ring) & {pct(V1[0],1)} & {pct(V1[1],1)} & --- & \\BW{{}}, diverges \\\\",
         f"Symmetrized $K_{{\\mathrm{{PG}}}}+K_{{\\mathrm{{PG}}}}^\\top-K_W$, 2-ring & {sym_cell(2,0,'symmetrised')} & {sym_cell(2,1,'symmetrised')} & --- & \\BW{{}} \\\\",
         f"\\quad transpose term alone, 2-ring & {sym_cell(2,0,'transpose_term')} & {sym_cell(2,1,'transpose_term')} & --- & \\\\",
         f"Symmetrized, 1-ring & {sym_cell(1,0,'symmetrised')} & {sym_cell(1,1,'symmetrised')} & --- & \\BW{{}} \\\\",
         f"\\quad transpose term alone, 1-ring & {sym_cell(1,0,'transpose_term')} & {sym_cell(1,1,'transpose_term')} & --- & \\\\",
         r'\bottomrule\end{tabular}\end{table}']
(TAB/'removals.tex').write_text('\n'.join(lines)+'\n')
print('tables written')
