# bitnet_decode
BitNet b1.58 2B-4T shapes: 7 matrices x 30 layers = 2.08 B weights; allocated 6 layers (f32 1667 MB, ternary 104 MB) and scaled x5 to 30 layers
per-token weight traffic: f32 8.34 GB, int8 2.08 GB, ternary 521 MB

### batch 1  (milliseconds per token for the 210 linear layers; tok/s = 1000 / ms, linears only)

| threads | method | ms/token | tok/s (linears only) | weight GB/s | vs f32 row-dot | vs bf16 row-dot | vs int8 dense |
|--:|---|--:|--:|--:|--:|--:|--:|
| 1 | f32 row-dot | 756.0 | 1.3 | 11.0 | - | - | - |
| 1 | bf16 row-dot (vdpbf16ps) | 502.7 | 2.0 | 8.3 | 1.5x | - | - |
| 1 | int8 dense VNNI | 197.4 | 5.1 | 10.6 | 3.8x | 2.5x | - |
| 1 | ternary 2-bit (this kernel) | 59.0 | 17.0 | 8.8 | 12.8x | 8.5x | 3.3x |
| 4 | f32 row-dot | 263.8 | 3.8 | 31.6 | - | - | - |
| 4 | bf16 row-dot (vdpbf16ps) | 168.8 | 5.9 | 24.7 | 1.6x | - | - |
| 4 | int8 dense VNNI | 73.7 | 13.6 | 28.3 | 3.6x | 2.3x | - |
| 4 | ternary 2-bit (this kernel) | 39.6 | 25.3 | 13.2 | 6.7x | 4.3x | 1.9x |

### batch 8  (milliseconds per token for the 210 linear layers; tok/s = 1000 / ms, linears only)

| threads | method | ms/token | tok/s (linears only) | weight GB/s | vs f32 row-dot | vs bf16 row-dot | vs int8 dense |
|--:|---|--:|--:|--:|--:|--:|--:|
| 1 | f32 row-dot | 2326.4 | 0.4 | 3.6 | - | - | - |
| 1 | bf16 row-dot (vdpbf16ps) | 1459.3 | 0.7 | 2.9 | 1.6x | - | - |
| 1 | int8 dense VNNI | 566.3 | 1.8 | 3.7 | 4.1x | 2.6x | - |
| 1 | ternary 2-bit (this kernel) | 205.2 | 4.9 | 2.5 | 11.3x | 7.1x | 2.8x |

