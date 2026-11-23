p="docs/papers/berger-spectrum/paper.tex"; s=open(p).read()
def rep(o,n):
    global s; assert s.count(o)==1,("COUNT",s.count(o),o[:70]); s=s.replace(o,n)
I=lambda x:r"\redins{"+x+"}"; D=lambda x:r"\reddel{"+x+"}"
rep(r"""\newcommand{\su}{\mathfrak{su}(2)}""", r"""% Version 3 draft (21 November 2026): additions as a redline. \redlinetrue shows them (insertions blue, deletions red
% and struck out); \redlinefalse gives the accepted text.
\usepackage[normalem]{ulem}
\newif\ifredline
\redlinetrue
\newcommand{\redins}[1]{\ifredline{\color{blue}#1}\else#1\fi}
\newcommand{\reddel}[1]{\ifredline{\color{red}\sout{#1}}\fi}

\newcommand{\su}{\mathfrak{su}(2)}""")
rep(r"""\date{November 2026 (version 2)}""", r"""\date{November 2026 (version """+D("2")+I("3, draft")+")}")
rep(r""" At $t=4$ the low spectrum on the
Poincar\'e sphere is classified below $136$.""", I(r""" Across all circle-symmetric fields on these spheres, the same
field is an exact eigenvector with eigenvalue $(x+(j+2)/\sqrt t)^2+2\sqrt t\,x-y^2$, where $x$ and $y$ are the
field's strengths along and across the long circle, and it is the floor exactly on one side of an algebraic edge in
$x$; the thresholds of the Berger field are where its path crosses that edge.""")+r""" At $t=4$ the low spectrum on the
Poincar\'e sphere is classified below $136$.""")
rep(r"""\end{itemize}
Section~\ref{sec:family} follows the floor across the Berger family.""", I(r"""  \item \emph{Where the twist is the floor} (Section~\ref{sec:edge}): across every circle-symmetric field on
    $(t,1,1)$ the twist's eigenvalue is in closed form (Proposition~\ref{prop:twistfamily}); it is the floor exactly
    up to an edge in the field's strength along the long circle, the root of an explicit polynomial
    (Proposition~\ref{prop:edge}), and the Berger thresholds are the crossings of that edge.
""")+r"""\end{itemize}
Section~\ref{sec:family} follows the floor across the Berger family"""+I(r""", and Section~\ref{sec:edge} across every
circle-symmetric field""")+".")
newsec = r"""
\redins{\section{Where the twist is the floor}\label{sec:edge}

The Berger field is one member of a larger family. A left-invariant field on $(t,1,1)$ that is symmetric under the
rotation about the long axis has, up to gauge, the form $M_0=x\,\tau_0$, $M_1=y\,\tau_1$, $M_2=y\,\tau_2$: a strength
$x$ along the long circle and a strength $y$ across it. The Berger field is $x=(2-t)/\sqrt t$, $y=\sqrt t$; the
multiples $A=s\theta$ of the Maurer--Cartan form are $x=2s/\sqrt t$, $y=2s$; the squashed meron lies in the family
too. By Remark~\ref{rem:counting} the twist is an exact gauge-orthogonal eigenvector of every member.

\begin{proposition}[The twist across the family]\label{prop:twistfamily}
For every $t>0$, $x,y\in\RR$ and $j\ge0$, the twist's eigenvalue is
\[
  \lambda(x,y,t,j)=\Bigl(x+\frac{j+2}{\sqrt t}\Bigr)^2+2\sqrt t\,x-y^2 .
\]
On the Berger field it is $\lambda_t(j)$. The cost of lifting the twist from the sector $j=0$ to the sector $j$ is
$(j^2+4j)/t+2xj/\sqrt t$; the second term lowers it if and only if $x<0$, that is, when the field along the long
circle turns against the twist's own rotation.
\end{proposition}
\begin{proof}
The twist is alone in its charge block, so its eigenvalue is the diagonal entry of $H+GG^*$ there: by the
Weitzenb\"ock form, $\norm{X_0b}^2+\norm{X_1b}^2+\norm{X_2b}^2+\langle\beta,R_0\beta\rangle$ for $\norm b=1$,
with $R_0=\mathrm{Ric}\otimes I-2[F,\cdot\,]$. Symbolic in $x$, $y$, $t$ and checked against the general builder at
$40$ random points to $1.4\times10^{-15}$ (\path{scripts/ricci_su2_twist_family_closed_form.py}).
\end{proof}

The rows of every charge block are likewise in closed form in $(x,y,t)$, so whether the twist is the floor is a
comparison between $\lambda$ and the lowest eigenvalue of finitely many competing blocks.

\begin{proposition}[The edge]\label{prop:edge}
Let $P(x,y,\sqrt t)$ be the numerator of $\det\bigl(B_{4,0}-\lambda(x,y,t,4)\bigr)$, with $B_{4,0}$ the charge-$0$
block of the sector $j=4$. Over $\mathbb Q(\sqrt2,\sqrt3)$ it factors as $P=P_1P_2$, with $P_2$ of degree $7$, $8$ and
$16$ in $x$, $y$ and $s=\sqrt t$. On the Berger path $x=(2-s^2)/s$, $y=s$, the factor $P_2$ contains the
polynomial of Remark~\ref{rem:tstar} whose root is $t_A$.
\end{proposition}
\begin{proof}
Exact symbolic determinant and factorization, and exact polynomial division on the Berger path
(\path{scripts/ricci_su2_edge_polynomial.py}).
\end{proof}

\paragraph{The edge, numerically.} In the sectors $j\le12$, at every $(t,y)$ computed, the twist is the floor of
$H+GG^*$ for $x$ below an edge $x^*(t,y)$ and not above it, and the same holds for the gauge-orthogonal spectrum
with a slightly higher edge. For $y\gtrsim1$ the competitor that sets the edge is the block $B_{4,0}$, and $x^*$ is a
root of $P_2$ (to $10^{-8}$); at small $y$ it is a block of the sector $j=2$ or $3$. The edge rises with the stretch:
the gauge-orthogonal edge is $-0.66$, $-0.36$, $-0.19$, $0.008$, $0.14$ at $t=2,3,4,6,8$ for $y=\sqrt t$, so for a
strongly stretched sphere with a strong cross field the twist is the floor even when $x$ is slightly positive.
Along the Berger path the crossings are exact: the $H+GG^*$ edge is crossed at $t=2.7311222782381144$ (the root
$t_A$ of Remark~\ref{rem:tstar}, to $9\times10^{-16}$) and the gauge-orthogonal edge at $t=2.698026940521$ ($t^*$,
to $10^{-12}$), both through the block $B_{4,0}$ (\path{scripts/ricci_su2_exact_edge.py}). The thresholds of
Remark~\ref{rem:tstar} are therefore not special to the Berger field: they are where its path meets the edge.

\begin{figure}[t]
  \centering
  \includegraphics[width=\textwidth]{figures/fig_twist_map.pdf}
  \caption{Where the twist is the floor, over the circle-symmetric family at $t=1,2,3,4,6$ (sectors $j\le8$;
  double precision): number of sectors in which the twist is the floor; black outline, all nine; dashed, $x=0$.
  Star: the Berger field; diamond: the squashed meron; dashed line: $A=s\theta$.}
  \label{fig:twistmap}
\end{figure}

\begin{theorem}[The floor on a domain]\label{thm:domain}
On $D=\{x\in[-3,-1.5],\ y\in[0.5,3],\ t\in[3,10]\}$ the twist is the lowest eigenvalue of $H+GG^*$ in every sector,
with multiplicity exactly two, and the lowest gauge-orthogonal eigenvalue there.
\end{theorem}
\begin{proof}[Proof (draft)]
By the charge blocks, as in Theorem~\ref{thm:range}. \emph{Sectors $j\ge80$:} the Gershgorin quantity is
$G=4\xi(2l-\xi)(1-1/t)+lK_a+E_a-(\text{ladder})$ with $K_a=4-8/t-4(w_a+x)/\sqrt t>0$ for every non-twist type on
$D$; it is proved positive for $l\ge40$ on $D$ by interval bisection (\path{scripts/ricci_su2_family_large_j.py}).
\emph{Sectors $j<80$:} every non-twist block is certified above $\lambda$ on $D$ by a mean-value interval
certificate (\path{scripts/ricci_su2_family_block_cert.py}). \textbf{[Draft: this half is running; the theorem is
stated only once it completes.]}
\end{proof}

\paragraph{What does not decide it.} Three simpler readings were tested, each written down before its data, and each
failed: that the rotation symmetry makes the twist the floor (on the squashed meron and on two $SU(3)$ families with the
Maurer--Cartan field it is the floor nowhere); that a field strength aligned with the twist's spin makes it the floor
(for $A=s\theta$ with $s>1$, aligned throughout, it is the floor only of $j=0$); and that alignment together with the
twist alone spanning the lowest eigenspace of $R_0$ does (on $35$ such points of $A=s\theta$ on $(t,1,1)$, none). The
curvature term alone does not decide it; the comparison above, governed by $x$, does.}
"""
rep(r"""
\section{Checks}\label{sec:checks}""", newsec + r"""
\section{Checks}\label{sec:checks}""")
rep(r"""\paragraph{Open questions.} The floor outside $[2.75,10]$;""", r"""\paragraph{Open questions.} """+I(r"""The edge $x^*(t,y)$ as a theorem in every sector, and its other competitors in closed form; """)+r"""The floor outside $[2.75,10]$;""")
rep(r"""  \redins{\item the outward error budget""", r"""  \redins{\item Section~\ref{sec:edge}: \path{scripts/ricci_su2_twist_family_closed_form.py},
    \path{scripts/ricci_su2_exact_edge.py},\\ \path{scripts/ricci_su2_edge_polynomial.py},
    \path{scripts/ricci_su2_twist_family_map.py},\\ \path{scripts/ricci_su2_family_large_j.py},
    \path{scripts/ricci_su2_family_block_cert.py};}
  \redins{\item the outward error budget""") if r"""  \redins{\item the outward error budget""" in s else rep(r"""  \item the outward error budget""", r"""  \redins{\item Section~\ref{sec:edge}: \path{scripts/ricci_su2_twist_family_closed_form.py},
    \path{scripts/ricci_su2_exact_edge.py},\\ \path{scripts/ricci_su2_edge_polynomial.py},
    \path{scripts/ricci_su2_twist_family_map.py},\\ \path{scripts/ricci_su2_family_large_j.py},
    \path{scripts/ricci_su2_family_block_cert.py};}
  \item the outward error budget""")
open(p,"w").write(s); print("ok")
