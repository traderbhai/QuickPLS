#!/usr/bin/env python3
"""Fail-closed QualificationSpec V2 work-evidence factory for MICOM v3.

This factory freezes the MICOM-only scientific and operational contract and
audits the transparent NumPy work report.  It never executes QuickPLS, edits
CapabilityRegistryV2, changes a method-promotion manifest, attaches immutable
qualification receipts, or grants promotion authority.
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
SPEC_PATH = VALIDATION / "qualification_v2" / "micom_v3.qualification.json"
REGISTRY_PATH = VALIDATION / "capabilities" / "capability_registry_v2.json"
MANIFEST_PATH = VALIDATION / "methods" / "micom_permutation_mga_v3.manifest.json"
AUDIT_PATH = (
    VALIDATION
    / "results"
    / "method_factory"
    / "micom_v3"
    / "qualification_factory_audit.json"
)
ORACLE_REPORT_PATH = (
    VALIDATION
    / "results"
    / "method_factory"
    / "micom_v3"
    / "work"
    / "independent_oracle.json"
)

sys.path.insert(0, str(VALIDATION))

import micom_v3_oracle as oracle  # noqa: E402
from qualification_spec_v2 import (  # noqa: E402
    canonical_sha256,
    strict_load_json,
    validate_spec_path,
)


QUALIFICATION_ID = "qpls3.groups.micom.qualification_v2"
CAPABILITY_ID = "smartpls.micom"
CELL_ID = "qpls3.groups.micom_permutation_mga"
# CapabilityRegistryV2 currently shares this candidate version with permutation
# MGA.  The split is an explicit blocker, not an authority to qualify MGA.
METHOD_VERSION = "pls_mga_permutation_v3"
SPEC_FROZEN_AT_UTC = "2026-08-15T06:30:00Z"
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
    "docs/methods/MICOM_V3.md",
    "validation/micom_v3_oracle.py",
    "validation/micom_v3_qualification_factory.py",
    "validation/micom_mga_v3_reference.py",
    "validation/micom_v2_reference.py",
    "validation/results/micom_v2_reference.csv",
    "validation/methods/micom_permutation_mga_v3.manifest.json",
    "validation/capabilities/capability_registry_v2.json",
    "validation/qualification_spec_v2.py",
    "validation/qualification_v2/micom_v3.qualification.json",
    "validation/qualification_v2/qualification_spec_v2.schema.json",
    "validation/test_micom_v3_oracle.py",
    "validation/test_micom_v3_qualification_factory.py",
)


def _axis(
    identifier: str,
    label: str,
    values: tuple[tuple[str, str], ...],
) -> dict[str, Any]:
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
    resamples: int,
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
            "resamples": resamples,
            "groups": 2,
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


def _comparison(
    output_id: str,
    rule: str,
    rationale: str,
    **parameters: Any,
) -> dict[str, Any]:
    return {
        "output_id": output_id,
        "rule": rule,
        "rationale": rationale,
        **parameters,
    }


def build_spec() -> dict[str, Any]:
    estimand_ids = (
        "configural_review",
        "compositional_invariance",
        "mean_equality",
        "variance_equality",
        "permutation_accounting",
    )
    axes = [
        _axis(
            "model_topology",
            "PLS composite topology",
            (
                ("single_composite", "One hand-checkable composite."),
                ("recursive_multi_composite", "Several composites in one recursive PLS model."),
            ),
        ),
        _axis(
            "measurement_model",
            "Composite construction",
            (
                ("mode_a", "Mode A composite blocks."),
                ("mode_b_or_mixed", "Mode B or mixed supported composite blocks."),
            ),
        ),
        _axis(
            "data_distribution",
            "Distribution and group shape",
            (
                ("gaussian_balanced", "Gaussian indicators and balanced groups."),
                ("nongaussian_imbalanced", "Nongaussian indicators and bounded unequal groups."),
            ),
        ),
        _axis(
            "missingness",
            "Complete-case treatment",
            (
                ("complete", "No missing value reaches the estimator."),
                ("declared_listwise", "The same listwise treatment is applied to both groups."),
            ),
        ),
        _axis(
            "input_type",
            "Input representation",
            (
                ("raw_observations", "Raw observations with stable case and group identities."),
                ("matrix_rejected", "Covariance/correlation-only input is rejected before MICOM."),
            ),
        ),
        _axis(
            "workload",
            "Permutation workload",
            (
                ("initial_500", "Initial 500-permutation diagnostic run."),
                ("final_5000_10000", "Final 5,000 or 10,000 indexed permutations."),
            ),
        ),
        _axis(
            "workers",
            "Worker count",
            (
                ("one_worker", "Single-worker deterministic execution."),
                ("multiple_workers", "Supported parallel execution with identical indexed results."),
            ),
        ),
    ]
    profiles = [
        _profile(
            "micro_exact",
            "Closed-form fixed-weight and typed-boundary cases.",
            rows=20,
            indicators=2,
            constructs=1,
            resamples=19,
        ),
        _profile(
            "applied",
            "Typical two-group published-research model at final inference size.",
            rows=500,
            indicators=30,
            constructs=8,
            resamples=5_000,
        ),
        _profile(
            "large",
            "Large two-group MICOM model with full ledgers.",
            rows=10_000,
            indicators=80,
            constructs=20,
            resamples=10_000,
        ),
        _profile(
            "maximum_axis",
            "Separate maximum row, indicator, construct, and permutation axes.",
            rows=100_000,
            indicators=300,
            constructs=100,
            resamples=10_000,
        ),
        _profile(
            "compound_stress",
            "Combined large model, row pool, and final permutation workload.",
            rows=50_000,
            indicators=150,
            constructs=50,
            resamples=10_000,
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
            "One preregistered pairwise design covers every value pair.",
            all_selections,
        ),
        _combination(
            "micro_hand_boundaries",
            "micro_exact",
            "targeted",
            "Hand calculations, hierarchy, orientation, and typed failures.",
            first_selections,
        ),
        _combination(
            "large_group_ledger",
            "large",
            "targeted",
            "Large no-retry ledger and deterministic parallel scheduling.",
            second_selections,
        ),
        *[
            _combination(
                f"maximum_{dimension}",
                "maximum_axis",
                "targeted",
                f"Stress only the {dimension} maximum while other dimensions remain applied.",
                all_selections,
                (dimension,),
            )
            for dimension in ("rows", "indicators", "constructs", "resamples")
        ],
        _combination(
            "compound_rows_indicators_constructs_resamples",
            "compound_stress",
            "compound",
            "Stress rows, model breadth, and permutation count together.",
            all_selections,
            ("rows", "indicators", "constructs", "resamples"),
        ),
    ]
    preprocessing = [
        {
            "id": "require_qualitative_configural_review",
            "order": 0,
            "operation": "Require researcher review of indicator meaning, coding, treatment, specification, and algorithm settings; do not infer Step 1 statistically.",
            "parameters": {"computed": False, "attestation_required": True},
            "applies_to": ["micom_step_1"],
        },
        {
            "id": "bind_two_groups_and_stable_cases",
            "order": 1,
            "operation": "Bind exactly two selected group values and one unique stable case identity per included observation.",
            "parameters": {"groups": 2, "duplicate_case_id": "error"},
            "applies_to": ["selected_raw_pool"],
        },
        {
            "id": "apply_identical_complete_case_policy",
            "order": 2,
            "operation": "Apply the same declared complete-case treatment to both groups and retain exact inclusion counts.",
            "parameters": {"silent_zero_fill": False},
            "applies_to": ["selected_raw_pool"],
        },
        {
            "id": "canonicalize_rows_and_group_direction",
            "order": 3,
            "operation": "Sort by stable case identity and derive a canonical unordered group pair while preserving requested A-minus-B reporting direction.",
            "parameters": {"row_order_invariant": True, "group_swap_coupled": True},
            "applies_to": ["permutation_plan"],
        },
        {
            "id": "fit_pooled_and_observed_groups",
            "order": 4,
            "operation": "Fit the same declared PLS composite model to pooled data and both observed groups.",
            "parameters": {"settings_identical": True},
            "applies_to": ["pooled_fit", "observed_group_fits"],
        },
        {
            "id": "align_composite_orientation",
            "order": 5,
            "operation": "Align each group-specific composite score to its pooled score reference; reject an undefined orthogonal orientation.",
            "parameters": {"reference": "pooled_composite_score"},
            "applies_to": ["micom_step_2"],
        },
        {
            "id": "generate_size_preserving_indexed_partitions",
            "order": 6,
            "operation": "Assign observations without replacement, preserve both group sizes, and attempt each requested replicate exactly once.",
            "parameters": {"retry_policy": "none", "replacement": False},
            "applies_to": ["permutation_plan", "permutation_ledger"],
        },
        {
            "id": "calculate_compositional_invariance",
            "order": 7,
            "operation": "Apply both group-specific weight vectors to the pooled indicator matrix and compare their score correlation with the lower-tail permutation distribution.",
            "parameters": {"tail": "lower", "null_correlation": 1.0},
            "applies_to": ["micom_step_2"],
        },
        {
            "id": "calculate_mean_and_variance_equality",
            "order": 8,
            "operation": "Split pooled-model composite scores by observed/permuted membership and compare obtained mean differences and log variance ratios with two-sided permutation intervals.",
            "parameters": {"variance_statistic": "log_ratio"},
            "applies_to": ["micom_step_3"],
        },
        {
            "id": "derive_hierarchical_decisions_and_accounting",
            "order": 9,
            "operation": "Declare partial invariance only after Steps 1-2, full invariance only after Step 3, and retain every requested replicate outcome.",
            "parameters": {"failed_replicates_retained": True},
            "applies_to": ["micom_decisions", "permutation_ledger"],
        },
    ]
    estimands = [
        {
            "id": "configural_review",
            "label": "Configural invariance review",
            "definition": "A qualitative prerequisite covering equivalent indicator meaning, specification, coding, data treatment, and algorithm settings across groups.",
            "unit": "review status and checklist",
            "output_ids": ["step1_review_status"],
        },
        {
            "id": "compositional_invariance",
            "label": "Compositional invariance",
            "definition": "Correlation between pooled-data composite scores formed with Group A and Group B weights, with lower-tail permutation inference.",
            "unit": "correlation, probability, quantile, and decision",
            "output_ids": [
                "compositional_correlation",
                "compositional_lower_quantile",
                "compositional_p_value",
                "compositional_decision",
            ],
        },
        {
            "id": "mean_equality",
            "label": "Equality of composite means",
            "definition": "A-minus-B difference in pooled-model composite-score means compared with its permutation interval.",
            "unit": "standardized composite-score units",
            "output_ids": ["mean_difference", "mean_interval", "mean_p_value", "mean_equality_decision"],
        },
        {
            "id": "variance_equality",
            "label": "Equality of composite variances",
            "definition": "Log of the Group A to Group B pooled-score sample-variance ratio compared with its permutation interval.",
            "unit": "log variance ratio",
            "output_ids": ["log_variance_ratio", "variance_interval", "variance_p_value", "variance_equality_decision"],
        },
        {
            "id": "permutation_accounting",
            "label": "Indexed permutation accounting",
            "definition": "Every requested replicate identity, size-preserving partition hash, Step 2/3 status, and stable failure code with no replacement retry.",
            "unit": "typed ledger and integer counts",
            "output_ids": ["permutation_ledger", "permutation_accounting"],
        },
    ]
    model_predicates = [
        {
            "id": "composite_estimand_required",
            "expression": "every assessed construct is an explicit supported PLS composite estimand",
            "on_violation": "error",
            "diagnostic_code": "micom.model.composite_required",
        },
        {
            "id": "identical_group_model",
            "expression": "both groups use identical indicator sets, model relations, preprocessing, and PLS settings",
            "on_violation": "error",
            "diagnostic_code": "micom.model.configural_mismatch",
        },
        {
            "id": "configural_review_confirmed",
            "expression": "a named researcher confirms every qualitative Step 1 item before computation",
            "on_violation": "error",
            "diagnostic_code": "micom.model.configural_review_required",
        },
    ]
    data_predicates = [
        {
            "id": "raw_complete_input",
            "expression": "input is finite raw observations after one identical declared complete-case policy",
            "on_violation": "error",
            "diagnostic_code": "micom.data.raw_complete_required",
        },
        {
            "id": "exactly_two_selected_groups",
            "expression": "the run selects exactly two nonempty group values and preserves both observed sizes",
            "on_violation": "error",
            "diagnostic_code": "micom.data.two_groups_required",
        },
        {
            "id": "minimum_group_size",
            "expression": "each group has at least ten complete model cases",
            "on_violation": "error",
            "diagnostic_code": "micom.data.group_too_small",
        },
        {
            "id": "bounded_group_imbalance",
            "expression": "the larger-to-smaller group ratio is at most ten under the bounded candidate policy",
            "on_violation": "error",
            "diagnostic_code": "micom.data.extreme_group_imbalance",
        },
        {
            "id": "stable_case_identity",
            "expression": "every selected observation has one unique stable case identity",
            "on_violation": "error",
            "diagnostic_code": "micom.data.case_identity_invalid",
        },
    ]
    oracles = [
        {
            "id": "henseler_ringle_sarstedt_2016",
            "kind": "primary_literature",
            "citation": "Henseler, J., Ringle, C. M., and Sarstedt, M. (2016), Testing measurement invariance of composites using partial least squares, DOI 10.1108/IMR-09-2014-0304.",
            "locator": "https://doi.org/10.1108/IMR-09-2014-0304",
            "independence_group": "primary_micom_method",
            "runtime_policy": "no_runtime_dependency",
            "implementation": None,
            "covered_estimand_ids": list(estimand_ids),
        },
        {
            "id": "closed_form_fixed_weight_partition_case",
            "kind": "hand_calculation",
            "citation": "Closed-form fixed-weight c=1 case plus exhaustive finite mean/log-variance partition identities.",
            "locator": "validation/test_micom_v3_oracle.py",
            "independence_group": "closed_form_micom_hand_case",
            "runtime_policy": "development_validation_only",
            "implementation": None,
            "covered_estimand_ids": list(estimand_ids),
        },
        {
            "id": "transparent_numpy_micom_v3",
            "kind": "independent_implementation",
            "citation": "Transparent validation-only NumPy PLS MICOM oracle with stable case-hash partitions, orientation alignment, and no-retry ledger.",
            "locator": "validation/micom_v3_oracle.py",
            "independence_group": "numpy_micom_validation_oracle",
            "runtime_policy": "development_validation_only",
            "implementation": {
                "name": oracle.ORACLE_VERSION,
                "version": oracle.ORACLE_VERSION,
                "maintainer": "QuickPLS validation-only independent oracle",
            },
            "covered_estimand_ids": list(estimand_ids),
        },
    ]
    comparison_contract = {
        "outputs": [
            _comparison("step1_review_status", "exact", "Step 1 is categorical review provenance, never a fuzzy numerical result."),
            *[
                _comparison(
                    output_id,
                    "abs_relative",
                    "The product and frozen indexed oracle must agree at double-precision reporting tolerance.",
                    absolute_tolerance=2e-6,
                    relative_tolerance=1e-6,
                )
                for output_id in (
                    "compositional_correlation",
                    "compositional_lower_quantile",
                    "compositional_p_value",
                    "mean_difference",
                    "mean_p_value",
                    "log_variance_ratio",
                    "variance_p_value",
                )
            ],
            *[
                _comparison(
                    output_id,
                    "matrix_norm",
                    "Both endpoints for every construct must agree in stable construct order.",
                    absolute_tolerance=2e-6,
                    relative_tolerance=1e-6,
                    norm="maximum",
                    elementwise_tolerance=2e-6,
                )
                for output_id in ("mean_interval", "variance_interval")
            ],
            *[
                _comparison(output_id, "exact", "Scientific decisions and ledger identities are categorical exact outputs.")
                for output_id in (
                    "compositional_decision",
                    "mean_equality_decision",
                    "variance_equality_decision",
                    "permutation_ledger",
                    "permutation_accounting",
                )
            ],
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
        ("applied", 1_800.0, 8 * 1024**3, 256 * 1024**2),
        ("large", 7_200.0, 12 * 1024**3, 1024 * 1024**2),
        ("maximum_axis", 14_400.0, 12 * 1024**3, 2 * 1024**3),
        ("compound_stress", 14_400.0, 12 * 1024**3, 2 * 1024**3),
    )
    workstation_budgets = (
        ("micro_exact", 30.0, 1024 * 1024**2, 16 * 1024**2),
        ("applied", 900.0, 12 * 1024**3, 256 * 1024**2),
        ("large", 3_600.0, 24 * 1024**3, 1024 * 1024**2),
        ("maximum_axis", 7_200.0, 24 * 1024**3, 2 * 1024**3),
        ("compound_stress", 7_200.0, 24 * 1024**3, 2 * 1024**3),
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
            "canonical_projection_id": "canonical_result_document_v2_micom_projection",
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
                {"phase": "resample", "applicability": "required", "not_applicable_reason": None},
                {
                    "phase": "compare",
                    "applicability": "not_applicable",
                    "not_applicable_reason": "MICOM does not compare saved competing models.",
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
            "execution_kind": "stochastic",
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
            "source_manifest_path": "validation/methods/micom_permutation_mga_v3.manifest.json",
            "status": "compatibility_only",
            "unresolved_items": [
                "CapabilityRegistryV2 and the v1 manifest still combine MICOM with structural-path permutation MGA; MICOM needs an independently qualified option cell or explicit subcell contract.",
                "The exact micom_v3_1 product path now implements the exactly-once no-retry contract, but no frozen product-to-oracle comparison receipt is attached.",
                "No current product result is compared with the independent MICOM v3.1 oracle under the same canonical partition plan.",
                "The candidate has not qualified Mode B or mixed composites, controls, interactions, higher-order constructs, missing-data policies, arbitrary declared groups, or deterministic pairwise orchestration.",
                "Qualification-sized generative calibration, power, failure-rate, adversarial, archive/export, frontend, packaged Windows, accessibility, performance, soak, and independent scientific-review evidence is missing.",
            ],
        },
        "scientific_contract": {
            "estimands": estimands,
            "preprocessing": preprocessing,
            "model_predicates": model_predicates,
            "data_predicates": data_predicates,
            "oracles": oracles,
            "oracle_exception": None,
        },
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
            raise FileNotFoundError(f"MICOM qualification source missing: {relative}")
        rows.append(
            {
                "path": relative,
                "size_bytes": path.stat().st_size,
                "sha256": sha256_file(path),
            }
        )
    return rows


def _registry_cell() -> tuple[dict[str, Any], dict[str, Any]]:
    registry = strict_load_json(REGISTRY_PATH)
    for capability in registry["capabilities"]:
        if capability["capability_id"] != CAPABILITY_ID:
            continue
        for cell in capability["option_cells"]:
            if cell["cell_id"] == CELL_ID:
                return capability, cell
    raise ValueError("MICOM capability cell is missing from CapabilityRegistryV2")


def build_audit() -> dict[str, Any]:
    validation = validate_spec_path(
        SPEC_PATH,
        repository_root=ROOT,
        registry_path=REGISTRY_PATH,
        require_registry=True,
    )
    spec = strict_load_json(SPEC_PATH)
    manifest = strict_load_json(MANIFEST_PATH)
    work = strict_load_json(ORACLE_REPORT_PATH)
    capability, cell = _registry_cell()
    descriptors = source_descriptors()
    source_hash = canonical_sha256(descriptors)
    manifest_evidence = manifest["qualification"]["evidence"]
    manifest_empty = all(not rows for rows in manifest_evidence.values())
    checks = {
        "qualification_spec_valid_and_registry_linked": (
            validation["passed"]
            and validation["schema_valid"]
            and validation["semantic_valid"]
            and validation["registry_verified"]
        ),
        "qualification_spec_compatibility_only": (
            spec["migration"]["status"] == "compatibility_only"
            and bool(spec["migration"]["unresolved_items"])
        ),
        "qualification_receipts_empty": spec["evidence_contract"]["receipts"] == [],
        "work_report_passes_but_is_non_promotional": (
            work["passed"]
            and work["work_evidence_only"]
            and work["qualification_ready"] is False
            and work["promotion_requested"] is False
        ),
        "step1_is_review_only": work["checks"]["step1_is_review_not_computation"],
        "micom_is_not_mga_or_consistent_permutation": work["checks"]["micom_only_scope"],
        "permutations_are_exactly_once_no_retry": work["checks"]["exact_no_retry_accounting"],
        "metamorphic_and_typed_boundaries_pass": all(
            work["checks"][check]
            for check in (
                "group_swap_signed_reversal",
                "group_swap_probabilities_equal",
                "group_swap_decisions_equal",
                "row_reorder_invariant",
                "construct_declaration_reorder_invariant",
                "same_seed_repeat_exact",
                "different_seed_changes_plan",
                "typed_boundaries_exact",
            )
        ),
        "manifest_remains_absent_with_no_evidence": (
            manifest["qualification"]["declared_state"] == "absent"
            and manifest_empty
        ),
        "registry_remains_absent_labs": (
            capability["coverage_state"] == "absent"
            and capability["evidence_state"] == "absent"
            and capability["surface"] == "labs"
            and cell["coverage_state"] == "absent"
            and cell["evidence_state"] == "absent"
            and cell["surface"] == "labs"
            and cell["capability_version"] == METHOD_VERSION
        ),
    }
    role_matrix = []
    for role in EXPECTED_REQUIRED_ROLES:
        if role == "method_contract" and checks["qualification_spec_valid_and_registry_linked"]:
            status = "work_evidence_only"
            reasons = ["No immutable QualificationSpec V2 receipt is attached."]
        elif role == "oracle_independence" and checks["work_report_passes_but_is_non_promotional"]:
            status = "work_evidence_only"
            reasons = [
                "The transparent oracle is source work, not a current product comparison or immutable qualification execution receipt."
            ]
        elif role == "adversarial_boundaries" and checks["metamorphic_and_typed_boundaries_pass"]:
            status = "work_evidence_only"
            reasons = [
                "Source-level typed boundaries pass, but product, packaged, persistence, and scale boundaries have not run."
            ]
        else:
            status = "blocked"
            reasons = [f"Qualification receipt for {role} is absent."]
        role_matrix.append(
            {
                "role": role,
                "status": status,
                "candidate_receipt_emitted": False,
                "reasons": reasons,
            }
        )
    blockers = sorted(
        {
            *spec["migration"]["unresolved_items"],
            *work["remaining_blockers"],
            "No immutable QualificationSpec V2 receipt is attached for any required role.",
            "Independent scientific review is not recorded.",
        }
    )
    return {
        "schema_version": 1,
        "report_kind": "micom_v3_qualification_factory_audit",
        "qualification_id": QUALIFICATION_ID,
        "capability_id": CAPABILITY_ID,
        "cell_id": CELL_ID,
        "method_version": METHOD_VERSION,
        "spec_frozen_at_utc": SPEC_FROZEN_AT_UTC,
        "source_artifacts": descriptors,
        "source_set_sha256": source_hash,
        "scenario_set_sha256": canonical_sha256(spec["scenario_contract"]),
        "oracle_work_report": {
            "path": str(ORACLE_REPORT_PATH.relative_to(ROOT)).replace("\\", "/"),
            "size_bytes": ORACLE_REPORT_PATH.stat().st_size,
            "sha256": sha256_file(ORACLE_REPORT_PATH),
            "passed": work["passed"],
            "work_evidence_only": True,
        },
        "checks": checks,
        "passed": all(checks.values()),
        "role_matrix": role_matrix,
        "candidate_receipt_descriptors": [],
        "attached_receipt_count": 0,
        "registry_mutated": False,
        "manifest_mutated": False,
        "qualification_spec_receipts_mutated": False,
        "scientific_review_satisfied": False,
        "qualification_ready": False,
        "promotion_allowed": False,
        "remaining_blockers": blockers,
        "note": (
            "This is source-bound work evidence only. It cannot qualify MICOM, "
            "permutation MGA, consistent permutation, or the shared registry cell."
        ),
    }


def _write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )


def write_factory_artifacts(permutations: int = 39) -> dict[str, Any]:
    before = {
        path: sha256_file(path)
        for path in (REGISTRY_PATH, MANIFEST_PATH)
    }
    _write_json(SPEC_PATH, build_spec())
    oracle.write_work_report(ORACLE_REPORT_PATH, permutations)
    audit = build_audit()
    after = {path: sha256_file(path) for path in before}
    if before != after:
        raise RuntimeError("MICOM qualification factory mutated registry or manifest")
    _write_json(AUDIT_PATH, audit)
    return audit


def verify_checked_in_factory() -> dict[str, Any]:
    if not all(path.is_file() for path in (SPEC_PATH, ORACLE_REPORT_PATH, AUDIT_PATH)):
        raise FileNotFoundError("checked-in MICOM v3 qualification artifacts are missing")
    audit = strict_load_json(AUDIT_PATH)
    spec = strict_load_json(SPEC_PATH)
    work = strict_load_json(ORACLE_REPORT_PATH)
    validation = validate_spec_path(
        SPEC_PATH,
        repository_root=ROOT,
        registry_path=REGISTRY_PATH,
        require_registry=True,
    )
    errors = []
    current_sources = source_descriptors()
    if audit.get("source_artifacts") != current_sources:
        errors.append("factory_source_descriptors_stale")
    if audit.get("source_set_sha256") != canonical_sha256(current_sources):
        errors.append("factory_source_set_hash_stale")
    if audit.get("scenario_set_sha256") != canonical_sha256(spec["scenario_contract"]):
        errors.append("factory_scenario_hash_stale")
    descriptor = audit.get("oracle_work_report", {})
    if descriptor.get("sha256") != sha256_file(ORACLE_REPORT_PATH):
        errors.append("oracle_work_report_hash_stale")
    if descriptor.get("size_bytes") != ORACLE_REPORT_PATH.stat().st_size:
        errors.append("oracle_work_report_size_stale")
    if not work.get("passed") or not work.get("work_evidence_only"):
        errors.append("oracle_work_report_invalid")
    if not validation["passed"]:
        errors.append("qualification_spec_invalid")
    if spec["evidence_contract"]["receipts"]:
        errors.append("qualification_receipts_attached")
    if audit.get("candidate_receipt_descriptors"):
        errors.append("factory_emitted_candidate_receipt")
    if audit.get("qualification_ready") is not False or audit.get("promotion_allowed") is not False:
        errors.append("factory_makes_promotion_claim")
    if audit.get("scientific_review_satisfied") is not False:
        errors.append("factory_claims_scientific_review")
    if any(
        audit.get(field) is not False
        for field in (
            "registry_mutated",
            "manifest_mutated",
            "qualification_spec_receipts_mutated",
        )
    ):
        errors.append("factory_records_forbidden_state_mutation")
    return {
        "passed": not errors,
        "errors": errors,
        "work_evidence_roles": [
            row["role"]
            for row in audit["role_matrix"]
            if row["status"] == "work_evidence_only"
        ],
        "qualification_ready": False,
        "promotion_allowed": False,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--verify", action="store_true")
    parser.add_argument("--permutations", type=int, default=39)
    args = parser.parse_args()
    if args.write == args.verify:
        parser.error("select exactly one of --write or --verify")
    result = (
        write_factory_artifacts(args.permutations)
        if args.write
        else verify_checked_in_factory()
    )
    print(json.dumps(result, indent=2, sort_keys=True, allow_nan=False))
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
