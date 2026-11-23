import sys
p = "docs/papers/berger-spectrum/paper.tex"
s = open(p).read()
def rep(o, n):
    global s
    assert s.count(o) == 1, ("count", s.count(o), o[:80])
    s = s.replace(o, n)

# --- abstract
rep(r"""$[j(j+2)-2(t-3)j-2(t-2)(t+4)]/t$. An interval-in-$t$ version of the certificate proves that it is the floor of
its sector for every $t\in[2.75,10]$ in the sectors $j\le12$, $20$ and $24$. At $t=4$""",
r"""$[j(j+2)-2(t-3)j-2(t-2)(t+4)]/t$. \reddel{An interval-in-$t$ version of the certificate proves that it is the floor of
its sector for every $t\in[2.75,10]$ in the sectors $j\le12$, $20$ and $24$.}\redins{It is the floor of every sector
for every $t\in[2.75,10]$, in particular at $t=3$: a conserved charge splits each sector into blocks of size at most
nine, interval certificates of the blocks cover $j<100$, and a closed-form Gershgorin bound covers $j\ge100$. The
eigenvector is a lowest Landau level: it is annihilated by the transverse raising operator, and its internal
frame is the lowest eigenvector of the curvature term of the Weitzenb\"ock identity.} At $t=4$""")

# --- main results bullets
rep(r"""its sector is proved for $j=0,12,20,24$ (Theorem~\ref{thm:gap}) and checked in double precision for
    $j\le120$; for general $j$ it is not proved.""",
r"""its sector is proved for $j=0,12,20,24$ (Theorem~\ref{thm:gap})\reddel{ and checked in double precision for
    $j\le120$; for general $j$ it is not proved}\redins{, with a gap of $1/3$, and for every $j$ by
    Theorem~\ref{thm:range}}.""")
rep(r"""(Theorem~\ref{thm:t4}) and for every $t\in[2.75,10]$ in fifteen sectors at once (Theorem~\ref{thm:range}),
    with the stretches""",
r"""(Theorem~\ref{thm:t4}) and for every $t\in[2.75,10]$ in \reddel{fifteen sectors at once}\redins{every sector}
    (Theorem~\ref{thm:range})\redins{; its eigenvector is a lowest Landau level
    (Proposition~\ref{prop:landau})},
    with the stretches""")

# --- remark after the certificate table
rep(r"""this. Minimality for $j>120$ is not needed for the gap and is not proved.""",
r"""this. Minimality for $j>120$ is not needed for the gap\reddel{ and is not proved}\redins{; it holds for every $j$
by Theorem~\ref{thm:range} at $t=3$}.""")

# --- boxed formula paragraph
rep(r"""floor of every sector below $j=30$ that survives on the Poincar\'e sphere, and bounds every higher
sector.""",
r"""floor of every sector below $j=30$ that survives on the Poincar\'e sphere, and bounds every higher
sector\redins{; Theorem~\ref{thm:range} shows that it is the floor of every sector}.""")

