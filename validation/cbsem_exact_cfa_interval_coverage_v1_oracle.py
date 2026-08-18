#!/usr/bin/env python3
"""Independent SciPy arithmetic oracle for exact-CFA coverage evidence.

This module does not call QuickPLS and does not generate campaign decisions.
It reconstructs interval arithmetic and exact binomial bounds from serialized
ledgers. The validation runner compares its own counts/bounds to this receipt.
"""

from __future__ import annotations

import argparse
import json
import math
import struct
from pathlib import Path
from typing import Any, Mapping, Sequence

import numpy as np
from scipy.stats import beta, bootstrap, norm

ENDPOINT_RELATIVE_TOLERANCE = 1.0e-12
ENDPOINT_ABSOLUTE_TOLERANCE = 1.0e-15
IEEE_BINARY64_UNIT_ROUNDOFF = 2.0**-53
PERCENTILE_MEAN_VALIDATION_METHOD = (
    "math_fsum_reference_higham_gamma_b_minus_1_outward_binary64_v1"
)
PERCENTILE_BIAS_BINDING_METHOD = "observed_mean_minus_point_binary64_bit_exact_v1"
BCA_ACCELERATION_METHOD = (
    "complete_delete_one_jackknife_neumaier_mean_squares_cubes_acceleration_v2"
)
BCA_ADJUSTED_PROBABILITY_METHOD = (
    "efron_bca_statrs_inverse_normal_libm_erfc_cdf_adjustment_v2"
)


def clopper_pearson_two_sided(
    successes: int, trials: int, confidence: float = 0.95
) -> tuple[float, float]:
    _validate_binomial(successes, trials, confidence)
    alpha = 1.0 - confidence
    lower = 0.0 if successes == 0 else float(beta.ppf(alpha / 2.0, successes, trials - successes + 1))
    upper = 1.0 if successes == trials else float(beta.ppf(1.0 - alpha / 2.0, successes + 1, trials - successes))
    return lower, upper


def bonferroni_clopper_pearson_two_sided(
    successes: int,
    trials: int,
    cells: int,
    family_confidence: float = 0.95,
) -> tuple[float, float]:
    if type(cells) is not int or cells <= 0:
        raise ValueError("cells must be a positive integer")
    marginal = 1.0 - (1.0 - family_confidence) / cells
    return clopper_pearson_two_sided(successes, trials, marginal)


def bonferroni_clopper_pearson_lower(
    successes: int,
    trials: int,
    cells: int,
    family_confidence: float = 0.95,
) -> float:
    _validate_binomial(successes, trials, family_confidence)
    if type(cells) is not int or cells <= 0:
        raise ValueError("cells must be a positive integer")
    if successes == 0:
        return 0.0
    alpha = (1.0 - family_confidence) / cells
    return float(beta.ppf(alpha, successes, trials - successes + 1))


def _validate_binomial(successes: int, trials: int, confidence: float) -> None:
    if (
        type(successes) is not int
        or type(trials) is not int
        or trials <= 0
        or successes < 0
        or successes > trials
        or not math.isfinite(confidence)
        or not 0.0 < confidence < 1.0
    ):
        raise ValueError("invalid exact-binomial request")


def type7(values: Sequence[float], probability: float) -> float:
    array = np.asarray(values, dtype=np.float64)
    if array.ndim != 1 or array.size == 0 or not np.isfinite(array).all():
        raise ValueError("type-7 input must be a nonempty finite vector")
    if not math.isfinite(probability) or not 0.0 <= probability <= 1.0:
        raise ValueError("type-7 probability is invalid")
    return float(np.quantile(array, probability, method="linear"))


def dataset_cluster_failure_summary(
    values: Sequence[float],
    seed: int,
    resamples: int = 10_000,
    confidence: float = 0.95,
) -> dict[str, float]:
    """Independent SciPy percentile-bootstrap summary over dataset clusters."""

    array = np.asarray(values, dtype=np.float64)
    if (
        array.ndim != 1
        or array.size < 2
        or not np.isfinite(array).all()
        or np.any((array < 0.0) | (array > 1.0))
        or type(seed) is not int
        or seed < 0
        or type(resamples) is not int
        or resamples <= 0
        or not 0.0 < confidence < 1.0
    ):
        raise ValueError("invalid dataset-cluster failure summary request")
    rng = np.random.Generator(np.random.PCG64DXSM(seed))
    interval = bootstrap(
        (array,),
        np.mean,
        n_resamples=resamples,
        vectorized=True,
        paired=False,
        confidence_level=confidence,
        method="percentile",
        rng=rng,
    ).confidence_interval
    quantiles = np.quantile(array, [0.05, 0.5, 0.95], method="linear")
    return {
        "mean": float(np.mean(array)),
        "p05": float(quantiles[0]),
        "p50": float(quantiles[1]),
        "p95": float(quantiles[2]),
        "cluster_bootstrap_mean_ci_lower": float(interval.low),
        "cluster_bootstrap_mean_ci_upper": float(interval.high),
    }


