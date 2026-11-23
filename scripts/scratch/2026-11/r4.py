p='paper.tex'; s=open(p).read()
def rep(old,new):
    global s
    n=s.count(old); assert n==1,(n,old[:80]); s=s.replace(old,new)
rep(r"""On shape-regular meshes the curl of a Whitney variation obeys the inverse estimate
$\norm{W_{D\eta}}\le C_{\mathrm{inv}}h^{-1}\norm{W_\eta}$~\cite{hiptmair2002}.""",
r"""On a quasi-uniform family of meshes, shape-regular with $h_{\max}\le c\,h_{\min}$, the curl of a
Whitney variation obeys the inverse estimate
$\norm{W_{D\eta}}\le C_{\mathrm{inv}}h^{-1}\norm{W_\eta}$~\cite{hiptmair2002}; on a shape-regular
family alone it holds with the local sizes $h_K$ in place of $h$. The meshes used here are
quasi-uniform: $h_{\max}/h_{\min}\le1.14$ at every level.""")
rep(r"""\[
  \norm{P_K[\cR_K\Phi_F]-F}_{L^\infty(K)} \;\le\; C\,(1+\Lambda_K)\,h^2\,|F|_{W^{2,\infty}(\mathcal{N}(K))},
\]
where both fields are compared in $K$'s chart, and $C$ depends only on the shape-regularity
constant of the mesh and on the number of rings.""",
r"""\[
  \norm{P_K[\cR_K\Phi_F]-F}_{L^\infty(K)} \;\le\; C\,(1+\Lambda_K)\,h^2\,\norm{F}_{W^{2,\infty}(\mathcal{N}(K))},
\]
where both fields are compared in $K$'s chart, $\Lambda_K$ is measured in the scaled units of the
proof, and $C$ depends only on the shape-regularity constant of the mesh and on the number of rings.""")
rep(r"""On $\mathcal N(K)$, whose diameter is at most $c_1d$ for a fixed number of rings,""",
r"""On a conforming shape-regular mesh, cells that share a vertex have comparable diameters, with a
ratio bounded in terms of the shape-regularity constant; so $\mathcal N(K)$, for a fixed number of
rings, has diameter at most $c_1d$, and""")
rep(r"""of tangency, its derivatives up to second order are bounded by constants that again depend only
on shape regularity. So $|\tilde F|_{W^{2,\infty}}\le C|F|_{W^{2,\infty}}$, and the chart norm on $K$
is equivalent to the metric norm with constants of the same kind.""",
r"""of tangency, its derivatives up to second order are bounded by constants that again depend only
on shape regularity. Second derivatives of the pull-back mix derivatives of $F$ of orders zero to
two, so the bound is in the full norm, $|\tilde F|_{W^{2,\infty}}\le C\norm{F}_{W^{2,\infty}}$, and
the chart norm on $K$ is equivalent to the metric norm with constants of the same kind.""")
rep(r"""Combining Lemma~\ref{lem:acc} with~\eqref{eq:pgbound} gives a consistency error of order
$\Lambda h$, where $\Lambda=\max_K\Lambda_K$. Consistency requires only that $\Lambda$ be
bounded.""",
r"""Combining Lemma~\ref{lem:acc} with~\eqref{eq:pgbound} gives a consistency error of order
$\Lambda h$, where $\Lambda=\max_K\Lambda_K$. Consistency requires only that $\Lambda$ be
bounded uniformly over the mesh family. Full column rank on each patch does not give that; it is a
hypothesis, measured on the meshes computed here (Section~\ref{sec:conditioning}), not proved for a
general family. The convergence results below are finite-level evidence under it, not a theorem
about every refinement.""")
open(p,'w').write(s); print('ok')
