# BitNet b1.58 2B-4T through the ternary x int8 kernel — raw results

Produced 2026-10-26 on a shared 4-vCPU Xeon (AVX-512 VNNI, AMX present), rustc 1.99.1,
torch 2.14 CPU, transformers 5.21. Interpretation and caveats: `docs/EXPERIMENTS.md`.

| file | produced by |
|---|---|
| `perplexity_bf16.json` | `scripts/bitnet_eval.py --windows 16 --seqlen 512 --gen-tokens 48` (wikitext-2 test, 8,176 scored tokens) |
| `perplexity_fp32.json` | `scripts/bitnet_eval.py --dtype float32 --windows 4 --seqlen 512` (first 2,044 of those tokens) |
| `arc_easy_300.json` | `scripts/bitnet_eval.py --arc 300 --arc-only` (first 300 ARC-Easy test questions) |
| `decode_shapes_run{1,2}.md` | `cargo bench --bench bitnet_decode`, before the bf16 baseline was added (f32, int8, ternary) |
| `decode_shapes_run{3,4}_with_bf16.md` | the same benchmark with the bf16 row-dot baseline added; these are the headline runs. The machine is shared, so expect run-to-run spread (ternary 1-thread: 54, 86, 65, 59 ms/token over the four runs) |

Model: `microsoft/bitnet-b1.58-2B-4T` (packed checkpoint, 1,178,623,988 bytes).

## Update: thread scaling

The weak four-thread scaling noted above was fixed with a spinning pool: 1.2x with rayon, 2.6-3.1x now (matmul-only). End-to-end in the Hugging Face harness it only helps when torch and the kernel do not share all cores (`--threads 2 --kernel-threads 2`: 7.3-7.8 tok/s against 5.96 before) and is worse with both at 4. See `docs/EXPERIMENTS.md` (last entry) and `decode_spinpool_run*.md`.
