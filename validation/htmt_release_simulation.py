#!/usr/bin/env python3
"""Pre-registered HTMT recovery/simulation gate.

The default mode is a lightweight source audit: it cross-checks two transparent
implementations and validates the frozen Monte Carlo design.  It explicitly
does not turn those checks into qualification evidence.  A later high-cost run
may be supplied as ``--qualification-report`` and is accepted only when every
pre-registered scenario and Monte Carlo accounting rule is present.
"""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from statistics import NormalDist
from typing import Any

from htmt_plus_v1_factory_common import (
    QUALIFICATION_SPEC_PATH,
    ROOT,
    canonical_sha256,
    require_exact_case_ids,
    strict_load_json,
    write_identity_report,
)


SOURCE = "validation/htmt_release_simulation.py"
STANDARD_POINT = ROOT / "validation" / "results" / "htmt_reference.json"
SCIPY_REFERENCE = ROOT / "validation" / "results" / "htmt_scipy_reference.json"
STANDARD_BOOTSTRAP = (
    ROOT / "validation" / "results" / "htmt_bootstrap_inference_reference.json"
)
TOLERANCE = 1e-10


def _compare_cell(left: dict[str, Any], right: dict[str, Any]) -> tuple[bool, float]:
    if left.get("status") != right.get("status") or left.get("reason") != right.get(
        "reason"
    ):
        return False, math.inf
    left_value = left.get("value")
    right_value = right.get("value")
    if left_value is None or right_value is None:
        return left_value is right_value, 0.0 if left_value is right_value else math.inf
    error = abs(float(left_value) - float(right_value))
    return math.isfinite(error) and error <= TOLERANCE, error


def compare_point_references() -> dict[str, Any]:
    standard = strict_load_json(STANDARD_POINT)
    scipy = strict_load_json(SCIPY_REFERENCE)["point"]
    baseline = standard["fixtures"][0]
    checks: list[bool] = []
    errors: list[float] = []
    for key in ("htmt_original", "htmt_plus"):
        for standard_row, scipy_row in zip(baseline[key], scipy[key]):
            for standard_cell, scipy_cell in zip(standard_row, scipy_row):
                passed, error = _compare_cell(standard_cell, scipy_cell)
                checks.append(passed)
                errors.append(error)
    expected_cells = 2 * len(scipy["constructs"]) ** 2
    return {
        "passed": (
            standard["method_versions"]["htmt_plus"] == "ringle_et_al_htmt_plus_v1"
            and standard["method_versions"]["htmt_original"] == "henseler_et_al_htmt_v1"
            and baseline["constructs"] == scipy["constructs"]
            and len(checks) == expected_cells
            and all(checks)
        ),
        "compared_cells": len(checks),
        "expected_cells": expected_cells,
        "maximum_absolute_error": max(errors, default=math.inf),
        "construct_order_matches": baseline["constructs"] == scipy["constructs"],
    }


def compare_bootstrap_references() -> dict[str, Any]:
    standard = strict_load_json(STANDARD_BOOTSTRAP)
    scipy = strict_load_json(SCIPY_REFERENCE)["bootstrap"]
    standard_by_id = {row["id"]: row for row in standard["scenarios"]}
    scipy_by_id = {row["id"]: row for row in scipy["scenarios"]}
    fields = (
        "original",
        "bootstrap_mean",
        "bias",
        "standard_error",
        "bias_correction",
        "lower_probability",
        "upper_probability",
        "lower",
        "upper",
    )
    maximum_error = 0.0
    decisions_equal = True
    digests_equal = True
    for scenario_id in set(standard_by_id) & set(scipy_by_id):
        left = standard_by_id[scenario_id]
        right = scipy_by_id[scenario_id]
        for field in fields:
            maximum_error = max(
                maximum_error,
                abs(float(left["expected"][field]) - float(right["expected"][field])),
            )
        decisions_equal = decisions_equal and (
            left["expected"]["upper_bound_below_critical_value"]
            == right["expected"]["upper_bound_below_critical_value"]
        )
        digests_equal = digests_equal and (
            left["usable_replicate_indices_sha256"]
            == right["usable_replicate_indices_sha256"]
        )
    metadata_equal = (
        standard["method"] == scipy["interval_method"]
        and standard["test_type"] == scipy["test_type"]
        and standard["significance_level"] == scipy["significance_level"]
        and standard["equivalent_two_sided_confidence_level"]
        == scipy["equivalent_two_sided_confidence_level"]
        and standard["critical_value"] == scipy["critical_value"]
    )
    return {
        "passed": (
            set(standard_by_id) == set(scipy_by_id)
            and bool(standard_by_id)
            and metadata_equal
            and decisions_equal
            and digests_equal
            and maximum_error <= TOLERANCE
        ),
        "scenario_ids": sorted(standard_by_id),
        "exact_scenario_membership": set(standard_by_id) == set(scipy_by_id),
        "metadata_equal": metadata_equal,
        "decisions_equal": decisions_equal,
        "replicate_index_digests_equal": digests_equal,
        "maximum_absolute_error": maximum_error,
    }


