#!/usr/bin/env python3
"""Fail-closed current-source QualificationSpec V2 factory for PLS-PM v1.

The factory creates a new source-hash-addressed snapshot.  It never overwrites
the historical PLS promotion reports, executes QuickPLS, builds Rust/TypeScript,
attaches receipts to the specification, edits the method manifest or registry,
or claims promotion.  A source-contract candidate is emitted only when the
current spec/manifest/registry contract passes.  All product-execution roles
remain blocked unless a separately frozen current-build envelope is supplied.
"""

from __future__ import annotations

import argparse
import copy
import csv
import hashlib
import json
import math
import os
import platform
import subprocess
import sys
import time
import uuid
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable, Mapping, Sequence


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
sys.path.insert(0, str(VALIDATION))

import pls_algorithm_v1_current_source_oracle as oracle  # noqa: E402
from qualification_spec_v2 import (  # noqa: E402
    canonical_sha256,
    strict_load_json,
    validate_spec_path,
)


QUALIFICATION_ID = "qpls3.pls.algorithm.qualification_v2"
CAPABILITY_ID = "smartpls.pls_algorithm"
CELL_ID = "qpls3.pls.algorithm"
METHOD_VERSION = "pls_pm_v1"
SPEC_FROZEN_AT_UTC = "2026-08-15T01:00:00Z"
OFFICIAL_SNAPSHOT_DATE = "2026-08-15"
OFFICIAL_PLS_URL = (
    "https://smartpls.com/documentation/algorithms-and-techniques/core-algorithm/pls/"
)
OFFICIAL_MODEL_URL = (
    "https://smartpls.com/documentation/functionalities/"
    "composite-and-common-factor-and-mixed-models/"
)

SPEC_PATH = VALIDATION / "qualification_v2" / "pls_algorithm_v1.qualification.json"
BASELINE_FIXTURE_PATH = (
    VALIDATION / "qualification_v2" / "fixtures" / "pls_algorithm_v1.migrated.json"
)
REGISTRY_PATH = VALIDATION / "capabilities" / "capability_registry_v2.json"
MANIFEST_PATH = VALIDATION / "methods" / "pls_algorithm_v1.manifest.json"
RESULT_ROOT = (
    VALIDATION / "results" / "method_factory" / "pls_algorithm_v1" / "current_source_v2"
)
INTERNAL_HARNESS_PACKAGE = "quickpls-desktop"
INTERNAL_HARNESS_TARGET = "quickpls_desktop_lib"
INTERNAL_HARNESS_TEST = (
    "pls_algorithm_current_product_qualification::"
    "current_product_pls_qualification_work_harness"
)
INTERNAL_HARNESS_ORCHESTRATOR = "validation/pls_algorithm_v1_current_product_work.py"
CURRENT_PRODUCT_VARIANTS: Mapping[str, tuple[str, str]] = {
    "path_mode_a": ("path", "mode_a"),
    "path_mode_b": ("path", "mode_b"),
    "factor_mode_a": ("factor", "mode_a"),
    "pca_mode_a": ("pca", "mode_a"),
}
CURRENT_PRODUCT_DATASET_FINGERPRINT = (
    "v2:9b814a8a22eace42776bc479c637de36bc973dfd8a7b3d5b3e60baba98803373"
)
CURRENT_PRODUCT_COMPILER_VERSION = "recipe_v4_to_compiled_pls_plan_v2_v2"
CURRENT_PRODUCT_LEGACY_ADAPTER_VERSION = "compiled_recipe_v4_pls_plan_v2_execution_v3"
CURRENT_PRODUCT_CURRENT_ADAPTER_VERSION = "compiled_recipe_v4_pls_plan_v2_execution_v5"
CURRENT_PRODUCT_ADAPTER_VERSIONS: Mapping[str, str] = {
    "path_mode_a": CURRENT_PRODUCT_CURRENT_ADAPTER_VERSION,
    "path_mode_b": CURRENT_PRODUCT_CURRENT_ADAPTER_VERSION,
    "factor_mode_a": CURRENT_PRODUCT_CURRENT_ADAPTER_VERSION,
    "pca_mode_a": CURRENT_PRODUCT_LEGACY_ADAPTER_VERSION,
}
CURRENT_PRODUCT_COMPILER_TARGET = "pls_plan_v2"
CURRENT_PRODUCT_WORK_BLOCKERS = (
    "The bounded four-variant current-product matrix is not the complete "
    "QualificationSpec scenario contract.",
    "Qualification-sized generative, adversarial, export, native packaged, "
    "performance, soak, and scientific-review roles remain absent.",
    "This work envelope has no authority to attach candidate receipts or mutate "
    "the separately governed scoped-Standard Registry, manifest, beta, or release state.",
)

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

PRODUCT_BUILD_SOURCE_PATHS = (
    "Cargo.lock",
    "Cargo.toml",
    "crates/qpls-core/Cargo.toml",
    "crates/qpls-core/src/analysis_recipe_v4.rs",
    "crates/qpls-core/src/canonical_result_v2.rs",
    "crates/qpls-core/src/capability_registry_v2.rs",
    "crates/qpls-core/src/compiled_pls_plan_v2.rs",
    "crates/qpls-core/src/contract.rs",
    "crates/qpls-core/src/lib.rs",
    "crates/qpls-core/src/recipe_v4_compiler.rs",
    "crates/qpls-core/src/sem_model_v4.rs",
    "crates/qpls-core/src/validation.rs",
    "crates/qpls-data/Cargo.toml",
    "crates/qpls-data/src/lib.rs",
    "crates/qpls-estimation/Cargo.toml",
    "crates/qpls-estimation/src/lib.rs",
    "crates/qpls-estimation/src/pls.rs",
    "crates/qpls-project/Cargo.toml",
    "crates/qpls-project/src/archive_integrity.rs",
    "crates/qpls-project/src/canonical_result_document_v2.rs",
    "crates/qpls-project/src/lib.rs",
    "crates/qpls-project/src/pls_score_execution_v2.rs",
    "crates/qpls-project/src/project_schema_v6.rs",
    "crates/qpls-runner/Cargo.toml",
    "crates/qpls-runner/src/lib.rs",
    "crates/qpls-runner/src/recipe_v4_pls_execution.rs",
    "src-tauri/Cargo.toml",
    "src-tauri/build.rs",
    "src-tauri/src/lib.rs",
    "src-tauri/src/pls_algorithm_current_product_qualification.rs",
    "src-tauri/src/project_schema6_result_append.rs",
    "src-tauri/src/project_schema6_result_read.rs",
    "src-tauri/src/recipe_v4_canonical_result.rs",
    "validation/fixtures/simple_reflective.csv",
    INTERNAL_HARNESS_ORCHESTRATOR,
)

SOURCE_PATHS = (
    *PRODUCT_BUILD_SOURCE_PATHS,
    "docs/methods/PLS_ALGORITHM_V1_QUALIFICATION_STATUS.md",
    "docs/methods/PLS_PM_V1.md",
    "docs/methods/PLS_SCORE_EXECUTION_V2_VALIDATION_STRATEGY.md",
    "validation/Resolve-Rscript.ps1",
    "validation/capabilities/capability_registry_v2.json",
    "validation/method_promotion_manifest.py",
    "validation/methods/method_promotion_manifest.schema.json",
    "validation/methods/pls_algorithm_v1.manifest.json",
    "validation/pls_algorithm_v1_current_source_oracle.py",
    "validation/pls_algorithm_v1_current_build_receipts.py",
    "validation/pls_algorithm_v1_qualification_factory.py",
    "validation/python_plspm_reference.py",
    "validation/qualification_spec_v2.py",
    "validation/qualification_v2/fixtures/pls_algorithm_v1.migrated.json",
    "validation/qualification_v2/pls_algorithm_v1.qualification.json",
    "validation/qualification_v2/qualification_spec_v2.schema.json",
    "validation/r_csem_reference.R",
    "validation/r_csem_threecommonfactors_reference.R",
    "validation/test_pls_algorithm_v1_current_source_oracle.py",
    "validation/test_pls_algorithm_v1_current_product_work.py",
    "validation/test_pls_algorithm_v1_qualification_factory.py",
)

FULL_PARITY_COVERAGE_BACKLOG = (
    "future_full_parity.coverage.sum_score_and_predefined_weight_semantics_beyond_bounded_unit_variance_execution",
    "future_full_parity.coverage.standard_mean_centered_unstandardized_result_breadth",
    "future_full_parity.coverage.full_supported_sem_model_shapes_through_sem_model_v4_compiler",
    "future_full_parity.coverage.complete_location_parameter_and_result_table_breadth",
)

FULL_PARITY_EVIDENCE_BACKLOG = (
    "future_full_parity.evidence.current_product_build_receipt",
    "future_full_parity.evidence.product_to_oracle_comparison_receipt",
    "future_full_parity.evidence.standard_individual_initial_outer_weights_matrix",
    "future_full_parity.evidence.unit_custom_mixed_fixed_only_score_execution_matrix",
    "future_full_parity.evidence.qualification_sized_bias_recovery_and_failure_simulation",
    "future_full_parity.evidence.product_adversarial_and_worker_matrix",
    "future_full_parity.evidence.real_runner_schema6_roundtrip_and_tamper_receipt",
    "future_full_parity.evidence.cross_format_semantic_readback",
    "future_full_parity.evidence.frontend_and_packaged_windows_receipts",
    "future_full_parity.evidence.performance_scale_soak_and_cancellation_receipts",
    "future_full_parity.evidence.independent_scientific_review",
)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def repository_path(path: Path) -> str:
    return path.resolve().relative_to(ROOT.resolve()).as_posix()


def source_descriptors(paths: Iterable[str] = SOURCE_PATHS) -> list[dict[str, Any]]:
    rows = []
    for relative in sorted(set(paths)):
        candidate = Path(relative)
        if candidate.is_absolute() or ".." in candidate.parts or "\\" in relative:
            raise ValueError(f"unsafe source path: {relative}")
        path = ROOT / candidate
        if not path.is_file():
            raise FileNotFoundError(f"PLS qualification source missing: {relative}")
        rows.append(
            {
                "path": relative,
                "size_bytes": path.stat().st_size,
                "sha256": sha256_file(path),
            }
        )
    return rows


def _write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )


def _require_frozen_qualification_spec(
    expected_spec: Mapping[str, Any], *, path: Path | None = None
) -> None:
    """Require an exact pre-frozen spec without changing its bytes or mtime."""

    target = path or SPEC_PATH
    if not target.is_file():
        raise FileNotFoundError(
            "frozen QualificationSpec is missing; run --refresh-spec before the "
            "source-bound product harness"
        )
    if strict_load_json(target) != expected_spec:
        raise ValueError(
            "frozen QualificationSpec differs from the deterministic builder; run "
            "--refresh-spec before source freeze, then regenerate current-product work"
        )


def refresh_qualification_spec(*, path: Path | None = None) -> dict[str, Any]:
    """Explicitly refresh only the deterministic spec before source qualification."""

    target = path or SPEC_PATH
    expected_spec = build_spec()
    if target.is_file() and strict_load_json(target) == expected_spec:
        changed = False
    else:
        _write_json(target, expected_spec)
        changed = True
    return {
        "passed": True,
        "refreshed": changed,
        "qualification_ready": False,
        "promotion_allowed": False,
        "spec": _descriptor(target),
        "next_step": (
            "Freeze all bound sources, then generate a fresh source-bound current-product "
            "work envelope before publishing the factory snapshot."
        ),
    }


def build_spec() -> dict[str, Any]:
    """Migrate contract content, never legacy receipts, into a fail-closed spec."""

    source = strict_load_json(BASELINE_FIXTURE_PATH)
    spec = copy.deepcopy(source)
    spec["identity"].update(
        {
            "qualification_id": QUALIFICATION_ID,
            "method_version": METHOD_VERSION,
            "spec_frozen_at_utc": SPEC_FROZEN_AT_UTC,
            "capability_cell": {
                "registry_schema_version": 2,
                "capability_id": CAPABILITY_ID,
                "capability_version": METHOD_VERSION,
                "cell_id": CELL_ID,
            },
        }
    )
    spec["migration"] = {
        "source_kind": "qualification_v1_manifest",
        "source_schema_version": 1,
        "source_manifest_path": "validation/methods/pls_algorithm_v1.manifest.json",
        "status": "compatibility_only",
        "unresolved_items": [
            *FULL_PARITY_COVERAGE_BACKLOG,
            *FULL_PARITY_EVIDENCE_BACKLOG,
        ],
    }
    spec["evidence_contract"]["receipts"] = []
    if tuple(spec["evidence_contract"]["required_roles"]) != EXPECTED_REQUIRED_ROLES:
        raise ValueError("baseline required QualificationSpec V2 roles changed")

    profiles = {
        row["id"]: row for row in spec["scenario_contract"]["complexity_profiles"]
    }
    profiles["maximum_axis"]["workload"].update(
        {"rows": 100_000, "indicators": 300, "constructs": 100}
    )
    profiles["compound_stress"]["workload"].update(
        {"rows": 100_000, "indicators": 300, "constructs": 100}
    )
    spec["scenario_contract"]["monte_carlo_policy"]["maximum_half_width"] = 0.01
    for output in spec["comparison_contract"]["outputs"]:
        if output["output_id"] == "recovery_coverage":
            output["maximum_half_width"] = 0.01

    performance = spec["operational_contract"]["performance"]
    performance["hardware_classes"] = [
        {
            "id": "windows_standard_6c16g",
            "os_family": "windows",
            "architecture": "x86_64",
            "minimum_logical_cores": 6,
            "minimum_memory_gib": 16,
            "notes": "Product-finalization Standard reference hardware; not a customer minimum-system claim.",
        }
    ]
    performance["baseline_policy"].update(
        {
            "warmup_runs": 1,
            "measured_runs": 5,
            "maximum_runtime_regression_percent": 20,
            "maximum_memory_regression_percent": 20,
        }
    )
    for budget in performance["budgets"]:
        budget["hardware_class_id"] = "windows_standard_6c16g"
        budget["maximum_peak_working_set_bytes"] = min(
            budget["maximum_peak_working_set_bytes"], 12 * 1024**3
        )
    archive = spec["operational_contract"]["archive"]
    archive.update(
        {
            "current_schema_version": 6,
            "readable_schema_versions": [1, 2, 3, 4, 5, 6],
            "writable_schema_versions": [6],
        }
    )
    spec["operational_contract"]["export"]["canonical_projection_id"] = (
        "canonical_result_document_v2.pls_algorithm"
    )
    return spec


def _registry_cell() -> tuple[dict[str, Any], dict[str, Any]]:
    registry = strict_load_json(REGISTRY_PATH)
    for capability in registry["capabilities"]:
        if capability["capability_id"] != CAPABILITY_ID:
            continue
        for cell in capability["option_cells"]:
            if cell["cell_id"] == CELL_ID:
                return capability, cell
    raise ValueError("PLS algorithm option cell missing from CapabilityRegistryV2")


