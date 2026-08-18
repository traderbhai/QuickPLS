#!/usr/bin/env python3
"""Build and verify the frozen HTMT QualificationSpec V2 contract.

The generated specification is intentionally ``compatibility_only`` with no
receipts.  It pre-registers the complete qualification workload without
claiming that the current bounded HTMT product surface has passed it.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
OUTPUT = VALIDATION / "qualification_v2" / "htmt_plus_v1.qualification.json"
REGISTRY = VALIDATION / "capabilities" / "capability_registry_v2.json"
sys.path.insert(0, str(VALIDATION))

from qualification_spec_v2 import (  # noqa: E402
    strict_load_json,
    validate_spec_document,
)


FROZEN_AT = "2026-08-14T16:00:00Z"
QUALIFICATION_ID = "qpls3.assessment.htmt.qualification_v2"
METHOD_VERSION = "ringle_et_al_htmt_plus_v1"


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
    "model_topology": ["two_construct_pair", "multi_construct_recursive"],
    "measurement_model": ["reflective_two_item", "reflective_mixed_block_sizes"],
    "data_distribution": ["gaussian_positive_loadings", "mixed_sign_skewed_heavy_tail"],
    "missingness": ["complete", "listwise_mcar_five_percent"],
    "input_type": ["raw_rows", "summary_matrix_rejection"],
    "workload": ["point_assessment", "complete_bootstrap_inference"],
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
        "micro_exact": (15.0, 512 * 1024**2, 16 * 1024**2),
        "applied": (900.0, 8 * 1024**3, 512 * 1024**2),
        "large": (3600.0, 12 * 1024**3, 2 * 1024**3),
        "maximum_axis": (7200.0, 12 * 1024**3, 4 * 1024**3),
        "compound_stress": (7200.0, 12 * 1024**3, 4 * 1024**3),
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
    scientific_outputs = {
        "original_htmt_point": ["htmt_original_matrix"],
        "htmt_plus_point": ["htmt_plus_matrix"],
        "complete_bootstrap_inference": [
            "htmt_original_bc_lower",
            "htmt_original_bc_upper",
            "htmt_plus_bc_lower",
            "htmt_plus_bc_upper",
            "htmt_original_decision_0_90",
            "htmt_plus_decision_0_90",
        ],
    }
    all_estimands = list(scientific_outputs)
    return {
        "schema_version": 2,
        "identity": {
            "qualification_id": QUALIFICATION_ID,
            "method_version": METHOD_VERSION,
            "execution_kind": "hybrid",
            "potentially_long_running": True,
            "spec_frozen_at_utc": FROZEN_AT,
            "capability_cell": {
                "registry_schema_version": 2,
                "capability_id": "smartpls.htmt",
                "capability_version": METHOD_VERSION,
                "cell_id": "qpls3.assessment.htmt",
            },
        },
        "migration": {
            "source_kind": "qualification_v1_manifest",
            "source_schema_version": 1,
            "source_manifest_path": "validation/methods/htmt_plus_v1.manifest.json",
            "status": "compatibility_only",
            "unresolved_items": [
                "product.configurable_one_and_two_sided_htmt_inference_missing",
                "product.percentile_studentized_and_true_bca_htmt_inference_missing",
                "product.plsc_consistent_bootstrap_htmt_inference_missing",
                "model.sem_model_v4_factor_composite_semantics_not_integrated",
                "data.missing_value_policies_beyond_listwise_not_integrated",
                "export.svg_pdf_png_semantic_readback_not_integrated",
                "evidence.current_source_bound_candidate_build_missing",
                "evidence.full_monte_carlo_qualification_not_run",
                "evidence.maximum_axis_compound_stress_and_soak_not_run",
                "evidence.packaged_windows_accessibility_matrix_not_run",
                "evidence.independent_scientific_review_not_recorded",
            ],
        },
        "scientific_contract": {
            "estimands": [
                {
                    "id": "original_htmt_point",
                    "label": "Original signed HTMT",
                    "definition": "Arithmetic mean of signed heterotrait correlations divided by the geometric mean of the two positive signed monotrait means.",
                    "unit": "correlation ratio",
                    "output_ids": scientific_outputs["original_htmt_point"],
                },
                {
                    "id": "htmt_plus_point",
                    "label": "HTMT+",
                    "definition": "Arithmetic mean of absolute heterotrait correlations divided by the geometric mean of the two absolute monotrait means; values are not clamped at one.",
                    "unit": "absolute-correlation ratio",
                    "output_ids": scientific_outputs["htmt_plus_point"],
                },
                {
                    "id": "complete_bootstrap_inference",
                    "label": "Complete-bootstrap HTMT inference",
                    "definition": "Bias-corrected Type-7 interval endpoints from fixed indexed case resamples and the one-tailed alpha .05 decision that the 90 percent equivalent upper endpoint is strictly below .90.",
                    "unit": "correlation ratio and Boolean decision",
                    "output_ids": scientific_outputs["complete_bootstrap_inference"],
                },
            ],
            "preprocessing": [
                {
                    "id": "model_wide_complete_cases",
                    "order": 0,
                    "operation": "Retain the same model-wide finite complete cases used by the supported PLS estimator.",
                    "parameters": {
                        "policy": "listwise_deletion",
                        "missing_to_zero": False,
                    },
                    "applies_to": ["raw_numeric_rows"],
                },
                {
                    "id": "pearson_sample_correlations",
                    "order": 1,
                    "operation": "Calculate Pearson correlations in deterministic recipe indicator order using sample-centered cross-products.",
                    "parameters": {
                        "correlation": "pearson",
                        "order": "recipe_indicator_order",
                        "nonfinite": "typed_error",
                    },
                    "applies_to": ["retained_numeric_rows"],
                },
                {
                    "id": "indexed_case_resampling",
                    "order": 2,
                    "operation": "For complete bootstrap inference, recompute both HTMT definitions on every preplanned indexed case resample without retries or replacement draws.",
                    "parameters": {
                        "retry_policy": "no_retry_fixed_preplanned_primary_draws_v1",
                        "minimum_usable_fraction": 0.9,
                        "replicate_index_digest": "sha256_u32_le_v1",
                    },
                    "applies_to": ["complete_bootstrap_inference"],
                },
                {
                    "id": "bias_corrected_type7_summary",
                    "order": 3,
                    "operation": "Apply midrank bias correction and Type-7 quantiles for the fixed one-tailed upper alpha .05 workflow represented by equivalent 90 percent two-sided endpoints.",
                    "parameters": {
                        "interval": "bias_corrected_percentile_type7_v1",
                        "bca": False,
                        "test_type": "one_tailed_upper",
                        "alpha": 0.05,
                        "critical_value": 0.9,
                        "decision": "upper_strictly_below_critical_value",
                    },
                    "applies_to": ["usable_pairwise_bootstrap_values"],
                },
            ],
            "model_predicates": [
                {
                    "id": "supported_pls_measurement_pairs",
                    "expression": "Each inferred pair contains two distinct reflective constructs with at least two indicators each; formative or single-indicator participation is typed not applicable.",
                    "on_violation": "not_applicable",
                    "diagnostic_code": "htmt.reflective_pair_required",
                },
                {
                    "id": "supported_pls_graph",
                    "expression": "The containing model is an otherwise supported recursive PLS-SEM model and every indicator binding is unique and resolvable.",
                    "on_violation": "error",
                    "diagnostic_code": "htmt.supported_pls_model_required",
                },
            ],
            "data_predicates": [
                {
                    "id": "raw_numeric_rows_required",
                    "expression": "Current HTMT execution receives row-level numeric observations; covariance or correlation matrices are rejected before calculation.",
                    "on_violation": "not_applicable",
                    "diagnostic_code": "htmt.raw_rows_required",
                },
                {
                    "id": "finite_complete_case_support",
                    "expression": "At least three model-wide complete rows remain, every participating indicator has nonzero variance, and every computed correlation is finite within tolerance.",
                    "on_violation": "error",
                    "diagnostic_code": "htmt.insufficient_or_degenerate_data",
                },
            ],
            "oracles": [
                {
                    "id": "henseler_2015",
                    "kind": "primary_literature",
                    "citation": "Henseler, Ringle, and Sarstedt (2015), A New Criterion for Assessing Discriminant Validity in Variance-Based SEM, DOI 10.1007/s11747-014-0403-8.",
                    "locator": "docs/methods/PLS_HTMT_V1.md",
                    "independence_group": "primary_henseler_2015",
                    "runtime_policy": "no_runtime_dependency",
                    "implementation": None,
                    "covered_estimand_ids": [
                        "original_htmt_point",
                        "complete_bootstrap_inference",
                    ],
                },
                {
                    "id": "ringle_2023",
                    "kind": "primary_literature",
                    "citation": "Ringle et al. (2023), A Perspective on Using Partial Least Squares Structural Equation Modelling in Data Articles, DOI 10.1016/j.dib.2023.109074.",
                    "locator": "validation/results/htmt_published_ringle_2023.json",
                    "independence_group": "primary_ringle_2023",
                    "runtime_policy": "no_runtime_dependency",
                    "implementation": None,
                    "covered_estimand_ids": [
                        "htmt_plus_point",
                        "complete_bootstrap_inference",
                    ],
                },
                {
                    "id": "transparent_standard_library_reference",
                    "kind": "independent_implementation",
                    "citation": "Transparent Python standard-library HTMT and BC Type-7 reference maintained outside the Rust product engine.",
                    "locator": "validation/htmt_reference.py",
                    "independence_group": "transparent_python_standard_library",
                    "runtime_policy": "development_validation_only",
                    "implementation": {
                        "name": "QuickPLS transparent Python HTMT reference",
                        "version": "1",
                        "maintainer": "QuickPLS validation authors",
                    },
                    "covered_estimand_ids": all_estimands,
                },
                {
                    "id": "numpy_scipy_reference",
                    "kind": "independent_implementation",
                    "citation": "Independent NumPy/SciPy vectorized HTMT and bias-corrected quantile implementation used only for development validation.",
                    "locator": "validation/htmt_scipy_reference.py",
                    "independence_group": "numpy_scipy_implementation",
                    "runtime_policy": "development_validation_only",
                    "implementation": {
                        "name": "NumPy and SciPy validation implementation",
                        "version": "numpy-1.26.4+scipy-1.15.2",
                        "maintainer": "NumPy and SciPy maintainers",
                    },
                    "covered_estimand_ids": all_estimands,
                },
                {
                    "id": "csem_original_htmt",
                    "kind": "independent_implementation",
                    "citation": "cSEM 0.6.1 calculateHTMT with absolute false, validation-only executable comparison.",
                    "locator": "validation/results/htmt_csem_comparison.json",
                    "independence_group": "csem_0_6_1",
                    "runtime_policy": "development_validation_only",
                    "implementation": {
                        "name": "cSEM",
                        "version": "0.6.1",
                        "maintainer": "Independent cSEM maintainers",
                    },
                    "covered_estimand_ids": ["original_htmt_point"],
                },
                {
                    "id": "seminr_htmt_plus",
                    "kind": "independent_implementation",
                    "citation": "seminr 2.5.0 mean-absolute-correlation HTMT comparison, validation-only.",
                    "locator": "validation/results/htmt_seminr_comparison.json",
                    "independence_group": "seminr_2_5_0",
                    "runtime_policy": "development_validation_only",
                    "implementation": {
                        "name": "seminr",
                        "version": "2.5.0",
                        "maintainer": "Independent seminr maintainers",
                    },
                    "covered_estimand_ids": ["htmt_plus_point"],
                },
            ],
            "oracle_exception": None,
        },
        "scenario_contract": {
            "axes": [
                _axis(
                    "model_topology",
                    "PLS structural topology",
                    {
                        "two_construct_pair": "Two reflective constructs with one directed structural path.",
                        "multi_construct_recursive": "At least six constructs in a branching and mediating recursive model.",
                    },
                ),
                _axis(
                    "measurement_model",
                    "Measurement blocks",
                    {
                        "reflective_two_item": "Every applicable block has the supported two-indicator minimum.",
                        "reflective_mixed_block_sizes": "Reflective blocks contain two through eight indicators; negative-contract formative blocks are included separately.",
                    },
                ),
                _axis(
                    "data_distribution",
                    "Population distribution",
                    {
                        "gaussian_positive_loadings": "Gaussian factors with positive loadings and known discriminant separation.",
                        "mixed_sign_skewed_heavy_tail": "Sign-reversed indicators under skewed and heavy-tailed finite populations.",
                    },
                ),
                _axis(
                    "missingness",
                    "Missing-data condition",
                    {
                        "complete": "No missing observations.",
                        "listwise_mcar_five_percent": "Five percent MCAR cells evaluated under the current model-wide listwise policy.",
                    },
                ),
                _axis(
                    "input_type",
                    "Input representation",
                    {
                        "raw_rows": "Supported row-level numeric observations.",
                        "summary_matrix_rejection": "Covariance or correlation input must produce an explicit unsupported-input diagnostic in the current bounded implementation.",
                    },
                ),
                _axis(
                    "workload",
                    "Calculation workload",
                    {
                        "point_assessment": "Original HTMT and HTMT+ point matrices only.",
                        "complete_bootstrap_inference": "Full PLS re-estimation and HTMT recomputation for every indexed resample.",
                    },
                ),
                _axis(
                    "workers",
                    "Worker scheduling",
                    {
                        "one_worker": "Serial reference execution.",
                        "reference_parallel_workers": "Parallel execution on all workers allowed by the reference hardware class.",
                    },
                ),
            ],
            "complexity_profiles": [
                _profile(
                    "micro_exact",
                    "Hand-checkable two-construct fixture including tie and threshold cases.",
                    rows=40,
                    indicators=4,
                    constructs=2,
                    resamples=199,
                ),
                _profile(
                    "applied",
                    "Typical applied reflective PLS model with final-report bootstrap size.",
                    rows=500,
                    indicators=24,
                    constructs=6,
                    resamples=5000,
                ),
                _profile(
                    "large",
                    "Large research model used for repeatability and result-size checks.",
                    rows=5000,
                    indicators=80,
                    constructs=20,
                    resamples=5000,
                ),
                _profile(
                    "maximum_axis",
                    "Separate maximum-row, indicator, construct, and resample axes.",
                    rows=100000,
                    indicators=300,
                    constructs=100,
                    resamples=10000,
                ),
                _profile(
                    "compound_stress",
                    "Combined high-dimensional complete bootstrap workload.",
                    rows=50000,
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
                    "Cover every pair of values across all mandatory scenario axes.",
                ),
                _combination(
                    "micro_exact_thresholds",
                    "micro_exact",
                    "targeted",
                    "Check hand calculations, ties, exact .90 threshold direction, and typed applicability.",
                    selections=_selections(
                        model_topology=["two_construct_pair"],
                        measurement_model=["reflective_two_item"],
                        data_distribution=["gaussian_positive_loadings"],
                        missingness=["complete"],
                        input_type=["raw_rows"],
                        workload=["complete_bootstrap_inference"],
                        workers=["one_worker"],
                    ),
                ),
                _combination(
                    "large_worker_repeatability",
                    "large",
                    "targeted",
                    "Require exact replicate ledgers and analytical payloads under serial and parallel scheduling.",
                    selections=_selections(
                        model_topology=["multi_construct_recursive"],
                        measurement_model=["reflective_mixed_block_sizes"],
                        data_distribution=["mixed_sign_skewed_heavy_tail"],
                        missingness=["complete"],
                        input_type=["raw_rows"],
                        workload=["complete_bootstrap_inference"],
                    ),
                ),
                *[
                    _combination(
                        f"maximum_{dimension}",
                        "maximum_axis",
                        "targeted",
                        f"Exercise the declared maximum {dimension} axis while other dimensions are controlled.",
                        stressed=[dimension],
                        selections=_selections(
                            model_topology=["multi_construct_recursive"],
                            measurement_model=["reflective_mixed_block_sizes"],
                            data_distribution=["gaussian_positive_loadings"],
                            missingness=["complete"],
                            input_type=["raw_rows"],
                            workload=["complete_bootstrap_inference"],
                            workers=["reference_parallel_workers"],
                        ),
                    )
                    for dimension in ("rows", "indicators", "constructs", "resamples")
                ],
                _combination(
                    "compound_resampling_memory_cancellation",
                    "compound_stress",
                    "compound",
                    "Combine high row, indicator, construct, and resample counts with cancellation, archive, and export checks.",
                    stressed=["rows", "indicators", "constructs", "resamples"],
                    selections=_selections(
                        model_topology=["multi_construct_recursive"],
                        measurement_model=["reflective_mixed_block_sizes"],
                        data_distribution=["mixed_sign_skewed_heavy_tail"],
                        missingness=["listwise_mcar_five_percent"],
                        input_type=["raw_rows"],
                        workload=["complete_bootstrap_inference"],
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
                _comparison(
                    "htmt_original_matrix",
                    "matrix_norm",
                    "Original signed HTMT requires exact key membership plus global and elementwise numerical agreement.",
                    absolute_tolerance=1e-10,
                    relative_tolerance=1e-8,
                    norm="maximum",
                    elementwise_tolerance=1e-10,
                ),
                _comparison(
                    "htmt_plus_matrix",
                    "matrix_norm",
                    "HTMT+ requires exact key membership and preserves legitimate values above one.",
                    absolute_tolerance=1e-10,
                    relative_tolerance=1e-8,
                    norm="maximum",
                    elementwise_tolerance=1e-10,
                ),
                *[
                    _comparison(
                        output_id,
                        "abs_relative",
                        "Bias-corrected Type-7 endpoint must agree with independent implementations after identical usable-index selection.",
                        absolute_tolerance=1e-10,
                        relative_tolerance=1e-8,
                    )
                    for output_id in (
                        "htmt_original_bc_lower",
                        "htmt_original_bc_upper",
                        "htmt_plus_bc_lower",
                        "htmt_plus_bc_upper",
                    )
                ],
                _comparison(
                    "htmt_original_decision_0_90",
                    "exact",
                    "The decision is exactly upper less than .90; equality must fail.",
                ),
                _comparison(
                    "htmt_plus_decision_0_90",
                    "exact",
                    "The decision is exactly upper less than .90; equality must fail.",
                ),
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
                "canonical_projection_id": "canonical_result_document_v2_htmt_projection",
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
                        "not_applicable_reason": "HTMT does not execute a competing-model comparison phase.",
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
        if report["passed"] and generated_matches and not report["qualification_ready"]
        else 1
    )


if __name__ == "__main__":
    raise SystemExit(main())
