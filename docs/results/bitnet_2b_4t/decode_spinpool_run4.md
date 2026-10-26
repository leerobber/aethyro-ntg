# bitnet_decode
BitNet b1.58 2B-4T shapes: 7 matrices x 30 layers = 2.08 B weights; allocated 6 layers (f32 1667 MB, ternary 104 MB) and scaled x5 to 30 layers
per-token weight traffic: f32 8.34 GB, int8 2.08 GB, ternary 521 MB

### batch 1  (milliseconds per token for the 210 linear layers; tok/s = 1000 / ms, linears only)

| threads | method | ms/token | tok/s (linears only) | weight GB/s | vs f32 row-dot | vs bf16 row-dot | vs int8 dense |
|--:|---|--:|--:|--:|--:|--:|--:|
| 1 | f32 row-dot | 859.0 | 1.2 | 9.7 | - | - | - |
| 1 | bf16 row-dot (vdpbf16ps) | 522.3 | 1.9 | 8.0 | 1.6x | - | - |
| 1 | int8 dense VNNI | 191.6 | 5.2 | 10.9 | 4.5x | 2.7x | - |
| 1 | ternary 2-bit (this kernel) | 77.4 | 12.9 | 6.7 | 11.1x | 6.7x | 2.5x |
| 4 | f32 row-dot | 342.5 | 2.9 | 24.3 | - | - | - |
| 4 | bf16 row-dot (vdpbf16ps) | 182.0 | 5.5 | 22.9 | 1.9x | - | - |
| 4 | int8 dense VNNI | 91.2 | 11.0 | 22.9 | 3.8x | 2.0x | - |
| 4 | ternary 2-bit (this kernel) | 45.1 | 22.2 | 11.6 | 7.6x | 4.0x | 2.0x |
| 4 | ternary 2-bit, spin pool | 18.0 | 55.6 | 29.0 | 19.0x | 10.1x | 5.1x |

Ternary-only thread scaling, decode (ms per token for the 210 linear layers, 9 interleaved rounds):

| threads | rayon median (min) | spin median (min) | spin speed-up vs 1 thread (median) |
|--:|--:|--:|--:|
| 1 | 46.6 (36.7) | 46.6 (36.7) | 1.00x |
| 2 | 42.1 (30.9) | 24.6 (21.3) | 1.89x |
| 3 | 36.1 (27.0) | 18.0 (15.8) | 2.59x |
| 4 | 37.8 (30.2) | 14.8 (11.7) | 3.14x |

### batch 8  (milliseconds per token for the 210 linear layers; tok/s = 1000 / ms, linears only)

| threads | method | ms/token | tok/s (linears only) | weight GB/s | vs f32 row-dot | vs bf16 row-dot | vs int8 dense |
|--:|---|--:|--:|--:|--:|--:|--:|
| 1 | f32 row-dot | 2358.2 | 0.4 | 3.5 | - | - | - |
| 1 | bf16 row-dot (vdpbf16ps) | 1468.9 | 0.7 | 2.8 | 1.6x | - | - |
| 1 | int8 dense VNNI | 565.6 | 1.8 | 3.7 | 4.2x | 2.6x | - |
| 1 | ternary 2-bit (this kernel) | 210.5 | 4.8 | 2.5 | 11.2x | 7.0x | 2.7x |

