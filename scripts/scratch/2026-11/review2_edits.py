p="docs/papers/berger-spectrum/paper.tex"; s=open(p).read()
def rep(o,n):
    global s; assert s.count(o)==1,("COUNT",s.count(o),o[:70]); s=s.replace(o,n)
I=lambda x:r"\redins{"+x+"}"; D=lambda x:r"\reddel{"+x+"}"
rep(r"""So the field has exactly three \emph{physical} growth directions, $-14/3$ (twice) and
$16-2\sqrt{73}$, each orthogonal to the gauge directions.""",
r"""So the """+D("field")+I("left-invariant sector")+r""" has exactly three \emph{physical} growth directions, $-14/3$ (twice) and
$16-2\sqrt{73}$, each orthogonal to the gauge directions."""+I(r""" On $S^3$ other sectors add more: at $t=3$ the
sectors $j=1$ and $j=2$ have four each, among them their twists (Corollary~\ref{cor:family}). On the Poincar\'e
sphere those sectors do not survive, and the three are all."""))
rep(r"""So at $t=4$ there are still exactly three growth directions,""", r"""So at $t=4$ """+I(r"""the Poincar\'e sphere """)+D("there are")+I("has")+r""" still exactly three growth directions,""")
rep(r"""\paragraph{The Hessian and the slice.}""", I(r"""\paragraph{Terms.} A field is \emph{at rest} if it is a Yang--Mills field, a critical point of the
energy. A \emph{growth mode} is a direction of negative second variation, a negative eigenvalue of the Hessian
below. The \emph{stretch} is the parameter $t$ of the Berger metric $(t,1,1)$ of Section~\ref{sec:family}; the
main case is $t=3$. The \emph{twist} of the sector of spin $j/2$ is the field
$b=f\,(\hat\theta^1+i\hat\theta^2)\otimes(\tau_1+i\tau_2)$ of Proposition~\ref{prop:bottom}, with $f$ the top
weight along the long axis: it turns in the two short directions and, in step, in two internal directions. A
twist \emph{goes free} at the stretch where its eigenvalue crosses zero; beyond it, it is a growth mode.

""")+r"""\paragraph{The Hessian and the slice.}""")
rep(r"""with the stretches at which each twist goes free in closed form""", r"""with the stretches at which each twist """+I(r"""(the circular field of Proposition~\ref{prop:bottom}; terms in Section~\ref{sec:setting}) """)+r"""goes free in closed form""")
rep(r"""above the field's growth mode, one third of""", r"""above the field's growth mode"""+I(" (its lowest, negative eigenvalue)")+r""", one third of""")
rep(r"""\bibitem{klingschueth2022} J.~Kling, D.~Schueth, On the Dirac spectrum of homogeneous $3$-spheres, \href{https://arxiv.org/abs/2204.12990}{arXiv:2204.12990}.""",
    r"""\bibitem{klingschueth2022} J.~Kling, D.~Schueth, On the Dirac spectrum of homogeneous $3$-spheres, """+I(r"""\emph{J.\ Geom.\ Anal.} (2022), \href{https://doi.org/10.1007/s12220-022-00997-x}{doi:10.1007/s12220-022-00997-x}, """)+r"""\href{https://arxiv.org/abs/2204.12990}{arXiv:2204.12990}.""")
rep(r"""\bibitem{klc2021} S.~Kumar,""", r"""\bibitem{klc2021} """+D(r"S.~Kumar")+I(r"K.~Kumar")+r""",""")
rep(r"""Eigenmodes of dodecahedral space, 2004, \href""", r"""Eigenmodes of dodecahedral space, """+D("2004")+I(r"\emph{Class.\ Quantum Grav.} 21 (2004) 2455--2464")+r""", \href""")
rep(r"""The smallest Laplace eigenvalue of homogeneous $3$-spheres, \href""", r"""The smallest Laplace eigenvalue of homogeneous $3$-spheres, """+I(r"""\emph{Bull.\ Lond.\ Math.\ Soc.} 51 (2019) 49--69, """)+r"""\href""")
rep(r"""certain symmetric spaces, preprint MPI/89-49, Max-Planck-Institut f\"ur Mathematik, Bonn, 1989.""",
    r"""certain symmetric spaces, """+D(r"""preprint MPI/89-49, Max-Planck-Institut f\"ur Mathematik, Bonn, 1989""")+I(r"""\emph{Kodai Math.\ J.} 13 (1990) 317--332""")+".")
rep(r"""\bibitem{higham2002} N.~J.~Higham, \emph{Accuracy and Stability of Numerical Algorithms}, 2nd ed., SIAM, Philadelphia, 2002.""",
    r"""\bibitem{hitchin1974} N.~Hitchin, Harmonic spinors, \emph{Adv.\ Math.} 14 (1974) 1--55.""")
