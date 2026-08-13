#!/usr/bin/env python3
"""Fail-closed promotion audit for structural-path randomization v1.

The method may advance only from its own current evidence. Generic/bootstrap
inference reports are deliberately absent from this audit: the reference,
exact Rust boundary, focused frontend, three-viewport visual, and dedicated
packaged Tauri reports are independently bound by identity, content, hashes,
and freshness.
"""

from __future__ import annotations

import hashlib
import json
import math
import re
import struct
import zipfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable

from jsonschema import Draft202012Validator, FormatChecker


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "structural_path_randomization_method_promotion_audit.json"

FEATURE_ID = "qpls3.inference.structural_path_randomization"
METHOD_VERSION = "freedman_lane_permutation_v1"
CATALOGUE_SNAPSHOT_DATE = "2026-08-12"
TARGET = "quickpls3_structural_path_randomization_v1_promotion"
PLAN_OPERATION = "pls_pm_freedman_lane_v1"
ANALYSIS_RESULT_SCHEMA_VERSION = 1
ANALYSIS_RECIPE_SCHEMA_VERSION = 3
PLS_INFERENCE_PAYLOAD_KIND = "pls_pm_v3"
PROJECT_ARCHIVE_SCHEMA_VERSION = 5
EXPECTED_PROVENANCE_METHOD_VERSION = (
    "pls_pm_v1+pls_mediation_v1+pls_assessment_v7+freedman_lane_permutation_v1"
)

REFERENCE_REPORT = "validation/results/structural_path_randomization_reference_report.json"
BOUNDARY_REPORT = "validation/results/structural_path_randomization_boundary_test_report.json"
FRONTEND_REPORT = "validation/results/structural_path_randomization_frontend_gate_report.json"
VISUAL_REPORT = "validation/results/v247_native_desktop_visual_acceptance.json"
PACKAGED_SOURCE_REPORT = (
    "validation/results/v247_tauri_native_acceptance_structural_path_randomization.json"
)
PACKAGED_REPORT = (
    "validation/results/structural_path_randomization_v1_packaged_acceptance.json"
)
PACKAGED_SCHEMA = "validation/structural_path_randomization_v1_packaged_acceptance.schema.json"
PACKAGED_EVIDENCE_KIND = (
    "quickpls3_scoped_tauri_structural_path_randomization_v1_acceptance"
)
ACCEPTANCE_SCOPE = "structural_path_randomization"
REFERENCE_EVIDENCE_SOURCE_PATHS = (
    "validation/structural_path_randomization_reference.py",
    "validation/structural_path_randomization_reference.R",
    "validation/r_runtime.py",
    "validation/test_structural_path_randomization_reference.py",
    "validation/structural_path_randomization_boundary_gate.py",
    "validation/structural_path_randomization_frontend_gate.py",
    "validation/structural_path_randomization_method_promotion_audit.py",
    "validation/test_structural_path_randomization_method_promotion_audit.py",
    "validation/inference_publication_audit.py",
    "validation/inference_method_promotion_audit.py",
)

EXPECTED_PARAMETERS = (
    '["path",["comp","satisfaction"]]',
    '["path",["like","satisfaction"]]',
    '["path",["satisfaction","loyalty"]]',
)
FROZEN_INDICES = {
    "0": [1, 8, 2, 9, 7, 6, 3, 10, 0, 11, 5, 4],
    "98": [0, 5, 1, 10, 6, 2, 4, 7, 9, 11, 3, 8],
}
REFERENCE_CHECKS = frozenset(
    {
        "release_cli_exact_path_and_freshness",
        "schema_v2_to_v3_recipe_migration",
        "exact_ordered_three_path_manifest",
        "exact_permutation_plan_and_method_version",
        "frozen_deterministic_indices_0_and_98",
        "python_fixed_score_freedman_lane_coefficients",
        "python_exact_exceedances_and_plus_one_probabilities",
        "r_full_coefficient_recomputation",
        "r_exact_exceedances_and_plus_one_probabilities",
        "workers_1_and_4_exact_payload_equality",
        "calibrated_null_boundary",
        "calibrated_power_boundary",
    }
)
BOUNDARY_SUITES = {
    "qpls_resampling": frozenset(
        {
            "indexed_permutations_are_bijections_and_replicate_specific",
            "freedman_lane_reference_seam_matches_orthogonal_multi_predictor_fixture_and_rejects_invalid_indices",
            "pls_freedman_lane_multi_path_is_seeded_worker_invariant_and_progressive",
            "pls_freedman_lane_multi_path_cancellation_discards_partial_output",
            "permutation_wire_contract_rejects_unknown_fields",
        }
    ),
    "qpls_project": frozenset(
        {
            "permutation_pls_multi_path_appends_round_trips_and_rejects_manifest_tampering",
        }
    ),
}
BOUNDARY_SOURCE_CRATES = {
    "qpls_resampling": (
        "qpls-core",
        "qpls-data",
        "qpls-estimation",
        "qpls-resampling",
    ),
    "qpls_project": (
        "qpls-core",
        "qpls-data",
        "qpls-estimation",
        "qpls-assessment",
        "qpls-resampling",
        "qpls-runner",
        "qpls-project",
    ),
}
FRONTEND_TEST_FILES = (
    "src/native/nativeStructuralPathRandomization.test.ts",
    "src/native/nativeAnalysisCatalog.test.ts",
    "src/native/nativeAnalysisRecipe.test.ts",
    "src/native/nativeCalculationRequest.test.ts",
    "src/native/NativeCalculationDialog.test.ts",
    "src/native/nativeCanonicalProject.test.ts",
    "src/native/nativeResults.test.ts",
    "src/native/nativeExportTables.test.ts",
    "src/domain/resultInterpretation.test.ts",
    "src/store.test.ts",
    "src/components/ReportsWorkspace.test.ts",
    "src/native/NativeExportDialog.test.ts",
    "src/domain/publicationDiagram.test.ts",
    "src/components/TrustCenterWorkspace.test.tsx",
    "src/native/nativeControllerContracts.test.ts",
)
VISUAL_VIEWPORTS = {
    "1024x700": (1024, 700),
    "1280x720": (1280, 720),
    "1440x900": (1440, 900),
}
VISUAL_RANDOMIZATION_STATE = "structural-path-randomization-dialog"
VISUAL_RANDOMIZATION_SCREENSHOTS = tuple(
    {
        "path": (
            "validation/results/screens/v247-native-desktop-visual/"
            f"11-{VISUAL_RANDOMIZATION_STATE}-{viewport}.png"
        ),
        "viewport": viewport,
        "state": VISUAL_RANDOMIZATION_STATE,
    }
    for viewport in VISUAL_VIEWPORTS
)
PACKAGED_SOURCE_CHECKS = frozenset(
    {
        "runtimePreflight",
        "structuralPathRandomizationFixtureProvisioning",
        "structuralPathRandomizationSetup",
        "structuralPathRandomizationCancellation",
        "structuralPathRandomizationResults",
        "structuralPathRandomizationExport",
        "structuralPathRandomizationArchive",
        "structuralPathRandomizationSaveReopen",
        "resources",
        "cleanup",
    }
)
PACKAGED_SOURCE_METADATA_CHECKS = frozenset({"runtime", "recentProjectsRestored"})
PACKAGED_VALIDATION_SOURCE_PATHS = (
    "validation/v247_tauri_native_acceptance.mjs",
    "validation/run_v247_structural_path_randomization_native_acceptance.ps1",
    "validation/monitor_quickpls_process_tree.ps1",
    "validation/windows_native_save_export.py",
    "validation/close_tauri_test_window.mjs",
)
PACKAGED_RESULT_COLUMNS = (
    "Path",
    "Original",
    "Exceedances",
    "Permutations",
    "Raw two-sided p",
)
PACKAGED_PATH_LABELS = ("X -> Y", "Z -> Y")
PACKAGED_WORKBOOK_SHEETS = (
    "Path coefficients",
    "Outer loadings",
    "Outer weights",
    "R-square",
    "Total effects",
    "Construct reliability and valid",
    "Cross loadings",
    "Fornell-Larcker criterion",
    "HTMT+",
    "Original HTMT",
    "Structural model",
    "Inner VIF values",
    "f-square effect sizes",
    "Model fit",
    "Construct cross-validated redun",
    "Structural path randomization",
    "Run provenance",
)
PACKAGED_WARNING = (
    "Candidate output: single-model Freedman-Lane randomization holds the original "
    "PLS construct scores fixed and reports unadjusted pathwise two-sided plus-one p "
    "values. Interpret these as conditional, approximate inference under exchangeable "
    "reduced-model residuals. Measurement-score uncertainty is not re-estimated, no "
    "multiplicity adjustment is applied, and current calibration covers homoscedastic "
    "Gaussian errors only."
)
PACKAGED_PROBABILITY_DISCLOSURE = (
    "Conditional/approximate two-sided plus-one probability under exchangeable "
    "reduced-model residuals; no multiplicity adjustment"
)
PACKAGED_QUALIFICATION_DISCLOSURE = (
    "Internal candidate/experimental product label; method-specific qualification "
    "evidence is tracked separately"
)
PACKAGED_SHARED_STRINGS = (
    "Structural path randomization",
    "Run provenance",
    "experimental",
    PACKAGED_WARNING,
    "Randomization method",
    METHOD_VERSION,
    "Randomization operation",
    PLAN_OPERATION,
    "Randomized structural paths",
    "2",
    "Requested path permutations",
    "10000",
    "Randomization estimand",
    "Structural path coefficients conditional on fixed original PLS construct scores",
    "Pathwise probability",
    PACKAGED_PROBABILITY_DISCLOSURE,
    "Qualification status",
    PACKAGED_QUALIFICATION_DISCLOSURE,
)


def load_optional(path: Path) -> dict[str, Any]:
    if not path.is_file():
        return {}
    try:
        value = json.loads(path.read_text(encoding="utf-8-sig"))
    except (OSError, UnicodeError, json.JSONDecodeError):
        return {}
    return value if isinstance(value, dict) else {}


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def finite_number(value: Any) -> bool:
    return (
        isinstance(value, (int, float))
        and not isinstance(value, bool)
        and math.isfinite(float(value))
    )


def reference_worker_rows_passed(
    worker_rows: list[Any], expected_fingerprint: Any
) -> bool:
    """Bind reference worker evidence to the frozen result-envelope contract."""
    return (
        isinstance(expected_fingerprint, str)
        and bool(expected_fingerprint)
        and len(worker_rows) == 2
        and all(isinstance(row, dict) for row in worker_rows)
        and [row.get("workers") for row in worker_rows] == [1, 4]
        and all(
            row.get("result_schema_version") == ANALYSIS_RESULT_SCHEMA_VERSION
            and row.get("recipe_id")
            == f"00000000-0000-0000-0000-{32_000 + row['workers']:012d}"
            and row.get("dataset_fingerprint") == expected_fingerprint
            and row.get("method_version") == EXPECTED_PROVENANCE_METHOD_VERSION
            and row.get("construct_score_rows") == 12
            and isinstance(row.get("parameters"), list)
            and len(row["parameters"]) == 3
            for row in worker_rows
        )
    )


def f64_bits(value: float) -> bytes:
    return struct.pack(">d", value)


def json_exact(left: Any, right: Any) -> bool:
    if type(left) is not type(right):
        return False
    if isinstance(left, float):
        return math.isfinite(left) and math.isfinite(right) and f64_bits(left) == f64_bits(right)
    if isinstance(left, list):
        return len(left) == len(right) and all(
            json_exact(left_item, right_item)
            for left_item, right_item in zip(left, right)
        )
    if isinstance(left, dict):
        return left.keys() == right.keys() and all(
            json_exact(left[key], right[key]) for key in left
        )
    return left == right