def audit_engine_case(document: Mapping[str, Any]) -> dict[str, Any]:
    """Recompute all available interval endpoints and ledger-derived statuses."""

    if document.get("status") != "completed":
        raise ValueError("oracle accepts only a completed engine case")
    original = document["original"]
    combined = document["bootstrap"]
    base = combined["base"]
    studentized = combined["studentized"]
    bca = document["bca"]
    truth_rows = document["truth"]
    parameter_ids = [row["parameter_id"] for row in original["refit"]["free_parameters"]]
    if [row["parameter_id"] for row in truth_rows] != parameter_ids:
        raise ValueError("truth order differs from point-result order")
    if base["parameter_ids"] != parameter_ids or studentized["parameter_ids"] != parameter_ids or bca["parameter_ids"] != parameter_ids:
        raise ValueError("interval parameter identities differ")

    reasons: list[str] = []
    endpoint_checks = 0
    arithmetic_checks = 0
    successes = base["successful_refits"]
    failures = base["failed_refits"]
    requested = int(base["requested_replicates"])
    minimum = int(base["minimum_usable_replicates"])
    expected_base_available = len(successes) >= minimum
    observed_base_available = base["inference"].get("status") == "available"
    if expected_base_available != observed_base_available:
        reasons.append("base_inference_status_differs_from_ledger")
    if len(successes) + len(failures) != requested:
        reasons.append("base_ledger_count_is_incomplete")

    if expected_base_available:
        by_parameter = {row["parameter_id"]: row for row in base["intervals"]}
        if set(by_parameter) != set(parameter_ids):
            reasons.append("percentile_interval_identity_differs")
        else:
            for column, parameter_id in enumerate(parameter_ids):
                estimates = [float(row["parameter_estimates"][column]) for row in successes]
                observed = by_parameter[parameter_id]
                point = float(original["refit"]["free_parameters"][column]["estimate"])
                expected_se = float(np.std(np.asarray(estimates, dtype=np.float64), ddof=1))
                expected_lower = type7(estimates, 0.025)
                expected_upper = type7(estimates, 0.975)
                arithmetic_checks += _compare_percentile_mean_bias(
                    observed.get("bootstrap_mean"),
                    observed.get("bias"),
                    estimates,
                    point,
                    f"percentile:{parameter_id}",
                    reasons,
                )
                for field, expected in (("original", point), ("standard_error", expected_se)):
                    arithmetic_checks += _compare_scalar(
                        observed.get(field),
                        expected,
                        f"percentile:{parameter_id}:{field}",
                        reasons,
                    )
                endpoint_checks += _compare_endpoint(
                    observed["percentile_lower"], expected_lower, f"percentile:{parameter_id}:lower", reasons
                )
                endpoint_checks += _compare_endpoint(
                    observed["percentile_upper"], expected_upper, f"percentile:{parameter_id}:upper", reasons
                )
    elif base["intervals"]:
        reasons.append("unavailable_percentile_inference_emitted_intervals")

    point_se_outcome = studentized["point_standard_errors"]["outcome"]
    point_se_available = point_se_outcome.get("status") == "available"
    receipts = studentized["refit_standard_errors"]
    usable_receipts = [row for row in receipts if row["outcome"].get("status") == "available"]
    expected_studentized_available = point_se_available and len(usable_receipts) >= minimum
    observed_studentized_available = studentized["inference"].get("status") == "available"
    if len(receipts) != len(successes) or [row["replicate_index"] for row in receipts] != [row["replicate_index"] for row in successes]:
        reasons.append("studentized_receipt_partition_or_order_differs")
    if int(studentized["studentized_usable_replicates"]) != len(usable_receipts):
        reasons.append("studentized_usable_count_differs")
    if expected_studentized_available != observed_studentized_available:
        reasons.append("studentized_inference_status_differs_from_ledger")
    student_intervals = {row["parameter_id"]: row["outcome"] for row in studentized["intervals"]}
    if set(student_intervals) != set(parameter_ids):
        reasons.append("studentized_interval_identity_differs")
    elif expected_studentized_available:
        point_estimates = [float(row["estimate"]) for row in original["refit"]["free_parameters"]]
        point_ses = [float(row["standard_error"]) for row in point_se_outcome["parameters"]]
        for column, parameter_id in enumerate(parameter_ids):
            pivots = []
            for witness, receipt in zip(successes, receipts, strict=True):
                if receipt["outcome"].get("status") != "available":
                    continue
                pivots.append(
                    (float(witness["parameter_estimates"][column]) - point_estimates[column])
                    / float(receipt["outcome"]["standard_errors"][column])
                )
            lower_pivot = type7(pivots, 0.025)
            upper_pivot = type7(pivots, 0.975)
            expected_lower = point_estimates[column] - upper_pivot * point_ses[column]
            expected_upper = point_estimates[column] - lower_pivot * point_ses[column]
            observed = student_intervals[parameter_id]
            if observed.get("status") != "available":
                reasons.append(f"studentized:{parameter_id}:unexpected_unavailable")
                continue
            for field, expected in (
                ("point_estimate", point_estimates[column]),
                ("point_standard_error", point_ses[column]),
                ("lower_pivot_quantile", lower_pivot),
                ("upper_pivot_quantile", upper_pivot),
            ):
                arithmetic_checks += _compare_scalar(
                    observed.get(field),
                    expected,
                    f"studentized:{parameter_id}:{field}",
                    reasons,
                )
            endpoint_checks += _compare_endpoint(
                observed["interval_lower"], expected_lower, f"studentized:{parameter_id}:lower", reasons
            )
            endpoint_checks += _compare_endpoint(
                observed["interval_upper"], expected_upper, f"studentized:{parameter_id}:upper", reasons
            )
    elif any(row.get("status") == "available" for row in student_intervals.values()):
        reasons.append("unavailable_studentized_inference_emitted_available_interval")

    delete_successes = bca["successful_delete_one_refits"]
    delete_failures = bca["failed_delete_one_refits"]
    delete_count = int(bca["delete_one_case_count"])
    expected_bca_global = expected_base_available and not delete_failures and len(delete_successes) == delete_count
    observed_bca_global = bca["inference"].get("status") == "available"
    if len(delete_successes) + len(delete_failures) != delete_count:
        reasons.append("delete_one_ledger_count_is_incomplete")
    if expected_bca_global != observed_bca_global:
        reasons.append("bca_inference_status_differs_from_ledgers")
    bca_intervals = {row["parameter_id"]: row["outcome"] for row in bca["intervals"]}
    if set(bca_intervals) != set(parameter_ids):
        reasons.append("bca_interval_identity_differs")
    elif expected_bca_global:
        for column, parameter_id in enumerate(parameter_ids):
            point = float(original["refit"]["free_parameters"][column]["estimate"])
            bootstrap_values = [float(row["parameter_estimates"][column]) for row in successes]
            jackknife_values = [float(row["parameter_estimates"][column]) for row in delete_successes]
            expected = _bca(bootstrap_values, point, jackknife_values)
            observed = bca_intervals[parameter_id]
            if expected["status"] != observed.get("status"):
                reasons.append(f"bca:{parameter_id}:status_differs")
                continue
            if expected["status"] == "available":
                for field in (
                    "point_estimate",
                    "bias_correction",
                    "acceleration",
                    "adjusted_lower_probability",
                    "adjusted_upper_probability",
                ):
                    arithmetic_checks += _compare_scalar(
                        observed.get(field),
                        expected[field],
                        f"bca:{parameter_id}:{field}",
                        reasons,
                    )
                endpoint_checks += _compare_endpoint(
                    observed["interval_lower"], expected["interval_lower"], f"bca:{parameter_id}:lower", reasons
                )
                endpoint_checks += _compare_endpoint(
                    observed["interval_upper"], expected["interval_upper"], f"bca:{parameter_id}:upper", reasons
                )
    elif any(row.get("status") == "available" for row in bca_intervals.values()):
        reasons.append("unavailable_bca_inference_emitted_available_interval")

    return {
        "schema_version": 1,
        "kind": "cbsem_exact_cfa_interval_coverage_oracle_receipt_v1",
        "status": "accepted" if not reasons else "rejected",
        "parameter_count": len(parameter_ids),
        "requested_replicates": requested,
        "point_successes": len(successes),
        "point_failures": len(failures),
        "studentized_usable": len(usable_receipts),
        "delete_one_successes": len(delete_successes),
        "delete_one_failures": len(delete_failures),
        "base_inference_available": expected_base_available,
        "studentized_inference_available": expected_studentized_available,
        "bca_inference_available": expected_bca_global,
        "endpoint_checks": endpoint_checks,
        "arithmetic_checks": arithmetic_checks,
        "endpoint_relative_tolerance": ENDPOINT_RELATIVE_TOLERANCE,
        "percentile_mean_validation_method": PERCENTILE_MEAN_VALIDATION_METHOD,
        "percentile_bias_binding_method": PERCENTILE_BIAS_BINDING_METHOD,
        "bca_acceleration_method": BCA_ACCELERATION_METHOD,
        "bca_adjusted_probability_method": BCA_ADJUSTED_PROBABILITY_METHOD,
        "reasons": sorted(set(reasons)),
    }