# --- Landau proposition after the proof of prop:family
rep(r"""orthogonality to every gauge direction) hold as identities in $j$ and $t$
(\path{scripts/ricci_su2_sector_formula_squashed.py}).
\end{proof}
""",
r"""orthogonality to every gauge direction) hold as identities in $j$ and $t$
(\path{scripts/ricci_su2_sector_formula_squashed.py}).
\end{proof}

\redins{The Weitzenb\"ock identity of Lemma~\ref{lem:weitz} holds on every Berger sphere, and it explains the
shape of the floor law.}

\redins{\begin{proposition}[The twist is a lowest Landau level]\label{prop:landau}
Let $t>0$.
\begin{enumerate}
  \item In every sector, $H+GG^*=\sum_iX_i^*X_i+R_0(t)$ with $X_i=\rho_i/\sqrt{g_i}+\Omega_i$, where $R_0(t)$ is a
    constant real symmetric $9\times9$ matrix with eigenvalues $12-8t$ $(\times2)$, $4(t-1)$,
    $-2\bigl(\sqrt{2t^2-2t+1}-1\bigr)$ $(\times2)$, $2\bigl(\sqrt{2t^2-2t+1}+1\bigr)$ $(\times2)$ and
    $3t-2\pm\sqrt{9t^2-4t+4}$. For $t\ge12/7$, $12-8t$ is the lowest of them.
  \item Write $b=f\otimes\beta$, with $\beta$ the internal frame $(\hat\theta^1+i\hat\theta^2)\otimes(\tau_1+i\tau_2)$,
    and $X_\pm=X_1\pm iX_2$. Then
    \[
      X_+b=0,\qquad X_0b=i\kappa\,b,\quad \kappa=\frac{2t-4-j}{\sqrt t},\qquad R_0(t)\beta=(12-8t)\beta .
    \]
  \item Consequently, in the identity of (1), the fibre term of $b$ is $\norm{X_0b}^2=\kappa^2$, the curvature term
    is $12-8t$, and the transverse term is $\norm{X_1b}^2+\norm{X_2b}^2=2(j+t)$:
    \[
      \lambda_t(j)=\kappa^2+2(j+t)+(12-8t).
    \]
\end{enumerate}
\end{proposition}
\begin{proof}
(1) The proof of Lemma~\ref{lem:weitz} with $t$ symbolic: every coefficient of degree one and two agrees in
$U(\su)$, identically in $t$ (\path{scripts/ricci_su2_weitzenbock_symbolic_t.py}). The eigenvalues are those of
the symbolic $R_0(t)$, and the comparison is elementary. (2) $(\rho_1+i\rho_2)f=0$ because $f$ is the top weight,
and $(\Omega_1+i\Omega_2)\beta=0$, $\Omega_0\beta=i(2t-4)\beta/\sqrt t$ and $R_0(t)\beta=(12-8t)\beta$ hold as
identities in $t$; $\rho_0f=-ij f$ (\path{scripts/ricci_su2_landau_symbolic.py}). (3) follows from (1), (2) and
Proposition~\ref{prop:family}.
\end{proof}}

\redins{\begin{remark}
In the language of a charged particle in a magnetic field, $X_+b=0$ is the lowest-Landau-level condition for the
two transverse covariant derivatives, $\kappa$ is the momentum along the fibre, and $12-8t$ is the most negative
curvature the internal frame can see. The momentum vanishes at $j=2t-4$, which is why the softest sector sits near
$j=t-4$ only after the transverse and curvature terms are added (Corollary~\ref{cor:family}). The statement that
$b$ is the \emph{floor} is not implied by (2): it needs every other state of the sector to cost more, which is
Theorem~\ref{thm:range}.
\end{remark}}
""")