def _hardware_fingerprint() -> dict[str, Any]:
    memory_gib = 0.0
    if os.name == "nt":
        import ctypes

        class MemoryStatus(ctypes.Structure):
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

        status = MemoryStatus()
        status.length = ctypes.sizeof(status)
        if ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(status)):
            memory_gib = status.total_physical / 1024**3
    return {
        "os": platform.platform(),
        "architecture": platform.machine() or "unknown",
        "cpu": platform.processor() or "unknown",
        "logical_cores": os.cpu_count() or 1,
        "memory_gib": round(memory_gib, 3),
    }


def describe_internal_product_harness() -> dict[str, Any]:
    """Describe the only permitted current-product execution seam.

    This source-contract factory never builds or executes the harness. The
    separate orchestrator performs a source-bound build, selects the exact lib
    test executable from Cargo JSON, binds its bytes, and directly invokes the
    ignored test. A customer CLI binary is deliberately not part of this seam.
    """

    return {
        "schema_version": 1,
        "surface": "internal_labs_ignored_lib_test",
        "package": INTERNAL_HARNESS_PACKAGE,
        "target": INTERNAL_HARNESS_TARGET,
        "test": INTERNAL_HARNESS_TEST,
        "orchestrator": INTERNAL_HARNESS_ORCHESTRATOR,
        "product_execution_performed_by_factory": False,
        "customer_cli_invoked": False,
        "customer_registry_admission_invoked": False,
        "source_bound_build_required": True,
        "self_executable_sha256_required": True,
        "candidate_receipt_output_allowed": False,
        "qualification_ready": False,
        "product_source_artifacts": source_descriptors(PRODUCT_BUILD_SOURCE_PATHS),
    }


def _run_command(
    command: Sequence[str], *, env: dict[str, str] | None = None, timeout: int = 600
) -> tuple[subprocess.CompletedProcess[str], dict[str, Any]]:
    started_at = datetime.now(timezone.utc)
    started = time.monotonic()
    completed = subprocess.run(
        list(command),
        cwd=ROOT,
        env={**os.environ, **(env or {})},
        capture_output=True,
        text=True,
        timeout=timeout,
        check=False,
    )
    finished_at = datetime.now(timezone.utc)
    return completed, {
        "command": list(command),
        "returncode": completed.returncode,
        "started_at_utc": started_at.isoformat().replace("+00:00", "Z"),
        "finished_at_utc": finished_at.isoformat().replace("+00:00", "Z"),
        "duration_seconds": round(time.monotonic() - started, 6),
        "stdout_sha256": hashlib.sha256(completed.stdout.encode()).hexdigest(),
        "stderr_sha256": hashlib.sha256(completed.stderr.encode()).hexdigest(),
    }


def _read_rows(path: Path) -> tuple[list[str], list[list[float]]]:
    with path.open(newline="", encoding="utf-8-sig") as handle:
        rows = list(csv.DictReader(handle))
    if not rows:
        raise ValueError(f"empty external data: {path}")
    variables = list(rows[0])
    return variables, [[float(row[name]) for name in variables] for row in rows]


def _oracle_values(result: dict[str, Any]) -> dict[tuple[str, str, str, str], float]:
    values: dict[tuple[str, str, str, str], float] = {}
    for row in result["paths"]:
        values[("path", row["source"], row["target"], "")] = row["coefficient"]
    for row in result["outer_estimates"]:
        key = (row["construct"], "", row["indicator"])
        values[("loading", *key)] = row["loading"]
        values[("weight", *key)] = row["weight"]
    return values


def _read_csem(path: Path) -> dict[str, dict[tuple[str, str, str, str], float]]:
    output: dict[str, dict[tuple[str, str, str, str], float]] = {}
    with path.open(newline="", encoding="utf-8-sig") as handle:
        for row in csv.DictReader(handle):
            variant = row.get("variant") or "PUBLISHED_PATH_MODE_A"
            key = (row["kind"], row["source"], row["target"], row["indicator"])
            if key in output.setdefault(variant, {}):
                raise ValueError(f"duplicate cSEM key: {variant} {key}")
            output[variant][key] = float(row["value"])
    return output


def _read_python_plspm(path: Path) -> dict[str, dict[tuple[str, str, str, str], float]]:
    document = strict_load_json(path)
    output: dict[str, dict[tuple[str, str, str, str], float]] = {}
    for variant in document["variants"]:
        values: dict[tuple[str, str, str, str], float] = {}
        for row in variant["paths"]:
            values[("path", row["source"], row["target"], "")] = float(row["value"])
        for row in variant["outer"]:
            values[("loading", row["construct"], "", row["indicator"])] = float(
                row["loading"]
            )
        output[variant["variant"]] = values
    return output


def _compare(
    engine: str,
    variant: str,
    actual: dict[tuple[str, str, str, str], float],
    expected: dict[tuple[str, str, str, str], float],
    *,
    tolerance: float = 1.0e-6,
) -> dict[str, Any]:
    shared = set(actual) & set(expected)
    differences = [abs(actual[key] - expected[key]) for key in shared]
    exact = set(actual) == set(expected)
    return {
        "engine": engine,
        "variant": variant,
        "passed": exact and bool(shared) and max(differences) <= tolerance,
        "exact_key_membership": exact,
        "compared_quantity_count": len(shared),
        "max_abs_difference": max(differences, default=None),
        "tolerance": tolerance,
        "missing_from_oracle": [
            list(key) for key in sorted(set(expected) - set(actual))
        ],
        "unexpected_from_oracle": [
            list(key) for key in sorted(set(actual) - set(expected))
        ],
    }


def run_external_reference_work(work_root: Path) -> dict[str, Any]:
    external = work_root / "external"
    external.mkdir(parents=True, exist_ok=False)
    executions: list[dict[str, Any]] = []

    resolved, record = _run_command(
        [
            "powershell",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            "validation/Resolve-Rscript.ps1",
        ],
        timeout=120,
    )
    executions.append(record)
    lines = [line.strip() for line in resolved.stdout.splitlines() if line.strip()]
    if resolved.returncode != 0 or len(lines) != 1 or not Path(lines[0]).is_file():
        raise RuntimeError("Rscript resolution failed or was ambiguous")
    rscript = lines[0]
    r_env = {"R_LIBS_USER": str((VALIDATION / "r-library").resolve())}
    csem_simple = external / "csem_simple.csv"
    csem_published_data = external / "csem_threecommonfactors.data.csv"
    csem_published = external / "csem_threecommonfactors.csv"
    python_plspm = external / "python_plspm.json"
    commands = [
        (
            [
                rscript,
                "--vanilla",
                str(VALIDATION / "r_csem_reference.R"),
                str(VALIDATION / "fixtures" / "simple_reflective.csv"),
                str(csem_simple),
            ],
            r_env,
        ),
        (
            [
                rscript,
                "--vanilla",
                str(VALIDATION / "r_csem_threecommonfactors_reference.R"),
                str(csem_published_data),
                str(csem_published),
            ],
            r_env,
        ),
        (
            [
                str(VALIDATION / ".venv" / "Scripts" / "python.exe"),
                "validation/python_plspm_reference.py",
                str(python_plspm),
            ],
            None,
        ),
    ]
    for command, environment in commands:
        completed, record = _run_command(command, env=environment, timeout=600)
        executions.append(record)
        if completed.returncode != 0:
            raise RuntimeError(f"external reference failed: {command}")

    variables, rows = _read_rows(VALIDATION / "fixtures" / "simple_reflective.csv")
    paths = [oracle.OraclePath("x", "y")]
    variants = {
        "PATH_MODE_A": ("path", "mode_a"),
        "MODE_B": ("path", "mode_b"),
        "FACTOR": ("factor", "mode_a"),
        "PCA": ("pca", "mode_a"),
    }
    csem = _read_csem(csem_simple)
    plspm = _read_python_plspm(python_plspm)
    comparisons: list[dict[str, Any]] = []
    for label, (scheme, mode) in variants.items():
        result = oracle.estimate_pls(
            rows,
            variables,
            [
                oracle.OracleConstruct("x", ("x1", "x2"), mode),
                oracle.OracleConstruct("y", ("y1", "y2"), mode),
            ],
            paths,
            weighting_scheme=scheme,
            tolerance=1.0e-12,
            max_iterations=10_000,
        )
        values = _oracle_values(result)
        comparisons.append(_compare("R cSEM 0.6.1", label, values, csem[label]))
        if label in plspm:
            keys = set(plspm[label])
            comparisons.append(
                _compare(
                    "python-plspm 0.5.7",
                    label,
                    {key: value for key, value in values.items() if key in keys},
                    plspm[label],
                )
            )

    variables, rows = _read_rows(csem_published_data)
    published = oracle.estimate_pls(
        rows,
        variables,
        [
            oracle.OracleConstruct("eta1", ("y11", "y12", "y13")),
            oracle.OracleConstruct("eta2", ("y21", "y22", "y23")),
            oracle.OracleConstruct("eta3", ("y31", "y32", "y33")),
        ],
        [
            oracle.OraclePath("eta1", "eta2"),
            oracle.OraclePath("eta1", "eta3"),
            oracle.OraclePath("eta2", "eta3"),
        ],
        tolerance=1.0e-12,
        max_iterations=10_000,
    )
    published_reference = next(iter(_read_csem(csem_published).values()))
    comparisons.append(
        _compare(
            "R cSEM 0.6.1 published threecommonfactors",
            "PUBLISHED_PATH_MODE_A",
            _oracle_values(published),
            published_reference,
        )
    )
    artifacts = [csem_simple, csem_published_data, csem_published, python_plspm]
    return {
        "schema_version": 1,
        "report_kind": "pls_algorithm_v1_external_oracle_work_v1",
        "passed": len(comparisons) == 8 and all(row["passed"] for row in comparisons),
        "work_evidence_only": True,
        "qualification_ready": False,
        "promotion_requested": False,
        "independence": {
            "primary_method_source": "Wold (1982) and Lohmoller (1989)",
            "computational_groups": ["r_csem_0_6_1", "python_plspm_0_5_7"],
            "runtime_policy": "development_validation_only_never_packaged",
            "quickpls_product_executed": False,
        },
        "comparisons": comparisons,
        "executions": executions,
        "artifacts": [
            {
                "path": repository_path(path),
                "size_bytes": path.stat().st_size,
                "sha256": sha256_file(path),
            }
            for path in artifacts
        ],
    }


def _snapshot_paths(source_hash: str) -> dict[str, Path]:
    root = RESULT_ROOT / f"source_{source_hash}"
    return {
        "root": root,
        "work": root / "work",
        "oracle": root / "work" / "transparent_numpy_oracle.json",
        "external": root / "work" / "independent_external_oracles.json",
        "receipt": root / "candidate_receipts" / "method_contract.qualification.json",
        "audit": root / "qualification_factory_audit.json",
    }


def _descriptor(path: Path) -> dict[str, Any]:
    return {
        "path": repository_path(path),
        "size_bytes": path.stat().st_size,
        "sha256": sha256_file(path),
    }


def _is_sha256(value: Any) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdef" for character in value)
    )


def _require_exact_keys(
    value: Any, expected: Iterable[str], *, subject: str
) -> Mapping[str, Any]:
    if not isinstance(value, Mapping):
        raise ValueError(f"{subject} must be an object")
    expected_keys = set(expected)
    observed_keys = set(value)
    if observed_keys != expected_keys:
        missing = sorted(expected_keys - observed_keys)
        unknown = sorted(observed_keys - expected_keys)
        raise ValueError(
            f"{subject} keys are not exact; missing={missing}, unknown={unknown}"
        )
    return value


def _require_exact_list(value: Any, *, length: int, subject: str) -> list[Any]:
    if not isinstance(value, list) or len(value) != length:
        raise ValueError(f"{subject} must contain exactly {length} items")
    return value


def _require_uuid(value: Any, *, subject: str) -> str:
    if not isinstance(value, str):
        raise ValueError(f"{subject} must be a UUID string")
    try:
        parsed = uuid.UUID(value)
    except (ValueError, AttributeError) as error:
        raise ValueError(f"{subject} is not a UUID") from error
    if str(parsed) != value:
        raise ValueError(f"{subject} is not a canonical lowercase UUID")
    return value


