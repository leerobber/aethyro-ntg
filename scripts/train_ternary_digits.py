#!/usr/bin/env python3
"""Train a small MLP on scikit-learn's digits three ways and export the ternary one.

  f32   : ordinary float network (the accuracy ceiling for this architecture)
  qat   : ternary weights (BitNet absmean) + int8 activations, trained with a
          straight-through estimator
  ptq   : the f32 network ternarised after training, no retraining (shows why QAT matters)

The QAT model is exported to kernel/tests/data/ternary_digits_mlp.json together with
integer reference values, so kernel/tests/ternary_trained_mlp.rs can check that the Rust
kernel reproduces the Python integer pipeline exactly. Needs numpy and scikit-learn;
the exported JSON is committed, so CI does not.

    python3 scripts/train_ternary_digits.py
"""
import json
import pathlib

import numpy as np
from sklearn.datasets import load_digits
from sklearn.model_selection import train_test_split

SEED = 0
HIDDEN = 128
EPOCHS = 300
BATCH = 64
LR = 3e-3
F32 = np.float32
OUT = pathlib.Path(__file__).resolve().parent.parent / "kernel/tests/data/ternary_digits_mlp.json"


def rhaz(v):
    """Round half away from zero (matches Rust's f32::round)."""
    return np.sign(v) * np.floor(np.abs(v) + F32(0.5))


def quant_rows(x):
    """Per-row symmetric int8: scale = max|x|/127, q = round(x/scale). Returns (q, scale)."""
    x = x.astype(F32)
    mx = np.abs(x).max(axis=1)
    scale = np.where(mx == 0, F32(1.0), mx / F32(127.0)).astype(F32)
    q = np.clip(rhaz(x / scale[:, None]), -127, 127)
    return q.astype(np.int8), scale


def ternarise(w):
    """BitNet absmean: gamma = mean|w|, w_q = clip(round(w/gamma), -1, 1)."""
    gamma = F32(np.abs(w).mean())
    q = np.clip(rhaz(w / gamma), -1, 1).astype(np.int8)
    return q, gamma


def fake_quant_rows(x):
    q, s = quant_rows(x)
    return q.astype(F32) * s[:, None]


def init(rng, n_in, n_out):
    return (rng.standard_normal((n_out, n_in)) / np.sqrt(n_in)).astype(F32)


def softmax(z):
    z = z - z.max(axis=1, keepdims=True)
    e = np.exp(z)
    return e / e.sum(axis=1, keepdims=True)


def forward(params, x, mode):
    w1, b1, w2, b2 = params
    if mode == "qat":
        q1, g1 = ternarise(w1)
        q2, g2 = ternarise(w2)
        w1e, w2e = q1.astype(F32) * g1, q2.astype(F32) * g2
        xe = fake_quant_rows(x)
    else:
        w1e, w2e, xe = w1, w2, x
    z1 = xe @ w1e.T + b1
    h = np.maximum(z1, 0)
    he = fake_quant_rows(h) if mode == "qat" else h
    z2 = he @ w2e.T + b2
    return z1, xe, he, w2e, z2


def train(mode, xtr, ytr, rng):
    params = [init(rng, 64, HIDDEN), np.zeros(HIDDEN, F32), init(rng, HIDDEN, 10), np.zeros(10, F32)]
    m = [np.zeros_like(p) for p in params]
    v = [np.zeros_like(p) for p in params]
    step = 0
    n = len(xtr)
    for _ in range(EPOCHS):
        order = rng.permutation(n)
        for i in range(0, n, BATCH):
            idx = order[i : i + BATCH]
            x, y = xtr[idx], ytr[idx]
            z1, xe, he, w2e, z2 = forward(params, x, mode)
            p = softmax(z2)
            p[np.arange(len(y)), y] -= 1
            dz2 = p / len(y)
            dw2 = dz2.T @ he
            db2 = dz2.sum(axis=0)
            dz1 = (dz2 @ w2e) * (z1 > 0)  # straight-through past the quantisers
            dw1 = dz1.T @ xe
            db1 = dz1.sum(axis=0)
            step += 1
            for k, (param, g) in enumerate(zip(params, [dw1, db1, dw2, db2])):
                m[k] = F32(0.9) * m[k] + F32(0.1) * g
                v[k] = F32(0.999) * v[k] + F32(0.001) * g * g
                mh = m[k] / F32(1 - 0.9**step)
                vh = v[k] / F32(1 - 0.999**step)
                param -= (F32(LR) * mh / (np.sqrt(vh) + F32(1e-8))).astype(F32)
    return params


