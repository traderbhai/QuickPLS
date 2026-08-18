from __future__ import annotations

import copy
import json
import math
import subprocess
import sys
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator, ValidationError

from validation.cbsem_exact_case_bootstrap_studentized_phase1b import (
    CASE_SCHEMA,
    DEFAULT_MANIFEST,
    MANIFEST_SCHEMA,
    REPORT_SCHEMA,
    _resource_reasons,
    clopper_pearson_upper,
    validate_completed_payload,
    validate_manifest,
)


ROOT = Path(__file__).resolve().parents[1]


def test_manifest_binds_current_sources_and_exact_independent_matrix() -> None:
    manifest, digest = validate_manifest(DEFAULT_MANIFEST)
    assert len(digest) == 64
    assert manifest["source_binding"]["rust_example_sha256"] == (
        "1a1578488d204ba4368b7f5b715669a40f025a4a7e4cb2c450e321a1b00956b0"
    )
    assert manifest["source_binding"]["binary_sha256"] == (
        "ae07b26bc72a4f2e2f1a6ffa72fd3caaf2ed6ee50c399073773e6a7655d6e5b4"
    )
    assert manifest["dimension_contract"]["d_optimizer_dimensions"] == 18
    assert [case["seed"] for case in manifest["cases"]] == [
        19001,
        19002,
        29001,
        29002,
        39001,
    ]
    assert all(case["seed"] not in {91, 93} for case in manifest["cases"])
    assert manifest["qualification_boundary"]["status"] == (
        "blocked_not_product_qualification"
    )


@pytest.mark.parametrize(
    ("failures", "trials", "expected"),
    [
        (0, 5000, 0.0005989670023149134),
        (13, 5000, 0.004130543586169635),
        (464, 5000, 0.09982678714535612),
        (465, 5000, 0.10003321741209238),
        (950, 10000, 0.09996065260652594),
        (951, 10000, 0.10006290087477764),
        (10000, 10000, 1.0),
    ],
)
def test_fixed_log_cdf_clopper_pearson_vectors(
    failures: int, trials: int, expected: float
) -> None:
    assert math.isclose(
        clopper_pearson_upper(failures, trials),
        expected,
        rel_tol=0.0,
        abs_tol=2.0e-15,
    )


def test_exact_typed_partitions_and_compact_ledgers_are_accepted() -> None:
    manifest, _ = validate_manifest(DEFAULT_MANIFEST)
    case = manifest["cases"][0]
    payload = _valid_payload(manifest, case)
    reasons, ledger = validate_completed_payload(payload, case, manifest)
    assert reasons == []
    assert ledger is not None
    assert ledger["point_successes"] == 4999
    assert ledger["point_failures_count"] == 1
    assert ledger["studentized_usable_refits"] == 4998
    assert ledger["standard_error_unavailable_count"] == 1
    assert ledger["point_failure_counts_by_kind"] == {
        "moment_matrix_not_positive_definite": 0,
        "non_convergence": 1,
        "inadmissible_solution": 0,
        "numerical_failure": 0,
    }
    assert ledger["standard_error_unavailable_counts_by_reason"][
        "singular_information"
    ] == 1
    assert ledger["one_sided_95_percent_upper_bound"] < 0.1


