#!/usr/bin/env python3
"""
scripts/bench_cpu_gpu.py
aethyro-ntg — CPU vs GPU matrix/memory/compute benchmarks.

Benchmarks (all run on CPU; GPU versions added when CUDA is available):
  1. Matrix multiplication (float32, various sizes)
  2. Batch matrix multiply (attention-shaped)
  3. Element-wise ops (fused GELU approximation)
  4. Memory bandwidth (large tensor copy)

Run standalone:
  python3 scripts/bench_cpu_gpu.py [--skip-gpu] [--reps N] [--warmup N]

Or use the setup wrapper (handles PyTorch install for Blackwell):
  ./scripts/setup_bench.sh [--reps N] [--warmup N]
"""
from __future__ import annotations

import argparse
import math
import sys
import time
from contextlib import contextmanager
from typing import Callable, Optional

# ── optional torch import ──────────────────────────────────────────────────────
try:
    import torch
    TORCH_OK = True
except ImportError:
    TORCH_OK = False

try:
    import numpy as np
    NP_OK = True
except ImportError:
    NP_OK = False

# ── CLI ────────────────────────────────────────────────────────────────────────
def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="CPU vs GPU micro-benchmark")
    p.add_argument("--skip-gpu", action="store_true", help="Skip GPU benchmarks")
    p.add_argument("--reps",   type=int, default=20,  help="Timed repetitions (default 20)")
    p.add_argument("--warmup", type=int, default=5,   help="Warm-up repetitions (default 5)")
    return p.parse_args()


# ── utilities ────────────────────────────────────────────────────────────────────
def sep(ch="─", n=68) -> str:
    return ch * n


def gflops(ops: float, seconds: float) -> str:
    return f"{ops / seconds / 1e9:.2f} GFLOP/s"


def gbps(bytes_: float, seconds: float) -> str:
    return f"{bytes_ / seconds / 1e9:.2f} GB/s"


@contextmanager
def timer():
    """Context manager that yields a dict with 'elapsed' seconds after exit."""
    t = {"elapsed": 0.0}
    start = time.perf_counter()
    try:
        yield t
    finally:
        t["elapsed"] = time.perf_counter() - start


def sync_if_cuda(device: "torch.device") -> None:
    if TORCH_OK and device.type == "cuda":
        torch.cuda.synchronize(device)