def _bca(bootstrap: Sequence[float], point: float, jackknife: Sequence[float]) -> dict[str, Any]:
    values = np.asarray(bootstrap, dtype=np.float64)
    deletes = np.asarray(jackknife, dtype=np.float64)
    less = int(np.count_nonzero(values < point))
    ties = int(np.count_nonzero(values == point))
    probability = (less + 0.5 * ties) / values.size
    if not 0.0 < probability < 1.0:
        return {"status": "unavailable", "reason": "bias_correction_probability_at_boundary"}
    z0 = float(norm.ppf(probability))
    jackknife_mean = math.fsum(float(value) for value in deletes) / deletes.size
    centered = [jackknife_mean - float(value) for value in deletes]
    sum_squares = math.fsum(value * value for value in centered)
    if sum_squares == 0.0:
        return {"status": "unavailable", "reason": "degenerate_jackknife_acceleration"}
    sum_cubes = math.fsum(value * value * value for value in centered)
    acceleration = sum_cubes / (6.0 * sum_squares**1.5)
    adjusted = []
    for nominal in (0.025, 0.975):
        z = float(norm.ppf(nominal))
        denominator = 1.0 - acceleration * (z0 + z)
        if not math.isfinite(denominator) or abs(denominator) <= 1.0e-12:
            return {"status": "unavailable", "reason": "singular_acceleration_adjustment"}
        probability = float(norm.cdf(z0 + (z0 + z) / denominator))
        if not math.isfinite(probability) or not 0.0 <= probability <= 1.0:
            return {"status": "unavailable", "reason": "invalid_adjusted_probability"}
        adjusted.append(probability)
    if adjusted[0] > adjusted[1]:
        return {"status": "unavailable", "reason": "adjusted_probability_order_invalid"}
    return {
        "status": "available",
        "point_estimate": point,
        "bias_correction": z0,
        "acceleration": acceleration,
        "adjusted_lower_probability": adjusted[0],
        "adjusted_upper_probability": adjusted[1],
        "interval_lower": type7(values, adjusted[0]),
        "interval_upper": type7(values, adjusted[1]),
    }


