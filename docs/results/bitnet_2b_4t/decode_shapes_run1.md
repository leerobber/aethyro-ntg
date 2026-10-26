# bitnet_decode
BitNet b1.58 2B-4T shapes: 7 matrices x 30 layers = 2.08 B weights; allocated 6 layers (f32 1667 MB, ternary 104 MB) and scaled x5 to 30 layers
per-token weight traffic: f32 8.34 GB, int8 2.08 GB, ternary 521 MB

### batch 1  (milliseconds per token for the 210 linear layers; tok/s = 1000 / ms, linears only)

| threads | method | ms/token | tok/s (linears only) | weight GB/s | vs f32 row-dot | vs int8 dense |
|--:|---|--:|--:|--:|--:|--:|
| 1 | f32 row-dot | 852.0 | 1.2 | 9.8 | - | - |
| 1 | int8 dense VNNI | 200.1 | 5.0 | 10.4 | 4.3x | - |
| 1 | ternary 2-bit (this kernel) | 53.8 | 18.6 | 9.7 | 15.8x | 3.7x |
| 4 | f32 row-dot | 286.8 | 3.5 | 29.1 | - | - |
| 4 | int8 dense VNNI | 81.9 | 12.2 | 25.5 | 3.5x | - |
| 4 | ternary 2-bit (this kernel) | 48.4 | 20.7 | 10.8 | 5.9x | 1.7x |

### batch 8  (milliseconds per token for the 210 linear layers; tok/s = 1000 / ms, linears only)

| threads | method | ms/token | tok/s (linears only) | weight GB/s | vs f32 row-dot | vs int8 dense |
|--:|---|--:|--:|--:|--:|--:|
| 1 | f32 row-dot | 2339.6 | 0.4 | 3.6 | - | - |
| 1 | int8 dense VNNI | 574.9 | 1.7 | 3.6 | 4.1x | - |
| 1 | ternary 2-bit (this kernel) | 240.5 | 4.2 | 2.2 | 9.7x | 2.4x |