def bench_fn(fn: Callable, warmup: int, reps: int, sync=None) -> float:
    """Return median wall-clock time in seconds over `reps` calls."""
    for _ in range(warmup):
        fn()
        if sync:
            sync()

    times = []
    for _ in range(reps):
        t0 = time.perf_counter()
        fn()
        if sync:
            sync()
        times.append(time.perf_counter() - t0)

    times.sort()
    return times[len(times) // 2]  # median


# ── benchmark definitions ────────────────────────────────────────────────────────────────

def run_matmul(device_label: str, device, sizes, reps: int, warmup: int) -> list[dict]:
    results = []
    for N in sizes:
        M = K = N
        a = torch.randn(M, K, dtype=torch.float32, device=device)
        b = torch.randn(K, N, dtype=torch.float32, device=device)

        _sync = (lambda: torch.cuda.synchronize(device)) if device.type == "cuda" else None
        elapsed = bench_fn(lambda: torch.matmul(a, b), warmup, reps, sync=_sync)

        flops = 2 * M * N * K
        results.append({
            "name": f"matmul {N}×{N}",
            "device": device_label,
            "elapsed_ms": elapsed * 1e3,
            "perf": gflops(flops, elapsed),
        })
    return results


def run_batch_matmul(device_label: str, device, reps: int, warmup: int) -> list[dict]:
    """Attention-shaped BMM: (B, H, S, D/H) x (B, H, D/H, S)."""
    results = []
    configs = [
        (4, 32, 512,  64),
        (4, 32, 2048, 64),
        (1, 32, 4096, 128),
    ]
    for B, H, S, D in configs:
        q = torch.randn(B, H, S, D, dtype=torch.float32, device=device)
        k = torch.randn(B, H, D, S, dtype=torch.float32, device=device)
        _sync = (lambda: torch.cuda.synchronize(device)) if device.type == "cuda" else None
        elapsed = bench_fn(lambda: torch.matmul(q, k), warmup, reps, sync=_sync)
        flops = 2 * B * H * S * S * D
        label = f"bmm B={B} H={H} S={S} D={D}"
        results.append({
            "name": label,
            "device": device_label,
            "elapsed_ms": elapsed * 1e3,
            "perf": gflops(flops, elapsed),
        })
    return results


def run_elementwise(device_label: str, device, reps: int, warmup: int) -> list[dict]:
    """Fused GELU approximation (tanh variant)."""
    results = []
    for n_elem in [1_000_000, 16_000_000, 256_000_000]:
        x = torch.randn(n_elem, dtype=torch.float32, device=device)
        sqrt2pi = math.sqrt(2.0 / math.pi)

        def gelu_approx():
            return 0.5 * x * (1 + torch.tanh(sqrt2pi * (x + 0.044715 * x ** 3)))

        _sync = (lambda: torch.cuda.synchronize(device)) if device.type == "cuda" else None
        elapsed = bench_fn(gelu_approx, warmup, reps, sync=_sync)
        bytes_ = n_elem * 4
        results.append({
            "name": f"gelu {n_elem//1_000_000}M elems",
            "device": device_label,
            "elapsed_ms": elapsed * 1e3,
            "perf": gbps(bytes_ * 2, elapsed),
        })
    return results


def run_bandwidth(device_label: str, device, reps: int, warmup: int) -> list[dict]:
    """Memory bandwidth: large tensor copy."""
    results = []
    for mb in [128, 512, 2048]:
        n = mb * 1024 * 1024 // 4
        src = torch.randn(n, dtype=torch.float32, device=device)
        _sync = (lambda: torch.cuda.synchronize(device)) if device.type == "cuda" else None
        elapsed = bench_fn(lambda: src.clone(), warmup, reps, sync=_sync)
        results.append({
            "name": f"memcpy {mb} MB",
            "device": device_label,
            "elapsed_ms": elapsed * 1e3,
            "perf": gbps(n * 4 * 2, elapsed),
        })
    return results


def run_numpy_matmul(sizes: list[int], reps: int, warmup: int) -> list[dict]:
    """NumPy CPU baseline."""
    results = []
    for N in sizes:
        a = np.random.randn(N, N).astype(np.float32)
        b = np.random.randn(N, N).astype(np.float32)
        elapsed = bench_fn(lambda: np.matmul(a, b), warmup, reps)
        flops = 2 * N * N * N
        results.append({
            "name": f"np.matmul {N}×{N}",
            "device": "CPU (NumPy/BLAS)",
            "elapsed_ms": elapsed * 1e3,
            "perf": gflops(flops, elapsed),
        })
    return results


# ── printing ──────────────────────────────────────────────────────────────────────────

def print_results(results: list[dict]) -> None:
    if not results:
        return
    by_name: dict[str, list[dict]] = {}
    for r in results:
        by_name.setdefault(r["name"], []).append(r)

    col_w = 30
    print(f"\n{'Benchmark':<{col_w}}  {'Device':<22}  {'ms':>8}  {'Throughput':>14}")
    print(sep())
    for name, rows in by_name.items():
        for r in rows:
            print(f"{r['name']:<{col_w}}  {r['device']:<22}  {r['elapsed_ms']:>8.2f}  {r['perf']:>14}")
        if len(rows) > 1:
            cpu_row = next((r for r in rows if "CPU" in r["device"] and "NumPy" not in r["device"]), None)
            gpu_row = next((r for r in rows if "GPU" in r["device"]), None)
            if cpu_row and gpu_row:
                speedup = cpu_row["elapsed_ms"] / gpu_row["elapsed_ms"]
                print(f"{'':>{col_w}}  {'→ GPU speedup':<22}  {'':>8}  {speedup:>13.1f}×")
        print()


# ── main ──────────────────────────────────────────────────────────────────────────────

def main() -> None:
    args = parse_args()

    if not TORCH_OK:
        print("PyTorch not installed. Run: ./scripts/setup_bench.sh")
        sys.exit(1)

    cuda_ok = torch.cuda.is_available() and not args.skip_gpu

    cpu = torch.device("cpu")
    gpu = torch.device("cuda") if cuda_ok else None

    print(f"PyTorch {torch.__version__}")
    print(f"CPU threads: {torch.get_num_threads()}")
    if cuda_ok:
        props = torch.cuda.get_device_properties(0)
        print(f"GPU: {props.name}  ({props.total_memory / 1024**3:.1f} GB VRAM, "
              f"{props.multi_processor_count} SMs, compute {props.major}.{props.minor})")
    else:
        print("GPU: not available" + (" (--skip-gpu set)" if args.skip_gpu else ""))

    torch.set_num_threads(torch.get_num_interop_threads())

    reps, warmup = args.reps, args.warmup
    all_results: list[dict] = []
    MATMUL_SIZES = [512, 1024, 2048, 4096]

    print(f"\n{sep()}")
    print("BENCHMARK 1 — Matrix Multiply (SGEMM, float32)")
    print(sep())
    if NP_OK:
        all_results += run_numpy_matmul(MATMUL_SIZES, reps, warmup)
    all_results += run_matmul("CPU (PyTorch)", cpu, MATMUL_SIZES, reps, warmup)
    if cuda_ok:
        all_results += run_matmul("GPU (CUDA)", gpu, MATMUL_SIZES, reps, warmup)
    print_results([r for r in all_results if "matmul" in r["name"].lower() and "np" not in r["name"].lower()])
    if NP_OK:
        print("NumPy/BLAS baseline:")
        print_results([r for r in all_results if "np" in r["name"].lower()])

    print(f"\n{sep()}")
    print("BENCHMARK 2 — Batch Matrix Multiply (attention-shaped)")
    print(sep())
    bmm_cpu = run_batch_matmul("CPU (PyTorch)", cpu, reps, warmup)
    all_results += bmm_cpu
    if cuda_ok:
        bmm_gpu = run_batch_matmul("GPU (CUDA)", gpu, reps, warmup)
        all_results += bmm_gpu
        combined = []
        for c, g in zip(bmm_cpu, bmm_gpu):
            combined += [c, g]
        print_results(combined)
    else:
        print_results(bmm_cpu)

    print(f"\n{sep()}")
    print("BENCHMARK 3 — Element-wise GELU (fused, memory-bound)")
    print(sep())
    ew_cpu = run_elementwise("CPU (PyTorch)", cpu, reps, warmup)
    all_results += ew_cpu
    if cuda_ok:
        ew_gpu = run_elementwise("GPU (CUDA)", gpu, reps, warmup)
        all_results += ew_gpu
        combined = []
        for c, g in zip(ew_cpu, ew_gpu):
            combined += [c, g]
        print_results(combined)
    else:
        print_results(ew_cpu)

    print(f"\n{sep()}")
    print("BENCHMARK 4 — Memory Bandwidth (tensor copy)")
    print(sep())
    bw_cpu = run_bandwidth("CPU (PyTorch)", cpu, reps, warmup)
    all_results += bw_cpu
    if cuda_ok:
        bw_gpu = run_bandwidth("GPU (CUDA)", gpu, reps, warmup)
        all_results += bw_gpu
        combined = []
        for c, g in zip(bw_cpu, bw_gpu):
            combined += [c, g]
        print_results(combined)
    else:
        print_results(bw_cpu)

    if cuda_ok:
        print(f"\n{sep()}")
        print("SPEEDUP SUMMARY  (CPU→GPU, higher = better for GPU)")
        print(sep())
        cpu_rows = {r["name"]: r for r in all_results if r["device"] == "CPU (PyTorch)"}
        gpu_rows = {r["name"]: r for r in all_results if r["device"] == "GPU (CUDA)"}
        speedups = []
        for name in cpu_rows:
            if name in gpu_rows:
                s = cpu_rows[name]["elapsed_ms"] / gpu_rows[name]["elapsed_ms"]
                speedups.append((name, s))

        if speedups:
            speedups.sort(key=lambda x: x[1], reverse=True)
            print(f"  {'Benchmark':<45}  {'Speedup':>8}")
            print("  " + "─" * 55)
            for name, s in speedups:
                bar = "█" * min(int(s), 40)
                print(f"  {name:<45}  {s:>6.1f}×  {bar}")
            avg = sum(s for _, s in speedups) / len(speedups)
            print(f"\n  Average GPU speedup: {avg:.1f}×")


if __name__ == "__main__":
    main()
