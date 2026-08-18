#!/usr/bin/env python3
"""Transparent PLSc consistent-bootstrap v1 arithmetic microreference.

This standard-library implementation intentionally does not implement PLSc.
It independently checks the frozen indexed-ledger hashes and the arithmetic
performed after full-refit PLSc parameter values already exist.  Its fixture
is explicitly non-promotional and below the product replicate minimum.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import struct
import sys
from pathlib import Path
from statistics import NormalDist
from typing import Any, Mapping, Sequence


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_FIXTURE = (
    ROOT / "validation" / "fixtures" / "consistent_bootstrap_v1_microcases.json"
)

SAMPLE_DIGEST_DOMAIN = b"QuickPLS PLSc consistent bootstrap sample v1\0"
PARAMETER_DIGEST_DOMAIN = b"QuickPLS PLSc consistent bootstrap parameters v1\0"
EXPECTED_IDENTITY = {
    "capability_id": "smartpls.consistent_bootstrapping",
    "cell_id": "qpls3.inference.consistent_bootstrap",
    "method_version": "plsc_bootstrap_v1",
    "estimator_method_version": "plsc_v2",
    "resampling_method_version": "indexed_resampling_v4",
    "operation": "plsc_consistent_bootstrap_v1",
    "retry_policy": "no_retry_no_replacement_fixed_indexed_draws_v1",
}
ALLOWED_FAILURE_REASONS = {
    "cancelled",
    "inadmissible_rho_a",
    "inadmissible_corrected_correlation",
    "plsc_nonconvergence",
    "singular_plsc_equation",
    "nonfinite_plsc_parameter",
    "parameter_identity_mismatch",
    "plsc_refit_failed",
}
REQUIRED_BOUNDARY_IDS = {
    "replicate_count_999",
    "replicate_count_10001",
    "studentized_requested",
    "permutation_requested",
    "raw_rows_required",
    "ordinary_pls_point_rejected",
    "formative_construct_rejected",
    "single_indicator_construct_rejected",
    "generated_interaction_rejected",
    "higher_order_construct_rejected",
    "group_inference_rejected",
    "case_weights_rejected",
    "pca_weighting_rejected",
    "insufficient_usable_replicates",
    "duplicate_ledger_index",
    "missing_ledger_index",
    "out_of_order_ledger_index",
    "sample_digest_tamper",
    "parameter_identity_tamper",
    "parameter_digest_tamper",
    "unknown_failure_reason",
    "delete_one_refit_failure",
    "nonfinite_parameter",
    "cancel_before_resampling",
    "cancel_during_resampling",
    "archive_method_version_tamper",
    "archive_dataset_fingerprint_tamper",
    "archive_member_checksum_tamper",
    "archive_duplicate_member",
    "archive_malformed_payload",
    "archive_legacy_reinterpretation",
    "archive_interrupted_save",
    "native_projection_wrong_feature",
    "native_projection_incomplete_accounting",
    "export_wrong_source_run",
    "product_hidden_while_evidence_absent",
}


class DuplicateKeyError(ValueError):
    """Raised when a supposedly immutable fixture contains a duplicate key."""


def _strict_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise DuplicateKeyError(f"duplicate JSON key: {key!r}")
        result[key] = value
    return result


def strict_load_json(path: Path) -> Any:
    return json.loads(
        path.read_text(encoding="utf-8"),
        object_pairs_hook=_strict_pairs,
        parse_constant=lambda value: (_ for _ in ()).throw(
            ValueError(f"non-finite JSON constant: {value}")
        ),
    )


def _u64_le(value: int) -> bytes:
    if isinstance(value, bool) or not isinstance(value, int) or not 0 <= value < 2**64:
        raise ValueError(f"value is not an unsigned 64-bit integer: {value!r}")
    return struct.pack("<Q", value)


def rust_scientific_12(value: float) -> str:
    """Match Rust ``format!(\"{value:.12e}\")`` exponent spelling."""

    number = float(value)
    if not math.isfinite(number):
        raise ValueError("parameter values must be finite")
    mantissa, exponent = format(number, ".12e").split("e")
    return f"{mantissa}e{int(exponent)}"


def sample_indices_sha256(indices: Sequence[int]) -> str:
    digest = hashlib.sha256()
    digest.update(SAMPLE_DIGEST_DOMAIN)
    digest.update(_u64_le(len(indices)))
    for index in indices:
        digest.update(_u64_le(index))
    return digest.hexdigest()


def parameter_values_sha256(values: Mapping[str, float]) -> str:
    digest = hashlib.sha256()
    digest.update(PARAMETER_DIGEST_DOMAIN)
    digest.update(_u64_le(len(values)))
    for parameter in sorted(values):
        if not isinstance(parameter, str):
            raise ValueError("parameter identities must be UTF-8 strings")
        parameter_bytes = parameter.encode("utf-8")
        value_bytes = rust_scientific_12(values[parameter]).encode("utf-8")
        digest.update(_u64_le(len(parameter_bytes)))
        digest.update(parameter_bytes)
        digest.update(_u64_le(len(value_bytes)))
        digest.update(value_bytes)
    return digest.hexdigest()


def type7_quantile(values: Sequence[float], probability: float) -> float:
    ordered = sorted(float(value) for value in values)
    if not ordered or any(not math.isfinite(value) for value in ordered):
        raise ValueError("Type-7 quantiles require finite non-empty values")
    if not math.isfinite(probability) or not 0.0 <= probability <= 1.0:
        raise ValueError("quantile probability must be in [0, 1]")
    position = (len(ordered) - 1) * probability
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return ordered[lower]
    return ordered[lower] + (position - lower) * (ordered[upper] - ordered[lower])


def sample_standard_error(values: Sequence[float]) -> float:
    numbers = [float(value) for value in values]
    if len(numbers) < 2 or any(not math.isfinite(value) for value in numbers):
        raise ValueError("sample standard error requires at least two finite values")
    mean = math.fsum(numbers) / len(numbers)
    return math.sqrt(math.fsum((value - mean) ** 2 for value in numbers) / (len(numbers) - 1))


def normal_reference_test(original: float, standard_error: float) -> dict[str, Any]:
    if (
        not math.isfinite(original)
        or not math.isfinite(standard_error)
        or standard_error <= sys.float_info.epsilon
    ):
        return {"available": False, "t_statistic": None, "p_value_two_sided": None}
    statistic = original / standard_error
    if not math.isfinite(statistic):
        return {"available": False, "t_statistic": None, "p_value_two_sided": None}
    probability = math.erfc(abs(statistic) / math.sqrt(2.0))
    return {
        "available": True,
        "t_statistic": statistic,
        "p_value_two_sided": min(1.0, max(0.0, probability)),
    }


def bca_interval(
    bootstrap_values: Sequence[float],
    original: float,
    jackknife_values: Sequence[float],
    confidence_level: float,
) -> dict[str, Any]:
    bootstrap = [float(value) for value in bootstrap_values]
    jackknife = [float(value) for value in jackknife_values]
    if (
        len(bootstrap) < 2
        or len(jackknife) < 3
        or not math.isfinite(original)
        or not math.isfinite(confidence_level)
        or not 0.0 < confidence_level < 1.0
        or any(not math.isfinite(value) for value in bootstrap)
        or any(not math.isfinite(value) for value in jackknife)
    ):
        return {"available": False, "reason": "invalid_bca_inputs"}

    count = len(bootstrap)
    below = sum(value < original for value in bootstrap)
    tied = sum(value == original for value in bootstrap)
    probability = (below + 0.5 * tied) / count
    probability = min(1.0 - 0.5 / count, max(0.5 / count, probability))
    normal = NormalDist()
    bias_correction = normal.inv_cdf(probability)

    jackknife_mean = math.fsum(jackknife) / len(jackknife)
    centered = [jackknife_mean - value for value in jackknife]
    sum_squares = math.fsum(value * value for value in centered)
    if not math.isfinite(sum_squares) or sum_squares <= sys.float_info.epsilon:
        return {"available": False, "reason": "degenerate_jackknife_acceleration"}
    acceleration = math.fsum(value**3 for value in centered) / (
        6.0 * sum_squares**1.5
    )
    if not math.isfinite(acceleration):
        return {"available": False, "reason": "nonfinite_jackknife_acceleration"}

    def adjusted_probability(nominal: float) -> float | None:
        z_value = normal.inv_cdf(nominal)
        denominator = 1.0 - acceleration * (bias_correction + z_value)
        if not math.isfinite(denominator) or abs(denominator) <= sys.float_info.epsilon:
            return None
        adjusted = normal.cdf(
            bias_correction + (bias_correction + z_value) / denominator
        )
        return min(1.0, max(0.0, adjusted)) if math.isfinite(adjusted) else None

    tail = (1.0 - confidence_level) / 2.0
    lower_probability = adjusted_probability(tail)
    upper_probability = adjusted_probability(1.0 - tail)
    if (
        lower_probability is None
        or upper_probability is None
        or lower_probability > upper_probability
    ):
        return {"available": False, "reason": "undefined_adjusted_quantiles"}
    return {
        "available": True,
        "reason": None,
        "bias_correction": bias_correction,
        "acceleration": acceleration,
        "lower": type7_quantile(bootstrap, lower_probability),
        "upper": type7_quantile(bootstrap, upper_probability),
    }


def summarize_parameter(
    values: Sequence[float],
    original: float,
    jackknife_values: Sequence[float],
    confidence_level: float,
) -> dict[str, Any]:
    bootstrap_mean = math.fsum(values) / len(values)
    standard_error = sample_standard_error(values)
    tail = (1.0 - confidence_level) / 2.0
    return {
        "bootstrap_mean": bootstrap_mean,
        "bias": bootstrap_mean - original,
        "sample_standard_error": standard_error,
        "normal_reference": normal_reference_test(original, standard_error),
        "percentile_lower": type7_quantile(values, tail),
        "percentile_upper": type7_quantile(values, 1.0 - tail),
        "bca": bca_interval(values, original, jackknife_values, confidence_level),
    }


def _close(actual: float, expected: float) -> bool:
    return math.isclose(actual, expected, rel_tol=1e-12, abs_tol=1e-12)


def _expect_number(
    errors: list[str], case_id: str, label: str, actual: float, expected: Any
) -> None:
    if isinstance(expected, bool) or not isinstance(expected, (int, float)):
        errors.append(f"{case_id}: expected {label} is not numeric")
    elif not _close(actual, float(expected)):
        errors.append(f"{case_id}: {label} expected {expected!r}, got {actual!r}")


def validate_fixture(document: Any) -> dict[str, Any]:
    errors: list[str] = []
    checks: list[str] = []
    if not isinstance(document, dict):
        return {
            "passed": False,
            "fixture_only": True,
            "qualification_evidence": False,
            "checks": [],
            "errors": ["fixture root must be an object"],
        }
    if document.get("schema_version") != 1:
        errors.append("fixture schema_version must equal 1")
    if document.get("identity") != EXPECTED_IDENTITY:
        errors.append("fixture method identity differs from the frozen v1 contract")
    scope = document.get("scope", {})
    if not isinstance(scope, dict) or scope.get("qualification_evidence") is not False:
        errors.append("fixture must explicitly declare qualification_evidence=false")
    if scope.get("fixture_kind") != "contract_microcase_non_product_replicate_count":
        errors.append("fixture must be labelled as a non-product contract microcase")
    if scope.get("product_minimum_replicates") != 1000 or scope.get(
        "product_maximum_replicates"
    ) != 10000:
        errors.append("fixture product replicate limits differ from v1")

    digest_contract = document.get("digest_contract", {})
    ordering_case = digest_contract.get("ordering_case", {}) if isinstance(digest_contract, dict) else {}
    entries = ordering_case.get("parameters_in_noncanonical_input_order", [])
    try:
        ordering_parameters = {entry["key"]: entry["value"] for entry in entries}
        if len(ordering_parameters) != len(entries):
            errors.append("digest ordering case contains a duplicate parameter identity")
        actual_canonical = {
            key: rust_scientific_12(value)
            for key, value in sorted(ordering_parameters.items())
        }
        if actual_canonical != ordering_case.get("canonical_scientific_values"):
            errors.append("digest ordering case canonical scientific values differ")
        if parameter_values_sha256(ordering_parameters) != ordering_case.get("expected_sha256"):
            errors.append("digest ordering case SHA-256 differs")
        checks.append("archive_stable_parameter_digest_ordering")
    except (KeyError, TypeError, ValueError) as error:
        errors.append(f"digest ordering case is malformed: {error}")

    microcases = document.get("microcases")
    if not isinstance(microcases, list):
        errors.append("microcases must be an array")
        microcases = []
    cases_by_id = {
        case.get("id"): case for case in microcases if isinstance(case, dict)
    }
    if len(cases_by_id) != len(microcases):
        errors.append("microcase identities must be unique non-null strings")

    primary = cases_by_id.get("symmetric_one_parameter_with_retained_failure")
    if not isinstance(primary, dict):
        errors.append("symmetric ledger microcase is missing")
    else:
        try:
            original = primary["original_parameters"]
            if not isinstance(original, dict) or len(original) != 1:
                raise ValueError("primary fixture requires exactly one original parameter")
            if parameter_values_sha256(original) != primary[
                "expected_original_parameter_values_sha256"
            ]:
                errors.append("primary original-parameter digest differs")
            ledger = primary["ledger"]
            requested = primary["requested_replicates"]
            if len(ledger) != requested:
                errors.append("primary ledger length differs from requested count")
            required = max(2, math.ceil(requested * primary["minimum_usable_fraction"]))
            if required != primary["required_usable_replicates"]:
                errors.append("primary minimum usable count differs from ceil(B * 0.9)")
            success_values: dict[str, list[float]] = {key: [] for key in original}
            failed = 0
            for position, row in enumerate(ledger):
                if row.get("replicate_index") != position:
                    errors.append("primary ledger is not in exact replicate-index order")
                if sample_indices_sha256(row["sample_indices"]) != row.get(
                    "expected_sample_indices_sha256"
                ):
                    errors.append(f"primary ledger sample digest differs at index {position}")
                if row.get("status") == "success":
                    parameters = row.get("parameters")
                    if not isinstance(parameters, dict) or set(parameters) != set(original):
                        errors.append(f"primary parameter identity differs at index {position}")
                        continue
                    if parameter_values_sha256(parameters) != row.get(
                        "expected_parameter_values_sha256"
                    ):
                        errors.append(f"primary parameter digest differs at index {position}")
                    for key, value in parameters.items():
                        success_values[key].append(float(value))
                    if any(key in row for key in ("reason_code", "message")):
                        errors.append(f"successful primary row {position} contains failure data")
                elif row.get("status") == "failed":
                    failed += 1
                    if row.get("reason_code") not in ALLOWED_FAILURE_REASONS:
                        errors.append(f"primary row {position} has an unknown failure reason")
                    if not isinstance(row.get("message"), str) or not row["message"].strip():
                        errors.append(f"primary row {position} has no failure diagnostic")
                    if "parameters" in row or "expected_parameter_values_sha256" in row:
                        errors.append(f"failed primary row {position} contains parameter data")
                else:
                    errors.append(f"primary row {position} has an unknown status")
            usable = sum(len(values) for values in success_values.values()) // len(original)
            expected = primary["expected"]
            if usable != expected["usable_replicates"] or failed != expected["failed_replicates"]:
                errors.append("primary usable/failed counts differ")
            if usable < required:
                errors.append("primary hand fixture unexpectedly fails its arithmetic 90% rule")
            parameter, original_value = next(iter(original.items()))
            summary = summarize_parameter(
                success_values[parameter],
                float(original_value),
                primary["jackknife_parameter_values"],
                primary["confidence_level"],
            )
            for label in (
                "bootstrap_mean",
                "bias",
                "sample_standard_error",
                "percentile_lower",
                "percentile_upper",
            ):
                _expect_number(errors, primary["id"], label, summary[label], expected[label])
            normal = summary["normal_reference"]
            if normal["available"] is not True:
                errors.append("primary normal-reference diagnostic is unexpectedly unavailable")
            else:
                _expect_number(
                    errors,
                    primary["id"],
                    "normal_reference_t_statistic",
                    normal["t_statistic"],
                    expected["normal_reference_t_statistic"],
                )
                _expect_number(
                    errors,
                    primary["id"],
                    "normal_reference_two_sided_p_value",
                    normal["p_value_two_sided"],
                    expected["normal_reference_two_sided_p_value"],
                )
            bca = summary["bca"]
            if bca["available"] is not expected["bca_available"]:
                errors.append("primary BCa availability differs")
            if bca["available"]:
                for actual_key, expected_key in (
                    ("bias_correction", "bca_bias_correction"),
                    ("acceleration", "bca_acceleration"),
                    ("lower", "bca_lower"),
                    ("upper", "bca_upper"),
                ):
                    _expect_number(
                        errors,
                        primary["id"],
                        expected_key,
                        bca[actual_key],
                        expected[expected_key],
                    )
            checks.extend(
                [
                    "ordered_failure_ledger_and_exact_accounting",
                    "sample_and_parameter_sha256_contract",
                    "sample_standard_error",
                    "type7_percentile_interval",
                    "two_sided_normal_reference",
                    "midrank_full_delete_one_bca_arithmetic",
                ]
            )
        except (KeyError, TypeError, ValueError, ZeroDivisionError) as error:
            errors.append(f"primary microcase is malformed: {error}")

    degenerate = cases_by_id.get("degenerate_standard_error_and_acceleration")
    if not isinstance(degenerate, dict):
        errors.append("degenerate arithmetic microcase is missing")
    else:
        try:
            result = summarize_parameter(
                degenerate["bootstrap_parameter_values"],
                degenerate["original"],
                degenerate["jackknife_parameter_values"],
                degenerate["confidence_level"],
            )
            expected = degenerate["expected"]
            for label in (
                "bootstrap_mean",
                "bias",
                "sample_standard_error",
                "percentile_lower",
                "percentile_upper",
            ):
                _expect_number(errors, degenerate["id"], label, result[label], expected[label])
            if result["normal_reference"]["available"] is not False:
                errors.append("degenerate normal-reference diagnostic must be unavailable")
            if result["bca"]["available"] is not False or result["bca"].get(
                "reason"
            ) != expected["bca_unavailable_reason"]:
                errors.append("degenerate BCa failure semantics differ")
            checks.append("degenerate_se_and_bca_fail_closed")
        except (KeyError, TypeError, ValueError, ZeroDivisionError) as error:
            errors.append(f"degenerate microcase is malformed: {error}")

    declared_reasons = document.get("allowed_primary_failure_reason_codes")
    if not isinstance(declared_reasons, list) or set(declared_reasons) != ALLOWED_FAILURE_REASONS:
        errors.append("allowed primary failure-reason set differs from v1")
    boundaries = document.get("boundary_cases")
    if not isinstance(boundaries, list):
        errors.append("boundary_cases must be an array")
        boundaries = []
    boundary_ids = [case.get("id") for case in boundaries if isinstance(case, dict)]
    if len(boundary_ids) != len(set(boundary_ids)):
        errors.append("boundary case identities must be unique")
    missing_boundaries = sorted(REQUIRED_BOUNDARY_IDS - set(boundary_ids))
    unexpected_boundaries = sorted(set(boundary_ids) - REQUIRED_BOUNDARY_IDS)
    if missing_boundaries:
        errors.append(f"required boundary cases are missing: {missing_boundaries}")
    if unexpected_boundaries:
        errors.append(f"unexpected boundary cases are present: {unexpected_boundaries}")
    if any(
        not isinstance(case, dict)
        or not isinstance(case.get("stage"), str)
        or not isinstance(case.get("expected"), str)
        or not case["expected"].strip()
        for case in boundaries
    ):
        errors.append("every boundary case requires a stage and expected behavior")
    checks.append("boundary_and_failure_case_inventory")

    return {
        "passed": not errors,
        "fixture_only": True,
        "qualification_evidence": False,
        "method_version": EXPECTED_IDENTITY["method_version"],
        "microcase_count": len(microcases),
        "boundary_case_count": len(boundaries),
        "checks": checks,
        "errors": errors,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("fixture", nargs="?", type=Path, default=DEFAULT_FIXTURE)
    args = parser.parse_args()
    try:
        document = strict_load_json(args.fixture)
        report = validate_fixture(document)
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as error:
        report = {
            "passed": False,
            "fixture_only": True,
            "qualification_evidence": False,
            "checks": [],
            "errors": [f"cannot load fixture: {type(error).__name__}: {error}"],
        }
    print(json.dumps(report, indent=2, sort_keys=True, allow_nan=False))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
