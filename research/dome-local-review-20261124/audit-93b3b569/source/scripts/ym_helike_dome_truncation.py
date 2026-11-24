"""Finite-reference L2 projection diagnostics; not an infinite-tail certificate."""
from fractions import Fraction as F
import math

import mpmath as mp

import ym_helike_j0 as j0
from ym_helike_dome import ONE, Moments, _mpf, dome_polynomial, mono_mul


def validate(N, g2, alpha, beta, orders, K_ref, m, dps):
    for name, value, lo, hi in (("N",N,0,32),("K_ref",K_ref,0,80),("dps",dps,40,200)):
        if type(value) is not int or not lo <= value <= hi:
            raise ValueError(f"{name} must be an integer in [{lo}, {hi}]")
    for name, value in (("g2",g2),("alpha",alpha),("beta",beta)):
        if isinstance(value,bool) or not isinstance(value,(int,float)) or not math.isfinite(value) or value <= 0:
            raise ValueError(f"{name} must be finite and positive")
    if not orders or any(type(k) is not int or not 0 <= k <= K_ref for k in orders):
        raise ValueError("K orders must satisfy 0 <= K <= K_ref")
    if len(set(orders)) != len(orders):
        raise ValueError("K orders must be distinct")
    if len(m) != 3 or any(type(k) is not int or k < 0 for k in m):
        raise ValueError("m must contain three nonnegative integer exponents")


def checked_remainder(norm2, projected2, dps, label):
    if not all(mp.isfinite(x) for x in (norm2, projected2)) or norm2 < 0 or projected2 < 0:
        raise ArithmeticError(f"{label}: invalid nonfinite or negative norm data")
    raw = norm2 - projected2
    tol = mp.mpf(10) ** (-(dps-20)) * max(abs(norm2),abs(projected2),1)
    if raw < -tol:
        raise ArithmeticError(f"{label}: materially negative projected squared norm")
    value = max(raw, mp.mpf(0))
    return value, dict(raw=mp.nstr(raw,dps),tolerance=mp.nstr(tol,dps),roundoff_clipped=bool(raw < 0))


def _rows(meta, S, reference, tails):
    dps = meta['dps']
    try:
        Li = mp.inverse(mp.cholesky(S))
    except (ValueError,ZeroDivisionError) as exc:
        raise ArithmeticError("origin Gram matrix is unresolved or not positive definite") from exc

    def exterior(item,label):
        norm2 = mp.mpf(item['norm2'])
        b = mp.matrix(item['overlaps'])
        if b.rows != S.rows or not all(mp.isfinite(x) for x in b):
            raise ArithmeticError("invalid overlap vector")
        y = Li*b
        projected = mp.fsum(x*x for x in y)
        value,diag = checked_remainder(norm2,projected,dps,label)
        return norm2,value,diag

    full2, pf2, ref_diag = exterior(reference,'reference')
    if full2 <= 0 or pf2 <= mp.mpf(ref_diag['tolerance']):
        raise ArithmeticError("outside-origin reference norm is unresolved at this precision")
    rows = []
    for K in meta['orders']:
        t2,pt2,tail_diag = exterior(tails[str(K)],'tail')
        row = {k:v for k,v in meta.items() if k != 'orders'}
        row.update(K=K,reference_is_finite=True,
            outside_fraction_of_dome=float(pf2/full2),total=float(mp.sqrt(t2/full2)),
            outside=float(mp.sqrt(pt2/pf2)),
            projection_diagnostics=dict(reference=ref_diag,tail=tail_diag))
        if not all(math.isfinite(row[k]) for k in ('outside_fraction_of_dome','total','outside')):
            raise ArithmeticError("nonfinite diagnostic output")
        rows.append(row)
    return rows


