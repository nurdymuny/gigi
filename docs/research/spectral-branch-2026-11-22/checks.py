"""Independent recomputation of the spectral branch's finite-model numbers (my script, same method as the paper's
description; not the paper's own code). Writes checks_output.json beside itself."""
import itertools, json, math, os
import numpy as np
from collections import Counter, deque

out = {}

def latin_squares(n=4, blocks=False):
    perms = list(itertools.permutations(range(n)))
    sols = []
    def ok_block(rows):
        for br in range(0, len(rows) - len(rows) % 2, 2):
            for bc in (0, 2):
                s = {rows[br][bc], rows[br][bc+1], rows[br+1][bc], rows[br+1][bc+1]}
                if len(s) < 4: return False
        return True
    def rec(rows):
        if len(rows) == n:
            sols.append(tuple(v for r in rows for v in r)); return
        for p in perms:
            if all(p[c] != r[c] for r in rows for c in range(n)):
                nr = rows + [p]
                if blocks and len(nr) % 2 == 0 and not ok_block(nr): continue
                rec(nr)
    rec([])
    return sols

def ham(a, b): return sum(x != y for x, y in zip(a, b))

def graph(sols, dist=4):
    S = np.array(sols); n = len(S)
    D = (S[:, None, :] != S[None, :, :]).sum(-1)
    A = (D == dist).astype(float)
    return D, A

def diameter_and_components(A):
    n = len(A); nb = [np.nonzero(A[i])[0] for i in range(n)]
    comp = [-1]*n; c = 0; diam = 0
    for s in range(n):
        if comp[s] == -1:
            q = deque([s]); comp[s] = c
            while q:
                u = q.popleft()
                for w in nb[u]:
                    if comp[w] == -1: comp[w] = c; q.append(w)
            c += 1
    if c == 1:
        for s in range(n):
            d = [-1]*n; d[s] = 0; q = deque([s])
            while q:
                u = q.popleft()
                for w in nb[u]:
                    if d[w] == -1: d[w] = d[u] + 1; q.append(w)
            diam = max(diam, max(d))
    return c, diam

def analyse(name, sols):
    D, A = graph(sols)
    n = len(sols); deg = A.sum(1)
    L = np.diag(deg) - A
    w, V = np.linalg.eigh(L)
    comps, diam = diameter_and_components(A)
    hist = Counter(D[np.triu_indices(n, 1)].tolist())
    top = V[:, -1]; top = top * np.sign(top[np.argmax(np.abs(top))])
    vals = sorted(set(np.round(top, 10)))
    # Fiedler sweep Cheeger estimate (an upper bound on h_G)
    f = V[:, 1]; order = np.argsort(f); best = 1e9
    inS = np.zeros(n, bool)
    for i in range(n - 1):
        inS[order[i]] = True
        s = min(i + 1, n - i - 1)
        cut = A[inS][:, ~inS].sum()
        best = min(best, cut / s)
    r = {
        'n': n, 'edges': int(A.sum() / 2), 'min_nonzero_distance': int(min(k for k in hist if k > 0)),
        'distance_histogram': {str(k): v for k, v in sorted(hist.items())},
        'degrees': sorted(Counter(deg.astype(int).tolist()).items()),
        'components': comps, 'diameter': diam,
        'mu1': float(w[1]), 'mu1_multiplicity': int(np.sum(np.abs(w - w[1]) < 1e-8)),
        'mu2_distinct': float(sorted(set(np.round(w, 8)))[2]),
        'mu_max': float(w[-1]), 'top_eigvec_values': [float(v) for v in vals],
        'cheeger_sweep_upper_bound': float(best),
        'bipartite_by_top_sign': bool(all(np.sign(top[i]) != np.sign(top[j]) for i, j in zip(*np.nonzero(np.triu(A))))),
    }
    out[name] = r
    return r

shid = latin_squares(blocks=True)
lat = latin_squares(blocks=False)
r1 = analyse('shidoku', shid)
r2 = analyse('latin4', lat)

# Expected completion count N_4 per class from the decomposition (p(4,4) = 1/C(16,4))
for name, sols in [('shidoku', shid), ('latin4', lat)]:
    D, A = graph(sols); deg = A.sum(1)
    out[name]['N4_by_degree'] = {str(int(d)): 1 + d / math.comb(16, 4) for d in sorted(set(deg))}

# Graph counterexamples to two continuous statements applied to graphs
def barbell(m):
    n = 2 * m; A = np.zeros((n, n))
    A[:m, :m] = 1; A[m:, m:] = 1; np.fill_diagonal(A, 0); A[m-1, m] = A[m, m-1] = 1
    L = np.diag(A.sum(1)) - A; w = np.linalg.eigvalsh(L)
    _, diam = diameter_and_components(A)
    return float(w[1]), diam
bb = {}
for m in (10, 50, 100):
    mu1, diam = barbell(m)
    bb[str(m)] = {'mu1': mu1, 'diameter': diam, 'Csp': mu1 * diam**2, 'one_over_D': 1 / diam, 'pi2': math.pi**2}
out['barbell'] = bb

# Test C's relaxed systems: full solution counts and minimum distances
perms = list(itertools.permutations(range(4)))
row_only = 24**4
out['row_only_solution_count'] = row_only
# row + columns 0 and 1 distinct
cnt = 0
for rows in itertools.product(perms, repeat=4):
    if len({r[0] for r in rows}) == 4 and len({r[1] for r in rows}) == 4: cnt += 1
out['row_plus_col01_solution_count'] = cnt
out['row_only_min_distance'] = 2  # swapping two entries of one row is a valid move at Hamming distance 2

path = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'checks_output.json')
json.dump(out, open(path, 'w'), indent=1)
print(json.dumps(out, indent=1)[:4000])