def preregistration_check() -> dict[str, Any]:
    spec = strict_load_json(QUALIFICATION_SPEC_PATH)
    scenario = spec["scenario_contract"]
    policy = scenario["monte_carlo_policy"]
    confidence = float(policy["confidence_level"])
    half_width = float(policy["maximum_half_width"])
    z = NormalDist().inv_cdf(0.5 + confidence / 2.0)
    worst_case_replications = math.ceil((z * z * 0.25) / (half_width * half_width))
    axes = {row["id"]: row for row in scenario["axes"]}
    profiles = {row["id"]: row for row in scenario["complexity_profiles"]}
    combinations = {row["id"]: row for row in scenario["mandatory_combinations"]}
    expected_axes = {
        "model_topology",
        "measurement_model",
        "data_distribution",
        "missingness",
        "input_type",
        "workload",
        "workers",
    }
    expected_profiles = {
        "micro_exact",
        "applied",
        "large",
        "maximum_axis",
        "compound_stress",
    }
    return {
        "passed": (
            set(axes) == expected_axes
            and set(profiles) == expected_profiles
            and any(row["coverage"] == "pairwise" for row in combinations.values())
            and any(row["coverage"] == "compound" for row in combinations.values())
            and confidence == 0.95
            and half_width <= 0.01
            and policy["failed_fits_in_denominator"] is True
            and worst_case_replications >= 9604
        ),
        "axes": sorted(axes),
        "profiles": sorted(profiles),
        "mandatory_combination_ids": sorted(combinations),
        "confidence_level": confidence,
        "maximum_half_width": half_width,
        "minimum_worst_case_replications": worst_case_replications,
        "failed_fits_in_denominator": policy["failed_fits_in_denominator"],
    }


def validate_qualification_report(path: Path) -> dict[str, Any]:
    document = strict_load_json(path)
    spec = strict_load_json(QUALIFICATION_SPEC_PATH)
    required_ids = [
        row["id"] for row in spec["scenario_contract"]["mandatory_combinations"]
    ]
    cases = require_exact_case_ids(document, required_ids)
    identity = {
        "passed": (
            document.get("qualification_id") == spec["identity"]["qualification_id"]
            and document.get("method_version") == spec["identity"]["method_version"]
            and document.get("scenario_set_sha256")
            == canonical_sha256(spec["scenario_contract"])
        )
    }
    monte_carlo = document.get("monte_carlo", {})
    accounting = {
        "passed": (
            isinstance(monte_carlo, dict)
            and int(monte_carlo.get("replications_per_estimated_proportion", 0)) >= 9604
            and float(monte_carlo.get("maximum_observed_half_width", math.inf)) <= 0.01
            and monte_carlo.get("failed_fits_in_denominator") is True
            and int(monte_carlo.get("unaccounted_fits", -1)) == 0
        ),
        "reported": monte_carlo,
    }
    return {
        "passed": identity["passed"] and cases["passed"] and accounting["passed"],
        "identity": identity,
        "cases": cases,
        "monte_carlo": accounting,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--qualification-report", type=Path)
    parser.add_argument("--admit", action="store_true")
    args = parser.parse_args()
    point = compare_point_references()
    bootstrap = compare_bootstrap_references()
    preregistration = preregistration_check()
    checks: dict[str, Any] = {
        "point_oracle_agreement": point,
        "bootstrap_oracle_agreement": bootstrap,
        "preregistration": preregistration,
    }
    blockers = [
        "current_source_bound_candidate_execution_not_supplied",
        "full_monte_carlo_recovery_and_coverage_run_not_supplied",
        "maximum_axis_and_compound_stress_not_executed",
    ]
    qualification_evidence = False
    if args.qualification_report:
        qualification = validate_qualification_report(args.qualification_report)
        checks["qualification_execution"] = qualification
        if qualification["passed"]:
            blockers = []
            qualification_evidence = args.admit
        else:
            blockers.append("qualification_report_failed_contract")
    passed = point["passed"] and bootstrap["passed"] and preregistration["passed"]
    if args.qualification_report:
        passed = passed and checks["qualification_execution"]["passed"]
    report = write_identity_report(
        "simulation_report",
        stage="generative",
        passed=passed,
        checks=checks,
        blockers=blockers,
        extras=[
            SOURCE,
            "validation/htmt_reference.py",
            "validation/htmt_scipy_reference.py",
            "validation/htmt_bootstrap_inference_reference.py",
            "validation/results/htmt_reference.json",
            "validation/results/htmt_scipy_reference.json",
            "validation/results/htmt_bootstrap_inference_reference.json",
        ],
        qualification_evidence=qualification_evidence,
    )
    print(
        json.dumps(
            {
                "passed": passed,
                "qualification_evidence": qualification_evidence,
                "blockers": blockers,
                "report": report.relative_to(ROOT).as_posix(),
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0 if passed and (not args.admit or qualification_evidence) else 1


if __name__ == "__main__":
    raise SystemExit(main())