def strict_json_bytes(value: bytes) -> Any:
    def object_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, item in pairs:
            if key in result:
                raise ValueError(f"duplicate JSON key {key}")
            result[key] = item
        return result

    return json.loads(value.decode("utf-8"), object_pairs_hook=object_pairs)


def exact_plus_one_probability(text: Any, exceedances: int, permutations: int) -> bool:
    if not isinstance(text, str) or not text or text != text.strip():
        return False
    try:
        observed = float(text)
    except ValueError:
        return False
    expected = (exceedances + 1) / (permutations + 1)
    return math.isfinite(observed) and f64_bits(observed) == f64_bits(expected)


def inspect_cancellation_archive_snapshot(
    root: Path, descriptor: Any, expected_phase: str
) -> dict[str, Any]:
    artifact = artifact_attestation(root, descriptor)
    outcome: dict[str, Any] = {
        "artifact": artifact,
        "contract": {},
        "errors": [],
        "passed": False,
    }
    relative = descriptor.get("path") if isinstance(descriptor, dict) else None
    expected_pattern = (
        r"^validation/results/v247-native-structural-path-randomization-"
        r"[0-9]+-[0-9]+-cancellation-" + re.escape(expected_phase) + r"\.qpls$"
    )
    if (
        not artifact["passed"]
        or not isinstance(relative, str)
        or re.fullmatch(expected_pattern, relative) is None
    ):
        outcome["errors"].append("snapshot_artifact_attestation_failed")
        return outcome
    archive_path = (root / relative).resolve()
    try:
        with zipfile.ZipFile(archive_path, "r") as archive:
            infos = archive.infolist()
            names = [info.filename for info in infos]
            if (
                len(infos) > 16_384
                or sum(info.file_size for info in infos) > 8 * 1024 * 1024 * 1024
                or len(names) != len(set(names))
                or "manifest.json" not in names
                or "project.json" not in names
                or any(
                    info.is_dir()
                    or info.flag_bits & 0x1
                    or len(info.filename.encode("utf-8")) > 1_024
                    or info.file_size > 512 * 1024 * 1024
                    or "\\" in info.filename
                    or info.filename.startswith("/")
                    or any(part in ("", ".", "..") for part in info.filename.split("/"))
                    for info in infos
                )
            ):
                raise ValueError("snapshot archive entry set is duplicate, encrypted, or unsafe")
            info_by_name = {info.filename: info for info in infos}
            if info_by_name["manifest.json"].file_size > 32 * 1024 * 1024:
                raise ValueError("snapshot manifest.json exceeds the inspection limit")
            if info_by_name["project.json"].file_size > 256 * 1024 * 1024:
                raise ValueError("snapshot project.json exceeds the inspection limit")
            manifest = strict_json_bytes(archive.read("manifest.json"))
            project = strict_json_bytes(archive.read("project.json"))
            if not isinstance(manifest, dict) or set(manifest) != {
                "schema_version", "project_id", "name", "created_at", "modified_at",
                "engine_version", "checksum_algorithm", "checksums",
            }:
                raise ValueError("snapshot manifest.json does not have the exact v5 shape")
            checksums = manifest.get("checksums")
            if (
                manifest.get("schema_version") != 5
                or manifest.get("checksum_algorithm") != "sha256"
                or manifest.get("name") != "Native Structural Path Randomization Acceptance"
                or not isinstance(manifest.get("project_id"), str)
                or not manifest["project_id"]
                or not isinstance(manifest.get("engine_version"), str)
                or not manifest["engine_version"]
                or not isinstance(checksums, dict)
                or "manifest.json" in checksums
                or set(checksums) != set(names) - {"manifest.json"}
            ):
                raise ValueError("snapshot manifest identity/checksum contract is invalid")
            for name, expected_digest in checksums.items():
                if (
                    not isinstance(name, str)
                    or not isinstance(expected_digest, str)
                    or re.fullmatch(r"[0-9a-f]{64}", expected_digest) is None
                ):
                    raise ValueError("snapshot checksum declaration is malformed")
                digest = hashlib.sha256()
                with archive.open(name, "r") as entry:
                    for block in iter(lambda: entry.read(1024 * 1024), b""):
                        digest.update(block)
                if digest.hexdigest() != expected_digest:
                    raise ValueError(f"snapshot checksum mismatch for {name}")
    except (OSError, UnicodeError, ValueError, KeyError, json.JSONDecodeError, zipfile.BadZipFile) as error:
        outcome["errors"].append(f"{type(error).__name__}: {error}")
        return outcome

    if not isinstance(project, dict) or set(project) != {
        "datasets", "models", "recipes", "layouts", "results",
    }:
        outcome["errors"].append("snapshot project.json does not have the exact document shape")
        return outcome
    datasets = project.get("datasets")
    models = project.get("models")
    recipes = project.get("recipes")
    results = project.get("results")
    layouts = project.get("layouts")
    if not (
        isinstance(datasets, list) and len(datasets) == 1
        and isinstance(models, list) and len(models) == 1
        and isinstance(recipes, list) and len(recipes) == 0
        and isinstance(results, list) and len(results) == 0
        and isinstance(layouts, dict)
        and isinstance(datasets[0], dict)
        and isinstance(models[0], dict)
    ):
        outcome["errors"].append("snapshot is not the exact one-dataset/one-model/zero-result state")
        return outcome
    dataset = datasets[0]
    model = models[0]
    workspace = layouts.get("workspace") if isinstance(layouts.get("workspace"), dict) else {}
    runs = workspace.get("runs") if isinstance(workspace.get("runs"), list) else []
    constructs = model.get("constructs") if isinstance(model.get("constructs"), list) else []
    paths = model.get("paths") if isinstance(model.get("paths"), list) else []
    if not all(isinstance(value, dict) for value in (*constructs, *paths, *runs)):
        outcome["errors"].append("snapshot construct/path/run arrays are not typed objects")
        return outcome
    labels = {
        construct.get("id"): construct.get("name")
        for construct in constructs
        if isinstance(construct.get("id"), str) and isinstance(construct.get("name"), str)
    }
    construct_labels = [construct.get("name") for construct in constructs]
    path_labels = [
        f"{labels.get(row.get('source'))} -> {labels.get(row.get('target'))}"
        for row in paths
    ]
    dataset_id = dataset.get("id")
    snapshot_valid = (
        isinstance(dataset_id, str)
        and f"data/{dataset_id}.arrow" in checksums
        and model.get("name") == "Structural Path Randomization Model"
        and workspace.get("activeModelId") == model.get("id")
        and construct_labels == ["X", "Z", "Y"]
        and path_labels == ["X -> Y", "Z -> Y"]
        and model.get("controls") == []
        and model.get("higher_order_constructs") == []
        and model.get("interactions") == []
        and runs == []
    )
    contract = {
        "datasetCount": len(datasets),
        "modelCount": len(models),
        "modelName": model.get("name"),
        "constructLabels": construct_labels,
        "pathLabels": path_labels,
        "recipeCount": len(recipes),
        "resultCount": len(results),
        "runCount": len(runs),
        "recipeIds": [],
        "resultIds": [],
        "runIds": [],
    }
    outcome["contract"] = contract
    outcome["passed"] = snapshot_valid
    if not snapshot_valid:
        outcome["errors"].append("snapshot logical model state is not exact")
    return outcome