# --- the range theorem: charge lemma + new statement
rep(r"""\begin{theorem}[The floor law on $2.75\le t\le10$]\label{thm:range}
For every $t\in[2.75,10]$ and every $j\in\{0,1,\dots,12,20,24\}$, $\lambda_t(j)$ is the lowest eigenvalue of
$H+GG^*$ in the sector of spin $j/2$, with multiplicity exactly two, and the lowest gauge-orthogonal eigenvalue
there.
\end{theorem}
\begin{proof}
$1{,}091$ slices tile $[2.75,10]$ exactly in each of the fifteen sectors, the finest $0.002$ wide near
$t=2.75$, where the gap above the floor is smallest ($0.16$ for $j=4$, $0.18$ for $j=1$). Every slice has
exactly four negative pivots and $\varepsilon<\delta$, the smallest margin $\delta-\varepsilon$ being $0.0018$.
A degenerate slice at $t=3$ reproduces Table~\ref{tab:cert}
(\path{scripts/ricci_su2_floor_interval_cert.py}, receipt
\path{research/su2-floor-family-20261119/interval_cert_2.75_10.json}).
\end{proof}""",
r"""\redins{\paragraph{Charge blocks.} The rotation about the long axis, acting at once on the sector, the frame index
and the Lie index, is a symmetry of the Berger field. Its charge makes the certificate independent of the size of
the sector. Take $\rho_0,\rho_1,\rho_2=-2iJ_z,-2iJ_x,-2iJ_y$ on $V_j$ (equivalent to the representation used so
far: same brackets), with weights $m\in\{-l,\dots,l\}$, $l=j/2$, and take the frame and Lie indices in the
eigenbasis of the rotation, with charges $c_f,c_l\in\{-1,0,1\}$.}

\redins{\begin{lemma}[Charge blocks]\label{lem:charge}
Let $t>0$ and $c=c_f+c_l$. In this basis $\Omega_0=i\operatorname{diag}\bigl(c(t-2)/\sqrt t\bigr)$, $\Omega_+$
raises $c$ by exactly one, $\Omega_-$ lowers it by exactly one, every nonzero entry of $\Omega_\pm$ has modulus
$\sqrt{2t}$, and $R_0(t)$ preserves $c$. Hence $H+GG^*$ preserves the charge $q=m+c$, and the sector of spin $j/2$
is the orthogonal sum of the charge blocks, each spanned by the at most nine states $(m,c_f,c_l)$ with
$m=q-c_f-c_l$, $|m|\le l$. The state of charge $l+2$ is the twist $b$ alone, and the state of charge $-l-2$ its
conjugate.
\end{lemma}
\begin{proof}
Exact, with $t$ symbolic (\path{scripts/ricci_su2_charge_selection_symbolic.py}). Since $\rho_0$ is diagonal and
$\rho_1\pm i\rho_2=-2iJ_\pm$ shifts $m$ by $\pm1$, each of $X_0$ and $X_\pm$ shifts $q$ by $0$ or $\pm1$, and
$\sum_iX_i^*X_i=X_0^*X_0+\tfrac12(X_+^*X_++X_-^*X_-)$ preserves $q$; so does $R_0(t)$.
\end{proof}}

\begin{theorem}[The floor law on $2.75\le t\le10$]\label{thm:range}
For every $t\in[2.75,10]$ and every \reddel{$j\in\{0,1,\dots,12,20,24\}$}\redins{$j\ge0$}, $\lambda_t(j)$ is the
lowest eigenvalue of $H+GG^*$ in the sector of spin $j/2$, with multiplicity exactly two, and the lowest
gauge-orthogonal eigenvalue there.
\end{theorem}
\begin{proof}
\redins{By Lemma~\ref{lem:charge} it suffices to show that every charge block other than the two twist blocks has
all its eigenvalues above $\lambda_t(j)$; the two twist blocks are $1\times1$ with entry $\lambda_t(j)$
(Proposition~\ref{prop:family}).}

\redins{\emph{Sectors $j\le99$.} Each block is enclosed for all $t$ in a slice at once and certified positive
definite after the shift $\lambda_t(j)+\eta$, as described above, with no negative pivot and $\varepsilon<\eta$
(\path{scripts/ricci_su2_block_interval_cert.py}). For $j\le40$, $12{,}596$ slices, smallest margin
$\eta-\varepsilon=1.7\times10^{-5}$ (receipt \path{block_cert_j40.json}); for $41\le j\le99$, @@J4199@@ (receipt
\path{block_cert_j41_99.json}).}

\redins{\emph{Sectors $j\ge100$.} Write $m=l-x$, $0\le x\le l$ (the map $(m,c_f,c_l)\mapsto(-m,-c_f,-c_l)$ preserves
everything below). For the state of type $(c_f,c_l)$, the diagonal entry of its block minus $\lambda_t(j)$ minus
the moduli of its off-diagonal entries is, identically in $l$, $x$ and $t$,
\[
  G=4x(2l-x)\Bigl(1-\frac1t\Bigr)+\frac{4l(2-c)(t-2)}{t}+E-2\sqrt{2t}\Bigl[n_-\sqrt{x(2l-x+1)}+n_+\sqrt{(2l-x)(x+1)}\Bigr],
\]
with $E=c^2(t-2)^2/t+4c(t-2)x/t+C_a-R_a+2(t-2)(t+4)/t$, where $C_a$ is the diagonal entry of
$-(\Omega_1^2+\Omega_2^2)+R_0(t)$, $R_a$ the sum of the moduli of its other entries in the same row, and $n_\pm$ the
numbers of nonzero entries of $\Omega_\pm$ in the column of the type
(\path{scripts/ricci_su2_charge_selection_symbolic.py}; the nine types are tabulated in
\path{scripts/ricci_su2_floor_large_j.py}). If $G>0$ on every row except the twist, every other block lies above
$\lambda_t(j)$ by Gershgorin's theorem. For $l\ge50$ and $t\in[2.75,10]$ this is proved by bisection in interval
arithmetic: for $x\le20$ on $(1/l,t)$, $G/l>6\times10^{-4}$ ($373$ boxes); for $x\ge20$, after dividing by $lx$, on
the relaxed box $(x/l,1/l,1/x,t)\in[0,1]\times[0,1/50]\times[0,1/20]\times[2.75,10]$, $G/(lx)>0.85$ ($9$ boxes)
(receipt \path{floor_large_j.json}).}

\redins{Receipts are in \path{research/su2-floor-family-20261119/}. Independently, a first certificate on the full
sector matrices of the fifteen sectors $j\le12$, $20$, $24$ agrees:} $1{,}091$ slices tile $[2.75,10]$ exactly in each of the fifteen sectors, the finest $0.002$ wide near
$t=2.75$, where the gap above the floor is smallest ($0.16$ for $j=4$, $0.18$ for $j=1$). Every slice has
exactly four negative pivots and $\varepsilon<\delta$, the smallest margin $\delta-\varepsilon$ being $0.0018$.
A degenerate slice at $t=3$ reproduces Table~\ref{tab:cert}
(\path{scripts/ricci_su2_floor_interval_cert.py}, receipt
\path{research/su2-floor-family-20261119/interval_cert_2.75_10.json}).
\end{proof}

\redins{At $t=3$ this completes Proposition~\ref{prop:bottom}: $(j(j+2)-14)/3$ is the floor of every sector of the
Berger field $(3,1,1)$, not only of those that survive on the Poincar\'e sphere.}""")

