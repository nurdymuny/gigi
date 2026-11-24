# helike-lab

A Rust solver (and, from milestone M1, a live viewer) for Yang–Mills Hessians at left-invariant fields on compact
groups. Spec: [`docs/research/helike-lab-rust-spec-2026-11-24.md`](../docs/research/helike-lab-rust-spec-2026-11-24.md).

This is a separate Cargo workspace. It is not part of the `gigi` crate and does not touch the engine, its endpoints or
GQL. The Python scripts in `scripts/` stay the reference: no Rust number is trusted until it matches them.

## What is general, and what is Helike

**The general layer** works for any compact group K, given:
- a frame algebra (structure constants);
- a left-invariant metric diagonal in that basis;
- a gauge algebra;
- a left-invariant field;
- a representation of the frame algebra (the Peter–Weyl sector).

| module | what it gives |
| --- | --- |
| `lie` | Lie algebras by structure constants. Covers su(2) in the frame convention [e_a, e_b] = 2ε e_c and the cross-product convention, su(N) in the generalised Gell-Mann basis, and any closed orthonormal basis of anti-Hermitian matrices. Includes Jacobi and ad-invariance checks. |
| `rep` | Representations: SU(2) spin j/2; SU(N) irreps (p, 0, …, 0, q); tensor products, duals, direct sums; the submodule generated from a vector by lowering operators. Checked by the homomorphism error and the Weyl dimension formula. |
| `sector` | The Hessian H and gauge map G in one sector (a port of `hessian_and_gauge`), curl in three dimensions, curvature, energy, Levi-Civita connection, Ricci. |
| `charge` | Splits a sector by any circle symmetry (frame generator plus gauge generator) into charge blocks of H, H + GG\* and the gauge slice. Signs are fitted, and the commutator and intertwiner residuals are reported, so a non-symmetry shows up as a large residual. |
| `problem` | A JSON problem description: any of the above in one file, solved by `helike solve`. |

**The Helike instance** (`helike`) is the circle-symmetric SU(2) family on the Berger spheres. It includes:
- the twist's closed form and the Floor law;
- the fibre and cross fields, and the flux spectrum;
- the ladder's sufficient floor condition and x_flux;
- the edge functions φ_A (charge blocks) and φ_S (gauge slice), and the edges.

## Parity (milestone M0)

`oracle/make_oracle.py` writes `oracle/oracle_m0.json` from the Python reference. `cargo test --release` compares
against it. Results at commit time:

| quantity | oracle | worst difference |
| --- | --- | --- |
| H, G, curl at 8 random SU(2) family members, j ≤ 4 | `hessian_and_gauge`, `helike_curl_spectrum.operators` | 7.1e-15 entrywise |
| full spectra; gauge-slice spectra (dense and blockwise) | numpy `eigvalsh`, `slice_spectrum` | 3.0e-13 |
| H, G for SU(3) family 2b at t = 0.7, 3, sectors (1,0) and (0,1); spectra and slice in (1,1) | `su3_irrep`, `hessian_and_gauge` | 4.4e-16 entrywise; 6.1e-13 spectra |
| φ_A and φ_S at 5 points, j ≤ 12, with the minimising sector and charge | `ricci_su2_exact_edge` | below 1e-12 |
| the edges x\*_A and x\*_S at 3 cells | `ricci_su2_exact_edge.edge` | below 1e-12 |
| By Enough Table 1, all 30 entries (`helike table1`) | the paper | 0 at six decimals |
| Berger crossings | t_A = 2.73112227823811362…, t\* ≈ 2.698026940520 | 2e-15; 1e-12 |

Each of these deliberate faults must break parity, and each does:
- the flux sign flipped;
- the frame-bracket term dropped;
- a flat metric;
- the lower flux block left untransposed.

An earlier version of the last fault was a no-op, because P^T = −P. The test caught that it caught nothing, and that
version was replaced.

## Using it

```sh
cargo test --release                                  # parity and generality tests (needs oracle/oracle_m0.json)
cargo run --release -p helike-cli -- solve examples/su3_hypercharge_stretch.json
cargo run --release -p helike-cli -- edge --t 3 --y 1.5 --spectrum a
cargo run --release -p helike-cli -- --out receipts/table1_j12.json table1
cargo run --release -p helike-cli -- verify receipts/table1_j12.json
cargo run --release -p helike-cli -- ladder --x -1 --y 1.7320508 --t 3
cargo run --release -p helike-cli -- map --y 1.5 --nt 25 --nx 41 --out receipts/map_y1.5.json
```

Each receipt records:
- the command and its parameters;
- the results;
- the git revision and dirty flag;
- a SHA-256 of the solver sources;
- the platform and thread count;
- a status label: `COMPUTED`, `DISPLAY_GRADE`, `PASS_CERTIFIED_FP` or `PASS_CERTIFIED_INTERVAL`.

`verify` re-runs the recorded command and compares the results.

## Status

**Computed, all in double precision on the CPU:**
- sector operators;
- charge blocks;
- gauge slices;
- the edge functions and edges.

**Not yet built:**
- the viewer (M1);
- GPU batched blocks (M2);
- `helike-quantum` (M3);
- interval enclosures (`inari`).

Nothing here is a proof.
