   Compiling aethyro-ntg-kernel v0.1.0 (/home/user/aethyro-ntg/kernel)
    Finished `bench` profile [optimized] target(s) in 15.68s
     Running benches/bitnet_decode.rs (/home/user/aethyro-ntg/target/release/deps/bitnet_decode-e33e54fd17c4e470)
# bitnet_decode
BitNet b1.58 2B-4T shapes: 7 matrices x 30 layers = 2.08 B weights; allocated 6 layers (f32 1667 MB, ternary 104 MB) and scaled x5 to 30 layers
per-token weight traffic: f32 8.34 GB, int8 2.08 GB, ternary 521 MB

### batch 1  (milliseconds per token for the 210 linear layers; tok/s = 1000 / ms, linears only)

| threads | method | ms/token | tok/s (linears only) | weight GB/s | vs f32 row-dot | vs bf16 row-dot | vs int8 dense |
|--:|---|--:|--:|--:|--:|--:|--:|
| 1 | f32 row-dot | 812.8 | 1.2 | 10.3 | - | - | - |
| 1 | bf16 row-dot (vdpbf16ps) | 476.4 | 2.1 | 8.7 | 1.7x | - | - |
| 1 | int8 dense VNNI | 180.6 | 5.5 | 11.5 | 4.5x | 2.6x | - |
| 1 | ternary 2-bit (this kernel) | 53.9 | 18.5 | 9.7 | 15.1x | 8.8x | 3.3x |
| 4 | f32 row-dot | 233.6 | 4.3 | 35.7 | - | - | - |
| 4 | bf16 row-dot (vdpbf16ps) | 151.4 | 6.6 | 27.5 | 1.5x | - | - |
| 4 | int8 dense VNNI | 65.2 | 15.3 | 32.0 | 3.6x | 2.3x | - |
| 4 | ternary 2-bit (this kernel) | 38.8 | 25.8 | 13.4 | 6.0x | 3.9x | 1.7x |
| 4 | ternary 2-bit, spin pool | 19.4 | 51.6 | 26.9 | 12.1x | 7.8x | 3.4x |

Ternary-only thread scaling, decode (ms per token for the 210 linear layers, 9 interleaved rounds):

| threads | rayon median (min) | spin median (min) | spin speed-up vs 1 thread (median) |
|--:|--:|--:|--:|
| 1 | 33.0 (32.1) | 33.0 (32.1) | 1.00x |
| 2 | 34.1 (27.2) | 20.6 (17.1) | 1.60x |
| 3 | 28.1 (26.5) | 15.3 (12.6) | 2.15x |
| 4 | 31.7 (28.4) | 12.6 (11.5) | 2.61x |

### batch 8  (milliseconds per token for the 210 linear layers; tok/s = 1000 / ms, linears only)

| threads | method | ms/token | tok/s (linears only) | weight GB/s | vs f32 row-dot | vs bf16 row-dot | vs int8 dense |
|--:|---|--:|--:|--:|--:|--:|--:|
| 1 | f32 row-dot | 2346.0 | 0.4 | 3.6 | - | - | - |
| 1 | bf16 row-dot (vdpbf16ps) | 1472.0 | 0.7 | 2.8 | 1.6x | - | - |
| 1 | int8 dense VNNI | 601.0 | 1.7 | 3.5 | 3.9x | 2.4x | - |
| 1 | ternary 2-bit (this kernel) | 218.6 | 4.6 | 2.4 | 10.7x | 6.7x | 2.7x |

