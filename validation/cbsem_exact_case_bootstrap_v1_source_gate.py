#!/usr/bin/env python3
"""Fast source-tier gate for the current exact-CFA bootstrap family."""

from __future__ import annotations

import argparse
import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

try:
    from validation.cbsem_exact_case_bootstrap_v1_contract_matrix import report as matrix_report
except ModuleNotFoundError:
    from cbsem_exact_case_bootstrap_v1_contract_matrix import report as matrix_report


ROOT = Path(__file__).resolve().parents[1]
FEATURE_ID = "qpls3.cbsem.bootstrap"
METHOD_VERSION = "cbsem_exact_case_bootstrap_v1"
CATALOGUE_SNAPSHOT_DATE = "2026-08-12"
SOURCE_REQUIREMENTS = {
    "compiler_identity": (
        "crates/qpls-core/src/recipe_v4_compiler.rs",
        (
            'CBSEM_EXACT_BOOTSTRAP_CAPABILITY_ID: &str = "smartpls.cbsem_bootstrapping"',
            'CBSEM_EXACT_BOOTSTRAP_CELL_ID: &str = "qpls3.cbsem.bootstrap"',
            "CBSEM_EXACT_BOOTSTRAP_CAPABILITY_VERSION: &str =",
            '"cbsem_exact_case_bootstrap_v1"',
        ),
    ),
    "standard_execution_access": (
        "src-tauri/src/recipe_v4_cbsem_execution.rs",
        (
            "InternalRecipeV4ExecutionSurfaceV1::Standard",
            "cbsem_exact_bootstrap_capability_cell_v1()",
            "recipe_v4.cbsem.capability_cell_mismatch",
        ),
    ),
    "desktop_controls": (
        "src/native/NativeRecipeV4CbsemWorkspace.tsx",
        (
            'id="nd-cbsem-v4-bootstrap-enabled"',
            'id="nd-cbsem-v4-bootstrap-interval"',
            'value="analytic_studentized_type7"',
            'value="bca_type7"',
            'id="nd-cbsem-v4-export-xlsx"',
            "canonicalResultDocumentV2ExportTables(displayedDocument)",
            'id: "canonical_run_provenance"',
            'id: "canonical_result_notes"',
        ),
    ),
    "schema3_route_separation": (
        "src/native/NativeCalculationDialog.tsx",
        (
            'id="nd-calculation-cbsem-exact-bootstrap-route"',
            "historical percentile-only schema-3 recipe",
            "Clear archived bootstrap setting",
        ),
    ),
    "standard_persistence_access": (
        "src-tauri/src/project_schema6_result_append.rs",
        (
            'STANDARD_EXACT_CBSEM_SURFACE: &str = "standard_exact_cbsem"',
            '("smartpls.cbsem_bootstrapping", "qpls3.cbsem.bootstrap")',
        ),
    ),
    "strict_archive_validation": (
        "crates/qpls-project/src/project_schema_v6.rs",
        (
            'cell.capability_id == "smartpls.cbsem_bootstrapping"',
            'cell.capability_version == "cbsem_exact_case_bootstrap_v1"',
            "validate_recipe_v4_cbsem_exact_bootstrap_document_v1",
        ),
    ),
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run_gate() -> dict[str, Any]:
    checks: dict[str, Any] = {}
    source_artifacts: list[dict[str, str]] = []
    for check_id, (relative, tokens) in SOURCE_REQUIREMENTS.items():
        path = ROOT / relative
        text = path.read_text(encoding="utf-8") if path.is_file() else ""
        missing = [token for token in tokens if token not in text]
        checks[check_id] = {"passed": not missing, "path": relative, "missing_tokens": missing}
        if path.is_file():
            source_artifacts.append({"path": relative, "sha256": sha256(path)})

    matrix = matrix_report()
    checks["compact_independent_matrix"] = {
        "passed": matrix["passed"],
        "kind": matrix["kind"],
        "scope": matrix["scope"],
    }
    matrix_path = ROOT / "validation/cbsem_exact_case_bootstrap_v1_contract_matrix.py"
    source_artifacts.append({"path": str(matrix_path.relative_to(ROOT)).replace("\\", "/"), "sha256": sha256(matrix_path)})
    return {
        "passed": all(check["passed"] for check in checks.values()),
        "feature_id": FEATURE_ID,
        "method_version": METHOD_VERSION,
        "catalogue_snapshot_date": CATALOGUE_SNAPSHOT_DATE,
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "kind": "cbsem_exact_case_bootstrap_v1_source_gate",
        "checks": checks,
        "source_artifacts": source_artifacts,
        "integrity_scope": "structural, accounting, arithmetic, identity, and unkeyed checksum consistency",
        "authentication_exclusion": "malicious coordinated semantic rewrite and trusted-origin authentication are not claimed",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    document = run_gate()
    payload = json.dumps(document, indent=2, sort_keys=True, allow_nan=False) + "\n"
    if args.output:
        target = args.output if args.output.is_absolute() else ROOT / args.output
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(payload, encoding="utf-8")
    print(payload, end="")
    return 0 if document["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
