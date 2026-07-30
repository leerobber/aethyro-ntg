#!/usr/bin/env python3
"""Generates simple PNG app icons for the Aethyro dashboard PWA (stdlib only, no Pillow)."""
import struct
import zlib
import math
import sys
import os

BG = (13, 17, 23)        # dark background
ACCENT = (88, 166, 255)  # aethyro blue
ACCENT2 = (163, 113, 247)  # violet accent


def make_png(path, size, maskable=False):
    pad = int(size * 0.18) if maskable else 0
    cx, cy = size / 2, size / 2
    r_outer = (size - 2 * pad) * 0.42
    rows = []
    for y in range(size):
        row = bytearray()
        row.append(0)  # filter type 0 (none)
        for x in range(size):
            dx, dy = x - cx, y - cy
            dist = math.hypot(dx, dy)
            angle = (math.atan2(dy, dx) + math.pi) / (2 * math.pi)  # 0..1

            if dist > r_outer:
                r, g, b, a = BG[0], BG[1], BG[2], 255
            else:
                # three ternary "lobes" (–, 0, +) blended between accent colors
                lobe = (angle * 3) % 1.0
                t = 0.5 + 0.5 * math.sin(lobe * 2 * math.pi)
                r = int(ACCENT[0] + (ACCENT2[0] - ACCENT[0]) * t)
                g = int(ACCENT[1] + (ACCENT2[1] - ACCENT[1]) * t)
                b = int(ACCENT[2] + (ACCENT2[2] - ACCENT[2]) * t)
                a = 255
                # inner dark core dot for a "node" look
                if dist < r_outer * 0.22:
                    r, g, b = BG
            row += bytes((r, g, b, a))
        rows.append(bytes(row))
    raw = b"".join(rows)
    compressed = zlib.compress(raw, 9)

    def chunk(tag, data):
        c = tag + data
        return struct.pack(">I", len(data)) + c + struct.pack(">I", zlib.crc32(c) & 0xFFFFFFFF)

    sig = b"\x89PNG\r\n\x1a\n"
    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)  # 8-bit RGBA
    png = sig + chunk(b"IHDR", ihdr) + chunk(b"IDAT", compressed) + chunk(b"IEND", b"")
    with open(path, "wb") as f:
        f.write(png)
    print(f"wrote {path} ({size}x{size})")


if __name__ == "__main__":
    outdir = sys.argv[1] if len(sys.argv) > 1 else "."
    os.makedirs(outdir, exist_ok=True)
    make_png(os.path.join(outdir, "icon-192.png"), 192, maskable=False)
    make_png(os.path.join(outdir, "icon-512.png"), 512, maskable=False)
    make_png(os.path.join(outdir, "icon-maskable-512.png"), 512, maskable=True)
    make_png(os.path.join(outdir, "apple-touch-icon.png"), 180, maskable=True)