def _compare_endpoint(observed: Any, expected: float, label: str, reasons: list[str]) -> int:
    return _compare_scalar(observed, expected, f"endpoint:{label}", reasons)


def _binary64_bits(value: float) -> bytes:
    return struct.pack(">d", value)


def _next_up_finite(value: float, label: str) -> float:
    if not math.isfinite(value) or value < 0.0:
        raise ValueError(f"{label} is not a finite nonnegative binary64 value")
    outward = math.nextafter(value, math.inf)
    if not math.isfinite(outward):
        raise ValueError(f"{label} outward rounding overflowed")
    return outward


def _percentile_mean_bias_reference(
    values: Sequence[float], observed_mean: float, observed_bias: float, point: float
) -> tuple[float, float, float, float]:
    """Return fsum centers and rigorous forward-error envelopes.

    For B finite binary64 inputs accumulated left-to-right without overflow,
    the standard model gives |fl(sum)-sum| <= gamma_(B-1) * sum(abs(x)),
    where gamma_k = k*u/(1-k*u), u=2^-53. Every positive bound operation is
    rounded toward +infinity with nextafter. One ulp for each serialized mean
    center accounts conservatively for the final division and fsum-reference
    rounding. The bias adds one ulp for each binary64 bias center and is
    separately required to be the exact binary64 subtraction of the
    serialized mean and point.
    """

    if (
        isinstance(observed_mean, bool)
        or not isinstance(observed_mean, (int, float))
        or isinstance(observed_bias, bool)
        or not isinstance(observed_bias, (int, float))
        or isinstance(point, bool)
        or not isinstance(point, (int, float))
    ):
        raise ValueError("mean, bias, and point must be binary64-compatible scalars")
    observed_mean = float(observed_mean)
    observed_bias = float(observed_bias)
    point = float(point)
    count = len(values)
    if count <= 0 or not all(math.isfinite(float(value)) for value in values):
        raise ValueError("mean ledger must be a nonempty finite ordered vector")
    if not all(math.isfinite(value) for value in (observed_mean, observed_bias, point)):
        raise ValueError("serialized mean arithmetic is nonfinite")
    factor = (count - 1) * IEEE_BINARY64_UNIT_ROUNDOFF
    if not math.isfinite(factor) or factor >= 1.0:
        raise ValueError("mean ledger is outside the gamma theorem domain")

    # Fail closed if the engine's declared left-to-right operation would have
    # overflowed at an intermediate step. The theorem does not cover overflow.
    running = 0.0
    for value in values:
        running += float(value)
        if not math.isfinite(running):
            raise ValueError("left-to-right mean accumulation overflowed")
    if not math.isfinite(running / count):
        raise ValueError("mean division overflowed")

    try:
        reference_mean = math.fsum(float(value) for value in values) / count
        absolute_sum = math.fsum(abs(float(value)) for value in values)
    except OverflowError as error:
        raise ValueError("fsum reference arithmetic overflowed") from error
    if not math.isfinite(reference_mean) or not math.isfinite(absolute_sum):
        raise ValueError("fsum reference arithmetic is nonfinite")

    gamma = _next_up_finite(factor / (1.0 - factor), "gamma")
    absolute_sum = _next_up_finite(absolute_sum, "absolute sum")
    accumulation_bound = _next_up_finite(gamma * absolute_sum, "sum error product")
    accumulation_bound = _next_up_finite(
        accumulation_bound / count, "mean accumulation bound"
    )
    mean_bound = _next_up_finite(
        accumulation_bound + math.ulp(observed_mean), "observed mean bound"
    )
    mean_bound = _next_up_finite(
        mean_bound + math.ulp(reference_mean), "reference mean bound"
    )
    reference_bias = reference_mean - point
    if not math.isfinite(reference_bias):
        raise ValueError("reference bias arithmetic is nonfinite")
    bias_bound = _next_up_finite(
        mean_bound + math.ulp(observed_bias), "observed bias bound"
    )
    bias_bound = _next_up_finite(
        bias_bound + math.ulp(reference_bias), "reference bias bound"
    )
    return reference_mean, reference_bias, mean_bound, bias_bound


