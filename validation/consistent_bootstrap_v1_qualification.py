#!/usr/bin/env python3
"""Build and verify the frozen PLSc consistent-bootstrap QualificationSpec V2.

This is a contract scaffold, not qualification evidence.  The generated spec
is deliberately ``compatibility_only`` with no receipts, so validation can
confirm completeness of the preregistration without making the method
customer-executable or promotion-ready.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
OUTPUT = (
    VALIDATION
    / "qualification_v2"
    / "consistent_bootstrap_v1.qualification.json"
)
REGISTRY = VALIDATION / "capabilities" / "capability_registry_v2.json"
sys.path.insert(0, str(VALIDATION))

from qualification_spec_v2 import strict_load_json, validate_spec_document  # noqa: E402


FROZEN_AT = "2026-08-14T16:30:00Z"
QUALIFICATION_ID = "qpls3.inference.consistent_bootstrap.qualification_v2"
CAPABILITY_ID = "smartpls.consistent_bootstrapping"
CELL_ID = "qpls3.inference.consistent_bootstrap"
METHOD_VERSION = "plsc_bootstrap_v1"


def _axis(identifier: str, label: str, values: dict[str, str]) -> dict[str, Any]:
    return {
        "id": identifier,
        "label": label,
        "values": [
            {"id": value_id, "description": description}
            for value_id, description in values.items()
        ],
    }


AXIS_VALUES: dict[str, list[str]] = {
    "model_topology": ["two_construct_chain", "recursive_mediation_branch"],
    "measurement_model": ["reflective_two_item", "reflective_mixed_block_sizes"],
    "data_distribution": ["gaussian_well_conditioned", "skewed_heavy_tail_mixed_sign"],
    "missingness": ["complete", "listwise_mcar_five_percent"],
    "input_type": ["raw_rows", "summary_matrix_rejection"],
    "workload": ["all_refits_usable", "inadmissible_refits_retained"],
    "workers": ["one_worker", "reference_parallel_workers"],
}


def _selections(**overrides: list[str]) -> dict[str, list[str]]:
    selected = {axis: values[:] for axis, values in AXIS_VALUES.items()}
    selected.update(overrides)
    return selected


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
            "groups": 1,
            "candidate_models": 1,
        },
    }


def _combination(
    identifier: str,
    profile_id: str,
    coverage: str,
    purpose: str,
    *,
    stressed: list[str] | None = None,
    selections: dict[str, list[str]] | None = None,
) -> dict[str, Any]:
    return {
        "id": identifier,
        "profile_id": profile_id,
        "coverage": coverage,
        "purpose": purpose,
        "stressed_dimensions": stressed or [],
        "selections": selections or _selections(),
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


def _performance_budgets() -> list[dict[str, Any]]:
    profiles = {
        "micro_exact": (120.0, 512 * 1024**2, 16 * 1024**2),
        "applied": (900.0, 4 * 1024**3, 256 * 1024**2),
        "large": (3600.0, 12 * 1024**3, 1024 * 1024**2),
        "maximum_axis": (7200.0, 12 * 1024**3, 2 * 1024**3),
        "compound_stress": (7200.0, 12 * 1024**3, 2 * 1024**3),
    }
    budgets: list[dict[str, Any]] = []
    for hardware in ("standard", "workstation"):
        memory_multiplier = 1 if hardware == "standard" else 2
        for profile_id, (elapsed, memory, result_size) in profiles.items():
            budgets.append(
                {
                    "profile_id": profile_id,
                    "hardware_class_id": hardware,
                    "maximum_elapsed_seconds": elapsed,
                    "maximum_peak_working_set_bytes": memory * memory_multiplier,
                    "maximum_result_bytes": result_size,
                    "maximum_cancellation_latency_seconds": 1.0,
                }
            )
    return budgets


def build_spec() -> dict[str, Any]:
    parameter_outputs = [
        "plsc_parameter_estimates",
        "bootstrap_means",
        "bootstrap_biases",
        "bootstrap_standard_errors",
    ]
    percentile_outputs = [
        "normal_reference_t_statistics",
        "normal_reference_two_sided_p_values",
        "percentile_lower_bounds",
        "percentile_upper_bounds",
    ]
    bca_outputs = [
        "bca_bias_corrections",
        "bca_accelerations",
        "bca_lower_bounds",
        "bca_upper_bounds",
        "bca_unavailable_reasons",
    ]
    accounting_outputs = [
        "requested_usable_failed_counts",
        "replicate_ledger",
        "failed_replicate_reasons",
        "failed_jackknife_reasons",
        "method_plan_provenance",
        "parameter_value_digests",
    ]
    estimand_ids = [
        "plsc_parameter_resampling_distribution",
        "large_sample_and_percentile_inference",
        "conditional_bca_inference",
        "deterministic_execution_accounting",
    ]
    numerical_outputs = [
        *parameter_outputs,
        *percentile_outputs,
        *bca_outputs[:-1],
    ]
    exact_outputs = [bca_outputs[-1], *accounting_outputs]

    return {
        "schema_version": 2,
        "identity": {
            "qualification_id": QUALIFICATION_ID,
            "method_version": METHOD_VERSION,
            "execution_kind": "stochastic",
            "potentially_long_running": True,
            "spec_frozen_at_utc": FROZEN_AT,
            "capability_cell": {
                "registry_schema_version": 2,
                "capability_id": CAPABILITY_ID,
                "cell_id": CELL_ID,
                "capability_version": METHOD_VERSION,
            },
        },
        "migration": {
            "source_kind": "qualification_v1_manifest",
            "source_schema_version": 1,
            "source_manifest_path": "validation/methods/consistent_bootstrap_v1.manifest.json",
            "status": "compatibility_only",
            "unresolved_items": [
                "coverage.selectable_test_direction_missing",
                "coverage.selectable_interval_family_missing",
                "coverage.complete_measurement_assessment_inference_missing",
                "coverage.broader_plsc_model_shapes_and_defaults_missing",
                "evidence.full_plsc_independent_oracle_missing",
                "evidence.second_independently_maintained_full_plsc_oracle_missing",
                "evidence.preregistered_bias_coverage_and_failure_simulation_not_run",
                "evidence.adversarial_and_metamorphic_matrix_not_run",
                "evidence.archive_and_cross_format_readback_receipts_missing",
                "evidence.packaged_windows_accessibility_matrix_not_run",
                "evidence.performance_scale_and_soak_not_run",
                "evidence.independent_scientific_review_not_recorded",
                "persistence.successful_replicate_vectors_not_replayable_from_archive",
                "method.minimum_usable_fraction_requires_independent_justification",
            ],
        },
        "scientific_contract": {
            "estimands": [
                {
                    "id": "plsc_parameter_resampling_distribution",
                    "label": "Full-refit PLSc bootstrap parameter distribution",
                    "definition": "For every canonical rho_A, corrected construct correlation, loading, weight, path, direct/indirect/total effect, and corrected R-squared parameter, re-estimate plsc_v2 on each accepted indexed empirical case resample after deterministic sign alignment.",
                    "unit": "parameter-specific standardized PLSc scale",
                    "output_ids": parameter_outputs,
                },
                {
                    "id": "large_sample_and_percentile_inference",
                    "label": "Normal-reference diagnostics and percentile intervals",
                    "definition": "Use successful full-refit PLSc replicates to calculate the sample bootstrap standard error, two-sided standard-normal reference probability, and Type-7 percentile endpoints at the requested confidence level.",
                    "unit": "parameter scale, test statistic, and probability",
                    "output_ids": percentile_outputs,
                },
                {
                    "id": "conditional_bca_inference",
                    "label": "Conditional full-refit BCa intervals",
                    "definition": "Use midrank bias correction and full delete-one plsc_v2 acceleration; any required delete-one failure makes BCa unavailable without changing percentile inference.",
                    "unit": "parameter scale and explicit availability state",
                    "output_ids": bca_outputs,
                },
                {
                    "id": "deterministic_execution_accounting",
                    "label": "Indexed execution and failure accounting",
                    "definition": "Bind every requested replicate to its deterministic sample-index digest and success or typed failure outcome, with no retry or replacement and exact method, estimator, seed, operation, and count provenance.",
                    "unit": "counts, identities, reason codes, and SHA-256 digests",
                    "output_ids": accounting_outputs,
                },
            ],
            "preprocessing": [
                {
                    "id": "model_wide_complete_cases",
                    "order": 0,
                    "operation": "Resolve the model indicator set and retain the same finite model-wide complete rows used by the bounded plsc_v2 point estimator.",
                    "parameters": {
                        "policy": "listwise_deletion",
                        "missing_to_zero": False,
                        "row_identity_preserved": True,
                    },
                    "applies_to": ["raw_numeric_rows"],
                },
                {
                    "id": "indexed_empirical_case_resampling",
                    "order": 1,
                    "operation": "For each fixed replicate index, sample the complete-case frame with replacement using the domain-separated plsc_consistent_bootstrap_v1 stream.",
                    "parameters": {
                        "operation": "plsc_consistent_bootstrap_v1",
                        "retry_policy": "no_retry_no_replacement_fixed_indexed_draws_v1",
                        "replicate_range": "1000..=10000",
                    },
                    "applies_to": ["complete_case_sampling_frame"],
                },
                {
                    "id": "full_plsc_v2_refit_and_orientation",
                    "order": 2,
                    "operation": "Re-estimate all plsc_v2 weights, rho_A corrections, corrected correlations, loadings, structural coefficients, effects, and R-squared values, then align construct orientation to the original solution.",
                    "parameters": {
                        "estimator": "plsc_v2",
                        "ordinary_pls_estimates_reused": False,
                        "orientation": "original_construct_score_covariance",
                    },
                    "applies_to": ["each_primary_and_delete_one_sample"],
                },
                {
                    "id": "canonical_parameter_and_failure_projection",
                    "order": 3,
                    "operation": "Extract the exact canonical PLSc parameter manifest, reject identity drift or nonfinite values, and retain every failed refit under one stable reason code.",
                    "parameters": {
                        "minimum_usable_fraction": 0.9,
                        "parameter_digest": "sha256_archive_stable_13_significant_digits_v1",
                        "failure_replacement": False,
                    },
                    "applies_to": ["refit_results"],
                },
                {
                    "id": "percentile_normal_and_conditional_bca_summary",
                    "order": 4,
                    "operation": "Summarize successful primary refits with sample standard errors, Type-7 percentiles, two-sided normal-reference diagnostics, and conditional full-refit BCa intervals.",
                    "parameters": {
                        "quantile": "type7",
                        "test": "two_sided_standard_normal_reference",
                        "bca_bias": "midrank",
                        "bca_delete_one": "full_plsc_v2",
                    },
                    "applies_to": ["canonical_parameter_distributions"],
                },
            ],
            "model_predicates": [
                {
                    "id": "bounded_reflective_plsc_model",
                    "expression": "Every construct is reflective with at least two uniquely bound indicators and the recursive graph satisfies the exact plsc_v2 path-or-factor-weighting contract.",
                    "on_violation": "not_applicable",
                    "diagnostic_code": "plsc_bootstrap.unsupported_model",
                },
                {
                    "id": "no_generated_or_grouped_extensions",
                    "expression": "Interactions, higher-order constructs, case weights, group inference, PCA weighting, and matrix-only estimation are absent.",
                    "on_violation": "not_applicable",
                    "diagnostic_code": "plsc_bootstrap.unsupported_extension",
                },
                {
                    "id": "converged_admissible_plsc_point",
                    "expression": "The linked point estimate is converged plsc_v2 and has finite, admissible rho_A and attenuation-corrected parameters with the complete canonical identity set.",
                    "on_violation": "error",
                    "diagnostic_code": "plsc_bootstrap.invalid_point_estimate",
                },
            ],
            "data_predicates": [
                {
                    "id": "raw_numeric_rows_required",
                    "expression": "Input is raw row-level numeric data; covariance and correlation matrices are rejected for empirical case resampling.",
                    "on_violation": "not_applicable",
                    "diagnostic_code": "plsc_bootstrap.raw_rows_required",
                },
                {
                    "id": "complete_case_support",
                    "expression": "The bounded listwise policy leaves enough finite complete rows for the point estimate, every primary resample, and every required delete-one refit.",
                    "on_violation": "error",
                    "diagnostic_code": "plsc_bootstrap.insufficient_complete_cases",
                },
                {
                    "id": "supported_resampling_settings",
                    "expression": "Bootstrap replicates are 1000 through 10000, confidence is finite strictly between zero and one, workers are supported, and studentized and permutation counts are zero.",
                    "on_violation": "error",
                    "diagnostic_code": "plsc_bootstrap.unsupported_settings",
                },
            ],
            "oracles": [
                {
                    "id": "dijkstra_henseler_2015_plsc",
                    "kind": "primary_literature",
                    "citation": "Dijkstra and Henseler (2015), Consistent and Asymptotically Normal PLS Estimators for Linear Structural Equations, DOI 10.1016/j.csda.2014.07.008; and Consistent Partial Least Squares Path Modeling, DOI 10.25300/MISQ/2015/39.2.02.",
                    "locator": "docs/methods/CONSISTENT_BOOTSTRAP_V1.md",
                    "independence_group": "primary_plsc_method",
                    "runtime_policy": "no_runtime_dependency",
                    "implementation": None,
                    "covered_estimand_ids": [
                        "plsc_parameter_resampling_distribution"
                    ],
                },
                {
                    "id": "efron_tibshirani_bootstrap",
                    "kind": "primary_literature",
                    "citation": "Efron (1987), Better Bootstrap Confidence Intervals, JASA 82(397), and Efron and Tibshirani (1993), An Introduction to the Bootstrap.",
                    "locator": "docs/methods/CONSISTENT_BOOTSTRAP_V1_QUALIFICATION.md",
                    "independence_group": "primary_bootstrap_method",
                    "runtime_policy": "no_runtime_dependency",
                    "implementation": None,
                    "covered_estimand_ids": estimand_ids,
                },
                {
                    "id": "required_independent_python_full_plsc_reference",
                    "kind": "independent_implementation",
                    "citation": "Required future independently maintained Python implementation of the complete indexed full-refit PLSc bootstrap; this oracle has not been supplied or executed.",
                    "locator": "validation/oracles/consistent_bootstrap_v1_full_plsc_reference.py",
                    "independence_group": "required_independent_python_full_plsc_oracle",
                    "runtime_policy": "development_validation_only",
                    "implementation": {
                        "name": "Required independent Python full-refit PLSc bootstrap oracle",
                        "version": "required_contract_v1_not_supplied",
                        "maintainer": "Independent Python oracle owner unassigned",
                    },
                    "covered_estimand_ids": estimand_ids,
                },
                {
                    "id": "required_independent_r_full_plsc_reference",
                    "kind": "independent_implementation",
                    "citation": "Required future independently maintained R implementation of the complete indexed full-refit PLSc bootstrap; this oracle has not been supplied or executed.",
                    "locator": "validation/oracles/consistent_bootstrap_v1_full_plsc_reference.R",
                    "independence_group": "required_independent_r_full_plsc_oracle",
                    "runtime_policy": "development_validation_only",
                    "implementation": {
                        "name": "Required independent R full-refit PLSc bootstrap oracle",
                        "version": "required_contract_v1_not_supplied",
                        "maintainer": "Independent R oracle owner unassigned",
                    },
                    "covered_estimand_ids": estimand_ids,
                },
                {
                    "id": "transparent_python_arithmetic_microreference",
                    "kind": "hand_calculation",
                    "citation": "Transparent Python standard-library microreference for ledger hashing, Type-7, normal-reference, and BCa arithmetic; it does not implement or qualify the PLSc estimator.",
                    "locator": "validation/consistent_bootstrap_v1_reference.py",
                    "independence_group": "python_standard_library_arithmetic_microreference",
                    "runtime_policy": "development_validation_only",
                    "implementation": None,
                    "covered_estimand_ids": [
                        "large_sample_and_percentile_inference",
                        "conditional_bca_inference",
                        "deterministic_execution_accounting",
                    ],
                },
                {
                    "id": "transparent_r_arithmetic_microreference",
                    "kind": "hand_calculation",
                    "citation": "Transparent base-R microreference for sample standard error, Type-7, standard-normal probability, and BCa arithmetic; it does not implement or qualify the PLSc estimator.",
                    "locator": "validation/consistent_bootstrap_v1_reference.R",
                    "independence_group": "base_r_arithmetic_microreference",
                    "runtime_policy": "development_validation_only",
                    "implementation": None,
                    "covered_estimand_ids": [
                        "large_sample_and_percentile_inference",
                        "conditional_bca_inference",
                    ],
                },
            ],
            "oracle_exception": None,
        },
        "scenario_contract": {
            "axes": [
                _axis(
                    "model_topology",
                    "Recursive PLSc topology",
                    {
                        "two_construct_chain": "Two reflective constructs with one directed path.",
                        "recursive_mediation_branch": "At least six reflective constructs with branching and serial/parallel indirect effects.",
                    },
                ),
                _axis(
                    "measurement_model",
                    "Reflective measurement blocks",
                    {
                        "reflective_two_item": "Every block has the supported two-indicator minimum.",
                        "reflective_mixed_block_sizes": "Reflective blocks contain two through eight indicators with mixed reliability.",
                    },
                ),
                _axis(
                    "data_distribution",
                    "Population distribution and orientation",
                    {
                        "gaussian_well_conditioned": "Gaussian common factors with admissible reliabilities and positive anchors.",
                        "skewed_heavy_tail_mixed_sign": "Skewed and heavy-tailed observations with sign-reversed indicators and near-admissible corrections.",
                    },
                ),
                _axis(
                    "missingness",
                    "Missing-data condition",
                    {
                        "complete": "No missing observations.",
                        "listwise_mcar_five_percent": "Five percent MCAR cells under the frozen model-wide listwise policy.",
                    },
                ),
                _axis(
                    "input_type",
                    "Input representation",
                    {
                        "raw_rows": "Supported row-level numeric observations.",
                        "summary_matrix_rejection": "Covariance or correlation input must return the exact not-applicable diagnostic.",
                    },
                ),
                _axis(
                    "workload",
                    "Refit admissibility",
                    {
                        "all_refits_usable": "All primary and delete-one PLSc refits are admissible.",
                        "inadmissible_refits_retained": "Some primary or delete-one refits fail and remain in the fixed ledger without retry.",
                    },
                ),
                _axis(
                    "workers",
                    "Worker scheduling",
                    {
                        "one_worker": "Serial reference execution.",
                        "reference_parallel_workers": "Parallel execution on the available reference-hardware workers.",
                    },
                ),
            ],
            "complexity_profiles": [
                _profile(
                    "micro_exact",
                    "Minimum executable PLSc model plus separate hand-checkable arithmetic microcases.",
                    rows=20,
                    indicators=4,
                    constructs=2,
                    resamples=1000,
                ),
                _profile(
                    "applied",
                    "Typical applied reflective PLSc model with final-report bootstrap size.",
                    rows=500,
                    indicators=24,
                    constructs=6,
                    resamples=5000,
                ),
                _profile(
                    "large",
                    "Large routine model used for determinism, result-size, and cancellation evidence.",
                    rows=5000,
                    indicators=80,
                    constructs=20,
                    resamples=5000,
                ),
                _profile(
                    "maximum_axis",
                    "Separate maximum row, indicator, construct, and resample axes.",
                    rows=100000,
                    indicators=300,
                    constructs=100,
                    resamples=10000,
                ),
                _profile(
                    "compound_stress",
                    "Combined high-dimensional full-refit PLSc bootstrap workload.",
                    rows=20000,
                    indicators=150,
                    constructs=50,
                    resamples=10000,
                ),
            ],
            "mandatory_combinations": [
                _combination(
                    "applied_pairwise_matrix",
                    "applied",
                    "pairwise",
                    "Cover every pair of values across the seven mandatory axes.",
                ),
                _combination(
                    "micro_exact_ledger_and_intervals",
                    "micro_exact",
                    "targeted",
                    "Check canonical identities, exact counts, sample and parameter digests, Type-7 endpoints, normal-reference values, and symmetric BCa arithmetic.",
                    selections=_selections(
                        model_topology=["two_construct_chain"],
                        measurement_model=["reflective_two_item"],
                        data_distribution=["gaussian_well_conditioned"],
                        missingness=["complete"],
                        input_type=["raw_rows"],
                        workload=["all_refits_usable", "inadmissible_refits_retained"],
                        workers=["one_worker"],
                    ),
                ),
                _combination(
                    "large_worker_and_failure_invariance",
                    "large",
                    "targeted",
                    "Require the same ordered ledger, failures, digests, and analytical payload under serial and parallel scheduling.",
                    selections=_selections(
                        model_topology=["recursive_mediation_branch"],
                        measurement_model=["reflective_mixed_block_sizes"],
                        data_distribution=["skewed_heavy_tail_mixed_sign"],
                        missingness=["listwise_mcar_five_percent"],
                        input_type=["raw_rows"],
                        workload=["inadmissible_refits_retained"],
                    ),
                ),
                *[
                    _combination(
                        f"maximum_{dimension}",
                        "maximum_axis",
                        "targeted",
                        f"Exercise only the declared maximum {dimension} axis while the other dimensions remain controlled at applied values.",
                        stressed=[dimension],
                        selections=_selections(
                            model_topology=["recursive_mediation_branch"],
                            measurement_model=["reflective_mixed_block_sizes"],
                            data_distribution=["gaussian_well_conditioned"],
                            missingness=["complete"],
                            input_type=["raw_rows"],
                            workload=["all_refits_usable"],
                            workers=["reference_parallel_workers"],
                        ),
                    )
                    for dimension in ("rows", "indicators", "constructs", "resamples")
                ],
                _combination(
                    "compound_full_refit_cancellation_and_archive",
                    "compound_stress",
                    "compound",
                    "Combine high row, indicator, construct, and resample counts with failure accounting, cancellation, archive, and export checks.",
                    stressed=["rows", "indicators", "constructs", "resamples"],
                    selections=_selections(
                        model_topology=["recursive_mediation_branch"],
                        measurement_model=["reflective_mixed_block_sizes"],
                        data_distribution=["skewed_heavy_tail_mixed_sign"],
                        missingness=["listwise_mcar_five_percent"],
                        input_type=["raw_rows"],
                        workload=["inadmissible_refits_retained"],
                        workers=["reference_parallel_workers"],
                    ),
                ),
            ],
            "monte_carlo_policy": {
                "confidence_level": 0.95,
                "maximum_half_width": 0.01,
                "failed_fits_in_denominator": True,
            },
        },
        "comparison_contract": {
            "outputs": [
                *[
                    _comparison(
                        output_id,
                        "abs_relative",
                        "Canonical PLSc values and interval arithmetic must agree with independently maintained full-refit references after the same indexed sample and sign alignment; tolerances are tighter than the source estimator acceptance boundary.",
                        absolute_tolerance=1e-8,
                        relative_tolerance=1e-6,
                    )
                    for output_id in numerical_outputs
                ],
                *[
                    _comparison(
                        output_id,
                        "exact",
                        "Identifiers, counts, availability, reason codes, ordered ledger rows, provenance, and digests are deterministic contract values and permit no numerical tolerance.",
                    )
                    for output_id in exact_outputs
                ],
            ]
        },
        "operational_contract": {
            "performance": {
                "hardware_classes": [
                    {
                        "id": "standard",
                        "os_family": "windows",
                        "architecture": "x86_64",
                        "minimum_logical_cores": 6,
                        "minimum_memory_gib": 16,
                        "notes": "Product-finalization standard reference class.",
                    },
                    {
                        "id": "workstation",
                        "os_family": "windows",
                        "architecture": "x86_64",
                        "minimum_logical_cores": 12,
                        "minimum_memory_gib": 32,
                        "notes": "Product-finalization workstation reference class.",
                    },
                ],
                "baseline_policy": {
                    "warmup_runs": 1,
                    "measured_runs": 5,
                    "statistic": "median",
                    "maximum_runtime_regression_percent": 20.0,
                    "maximum_memory_regression_percent": 20.0,
                },
                "budgets": _performance_budgets(),
            },
            "archive": {
                "current_schema_version": 5,
                "readable_schema_versions": [1, 2, 3, 4, 5],
                "writable_schema_versions": [5],
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
                "canonical_projection_id": "canonical_result_document_v2_plsc_consistent_bootstrap",
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
                    {
                        "phase": "validate",
                        "applicability": "required",
                        "not_applicable_reason": None,
                    },
                    {
                        "phase": "estimate",
                        "applicability": "required",
                        "not_applicable_reason": None,
                    },
                    {
                        "phase": "resample",
                        "applicability": "required",
                        "not_applicable_reason": None,
                    },
                    {
                        "phase": "compare",
                        "applicability": "not_applicable",
                        "not_applicable_reason": "PLSc consistent bootstrapping does not execute a competing-model comparison phase.",
                    },
                    {
                        "phase": "export",
                        "applicability": "required",
                        "not_applicable_reason": None,
                    },
                ],
                "no_partial_visible_result": True,
                "no_partial_committed_result": True,
                "archive_unchanged": True,
                "same_settings_retry": True,
            },
        },
        "evidence_contract": {
            "required_roles": [
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
            ],
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


def verify(document: dict[str, Any]) -> dict[str, Any]:
    registry = strict_load_json(REGISTRY)
    return validate_spec_document(
        document,
        registry_document=registry,
        require_registry=True,
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--write",
        action="store_true",
        help="Write the deterministic frozen JSON before checking it.",
    )
    args = parser.parse_args()
    expected = build_spec()
    if args.write:
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(
            json.dumps(expected, indent=2, sort_keys=True, allow_nan=False) + "\n",
            encoding="utf-8",
        )
    if not OUTPUT.is_file():
        print(f"missing frozen specification: {OUTPUT}", file=sys.stderr)
        return 1
    actual = strict_load_json(OUTPUT)
    report = verify(actual)
    generated_matches = actual == expected
    result = {
        **report,
        "generated_matches_frozen_json": generated_matches,
        "migration_status": actual.get("migration", {}).get("status"),
        "receipt_count": len(actual.get("evidence_contract", {}).get("receipts", [])),
    }
    print(json.dumps(result, indent=2, sort_keys=True, allow_nan=False))
    return (
        0
        if report["passed"]
        and generated_matches
        and not report["qualification_ready"]
        and result["receipt_count"] == 0
        else 1
    )


if __name__ == "__main__":
    raise SystemExit(main())
