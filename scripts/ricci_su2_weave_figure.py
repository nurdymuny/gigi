"""The weave figure (docs/research/skeleton-test2-family1-results-2026-11-20.md, "The weave").

The top-weight block of H in the sector of spin j/2: H compressed to {e_i (x) e_a (x) f}, with f the top weight of
J0 = (i/2) rho_0 and (i, a) the frame and Lie indices. On the squashed plane (i, a in {1, 2}) at t = 3,
3 * block - (j^2 + 3j) I is the 4 x 4 matrix of Proposition 4.1 (checked here). The script compares where the entries
are real, imaginary or zero at t = 1 and t = 3 for j = 2, 4, 6, 8, and draws the j = 4 block tiled 3 x 3.
usage: python scripts/ricci_su2_weave_figure.py OUT.png
"""
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_sector_gauge_check as C     # noqa: E402
import ricci_su2_spin_sector as S            # noqa: E402


def top_block(t, j, frames=(0, 1, 2), lies=(0, 1, 2)):
    rho = S.spin_rep(j)
    J0 = 0.5j * rho[0]
    w, V = np.linalg.eigh(0.5 * (J0 + J0.conj().T))
    if abs(w[-1] - j / 2) > 1e-9:
        raise SystemExit("top weight not found")
    f = V[:, -1]
    H, _ = C.hessian_and_gauge([t, 1, 1], j)
    cols = []
    for i in frames:
        for a in lies:
            e = np.zeros(9); e[3 * i + a] = 1
            cols.append(np.kron(e, f))
    P = np.array(cols).T
    return P.conj().T @ H @ P


def kind(M, tol=1e-9):
    K = np.zeros(M.shape, int)
    K[np.abs(M.real) > tol] = 1
    K[np.abs(M.imag) > tol] = 2
    if np.any((np.abs(M.real) > tol) & (np.abs(M.imag) > tol)):
        raise SystemExit("an entry is neither real nor imaginary")
    return K


def main():
    out = Path(sys.argv[1])
    j = 4
    M = 3 * top_block(3.0, j, (1, 2), (1, 2)) - (j * j + 3 * j) * np.eye(4)
    paper = np.array([[14, 2j * j, -1j * j, 28], [-2j * j, 5, -19, -1j * j], [1j * j, -19, 5, 2j * j],
                      [28, 1j * j, -2j * j, 14]])
    print("Proposition 4.1 block reproduced (j = 4):", np.allclose(M, paper, atol=1e-9))
    same = {jj: bool((kind(top_block(1.0, jj)) == kind(top_block(3.0, jj))).all()) for jj in (2, 4, 6, 8)}
    print("real / imaginary / zero pattern identical at t = 1 and t = 3:", same)

    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    from matplotlib.colors import ListedColormap
    from matplotlib.patches import Patch
    ink, muted, surface, real_c, imag_c = "#1f2a2c", "#5f6b6d", "#fcfcfb", "#2a78d6", "#eb6834"
    cmap = ListedColormap([surface, real_c, imag_c])
    fig, axs = plt.subplots(1, 2, figsize=(12, 6.4), facecolor=surface)
    labels = [f"{i}{a}" for i in range(3) for a in range(3)]
    for ax, t in ((axs[0], 1.0), (axs[1], 3.0)):
        ax.imshow(np.tile(kind(top_block(t, j)), (3, 3)), cmap=cmap, vmin=0, vmax=2, interpolation="nearest")
        for x in range(0, 28, 9):
            ax.axhline(x - 0.5, color=muted, lw=1.2); ax.axvline(x - 0.5, color=muted, lw=1.2)
        for r in (4, 5, 7, 8):
            for c in (4, 5, 7, 8):
                ax.add_patch(plt.Rectangle((9 + c - 0.5, 9 + r - 0.5), 1, 1, fill=False, ec="#0b0b0b", lw=1.0))
        ax.set_xticks(range(9, 18)); ax.set_xticklabels(labels, fontsize=7, color=muted)
        ax.set_yticks(range(9, 18)); ax.set_yticklabels(labels, fontsize=7, color=muted)
        ax.set_title(f"t = {t:g}" + ("  (round sphere)" if t == 1.0 else "  (Berger, the Floor Theorem)"),
                     fontsize=12, color=ink)
        for s in ax.spines.values():
            s.set_visible(False)
        ax.tick_params(length=0)
    fig.suptitle("The 9×9 top-weight block (j = 4), tiled 3×3: where entries are real, imaginary or zero",
                 fontsize=13, color=ink)
    fig.legend(handles=[Patch(color=real_c, label="real entry"), Patch(color=imag_c, label="imaginary entry"),
                        Patch(facecolor=surface, edgecolor=muted, label="zero"),
                        Patch(facecolor="none", edgecolor="#0b0b0b", label="the squashed-plane 4×4 (centre tile)")],
               loc="lower center", ncol=4, frameon=False, fontsize=10)
    fig.text(0.5, 0.085, "Tick labels: frame index, Lie index (0 = the long axis). Same pattern for j = 2, 6, 8.",
             ha="center", fontsize=9, color=muted)
    plt.tight_layout(rect=(0, 0.1, 1, 0.95))
    fig.savefig(out, dpi=130, facecolor=surface)
    print("saved", out)


if __name__ == "__main__":
    main()
