import sys
p = sys.argv[1]; s = open(p).read()

def swap(old_start, old_end, new, include_end=False):
    global s
    a = s.index(old_start); b = s.index(old_end, a)
    if include_end: b += len(old_end)
    s = s[:a] + new + s[b:]

# ---- abstract sentence --------------------------------------------------------------------
swap("what stabilizes it. In the language of a magnetic fastener,",
     "symmetrized variant we tested fails.",
r"""what stabilizes it. In everyday terms the method fits a shape to a guide: it turns each
neighbour into register by the symmetry of the space, holds exactly only what must be
held, lets the rest of the evidence vote, and refuses a piece that does not fit. The
force is accurate and stable in every mode we decomposed. It is not reciprocal, in the
sense that it is not the gradient of an energy, and we document, quantitatively, why every
""", include_end=False)

# ---- introduction opening (through the magnet paragraphs) ----------------------------------
swap("An archaeologist who finds a single rim sherd",
     "Lattice gauge theory and geometric flows meet in a single question.",
r"""A child with a shape-sorter does not force the square block into the round hole. She
turns the block until its outline matches an opening, and when no opening matches, she
stops. Nobody has to be taught that this is intelligence. Fitting a shape against a
guide, turning it into register, holding it where it matches and refusing where it does
not, is among the first things people recognise as understanding, long before anything
is measured. This paper grew from that observation, first put to us as a magnet: pieces
that match should be drawn into register, not clamped into place.

The archaeologist's rim chart is the same act made quantitative. Given a single rim
sherd from a broken pot, she lays its curved edge against a sheet of concentric arcs and
turns it until it sits flush on one of them. The arc it matches gives the width of the
pot. Only the edge is used; the body of the pot is gone.

The reading is only as good as the amount of edge the guide gets to check. A sherd short
enough to offer only three trustworthy points sits flush against \emph{some} arc no
matter what: three points determine a circle exactly, with nothing to spare. A chip in
the rim then does not read as a chip. It reads as curvature, and on a short arc a small
displacement of one point swings the fitted radius enormously. The fit cannot tell damage
from shape, because it has nothing left over to check against. Give it more edge (a
longer sherd, or neighbouring sherds that join it) and the match is
\emph{overdetermined}. Now a chip is outvoted rather than obeyed. And if the rim really
is a circle, the best fit still recovers that circle exactly: the extra edge costs no
accuracy on true shape and buys immunity to damage. When no arc fits at all, the honest
reading is no reading.

This paper is that act on a $3$-manifold. The pot is a smooth connection on the
Poincar\'e homology sphere. The sherds are the cells of a mesh, and, as with the rim, we
are given only numbers on their edges; the curvature between those edges is what we must
read. The guide is fixed. Every force in this paper is tested against the original
Whitney variations of the mesh, and the recovered curvature is the piece that is turned
to fit them.

Two of the guide's moves were already present in the recovery we start from. Each
neighbouring cell is turned into register before it is compared, by the exact symmetry
that relates the sheets of the cover. A neighbour that does not land in register, or a
fit that is ill-posed, is refused rather than forced. What that recovery lacked was edge
to spare. Read from each cell and its immediate neighbours, it is exactly determined,
like three points on a short arc. It is accurate on smooth data, but it reads every rough
pattern as large curvature, and the dynamics built on it are unstable. Read from
neighbours of neighbours, the fit is overdetermined: the cell's own faces are held
exactly, and every other face casts a vote. The fit stays exact on the shapes it is
meant to capture, and the instability disappears. Redundancy, not resolution, is the
stabilizer.

A guide does not move to meet the piece, and neither does ours: the force is not
symmetric, and so it is not the gradient of an energy. Whether a \emph{reciprocal}
version exists, in which each piece's pull on the others matches the pull it receives,
is a separate question. Section~\ref{sec:magnet} states exactly what it would require.

""")

# ---- discussion subsection --------------------------------------------------------------
swap(r"\subsection{Alignment, not force}\label{sec:magnet}",
     r"\paragraph{Limitations.}",
r"""\subsection{Fitting to a guide}\label{sec:magnet}

The introduction described the method as fitting a shape to a guide. Each part of that
description is a property of the force that can be checked, and Table~\ref{tab:guide}
checks it for every force law in this paper.

\begin{table}[h]\centering\small
\caption{Fitting to a guide, scored for each force law. ``Fixed guide'': tested against
the original Whitney variations. ``Stable'': no physical mode with negative real part
(S3); for two rings the stronger certificate $\mu>0$ holds at $n=1,2$. ``Reciprocal'':
symmetric, so the force is the gradient of an energy.}
\label{tab:guide}
\begin{tabular}{lcccc}\toprule
Force law & Fixed guide & Stable & Accurate (\BW{}) & Reciprocal \\ \midrule
Whitney & yes & yes & no ($26\%$ floor) & yes \\
Variational recovery & no & yes & no (diverges) & yes \\
Symmetrized Petrov--Galerkin & partly & no & no (grows) & yes \\
Petrov--Galerkin, one ring & yes & no & yes & no \\
\textbf{Petrov--Galerkin, two rings} & \textbf{yes} & \textbf{yes} & \textbf{yes} & \textbf{no} \\
\bottomrule\end{tabular}
\end{table}

One pattern in the table is worth stating plainly. \emph{Every accurate force keeps the
guide fixed, and every force that moves the guide is inaccurate.} A fixed guide is not
sufficient, as Whitney shows, but on these meshes it has been necessary.

\paragraph{Turning into register.} A neighbouring cell stored on another sheet of the
cover is not compared with $K$ as it lies. It is first turned onto $K$'s sheet by the
exact deck transformation, and only then may its fluxes act. Both recoveries do this,
and without it the fit's condition number reaches $2.4\times10^{14}$. Turning into
register is what makes the comparison meaningful at all; it is not what restores
stability.

\paragraph{Refusing what does not fit.} The recovery refuses rather than forces. If a
transported neighbour's shared vertices do not coincide with $K$'s to $10^{-12}$, or if
the flux map is rank-deficient or worse conditioned than $10^{12}$, no curvature is
produced. None of these refusals was triggered on the meshes reported here; they are the
method's way of saying ``this piece does not belong here''.

\paragraph{Edge to spare.} This is the one change. In the one-ring fit every flux is a
hard constraint, so a rough flux has nowhere to go but into the curvature. In the
two-ring fit only $K$'s own faces are hard; the rest are fitted in least squares, and a
rough flux is outvoted rather than obeyed. The amplification falls from $125$--$254$ to
a mesh-uniform $31$--$47$ (Figure~\ref{fig:conditioning}), and stability follows.

\paragraph{Reciprocity.} The missing property is the one a guide does not have. Our
force is not symmetric, and Section~\ref{sec:removals} shows why the obvious
symmetrizations fail: they make the pull reciprocal by letting the guide move, and the
accuracy goes with it. The condition $\cR^\top Hc_F\approx j^\top Hc_F$ says what a
reciprocal version must achieve: on smooth fields, the piece and the guide must agree
from both sides. An earlier probe in the accompanying reports placed a symmetric
projection layer between the two. It cut the Whitney floor from $26\%$ to about $6\%$
but still floored. A force that is accurate, stable and reciprocal is the open problem
this paper leaves.

The language of this section is ordinary rather than physical: no field, potential or
magnetic Laplacian enters the recovery.

""")
open(p, 'w').write(s)
print('applied')
