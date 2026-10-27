#!/usr/bin/env python3
"""Run BitNet b1.58 2B-4T through the ternary x int8 kernel and compare with the official path.

The model is Microsoft's natively ternary-trained BitNet b1.58 (microsoft/bitnet-b1.58-2B-4T).
Everything except the 210 BitLinear matmuls (embeddings, RMSNorm, RoPE, attention, squared
ReLU, lm_head) is the official transformers code. In the "kernel" path each BitLinear is
replaced by a call into libntg_kernel (per-row int8 activation quantisation exactly as
PyTorch does it, exact integer GEMM on 2-bit packed weights, dequantisation).

Measures, on wikitext-2 test:
  * perplexity of the official path and of the kernel path,
  * top-1 next-token agreement and mean |delta log-prob| between them,
  * greedy-generation text and decode tokens/s for both,
  * optionally (--arc N) zero-shot ARC-Easy accuracy of both on the same N questions.

Needs: torch, transformers>=5, datasets, a built libntg_kernel.so
(cargo build --release --lib in kernel/), and the model directory from
`huggingface-cli download microsoft/bitnet-b1.58-2B-4T`.

    python3 scripts/bitnet_eval.py --model-dir DIR [--windows 16] [--seqlen 512] [--threads 4]
    python3 scripts/bitnet_eval.py --model-dir DIR --check      # one short window, fast
"""
import argparse
import ctypes
import json
import math
import pathlib
import os
import time

import numpy as np
import torch
from transformers import AutoModelForCausalLM, AutoTokenizer
from transformers.integrations.bitnet import AutoBitLinear

ROOT = pathlib.Path(__file__).resolve().parent.parent


def load_lib():
    path = os.environ.get("NTG_KERNEL_LIB") or ROOT / "target/release/libntg_kernel.so"
    lib = ctypes.CDLL(str(path))
    lib.ntg_ternary_matrix_new.restype = ctypes.c_void_p
    lib.ntg_ternary_matrix_new.argtypes = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_size_t, ctypes.c_float]
    lib.ntg_ternary_matrix_free.argtypes = [ctypes.c_void_p]
    lib.ntg_ternary_matrix_bytes.restype = ctypes.c_size_t
    lib.ntg_ternary_matrix_bytes.argtypes = [ctypes.c_void_p]
    lib.ntg_ternary_forward_f32.restype = ctypes.c_int
    lib.ntg_ternary_forward_f32.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_size_t, ctypes.c_void_p]
    lib.ntg_ternary_set_threads.restype = ctypes.c_int
    lib.ntg_ternary_set_threads.argtypes = [ctypes.c_size_t]
    lib.ntg_ternary_best_kernel.restype = ctypes.c_int
    if hasattr(lib, "ntg_ternary_hold"):  # absent in libraries built before the hold API
        lib.ntg_ternary_hold.restype = ctypes.c_int
        lib.ntg_ternary_hold.argtypes = [ctypes.c_int]
    lib.ntg_int8_matrix_new_bf16.restype = ctypes.c_void_p
    lib.ntg_int8_matrix_new_bf16.argtypes = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_size_t]
    lib.ntg_int8_matrix_free.argtypes = [ctypes.c_void_p]
    lib.ntg_int8_matrix_bytes.restype = ctypes.c_size_t
    lib.ntg_int8_matrix_bytes.argtypes = [ctypes.c_void_p]
    lib.ntg_int8_forward_f32.restype = ctypes.c_int
    lib.ntg_int8_forward_f32.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_size_t, ctypes.c_void_p]
    return lib


class KernelLinear(torch.nn.Module):
    """Drop-in for AutoBitLinear (offline mode) that runs the matmul in libntg_kernel."""

    def __init__(self, lib, ref: AutoBitLinear):
        super().__init__()
        self.lib = lib
        w = ref.weight.detach()
        w_i8 = w.to(torch.int8).contiguous()
        assert bool(((w_i8 >= -1) & (w_i8 <= 1)).all()), "weights are not ternary"
        assert bool((w_i8.to(w.dtype) == w).all()), "weights are not exactly -1/0/1"
        self.rows, self.cols = w_i8.shape
        self.weight_scale = float(ref.weight_scale.float().item())
        self.handle = lib.ntg_ternary_matrix_new(w_i8.data_ptr(), self.rows, self.cols, self.weight_scale)
        assert self.handle, "ntg_ternary_matrix_new rejected the weights"
        self.packed_bytes = lib.ntg_ternary_matrix_bytes(self.handle)
        self.rms_norm = ref.rms_norm
        self.bias = ref.bias

    def forward(self, x):
        if self.rms_norm is not None:
            x = self.rms_norm(x)
        lead = x.shape[:-1]
        xf = x.reshape(-1, self.cols).to(torch.float32).contiguous()
        out = torch.empty(xf.shape[0], self.rows, dtype=torch.float32)
        rc = self.lib.ntg_ternary_forward_f32(self.handle, xf.data_ptr(), xf.shape[0], out.data_ptr())
        assert rc == 0, f"kernel returned {rc}"
        out = out.to(x.dtype).reshape(*lead, self.rows)
        if self.bias is not None:
            out = out + self.bias
        return out

    def __del__(self):
        try:
            self.lib.ntg_ternary_matrix_free(self.handle)
        except Exception:
            pass


