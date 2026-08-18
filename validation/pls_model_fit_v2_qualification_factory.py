#!/usr/bin/env python3
"""Fail-closed, source-bound qualification factory for PLS model fit v2.

The factory does not run Rust, Node, GUI, archive, packaged, or performance
workloads.  It verifies the frozen QualificationSpec, current registry and v1
manifest state, independent validation work, and optional immutable execution
envelopes produced elsewhere.  It emits a candidate receipt descriptor only
when the corresponding role is genuinely satisfied.  It never attaches a
receipt to the QualificationSpec or changes a capability/evidence state.
"""

from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
import os
import platform
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any, Iterable, Sequence

import jsonschema


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
SPEC_PATH = VALIDATION / "qualification_v2" / "pls_model_fit_exact_v1.qualification.json"
MANIFEST_PATH = VALIDATION / "methods" / "pls_model_fit_v2.manifest.json"
REGISTRY_PATH = VALIDATION / "capabilities" / "capability_registry_v2.json"
REPORT_ROOT = VALIDATION / "results" / "method_factory" / "pls_model_fit_v2"
WORK_ROOT = REPORT_ROOT / "work"
INPUT_ROOT = REPORT_ROOT / "qualification_inputs"
RECEIPT_ROOT = REPORT_ROOT / "receipts"
AUDIT_PATH = REPORT_ROOT / "qualification_factory_audit.json"
METHOD_CONTRACT_PATH = RECEIPT_ROOT / "method_contract.qualification.json"

sys.path.insert(0, str(VALIDATION))

from qualification_spec_v2 import (  # noqa: E402
    canonical_sha256,
    strict_load_json,
    validate_spec_path,
)


QUALIFICATION_ID = "qpls3.assessment.model_fit.exact.qualification_v2"
CAPABILITY_ID = "smartpls.model_fit"
CELL_ID = "qpls3.assessment.model_fit"
METHOD_VERSION = "pls_model_fit_v2"
EXPECTED_REQUIRED_ROLES = (
    "method_contract",
    "kernel_execution",
    "oracle_independence",
    "generative_recovery",
    "adversarial_boundaries",
    "archive_persistence",
    "cross_format_export",
    "frontend_contract",
    "packaged_windows_e2e",
    "performance_scale",
)
ROLE_STAGES = {
    "method_contract": "contract",
    "kernel_execution": "kernel",
    "oracle_independence": "oracle",
    "generative_recovery": "generative",
    "adversarial_boundaries": "adversarial",
    "archive_persistence": "persistence_export",
    "cross_format_export": "persistence_export",
    "frontend_contract": "persistence_export",
    "packaged_windows_e2e": "packaged_windows",
    "performance_scale": "scale_reliability",
}
REQUIRED_CHECK_IDS = {
    "kernel_execution": (
        "product.point_fit_saturated_and_estimated",
        "product.exact_fit_separate_full_refits",
        "product.type7_hi95_hi99",
        "product.fixed_requested_usable_failure_ledger",
        "product.seed_and_worker_identity",
        "product.pls_pm_and_plsc_supported_shapes",
        "product.typed_matrix_and_nonconvergence_failures",
    ),
    "oracle_independence": (
        "oracle.independent_full_pipeline",
        "oracle.second_reference_or_approved_exception",
        "oracle.pls_pm_plsc_and_supported_shape_breadth",
        "oracle.frozen_index_and_failure_ledger_comparison",
    ),
    "generative_recovery": (
        "simulation.preregistered_scenario_matrix",
        "simulation.type_i_error_power_coverage_failure_rate",
        "simulation.maximum_one_percentage_point_half_width",
        "simulation.minimum_999_exact_fit_draws",
        "simulation.product_engine_worker_matrix",
    ),
    "adversarial_boundaries": (
        "adversarial.product_engine_boundaries",
        "adversarial.archive_and_native_fail_closed",
        "adversarial.metamorphic_worker_gui_cli_save_reopen",
        "adversarial.supported_shape_matrix",
    ),
    "archive_persistence": (
        "archive.real_runner_append_save_close_reopen",
        "archive.current_legacy_future_read_policy",
        "archive.interrupted_save_and_cancellation",
        "archive.structural_and_semantic_tamper_rejection",
        "archive.same_run_exact_fit_identity",
    ),
    "cross_format_export": (
        "export.csv_readback",
        "export.xlsx_readback",
        "export.html_readback",
        "export.svg_readback",
        "export.pdf_readback",
        "export.png_source_identity",
        "export.canonical_cells_precision_warnings_provenance",
    ),
    "frontend_contract": (
        "frontend.compatible_setup_and_preflight",
        "frontend.point_and_exact_results_linked_to_same_run",
        "frontend.accessible_tables_and_non_colour_semantics",
        "frontend.labs_one_warning_policy",
        "frontend.cancellation_no_partial_result",
    ),
    "packaged_windows_e2e": (
        "packaged.installed_offline_workflow",
        "packaged.portable_offline_workflow",
        "packaged.invalid_setup_cancel_retry_save_reopen_export",
        "packaged.viewport_scaling_keyboard_pointer_accessibility",
        "packaged.clean_process_exit",
    ),
    "performance_scale": (
        "performance.standard_and_workstation_profiles",
        "performance.five_runs_after_warmup",
        "performance.median_and_high_percentile_budgets",
        "performance.maximum_axis_and_compound_stress",
        "performance.cancellation_memory_soak_regression",
    ),
}

