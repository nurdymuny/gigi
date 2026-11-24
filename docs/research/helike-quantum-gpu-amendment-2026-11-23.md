# Helike quantum probe GPU implementation amendment

**Bee Rosa Davis | Davis Geometric**  
23 November 2026. Amendment 1 to the
[quantum-probe registration](helike-quantum-probe-preregistration-2026-11-23.md).

## Requirement

The experiment must have a verified execution path on my GPU. We adapt the
implementation to the hardware, not the Hamiltonian to the hardware. The
registered model, probes, couplings, basis cutoffs and numerical acceptance
thresholds are unchanged. No interacting quantum results have been seen.

## Hardware and environment observed

The local inventory reports an NVIDIA GeForce RTX 5070 Laptop GPU, 8151 MiB
total memory, compute capability 12.0, and driver 591.44. The existing Windows
Python 3.12.10 installation has PyTorch 2.11.0.dev20260108+cu128, CUDA runtime
12.8, and native sm_120 support in its architecture list. CUDA is available.
No driver or package upgrade is needed to begin the preflight; the environment
must be pinned and recorded rather than silently upgraded during the study.

This runtime matches NVIDIA's documented first toolkit support for compute
capability 12.0: [CUDA architecture matrix](https://docs.nvidia.com/datacenter/tesla/drivers/latest/cuda-toolkit-driver-and-architecture-matrix.html).
Availability alone is not a numerical acceptance test.

## CPU and GPU responsibilities

The CPU retains the exact symbolic identities, rational Gaussian moments,
independent small-basis construction and high-precision conditioning checks.
Their role is to check the numerical calculation, not to imitate a GPU result.

The GPU path uses the existing PyTorch CUDA installation for double-precision
Hermitian eigensolves, triangular solves, block matrix-vector products, Krylov
iterations and probe/propagator evaluation. Keep the working matrices and Krylov
vectors resident on the device. Transfer input once per problem and return
results and diagnostics at declared boundaries; do not transfer every iteration.
Benchmark numerical assembly separately before moving it to the GPU. Exact
symbolic algebra is not to be replaced by sampled or rounded assembly.

For the generalized eigenproblem H c = E S c, the Gram matrix S is not an
optional normalization. With S = L L^dagger, transform to

\[
 \widetilde H=L^{-1}H L^{-\dagger},\qquad c=L^{-\dagger}u.
\]

Use triangular solves, not explicit matrix inversion. Check the original residual
H c - E S c and c^dagger S c, not only the transformed residual. A failed or
ill-conditioned factorization must trigger the registered precision/refusal
policy, not a diagonal shift or removal of basis directions to make CUDA work.

Real arithmetic is explicitly float64; complex arithmetic is complex128.
Disable automatic mixed precision and TF32 on the experimental path. No FP16,
BF16 or float32 calculation can satisfy the 1e-10 acceptance gates by declaration.
The independent CPU route evaluates the same complete basis, not a cheaper
model. Compare whole degenerate-level projectors and transition kernels, not
the signs or individual vectors returned by different eigensolvers.

PyTorch documents double/complex-double support for
[`torch.linalg.eigh`](https://docs.pytorch.org/docs/2.11/generated/torch.linalg.eigh.html)
and warns that backend results need not be bitwise identical; ill-conditioning
also requires explicit attention. The acceptance target is the registered
residual and agreement tolerances, not bitwise equality.
[Numerical accuracy guidance](https://docs.pytorch.org/docs/2.11/notes/numerical_accuracy.html).

## Memory and timing

The first implementation has a 4 GiB device-memory budget, leaving room for the
desktop and other work. Retain the existing 8 GiB host-memory and 45-minute
survey ceilings. One process owns the GPU; do not run competing coupling jobs.
Check currently free memory before allocation and reserve at least 1 GiB of
headroom. Log allocator peak allocated/reserved memory as well as device free
memory; solver workspace and CUDA context are not zero-cost.

The largest registered singlet dimension is 1134. One dense real double matrix
of that size occupies 10,287,648 bytes (about 9.81 MiB); a complex double matrix
takes twice that. This is storage arithmetic, not a total solver-memory estimate
or evidence that matrix assembly will be fast. Workspaces, reference calculations,
intermediate polynomials and extra-cutoff residual checks are budgeted separately.

Warm-up, assembly, host/device transfers, solve, validation and total elapsed time
are recorded separately. Synchronize CUDA at timing boundaries. Compare CPU and
GPU on the same matrices and tolerances, with identical BLAS/thread settings
recorded. A cold load is not compared with a warmed solve.

The GPU path is mandatory and must work at every registered size. The default
dispatcher may use CPU for small jobs only after a measured comparison, and must
report that choice. A request for `device=cuda` cannot silently fall back to CPU.
No speedup claim is made until the actual experiment, including assembly and
verification, has been timed. Accelerating only a small stage may not accelerate
the full calculation.

## Focused gates before the physics run

| Gate | Required evidence | Defect that must fail |
| --- | --- | --- |
| GPU00 Device | Real CUDA tensors and native device execution; environment receipt | CPU fallback reported as GPU success; unavailable device |
| GPU01 Precision | float64 and complex128 solves meet the original tolerances | Quiet float32 cast; mixed precision; nonfinite result |
| GPU02 Planted spectrum | Known-spectrum symmetric and complex-Hermitian fixtures, including a sixfold level; CPU agreement and projector checks | Perturbed answer; one vector substituted for a degenerate projector |
| GPU03 Metric | Nonidentity Gram fixture agrees in original generalized residual and S-normalization | Ignore S; invert L on the wrong side |
| GPU04 Experiment | Actual N = 4 independent constructions and all HP gates also pass with CUDA | Correct toy fixture but incorrect physical Hamiltonian or projection |
| GPU05 Resources | Bounded allocations, explicit resource stop and synchronized timing | OOM hidden by fallback; incomplete run leaves a PASS |

GPU00--GPU05 are additional requirements, not replacements for HP00--HP10.
The hardware smoke test covers only parts of GPU00--GPU02. In particular, it
does not construct the quantum model or validate the generalized solver.

The reproducible preflight script is
[`ym_helike_gpu_preflight.py`](../../scripts/ym_helike_gpu_preflight.py).
Its fixtures have planted spectra and are explicitly labelled non-physics data.
Baseline and deliberate-defect receipts belong in
`research/helike-quantum-probe-20261123/gpu-preflight/` and must include the actual
device, dtype, software versions, hashes, errors and terminal status.

```text
python scripts/ym_helike_gpu_preflight.py --out research/helike-quantum-probe-20261123/gpu-preflight/baseline.json
python scripts/ym_helike_gpu_preflight.py --inject-defect cpu
python scripts/ym_helike_gpu_preflight.py --inject-defect float32
python scripts/ym_helike_gpu_preflight.py --inject-defect corrupt_answer
```

The baseline must exit 0; every deliberate defect must exit 1. GPU support is
ready for implementing the experiment only after the baseline passes. Full
GPU acceptance remains open until the actual model and generalized problem
have passed the additional gates.
