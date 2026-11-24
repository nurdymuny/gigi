"""SU(2) sphere matrix model. Exact invariant coefficients, CPU/GPU numerics.

No fitted potential, gauge penalty, diagonal shift, or omitted physical sector.
The registration and implementation note define conventions and acceptance.
"""

import itertools as it
import math
import time
from functools import lru_cache

import numpy as np
import scipy.linalg as la
import scipy.sparse as sparse
from threadpoolctl import threadpool_limits

UNITS = np.array([1 << (5*i) for i in range(9)], dtype=np.int64)
TOL = 1e-10


def multiply(a, b):
    out = {}
    for x, ax in a.items():
        for y, by in b.items():
            out[x+y] = out.get(x+y, 0) + ax*by
    return {x: v for x, v in out.items() if v}


def decode(codes):
    return ((np.asarray(codes)[:, None] >> (5*np.arange(9))) & 31).astype(np.int16)


def gram_polynomial(i, j):
    return {int(UNITS[3*i+a] + UNITS[3*j+a]): 1 for a in range(3)}


def determinant_polynomial():
    return {int(sum(UNITS[3*i+p[i]] for i in range(3))):
            (-1)**sum(p[i] > p[j] for i in range(3) for j in range(i+1, 3))
            for p in it.permutations(range(3))}


def terms(poly):
    return [(float(coef), tuple(i for i in range(9) for _ in range((code >> (5*i)) & 31)))
            for code, coef in poly.items() if coef]


def add_into(target, source, scale=1):
    for code, coef in source.items():
        target[code] = target.get(code, 0) + scale*coef


@lru_cache(maxsize=2)
def build_basis(cutoff):
    if type(cutoff) is not int or cutoff < 0 or cutoff > 12:
        raise ValueError("registered cutoff must be an integer from 0 through 12")
    grams = [gram_polynomial(i, j) for i in range(3) for j in range(i, 3)]
    det = determinant_polynomial()

    @lru_cache(None)
    def gram_product(indices):
        return multiply(gram_product(indices[:-1]), grams[indices[-1]]) if indices else {0: 1}

    polynomials, degrees = [], []
    for degree in range(cutoff+1):
        if degree == 1:
            continue
        odd = degree % 2
        count = (degree-3*odd)//2
        for indices in it.combinations_with_replacement(range(6), count):
            poly = gram_product(indices)
            polynomials.append(multiply(poly, det) if odd else poly)
            degrees.append(degree)
    codes = np.array(sorted(set().union(*(p.keys() for p in polynomials))), dtype=np.int64)
    states = decode(codes)
    factorials = np.array([math.factorial(k) for k in range(cutoff+1)], dtype=float)
    weights = np.sqrt(np.prod(factorials[states], axis=1) / 2.0**states.sum(axis=1))
    row, col, data = [], [], []
    for j, poly in enumerate(polynomials):
        indices = np.searchsorted(codes, list(poly))
        row.extend(indices)
        col.extend([j]*len(poly))
        data.extend(np.array(list(poly.values()))*weights[indices])
    c = sparse.csc_matrix((data, (row, col)), shape=(len(codes), len(polynomials)))
    norms = np.sqrt(np.asarray(c.power(2).sum(axis=0)).ravel())
    c = (c @ sparse.diags(1/norms)).tocsc()
    metric = (c.T @ c).toarray()
    return {"cutoff": cutoff, "codes": codes, "states": states, "C": c,
            "S": metric, "degrees": np.array(degrees)}


def transitions(states, codes, polynomial_terms):
    """Untruncated coordinate action: intermediate oscillator degrees are retained."""
    for coefficient, axes in polynomial_terms:
        for signs in it.product((-1, 1), repeat=len(axes)):
            values = np.full(len(codes), coefficient)
            offsets = np.zeros(9, dtype=np.int16)
            for axis, sign in zip(reversed(axes), signs):
                n = states[:, axis] + offsets[axis]
                values *= np.sqrt(np.maximum(n + int(sign > 0), 0)/2)
                offsets[axis] += sign
            valid = values != 0
            if np.any(valid):
                yield np.flatnonzero(valid), codes[valid] + int(offsets @ UNITS), values[valid]