def _compare_percentile_mean_bias(
    observed_mean: Any,
    observed_bias: Any,
    values: Sequence[float],
    point: float,
    label: str,
    reasons: list[str],
) -> int:
    try:
        reference_mean, reference_bias, mean_bound, bias_bound = (
            _percentile_mean_bias_reference(values, observed_mean, observed_bias, point)
        )
    except (OverflowError, ValueError) as error:
        reasons.append(f"arithmetic_precondition_failed:{label}:{type(error).__name__}")
        return 2
    observed_mean = float(observed_mean)
    observed_bias = float(observed_bias)
    if abs(observed_mean - reference_mean) > mean_bound:
        reasons.append(f"arithmetic_mismatch:{label}:bootstrap_mean")
    recomputed_bias = observed_mean - float(point)
    if not math.isfinite(recomputed_bias) or _binary64_bits(observed_bias) != _binary64_bits(
        recomputed_bias
    ):
        reasons.append(f"arithmetic_mismatch:{label}:bias_binary64_binding")
    if abs(observed_bias - reference_bias) > bias_bound:
        reasons.append(f"arithmetic_mismatch:{label}:bias")
    return 2


def _compare_scalar(observed: Any, expected: float, label: str, reasons: list[str]) -> int:
    if (
        isinstance(observed, bool)
        or not isinstance(observed, (int, float))
        or not math.isfinite(float(observed))
        or not math.isclose(
            float(observed),
            expected,
            rel_tol=ENDPOINT_RELATIVE_TOLERANCE,
            abs_tol=ENDPOINT_ABSOLUTE_TOLERANCE,
        )
    ):
        reasons.append(f"arithmetic_mismatch:{label}")
    return 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine-result", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    document = json.loads(args.engine_result.read_text(encoding="utf-8"))
    receipt = audit_engine_case(document)
    payload = json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    if args.output:
        if args.output.exists():
            parser.error(f"append-only output already exists: {args.output}")
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(payload, encoding="utf-8")
    else:
        print(payload, end="")
    return 0 if receipt["status"] == "accepted" else 2


if __name__ == "__main__":
    raise SystemExit(main())
