p='paper.tex'; s=open(p).read()
def rep(old,new):
    global s
    n=s.count(old); assert n==1,(n,old[:80]); s=s.replace(old,new)
rep(r"""We prove that the
bottom of the spin-$j/2$ sector of the linearised $SU(2)$ Yang--Mills operator at the Berger field
is $(j(j+2)-14)/3$. On the Poincar\'e sphere the first sector above the left-invariant one therefore
begins""",
r"""In the spin-$j/2$ sector of the linearised $SU(2)$ Yang--Mills operator at the Berger field,
$(j(j+2)-14)/3$ is an exact eigenvalue, and we prove that it is the bottom of its sector in the four low
sectors that survive on the Poincar\'e sphere. There the first sector above the left-invariant one
begins""")
rep(r"""derived on an explicit eigenvector and shown to
    be the bottom of its sector (Theorem~\ref{thm:gap}).""",
r"""an exact eigenvalue on an explicit eigenvector in
    every sector, proved to be the bottom of its sector for $j=0,12,20,24$, with every higher surviving
    sector bounded above $142$ (Theorem~\ref{thm:gap}); minimality for general $j$ is not proved.""")
rep(r"""exact: the bottom of every sector is $(j(j+2)-14)/3$, proved with certified computation, and on the
Poincar\'e sphere it is set by the sphere's first scalar eigenvalue.""",
r"""exact where it matters: $(j(j+2)-14)/3$ is an eigenvalue in every sector and, by certified computation,
the bottom of each low sector that survives on the Poincar\'e sphere, where the gap is set by the
sphere's first scalar eigenvalue.""")
rep(r"""\paragraph{The floor of every sector.} The spectrum has a closed form at its floor.""",
r"""\paragraph{The floor formula.} The spectrum has a closed form at its floor in the sectors that matter.""")
rep(r"""Proposition~\ref{prop:sector} gives an eigenvalue in every sector. The next result shows that it is
the bottom of its sector, and that this makes the gap exact.""",
r"""Proposition~\ref{prop:sector} gives an eigenvalue in every sector. The next result shows that it is
the bottom of its sector in the four low sectors that survive on the Poincar\'e sphere, and that this
makes the gap exact; for general $j$ minimality is not proved.""")
rep(r"""growth direction and the bottom of every higher sector are one formula""",
r"""growth direction and the bottom of each surviving sector below $j=30$ are one formula""")
rep(r"""    no other eigenvalue of $H+GG^*$ lies below $\lambda(j)+1/3$; numerically the same holds for
    every $j\le120$;""",
r"""    every other eigenvalue of $H+GG^*$ is at least $\lambda(j)+1/3-10^{-53}$; numerically the same
    picture holds for every $j\le120$;""")
a=r"For $j=0,12,20,24$ the remaining eigenvalues are certified by inertia."
b="The\neigenvector of Proposition~\\ref{prop:sector} lies in the slice, so $\\lambda(j)$ is the minimum there."
i=s.index(a); j=s.index(b)
s=s[:i]+r"""For $j=0,12,20,24$ the rest is a rigorous count of the eigenvalues below
$\mu=\lambda(j)+1/3$. The matrix $H+GG^*-\mu$ is enclosed in interval arithmetic (mpmath's \texttt{iv}
context: outward-rounded endpoints, $200$ bits, $\sqrt3$ and every entry enclosed) and realified, which
doubles every eigenvalue. The symmetric midpoint matrix $M$ of its lower triangle lies within $\rho$ of
the exact matrix; $M$ is factored $LDL^\top=M+\Delta M$ without pivoting, with
$|\Delta M|\le\gamma_N|L||D||L^\top|$~\cite[Thm.~9.3]{higham2002}. Since $LDL^\top$ is congruent to $D$,
Sylvester's law gives the exact inertia of $M+\Delta M$, and Weyl's inequality with
$\varepsilon=\rho+\norm{\gamma_N|L||D||L^\top|}_F$ transfers it: $k$ negative pivots allow at most $k/2$
eigenvalues of $H+GG^*$ below $\mu-\varepsilon$. In every case $D$ has four negative pivots, the
smallest pivot has absolute value at least $0.63$, and $\varepsilon<3\times10^{-54}$. So at most two
eigenvalues lie below $\lambda(j)+1/3-\varepsilon$; both copies of $\lambda(j)$ are among them, so
$\lambda(j)$ is the lowest eigenvalue, twice, and every other is at least $\lambda(j)+1/3-\varepsilon$.
At $\mu=\lambda(j)-1/3$ no pivot is negative, a consistency check. Every pivot is recorded, and the
script refuses to certify on a zero pivot, a different count, or $\varepsilon\ge1/3$; the companion
paper~\cite{davisberger} tabulates the data. """+s[j:]
open(p,'w').write(s)
print("ok")