def polynomial_matrix(basis, polynomial_terms):
    codes, states = basis["codes"], basis["states"]
    row, col, data = [], [], []
    for columns, targets, values in transitions(states, codes, polynomial_terms):
        indices = np.searchsorted(codes, targets)
        keep = (indices < len(codes)) & (codes[np.minimum(indices, len(codes)-1)] == targets)
        row.append(indices[keep]); col.append(columns[keep]); data.append(values[keep])
    if not row:
        return sparse.csr_matrix((len(codes), len(codes)))
    return sparse.coo_matrix((np.concatenate(data), (np.concatenate(row), np.concatenate(col))),
                             shape=(len(codes), len(codes))).tocsr()


def rotation(basis, axis, spatial=False):
    """Real infinitesimal rotations (the Hermitian generator is -i times this)."""
    codes, states = basis["codes"], basis["states"]
    a, b = (axis+1) % 3, (axis+2) % 3
    rows, cols, data = [], [], []
    for other in range(3):
        ia, ib = (3*a+other, 3*b+other) if spatial else (3*other+a, 3*other+b)
        for dest, src, sign in ((ia, ib, 1), (ib, ia, -1)):
            valid = states[:, src] > 0
            columns = np.flatnonzero(valid)
            targets = codes[valid] + UNITS[dest] - UNITS[src]
            indices = np.searchsorted(codes, targets)
            keep = (indices < len(codes)) & (codes[np.minimum(indices, len(codes)-1)] == targets)
            rows.extend(indices[keep]); cols.extend(columns[keep])
            data.extend(sign*np.sqrt(states[columns[keep], src].astype(np.float64)
                                     *(states[columns[keep], dest]+1)))
    return sparse.csr_matrix((data, (rows, cols)), shape=(len(codes), len(codes)))


def operator_polynomials():
    v4, scalar = {}, {}
    trace = {}
    for i in range(3):
        add_into(trace, gram_polynomial(i, i))
        for j in range(3):
            add_into(scalar, multiply(gram_polynomial(i, j), gram_polynomial(i, j)))
    add_into(scalar, multiply(trace, trace), -1/3)
    for i, j in it.combinations(range(3), 2):
        for a, b in it.combinations(range(3), 2):
            minor = {int(UNITS[3*i+a]+UNITS[3*j+b]): 1,
                     int(UNITS[3*i+b]+UNITS[3*j+a]): -1}
            add_into(v4, multiply(minor, minor), 0.5)
    tensor = []
    for coefficients in (
        ((1, 1, 1/np.sqrt(2)), (2, 2, -1/np.sqrt(2))),
        ((1, 2, np.sqrt(2)),), ((0, 1, np.sqrt(2)),), ((0, 2, np.sqrt(2)),),
        ((0, 0, -2/np.sqrt(6)), (1, 1, 1/np.sqrt(6)), (2, 2, 1/np.sqrt(6))),
    ):
        poly = {}
        for i, j, coef in coefficients:
            add_into(poly, gram_polynomial(i, j), coef)
        tensor.append(poly)
    return {"D": determinant_polynomial(), "V4": v4, "scalar": scalar,
            "trace": trace, "tensor": tensor}


def build_operators(basis):
    c = basis["C"]
    polynomials = operator_polynomials()
    project = lambda p: (c.T @ polynomial_matrix(basis, terms(p)) @ c).toarray()
    operators = {name: project(polynomials[name]) for name in ("D", "V4", "scalar", "trace")}
    operators["tensor"] = [project(p) for p in polynomials["tensor"]]
    operators["H0"] = (c.T @ sparse.diags(basis["states"].sum(axis=1)+4.5) @ c).toarray()
    operators["J2"] = np.zeros_like(basis["S"])
    gauss = []
    for axis in range(3):
        spatial = rotation(basis, axis, spatial=True) @ c
        operators["J2"] += (spatial.T @ spatial).toarray()
        colour = rotation(basis, axis) @ c
        gauss.append(float(sparse.linalg.norm(colour)))
    operators["gauss_norms"] = gauss
    if max(gauss) > TOL:
        raise ArithmeticError(f"Gauss constraint failed: {gauss}")
    return operators


