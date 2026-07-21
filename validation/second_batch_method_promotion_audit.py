#!/usr/bin/env python3
"""Aggregate audit for the v1.2.1 second-batch method promotion gate."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "second_batch_method_promotion_audit.json"

REQUIRED = [
    "mediation_method_promotion_audit.json",
    "moderation_method_promotion_audit.json",
    "plsc_method_promotion_audit.json",
    "wpls_method_promotion_audit.json",
    "ipma_method_promotion_audit.json",
    "plspredict_method_promotion_audit.json",
    "nca_method_promotion_audit.json",
    "second_batch_product_enforcement_audit.json",
]


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> int:
    artifacts = []
    for name in REQUIRED:
        path = RESULTS / name
        present = path.exists()
        value = load(path) if present else {}
        artifacts.append({
            "path": str(path.relative_to(ROOT)),
            "present": present,
            "passed": value.get("passed") is True,
        })
    registry = load(ROOT / "validation" / "development_slices.json")
    slices = {item["id"]: item for item in registry["slices"]}
    gate = slices.get("v1_2_1_second_batch_method_promotion")
    gate_present = gate is not None
    gate_all_passed = gate_present and all(item.get("status") == "passed" for item in gate.get("gates", []))
    passed = all(item["present"] and item["passed"] for item in artifacts) and gate_all_passed
    OUTPUT.write_text(json.dumps({
        "schema_version": 1,
        "target": "v1_2_1_second_batch_method_promotion",
        "passed": passed,
        "artifacts": artifacts,
        "registry_gate_present": gate_present,
        "registry_gate_all_passed": gate_all_passed,
        "note": "Second-batch promotion is bounded to the documented scopes; later-batch methods remain experimental.",
    }, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {OUTPUT} | passed={passed}")
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
