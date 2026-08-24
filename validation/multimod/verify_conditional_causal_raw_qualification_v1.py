#!/usr/bin/env python3
"""Independently verify the MultiMod conditional/causal raw-runner receipts.

The Rust examples are evidence producers, not self-certifying tests.  This
stdlib-only verifier recomputes the numerical identities that are feasible
outside the product implementation and checks the exact acceptance matrix.
It emits one deterministic machine-readable receipt for dependency gates.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import sys
from pathlib import Path
from statistics import NormalDist
from typing import Any, Callable, Iterable

import conditional_causal_shards_v1 as shard_contract


CONDITIONAL_CELLS = {
    "conditional.multi_two_way_percentile.v2::explicit_path_target_math",
    "conditional.multi_two_way_percentile.v2::shared_ledger_percentile_type7",
    "conditional.multi_two_way_percentile.v2::both_stage_multiple_long_path",
    "conditional.multi_two_way_percentile.v2::all_predeclared_alternatives",
    "conditional.multi_two_way_bca.v2::explicit_path_target_math",
    "conditional.multi_two_way_bca.v2::complete_delete_one_bca",
    "conditional.multi_two_way_bca.v2::all_predeclared_alternatives",
    "conditional.multi_two_way_bca.v2::incomplete_jackknife_fail_closed",
    "conditional.studentized.v2::nested_studentized",
    "conditional.studentized.v2::no_percentile_fallback",
    "conditional.studentized.v2::all_predeclared_alternatives",
    "conditional.studentized.v2::outer_inner_budget_limits",
    "conditional.bounded_three_way_percentile.v2::complete_lower_order_closure",
    "conditional.bounded_three_way_percentile.v2::derivatives_and_cross_derivatives",
    "conditional.bounded_three_way_percentile.v2::shared_ledger_percentile_type7",
    "conditional.bounded_three_way_percentile.v2::all_predeclared_alternatives",
    "conditional.multiple_hoc_percentile.v2::hoc_dependency_before_products",
    "conditional.multiple_hoc_percentile.v2::disjoint_nonnested_single_approach",
    "conditional.multiple_hoc_percentile.v2::shared_ledger_percentile_type7_two_sided",
    "conditional.grouped_percentile.v2::group_stratified_shared_ledger",
    "conditional.grouped_percentile.v2::percentile_type7_two_sided",
    "conditional.grouped_percentile.v2::two_to_twenty_group_bounds",
    "conditional.case_weighted_percentile.v2::positive_normalized_case_weights",
    "conditional.case_weighted_percentile.v2::row_weight_resampling",
    "conditional.case_weighted_percentile.v2::percentile_type7_two_sided",
    "conditional.case_weighted_percentile.v2::kish_ess_and_ratio_guards",
    "conditional.frequency_weighted_percentile.v2::count_space_point_equivalence",
    "conditional.frequency_weighted_percentile.v2::multinomial_count_bootstrap_equivalence",
    "conditional.frequency_weighted_percentile.v2::percentile_type7_two_sided",
    "conditional.frequency_weighted_percentile.v2::exact_integer_total_guard",
}

CAUSAL_CELLS = {
    "interventional.observed_gcomp.v1::observed_equation_point_fit",
    "interventional.observed_gcomp.v1::parametric_g_computation",
    "interventional.observed_gcomp.v1::known_target_simulation",
    "interventional.observed_gcomp.v1::causal_wording_guard",
    "interventional.observed_gcomp.v1::positivity_diagnostics",
    "interventional.observed_gcomp.v1::identification_failure_guards",
}

HEX = set("0123456789abcdef")
ALTERNATIVES = {"two_sided", "less", "greater"}
SHARD_IDS = {
    family: [row["shard_id"] for row in shard_contract.expected_specs(family)]
    for family in ("conditional", "causal")
}


def canonical_json(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def is_sha256(value: Any) -> bool:
    return isinstance(value, str) and len(value) == 64 and set(value.lower()) <= HEX


def is_lower_hex(value: Any, length: int) -> bool:
    return (
        isinstance(value, str)
        and len(value) == length
        and set(value) <= HEX
    )


def close(left: float, right: float, tolerance: float = 5.0e-8) -> bool:
    return math.isfinite(left) and math.isfinite(right) and abs(left - right) <= tolerance * max(
        1.0, abs(left), abs(right)
    )


def type7(values: Iterable[float], probability: float) -> float:
    ordered = sorted(float(value) for value in values)
    if not ordered or not 0.0 <= probability <= 1.0 or not all(map(math.isfinite, ordered)):
        raise ValueError("invalid Type-7 quantile inputs")
    if len(ordered) == 1:
        return ordered[0]
    h = (len(ordered) - 1) * probability
    lower = math.floor(h)
    upper = math.ceil(h)
    return ordered[lower] + (h - lower) * (ordered[upper] - ordered[lower])


def tail_probabilities(confidence: float, alternative: str) -> tuple[float | None, float | None]:
    alpha = 1.0 - confidence
    if alternative == "two_sided":
        return alpha / 2.0, 1.0 - alpha / 2.0
    if alternative == "less":
        return None, confidence
    if alternative == "greater":
        return 1.0 - confidence, None
    raise ValueError(f"unknown alternative {alternative}")


class Audit:
    def __init__(self) -> None:
        self.checks: list[dict[str, Any]] = []

    def add(self, check_id: str, passed: bool, detail: Any) -> None:
        self.checks.append(
            {
                "check_id": check_id,
                "status": "passed" if passed else "failed",
                "detail": detail,
            }
        )

    def guard(self, check_id: str, operation: Callable[[], tuple[bool, Any]]) -> None:
        try:
            passed, detail = operation()
            self.add(check_id, bool(passed), detail)
        except Exception as error:  # receipt validation must fail closed
            self.add(check_id, False, f"{type(error).__name__}: {error}")

    @property
    def passed(self) -> bool:
        return all(check["status"] == "passed" for check in self.checks)


def collect_cell_ids(value: Any) -> set[str]:
    cells: set[str] = set()
    if isinstance(value, dict):
        declared = value.get("cell_ids")
        if isinstance(declared, list):
            cells.update(cell for cell in declared if isinstance(cell, str))
        for child in value.values():
            cells.update(collect_cell_ids(child))
    elif isinstance(value, list):
        for child in value:
            cells.update(collect_cell_ids(child))
    return cells


def verify_shard_execution_receipt(
    report: dict[str, Any],
    family: str,
    required: bool,
    plan_path: Path | None = None,
    shard_dir: Path | None = None,
    producer_executable: Path | None = None,
    expected_source_commit: str | None = None,
) -> tuple[bool, Any]:
    receipt = report.get("shard_execution_receipt")
    if receipt is None:
        return (not required), {
            "required": required,
            "status": "missing" if required else "legacy_monolithic_not_required",
        }
    expected_ids = SHARD_IDS[family]
    rows = receipt.get("shards") if isinstance(receipt, dict) else None
    actual_ids = (
        [row.get("shard_id") for row in rows if isinstance(row, dict)]
        if isinstance(rows, list)
        else []
    )
    row_shape_is_exact = isinstance(rows, list) and all(
        isinstance(row, dict)
        and set(row) == {"shard_id", "receipt_sha256", "result_sha256"}
        and is_lower_hex(row.get("receipt_sha256"), 64)
        and is_lower_hex(row.get("result_sha256"), 64)
        for row in rows
    )
    valid = (
        isinstance(receipt, dict)
        and receipt.get("schema_version") == 1
        and receipt.get("family") == family
        and is_lower_hex(receipt.get("plan_sha256"), 64)
        and is_lower_hex(receipt.get("producer_executable_sha256"), 64)
        and is_lower_hex(receipt.get("source_commit"), 40)
        and row_shape_is_exact
        and actual_ids == expected_ids
        and len(set(actual_ids)) == len(actual_ids)
    )
    binding_status = "not_required"
    binding_error = None
    if required:
        missing_inputs = [
            name
            for name, value in (
                ("plan_path", plan_path),
                ("shard_dir", shard_dir),
                ("producer_executable", producer_executable),
                ("expected_source_commit", expected_source_commit),
            )
            if value is None
        ]
        if missing_inputs:
            valid = False
            binding_status = "missing_inputs"
            binding_error = f"missing portable receipt inputs: {', '.join(missing_inputs)}"
        elif valid:
            try:
                assert plan_path is not None
                assert shard_dir is not None
                assert producer_executable is not None
                assert expected_source_commit is not None
                executable_sha256 = shard_contract.sha256_file(producer_executable)
                expected_report = shard_contract.build_aggregate_report(
                    family,
                    plan_path,
                    shard_dir,
                    executable_sha256,
                    expected_source_commit,
                )
                valid = report == expected_report
                binding_status = "exact" if valid else "aggregate_mismatch"
                if not valid:
                    binding_error = (
                        "report does not exactly match the receipt-verified deterministic aggregate"
                    )
            except (
                OSError,
                KeyError,
                TypeError,
                ValueError,
                json.JSONDecodeError,
                shard_contract.ContractError,
            ) as error:
                valid = False
                binding_status = "recomputation_failed"
                binding_error = f"{type(error).__name__}: {error}"
    return valid, {
        "required": required,
        "expected_shard_ids": expected_ids,
        "actual_shard_ids": actual_ids,
        "plan_sha256": receipt.get("plan_sha256") if isinstance(receipt, dict) else None,
        "producer_executable_sha256": (
            receipt.get("producer_executable_sha256")
            if isinstance(receipt, dict)
            else None
        ),
        "source_commit": receipt.get("source_commit") if isinstance(receipt, dict) else None,
        "binding_status": binding_status,
        "binding_error": binding_error,
    }


def conditional_leaf_cases(report: dict[str, Any]) -> list[dict[str, Any]]:
    leaves: list[dict[str, Any]] = []
    for case in report.get("cases", []):
        if "result" in case:
            leaves.append(case)
        elif case.get("case_id") == "frequency_weighted:count_space_and_physical_expansion":
            leaves.extend([case["compact_count_space"], case["physical_expansion_reference"]])
    return leaves


def interval_of_first_target(case: dict[str, Any]) -> tuple[float, str, dict[str, Any]]:
    target = case["result"]["targets"][0]
    interval = target["interval"]
    return float(target["estimate"]), interval["alternative"], interval


def verify_compiler_and_ledger(case: dict[str, Any]) -> tuple[bool, Any]:
    receipt = case.get("compiler_receipt", {})
    digests = [
        receipt.get("recipe_analytical_sha256"),
        receipt.get("config_sha256"),
        receipt.get("model_scientific_sha256"),
        receipt.get("dataset_fingerprint"),
        receipt.get("plan_sha256"),
        receipt.get("analytical_identity_sha256"),
    ]
    provenance = case.get("result", {}).get("provenance", {})
    identity_ok = (
        case.get("deterministic_recompile_equal") is True
        and all(is_sha256(value) for value in digests)
        and isinstance(case.get("compiled_plan"), dict)
        and receipt.get("recipe_id") == case.get("recipe_id")
        and receipt.get("model_id") == case.get("model_id")
        and provenance.get("recipe_analytical_sha256") == receipt.get("recipe_analytical_sha256")
        and provenance.get("model_scientific_sha256") == receipt.get("model_scientific_sha256")
        and provenance.get("dataset_fingerprint") == case.get("dataset_fingerprint")
    )
    return identity_ok, {
        "case_id": case.get("case_id"),
        "deterministic_recompile_equal": case.get("deterministic_recompile_equal"),
        "receipt_digests_valid": all(is_sha256(value) for value in digests),
    }


def ledger_ok(ledger: dict[str, Any], target_count: int) -> bool:
    widths = {int(value) for value in ledger.get("successful_target_vector_widths", [])}
    vectors = ledger.get("successful_target_vectors", [])
    return (
        ledger.get("complete") is True
        and isinstance(ledger.get("requested"), int)
        and ledger.get("usable", -1) >= ledger.get("minimum_required", 10**18)
        and ledger.get("record_count") == ledger.get("requested")
        and ledger.get("unique_record_identity_count", ledger.get("record_count"))
        == ledger.get("record_count")
        and is_sha256(ledger.get("execution_identity_sha256"))
        and is_sha256(ledger.get("ledger_sha256"))
        and (not widths or widths == {target_count})
        and ledger.get("expected_target_vector_width") == target_count
        and len(ledger.get("first_target_draws", [])) == ledger.get("usable")
        and len(vectors) == ledger.get("usable")
        and len({int(vector["replicate_index"]) for vector in vectors}) == len(vectors)
        and all(len(vector.get("target_values", [])) == target_count for vector in vectors)
    )


def all_case_ledgers(case: dict[str, Any]) -> list[dict[str, Any]]:
    evidence = case["evidence"]
    kind = evidence["kind"]
    if kind == "percentile_case":
        return [evidence["bootstrap"]]
    if kind == "bca_case":
        return [evidence["bootstrap"], evidence["delete_one"]]
    if kind == "studentized_case":
        return [evidence["observed_inner"]]
    if kind == "grouped_stratified":
        return [group["ledger"] for group in evidence["groups"]]
    if kind == "frequency_count_space":
        return [evidence["bootstrap"]]
    raise ValueError(f"unknown evidence kind {kind}")


def conditional_generic(case: dict[str, Any]) -> tuple[bool, Any]:
    target_count = len(case["result"]["targets"])
    ledgers = all_case_ledgers(case)
    basic = verify_compiler_and_ledger(case)[0]
    usable = {int(target["usable_replicates"]) for target in case["result"]["targets"]}
    intervals = [target.get("interval") for target in case["result"]["targets"]]
    return (
        basic
        and target_count > 0
        and all(ledger_ok(ledger, int(ledger["expected_target_vector_width"])) for ledger in ledgers)
        and case.get("one_result_ledger_for_all_targets") is True
        and len(usable) == 1
        and all(isinstance(interval, dict) for interval in intervals),
        {
            "case_id": case["case_id"],
            "targets": target_count,
            "usable_counts": sorted(usable),
            "evidence_kind": case["evidence"]["kind"],
        },
    )


def bca_expected_for_target(
    point: float,
    alternative: str,
    confidence_level: float,
    bootstrap: list[float],
    jackknife: list[float],
) -> tuple[float | None, float | None]:
    normal = NormalDist()
    below = sum(value < point for value in bootstrap)
    ties = sum(value == point for value in bootstrap)
    fraction = (below + 0.5 * ties) / len(bootstrap)
    floor = 0.5 / len(bootstrap)
    z0 = normal.inv_cdf(min(1.0 - floor, max(floor, fraction)))
    mean = sum(jackknife) / len(jackknife)
    centered = [mean - value for value in jackknife]
    numerator = sum(value**3 for value in centered)
    denominator_base = sum(value**2 for value in centered)
    acceleration = 0.0 if denominator_base == 0.0 else numerator / (6.0 * denominator_base**1.5)

    def adjust(probability: float) -> float:
        z_alpha = normal.inv_cdf(probability)
        numerator_local = z0 + z_alpha
        transformed = z0 + numerator_local / (1.0 - acceleration * numerator_local)
        return min(1.0, max(0.0, normal.cdf(transformed)))

    lower_p, upper_p = tail_probabilities(confidence_level, alternative)
    return (
        None if lower_p is None else type7(bootstrap, adjust(lower_p)),
        None if upper_p is None else type7(bootstrap, adjust(upper_p)),
    )


def bca_expected(case: dict[str, Any]) -> tuple[float | None, float | None]:
    point, alternative, interval = interval_of_first_target(case)
    bootstrap = case["evidence"]["bootstrap"]["first_target_draws"]
    jackknife = case["evidence"]["delete_one"]["first_target_draws"]
    return bca_expected_for_target(
        point,
        alternative,
        float(interval["confidence_level"]),
        bootstrap,
        jackknife,
    )


def endpoints_match(interval: dict[str, Any], expected: tuple[float | None, float | None]) -> bool:
    actual = (interval.get("lower"), interval.get("upper"))
    for left, right in zip(actual, expected):
        if left is None or right is None:
            if left is not None or right is not None:
                return False
        elif not close(float(left), float(right), 2.0e-10):
            return False
    return True


def verify_bca(cases: list[dict[str, Any]]) -> tuple[bool, Any]:
    selected = [case for case in cases if case["case_id"].startswith("bca:")]
    alternatives = {
        interval_of_first_target(case)[1]
        for case in selected
        if case.get("fixture_role") == "non_null"
    }
    details = []
    valid = len(selected) == 6 and alternatives == ALTERNATIVES
    for case in selected:
        evidence = case["evidence"]
        targets = case["result"]["targets"]
        bootstrap_vectors = [
            vector["target_values"] for vector in evidence["bootstrap"]["successful_target_vectors"]
        ]
        jackknife_vectors = [
            vector["target_values"] for vector in evidence["delete_one"]["successful_target_vectors"]
        ]
        target_details = []
        all_targets_ok = True
        for target_index, target in enumerate(targets):
            interval = target["interval"]
            expected = bca_expected_for_target(
                float(target["estimate"]),
                interval["alternative"],
                float(interval["confidence_level"]),
                [float(vector[target_index]) for vector in bootstrap_vectors],
                [float(vector[target_index]) for vector in jackknife_vectors],
            )
            target_ok = interval["family"] == "full_delete_one_bca" and endpoints_match(interval, expected)
            all_targets_ok &= target_ok
            target_details.append(
                {
                    "target_id": target["target_id"],
                    "expected": expected,
                    "actual": interval,
                    "status": "passed" if target_ok else "failed",
                }
            )
        case_ok = (
            evidence["kind"] == "bca_case"
            and evidence.get("complete_delete_one") is True
            and evidence["delete_one"]["usable"] == evidence["delete_one"]["requested"]
            and evidence["delete_one"]["requested"] == case["dataset_rows"]
            and len(bootstrap_vectors) == evidence["bootstrap"]["usable"]
            and len(jackknife_vectors) == evidence["delete_one"]["usable"]
            and all_targets_ok
        )
        valid &= case_ok
        details.append({"case_id": case["case_id"], "targets": target_details})
    return valid, details


def verify_studentized(cases: list[dict[str, Any]]) -> tuple[bool, Any]:
    selected = [case for case in cases if case["case_id"].startswith("studentized:")]
    alternatives = {interval_of_first_target(case)[1] for case in selected}
    valid = len(selected) == 3 and alternatives == ALTERNATIVES
    details = []
    for case in selected:
        evidence = case["evidence"]
        nested = evidence["nested"]
        targets = case["result"]["targets"]
        observed_vectors = [
            vector["target_values"]
            for vector in evidence["observed_inner"]["successful_target_vectors"]
        ]
        outer = nested["outer_first_target_summaries"]
        target_details = []
        all_targets_ok = True
        for target_index, target in enumerate(targets):
            point = float(target["estimate"])
            interval = target["interval"]
            alternative = interval["alternative"]
            observed_values = [float(vector[target_index]) for vector in observed_vectors]
            observed_mean = sum(observed_values) / len(observed_values)
            observed_se = math.sqrt(
                sum((value - observed_mean) ** 2 for value in observed_values)
                / (len(observed_values) - 1)
            )
            usable_outer = [
                item
                for item in outer
                if len(item.get("outer_target_values", [])) == len(targets)
                and len(item.get("inner_target_standard_errors", [])) == len(targets)
                and item["inner_target_standard_errors"][target_index] is not None
                and item["inner_target_standard_errors"][target_index] > 0.0
            ]
            pivots = [
                (item["outer_target_values"][target_index] - point)
                / item["inner_target_standard_errors"][target_index]
                for item in usable_outer
            ]
            alpha = 1.0 - float(interval["confidence_level"])
            if alternative == "two_sided":
                expected = (
                    point - type7(pivots, 1.0 - alpha / 2.0) * observed_se,
                    point - type7(pivots, alpha / 2.0) * observed_se,
                )
            elif alternative == "less":
                expected = (None, point - type7(pivots, alpha) * observed_se)
            else:
                expected = (point - type7(pivots, 1.0 - alpha) * observed_se, None)
            target_ok = (
                len(usable_outer) == nested.get("usable_outer")
                and interval["family"] == "complete_nested_studentized"
                and endpoints_match(interval, expected)
            )
            all_targets_ok &= target_ok
            target_details.append(
                {
                    "target_id": target["target_id"],
                    "expected": expected,
                    "actual": interval,
                    "outer_pivots": len(pivots),
                    "status": "passed" if target_ok else "failed",
                }
            )
        case_ok = (
            evidence["kind"] == "studentized_case"
            and evidence.get("no_percentile_fallback") is True
            and nested.get("complete") is True
            and nested.get("record_count") == nested.get("requested_outer")
            and nested.get("usable_outer") >= nested.get("minimum_outer_required")
            and len(outer) == nested.get("usable_outer")
            and all(item["inner_usable"] >= nested["minimum_inner_required"] for item in outer)
            and len(observed_vectors) == evidence["observed_inner"]["usable"]
            and all_targets_ok
        )
        valid &= case_ok
        details.append({"case_id": case["case_id"], "targets": target_details})
    return valid, details


def percentile_expected(
    values: list[float], interval: dict[str, Any]
) -> tuple[float | None, float | None]:
    lower_probability, upper_probability = tail_probabilities(
        float(interval["confidence_level"]), interval["alternative"]
    )
    return (
        None if lower_probability is None else type7(values, lower_probability),
        None if upper_probability is None else type7(values, upper_probability),
    )


def conditional_percentile_vectors(case: dict[str, Any]) -> list[list[float]]:
    evidence = case["evidence"]
    if evidence["kind"] in {"percentile_case", "frequency_count_space"}:
        return [
            [float(value) for value in vector["target_values"]]
            for vector in evidence["bootstrap"]["successful_target_vectors"]
        ]
    if evidence["kind"] != "grouped_stratified":
        raise ValueError(f"case {case['case_id']} is not percentile evidence")
    groups = sorted(evidence["groups"], key=lambda group: group["group_id"])
    if not groups:
        raise ValueError("grouped percentile evidence has no groups")
    vectors_by_group: dict[str, dict[int, list[float]]] = {}
    template_count = int(groups[0]["ledger"]["expected_target_vector_width"])
    for group in groups:
        ledger = group["ledger"]
        if int(ledger["expected_target_vector_width"]) != template_count:
            raise ValueError("grouped template dimensions differ")
        vectors_by_group[group["group_id"]] = {
            int(vector["replicate_index"]): [float(value) for value in vector["target_values"]]
            for vector in ledger["successful_target_vectors"]
        }
    shared_indices = set.intersection(
        *(set(vectors) for vectors in vectors_by_group.values())
    )
    config = case["conditional_config"]
    contrasts = sorted(config.get("group_contrasts", []), key=lambda value: value["contrast_id"])
    first_group_local_targets = case["result"]["targets"][:template_count]
    contrast_template_indices = [
        index
        for index, target in enumerate(first_group_local_targets)
        if target["kind"] != "probe_contrast"
    ]
    combined = []
    for replicate_index in sorted(shared_indices):
        values: list[float] = []
        for group in groups:
            values.extend(vectors_by_group[group["group_id"]][replicate_index])
        for contrast in contrasts:
            left = vectors_by_group[contrast["left_group_id"]][replicate_index]
            right = vectors_by_group[contrast["right_group_id"]][replicate_index]
            values.extend(left[index] - right[index] for index in contrast_template_indices)
        combined.append(values)
    return combined


def verify_conditional_percentile(cases: list[dict[str, Any]]) -> tuple[bool, Any]:
    selected = [
        case
        for case in cases
        if case["evidence"]["kind"]
        in {"percentile_case", "grouped_stratified", "frequency_count_space"}
    ]
    valid = bool(selected)
    details = []
    for case in selected:
        vectors = conditional_percentile_vectors(case)
        targets = case["result"]["targets"]
        case_ok = bool(vectors) and all(len(vector) == len(targets) for vector in vectors)
        failed_targets = []
        for target_index, target in enumerate(targets):
            interval = target["interval"]
            expected = percentile_expected(
                [vector[target_index] for vector in vectors], interval
            )
            target_ok = (
                interval["family"] == "type_7_percentile"
                and endpoints_match(interval, expected)
            )
            case_ok &= target_ok
            if not target_ok:
                failed_targets.append(
                    {
                        "target_id": target["target_id"],
                        "expected": expected,
                        "actual": interval,
                    }
                )
        usable = {int(target["usable_replicates"]) for target in targets}
        case_ok &= usable == {len(vectors)}
        valid &= case_ok
        details.append(
            {
                "case_id": case["case_id"],
                "targets_recomputed": len(targets),
                "usable_vectors": len(vectors),
                "failed_targets": failed_targets,
            }
        )
    return valid, details


def semantic_target(target: dict[str, Any]) -> tuple[Any, ...]:
    return (
        target["kind"],
        target["path_id"],
        target.get("group_id"),
        tuple(sorted(target.get("probe_values", {}).items())),
        tuple(target.get("derivative_variables", [])),
    )


Polynomial = dict[tuple[tuple[str, int], ...], float]


def multiply_polynomials(left: Polynomial, right: Polynomial) -> Polynomial:
    output: Polynomial = {}
    for left_powers, left_coefficient in left.items():
        for right_powers, right_coefficient in right.items():
            powers = dict(left_powers)
            for moderator, exponent in right_powers:
                powers[moderator] = powers.get(moderator, 0) + exponent
            key = tuple(sorted((moderator, exponent) for moderator, exponent in powers.items() if exponent))
            output[key] = output.get(key, 0.0) + left_coefficient * right_coefficient
    return output


def edge_polynomial(edge: dict[str, Any]) -> Polynomial:
    output: Polynomial = {(): float(edge["intercept"])}
    for coefficient in edge.get("linear_coefficients", []):
        key = ((coefficient["moderator_id"], 1),)
        output[key] = output.get(key, 0.0) + float(coefficient["estimate"])
    for coefficient in edge.get("pairwise_coefficients", []):
        moderators = [coefficient["first_moderator_id"], coefficient["second_moderator_id"]]
        powers: dict[str, int] = {}
        for moderator in moderators:
            powers[moderator] = powers.get(moderator, 0) + 1
        key = tuple(sorted(powers.items()))
        output[key] = output.get(key, 0.0) + float(coefficient["estimate"])
    return output


def evaluate_polynomial(
    polynomial: Polynomial,
    probe: dict[str, float],
    derivative_variables: Iterable[str] = (),
) -> float:
    derivative_counts: dict[str, int] = {}
    for moderator in derivative_variables:
        derivative_counts[moderator] = derivative_counts.get(moderator, 0) + 1
    total = 0.0
    for powers_tuple, coefficient in polynomial.items():
        powers = dict(powers_tuple)
        multiplier = float(coefficient)
        eliminated = False
        for moderator, count in derivative_counts.items():
            exponent = powers.get(moderator, 0)
            if exponent < count:
                eliminated = True
                break
            for factor in range(count):
                multiplier *= exponent - factor
            powers[moderator] = exponent - count
        if eliminated:
            continue
        for moderator, exponent in powers.items():
            if exponent:
                if moderator not in probe:
                    raise ValueError(f"probe omits moderator {moderator}")
                multiplier *= float(probe[moderator]) ** exponent
        total += multiplier
    if not math.isfinite(total):
        raise ValueError("independent conditional target is nonfinite")
    return total


def conditional_point_math(case: dict[str, Any]) -> tuple[bool, Any]:
    """Rebuild every point target from emitted edge functions and frozen probes.

    This deliberately does not consume the runner's target vector or any
    SUT-emitted polynomial. The edge products, sums, derivatives, scalar index,
    finite probe contrasts, direct effects, and group differences are all
    reconstructed here.
    """

    config = case["conditional_config"]
    if any(probe.get("scale") != "standardized_score" for probe in config.get("probes", [])):
        raise ValueError("qualification oracle requires frozen standardized-score probes")
    point_fits = case.get("original_sample_point_fits", [])
    if not point_fits:
        raise ValueError("original-sample edge functions are absent")
    fits = {point_fit.get("group_id"): point_fit["point"] for point_fit in point_fits}
    if len(fits) != len(point_fits):
        raise ValueError("duplicate point-fit stratum identity")
    if any(
        point.get("receipt", {}).get("contract") is None
        or point.get("receipt", {}).get("raw_scientific_gamma_and_delta") is not True
        for point in fits.values()
    ):
        raise ValueError("point fit lacks the validated scientific gamma/delta receipt")

    paths = {path["path_id"]: path["ordered_relation_ids"] for path in config["paths"]}
    tuples = {
        probe["tuple_id"]: {key: float(value) for key, value in probe["values_by_moderator"].items()}
        for probe in config.get("explicit_joint_tuples", [])
    }
    contrasts = config.get("probe_contrasts", [])
    group_contrasts = config.get("group_contrasts", [])

    def fit_math(group_id: str | None) -> tuple[dict[str, Polynomial], dict[str, dict[str, Any]]]:
        point = fits[group_id]
        edges = {edge["relation_id"]: edge for edge in point["edges"]}
        polynomials: dict[str, Polynomial] = {}
        for path_id, relation_ids in paths.items():
            polynomial: Polynomial = {(): 1.0}
            for relation_id in relation_ids:
                polynomial = multiply_polynomials(polynomial, edge_polynomial(edges[relation_id]))
            polynomials[path_id] = polynomial
        return polynomials, edges

    fitted = {group_id: fit_math(group_id) for group_id in fits}

    def endpoint_path_ids(path_id: str, edges: dict[str, dict[str, Any]]) -> list[str]:
        relation_ids = paths[path_id]
        source = edges[relation_ids[0]]["source_id"]
        target = edges[relation_ids[-1]]["target_id"]
        matches = []
        for candidate, candidate_relations in paths.items():
            if (
                edges[candidate_relations[0]]["source_id"] == source
                and edges[candidate_relations[-1]]["target_id"] == target
            ):
                matches.append(candidate)
        return sorted(matches)

    def local_target(target: dict[str, Any], group_id: str | None, inferred_kind: str | None = None) -> float:
        polynomials, edges = fitted[group_id]
        kind = inferred_kind or target["kind"]
        path_id = target["path_id"]
        probe = {key: float(value) for key, value in target.get("probe_values", {}).items()}
        if kind == "conditional_specific_indirect":
            return evaluate_polynomial(polynomials[path_id], probe)
        if kind == "conditional_total_indirect":
            return sum(
                evaluate_polynomial(polynomials[candidate], probe)
                for candidate in endpoint_path_ids(path_id, edges)
            )
        if kind == "conditional_total_effect":
            relation_ids = paths[path_id]
            source = edges[relation_ids[0]]["source_id"]
            outcome = edges[relation_ids[-1]]["target_id"]
            direct = [edge for edge in edges.values() if edge["source_id"] == source and edge["target_id"] == outcome]
            if len(direct) != 1:
                raise ValueError(f"independent total effect found {len(direct)} direct edges for {source}->{outcome}")
            indirect = sum(
                evaluate_polynomial(polynomials[candidate], probe)
                for candidate in endpoint_path_ids(path_id, edges)
            )
            return indirect + evaluate_polynomial(edge_polynomial(direct[0]), probe)
        if kind == "scalar_index_of_moderated_mediation":
            variables = target.get("derivative_variables", [])
            if len(variables) != 1:
                raise ValueError("scalar index target lacks its unique moderator identity")
            moderator = variables[0]
            allowed = {(), ((moderator, 1),)}
            material = {powers for powers, coefficient in polynomials[path_id].items() if abs(coefficient) > 1.0e-14}
            if not material <= allowed:
                raise ValueError(f"path {path_id} is not independently affine in {moderator}")
            return polynomials[path_id].get(((moderator, 1),), 0.0)
        if kind in {"local_first_derivative", "local_second_derivative", "local_cross_derivative"}:
            return evaluate_polynomial(polynomials[path_id], probe, target.get("derivative_variables", []))
        if kind == "probe_contrast":
            if len(contrasts) != 1:
                raise ValueError("fixture probe-contrast identity is not unique")
            contrast = contrasts[0]
            return evaluate_polynomial(polynomials[path_id], tuples[contrast["left_tuple_id"]]) - evaluate_polynomial(
                polynomials[path_id], tuples[contrast["right_tuple_id"]]
            )
        raise ValueError(f"unsupported independent local target kind {kind}")

    comparisons: list[dict[str, Any]] = []
    valid = True
    for target in case["result"]["targets"]:
        if target["kind"] == "group_contrast":
            if len(group_contrasts) != 1:
                raise ValueError("fixture group-contrast identity is not unique")
            estimands = config["estimands"]
            if estimands.get("conditional_total_indirect") or estimands.get("conditional_total_effect"):
                raise ValueError("group contrast target kind is ambiguous for this oracle fixture")
            contrast = group_contrasts[0]
            if target.get("derivative_variables"):
                inferred_kind = (
                    "local_first_derivative"
                    if len(target["derivative_variables"]) == 1
                    else (
                        "local_second_derivative"
                        if len(set(target["derivative_variables"])) == 1
                        else "local_cross_derivative"
                    )
                )
            elif target.get("probe_values"):
                inferred_kind = "conditional_specific_indirect"
            else:
                inferred_kind = "scalar_index_of_moderated_mediation"
            expected = local_target(target, contrast["left_group_id"], inferred_kind) - local_target(
                target, contrast["right_group_id"], inferred_kind
            )
        else:
            expected = local_target(target, target.get("group_id"))
        actual = float(target["estimate"])
        target_ok = close(actual, expected, 2.0e-10)
        valid &= target_ok
        comparisons.append(
            {
                "target_id": target["target_id"],
                "kind": target["kind"],
                "expected": expected,
                "actual": actual,
                "absolute_error": abs(actual - expected),
                "status": "passed" if target_ok else "failed",
            }
        )
    kinds = sorted({comparison["kind"] for comparison in comparisons})
    return valid and len(comparisons) == len(case["result"]["targets"]), {
        "case_id": case["case_id"],
        "targets_recomputed": len(comparisons),
        "target_kinds": kinds,
        "maximum_absolute_error": max((comparison["absolute_error"] for comparison in comparisons), default=None),
        "failed_targets": [comparison for comparison in comparisons if comparison["status"] == "failed"],
    }


def verify_all_conditional_point_math(cases: list[dict[str, Any]]) -> tuple[bool, Any]:
    results = [conditional_point_math(case) for case in cases]
    return all(result[0] for result in results), [result[1] for result in results]


def verify_frequency(report: dict[str, Any]) -> tuple[bool, Any]:
    wrapper = next(
        case
        for case in report["cases"]
        if case["case_id"] == "frequency_weighted:count_space_and_physical_expansion"
    )
    compact = wrapper["compact_count_space"]
    expanded = wrapper["physical_expansion_reference"]
    compact_targets = {semantic_target(target): target["estimate"] for target in compact["result"]["targets"]}
    expanded_targets = {semantic_target(target): target["estimate"] for target in expanded["result"]["targets"]}
    bootstrap = compact["evidence"]["bootstrap"]
    draws = bootstrap["count_space_draws"]
    counts_ok = all(
        len(draw["counts"]) == compact["dataset_rows"]
        and sum(draw["counts"]) == draw["total_count"]
        and is_sha256(draw["counts_sha256"])
        and is_sha256(draw["draw_identity_sha256"])
        for draw in draws
    )
    equivalent = compact_targets.keys() == expanded_targets.keys() and all(
        close(compact_targets[key], expanded_targets[key], 2.0e-10) for key in compact_targets
    )
    return (
        compact["evidence"]["kind"] == "frequency_count_space"
        and compact["evidence"].get("physical_expansion_used") is False
        and len(draws) == bootstrap["requested"]
        and counts_ok
        and equivalent,
        {
            "compact_rows": compact["dataset_rows"],
            "expanded_rows": expanded["dataset_rows"],
            "target_count": len(compact_targets),
            "count_space_draws": len(draws),
        },
    )


def verify_conditional(report: dict[str, Any], audit: Audit) -> set[str]:
    leaves = conditional_leaf_cases(report)
    covered = collect_cell_ids(report)
    audit.add(
        "conditional.raw.profile_matrix",
        report.get("schema_version") == 1
        and report.get("producer_id") == "qpls.multimod.conditional.raw-qualification.v1"
        and report.get("execution_contract") == "public_recipe_v4_compiler_plus_builtin_raw_runner"
        and set(report.get("required_cell_ids", [])) == CONDITIONAL_CELLS
        and CONDITIONAL_CELLS <= covered
        and len(leaves) == 20,
        {"leaf_cases": len(leaves), "covered_cells": sorted(covered)},
    )
    generic_results = [conditional_generic(case) for case in leaves]
    audit.add(
        "conditional.raw.production_identity",
        all(result[0] for result in generic_results),
        [result[1] for result in generic_results],
    )
    audit.guard(
        "conditional.raw.independent_point_target_math",
        lambda: verify_all_conditional_point_math(leaves),
    )
    audit.guard(
        "conditional.raw.percentile_type7_all_targets",
        lambda: verify_conditional_percentile(leaves),
    )

    multi = [case for case in leaves if case["case_id"].startswith("multi_path_percentile:")]
    alternatives = {interval_of_first_target(case)[1] for case in multi}
    path_ids = set().union(*(set(case["path_id_inventory"]) for case in multi))
    kinds = set().union(*(set(case["target_kind_inventory"]) for case in multi))
    required_kinds = {
        '"conditional_specific_indirect"',
        '"conditional_total_indirect"',
        '"conditional_total_effect"',
        '"local_first_derivative"',
        '"local_second_derivative"',
        '"local_cross_derivative"',
        '"probe_contrast"',
    }
    audit.add(
        "conditional.raw.all_alternatives",
        len(multi) == 3 and alternatives == ALTERNATIVES,
        {"alternatives": sorted(alternatives)},
    )
    audit.add(
        "conditional.raw.probes_derivatives_contrasts",
        {"first_stage_2_edges", "second_stage_2_edges", "both_stage_2_edges", "long_path_6_edges"}
        <= path_ids
        and required_kinds <= kinds
        and all(case["compiled_plan"].get("profile") == "multi_two_way_percentile" for case in multi)
        and all(len(case["compiled_plan"].get("paths", [])) == 4 for case in multi)
        and all(len(case["compiled_plan"].get("interactions", [])) == 5 for case in multi),
        {
            "paths": sorted(path_ids),
            "target_kinds": sorted(kinds),
            "compiled_interaction_counts": [len(case["compiled_plan"].get("interactions", [])) for case in multi],
        },
    )
    audit.guard("conditional.raw.bca_full_delete_one", lambda: verify_bca(leaves))
    audit.guard("conditional.raw.studentized_nested", lambda: verify_studentized(leaves))

    three_cases = [case for case in leaves if case["case_id"].startswith("bounded_three_way:")]
    three = next(case for case in three_cases if case["case_id"] == "bounded_three_way:two_sided")
    audit.add(
        "conditional.raw.three_way_closure",
        set(three["path_id_inventory"]) == {"three_way_x_path", "lower_order_z_path"}
        and {interval_of_first_target(case)[1] for case in three_cases} == ALTERNATIVES
        and '"local_cross_derivative"' in set(three["target_kind_inventory"])
        and three["compiled_plan"].get("profile") == "bounded_three_way_percentile"
        and sorted(len(interaction["operands"]) for interaction in three["compiled_plan"].get("interactions", []))
        == [2, 2, 2, 3],
        {
            "paths": three["path_id_inventory"],
            "target_kinds": three["target_kind_inventory"],
            "operand_arities": sorted(
                len(interaction["operands"]) for interaction in three["compiled_plan"].get("interactions", [])
            ),
        },
    )
    hoc = next(case for case in leaves if case["case_id"] == "multiple_hoc:four_disjoint")
    audit.add(
        "conditional.raw.hoc_four_disjoint",
        set(hoc["path_id_inventory"]) == {f"hoc{index}_path" for index in range(1, 5)}
        and hoc["compiled_plan"].get("profile") == "multiple_hoc_percentile"
        and len(hoc["compiled_plan"].get("hocs", [])) == 4
        and len(hoc["compiled_plan"].get("interactions", [])) == 2,
        {
            "paths": hoc["path_id_inventory"],
            "compiled_hocs": len(hoc["compiled_plan"].get("hocs", [])),
            "compiled_interactions": len(hoc["compiled_plan"].get("interactions", [])),
        },
    )
    grouped = next(case for case in leaves if case["case_id"] == "grouped:stratified")
    groups = grouped["evidence"].get("groups", [])
    audit.add(
        "conditional.raw.grouped_stratified",
        {group["group_id"] for group in groups} == {"group-a", "group-b"}
        and all(group["ledger"]["complete"] for group in groups)
        and grouped["compiled_plan"].get("profile") == "grouped_percentile",
        {"group_ids": [group["group_id"] for group in groups]},
    )
    weighted = next(case for case in leaves if case["case_id"] == "case_weighted:positive")
    weights = weighted["analysis_frame"]["strata"][0]["case_weights"]
    kish = sum(weights) ** 2 / sum(value * value for value in weights)
    weight_ledger = weighted["evidence"]["bootstrap"]
    audit.add(
        "conditional.raw.case_weights_travel",
        weights
        and all(math.isfinite(value) and value > 0.0 for value in weights)
        and max(weights) / min(weights) <= 1.0e6
        and weight_ledger["draws_with_case_weights"] == weight_ledger["requested"]
        and weight_ledger["draws_without_case_weights"] == 0
        and weight_ledger["unique_case_weight_identity_count"] > 0
        and any("Kish effective sample size" in warning for warning in weighted["result"]["warnings"]),
        {"kish_ess": kish, "weighted_draws": weight_ledger["draws_with_case_weights"]},
    )
    audit.guard("conditional.raw.frequency_compact_equals_expanded", lambda: verify_frequency(report))

    boundary = report.get("qualification_boundary_guards", {})
    boundary_statuses = {
        name: boundary.get(name, {}).get("status")
        for name in [
            "incomplete_bca_jackknife",
            "studentized_outer_inner_budget",
            "group_count_below_minimum",
            "group_count_above_maximum",
            "case_weight_ratio",
            "noninteger_frequency",
            "excessive_frequency_total",
        ]
    }
    audit.add(
        "conditional.raw.boundary_guards",
        all(status == "blocked" for status in boundary_statuses.values())
        and "full delete-one jackknife" in boundary["incomplete_bca_jackknife"].get("error", "")
        and boundary["studentized_outer_inner_budget"].get("code")
        == "general_sem_conditional_process_v2.studentized_budget"
        and boundary["group_count_below_minimum"].get("code")
        == "general_sem_conditional_process_v2.group_count"
        and boundary["group_count_above_maximum"].get("code")
        == "general_sem_conditional_process_v2.group_count"
        and "exceeds 1e6" in boundary["case_weight_ratio"].get("error", "")
        and "not a positive exact integer" in boundary["noninteger_frequency"].get("error", "")
        and "exceeds 2^53-1" in boundary["excessive_frequency_total"].get("error", ""),
        {"statuses": boundary_statuses, "details": boundary},
    )

    blockers = report.get("unsupported_intersections", {})
    expected_blockers = {
        "group_plus_weight": "general_sem_conditional_process_v2.weight_profile",
        "hoc_plus_group": "general_sem_conditional_process_v2.group_profile",
        "three_way_plus_hoc": "general_sem_conditional_process_v2.hoc_profile",
        "studentized_outside_profile": "general_sem_conditional_process_v2.interval_profile",
    }
    audit.add(
        "conditional.raw.unsupported_intersections",
        set(blockers) == set(expected_blockers)
        and all(
            blockers[name].get("status") == "blocked" and blockers[name].get("code") == code
            for name, code in expected_blockers.items()
        ),
        blockers,
    )
    audit.add(
        "conditional.raw.shared_ledgers",
        all(case.get("one_result_ledger_for_all_targets") is True for case in leaves)
        and all(len(set(case["target_usable_replicate_counts"])) == 1 for case in leaves),
        {"cases": len(leaves)},
    )
    return covered


def dot(left: list[float], right: list[float]) -> float:
    return sum(a * b for a, b in zip(left, right))


def canonical_equations(equations: list[dict[str, Any]]) -> list[dict[str, Any]]:
    canonical = json.loads(json.dumps(equations))
    for equation in canonical:
        for term in equation["terms"]:
            term["factor_variable_ids"].sort()
        equation["terms"].sort(key=lambda term: (term["term_id"], term["factor_variable_ids"]))
    return canonical


def fit_equation(equation: dict[str, Any], columns: dict[str, list[float]]) -> dict[str, Any]:
    outcome = columns[equation["outcome_variable_id"]]
    count = len(outcome)
    design = [[1.0] * count]
    for term in equation["terms"]:
        design.append(
            [
                math.prod(columns[factor][row] for factor in term["factor_variable_ids"])
                for row in range(count)
            ]
        )
    q_columns: list[list[float]] = []
    upper = [[0.0 for _ in design] for _ in design]
    for column_index, source in enumerate(design):
        residual = source[:]
        for prior in range(column_index):
            upper[prior][column_index] = dot(q_columns[prior], residual)
            residual = [
                value - upper[prior][column_index] * q_columns[prior][row]
                for row, value in enumerate(residual)
            ]
        residual_norm = math.sqrt(dot(residual, residual))
        if residual_norm <= 1.0e-12 * max(1.0, math.sqrt(dot(source, source))):
            raise ValueError(f"rank deficient oracle equation {equation['equation_id']}")
        upper[column_index][column_index] = residual_norm
        q_columns.append([value / residual_norm for value in residual])
    projected = [dot(column, outcome) for column in q_columns]
    coefficients = [0.0] * len(design)
    for row in range(len(design) - 1, -1, -1):
        remainder = sum(upper[row][column] * coefficients[column] for column in range(row + 1, len(design)))
        coefficients[row] = (projected[row] - remainder) / upper[row][row]
    fitted = [((), coefficients[0])]
    fitted.extend(
        (tuple(term["factor_variable_ids"]), coefficients[index + 1])
        for index, term in enumerate(equation["terms"])
    )
    return {"outcome": equation["outcome_variable_id"], "coefficients": fitted}


def predict(equation: dict[str, Any], context: dict[str, float]) -> float:
    return sum(coefficient * math.prod(context[factor] for factor in factors) for factors, coefficient in equation["coefficients"])


def independent_gcomp(prepared: dict[str, Any]) -> dict[str, float]:
    input_value = prepared["input"]
    columns = {column["variable_id"]: [float(value) for value in column["values"]] for column in input_value["columns"]}
    equations = canonical_equations(input_value["equations"])
    fitted = [fit_equation(equation, columns) for equation in equations]
    mediator_count = len(input_value["ordered_mediator_variable_ids"])
    treatment = input_value["treatment_variable_id"]
    count = len(next(iter(columns.values())))

    def mean(outcome_x: float, mediator_x: float) -> float:
        total = 0.0
        for row in range(count):
            context: dict[str, float] = {}
            for variable in input_value["adjustment_covariate_variable_ids"] + input_value["baseline_moderator_variable_ids"]:
                context[variable] = input_value.get("baseline_moderator_intervention_values", {}).get(
                    variable, columns[variable][row]
                )
            context[treatment] = mediator_x
            for equation in fitted[:mediator_count]:
                context[equation["outcome"]] = predict(equation, context)
            context[treatment] = outcome_x
            total += predict(fitted[-1], context)
        return total / count

    contrast = input_value["treatment_contrast"]
    x0, x1 = float(contrast["x0"]), float(contrast["x1"])
    y00, y10, y11 = mean(x0, x0), mean(x1, x0), mean(x1, x1)
    return {
        "interventional_direct_effect": y10 - y00,
        "joint_interventional_indirect_effect": y11 - y10,
        "total_interventional_contrast": y11 - y00,
    }


def analytic_causal_recovery(case: dict[str, Any]) -> tuple[bool, Any]:
    contract = case.get("dgp", {}).get("recovery_contract", {})
    truths = contract.get("analytic_targets_by_path", {})
    tolerance = float(contract.get("maximum_absolute_recovery_error", math.nan))
    minimum_signal = float(contract.get("minimum_absolute_nonzero_target", math.nan))
    minimum_fraction = float(contract.get("minimum_recovered_nonzero_fraction", math.nan))
    require_detection = contract.get("confidence_intervals_must_exclude_zero") is True
    effects = {(effect["path_id"], effect["estimand"]): effect for effect in case["result"]["effects"]}
    comparisons: list[dict[str, Any]] = []
    recovered = 0
    valid = (
        truths
        and math.isfinite(tolerance)
        and tolerance > 0.0
        and math.isfinite(minimum_signal)
        and minimum_signal > 0.0
        and 0.0 < minimum_fraction <= 1.0
        and contract.get("power_claim", "").startswith("none;")
    )
    for path_id, estimands in truths.items():
        for estimand, truth_value in estimands.items():
            truth = float(truth_value)
            effect = effects.get((path_id, estimand))
            if effect is None:
                valid = False
                comparisons.append({"path_id": path_id, "estimand": estimand, "status": "missing"})
                continue
            actual = float(effect["estimate"])
            interval = effect.get("interval", {})
            lower = interval.get("lower")
            upper = interval.get("upper")
            detected = (
                (lower is not None and float(lower) > 0.0)
                or (upper is not None and float(upper) < 0.0)
            )
            target_ok = (
                math.isfinite(truth)
                and abs(truth) >= minimum_signal
                and abs(actual - truth) <= tolerance
                and (not require_detection or detected)
            )
            valid &= target_ok
            recovered += int(target_ok)
            comparisons.append(
                {
                    "path_id": path_id,
                    "estimand": estimand,
                    "truth": truth,
                    "actual": actual,
                    "absolute_error": abs(actual - truth),
                    "interval_excludes_zero": detected,
                    "status": "passed" if target_ok else "failed",
                }
            )
    observed_keys = set(effects)
    truth_keys = {(path_id, estimand) for path_id, estimands in truths.items() for estimand in estimands}
    valid &= observed_keys == truth_keys and bool(comparisons)
    recovered_fraction = recovered / len(comparisons) if comparisons else 0.0
    valid &= recovered_fraction >= minimum_fraction
    return valid, {
        "case_id": case.get("case_id"),
        "maximum_absolute_recovery_error": tolerance,
        "minimum_absolute_nonzero_target": minimum_signal,
        "required_recovered_fraction": minimum_fraction,
        "actual_recovered_fraction": recovered_fraction,
        "power_claim": contract.get("power_claim"),
        "comparisons": comparisons,
    }


def causal_role_and_path_contract(case: dict[str, Any]) -> tuple[bool, Any]:
    dgp = case["dgp"]
    expected_edges = sorted(int(value) for value in dgp["selected_path_edges"])
    actual_edges: list[int] = []
    details: list[dict[str, Any]] = []
    valid = True
    for prepared in case["prepared_paths"]:
        input_value = prepared["input"]
        mediators = input_value["ordered_mediator_variable_ids"]
        equations = input_value["equations"]
        outcomes = [equation["outcome_variable_id"] for equation in equations]
        expected_outcomes = mediators + [input_value["outcome_variable_id"]]
        columns = {column["variable_id"] for column in input_value["columns"]}
        required_roles = {
            input_value["treatment_variable_id"],
            input_value["outcome_variable_id"],
            *mediators,
            *input_value["adjustment_covariate_variable_ids"],
            *input_value["baseline_moderator_variable_ids"],
        }
        contrast = input_value["treatment_contrast"]
        treatment_kind_ok = (
            dgp["treatment_kind"] == "binary"
            and contrast.get("kind") == "binary"
            and close(float(contrast["x0"]), 0.0)
            and close(float(contrast["x1"]), 1.0)
        ) or (
            dgp["treatment_kind"] == "continuous"
            and contrast.get("kind") == "continuous_contrast"
            and close(float(contrast["x0"]), float(dgp["contrast"]["x0"]))
            and close(float(contrast["x1"]), float(dgp["contrast"]["x1"]))
        )
        path_ok = (
            outcomes == expected_outcomes
            and required_roles <= columns
            and input_value["adjustment_covariate_variable_ids"] == dgp["adjustment_set"]
            and len(equations) == len(mediators) + 1
            and treatment_kind_ok
            and input_value["treatment_variable_id"] not in mediators
            and all(
                equation["outcome_variable_id"] != input_value["treatment_variable_id"]
                for equation in equations
            )
        )
        valid &= path_ok
        actual_edges.append(len(mediators) + 1)
        details.append(
            {
                "path_id": prepared["path_id"],
                "equation_outcome_roles": outcomes,
                "ordered_mediators": mediators,
                "adjustment_set": input_value["adjustment_covariate_variable_ids"],
                "baseline_moderators": input_value["baseline_moderator_variable_ids"],
                "treatment_contrast": contrast,
                "status": "passed" if path_ok else "failed",
            }
        )
    valid &= sorted(actual_edges) == expected_edges
    return valid, {"case_id": case["case_id"], "paths": details}


def causal_bootstrap_type7(case: dict[str, Any]) -> tuple[bool, Any]:
    ordered_effects = case["result"]["effects"]
    ledger = case["bootstrap_evidence"]
    bootstrap_vectors = [
        [float(value) for value in vector["target_values"]]
        for vector in ledger.get("successful_target_vectors", [])
    ]
    valid = bool(bootstrap_vectors) and all(
        len(vector) == len(ordered_effects) for vector in bootstrap_vectors
    )
    details = []
    for target_index, effect in enumerate(ordered_effects):
        interval = effect["interval"]
        expected_interval = percentile_expected(
            [vector[target_index] for vector in bootstrap_vectors], interval
        )
        target_ok = (
            interval["family"] == "type_7_two_sided_percentile"
            and interval["alternative"] == "two_sided"
            and endpoints_match(interval, expected_interval)
        )
        valid &= target_ok
        details.append(
            {
                "target_id": effect["target_id"],
                "expected": expected_interval,
                "actual": interval,
                "status": "passed" if target_ok else "failed",
            }
        )
    valid &= len(bootstrap_vectors) == ledger.get("usable")
    return valid, {"case_id": case["case_id"], "targets": details}


def verify_causal_case(case: dict[str, Any]) -> tuple[bool, Any]:
    basic = verify_compiler_and_ledger(case)[0]
    ordered_effects = case["result"]["effects"]
    effects = {(effect["path_id"], effect["estimand"]): effect for effect in ordered_effects}
    independent: dict[str, dict[str, float]] = {}
    valid = basic
    for prepared in case["prepared_paths"]:
        path_id = prepared["path_id"]
        expected = independent_gcomp(prepared)
        independent[path_id] = expected
        for estimand, estimate in expected.items():
            actual = effects.get((path_id, estimand))
            valid &= actual is not None and close(float(actual["estimate"]), estimate, 5.0e-9)
    analytic_recovery = analytic_causal_recovery(case)
    valid &= analytic_recovery[0]
    role_contract = causal_role_and_path_contract(case)
    valid &= role_contract[0]
    ledger = case["bootstrap_evidence"]
    bootstrap_check = causal_bootstrap_type7(case)
    valid &= bootstrap_check[0]
    valid &= (
        ledger_ok(ledger, len(effects))
        and case["compiled_plan"].get("kind") == "interventional_causal_mediation_v1"
        and set(case["compiled_plan"].get("path_ids", []))
        == {prepared["path_id"] for prepared in case["prepared_paths"]}
        and case.get("target_ids_are_unique") is True
        and case.get("interpretation_contains_assumption_dependent_interventional_estimate") is True
        and case.get("interpretation_avoids_causality_established") is True
    )
    return valid, {
        "case_id": case["case_id"],
        "independent_targets": independent,
        "analytic_recovery": analytic_recovery[1],
        "role_and_path_contract": role_contract[1],
        "independent_bootstrap_type7": bootstrap_check[1],
    }


def verify_causal(report: dict[str, Any], audit: Audit) -> set[str]:
    covered = collect_cell_ids(report)
    cases = report.get("cases", [])
    audit.add(
        "causal.raw.profile_matrix",
        report.get("schema_version") == 1
        and report.get("producer_id") == "qpls.multimod.interventional.raw-qualification.v1"
        and report.get("execution_contract") == "public_recipe_v4_compiler_plus_raw_observed_g_computation_runner"
        and set(report.get("required_cell_ids", [])) == CAUSAL_CELLS
        and CAUSAL_CELLS <= covered
        and len(cases) == 3,
        {"cases": len(cases), "covered_cells": sorted(covered)},
    )
    independent_results = [verify_causal_case(case) for case in cases]
    audit.add(
        "causal.raw.independent_gcomp",
        all(result[0] for result in independent_results),
        [result[1] for result in independent_results],
    )
    analytic_results = [analytic_causal_recovery(case) for case in cases]
    audit.add(
        "causal.raw.predeclared_analytic_truth_recovery",
        all(result[0] for result in analytic_results),
        [result[1] for result in analytic_results],
    )
    role_results = [causal_role_and_path_contract(case) for case in cases]
    audit.add(
        "causal.raw.observed_role_and_equation_contract",
        all(result[0] for result in role_results),
        [result[1] for result in role_results],
    )
    bootstrap_results = [causal_bootstrap_type7(case) for case in cases]
    audit.add(
        "causal.raw.bootstrap_type7_all_targets",
        all(result[0] for result in bootstrap_results),
        [result[1] for result in bootstrap_results],
    )
    by_id = {case["case_id"]: case for case in cases}
    audit.add(
        "causal.raw.binary_known_target",
        {"binary_two_edge_path", "binary_four_edge_path"} <= set(by_id)
        and all(
            by_id[case_id]["dgp"]["treatment_kind"] == "binary"
            for case_id in {"binary_two_edge_path", "binary_four_edge_path"}
        ),
        {
            case_id: by_id.get(case_id, {}).get("dgp")
            for case_id in ["binary_two_edge_path", "binary_four_edge_path"]
        },
    )
    audit.add(
        "causal.raw.continuous_known_target",
        "continuous_three_edge_path" in by_id
        and by_id["continuous_three_edge_path"]["dgp"]["treatment_kind"] == "continuous",
        by_id.get("continuous_three_edge_path", {}).get("dgp"),
    )
    edge_lengths = {
        len(path["input"]["ordered_mediator_variable_ids"]) + 1
        for case in cases
        for path in case["prepared_paths"]
    }
    audit.add("causal.raw.path_lengths_2_3_4", edge_lengths == {2, 3, 4}, sorted(edge_lengths))
    audit.add(
        "causal.raw.wording",
        all(
            case.get("interpretation_contains_assumption_dependent_interventional_estimate") is True
            and case.get("interpretation_avoids_causality_established") is True
            for case in cases
        ),
        [case["result"]["interpretation_label"] for case in cases],
    )
    audit.add(
        "causal.raw.bootstrap_identity",
        all(
            ledger_ok(case["bootstrap_evidence"], len(case["result"]["effects"]))
            and case["result"]["replicate_ledger"]["ledger_sha256"]
            == case["bootstrap_evidence"]["ledger_sha256"]
            for case in cases
        ),
        [case["bootstrap_evidence"]["ledger_sha256"] for case in cases],
    )
    blockers = report.get("assumption_and_scope_blockers", {})
    audit.add(
        "causal.raw.positivity_blocker",
        blockers.get("positivity_failure_from_raw_runner", {}).get("status") == "blocked"
        and "positivity" in blockers.get("positivity_failure_from_raw_runner", {}).get("error", "").lower(),
        blockers.get("positivity_failure_from_raw_runner"),
    )
    config_names = {
        "missing_adjustment_set": "interventional_causal_mediation_v1.adjustment_set_missing",
        "temporal_order_unreviewed": "interventional_causal_mediation_v1.identification",
        "recanting_witness_not_excluded": "interventional_causal_mediation_v1.identification",
        "exposure_induced_confounding_not_excluded": "interventional_causal_mediation_v1.identification",
    }
    audit.add(
        "causal.raw.identification_blockers",
        all(
            blockers.get(name, {}).get("status") == "blocked"
            and blockers[name].get("code") == code
            for name, code in config_names.items()
        ),
        {name: blockers.get(name) for name in sorted(config_names)},
    )
    scope_names = {
        "natural_or_cross_world_effect_request",
        "recanting_witness_present",
        "exposure_induced_confounder_present",
        "latent_composite_or_hoc_role_request",
        "group_request_compile",
        "weight_request_compile",
    }
    audit.add(
        "causal.raw.unsupported_scope",
        all(blockers.get(name, {}).get("status") == "blocked" for name in scope_names)
        and all(
            "UnsupportedRoleOrFeature" in blockers[name].get("codes", [])
            for name in {
                "natural_or_cross_world_effect_request",
                "recanting_witness_present",
                "exposure_induced_confounder_present",
                "latent_composite_or_hoc_role_request",
            }
        )
        and all(
            "causal" in blockers[name].get("error", "").lower()
            for name in {"group_request_compile", "weight_request_compile"}
        )
        and report.get("api_boundaries", {}).get("recipe_v4_natural_or_cross_world_request", "").startswith(
            "not_representable_by_design"
        ),
        {name: blockers.get(name) for name in sorted(scope_names)},
    )
    audit.add(
        "causal.raw.persistence_ready_identities",
        all(verify_compiler_and_ledger(case)[0] for case in cases)
        and all(case.get("target_ids_are_unique") is True for case in cases),
        {"cases": [case["case_id"] for case in cases]},
    )
    return covered


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--family", choices=("conditional", "causal"), required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--expected-scale", choices=("development", "qualification"), required=True)
    parser.add_argument("--require-shard-receipts", action="store_true")
    parser.add_argument("--shard-plan", type=Path)
    parser.add_argument("--shard-dir", type=Path)
    parser.add_argument("--producer-executable", type=Path)
    parser.add_argument("--expected-source-commit")
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()

    raw = arguments.report.read_bytes()
    report = json.loads(raw)
    audit = Audit()
    audit.add("receipt.scale", report.get("scale") == arguments.expected_scale, report.get("scale"))
    audit.add("receipt.seed", report.get("seed") == 42, report.get("seed"))
    audit.add("receipt.no_self_qualification", report.get("qualification_claim") == "none", report.get("qualification_claim"))
    audit.add(
        "receipt.baseline_shard_execution",
        (
            not arguments.require_shard_receipts
            or (
                report.get("metamorphism") == "baseline"
                and report.get("sign_columns") is None
                and report.get("workers") == 1
            )
        ),
        {
            "required": arguments.require_shard_receipts,
            "metamorphism": report.get("metamorphism"),
            "sign_columns": report.get("sign_columns"),
            "workers": report.get("workers"),
        },
    )
    shard_receipt = verify_shard_execution_receipt(
        report,
        arguments.family,
        arguments.require_shard_receipts,
        arguments.shard_plan,
        arguments.shard_dir,
        arguments.producer_executable,
        arguments.expected_source_commit,
    )
    audit.add(
        f"{arguments.family}.raw.complete_atomic_shard_inventory",
        shard_receipt[0],
        shard_receipt[1],
    )
    if arguments.family == "conditional":
        covered = verify_conditional(report, audit)
        expected = CONDITIONAL_CELLS
        gate_id = "qpls.v256.multimod.conditional.raw_qualification.v1"
    else:
        covered = verify_causal(report, audit)
        expected = CAUSAL_CELLS
        gate_id = "qpls.v256.multimod.causal.raw_qualification.v1"

    receipt = {
        "schema_version": 1,
        "gate_id": gate_id,
        "family": arguments.family,
        "status": "passed" if audit.passed else "failed",
        "source_report_sha256": hashlib.sha256(raw).hexdigest(),
        "source_report_path": str(arguments.report.resolve()),
        "expected_scale": arguments.expected_scale,
        "covered_cell_ids": sorted(covered & expected),
        "missing_cell_ids": sorted(expected - covered),
        "scientific_checks": audit.checks,
        "results": {arguments.family: {"checks": audit.checks}},
    }
    receipt["receipt_sha256"] = hashlib.sha256(canonical_json(receipt)).hexdigest()
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if not audit.passed:
        for check in audit.checks:
            if check["status"] != "passed":
                print(f"FAIL {check['check_id']}: {check['detail']}", file=sys.stderr)
        return 1
    print(f"PASS {gate_id} {receipt['receipt_sha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