class KernelLMHead(torch.nn.Module):
    """The output projection (a bf16 matrix, 0.66 GB for this model) quantised to int8 per
    output row and run through libntg_kernel. Not ternary, so this is a lossy change on top of
    the exact ternary kernel; measure it separately."""

    def __init__(self, lib, ref: torch.nn.Linear):
        super().__init__()
        assert ref.weight.dtype == torch.bfloat16 and ref.bias is None
        self.lib = lib
        w = ref.weight.detach().contiguous()
        self.rows, self.cols = w.shape
        self.handle = lib.ntg_int8_matrix_new_bf16(w.view(torch.int16).data_ptr(), self.rows, self.cols)
        assert self.handle, "ntg_int8_matrix_new_bf16 failed"
        self.packed_bytes = lib.ntg_int8_matrix_bytes(self.handle)

    def forward(self, x):
        lead = x.shape[:-1]
        xf = x.reshape(-1, self.cols).to(torch.float32).contiguous()
        out = torch.empty(xf.shape[0], self.rows, dtype=torch.float32)
        rc = self.lib.ntg_int8_forward_f32(self.handle, xf.data_ptr(), xf.shape[0], out.data_ptr())
        assert rc == 0, f"int8 kernel returned {rc}"
        return out.to(x.dtype).reshape(*lead, self.rows)

    def __del__(self):
        try:
            self.lib.ntg_int8_matrix_free(self.handle)
        except Exception:
            pass


def hold_pool_during_forward(model, lib):
    """Keep the kernel's worker threads awake for the whole of each forward pass (they spin
    through the torch ops between matmuls), and let them sleep between passes."""
    def hold(on):
        def hook(*_):
            lib.ntg_ternary_hold(on)  # a hook must return None, or torch replaces the inputs/outputs

        return hook

    model.register_forward_pre_hook(hold(1))
    model.register_forward_hook(hold(0))


def swap_linears(model, lib):
    swaps = [(n, m) for n, m in model.named_modules() if isinstance(m, AutoBitLinear)]
    f32_bytes = packed = 0
    for name, mod in swaps:
        new = KernelLinear(lib, mod)
        f32_bytes += mod.weight.numel() * 4
        packed += new.packed_bytes
        model.set_submodule(name, new)
    return len(swaps), f32_bytes, packed


@torch.no_grad()
def eval_windows(model, windows):
    """Per-token target log-probs and argmax for each window."""
    logps, tops = [], []
    for ids in windows:
        logits = model(ids.unsqueeze(0)).logits[0, :-1].float()
        lp = torch.log_softmax(logits, dim=-1)
        tgt = ids[1:]
        logps.append(lp.gather(1, tgt.unsqueeze(1)).squeeze(1))
        tops.append(lp.argmax(dim=-1))
    return torch.cat(logps), torch.cat(tops)


def load_arc(n):
    from datasets import load_dataset

    ds = load_dataset("allenai/ai2_arc", "ARC-Easy", split="test")
    return [ds[i] for i in range(n)]


@torch.no_grad()
def arc_eval(model, tok, items):
    """Zero-shot multiple choice in the lm-evaluation-harness style: score each choice by the
    log-likelihood of " <choice>" after "Question: ...\nAnswer:". Returns per-question
    (picked by sum, picked by sum/char-length, gold index)."""
    picks, picks_norm, gold = [], [], []
    for it in items:
        ctx = f"Question: {it['question']}\nAnswer:"
        ctx_ids = tok(ctx).input_ids
        raw, norm = [], []
        for ch in it["choices"]["text"]:
            whole = tok(ctx + " " + ch).input_ids
            ids = torch.tensor([whole])
            lp = torch.log_softmax(model(ids).logits[0].float(), dim=-1)
            tgt = torch.tensor(whole[len(ctx_ids):])
            total = float(lp[len(ctx_ids) - 1 : len(whole) - 1].gather(1, tgt.unsqueeze(1)).sum())
            raw.append(total)
            norm.append(total / len(ch))
        picks.append(int(np.argmax(raw)))
        picks_norm.append(int(np.argmax(norm)))
        gold.append(it["choices"]["label"].index(it["answerKey"]))
    return np.array(picks), np.array(picks_norm), np.array(gold)


