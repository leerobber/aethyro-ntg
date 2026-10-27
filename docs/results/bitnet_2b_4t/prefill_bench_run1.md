   Compiling aethyro-ntg-kernel v0.1.0 (/home/user/aethyro-ntg/kernel)
    Finished `bench` profile [optimized] target(s) in 15.95s
     Running benches/bitnet_prefill.rs (/home/user/aethyro-ntg/target/release/deps/bitnet_prefill-2ab5874e593f77af)
# bitnet_prefill
kernel: avx512-vnni; one layer = 69.5 M weights; whole model = 30 layers
VNNI register-only ceiling, one core: 242 GMAC/s

### batch 8  (one layer's seven products; whole-model prefill = linears only)

| method | ms per layer | GMAC/s | % of 1-core ceiling | prefill tok/s (30 layers) |
|---|--:|--:|--:|--:|
| int8 dense VNNI control, 1 thread | 18.5 | 30.0 | 12% | 14 |
| ternary row layout, 1 thread | 6.7 | 82.8 | 34% | 40 |
| ternary row layout, 4 threads | 1.9 | 292.0 | 30% | 140 |
| ternary panel kernel, 1 thread | 4.1 | 137.2 | 57% | 66 |
| ternary panel kernel, 4 threads | 1.1 | 483.3 | 50% | 232 |

### batch 32  (one layer's seven products; whole-model prefill = linears only)

| method | ms per layer | GMAC/s | % of 1-core ceiling | prefill tok/s (30 layers) |
|---|--:|--:|--:|--:|
| int8 dense VNNI control, 1 thread | 48.7 | 45.6 | 19% | 22 |
| ternary row layout, 1 thread | 27.1 | 81.9 | 34% | 39 |
| ternary row layout, 4 threads | 7.1 | 311.8 | 32% | 150 |
| ternary panel kernel, 1 thread | 13.6 | 163.7 | 68% | 79 |
| ternary panel kernel, 4 threads | 3.8 | 584.6 | 60% | 280 |

### batch 128  (one layer's seven products; whole-model prefill = linears only)

| method | ms per layer | GMAC/s | % of 1-core ceiling | prefill tok/s (30 layers) |
|---|--:|--:|--:|--:|
| int8 dense VNNI control, 1 thread | 178.1 | 49.9 | 21% | 24 |
| ternary row layout, 1 thread | 114.2 | 77.9 | 32% | 37 |
| ternary row layout, 4 threads | 34.5 | 257.7 | 27% | 124 |
| ternary panel kernel, 1 thread | 54.3 | 163.9 | 68% | 79 |
| ternary panel kernel, 4 threads | 15.1 | 590.7 | 61% | 283 |

### batch 512  (one layer's seven products; whole-model prefill = linears only)

| method | ms per layer | GMAC/s | % of 1-core ceiling | prefill tok/s (30 layers) |
|---|--:|--:|--:|--:|
| int8 dense VNNI control, 1 thread | 686.4 | 51.8 | 21% | 25 |
| ternary row layout, 1 thread | 443.0 | 80.3 | 33% | 39 |
| ternary row layout, 4 threads | 114.4 | 310.8 | 32% | 149 |
| ternary panel kernel, 1 thread | 228.7 | 155.5 | 64% | 75 |
| ternary panel kernel, 4 threads | 66.0 | 538.8 | 56% | 259 |

