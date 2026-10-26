# bitnet_decode
BitNet b1.58 2B-4T shapes: 7 matrices x 30 layers = 2.08 B weights; allocated 6 layers (f32 1667 MB, ternary 104 MB) and scaled x5 to 30 layers
per-token weight traffic: f32 8.34 GB, int8 2.08 GB, ternary 521 MB

### batch 1  (milliseconds per token for the 210 linear layers; tok/s = 1000 / ms, linears only)

| threads | method | ms/token | tok/s (linears only) | weight GB/s | vs f32 row-dot | vs int8 dense |
|--:|---|--:|--:|--:|--:|--:|
| 1 | f32 row-dot | 832.3 | 1.2 | 10.0 | - | - |
| 1 | int8 dense VNNI | 227.6 | 4.4 | 9.2 | 3.7x | - |
| 1 | ternary 2-bit (this kernel) | 86.1 | 11.6 | 6.1 | 9.7x | 2.6x |
| 4 | f32 row-dot | 305.4 | 3.3 | 27.3 | - | - |
| 4 | int8 dense VNNI | 116.2 | 8.6 | 17.9 | 2.6x | - |
| 4 | ternary 2-bit (this kernel) | 50.2 | 19.9 | 10.4 | 6.1x | 2.3x |

### batch 8  (milliseconds per token for the 210 linear layers; tok/s = 1000 / ms, linears only)

| threads | method | ms/token | tok/s (linears only) | weight GB/s | vs f32 row-dot | vs int8 dense |
|--:|---|--:|--:|--:|--:|--:|
| 1 | f32 row-dot | 2283.3 | 0.4 | 3.7 | - | - |
| 1 | int8 dense VNNI | 566.6 | 1.8 | 3.7 | 4.0x | - |
| 1 | ternary 2-bit (this kernel) | 198.3 | 5.0 | 2.6 | 11.5x | 2.9x |

