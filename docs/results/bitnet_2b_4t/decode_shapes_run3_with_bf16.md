# bitnet_decode
BitNet b1.58 2B-4T shapes: 7 matrices x 30 layers = 2.08 B weights; allocated 6 layers (f32 1667 MB, ternary 104 MB) and scaled x5 to 30 layers
per-token weight traffic: f32 8.34 GB, int8 2.08 GB, ternary 521 MB

### batch 1  (milliseconds per token for the 210 linear layers; tok/s = 1000 / ms, linears only)

| threads | method | ms/token | tok/s (linears only) | weight GB/s | vs f32 row-dot | vs bf16 row-dot | vs int8 dense |
|--:|---|--:|--:|--:|--:|--:|--:|
| 1 | f32 row-dot | 758.2 | 1.3 | 11.0 | - | - | - |
| 1 | bf16 row-dot (vdpbf16ps) | 500.1 | 2.0 | 8.3 | 1.5x | - | - |
| 1 | int8 dense VNNI | 194.6 | 5.1 | 10.7 | 3.9x | 2.6x | - |
| 1 | ternary 2-bit (this kernel) | 65.2 | 15.3 | 8.0 | 11.6x | 7.7x | 3.0x |
| 4 | f32 row-dot | 271.6 | 3.7 | 30.7 | - | - | - |
| 4 | bf16 row-dot (vdpbf16ps) | 172.6 | 5.8 | 24.2 | 1.6x | - | - |
| 4 | int8 dense VNNI | 74.2 | 13.5 | 28.1 | 3.7x | 2.3x | - |
| 4 | ternary 2-bit (this kernel) | 40.6 | 24.7 | 12.8 | 6.7x | 4.3x | 1.8x |

### batch 8  (milliseconds per token for the 210 linear layers; tok/s = 1000 / ms, linears only)

| threads | method | ms/token | tok/s (linears only) | weight GB/s | vs f32 row-dot | vs bf16 row-dot | vs int8 dense |
|--:|---|--:|--:|--:|--:|--:|--:|
| 1 | f32 row-dot | 2290.8 | 0.4 | 3.6 | - | - | - |
| 1 | bf16 row-dot (vdpbf16ps) | 1332.4 | 0.8 | 3.1 | 1.7x | - | - |
| 1 | int8 dense VNNI | 554.9 | 1.8 | 3.8 | 4.1x | 2.4x | - |
| 1 | ternary 2-bit (this kernel) | 161.4 | 6.2 | 3.2 | 14.2x | 8.3x | 3.4x |