def packaged_cancellation_contract(
    packaged_value: Any,
    source_value: Any,
    artifacts_value: Any,
    root: Path,
) -> dict[str, Any]:
    packaged = packaged_value if isinstance(packaged_value, dict) else {}
    source = source_value if isinstance(source_value, dict) else {}
    expected_keys = {
        "passed", "activeLifecycleCaptured", "activeState", "cancelButtonCount",
        "cancelButtonEnabled", "cancelClickDispatched", "terminalOutcome",
        "completionWonRace", "cancelledState", "cancelledMessage",
        "cancellationLogNewest", "cancellationLogExact",
        "archiveBeforeSnapshot", "archiveAfterSnapshot", "archiveSnapshotsByteIdentical",
        "zeroResultRecipeRunDelta", "noPartialResult", "cancellationSetup",
        "exactFrozenSetupOnRetry", "retrySetup", "completionSetup", "completionSetupExact",
        "retryCompleted", "retryRunId", "retryNewIdentity",
    }
    state_keys = {
        "ariaBusy", "status", "phase", "message", "progressValue", "progressMax",
        "logEntries",
    }

    def active_state_contract(value: Any) -> bool:
        return (
            isinstance(value, dict)
            and set(value) == state_keys
            and value.get("ariaBusy") == "true"
            and value.get("status") in {"queued", "validating", "running"}
            and isinstance(value.get("phase"), str)
            and bool(value["phase"].strip())
            and isinstance(value.get("message"), str)
            and (value.get("progressValue") is None or isinstance(value.get("progressValue"), str))
            and (value.get("progressMax") is None or isinstance(value.get("progressMax"), str))
            and isinstance(value.get("logEntries"), int)
            and not isinstance(value.get("logEntries"), bool)
            and value["logEntries"] >= 0
        )

    cancelled_state_keys = {*state_keys, "logMessages"}

    def cancelled_state_contract(value: Any) -> bool:
        return (
            isinstance(value, dict)
            and set(value) == cancelled_state_keys
            and value.get("ariaBusy") == "false"
            and value.get("status") == "cancelled"
            and value.get("phase") == "Cancelled"
            and value.get("message") == "Calculation cancelled."
            and (value.get("progressValue") is None or isinstance(value.get("progressValue"), str))
            and (value.get("progressMax") is None or isinstance(value.get("progressMax"), str))
            and isinstance(value.get("logEntries"), int)
            and not isinstance(value.get("logEntries"), bool)
            and value["logEntries"] >= 2
            and isinstance(value.get("logMessages"), list)
            and all(isinstance(row, str) for row in value["logMessages"])
            and value["logMessages"][:2]
            == ["Calculation cancelled.", "Cancellation requested."]
        )

    setup_keys = {
        "catalogCount", "selectedMethod", "permutations", "workers", "seed",
        "bootstrapControls", "groupControls", "scopeLabel", "scope", "blockers",
        "startLabel", "startEnabled",
    }

    def setup_readback_contract(value: Any, workers: int, start_label: str) -> bool:
        if not isinstance(value, dict) or set(value) != setup_keys:
            return False
        return (
            value.get("catalogCount") == 14
            and value.get("selectedMethod") == "Structural Path Randomization"
            and value.get("permutations") == {
                "count": 1, "type": "number", "minimum": "99", "maximum": "10000",
                "step": "1", "value": "10000",
            }
            and value.get("workers") == {
                "count": 1, "type": "number", "minimum": "1", "maximum": "64",
                "value": str(workers),
            }
            and value.get("seed") == {
                "count": 1, "type": "number", "minimum": "0",
                "maximum": "4294967295", "value": "20260718",
            }
            and value.get("bootstrapControls") == 0
            and value.get("groupControls") == 0
            and value.get("scopeLabel") == "Candidate scope"
            and value.get("scope") == PACKAGED_WARNING
            and value.get("blockers") == []
            and value.get("startLabel") == start_label
            and value.get("startEnabled") is True
        )

    def archive_snapshot_contract(value: Any, phase: str, actual: dict[str, Any]) -> bool:
        artifact = value.get("artifact") if isinstance(value, dict) else None
        logical = actual.get("contract") if isinstance(actual, dict) else None
        return (
            isinstance(value, dict)
            and set(value) == {
                "phase", "artifact", "sourcePath", "sourceSize", "sourceSha256",
                "sourceMtimeNsBefore", "sourceMtimeNsAfter", "sourceStableDuringCopy",
                "snapshotMatchesSource", "datasetCount", "modelCount", "modelName",
                "constructLabels", "pathLabels", "recipeCount", "resultCount",
                "runCount", "recipeIds", "resultIds", "runIds",
            }
            and value.get("phase") == phase
            and isinstance(artifact, dict)
            and set(artifact) == {"path", "size", "sha256"}
            and re.fullmatch(
                r"validation/results/v247-native-structural-path-randomization-"
                rf"[0-9]+-[0-9]+-cancellation-{phase}\.qpls",
                artifact.get("path", ""),
            ) is not None
            and isinstance(artifact.get("size"), int)
            and not isinstance(artifact.get("size"), bool)
            and artifact["size"] > 0
            and isinstance(artifact.get("sha256"), str)
            and re.fullmatch(r"[0-9a-f]{64}", artifact["sha256"]) is not None
            and isinstance(value.get("sourcePath"), str)
            and re.fullmatch(
                r"validation/results/v247-native-structural-path-randomization-[0-9]+-[0-9]+\.qpls",
                value["sourcePath"],
            ) is not None
            and value.get("sourceSize") == artifact["size"]
            and value.get("sourceSha256") == artifact["sha256"]
            and isinstance(value.get("sourceMtimeNsBefore"), str)
            and value["sourceMtimeNsBefore"].isdigit()
            and value.get("sourceMtimeNsAfter") == value["sourceMtimeNsBefore"]
            and value.get("sourceStableDuringCopy") is True
            and value.get("snapshotMatchesSource") is True
            and actual.get("passed") is True
            and isinstance(logical, dict)
            and all(json_exact(value.get(key), logical.get(key)) for key in logical)
        )

    artifacts = artifacts_value if isinstance(artifacts_value, dict) else {}
    before_value = packaged.get("archiveBeforeSnapshot")
    after_value = packaged.get("archiveAfterSnapshot")
    before_top = artifacts.get("cancellation_archive_before")
    after_top = artifacts.get("cancellation_archive_after")
    before_actual = inspect_cancellation_archive_snapshot(root, before_top, "before")
    after_actual = inspect_cancellation_archive_snapshot(root, after_top, "after")
    source_match = bool(packaged) and json_exact(packaged, source)
    exact_keys = set(packaged) == expected_keys
    active = active_state_contract(packaged.get("activeState"))
    cancelled = cancelled_state_contract(packaged.get("cancelledState"))
    cancellation_setup = setup_readback_contract(
        packaged.get("cancellationSetup"), 1, "Start path randomization"
    )
    retry_setup = setup_readback_contract(
        packaged.get("retrySetup"), 1, "Retry path randomization"
    )
    completion_setup = setup_readback_contract(
        packaged.get("completionSetup"), 4, "Retry path randomization"
    )
    archive_before = archive_snapshot_contract(before_value, "before", before_actual)
    archive_after = archive_snapshot_contract(after_value, "after", after_actual)
    top_level_binding = (
        isinstance(before_value, dict)
        and isinstance(after_value, dict)
        and json_exact(before_value.get("artifact"), before_top)
        and json_exact(after_value.get("artifact"), after_top)
    )
    archive_identity = archive_before and archive_after and (
        before_value["artifact"]["path"] != after_value["artifact"]["path"]
        and before_value["artifact"]["size"] == after_value["artifact"]["size"]
        and before_value["artifact"]["sha256"] == after_value["artifact"]["sha256"]
        and before_value["sourcePath"] == after_value["sourcePath"]
    )
    passed = (
        source_match
        and exact_keys
        and packaged.get("passed") is True
        and packaged.get("activeLifecycleCaptured") is True
        and packaged.get("cancelButtonCount") == 1
        and packaged.get("cancelButtonEnabled") is True
        and packaged.get("cancelClickDispatched") is True
        and packaged.get("terminalOutcome") == "cancelled"
        and packaged.get("completionWonRace") is False
        and active
        and cancelled
        and packaged.get("cancelledMessage") == "Calculation cancelled."
        and packaged.get("cancellationLogNewest")
        == ["Calculation cancelled.", "Cancellation requested."]
        and packaged.get("cancellationLogExact") is True
        and top_level_binding
        and archive_identity
        and packaged.get("archiveSnapshotsByteIdentical") is True
        and packaged.get("zeroResultRecipeRunDelta") is True
        and packaged.get("noPartialResult") is True
        and cancellation_setup
        and packaged.get("exactFrozenSetupOnRetry") is True
        and retry_setup
        and completion_setup
        and packaged.get("completionSetupExact") is True
        and packaged.get("retryCompleted") is True
        and isinstance(packaged.get("retryRunId"), str)
        and bool(packaged["retryRunId"].strip())
        and packaged.get("retryNewIdentity") is True
    )
    return {
        "source_match": source_match,
        "exact_keys": exact_keys,
        "active_state": active,
        "cancelled_state": cancelled,
        "cancellation_setup": cancellation_setup,
        "retry_setup": retry_setup,
        "completion_setup": completion_setup,
        "top_level_binding": top_level_binding,
        "archive_before_snapshot": before_actual,
        "archive_after_snapshot": after_actual,
        "archive_identity": archive_identity,
        "passed": passed,
    }


def packaged_results_contract(value: Any) -> dict[str, Any]:
    check = value if isinstance(value, dict) else {}
    rows = check.get("rows") if isinstance(check.get("rows"), list) else []
    row_contract = len(rows) == len(PACKAGED_PATH_LABELS)
    parsed_rows: list[dict[str, Any]] = []
    if row_contract:
        for expected_path, row in zip(PACKAGED_PATH_LABELS, rows):
            if not isinstance(row, list) or len(row) != 5:
                row_contract = False
                break
            path, original_text, exceedance_text, permutation_text, probability_text = row
            integer_text = (
                isinstance(exceedance_text, str)
                and re.fullmatch(r"(?:0|[1-9][0-9]*)", exceedance_text) is not None
            )
            exceedances = int(exceedance_text) if integer_text else -1
            original = None
            if isinstance(original_text, str) and original_text == original_text.strip():
                try:
                    original = float(original_text)
                except ValueError:
                    original = None
            valid = (
                path == expected_path
                and original is not None
                and math.isfinite(original)
                and integer_text
                and 0 <= exceedances <= 10_000
                and permutation_text == "10000"
                and exact_plus_one_probability(probability_text, exceedances, 10_000)
            )
            parsed_rows.append(
                {
                    "path": path,
                    "original": original,
                    "exceedances": exceedances,
                    "permutations": 10_000,
                    "p_value_text": probability_text,
                    "passed": valid,
                }
            )
            row_contract = row_contract and valid

    details = check.get("runDetails") if isinstance(check.get("runDetails"), dict) else {}
    properties = details.get("properties") if isinstance(details.get("properties"), dict) else {}
    run_details = (
        properties.get("Method") == "Structural Path Randomization"
        and properties.get("Recorded seed") == "20260718"
        and properties.get("Method version") == EXPECTED_PROVENANCE_METHOD_VERSION
        and isinstance(properties.get("Recipe"), str)
        and bool(properties["Recipe"].strip())
        and isinstance(properties.get("Dataset fingerprint"), str)
        and bool(properties["Dataset fingerprint"].strip())
        and isinstance(details.get("logEntries"), int)
        and not isinstance(details.get("logEntries"), bool)
        and details["logEntries"] >= 1
    )
    exact_identity = (
        check.get("passed") is True
        and check.get("runId") == check.get("selectedRunId")
        and isinstance(check.get("runId"), str)
        and bool(check["runId"])
        and check.get("initialSelectedTable") == "model_estimates"
        and check.get("group") == "Inference"
        and check.get("tableId") == "permutation"
        and check.get("title") == "Structural path randomization"
        and check.get("warning") == PACKAGED_WARNING
        and check.get("columns") == list(PACKAGED_RESULT_COLUMNS)
        and check.get("pathOrder") == list(PACKAGED_PATH_LABELS)
        and check.get("noBootstrapTables") is True
        and check.get("noPlaceholderValues") is True
    )
    return {
        "exact_identity": exact_identity,
        "rows": parsed_rows,
        "row_arithmetic": row_contract,
        "run_details": run_details,
        "passed": exact_identity and row_contract and run_details,
    }


def observed_path_matches_artifact(root: Path, observed: Any, descriptor: Any) -> bool:
    if not isinstance(observed, str) or not observed:
        return False
    relative = descriptor.get("path") if isinstance(descriptor, dict) else None
    if not isinstance(relative, str) or not relative:
        return False
    try:
        expected = (root / relative).resolve()
        expected.relative_to(root.resolve())
        return Path(observed).resolve() == expected
    except (OSError, ValueError):
        return False


def packaged_export_contract(
    value: Any,
    artifact: Any = None,
    root: Path = ROOT,
) -> dict[str, Any]:
    check = value if isinstance(value, dict) else {}
    native = check.get("nativeXlsx") if isinstance(check.get("nativeXlsx"), dict) else {}
    helper = native.get("helper") if isinstance(native.get("helper"), dict) else {}
    ready = helper.get("ready") if isinstance(helper.get("ready"), dict) else {}
    completion = helper.get("completion") if isinstance(helper.get("completion"), dict) else {}
    workbook = completion.get("workbook") if isinstance(completion.get("workbook"), dict) else {}
    exact_expected = (
        check.get("expectedSheets")
        == ["Structural path randomization", "Run provenance"]
        and check.get("expectedSharedStrings") == list(PACKAGED_SHARED_STRINGS)
    )
    workbook_binding = (
        completion.get("passed") is True
        and workbook.get("requiredSharedStrings") == list(PACKAGED_SHARED_STRINGS)
        and workbook.get("sheetNames") == list(PACKAGED_WORKBOOK_SHEETS)
        and workbook.get("requiredSheets")
        == ["Structural path randomization", "Run provenance"]
        and native.get("workbookSheets") == list(PACKAGED_WORKBOOK_SHEETS)
        and native.get("exactRequiredSheets") is True
        and native.get("noBootstrapSheets") is True
    )
    artifact_row = artifact_attestation(root, artifact)
    artifact_path = artifact.get("path") if isinstance(artifact, dict) else None
    artifact_size = artifact.get("size") if isinstance(artifact, dict) else None
    artifact_sha256 = artifact.get("sha256") if isinstance(artifact, dict) else None
    artifact_binding = (
        artifact_row["passed"]
        and isinstance(artifact_path, str)
        and artifact_path.startswith("validation/results/")
        and artifact_path.lower().endswith(".xlsx")
        and observed_path_matches_artifact(root, native.get("targetPath"), artifact)
        and observed_path_matches_artifact(root, ready.get("targetPath"), artifact)
        and observed_path_matches_artifact(root, completion.get("targetPath"), artifact)
        and observed_path_matches_artifact(root, workbook.get("path"), artifact)
        and native.get("file") == {"size": artifact_size, "isFile": True}
        and workbook.get("size") == artifact_size
        and workbook.get("sha256") == artifact_sha256
    )
    return {
        "exact_expected_content": exact_expected,
        "workbook_binding": workbook_binding,
        "artifact_binding": artifact_binding,
        "passed": (
            check.get("passed") is True
            and exact_expected
            and workbook_binding
            and artifact_binding
        ),
    }


