"""GPU hardware/precision smoke test. Planted fixtures, not Yang-Mills results."""

import argparse
import hashlib
import json
import os
from datetime import datetime, timezone
from pathlib import Path


def publish(path, receipt):
    if path is None:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".pending")
    temporary.write_text(json.dumps(receipt, indent=2, allow_nan=False) + "\n", encoding="utf-8")
    temporary.replace(path)


def run(receipt, defect):
    import numpy as np
    import scipy
    import scipy.linalg as la
    import torch

    if not torch.cuda.is_available():
        raise RuntimeError("CUDA required; CPU fallback is forbidden")
    torch.set_num_threads(1)
    torch.backends.cuda.matmul.allow_tf32 = False
    torch.backends.cudnn.allow_tf32 = False
    properties = torch.cuda.get_device_properties(0)
    free, total = torch.cuda.mem_get_info(0)
    receipt["environment"] = {
        "torch": torch.__version__, "cuda_runtime": torch.version.cuda,
        "numpy": np.__version__, "scipy": scipy.__version__,
        "gpu": properties.name, "capability": [properties.major, properties.minor],
        "architectures": torch.cuda.get_arch_list(),
        "device_total_bytes": total, "device_free_before_bytes": free,
        "blas_thread_environment": {key: os.environ.get(key) for key in (
            "OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS")},
    }
    if free < 1024**3 + 512 * 1024**2:
        raise RuntimeError("Insufficient free VRAM for smoke test and 1 GiB headroom")
    torch.cuda.reset_peak_memory_stats(0)
    cases = [(n, False) for n in (29, 91, 238, 546, 1134)] + [(91, True)]
    if defect:
        cases = cases[:1]
    receipt["cases"] = []
    for n, complex_case in cases:
        # Orthogonal DCT basis supplies a spectrum and level projector a priori.
        rows = np.arange(n, dtype=np.float64)[:, None]
        cols = np.arange(n, dtype=np.float64)[None, :]
        basis = np.sqrt(2.0 / n) * np.cos(np.pi * (rows + 0.5) * cols / n)
        basis[:, 0] /= np.sqrt(2.0)
        if complex_case:
            basis = np.exp(1j * np.arange(n) / n)[:, None] * basis
        expected = np.concatenate(([4.5], np.full(6, 6.5), np.linspace(7.5, 20.0, n - 7)))
        matrix = (basis * expected) @ basis.conj().T
        matrix = (matrix + matrix.conj().T) / 2
        cpu_values, cpu_vectors = la.eigh(matrix, driver="evd")
        dtype = torch.complex128 if complex_case else torch.float64
        if defect == "float32":
            dtype = torch.float32
        gpu = torch.as_tensor(matrix, dtype=dtype,
                              device="cpu" if defect == "cpu" else "cuda:0")
        if gpu.device.type != "cuda":
            raise RuntimeError("Device gate: numerical tensor is not on CUDA")
        if gpu.dtype not in (torch.float64, torch.complex128):
            raise RuntimeError("Precision gate: float64 or complex128 required")
        if not bool(torch.isfinite(gpu).all()):
            raise RuntimeError("Nonfinite input")
        with torch.inference_mode(), torch.autocast(device_type="cuda", enabled=False):
            values, vectors = torch.linalg.eigh(gpu)
            torch.cuda.synchronize()
            if values.device.type != "cuda" or vectors.device.type != "cuda":
                raise RuntimeError("Output device gate failed")
            values_np = values.cpu().numpy()
            vectors_np = vectors.cpu().numpy()
        if defect == "corrupt_answer":
            values_np[0] += 0.01
        scale = max(1.0, float(np.max(np.abs(expected))))
        projector = vectors_np[:, 1:7] @ vectors_np[:, 1:7].conj().T
        expected_projector = basis[:, 1:7] @ basis[:, 1:7].conj().T
        cpu_projector = cpu_vectors[:, 1:7] @ cpu_vectors[:, 1:7].conj().T
        errors = {
            "planted_spectrum_scaled": float(np.max(np.abs(values_np - expected)) / scale),
            "cpu_spectrum_scaled": float(np.max(np.abs(values_np - cpu_values)) / scale),
            "residual_max_column_scaled": float(np.max(np.linalg.norm(
                matrix @ vectors_np - vectors_np * values_np, axis=0)) / scale),
            "orthogonality_frobenius": float(np.linalg.norm(
                vectors_np.conj().T @ vectors_np - np.eye(n))),
            "planted_projector_frobenius": float(np.linalg.norm(projector - expected_projector)),
            "cpu_projector_frobenius": float(np.linalg.norm(projector - cpu_projector)),
        }
        multiplicity = int(np.count_nonzero(np.abs(values_np - 6.5) <= 6.5e-10))
        good = multiplicity == 6 and all(np.isfinite(v) and v <= 1e-10 for v in errors.values())
        receipt["cases"].append({
            "dimension": n, "dtype": str(gpu.dtype), "device": str(gpu.device),
            "level_multiplicity": multiplicity, "errors": errors,
            "status": "PASS" if good else "FAIL",
        })
        if not good:
            raise RuntimeError(f"Planted numerical fixture failed: dimension {n}")
        del gpu, values, vectors
    receipt["peak_torch_allocated_bytes"] = torch.cuda.max_memory_allocated(0)
    receipt["peak_torch_reserved_bytes"] = torch.cuda.max_memory_reserved(0)
    receipt["memory_scope"] = "PyTorch allocator only; excludes some library/context allocations"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--inject-defect", choices=("cpu", "float32", "corrupt_answer"))
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    receipt = {
        "status": "RUNNING", "scope": "synthetic_gpu_preflight_only",
        "quantum_spectrum_status": "NOT_RUN", "created_utc": datetime.now(timezone.utc).isoformat(),
        "injected_defect": args.inject_defect, "tolerance": 1e-10,
        "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "amendment_sha256": hashlib.sha256((root / "docs/research/helike-quantum-gpu-amendment-2026-11-23.md").read_bytes()).hexdigest(),
    }
    publish(args.out, receipt)
    try:
        run(receipt, args.inject_defect)
        receipt["status"] = "PASS"
    except Exception as error:
        receipt.update(status="FAIL", error=f"{type(error).__name__}: {error}")
    publish(args.out, receipt)
    print(json.dumps(receipt, indent=2, allow_nan=False))
    return 0 if receipt["status"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