def test_duplicate_unknown_retry_iteration_and_dimension_tamper_fail_closed() -> None:
    manifest, _ = validate_manifest(DEFAULT_MANIFEST)
    case = manifest["cases"][0]
    payload = _valid_payload(manifest, case)

    payload["result"]["base"]["successful_refits"][-1]["replicate_index"] = 0
    reasons, ledger = validate_completed_payload(payload, case, manifest)
    assert "point_ledger_duplicate_missing_or_out_of_order" in reasons
    assert ledger is None
    payload = _valid_payload(manifest, case)

    payload["result"]["base"]["failed_refits"][0]["kind"] = "invented"
    reasons, ledger = validate_completed_payload(payload, case, manifest)
    assert "point_failure_untyped_or_invalid" in reasons
    assert ledger is None
    payload = _valid_payload(manifest, case)

    payload["result"]["studentized"]["refit_standard_errors"][0]["outcome"][
        "reason"
    ] = "invented"
    reasons, ledger = validate_completed_payload(payload, case, manifest)
    assert "standard_error_unavailable_reason_unknown" in reasons
    assert ledger is None
    payload = _valid_payload(manifest, case)

    payload["result"]["base"]["retry_policy"] = "retry"
    reasons, ledger = validate_completed_payload(payload, case, manifest)
    assert "method_schedule_retry_or_minimum_binding_invalid" in reasons
    assert ledger is None
    payload = _valid_payload(manifest, case)

    payload["result"]["base"]["successful_refits"][0]["iterations"] = 0
    reasons, ledger = validate_completed_payload(payload, case, manifest)
    assert "point_success_integrity_invalid" in reasons
    assert ledger is None
    payload = _valid_payload(manifest, case)

    payload["case"]["dimensions"]["d_optimizer_dimensions"] = 17
    reasons, _ = validate_completed_payload(payload, case, manifest)
    assert "dimension_d_optimizer_dimensions_mismatch" in reasons


def test_resource_gate_rejects_nan_elapsed_and_missing_or_zero_rss() -> None:
    manifest, _ = validate_manifest(DEFAULT_MANIFEST)
    case = manifest["cases"][0]
    payload = _valid_payload(manifest, case)
    observation = {
        "orphan_processes": 0,
        "peak_working_set_bytes": 0,
        "elapsed_seconds": math.nan,
    }
    reasons = _resource_reasons(
        observation, payload, case, manifest, total_memory_bytes=16 * 1024**3
    )
    assert "peak_working_set_missing_or_nonpositive" in reasons
    assert "elapsed_seconds_missing_nonfinite_or_nonpositive" in reasons


def test_strict_schemas_lock_cases_and_never_allow_product_qualification() -> None:
    manifest = json.loads(DEFAULT_MANIFEST.read_text(encoding="utf-8"))
    manifest_schema = json.loads(MANIFEST_SCHEMA.read_text(encoding="utf-8"))
    case_schema = json.loads(CASE_SCHEMA.read_text(encoding="utf-8"))
    report_schema = json.loads(REPORT_SCHEMA.read_text(encoding="utf-8"))
    for schema in (manifest_schema, case_schema, report_schema):
        Draft202012Validator.check_schema(schema)
    Draft202012Validator(manifest_schema).validate(manifest)
    tampered = copy.deepcopy(manifest)
    tampered["cases"][0]["seed"] = 91
    with pytest.raises(ValidationError):
        Draft202012Validator(manifest_schema).validate(tampered)
    accepted_without_evidence = {
        "schema_version": 1,
        "kind": "cbsem_exact_case_bootstrap_studentized_phase1b_case_v1",
        "case": manifest["cases"][0],
        "manifest_sha256": "a" * 64,
        "status": "accepted_resource_candidate_evidence",
        "acceptance_reasons": [],
        "probe": {},
        "warmup_runs": [],
        "measured_runs": [],
        "aggregates": None,
        "dimensions": None,
        "scientific_result_sha256": None,
        "ledger": None,
        "runtime_acceptance": None,
        "cancellation": None,
        "qualification_boundary": manifest["qualification_boundary"],
    }
    with pytest.raises(ValidationError):
        Draft202012Validator(case_schema).validate(accepted_without_evidence)
    assert report_schema["properties"]["qualification_boundary"]["properties"][
        "status"
    ]["const"] == "blocked_not_product_qualification"


