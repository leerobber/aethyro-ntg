"""Time one decode token's worth of ternary matmuls (BitNet 2B-4T shapes, 6 layers cycled and
scaled to 30) through the C ABI with no torch in the loop, at 1, 2 and 4 kernel threads.
Compare with `cargo bench --bench bitnet_decode` (same matmuls, no C ABI). Idle-spin budget in
microseconds: NTG_SPIN_US (default 200). NTG_KERNEL_LIB points at another build of the library
(for A/B runs). `--fixed` adds a per-call fixed-cost measurement. Needs `cargo build --release` first.
"""
import pathlib, sys, time

import numpy as np

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import bitnet_eval as be
lib = be.load_lib()
SHAPES = [(2560,2560),(2560,640),(2560,640),(2560,2560),(2560,6912),(2560,6912),(6912,2560)]  # (in,out)
rng = np.random.default_rng(1)
mats = []
for layer in range(6):
    for (i, o) in SHAPES:
        w = rng.integers(-1, 2, size=(o, i), dtype=np.int8)
        h = lib.ntg_ternary_matrix_new(w.ctypes.data, o, i, 0.01)
        assert h
        mats.append((h, i, o, rng.standard_normal((1, i)).astype(np.float32), np.empty((1, o), np.float32)))
        del w
print(f"{len(mats)} matrices")
for K in (1, 2, 4):
    assert lib.ntg_ternary_set_threads(K) == 0
    def tok():
        t = time.perf_counter()
        for h, i, o, x, out in mats:
            rc = lib.ntg_ternary_forward_f32(h, x.ctypes.data, 1, out.ctypes.data)
            assert rc == 0
        return (time.perf_counter() - t) * 1e3 * 30 / 6   # scale 6 layers -> 30
    tok(); tok()
    s = sorted(tok() for _ in range(15))
    print(f"threads {K}: ms/token for 210 linears via ctypes FFI: median {s[len(s)//2]:.1f}  min {s[0]:.1f}  max {s[-1]:.1f}")

if "--fixed" in sys.argv:
    # Per-call fixed cost: a tiny output matrix, so the activation quantiser and the call itself
    # dominate (one thread).
    assert lib.ntg_ternary_set_threads(1) == 0
    for cols in (2560, 6912):
        w = rng.integers(-1, 2, size=(8, cols), dtype=np.int8)
        h = lib.ntg_ternary_matrix_new(w.ctypes.data, 8, cols, 0.01)
        x = rng.standard_normal((1, cols)).astype(np.float32)
        out = np.empty((1, 8), np.float32)
        for _ in range(50):
            lib.ntg_ternary_forward_f32(h, x.ctypes.data, 1, out.ctypes.data)
        n = 3000
        t0 = time.perf_counter()
        for _ in range(n):
            lib.ntg_ternary_forward_f32(h, x.ctypes.data, 1, out.ctypes.data)
        print(f"fixed cost, {cols} inputs, 8 outputs: {(time.perf_counter() - t0) / n * 1e6:.1f} us per call")

