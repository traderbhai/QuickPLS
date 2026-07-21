#!/usr/bin/env python3
"""Aggregate audit for the v1.2.4 CB-SEM/CFA and GSCA promotion gate."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "fifth_batch_method_promotion_audit.json"
TARGET = "v1_2_4_cbsem_gsca_promotion"
REQUIRED = [
    "cbsem_cfa_method_promotion_audit.json",
    "gsca_method_promotion_audit.json",
    "fifth_batch_product_enforcement_audit.json",
]


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> int:
    artifacts = []
    for name in REQUIRED:
        path = RESULTS / name
        value = load(path) if path.exists() else {}
        artifacts.append({"path": str(path.relative_to(ROOT)), "present": path.exists(), "passed": value.get("passed") is True})
    registry = load(ROOT / "validation" / "development_slices.json")
    gate = {item["id"]: item for item in registry["slices"]}.get(TARGET)
    gate_present = gate is not None
    gate_all_passed = gate_present and all(item.get("status") == "passed" for item in gate.get("gates", []))
    passed = all(item["present"] and item["passed"] for item in artifacts) and gate_all_passed
    OUTPUT.write_text(json.dumps({
        "schema_version": 1,
        "target": TARGET,
        "passed": passed,
        "artifacts": artifacts,
        "registry_gate_present": gate_present,
        "registry_gate_all_passed": gate_all_passed,
        "note": "Fifth-batch promotion is bounded to documented single-group CB-SEM/CFA ML and deterministic GSCA scopes.",
    }, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {OUTPUT} | passed={passed}")
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