SOURCE_PATHS = (
    "crates/qpls-assessment/src/lib.rs",
    "crates/qpls-project/src/lib.rs",
    "crates/qpls-resampling/src/pls_model_fit_exact.rs",
    "crates/qpls-runner/src/lib.rs",
    "docs/methods/PLS_MODEL_FIT_EXACT_V1.md",
    "docs/methods/PLS_MODEL_FIT_FULL_REFIT_ORACLE_V1.md",
    "docs/methods/PLS_MODEL_FIT_V2.md",
    "src/native/nativeExportTables.test.ts",
    "src/native/nativePlsModelFit.test.ts",
    "validation/capabilities/capability_registry_v2.json",
    "validation/fixtures/simple_reflective.csv",
    "validation/fixtures/simple_reflective.recipe.json",
    "validation/method_promotion_manifest.py",
    "validation/methods/method_promotion_manifest.schema.json",
    "validation/methods/pls_model_fit_v2.manifest.json",
    "validation/pls_model_fit_exact_v1_reference.py",
    "validation/pls_model_fit_full_refit_oracle.py",
    "validation/pls_model_fit_full_refit_oracle.schema.json",
    "validation/pls_model_fit_v2_qualification_evidence.py",
    "validation/pls_model_fit_v2_qualification_evidence.schema.json",
    "validation/pls_model_fit_v2_qualification_factory.py",
    "validation/pls_model_fit_v2_qualification_factory.schema.json",
    "validation/pls_model_fit_v2_reference.py",
    "validation/qualification_spec_v2.py",
    "validation/qualification_v2/pls_model_fit_exact_v1.qualification.json",
    "validation/qualification_v2/qualification_spec_v2.schema.json",
    "validation/results/pls_quickpls_path_mode_a.json",
    "validation/test_pls_model_fit_exact_v1_qualification.py",
    "validation/test_pls_model_fit_exact_v1_reference.py",
    "validation/test_pls_model_fit_full_refit_oracle.py",
    "validation/test_pls_model_fit_v2_qualification_evidence.py",
    "validation/test_pls_model_fit_v2_qualification_factory.py",
    "validation/test_pls_model_fit_v2_reference.py",
)
WORK_EVIDENCE_PATHS = {
    "oracle_independence": WORK_ROOT / "independent_full_refit_oracle.json",
    "generative_recovery": WORK_ROOT / "generative_calibration.json",
    "adversarial_boundaries": WORK_ROOT / "adversarial_matrix.json",
}
WORK_EVIDENCE_SCHEMAS = {
    "oracle_independence": VALIDATION / "pls_model_fit_full_refit_oracle.schema.json",
    "generative_recovery": (
        VALIDATION / "pls_model_fit_v2_qualification_evidence.schema.json"
    ),
    "adversarial_boundaries": (
        VALIDATION / "pls_model_fit_v2_qualification_evidence.schema.json"
    ),
}


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _is_sha256(value: object) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdef" for character in value)
    )


def _repository_file(relative: object) -> Path | None:
    if not isinstance(relative, str) or not relative or "\\" in relative:
        return None
    candidate = Path(relative)
    if candidate.is_absolute() or ".." in candidate.parts:
        return None
    path = (ROOT / candidate).resolve()
    try:
        path.relative_to(ROOT.resolve())
    except ValueError:
        return None
    return path if path.is_file() else None


def repository_path(path: Path) -> str:
    return path.resolve().relative_to(ROOT.resolve()).as_posix()


def source_descriptors(paths: Iterable[str] = SOURCE_PATHS) -> list[dict[str, Any]]:
    descriptors = []
    for relative in sorted(set(paths)):
        candidate = Path(relative)
        if candidate.is_absolute() or ".." in candidate.parts or "\\" in relative:
            raise ValueError(f"unsafe qualification source path: {relative!r}")
        path = ROOT / candidate
        if not path.is_file():
            raise FileNotFoundError(f"required qualification source is missing: {relative}")
        descriptors.append(
            {
                "path": relative,
                "size_bytes": path.stat().st_size,
                "sha256": sha256_file(path),
            }
        )
    return descriptors