def inspect_packaged_archive_artifact(root: Path, descriptor: Any) -> dict[str, Any]:
    artifact = artifact_attestation(root, descriptor)
    outcome: dict[str, Any] = {
        "artifact": artifact,
        "contract": {},
        "errors": [],
        "passed": False,
    }
    if not artifact["passed"] or not isinstance(descriptor, dict):
        outcome["errors"].append("artifact_attestation_failed")
        return outcome
    archive_path = (root / descriptor["path"]).resolve()
    try:
        with zipfile.ZipFile(archive_path, "r") as archive:
            infos = archive.infolist()
            names = [info.filename for info in infos]
            if (
                len(infos) > 16_384
                or sum(info.file_size for info in infos) > 8 * 1024 * 1024 * 1024
                or len(names) != len(set(names))
                or "manifest.json" not in names
                or "project.json" not in names
                or any(
                    info.is_dir()
                    or info.flag_bits & 0x1
                    or len(info.filename.encode("utf-8")) > 1_024
                    or info.file_size > 512 * 1024 * 1024
                    or "\\" in info.filename
                    or info.filename.startswith("/")
                    or any(part in ("", ".", "..") for part in info.filename.split("/"))
                    for info in infos
                )
            ):
                raise ValueError("archive entry set is duplicate, encrypted, or unsafe")
            info_by_name = {info.filename: info for info in infos}
            if info_by_name["manifest.json"].file_size > 32 * 1024 * 1024:
                raise ValueError("manifest.json exceeds the promotion inspection limit")
            if info_by_name["project.json"].file_size > 256 * 1024 * 1024:
                raise ValueError("project.json exceeds the promotion inspection limit")
            manifest_bytes = archive.read("manifest.json")
            project_bytes = archive.read("project.json")
            manifest = strict_json_bytes(manifest_bytes)
            project = strict_json_bytes(project_bytes)
            if not isinstance(manifest, dict) or set(manifest) != {
                "schema_version", "project_id", "name", "created_at", "modified_at",
                "engine_version", "checksum_algorithm", "checksums",
            }:
                raise ValueError("manifest.json does not have the exact v5 shape")
            checksums = manifest.get("checksums")
            if (
                manifest.get("schema_version") != 5
                or manifest.get("checksum_algorithm") != "sha256"
                or not isinstance(manifest.get("project_id"), str)
                or not manifest["project_id"]
                or manifest.get("name") != "Native Structural Path Randomization Acceptance"
                or not isinstance(manifest.get("engine_version"), str)
                or not manifest["engine_version"]
                or not isinstance(checksums, dict)
                or "manifest.json" in checksums
                or set(checksums) != set(names) - {"manifest.json"}
                or any(
                    not isinstance(name, str)
                    or not isinstance(digest, str)
                    or re.fullmatch(r"[0-9a-f]{64}", digest) is None
                    for name, digest in checksums.items()
                )
            ):
                raise ValueError("manifest identity/checksum contract is invalid")
            computed_checksums: dict[str, str] = {}
            for name, expected_digest in checksums.items():
                digest = hashlib.sha256()
                with archive.open(name, "r") as entry:
                    for block in iter(lambda: entry.read(1024 * 1024), b""):
                        digest.update(block)
                computed = digest.hexdigest()
                computed_checksums[name] = computed
                if computed != expected_digest:
                    raise ValueError(f"checksum mismatch for {name}")
    except (OSError, UnicodeError, ValueError, KeyError, json.JSONDecodeError, zipfile.BadZipFile) as error:
        outcome["errors"].append(f"{type(error).__name__}: {error}")
        return outcome

    if not isinstance(project, dict) or set(project) != {
        "datasets", "models", "recipes", "layouts", "results",
    }:
        outcome["errors"].append("project.json does not have the exact document shape")
        return outcome
    datasets = project.get("datasets")
    models = project.get("models")
    recipes = project.get("recipes")
    layouts = project.get("layouts")
    results = project.get("results")
    if not (
        isinstance(datasets, list) and len(datasets) == 1
        and isinstance(models, list) and len(models) == 1
        and isinstance(recipes, list) and len(recipes) == 1
        and isinstance(results, list) and len(results) == 1
        and isinstance(layouts, dict)
        and all(isinstance(value, dict) for value in (*datasets, *models, *recipes, *results))
    ):
        outcome["errors"].append("project does not contain exactly one dataset/model/recipe/result")
        return outcome

    dataset, model, recipe, result = datasets[0], models[0], recipes[0], results[0]
    recipe_model = recipe.get("model") if isinstance(recipe.get("model"), dict) else {}
    settings = recipe.get("settings") if isinstance(recipe.get("settings"), dict) else {}
    provenance = result.get("provenance") if isinstance(result.get("provenance"), dict) else {}
    provenance_settings = provenance.get("settings") if isinstance(provenance.get("settings"), dict) else {}
    payload = result.get("payload") if isinstance(result.get("payload"), dict) else {}
    estimation = payload.get("estimation") if isinstance(payload.get("estimation"), dict) else {}
    permutation = payload.get("permutation") if isinstance(payload.get("permutation"), dict) else {}
    plan = permutation.get("plan") if isinstance(permutation.get("plan"), dict) else {}
    parameters = permutation.get("parameters") if isinstance(permutation.get("parameters"), list) else []
    constructs = recipe_model.get("constructs") if isinstance(recipe_model.get("constructs"), list) else []
    paths = recipe_model.get("paths") if isinstance(recipe_model.get("paths"), list) else []
    result_paths = estimation.get("paths") if isinstance(estimation.get("paths"), list) else []
    if not all(isinstance(value, dict) for value in (*constructs, *paths, *result_paths, *parameters)):
        outcome["errors"].append("construct/path/parameter arrays are not typed objects")
        return outcome
    construct_labels = {
        construct.get("id"): construct.get("name")
        for construct in constructs
        if isinstance(construct.get("id"), str) and isinstance(construct.get("name"), str)
    }
    construct_names = [construct.get("name") for construct in constructs]
    expected_label_pairs = [["X", "Y"], ["Z", "Y"]]
    canonical_paths = [
        path
        for construct in constructs
        for path in paths
        if path.get("target") == construct.get("id")
    ]
    recipe_path_ids = [[path.get("source"), path.get("target")] for path in canonical_paths]
    result_path_ids = [[path.get("source"), path.get("target")] for path in result_paths]
    recipe_label_pairs = [
        [construct_labels.get(source), construct_labels.get(target)]
        for source, target in recipe_path_ids
    ]
    result_label_pairs = [
        [construct_labels.get(source), construct_labels.get(target)]
        for source, target in result_path_ids
    ]
    path_labels = [f"{source} -> {target}" for source, target in result_label_pairs]
    expected_parameter_ids = [
        json.dumps(["path", ids], separators=(",", ":")) for ids in result_path_ids
    ]
    parameter_contract = len(parameters) == len(result_paths) == 2
    for index, parameter in enumerate(parameters):
        result_path = result_paths[index] if index < len(result_paths) else {}
        exceedances = parameter.get("exceedances")
        permutations = parameter.get("permutations")
        probability = parameter.get("p_value_two_sided")
        original = parameter.get("original")
        coefficient = result_path.get("coefficient")
        parameter_contract = parameter_contract and (
            set(parameter) == {
                "exceedances", "original", "p_value_two_sided", "parameter", "permutations",
            }
            and parameter.get("parameter") == expected_parameter_ids[index]
            and finite_number(original)
            and finite_number(coefficient)
            and f64_bits(float(original)) == f64_bits(float(coefficient))
            and isinstance(exceedances, int)
            and not isinstance(exceedances, bool)
            and 0 <= exceedances <= 10_000
            and permutations == 10_000
            and finite_number(probability)
            and f64_bits(float(probability)) == f64_bits((exceedances + 1) / 10_001)
        )

    workspace = layouts.get("workspace") if isinstance(layouts.get("workspace"), dict) else {}
    runs = workspace.get("runs") if isinstance(workspace.get("runs"), list) else []
    matching_runs = [run for run in runs if isinstance(run, dict) and run.get("id") == result.get("id")]
    run = matching_runs[0] if len(matching_runs) == 1 else {}
    snapshot = run.get("modelSnapshot") if isinstance(run.get("modelSnapshot"), dict) else {}
    snapshot_nodes = snapshot.get("nodes") if isinstance(snapshot.get("nodes"), list) else []
    snapshot_edges = snapshot.get("edges") if isinstance(snapshot.get("edges"), list) else []
    dataset_id = dataset.get("id")
    dataset_fingerprint = dataset.get("fingerprint")
    method_config = recipe.get("method_config")
    exact_provenance_settings = json_exact(settings, provenance_settings)
    settings_valid = (
        settings.get("method") == "pls_pm"
        and settings.get("weighting_scheme") == "path"
        and finite_number(settings.get("tolerance"))
        and isinstance(settings.get("max_iterations"), int)
        and not isinstance(settings.get("max_iterations"), bool)
        and settings.get("bootstrap_samples") == 0
        and settings.get("studentized_inner_samples") == 0
        and settings.get("permutation_samples") == 10_000
        and settings.get("seed") == 20_260_718
        and settings.get("workers") == 4
        and type(settings.get("confidence_level")) is float
        and f64_bits(settings["confidence_level"]) == f64_bits(0.95)
        and settings.get("preprocessing") == "standardized"
        and settings.get("missing_data") == "listwise_deletion"
        and settings.get("case_weight_column") is None
        and exact_provenance_settings
    )
    model_snapshot_valid = (
        len(snapshot_nodes) == 3
        and [node.get("id") for node in snapshot_nodes if isinstance(node, dict)]
        == [construct.get("id") for construct in constructs]
        and len(snapshot_edges) == 2
        and [[edge.get("source"), edge.get("target")] for edge in snapshot_edges if isinstance(edge, dict)]
        == recipe_path_ids
    )
    archive_valid = (
        isinstance(dataset_id, str)
        and f"data/{dataset_id}.arrow" in checksums
        and isinstance(dataset_fingerprint, str)
        and bool(dataset_fingerprint)
        and json_exact(model, recipe_model)
        and construct_names == ["X", "Z", "Y"]
        and len(construct_labels) == 3
        and recipe_label_pairs == expected_label_pairs
        and result_label_pairs == expected_label_pairs
        and result.get("schema_version") == ANALYSIS_RESULT_SCHEMA_VERSION
        and result.get("status") == "completed"
        and provenance.get("recipe_id") == recipe.get("id")
        and provenance.get("dataset_fingerprint") == recipe.get("dataset_fingerprint") == dataset_fingerprint
        and provenance.get("method") == "pls_pm"
        and provenance.get("method_version") == EXPECTED_PROVENANCE_METHOD_VERSION
        and provenance.get("seed") == 20_260_718
        and recipe.get("schema_version") == ANALYSIS_RECIPE_SCHEMA_VERSION
        and method_config == {"kind": "pls_permutation"}
        and recipe.get("metadata", {}).get("status") == "candidate_freedman_lane_path_randomization_scope"
        and settings_valid
        and payload.get("kind") == PLS_INFERENCE_PAYLOAD_KIND
        and estimation.get("method_version") == "pls_pm_v1"
        and payload.get("bootstrap") is None
        and set(permutation) == {"method_version", "parameters", "plan"}
        and permutation.get("method_version") == METHOD_VERSION
        and set(plan) == {"master_seed", "operation", "permutations"}
        and plan == {"master_seed": 20_260_718, "operation": PLAN_OPERATION, "permutations": 10_000}
        and parameter_contract
        and len(matching_runs) == 1
        and run.get("method") == "Structural Path Randomization"
        and run.get("status") == "completed"
        and run.get("modelId") == recipe_model.get("id")
        and isinstance(run.get("logs"), list)
        and len(run["logs"]) >= 1
        and model_snapshot_valid
    )
    project_checksum = computed_checksums["project.json"]
    contract = {
        "manifest": {
            "schemaVersion": manifest["schema_version"],
            "engineVersion": manifest["engine_version"],
            "checksumAlgorithm": manifest["checksum_algorithm"],
            "declaredProjectChecksum": checksums["project.json"],
            "projectChecksum": project_checksum,
            "projectChecksumMatches": checksums["project.json"] == project_checksum,
        },
        "project": {
            "modelCount": len(models),
            "recipeCount": len(recipes),
            "resultCount": len(results),
            "matchingRunCount": len(matching_runs),
        },
        "resultId": result.get("id"),
        "resultSchemaVersion": result.get("schema_version"),
        "resultStatus": result.get("status"),
        "payloadKind": payload.get("kind"),
        "estimationMethodVersion": estimation.get("method_version"),
        "bootstrapAbsent": payload.get("bootstrap") is None,
        "provenanceMethod": provenance.get("method"),
        "provenanceMethodVersion": provenance.get("method_version"),
        "recipeId": recipe.get("id"),
        "recipeSchemaVersion": recipe.get("schema_version"),
        "recipeMethodConfig": method_config,
        "recipeMethodConfigExact": method_config == {"kind": "pls_permutation"},
        "recipeStatus": recipe.get("metadata", {}).get("status"),
        "settings": {
            "method": settings.get("method"),
            "weightingScheme": settings.get("weighting_scheme"),
            "preprocessing": settings.get("preprocessing"),
            "missingData": settings.get("missing_data"),
            "bootstrapSamples": settings.get("bootstrap_samples"),
            "studentizedInnerSamples": settings.get("studentized_inner_samples"),
            "permutationSamples": settings.get("permutation_samples"),
            "seed": settings.get("seed"),
            "workers": settings.get("workers"),
            "confidenceLevel": settings.get("confidence_level"),
            "caseWeightColumn": settings.get("case_weight_column"),
            "exactProvenanceMatch": exact_provenance_settings,
        },
        "constructs": [
            {"id": row.get("id"), "name": row.get("name"), "indicators": row.get("indicators")}
            for row in constructs
        ],
        "recipePathIds": recipe_path_ids,
        "resultPathIds": result_path_ids,
        "pathLabels": path_labels,
        "pathOrderExact": recipe_label_pairs == expected_label_pairs and result_label_pairs == expected_label_pairs,
        "permutation": {
            "exactKeys": set(permutation) == {"method_version", "parameters", "plan"},
            "methodVersion": permutation.get("method_version"),
            "planExactKeys": set(plan) == {"master_seed", "operation", "permutations"},
            "permutations": plan.get("permutations"),
            "masterSeed": plan.get("master_seed"),
            "operation": plan.get("operation"),
            "parameterCount": len(parameters),
            "parameterIds": [row.get("parameter") for row in parameters],
            "exceedances": [row.get("exceedances") for row in parameters],
            "pValues": [row.get("p_value_two_sided") for row in parameters],
            "parameterContract": parameter_contract,
        },
        "run": {
            "id": run.get("id"),
            "method": run.get("method"),
            "status": run.get("status"),
            "modelId": run.get("modelId"),
            "snapshotNodes": len(snapshot_nodes),
            "snapshotEdges": len(snapshot_edges),
            "logs": len(run.get("logs", [])) if isinstance(run.get("logs"), list) else 0,
        },
    }
    outcome["contract"] = contract
    outcome["passed"] = archive_valid
    if not archive_valid:
        outcome["errors"].append("archive scientific identity contract failed")
    return outcome


