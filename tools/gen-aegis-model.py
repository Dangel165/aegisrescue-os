#!/usr/bin/env python3
"""Generate aegis_classifier.aegismodel (+ optional ONNX) for offline AI inference."""

from __future__ import annotations

import json
import struct
import sys
from pathlib import Path

INPUT_DIM = 14 + 256  # must match FeatureVector::FEATURE_DIM
NUM_CLASSES = 7
CLASS_NAMES = [
    "Ransomware",
    "Rootkit",
    "Trojan",
    "Worm",
    "Backdoor",
    "Cryptominer",
    "Clean",
]

# Feature index map (see core/aegis-ai/src/features.rs to_vec)
IDX = {
    "size": 0,
    "entropy": 1,
    "section": 2,
    "import": 3,
    "wx": 4,
    "packing": 5,
    "suspicious": 6,
    "stripped": 7,
    "unsigned": 8,
    "network": 9,
    "process": 10,
    "file": 11,
    "crypto": 12,
    "registry": 13,
}


def zero_weights() -> list[list[float]]:
    return [[0.0] * INPUT_DIM for _ in range(NUM_CLASSES)]


def build_heuristic_weights() -> tuple[list[list[float]], list[float]]:
    """Mirror aegis-ai heuristic_classify scoring as a linear model."""
    w = zero_weights()
    b = [0.0] * NUM_CLASSES

    def add(cls: int, idx: int, val: float) -> None:
        w[cls][idx] += val

    # Ransomware
    add(0, IDX["size"], 0.1)
    add(0, IDX["entropy"], 0.4)
    add(0, IDX["suspicious"], 0.3)
    add(0, IDX["crypto"], 0.3)
    add(0, IDX["file"], 0.2)
    add(0, IDX["wx"], 0.2)
    for i in range(14, INPUT_DIM):
        add(0, i, 0.02)  # flat histogram → ransomware hint

    # Rootkit
    add(1, IDX["process"], 0.5)
    add(1, IDX["stripped"], 0.2)
    add(1, IDX["section"], -0.2)

    # Trojan
    add(2, IDX["unsigned"], 0.15)
    add(2, IDX["network"], 0.4)
    add(2, IDX["process"], 0.3)
    add(2, IDX["registry"], 0.2)

    # Worm
    add(3, IDX["network"], 0.5)
    add(3, IDX["process"], 0.3)

    # Backdoor
    add(4, IDX["network"], 0.4)
    add(4, IDX["crypto"], 0.3)
    add(4, IDX["registry"], 0.2)

    # Cryptominer
    add(5, IDX["crypto"], 0.5)
    add(5, IDX["network"], 0.3)

    # Clean baseline
    b[6] = 0.5
    for cls in range(NUM_CLASSES - 1):
        b[6] -= 0.05

    return w, b


def softmax(logits: list[float]) -> list[float]:
    m = max(logits)
    exps = [pow(2.718281828, x - m) for x in logits]
    s = sum(exps) or 1.0
    return [e / s for e in exps]


def write_aegismodel(out: Path, weights: list[list[float]], bias: list[float]) -> None:
    payload = {
        "version": 1,
        "input_dim": INPUT_DIM,
        "num_classes": NUM_CLASSES,
        "class_names": CLASS_NAMES,
        "weights": weights,
        "bias": bias,
        "model_type": "linear_softmax",
        "description": "Heuristic-aligned linear classifier for AegisRescue offline inference",
    }
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    print(f"wrote {out} ({out.stat().st_size} bytes)")


def write_onnx(out: Path, weights: list[list[float]], bias: list[float]) -> bool:
    try:
        from onnx import TensorProto, helper
        import onnx
    except ImportError:
        print("onnx package not installed - skipping aegis_classifier.onnx")
        return False

    w_flat = [x for row in weights for x in row]

    input_tensor = helper.make_tensor_value_info("input", TensorProto.FLOAT, [1, INPUT_DIM])
    output_tensor = helper.make_tensor_value_info("output", TensorProto.FLOAT, [1, NUM_CLASSES])

    w_init = helper.make_tensor("W", TensorProto.FLOAT, [NUM_CLASSES, INPUT_DIM], w_flat)
    b_init = helper.make_tensor("B", TensorProto.FLOAT, [NUM_CLASSES], bias)

    gemm = helper.make_node("Gemm", ["input", "W", "B"], ["logits"], alpha=1.0, beta=1.0, transB=1)
    softmax_node = helper.make_node("Softmax", ["logits"], ["output"], axis=1)

    graph = helper.make_graph(
        [gemm, softmax_node],
        "aegis_classifier",
        [input_tensor],
        [output_tensor],
        [w_init, b_init],
    )
    model = helper.make_model(graph, opset_imports=[helper.make_opsetid("", 13)])
    out.parent.mkdir(parents=True, exist_ok=True)
    onnx.save(model, str(out))
    print(f"wrote {out} ({out.stat().st_size} bytes)")
    return True


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    models_dir = root / "data" / "models"
    weights, bias = build_heuristic_weights()
    write_aegismodel(models_dir / "aegis_classifier.aegismodel", weights, bias)
    write_onnx(models_dir / "aegis_classifier.onnx", weights, bias)
    return 0


if __name__ == "__main__":
    sys.exit(main())