def source_set_sha256(descriptors: Sequence[dict[str, Any]]) -> str:
    return canonical_sha256(
        [
            {
                "path": row["path"],
                "size_bytes": row["size_bytes"],
                "sha256": row["sha256"],
            }
            for row in descriptors
        ]
    )


def _parse_utc(value: str) -> datetime:
    if not isinstance(value, str) or not value.endswith("Z"):
        raise ValueError("UTC timestamp must end in Z")
    parsed = datetime.fromisoformat(value[:-1] + "+00:00")
    if parsed.utcoffset() != timedelta(0):
        raise ValueError("timestamp must use UTC")
    return parsed


def _generated_at(value: str | None) -> str:
    if value is None:
        value = datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")
    parsed = _parse_utc(value)
    frozen = _parse_utc(
        strict_load_json(SPEC_PATH)["identity"]["spec_frozen_at_utc"]
    )
    if parsed < frozen:
        raise ValueError("factory timestamp predates the frozen QualificationSpec")
    if parsed > datetime.now(timezone.utc) + timedelta(minutes=5):
        raise ValueError("factory timestamp is implausibly in the future")
    return value


def _memory_gib() -> float:
    if os.name == "nt":
        class MemoryStatusEx(ctypes.Structure):
            _fields_ = [
                ("length", ctypes.c_ulong),
                ("memory_load", ctypes.c_ulong),
                ("total_physical", ctypes.c_ulonglong),
                ("available_physical", ctypes.c_ulonglong),
                ("total_page_file", ctypes.c_ulonglong),
                ("available_page_file", ctypes.c_ulonglong),
                ("total_virtual", ctypes.c_ulonglong),
                ("available_virtual", ctypes.c_ulonglong),
                ("available_extended_virtual", ctypes.c_ulonglong),
            ]

        status = MemoryStatusEx()
        status.length = ctypes.sizeof(MemoryStatusEx)
        if ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(status)):
            return round(status.total_physical / (1024**3), 3)
    if hasattr(os, "sysconf"):
        pages = os.sysconf("SC_PHYS_PAGES")
        page_size = os.sysconf("SC_PAGE_SIZE")
        return round(pages * page_size / (1024**3), 3)
    return 0.001


def hardware_fingerprint() -> dict[str, Any]:
    architecture = platform.machine().casefold()
    if architecture in {"amd64", "x64", "x86_64"}:
        architecture = "x86_64"
    if architecture != "x86_64":
        raise ValueError(f"qualification factory requires x86_64, found {architecture!r}")
    return {
        "os": f"{platform.system()} {platform.release()}",
        "architecture": architecture,
        "cpu": platform.processor() or os.environ.get("PROCESSOR_IDENTIFIER", "unknown-cpu"),
        "logical_cores": os.cpu_count() or 1,
        "memory_gib": _memory_gib(),
    }


def _find_registry_cell(registry: dict[str, Any]) -> tuple[dict[str, Any], dict[str, Any]]:
    matches = [
        row
        for row in registry.get("capabilities", [])
        if row.get("capability_id") == CAPABILITY_ID
    ]
    if len(matches) != 1:
        raise ValueError("model-fit capability registry row is missing or duplicated")
    capability = matches[0]
    cells = [
        row
        for row in capability.get("option_cells", [])
        if row.get("cell_id") == CELL_ID
    ]
    if len(cells) != 1:
        raise ValueError("model-fit capability option cell is missing or duplicated")
    return capability, cells[0]


def _identity(spec: dict[str, Any]) -> dict[str, str]:
    identity = spec["identity"]
    cell = identity["capability_cell"]
    expected = {
        "qualification_id": QUALIFICATION_ID,
        "capability_id": CAPABILITY_ID,
        "cell_id": CELL_ID,
        "method_version": METHOD_VERSION,
    }
    actual = {
        "qualification_id": identity["qualification_id"],
        "capability_id": cell["capability_id"],
        "cell_id": cell["cell_id"],
        "method_version": identity["method_version"],
    }
    if actual != expected or cell["capability_version"] != METHOD_VERSION:
        raise ValueError(f"model-fit qualification identity mismatch: {actual!r}")
    return expected


def _descriptor_current(descriptor: dict[str, Any]) -> bool:
    path = _repository_file(descriptor.get("path"))
    return (
        path is not None
        and path.stat().st_size == descriptor.get("size_bytes")
        and _is_sha256(descriptor.get("sha256"))
        and sha256_file(path) == descriptor["sha256"]
    )


