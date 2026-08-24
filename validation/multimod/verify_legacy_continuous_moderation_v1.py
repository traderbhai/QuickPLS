#!/usr/bin/env python3
"""Fail-closed archive identity and public-CLI replay for legacy moderation V1."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path
from typing import Any


SUITE_ID = "qpls_multimod_legacy_continuous_moderation_v1"
ARCHIVE_SHA256 = "283185ebc4f4c51fc5643c0b64c77a9099c7a5231683d07f10f3848845bc514d"
ARCHIVE_SIZE_BYTES = 28_542
RESULT_ID = "fa5ac33f-4b8a-458b-a889-7b005d95b403"
RECIPE_ID = "fd7ae1a4-fbd2-4b2a-b021-6ee590fa38a2"
DATASET_ID = "103c2aea-494c-4273-b85b-33c7fbc189a3"
DATASET_FINGERPRINT = "v2:4461c776997c9e4723276a54dc1f5ac95c1380fc78047271f28fdfe52006d1f7"
MODEL_ID = "68df9553-f48b-46c4-b60a-e7fb67ee6a48"
TABLE_ID = "moderation_simple_slopes"
METHOD = "pls_pm"
METHOD_VERSION = (
    "pls_pm_v1+pls_mediation_v1+pls_two_stage_moderation_v1+pls_assessment_v8+"
    "indexed_resampling_v4+htmt_bias_corrected_bootstrap_inference_v1"
)
SEED = 20_260_718
NUMERIC_TOLERANCE = 1e-12
# A derived t ratio is ill-conditioned when its denominator is at machine scale;
# this policy does not apply to the corresponding estimate or standard error.
DEGENERATE_STANDARD_ERROR_THRESHOLD = 1e-12
DEGENERATE_T_STATISTIC_MINIMUM_MAGNITUDE = 1e12

EXPECTED_ARCHIVE_IDENTITY: dict[str, Any] = {
    "archive_schema": 5,
    "result_id": RESULT_ID,
    "status": "completed",
    "method": METHOD,
    "method_version": METHOD_VERSION,
    "payload_kind": "pls_pm_v2",
    "model_id": MODEL_ID,
    "construct_count": 4,
    "structural_path_count": 3,
    "interaction_count": 1,
    "higher_order_count": 0,
    "mediated_effect_count": 0,
    "moderation_probe_counts": [3],
    "regression_type": None,
    "process_model": None,
    "cbsem_model_type": None,
    "table_id": TABLE_ID,
    "table_backing_key": "estimates",
    "table_backing_count": 1,
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def object_value(value: Any, label: str) -> dict[str, Any]:
    require(isinstance(value, dict), f"{label} must be an object")
    return value


def array_value(value: Any, label: str) -> list[Any]:
    require(isinstance(value, list), f"{label} must be an array")
    return value


def unique_by_id(values: Any, expected_id: str, label: str) -> dict[str, Any]:
    rows = array_value(values, label)
    matches = [item for item in rows if isinstance(item, dict) and item.get("id") == expected_id]
    require(len(matches) == 1, f"{label} must contain exactly one {expected_id}")
    return matches[0]


def read_project(archive_path: Path) -> dict[str, Any]:
    with zipfile.ZipFile(archive_path, "r") as archive:
        project_entries = [item for item in archive.infolist() if item.filename == "project.json"]
        require(len(project_entries) == 1, "archive must contain exactly one project.json")
        return object_value(json.loads(archive.read(project_entries[0])), "project")


def run_identity_helper(
    repository_root: Path, archive_path: Path, result_id: str, table_id: str
) -> dict[str, Any]:
    helper = repository_root / "validation" / "v255_named_archive_identity.py"
    require(helper.is_file(), f"named archive identity helper is missing: {helper}")
    completed = subprocess.run(
        [
            sys.executable,
            str(helper),
            "--archive",
            str(archive_path),
            "--result-id",
            result_id,
            "--table-id",
            table_id,
        ],
        cwd=repository_root,
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=120,
    )
    require(
        completed.returncode == 0,
        "named archive identity helper failed: " + completed.stderr[-2000:].strip(),
    )
    receipt = object_value(json.loads(completed.stdout), "named archive identity receipt")
    require(receipt.get("passed") is True, "named archive identity helper did not pass")
    require(
        receipt.get("suite_id") == "quickpls_v255_named_archive_identity_v1",
        "named archive identity helper suite mismatch",
    )
    identity = object_value(receipt.get("identity"), "named archive identity")
    require(identity == EXPECTED_ARCHIVE_IDENTITY, "named archive identity does not match the frozen V1 identity")
    return identity


def compare_json(
    expected: Any,
    actual: Any,
    path: str,
    state: dict[str, Any],
) -> None:
    if isinstance(expected, bool) or isinstance(actual, bool):
        require(type(expected) is type(actual) and expected == actual, f"{path}: Boolean mismatch")
        return
    if isinstance(expected, (int, float)) and isinstance(actual, (int, float)):
        if isinstance(expected, int) and isinstance(actual, int):
            require(expected == actual, f"{path}: integer mismatch ({expected} != {actual})")
            state["numeric_values_compared"] += 1
            return
        left = float(expected)
        right = float(actual)
        require(math.isfinite(left) and math.isfinite(right), f"{path}: nonfinite numeric value")
        difference = abs(left - right)
        state["numeric_values_compared"] += 1
        state["max_abs_numeric_difference"] = max(state["max_abs_numeric_difference"], difference)
        require(
            difference <= NUMERIC_TOLERANCE,
            f"{path}: numeric difference {difference} exceeds {NUMERIC_TOLERANCE}",
        )
        return
    if isinstance(expected, dict) and isinstance(actual, dict):
        require(set(expected) == set(actual), f"{path}: object field identity mismatch")
        for key in sorted(expected):
            if key == "t_statistic" and compare_degenerate_t_statistic(
                expected, actual, f"{path}.{key}", state
            ):
                continue
            compare_json(expected[key], actual[key], f"{path}.{key}", state)
        return
    if isinstance(expected, list) and isinstance(actual, list):
        require(len(expected) == len(actual), f"{path}: array length mismatch")
        for index, (left, right) in enumerate(zip(expected, actual, strict=True)):
            compare_json(left, right, f"{path}[{index}]", state)
        return
    require(type(expected) is type(actual) and expected == actual, f"{path}: value or type mismatch")


def compare_degenerate_t_statistic(
    expected: dict[str, Any],
    actual: dict[str, Any],
    path: str,
    state: dict[str, Any],
) -> bool:
    expected_standard_error = expected.get("standard_error")
    actual_standard_error = actual.get("standard_error")
    standard_errors = (expected_standard_error, actual_standard_error)
    if any(
        isinstance(value, bool) or not isinstance(value, (int, float))
        for value in standard_errors
    ):
        return False
    expected_standard_error = float(expected_standard_error)
    actual_standard_error = float(actual_standard_error)
    if not (
        math.isfinite(expected_standard_error)
        and math.isfinite(actual_standard_error)
        and expected_standard_error <= DEGENERATE_STANDARD_ERROR_THRESHOLD
        and actual_standard_error <= DEGENERATE_STANDARD_ERROR_THRESHOLD
    ):
        return False

    expected_t_statistic = expected.get("t_statistic")
    actual_t_statistic = actual.get("t_statistic")
    if (
        isinstance(expected_t_statistic, bool)
        or isinstance(actual_t_statistic, bool)
        or not isinstance(expected_t_statistic, (int, float))
        or not isinstance(actual_t_statistic, (int, float))
    ):
        return False
    require(
        expected_standard_error >= 0.0 and actual_standard_error >= 0.0,
        f"{path}: corresponding standard errors must be nonnegative",
    )
    expected_t_statistic = float(expected_t_statistic)
    actual_t_statistic = float(actual_t_statistic)
    require(
        math.isfinite(expected_t_statistic) and math.isfinite(actual_t_statistic),
        f"{path}: degenerate-standard-error t statistics must be finite",
    )
    require(
        math.copysign(1.0, expected_t_statistic)
        == math.copysign(1.0, actual_t_statistic),
        f"{path}: degenerate-standard-error t statistics must have the same sign",
    )
    require(
        abs(expected_t_statistic) >= DEGENERATE_T_STATISTIC_MINIMUM_MAGNITUDE
        and abs(actual_t_statistic) >= DEGENERATE_T_STATISTIC_MINIMUM_MAGNITUDE,
        f"{path}: degenerate-standard-error t-statistic magnitude is too small",
    )
    state["numeric_values_compared"] += 1
    state["degenerate_t_statistics_compared"] += 1
    state["max_abs_degenerate_t_statistic_difference"] = max(
        state["max_abs_degenerate_t_statistic_difference"],
        abs(expected_t_statistic - actual_t_statistic),
    )
    return True


def write_json_atomic(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_text(
        json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    os.replace(temporary, path)


def verify(arguments: argparse.Namespace) -> dict[str, Any]:
    repository_root = arguments.repository_root.resolve(strict=True)
    archive_path = arguments.archive.resolve(strict=True)
    executable = arguments.qpls_executable.resolve(strict=True)

    require(arguments.expected_archive_sha256 == ARCHIVE_SHA256, "archive SHA binding is not the frozen value")
    require(arguments.result_id == RESULT_ID, "result ID binding is not the frozen value")
    require(arguments.recipe_id == RECIPE_ID, "recipe ID binding is not the frozen value")
    require(arguments.dataset_id == DATASET_ID, "dataset ID binding is not the frozen value")
    require(arguments.table_id == TABLE_ID, "table ID binding is not the frozen value")
    require(executable.is_file(), f"candidate qpls executable is missing: {executable}")
    require(archive_path.is_file(), f"legacy archive is missing: {archive_path}")

    archive_sha256 = sha256_file(archive_path)
    require(archive_sha256 == ARCHIVE_SHA256, "legacy archive SHA-256 mismatch")
    require(archive_path.stat().st_size == ARCHIVE_SIZE_BYTES, "legacy archive byte length mismatch")

    archive_identity = run_identity_helper(repository_root, archive_path, RESULT_ID, TABLE_ID)
    project = read_project(archive_path)
    target_result = unique_by_id(project.get("results"), RESULT_ID, "project.results")
    target_recipe = unique_by_id(project.get("recipes"), RECIPE_ID, "project.recipes")
    target_dataset = unique_by_id(project.get("datasets"), DATASET_ID, "project.datasets")

    provenance = object_value(target_result.get("provenance"), "target provenance")
    target_payload = object_value(target_result.get("payload"), "target payload")
    expected_stable_provenance = {
        "recipe_id": RECIPE_ID,
        "dataset_fingerprint": DATASET_FINGERPRINT,
        "method": METHOD,
        "method_version": METHOD_VERSION,
        "seed": SEED,
        "settings": provenance.get("settings"),
    }
    actual_target_stable_provenance = {
        key: provenance.get(key) for key in expected_stable_provenance
    }
    require(target_result.get("status") == "completed", "frozen result is not completed")
    require(
        actual_target_stable_provenance == expected_stable_provenance,
        "frozen result stable provenance mismatch",
    )
    require(target_recipe.get("schema_version") == 3, "frozen recipe schema identity mismatch")
    require(target_recipe.get("dataset_fingerprint") == DATASET_FINGERPRINT, "recipe dataset fingerprint mismatch")
    require(target_recipe.get("settings") == provenance.get("settings"), "recipe/result settings mismatch")
    require(
        object_value(target_recipe.get("model"), "target recipe model").get("id") == MODEL_ID,
        "recipe model ID mismatch",
    )
    require(
        object_value(target_recipe.get("method_config"), "target recipe method config").get("kind")
        == "pls_bootstrap",
        "recipe method-config identity mismatch",
    )
    require(target_dataset.get("fingerprint") == DATASET_FINGERPRINT, "dataset fingerprint mismatch")
    require(
        object_value(target_dataset.get("schema"), "target dataset schema").get("case_count") == 120,
        "dataset case-count identity mismatch",
    )

    with tempfile.TemporaryDirectory(prefix="qpls-legacy-moderation-v1-") as temporary_directory:
        candidate_output = Path(temporary_directory) / "candidate-result.json"
        completed = subprocess.run(
            [
                str(executable),
                "run",
                str(archive_path),
                "--recipe-id",
                RECIPE_ID,
                "--output",
                str(candidate_output),
                "--allow-experimental",
            ],
            cwd=repository_root,
            check=False,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=900,
        )
        require(
            completed.returncode == 0,
            "public qpls archive replay failed: " + completed.stderr[-4000:].strip(),
        )
        require(candidate_output.is_file(), "public qpls archive replay emitted no result")
        candidate_result = object_value(json.loads(candidate_output.read_text(encoding="utf-8")), "candidate result")

    require(candidate_result.get("status") == "completed", "candidate replay is not completed")
    candidate_provenance = object_value(candidate_result.get("provenance"), "candidate provenance")
    candidate_stable_provenance = {
        key: candidate_provenance.get(key) for key in expected_stable_provenance
    }
    require(
        candidate_stable_provenance == expected_stable_provenance,
        "candidate stable provenance differs from the frozen result",
    )
    candidate_payload = object_value(candidate_result.get("payload"), "candidate payload")
    comparison = {
        "numeric_values_compared": 0,
        "max_abs_numeric_difference": 0.0,
        "degenerate_t_statistics_compared": 0,
        "max_abs_degenerate_t_statistic_difference": 0.0,
    }
    compare_json(target_payload, candidate_payload, "$.payload", comparison)
    require(comparison["numeric_values_compared"] > 0, "candidate payload comparison covered no numeric values")

    return {
        "schema_version": 1,
        "suite_id": SUITE_ID,
        "passed": True,
        "archive": {
            "path": archive_path.relative_to(repository_root).as_posix(),
            "sha256": archive_sha256,
            "size_bytes": ARCHIVE_SIZE_BYTES,
        },
        "identity": {
            "result_id": RESULT_ID,
            "recipe_id": RECIPE_ID,
            "recipe_schema_version": 3,
            "dataset_id": DATASET_ID,
            "dataset_fingerprint": DATASET_FINGERPRINT,
            "model_id": MODEL_ID,
            "table_id": TABLE_ID,
            "archive_projection": archive_identity,
        },
        "candidate": {
            "qpls_executable_sha256": sha256_file(executable),
            "public_command": (
                "qpls run <archive> --recipe-id <recipe-id> --output <result.json> "
                "--allow-experimental"
            ),
            "experimental_switch_used": True,
        },
        "comparison": {
            "payload_structure_and_non_numeric_values": "exact",
            "numeric_tolerance": NUMERIC_TOLERANCE,
            "degenerate_t_statistic_policy": {
                "corresponding_standard_error_maximum": DEGENERATE_STANDARD_ERROR_THRESHOLD,
                "minimum_absolute_t_statistic": DEGENERATE_T_STATISTIC_MINIMUM_MAGNITUDE,
                "finite_values_required": True,
                "same_sign_required": True,
            },
            **comparison,
            "stable_provenance_fields": sorted(expected_stable_provenance),
            "ignored_identity_fields": [
                "result.id",
                "provenance.engine_version",
                "provenance.started_at",
                "provenance.completed_at",
            ],
        },
    }


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repository-root", required=True, type=Path)
    parser.add_argument("--archive", required=True, type=Path)
    parser.add_argument("--expected-archive-sha256", required=True)
    parser.add_argument("--result-id", required=True)
    parser.add_argument("--recipe-id", required=True)
    parser.add_argument("--dataset-id", required=True)
    parser.add_argument("--table-id", required=True)
    parser.add_argument("--qpls-executable", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    return parser.parse_args()


def main() -> int:
    arguments = parse_arguments()
    try:
        receipt = verify(arguments)
        write_json_atomic(arguments.output, receipt)
        print(json.dumps(receipt, separators=(",", ":"), allow_nan=False))
        return 0
    except Exception as error:  # noqa: BLE001 - qualification CLI must fail closed.
        receipt = {
            "schema_version": 1,
            "suite_id": SUITE_ID,
            "passed": False,
            "error": str(error),
        }
        write_json_atomic(arguments.output, receipt)
        print(json.dumps(receipt, separators=(",", ":")), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
