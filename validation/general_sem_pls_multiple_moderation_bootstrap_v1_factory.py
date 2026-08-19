#!/usr/bin/env python3
"""Mint source-bound engine evidence for General SEM moderation bootstrap v1."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Sequence


ROOT = Path(__file__).resolve().parents[1]
FEATURE_ID = "qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap"
METHOD_VERSION = (
    "general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1"
)
CATALOGUE_SNAPSHOT_DATE = "2026-08-19"
GENERATED_AT_UTC = "2026-08-19T12:00:00Z"
DEFAULT_OUTPUT = (
    ROOT
    / "validation/results/method_factory/"
    "general_sem_pls_multiple_moderation_bootstrap_v1/engine_evidence.identity.json"
)

# Deliberately narrow, current source binding for this engine-only checkpoint.
# The report is minted only after concurrent production/native edits stabilize.
SOURCE_PATHS = (
    "crates/qpls-core/src/canonical_result_v2.rs",
    "crates/qpls-core/src/compiled_pls_plan_v3.rs",
    "crates/qpls-core/src/general_sem_capability_preflight_v1.rs",
    "crates/qpls-core/src/general_sem_recipe_compiler_v1.rs",
    "crates/qpls-estimation/src/general_sem_pls_interactions_v1.rs",
    "crates/qpls-project/src/canonical_result_document_v2.rs",
    "crates/qpls-project/src/project_schema_v6.rs",
    "crates/qpls-resampling/src/general_sem_pls_bootstrap_v1.rs",
    "crates/qpls-runner/src/recipe_v4_general_sem_pls_execution.rs",
    "docs/methods/GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_V1.md",
    "src-tauri/src/recipe_v4_general_sem_canonical_result.rs",
    "src-tauri/src/recipe_v4_general_sem_pls_jobs.rs",
    "src/domain/canonicalGeneralSemResultsV1.ts",
    "src/domain/generalSemCapabilityPreflightV1.test.ts",
    "src/domain/generalSemCapabilityPreflightV1.ts",
    "src/domain/internalRecipeV4GeneralSemWorkspace.test.ts",
    "src/domain/internalRecipeV4GeneralSemWorkspace.ts",
    "src/native/NativeRecipeV4GeneralSemWorkspace.test.tsx",
    "src/native/NativeRecipeV4GeneralSemWorkspace.tsx",
    "src/native/nativeCanonicalResultDocumentV2.test.ts",
    "validation/capabilities/general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1.cell.manifest.json",
    "validation/general_sem_pls_multiple_moderation_bootstrap_v1_factory.py",
    "validation/general_sem_pls_multiple_moderation_bootstrap_v1_reference.py",
    "validation/method_promotion_manifest.py",
    "validation/methods/general_sem_pls_multiple_moderation_bootstrap_v1.manifest.json",
    "validation/methods/method_promotion_manifest.schema.json",
    "validation/test_general_sem_pls_multiple_moderation_bootstrap_v1_factory.py",
    "validation/test_general_sem_pls_multiple_moderation_bootstrap_v1_reference.py",
    "validation/test_method_promotion_manifest.py",
)


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _source_descriptor(relative: str) -> dict[str, object]:
    path = ROOT / relative
    if not path.is_file():
        raise FileNotFoundError(f"source artifact is missing: {relative}")
    return {
        "path": relative,
        "size": path.stat().st_size,
        "sha256": _sha256(path),
    }


def execute_reference() -> dict[str, object]:
    command = [
        sys.executable,
        str(ROOT / "validation/general_sem_pls_multiple_moderation_bootstrap_v1_reference.py"),
    ]
    completed = subprocess.run(
        command,
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    if completed.returncode != 0:
        detail = completed.stderr.strip() or completed.stdout.strip()
        raise RuntimeError(f"independent reference failed: {detail}")
    report = json.loads(completed.stdout)
    if not isinstance(report, dict):
        raise TypeError("independent reference report must be an object")
    return report


def build_report(generated_at_utc: str = GENERATED_AT_UTC) -> dict[str, object]:
    reference = execute_reference()
    identity_matches = (
        reference.get("feature_id") == FEATURE_ID
        and reference.get("method_version") == METHOD_VERSION
        and reference.get("catalogue_snapshot_date") == CATALOGUE_SNAPSHOT_DATE
    )
    gamma_rows = reference.get("gamma_inference", [])
    gamma_only = (
        isinstance(gamma_rows, list)
        and len(gamma_rows) == 4
        and all(
            isinstance(row, dict)
            and isinstance(row.get("target"), dict)
            and row["target"].get("kind") == "interaction_scientific_rescaled_gamma"
            for row in gamma_rows
        )
    )
    checks = {
        "independent_reference_executed": reference.get("passed") is True,
        "independent_reference_identity_matches": identity_matches,
        "observed_score_scope_disclosed": reference.get("reference_scope")
        == "independent_observed_score_gamma_only_smoke_v1",
        "same_focal_joint_case_bootstrap": reference.get("checks", {}).get(
            "same_focal_complete_joint_case_bootstrap"
        )
        is True,
        "different_focal_joint_case_bootstrap": reference.get("checks", {}).get(
            "different_focal_complete_joint_case_bootstrap"
        )
        is True,
        "score_sign_product_scale_and_joint_refit_checked": all(
            reference.get("checks", {}).get(key) is True
            for key in (
                "score_vector_sign_alignment_precedes_products",
                "product_scale_recomputed_per_replicate",
                "complete_joint_point_contract_reconciled_per_replicate",
            )
        ),
        "type7_b_minus_one_plus_one_and_gate_checked": all(
            reference.get("checks", {}).get(key) is True
            for key in (
                "type7_percentile_microcase",
                "sample_standard_error_b_minus_one_microcase",
                "null_centered_plus_one_p_microcase",
                "exact_ninety_percent_gate_accepts_18_of_20",
                "exact_ninety_percent_gate_rejects_17_of_20",
            )
        ),
        "indexed_replay_and_evaluation_order_checked": reference.get("checks", {}).get(
            "indexed_replay_and_evaluation_order_invariant"
        )
        is True,
        "gamma_only_inference_inventory": gamma_only,
        "qualification_not_claimed": reference.get("qualification_ready") is False
        and reference.get("promotion_allowed") is False,
        "current_engine_native_and_test_sources_bound": True,
        "full_independent_pls_oracle_complete": False,
        "qualification_scale_coverage_and_null_calibration_complete": False,
        "semantic_export_packaged_performance_and_review_complete": False,
        "release_qualification_complete": False,
    }
    passed = all(
        value
        for key, value in checks.items()
        if key
        not in {
            "full_independent_pls_oracle_complete",
            "qualification_scale_coverage_and_null_calibration_complete",
            "semantic_export_packaged_performance_and_review_complete",
            "release_qualification_complete",
        }
    )
    return {
        "schema_version": 1,
        "report_kind": "quickpls_method_factory_identity_report",
        "role": "method_spec+independent_reference+simulation_report+boundary_report",
        "feature_id": FEATURE_ID,
        "method_version": METHOD_VERSION,
        "catalogue_snapshot_date": CATALOGUE_SNAPSHOT_DATE,
        "generated_at_utc": generated_at_utc,
        "passed": passed,
        "qualification_ready": False,
        "promotion_allowed": False,
        "evidence_scope": "engine_only_independent_observed_score_gamma_only_smoke_and_source_binding",
        "checks": checks,
        "reference_execution": reference,
        "source_artifacts": [_source_descriptor(path) for path in sorted(SOURCE_PATHS)],
    }


def _canonical_json(report: object) -> str:
    return json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n"


def _load_json(path: Path) -> object:
    return json.loads(path.read_text(encoding="utf-8"))


def _utc_now() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--output", type=Path, help="write the current identity report")
    mode.add_argument(
        "--check-report",
        type=Path,
        help="fail unless an existing report exactly matches current sources",
    )
    arguments = parser.parse_args(argv)
    if arguments.check_report is not None:
        expected = _load_json(arguments.check_report)
        if not isinstance(expected, dict) or not isinstance(
            expected.get("generated_at_utc"), str
        ):
            print(
                f"invalid moderation-bootstrap engine report: {arguments.check_report}",
                file=sys.stderr,
            )
            return 1
        report = build_report(expected["generated_at_utc"])
        if expected != report:
            print(
                f"stale or mismatched moderation-bootstrap engine report: {arguments.check_report}",
                file=sys.stderr,
            )
            return 1
        print(f"moderation-bootstrap engine report is current: {arguments.check_report}")
        return 0
    report = build_report(_utc_now())
    if arguments.output is not None:
        output = arguments.output
        if not output.is_absolute():
            output = ROOT / output
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(_canonical_json(report), encoding="utf-8", newline="\n")
        try:
            display_path = output.relative_to(ROOT).as_posix()
        except ValueError:
            display_path = str(output)
        print(f"wrote {display_path}")
        return 0
    print(_canonical_json(report), end="")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