def _audit_work_evidence(
    role: str,
    path: Path,
    *,
    scenario_sha: str,
) -> dict[str, Any]:
    result: dict[str, Any] = {
        "role": role,
        "path": repository_path(path),
        "exists": path.is_file(),
        "current": False,
        "passed_work_checks": False,
        "qualification_role_satisfied": False,
        "receipt_eligible": False,
        "errors": [],
    }
    if not path.is_file():
        result["errors"].append("work_evidence_missing")
        return result
    document = strict_load_json(path)
    try:
        jsonschema.Draft202012Validator(
            strict_load_json(WORK_EVIDENCE_SCHEMAS[role])
        ).validate(document)
    except jsonschema.ValidationError as error:
        result["errors"].append(
            f"work_evidence_schema_invalid:{error.json_path}"
        )
        return result
    result["sha256"] = sha256_file(path)
    result["size_bytes"] = path.stat().st_size
    if role == "oracle_independence":
        reference = document.get("reference", {})
        source_path = _repository_file(reference.get("source"))
        current = (
            document.get("kind") == "pls_model_fit_full_refit_oracle_work_report_v1"
            and document.get("passed") is True
            and document.get("qualification_ready") is False
            and document.get("promotion_requested") is False
            and source_path is not None
            and reference.get("source_sha256") == sha256_file(source_path)
            and document.get("frozen_product_comparison", {}).get("passed") is True
            and document.get("frozen_product_comparison", {}).get("role")
            == "behavioral_comparator_only_not_numerical_oracle"
        )
        fixtures = document.get("fixtures", {})
        for label in ("recipe", "data", "frozen_product"):
            fixture = _repository_file(fixtures.get(label))
            current = current and fixture is not None
            if fixture is not None:
                current = current and fixtures.get(f"{label}_sha256") == sha256_file(
                    fixture
                )
        result["current"] = bool(current)
        result["passed_work_checks"] = bool(document.get("passed") and current)
        result["errors"] = [] if current else ["oracle_work_report_stale_or_invalid"]
    else:
        source_path = _repository_file(document.get("source"))
        current = (
            source_path is not None
            and document.get("source_sha256") == sha256_file(source_path)
            and document.get("scenario_set_sha256") == scenario_sha
            and document.get("qualification_role_satisfied") is False
            and document.get("receipt_eligible") is False
            and isinstance(document.get("blockers"), list)
            and bool(document.get("blockers"))
        )
        result["current"] = bool(current)
        result["passed_work_checks"] = bool(
            current and document.get("passed_work_checks") is True
        )
        result["errors"] = [] if current else ["work_report_stale_or_invalid"]
    return result


def _expected_execution_path(role: str) -> Path:
    return INPUT_ROOT / f"{role}.qualification.json"