def ppl(logps):
    return math.exp(-float(logps.mean()))


@torch.no_grad()
def generate(model, tok, prompt, new_tokens):
    ids = tok(prompt, return_tensors="pt").input_ids
    t0 = time.perf_counter()
    out = model.generate(ids, max_new_tokens=new_tokens, do_sample=False, use_cache=True)
    dt = time.perf_counter() - t0
    gen = out[0, ids.shape[1]:]
    return tok.decode(gen, skip_special_tokens=True), len(gen), dt


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--model-dir", required=True)
    ap.add_argument("--windows", type=int, default=16)
    ap.add_argument("--seqlen", type=int, default=512)
    ap.add_argument("--threads", type=int, default=4, help="torch intra-op threads")
    ap.add_argument("--kernel-threads", type=int, default=None,
                    help="threads for the ternary kernel pool (default: --threads). Torch's own threads and a\n                    spinning kernel pool compete for cores; try --threads 1 --kernel-threads N")
    ap.add_argument("--gen-tokens", type=int, default=48)
    ap.add_argument("--check", action="store_true", help="one short window, no generation")
    ap.add_argument("--hold-pool", action="store_true",
                    help="keep the kernel threads spinning through each forward pass (use when torch threads + kernel threads - 1 <= cores)")
    ap.add_argument("--int8-lm-head", action="store_true",
                    help="also run the output projection through the int8 kernel (lossy; not ternary)")
    ap.add_argument("--arc", type=int, default=0, help="also run N zero-shot ARC-Easy questions")
    ap.add_argument("--arc-only", action="store_true", help="skip perplexity and generation")
    ap.add_argument("--dtype", choices=["bfloat16", "float32"], default="bfloat16",
                    help="float32 removes bf16 rounding from the official path, isolating the kernel's own error")
    ap.add_argument("--save-logps", help="save per-token log-probs of both paths to this .npz")
    ap.add_argument("--out", help="write results JSON here")
    args = ap.parse_args()
    if args.check:
        args.windows, args.seqlen, args.gen_tokens = 1, 64, 0

    torch.set_num_threads(args.threads)
    lib = load_lib()
    kthreads = args.kernel_threads or args.threads
    assert lib.ntg_ternary_set_threads(kthreads) == 0
    kernel_names = {0: "scalar", 1: "avx2", 2: "avx512-vnni"}
    print(f"kernel: {kernel_names[lib.ntg_ternary_best_kernel()]}; threads: torch {args.threads}, kernel {kthreads}", flush=True)

    tok = AutoTokenizer.from_pretrained(args.model_dir)
    t0 = time.perf_counter()
    # The official loader keeps unpacked BitLinear weights in bf16 whatever dtype is requested,
    # so load in bf16 and widen afterwards. The conversion is exact (weights are -1/0/1 and
    # every other parameter is already bf16), so this is the same model in float32 arithmetic.
    model = AutoModelForCausalLM.from_pretrained(args.model_dir, dtype=torch.bfloat16)
    if args.dtype == "float32":
        torch.nn.Module.float(model)  # transformers' own .float() refuses quantized models
    model.eval()
    print(f"loaded official model in {time.perf_counter() - t0:.0f}s", flush=True)
    n_bit = sum(isinstance(m, AutoBitLinear) for m in model.modules())
    print(f"BitLinear modules: {n_bit}", flush=True)

    from datasets import load_dataset

    arc_items = load_arc(args.arc) if args.arc else []
    if args.arc_only:
        args.windows = 0
        args.gen_tokens = 0
    text = "\n\n".join(load_dataset("Salesforce/wikitext", "wikitext-2-raw-v1", split="test")["text"])
    ids = tok(text, return_tensors="pt").input_ids[0]
    ids = ids[: args.windows * args.seqlen]
    windows = [ids[i * args.seqlen : (i + 1) * args.seqlen] for i in range(args.windows)]
    print(f"{len(windows)} windows x {args.seqlen} tokens = {sum(len(w) - 1 for w in windows)} scored tokens", flush=True)

    prompt = "The capital of France is"
    results = {"model": "microsoft/bitnet-b1.58-2B-4T", "windows": len(windows), "seqlen": args.seqlen,
               "threads": args.threads, "kernel_threads": kthreads, "int8_lm_head": args.int8_lm_head, "dtype": args.dtype,
               "kernel": kernel_names[lib.ntg_ternary_best_kernel()]}

    results["official"], results["kernel_path"] = {}, {}
    do_ppl = len(windows) > 0

    def arc_section(label, store):
        t = time.perf_counter()
        picks, picks_norm, gold = arc_eval(model, tok, arc_items)
        store["arc_easy_acc"] = float((picks == gold).mean())
        store["arc_easy_acc_norm"] = float((picks_norm == gold).mean())
        store["arc_s"] = time.perf_counter() - t
        print(f"{label}: ARC-Easy ({len(gold)} q) acc {store['arc_easy_acc']:.4f}  acc_norm {store['arc_easy_acc_norm']:.4f}"
              f"  ({store['arc_s']:.0f}s)", flush=True)
        return picks, picks_norm

    # ---- official path
    if do_ppl:
        t0 = time.perf_counter()
        lp_ref, top_ref = eval_windows(model, windows)
        results["official"].update(ppl=ppl(lp_ref), eval_s=time.perf_counter() - t0)
        print(f"official : ppl {results['official']['ppl']:.4f}  ({results['official']['eval_s']:.0f}s)", flush=True)
    if args.gen_tokens:
        text_ref, n, dt = generate(model, tok, prompt, args.gen_tokens)
        results["official"].update(gen_text=text_ref, gen_tok_s=n / dt)
        print(f"official : {n / dt:.2f} tok/s  | {text_ref!r}", flush=True)
    if arc_items:
        arc_o = arc_section("official ", results["official"])

    # ---- kernel path
    n_swapped, f32_bytes, packed = swap_linears(model, lib)
    if args.hold_pool:
        hold_pool_during_forward(model, lib)
    results["swapped_linears"] = n_swapped
    if args.int8_lm_head:
        head = KernelLMHead(lib, model.lm_head)
        model.lm_head = head
        results["int8_lm_head_bytes"] = head.packed_bytes
        print(f"int8 lm_head: bf16 {head.rows * head.cols * 2 / 1e9:.2f} GB -> int8 {head.packed_bytes / 1e9:.2f} GB", flush=True)
    results["linear_weight_bytes_f32"] = f32_bytes
    results["linear_weight_bytes_packed"] = packed
    print(f"swapped {n_swapped} linears: f32 {f32_bytes / 1e9:.2f} GB -> packed {packed / 1e9:.3f} GB", flush=True)
    if do_ppl:
        t0 = time.perf_counter()
        lp_k, top_k = eval_windows(model, windows)
        results["kernel_path"].update(ppl=ppl(lp_k), eval_s=time.perf_counter() - t0)
        print(f"kernel   : ppl {results['kernel_path']['ppl']:.4f}  ({results['kernel_path']['eval_s']:.0f}s)", flush=True)
        results["top1_agreement"] = float((top_ref == top_k).float().mean())
        results["mean_abs_logprob_diff"] = float((lp_ref - lp_k).abs().mean())
        results["max_abs_logprob_diff"] = float((lp_ref - lp_k).abs().max())
        print(f"top-1 agreement {results['top1_agreement']:.4f}; mean |dlogp| {results['mean_abs_logprob_diff']:.5f}; "
              f"max |dlogp| {results['max_abs_logprob_diff']:.4f}", flush=True)
    if args.gen_tokens:
        text_k, n, dt = generate(model, tok, prompt, args.gen_tokens)
        results["kernel_path"].update(gen_text=text_k, gen_tok_s=n / dt)
        print(f"kernel   : {n / dt:.2f} tok/s  | {text_k!r}", flush=True)
        results["generation_identical"] = text_k == results["official"]["gen_text"]
        print(f"greedy generations identical: {results['generation_identical']}", flush=True)
    if arc_items:
        arc_k = arc_section("kernel   ", results["kernel_path"])
        results["arc_easy_same_pick_fraction"] = float((arc_o[0] == arc_k[0]).mean())
        print(f"ARC-Easy: both paths pick the same answer on {results['arc_easy_same_pick_fraction']:.3f} of questions", flush=True)

    if args.save_logps and do_ppl:
        np.savez(args.save_logps, official=lp_ref.numpy(), kernel=lp_k.numpy(),
                 top_official=top_ref.numpy(), top_kernel=top_k.numpy())
    if args.out:
        pathlib.Path(args.out).write_text(json.dumps(results, indent=2))
        print(f"wrote {args.out}")


if __name__ == "__main__":
    main()
