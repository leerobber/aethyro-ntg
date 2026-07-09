#!/usr/bin/env python3
"""Layer ingest helper for aethyro-ntg native runtime.

Contract (mandatory for Runtime::forward_native_parallel):
  After building nodes for each layer, node IDs must be dense and
  sequential starting at 0:  node.id == index in the layer list.

This script validates that contract on a simple JSON layer description
and can emit a summary suitable for offline checks / CI.

Example layer JSON (stdin or --file):
  {
    "layers": [
      {"nodes": [{"id": 0, "weight_len": 64}, {"id": 1, "weight_len": 64}]},
      {"nodes": [{"id": 0, "weight_len": 128}]}
    ]
  }
"""

from __future__ import annotations

import argparse
import json
import sys
from typing import Any


def validate_layer_nodes(layer_nodes: list[dict[str, Any]], layer_index: int) -> None:
    """Raise ValueError if node IDs are not sequential starting at 0."""
    for i, node in enumerate(layer_nodes):
        node_id = node.get("id")
        if node_id != i:
            raise ValueError(
                f"Layer node IDs must be sequential starting at 0. "
                f"Got node.id={node_id} at position {i} (layer {layer_index})"
            )


def validate_document(doc: dict[str, Any]) -> int:
    layers = doc.get("layers")
    if not isinstance(layers, list):
        raise ValueError("document must contain a 'layers' list")

    total_nodes = 0
    for li, layer in enumerate(layers):
        nodes = layer.get("nodes") if isinstance(layer, dict) else None
        if not isinstance(nodes, list):
            raise ValueError(f"layer {li} must contain a 'nodes' list")
        validate_layer_nodes(nodes, li)
        total_nodes += len(nodes)
    return total_nodes


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--file",
        "-f",
        help="JSON file to validate (default: stdin)",
    )
    args = parser.parse_args(argv)

    if args.file:
        with open(args.file, encoding="utf-8") as f:
            raw = f.read()
    else:
        raw = sys.stdin.read()

    if not raw.strip():
        print("ingest: empty input", file=sys.stderr)
        return 2

    doc = json.loads(raw)
    total = validate_document(doc)
    print(f"ok: {len(doc['layers'])} layer(s), {total} node(s), sequential IDs enforced")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, json.JSONDecodeError) as e:
        print(f"ingest error: {e}", file=sys.stderr)
        raise SystemExit(1)
