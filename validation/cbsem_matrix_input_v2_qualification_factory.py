#!/usr/bin/env python3
"""Fail-closed QualificationSpec V2 work-evidence factory for CB-SEM matrices.

The factory writes a compatibility-only candidate specification and a
source-bound audit.  It never edits Capability Registry V2, a method-promotion
manifest, or the specification's empty receipt list.  Passing validation work
is kept distinct from immutable product qualification evidence.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any, Iterable


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
SPEC_PATH = (
    VALIDATION / "qualification_v2" / "cbsem_matrix_input_v2.qualification.json"
)
AUDIT_PATH = (
    VALIDATION
    / "results"
    / "method_factory"
    / "cbsem_matrix_input_v2"
    / "qualification_factory_audit.json"
)
ORACLE_REPORT_PATH = (
    VALIDATION
    / "results"
    / "method_factory"
    / "cbsem_matrix_input_v2"
    / "work"
    / "independent_oracle.json"
)
REGISTRY_PATH = VALIDATION / "capabilities" / "capability_registry_v2.json"

sys.path.insert(0, str(VALIDATION))

import cbsem_matrix_input_v2_oracle as oracle  # noqa: E402
from qualification_spec_v2 import (  # noqa: E402
    canonical_sha256,
    strict_load_json,
    validate_spec_path,
)


QUALIFICATION_ID = "qpls3.cbsem.ml.matrix_input.qualification_v2"
CAPABILITY_ID = "smartpls.cbsem"
CELL_ID = "qpls3.cbsem.ml.matrix_input"
METHOD_VERSION = "cbsem_ml_compiled_moment_input_v2"
SPEC_FROZEN_AT_UTC = "2026-08-14T18:00:00Z"
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
SOURCE_PATHS = (
    "docs/methods/CBSEM_MATRIX_INPUT_V2.md",
    "validation/capabilities/capability_registry_v2.json",
    "validation/capabilities/cbsem_matrix_input_v2.cell.manifest.json",
    "validation/cbsem_matrix_input_v2_oracle.py",
    "validation/cbsem_matrix_input_v2_qualification_factory.py",
    "validation/qualification_spec_v2.py",
    "validation/qualification_v2/cbsem_matrix_input_v2.qualification.json",
    "validation/qualification_v2/qualification_spec_v2.schema.json",
    "validation/test_cbsem_matrix_input_v2_oracle.py",
    "validation/test_cbsem_matrix_input_v2_qualification_factory.py",
)


def _axis(identifier: str, label: str, values: tuple[tuple[str, str], ...]) -> dict[str, Any]:
    return {
        "id": identifier,
        "label": label,
        "values": [
            {"id": value_id, "description": description}
            for value_id, description in values
        ],
    }


def _profile(
    identifier: str,
    description: str,
    *,
    rows: int,
    indicators: int,
    constructs: int,
) -> dict[str, Any]:
    return {
        "id": identifier,
        "description": description,
        "applicability": "required",
        "not_applicable_reason": None,
        "workload": {
            "rows": rows,
            "indicators": indicators,
            "constructs": constructs,
            "resamples": 0,
            "groups": 1,
            "candidate_models": 1,
        },
    }


def _combination(
    identifier: str,
    profile_id: str,
    coverage: str,
    purpose: str,
    selections: dict[str, list[str]],
    stressed_dimensions: Iterable[str] = (),
) -> dict[str, Any]:
    return {
        "id": identifier,
        "profile_id": profile_id,
        "coverage": coverage,
        "purpose": purpose,
        "stressed_dimensions": list(stressed_dimensions),
        "selections": selections,
    }


def _budget(
    profile_id: str,
    hardware_class_id: str,
    elapsed: float,
    memory: int,
    result_bytes: int,
) -> dict[str, Any]:
    return {
        "profile_id": profile_id,
        "hardware_class_id": hardware_class_id,
        "maximum_elapsed_seconds": elapsed,
        "maximum_peak_working_set_bytes": memory,
        "maximum_result_bytes": result_bytes,
        "maximum_cancellation_latency_seconds": 1.0,
    }


def build_spec() -> dict[str, Any]:
    estimand_ids = (
        "canonical_moment",
        "ml_parameters",
        "optimizer_solution",
    )
    axes = [
        _axis(
            "model_topology",
            "CB-SEM topology",
            (
                ("single_factor_cfa", "Marker-identified common-factor CFA."),
                ("recursive_latent_sem", "Recursive multi-factor latent SEM."),
            ),
        ),
        _axis(
            "measurement_model",
            "Measurement configuration",
            (
                ("three_indicator_marker", "Hand-checkable three-indicator marker block."),
                ("multiple_reflective_blocks", "Multiple continuous reflective common-factor blocks."),
            ),
        ),
        _axis(
            "data_distribution",
            "Moment conditioning",
            (
                ("gaussian_well_conditioned", "Gaussian positive-definite moments."),
                ("mixed_scale_near_singular", "Mixed scales and difficult positive-definite conditioning."),
            ),
        ),
        _axis(
            "missingness",
            "Missing-data behavior",
            (
                ("complete", "No missing value reaches the estimator."),
                ("raw_listwise_or_matrix_rejection", "Raw listwise deletion or typed matrix missing-value rejection."),
            ),
        ),
        _axis(
            "input_type",
            "Input representation",
            (
                ("raw_rows", "Raw complete observations converted with denominator n."),
                ("covariance_matrix", "Covariance matrix with explicit sample size and denominator."),
                ("scaled_correlation_matrix", "Correlation matrix with explicit positive scales."),
            ),
        ),
        _axis(
            "workload",
            "Execution workload",
            (
                ("point_estimation", "One immutable ML point-estimation run."),
                ("repeated_equivalence_matrix", "Repeated representation, order, and retry comparisons."),
            ),
        ),
        _axis(
            "workers",
            "Worker configuration",
            (
                ("one_worker", "The bounded deterministic ML execution worker."),
                ("alternate_worker_request", "An alternate request must preserve results or fail before execution."),
            ),
        ),
    ]
    profiles = [
        _profile(
            "micro_exact",
            "Three-indicator one-factor hand case and exhaustive typed boundaries.",
            rows=10,
            indicators=3,
            constructs=1,
        ),
        _profile(
            "applied",
            "Typical research covariance or scaled-correlation recursive SEM.",
            rows=1_000,
            indicators=30,
            constructs=10,
        ),
        _profile(
            "large",
            "Large routine matrix-input model for scheduling and result-size checks.",
            rows=10_000,
            indicators=80,
            constructs=20,
        ),
        _profile(
            "maximum_axis",
            "Separate maximum sample-size, indicator, and construct axes.",
            rows=100_000,
            indicators=300,
            constructs=100,
        ),
        _profile(
            "compound_stress",
            "Combined high sample metadata and dense model/result dimensions.",
            rows=50_000,
            indicators=150,
            constructs=50,
        ),
    ]
    all_selections = {
        axis["id"]: [value["id"] for value in axis["values"]] for axis in axes
    }
    first_selections = {
        axis["id"]: [axis["values"][0]["id"]] for axis in axes
    }
    second_selections = {
        axis["id"]: [axis["values"][-1]["id"]] for axis in axes
    }
    combinations = [
        _combination(
            "applied_pairwise_all_values",
            "applied",
            "pairwise",
            "One preregistered pairwise matrix covers every value pair across all axes.",
            all_selections,
        ),
        _combination(
            "micro_hand_and_boundaries",
            "micro_exact",
            "targeted",
            "Exact population, representation, gradient, stable-ID, and typed failure microcases.",
            first_selections,
        ),
        _combination(
            "large_difficult_matrix",
            "large",
            "targeted",
            "Large mixed-scale recursive matrix case with repeated execution.",
            second_selections,
        ),
        *[
            _combination(
                f"maximum_{dimension}",
                "maximum_axis",
                "targeted",
                f"Stress only the declared {dimension} maximum while other dimensions remain applied.",
                all_selections,
                (dimension,),
            )
            for dimension in ("rows", "indicators", "constructs")
        ],
        _combination(
            "compound_rows_indicators_constructs",
            "compound_stress",
            "compound",
            "Stress sample metadata, matrix width, and structural breadth together.",
            all_selections,
            ("rows", "indicators", "constructs"),
        ),
    ]
    oracles = [
        {
            "id": "bollen_ml_covariance_structure",
            "kind": "primary_literature",
            "citation": "Bollen, K. A. (1989), Structural Equations with Latent Variables, Wiley, DOI 10.1002/9781118619179.",
            "locator": "https://doi.org/10.1002/9781118619179",
            "independence_group": "primary_sem_literature",
            "runtime_policy": "no_runtime_dependency",
            "implementation": None,
            "covered_estimand_ids": list(estimand_ids),
        },
        {
            "id": "three_indicator_closed_form_hand_case",
            "kind": "hand_calculation",
            "citation": "Marker-identified three-indicator covariance identities derived directly from Sigma = lambda phi lambda' + theta.",
            "locator": "validation/cbsem_matrix_input_v2_oracle.py",
            "independence_group": "closed_form_hand_derivation",
            "runtime_policy": "development_validation_only",
            "implementation": None,
            "covered_estimand_ids": list(estimand_ids),
        },
        {
            "id": "numpy_scipy_transparent_ml",
            "kind": "independent_implementation",
            "citation": "Transparent validation-only NumPy/SciPy ML covariance-discrepancy implementation with analytic-gradient audit.",
            "locator": "validation/cbsem_matrix_input_v2_oracle.py",
            "independence_group": "numpy_scipy_validation_oracle",
            "runtime_policy": "development_validation_only",
            "implementation": {
                "name": oracle.ORACLE_VERSION,
                "version": oracle.ORACLE_VERSION,
                "maintainer": "QuickPLS validation-only independent oracle",
            },
            "covered_estimand_ids": list(estimand_ids),
        },
    ]
    preprocessing = [
        {
            "id": "bind_canonical_variable_order",
            "order": 0,
            "operation": "Resolve every observed variable to one exact source column and reorder matrices to canonical model order.",
            "parameters": {"duplicate_sources": "error", "missing_sources": "error"},
            "applies_to": ["raw_rows", "covariance_matrix", "scaled_correlation_matrix"],
        },
        {
            "id": "apply_raw_listwise_policy",
            "order": 1,
            "operation": "For raw input only, omit rows with any bound missing cell; matrix cells may not be missing.",
            "parameters": {"raw_policy": "listwise", "matrix_missing": "error"},
            "applies_to": ["raw_rows", "matrix_cells"],
        },
        {
            "id": "form_or_accept_source_moments",
            "order": 2,
            "operation": "Form raw covariance with denominator n or accept the declared covariance/correlation matrix.",
            "parameters": {"raw_denominator": "maximum_likelihood_n"},
            "applies_to": ["canonical_ml_covariance"],
        },
        {
            "id": "apply_correlation_scales",
            "order": 3,
            "operation": "Convert correlation to covariance using one explicit finite positive standard deviation per variable.",
            "parameters": {"implicit_unit_scales": False},
            "applies_to": ["scaled_correlation_matrix"],
        },
        {
            "id": "normalize_covariance_denominator",
            "order": 4,
            "operation": "Multiply an n-1 covariance by (n-1)/n; leave an n covariance unchanged.",
            "parameters": {"target_denominator": "maximum_likelihood_n"},
            "applies_to": ["covariance_matrix", "scaled_correlation_matrix"],
        },
        {
            "id": "validate_moment_matrix",
            "order": 5,
            "operation": "Require exact shape, finite symmetry, valid correlation bounds, and strict positive definiteness.",
            "parameters": {"positive_definite": "strict"},
            "applies_to": ["canonical_ml_covariance"],
        },
        {
            "id": "estimate_marker_ml_model",
            "order": 6,
            "operation": "Minimize log|Sigma| + trace(S Sigma^-1) - log|S| - p and retain objective, gradient, and convergence.",
            "parameters": {"estimator": "ml", "marker_loading": 1.0},
            "applies_to": ["ml_parameters", "optimizer_solution"],
        },
    ]
    model_predicates = [
        {
            "id": "common_factor_only",
            "expression": "all latent measurement estimands are common factors, not composites",
            "on_violation": "error",
            "diagnostic_code": "cbsem.matrix.model.common_factor_required",
        },
        {
            "id": "marker_identification",
            "expression": "each factor has one marker loading fixed exactly to one and at least two indicators",
            "on_violation": "error",
            "diagnostic_code": "cbsem.matrix.model.marker_identification_required",
        },
        {
            "id": "recursive_supported_relations",
            "expression": "the structural graph is recursive and every estimated covariance or residual variance is explicit",
            "on_violation": "error",
            "diagnostic_code": "cbsem.matrix.model.relation_unsupported",
        },
    ]
    data_predicates = [
        {
            "id": "supported_input_kind",
            "expression": "input is raw, covariance, or correlation with exact data binding",
            "on_violation": "error",
            "diagnostic_code": "cbsem.matrix.data.input_kind_invalid",
        },
        {
            "id": "exact_sample_size",
            "expression": "used sample size is an exact integer of at least ten and equals matrix metadata",
            "on_violation": "error",
            "diagnostic_code": "cbsem.matrix.data.sample_size_mismatch",
        },
        {
            "id": "valid_matrix_shape",
            "expression": "matrix is square and exactly matches the bound variable count",
            "on_violation": "error",
            "diagnostic_code": "cbsem.matrix.data.shape_invalid",
        },
        {
            "id": "strict_positive_definiteness",
            "expression": "canonical covariance is finite, symmetric, and strictly positive definite",
            "on_violation": "error",
            "diagnostic_code": "cbsem.matrix.data.non_positive_definite",
        },
        {
            "id": "correlation_scale_complete",
            "expression": "correlation input supplies one finite positive standard deviation per variable",
            "on_violation": "error",
            "diagnostic_code": "cbsem.matrix.data.correlation_scale_required",
        },
        {
            "id": "denominator_explicit",
            "expression": "matrix covariance denominator is sample_n_minus_one or maximum_likelihood_n",
            "on_violation": "error",
            "diagnostic_code": "cbsem.matrix.data.denominator_invalid",
        },
    ]
    scientific_contract = {
        "estimands": [
            {
                "id": "canonical_moment",
                "label": "Canonical ML covariance",
                "definition": "The exact denominator-n covariance matrix consumed by ML after raw formation or explicit matrix conversion.",
                "unit": "observed-variable covariance units",
                "output_ids": ["canonical_ml_covariance"],
            },
            {
                "id": "ml_parameters",
                "label": "Marker-identified ML parameters",
                "definition": "Unstandardized loadings, regressions, variances, and covariances bound to stable SemModelV4 parameter identities.",
                "unit": "parameter-specific raw scale",
                "output_ids": ["unstandardized_parameters", "stable_parameter_ids"],
            },
            {
                "id": "optimizer_solution",
                "label": "ML objective and convergence solution",
                "definition": "Final discrepancy objective, gradient norm, and typed convergence state for the admissible solution.",
                "unit": "objective, gradient norm, and status",
                "output_ids": ["objective", "gradient_norm", "convergence_status"],
            },
        ],
        "preprocessing": preprocessing,
        "model_predicates": model_predicates,
        "data_predicates": data_predicates,
        "oracles": oracles,
        "oracle_exception": None,
    }
    comparison_contract = {
        "outputs": [
            {
                "output_id": "canonical_ml_covariance",
                "rule": "matrix_norm",
                "rationale": "Equivalent raw, covariance, and explicitly scaled correlation inputs must agree at double-precision moment conversion scale.",
                "absolute_tolerance": 2e-12,
                "relative_tolerance": 1e-10,
                "norm": "maximum",
                "elementwise_tolerance": 2e-12,
            },
            {
                "output_id": "unstandardized_parameters",
                "rule": "abs_relative",
                "rationale": "The independent analytic-gradient optimizer and hand solution are compared on each identified parameter without widening for fixture deltas.",
                "absolute_tolerance": 2e-6,
                "relative_tolerance": 1e-6,
            },
            {
                "output_id": "stable_parameter_ids",
                "rule": "exact",
                "rationale": "Scientific parameter identities are categorical and must never be fuzzy-matched.",
            },
            {
                "output_id": "objective",
                "rule": "abs_relative",
                "rationale": "The just-identified microcase has an approximately zero ML discrepancy; applied cases retain a relative component.",
                "absolute_tolerance": 2e-10,
                "relative_tolerance": 1e-8,
            },
            {
                "output_id": "gradient_norm",
                "rule": "abs_relative",
                "rationale": "Convergence must be accompanied by an independently evaluated analytic-gradient norm.",
                "absolute_tolerance": 2e-6,
                "relative_tolerance": 1e-6,
            },
            {
                "output_id": "convergence_status",
                "rule": "exact",
                "rationale": "Converged, nonconverged, and inadmissible states are typed categorical outcomes.",
            },
        ]
    }
    hardware = [
        {
            "id": "standard",
            "os_family": "windows",
            "architecture": "x86_64",
            "minimum_logical_cores": 6,
            "minimum_memory_gib": 16,
            "notes": "Product-finalization standard Windows reference class.",
        },
        {
            "id": "workstation",
            "os_family": "windows",
            "architecture": "x86_64",
            "minimum_logical_cores": 12,
            "minimum_memory_gib": 32,
            "notes": "Product-finalization workstation Windows reference class.",
        },
    ]
    standard_budgets = (
        ("micro_exact", 30.0, 512 * 1024**2, 16 * 1024**2),
        ("applied", 600.0, 4 * 1024**3, 128 * 1024**2),
        ("large", 3_600.0, 8 * 1024**3, 512 * 1024**2),
        ("maximum_axis", 7_200.0, 12 * 1024**3, 1024 * 1024**2),
        ("compound_stress", 7_200.0, 12 * 1024**3, 1024 * 1024**2),
    )
    workstation_budgets = (
        ("micro_exact", 30.0, 1024 * 1024**2, 16 * 1024**2),
        ("applied", 600.0, 8 * 1024**3, 128 * 1024**2),
        ("large", 3_600.0, 16 * 1024**3, 512 * 1024**2),
        ("maximum_axis", 7_200.0, 24 * 1024**3, 1024 * 1024**2),
        ("compound_stress", 7_200.0, 24 * 1024**3, 1024 * 1024**2),
    )
    operational_contract = {
        "performance": {
            "hardware_classes": hardware,
            "baseline_policy": {
                "warmup_runs": 1,
                "measured_runs": 5,
                "statistic": "median",
                "maximum_runtime_regression_percent": 20.0,
                "maximum_memory_regression_percent": 20.0,
            },
            "budgets": [
                *[
                    _budget(profile, "standard", elapsed, memory, result_bytes)
                    for profile, elapsed, memory, result_bytes in standard_budgets
                ],
                *[
                    _budget(profile, "workstation", elapsed, memory, result_bytes)
                    for profile, elapsed, memory, result_bytes in workstation_budgets
                ],
            ],
        },
        "archive": {
            "current_schema_version": 6,
            "readable_schema_versions": [1, 2, 3, 4, 5, 6],
            "writable_schema_versions": [6],
            "future_schema_policy": "verified_read_only",
            "corruption_cases": [
                "feature_identity",
                "method_version",
                "dataset_fingerprint",
                "checksum",
                "duplicate_entry",
                "malformed_payload",
                "legacy_reinterpretation",
                "interrupted_save",
            ],
        },
        "export": {
            "formats": ["csv", "xlsx", "html", "svg", "pdf", "png"],
            "semantic_readback_formats": ["csv", "xlsx", "html", "svg", "pdf"],
            "canonical_projection_id": "canonical_result_document_v2_cbsem_matrix_input_projection",
            "same_run_required": True,
            "provenance_required": True,
            "validation_witness_excluded": True,
        },
        "windows": {
            "package_kinds": ["installed", "portable"],
            "viewports": ["1024x700", "1280x720", "1440x900"],
            "display_scale_percent": [100, 125, 150, 200],
            "offline_required": True,
            "keyboard_only_required": True,
            "accessible_tables_required": True,
            "real_pointer_required": True,
        },
        "cancellation": {
            "required_for_potentially_long_operations": True,
            "maximum_latency_seconds": 1.0,
            "phases": [
                {"phase": "validate", "applicability": "required", "not_applicable_reason": None},
                {"phase": "estimate", "applicability": "required", "not_applicable_reason": None},
                {
                    "phase": "resample",
                    "applicability": "not_applicable",
                    "not_applicable_reason": "This point-estimation cell has no resampling phase.",
                },
                {
                    "phase": "compare",
                    "applicability": "not_applicable",
                    "not_applicable_reason": "This cell has no competing-model comparison phase.",
                },
                {"phase": "export", "applicability": "required", "not_applicable_reason": None},
            ],
            "no_partial_visible_result": True,
            "no_partial_committed_result": True,
            "archive_unchanged": True,
            "same_settings_retry": True,
        },
    }
    return {
        "schema_version": 2,
        "identity": {
            "qualification_id": QUALIFICATION_ID,
            "method_version": METHOD_VERSION,
            "execution_kind": "iterative",
            "potentially_long_running": True,
            "spec_frozen_at_utc": SPEC_FROZEN_AT_UTC,
            "capability_cell": {
                "registry_schema_version": 2,
                "capability_id": CAPABILITY_ID,
                "capability_version": METHOD_VERSION,
                "cell_id": CELL_ID,
            },
        },
        "migration": {
            "source_kind": "qualification_v1_manifest",
            "source_schema_version": 1,
            "source_manifest_path": "validation/methods/cbsem_ml_v1.manifest.json",
            "status": "compatibility_only",
            "unresolved_items": [
                "The independent work report has no frozen current-product numerical comparison.",
                "A second maintained external SEM implementation is unavailable locally and no approved oracle exception exists.",
                "Qualification-sized generative, product adversarial, archive/export, frontend, packaged Windows, performance, soak, and scientific-review evidence are missing.",
            ],
        },
        "scientific_contract": scientific_contract,
        "scenario_contract": {
            "axes": axes,
            "complexity_profiles": profiles,
            "mandatory_combinations": combinations,
            "monte_carlo_policy": {
                "confidence_level": 0.95,
                "maximum_half_width": 0.01,
                "failed_fits_in_denominator": True,
            },
        },
        "comparison_contract": comparison_contract,
        "operational_contract": operational_contract,
        "evidence_contract": {
            "required_roles": list(EXPECTED_REQUIRED_ROLES),
            "receipt_contract": {
                "hash_algorithm": "sha256",
                "identity_fields": [
                    "qualification_id",
                    "capability_id",
                    "cell_id",
                    "method_version",
                    "source_set_sha256",
                    "scenario_set_sha256",
                    "build_fingerprint",
                ],
                "source_descriptors_required": True,
                "hardware_fingerprint_required": True,
                "scenario_set_hash_required": True,
            },
            "receipts": [],
        },
    }


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def source_descriptors() -> list[dict[str, Any]]:
    rows = []
    for relative in SOURCE_PATHS:
        path = ROOT / relative
        if not path.is_file():
            raise FileNotFoundError(f"qualification source missing: {relative}")
        rows.append(
            {
                "path": relative,
                "size_bytes": path.stat().st_size,
                "sha256": sha256_file(path),
            }
        )
    return rows


def _oracle_work() -> dict[str, Any]:
    expected = oracle.build_report()
    if not ORACLE_REPORT_PATH.is_file():
        return {"exists": False, "current": False, "passed_work_checks": False}
    actual = strict_load_json(ORACLE_REPORT_PATH)
    return {
        "exists": True,
        "current": actual == expected,
        "passed_work_checks": actual == expected and actual["passed_work_checks"] is True,
        "qualification_role_satisfied": actual.get("qualification_role_satisfied") is True,
        "receipt_eligible": actual.get("receipt_eligible") is True,
        "path": ORACLE_REPORT_PATH.relative_to(ROOT).as_posix(),
        "sha256": sha256_file(ORACLE_REPORT_PATH),
        "blockers": actual.get("blockers", []),
    }


def build_audit() -> dict[str, Any]:
    result = validate_spec_path(
        SPEC_PATH,
        repository_root=ROOT,
        registry_path=REGISTRY_PATH,
        require_registry=True,
    )
    oracle_work = _oracle_work()
    work_roles = {
        "oracle_independence": oracle_work,
        "generative_recovery": oracle_work,
        "adversarial_boundaries": oracle_work,
    }
    role_matrix = []
    for role in EXPECTED_REQUIRED_ROLES:
        if role == "method_contract":
            status = "work_evidence_only" if result["passed"] else "blocked"
            work = {
                "spec_schema_and_semantics_passed": result["passed"],
                "qualification_ready": result["qualification_ready"],
            }
        elif role in work_roles:
            status = (
                "work_evidence_only"
                if work_roles[role]["passed_work_checks"]
                else "blocked"
            )
            work = work_roles[role]
        else:
            status = "blocked"
            work = {
                "exists": False,
                "reason": "immutable_product_execution_envelope_missing",
            }
        role_matrix.append(
            {
                "role": role,
                "status": status,
                "work_evidence": work,
                "candidate_receipt_emitted": False,
            }
        )
    sources = source_descriptors()
    blockers = [
        "registered_matrix_input_cell_evidence_absent",
        "migration_status_compatibility_only",
        "no_frozen_current_product_numerical_comparison",
        "second_maintained_external_sem_reference_unavailable",
        "qualification_sized_generative_campaign_missing",
        "product_adversarial_execution_envelope_missing",
        "archive_export_frontend_packaged_windows_performance_soak_and_scientific_review_missing",
    ]
    return {
        "schema_version": 1,
        "report_kind": "cbsem_matrix_input_v2_qualification_factory_audit",
        "qualification_id": QUALIFICATION_ID,
        "capability_id": CAPABILITY_ID,
        "cell_id": CELL_ID,
        "method_version": METHOD_VERSION,
        "spec_path": SPEC_PATH.relative_to(ROOT).as_posix(),
        "spec_passed": result["passed"],
        "spec_qualification_ready": result["qualification_ready"],
        "spec_migration_status": build_spec()["migration"]["status"],
        "scenario_set_sha256": canonical_sha256(build_spec()["scenario_contract"]),
        "source_artifacts": sources,
        "source_set_sha256": canonical_sha256(sources),
        "role_matrix": role_matrix,
        "candidate_receipt_descriptors": [],
        "qualification_ready": False,
        "promotion_allowed": False,
        "registry_mutated": False,
        "manifest_mutated": False,
        "qualification_spec_receipts_mutated": False,
        "blockers": blockers,
    }


def _write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )


def write_factory_artifacts() -> None:
    _write_json(SPEC_PATH, build_spec())
    oracle.write_report(ORACLE_REPORT_PATH)
    _write_json(AUDIT_PATH, build_audit())


def verify_checked_in_factory() -> dict[str, Any]:
    errors = []
    expected_spec = build_spec()
    if not SPEC_PATH.is_file() or strict_load_json(SPEC_PATH) != expected_spec:
        errors.append("qualification_spec_missing_or_stale")
    if not ORACLE_REPORT_PATH.is_file() or strict_load_json(
        ORACLE_REPORT_PATH
    ) != oracle.build_report():
        errors.append("oracle_work_report_missing_or_stale")
    if not errors:
        expected_audit = build_audit()
        if not AUDIT_PATH.is_file() or strict_load_json(AUDIT_PATH) != expected_audit:
            errors.append("qualification_factory_audit_missing_or_stale")
    return {"passed": not errors, "errors": errors}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.write:
        write_factory_artifacts()
    result = verify_checked_in_factory()
    audit = strict_load_json(AUDIT_PATH) if AUDIT_PATH.is_file() else None
    print(
        json.dumps(
            {
                "passed": result["passed"],
                "errors": result["errors"],
                "qualification_ready": audit.get("qualification_ready") if audit else False,
                "promotion_allowed": audit.get("promotion_allowed") if audit else False,
                "candidate_receipts": len(audit.get("candidate_receipt_descriptors", [])) if audit else 0,
                "blockers": audit.get("blockers", []) if audit else [],
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
