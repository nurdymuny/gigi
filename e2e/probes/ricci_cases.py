"""Declared Ricci fixtures and exact query construction, fixed before live runs."""
import itertools


def fixtures():
    cases = {
        "round_shrink": ([1.,1.,1.],0.2,16,False,1e-8),
        "round_normalized": ([1.,1.,1.],2.,8,True,1e-8),
        "berger_squash": ([0.25,1.,1.],0.05,256,False,1e-8),
        "berger_stretch": ([3.,1.,1.],0.05,256,False,1e-8),
        "triaxial": ([0.5,1.,2.],0.05,256,False,1e-8),
        "rounding": ([0.5,1.,2.],2.,1024,True,1e-8),
        "negative_scalar": ([5.,1.,1.],0.03,256,False,1e-8),
        "permuted": ([2.,0.5,1.],0.05,256,False,1e-8),
        "scaled": ([5.,10.,20.],0.5,256,False,1e-8),
    }
    for n in [16,32,64]:
        cases[f"refine_{n}"] = ([0.5,1.,2.],0.1,n,True,1e-3)
    for i,g in enumerate(itertools.product([0.5,1.,2.,5.], repeat=3)):
        cases[f"grid_{i}"] = (list(g),0.00001,4,False,1e-8)
    return cases


def query(case):
    g,t,n,normalized,tol = case
    # The GQL number lexer accepts decimal literals; avoid exponent notation.
    dec = lambda x: format(x,".15f").rstrip("0").rstrip(".")
    return (f"RICCI_FLOW SU2 METRIC ({','.join(dec(x) for x in g)}) "
            f"TIME {dec(t)} STEPS {n} " + ("NORMALIZED " if normalized else "")
            + f"TOLERANCE {dec(tol)};")


REFUSALS = {
    "extinction": ("RICCI_FLOW SU2 METRIC (1,1,1) TIME 0.25 STEPS 64;", "positive"),
    "past_extinction": ("RICCI_FLOW SU2 METRIC (1,1,1) TIME 0.3 STEPS 64;", "positive"),
    "underresolved": ("RICCI_FLOW SU2 METRIC (0.5,1,2) TIME 0.1 STEPS 2 NORMALIZED TOLERANCE 0.000000000001;", "estimate"),
    "invalid_metric": ("RICCI_FLOW SU2 METRIC (-1,1,1) TIME 0.1 STEPS 64;", "METRIC"),
    "work_cap": ("RICCI_FLOW SU2 METRIC (1,1,1) TIME 0.1 STEPS 4097;", "STEPS"),
    "unsupported_manifold": ("RICCI_FLOW TORUS METRIC (1,1,1) TIME 0.1 STEPS 64;", "SU2"),
}