def packaged_archive_contract(
    value: Any,
    result_rows: Any,
    artifact: Any = None,
    root: Path = ROOT,
) -> dict[str, Any]:
    check = value if isinstance(value, dict) else {}
    permutation = check.get("permutation") if isinstance(check.get("permutation"), dict) else {}
    constructs = check.get("constructs") if isinstance(check.get("constructs"), list) else []
    recipe_paths = check.get("recipePathIds") if isinstance(check.get("recipePathIds"), list) else []
    result_paths = check.get("resultPathIds") if isinstance(check.get("resultPathIds"), list) else []
    labels = check.get("pathLabels") if isinstance(check.get("pathLabels"), list) else []
    names_to_ids = {
        row.get("name"): row.get("id")
        for row in constructs
        if isinstance(row, dict)
        and isinstance(row.get("name"), str)
        and isinstance(row.get("id"), str)
    }
    expected_paths = [
        [names_to_ids.get("X"), names_to_ids.get("Y")],
        [names_to_ids.get("Z"), names_to_ids.get("Y")],
    ]
    ids = permutation.get("parameterIds") if isinstance(permutation.get("parameterIds"), list) else []
    expected_ids = [
        json.dumps(["path", path_ids], separators=(",", ":"))
        for path_ids in expected_paths
    ]
    exceedances = permutation.get("exceedances") if isinstance(permutation.get("exceedances"), list) else []
    probabilities = permutation.get("pValues") if isinstance(permutation.get("pValues"), list) else []
    parameter_arithmetic = (
        len(exceedances) == 2
        and len(probabilities) == 2
        and all(
            isinstance(exceedance, int)
            and not isinstance(exceedance, bool)
            and 0 <= exceedance <= 10_000
            and finite_number(probability)
            and f64_bits(float(probability))
            == f64_bits((exceedance + 1) / 10_001)
            for exceedance, probability in zip(exceedances, probabilities)
        )
    )
    row_binding = (
        isinstance(result_rows, list)
        and len(result_rows) == 2
        and [row.get("path") for row in result_rows] == list(PACKAGED_PATH_LABELS)
        and [row.get("exceedances") for row in result_rows] == exceedances
        and all(
            exact_plus_one_probability(row.get("p_value_text"), exceedance, 10_000)
            and f64_bits(float(row["p_value_text"])) == f64_bits(float(probability))
            for row, exceedance, probability in zip(result_rows, exceedances, probabilities)
        )
    )
    exact_order = (
        list(names_to_ids) == ["X", "Z", "Y"]
        and all(isinstance(value, str) and value for value in names_to_ids.values())
        and len(set(names_to_ids.values())) == 3
        and recipe_paths == expected_paths
        and result_paths == expected_paths
        and labels == list(PACKAGED_PATH_LABELS)
        and ids == expected_ids
    )
    archive_inspection = inspect_packaged_archive_artifact(root, artifact)
    artifact_path = artifact.get("path") if isinstance(artifact, dict) else None
    independent_contract = (
        archive_inspection.get("contract")
        if isinstance(archive_inspection.get("contract"), dict)
        else {}
    )
    archive_claims_match = (
        archive_inspection["passed"]
        and bool(independent_contract)
        and all(
            key in check and json_exact(check.get(key), expected)
            for key, expected in independent_contract.items()
        )
    )
    artifact_binding = (
        archive_inspection["passed"]
        and isinstance(artifact_path, str)
        and artifact_path.startswith(
            "validation/results/v247-native-structural-path-randomization-"
        )
        and artifact_path.endswith(".qpls")
        and archive_claims_match
    )
    return {
        "exact_parameter_order": exact_order,
        "parameter_arithmetic": parameter_arithmetic,
        "result_archive_binding": row_binding,
        "archive_inspection": archive_inspection,
        "archive_claims_match": archive_claims_match,
        "artifact_binding": artifact_binding,
        "passed": (
            check.get("passed") is True
            and check.get("pathOrderExact") is True
            and permutation.get("parameterCount") == 2
            and permutation.get("parameterContract") is True
            and exact_order
            and parameter_arithmetic
            and row_binding
            and artifact_binding
        ),
    }


def source_paths_for_crates(root: Path, crate_names: Iterable[str]) -> tuple[Path, ...]:
    paths: list[Path] = [root / "Cargo.toml", root / "Cargo.lock"]
    for crate_name in crate_names:
        crate = root / "crates" / crate_name
        paths.append(crate / "Cargo.toml")
        paths.extend(sorted(crate.rglob("*.rs"), key=lambda path: path.as_posix()))
    return tuple(dict.fromkeys(path.resolve() for path in paths))


def reference_product_sources(root: Path) -> tuple[Path, ...]:
    return source_paths_for_crates(
        root,
        (
            "qpls-core",
            "qpls-data",
            "qpls-assessment",
            "qpls-estimation",
            "qpls-resampling",
            "qpls-runner",
            "qpls-project",
            "qpls-cli",
        ),
    )


def frontend_source_paths(root: Path) -> tuple[Path, ...]:
    fixed = (
        root / "package.json",
        root / "package-lock.json",
        root / "index.html",
        root / "tsconfig.json",
        root / "tsconfig.app.json",
        root / "tsconfig.node.json",
        root / "vite.config.ts",
    )
    typescript = tuple(
        sorted(
            (path for suffix in ("*.ts", "*.tsx") for path in (root / "src").rglob(suffix)),
            key=lambda path: path.as_posix(),
        )
    )
    stylesheets = tuple(
        sorted((root / "src").rglob("*.css"), key=lambda path: path.as_posix())
    )
    return tuple(
        dict.fromkeys(path.resolve() for path in (*fixed, *typescript, *stylesheets))
    )


def packaged_product_sources(root: Path) -> tuple[Path, ...]:
    tauri = tuple(
        sorted(
            (
                path
                for pattern in ("*.rs", "*.json", "*.toml")
                for path in (root / "src-tauri").rglob(pattern)
                if path.is_file()
            ),
            key=lambda path: path.as_posix(),
        )
    )
    return tuple(
        dict.fromkeys(
            path.resolve()
            for path in (
                *reference_product_sources(root),
                *frontend_source_paths(root),
                *tauri,
            )
        )
    )


def file_descriptor(root: Path, path: Path) -> dict[str, Any]:
    return {
        "path": path.resolve().relative_to(root.resolve()).as_posix(),
        "size": path.stat().st_size,
        "sha256": sha256_file(path),
    }


def artifact_attestation(
    root: Path,
    descriptor: Any,
    *,
    expected_path: str | None = None,
) -> dict[str, Any]:
    evidence = {
        "path": descriptor.get("path") if isinstance(descriptor, dict) else None,
        "reported_size": descriptor.get("size") if isinstance(descriptor, dict) else None,
        "reported_sha256": descriptor.get("sha256") if isinstance(descriptor, dict) else None,
        "present": False,
        "inside_repository": False,
        "actual_size": None,
        "actual_sha256": None,
        "passed": False,
    }
    relative = evidence["path"]
    if (
        not isinstance(relative, str)
        or not relative
        or not isinstance(evidence["reported_size"], int)
        or isinstance(evidence["reported_size"], bool)
        or evidence["reported_size"] <= 0
        or not isinstance(evidence["reported_sha256"], str)
        or re.fullmatch(r"[0-9a-f]{64}", evidence["reported_sha256"]) is None
        or (expected_path is not None and relative != expected_path)
    ):
        return evidence
    path = (root / relative).resolve()
    try:
        path.relative_to(root.resolve())
    except ValueError:
        return evidence
    evidence["inside_repository"] = True
    if not path.is_file():
        return evidence
    evidence["present"] = True
    evidence["actual_size"] = path.stat().st_size
    evidence["actual_sha256"] = sha256_file(path)
    evidence["passed"] = (
        evidence["actual_size"] == evidence["reported_size"]
        and evidence["actual_sha256"] == evidence["reported_sha256"]
    )
    return evidence


