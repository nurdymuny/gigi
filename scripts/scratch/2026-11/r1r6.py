p='paper.tex'; s=open(p).read()
def rep(old,new):
    global s
    n=s.count(old); assert n==1,(n,old[:80]); s=s.replace(old,new)
rep(r"""\paragraph{Why the same fit applies.} In Fock--Schwinger gauge at the cell's centre,
$A(0)=0$ and $dF=-[A\wedge F]$ vanishes at the centre, entering only at the next order. To
the order of a linear model, each $\mathfrak{su}(2)$ component of $F$ is therefore a closed
linear field. The paper's fit map $\cR_K$ depends only on geometry, so it applies unchanged
to the three components of the registered fluxes. Amplification and conditioning are
exactly those of Section~\ref{sec:conditioning}.""",
r"""\paragraph{Why the same fit applies, to leading order.} In Fock--Schwinger gauge at the cell's
centre, $A(0)=0$ and $dF=-[A\wedge F]$ vanishes at the centre, entering only at the next order. To
the order of a linear model, each $\mathfrak{su}(2)$ component of $F$ is therefore a closed
linear field. The paper's fit map $\cR_K$ depends only on geometry, so it is applied unchanged
to the three components of the registered fluxes. Amplification and conditioning are
exactly those of Section~\ref{sec:conditioning}. This is a leading-order argument, and it is
used as one. Register runs along a fixed breadth-first tree, which agrees with radial
Fock--Schwinger transport only to leading order.

\paragraph{The own faces in $SU(2)$.} In the Abelian case the cell's own faces are held exactly.
In $SU(2)$ they cannot be. The four own-face fluxes of a closed linear field sum to zero (its flux
map has rank three), but the Bianchi identity of a lattice field is multiplicative: neither the
logarithms nor the vector parts of the registered face holonomies sum to zero on a finite cell. No
closed linear field matches all four readings, and the fit, whose own-face step is a
pseudoinverse, matches them in least squares instead. We measured the residual on every cell
(\path{scripts/ricci_su2_ownface_residual.py}). It equals, to rounding, the smallest residual
any closed linear field can achieve, the component of the readings along the flux map's null
direction, and it falls like $h^2$: its median over cells is $2.53\%$, $0.60\%$ and $0.27\%$
of the own readings at $n=1,2,3$ (maxima $4.3\%$, $0.82\%$, $0.37\%$), the same for both
readings of the face. So in $SU(2)$ the own faces are held exactly only in the Abelian sense; the
non-Abelian readings are held up to this residual, a relative perturbation of order $h^2$ in the
data. The convergence results below include it; they do not rest on exact preservation.""")
rep(r"""Here $D_i=U_i-1$ is the defect of face $i$'s holonomy registered at vertex $0$, and $M$ is the
metric Whitney face mass.""",
r"""Here $D_i=U_i-1$ is the defect of face $i$'s holonomy registered at vertex $0$, and $M$ is the
metric Whitney face mass.""")
rep(r"""The recovery and the native action read each face's turn differently. The recovery
reads it by its arc, $\log U_f$; the native action reads it by its chord, $U_f-1$, which is
Wilson's. The companion paper~\cite{threeways} works out what separates the two readings,
together with a third, the tangent. On these meshes the face angles are at most $0.094$,
and the relative chord--arc energy gap $a^2/12$ is below $0.08\%$ at every face, far below
the errors reported here. Mixing the two readings is therefore not a source of the
measured error.""",
r"""\paragraph{Which reading of the face.} In the force both steps read each face by its chord,
Wilson's $U_f-1$: the action through the defect, and the recovery through the defect's vector
part (\texttt{flux="defect"} in the code). The recovery tests of Section~\ref{sec:su2rec} read
the arc, $\log U_f$. The companion paper~\cite{threeways} works out what separates the readings,
together with a third, the tangent. On these meshes the face angles are at most $0.094$, and the
chord--arc energy gap $a^2/12$ is below $0.08\%$ at every face. A small energy gap does not by
itself bound the gap between forces, which are derivatives (for one face the ratio of the two
derivatives is $\sin a/a$, not the energy ratio), so we measured the force directly
(\path{scripts/ricci_su2_force_log_vs_defect.py}). With the arc in the recovery instead of the
chord, the two-ring force changes by $0.090\%$, $0.0084\%$ and $0.0021\%$ of its norm at
$n=1,2,3$, and its error by at most $0.011$ percentage points ($36.25\%$, $7.43\%$, $4.69\%$
either way). The choice of reading is not a source of the measured error.""")
rep(r"""The same reading carries over to
$SU(2)$, where it converges""",
r"""The same reading carries over to
$SU(2)$, with the own faces held up to a non-Abelian residual that falls like $h^2$, where it
converges""")
open(p,'w').write(s); print('ok')