rep(r"""the Dirac operator~\cite{bar1992,klingschueth2022}""", r"""the Dirac operator~\cite{hitchin1974,bar1992,klingschueth2022}""")
rep(r"""given by the Molien series $(1+t^{30})/((1-t^{12})(1-t^{20}))$ of
Klein's invariants~\cite{klein1884}:""", r"""given by the Molien series $(1+t^{30})/((1-t^{12})(1-t^{20}))$ of
"""+D(r"Klein's invariants~\cite{klein1884}")+I(r"the binary icosahedral group, whose invariants go back to Klein~\cite{klein1884}")+":")
rep(r"""The first has eigenvector $(-1,-i,-i,1)$, which is $b$
up to a factor.""", r"""The first has eigenvector $(-1,-i,-i,1)$, which is $b$
up to a factor."""+I(" The other three are eigenvalues of this compression only, not of $H$."))
rep(r"""$(j^2+jt+5t^2-4t\pm t\sqrt{j^2+t^2})/t$. The first has eigenvector $(-1,-i,-i,1)$, and""",
    r"""$(j^2+jt+5t^2-4t\pm t\sqrt{j^2+t^2})/t$"""+I(" (the last three of the compression only)")+r""". The first has eigenvector $(-1,-i,-i,1)$, and""")
rep(r"""gives $-2$, $1$, $6$, $7$ and $10$ in spins $0$ to $2$.""", r"""gives $-2$, $1$, $6$, $7$ and $10$ """+I(r"as the lowest gauge-orthogonal eigenvalues of the sectors $j=0,\dots,4$, ")+r"""in spins $0$ to $2$.""")
rep(r"""resolve the left-invariant sector and
the bottom of spin $6$ (Table~\ref{tab:lattice}).""", r"""resolve the left-invariant sector and
"""+D(r"the bottom of spin $6$")+I(r"two eigenvalues of $H$ in spin $6$: $154/3=51.33$, the floor, and $59.52$, beside the second gauge-orthogonal value $59.50$; $H$ itself has lower, gauge-dominated eigenvalues there, and these two are matched by value")+r""" (Table~\ref{tab:lattice}).""")
rep(r"""Squares and circles: lattice eigenvalues near the bottom of spins $0$ and $6$""", r"""Squares and circles: lattice eigenvalues near the """+D(r"bottom of spins $0$ and $6$")+I(r"left-invariant values and the lowest gauge-orthogonal values of spin $6$"))
rep(r"""Since $\rho_0$ is diagonal and""", I(r"""For every $j$, the antiunitary map $J\otimes K$, with $J$ the real ($j$ even) or quaternionic ($j$ odd)
structure of $V_j$ and $K$ complex conjugation on the frame and Lie indices, commutes with $H+GG^*$, whose
coefficients are real, and reverses the charge; so the block of charge $-l-2$ has the same entry as the twist's.
""")+r"""Since $\rho_0$ is diagonal and""")
rep(r"""definite after the shift $\lambda_t(j)+\eta$, as described above,""", r"""definite after the shift $\lambda_t(j)+\eta$, as described above"""+I(r""" ($\eta>0$ plays the role of $\delta$ there, chosen per block and slice as half the double-precision gap)""")+",")
rep(r"""$\omega^2:=\sum_i\norm{\Omega_i}^2=76/3$.""", r"""$\omega^2:=\sum_i\norm{\Omega_i}"""+I("_2")+r"""^2=76/3$"""+I(" (operator norms)")+".")
rep(r"""  \item \emph{Factorization.} $M$ is factored without pivoting in $200$-bit arithmetic,
    $LDL^\top=M+\Delta M$, with $|\Delta M|\le\gamma_N|L||D||L^\top|$, $\gamma_N=Nu/(1-Nu)$,
    $u=2^{-200}$: the componentwise bound for Gaussian elimination~\cite[Thm.~9.3]{higham2002}, applied
    to the symmetric elimination.
  \item \emph{Inertia.} $LDL^\top$ is congruent to $D$, so by Sylvester's law $M+\Delta M$ has exactly as
    many negative eigenvalues as $D$ has negative pivots, and none equal to zero. Every pivot is recorded.
  \item \emph{Perturbation.} Let $\varepsilon=\rho+\norm{\gamma_N|L||D||L^\top|}_F$\redins{, an upper bound
    evaluated in interval arithmetic (\path{scripts/ricci_su2_outward.py})}.""",
r"""  \item \emph{Factorization.} $M$ is factored without pivoting in $200$-bit arithmetic"""+D(r""",
    $LDL^\top=M+\Delta M$, with $|\Delta M|\le\gamma_N|L||D||L^\top|$, $\gamma_N=Nu/(1-Nu)$,
    $u=2^{-200}$: the componentwise bound for Gaussian elimination, applied
    to the symmetric elimination""")+I(r""". The stored factors, $L$ unit lower triangular and $D$ diagonal, define an
    exact matrix $LDL^\top$. Its distance from $M$ is bounded a posteriori by $\norm{M-LDL^\top}_F$, evaluated
    in interval arithmetic with outward rounding, so no rounding-error constant of the factorization
    enters""")+r""".
  \item \emph{Inertia.} $LDL^\top$ is congruent to $D$, so by Sylvester's law """+D(r"$M+\Delta M$")+I(r"$LDL^\top$")+r""" has exactly as
    many negative eigenvalues as $D$ has negative pivots, and none equal to zero. Every pivot is recorded.
  \item \emph{Perturbation.} Let $\varepsilon=\rho+"""+D(r"\norm{\gamma_N|L||D||L^\top|}_F")+I(r"\norm{M-LDL^\top}_F")+r"""$\redins{, an upper bound
    evaluated in interval arithmetic (\path{scripts/ricci_su2_outward.py})}.""")