def solve_generalized(h, s, device="cuda"):
    if device not in ("cpu", "cuda"):
        raise ValueError("device must be cpu or cuda; no automatic fallback")
    for matrix in (h, s):
        if matrix.dtype not in (np.dtype("float64"), np.dtype("complex128")):
            raise ValueError("float64 or complex128 required")
        if not np.isfinite(matrix).all():
            raise ValueError("finite inputs required")
        if la.norm(matrix-matrix.conj().T) > 1e-12*max(1., la.norm(matrix)):
            raise ValueError("Hermitian matrices required")
    started = time.perf_counter()
    if device == "cpu":
        try:
            with threadpool_limits(limits=1):
                energies, vectors = la.eigh(h, s, driver="gvd")
        except la.LinAlgError as exc:
            raise ValueError("positive definite Gram matrix required") from exc
        actual = "cpu"
    else:
        import torch
        if not torch.cuda.is_available():
            raise RuntimeError("CUDA unavailable; CPU fallback forbidden")
        if torch.cuda.mem_get_info()[0] < 1024**3 + 40*h.nbytes:
            raise MemoryError("insufficient GPU headroom")
        torch.backends.cuda.matmul.allow_tf32 = False
        torch.backends.cudnn.allow_tf32 = False
        with torch.inference_mode(), torch.autocast("cuda", enabled=False):
            a = torch.as_tensor(h, device="cuda")
            metric = torch.as_tensor(s, device="cuda")
            l, info = torch.linalg.cholesky_ex(metric)
            if int(info) != 0:
                raise ValueError("positive definite Gram matrix required by Cholesky")
            left = torch.linalg.solve_triangular(l, a, upper=False)
            white = torch.linalg.solve_triangular(l, left.mH, upper=False).mH
            if float(torch.linalg.norm(white-white.mH)) > 1e-10*max(1., float(torch.linalg.norm(white))):
                raise ArithmeticError("whitened operator lost Hermiticity")
            e, u = torch.linalg.eigh((white+white.mH)/2)
            v = torch.linalg.solve_triangular(l.mH, u, upper=True)
            torch.cuda.synchronize()
            energies, vectors = e.cpu().numpy(), v.cpu().numpy()
            actual = str(a.device)
    residual = float(np.max(la.norm(h@vectors-(s@vectors)*energies, axis=0)
                           / np.maximum(1., (la.norm(h)+np.abs(energies)*la.norm(s))*la.norm(vectors, axis=0))))
    orthogonality = float(la.norm(vectors.conj().T@s@vectors-np.eye(len(energies))))
    if not np.isfinite(energies).all() or residual > TOL or orthogonality > TOL:
        raise ArithmeticError(f"eigensolver validation failed: {residual=}, {orthogonality=}")
    return energies, vectors, {"device": actual, "residual": residual,
                              "metric_orthogonality": orthogonality,
                              "elapsed_seconds": time.perf_counter()-started}


def oscillator_codes(cutoff):
    def visit(axis, remaining, code):
        if axis == 9:
            yield code
            return
        for value in range(remaining+1):
            yield from visit(axis+1, remaining-value, code+value*int(UNITS[axis]))
    return np.array(sorted(visit(0, cutoff, 0)), dtype=np.int64)


