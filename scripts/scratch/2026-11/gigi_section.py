p = 'docs/papers/chord-arc-tangent/paper.tex'; s = open(p).read()
def rep(a, b):
    global s
    assert s.count(a) == 1, a[:60]; s = s.replace(a, b)

rep(r"""\usepackage{url}""", r"""\usepackage{url}
\usepackage{listings}
\lstdefinestyle{rust}{basicstyle=\ttfamily\footnotesize, columns=fullflexible, keepspaces=true,
  frame=lines, framesep=4pt, xleftmargin=6pt, xrightmargin=6pt, showstringspaces=false,
  morekeywords={let,mut,for,in,fn,pub}}""")

section = r"""\section{The computation in GIGI}\label{sec:gigi}

The numbers in this paper sit in two places, and it matters which is which. Some of the
three paths are already part of \textsc{gigi}, the open-source geometric engine our
curvature-recovery work runs in (PolyForm Noncommercial 1.0.0,
\url{https://github.com/nurdymuny/gigi}). Others exist so far only in this paper's
checking script. Table~\ref{tab:gigi} says which. Paths are relative to the repository
root at commit \texttt{1e8ae17}.

\begin{table}[h!]\centering\small
\caption{Where each idea of the paper lives. ``Engine'' means native Rust in \textsc{gigi};
``script'' means Python in this directory or the repository's \path{scripts/}.}
\label{tab:gigi}
\begin{tabular}{p{4.2cm}p{6.6cm}l}\toprule
Idea & Where & Kind \\ \midrule
The turn as a number, $q_0=\cos a=\tfrac12\operatorname{Re}\operatorname{tr}U$ &
  \texttt{PLAQUETTE} primitive, \path{src/gauge/plaquette.rs}, one $q_0$ per face & engine \\
Half the chord and sameness, $1-S=d^2=(1-\cos\theta)/2$ &
  \texttt{chord\_d\_sq}, \path{src/dials.rs} & engine \\
Chord energy, $D=U_f-1$, weighted by the face mass &
  \texttt{cell\_action}, \path{src/gauge/ricci_simplicial.rs} & engine \\
Exact links (transport along the arc) on the Berger metric &
  \texttt{levi\_civita\_link}, \path{src/gauge/ricci_link.rs} & engine \\
The arc as a map, $\exp$ on $\mathfrak{su}(2)$ &
  Rodrigues exponential, \path{src/gauge/lie_exp.rs} & engine \\
Time stepping &
  \texttt{SYMPLECTIC\_FLOW}: kick--drift--kick leapfrog whose drift is that exponential,
  \path{src/gauge/symplectic_flow.rs} & engine \\
Face angles of Table~\ref{tab:angles} &
  \texttt{face\_angles.py}, from the outputs of \path{examples/ricci_operator_probe.rs} &
  script on engine output \\
Arc (Manton) energy, tangent energy, the $2{:}1$ blend &
  \texttt{verify.py} & script \\
Cayley map and Crank--Nicolson step & \texttt{verify.py} & script \\
Haar sameness and Bertrand & \path{scripts/haar_sameness.py}, \texttt{verify.py} & script \\
\bottomrule\end{tabular}
\end{table}

\paragraph{The chord, as the engine computes it.} The native $SU(2)$ action of
Section~\ref{sec:mesh} is the chord energy, exactly. Each face's holonomy is formed from its
three links, and one is subtracted from its scalar part:
\begin{lstlisting}[style=rust]
let (a, b, c) = [(0, 3, 1), (0, 4, 2), (1, 5, 2), (3, 5, 4)][face];
let mut h = product(product(u[a], u[b]), inverse(u[c]));
h[0] = h[0] - Jet::constant(1.);      // the chord: D = U_f - 1
...
energy = energy + Jet::constant(2. * mass[(i, j)]) * defects[i][k] * defects[j][k];
\end{lstlisting}
The variables are forward-mode jets, so the same pass returns all eighteen right-trivialised
derivatives exactly. The face angles of Table~\ref{tab:angles} are read from what this
code produced on the frozen meshes: the native probe \path{examples/ricci_operator_probe.rs},
built from the source above, whose executable is pinned in the receipts of that work by its
SHA-256 (\texttt{71fb30db\ldots}).

\paragraph{The engine steps along the arc, not the tangent.} \textsc{gigi}'s integrator,
\texttt{SYMPLECTIC\_FLOW}, moves each link by the exponential map, the arc, with a
kick--drift--kick leapfrog driven by the Wilson force. The research flow under development
in the curvature-recovery work uses a Runge--Kutta--Munthe-Kaas method of order four, also
built on the exponential. Neither uses the Cayley map. The Crank--Nicolson step of
Section~\ref{sec:time} is therefore a proposal for the engine, not a description of it.

\paragraph{What is not in the engine.} The arc (Manton) energy, the tangent energy, the
combination of Proposition~\ref{prop:energy} and the Cayley construction are computed only
in \texttt{verify.py}. None is an engine verb, and nothing in this paper claims they have
been tested inside a simulation.

\begin{plain}
Some of this paper is already inside the engine we use: it reads how far each face has
turned, measures the shortcut, and builds its energy from the shortcut, exactly as
Section~\ref{sec:mesh} assumes. Its clock moves things along the true curve. The detour and
the two-to-one blend live only in the checking script for now. Putting the detour's step into
the engine's clock is one of the open items below.
\end{plain}

"""
rep(r"""\section{What is claimed, and what is open}\label{sec:claims}""", section + r"""\section{What is claimed, and what is open}\label{sec:claims}""")
rep(r"""  \item A Cayley (Crank--Nicolson) time integrator for the $SU(2)$ flow, tested against
    exact trajectories, with its step size set by the numerical range.""",
    r"""  \item A Cayley (Crank--Nicolson) time integrator for the $SU(2)$ flow, tested against
    exact trajectories, with its step size set by the numerical range. \textsc{gigi}'s
    current integrator steps with the exponential (Section~\ref{sec:gigi}).""")
rep(r"""the natural rule to try first. The numerical range we are measuring tells us how big its
steps may be.""", r"""the natural rule to try first. The numerical range we are measuring tells us how big its
steps may be. The engine's clock today steps along the true curve instead
(Section~\ref{sec:gigi}).""")
open(p, 'w').write(s); print('section added')
