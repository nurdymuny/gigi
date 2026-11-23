p='paper.tex'; s=open(p).read()
def rep(old,new):
    global s
    n=s.count(old); assert n==1,(n,old[:80]); s=s.replace(old,new)
rep(r"""so the only growth is the field's own motion:
over the window $[0,0.02]$ of the time-integration experiment, the envelope is
$e^{4.664\times0.02}\approx1.098$ for both forces.""",
r"""so the only growth is the field's own motion.
For the operator frozen at the initial Berger field, with the mass held fixed, the envelope over the
window $[0,0.02]$ of the time-integration experiment is $e^{4.664\times0.02}\approx1.098$ for both
forces. This is a bound for the frozen linearization only. Along the actual flow the mass $B(t)$
changes with the metric, and even the linear energy identity gains a term,
$\tfrac{d}{dt}\tfrac12\xi^\top B\xi=-\xi^\top\operatorname{sym}(K)\xi+\tfrac12\xi^\top\dot B\xi$;
linearizing along the moving trajectory adds others. The envelope therefore does not certify the
nonlinear moving-metric run; the agreement measured over that window (Section~\ref{sec:su2flow}) is a
separate, empirical result.""")
rep(r"""Proposition~\ref{prop:diss} does not apply here, and the envelope above is the honest bound.""",
r"""Proposition~\ref{prop:diss} does not apply here, and the frozen envelope above is the honest bound.""")
rep(r"""rule. Its linearisation is Crank--Nicolson, stable for any step, and every update stays
exactly on $SU(2)$.""",
r"""rule. Its linearisation is Crank--Nicolson, which is A-stable: stable for any step on the
dissipative modes. On the field's few growing modes ($\dot a=-\lambda a$ with $\lambda<0$) its factor
$(1-h\lambda/2)/(1+h\lambda/2)$ is singular at $h=-2/\lambda$, so the step must stay well below that;
the steps used here are at least a hundred times smaller. Every update stays exactly on $SU(2)$,
which is group membership, not a statement about accuracy or stability.""")
open(p,'w').write(s); print('ok')