@lru_cache(maxsize=1)
def independent_reference(cutoff, g2):
    """Complete oscillator space and Gauss kernel; no invariant-polynomial input."""
    if cutoff != 4:
        raise ValueError("independent dense oracle is bounded at N=4")
    codes = oscillator_codes(cutoff)
    states = decode(codes)
    pos = {int(code): i for i, code in enumerate(codes)}
    casimir = np.zeros((len(codes), len(codes)))
    for a, b in ((1, 2), (2, 0), (0, 1)):
        generator = np.zeros_like(casimir)
        for column, state in enumerate(states):
            for row in range(3):
                for dest, src, sign in ((3*row+a, 3*row+b, 1), (3*row+b, 3*row+a, -1)):
                    if state[src]:
                        target = int(codes[column]+UNITS[dest]-UNITS[src])
                        generator[pos[target], column] += sign*math.sqrt(int(state[src])*(int(state[dest])+1))
        casimir += generator.T@generator
    with threadpool_limits(limits=1):
        charges, rotations = la.eigh(casimir, driver="evd")
    physical = rotations[:, np.abs(charges) < 1e-10]
    if la.norm(casimir@physical) > TOL or la.norm(physical.T@physical-np.eye(physical.shape[1])) > TOL:
        raise ArithmeticError("independent Gauss kernel residual or orthogonality failed")
    expanded_codes = oscillator_codes(cutoff+4)
    expanded_states = decode(expanded_codes)
    index = {int(code): i for i, code in enumerate(expanded_codes)}
    embedded = np.zeros((len(expanded_codes), physical.shape[1]))
    embedded[np.searchsorted(expanded_codes, codes)] = physical
    coordinates = []
    for axis in range(9):
        rows, columns, vals = [], [], []
        for col, state in enumerate(expanded_states):
            for delta, count in ((1, int(state[axis])+1), (-1, int(state[axis]))):
                if count == 0:
                    continue
                target = int(expanded_codes[col]+delta*UNITS[axis])
                if target in index:
                    rows.append(index[target]); columns.append(col); vals.append(math.sqrt(count/2))
        coordinates.append(sparse.csr_matrix((vals, (rows, columns)),
                                             shape=(len(expanded_codes), len(expanded_codes))))
    def product(axes, vec):
        for axis in reversed(axes):
            vec = coordinates[axis] @ vec
        return vec
    action = (expanded_states.sum(axis=1)+4.5)[:, None]*embedded
    for permutation in it.permutations(range(3)):
        sign = (-1)**sum(permutation[i] > permutation[j] for i in range(3) for j in range(i+1, 3))
        action -= 3*math.sqrt(g2)*sign*product([3*i+permutation[i] for i in range(3)], embedded)
    for i, j in it.combinations(range(3), 2):
        for a, b in it.combinations(range(3), 2):
            def minor(vec):
                return product([3*i+a, 3*j+b], vec)-product([3*i+b, 3*j+a], vec)
            action += g2/2*minor(minor(embedded))
    h = embedded.T@action
    grams = [[sum((product([3*i+a, 3*j+a], embedded) for a in range(3))) for j in range(3)] for i in range(3)]
    tensor_actions = [(grams[1][1]-grams[2][2])/np.sqrt(2), np.sqrt(2)*grams[1][2],
                      np.sqrt(2)*grams[0][1], np.sqrt(2)*grams[0][2],
                      (-2*grams[0][0]+grams[1][1]+grams[2][2])/np.sqrt(6)]
    scalar = sum(a.T@a for a in tensor_actions)
    return {"codes": codes, "physical": physical, "dimension": physical.shape[1],
            "H": h, "energies": la.eigvalsh(h), "tensor": [embedded.T@a for a in tensor_actions],
            "scalar": scalar, "gauss_residual": float(la.norm(casimir@physical))}


def compare_reference(basis, operators, reference):
    c = np.zeros((len(reference["codes"]), basis["C"].shape[1]))
    c[np.searchsorted(reference["codes"], basis["codes"])] = basis["C"].toarray()
    change = reference["physical"].T @ c
    errors = [la.norm(reference["physical"]@change-c), la.norm(change.T@change-basis["S"])]
    h = operators["H0"]-3*operators["D"]+operators["V4"]
    errors.append(la.norm(change.T@reference["H"]@change-h)/max(1., la.norm(h)))
    for actual, independent in zip(operators["tensor"]+[operators["scalar"]], reference["tensor"]+[reference["scalar"]]):
        errors.append(la.norm(change.T@independent@change-actual)/max(1., la.norm(actual)))
    return float(max(errors))


def energy_groups(energies):
    groups, start = [], 0
    for i in range(1, len(energies)+1):
        if i == len(energies) or energies[i]-energies[start] > 1e-8*max(1., abs(energies[start])):
            groups.append(np.arange(start, i))
            start = i
    return groups