def accuracy(logits, y):
    return float((logits.argmax(axis=1) == y).mean())


def int_pipeline(q1, g1, b1, q2, g2, b2, x):
    """The integer pipeline the Rust kernel runs. Returns (pred, logits, layer-1 accumulators)."""
    xq, sx = quant_rows(x)
    acc1 = xq.astype(np.int64) @ q1.astype(np.int64).T
    z1 = (acc1.astype(np.int32).astype(F32) * sx[:, None]) * g1 + b1
    h = np.maximum(z1, F32(0)).astype(F32)
    hq, sh = quant_rows(h)
    acc2 = hq.astype(np.int64) @ q2.astype(np.int64).T
    z2 = (acc2.astype(np.int32).astype(F32) * sh[:, None]) * g2 + b2
    return z2.argmax(axis=1), z2, acc1.astype(np.int32)


def main():
    d = load_digits()
    pix = d.data.astype(np.int32)  # 0..16
    x_all = (pix.astype(F32) / F32(16.0)).astype(F32)
    y_all = d.target
    idx = np.arange(len(y_all))
    xtr, xte, ytr, yte, itr, ite = train_test_split(
        x_all, y_all, idx, test_size=0.25, stratify=y_all, random_state=SEED
    )

    p_f32 = train("f32", xtr, ytr, np.random.default_rng(SEED))
    p_qat = train("qat", xtr, ytr, np.random.default_rng(SEED))

    acc_f32 = accuracy(forward(p_f32, xte, "f32")[4], yte)

    q1, g1 = ternarise(p_qat[0])
    q2, g2 = ternarise(p_qat[2])
    pred_qat, z_qat, acc1 = int_pipeline(q1, g1, p_qat[1], q2, g2, p_qat[3], xte)
    acc_qat = float((pred_qat == yte).mean())

    r1, h1 = ternarise(p_f32[0])
    r2, h2 = ternarise(p_f32[2])
    pred_ptq, _, _ = int_pipeline(r1, h1, p_f32[1], r2, h2, p_f32[3], xte)
    acc_ptq = float((pred_ptq == yte).mean())

    zero_frac = float((np.concatenate([q1.ravel(), q2.ravel()]) == 0).mean())

    data = {
        "dataset": "sklearn load_digits (1797 samples, 8x8, pixels 0..16)",
        "seed": SEED,
        "hidden": HIDDEN,
        "epochs": EPOCHS,
        "train_samples": int(len(ytr)),
        "test_samples": int(len(yte)),
        "accuracy": {
            "f32": acc_f32,
            "ternary_qat_int8": acc_qat,
            "ternary_ptq_int8": acc_ptq,
        },
        "ternary_zero_fraction": zero_frac,
        "gamma1": float(g1),
        "gamma2": float(g2),
        "b1": [float(v) for v in p_qat[1]],
        "b2": [float(v) for v in p_qat[3]],
        "w1": q1.ravel().astype(int).tolist(),
        "w2": q2.ravel().astype(int).tolist(),
        "test_pixels": pix[ite].tolist(),
        "test_labels": yte.astype(int).tolist(),
        "ref_pred": pred_qat.astype(int).tolist(),
        "ref_logits_first16": [[float(v) for v in row] for row in z_qat[:16]],
        "ref_acc1_first8": acc1[:8].astype(int).tolist(),
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(data, separators=(",", ":")))
    print(f"f32 {acc_f32:.4f} | ternary QAT + int8 {acc_qat:.4f} | ternary PTQ + int8 {acc_ptq:.4f}")
    print(f"ternary zero fraction {zero_frac:.3f}; wrote {OUT} ({OUT.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
