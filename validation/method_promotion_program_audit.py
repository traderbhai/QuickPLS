import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULT = ROOT / "validation" / "results" / "method_promotion_program_audit.json"


REQUIRED_DOCS = [
    "docs/METHOD_PROMOTION_CRITERIA.md",
    "docs/METHOD_PROMOTION_PROGRAM_V1_2.md",
    "docs/V1_COMPATIBILITY_MATRIX.md",
    "docs/METHOD_COMPATIBILITY.md",
    "docs/V1_SUPPORTED_SCOPE.md",
]

PROMOTION_MATRIX = ROOT / "validation" / "results" / "method_promotion_matrix_v1_2.json"


def read_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def parse_method_compatibility():
    path = ROOT / "docs" / "METHOD_COMPATIBILITY.md"
    rows = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.startswith("| ") or "---" in line or "Family" in line:
            continue
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if len(cells) >= 4:
            rows.append(
                {
                    "family": cells[0],
                    "method": cells[1],
                    "foundation_status": cells[2],
                    "stable_output": cells[3],
                    "experimental": "experimental" in cells[2].lower() or cells[3].lower() == "no",
                }
            )
    return rows


def main():
    registry = read_json(ROOT / "validation" / "development_slices.json")
    slices = {item["id"]: item for item in registry["slices"]}
    docs = {doc: (ROOT / doc).exists() for doc in REQUIRED_DOCS}
    method_rows = parse_method_compatibility()
    matrix = read_json(PROMOTION_MATRIX) if PROMOTION_MATRIX.exists() else {}
    experimental_rows = [row for row in method_rows if row["experimental"]]
    validated_rows = [row for row in method_rows if not row["experimental"]]
    promotion_slice = slices.get("v1_2_method_promotion_program")
    open_gates = []
    passed_gates = []
    if promotion_slice:
        for gate in promotion_slice.get("gates", []):
            if gate.get("status") == "passed":
                passed_gates.append(gate["name"])
            if gate.get("status") in {"open", "blocked"}:
                open_gates.append(gate["name"])

    criteria_text = (ROOT / "docs" / "METHOD_PROMOTION_CRITERIA.md").read_text(encoding="utf-8")
    criteria_checks = {
        "requires_two_references": bool(re.search(r"two independent references", criteria_text, re.I)),
        "requires_simulation": bool(re.search(r"Simulation evidence", criteria_text, re.I)),
        "requires_gui_cli_export": bool(re.search(r"GUI and CLI parity", criteria_text, re.I)),
        "requires_reproducibility": bool(re.search(r"Reproducibility evidence", criteria_text, re.I)),
        "retains_known_differences": bool(re.search(r"Known-difference", criteria_text, re.I)),
    }

    report = {
        "passed": bool(
            all(docs.values())
            and matrix.get("target") == "v1_2_method_promotion_program"
            and matrix.get("first_batch_rows", 0) >= 5
            and promotion_slice is not None
            and registry.get("current_stage") == "v1_2_method_promotion_program"
            and all(criteria_checks.values())
            and len(method_rows) > 0
            and len(experimental_rows) > 0
        ),
        "current_stage": registry.get("current_stage"),
        "promotion_slice_present": promotion_slice is not None,
        "required_docs": docs,
        "criteria_checks": criteria_checks,
        "promotion_matrix": {
            "path": str(PROMOTION_MATRIX.relative_to(ROOT)),
            "present": PROMOTION_MATRIX.exists(),
            "target": matrix.get("target"),
            "first_batch_rows": matrix.get("first_batch_rows"),
            "first_batch_promotion_ready": matrix.get("first_batch_promotion_ready"),
        },
        "method_rows": len(method_rows),
        "experimental_or_not_stable_rows": len(experimental_rows),
        "validated_method_rows": len(validated_rows),
        "promotion_slice_passed_gates": passed_gates,
        "promotion_slice_open_gates": open_gates,
        "initial_priority": [
            "PLS core stable run envelope",
            "Assessment metrics",
            "Inference/resampling for documented PLS settings",
            "Standalone PCA",
            "OLS regression",
        ],
        "note": (
            "This audit verifies that the method-promotion program is correctly started. "
            "It does not promote any calculation by itself; open gates remain until method-specific "
            "reference, simulation, product, export, and documentation evidence is complete."
        ),
    }

    RESULT.parent.mkdir(parents=True, exist_ok=True)
    RESULT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(f"wrote {RESULT} | passed={report['passed']} | open_gates={len(open_gates)}")
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