def binary_attestation(
    root: Path,
    descriptor: Any,
    expected_path: str,
    source_paths: Iterable[Path],
) -> dict[str, Any]:
    artifact = artifact_attestation(root, descriptor, expected_path=expected_path)
    binary = root / expected_path
    freshness = [
        {
            "path": path.relative_to(root).as_posix() if path.is_absolute() else str(path),
            "present": path.is_file(),
            "binary_not_older": (
                binary.is_file()
                and path.is_file()
                and binary.stat().st_mtime_ns >= path.stat().st_mtime_ns
            ),
        }
        for path in source_paths
    ]
    return {
        "artifact": artifact,
        "source_freshness": freshness,
        "passed": artifact["passed"]
        and bool(freshness)
        and all(row["present"] and row["binary_not_older"] for row in freshness),
    }


def report_freshness(report: Path, sources: Iterable[Path]) -> dict[str, Any]:
    rows = [
        {
            "path": path.as_posix(),
            "present": path.is_file(),
            "report_not_older": (
                report.is_file()
                and path.is_file()
                and report.stat().st_mtime_ns >= path.stat().st_mtime_ns
            ),
        }
        for path in sources
    ]
    return {
        "sources": rows,
        "passed": bool(rows)
        and all(row["present"] and row["report_not_older"] for row in rows),
    }


def exact_check_rows(value: Any, expected: frozenset[str]) -> dict[str, Any]:
    if isinstance(value, list):
        names = [row.get("name") for row in value if isinstance(row, dict)]
        passed = all(
            isinstance(row, dict) and row.get("passed") is True for row in value
        )
    elif isinstance(value, dict):
        names = list(value)
        passed = all(
            item is True or (isinstance(item, dict) and item.get("passed") is True)
            for item in value.values()
        )
    else:
        names = []
        passed = False
    observed = frozenset(name for name in names if isinstance(name, str))
    return {
        "expected": sorted(expected),
        "observed": sorted(observed),
        "passed": observed == expected and len(names) == len(expected) and passed,
    }


def packaged_source_check_rows(value: Any) -> dict[str, Any]:
    checks = value if isinstance(value, dict) else {}
    required = exact_check_rows(
        {name: checks.get(name) for name in PACKAGED_SOURCE_CHECKS},
        PACKAGED_SOURCE_CHECKS,
    )
    runtime = checks.get("runtime") if isinstance(checks.get("runtime"), dict) else {}
    viewport = runtime.get("viewport") if isinstance(runtime.get("viewport"), dict) else {}
    runtime_contract = (
        set(runtime) == {"title", "tauriRuntime", "viewport", "surface"}
        and runtime.get("title") == "QuickPLS"
        and runtime.get("tauriRuntime") is True
        and runtime.get("surface") == "launcher"
        and set(viewport) == {"width", "height", "dpr"}
        and isinstance(viewport.get("width"), int)
        and not isinstance(viewport.get("width"), bool)
        and viewport["width"] >= 1024
        and isinstance(viewport.get("height"), int)
        and not isinstance(viewport.get("height"), bool)
        and viewport["height"] >= 700
        and isinstance(viewport.get("dpr"), (int, float))
        and not isinstance(viewport.get("dpr"), bool)
        and math.isfinite(float(viewport["dpr"]))
        and viewport["dpr"] > 0
    )
    expected = PACKAGED_SOURCE_CHECKS | PACKAGED_SOURCE_METADATA_CHECKS
    observed = frozenset(name for name in checks if isinstance(name, str))
    recent_projects_restored = checks.get("recentProjectsRestored") is True
    return {
        "expected": sorted(expected),
        "observed": sorted(observed),
        "required_checks": required,
        "runtime_contract": runtime_contract,
        "recent_projects_restored": recent_projects_restored,
        "passed": observed == expected
        and len(checks) == len(expected)
        and required["passed"]
        and runtime_contract
        and recent_projects_restored,
    }


def frontend_test_rows_passed(value: Any) -> bool:
    if not isinstance(value, list) or len(value) != len(FRONTEND_TEST_FILES):
        return False
    if not all(isinstance(row, dict) for row in value):
        return False
    observed = [row.get("path") for row in value]
    return (
        len(observed) == len(set(observed))
        and frozenset(observed) == frozenset(FRONTEND_TEST_FILES)
        and all(
            row.get("status") == "passed"
            and isinstance(row.get("assertions"), int)
            and not isinstance(row.get("assertions"), bool)
            and row["assertions"] > 0
            and row.get("passed_assertions") == row["assertions"]
            and row.get("failed_assertions") == 0
            for row in value
        )
    )


def reference_attestation(root: Path) -> dict[str, Any]:
    path = root / REFERENCE_REPORT
    report = load_optional(path)
    checks = exact_check_rows(report.get("checks"), REFERENCE_CHECKS)
    tested_product = report.get("tested_product")
    tested_product = tested_product if isinstance(tested_product, dict) else {}
    cli = binary_attestation(
        root,
        tested_product.get("release_cli"),
        "target/release/qpls.exe",
        reference_product_sources(root),
    )
    contract = report.get("contract") if isinstance(report.get("contract"), dict) else {}
    python = report.get("python_reference") if isinstance(report.get("python_reference"), dict) else {}
    r_reference = report.get("r_reference") if isinstance(report.get("r_reference"), dict) else {}
    calibration = report.get("calibration") if isinstance(report.get("calibration"), dict) else {}
    fixtures = report.get("fixtures") if isinstance(report.get("fixtures"), dict) else {}
    source_descriptors = (
        report.get("evidence_sources")
        if isinstance(report.get("evidence_sources"), list)
        else []
    )
    python_rows = python.get("parameters") if isinstance(python.get("parameters"), list) else []
    r_rows = r_reference.get("comparisons") if isinstance(r_reference.get("comparisons"), list) else []
    null = calibration.get("null") if isinstance(calibration.get("null"), dict) else {}
    signal = calibration.get("signal") if isinstance(calibration.get("signal"), dict) else {}
    design = calibration.get("design") if isinstance(calibration.get("design"), dict) else {}
    thresholds = calibration.get("thresholds") if isinstance(calibration.get("thresholds"), dict) else {}
    expected_null_maximum = 0.05 + 3.5 * math.sqrt(0.05 * 0.95 / 240.0)

    python_exact = (
        len(python_rows) == 3
        and all(isinstance(row, dict) for row in python_rows)
        and [row.get("parameter") for row in python_rows] == list(EXPECTED_PARAMETERS)
        and all(
            row.get("ordinal") == ordinal
            and row.get("stored_exceedances") == row.get("reference_exceedances")
            and row.get("stored_p_value") == row.get("reference_p_value")
            and finite_number(row.get("minimum_absolute_threshold_margin"))
            and row["minimum_absolute_threshold_margin"] > 0
            and isinstance(row.get("coefficient_sha256_le_f64"), str)
            and re.fullmatch(r"[0-9a-f]{64}", row["coefficient_sha256_le_f64"])
            is not None
            for ordinal, row in enumerate(python_rows)
        )
    )
    r_exact = (
        len(r_rows) == 3
        and all(isinstance(row, dict) for row in r_rows)
        and [row.get("parameter") for row in r_rows] == list(EXPECTED_PARAMETERS)
        and all(
            row.get("coefficient_count") == 999
            and row.get("exceedances") == python_rows[index].get("stored_exceedances")
            and row.get("p_value_two_sided") == python_rows[index].get("stored_p_value")
            and finite_number(row.get("maximum_relative_difference_from_python"))
            and 0 <= row["maximum_relative_difference_from_python"] <= 8e-10
            for index, row in enumerate(r_rows)
        )
    )
    null_interval = null.get("wilson_95")
    power_interval = signal.get("wilson_95")
    calibration_exact = (
        calibration.get("passed") is True
        and design
        == {
            "replications": 240,
            "permutations": 199,
            "cases": 80,
            "alpha": 0.05,
            "focal_nuisance_correlation": 0.35,
            "nuisance_coefficient": 0.8,
            "signal_coefficient": 0.65,
            "noise_standard_deviation": 1.0,
            "outcome_noise_distribution": "homoscedastic_standard_gaussian",
            "error_variance_model": "constant",
            "paired_null_signal_scenarios": True,
            "data_rng": "numpy_pcg64",
            "data_seed": 20_260_813,
            "permutation_seed": 20_260_814,
        }
        and thresholds.get("null_boundary_formula")
        == "alpha + 3.5 * sqrt(alpha * (1-alpha) / replications)"
        and thresholds.get("null_wilson_95_must_contain_alpha") is True
        and thresholds.get("power_wilson_95_lower_minimum") == 0.8
        and thresholds.get("null_rejection_rate_maximum") == expected_null_maximum
        and finite_number(null.get("rate"))
        and null["rate"] <= expected_null_maximum < 0.10
        and isinstance(null_interval, list)
        and len(null_interval) == 2
        and null_interval[0] <= 0.05 <= null_interval[1]
        and isinstance(power_interval, list)
        and len(power_interval) == 2
        and power_interval[0] >= 0.8
    )
    r_artifacts: list[dict[str, Any]] = []
    inputs = r_reference.get("inputs")
    if isinstance(inputs, dict):
        r_artifacts.extend(artifact_attestation(root, item) for item in inputs.values())
    r_artifacts.append(artifact_attestation(root, r_reference.get("output")))
    fixture_artifacts = [
        artifact_attestation(
            root,
            fixtures.get("data"),
            expected_path="validation/fixtures/corporate_reputation.csv",
        ),
        artifact_attestation(
            root,
            fixtures.get("source_schema_v2_recipe"),
            expected_path="validation/fixtures/corporate_reputation.recipe.json",
        ),
    ]
    generated_recipes = fixtures.get("generated_schema_v3_recipes")
    if isinstance(generated_recipes, list) and len(generated_recipes) == 2:
        for worker, descriptor in zip((1, 4), generated_recipes):
            fixture_artifacts.append(
                artifact_attestation(
                    root,
                    descriptor,
                    expected_path=(
                        "validation/results/structural_path_randomization_reference/"
                        f"workers_{worker}.recipe.json"
                    ),
                )
            )
    source_recipe = load_optional(
        root / "validation" / "fixtures" / "corporate_reputation.recipe.json"
    )
    expected_fingerprint = source_recipe.get("dataset_fingerprint")
    worker_rows = tested_product.get("workers")
    worker_rows = worker_rows if isinstance(worker_rows, list) else []
    sources = (
        *reference_product_sources(root),
        root / "validation" / "structural_path_randomization_reference.py",
        root / "validation" / "structural_path_randomization_reference.R",
        root / "validation" / "fixtures" / "corporate_reputation.csv",
        root / "validation" / "fixtures" / "corporate_reputation.recipe.json",
    )
    freshness = report_freshness(path, sources)
    source_descriptor_map = {
        row.get("path"): row for row in source_descriptors if isinstance(row, dict)
    }
    source_manifest = {
        "expected": list(REFERENCE_EVIDENCE_SOURCE_PATHS),
        "observed": list(source_descriptor_map),
        "artifacts": [
            artifact_attestation(
                root,
                source_descriptor_map.get(relative),
                expected_path=relative,
            )
            for relative in REFERENCE_EVIDENCE_SOURCE_PATHS
        ],
    }
    source_manifest["passed"] = (
        len(source_descriptors) == len(REFERENCE_EVIDENCE_SOURCE_PATHS)
        and list(source_descriptor_map) == list(REFERENCE_EVIDENCE_SOURCE_PATHS)
        and all(row["passed"] for row in source_manifest["artifacts"])
    )
    report_artifact = file_descriptor(root, path) if path.is_file() else None
    identity = {
        "passed": report.get("passed") is True,
        "target": report.get("target") == "structural_path_randomization_reference_v1",
        "feature_id": report.get("feature_id") == FEATURE_ID,
        "method_version": report.get("method_version") == METHOD_VERSION,
        "catalogue_snapshot_date": report.get("catalogue_snapshot_date")
        == CATALOGUE_SNAPSHOT_DATE,
        "plan": contract.get("plan_operation") == PLAN_OPERATION
        and contract.get("seed") == 20_260_718
        and contract.get("permutations") == 999,
        "parameters": contract.get("ordered_parameters") == list(EXPECTED_PARAMETERS),
        "frozen_indices": contract.get("frozen_indices") == FROZEN_INDICES,
        "provenance": reference_worker_rows_passed(worker_rows, expected_fingerprint),
    }
    return {
        "path": REFERENCE_REPORT,
        "identity": identity,
        "exact_checks": checks,
        "tested_cli": cli,
        "python_exact": python_exact,
        "r_exact": r_exact,
        "r_artifacts": r_artifacts,
        "fixture_artifacts": fixture_artifacts,
        "source_manifest": source_manifest,
        "report_artifact": report_artifact,
        "calibration_exact": calibration_exact,
        "freshness": freshness,
        "passed": (
            all(identity.values())
            and checks["passed"]
            and cli["passed"]
            and python_exact
            and r_exact
            and len(r_artifacts) == 4
            and all(row["passed"] for row in r_artifacts)
            and len(fixture_artifacts) == 4
            and all(row["passed"] for row in fixture_artifacts)
            and source_manifest["passed"]
            and report_artifact is not None
            and calibration_exact
            and freshness["passed"]
        ),
    }


