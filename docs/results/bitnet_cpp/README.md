# bitnet.cpp vs the ternary x int8 kernel on BitNet b1.58 2B-4T (2026-10-26)

Same machine, same model, same session as `docs/results/bitnet_2b_4t/`: a shared 4-vCPU
Intel Xeon @ 2.1 GHz VM (AVX-512 VNNI and AMX present), 15 GB RAM. Not tatortot.

## What was run

* **bitnet.cpp**: `microsoft/BitNet` at llama.cpp fork commit `390c30775` (build 9918),
  built by its own `setup_env.py -q i2_s` with the system clang, no changes.
  Model: the official `microsoft/BitNet-b1.58-2B-4T-gguf` `ggml-model-i2_s.gguf` (1.10 GiB,
  includes the f16 embedding / output matrix). Measured with its `llama-bench`,
  `-p 128 -n 64 -r 5`, twice.
* **This repo**: `scripts/bitnet_eval.py` (Hugging Face `generate`, greedy, 64 new tokens,
  prompt about Paris) with the 210 BitLinear layers swapped to `ternary_i8_gemm`.
  One run per thread count.

## Results (tokens/s, higher is better)

| | 1 thread | 2 threads | 4 threads |
|---|--:|--:|--:|
| bitnet.cpp decode (tg64) | 6.67, 6.77 | 12.09, 12.58 | 21.67, 20.86 |
| bitnet.cpp prefill (pp128) | 57.1, 56.7 | 102.7, 99.4 | 187.1, 170.5 |
| this kernel in HF `generate`, decode | 5.17 | not run | 5.96 |
| official HF path (bf16), decode | 1.45 | not run | 3.86 |
| this kernel, prefill (512-token windows) | not run | not run | about 62 (133 s for 8,192 tokens) |
| official HF path, prefill | not run | not run | about 205 (40 s for 8,192 tokens) |

A first bitnet.cpp run read 4 threads as 14.4 +/- 12.1 tok/s decode and slower prefill than
at one thread; the two repeats above were stable (+/- 0.6). The VM is noisy; use the repeats.

## What this does and does not show

* **Whole model, bitnet.cpp is faster than my harness**: 1.3x at one thread, about 3.5x at
  four. bitnet.cpp's decode scales 3.2x from one to four threads; the ternary kernel's
  matmuls scaled 1.5x (see `docs/EXPERIMENTS.md`), so thread scaling is the biggest gap.
* **This is not a kernel-against-kernel ratio.** My path pays Python and Hugging Face
  overhead: at one thread a token takes about 193 ms of which the 210 linears are 59-65 ms.
  bitnet.cpp's 148 ms per token at one thread includes its own attention and the f16
  output matrix (a 0.66 GB read per token; torch needs 66 ms for the same matrix in bf16 at
  one thread, ggml's number is unknown). I could not split bitnet.cpp's time by operator
  (no `perf`; `gdb` sampling returned no symbols), so I cannot say whether its I2_S matmul
  or mine is faster per layer. The honest answer to that question is still "unknown".
* **Prefill**: bitnet.cpp (170-187 tok/s at four threads, 128-token prompt) is about on par
  with torch's bf16 path (about 205 tok/s, 512-token windows) and about 3x faster than my
  kernel (about 62 tok/s). Different prompt lengths, so approximate.
* **Accuracy of bitnet.cpp was not measured**; I only checked that it generates text. Its
  activation quantisation may differ from the official model's.
* Single machine, small repeat counts, no tatortot run.

## Reproduce

```bash
git clone --recursive https://github.com/microsoft/BitNet && cd BitNet
python3 setup_env.py -md <dir containing ggml-model-i2_s.gguf> -q i2_s
./build/bin/llama-bench -m <that .gguf> -p 128 -n 64 -t 1,2,4 -r 5
python3 scripts/bitnet_eval.py --model-dir <HF model dir> --windows 1 --threads 4 --gen-tokens 64
```

Installing BitNet's `requirements.txt` downgrades `numpy` and `transformers` in a shared
Python environment and breaks `bitnet_eval.py`; use a separate virtualenv.