rep(r"""than $0.63$, and $\varepsilon<3\times10^{-54}$ (Table""", r"""than $0.63$, and $\varepsilon<"""+D(r"3\times10^{-54}")+I(r"7\times10^{-56}")+r"""$ (Table""")
rep(r"""realified matrix; $\rho$: enclosure radius; backward: $\norm{\gamma_N|L||D||L^\top|}_F$;
$\varepsilon=\rho+{}$backward. At $\mu=\lambda(j)-1/3$ no sector has a negative pivot,
$\varepsilon\le2.1\times10^{-54}$ and""", r"""realified matrix; $\rho$: enclosure radius; """+D(r"""backward: $\norm{\gamma_N|L||D||L^\top|}_F$;
$\varepsilon=\rho+{}$backward""")+I(r"residual: $\norm{M-LDL^\top}_F$; $\varepsilon=\rho+{}$residual")+r""". At $\mu=\lambda(j)-1/3$ no sector has a negative pivot,
$\varepsilon\le"""+D(r"2.1\times10^{-54}")+I(r"6.2\times10^{-56}")+r"""$ and""")
rep(r"""$j$ & $\lambda(j)$ & $N$ & $\rho$ & backward & $\varepsilon$""", r"""$j$ & $\lambda(j)$ & $N$ & $\rho$ & """+D("backward")+I("residual")+r""" & $\varepsilon$""")
for o,n in ((r"& $9.3\times10^{-58}$ & $1.2\times10^{-57}$ &", r"& $"+D(r"9.3\times10^{-58}")+I(r"5.5\times10^{-59}")+r"$ & $"+D(r"1.2\times10^{-57}")+I(r"3.5\times10^{-58}")+r"$ &"),
            (r"& $3.2\times10^{-55}$ & $3.3\times10^{-55}$ &", r"& $"+D(r"3.2\times10^{-55}")+I(r"6.3\times10^{-57}")+r"$ & $"+D(r"3.3\times10^{-55}")+I(r"1.4\times10^{-56}")+r"$ &"),
            (r"& $1.4\times10^{-54}$ & $1.4\times10^{-54}$ &", r"& $"+D(r"1.4\times10^{-54}")+I(r"1.9\times10^{-56}")+r"$ & $"+D(r"1.4\times10^{-54}")+I(r"4.3\times10^{-56}")+r"$ &"),
            (r"& $2.6\times10^{-54}$ & $2.7\times10^{-54}$ &", r"& $"+D(r"2.6\times10^{-54}")+I(r"2.9\times10^{-56}")+r"$ & $"+D(r"2.7\times10^{-54}")+I(r"6.6\times10^{-56}")+r"$ &")):
    rep(o,n)
rep(r"""midpoints, and $\rho$, the backward-error term and $\varepsilon$ are evaluated with outward rounding}.""",
    r"""midpoints, and $\rho$, the residual $\norm{M-LDL^\top}_F$ and $\varepsilon$ are evaluated with outward rounding}.""")
rep(r"""All programs and certificates are in the \textsc{GIGI} repository,
\url{https://github.com/nurdymuny/gigi},""", r"""All programs and certificates are in the \textsc{GIGI} repository,
\url{https://github.com/nurdymuny/gigi}"""+I(r""" (tag \texttt{floor-theorem-v2}; the scripts are on that tag, not yet on the main branch)""")+",")
open(p,"w").write(s); print("ok")
