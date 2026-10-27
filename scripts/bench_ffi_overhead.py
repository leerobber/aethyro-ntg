"""Time one decode token's worth of ternary matmuls (BitNet 2B-4T shapes, 6 layers cycled and
scaled to 30) through the C ABI with no torch in the loop, at 1, 2 and 4 kernel threads.
Compare with `cargo bench --bench bitnet_decode` (same matmuls, no C ABI). Idle-spin budget:
NTG_SPIN_ITERS / NTG_YIELD_ITERS. Needs `cargo build --release` first.
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
