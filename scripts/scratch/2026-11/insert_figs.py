import json
p = 'paper.tex'; s = open(p).read()
S = json.load(open('data/figures_3d_summary.json')); c = S['chip']; nb = S['neighbourhood']
vals = {'@R1C@': str(1+nb['ring1_cells']), '@F1@': str(nb['faces_one_ring']), '@R2C@': str(nb['two_ring_cells']),
        '@F2@': str(nb['faces_two_ring']), '@NS@': str(c['n_stat']), '@MED@': f"{c['ratio_median']:.1f}",
        '@Q25@': f"{c['ratio_q25']:.1f}", '@Q75@': f"{c['ratio_q75']:.1f}", '@MIN@': f"{c['ratio_min']:.2f}"}
def fill(t):
    for k, v in vals.items(): t = t.replace(k, v)
    return t
def rep(a, b):
    global s
    assert s.count(a) == 1, a[:70]; s = s.replace(a, fill(b))

rep(r"""  \includegraphics[width=\textwidth]{figures/fig_schematic.pdf}
  \caption{Schematic, two-dimensional analogue of the recovery neighbourhoods. In three
  dimensions the one-ring star has $16$ faces and $5$ closure relations, so the closed
  linear fit (eleven unknowns) is exactly determined. The two-ring neighbourhood
  oversamples it.}""", r"""  \includegraphics[width=\textwidth]{figures/fig_neighbourhood_3d.pdf}
  \caption{The recovery neighbourhoods of an actual cell $K$ of $\T_2$, drawn in $K$'s own
  chart after every neighbour has been carried onto $K$'s sheet. One ring: $K$ and its four
  face neighbours, @R1C@ cells and @F1@ faces. With $5$ closure relations that leaves exactly the
  eleven fluxes the closed linear fit needs, so the fit is exactly determined. Two rings:
  @R2C@ cells and @F2@ faces, so the same eleven coefficients are overdetermined.}""")

rep(r"""\paragraph{The price is size.}""", r"""\paragraph{Seeing the amplification.} Figure~\ref{fig:chip} shows what an amplification
of $125$ means for a single cell. One edge value in a neighbouring cell is changed by $5\%$
of the median edge value, a chip in the rim, and the recovered curvature inside $K$ is
compared before and after. The one-ring curvature swings; the two-ring curvature barely
moves. Over @NS@ randomly chosen cells and chips on $\T_2$, the one-ring response is a median
@MED@ times the two-ring response (quartiles @Q25@ and @Q75@). That is larger than the
ratio of amplifications, because least squares does two things at once: it bounds the
amplification, and it dilutes a single bad value among many good ones. The ratio is not
always above one (the smallest is @MIN@), but the typical cell is far steadier.
Figure~\ref{fig:amp3d} shows the same contrast for every cell of $\T_2$ at once.

\begin{figure}[t]
  \centering
  \includegraphics[width=\textwidth]{figures/fig_chip_3d.pdf}
  \caption{A chip in the rim. The same small change to one neighbouring edge value
  (marked), and the resulting change in the recovered curvature at points inside $K$, drawn
  to the same arrow scale in both panels.}
  \label{fig:chip}
\end{figure}

\begin{figure}[t]
  \centering
  \includegraphics[width=\textwidth]{figures/fig_amplification_3d.pdf}
  \caption{Fit amplification $\norm{\cR_K}_2$ for every cell of $\T_2$, one dot per cell at
  its centroid, all $960$ cells carried onto one sheet of the cover and drawn in the chart
  of one of them. Same logarithmic colour scale in both panels.}
  \label{fig:amp3d}
\end{figure}

\paragraph{The price is size.}""")

rep(r"""(Table~\ref{tab:su2force}). The native force does not pass:""",
    r"""(Table~\ref{tab:su2force}, Figure~\ref{fig:su2conv}). The native force does not pass:""")
rep(r"""\paragraph{Stability.} As in Section~\ref{sec:conditioning},""", r"""\begin{figure}[t]
  \centering
  \includegraphics[width=\textwidth]{figures/fig_su2_convergence.pdf}
  \caption{$SU(2)$ under refinement. (a) The covariant two-ring recovery on the exact
  Levi-Civita fields of the round and Berger metrics converges at second order; without
  register it loses an order, and the abelianised flux stalls. (b) The force on the Berger
  field: the native force stays near $25\%$, the two-ring Petrov--Galerkin force falls below
  the \BW{} threshold, and without register it stalls near $29\%$.}
  \label{fig:su2conv}
\end{figure}

\paragraph{Stability.} As in Section~\ref{sec:conditioning},""")
rep(r"""The two-ring force has no rough modes at either level (Table~\ref{tab:su2stab}).""", r"""\begin{figure}[t]
  \centering
  \includegraphics[width=\textwidth]{figures/fig_su2_spectra.pdf}
  \caption{Generalized spectra of the linearised $SU(2)$ force at $n=2$, against the native
  tangent mass (signed-logarithmic real axis). The shaded band is the pre-registered
  ``rough'' region, $\operatorname{Re}\lambda<-10$.}
  \label{fig:su2spec}
\end{figure}

The two-ring force has no rough modes at either level (Table~\ref{tab:su2stab},
Figure~\ref{fig:su2spec}).""")
open(p, 'w').write(s); print('inserted')
