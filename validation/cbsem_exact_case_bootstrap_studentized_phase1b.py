#!/usr/bin/env python3
"""Run the immutable Phase1b exact-CFA studentized evidence matrix.

Phase1b is append-only applied resource-candidate evidence. Even a fully
accepted matrix is not product qualification or permission to expose a cap.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import platform
import re
import tempfile
import uuid
from pathlib import Path
from typing import Any, Mapping

from jsonschema import Draft202012Validator
try:
    from validation.cbsem_exact_case_bootstrap_studentized_benchmark import (
        ROOT,
        command,
        measured_run,
        sha256,
    )
    from validation.cbsem_exact_case_bootstrap_studentized_phase0 import (
        load_json,
        write_new_json,
    )
    from validation.complexity_performance_measure import detected_total_memory_bytes
    from validation.complexity_performance_v2 import aggregate_runs
except ModuleNotFoundError:
    from cbsem_exact_case_bootstrap_studentized_benchmark import (
        ROOT,
        command,
        measured_run,
        sha256,
    )
    from cbsem_exact_case_bootstrap_studentized_phase0 import load_json, write_new_json
    from complexity_performance_measure import detected_total_memory_bytes
    from complexity_performance_v2 import aggregate_runs


DEFAULT_MANIFEST = ROOT / "validation/cbsem_exact_case_bootstrap_studentized_phase1b_manifest_v1.json"
MANIFEST_SCHEMA = ROOT / "validation/cbsem_exact_case_bootstrap_studentized_phase1b_manifest_v1.schema.json"
CASE_SCHEMA = ROOT / "validation/cbsem_exact_case_bootstrap_studentized_phase1b_case_v1.schema.json"
REPORT_SCHEMA = ROOT / "validation/cbsem_exact_case_bootstrap_studentized_phase1b_report_v1.schema.json"
DEFAULT_OUTPUT_ROOT = ROOT / "validation/results/cbsem_exact_case_bootstrap_studentized_phase1b"
CASE_KIND = "cbsem_exact_case_bootstrap_studentized_phase1b_case_v1"
REPORT_KIND = "cbsem_exact_case_bootstrap_studentized_phase1b_report_v1"
SHA256_PATTERN = re.compile(r"^[0-9a-f]{64}$")
INDEX_DIGEST_METHOD = "sha256_canonical_json_ordered_u32_array_v1"


def _bound_file(binding: Mapping[str, Any], path_key: str, digest_key: str) -> Path:
    path = (ROOT / str(binding[path_key])).resolve()
    if not path.is_file() or sha256(path) != binding[digest_key]:
        raise ValueError(f"immutable binding failed for {path}")
    return path


def validate_manifest(path: Path) -> tuple[dict[str, Any], str]:
    manifest = load_json(path)
    schema = load_json(MANIFEST_SCHEMA)
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(manifest)

    source = manifest["source_binding"]
    for path_key, digest_key in (
        ("data_path", "data_sha256"),
        ("recipe_path", "recipe_sha256"),
        ("rust_example_path", "rust_example_sha256"),
        ("binary_path", "binary_sha256"),
    ):
        _bound_file(source, path_key, digest_key)
    validation_binding = manifest["validation_binding"]
    for path_key, digest_key in (
        ("runner_path", "runner_sha256"),
        ("manifest_schema_path", "manifest_schema_sha256"),
        ("case_schema_path", "case_schema_sha256"),
        ("report_schema_path", "report_schema_sha256"),
    ):
        _bound_file(validation_binding, path_key, digest_key)

    phase0 = manifest["phase0_runtime_binding"]
    report = load_json(_bound_file(phase0, "report_path", "report_sha256"))
    baseline = load_json(
        _bound_file(phase0, "baseline_case_path", "baseline_case_sha256")
    )
    if report.get("status") != "phase0_measurement_complete_no_caps_or_qualification":
        raise ValueError("Phase0 binding is not the completed measurement authority")
    if baseline.get("status") != "survivor_measurement_recorded":
        raise ValueError("Phase0 runtime baseline did not survive its frozen matrix")
    observed_p95 = baseline.get("aggregates", {}).get("p95_elapsed_seconds")
    if not _same_float(observed_p95, phase0["baseline_p95_elapsed_seconds"]):
        raise ValueError("Phase0 runtime baseline p95 drifted")
    baseline_case = baseline.get("case", {})
    if (
        baseline_case.get("replicates") != phase0["baseline_replicates"]
        or baseline_case.get("workers") != phase0["baseline_workers"]
        or baseline_case.get("rows") != 180
        or baseline_case.get("factors") != 3
    ):
        raise ValueError("Phase0 runtime baseline cell drifted")

    expected = [
        ("resource_n180_v9_p18_d18_b5000_w12_seed19001", 1, "resource", 5000, 19001, None),
        ("stability_n180_v9_p18_d18_b5000_w12_seed19002", 2, "stability", 5000, 19002, None),
        ("resource_n180_v9_p18_d18_b10000_w12_seed29001", 3, "resource", 10000, 29001, None),
        ("stability_n180_v9_p18_d18_b10000_w12_seed29002", 4, "stability", 10000, 29002, None),
        ("cancellation_n180_v9_p18_d18_b10000_w12_seed39001", 5, "cancellation", 10000, 39001, 10),
    ]
    cases = manifest["cases"]
    seeds: list[int] = []
    for case, fixed in zip(cases, expected, strict=True):
        observed = (
            case["case_id"],
            case["order"],
            case["mode"],
            case["replicates"],
            case["seed"],
            case["cancel_after"],
        )
        if observed != fixed or case["workers"] != 12:
            raise ValueError(f"Phase1b case drifted: {case['case_id']}")
        seeds.append(case["seed"])
        if case["mode"] != "cancellation":
            derived = (
                float(phase0["baseline_p95_elapsed_seconds"])
                * case["replicates"]
                / phase0["baseline_replicates"]
                * phase0["safety_factor"]
            )
            if not _same_float(case["maximum_single_run_elapsed_seconds"], derived):
                raise ValueError(f"{case['case_id']} single-run ceiling was retrofitted")
            if case["mode"] == "resource":
                if not _same_float(case["maximum_p95_elapsed_seconds"], derived):
                    raise ValueError(f"{case['case_id']} p95 ceiling was retrofitted")
            elif case["maximum_p95_elapsed_seconds"] is not None:
                raise ValueError("single stability evidence cannot claim a p95 ceiling")
        elif (
            case["maximum_single_run_elapsed_seconds"] is not None
            or case["maximum_p95_elapsed_seconds"] is not None
        ):
            raise ValueError("cancellation cannot invent a completion-runtime ceiling")
    if len(seeds) != len(set(seeds)) or {91, 93}.intersection(seeds):
        raise ValueError("Phase1b seeds must be unique and unseen")
    return manifest, sha256(path)


def _same_float(left: Any, right: Any) -> bool:
    return isinstance(left, (int, float)) and not isinstance(left, bool) and math.isclose(
        float(left), float(right), rel_tol=0.0, abs_tol=1.0e-12
    )


def _finite(value: Any) -> bool:
    return (
        isinstance(value, (int, float))
        and not isinstance(value, bool)
        and math.isfinite(float(value))
    )


def _sha(value: Any) -> bool:
    return isinstance(value, str) and SHA256_PATTERN.fullmatch(value) is not None


def _indices_sha256(indices: list[int]) -> str:
    payload = json.dumps(indices, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def clopper_pearson_upper(failures: int, trials: int, confidence: float = 0.95) -> float:
    if (
        type(failures) is not int
        or type(trials) is not int
        or not 0 <= failures <= trials
        or trials <= 0
        or not _finite(confidence)
        or not 0.0 < confidence < 1.0
    ):
        raise ValueError("invalid one-sided binomial-bound inputs")
    if failures == trials:
        return 1.0
    target_log_cdf = math.log1p(-confidence)
    log_combinations = [
        math.lgamma(trials + 1)
        - math.lgamma(successes + 1)
        - math.lgamma(trials - successes + 1)
        for successes in range(failures + 1)
    ]
    lower = 0.0
    upper = 1.0
    for _ in range(128):
        midpoint = (lower + upper) / 2.0
        if (
            _binomial_log_cdf(
                failures, trials, midpoint, log_combinations
            )
            > target_log_cdf
        ):
            lower = midpoint
        else:
            upper = midpoint
    value = upper
    if not math.isfinite(value) or not 0.0 <= value <= 1.0:
        raise ValueError("one-sided Clopper-Pearson upper bound is invalid")
    return value


def _binomial_log_cdf(
    maximum_successes: int,
    trials: int,
    probability: float,
    log_combinations: list[float],
) -> float:
    if probability <= 0.0:
        return 0.0
    if probability >= 1.0:
        return 0.0 if maximum_successes == trials else -math.inf
    log_probability = math.log(probability)
    log_complement = math.log1p(-probability)
    terms = [
        log_combinations[successes]
        + successes * log_probability
        + (trials - successes) * log_complement
        for successes in range(maximum_successes + 1)
    ]
    maximum = max(terms)
    return maximum + math.log(
        math.fsum(math.exp(term - maximum) for term in terms)
    )


def _full_command(binary: Path, result_path: Path, case: Mapping[str, Any]) -> list[str]:
    return command(
        binary,
        result_path,
        rows=180,
        factors=3,
        replicates=int(case["replicates"]),
        workers=int(case["workers"]),
        seed=int(case["seed"]),
        cancel_after=case["cancel_after"],
    )


def _execute(
    binary: Path,
    result_path: Path,
    case: Mapping[str, Any],
    phase: str,
    index: int,
) -> tuple[dict[str, Any], dict[str, Any]]:
    result_path.unlink(missing_ok=True)
    return measured_run(_full_command(binary, result_path, case), result_path, phase, index)


def _dimensions_and_fixture_reasons(
    payload: Mapping[str, Any], case: Mapping[str, Any], manifest: Mapping[str, Any]
) -> list[str]:
    reasons: list[str] = []
    observed_case = payload.get("case", {})
    dimensions = observed_case.get("dimensions", {}) if isinstance(observed_case, Mapping) else {}
    expected_dimensions = manifest["dimension_contract"]
    for key, expected in expected_dimensions.items():
        if dimensions.get(key) != expected:
            reasons.append(f"dimension_{key}_mismatch")
    if (
        observed_case.get("requested_replicates") != case["replicates"]
        or observed_case.get("workers") != case["workers"]
        or observed_case.get("seed") != case["seed"]
    ):
        reasons.append("case_settings_mismatch")
    fixture = payload.get("fixture", {})
    source = manifest["source_binding"]
    if (
        fixture.get("data_path") != source["data_path"]
        or fixture.get("data_sha256") != source["data_sha256"]
        or fixture.get("recipe_path") != source["recipe_path"]
        or fixture.get("recipe_sha256") != source["recipe_sha256"]
    ):
        reasons.append("fixture_binding_mismatch")
    return reasons


def validate_completed_payload(
    payload: Mapping[str, Any], case: Mapping[str, Any], manifest: Mapping[str, Any]
) -> tuple[list[str], dict[str, Any] | None]:
    reasons = _dimensions_and_fixture_reasons(payload, case, manifest)
    if payload.get("status") != "completed":
        reasons.append("completed_status_missing")
    if payload.get("qualification_status") != "measurement_only_no_caps_or_promotion":
        reasons.append("measurement_only_marker_missing")
    if not _sha(payload.get("scientific_result_sha256")):
        reasons.append("scientific_result_digest_missing_or_invalid")
    result = payload.get("result")
    if not isinstance(result, Mapping):
        return sorted(set(reasons + ["result_missing"])), None
    base = result.get("base")
    studentized = result.get("studentized")
    if not isinstance(base, Mapping) or not isinstance(studentized, Mapping):
        return sorted(set(reasons + ["base_or_studentized_result_missing"])), None

    acceptance = manifest["acceptance_contract"]
    requested = int(case["replicates"])
    minimum = max(
        int(acceptance["minimum_usable_floor"]),
        math.ceil(requested * float(acceptance["minimum_usable_fraction"])),
    )
    parameter_ids = base.get("parameter_ids")
    if (
        not isinstance(parameter_ids, list)
        or len(parameter_ids) != 18
        or any(not isinstance(value, str) or not value for value in parameter_ids)
        or len(set(parameter_ids)) != 18
        or studentized.get("parameter_ids") != parameter_ids
    ):
        reasons.append("stable_parameter_identity_or_order_invalid")

    if (
        base.get("method_version") != "cbsem_exact_case_bootstrap_v1"
        or studentized.get("method_version")
        != "cbsem_exact_case_bootstrap_analytic_studentized_interval_v1"
        or base.get("requested_replicates") != requested
        or base.get("attempted_refits") != requested
        or base.get("seed") != case["seed"]
        or base.get("retry_policy") != acceptance["retry_policy"]
        or base.get("max_attempts_per_replicate") != acceptance["max_attempts_per_replicate"]
        or base.get("minimum_usable_replicates") != minimum
        or studentized.get("minimum_usable_replicates") != minimum
        or not _same_float(base.get("minimum_usable_fraction"), 0.9)
        or not _same_float(studentized.get("minimum_usable_fraction"), 0.9)
    ):
        reasons.append("method_schedule_retry_or_minimum_binding_invalid")

    successes = base.get("successful_refits")
    failures = base.get("failed_refits")
    if not isinstance(successes, list) or not isinstance(failures, list):
        return sorted(set(reasons + ["point_refit_ledgers_missing"])), None
    success_indices: list[int] = []
    for row in successes:
        if not isinstance(row, Mapping) or type(row.get("replicate_index")) is not int:
            reasons.append("point_success_index_invalid")
            continue
        success_indices.append(row["replicate_index"])
        estimates = row.get("parameter_estimates")
        if (
            not _sha(row.get("sampling_positions_sha256"))
            or not _sha(row.get("sample_indices_sha256"))
            or not isinstance(estimates, list)
            or len(estimates) != 18
            or any(not _finite(value) for value in estimates)
            or type(row.get("iterations")) is not int
            or row.get("iterations", 0) <= 0
            or not _finite(row.get("objective"))
            or not _finite(row.get("gradient_norm"))
            or float(row.get("gradient_norm", -1.0)) < 0.0
        ):
            reasons.append("point_success_integrity_invalid")
    compact_failures: list[dict[str, Any]] = []
    failure_indices: list[int] = []
    known_failure_kinds = set(acceptance["known_point_failure_kinds"])
    point_failure_counts = {
        kind: 0 for kind in acceptance["known_point_failure_kinds"]
    }
    for row in failures:
        if not isinstance(row, Mapping) or type(row.get("replicate_index")) is not int:
            reasons.append("point_failure_index_invalid")
            continue
        failure_indices.append(row["replicate_index"])
        if (
            row.get("kind") not in known_failure_kinds
            or not isinstance(row.get("message"), str)
            or not row["message"].strip()
            or not _sha(row.get("sampling_positions_sha256"))
            or not _sha(row.get("sample_indices_sha256"))
        ):
            reasons.append("point_failure_untyped_or_invalid")
        elif row.get("kind") in point_failure_counts:
            point_failure_counts[row["kind"]] += 1
        compact_failures.append(
            {
                "replicate_index": row["replicate_index"],
                "kind": row.get("kind"),
                "message": row.get("message"),
                "sampling_positions_sha256": row.get("sampling_positions_sha256"),
                "sample_indices_sha256": row.get("sample_indices_sha256"),
            }
        )
    if (
        success_indices != sorted(success_indices)
        or failure_indices != sorted(failure_indices)
        or sorted(success_indices + failure_indices) != list(range(requested))
        or set(success_indices).intersection(failure_indices)
    ):
        reasons.append("point_ledger_duplicate_missing_or_out_of_order")
    if (
        base.get("usable_replicates") != len(successes)
        or base.get("failed_replicates") != len(failures)
        or len(successes) + len(failures) != requested
    ):
        reasons.append("point_ledger_counts_invalid")

    se_receipts = studentized.get("refit_standard_errors")
    if not isinstance(se_receipts, list):
        return sorted(set(reasons + ["standard_error_ledger_missing"])), None
    se_indices: list[int] = []
    available_indices: list[int] = []
    compact_unavailable: list[dict[str, Any]] = []
    known_se_reasons = set(acceptance["known_standard_error_unavailable_reasons"])
    se_unavailable_counts = {
        reason: 0 for reason in acceptance["known_standard_error_unavailable_reasons"]
    }
    for row in se_receipts:
        if not isinstance(row, Mapping) or type(row.get("replicate_index")) is not int:
            reasons.append("standard_error_replicate_index_invalid")
            continue
        index = row["replicate_index"]
        se_indices.append(index)
        outcome = row.get("outcome")
        if not isinstance(outcome, Mapping):
            reasons.append("standard_error_outcome_missing")
            continue
        if outcome.get("status") == "available":
            standard_errors = outcome.get("standard_errors")
            if (
                outcome.get("information_method")
                != "cbsem_ml_expected_information_delta_method_v1"
                or not isinstance(standard_errors, list)
                or len(standard_errors) != 18
                or any(not _finite(value) or float(value) <= 0.0 for value in standard_errors)
            ):
                reasons.append("available_standard_error_outcome_invalid")
            available_indices.append(index)
        elif outcome.get("status") == "unavailable":
            if outcome.get("reason") not in known_se_reasons:
                reasons.append("standard_error_unavailable_reason_unknown")
            elif outcome.get("reason") in se_unavailable_counts:
                se_unavailable_counts[outcome["reason"]] += 1
            compact_unavailable.append(
                {"replicate_index": index, "reason": outcome.get("reason")}
            )
        else:
            reasons.append("standard_error_outcome_status_unknown")
    if se_indices != success_indices or len(se_receipts) != len(successes):
        reasons.append("standard_error_ledger_does_not_match_point_success_order")
    if studentized.get("studentized_usable_replicates") != len(available_indices):
        reasons.append("studentized_usable_count_invalid")

    base_status = base.get("inference", {}).get("status")
    studentized_status = studentized.get("inference", {}).get("status")
    if base_status != "available" or studentized_status != "available":
        reasons.append("base_or_studentized_inference_unavailable")
    if len(successes) < minimum or len(available_indices) < minimum:
        reasons.append("usable_refits_below_contract_minimum")
    if not _available_interval_order(base.get("intervals"), parameter_ids, "parameter_id"):
        reasons.append("base_interval_identity_or_numeric_projection_invalid")
    if not _studentized_interval_order(studentized.get("intervals"), parameter_ids):
        reasons.append("studentized_interval_identity_or_numeric_projection_invalid")

    metrics = payload.get("metrics", {})
    if (
        metrics.get("successful_point_refits") != len(successes)
        or metrics.get("failed_point_refits") != len(failures)
        or metrics.get("analytic_se_unavailable_refits") != len(compact_unavailable)
        or metrics.get("studentized_usable_refits") != len(available_indices)
    ):
        reasons.append("metrics_do_not_bind_exact_ledgers")
    total_unusable = requested - len(available_indices)
    upper = clopper_pearson_upper(
        total_unusable, requested, acceptance["one_sided_confidence_level"]
    )
    if not upper < acceptance["maximum_total_unusable_fraction_upper_bound"]:
        reasons.append("total_unusable_one_sided_upper_bound_not_below_0_10")

    ledger_integrity_reasons = {
        "stable_parameter_identity_or_order_invalid",
        "method_schedule_retry_or_minimum_binding_invalid",
        "point_success_index_invalid",
        "point_success_integrity_invalid",
        "point_failure_index_invalid",
        "point_failure_untyped_or_invalid",
        "point_ledger_duplicate_missing_or_out_of_order",
        "point_ledger_counts_invalid",
        "standard_error_replicate_index_invalid",
        "standard_error_outcome_missing",
        "available_standard_error_outcome_invalid",
        "standard_error_unavailable_reason_unknown",
        "standard_error_outcome_status_unknown",
        "standard_error_ledger_does_not_match_point_success_order",
        "studentized_usable_count_invalid",
    }
    if ledger_integrity_reasons.intersection(reasons):
        return sorted(set(reasons)), None
    ledger = {
        "validation_status": "exact_partition_and_typed_outcomes_validated",
        "requested_replicates": requested,
        "attempted_refits": base.get("attempted_refits"),
        "point_successes": len(successes),
        "point_failures_count": len(failures),
        "studentized_usable_refits": len(available_indices),
        "standard_error_unavailable_count": len(compact_unavailable),
        "minimum_usable_replicates": minimum,
        "retry_policy": base.get("retry_policy"),
        "max_attempts_per_replicate": base.get("max_attempts_per_replicate"),
        "base_inference_status": base_status,
        "studentized_inference_status": studentized_status,
        "point_partition_complete": sorted(success_indices + failure_indices)
        == list(range(requested)),
        "standard_error_partition_complete": se_indices == success_indices,
        "point_failure_counts_by_kind": point_failure_counts,
        "standard_error_unavailable_counts_by_reason": se_unavailable_counts,
        "point_failures": compact_failures,
        "standard_error_unavailable": compact_unavailable,
        "replicate_indices_digest_method": INDEX_DIGEST_METHOD,
        "successful_replicate_indices_sha256": _indices_sha256(success_indices),
        "standard_error_available_replicate_indices_sha256": _indices_sha256(
            available_indices
        ),
        "total_studentized_unusable": total_unusable,
        "total_studentized_unusable_fraction": total_unusable / requested,
        "one_sided_bound_method": acceptance["one_sided_failure_bound_method"],
        "one_sided_confidence_level": acceptance["one_sided_confidence_level"],
        "one_sided_95_percent_upper_bound": upper,
        "upper_bound_threshold": acceptance[
            "maximum_total_unusable_fraction_upper_bound"
        ],
    }
    return sorted(set(reasons)), ledger


def _available_interval_order(
    rows: Any, parameter_ids: Any, parameter_key: str
) -> bool:
    if not isinstance(rows, list) or not isinstance(parameter_ids, list):
        return False
    if [row.get(parameter_key) for row in rows if isinstance(row, Mapping)] != parameter_ids:
        return False
    numeric = ("original", "bootstrap_mean", "bias", "standard_error", "percentile_lower", "percentile_upper")
    return all(
        isinstance(row, Mapping)
        and all(_finite(row.get(key)) for key in numeric)
        and row.get("percentile_lower") <= row.get("percentile_upper")
        for row in rows
    )


def _studentized_interval_order(rows: Any, parameter_ids: Any) -> bool:
    if not isinstance(rows, list) or not isinstance(parameter_ids, list):
        return False
    if [row.get("parameter_id") for row in rows if isinstance(row, Mapping)] != parameter_ids:
        return False
    for row in rows:
        if not isinstance(row, Mapping) or not isinstance(row.get("outcome"), Mapping):
            return False
        outcome = row["outcome"]
        if outcome.get("status") != "available":
            return False
        keys = (
            "point_estimate",
            "point_standard_error",
            "lower_pivot_quantile",
            "upper_pivot_quantile",
            "interval_lower",
            "interval_upper",
        )
        if (
            any(not _finite(outcome.get(key)) for key in keys)
            or outcome.get("point_standard_error") <= 0.0
            or outcome.get("interval_lower") > outcome.get("interval_upper")
        ):
            return False
    return True


def _resource_reasons(
    observation: Mapping[str, Any],
    payload: Mapping[str, Any],
    case: Mapping[str, Any],
    manifest: Mapping[str, Any],
    total_memory_bytes: int,
) -> list[str]:
    acceptance = manifest["acceptance_contract"]
    reasons: list[str] = []
    if observation.get("orphan_processes") != 0:
        reasons.append("orphan_process_detected")
    if total_memory_bytes <= 0:
        reasons.append("physical_memory_unavailable")
    peak_working_set = observation.get("peak_working_set_bytes")
    if not _finite(peak_working_set) or float(peak_working_set) <= 0.0:
        reasons.append("peak_working_set_missing_or_nonpositive")
    elif total_memory_bytes > 0 and float(peak_working_set) > (
        total_memory_bytes
        * float(acceptance["maximum_peak_working_set_fraction_of_physical_ram"])
    ):
        reasons.append("peak_working_set_exceeded_50_percent_physical_ram")
    elapsed = observation.get("elapsed_seconds")
    if not _finite(elapsed) or float(elapsed) <= 0.0:
        reasons.append("elapsed_seconds_missing_nonfinite_or_nonpositive")
    elif float(elapsed) > float(case["maximum_single_run_elapsed_seconds"]):
        reasons.append("execution_exceeded_predeclared_runtime_ceiling")
    combined = payload.get("metrics", {}).get("combined_s2_json_bytes")
    if not isinstance(combined, int) or combined <= 0:
        reasons.append("combined_s2_size_missing")
    else:
        if combined > acceptance["maximum_text_export_proxy_bytes"]:
            reasons.append("combined_s2_exceeded_128_mib_text_proxy")
        if combined > acceptance["maximum_project_document_proxy_bytes"]:
            reasons.append("combined_s2_exceeded_256_mib_project_proxy")
    return reasons


def run_completed_case(
    binary: Path,
    case: Mapping[str, Any],
    result_path: Path,
    manifest: Mapping[str, Any],
    manifest_sha256: str,
    total_memory_bytes: int,
) -> dict[str, Any]:
    is_resource = case["mode"] == "resource"
    schedule = (
        [("probe", 0), ("warmup", 0)]
        + [("measured", index) for index in range(5)]
        if is_resource
        else [("stability", 0)]
    )
    observations: list[dict[str, Any]] = []
    payloads: list[dict[str, Any]] = []
    ledgers: list[dict[str, Any]] = []
    reasons: list[str] = []
    for phase, index in schedule:
        observation, payload = _execute(binary, result_path, case, phase, index)
        observations.append(observation)
        payloads.append(payload)
        payload_reasons, ledger = validate_completed_payload(payload, case, manifest)
        run_reasons = payload_reasons + _resource_reasons(
            observation, payload, case, manifest, total_memory_bytes
        )
        reasons.extend(run_reasons)
        if ledger is not None:
            ledgers.append(ledger)
        if run_reasons:
            break
    digests = {payload.get("scientific_result_sha256") for payload in payloads}
    if len(digests) != 1 or None in digests:
        reasons.append("same_seed_scientific_digest_drift")
    if len(ledgers) != len(payloads) or any(ledger != ledgers[0] for ledger in ledgers[1:]):
        reasons.append("same_seed_compact_ledger_drift")
    measured = [
        observation
        for observation in observations
        if observation.get("phase") == "measured"
    ]
    aggregates = aggregate_runs(measured) if len(measured) == 5 else None
    expected_runs = 7 if is_resource else 1
    if len(observations) != expected_runs:
        reasons.append("predeclared_resource_or_stability_run_set_incomplete")
    if is_resource and (
        aggregates is None
        or float(aggregates["p95_elapsed_seconds"])
        > float(case["maximum_p95_elapsed_seconds"])
    ):
        reasons.append("measured_p95_exceeded_predeclared_runtime_ceiling")
    reasons = sorted(set(reasons))
    representative = payloads[-1]
    dimensions = _project_dimensions(representative, manifest)
    return {
        "schema_version": 1,
        "kind": CASE_KIND,
        "case": dict(case),
        "manifest_sha256": manifest_sha256,
        "status": "accepted_resource_candidate_evidence" if not reasons else "rejected",
        "acceptance_reasons": reasons,
        "probe": observations[0],
        "warmup_runs": [row for row in observations if row.get("phase") == "warmup"],
        "measured_runs": measured,
        "aggregates": aggregates,
        "dimensions": dimensions,
        "scientific_result_sha256": next(iter(digests)) if len(digests) == 1 else None,
        "ledger": ledgers[-1] if ledgers else None,
        "runtime_acceptance": {
            "evidence_role": "same_seed_resource_repeats_not_independent"
            if is_resource
            else "single_independent_stability_seed",
            "maximum_single_run_elapsed_seconds": case[
                "maximum_single_run_elapsed_seconds"
            ],
            "maximum_p95_elapsed_seconds": case["maximum_p95_elapsed_seconds"],
            "observed_p95_elapsed_seconds": None
            if aggregates is None
            else aggregates["p95_elapsed_seconds"],
        },
        "cancellation": None,
        "qualification_boundary": manifest["qualification_boundary"],
    }


def run_cancellation_case(
    binary: Path,
    case: Mapping[str, Any],
    result_path: Path,
    manifest: Mapping[str, Any],
    manifest_sha256: str,
    total_memory_bytes: int,
) -> dict[str, Any]:
    observation, payload = _execute(binary, result_path, case, "cancellation", 0)
    reasons = _dimensions_and_fixture_reasons(payload, case, manifest)
    acceptance = manifest["acceptance_contract"]
    latency = payload.get("cancellation_latency_seconds")
    if payload.get("status") != "cancelled_as_requested":
        reasons.append("typed_cancellation_outcome_missing")
    if not _finite(latency) or float(latency) > acceptance[
        "maximum_cancellation_terminal_latency_seconds"
    ]:
        reasons.append("cancellation_terminal_latency_exceeded_one_second")
    if observation.get("orphan_processes") != 0:
        reasons.append("orphan_process_detected")
    if total_memory_bytes <= 0:
        reasons.append("physical_memory_unavailable")
    peak_working_set = observation.get("peak_working_set_bytes")
    if not _finite(peak_working_set) or float(peak_working_set) <= 0.0:
        reasons.append("peak_working_set_missing_or_nonpositive")
    elif total_memory_bytes > 0 and float(peak_working_set) > (
        total_memory_bytes
        * float(acceptance["maximum_peak_working_set_fraction_of_physical_ram"])
    ):
        reasons.append("peak_working_set_exceeded_50_percent_physical_ram")
    elapsed = observation.get("elapsed_seconds")
    if not _finite(elapsed) or float(elapsed) <= 0.0:
        reasons.append("elapsed_seconds_missing_nonfinite_or_nonpositive")
    reasons = sorted(set(reasons))
    return {
        "schema_version": 1,
        "kind": CASE_KIND,
        "case": dict(case),
        "manifest_sha256": manifest_sha256,
        "status": "accepted_resource_candidate_evidence" if not reasons else "rejected",
        "acceptance_reasons": reasons,
        "probe": observation,
        "warmup_runs": [],
        "measured_runs": [],
        "aggregates": None,
        "dimensions": _project_dimensions(payload, manifest),
        "scientific_result_sha256": None,
        "ledger": None,
        "runtime_acceptance": None,
        "cancellation": {
            "typed_status": payload.get("status"),
            "terminal_latency_seconds": latency,
        },
        "qualification_boundary": manifest["qualification_boundary"],
    }


def rejected_case_receipt(
    case: Mapping[str, Any],
    manifest: Mapping[str, Any],
    manifest_sha256: str,
    reason: str,
) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "kind": CASE_KIND,
        "case": dict(case),
        "manifest_sha256": manifest_sha256,
        "status": "rejected",
        "acceptance_reasons": [reason],
        "probe": None,
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


def _project_dimensions(
    payload: Mapping[str, Any], manifest: Mapping[str, Any]
) -> dict[str, int] | None:
    dimensions = payload.get("case", {}).get("dimensions", {})
    if not isinstance(dimensions, Mapping):
        return None
    keys = tuple(manifest["dimension_contract"])
    if any(type(dimensions.get(key)) is not int for key in keys):
        return None
    return {key: dimensions[key] for key in keys}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--output-root", type=Path, default=DEFAULT_OUTPUT_ROOT)
    parser.add_argument("--run-id")
    parser.add_argument("--execute", action="store_true")
    args = parser.parse_args()
    if args.manifest.resolve() != DEFAULT_MANIFEST.resolve():
        parser.error("Phase1b accepts only its checked-in predeclared manifest")
    manifest, manifest_sha256 = validate_manifest(args.manifest.resolve())
    if not args.execute:
        print(
            json.dumps(
                {
                    "status": "dry_run_no_workloads_executed",
                    "manifest_sha256": manifest_sha256,
                    "binary_sha256": manifest["source_binding"]["binary_sha256"],
                    "cases": [case["case_id"] for case in manifest["cases"]],
                    "qualification_status": "blocked_not_product_qualification",
                },
                indent=2,
            )
        )
        return 0

    binary = (ROOT / manifest["source_binding"]["binary_path"]).resolve()
    run_id = args.run_id or f"phase1b-{uuid.uuid4()}"
    if re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]*", run_id) is None:
        parser.error("--run-id must be one path-safe identifier")
    output_root = args.output_root.resolve()
    validation_results = (ROOT / "validation/results").resolve()
    if not output_root.is_relative_to(validation_results):
        parser.error("--output-root must remain under validation/results")
    run_directory = output_root / run_id
    if run_directory.exists():
        parser.error(f"append-only run directory already exists: {run_directory}")
    run_directory.mkdir(parents=True)

    total_memory_bytes = detected_total_memory_bytes()
    case_schema = load_json(CASE_SCHEMA)
    Draft202012Validator.check_schema(case_schema)
    receipt_rows: list[dict[str, Any]] = []
    receipts: list[dict[str, Any]] = []
    with tempfile.TemporaryDirectory(prefix="qpls-phase1b-") as raw:
        result_path = Path(raw) / "result.json"
        for case in manifest["cases"]:
            try:
                if case["mode"] == "cancellation":
                    receipt = run_cancellation_case(
                        binary,
                        case,
                        result_path,
                        manifest,
                        manifest_sha256,
                        total_memory_bytes,
                    )
                else:
                    receipt = run_completed_case(
                        binary,
                        case,
                        result_path,
                        manifest,
                        manifest_sha256,
                        total_memory_bytes,
                    )
            except Exception as error:  # preserve independent later cells
                receipt = rejected_case_receipt(
                    case,
                    manifest,
                    manifest_sha256,
                    f"child_execution_error:{type(error).__name__}:{error}",
                )
            try:
                Draft202012Validator(case_schema).validate(receipt)
            except Exception as schema_error:
                receipt = rejected_case_receipt(
                    case,
                    manifest,
                    manifest_sha256,
                    f"receipt_schema_error:{type(schema_error).__name__}:{schema_error}",
                )
                Draft202012Validator(case_schema).validate(receipt)
            receipt_path = run_directory / "cases" / f"{case['case_id']}.json"
            digest = write_new_json(receipt_path, receipt)
            receipts.append(receipt)
            receipt_rows.append(
                {
                    "case_id": case["case_id"],
                    "path": receipt_path.relative_to(ROOT).as_posix(),
                    "sha256": digest,
                    "status": receipt["status"],
                    "acceptance_reasons": receipt["acceptance_reasons"],
                }
            )

    accepted = sum(
        receipt["status"] == "accepted_resource_candidate_evidence"
        for receipt in receipts
    )
    all_accepted = accepted == len(receipts)
    reasons = sorted(
        {
            reason
            for receipt in receipts
            for reason in receipt["acceptance_reasons"]
        }
    )
    report = {
        "schema_version": 1,
        "kind": REPORT_KIND,
        "run_id": run_id,
        "status": (
            "phase1b_applied_resource_candidate_evidence_passed_qualification_blocked"
            if all_accepted
            else "phase1b_applied_resource_candidate_evidence_failed_qualification_blocked"
        ),
        "manifest_path": DEFAULT_MANIFEST.relative_to(ROOT).as_posix(),
        "manifest_sha256": manifest_sha256,
        "source_binding": manifest["source_binding"],
        "validation_binding": manifest["validation_binding"],
        "physical_memory_bytes": total_memory_bytes,
        "hardware": {
            "os": platform.system(),
            "os_release": platform.release(),
            "architecture": platform.machine(),
            "processor": platform.processor(),
            "logical_cores": os.cpu_count(),
        },
        "case_receipts": receipt_rows,
        "acceptance_summary": {
            "status": "all_predeclared_cases_accepted"
            if all_accepted
            else "one_or_more_predeclared_cases_rejected",
            "reasons": reasons,
            "accepted_cases": accepted,
            "total_cases": len(receipts),
        },
        "qualification_boundary": manifest["qualification_boundary"],
    }
    report_schema = load_json(REPORT_SCHEMA)
    Draft202012Validator.check_schema(report_schema)
    Draft202012Validator(report_schema).validate(report)
    write_new_json(run_directory / "phase1b-report.json", report)
    print(json.dumps({"status": report["status"], "run_directory": str(run_directory)}, indent=2))
    return 0 if all_accepted else 2


if __name__ == "__main__":
    raise SystemExit(main())