def verify_execution_envelope(
    role: str,
    *,
    identity: dict[str, str],
    generated_after: datetime,
    sources: Sequence[dict[str, Any]],
    source_sha: str,
    scenario_sha: str,
    build_fingerprint: str,
) -> dict[str, Any]:
    path = _expected_execution_path(role)
    result: dict[str, Any] = {
        "path": repository_path(path),
        "exists": path.is_file(),
        "eligible": False,
        "errors": [],
    }
    if not path.is_file():
        result["errors"].append("immutable_execution_envelope_missing")
        return result
    try:
        document = strict_load_json(path)
        errors: list[str] = []
        if document.get("schema_version") != 1:
            errors.append("execution_envelope_schema_version_mismatch")
        if document.get("report_kind") != "pls_model_fit_v2_qualification_execution_v1":
            errors.append("execution_envelope_kind_mismatch")
        if document.get("role") != role or document.get("stage") != ROLE_STAGES[role]:
            errors.append("execution_envelope_role_or_stage_mismatch")
        if document.get("passed") is not True:
            errors.append("execution_envelope_not_passing")
        if document.get("qualification_evidence") is not True:
            errors.append("execution_envelope_not_qualification_evidence")
        if document.get("receipt_eligible") is not True:
            errors.append("execution_envelope_not_receipt_eligible")
        for field, expected in identity.items():
            if document.get(field) != expected:
                errors.append(f"execution_envelope_{field}_mismatch")
        if document.get("source_set_sha256") != source_sha:
            errors.append("execution_envelope_source_set_mismatch")
        if document.get("scenario_set_sha256") != scenario_sha:
            errors.append("execution_envelope_scenario_set_mismatch")
        if document.get("build_fingerprint") != build_fingerprint:
            errors.append("execution_envelope_build_fingerprint_mismatch")
        if build_fingerprint.startswith("validation-source-contract:"):
            errors.append("execution_envelope_product_build_fingerprint_required")
        if document.get("source_artifacts") != list(sources):
            errors.append("execution_envelope_source_descriptors_mismatch")
        try:
            timestamp = _parse_utc(document.get("generated_at_utc"))
            if timestamp < generated_after:
                errors.append("execution_envelope_predates_frozen_spec")
            if timestamp > datetime.now(timezone.utc) + timedelta(minutes=5):
                errors.append("execution_envelope_future_timestamp")
        except (TypeError, ValueError):
            errors.append("execution_envelope_timestamp_invalid")
        observed_checks = document.get("checks")
        if not isinstance(observed_checks, list):
            errors.append("execution_envelope_checks_missing")
            observed_checks = []
        check_ids = [row.get("check_id") for row in observed_checks if isinstance(row, dict)]
        if set(check_ids) != set(REQUIRED_CHECK_IDS[role]) or len(check_ids) != len(
            REQUIRED_CHECK_IDS[role]
        ):
            errors.append("execution_envelope_required_checks_not_exact")
        if not all(
            isinstance(row, dict) and row.get("passed") is True
            for row in observed_checks
        ):
            errors.append("execution_envelope_check_failed")
        commands = document.get("commands")
        if not isinstance(commands, list) or not commands:
            errors.append("execution_envelope_commands_missing")
            commands = []
        command_proof_valid = True
        for row in commands:
            if not (
                isinstance(row, dict)
                and isinstance(row.get("command"), list)
                and bool(row.get("command"))
                and all(isinstance(token, str) and token for token in row["command"])
                and row.get("returncode") == 0
                and _is_sha256(row.get("stdout_sha256"))
                and _is_sha256(row.get("stderr_sha256"))
                and isinstance(row.get("duration_seconds"), (int, float))
                and row["duration_seconds"] >= 0
            ):
                command_proof_valid = False
                continue
            try:
                started = _parse_utc(row.get("started_at_utc"))
                finished = _parse_utc(row.get("finished_at_utc"))
                if started < generated_after or finished < started:
                    command_proof_valid = False
            except (TypeError, ValueError):
                command_proof_valid = False
        if not command_proof_valid:
            errors.append("execution_envelope_command_proof_invalid")
        artifacts = document.get("artifacts")
        if not isinstance(artifacts, list) or not artifacts:
            errors.append("execution_envelope_artifacts_missing")
            artifacts = []
        if not all(isinstance(row, dict) and _descriptor_current(row) for row in artifacts):
            errors.append("execution_envelope_artifact_hash_mismatch")
        fingerprint = document.get("hardware_fingerprint")
        if not isinstance(fingerprint, dict) or set(fingerprint) != {
            "os",
            "architecture",
            "cpu",
            "logical_cores",
            "memory_gib",
        }:
            errors.append("execution_envelope_hardware_fingerprint_invalid")
        elif fingerprint.get("architecture") != "x86_64":
            errors.append("execution_envelope_hardware_architecture_invalid")
        if role in {"packaged_windows_e2e", "performance_scale"} and (
            not isinstance(fingerprint, dict)
            or not str(fingerprint.get("os", "")).casefold().startswith("windows")
        ):
            errors.append("execution_envelope_windows_hardware_required")
        if role in {"packaged_windows_e2e", "performance_scale"} and isinstance(
            fingerprint, dict
        ):
            hardware_classes = strict_load_json(SPEC_PATH)["operational_contract"][
                "performance"
            ]["hardware_classes"]
            if not any(
                fingerprint.get("architecture") == hardware_class["architecture"]
                and fingerprint.get("logical_cores", 0)
                >= hardware_class["minimum_logical_cores"]
                and fingerprint.get("memory_gib", 0)
                >= hardware_class["minimum_memory_gib"]
                for hardware_class in hardware_classes
            ):
                errors.append("execution_envelope_hardware_class_not_satisfied")
        result.update(
            {
                "eligible": not errors,
                "errors": errors,
                "sha256": sha256_file(path),
                "size_bytes": path.stat().st_size,
                "generated_at_utc": document.get("generated_at_utc"),
                "hardware_fingerprint": fingerprint,
            }
        )
        return result
    except (OSError, ValueError, json.JSONDecodeError) as error:
        result["errors"].append(f"execution_envelope_parse_failure:{type(error).__name__}")
        return result


def _receipt_descriptor(
    role: str,
    path: Path,
    *,
    identity: dict[str, str],
    generated_at: str,
    source_sha: str,
    scenario_sha: str,
    build_fingerprint: str,
    hardware: dict[str, Any],
    expected_size_bytes: int | None = None,
    expected_sha256: str | None = None,
) -> dict[str, Any]:
    size_bytes = path.stat().st_size
    digest = sha256_file(path)
    if expected_size_bytes is not None and size_bytes != expected_size_bytes:
        raise RuntimeError(f"receipt artifact changed size after verification: {path}")
    if expected_sha256 is not None and digest != expected_sha256:
        raise RuntimeError(f"receipt artifact changed hash after verification: {path}")
    return {
        "role": role,
        "stage": ROLE_STAGES[role],
        "evidence_class": "qualification",
        **identity,
        "path": repository_path(path),
        "size_bytes": size_bytes,
        "sha256": digest,
        "generated_at_utc": generated_at,
        "source_set_sha256": source_sha,
        "scenario_set_sha256": scenario_sha,
        "build_fingerprint": build_fingerprint,
        "hardware_fingerprint": hardware,
    }