def boundary_attestation(root: Path) -> dict[str, Any]:
    path = root / BOUNDARY_REPORT
    report = load_optional(path)
    checks = report.get("checks") if isinstance(report.get("checks"), dict) else {}
    executions = report.get("executions") if isinstance(report.get("executions"), dict) else {}
    descriptors = (
        report.get("test_executables")
        if isinstance(report.get("test_executables"), dict)
        else {}
    )
    exact_sets = (
        frozenset(checks) == frozenset(BOUNDARY_SUITES)
        and all(
            isinstance(checks.get(target), dict)
            and frozenset(checks[target]) == expected
            and all(checks[target].get(name) is True for name in expected)
            for target, expected in BOUNDARY_SUITES.items()
        )
    )
    expected_all = frozenset().union(*BOUNDARY_SUITES.values())
    exact_executions = (
        frozenset(executions) == expected_all
        and all(
            isinstance(row, dict)
            and row.get("passed") is True
            and row.get("exit_code") == 0
            and row.get("full_name") in {name, f"tests::{name}"}
            and row.get("target")
            == next(target for target, names in BOUNDARY_SUITES.items() if name in names)
            for name, row in executions.items()
        )
    )
    executable_rows: dict[str, Any] = {}
    binary_freshness = report.get("test_executables_not_older_than_sources")
    for target, expected in BOUNDARY_SUITES.items():
        descriptor = descriptors.get(target)
        artifact = artifact_attestation(root, descriptor)
        expected_prefix = f"target/release/deps/{target}-"
        path_value = descriptor.get("path") if isinstance(descriptor, dict) else ""
        sources = source_paths_for_crates(root, BOUNDARY_SOURCE_CRATES[target])
        binary_path = root / path_value if isinstance(path_value, str) else root / "missing"
        executable_rows[target] = {
            "artifact": artifact,
            "exact_path": isinstance(path_value, str)
            and path_value.startswith(expected_prefix)
            and path_value.endswith(".exe"),
            "source_freshness": bool(sources)
            and binary_path.is_file()
            and all(
                source.is_file()
                and binary_path.stat().st_mtime_ns >= source.stat().st_mtime_ns
                for source in sources
            ),
            "reported_freshness": isinstance(binary_freshness, dict)
            and binary_freshness.get(target) is True,
        }
        executable_rows[target]["passed"] = all(
            (
                executable_rows[target]["artifact"]["passed"],
                executable_rows[target]["exact_path"],
                executable_rows[target]["source_freshness"],
                executable_rows[target]["reported_freshness"],
            )
        )
    freshness = report_freshness(
        path,
        (
            root / "validation" / "structural_path_randomization_boundary_gate.py",
            *(source for names in BOUNDARY_SOURCE_CRATES.values() for source in source_paths_for_crates(root, names)),
        ),
    )
    identity = (
        report.get("passed") is True
        and report.get("target")
        == "structural_path_randomization_v1_focused_rust_boundary_tests"
        and report.get("feature_id") == FEATURE_ID
        and report.get("method_version") == METHOD_VERSION
        and report.get("environment") == {"CARGO_BUILD_JOBS": "1", "test_threads": 1}
        and report.get("source_stable_during_gate") is True
    )
    return {
        "path": BOUNDARY_REPORT,
        "identity": identity,
        "exact_sets": exact_sets,
        "exact_executions": exact_executions,
        "executables": executable_rows,
        "freshness": freshness,
        "passed": identity
        and exact_sets
        and exact_executions
        and len(expected_all) == 6
        and all(row["passed"] for row in executable_rows.values())
        and freshness["passed"],
    }


def frontend_attestation(root: Path) -> dict[str, Any]:
    path = root / FRONTEND_REPORT
    report = load_optional(path)
    vitest = report.get("vitest") if isinstance(report.get("vitest"), dict) else {}
    tsc = report.get("tsc") if isinstance(report.get("tsc"), dict) else {}
    rows = vitest.get("test_files") if isinstance(vitest.get("test_files"), list) else []
    exact_rows = frontend_test_rows_passed(rows)
    source_descriptors = (
        report.get("source_artifacts")
        if isinstance(report.get("source_artifacts"), list)
        else []
    )
    current_sources = (
        *frontend_source_paths(root),
        root / "validation" / "structural_path_randomization_frontend_gate.py",
    )
    descriptor_map = {
        row.get("path"): row for row in source_descriptors if isinstance(row, dict)
    }
    source_binding = (
        set(descriptor_map)
        == {source.relative_to(root).as_posix() for source in current_sources}
        and all(
            artifact_attestation(root, descriptor_map[source.relative_to(root).as_posix()])[
                "passed"
            ]
            for source in current_sources
        )
    )
    freshness = report_freshness(path, current_sources)
    identity = (
        report.get("passed") is True
        and report.get("target") == "structural_path_randomization_v1_focused_frontend_gate"
        and report.get("feature_id") == FEATURE_ID
        and report.get("method_version") == METHOD_VERSION
        and report.get("test_files") == list(FRONTEND_TEST_FILES)
        and report.get("source_stable_during_gate") is True
    )
    commands = report.get("commands") if isinstance(report.get("commands"), dict) else {}
    exact_commands = (
        commands.get("vitest")
        == ["npx", "vitest", "run", *FRONTEND_TEST_FILES, "--reporter=json"]
        and commands.get("tsc") == ["npx", "tsc", "-b", "--pretty", "false"]
    )
    return {
        "path": FRONTEND_REPORT,
        "identity": identity,
        "exact_commands": exact_commands,
        "exact_test_rows": exact_rows,
        "source_binding": source_binding,
        "freshness": freshness,
        "passed": identity
        and exact_commands
        and vitest.get("passed") is True
        and tsc.get("passed") is True
        and exact_rows
        and source_binding
        and freshness["passed"],
    }


def visual_row_passed(row: Any, viewport: str) -> bool:
    if not isinstance(row, dict):
        return False
    linkage = row.get("linkage") if isinstance(row.get("linkage"), dict) else {}
    truth = row.get("truthAndOverflow") if isinstance(row.get("truthAndOverflow"), dict) else {}
    overflow = truth.get("horizontalOverflow") if isinstance(truth.get("horizontalOverflow"), dict) else {}
    close = row.get("closeFocus") if isinstance(row.get("closeFocus"), dict) else {}
    description = row.get("methodDescription")
    return (
        row.get("viewport") == viewport
        and row.get("dialogOpened") is True
        and row.get("pointerSelected") is True
        and linkage.get("expectedKind") == "pls_permutation"
        and linkage.get("expectedLabel") == "Structural Path Randomization"
        and linkage.get("selectedCount") == 1
        and linkage.get("panelCount") == 1
        and linkage.get("headingCount") == 1
        and linkage.get("linkage") is True
        and row.get("permutationsInputCount") == 1
        and row.get("permutationsInputType") == "number"
        and row.get("permutationsInputValue") == "999"
        and row.get("expectedDefaultPermutations") == "999"
        and row.get("bootstrapSamplesInputCount") == 0
        and row.get("mutuallyExclusive") is True
        and isinstance(description, str)
        and re.search(
            r"single-model Freedman(?:\u2013|-|\s)Lane randomization",
            description,
            re.IGNORECASE,
        )
        is not None
        and re.search(r"structural paths", description, re.IGNORECASE) is not None
        and re.search(
            r"fixed original PLS construct scores", description, re.IGNORECASE
        )
        is not None
        and re.search(
            r"unadjusted pathwise p values", description, re.IGNORECASE
        )
        is not None
        and row.get("distinctFromMgaAndMicom") is True
        and truth.get("noFabricatedRunState") is True
        and overflow.get("dialogOutsideViewport") is False
        and overflow.get("dialogContentOverflow") is False
        and overflow.get("pageOverflow") is False
        and truth.get("noHorizontalOverflow") is True
        and close.get("dialogClosed") is True
        and close.get("focusRestored") is True
    )


def visual_screenshot_attestation(root: Path, value: Any) -> dict[str, Any]:
    source_rows = value if isinstance(value, list) else []
    selected = [
        row
        for row in source_rows
        if isinstance(row, dict) and row.get("state") == VISUAL_RANDOMIZATION_STATE
    ]
    rows: list[dict[str, Any]] = []
    for index, expected in enumerate(VISUAL_RANDOMIZATION_SCREENSHOTS):
        descriptor = selected[index] if index < len(selected) else None
        artifact = artifact_attestation(
            root,
            descriptor,
            expected_path=expected["path"],
        )
        exact_descriptor = (
            isinstance(descriptor, dict)
            and set(descriptor) == {"path", "size", "sha256", "viewport", "state"}
            and descriptor.get("viewport") == expected["viewport"]
            and descriptor.get("state") == expected["state"]
            and isinstance(descriptor.get("size"), int)
            and not isinstance(descriptor.get("size"), bool)
            and descriptor["size"] > 0
            and isinstance(descriptor.get("sha256"), str)
            and re.fullmatch(r"[0-9a-f]{64}", descriptor["sha256"]) is not None
        )
        rows.append(
            {
                "expected": expected,
                "descriptor": descriptor,
                "exact_descriptor": exact_descriptor,
                "artifact": artifact,
                "passed": exact_descriptor and artifact["passed"],
            }
        )
    selected_paths = [row.get("path") for row in selected if isinstance(row, dict)]
    exact_set = (
        len(selected) == len(VISUAL_RANDOMIZATION_SCREENSHOTS)
        and len(selected_paths) == len(set(selected_paths))
    )
    return {
        "expected_count": len(VISUAL_RANDOMIZATION_SCREENSHOTS),
        "observed_count": len(selected),
        "rows": rows,
        "exact_set": exact_set,
        "passed": exact_set and all(row["passed"] for row in rows),
    }


