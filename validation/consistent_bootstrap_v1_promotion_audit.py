#!/usr/bin/env python3
"""Fail-closed QualificationSpec V2 audit for PLSc consistent bootstrap v1.

The default command represents the promotion decision and therefore exits
non-zero until every required immutable receipt is admitted.  ``--scaffold-only``
is the narrow CI mode: it succeeds only when the preregistration is valid and
the capability remains visibly unqualified.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
sys.path.insert(0, str(VALIDATION))

from consistent_bootstrap_v1_qualification import (  # noqa: E402
    CAPABILITY_ID,
    CELL_ID,
    FROZEN_AT,
    METHOD_VERSION,
    OUTPUT as SPEC_PATH,
    build_spec,
    verify as verify_qualification,
)
from consistent_bootstrap_v1_reference import (  # noqa: E402
    DEFAULT_FIXTURE,
    strict_load_json,
    validate_fixture,
)
from method_promotion_manifest import validate_manifest  # noqa: E402


MANIFEST_PATH = VALIDATION / "methods" / "consistent_bootstrap_v1.manifest.json"
REGISTRY_PATH = VALIDATION / "capabilities" / "capability_registry_v2.json"
DOC_PATH = ROOT / "docs" / "methods" / "CONSISTENT_BOOTSTRAP_V1_QUALIFICATION.md"
PYTHON_REFERENCE_PATH = VALIDATION / "consistent_bootstrap_v1_reference.py"
R_REFERENCE_PATH = VALIDATION / "consistent_bootstrap_v1_reference.R"
REPORT_PATH = (
    VALIDATION
    / "results"
    / "method_factory"
    / "consistent_bootstrap_v1"
    / "qualification_scaffold_audit.json"
)

PRODUCT_INTEGRATION_PATHS = [
    "crates/qpls-resampling/src/consistent_bootstrap.rs",
    "crates/qpls-runner/src/lib.rs",
    "crates/qpls-project/src/lib.rs",
    "crates/qpls-cli/src/main.rs",
    "src/native/nativeConsistentBootstrap.ts",
    "src/native/nativeConsistentBootstrap.test.ts",
    "src/native/nativeResults.ts",
    "src/native/nativeExportTables.ts",
]


REQUIREMENTS: list[dict[str, Any]] = [
    {
        "role": "method_contract",
        "status": "scaffold_only",
        "admission_condition": "Freeze equations, PLSc parameter families, preprocessing, supported predicates, settings, interval and failure semantics, exact result schema, and official parity references; obtain independent method-family review.",
        "scaffold_artifacts": [
            "validation/qualification_v2/consistent_bootstrap_v1.qualification.json",
            "validation/fixtures/consistent_bootstrap_v1_microcases.json",
            "docs/methods/CONSISTENT_BOOTSTRAP_V1_QUALIFICATION.md",
        ],
    },
    {
        "role": "kernel_execution",
        "status": "missing_evidence",
        "admission_condition": "Execute full-refit plsc_v2 at 1,000 through 10,000 indexed case resamples and full delete-one refits; bind source set, scenario set, build fingerprint, exact ledger, result payload, and typed failures in an immutable receipt.",
        "scaffold_artifacts": [
            "validation/consistent_bootstrap_v1_reference.py",
            "validation/fixtures/consistent_bootstrap_v1_microcases.json",
        ],
    },
    {
        "role": "oracle_independence",
        "status": "missing_evidence",
        "admission_condition": "Compare every PLSc parameter family and interval to two independently maintained full-reestimation implementations, or approve and document the stricter oracle exception allowed by QualificationSpec V2.",
        "scaffold_artifacts": [
            "validation/consistent_bootstrap_v1_reference.py",
            "validation/consistent_bootstrap_v1_reference.R",
        ],
    },
    {
        "role": "generative_recovery",
        "status": "missing_evidence",
        "admission_condition": "Run preregistered bias, standard-error, percentile/BCa coverage, Type-I error, power, and failure-rate simulations with failed fits retained in every denominator and 95% Monte Carlo half-width at most one percentage point unless justified.",
        "scaffold_artifacts": [],
    },
    {
        "role": "adversarial_boundaries",
        "status": "inventory_only",
        "admission_condition": "Execute every declared boundary plus row/declaration reorder, sign alignment, worker-count, seed-repeat, save/reopen, GUI/CLI, nonfinite, collinear, small-sample, and cancellation mutations; accept only correct output or a specific typed failure.",
        "scaffold_artifacts": [
            "validation/fixtures/consistent_bootstrap_v1_microcases.json"
        ],
    },
    {
        "role": "archive_persistence",
        "status": "missing_evidence",
        "admission_condition": "Run through the real project runner; append atomically; save, close, reopen, and compare canonical payloads; cover schema 1-5, future read-only, interrupted save/recovery, checksum/duplicate-key/member/method/dataset/ledger/interval tampering, and explicitly resolve replayability of successful replicate parameter digests.",
        "scaffold_artifacts": [],
    },
    {
        "role": "cross_format_export",
        "status": "missing_evidence",
        "admission_condition": "Export the same canonical run to CSV, XLSX, HTML, SVG, PDF, and PNG; semantically read back CSV, XLSX, HTML, SVG, and PDF; compare table IDs, cells, precision, missing values, ordering, warnings, provenance, chart data, and source-run identity.",
        "scaffold_artifacts": [],
    },
    {
        "role": "frontend_contract",
        "status": "missing_evidence",
        "admission_condition": "Prove exact GUI/CLI/native recipe equivalence, actionable unsupported-state diagnostics, fail-closed malformed payload handling, accessible result/failure tables, cancellation/retry, and continued non-selectability while registry evidence is absent.",
        "scaffold_artifacts": ["src/native/nativeConsistentBootstrap.test.ts"],
    },
    {
        "role": "packaged_windows_e2e",
        "status": "missing_evidence",
        "admission_condition": "Exercise installed and portable offline Windows builds at every required viewport and scaling level, including keyboard/pointer access, invalid setup, execute, cancel/retry, save/reopen, export/readback, clean exit, and no orphan process/listener.",
        "scaffold_artifacts": [],
    },
    {
        "role": "performance_scale",
        "status": "missing_evidence",
        "admission_condition": "Measure five post-warmup runs on standard and workstation hardware for micro, applied, large, each maximum axis, and compound stress; enforce elapsed, memory, result-size, cancellation-latency, soak, and 20% regression gates.",
        "scaffold_artifacts": [],
    },
]


BLOCKERS = [
    "coverage.selectable_test_direction_missing",
    "coverage.selectable_interval_family_missing",
    "coverage.complete_measurement_assessment_inference_missing",
    "coverage.broader_plsc_model_shapes_and_defaults_missing",
    "evidence.full_plsc_independent_oracle_missing",
    "evidence.second_independently_maintained_full_plsc_oracle_missing",
    "evidence.preregistered_bias_coverage_and_failure_simulation_not_run",
    "evidence.adversarial_and_metamorphic_matrix_not_run",
    "evidence.archive_and_cross_format_readback_receipts_missing",
    "evidence.gui_cli_native_equivalence_receipt_missing",
    "evidence.packaged_windows_accessibility_matrix_not_run",
    "evidence.performance_scale_and_soak_not_run",
    "evidence.independent_scientific_review_not_recorded",
    "oracle.base_r_microreference_not_executed_in_this_scaffold_gate",
    "persistence.successful_replicate_vectors_not_replayable_from_archive",
    "method.minimum_usable_fraction_requires_independent_justification",
    "qualification.compatibility_only_migration_not_complete",
    "qualification.all_ten_immutable_evidence_roles_have_zero_admitted_receipts",
]


def _find_registry_cell(registry: dict[str, Any]) -> tuple[dict[str, Any], dict[str, Any]]:
    rows = [
        row
        for row in registry.get("capabilities", [])
        if row.get("capability_id") == CAPABILITY_ID
    ]
    if len(rows) != 1:
        raise ValueError(f"expected one registry row for {CAPABILITY_ID}, found {len(rows)}")
    cells = [
        cell
        for cell in rows[0].get("option_cells", [])
        if cell.get("cell_id") == CELL_ID
    ]
    if len(cells) != 1:
        raise ValueError(f"expected one registry cell for {CELL_ID}, found {len(cells)}")
    return rows[0], cells[0]


def build_report() -> dict[str, Any]:
    errors: list[str] = []
    try:
        spec = strict_load_json(SPEC_PATH)
        expected_spec = build_spec()
        spec_report = verify_qualification(spec)
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as error:
        spec = {}
        expected_spec = {}
        spec_report = {"passed": False, "qualification_ready": False, "errors": [str(error)]}
        errors.append(f"qualification spec cannot be checked: {type(error).__name__}: {error}")
    if spec != expected_spec:
        errors.append("frozen qualification JSON differs from its deterministic generator")
    if not spec_report.get("passed"):
        errors.append("QualificationSpec V2 schema or semantic validation failed")
    if spec_report.get("qualification_ready"):
        errors.append("compatibility-only zero-receipt scaffold unexpectedly claims qualification")
    receipts = spec.get("evidence_contract", {}).get("receipts", []) if isinstance(spec, dict) else []
    if receipts:
        errors.append("scaffold must contain zero admitted evidence receipts")

    try:
        fixture_report = validate_fixture(strict_load_json(DEFAULT_FIXTURE))
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as error:
        fixture_report = {"passed": False, "errors": [str(error)]}
        errors.append(f"microcase fixture cannot be checked: {type(error).__name__}: {error}")
    if not fixture_report.get("passed"):
        errors.append("transparent Python arithmetic/ledger microreference failed")
    if fixture_report.get("qualification_evidence") is not False:
        errors.append("microreference must remain explicitly non-promotional")

    try:
        registry = strict_load_json(REGISTRY_PATH)
        registry_row, registry_cell = _find_registry_cell(registry)
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as error:
        registry_row, registry_cell = {}, {}
        errors.append(f"registry cell cannot be checked: {type(error).__name__}: {error}")
    expected_cell_state = {
        "coverage_state": "partial",
        "evidence_state": "absent",
        "surface": "labs",
        "capability_version": METHOD_VERSION,
    }
    actual_cell_state = {key: registry_cell.get(key) for key in expected_cell_state}
    if actual_cell_state != expected_cell_state:
        errors.append(
            f"registry cell is not at the frozen fail-closed state: {actual_cell_state!r}"
        )
    if registry_row.get("legacy_row", {}).get("status") != "absent":
        errors.append("legacy projection must remain absent while evidence is absent")

    manifest_report = validate_manifest(MANIFEST_PATH, ROOT, verify_evidence=True)
    if not manifest_report.get("passed"):
        errors.append("legacy method-promotion manifest validation failed")
    if manifest_report.get("declared_state") != "absent" or manifest_report.get(
        "derived_state"
    ) != "absent":
        errors.append("legacy method-promotion state must derive absent")

    required_files = [
        SPEC_PATH,
        DEFAULT_FIXTURE,
        DOC_PATH,
        PYTHON_REFERENCE_PATH,
        R_REFERENCE_PATH,
        *[ROOT / path for path in PRODUCT_INTEGRATION_PATHS],
    ]
    missing_files = [
        path.relative_to(ROOT).as_posix() for path in required_files if not path.is_file()
    ]
    if missing_files:
        errors.append(f"required scaffold/integration paths are missing: {missing_files}")

    required_roles = spec.get("evidence_contract", {}).get("required_roles", []) if isinstance(spec, dict) else []
    requirement_roles = [requirement["role"] for requirement in REQUIREMENTS]
    if required_roles != requirement_roles:
        errors.append("promotion-audit requirement order differs from QualificationSpec V2")

    scaffold_valid = not errors
    promotion_allowed = bool(
        scaffold_valid
        and spec_report.get("qualification_ready")
        and receipts
        and registry_cell.get("coverage_state") == "full"
        and registry_cell.get("evidence_state") == "release_qualified"
        and registry_cell.get("surface") == "standard"
    )
    # This scaffold must fail closed.  Any future true value requires replacing
    # this audit with receipt-backed admission rather than weakening the rule.
    if promotion_allowed:
        errors.append("promotion became possible through a scaffold-only audit")
        scaffold_valid = False
        promotion_allowed = False

    return {
        "schema_version": 1,
        "generated_at_utc": FROZEN_AT,
        "identity": {
            "capability_id": CAPABILITY_ID,
            "cell_id": CELL_ID,
            "method_version": METHOD_VERSION,
            "qualification_id": spec.get("identity", {}).get("qualification_id")
            if isinstance(spec, dict)
            else None,
        },
        "passed": promotion_allowed,
        "scaffold_valid": scaffold_valid,
        "qualification_ready": bool(spec_report.get("qualification_ready")),
        "promotion_allowed": promotion_allowed,
        "current_state": {
            "registry": actual_cell_state,
            "legacy_row_status": registry_row.get("legacy_row", {}).get("status"),
            "manifest_declared_state": manifest_report.get("declared_state"),
            "manifest_derived_state": manifest_report.get("derived_state"),
            "migration_status": spec.get("migration", {}).get("status")
            if isinstance(spec, dict)
            else None,
            "receipt_count": len(receipts),
        },
        "scaffold_checks": {
            "qualification_spec": spec_report,
            "microreference": fixture_report,
            "legacy_manifest": {
                "passed": manifest_report.get("passed"),
                "declared_state": manifest_report.get("declared_state"),
                "derived_state": manifest_report.get("derived_state"),
                "errors": manifest_report.get("errors", []),
            },
            "r_microreference_present_but_not_executed": R_REFERENCE_PATH.is_file(),
            "missing_files": missing_files,
        },
        "evidence_requirements": REQUIREMENTS,
        "promotion_blockers": BLOCKERS,
        "errors": errors,
        "decision": "BLOCK_PROMOTION_KEEP_EVIDENCE_ABSENT",
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--write",
        action="store_true",
        help="Write the deterministic fail-closed audit report.",
    )
    parser.add_argument(
        "--scaffold-only",
        action="store_true",
        help="Succeed only if the scaffold validates and promotion remains blocked.",
    )
    args = parser.parse_args()
    report = build_report()
    if args.write:
        REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
        REPORT_PATH.write_text(
            json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n",
            encoding="utf-8",
        )
    print(json.dumps(report, indent=2, sort_keys=True, allow_nan=False))
    if args.scaffold_only:
        return 0 if report["scaffold_valid"] and not report["promotion_allowed"] else 1
    return 0 if report["promotion_allowed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