def _write_json(path: Path, document: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(document, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )


def build_factory_documents(
    *,
    generated_at: str,
    build_fingerprint: str | None = None,
) -> tuple[dict[str, Any], dict[str, Any]]:
    initial_hashes = {
        "qualification_spec": sha256_file(SPEC_PATH),
        "manifest": sha256_file(MANIFEST_PATH),
        "registry": sha256_file(REGISTRY_PATH),
    }
    spec = strict_load_json(SPEC_PATH)
    manifest = strict_load_json(MANIFEST_PATH)
    registry = strict_load_json(REGISTRY_PATH)
    identity = _identity(spec)
    capability, cell = _find_registry_cell(registry)
    required_roles = tuple(spec["evidence_contract"]["required_roles"])
    if required_roles != EXPECTED_REQUIRED_ROLES:
        raise ValueError("QualificationSpec required receipt roles changed")
    sources = source_descriptors()
    source_sha = source_set_sha256(sources)
    scenario_sha = canonical_sha256(spec["scenario_contract"])
    build_fingerprint = build_fingerprint or f"validation-source-contract:{source_sha}"
    hardware = hardware_fingerprint()
    spec_validation = validate_spec_path(
        SPEC_PATH,
        repository_root=ROOT,
        registry_path=REGISTRY_PATH,
        require_registry=True,
    )
    manifest_evidence = manifest["qualification"]["evidence"]
    manifest_empty = all(not manifest_evidence[stage] for stage in manifest_evidence)
    contract_checks = {
        "qualification_spec_schema_semantics_and_registry_pass": (
            spec_validation["passed"]
            and spec_validation["schema_valid"]
            and spec_validation["semantic_valid"]
            and spec_validation["registry_verified"]
        ),
        "qualification_spec_is_compatibility_only": (
            spec["migration"]["status"] == "compatibility_only"
            and bool(spec["migration"]["unresolved_items"])
        ),
        "qualification_spec_has_zero_attached_receipts": (
            spec["evidence_contract"]["receipts"] == []
            and spec_validation["receipts_verified"] is False
            and spec_validation["qualification_ready"] is False
        ),
        "contract_sections_are_frozen_and_nonempty": all(
            bool(spec[section])
            for section in (
                "scientific_contract",
                "scenario_contract",
                "comparison_contract",
                "operational_contract",
                "evidence_contract",
            )
        ),
        "required_receipt_roles_are_exact": required_roles == EXPECTED_REQUIRED_ROLES,
        "manifest_state_is_absent_with_no_admitted_evidence": (
            manifest["feature"]["id"] == CELL_ID
            and manifest["feature"]["method_version"] == METHOD_VERSION
            and manifest["qualification"]["declared_state"] == "absent"
            and manifest_empty
        ),
        "registry_state_remains_truthful_and_nonstandard": (
            capability["coverage_state"] == "partial"
            and capability["evidence_state"] == "absent"
            and capability["surface"] == "labs"
            and cell["coverage_state"] == "partial"
            and cell["evidence_state"] == "absent"
            and cell["surface"] == "labs"
            and cell["capability_version"] == METHOD_VERSION
        ),
        "source_descriptors_are_current": all(_descriptor_current(row) for row in sources),
    }
    contract_passed = all(contract_checks.values())
    if not contract_passed:
        failed = sorted(key for key, passed in contract_checks.items() if not passed)
        raise RuntimeError(
            "method-contract qualification checks failed: " + ", ".join(failed)
        )
    method_contract = {
        "schema_version": 1,
        "report_kind": "pls_model_fit_v2_method_contract_qualification_v1",
        "role": "method_contract",
        "stage": "contract",
        "passed": contract_passed,
        "qualification_evidence": contract_passed,
        "receipt_eligible": contract_passed,
        "qualification_ready": False,
        "promotion_authority": False,
        **identity,
        "generated_at_utc": generated_at,
        "spec_frozen_at_utc": spec["identity"]["spec_frozen_at_utc"],
        "source_set_sha256": source_sha,
        "scenario_set_sha256": scenario_sha,
        "build_fingerprint": build_fingerprint,
        "hardware_fingerprint": hardware,
        "source_artifacts": sources,
        "checks": contract_checks,
        "role_blockers": [] if contract_passed else sorted(
            key for key, passed in contract_checks.items() if not passed
        ),
        "admission_policy": (
            "candidate descriptor only; separate reviewed attachment is required and "
            "cannot occur until every QualificationSpec V2 role is satisfied"
        ),
    }

    work_evidence = [
        _audit_work_evidence(role, path, scenario_sha=scenario_sha)
        for role, path in WORK_EVIDENCE_PATHS.items()
    ]
    work_by_role = {row["role"]: row for row in work_evidence}
    execution_rows = {}
    for role in EXPECTED_REQUIRED_ROLES[1:]:
        execution_rows[role] = verify_execution_envelope(
            role,
            identity=identity,
            generated_after=_parse_utc(spec["identity"]["spec_frozen_at_utc"]),
            sources=sources,
            source_sha=source_sha,
            scenario_sha=scenario_sha,
            build_fingerprint=build_fingerprint,
        )

    role_matrix = []
    for role in EXPECTED_REQUIRED_ROLES:
        if role == "method_contract":
            status = "candidate_receipt_emitted" if contract_passed else "blocked"
            reasons = [] if contract_passed else method_contract["role_blockers"]
            execution = None
        else:
            execution = execution_rows[role]
            if execution["eligible"]:
                status = "candidate_receipt_emitted"
                reasons = []
            elif role in work_by_role and work_by_role[role]["passed_work_checks"]:
                status = "work_evidence_only"
                reasons = [
                    "independent_work_is_not_an_immutable_qualification_execution_envelope",
                    *execution["errors"],
                ]
            else:
                status = "blocked"
                reasons = list(execution["errors"])
        role_matrix.append(
            {
                "role": role,
                "stage": ROLE_STAGES[role],
                "status": status,
                "candidate_receipt_emitted": status == "candidate_receipt_emitted",
                "work_evidence": work_by_role.get(role),
                "execution_envelope": execution,
                "reasons": reasons,
            }
        )

    audit = {
        "schema_version": 1,
        "report_kind": "pls_model_fit_v2_qualification_factory_audit_v1",
        **identity,
        "generated_at_utc": generated_at,
        "spec_frozen_at_utc": spec["identity"]["spec_frozen_at_utc"],
        "source_set_sha256": source_sha,
        "scenario_set_sha256": scenario_sha,
        "build_fingerprint": build_fingerprint,
        "hardware_fingerprint": hardware,
        "source_artifacts": sources,
        "source_state_before": initial_hashes,
        "state_snapshot": {
            "qualification_spec_migration": spec["migration"]["status"],
            "qualification_spec_attached_receipt_count": len(
                spec["evidence_contract"]["receipts"]
            ),
            "manifest_declared_state": manifest["qualification"]["declared_state"],
            "manifest_evidence_arrays_empty": manifest_empty,
            "registry_coverage_state": cell["coverage_state"],
            "registry_evidence_state": cell["evidence_state"],
            "registry_surface": cell["surface"],
        },
        "method_contract_checks": contract_checks,
        "work_evidence": work_evidence,
        "role_matrix": role_matrix,
        "candidate_receipt_descriptors": [],
        "attached_receipt_count": 0,
        "registry_mutated": False,
        "manifest_mutated": False,
        "qualification_spec_mutated": False,
        "scientific_review_satisfied": False,
        "qualification_ready": False,
        "promotion_allowed": False,
        "remaining_blockers": sorted(
            {
                *spec["migration"]["unresolved_items"],
                "review.independent_scientific_review_not_recorded",
                *(
                    f"receipt.{row['role']}_not_emitted"
                    for row in role_matrix
                    if not row["candidate_receipt_emitted"]
                ),
            }
        ),
        "execution_envelope_contract": {
            "input_directory": repository_path(INPUT_ROOT),
            "report_kind": "pls_model_fit_v2_qualification_execution_v1",
            "required_source_set_sha256": source_sha,
            "required_scenario_set_sha256": scenario_sha,
            "required_build_fingerprint": build_fingerprint,
            "required_check_ids": REQUIRED_CHECK_IDS,
            "policy": (
                "Test-source presence or an interactive green command is never enough. "
                "Each non-contract role requires a current immutable execution envelope, "
                "exact check IDs, successful command proof, and hash-current artifacts."
            ),
        },
        "note": (
            "This audit is fail-closed and non-promotional. It may emit candidate role "
            "descriptors, but it never edits the QualificationSpec, v1 manifest, or "
            "CapabilityRegistryV2 and cannot establish overall qualification readiness."
        ),
    }
    return method_contract, audit


def write_factory_documents(
    *,
    generated_at: str | None = None,
    build_fingerprint: str | None = None,
) -> dict[str, Any]:
    generated_at = _generated_at(generated_at)
    before = {
        SPEC_PATH: sha256_file(SPEC_PATH),
        MANIFEST_PATH: sha256_file(MANIFEST_PATH),
        REGISTRY_PATH: sha256_file(REGISTRY_PATH),
    }
    method_contract, audit = build_factory_documents(
        generated_at=generated_at,
        build_fingerprint=build_fingerprint,
    )
    _write_json(METHOD_CONTRACT_PATH, method_contract)
    identity = {
        key: audit[key]
        for key in ("qualification_id", "capability_id", "cell_id", "method_version")
    }
    descriptors = []
    if method_contract["receipt_eligible"]:
        descriptors.append(
            _receipt_descriptor(
                "method_contract",
                METHOD_CONTRACT_PATH,
                identity=identity,
                generated_at=generated_at,
                source_sha=audit["source_set_sha256"],
                scenario_sha=audit["scenario_set_sha256"],
                build_fingerprint=audit["build_fingerprint"],
                hardware=audit["hardware_fingerprint"],
            )
        )
    for row in audit["role_matrix"]:
        role = row["role"]
        if role == "method_contract" or not row["candidate_receipt_emitted"]:
            continue
        execution = row["execution_envelope"]
        path = ROOT / execution["path"]
        descriptors.append(
            _receipt_descriptor(
                role,
                path,
                identity=identity,
                generated_at=execution["generated_at_utc"],
                source_sha=audit["source_set_sha256"],
                scenario_sha=audit["scenario_set_sha256"],
                build_fingerprint=audit["build_fingerprint"],
                hardware=execution["hardware_fingerprint"],
                expected_size_bytes=execution["size_bytes"],
                expected_sha256=execution["sha256"],
            )
        )
    audit["candidate_receipt_descriptors"] = descriptors
    after = {path: sha256_file(path) for path in before}
    audit["qualification_spec_mutated"] = before[SPEC_PATH] != after[SPEC_PATH]
    audit["manifest_mutated"] = before[MANIFEST_PATH] != after[MANIFEST_PATH]
    audit["registry_mutated"] = before[REGISTRY_PATH] != after[REGISTRY_PATH]
    if any(
        audit[field]
        for field in (
            "qualification_spec_mutated",
            "manifest_mutated",
            "registry_mutated",
        )
    ):
        raise RuntimeError("qualification factory detected forbidden state mutation")
    _write_json(AUDIT_PATH, audit)
    return audit


def verify_checked_in_factory() -> dict[str, Any]:
    if not METHOD_CONTRACT_PATH.is_file() or not AUDIT_PATH.is_file():
        raise FileNotFoundError("checked-in model-fit qualification factory artifacts missing")
    audit = strict_load_json(AUDIT_PATH)
    method_contract = strict_load_json(METHOD_CONTRACT_PATH)
    sources = source_descriptors()
    expected_source_sha = source_set_sha256(sources)
    spec = strict_load_json(SPEC_PATH)
    errors = []
    if audit.get("source_artifacts") != sources:
        errors.append("factory_source_descriptors_stale")
    if audit.get("source_set_sha256") != expected_source_sha:
        errors.append("factory_source_set_hash_stale")
    if audit.get("scenario_set_sha256") != canonical_sha256(spec["scenario_contract"]):
        errors.append("factory_scenario_hash_stale")
    if method_contract.get("source_set_sha256") != expected_source_sha:
        errors.append("method_contract_source_set_hash_stale")
    descriptors = audit.get("candidate_receipt_descriptors", [])
    if [row.get("role") for row in descriptors] != ["method_contract"]:
        errors.append("unexpected_candidate_receipt_roles")
    elif not _descriptor_current(descriptors[0]):
        errors.append("method_contract_receipt_descriptor_stale")
    if spec["evidence_contract"]["receipts"]:
        errors.append("qualification_spec_receipts_were_attached")
    if audit.get("qualification_ready") is not False or audit.get("promotion_allowed") is not False:
        errors.append("factory_makes_promotion_claim")
    if audit.get("scientific_review_satisfied") is not False:
        errors.append("factory_claims_scientific_review")
    if any(
        audit.get(field) is not False
        for field in (
            "qualification_spec_mutated",
            "manifest_mutated",
            "registry_mutated",
        )
    ):
        errors.append("factory_records_forbidden_state_mutation")
    return {
        "passed": not errors,
        "errors": errors,
        "candidate_receipt_roles": [row.get("role") for row in descriptors],
        "qualification_ready": False,
        "promotion_allowed": False,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--verify", action="store_true")
    parser.add_argument("--generated-at")
    parser.add_argument("--build-fingerprint")
    arguments = parser.parse_args()
    if arguments.write == arguments.verify:
        parser.error("select exactly one of --write or --verify")
    if arguments.write:
        report = write_factory_documents(
            generated_at=arguments.generated_at,
            build_fingerprint=arguments.build_fingerprint,
        )
        summary = {
            "candidate_receipt_roles": [
                row["role"] for row in report["candidate_receipt_descriptors"]
            ],
            "qualification_ready": report["qualification_ready"],
            "promotion_allowed": report["promotion_allowed"],
            "blocked_role_count": sum(
                not row["candidate_receipt_emitted"] for row in report["role_matrix"]
            ),
        }
        print(json.dumps(summary, indent=2, sort_keys=True))
        return 0
    result = verify_checked_in_factory()
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