def test_dry_run_executes_no_workloads() -> None:
    completed = subprocess.run(
        [
            sys.executable,
            str(
                ROOT
                / "validation/cbsem_exact_case_bootstrap_studentized_phase1b.py"
            ),
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert completed.returncode == 0, completed.stderr
    output = json.loads(completed.stdout)
    assert output["status"] == "dry_run_no_workloads_executed"
    assert output["qualification_status"] == "blocked_not_product_qualification"


def _valid_payload(manifest: dict, case: dict) -> dict:
    requested = case["replicates"]
    parameter_ids = [f"parameter_{index:02d}" for index in range(18)]
    success_indices = list(range(requested - 1))
    successes = [
        {
            "replicate_index": index,
            "sampling_positions_sha256": "a" * 64,
            "sample_indices_sha256": "b" * 64,
            "parameter_estimates": [1.0] * 18,
            "iterations": 1,
            "objective": 1.0,
            "gradient_norm": 0.0,
        }
        for index in success_indices
    ]
    failure = {
        "replicate_index": requested - 1,
        "kind": "non_convergence",
        "message": "typed nonconvergence",
        "sampling_positions_sha256": "c" * 64,
        "sample_indices_sha256": "d" * 64,
    }
    se_receipts = [
        {
            "replicate_index": index,
            "outcome": (
                {"status": "unavailable", "reason": "singular_information"}
                if index == 0
                else {
                    "status": "available",
                    "information_method": "cbsem_ml_expected_information_delta_method_v1",
                    "standard_errors": [0.1] * 18,
                }
            ),
        }
        for index in success_indices
    ]
    base_intervals = [
        {
            "parameter_id": parameter_id,
            "original": 1.0,
            "bootstrap_mean": 1.0,
            "bias": 0.0,
            "standard_error": 0.1,
            "percentile_lower": 0.8,
            "percentile_upper": 1.2,
            "usable_replicates": requested - 1,
        }
        for parameter_id in parameter_ids
    ]
    studentized_intervals = [
        {
            "parameter_id": parameter_id,
            "outcome": {
                "status": "available",
                "point_estimate": 1.0,
                "point_standard_error": 0.1,
                "lower_pivot_quantile": -2.0,
                "upper_pivot_quantile": 2.0,
                "interval_lower": 0.8,
                "interval_upper": 1.2,
                "usable_replicates": requested - 2,
            },
        }
        for parameter_id in parameter_ids
    ]
    minimum = max(1000, math.ceil(0.9 * requested))
    source = manifest["source_binding"]
    return {
        "schema_version": 1,
        "kind": "cbsem_exact_case_bootstrap_studentized_benchmark_run_v1",
        "status": "completed",
        "qualification_status": "measurement_only_no_caps_or_promotion",
        "case": {
            "requested_replicates": requested,
            "workers": case["workers"],
            "seed": case["seed"],
            "dimensions": dict(manifest["dimension_contract"]),
        },
        "fixture": {
            "data_path": source["data_path"],
            "data_sha256": source["data_sha256"],
            "recipe_path": source["recipe_path"],
            "recipe_sha256": source["recipe_sha256"],
        },
        "metrics": {
            "successful_point_refits": requested - 1,
            "failed_point_refits": 1,
            "analytic_se_unavailable_refits": 1,
            "studentized_usable_refits": requested - 2,
            "combined_s2_json_bytes": 1024,
        },
        "scientific_result_sha256": "e" * 64,
        "result": {
            "base": {
                "method_version": "cbsem_exact_case_bootstrap_v1",
                "requested_replicates": requested,
                "attempted_refits": requested,
                "usable_replicates": requested - 1,
                "failed_replicates": 1,
                "seed": case["seed"],
                "retry_policy": "no_retry_fixed_preplanned_primary_draws_v1",
                "max_attempts_per_replicate": 1,
                "minimum_usable_fraction": 0.9,
                "minimum_usable_replicates": minimum,
                "parameter_ids": parameter_ids,
                "inference": {"status": "available"},
                "intervals": base_intervals,
                "successful_refits": successes,
                "failed_refits": [failure],
            },
            "studentized": {
                "method_version": "cbsem_exact_case_bootstrap_analytic_studentized_interval_v1",
                "minimum_usable_fraction": 0.9,
                "minimum_usable_replicates": minimum,
                "studentized_usable_replicates": requested - 2,
                "parameter_ids": parameter_ids,
                "inference": {"status": "available"},
                "intervals": studentized_intervals,
                "refit_standard_errors": se_receipts,
            },
        },
    }