def _require_finite_number(value: Any, *, subject: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ValueError(f"{subject} must be numeric")
    numeric = float(value)
    if not math.isfinite(numeric):
        raise ValueError(f"{subject} must be finite")
    return numeric


def _serialized_json_sha256(value: Any) -> str:
    """Match serde_json's compact field-order-preserving hashing for receipts/plans."""

    payload = json.dumps(
        value,
        ensure_ascii=False,
        allow_nan=False,
        separators=(",", ":"),
    ).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def _require_artifact_descriptor_shape(value: Any, *, subject: str) -> None:
    descriptor = _require_exact_keys(
        value, {"path", "size_bytes", "sha256"}, subject=subject
    )
    if (
        not isinstance(descriptor["path"], str)
        or not descriptor["path"]
        or isinstance(descriptor["size_bytes"], bool)
        or not isinstance(descriptor["size_bytes"], int)
        or descriptor["size_bytes"] < 0
        or not _is_sha256(descriptor["sha256"])
    ):
        raise ValueError(f"{subject} descriptor shape is invalid")


def _resolve_artifact_descriptor(root: Path, descriptor: Any, *, subject: str) -> Path:
    _require_artifact_descriptor_shape(descriptor, subject=subject)
    if not isinstance(descriptor, Mapping):
        raise ValueError(f"{subject} descriptor is missing")
    relative = descriptor.get("path")
    if (
        not isinstance(relative, str)
        or not relative
        or "\\" in relative
        or Path(relative).is_absolute()
        or ".." in Path(relative).parts
    ):
        raise ValueError(f"{subject} descriptor path is unsafe")
    resolved_root = root.resolve(strict=True)
    lexical = resolved_root / relative
    current = lexical
    while current != resolved_root:
        metadata = current.lstat()
        if (
            metadata.st_file_attributes & 0x400
            if os.name == "nt"
            else current.is_symlink()
        ):
            raise ValueError(f"{subject} artifact path contains a symlink")
        current = current.parent
    resolved = lexical.resolve(strict=True)
    try:
        resolved.relative_to(resolved_root)
    except ValueError as error:
        raise ValueError(f"{subject} artifact escapes its work root") from error
    if not resolved.is_file():
        raise ValueError(f"{subject} artifact is not a regular file")
    if descriptor.get("size_bytes") != resolved.stat().st_size:
        raise ValueError(f"{subject} artifact size mismatch")
    if descriptor.get("sha256") != sha256_file(resolved):
        raise ValueError(f"{subject} artifact digest mismatch")
    return resolved


def _current_product_capability_cell() -> dict[str, Any]:
    return {
        "registry_schema_version": 2,
        "capability_id": CAPABILITY_ID,
        "cell_id": CELL_ID,
        "capability_version": METHOD_VERSION,
    }


def _variant_ordinal(variant: str) -> int:
    try:
        return list(CURRENT_PRODUCT_VARIANTS).index(variant)
    except ValueError as error:
        raise ValueError(f"unknown current-product variant: {variant}") from error


def _current_product_adapter_version(variant: str) -> str:
    _variant_ordinal(variant)
    return CURRENT_PRODUCT_ADAPTER_VERSIONS[variant]


def _current_product_requires_convergence_receipt(variant: str) -> bool:
    weighting, _ = CURRENT_PRODUCT_VARIANTS[variant]
    return weighting in {"path", "factor"}


def _expected_point_estimate_attribution(preprocessing: str) -> dict[str, str]:
    indicator_scales = {
        "standardized": ("sample_mean", "sample_standard_deviation"),
        "mean_centered": ("sample_mean", "unit_scale"),
        "unstandardized": ("no_centering", "unit_scale"),
    }
    try:
        indicator_centering, indicator_scaling = indicator_scales[preprocessing]
    except KeyError as error:
        raise ValueError(
            f"unsupported current-product preprocessing attribution: {preprocessing}"
        ) from error
    return {
        "contract_version": "pls_point_estimate_attribution_v1",
        "preprocessing": preprocessing,
        "indicator_centering": indicator_centering,
        "indicator_scaling": indicator_scaling,
        "outer_weights": "preprocessed_indicator_to_unit_variance_construct_score",
        "outer_loadings": "indicator_construct_score_correlation",
        "construct_scores": "zero_mean_unit_variance_construct_score",
        "structural_paths": "standardized_construct_score_regression",
        "effects": "standardized_structural_path_decomposition",
    }


def _expected_current_product_model(variant: str, dataset_id: str) -> dict[str, Any]:
    index = _variant_ordinal(variant)
    _, mode = CURRENT_PRODUCT_VARIANTS[variant]
    model_id = str(uuid.UUID(int=0x1000 + index))
    variables: list[dict[str, Any]] = [
        {
            "kind": "composite",
            "id": "construct:x",
            "label": "X",
            "weighting": {"kind": mode},
        },
        {
            "kind": "composite",
            "id": "construct:y",
            "label": "Y",
            "weighting": {"kind": mode},
        },
    ]
    variables.extend(
        {
            "kind": "observed",
            "id": f"observed:{indicator}",
            "label": indicator,
            "source_column": indicator,
            "scale": "continuous",
            "role": "indicator",
            "categories": [],
            "value_labels": {},
            "missing_markers": [],
            "transformation_lineage": [],
        }
        for indicator in ("x1", "x2", "y1", "y2")
    )
    relations: list[dict[str, Any]] = []
    parameters: list[dict[str, Any]] = []
    for construct, label, indicator, suffix in (
        ("x", "X", "x1", "78_7831"),
        ("x", "X", "x2", "78_7832"),
        ("y", "Y", "y1", "79_7931"),
        ("y", "Y", "y2", "79_7932"),
    ):
        parameter_id = f"parameter_{suffix}"
        if mode == "mode_a":
            relations.append(
                {
                    "kind": "measurement_effect",
                    "id": f"measurement_{suffix}",
                    "construct": f"construct:{construct}",
                    "indicator": f"observed:{indicator}",
                    "parameter": parameter_id,
                }
            )
            target = {
                "kind": "loading",
                "construct": f"construct:{construct}",
                "indicator": f"observed:{indicator}",
            }
            parameter_label = f"{label} -> {indicator}"
        else:
            relations.append(
                {
                    "kind": "measurement_causal",
                    "id": f"measurement_{suffix}",
                    "indicator": f"observed:{indicator}",
                    "composite": f"construct:{construct}",
                    "parameter": parameter_id,
                }
            )
            target = {
                "kind": "weight",
                "indicator": f"observed:{indicator}",
                "composite": f"construct:{construct}",
            }
            parameter_label = f"{indicator} -> {label}"
        parameters.append(
            {
                "kind": "free",
                "id": parameter_id,
                "label": parameter_label,
                "target": target,
                "start": None,
                "lower": None,
                "upper": None,
                "equality_label": None,
                "group_overrides": [],
            }
        )
    relations.append(
        {
            "kind": "structural",
            "id": "structural_78_79",
            "source": "construct:x",
            "target": "construct:y",
            "parameter": "regression_78_79",
            "intercept_parameter": None,
        }
    )
    parameters.append(
        {
            "kind": "free",
            "id": "regression_78_79",
            "label": "x -> y",
            "target": {
                "kind": "regression",
                "source": "construct:x",
                "target": "construct:y",
            },
            "start": None,
            "lower": None,
            "upper": None,
            "equality_label": None,
            "group_overrides": [],
        }
    )
    return {
        "schema_version": 4,
        "id": model_id,
        "name": f"PLS current-product {variant}",
        "variables": variables,
        "relations": relations,
        "parameters": parameters,
        "constraints": [],
        "derived_terms": [],
        "group": {"kind": "single_group"},
        "data_binding": {
            "kind": "raw",
            "dataset_id": dataset_id,
            "missing_data": "listwise_deletion",
            "weight": None,
            "cluster_variable": None,
            "strata_variable": None,
        },
        "annotations": [],
        "presentation": {"kind": "none"},
    }


def _validate_current_product_request(request: Any, *, variant: str) -> dict[str, Any]:
    request = _require_exact_keys(
        request,
        {
            "surface",
            "experimentalLabsEnabled",
            "residentData",
            "datasetId",
            "datasetFingerprint",
            "recipe",
            "model",
            "compilerTarget",
            "capabilityCell",
        },
        subject=f"{variant} request",
    )
    dataset_id = _require_uuid(request["datasetId"], subject=f"{variant} dataset id")
    if (
        request["surface"] != "internal_labs"
        or request["experimentalLabsEnabled"] is not True
        or request["residentData"] != "project_resident"
        or request["datasetFingerprint"] != CURRENT_PRODUCT_DATASET_FINGERPRINT
        or request["compilerTarget"] != CURRENT_PRODUCT_COMPILER_TARGET
        or request["capabilityCell"] != _current_product_capability_cell()
    ):
        raise ValueError(
            f"{variant} request surface/dataset/compiler identity is invalid"
        )
    index = _variant_ordinal(variant)
    weighting, _ = CURRENT_PRODUCT_VARIANTS[variant]
    model = request["model"]
    expected_model = _expected_current_product_model(variant, dataset_id)
    if model != expected_model:
        raise ValueError(f"{variant} SemModelV4 is not the exact harness model")
    recipe = _require_exact_keys(
        request["recipe"],
        {
            "schema_version",
            "id",
            "created_at",
            "dataset_fingerprint",
            "model_binding",
            "estimand_confirmation",
            "settings",
            "method_config",
            "metadata",
            "legacy_source",
        },
        subject=f"{variant} Recipe-v4",
    )
    recipe_id = str(uuid.UUID(int=0x2000 + index))
    model_digest = recipe.get("model_binding", {}).get("scientific_sha256")
    expected_settings = {
        "method": "pls_pm",
        "weighting_scheme": weighting,
        "tolerance": 1.0e-7,
        "max_iterations": 3000,
        "bootstrap_samples": 0,
        "studentized_inner_samples": 0,
        "permutation_samples": 0,
        "seed": 20_260_815,
        "workers": 1,
        "confidence_level": 0.95,
        "preprocessing": "standardized",
        "missing_data": "listwise_deletion",
        "case_weight_column": None,
    }
    if (
        recipe["schema_version"] != 4
        or recipe["id"] != recipe_id
        or recipe["created_at"] != f"2026-08-15T00:00:0{index}Z"
        or recipe["dataset_fingerprint"] != CURRENT_PRODUCT_DATASET_FINGERPRINT
        or recipe["model_binding"]
        != {
            "kind": "project_sem_model_v4_reference",
            "model_id": expected_model["id"],
            "scientific_sha256": model_digest,
        }
        or not _is_sha256(model_digest)
        or _serialized_json_sha256(model) != model_digest
        or recipe["estimand_confirmation"] != "confirmed_composite"
        or recipe["settings"] != expected_settings
        or recipe["method_config"] != {"kind": "pls_algorithm"}
        or recipe["metadata"]
        != {"qualification_use": "current_product_work_evidence_only"}
    ):
        raise ValueError(f"{variant} Recipe-v4 identity/settings are invalid")
    legacy = _require_exact_keys(
        recipe["legacy_source"],
        {"source_schema_version", "source_recipe_sha256"},
        subject=f"{variant} legacy Recipe identity",
    )
    if legacy["source_schema_version"] != 3 or not _is_sha256(
        legacy["source_recipe_sha256"]
    ):
        raise ValueError(f"{variant} legacy Recipe identity is invalid")
    return {
        "request": request,
        "dataset_id": dataset_id,
        "dataset_fingerprint": CURRENT_PRODUCT_DATASET_FINGERPRINT,
        "model": model,
        "model_id": expected_model["id"],
        "model_digest": model_digest,
        "recipe": recipe,
        "recipe_id": recipe_id,
    }


def _validate_current_product_compiled(
    compiled: Any, *, variant: str, identity: Mapping[str, Any]
) -> Mapping[str, Any]:
    compiled = _require_exact_keys(
        compiled, {"plan", "receipt"}, subject=f"{variant} compiled artifact"
    )
    wrapper = _require_exact_keys(
        compiled["plan"], {"kind", "plan"}, subject=f"{variant} compiled plan"
    )
    plan = _require_exact_keys(
        wrapper["plan"],
        {"model_id", "scientific_hash", "dataset_id", "blocks", "paths"},
        subject=f"{variant} compiled PLS plan",
    )
    _, mode = CURRENT_PRODUCT_VARIANTS[variant]
    expected_blocks = []
    for construct, indicators in (("x", ("x1", "x2")), ("y", ("y1", "y2"))):
        expected_blocks.append(
            {
                "construct_id": f"construct:{construct}",
                "mode": mode,
                "indicators": [
                    {
                        "variable_id": f"observed:{indicator}",
                        "source_column": indicator,
                        "parameter_id": (
                            "parameter_78_7831"
                            if indicator == "x1"
                            else "parameter_78_7832"
                            if indicator == "x2"
                            else "parameter_79_7931"
                            if indicator == "y1"
                            else "parameter_79_7932"
                        ),
                    }
                    for indicator in indicators
                ],
            }
        )
    if (
        wrapper["kind"] != CURRENT_PRODUCT_COMPILER_TARGET
        or plan["model_id"] != identity["model_id"]
        or plan["scientific_hash"] != identity["model_digest"]
        or plan["dataset_id"] != identity["dataset_id"]
        or plan["blocks"] != expected_blocks
        or plan["paths"]
        != [
            {
                "relation_id": "structural_78_79",
                "source": "construct:x",
                "target": "construct:y",
                "parameter_id": "regression_78_79",
            }
        ]
    ):
        raise ValueError(f"{variant} compiled PLS plan is not exact")
    receipt = _require_exact_keys(
        compiled["receipt"],
        {
            "schema_version",
            "recipe_id",
            "recipe_document_sha256",
            "recipe_analytical_sha256",
            "model_id",
            "model_document_sha256",
            "model_scientific_sha256",
            "dataset_fingerprint",
            "compiler_target",
            "compiler_version",
            "capability_cell",
            "plan_sha256",
            "analytical_identity_sha256",
        },
        subject=f"{variant} compilation receipt",
    )
    if (
        receipt["schema_version"] != 1
        or receipt["recipe_id"] != identity["recipe_id"]
        or not _is_sha256(receipt["recipe_document_sha256"])
        or not _is_sha256(receipt["recipe_analytical_sha256"])
        or receipt["model_id"] != identity["model_id"]
        or receipt["model_document_sha256"] != identity["model_digest"]
        or receipt["model_scientific_sha256"] != identity["model_digest"]
        or receipt["dataset_fingerprint"] != identity["dataset_fingerprint"]
        or receipt["compiler_target"] != CURRENT_PRODUCT_COMPILER_TARGET
        or receipt["compiler_version"] != CURRENT_PRODUCT_COMPILER_VERSION
        or receipt["capability_cell"] != _current_product_capability_cell()
        or receipt["plan_sha256"] != _serialized_json_sha256(wrapper)
        or not _is_sha256(receipt["analytical_identity_sha256"])
    ):
        raise ValueError(f"{variant} compilation receipt identity is invalid")
    return receipt


def _expected_current_product_quantities(variant: str) -> dict[tuple[str, ...], float]:
    fixture = VALIDATION / "fixtures" / "simple_reflective.csv"
    with fixture.open(newline="", encoding="utf-8-sig") as handle:
        source_rows = list(csv.DictReader(handle))
    if not source_rows:
        raise ValueError("current-product oracle fixture is empty")
    variables = list(source_rows[0])
    rows = [[float(row[name]) for name in variables] for row in source_rows]
    weighting, mode = CURRENT_PRODUCT_VARIANTS[variant]
    expected = oracle.estimate_pls(
        rows,
        variables,
        [
            oracle.OracleConstruct("x", ("x1", "x2"), mode),
            oracle.OracleConstruct("y", ("y1", "y2"), mode),
        ],
        [oracle.OraclePath("x", "y")],
        weighting_scheme=weighting,
    )
    quantities: dict[tuple[str, ...], float] = {}
    for row in expected["outer_estimates"]:
        construct = f"construct:{row['construct']}"
        quantities[("weight", construct, str(row["indicator"]))] = float(row["weight"])
        quantities[("loading", construct, str(row["indicator"]))] = float(
            row["loading"]
        )
    for row in expected["paths"]:
        quantities[
            ("path", f"construct:{row['source']}", f"construct:{row['target']}")
        ] = float(row["coefficient"])
    return quantities


def _validate_current_product_point_estimate_attribution(
    value: Any, *, variant: str, identity: Mapping[str, Any]
) -> Mapping[str, Any]:
    expected = _expected_point_estimate_attribution(
        str(identity["recipe"]["settings"]["preprocessing"])
    )
    attribution = _require_exact_keys(
        value,
        expected,
        subject=f"{variant} point-estimate attribution",
    )
    if attribution != expected:
        raise ValueError(
            f"{variant} point-estimate attribution differs from its Recipe-v4 preprocessing"
        )
    return attribution


def _validate_current_product_convergence_receipt(
    value: Any,
    *,
    variant: str,
    identity: Mapping[str, Any],
    iterations: int,
) -> Mapping[str, Any]:
    receipt = _require_exact_keys(
        value,
        {
            "contract_version",
            "weighting_scheme",
            "maximum_iterations",
            "stop_criterion",
            "comparison",
            "performed_iterations",
            "estimated_block_updates",
            "termination_reason",
            "final_max_outer_weight_change",
            "blocks",
        },
        subject=f"{variant} algorithm convergence receipt",
    )
    settings = identity["recipe"]["settings"]
    weighting, mode = CURRENT_PRODUCT_VARIANTS[variant]
    final_change = _require_finite_number(
        receipt["final_max_outer_weight_change"],
        subject=f"{variant} final outer-weight change",
    )
    integer_fields = (
        "maximum_iterations",
        "performed_iterations",
        "estimated_block_updates",
    )
    if (
        any(
            isinstance(receipt[field], bool)
            or not isinstance(receipt[field], int)
            or receipt[field] < 1
            for field in integer_fields
        )
        or receipt["contract_version"] != "pls_algorithm_convergence_receipt_v1"
        or receipt["weighting_scheme"] != weighting
        or receipt["maximum_iterations"] != settings["max_iterations"]
        or receipt["stop_criterion"] != settings["tolerance"]
        or receipt["comparison"] != "less_than_or_equal"
        or receipt["performed_iterations"] != iterations
        or receipt["estimated_block_updates"] != iterations * 2
        or receipt["termination_reason"] != "converged_tolerance"
        or final_change < 0.0
        or final_change > settings["tolerance"]
    ):
        raise ValueError(f"{variant} algorithm convergence receipt is invalid")
    blocks = _require_exact_list(
        receipt["blocks"],
        length=2,
        subject=f"{variant} algorithm convergence blocks",
    )
    update_rule = "mode_a_covariance" if mode == "mode_a" else "mode_b_ols"
    expected_blocks = [
        ("construct:x", ["x1", "x2"]),
        ("construct:y", ["y1", "y2"]),
    ]
    for index, (block, (construct_id, indicator_order)) in enumerate(
        zip(blocks, expected_blocks, strict=True)
    ):
        block = _require_exact_keys(
            block,
            {"construct_id", "indicator_order", "update_rule", "initialization"},
            subject=f"{variant} algorithm convergence block {index}",
        )
        if block != {
            "construct_id": construct_id,
            "indicator_order": indicator_order,
            "update_rule": update_rule,
            "initialization": "standard_unit_weights",
        }:
            raise ValueError(
                f"{variant} algorithm convergence block order is invalid"
            )
    return receipt


def _validate_current_product_execution(
    execution: Any,
    *,
    variant: str,
    identity: Mapping[str, Any],
    receipt: Mapping[str, Any],
) -> dict[str, Any]:
    execution = _require_exact_keys(
        execution,
        {"schema_version", "provenance", "estimation"},
        subject=f"{variant} execution",
    )
    provenance = _require_exact_keys(
        execution["provenance"],
        {
            "adapter_version",
            "compilation_receipt",
            "projected_recipe_schema_version",
            "projected_recipe_sha256",
            "dataset_id",
            "estimator_method_version",
        },
        subject=f"{variant} execution provenance",
    )
    if (
        execution["schema_version"] != 1
        or provenance["adapter_version"] != _current_product_adapter_version(variant)
        or provenance["compilation_receipt"] != receipt
        or provenance["projected_recipe_schema_version"] != 3
        or not _is_sha256(provenance["projected_recipe_sha256"])
        or provenance["dataset_id"] != identity["dataset_id"]
        or provenance["estimator_method_version"] != METHOD_VERSION
    ):
        raise ValueError(f"{variant} execution provenance is invalid")
    estimation_keys = {
        "method_version",
        "converged",
        "iterations",
        "point_estimate_attribution",
        "used_observations",
        "omitted_observations",
        "transforms",
        "construct_scores",
        "outer_estimates",
        "paths",
        "control_estimates",
        "effects",
        "mediation",
        "plsc",
        "endogeneity",
        "nonlinear_effects",
        "moderated_mediation",
        "cta_pls",
        "wpls",
        "cca",
        "predict",
        "segmentation",
        "mga",
        "micom",
        "mga_permutation",
        "fimix",
        "ipma",
        "cbsem",
        "pca",
        "regression",
        "nca",
        "gsca",
        "r_squared",
        "warnings",
    }
    if _current_product_requires_convergence_receipt(variant):
        estimation_keys.add("algorithm_convergence_receipt")
    estimation = _require_exact_keys(
        execution["estimation"],
        estimation_keys,
        subject=f"{variant} estimation",
    )
    if (
        estimation["method_version"] != METHOD_VERSION
        or estimation["converged"] is not True
        or isinstance(estimation["iterations"], bool)
        or not isinstance(estimation["iterations"], int)
        or not 1 <= estimation["iterations"] <= 3000
        or estimation["used_observations"] != 6
        or estimation["omitted_observations"] != 0
        or estimation["control_estimates"] != []
        or estimation["warnings"] != []
    ):
        raise ValueError(f"{variant} legacy PLS estimation identity is invalid")
    _validate_current_product_point_estimate_attribution(
        estimation["point_estimate_attribution"], variant=variant, identity=identity
    )
    if _current_product_requires_convergence_receipt(variant):
        _validate_current_product_convergence_receipt(
            estimation["algorithm_convergence_receipt"],
            variant=variant,
            identity=identity,
            iterations=int(estimation["iterations"]),
        )
    for excluded in (
        "plsc",
        "endogeneity",
        "nonlinear_effects",
        "moderated_mediation",
        "cta_pls",
        "wpls",
        "cca",
        "predict",
        "segmentation",
        "mga",
        "micom",
        "mga_permutation",
        "fimix",
        "ipma",
        "cbsem",
        "pca",
        "regression",
        "nca",
        "gsca",
    ):
        if estimation[excluded] is not None:
            raise ValueError(f"{variant} unexpectedly executed {excluded}")
    transforms = _require_exact_list(
        estimation["transforms"], length=4, subject=f"{variant} transforms"
    )
    if [row.get("indicator") for row in transforms if isinstance(row, Mapping)] != [
        "x1",
        "x2",
        "y1",
        "y2",
    ]:
        raise ValueError(f"{variant} transform indicator membership is invalid")
    for index, transform in enumerate(transforms):
        transform = _require_exact_keys(
            transform,
            {"indicator", "mean", "scale"},
            subject=f"{variant} transform {index}",
        )
        _require_finite_number(transform["mean"], subject=f"{variant} transform mean")
        scale = _require_finite_number(
            transform["scale"], subject=f"{variant} transform scale"
        )
        if scale <= 0:
            raise ValueError(f"{variant} transform scale must be positive")
    scores = _require_exact_keys(
        estimation["construct_scores"],
        {"construct:x", "construct:y"},
        subject=f"{variant} construct scores",
    )
    for construct, values in scores.items():
        values = _require_exact_list(
            values, length=6, subject=f"{variant} {construct} scores"
        )
        for position, value in enumerate(values):
            _require_finite_number(
                value, subject=f"{variant} {construct} score {position}"
            )

    quantities: dict[tuple[str, ...], float] = {}
    outer_rows = _require_exact_list(
        estimation["outer_estimates"],
        length=4,
        subject=f"{variant} outer estimates",
    )
    expected_outer_membership = [
        ("construct:x", "x1"),
        ("construct:x", "x2"),
        ("construct:y", "y1"),
        ("construct:y", "y2"),
    ]
    observed_outer_membership = []
    for index, row in enumerate(outer_rows):
        row = _require_exact_keys(
            row,
            {"construct", "indicator", "weight", "loading"},
            subject=f"{variant} outer estimate {index}",
        )
        membership = (str(row["construct"]), str(row["indicator"]))
        observed_outer_membership.append(membership)
        quantities[("weight", *membership)] = _require_finite_number(
            row["weight"], subject=f"{variant} outer weight {membership}"
        )
        quantities[("loading", *membership)] = _require_finite_number(
            row["loading"], subject=f"{variant} outer loading {membership}"
        )
    if observed_outer_membership != expected_outer_membership:
        raise ValueError(f"{variant} outer quantity membership is invalid")
    path_rows = _require_exact_list(
        estimation["paths"], length=1, subject=f"{variant} paths"
    )
    path = _require_exact_keys(
        path_rows[0],
        {"source", "target", "coefficient"},
        subject=f"{variant} structural path",
    )
    if (path["source"], path["target"]) != ("construct:x", "construct:y"):
        raise ValueError(f"{variant} structural path identity is invalid")
    path_coefficient = _require_finite_number(
        path["coefficient"], subject=f"{variant} path coefficient"
    )
    quantities[("path", "construct:x", "construct:y")] = path_coefficient
    expected_quantities = _expected_current_product_quantities(variant)
    if (
        set(quantities) != set(expected_quantities)
        or max(abs(quantities[key] - expected_quantities[key]) for key in quantities)
        > 1.0e-6
    ):
        raise ValueError(f"{variant} stable product/oracle quantities are invalid")

    effects = _require_exact_list(
        estimation["effects"], length=1, subject=f"{variant} effects"
    )
    effect = _require_exact_keys(
        effects[0],
        {"source", "target", "direct", "indirect", "total"},
        subject=f"{variant} effect",
    )
    if (
        effect["source"] != "construct:x"
        or effect["target"] != "construct:y"
        or effect["direct"] != path["coefficient"]
        or effect["indirect"] != 0.0
        or effect["total"] != path["coefficient"]
    ):
        raise ValueError(f"{variant} effect does not match its structural path")
    mediation = _require_exact_keys(
        estimation["mediation"],
        {"method_version", "tolerance", "estimates", "warnings"},
        subject=f"{variant} mediation",
    )
    mediation_rows = _require_exact_list(
        mediation["estimates"], length=1, subject=f"{variant} mediation estimates"
    )
    mediation_row = _require_exact_keys(
        mediation_rows[0],
        {
            "source",
            "target",
            "direct",
            "indirect",
            "total",
            "variance_accounted_for",
            "classification",
            "warning",
        },
        subject=f"{variant} mediation estimate",
    )
    if (
        mediation["method_version"] != "pls_mediation_v1"
        or mediation["tolerance"] != 1.0e-12
        or not isinstance(mediation["warnings"], list)
        or mediation_row["source"] != "construct:x"
        or mediation_row["target"] != "construct:y"
        or mediation_row["direct"] != path["coefficient"]
        or mediation_row["indirect"] != 0.0
        or mediation_row["total"] != path["coefficient"]
        or mediation_row["variance_accounted_for"] != 0.0
        or mediation_row["classification"] != "direct_only"
        or not isinstance(mediation_row["warning"], str)
    ):
        raise ValueError(f"{variant} mediation projection is invalid")
    r_squared = _require_exact_keys(
        estimation["r_squared"],
        {"construct:y"},
        subject=f"{variant} R-squared",
    )
    r_squared_value = _require_finite_number(
        r_squared["construct:y"], subject=f"{variant} R-squared value"
    )
    if abs(r_squared_value - path_coefficient * path_coefficient) > 1.0e-12:
        raise ValueError(f"{variant} R-squared does not match the structural path")
    return {
        "execution": execution,
        "provenance": provenance,
        "estimation": estimation,
        "quantities": quantities,
    }


def _canonical_table_values(
    table: Any,
    *,
    variant: str,
    table_id: str,
    column_ids: Sequence[str],
    row_ids: Sequence[str],
) -> list[list[Any]]:
    table = _require_exact_keys(
        table,
        {
            "id",
            "title",
            "description",
            "columns",
            "rows",
            "footnote_ids",
            "capability_cells",
        },
        subject=f"{variant} canonical table {table_id}",
    )
    if (
        table["id"] != table_id
        or not isinstance(table["title"], str)
        or not isinstance(table["description"], str)
        or table["footnote_ids"] != []
        or table["capability_cells"] != [_current_product_capability_cell()]
    ):
        raise ValueError(f"{variant} canonical table {table_id} metadata is invalid")
    columns = _require_exact_list(
        table["columns"],
        length=len(column_ids),
        subject=f"{variant} canonical table {table_id} columns",
    )
    column_data_types: list[str] = []
    for index, (column, expected_id) in enumerate(
        zip(columns, column_ids, strict=True)
    ):
        if not isinstance(column, Mapping):
            raise ValueError(
                f"{variant} canonical table {table_id} column {index} is invalid"
            )
        column_keys = {"id", "label", "data_type", "description", "role"}
        if column.get("data_type") == "number":
            column_keys.add("default_precision")
        column = _require_exact_keys(
            column,
            column_keys,
            subject=f"{variant} canonical table {table_id} column {index}",
        )
        if (
            column["id"] != expected_id
            or not isinstance(column["label"], str)
            or not isinstance(column["data_type"], str)
            or not isinstance(column["description"], str)
            or not isinstance(column["role"], str)
            or (column["data_type"] == "number" and column["default_precision"] != 6)
        ):
            raise ValueError(
                f"{variant} canonical table {table_id} column identity is invalid"
            )
        column_data_types.append(str(column["data_type"]))
    rows = _require_exact_list(
        table["rows"],
        length=len(row_ids),
        subject=f"{variant} canonical table {table_id} rows",
    )
    values: list[list[Any]] = []
    for row_index, (row, expected_id) in enumerate(zip(rows, row_ids, strict=True)):
        row = _require_exact_keys(
            row,
            {"id", "cells"},
            subject=f"{variant} canonical table {table_id} row {row_index}",
        )
        cells = _require_exact_list(
            row["cells"],
            length=len(column_ids),
            subject=f"{variant} canonical table {table_id} row cells",
        )
        if row["id"] != expected_id:
            raise ValueError(
                f"{variant} canonical table {table_id} row identity is invalid"
            )
        row_values = []
        for cell_index, cell in enumerate(cells):
            cell = _require_exact_keys(
                cell,
                {"kind", "value"},
                subject=(
                    f"{variant} canonical table {table_id} row {row_index} "
                    f"cell {cell_index}"
                ),
            )
            if cell["kind"] not in {"boolean", "number", "text", "missing"}:
                raise ValueError(f"{variant} canonical cell kind is invalid")
            if cell["kind"] != column_data_types[cell_index]:
                raise ValueError(
                    f"{variant} canonical cell kind differs from its column"
                )
            if cell["kind"] == "number":
                _require_finite_number(
                    cell["value"], subject=f"{variant} canonical number"
                )
            row_values.append(cell["value"])
        values.append(row_values)
    return values


def _validate_current_product_canonical(
    canonical: Any,
    *,
    variant: str,
    project_id: str,
    identity: Mapping[str, Any],
    receipt: Mapping[str, Any],
    execution: Mapping[str, Any],
) -> dict[str, Any]:
    canonical = _require_exact_keys(
        canonical,
        {
            "schema_version",
            "document_id",
            "title",
            "provenance",
            "capability_cells",
            "sections",
            "tables",
            "charts",
            "notices",
            "exclusions",
            "footnotes",
            "presentation",
        },
        subject=f"{variant} canonical document",
    )
    index = _variant_ordinal(variant)
    run_id = str(uuid.UUID(int=0x3000 + index))
    document_id = f"result_{run_id}"
    provenance = _require_exact_keys(
        canonical["provenance"],
        {
            "run_id",
            "project_id",
            "model_id",
            "model_digest",
            "dataset_id",
            "dataset_fingerprint",
            "recipe_id",
            "recipe_digest",
            "capability_cell",
            "method_version",
            "engine_version",
            "seed",
            "workers",
            "started_at",
            "completed_at",
        },
        subject=f"{variant} canonical provenance",
    )
    if (
        canonical["schema_version"] != 2
        or canonical["document_id"] != document_id
        or canonical["title"] != "PLS-SEM results"
        or provenance
        != {
            "run_id": run_id,
            "project_id": project_id,
            "model_id": identity["model_id"],
            "model_digest": identity["model_digest"],
            "dataset_id": identity["dataset_id"],
            "dataset_fingerprint": CURRENT_PRODUCT_DATASET_FINGERPRINT.removeprefix(
                "v2:"
            ),
            "recipe_id": identity["recipe_id"],
            "recipe_digest": receipt["recipe_analytical_sha256"],
            "capability_cell": _current_product_capability_cell(),
            "method_version": METHOD_VERSION,
            "engine_version": _current_product_adapter_version(variant),
            "seed": 20_260_815,
            "workers": 1,
            "started_at": f"2026-08-15T00:00:{index * 2:02}Z",
            "completed_at": f"2026-08-15T00:00:{index * 2 + 1:02}Z",
        }
        or canonical["capability_cells"] != [_current_product_capability_cell()]
        or canonical["charts"] != []
        or canonical["notices"] != []
        or canonical["footnotes"] != []
    ):
        raise ValueError(f"{variant} canonical identity is invalid")
    sections = _require_exact_list(
        canonical["sections"], length=3, subject=f"{variant} canonical sections"
    )
    run_detail_table_ids = ["estimation_summary", "point_estimate_attribution"]
    if _current_product_requires_convergence_receipt(variant):
        run_detail_table_ids.extend(
            ["algorithm_convergence_receipt", "algorithm_block_order"]
        )
    expected_sections = [
        ("run_details", run_detail_table_ids),
        ("measurement_model", ["outer_model"]),
        ("structural_model", ["structural_paths", "effects", "r_squared"]),
    ]
    for position, (section, (section_id, table_ids)) in enumerate(
        zip(sections, expected_sections, strict=True)
    ):
        section = _require_exact_keys(
            section,
            {
                "id",
                "title",
                "description",
                "table_ids",
                "chart_ids",
                "capability_cells",
            },
            subject=f"{variant} canonical section {position}",
        )
        if (
            section["id"] != section_id
            or not isinstance(section["title"], str)
            or not isinstance(section["description"], str)
            or section["table_ids"] != table_ids
            or section["chart_ids"] != []
            or section["capability_cells"] != [_current_product_capability_cell()]
        ):
            raise ValueError(f"{variant} canonical section identity is invalid")
    expected_table_order = [
        "estimation_summary",
        "outer_model",
        "structural_paths",
        "effects",
        "r_squared",
        "point_estimate_attribution",
    ]
    if _current_product_requires_convergence_receipt(variant):
        expected_table_order.extend(
            ["algorithm_convergence_receipt", "algorithm_block_order"]
        )
    tables = _require_exact_list(
        canonical["tables"],
        length=len(expected_table_order),
        subject=f"{variant} canonical tables",
    )
    table_map = {
        table.get("id"): table for table in tables if isinstance(table, Mapping)
    }
    if (
        [table.get("id") for table in tables if isinstance(table, Mapping)]
        != expected_table_order
        or set(table_map) != set(expected_table_order)
        or len(table_map) != len(tables)
    ):
        raise ValueError(
            f"{variant} canonical table identity/score-execution absence is invalid"
        )
    estimation = execution["estimation"]
    summary_values = _canonical_table_values(
        table_map["estimation_summary"],
        variant=variant,
        table_id="estimation_summary",
        column_ids=(
            "converged",
            "iterations",
            "used_observations",
            "omitted_observations",
        ),
        row_ids=("run",),
    )
    if summary_values != [
        [
            estimation["converged"],
            float(estimation["iterations"]),
            float(estimation["used_observations"]),
            float(estimation["omitted_observations"]),
        ]
    ]:
        raise ValueError(
            f"{variant} canonical estimation summary differs from execution"
        )
    attribution = estimation["point_estimate_attribution"]
    attribution_values = _canonical_table_values(
        table_map["point_estimate_attribution"],
        variant=variant,
        table_id="point_estimate_attribution",
        column_ids=(
            "contract_version",
            "preprocessing",
            "indicator_centering",
            "indicator_scaling",
            "outer_weights",
            "outer_loadings",
            "construct_scores",
            "structural_paths",
            "effects",
        ),
        row_ids=("attribution",),
    )
    expected_attribution_values = [
        [
            attribution["contract_version"],
            attribution["preprocessing"],
            attribution["indicator_centering"],
            attribution["indicator_scaling"],
            attribution["outer_weights"],
            attribution["outer_loadings"],
            attribution["construct_scores"],
            attribution["structural_paths"],
            attribution["effects"],
        ]
    ]
    if attribution_values != expected_attribution_values:
        raise ValueError(
            f"{variant} canonical point-estimate attribution differs from execution"
        )
    if _current_product_requires_convergence_receipt(variant):
        convergence = estimation["algorithm_convergence_receipt"]
        convergence_values = _canonical_table_values(
            table_map["algorithm_convergence_receipt"],
            variant=variant,
            table_id="algorithm_convergence_receipt",
            column_ids=(
                "contract_version",
                "weighting_scheme",
                "maximum_iterations",
                "stop_criterion",
                "comparison",
                "performed_iterations",
                "estimated_block_updates",
                "termination_reason",
                "final_max_outer_weight_change",
            ),
            row_ids=("convergence",),
        )
        expected_convergence_values = [
            [
                convergence["contract_version"],
                convergence["weighting_scheme"],
                float(convergence["maximum_iterations"]),
                convergence["stop_criterion"],
                convergence["comparison"],
                float(convergence["performed_iterations"]),
                float(convergence["estimated_block_updates"]),
                convergence["termination_reason"],
                convergence["final_max_outer_weight_change"],
            ]
        ]
        if convergence_values != expected_convergence_values:
            raise ValueError(
                f"{variant} canonical algorithm convergence differs from execution"
            )
        block_rows = [
            (
                f"algorithm_block_{block_index:04}_indicator_{indicator_index:04}"
            )
            for block_index, block in enumerate(convergence["blocks"])
            for indicator_index, _ in enumerate(block["indicator_order"])
        ]
        block_values = _canonical_table_values(
            table_map["algorithm_block_order"],
            variant=variant,
            table_id="algorithm_block_order",
            column_ids=(
                "block_ordinal",
                "construct_id",
                "indicator_ordinal",
                "indicator_id",
                "update_rule",
                "initialization",
            ),
            row_ids=tuple(block_rows),
        )
        expected_block_values = [
            [
                float(block_index),
                block["construct_id"],
                float(indicator_index),
                indicator,
                block["update_rule"],
                block["initialization"],
            ]
            for block_index, block in enumerate(convergence["blocks"])
            for indicator_index, indicator in enumerate(block["indicator_order"])
        ]
        if block_values != expected_block_values:
            raise ValueError(
                f"{variant} canonical algorithm block order differs from execution"
            )
    outer_values = _canonical_table_values(
        table_map["outer_model"],
        variant=variant,
        table_id="outer_model",
        column_ids=("construct", "indicator", "weight", "loading"),
        row_ids=("outer_0000", "outer_0001", "outer_0002", "outer_0003"),
    )
    expected_outer_values = [
        [row["construct"], row["indicator"], row["weight"], row["loading"]]
        for row in estimation["outer_estimates"]
    ]
    if outer_values != expected_outer_values:
        raise ValueError(f"{variant} canonical stable outer quantities differ")
    path_values = _canonical_table_values(
        table_map["structural_paths"],
        variant=variant,
        table_id="structural_paths",
        column_ids=("source", "target", "coefficient"),
        row_ids=("path_0000",),
    )
    expected_path_values = [
        [row["source"], row["target"], row["coefficient"]]
        for row in estimation["paths"]
    ]
    if path_values != expected_path_values:
        raise ValueError(f"{variant} canonical stable path quantity differs")
    effect_values = _canonical_table_values(
        table_map["effects"],
        variant=variant,
        table_id="effects",
        column_ids=("source", "target", "direct", "indirect", "total"),
        row_ids=("effect_0000",),
    )
    expected_effect_values = [
        [
            row["source"],
            row["target"],
            row["direct"],
            row["indirect"],
            row["total"],
        ]
        for row in estimation["effects"]
    ]
    if effect_values != expected_effect_values:
        raise ValueError(f"{variant} canonical effects differ from execution")
    r_squared_values = _canonical_table_values(
        table_map["r_squared"],
        variant=variant,
        table_id="r_squared",
        column_ids=("construct", "r_squared"),
        row_ids=("r_squared_0000",),
    )
    if r_squared_values != [["construct:y", estimation["r_squared"]["construct:y"]]]:
        raise ValueError(f"{variant} canonical R-squared differs from execution")
    exclusions = _require_exact_list(
        canonical["exclusions"], length=1, subject=f"{variant} exclusions"
    )
    exclusion = _require_exact_keys(
        exclusions[0],
        {"id", "capability_cell", "title", "reason"},
        subject=f"{variant} point-estimation exclusion",
    )
    if (
        exclusion["id"] != "point_estimation_only"
        or exclusion["capability_cell"] != _current_product_capability_cell()
        or not isinstance(exclusion["title"], str)
        or not isinstance(exclusion["reason"], str)
    ):
        raise ValueError(f"{variant} point-estimation exclusion is invalid")
    if canonical["presentation"] != {
        "default_section_id": "structural_model",
        "default_table_id": "structural_paths",
        "precision": 4,
        "missing_value_label": "—",
        "chart_defaults": {},
    }:
        raise ValueError(f"{variant} canonical presentation contract is invalid")
    return {
        "canonical": canonical,
        "document_id": document_id,
        "run_id": run_id,
        "project_id": project_id,
    }


def _validate_current_product_archive_document(
    document: Any,
    *,
    subject: str,
    project_id: str,
    destination_archive_path: str,
    identities: Sequence[Mapping[str, Any]],
    canonicals: Sequence[Mapping[str, Any]],
    attachment_count: int,
    expected_attachment_digests: Mapping[str, str] | None = None,
) -> dict[str, str]:
    archive_keys = {
        "created_at",
        "datasets",
        "historical_recipes",
        "historical_results",
        "layouts",
        "models",
        "modified_at",
        "name",
        "origin",
        "project_id",
        "recipes",
        "schema_version",
    }
    if attachment_count > 0:
        archive_keys.add("canonical_result_documents")
    document = _require_exact_keys(
        document,
        archive_keys,
        subject=subject,
    )
    if (
        document["schema_version"] != 6
        or document["project_id"] != project_id
        or document["name"] != "PLS current-product work evidence"
        or not isinstance(document["created_at"], str)
        or not document["created_at"]
        or document["modified_at"] != "2026-08-15T00:00:00Z"
        or document["historical_recipes"] != []
        or document["historical_results"] != []
        or document["layouts"] != {}
    ):
        raise ValueError(f"{subject} project identity is invalid")
    if len(identities) != len(CURRENT_PRODUCT_VARIANTS) or len(canonicals) != len(
        CURRENT_PRODUCT_VARIANTS
    ):
        raise ValueError(f"{subject} validator identity breadth is incomplete")
    dataset_ids = {str(identity["dataset_id"]) for identity in identities}
    if len(dataset_ids) != 1:
        raise ValueError(f"{subject} variants do not share one resident dataset")
    dataset_id = next(iter(dataset_ids))
    datasets = _require_exact_list(
        document["datasets"], length=1, subject=f"{subject} datasets"
    )
    dataset = _require_exact_keys(
        datasets[0],
        {"fingerprint", "id", "name", "schema"},
        subject=f"{subject} dataset",
    )
    expected_column_schema = [
        {
            "column_type": "numeric",
            "label": None,
            "missing_markers": ["", "NA", "N/A", "."],
            "name": indicator,
            "scale_type": "continuous",
            "theoretical_max": None,
            "theoretical_min": None,
            "value_labels": {},
        }
        for indicator in ("x1", "x2", "y1", "y2")
    ]
    if (
        dataset["fingerprint"] != CURRENT_PRODUCT_DATASET_FINGERPRINT
        or dataset["id"] != dataset_id
        or dataset["name"] != "simple_reflective.csv"
        or dataset["schema"]
        != {
            "case_count": 6,
            "columns": expected_column_schema,
            "kind": "raw",
            "sample_size": None,
            "version": 1,
        }
    ):
        raise ValueError(f"{subject} resident dataset is invalid")
    models = _require_exact_list(
        document["models"], length=4, subject=f"{subject} models"
    )
    recipes = _require_exact_list(
        document["recipes"], length=4, subject=f"{subject} recipes"
    )
    for index, identity in enumerate(identities):
        model_record = _require_exact_keys(
            models[index], {"model_id", "payload"}, subject=f"{subject} model {index}"
        )
        payload = _require_exact_keys(
            model_record["payload"],
            {"kind", "model", "scientific_sha256"},
            subject=f"{subject} model payload {index}",
        )
        if (
            model_record["model_id"] != identity["model_id"]
            or payload["kind"] != "sem_model_v4"
            or payload["model"] != identity["model"]
            or payload["scientific_sha256"] != identity["model_digest"]
            or recipes[index] != identity["recipe"]
        ):
            raise ValueError(f"{subject} resident model/recipe {index} is invalid")
    origin = _require_exact_keys(
        document["origin"],
        {"kind", "lineage"},
        subject=f"{subject} project origin",
    )
    if origin["kind"] != "upgraded_copy":
        raise ValueError(f"{subject} project origin is invalid")
    lineage = _require_exact_keys(
        origin["lineage"],
        {
            "destination_archive_path",
            "historical_results_immutable",
            "source_archive_path",
            "source_archive_schema_version",
            "source_archive_sha256",
            "source_preservation",
            "source_project_id",
            "upgraded_at",
            "write_policy",
        },
        subject=f"{subject} upgrade lineage",
    )
    if (
        lineage["destination_archive_path"] != destination_archive_path
        or lineage["historical_results_immutable"] is not True
        or not isinstance(lineage["source_archive_path"], str)
        or not lineage["source_archive_path"].endswith("unchanged-source-v5.qpls")
        or lineage["source_archive_schema_version"] != 5
        or lineage["source_archive_sha256"] != "a" * 64
        or lineage["source_preservation"] != "required"
        or lineage["source_project_id"] != project_id
        or lineage["upgraded_at"] != "2026-08-15T00:00:00Z"
        or lineage["write_policy"] != "new_archive_only"
    ):
        raise ValueError(f"{subject} upgrade lineage is invalid")
    attachments = (
        _require_exact_list(
            document["canonical_result_documents"],
            length=attachment_count,
            subject=f"{subject} canonical attachments",
        )
        if attachment_count > 0
        else []
    )
    observed_digests: dict[str, str] = {}
    for index, attachment in enumerate(attachments):
        attachment = _require_exact_keys(
            attachment,
            {
                "canonical_document",
                "canonical_document_sha256",
                "document_id",
                "document_schema_version",
                "immutable",
                "run_id",
            },
            subject=f"{subject} canonical attachment {index}",
        )
        canonical = canonicals[index]
        digest = attachment["canonical_document_sha256"]
        if (
            attachment["canonical_document"] != canonical["canonical"]
            or attachment["document_id"] != canonical["document_id"]
            or attachment["document_schema_version"] != 2
            or attachment["immutable"] is not True
            or attachment["run_id"] != canonical["run_id"]
            or not _is_sha256(digest)
            or (
                expected_attachment_digests is not None
                and str(attachment["document_id"]) in expected_attachment_digests
                and expected_attachment_digests[str(attachment["document_id"])]
                != digest
            )
        ):
            raise ValueError(f"{subject} canonical attachment {index} is invalid")
        observed_digests[str(attachment["document_id"])] = str(digest)
    return observed_digests


def validate_current_product_work_envelope(
    *,
    source_hash: str,
    scenario_hash: str,
    result_root: Path | None = None,
    allowed_binary_root: Path | None = None,
) -> dict[str, Any]:
    """Strictly validate exactly one non-promotional current-product envelope."""

    if not _is_sha256(source_hash) or not _is_sha256(scenario_hash):
        raise ValueError("current-product source/scenario hashes are malformed")
    base = result_root or RESULT_ROOT
    source_root = base / f"source_{source_hash}"
    build_root = source_root / "product_builds"
    candidates = (
        sorted(build_root.glob("harness_*/current_product_work_envelope.json"))
        if build_root.is_dir()
        else []
    )
    if len(candidates) != 1:
        raise ValueError(
            "current-product work discovery requires exactly one source-bound envelope; "
            f"found {len(candidates)}"
        )
    envelope_path = candidates[0]
    harness_root = envelope_path.parent
    if (
        harness_root.parent.resolve() != build_root.resolve()
        or harness_root.is_symlink()
    ):
        raise ValueError("current-product harness directory is not a direct real child")
    envelope = strict_load_json(envelope_path)
    _require_exact_keys(
        envelope,
        {
            "schema_version",
            "report_kind",
            "qualification_id",
            "capability_id",
            "cell_id",
            "method_version",
            "generated_at_utc",
            "source_set_sha256",
            "scenario_set_sha256",
            "source_artifacts",
            "spec_sha256",
            "registry",
            "harness_binary",
            "build_fingerprint",
            "commands",
            "harness_report",
            "comparisons",
            "archive_smoke",
            "passed",
            "work_evidence_only",
            "qualification_ready",
            "promotion_authority",
            "candidate_receipt_descriptors",
            "customer_cli_invoked",
            "customer_registry_admission_invoked",
            "remaining_blockers",
        },
        subject="current-product envelope",
    )
    expected_header = {
        "schema_version": 2,
        "report_kind": "pls_algorithm_v1_current_product_work_envelope_v2",
        "qualification_id": QUALIFICATION_ID,
        "capability_id": CAPABILITY_ID,
        "cell_id": CELL_ID,
        "method_version": METHOD_VERSION,
        "source_set_sha256": source_hash,
        "scenario_set_sha256": scenario_hash,
        "passed": True,
        "work_evidence_only": True,
        "qualification_ready": False,
        "promotion_authority": False,
        "candidate_receipt_descriptors": [],
        "customer_cli_invoked": False,
        "customer_registry_admission_invoked": False,
    }
    if any(envelope.get(key) != value for key, value in expected_header.items()):
        raise ValueError("current-product envelope header is stale or promotional")
    generated_at = envelope["generated_at_utc"]
    try:
        generated_at_value = datetime.fromisoformat(
            str(generated_at).replace("Z", "+00:00")
        )
    except ValueError as error:
        raise ValueError("current-product envelope timestamp is invalid") from error
    if (
        not isinstance(generated_at, str)
        or not generated_at.endswith("Z")
        or generated_at_value.tzinfo != timezone.utc
        or envelope["remaining_blockers"] != list(CURRENT_PRODUCT_WORK_BLOCKERS)
    ):
        raise ValueError("current-product timestamp/blockers are stale")

    sources = source_descriptors()
    if (
        envelope.get("source_artifacts") != sources
        or canonical_sha256(sources) != source_hash
    ):
        raise ValueError("current-product envelope source descriptors are stale")
    spec = strict_load_json(SPEC_PATH)
    if canonical_sha256(spec["scenario_contract"]) != scenario_hash or envelope.get(
        "spec_sha256"
    ) != sha256_file(SPEC_PATH):
        raise ValueError("current-product envelope scenario/spec identity is stale")
    registry = strict_load_json(REGISTRY_PATH)
    registry_identity = envelope.get("registry")
    if not isinstance(registry_identity, Mapping) or registry_identity != {
        "path": repository_path(REGISTRY_PATH),
        "raw_sha256": sha256_file(REGISTRY_PATH),
        "canonical_sha256": canonical_sha256(registry),
        "registry_id": registry["registry_id"],
        "registry_version": registry["registry_version"],
        "exact_cell": {
            "capability_id": CAPABILITY_ID,
            "cell_id": CELL_ID,
            "capability_version": METHOD_VERSION,
            "coverage_state": "partial",
            "evidence_state": "absent",
            "surface": "labs",
        },
    }:
        raise ValueError("current-product envelope Registry identity is stale")

    binary = envelope.get("harness_binary")
    binary = _require_exact_keys(
        binary,
        {"path", "sha256", "size_bytes", "mtime_ns", "newest_bound_source_mtime_ns"},
        subject="current-product harness binary",
    )
    binary_relative = binary.get("path")
    if (
        not isinstance(binary_relative, str)
        or not binary_relative
        or "\\" in binary_relative
        or Path(binary_relative).is_absolute()
        or ".." in Path(binary_relative).parts
    ):
        raise ValueError("current-product harness binary path is unsafe")
    binary_lexical = ROOT / binary_relative
    binary_metadata = binary_lexical.lstat()
    if (
        binary_metadata.st_file_attributes & 0x400
        if os.name == "nt"
        else binary_lexical.is_symlink()
    ):
        raise ValueError("current-product harness binary path is a symlink")
    binary_path = binary_lexical.resolve(strict=True)
    binary_boundary = (
        allowed_binary_root or ROOT / "target" / "debug" / "deps"
    ).resolve(strict=True)
    try:
        binary_path.relative_to(binary_boundary)
    except ValueError as error:
        raise ValueError(
            "current-product harness binary escapes its allowed root"
        ) from error
    if not binary_path.is_file():
        raise ValueError("current-product harness binary is not a regular file")
    if (
        binary.get("size_bytes") != binary_path.stat().st_size
        or binary.get("sha256") != sha256_file(binary_path)
        or binary.get("mtime_ns") != binary_path.stat().st_mtime_ns
    ):
        raise ValueError("current-product harness binary descriptor is stale")
    newest_source_ns = max((ROOT / row["path"]).stat().st_mtime_ns for row in sources)
    if (
        binary.get("newest_bound_source_mtime_ns") != newest_source_ns
        or binary_path.stat().st_mtime_ns < newest_source_ns
    ):
        raise ValueError("current-product harness binary predates a bound source")
    binary_sha256 = str(binary["sha256"])
    if not _is_sha256(binary_sha256) or harness_root.name != f"harness_{binary_sha256}":
        raise ValueError("current-product harness directory/binary identity mismatch")
    if envelope.get("build_fingerprint") != (
        f"quickpls-desktop-test-sha256:{binary_sha256};source-set-sha256:{source_hash}"
    ):
        raise ValueError("current-product build fingerprint is stale")

    commands = envelope.get("commands")
    if not isinstance(commands, list) or len(commands) != 2:
        raise ValueError("current-product envelope must contain exactly two commands")
    expected_build = [
        "cargo",
        "test",
        "-p",
        INTERNAL_HARNESS_PACKAGE,
        "--lib",
        "--no-run",
        "--message-format=json",
    ]
    expected_run_tail = [
        "--ignored",
        "--exact",
        INTERNAL_HARNESS_TEST,
        "--nocapture",
        "--test-threads=1",
    ]
    if (
        not isinstance(commands[0], Mapping)
        or commands[0].get("label") != "build_internal_desktop_harness"
        or commands[0].get("command") != expected_build
        or commands[0].get("returncode") != 0
        or not isinstance(commands[1], Mapping)
        or commands[1].get("label") != "run_internal_desktop_harness"
        or commands[1].get("returncode") != 0
    ):
        raise ValueError("current-product command contract is stale")
    for index, command in enumerate(commands):
        _require_exact_keys(
            command,
            {
                "label",
                "command",
                "returncode",
                "started_at_utc",
                "finished_at_utc",
                "duration_seconds",
                "stdout",
                "stderr",
            },
            subject=f"current-product command {index}",
        )
        if (
            not isinstance(command["started_at_utc"], str)
            or not command["started_at_utc"]
            or not isinstance(command["finished_at_utc"], str)
            or not command["finished_at_utc"]
            or _require_finite_number(
                command["duration_seconds"],
                subject=f"current-product command {index} duration",
            )
            < 0
        ):
            raise ValueError(f"current-product command {index} timing is invalid")
    run_command = commands[1].get("command")
    if not isinstance(run_command, list) or len(run_command) != 1 + len(
        expected_run_tail
    ):
        raise ValueError("current-product direct harness command is malformed")
    try:
        observed_run_binary = Path(str(run_command[0])).resolve(strict=True)
    except OSError as error:
        raise ValueError(
            "current-product direct harness binary is unavailable"
        ) from error
    if observed_run_binary != binary_path or run_command[1:] != expected_run_tail:
        raise ValueError("current-product direct harness command identity is stale")
    command_text = json.dumps(commands, sort_keys=True).lower()
    if any(
        forbidden in command_text
        for forbidden in (
            "qpls-cli",
            "qpls.exe",
            "qpls run",
            "--allow-experimental",
            '"--cli"',
        )
    ):
        raise ValueError(
            "current-product command contract invokes a forbidden customer CLI"
        )
    for command in commands:
        _resolve_artifact_descriptor(
            harness_root, command.get("stdout"), subject=f"{command['label']} stdout"
        )
        _resolve_artifact_descriptor(
            harness_root, command.get("stderr"), subject=f"{command['label']} stderr"
        )

    harness_report_path = _resolve_artifact_descriptor(
        harness_root, envelope.get("harness_report"), subject="harness report"
    )
    harness = strict_load_json(harness_report_path)
    _require_exact_keys(
        harness,
        {
            "schema_version",
            "report_kind",
            "work_evidence_only",
            "qualification_ready",
            "promotion_authority",
            "customer_cli_invoked",
            "customer_registry_admission_invoked",
            "candidate_receipt_descriptors",
            "compiled_identity",
            "capability_cell",
            "variants",
            "archive",
        },
        subject="current-product harness report",
    )
    harness_header = {
        "schema_version": 1,
        "report_kind": "pls_algorithm_v1_current_product_harness_v1",
        "work_evidence_only": True,
        "qualification_ready": False,
        "promotion_authority": False,
        "customer_cli_invoked": False,
        "customer_registry_admission_invoked": False,
        "candidate_receipt_descriptors": [],
        "capability_cell": _current_product_capability_cell(),
    }
    if any(harness.get(key) != value for key, value in harness_header.items()):
        raise ValueError("current-product harness report is stale or promotional")
    compiled_identity = harness.get("compiled_identity")
    if not isinstance(compiled_identity, Mapping) or compiled_identity != {
        "source_set_sha256": source_hash,
        "scenario_set_sha256": scenario_hash,
        "executable": {
            "path": str(binary_path),
            "size_bytes": binary_path.stat().st_size,
            "sha256": binary_sha256,
        },
    }:
        raise ValueError("current-product harness compiled identity is stale")

    comparisons = envelope.get("comparisons")
    if not isinstance(comparisons, list):
        raise ValueError("current-product comparisons are missing")
    comparisons_by_id = {
        row.get("variant"): row for row in comparisons if isinstance(row, Mapping)
    }
    if set(comparisons_by_id) != set(CURRENT_PRODUCT_VARIANTS) or len(
        comparisons_by_id
    ) != len(comparisons):
        raise ValueError("current-product comparison variant set is not exact")
    for variant, comparison in comparisons_by_id.items():
        _require_exact_keys(
            comparison,
            {
                "variant",
                "passed",
                "exact_key_membership",
                "compared_quantity_count",
                "max_abs_difference",
                "tolerance",
                "missing_from_product",
                "unexpected_from_product",
                "construct_identity_map",
            },
            subject=f"current-product comparison {variant}",
        )
        maximum = comparison.get("max_abs_difference")
        identity_map = comparison.get("construct_identity_map")
        if (
            comparison.get("passed") is not True
            or comparison.get("exact_key_membership") is not True
            or comparison.get("compared_quantity_count") != 9
            or comparison.get("tolerance") != 1.0e-6
            or isinstance(maximum, bool)
            or not isinstance(maximum, (int, float))
            or not math.isfinite(maximum)
            or maximum < 0
            or maximum > 1.0e-6
            or comparison.get("missing_from_product") != []
            or comparison.get("unexpected_from_product") != []
            or not isinstance(identity_map, Mapping)
            or dict(identity_map) != {"x": "construct:x", "y": "construct:y"}
        ):
            raise ValueError(f"current-product comparison is invalid: {variant}")

    archive_smoke = envelope.get("archive_smoke")
    if archive_smoke != {
        "passed": True,
        "schema_version": 6,
        "scientific_result_appended_by_real_runner": True,
        "real_product_canonical_projection": True,
        "atomic_append_and_reopen_verified": True,
        "receipt_eligible": False,
    }:
        raise ValueError("current-product schema-6 envelope flags are invalid")
    archive_report = harness.get("archive")
    archive_report = _require_exact_keys(
        archive_report,
        {
            "schema_version",
            "canonical_result_document_count",
            "resident_dataset_verified",
            "models_and_recipes_verified",
            "atomic_append_verified",
            "final_reopen_verified",
            "source_archive_preserved_as_absent_fixture",
            "initial_write",
            "initial_document",
            "document",
        },
        subject="current-product harness schema-6 archive report",
    )
    if any(
        archive_report.get(key) != value
        for key, value in {
            "schema_version": 6,
            "canonical_result_document_count": 4,
            "resident_dataset_verified": True,
            "models_and_recipes_verified": True,
            "atomic_append_verified": True,
            "final_reopen_verified": True,
            "source_archive_preserved_as_absent_fixture": True,
        }.items()
    ):
        raise ValueError("current-product harness schema-6 archive flags are invalid")
    initial_write = _require_exact_keys(
        archive_report.get("initial_write"),
        {
            "schema_version",
            "project_id",
            "destination_archive_path",
            "document_sha256",
            "byte_length",
            "post_write_validated",
        },
        subject="current-product schema-6 initial write receipt",
    )
    project_id = _require_uuid(
        initial_write["project_id"], subject="current-product schema-6 project id"
    )
    append_archive_path = initial_write["destination_archive_path"]
    if (
        initial_write["schema_version"] != 6
        or not isinstance(append_archive_path, str)
        or not append_archive_path
        or initial_write["post_write_validated"] is not True
        or not _is_sha256(initial_write["document_sha256"])
        or isinstance(initial_write["byte_length"], bool)
        or not isinstance(initial_write["byte_length"], int)
        or initial_write["byte_length"] <= 0
    ):
        raise ValueError("current-product schema-6 initial write receipt is invalid")
    initial_archive_path = _resolve_artifact_descriptor(
        harness_report_path.parent,
        archive_report["initial_document"],
        subject="initial schema-6 project document",
    )
    if (
        sha256_file(initial_archive_path) != initial_write["document_sha256"]
        or initial_archive_path.stat().st_size != initial_write["byte_length"]
    ):
        raise ValueError("current-product initial schema-6 bytes differ from receipt")
    initial_archive_document = strict_load_json(initial_archive_path)
    archive_path = _resolve_artifact_descriptor(
        harness_report_path.parent,
        archive_report["document"],
        subject="schema-6 project document",
    )
    archive_document = strict_load_json(archive_path)

    variants = harness.get("variants")
    if not isinstance(variants, list):
        raise ValueError("current-product harness variants are missing")
    variants_by_id = {
        row.get("variant"): row for row in variants if isinstance(row, Mapping)
    }
    if set(variants_by_id) != set(CURRENT_PRODUCT_VARIANTS) or len(
        variants_by_id
    ) != len(variants):
        raise ValueError("current-product harness variant set is not exact")
    append_source_sha256 = initial_write["document_sha256"]
    identities: list[dict[str, Any]] = []
    compiled_receipts: list[Mapping[str, Any]] = []
    executions: list[dict[str, Any]] = []
    canonicals: list[dict[str, Any]] = []
    archive_steps: list[tuple[Path, Mapping[str, Any]]] = []
    artifact_paths: set[Path] = set()
    for index, variant in enumerate(CURRENT_PRODUCT_VARIANTS, start=1):
        row = variants_by_id[variant]
        _require_exact_keys(
            row,
            {
                "variant",
                "weighting_scheme",
                "measurement_mode",
                "recipe_v4_compiled",
                "real_runner_executed",
                "product_canonical_builder_executed",
                "schema6_append_reopened",
                "method_version",
                "compilation_receipt_sha256",
                "request",
                "compiled_artifact",
                "execution",
                "canonical_document",
                "append_receipt",
                "archive_after_append",
            },
            subject=f"current-product harness variant {variant}",
        )
        expected_weighting, expected_mode = CURRENT_PRODUCT_VARIANTS[variant]
        if (
            row.get("weighting_scheme") != expected_weighting
            or row.get("measurement_mode") != expected_mode
            or row.get("recipe_v4_compiled") is not True
            or row.get("real_runner_executed") is not True
            or row.get("product_canonical_builder_executed") is not True
            or row.get("schema6_append_reopened") is not True
            or row.get("method_version") != METHOD_VERSION
            or not _is_sha256(row.get("compilation_receipt_sha256"))
        ):
            raise ValueError(
                f"current-product harness variant flags are invalid: {variant}"
            )
        resolved_artifacts = {}
        for artifact_name in (
            "request",
            "compiled_artifact",
            "execution",
            "canonical_document",
        ):
            resolved_artifacts[artifact_name] = _resolve_artifact_descriptor(
                harness_report_path.parent,
                row[artifact_name],
                subject=f"{variant} {artifact_name}",
            )
        if artifact_paths.intersection(resolved_artifacts.values()):
            raise ValueError(f"{variant} reuses a cross-variant artifact path")
        artifact_paths.update(resolved_artifacts.values())
        request_document = strict_load_json(resolved_artifacts["request"])
        identity = _validate_current_product_request(request_document, variant=variant)
        compiled_document = strict_load_json(resolved_artifacts["compiled_artifact"])
        compiled_receipt = _validate_current_product_compiled(
            compiled_document, variant=variant, identity=identity
        )
        if row["compilation_receipt_sha256"] != _serialized_json_sha256(
            compiled_receipt
        ):
            raise ValueError(f"{variant} compilation receipt digest is invalid")
        execution_document = strict_load_json(resolved_artifacts["execution"])
        execution = _validate_current_product_execution(
            execution_document,
            variant=variant,
            identity=identity,
            receipt=compiled_receipt,
        )
        receipt = _require_exact_keys(
            row["append_receipt"],
            {
                "schema_version",
                "project_id",
                "archive_path",
                "source_document_sha256",
                "updated_document_sha256",
                "canonical_document_id",
                "run_id",
                "canonical_result_document_count",
                "source_verified_at_commit",
                "post_write_validated",
                "rollback_copy_removed",
            },
            subject=f"{variant} schema-6 append receipt",
        )
        canonical_document = strict_load_json(resolved_artifacts["canonical_document"])
        canonical = _validate_current_product_canonical(
            canonical_document,
            variant=variant,
            project_id=project_id,
            identity=identity,
            receipt=compiled_receipt,
            execution=execution,
        )
        archive_step_path = _resolve_artifact_descriptor(
            harness_report_path.parent,
            row["archive_after_append"],
            subject=f"{variant} schema-6 archive after append",
        )
        archive_step_document = strict_load_json(archive_step_path)
        if (
            receipt["schema_version"] != 6
            or receipt["project_id"] != project_id
            or receipt["archive_path"] != append_archive_path
            or receipt["source_document_sha256"] != append_source_sha256
            or receipt["updated_document_sha256"] != sha256_file(archive_step_path)
            or receipt["canonical_result_document_count"] != index
            or receipt["source_verified_at_commit"] is not True
            or receipt["post_write_validated"] is not True
            or receipt["rollback_copy_removed"] is not True
            or receipt["canonical_document_id"] != canonical["document_id"]
            or receipt["run_id"] != canonical["run_id"]
        ):
            raise ValueError(
                f"current-product canonical/append contract is invalid: {variant}"
            )
        append_source_sha256 = str(receipt["updated_document_sha256"])
        identities.append(identity)
        compiled_receipts.append(compiled_receipt)
        executions.append(execution)
        canonicals.append(canonical)
        archive_steps.append((archive_step_path, archive_step_document))
    for subject, values in {
        "model ids": [identity["model_id"] for identity in identities],
        "model digests": [identity["model_digest"] for identity in identities],
        "recipe ids": [identity["recipe_id"] for identity in identities],
        "recipe analytical digests": [
            receipt["recipe_analytical_sha256"] for receipt in compiled_receipts
        ],
        "recipe document digests": [
            receipt["recipe_document_sha256"] for receipt in compiled_receipts
        ],
        "plan digests": [receipt["plan_sha256"] for receipt in compiled_receipts],
        "compilation receipt digests": [
            _serialized_json_sha256(receipt) for receipt in compiled_receipts
        ],
        "analytical identities": [
            receipt["analytical_identity_sha256"] for receipt in compiled_receipts
        ],
        "projected recipe digests": [
            execution["provenance"]["projected_recipe_sha256"]
            for execution in executions
        ],
        "canonical document ids": [
            canonical["document_id"] for canonical in canonicals
        ],
        "run ids": [canonical["run_id"] for canonical in canonicals],
    }.items():
        if len(set(values)) != len(CURRENT_PRODUCT_VARIANTS):
            raise ValueError(f"current-product cross-variant {subject} are not unique")
    if (
        append_source_sha256 != sha256_file(archive_path)
        or archive_path.read_bytes() != archive_steps[-1][0].read_bytes()
    ):
        raise ValueError("current-product final schema-6 digest chain is invalid")
    _validate_current_product_archive_document(
        initial_archive_document,
        subject="initial schema-6 document",
        project_id=project_id,
        destination_archive_path=append_archive_path,
        identities=identities,
        canonicals=canonicals,
        attachment_count=0,
    )
    attachment_digests: dict[str, str] = {}
    for index, (_, archive_step_document) in enumerate(archive_steps, start=1):
        observed = _validate_current_product_archive_document(
            archive_step_document,
            subject=f"schema-6 append snapshot {index}",
            project_id=project_id,
            destination_archive_path=append_archive_path,
            identities=identities,
            canonicals=canonicals,
            attachment_count=index,
            expected_attachment_digests=attachment_digests,
        )
        attachment_digests.update(observed)
    final_digests = _validate_current_product_archive_document(
        archive_document,
        subject="final schema-6 document",
        project_id=project_id,
        destination_archive_path=append_archive_path,
        identities=identities,
        canonicals=canonicals,
        attachment_count=4,
        expected_attachment_digests=attachment_digests,
    )
    if final_digests != attachment_digests:
        raise ValueError("current-product final schema-6 attachments drifted")

    return {
        "schema_version": 1,
        "status": "validated_work_evidence_only",
        "passed": True,
        "work_evidence_only": True,
        "qualification_ready": False,
        "promotion_authority": False,
        "candidate_receipt_descriptors": [],
        "source_set_sha256": source_hash,
        "scenario_set_sha256": scenario_hash,
        "envelope": _descriptor(envelope_path),
        "harness_report": _descriptor(harness_report_path),
        "harness_binary": dict(binary),
        "variants": list(CURRENT_PRODUCT_VARIANTS),
        "maximum_abs_difference": max(
            float(row["max_abs_difference"]) for row in comparisons_by_id.values()
        ),
        "schema6_document": _descriptor(archive_path),
        "schema6_initial_document": _descriptor(initial_archive_path),
        "schema6_append_documents": [_descriptor(path) for path, _ in archive_steps],
        "strict_artifact_chain_validated": True,
        "legacy_score_execution_absence_validated": True,
        "stable_quantity_membership_count_per_variant": 9,
        "kernel_execution_work_evidence": True,
        "archive_persistence_work_evidence": True,
        "receipt_eligible": False,
    }


def _method_contract_checks(
    spec: dict[str, Any],
    manifest: dict[str, Any],
    capability: dict[str, Any],
    cell: dict[str, Any],
    validation: dict[str, Any],
    sources: list[dict[str, Any]],
) -> dict[str, bool]:
    evidence = manifest["qualification"]["evidence"]
    manifest_roles = {
        role
        for rows in evidence.values()
        for artifact in rows
        for role in artifact["roles"]
    }
    method_text = (ROOT / "docs/methods/PLS_PM_V1.md").read_text(encoding="utf-8")
    status_text = (
        ROOT / "docs/methods/PLS_ALGORITHM_V1_QUALIFICATION_STATUS.md"
    ).read_text(encoding="utf-8")
    return {
        "qualification_spec_schema_semantics_and_registry_pass": bool(
            validation["passed"]
            and validation["schema_valid"]
            and validation["semantic_valid"]
            and validation["registry_verified"]
        ),
        "qualification_spec_is_non_promotional_future_full_parity_backlog": bool(
            spec["migration"]["status"] == "compatibility_only"
            and spec["migration"]["unresolved_items"]
            and all(
                item.startswith("future_full_parity.")
                for item in spec["migration"]["unresolved_items"]
            )
        ),
        "qualification_spec_has_zero_attached_receipts": bool(
            spec["evidence_contract"]["receipts"] == []
            and validation["qualification_ready"] is False
        ),
        "required_receipt_roles_are_exact": tuple(
            spec["evidence_contract"]["required_roles"]
        )
        == EXPECTED_REQUIRED_ROLES,
        "scoped_manifest_is_separately_release_qualified": bool(
            manifest["feature"]["id"] == CELL_ID
            and manifest["feature"]["method_version"] == METHOD_VERSION
            and manifest["qualification"]["declared_state"] == "release_qualified"
            and manifest_roles
            == {
                "method_spec",
                "independent_reference",
                "simulation_report",
                "boundary_report",
                "persistence_report",
                "frontend_report",
                "export_report",
                "method_audit",
                "packaged_acceptance",
            }
        ),
        "registry_is_scoped_standard": bool(
            capability["coverage_state"] == "partial"
            and capability["evidence_state"] == "release_qualified"
            and capability["surface"] == "standard"
            and cell["coverage_state"] == "partial"
            and cell["evidence_state"] == "release_qualified"
            and cell["surface"] == "standard"
            and cell["capability_version"] == METHOD_VERSION
        ),
        "official_current_defaults_are_frozen": bool(
            "3,000" in method_text
            and "`1e-7`" in method_text
            and OFFICIAL_PLS_URL in status_text
            and OFFICIAL_SNAPSHOT_DATE in status_text
        ),
        "future_full_parity_coverage_and_evidence_backlog_is_distinct": bool(
            set(FULL_PARITY_COVERAGE_BACKLOG).issubset(
                spec["migration"]["unresolved_items"]
            )
            and set(FULL_PARITY_EVIDENCE_BACKLOG).issubset(
                spec["migration"]["unresolved_items"]
            )
        ),
        "archive_and_scale_contract_match_finalization_program": bool(
            spec["operational_contract"]["archive"]["current_schema_version"] == 6
            and spec["operational_contract"]["archive"]["writable_schema_versions"]
            == [6]
            and next(
                row
                for row in spec["scenario_contract"]["complexity_profiles"]
                if row["id"] == "maximum_axis"
            )["workload"]
            == {
                "rows": 100_000,
                "indicators": 300,
                "constructs": 100,
                "resamples": 0,
                "groups": 1,
                "candidate_models": 1,
            }
        ),
        "source_descriptors_are_complete": bool(
            len(sources) == len(set(SOURCE_PATHS))
            and all((ROOT / row["path"]).is_file() for row in sources)
        ),
    }


def _candidate_method_contract(
    *,
    generated_at: str,
    sources: list[dict[str, Any]],
    source_hash: str,
    scenario_hash: str,
    checks: dict[str, bool],
) -> dict[str, Any]:
    passed = all(checks.values())
    if not passed:
        failed = sorted(key for key, value in checks.items() if not value)
        raise RuntimeError("PLS method contract failed: " + ", ".join(failed))
    return {
        "schema_version": 1,
        "report_kind": "pls_algorithm_v1_method_contract_candidate_v1",
        "role": "method_contract",
        "stage": "contract",
        "passed": True,
        "qualification_evidence": True,
        "receipt_eligible": True,
        "qualification_ready": False,
        "promotion_authority": False,
        "qualification_id": QUALIFICATION_ID,
        "capability_id": CAPABILITY_ID,
        "cell_id": CELL_ID,
        "method_version": METHOD_VERSION,
        "generated_at_utc": generated_at,
        "spec_frozen_at_utc": SPEC_FROZEN_AT_UTC,
        "source_set_sha256": source_hash,
        "scenario_set_sha256": scenario_hash,
        "build_fingerprint": f"validation-source-contract:{source_hash}",
        "hardware_fingerprint": _hardware_fingerprint(),
        "source_artifacts": sources,
        "checks": checks,
        "role_blockers": [],
        "admission_policy": (
            "Candidate descriptor only. It is not attached to QualificationSpec V2 and "
            "cannot change evidence state until every required role and independent "
            "scientific review is satisfied."
        ),
    }


def _current_build_execution_contract(
    source_hash: str, scenario_hash: str
) -> dict[str, Any]:
    return {
        "policy": (
            "Central serialized execution only after all product sources freeze. The "
            "orchestrator must build and directly invoke the exact ignored quickpls-desktop "
            "lib-test harness; a stale or substituted executable, customer CLI result, "
            "interactive test result, or historical artifact is rejected."
        ),
        "required_source_set_sha256": source_hash,
        "required_scenario_set_sha256": scenario_hash,
        "required_product_source_paths": list(PRODUCT_BUILD_SOURCE_PATHS),
        "execution_surface": "internal_labs_ignored_lib_test",
        "harness_package": INTERNAL_HARNESS_PACKAGE,
        "harness_target": INTERNAL_HARNESS_TARGET,
        "harness_test": INTERNAL_HARNESS_TEST,
        "customer_cli_execution_allowed": False,
        "candidate_receipt_output_allowed": False,
        "central_commands": [
            [
                "python",
                INTERNAL_HARNESS_ORCHESTRATOR,
                "--source-set-sha256",
                source_hash,
                "--scenario-set-sha256",
                scenario_hash,
                "--write-new-snapshot",
            ],
        ],
        "required_future_roles": [
            "kernel_execution",
            "oracle_independence",
            "adversarial_boundaries",
            "archive_persistence",
        ],
        "generator_status": "implemented_not_executed_by_source_contract_factory",
        "generator_blocker": (
            "The source-bound internal harness is implemented, but this source-contract "
            "factory does not run Cargo or product code. Its separate work envelope has "
            "no receipt, Registry-admission, or promotion authority."
        ),
    }


def build_audit(
    *,
    generated_at: str,
    sources: list[dict[str, Any]],
    source_hash: str,
    transparent_report: dict[str, Any],
    external_report: dict[str, Any],
    paths: dict[str, Path],
) -> tuple[dict[str, Any], dict[str, Any]]:
    spec = strict_load_json(SPEC_PATH)
    manifest = strict_load_json(MANIFEST_PATH)
    capability, cell = _registry_cell()
    validation = validate_spec_path(
        SPEC_PATH,
        repository_root=ROOT,
        registry_path=REGISTRY_PATH,
        require_registry=True,
    )
    scenario_hash = canonical_sha256(spec["scenario_contract"])
    current_product_work = validate_current_product_work_envelope(
        source_hash=source_hash,
        scenario_hash=scenario_hash,
    )
    checks = _method_contract_checks(
        spec, manifest, capability, cell, validation, sources
    )
    candidate = _candidate_method_contract(
        generated_at=generated_at,
        sources=sources,
        source_hash=source_hash,
        scenario_hash=scenario_hash,
        checks=checks,
    )
    product_harness = describe_internal_product_harness()
    work_checks = {
        "transparent_numpy_oracle_passed": bool(
            transparent_report["passed"]
            and transparent_report["work_evidence_only"]
            and transparent_report["qualification_ready"] is False
        ),
        "hand_microcase_passed": bool(
            transparent_report["checks"]["hand_single_item_path_equals_correlation"]
        ),
        "deterministic_metamorphics_passed": bool(
            transparent_report["checks"]["metamorphics_within_tolerance"]
            and transparent_report["checks"]["same_seed_fixture_repeat_exact"]
            and transparent_report["checks"]["different_seed_changes_fixture"]
        ),
        "typed_boundaries_passed": bool(
            transparent_report["checks"]["typed_boundaries_exact"]
        ),
        "two_external_computational_groups_passed": bool(
            external_report["passed"]
            and external_report["work_evidence_only"]
            and external_report["independence"]["computational_groups"]
            == ["r_csem_0_6_1", "python_plspm_0_5_7"]
            and external_report["independence"]["quickpls_product_executed"] is False
        ),
        "external_comparisons_have_exact_key_membership": all(
            row["exact_key_membership"] for row in external_report["comparisons"]
        ),
        "product_execution_is_delegated_without_receipt_authority": bool(
            product_harness["product_execution_performed_by_factory"] is False
            and product_harness["customer_cli_invoked"] is False
            and product_harness["customer_registry_admission_invoked"] is False
            and product_harness["candidate_receipt_output_allowed"] is False
            and product_harness["qualification_ready"] is False
        ),
        "source_bound_current_product_work_envelope_validated": bool(
            current_product_work["passed"] is True
            and current_product_work["work_evidence_only"] is True
            and current_product_work["qualification_ready"] is False
            and current_product_work["promotion_authority"] is False
            and current_product_work["candidate_receipt_descriptors"] == []
            and current_product_work["receipt_eligible"] is False
        ),
    }
    if not all(work_checks.values()):
        failed = sorted(key for key, value in work_checks.items() if not value)
        raise RuntimeError("PLS work-evidence checks failed: " + ", ".join(failed))

    role_matrix = []
    for role in EXPECTED_REQUIRED_ROLES:
        if role == "method_contract":
            status = "candidate_receipt_emitted"
            reasons: list[str] = []
        elif role == "kernel_execution":
            status = "work_evidence_only"
            reasons = [
                "The source-bound four-variant current-product matrix passed, but it is not a QualificationSpec receipt and does not cover the complete kernel role."
            ]
        elif role == "oracle_independence":
            status = "work_evidence_only"
            reasons = [
                "The bounded current-product matrix agrees with the transparent oracle and two external implementations; the complete qualification role and independent review remain absent."
            ]
        elif role == "adversarial_boundaries":
            status = "work_evidence_only"
            reasons = [
                "Source-oracle metamorphics and typed failures pass; product-level adversarial, worker, desktop/native, and archive-tamper matrices remain absent."
            ]
        elif role == "archive_persistence":
            status = "work_evidence_only"
            reasons = [
                "The real runner/native canonical/schema-6 append and reopen path passed for four variants, but qualification-sized archive/tamper coverage and an attachable receipt remain absent."
            ]
        else:
            status = "blocked"
            reasons = [f"Qualification receipt for {role} is absent."]
        role_matrix.append(
            {
                "role": role,
                "status": status,
                "candidate_receipt_emitted": role == "method_contract",
                "reasons": reasons,
            }
        )

    candidate_descriptor = (
        _descriptor(paths["receipt"])
        if paths["receipt"].is_file()
        else {
            "path": repository_path(paths["receipt"]),
            "pending_write": True,
        }
    )
    audit = {
        "schema_version": 1,
        "report_kind": "pls_algorithm_v1_current_source_qualification_factory_audit_v1",
        "qualification_id": QUALIFICATION_ID,
        "capability_id": CAPABILITY_ID,
        "cell_id": CELL_ID,
        "method_version": METHOD_VERSION,
        "generated_at_utc": generated_at,
        "spec_frozen_at_utc": SPEC_FROZEN_AT_UTC,
        "official_smartpls_snapshot": {
            "checked_on": OFFICIAL_SNAPSHOT_DATE,
            "core_algorithm_url": OFFICIAL_PLS_URL,
            "factor_composite_url": OFFICIAL_MODEL_URL,
        },
        "source_set_sha256": source_hash,
        "scenario_set_sha256": scenario_hash,
        "build_fingerprint": f"validation-source-contract:{source_hash}",
        "source_artifacts": sources,
        "method_contract_checks": checks,
        "work_evidence_checks": work_checks,
        "work_evidence": {
            "transparent_numpy_oracle": _descriptor(paths["oracle"]),
            "independent_external_oracles": _descriptor(paths["external"]),
            "source_bound_current_product": current_product_work,
        },
        "internal_product_harness_contract": product_harness,
        "role_matrix": role_matrix,
        "candidate_receipt_descriptors": [candidate_descriptor],
        "attached_receipt_count": 0,
        "registry_mutated": False,
        "manifest_mutated": False,
        "qualification_spec_receipts_mutated": False,
        "scientific_review_satisfied": False,
        "qualification_ready": False,
        "promotion_allowed": False,
        "supportable_evidence_tier": "source_contract_candidate_only",
        "future_full_parity_coverage_backlog": list(FULL_PARITY_COVERAGE_BACKLOG),
        "future_full_parity_evidence_backlog": list(FULL_PARITY_EVIDENCE_BACKLOG),
        "current_build_execution_contract": _current_build_execution_contract(
            source_hash, scenario_hash
        ),
        "remaining_full_parity_backlog": [
            *FULL_PARITY_COVERAGE_BACKLOG,
            *FULL_PARITY_EVIDENCE_BACKLOG,
            "qualification.only_method_contract_candidate_descriptor_exists",
            "qualification.no_candidate_descriptor_is_attached_to_the_spec",
        ],
        "passed": all(checks.values()) and all(work_checks.values()),
        "note": (
            "This source-hash-addressed audit is non-promotional. It supports one "
            "method-contract candidate plus bounded source-oracle and current-product "
            "work for a future full-parity program only; it does not gate or authorize "
            "the separately qualified bounded scoped-Standard contract."
        ),
    }
    return candidate, audit


def write_factory_snapshot() -> dict[str, Any]:
    expected_spec = build_spec()
    _require_frozen_qualification_spec(expected_spec)
    protected = {
        REGISTRY_PATH: sha256_file(REGISTRY_PATH),
        MANIFEST_PATH: sha256_file(MANIFEST_PATH),
    }
    sources = source_descriptors()
    source_hash = canonical_sha256(sources)
    paths = _snapshot_paths(source_hash)
    scenario_hash = canonical_sha256(expected_spec["scenario_contract"])
    # The separately serialized product orchestrator runs first and creates
    # only product_builds/. The factory may then add its own create-new files,
    # but it never overwrites an existing factory snapshot.
    validate_current_product_work_envelope(
        source_hash=source_hash,
        scenario_hash=scenario_hash,
    )
    existing_children = (
        {child.name for child in paths["root"].iterdir()}
        if paths["root"].is_dir()
        else set()
    )
    if existing_children != {"product_builds"}:
        raise FileExistsError(
            "current-source snapshot must contain only the prevalidated product_builds "
            f"handoff before factory publication: {repository_path(paths['root'])}"
        )
    paths["work"].mkdir(parents=True, exist_ok=False)
    transparent_report = oracle.write_work_report(paths["oracle"])
    external_report = run_external_reference_work(paths["work"])
    _write_json(paths["external"], external_report)
    generated_at = datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")
    candidate, audit = build_audit(
        generated_at=generated_at,
        sources=sources,
        source_hash=source_hash,
        transparent_report=transparent_report,
        external_report=external_report,
        paths=paths,
    )
    _write_json(paths["receipt"], candidate)
    # The audit includes the candidate descriptor, so rebuild after the file exists.
    candidate, audit = build_audit(
        generated_at=generated_at,
        sources=sources,
        source_hash=source_hash,
        transparent_report=transparent_report,
        external_report=external_report,
        paths=paths,
    )
    _write_json(paths["audit"], audit)
    after = {path: sha256_file(path) for path in protected}
    if protected != after:
        raise RuntimeError("PLS qualification factory mutated registry or manifest")
    return audit


def verify_checked_in_snapshot() -> dict[str, Any]:
    expected_spec = build_spec()
    actual_spec = strict_load_json(SPEC_PATH)
    errors: list[str] = []
    if actual_spec != expected_spec:
        errors.append("qualification_spec_not_equal_to_current_fail_closed_builder")
    sources = source_descriptors()
    source_hash = canonical_sha256(sources)
    paths = _snapshot_paths(source_hash)
    for label in ("oracle", "external", "receipt", "audit"):
        if not paths[label].is_file():
            errors.append(f"current_source_{label}_missing")
    if errors:
        return {
            "passed": False,
            "errors": errors,
            "required_snapshot": repository_path(paths["root"]),
            "qualification_ready": False,
            "promotion_allowed": False,
        }
    transparent = strict_load_json(paths["oracle"])
    external = strict_load_json(paths["external"])
    candidate = strict_load_json(paths["receipt"])
    audit = strict_load_json(paths["audit"])
    try:
        current_product_work = validate_current_product_work_envelope(
            source_hash=source_hash,
            scenario_hash=canonical_sha256(actual_spec["scenario_contract"]),
        )
    except (FileNotFoundError, OSError, ValueError) as error:
        current_product_work = None
        errors.append(f"current_product_work_invalid:{type(error).__name__}:{error}")
    if audit.get("source_artifacts") != sources:
        errors.append("source_descriptors_stale")
    if audit.get("source_set_sha256") != source_hash:
        errors.append("source_set_hash_stale")
    if audit.get("scenario_set_sha256") != canonical_sha256(
        actual_spec["scenario_contract"]
    ):
        errors.append("scenario_set_hash_stale")
    expected_execution_contract = _current_build_execution_contract(
        source_hash, canonical_sha256(actual_spec["scenario_contract"])
    )
    if audit.get("current_build_execution_contract") != expected_execution_contract:
        errors.append("current_product_harness_execution_contract_stale")
    if (
        audit.get("internal_product_harness_contract")
        != describe_internal_product_harness()
    ):
        errors.append("internal_product_harness_contract_stale")
    if "local_cli_audit" in audit:
        errors.append("stale_customer_cli_audit_present")
    audit_work_evidence = audit.get("work_evidence")
    if current_product_work is not None and (
        not isinstance(audit_work_evidence, Mapping)
        or audit_work_evidence.get("source_bound_current_product")
        != current_product_work
    ):
        errors.append("source_bound_current_product_work_descriptor_stale")
    if audit.get("candidate_receipt_descriptors") != [_descriptor(paths["receipt"])]:
        errors.append("candidate_receipt_descriptor_stale")
    if (
        candidate.get("role") != "method_contract"
        or candidate.get("build_fingerprint")
        != f"validation-source-contract:{source_hash}"
    ):
        errors.append("candidate_method_contract_identity_invalid")
    if not transparent.get("passed") or not transparent.get("work_evidence_only"):
        errors.append("transparent_oracle_work_invalid")
    if not external.get("passed") or not external.get("work_evidence_only"):
        errors.append("external_oracle_work_invalid")
    if audit.get("supportable_evidence_tier") != "source_contract_candidate_only":
        errors.append("supportable_evidence_tier_overstated")
    if audit.get("attached_receipt_count") != 0:
        errors.append("audit_claims_attached_receipt")
    if actual_spec["evidence_contract"]["receipts"]:
        errors.append("qualification_spec_has_attached_receipt")
    if (
        audit.get("qualification_ready") is not False
        or audit.get("promotion_allowed") is not False
    ):
        errors.append("audit_makes_promotion_claim")
    if any(
        audit.get(field) is not False
        for field in (
            "registry_mutated",
            "manifest_mutated",
            "qualification_spec_receipts_mutated",
            "scientific_review_satisfied",
        )
    ):
        errors.append("audit_records_forbidden_state_change")
    candidate_roles = [
        row["role"] for row in audit["role_matrix"] if row["candidate_receipt_emitted"]
    ]
    if candidate_roles != ["method_contract"]:
        errors.append("unexpected_candidate_receipt_roles")
    work_evidence_roles = [
        row["role"]
        for row in audit["role_matrix"]
        if row["status"] == "work_evidence_only"
    ]
    if work_evidence_roles != [
        "kernel_execution",
        "oracle_independence",
        "adversarial_boundaries",
        "archive_persistence",
    ]:
        errors.append("current_product_work_roles_are_not_exact")
    return {
        "passed": not errors,
        "errors": errors,
        "snapshot": repository_path(paths["root"]),
        "source_set_sha256": source_hash,
        "candidate_receipt_roles": candidate_roles,
        "work_evidence_roles": work_evidence_roles,
        "supportable_evidence_tier": "source_contract_candidate_only",
        "qualification_ready": False,
        "promotion_allowed": False,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--refresh-spec", action="store_true")
    parser.add_argument("--write-new-snapshot", action="store_true")
    parser.add_argument("--verify", action="store_true")
    args = parser.parse_args()
    if sum((args.refresh_spec, args.write_new_snapshot, args.verify)) != 1:
        parser.error(
            "select exactly one of --refresh-spec, --write-new-snapshot, or --verify"
        )
    if args.refresh_spec:
        result = refresh_qualification_spec()
    elif args.write_new_snapshot:
        result = write_factory_snapshot()
    else:
        result = verify_checked_in_snapshot()
    print(json.dumps(result, indent=2, sort_keys=True, allow_nan=False))
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