def truncation_sweep(N,g2,alpha,beta,orders,K_ref,m=ONE,dps=80,progress=None):
    """Share the reference and origin Gram across a batch of truncation orders."""
    orders = list(orders)
    validate(N,g2,alpha,beta,orders,K_ref,m,dps)
    meta = dict(N=N,g2=float(g2),alpha=float(alpha),beta=float(beta),orders=orders,
                K_ref=K_ref,m=list(m),dps=dps)
    with mp.workdps(dps):
        g = mp.sqrt(_mpf(F(g2).limit_denominator(10**12)))
        al = _mpf(F(alpha).limit_denominator(10**12))
        be = _mpf(F(beta).limit_denominator(10**12))
        Mo = Moments()
        orig = [{mm:mp.mpf(1)} for mm in j0.basis(N)]
        S = mp.matrix(len(orig),len(orig))
        for i in range(len(orig)):
            for j in range(i,len(orig)):
                S[i,j] = S[j,i] = Mo.inner(orig[i],al,orig[j],al)
        def evidence(f):
            return dict(norm2=mp.nstr(Mo.inner(f,be,f,be),dps),
                        overlaps=[mp.nstr(Mo.inner(o,al,f,be),dps) for o in orig])
        ref = {mono_mul(m,k):v for k,v in dome_polynomial(K_ref,be/g).items()}
        reference = evidence(ref)
        if progress:
            progress('reference_ready',meta=meta,reference=reference)
        tails = {}
        for K in orders:
            low = {mono_mul(m,k):v for k,v in dome_polynomial(K,be/g).items()}
            tail = {k:ref.get(k,0)-low.get(k,0) for k in ref.keys()|low.keys() if ref.get(k,0)!=low.get(k,0)}
            tails[str(K)] = evidence(tail)
            if progress:
                progress('tail_ready',K=K,evidence=tails[str(K)])
        gram = [[mp.nstr(S[i,j],dps) for j in range(S.cols)] for i in range(S.rows)]
        out = dict(meta=meta,origin_gram=gram,reference=reference,tails=tails)
        out['rows'] = _rows(meta,S,reference,tails)
        return out


def replay_batch(batch):
    meta = batch['meta']
    validate(meta['N'],meta['g2'],meta['alpha'],meta['beta'],meta['orders'],meta['K_ref'],meta['m'],meta['dps'])
    n = len(j0.basis(meta['N']))
    if len(batch['origin_gram']) != n or any(len(r)!=n for r in batch['origin_gram']):
        raise ValueError("wrong origin Gram dimensions")
    if set(batch['tails']) != set(map(str,meta['orders'])):
        raise ValueError("incomplete tail evidence")
    with mp.workdps(meta['dps']):
        S = mp.matrix(batch['origin_gram'])
        if not all(mp.isfinite(x) for x in S) or S != S.T:
            raise ValueError("invalid origin Gram")
        rows = _rows(meta,S,batch['reference'],batch['tails'])
    if len(rows) != len(batch['rows']):
        raise ValueError("incomplete diagnostic rows")
    for a,b in zip(rows,batch['rows']):
        for k,v in a.items():
            if k in ('outside_fraction_of_dome','total','outside'):
                if not math.isclose(v,b[k],rel_tol=1e-10,abs_tol=1e-30):
                    raise ValueError("diagnostic differs from overlap replay: "+k)
            elif k != 'projection_diagnostics' and v != b[k]:
                raise ValueError("diagnostic metadata changed: "+k)
        for label in ('reference','tail'):
            if a['projection_diagnostics'][label]['roundoff_clipped'] != b['projection_diagnostics'][label]['roundoff_clipped']:
                raise ValueError("roundoff disclosure changed")
            with mp.workdps(meta['dps']):
                for field in ('raw','tolerance'):
                    expected=mp.mpf(a['projection_diagnostics'][label][field])
                    actual=mp.mpf(b['projection_diagnostics'][label][field])
                    tol=max(abs(expected),mp.mpf(a['projection_diagnostics'][label]['tolerance']))*mp.mpf('1e-10')
                    if not mp.isfinite(actual) or abs(actual-expected)>tol:
                        raise ValueError('cancellation disclosure differs from replay')
    return rows


def truncation_diagnostic(N,g2,alpha,beta,K,K_ref,m=ONE,dps=80):
    return truncation_sweep(N,g2,alpha,beta,[K],K_ref,m,dps)['rows'][0]