def visual_attestation(root: Path) -> dict[str, Any]:
    path = root / VISUAL_REPORT
    report = load_optional(path)
    checks = report.get("checks") if isinstance(report.get("checks"), dict) else {}
    rows = (
        checks.get("structuralPathRandomization")
        if isinstance(checks.get("structuralPathRandomization"), list)
        else []
    )
    observed_viewports = [row.get("viewport") for row in rows if isinstance(row, dict)]
    exact_rows = (
        len(rows) == 3
        and observed_viewports == list(VISUAL_VIEWPORTS)
        and all(visual_row_passed(row, viewport) for row, viewport in zip(rows, VISUAL_VIEWPORTS))
    )
    declared_viewports = report.get("viewports") if isinstance(report.get("viewports"), list) else []
    exact_viewports = declared_viewports == [
        {"id": name, "width": dimensions[0], "height": dimensions[1]}
        for name, dimensions in VISUAL_VIEWPORTS.items()
    ]
    screenshots = visual_screenshot_attestation(root, report.get("screenshots"))
    freshness = report_freshness(
        path,
        (
            *frontend_source_paths(root),
            root / "validation" / "v247_native_desktop_visual_acceptance.mjs",
            root / "validation" / "test_v247_native_desktop_visual_screenshot_integrity.py",
        ),
    )
    return {
        "path": VISUAL_REPORT,
        "exact_viewports": exact_viewports,
        "exact_method_rows": exact_rows,
        "screenshots": screenshots,
        "freshness": freshness,
        "passed": report.get("passed") is True
        and exact_viewports
        and exact_rows
        and screenshots["passed"]
        and report.get("failures") == []
        and report.get("consoleErrors") == []
        and freshness["passed"],
    }


def directory_manifest(root: Path, relative: str) -> dict[str, Any]:
    directory = root / relative
    files = (
        sorted(
            (path for path in directory.rglob("*") if path.is_file()),
            key=lambda path: path.relative_to(directory).as_posix(),
        )
        if directory.is_dir()
        else []
    )
    manifest = []
    total_size = 0
    digest = hashlib.sha256()
    for path in files:
        child = path.relative_to(directory).as_posix()
        size = path.stat().st_size
        contents_digest = sha256_file(path)
        manifest.append({"path": child, "size": size, "sha256": contents_digest})
        total_size += size
        digest.update(f"{child}\0{size}\0{contents_digest}\n".encode("utf-8"))
    return {
        "path": relative,
        "size": total_size,
        "file_count": len(manifest),
        "sha256": digest.hexdigest() if manifest else None,
        "manifest": manifest,
    }


def packaged_attestation(root: Path) -> dict[str, Any]:
    path = root / PACKAGED_REPORT
    schema_path = root / PACKAGED_SCHEMA
    source_path = root / PACKAGED_SOURCE_REPORT
    report = load_optional(path)
    schema = load_optional(schema_path)
    source = load_optional(source_path)
    schema_errors: list[str] = []
    try:
        Draft202012Validator.check_schema(schema)
        schema_errors = [
            error.message
            for error in Draft202012Validator(
                schema, format_checker=FormatChecker()
            ).iter_errors(report)
        ]
    except Exception as error:  # schema meta-validation must fail closed
        schema_errors = [f"{type(error).__name__}: {error}"]

    source_checks = packaged_source_check_rows(source.get("checks"))
    packaged_checks = report.get("checks") if isinstance(report.get("checks"), dict) else {}
    source_report_checks = source.get("checks") if isinstance(source.get("checks"), dict) else {}
    packaged_checks_pass = bool(packaged_checks) and all(
        isinstance(row, dict) and row.get("passed") is True
        for row in packaged_checks.values()
    )
    source_identity = (
        source.get("passed") is True
        and source.get("feature_id") == FEATURE_ID
        and source.get("method_version") == METHOD_VERSION
        and source.get("catalogue_snapshot_date") == CATALOGUE_SNAPSHOT_DATE
        and source.get("acceptance_scope") == ACCEPTANCE_SCOPE
        and source.get("failures") == []
        and source.get("consoleErrors") == []
    )
    identity = (
        report.get("schema_version") == "quickpls.packaged_acceptance.v1"
        and report.get("kind") == PACKAGED_EVIDENCE_KIND
        and report.get("passed") is True
        and report.get("feature_id") == FEATURE_ID
        and report.get("method_version") == METHOD_VERSION
        and report.get("catalogue_snapshot_date") == CATALOGUE_SNAPSHOT_DATE
        and report.get("target") == "windows_10_11_x64_packaged_tauri"
        and report.get("runtime") == "tauri-webview2-cdp"
        and report.get("source_report") == PACKAGED_SOURCE_REPORT
        and report.get("failures") == []
        and report.get("console_errors") == []
    )
    tested_product = (
        report.get("tested_product") if isinstance(report.get("tested_product"), dict) else {}
    )
    cli = binary_attestation(
        root,
        tested_product.get("qpls_cli_exe"),
        "target/release/qpls.exe",
        reference_product_sources(root),
    )
    desktop = binary_attestation(
        root,
        tested_product.get("quickpls_desktop_exe"),
        "target/release/quickpls-desktop.exe",
        packaged_product_sources(root),
    )
    dist_reported = tested_product.get("dist_bundle")
    dist_actual = directory_manifest(root, "dist")
    dist_passed = isinstance(dist_reported, dict) and dist_reported == dist_actual
    top_level_artifacts = (
        report.get("artifacts") if isinstance(report.get("artifacts"), dict) else {}
    )
    result_contract = packaged_results_contract(
        packaged_checks.get("structuralPathRandomizationResults")
    )
    export_contract = packaged_export_contract(
        packaged_checks.get("structuralPathRandomizationExport"),
        top_level_artifacts.get("xlsx"),
        root,
    )
    archive_contract = packaged_archive_contract(
        packaged_checks.get("structuralPathRandomizationArchive"),
        result_contract["rows"],
        top_level_artifacts.get("project_archive"),
        root,
    )
    cancellation_contract = packaged_cancellation_contract(
        packaged_checks.get("structuralPathRandomizationCancellation"),
        source_report_checks.get("structuralPathRandomizationCancellation"),
        top_level_artifacts,
        root,
    )

    described_artifacts: list[dict[str, Any]] = []

    def collect_descriptors(value: Any) -> None:
        if isinstance(value, dict):
            if {"path", "size", "sha256"}.issubset(value):
                described_artifacts.append(value)
            else:
                for child in value.values():
                    collect_descriptors(child)
        elif isinstance(value, list):
            for child in value:
                collect_descriptors(child)

    collect_descriptors(report.get("artifacts"))
    artifact_rows = [artifact_attestation(root, row) for row in described_artifacts]
    artifact_paths = [row["path"] for row in artifact_rows]
    artifacts_pass = (
        bool(artifact_rows)
        and len(artifact_paths) == len(set(artifact_paths))
        and all(row["passed"] for row in artifact_rows)
    )
    freshness = report_freshness(
        path,
        (
            source_path,
            schema_path,
            *(root / relative for relative in PACKAGED_VALIDATION_SOURCE_PATHS),
            *packaged_product_sources(root),
        ),
    )
    source_freshness = report_freshness(
        source_path,
        (
            *(root / relative for relative in PACKAGED_VALIDATION_SOURCE_PATHS),
            *packaged_product_sources(root),
        ),
    )
    return {
        "path": PACKAGED_REPORT,
        "schema": PACKAGED_SCHEMA,
        "source_report": PACKAGED_SOURCE_REPORT,
        "schema_errors": schema_errors,
        "identity": identity,
        "source_identity": source_identity,
        "source_exact_checks": source_checks,
        "packaged_checks_pass": packaged_checks_pass,
        "tested_cli": cli,
        "tested_desktop": desktop,
        "dist_reported": dist_reported,
        "dist_actual": dist_actual,
        "dist_passed": dist_passed,
        "result_contract": result_contract,
        "export_contract": export_contract,
        "archive_contract": archive_contract,
        "cancellation_contract": cancellation_contract,
        "artifacts": artifact_rows,
        "freshness": freshness,
        "source_freshness": source_freshness,
        "passed": not schema_errors
        and identity
        and source_identity
        and source_checks["passed"]
        and packaged_checks_pass
        and cli["passed"]
        and desktop["passed"]
        and dist_passed
        and result_contract["passed"]
        and export_contract["passed"]
        and archive_contract["passed"]
        and cancellation_contract["passed"]
        and artifacts_pass
        and freshness["passed"]
        and source_freshness["passed"],
    }


def documentation_attestation(root: Path) -> dict[str, Any]:
    path = root / "docs" / "methods" / "PERMUTATION_ENGINE_V1.md"
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeError):
        text = ""
    phrases = (
        METHOD_VERSION,
        "fixed construct scores",
        "remaining direct predecessors",
        "Fisher-Yates",
        "ChaCha20",
        "(exceedances + 1) / (P + 1)",
        "Cancellation discards partial output",
        "Projects store only",
        "not a full measurement-model re-estimation test",
    )
    checks = {phrase: phrase.casefold() in text.casefold() for phrase in phrases}
    return {
        "path": "docs/methods/PERMUTATION_ENGINE_V1.md",
        "sha256": sha256_file(path) if path.is_file() else None,
        "checks": checks,
        "passed": bool(text.strip()) and all(checks.values()),
    }


def audit(root: Path = ROOT) -> dict[str, Any]:
    reference = reference_attestation(root)
    boundary = boundary_attestation(root)
    frontend = frontend_attestation(root)
    visual = visual_attestation(root)
    packaged = packaged_attestation(root)
    documentation = documentation_attestation(root)
    evidence = {
        "reference": reference,
        "boundary": boundary,
        "frontend": frontend,
        "visual": visual,
        "packaged": packaged,
        "documentation": documentation,
    }
    passed = all(item["passed"] for item in evidence.values())
    return {
        "schema_version": 1,
        "target": TARGET,
        "feature_id": FEATURE_ID,
        "method_version": METHOD_VERSION,
        "catalogue_snapshot_date": CATALOGUE_SNAPSHOT_DATE,
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "promotion_state": "release_qualified" if passed else "candidate",
        "promoted_scope": (
            "Single-model two-sided Freedman-Lane structural-path randomization using "
            "the original converged PLS construct scores, intercept nuisance equations, "
            "path-specific deterministic streams, unadjusted plus-one probabilities, and "
            "exchangeable reduced-model residuals."
        ),
        "excluded_claims": [
            "No measurement-model re-estimation inside permutations.",
            "No multiplicity-adjusted path-family inference.",
            "No multigroup, MICOM, MGA, causal-proof, or numerical-identity claim.",
            "No support outside 99 through 10,000 permutations or the documented complete-case linear nuisance scope.",
            "The current null/power calibration covers paired homoscedastic Gaussian-error scenarios with constant variance only; it does not qualify heteroskedastic or broader non-Gaussian validity.",
        ],
        "evidence": evidence,
        "bootstrap_evidence_borrowed": False,
        "passed": passed,
    }


def main() -> int:
    report = audit(ROOT)
    RESULTS.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {OUTPUT} | passed={report['passed']}")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