def probe_report(energies, vectors, probes):
    groups = energy_groups(energies)
    if len(groups[0]) != 1:
        raise ValueError("ground-state multiplicity unresolved")
    amplitudes = np.column_stack([vectors.conj().T@o@vectors[:, 0] for o in probes])
    amplitudes[0] = 0
    norm = float(np.sum(np.abs(amplitudes)**2))
    result = {"absolute_gap": float(energies[1]-energies[0]), "excited_norm_squared": norm,
              "accessible_gap": None, "lowest_weight": 0., "levels": [],
              "accessibility_policy": "robust detection weight >= 1e-4; tiny weights are not exact zeros"}
    if norm <= 1e-24:
        return dict(result, status="ZERO_EXCITED_NORM")
    for indices in groups[1:]:
        kernel = amplitudes[indices].conj().T@amplitudes[indices]
        weight = float(np.trace(kernel).real/norm)
        gap = float(energies[indices].mean()-energies[0])
        result["levels"].append({"gap": gap, "multiplicity": len(indices), "weight": weight,
                                 "spread": float(np.ptp(energies[indices])),
                                 "kernel_real": kernel.real.tolist(), "kernel_imag": kernel.imag.tolist()})
        if result["accessible_gap"] is None and weight >= 1e-4:
            result["accessible_gap"] = gap
    result["lowest_weight"] = result["levels"][0]["weight"]
    result["status"] = ("DETECTED" if result["lowest_weight"] >= 1e-4 else
                        "HIGHER_LEVEL_ONLY" if result["lowest_weight"] == 0 else "LOWEST_OVERLAP_UNRESOLVED")
    return result


def expanded_action(basis, vectors, polynomial_terms, output_codes):
    fock = basis["C"]@vectors
    out = np.zeros((len(output_codes), vectors.shape[1]), dtype=fock.dtype)
    for columns, targets, coefficients in transitions(basis["states"], basis["codes"], polynomial_terms):
        indices = np.searchsorted(output_codes, targets)
        if np.any(indices >= len(output_codes)) or not np.array_equal(output_codes[indices], targets):
            raise ValueError("output space truncates an intermediate or final action")
        out[indices] += coefficients[:, None]*fock[columns]
    return out


def propagator_check(h, s, energies, vectors, probes):
    import torch
    l = la.cholesky(s, lower=True)
    left = la.solve_triangular(l, h, lower=True)
    white = la.solve_triangular(l, left.conj().T, lower=True).conj().T
    omega = l.conj().T@vectors[:, 0]
    phi = np.column_stack([la.solve_triangular(l, o@vectors[:, 0], lower=True) for o in probes])
    phi -= omega[:, None]*(omega.conj()@phi)[None, :]
    amplitudes = np.column_stack([vectors.conj().T@o@vectors[:, 0] for o in probes])
    amplitudes[0] = 0
    norm = max(1., float(np.sum(np.abs(phi)**2)))
    if not torch.cuda.is_available():
        raise RuntimeError("CUDA unavailable for propagator")
    a = torch.as_tensor((white+white.conj().T)/2-energies[0]*np.eye(len(h)), device="cuda")
    p = torch.as_tensor(phi, device="cuda")
    errors = []
    with torch.inference_mode():
        for time_value in (0., 0.25, 0.5, 1., 2., 4.):
            computed = (p.mH@torch.matrix_exp(-time_value*a)@p).cpu().numpy()
            expected = amplitudes.conj().T@(np.exp(-time_value*(energies-energies[0]))[:, None]*amplitudes)
            errors.append(float(la.norm(computed-expected)/norm))
    return {"max_error": max(errors), "errors": errors, "device": str(a.device)}


def validate_receipt(receipt, run_id, hashes, required_gates=("basis", "metric")):
    required = receipt.get("required_gates")
    return (receipt.get("status") == "PASS" and receipt.get("run_id") == run_id
            and receipt.get("source_hashes") == hashes and bool(hashes)
            and isinstance(required, list) and bool(required)
            and set(required) == set(required_gates) and len(required) == len(set(required))
            and all(receipt.get("gates", {}).get(key) is True for key in required))


def reported_groups(energies, vectors, channels):
    groups = energy_groups(energies)
    included = {0, 1}
    for channel in channels:
        result = probe_report(energies, vectors, channel)
        for index, level in enumerate(result["levels"], 1):
            if level["weight"] >= 1e-4:
                included.add(index)
                break
    return [groups[index] for index in sorted(included)]


