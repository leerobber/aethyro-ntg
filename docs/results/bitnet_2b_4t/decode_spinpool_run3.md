warning: variable does not need to be mutable
   --> kernel/benches/bitnet_decode.rs:383:21
    |
383 |                 let mut pass = |f: &mut dyn FnMut(&Lin)| {
    |                     ----^^^^
    |                     |
    |                     help: remove this `mut`
    |
    = note: `#[warn(unused_mut)]` (part of `#[warn(unused)]`) on by default

warning: `aethyro-ntg-kernel` (bench "bitnet_decode") generated 1 warning (run `cargo fix --bench "bitnet_decode" -p aethyro-ntg-kernel` to apply 1 suggestion)
    Finished `bench` profile [optimized] target(s) in 0.09s
     Running benches/bitnet_decode.rs (/home/user/aethyro-ntg/target/release/deps/bitnet_decode-e33e54fd17c4e470)
# bitnet_decode
BitNet b1.58 2B-4T shapes: 7 matrices x 30 layers = 2.08 B weights; allocated 6 layers (f32 1667 MB, ternary 104 MB) and scaled x5 to 30 layers
per-token weight traffic: f32 8.34 GB, int8 2.08 GB, ternary 521 MB

### batch 1  (milliseconds per token for the 210 linear layers; tok/s = 1000 / ms, linears only)

| threads | method | ms/token | tok/s (linears only) | weight GB/s | vs f32 row-dot | vs bf16 row-dot | vs int8 dense |
|--:|---|--:|--:|--:|--:|--:|--:|
| 1 | f32 row-dot | 793.5 | 1.3 | 10.5 | - | - | - |
| 1 | bf16 row-dot (vdpbf16ps) | 503.2 | 2.0 | 8.3 | 1.6x | - | - |
| 1 | int8 dense VNNI | 192.9 | 5.2 | 10.8 | 4.1x | 2.6x | - |
| 1 | ternary 2-bit (this kernel) | 64.9 | 15.4 | 8.0 | 12.2x | 7.8x | 3.0x |
| 4 | f32 row-dot | 363.9 | 2.7 | 22.9 | - | - | - |
| 4 | bf16 row-dot (vdpbf16ps) | 156.3 | 6.4 | 26.7 | 2.3x | - | - |
| 4 | int8 dense VNNI | 71.5 | 14.0 | 29.1 | 5.1x | 2.2x | - |
| 4 | ternary 2-bit (this kernel) | 41.5 | 24.1 | 12.5 | 8.8x | 3.8x | 1.7x |
| 4 | ternary 2-bit, spin pool | 13.5 | 74.3 | 38.7 | 27.0x | 11.6x | 5.3x |

Ternary-only thread scaling, decode (ms per token for the 210 linear layers, 9 interleaved rounds):

| threads | rayon median (min) | spin median (min) | spin speed-up vs 1 thread (median) |
|--:|--:|--:|--:|
| 1 | 38.6 (32.1) | 38.6 (32.1) | 1.00x |
| 2 | 37.7 (29.8) | 20.3 (17.4) | 1.90x |
| 3 | 30.6 (25.7) | 14.6 (13.5) | 2.63x |
| 4 | 32.2 (29.6) | 14.7 (10.8) | 2.63x |

### batch 8  (milliseconds per token for the 210 linear layers; tok/s = 1000 / ms, linears only)

| threads | method | ms/token | tok/s (linears only) | weight GB/s | vs f32 row-dot | vs bf16 row-dot | vs int8 dense |
|--:|---|--:|--:|--:|--:|--:|--:|
| 1 | f32 row-dot | 2304.3 | 0.4 | 3.6 | - | - | - |
| 1 | bf16 row-dot (vdpbf16ps) | 1462.4 | 0.7 | 2.9 | 1.6x | - | - |
| 1 | int8 dense VNNI | 536.4 | 1.9 | 3.9 | 4.3x | 2.7x | - |
| 1 | ternary 2-bit (this kernel) | 173.7 | 5.8 | 3.0 | 13.3x | 8.4x | 3.1x |