# --- remark tstar
rep(r"""In double precision, $\lambda_t(j)$ is the bottom of every sector $j\le40$ for every $t\in[2.7,10]$ tested.""",
r"""In double precision, $\lambda_t(j)$ is the bottom of every sector $j\le40$ for every $t\in[2.7,\reddel{10}\redins{2.75}]$ tested.""")

# --- discussion
rep(r"""rationally. Whether it is the \emph{floor} depends on $t$. It is the floor throughout $[2.75,10]$ in the
certified sectors, and""",
r"""rationally\redins{, and it is a lowest Landau level at every squashing (Proposition~\ref{prop:landau})}. Whether
it is the \emph{floor} depends on $t$. It is the floor throughout $[2.75,10]$ in \reddel{the
certified sectors}\redins{every sector}, and""")
rep(r"""\paragraph{Open questions.} Minimality for every $j$; the threshold""",
r"""\paragraph{Open questions.} \reddel{Minimality for every $j$}\redins{The floor outside $[2.75,10]$}; the threshold""")

# --- availability
rep(r"""  \item Theorem~\ref{thm:range}: \path{scripts/ricci_su2_floor_interval_cert.py};""",
r"""  \item Theorem~\ref{thm:range}: \path{scripts/ricci_su2_floor_interval_cert.py}\redins{,
    \path{scripts/ricci_su2_block_interval_cert.py} and \path{scripts/ricci_su2_floor_large_j.py}};
  \redins{\item Proposition~\ref{prop:landau}: \path{scripts/ricci_su2_weitzenbock_symbolic_t.py},
    \path{scripts/ricci_su2_landau_symbolic.py};}
  \redins{\item Lemma~\ref{lem:charge}: \path{scripts/ricci_su2_charge_selection_symbolic.py}, with the blocks in
    \path{scripts/ricci_su2_charge_block_direct.py} and their closed-form rows in
    \path{scripts/ricci_su2_block_rows_symbolic.py};}""")
open(p, "w").write(s); print("ok")
