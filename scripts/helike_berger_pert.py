"""Step 1 of the Berger probe design: exact order-g^2 perturbation theory for the soft two-quantum states.

H_t = H0 + V3 + V4 with H0 = sum_{k,a} (-1/2 d^2 + 1/2 omega_k^2 q_ka^2), V3 = -g s det q (s = sum_k omega_k) and
V4 = (g^2/2) |cof q|^2. Rows k of q are frame indices (k = 0 the long axis), columns a are colour.
q_ka = (a_ka + a_ka^dag) / sqrt(2 omega_k). V3 and V4 are finite polynomials in ladder operators, so V3|psi> is a
finite vector and the second-order sums are exact finite sums (no truncation).

For t > 1 the soft modes are rows 1, 2 (frequency 1/sqrt t). The colour-singlet soft pairs are
  Helike's channel  m = +-2:  sum_a (a_1a^dag +- i a_2a^dag)^2 |0>,
  the pair scalar   m = 0:    sum_a (a_1a^dag a_1a^dag + a_2a^dag a_2a^dag) |0>,
degenerate at harmonic order (gap 2/sqrt t). Long-axis charge is conserved, so each is non-degenerate in its sector
and the shifts are
  dE(psi) = <psi|V4|psi> + sum_{n} |<n|V3|psi>|^2 / (E_psi - E_n),
where V3 changes the quantum number by an odd amount, so no intermediate state lies in the degenerate pair.
"""
from __future__ import annotations

import itertools
import math
from collections import defaultdict

import numpy as np

MODES = [(k, a) for k in range(3) for a in range(3)]
IDX = {m: i for i, m in enumerate(MODES)}


def omega(t):
    return np.array([math.sqrt(t), 1 / math.sqrt(t), 1 / math.sqrt(t)])


def apply_q(state, k, a, om):
    """q_ka on a state {occupation tuple: amplitude}."""
    i = IDX[(k, a)]
    c = 1 / math.sqrt(2 * om[k])
    out = defaultdict(complex)
    for occ, amp in state.items():
        n = occ[i]
        if n > 0:
            o = list(occ); o[i] -= 1
            out[tuple(o)] += amp * c * math.sqrt(n)
        o = list(occ); o[i] += 1
        out[tuple(o)] += amp * c * math.sqrt(n + 1)
    return dict(out)


def add(a, b, s=1.):
    out = defaultdict(complex, a)
    for k, v in b.items():
        out[k] += s * v
    return {k: v for k, v in out.items() if abs(v) > 1e-15}


def apply_det(state, om):
    out = {}
    for p in itertools.permutations(range(3)):
        sign = np.linalg.det(np.eye(3)[list(p)])
        st = state
        for k in range(3):
            st = apply_q(st, k, p[k], om)
        out = add(out, st, sign)
    return out


def apply_cof(state, k, a, om):
    k1, k2 = (k + 1) % 3, (k + 2) % 3
    a1, a2 = (a + 1) % 3, (a + 2) % 3
    t1 = apply_q(apply_q(state, k2, a2, om), k1, a1, om)
    t2 = apply_q(apply_q(state, k2, a1, om), k1, a2, om)
    return add(t1, t2, -1.)


def apply_V3(state, g, om):
    return {k: -g * om.sum() * v for k, v in apply_det(state, om).items()}


def apply_V4(state, g, om):
    out = {}
    for k in range(3):
        for a in range(3):
            out = add(out, apply_cof(apply_cof(state, k, a, om), k, a, om), .5 * g * g)
    return out


def energy(occ, om):
    return sum(n * om[k] for (k, a), n in zip(MODES, occ))


def inner(a, b):
    return sum(np.conj(v) * b.get(k, 0) for k, v in a.items())


def normalise(st):
    n = math.sqrt(abs(inner(st, st)))
    return {k: v / n for k, v in st.items()}


VAC = (0,) * 9


def create(state, k, a, coef=1.):
    i = IDX[(k, a)]
    out = defaultdict(complex)
    for occ, amp in state.items():
        o = list(occ); o[i] += 1
        out[tuple(o)] += coef * amp * math.sqrt(occ[i] + 1)
    return dict(out)


def soft_pair(m):
    """Colour-singlet soft pair with long-axis charge m in {0, 2, -2}."""
    st = {}
    for a in range(3):
        if m == 0:
            for k in (1, 2):
                st = add(st, create(create({VAC: 1.}, k, a), k, a))
        else:
            s = 1j if m == 2 else -1j
            one = add(create({VAC: 1.}, 1, a), create({VAC: 1.}, 2, a, s))
            two = add(create(one, 1, a), create(one, 2, a, s))
            st = add(st, two)
    return normalise(st)


def shifts(psi, g, t, tol=1e-9):
    """(quartic first-order, cubic second-order) energy shifts of the H0 eigenstate psi."""
    om = omega(t)
    e_psi = energy(next(iter(psi)), om)
    first = inner(psi, apply_V4(psi, g, om)).real
    v3 = apply_V3(psi, g, om)
    second = 0.
    for occ, amp in v3.items():
        de = e_psi - energy(occ, om)
        if abs(de) < tol:
            raise ArithmeticError(f"resonant intermediate state {occ} at t = {t}")
        second += abs(amp) ** 2 / de
    return first, second


def splitting(g, t):
    """Excitation shifts (relative to the vacuum's shift) for m = +2 and the m = 0 pair scalar, and their split.
    Returns a dict with the quartic and cubic parts separated; harmonic gap is 2/sqrt t for both."""
    vq, vc = shifts({VAC: 1.}, g, t)
    out = {}
    for m in (2, 0):
        q4, q3 = shifts(soft_pair(m), g, t)
        out[m] = {"quartic": q4 - vq, "cubic": q3 - vc, "total": q4 + q3 - vq - vc}
    out["split"] = {k: out[2][k] - out[0][k] for k in ("quartic", "cubic", "total")}
    out["harmonic_gap"] = 2 / math.sqrt(t)
    return out