def exterior_diagnostics(basis, operators, energies, vectors, g2):
    codes = oscillator_codes(basis["cutoff"]+4)
    positions = np.searchsorted(codes, basis["codes"])
    outside = decode(codes).sum(axis=1) > basis["cutoff"]
    groups = reported_groups(energies, vectors, [operators["tensor"], [operators["scalar"]]])
    selected_indices = np.concatenate(groups)
    selected = vectors[:, selected_indices]
    polynomials = operator_polynomials()
    hterms = [(-3*math.sqrt(g2)*c, a) for c, a in terms(polynomials["D"])]
    hterms += [(g2*c, a) for c, a in terms(polynomials["V4"])]
    action = expanded_action(basis, selected, hterms, codes)
    fock = basis["C"]@selected
    action[positions] += (basis["states"].sum(axis=1)+4.5)[:, None]*fock
    action[positions] -= fock*energies[selected_indices]
    norms = la.norm(action, axis=0)
    level_norms = []
    offset = 0
    for group in groups:
        # Worst direction inside each degenerate level, not an arbitrary basis column.
        block = action[:, offset:offset+len(group)]
        gram = block.conj().T@block
        level_norms.append(float(np.sqrt(max(0., la.eigvalsh(gram)[-1]))))
        offset += len(group)
    del action
    tails = {}
    for name, polys in (("H2", polynomials["tensor"]), ("H0", [polynomials["scalar"]])):
        total, discarded = 0., 0.
        for poly in polys:
            response = expanded_action(basis, vectors[:, :1], terms(poly), codes)
            mean = float((fock[:, 0].conj()@response[positions, 0]).real)
            response[positions, 0] -= mean*fock[:, 0]
            total += float(np.vdot(response, response).real)
            discarded += float(np.vdot(response[outside], response[outside]).real)
        tails[name] = discarded/total if total > 1e-24 else None
    allowed = 1e-2*max(1., float(energies[1]-energies[0]))
    return {"state_indices": selected_indices.tolist(), "full_residual_norms": norms.tolist(), "level_worst_residuals": level_norms,
            "allowed_residual": allowed, "probe_discarded_fractions": tails,
            "residual_gate": max(level_norms) <= allowed,
            "tail_gate": all(x is not None and x <= 1e-3 for x in tails.values())}


def convergence(history):
    if len(history) < 3:
        return False
    last = history[-3:]
    if not last[-1]["exterior"]["residual_gate"] or not last[-1]["exterior"]["tail_gate"]:
        return False
    for a, b in zip(last, last[1:]):
        pairs = [(a[k], b[k]) for k in ("E0", "E1", "absolute_gap")]
        for channel in ("H0", "H2"):
            pairs.append((a[channel]["accessible_gap"], b[channel]["accessible_gap"]))
            if abs(a[channel]["lowest_weight"]-b[channel]["lowest_weight"]) > 0.01:
                return False
        if any(x is None or y is None or not np.isfinite([x, y]).all()
               or abs(x-y) > 1e-3*max(1., abs(y)) for x, y in pairs):
            return False
    return True


def gpu_krylov(h, seeds, vacuum, budgets=(8, 16, 32, 64)):
    """Count each H-vector product; no excited-state eigenvectors enter the seed."""
    import torch
    a = torch.as_tensor(h, device="cuda", dtype=torch.float64)
    omega = torch.as_tensor(vacuum, device="cuda", dtype=torch.float64)
    initial = seeds-vacuum[:, None]*(vacuum@seeds)[None, :]
    u, singular, _ = la.svd(initial, full_matrices=False)
    rank = int(np.count_nonzero(singular > 1e-12*max(1., singular[0])))
    if rank == 0:
        return {"status": "ZERO_SEED", "checkpoints": []}
    columns = [torch.as_tensor(u[:, i], device="cuda") for i in range(rank)]
    actions, checkpoints = [], []
    with torch.inference_mode():
        for step in range(min(max(budgets), len(h)-1)):
            if step == len(columns):
                break
            action = a@columns[step]
            actions.append(action)
            candidate = action.clone()
            for _ in range(2):
                candidate -= omega*torch.dot(omega, candidate)
                block = torch.stack(columns, dim=1)
                candidate -= block@(block.T@candidate)
            norm = float(torch.linalg.norm(candidate))
            if norm > 1e-12*max(1., float(torch.linalg.norm(action))):
                columns.append(candidate/norm)
            count = step+1
            if count in budgets or count == len(columns):
                block = torch.stack(columns[:count], dim=1)
                acted = torch.stack(actions, dim=1)
                small = block.T@acted
                e, v = torch.linalg.eigh((small+small.T)/2)
                residual = float(torch.linalg.norm(acted@v[:, 0]-e[0]*(block@v[:, 0])))
                checkpoints.append({"matvecs": count, "gap": float(e[0]), "residual": residual})
    return {"status": "COMPUTED", "device": "cuda:0", "checkpoints": checkpoints}
