#!/usr/bin/env python3
"""Independent oracle for raw FIMIX-PLS and PLS-POS V2 receipts."""

from __future__ import annotations

import argparse
import hashlib
import itertools
import json
import math
import struct
from collections import Counter, defaultdict
from pathlib import Path
from statistics import median
from typing import Any, Iterable


SUITE_ID = "qpls.multimod.heterogeneity.production-qualification.v2"
FIMIX = "qpls.fimix-pls.v2"
POS_PUBLISHED = "qpls.pls-pos.published.v2"
POS_INTERACTIONS = "qpls.pls-pos.destination-scored-interactions.v2"
TOL = 3.0e-9
TARGET_DIGEST_DOMAIN = b"quickpls:heterogeneity:target-payload:v2\0"
MULTISTART_PARTITION_DOMAIN = b"quickpls:heterogeneity:multistart-partition:v2\0"
MULTISTART_COEFFICIENT_DOMAIN = b"quickpls:heterogeneity:multistart-coefficients:v2\0"
MULTISTART_POSTERIOR_DOMAIN = b"quickpls:heterogeneity:multistart-posteriors:v2\0"
MULTISTART_PARAMETER_DOMAIN = b"quickpls:heterogeneity:multistart-parameters:v2\0"
MULTISTART_FIT_STATISTIC_DOMAIN = b"quickpls:heterogeneity:multistart-fit-statistic:v2\0"

# Exact identities consumed by the POS/common-metric/bootstrap dependency
# gates. This literal inventory keeps formatted check emission source-bound.
BOUND_GATE_CHECK_IDS = {
    "fimix.boundary.rank_deficient_fail_closed",
    "fimix.boundary.variance_collapse_fail_closed",
    "fimix.boundary.rare_class_fixture_exercised",
    "pos.published_p0.partition_recovery_ari",
    "pos.published_p0.independent_outcome_r2_objective",
    "pos.destination_p2.partition_recovery_ari",
    "pos.destination_p2.independent_outcome_r2_objective",
    "pos.destination_p23.partition_recovery_ari",
    "pos.destination_p23.independent_outcome_r2_objective",
    "pos.overlap.partition_recovery_ari",
    "pos.homogeneous_null.no_truth_recovery_claim",
    "pos.published_p0.k2_through_k5_candidate_profile_algorithm_identity",
    "pos.published_p0.k3.candidate_profile_algorithm_identity",
    "pos.published_p0.k4.candidate_profile_algorithm_identity",
    "pos.published_p0.k5.candidate_profile_algorithm_identity",
    "pos.common_metric.pos-published-p0-fixed-k-bootstrap.independent_micom_step2",
    "pos.common_metric.pos-published-p0-fixed-k-bootstrap.pass",
    "pos.common_metric.pos-destination-p2-fixed-k-bootstrap.independent_micom_step2",
    "pos.common_metric.pos-destination-p2-fixed-k-bootstrap.pass",
    "pos.common_metric.pos-destination-p23-fixed-k-bootstrap.independent_micom_step2",
    "pos.common_metric.pos-destination-p23-fixed-k-bootstrap.pass",
    "pos.common_metric.failure_fixture.independent_micom_step2",
    "pos.common_metric.failure_fixture.fail_closed",
    "heterogeneity.bootstrap.full_profile_matrix",
    "heterogeneity.bootstrap.fimix-p0-fixed-k-bootstrap.fixed_k_label_aligned_shared_ledger",
    "heterogeneity.bootstrap.fimix-p0-fixed-k-bootstrap.shared_validity_bitmap_type7",
    "heterogeneity.bootstrap.pos-published-p0-fixed-k-bootstrap.fixed_k_label_aligned_shared_ledger",
    "heterogeneity.bootstrap.pos-destination-p2-fixed-k-bootstrap.fixed_k_label_aligned_shared_ledger",
    "heterogeneity.bootstrap.pos-destination-p23-fixed-k-bootstrap.fixed_k_label_aligned_shared_ledger",
    "heterogeneity.bootstrap.pos_published_p0_k2_through_k5_inventory",
    "heterogeneity.bootstrap.pos-published-p0-k3-fixed-k-bootstrap.fixed_k_label_aligned_shared_ledger",
    "heterogeneity.bootstrap.pos-published-p0-k3-fixed-k-bootstrap.shared_validity_bitmap_type7",
    "heterogeneity.bootstrap.pos-published-p0-k4-fixed-k-bootstrap.fixed_k_label_aligned_shared_ledger",
    "heterogeneity.bootstrap.pos-published-p0-k4-fixed-k-bootstrap.shared_validity_bitmap_type7",
    "heterogeneity.bootstrap.pos-published-p0-k5-fixed-k-bootstrap.fixed_k_label_aligned_shared_ledger",
    "heterogeneity.bootstrap.pos-published-p0-k5-fixed-k-bootstrap.shared_validity_bitmap_type7",
    "heterogeneity.label_alignment.k3.exhaustive_k_factorial_decisions",
    "heterogeneity.label_alignment.k4.exhaustive_k_factorial_decisions",
    "heterogeneity.label_alignment.k5.exhaustive_k_factorial_decisions",
    "heterogeneity.multistart.independent_replay_present",
}


def close(left: float, right: float, tolerance: float = TOL) -> bool:
    return math.isfinite(left) and math.isfinite(right) and abs(left - right) <= tolerance * max(
        1.0, abs(left), abs(right)
    )


def file_sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def is_sha256(value: Any) -> bool:
    return isinstance(value, str) and len(value) == 64 and all(
        character in "0123456789abcdef" for character in value
    )


def is_dataset_fingerprint(value: Any) -> bool:
    return is_sha256(value[3:]) if isinstance(value, str) and value.startswith("v2:") else is_sha256(value)


def target_payload_sha256(values: list[float]) -> str:
    if not values or any(not math.isfinite(value) for value in values):
        raise ValueError("target digest requires a nonempty finite vector")
    digest = hashlib.sha256()
    digest.update(TARGET_DIGEST_DOMAIN)
    digest.update(struct.pack("<Q", len(values)))
    for value in values:
        digest.update(struct.pack("<d", value))
    return digest.hexdigest()


def multistart_partition_sha256(assignments: list[int]) -> str:
    if not assignments or any(value < 0 for value in assignments):
        raise ValueError("multistart partition digest requires nonnegative labels")
    digest = hashlib.sha256()
    digest.update(MULTISTART_PARTITION_DOMAIN)
    digest.update(struct.pack("<Q", len(assignments)))
    for value in assignments:
        digest.update(struct.pack("<Q", value))
    return digest.hexdigest()


def multistart_matrix_sha256(domain: bytes, rows: list[list[float]]) -> str:
    if not domain or not rows or any(not row for row in rows):
        raise ValueError("multistart matrix digest requires a nonempty matrix")
    digest = hashlib.sha256()
    digest.update(domain)
    digest.update(struct.pack("<Q", len(rows)))
    for row in rows:
        digest.update(struct.pack("<Q", len(row)))
        for value in row:
            if not math.isfinite(value):
                raise ValueError("multistart matrix digest requires finite values")
            digest.update(struct.pack("<d", value))
    return digest.hexdigest()


def multistart_fit_statistic_sha256(value: float) -> str:
    if not math.isfinite(value):
        raise ValueError("multistart fit statistic must be finite")
    digest = hashlib.sha256()
    digest.update(MULTISTART_FIT_STATISTIC_DOMAIN)
    digest.update(struct.pack("<d", value))
    return digest.hexdigest()


def multistart_alignment(
    reference: list[int], candidate: list[int], classes: int
) -> tuple[list[int], int, bool]:
    if not 2 <= classes <= 5 or len(reference) != len(candidate) or not reference:
        raise ValueError("multistart alignment requires equal nonempty K=2 through K=5 partitions")
    if set(reference) != set(range(classes)) or set(candidate) != set(range(classes)):
        raise ValueError("multistart alignment requires every label to be represented")
    overlap = [[0 for _ in range(classes)] for _ in range(classes)]
    for reference_label, candidate_label in zip(reference, candidate, strict=True):
        overlap[reference_label][candidate_label] += 1
    scored = [
        (
            sum(overlap[mapping[candidate_label]][candidate_label] for candidate_label in range(classes)),
            list(mapping),
        )
        for mapping in itertools.permutations(range(classes))
    ]
    best_score = max(score for score, _ in scored)
    best = [mapping for score, mapping in scored if score == best_score]
    return best[0], best_score, len(best) > 1


def reconstruct_label_alignment(alignment: dict[str, Any]) -> tuple[bool, int, int]:
    overlap = [[int(value) for value in row] for row in alignment.get("overlap", [])]
    classes = len(overlap)
    mapping = [int(value) for value in alignment.get("candidate_to_reference", [])]
    if not 2 <= classes <= 5 or len(mapping) != classes:
        return False, classes, 0
    if any(len(row) != classes or any(value < 0 for value in row) for row in overlap):
        return False, classes, 0
    if sorted(mapping) != list(range(classes)):
        return False, classes, 0
    reference_counts = [sum(row) for row in overlap]
    candidate_counts = [sum(overlap[row][column] for row in range(classes)) for column in range(classes)]
    observations = sum(reference_counts)
    if observations == 0 or any(value == 0 for value in reference_counts + candidate_counts):
        return False, classes, observations
    scores = [
        sum(overlap[permutation[candidate]][candidate] for candidate in range(classes))
        for permutation in itertools.permutations(range(classes))
    ]
    best_score = max(scores)
    retained_score = sum(overlap[mapping[candidate]][candidate] for candidate in range(classes))
    mutual_majority = all(
        2 * overlap[mapping[candidate]][candidate] > reference_counts[mapping[candidate]]
        and 2 * overlap[mapping[candidate]][candidate] > candidate_counts[candidate]
        for candidate in range(classes)
    )
    valid = (
        retained_score == best_score
        and int(alignment.get("matched_observations", -1)) == best_score
        and close(float(alignment.get("match_share", math.nan)), best_score / observations, 1.0e-12)
        and alignment.get("ambiguous") is (scores.count(best_score) > 1)
        and alignment.get("mutual_majority") is mutual_majority
    )
    return valid, classes, observations


def exhaustive_alignment_from_partitions(
    reference: list[int], candidate: list[int], classes: int
) -> dict[str, Any]:
    if not 3 <= classes <= 5 or not reference or len(reference) != len(candidate):
        raise ValueError("decision probe requires equal nonempty K=3 through K=5 partitions")
    if set(reference) != set(range(classes)) or set(candidate) != set(range(classes)):
        raise ValueError("decision probe must represent every label")
    overlap = [[0 for _ in range(classes)] for _ in range(classes)]
    for reference_label, candidate_label in zip(reference, candidate, strict=True):
        if not 0 <= reference_label < classes or not 0 <= candidate_label < classes:
            raise ValueError("decision probe contains an out-of-range label")
        overlap[reference_label][candidate_label] += 1
    permutations = list(itertools.permutations(range(classes)))
    scores = [
        sum(overlap[mapping[candidate_label]][candidate_label] for candidate_label in range(classes))
        for mapping in permutations
    ]
    best_score = max(scores)
    best_mappings = [
        list(mapping)
        for mapping, score in zip(permutations, scores, strict=True)
        if score == best_score
    ]
    reference_counts = [sum(row) for row in overlap]
    candidate_counts = [
        sum(overlap[reference_label][candidate_label] for reference_label in range(classes))
        for candidate_label in range(classes)
    ]
    mutual_majority_by_mapping = {
        ",".join(str(value) for value in mapping): all(
            2 * overlap[mapping[candidate_label]][candidate_label]
            > reference_counts[mapping[candidate_label]]
            and 2 * overlap[mapping[candidate_label]][candidate_label]
            > candidate_counts[candidate_label]
            for candidate_label in range(classes)
        )
        for mapping in best_mappings
    }
    return {
        "overlap": overlap,
        "permutations_evaluated": len(permutations),
        "best_score": best_score,
        "best_mappings": best_mappings,
        "ambiguous": len(best_mappings) > 1,
        "mutual_majority_by_mapping": mutual_majority_by_mapping,
    }


def validate_label_alignment_decision_matrix(report: dict[str, Any], checks: "Checks") -> None:
    probes = report.get("label_alignment_decision_matrix", [])
    contract_ok = (
        report.get("label_alignment_decision_contract")
        == "qpls_estimation::align_labels_exhaustive_v2"
    )
    by_k: dict[int, list[dict[str, Any]]] = defaultdict(list)
    for probe in probes:
        by_k[int(probe.get("k", -1))].append(probe)
    inventory_ok = len(probes) == 9 and set(by_k) == {3, 4, 5}
    for classes in range(3, 6):
        rows = by_k.get(classes, [])
        expected_cases = {
            f"k{classes}-nonidentity-mutual-majority": (False, True),
            f"k{classes}-ambiguous": (True, False),
            f"k{classes}-unique-nonmajority": (False, False),
        }
        observed_cases = {row.get("case_id") for row in rows}
        valid = (
            contract_ok
            and inventory_ok
            and len(rows) == 3
            and observed_cases == set(expected_cases)
        )
        details = []
        for row in rows:
            case_id = row.get("case_id")
            reference = [int(value) for value in row.get("reference", [])]
            candidate = [int(value) for value in row.get("candidate", [])]
            sut = row.get("sut_alignment", {})
            try:
                independent = exhaustive_alignment_from_partitions(reference, candidate, classes)
                retained_mapping = tuple(int(value) for value in sut.get("candidate_to_reference", []))
                retained_majority = independent["mutual_majority_by_mapping"].get(
                    ",".join(str(value) for value in retained_mapping)
                )
                retained_valid, retained_k, retained_n = reconstruct_label_alignment(sut)
                expected_ambiguous, expected_majority = expected_cases[case_id]
                row_ok = (
                    retained_valid
                    and retained_k == classes
                    and retained_n == len(reference)
                    and sut.get("overlap") == independent["overlap"]
                    and independent["permutations_evaluated"] == math.factorial(classes)
                    and independent["ambiguous"] is expected_ambiguous
                    and retained_majority is expected_majority
                    and sut.get("ambiguous") is expected_ambiguous
                    and sut.get("mutual_majority") is expected_majority
                    and row.get("expected_ambiguous") is expected_ambiguous
                    and row.get("expected_mutual_majority") is expected_majority
                    and (
                        case_id != f"k{classes}-nonidentity-mutual-majority"
                        or list(retained_mapping) != list(range(classes))
                    )
                )
            except (KeyError, TypeError, ValueError) as error:
                row_ok = False
                independent = {"error": f"{type(error).__name__}: {error}"}
            valid &= row_ok
            details.append(
                {
                    "case_id": case_id,
                    "passed": row_ok,
                    "independent": independent,
                    "sut_alignment": sut,
                }
            )
        checks.require(
            f"heterogeneity.label_alignment.k{classes}.exhaustive_k_factorial_decisions",
            valid,
            observed=details,
            expected=(
                f"exactly {math.factorial(classes)} independent mappings with nonidentity-pass, "
                "ambiguous-reject, and unique-nonmajority-reject decisions"
            ),
        )


def type7(values: Iterable[float], probability: float) -> float:
    ordered = sorted(float(value) for value in values)
    position = (len(ordered) - 1) * min(1.0, max(0.0, probability))
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return ordered[lower]
    fraction = position - lower
    return ordered[lower] * (1.0 - fraction) + ordered[upper] * fraction


class Checks:
    def __init__(self) -> None:
        self.rows: list[dict[str, Any]] = []
        self.coverage_gaps: list[dict[str, str]] = []

    def require(
        self,
        check_id: str,
        passed: bool,
        *,
        observed: Any = None,
        expected: Any = None,
        detail: str = "",
    ) -> None:
        self.rows.append(
            {
                "check_id": check_id,
                "status": "passed" if passed else "failed",
                "observed": observed,
                "expected": expected,
                "detail": detail,
            }
        )

    def gap(self, check_id: str, reason: str) -> None:
        self.coverage_gaps.append({"check_id": check_id, "status": "pending", "reason": reason})

    @property
    def passed(self) -> bool:
        return all(row["status"] == "passed" for row in self.rows)


def choose2(value: int) -> int:
    return value * (value - 1) // 2


def adjusted_rand_index(reference: list[int], candidate: list[int]) -> float:
    if len(reference) != len(candidate) or not reference:
        raise ValueError("ARI requires equal nonempty partitions")
    cells = Counter(zip(reference, candidate, strict=True))
    reference_counts = Counter(reference)
    candidate_counts = Counter(candidate)
    sum_cells = sum(choose2(value) for value in cells.values())
    sum_reference = sum(choose2(value) for value in reference_counts.values())
    sum_candidate = sum(choose2(value) for value in candidate_counts.values())
    total_pairs = choose2(len(reference))
    expected = sum_reference * sum_candidate / total_pairs
    maximum = 0.5 * (sum_reference + sum_candidate)
    return 1.0 if close(maximum, expected) else (sum_cells - expected) / (maximum - expected)


def fimix_result(cell: dict[str, Any], k: int = 2) -> dict[str, Any] | None:
    matches = [row["result"] for row in cell["evidence"]["fimix"] if row["k"] == k]
    if len(matches) > 1:
        raise ValueError(f"duplicate FIMIX K={k} evidence")
    return matches[0] if matches else None


def pos_result(cell: dict[str, Any], k: int = 2) -> dict[str, Any] | None:
    matches = [row["result"] for row in cell["evidence"]["pos"] if row["k"] == k]
    if len(matches) > 1:
        raise ValueError(f"duplicate POS K={k} evidence")
    return matches[0] if matches else None


def raw_fimix_input(cell: dict[str, Any]) -> dict[str, Any]:
    receipts = cell["evidence"]["raw_preparation"]
    if len(receipts) != 1 or "fimix_input" not in receipts[0]:
        raise ValueError("cell omitted its production pooled standardized FIMIX input")
    return receipts[0]["fimix_input"]


def logsumexp(values: list[float]) -> float:
    maximum = max(values)
    return maximum + math.log(sum(math.exp(value - maximum) for value in values))


def recompute_fimix_likelihood_and_posteriors(
    cell: dict[str, Any], result: dict[str, Any]
) -> tuple[float, list[list[float]]]:
    prepared = raw_fimix_input(cell)
    equations = prepared["equations"]
    observations = int(result["observations"])
    if prepared["metric"]["observation_count"] != observations:
        raise ValueError("prepared metric and FIMIX result observation counts differ")
    classes = result["classes"]
    class_equations = [
        {equation["equation_id"]: equation for equation in class_row["equations"]}
        for class_row in classes
    ]
    rows: list[list[float]] = []
    likelihood = 0.0
    log_two_pi = math.log(2.0 * math.pi)
    for observation in range(observations):
        components = []
        for class_index, class_row in enumerate(classes):
            component = math.log(float(class_row["proportion"]))
            for equation in equations:
                estimate = class_equations[class_index][equation["equation_id"]]
                coefficients = {
                    row["parameter_id"]: float(row["estimate"])
                    for row in estimate["coefficients"]
                }
                predicted = coefficients.get("(intercept)", 0.0)
                predicted += sum(
                    coefficients[predictor] * float(value)
                    for predictor, value in zip(
                        equation["predictor_ids"], equation["design"][observation], strict=True
                    )
                )
                residual = float(equation["outcome"][observation]) - predicted
                variance = float(estimate["residual_variance"])
                component += -0.5 * (log_two_pi + math.log(variance) + residual * residual / variance)
            components.append(component)
        row_likelihood = logsumexp(components)
        likelihood += row_likelihood
        rows.append([math.exp(value - row_likelihood) for value in components])
    return likelihood, rows


def validate_fimix_raw_likelihood(
    cell: dict[str, Any], result: dict[str, Any], checks: Checks, prefix: str
) -> None:
    likelihood, posteriors = recompute_fimix_likelihood_and_posteriors(cell, result)
    posterior_difference = max(
        abs(expected - observed)
        for expected_row, observed_row in zip(posteriors, result["posteriors"], strict=True)
        for expected, observed in zip(expected_row, observed_row, strict=True)
    )
    parameter_count = int(result["criteria"]["parameter_count"])
    observations = int(result["observations"])
    deviance = -2.0 * likelihood
    log_n = math.log(observations)
    criteria = {
        "aic": deviance + 2.0 * parameter_count,
        "aic3": deviance + 3.0 * parameter_count,
        "aic4": deviance + 4.0 * parameter_count,
        "bic": deviance + parameter_count * log_n,
        "caic": deviance + parameter_count * (log_n + 1.0),
        "hq": deviance + 2.0 * parameter_count * math.log(log_n),
    }
    checks.require(
        f"{prefix}.raw_design_logsumexp_likelihood_posteriors",
        raw_fimix_input(cell)["metric"]["source_sha256"]
        == result["metric"]["source_sha256"]
        and close(likelihood, float(result["log_likelihood"]), 2.0e-8)
        and posterior_difference <= 2.0e-8
        and all(close(result["criteria"][key], value, 2.0e-8) for key, value in criteria.items()),
        observed={
            "recomputed_log_likelihood": likelihood,
            "reported_log_likelihood": result["log_likelihood"],
            "maximum_posterior_difference": posterior_difference,
            "criteria_from_recomputed_likelihood": criteria,
        },
        expected="independent Gaussian-mixture log-sum-exp likelihood and complete posterior matrix",
    )


def validate_raw_authority(cell: dict[str, Any], checks: Checks, prefix: str) -> None:
    receipt = cell["compiler_receipt"]
    plan = cell["compiled_plan"]
    raw = cell["evidence"]["raw_preparation"]
    compiler_hashes = [
        receipt.get("model_scientific_sha256", ""),
        receipt.get("plan_sha256", ""),
        receipt.get("analytical_identity_sha256", ""),
    ]
    raw_value = raw[0] if len(raw) == 1 else {}
    fimix_input = raw_value.get("fimix_input", {})
    metric = fimix_input.get("metric", {})
    source_tokens = raw_value.get("source_row_tokens", [])
    observations = len(source_tokens)
    equations = fimix_input.get("equations", [])
    equation_ids = [equation.get("equation_id") for equation in equations]
    equation_contract = bool(equations) and len(set(equation_ids)) == len(equation_ids)
    for equation in equations:
        predictors = equation.get("predictor_ids", [])
        design = equation.get("design", [])
        outcome = equation.get("outcome", [])
        equation_contract &= (
            isinstance(equation.get("equation_id"), str)
            and bool(equation["equation_id"].strip())
            and isinstance(equation.get("outcome_id"), str)
            and bool(equation["outcome_id"].strip())
            and bool(predictors)
            and len(set(predictors)) == len(predictors)
            and all(isinstance(value, str) and value.strip() and value != "(intercept)" for value in predictors)
            and len(design) == observations
            and len(outcome) == observations
            and all(len(row) == len(predictors) for row in design)
            and all(math.isfinite(float(value)) for row in design for value in row)
            and all(math.isfinite(float(value)) for value in outcome)
        )
    raw_hashes = [
        raw_value.get("general_sem_plan_sha256", ""),
        raw_value.get("pooled_metric_sha256", ""),
        metric.get("source_sha256", ""),
    ]
    checks.require(
        f"{prefix}.production_raw_authority",
        receipt.get("target") == "pls_heterogeneity_v2"
        and plan.get("kind") == "pls_heterogeneity_v2"
        and len(raw) == 1
        and raw_value.get("method_version") == "qpls.heterogeneity.raw-preparation.v2"
        and raw_value.get("unique_analysis_positions") is True
        and raw_value.get("omitted_source_rows") == 0
        and observations == 400
        and len(set(source_tokens)) == observations
        and metric.get("observation_count") == observations
        and metric.get("scores_standardized_once_on_pooled_rows") is True
        and raw_value.get("pooled_metric_sha256") == metric.get("source_sha256")
        and equation_contract
        and all(is_sha256(value) for value in compiler_hashes + raw_hashes)
        and is_dataset_fingerprint(receipt.get("dataset_fingerprint", "")),
        observed={
            "target": receipt.get("target"),
            "plan": plan.get("kind"),
            "raw_receipts": len(raw),
            "source_rows": observations,
            "unique_source_rows": len(set(source_tokens)),
            "equations": len(equations),
        },
        expected="compiled Recipe V4 raw-scoring authority for all 400 rows",
    )


def validate_fimix_multistart_reproducibility(
    result: dict[str, Any], checks: Checks, prefix: str
) -> None:
    observations = int(result["observations"])
    classes = len(result["classes"])
    evidence = result["multistart_evidence"]
    receipts = evidence["completed_starts"]
    diagnostics = {
        int(row["start_index"]): row for row in result["starts"] if row["converged"]
    }
    receipt_indices = [int(row["start_index"]) for row in receipts]
    selected_index = int(result["selected_start_index"])
    selected_signatures = [
        [
            float(coefficient["estimate"])
            for equation in class_row["equations"]
            for coefficient in equation["coefficients"]
        ]
        for class_row in result["classes"]
    ]
    widths = [len(row) for row in selected_signatures]
    shape_and_digest_ok = (
        evidence["schema_version"] == 1
        and int(evidence["selected_start_index"]) == selected_index
        and int(evidence["observations"]) == observations
        and int(evidence["classes"]) == classes
        and int(evidence["required_reproducing_starts"])
        == int(result["stability"]["required_reproducing_starts"])
        and receipt_indices
        and receipt_indices[0] == selected_index
        and len(receipt_indices) == len(set(receipt_indices)) == len(diagnostics)
        and set(receipt_indices) == set(diagnostics)
        and all(width > 0 for width in widths)
    )
    normalized: list[dict[str, Any]] = []
    for receipt in receipts:
        start_index = int(receipt["start_index"])
        likelihood = float(receipt["final_log_likelihood"])
        assignments = [int(value) for value in receipt["canonical_hard_assignments"]]
        signatures = [
            [float(value) for value in row]
            for row in receipt["canonical_coefficient_signatures"]
        ]
        posteriors = [
            [float(value) for value in row] for row in receipt["canonical_posteriors"]
        ]
        receipt_ok = (
            start_index in diagnostics
            and struct.pack("<d", likelihood)
            == struct.pack("<d", float(diagnostics[start_index]["final_log_likelihood"]))
            and len(assignments) == observations
            and all(0 <= value < classes for value in assignments)
            and len(signatures) == classes
            and all(
                len(row) == widths[index] and all(math.isfinite(value) for value in row)
                for index, row in enumerate(signatures)
            )
            and len(posteriors) == observations
            and all(
                len(row) == classes
                and all(math.isfinite(value) and 0.0 <= value <= 1.0 for value in row)
                and close(sum(row), 1.0, 1.0e-10)
                for row in posteriors
            )
            and receipt["partition_sha256"] == multistart_partition_sha256(assignments)
            and receipt["coefficient_sha256"]
            == multistart_matrix_sha256(MULTISTART_COEFFICIENT_DOMAIN, signatures)
            and receipt["posterior_sha256"]
            == multistart_matrix_sha256(MULTISTART_POSTERIOR_DOMAIN, posteriors)
            and receipt["fit_statistic_sha256"]
            == multistart_fit_statistic_sha256(likelihood)
        )
        shape_and_digest_ok &= receipt_ok
        normalized.append(
            {
                "start_index": start_index,
                "fit": likelihood,
                "assignments": assignments,
                "signatures": signatures,
                "posteriors": posteriors,
            }
        )
    selected = normalized[0]
    shape_and_digest_ok &= (
        selected["assignments"] == [int(value) for value in result["hard_assignments"]]
        and selected["signatures"] == selected_signatures
        and selected["posteriors"]
        == [[float(value) for value in row] for row in result["posteriors"]]
        and struct.pack("<d", selected["fit"])
        == struct.pack("<d", float(result["log_likelihood"]))
    )
    relative_tolerance = float(evidence["relative_log_likelihood_tolerance"])
    coefficient_tolerance = float(evidence["maximum_coefficient_difference_tolerance"])
    posterior_tolerance = float(evidence["mean_posterior_difference_tolerance"])
    tolerance_identity_ok = (
        close(relative_tolerance, 1.0e-8, 1.0e-15)
        and close(coefficient_tolerance, 1.0e-6, 1.0e-15)
        and close(posterior_tolerance, 1.0e-4, 1.0e-15)
    )
    replayed = [selected_index]
    maximum_coefficient_difference = 0.0
    maximum_posterior_difference = 0.0
    for candidate in normalized[1:]:
        relative_difference = abs(candidate["fit"] - selected["fit"]) / (
            1.0 + abs(selected["fit"])
        )
        if relative_difference > relative_tolerance:
            continue
        mapping, _, ambiguous = multistart_alignment(
            selected["assignments"], candidate["assignments"], classes
        )
        if ambiguous:
            continue
        coefficient_difference = max(
            abs(reference - observed)
            for candidate_class, reference_class in enumerate(mapping)
            for reference, observed in zip(
                selected["signatures"][reference_class],
                candidate["signatures"][candidate_class],
                strict=True,
            )
        )
        posterior_difference = sum(
            abs(reference_row[reference_class] - candidate_row[candidate_class])
            for reference_row, candidate_row in zip(
                selected["posteriors"], candidate["posteriors"], strict=True
            )
            for candidate_class, reference_class in enumerate(mapping)
        ) / (observations * classes)
        if coefficient_difference <= coefficient_tolerance and posterior_difference <= posterior_tolerance:
            replayed.append(candidate["start_index"])
            maximum_coefficient_difference = max(
                maximum_coefficient_difference, coefficient_difference
            )
            maximum_posterior_difference = max(
                maximum_posterior_difference, posterior_difference
            )
    stability = result["stability"]
    replay_ok = (
        replayed == [int(value) for value in stability["reproducing_start_indices"]]
        and close(
            maximum_coefficient_difference,
            float(stability["maximum_aligned_coefficient_difference"]),
            1.0e-12,
        )
        and close(
            maximum_posterior_difference,
            float(stability["maximum_aligned_mean_posterior_difference"]),
            1.0e-12,
        )
        and bool(stability["stable"])
        is (len(replayed) >= int(evidence["required_reproducing_starts"]))
    )
    checks.require(
        f"{prefix}.independent_multistart_reproduction",
        shape_and_digest_ok and tolerance_identity_ok and replay_ok,
        observed={
            "completed_receipts": len(receipts),
            "replayed_reproducing_starts": replayed,
            "reported_reproducing_starts": stability["reproducing_start_indices"],
            "maximum_coefficient_difference": maximum_coefficient_difference,
            "mean_posterior_difference": maximum_posterior_difference,
        },
        expected="all completed starts, independently rebuilt digests, exhaustive alignment, and three frozen tolerances",
    )


def validate_fimix_math(
    result: dict[str, Any], checks: Checks, prefix: str, require_stable: bool = True
) -> None:
    observations = int(result["observations"])
    classes = len(result["classes"])
    posteriors = result["posteriors"]
    posterior_ok = (
        len(posteriors) == observations
        and all(len(row) == classes for row in posteriors)
        and all(all(0.0 <= value <= 1.0 for value in row) and close(sum(row), 1.0) for row in posteriors)
    )
    posterior_sizes = [sum(row[index] for row in posteriors) for index in range(classes)]
    class_ok = close(sum(row["proportion"] for row in result["classes"]), 1.0)
    class_ok &= all(
        close(row["effective_observations"], posterior_sizes[index])
        and close(row["proportion"], posterior_sizes[index] / observations)
        for index, row in enumerate(result["classes"])
    )
    checks.require(
        f"{prefix}.posterior_probability_contract",
        result["method_version"] == FIMIX and posterior_ok and class_ok,
        observed={"rows": len(posteriors), "classes": classes, "posterior_sizes": posterior_sizes},
        expected="complete N-by-K posterior matrix with rows/proportions summing to one",
    )
    criteria = result["criteria"]
    parameter_count = int(criteria["parameter_count"])
    per_class_parameters = sum(
        len(equation["coefficients"]) + 1 for equation in result["classes"][0]["equations"]
    )
    independently_counted_parameters = classes * per_class_parameters + classes - 1
    deviance = -2.0 * result["log_likelihood"]
    log_n = math.log(observations)
    expected = {
        "aic": deviance + 2.0 * parameter_count,
        "aic3": deviance + 3.0 * parameter_count,
        "aic4": deviance + 4.0 * parameter_count,
        "bic": deviance + parameter_count * log_n,
        "caic": deviance + parameter_count * (log_n + 1.0),
        "hq": deviance + 2.0 * parameter_count * math.log(log_n),
    }
    checks.require(
        f"{prefix}.optimized_likelihood_information_criteria",
        parameter_count == independently_counted_parameters
        and all(close(criteria[key], value) for key, value in expected.items()),
        observed={"parameter_count": parameter_count, **{key: criteria[key] for key in expected}},
        expected={"parameter_count": independently_counted_parameters, **expected},
        detail="independent criteria calculation from optimized likelihood and full p",
    )
    entropy = -sum(
        probability * math.log(probability)
        for row in posteriors
        for probability in row
        if probability > 0.0
    )
    normalized = 1.0 - entropy / (observations * math.log(classes))
    checks.require(
        f"{prefix}.posterior_entropy",
        close(entropy, result["entropy"]["raw"])
        and close(normalized, result["entropy"]["normalized_certainty"]),
        observed=result["entropy"],
        expected={"raw": entropy, "normalized_certainty": normalized},
    )
    converged = [row for row in result["starts"] if row["converged"]]
    monotone = True
    consecutive_contract = True
    for start in converged:
        trace = start["trace"]
        monotone &= bool(trace) and all(
            right["log_likelihood"] + 1.0e-10 >= left["log_likelihood"]
            for left, right in zip(trace, trace[1:])
        )
        consecutive_contract &= start["iterations"] == len(trace) - 1 or start["iterations"] == len(trace)
        consecutive_contract &= math.isfinite(start["maximum_likelihood_decrease"])
        consecutive_contract &= start["maximum_likelihood_decrease"] >= 0.0
    stability = result["stability"]
    validate_fimix_multistart_reproducibility(result, checks, prefix)
    checks.require(
        f"{prefix}.thirty_start_monotone_stability",
        len(result["starts"]) == 30
        and len(converged) >= 2
        and monotone
        and consecutive_contract
        and (not require_stable or stability["stable"] is True)
        and (
            not require_stable
            or len(stability["reproducing_start_indices"])
            >= stability["required_reproducing_starts"]
            >= 2
        ),
        observed={
            "starts": len(result["starts"]),
            "converged": len(converged),
            "reproducing": len(stability["reproducing_start_indices"]),
            "stable": stability["stable"],
        },
        expected="30 starts, monotone traces, at least two reproducing optima",
    )
    largest_predictor_count = max(
        sum(coefficient["parameter_id"] != "(intercept)" for coefficient in equation["coefficients"])
        for class_row in result["classes"]
        for equation in class_row["equations"]
    )
    dynamic_minimum = max(20, math.ceil(0.05 * observations), largest_predictor_count + 2)
    checks.require(
        f"{prefix}.dynamic_class_minimum",
        result["minimum_effective_class_size"] >= dynamic_minimum
        and all(row["proportion"] >= 0.05 for row in result["classes"]),
        observed={
            "minimum": result["minimum_effective_class_size"],
            "shares": [row["proportion"] for row in result["classes"]],
        },
        expected={"minimum": dynamic_minimum, "share": 0.05},
    )


def validate_fimix_recovery(report: dict[str, Any], checks: Checks) -> None:
    cells = report["fimix"]["strong_separation_recovery"]
    aris = []
    for index, cell in enumerate(cells):
        prefix = f"fimix.recovery.seed_{cell['config']['seed']}"
        validate_raw_authority(cell, checks, prefix)
        result = fimix_result(cell)
        checks.require(
            f"{prefix}.k2_evidence_present",
            result is not None,
            observed=len(cell["evidence"]["fimix"]),
            expected="one real K=2 FIMIX result",
        )
        if result is None:
            continue
        validate_fimix_math(result, checks, prefix)
        validate_fimix_raw_likelihood(cell, result, checks, prefix)
        ari = adjusted_rand_index(cell["true_classes"], result["hard_assignments"])
        aris.append(ari)
        checks.require(
            f"{prefix}.strong_separation_ari",
            ari >= 0.80,
            observed=ari,
            expected=">= 0.80",
        )
    checks.require(
        "fimix.recovery.median_ari",
        len(aris) == 5
        and len({cell["compiler_receipt"]["dataset_fingerprint"] for cell in cells}) == 5
        and median(aris) >= 0.80,
        observed={
            "aris": aris,
            "median": median(aris) if aris else None,
            "distinct_datasets": len(
                {cell["compiler_receipt"]["dataset_fingerprint"] for cell in cells}
            ),
        },
        expected="five predeclared seeds and median ARI >= 0.80",
    )


def validate_fimix_simulation_matrix(report: dict[str, Any], checks: Checks) -> None:
    section = report["fimix"]
    acceptance = section["simulation_acceptance"]
    specifications = [
        (
            "power_recovery",
            "fimix.simulation.power",
            10,
            acceptance["power_success_ari_minimum"],
            "power",
        ),
        (
            "overlap_recovery",
            "fimix.simulation.overlap",
            5,
            acceptance["overlap_median_ari_minimum"],
            "median",
        ),
        (
            "imbalance_recovery",
            "fimix.simulation.imbalance",
            5,
            acceptance["imbalance_median_ari_minimum"],
            "median",
        ),
        (
            "nonnormal_recovery",
            "fimix.simulation.nonnormal",
            5,
            acceptance["nonnormal_median_ari_minimum"],
            "median",
        ),
    ]
    for key, prefix, expected_count, threshold, aggregation in specifications:
        cells = section[key]
        aris = []
        scenario_evidence = True
        for cell in cells:
            validate_raw_authority(cell, checks, f"{prefix}.seed_{cell['config']['seed']}")
            result = fimix_result(cell)
            if result is None:
                continue
            validate_fimix_math(result, checks, f"{prefix}.seed_{cell['config']['seed']}")
            validate_fimix_raw_likelihood(
                cell, result, checks, f"{prefix}.seed_{cell['config']['seed']}"
            )
            aris.append(adjusted_rand_index(cell["true_classes"], result["hard_assignments"]))
            if key == "imbalance_recovery":
                shares = Counter(cell["true_classes"])
                scenario_evidence &= sorted(shares.values()) == [100, 300]
            if key == "nonnormal_recovery":
                predictor = [row[0] for row in raw_fimix_input(cell)["equations"][0]["design"]]
                mean = sum(predictor) / len(predictor)
                second = sum((value - mean) ** 2 for value in predictor) / len(predictor)
                fourth = sum((value - mean) ** 4 for value in predictor) / len(predictor)
                excess_kurtosis = fourth / (second * second) - 3.0
                scenario_evidence &= excess_kurtosis >= 0.50
        distinct = len({cell["compiler_receipt"]["dataset_fingerprint"] for cell in cells})
        if aggregation == "power":
            successes = sum(value >= threshold for value in aris)
            passed = (
                len(cells) == expected_count
                and len(aris) == expected_count
                and successes / expected_count >= acceptance["power_success_rate_minimum"]
            )
            expected = {
                "runs": expected_count,
                "ari_success": threshold,
                "success_rate": acceptance["power_success_rate_minimum"],
            }
        else:
            passed = (
                len(cells) == expected_count
                and len(aris) == expected_count
                and median(aris) >= threshold
            )
            expected = {"runs": expected_count, "median_ari": threshold}
        checks.require(
            f"{prefix}.predeclared_recovery",
            passed and distinct == expected_count and scenario_evidence,
            observed={
                "aris": aris,
                "median": median(aris) if aris else None,
                "distinct_datasets": distinct,
                "scenario_evidence": scenario_evidence,
            },
            expected=expected,
        )


def validate_candidate_table(report: dict[str, Any], checks: Checks) -> None:
    cell = report["fimix"]["candidate_k_table"]
    candidates = cell["analysis"]["candidates"]
    baseline = [row for row in candidates if row["method"]["kind"] == "pooled_baseline_v1"]
    segmented = [row for row in candidates if row["method"]["kind"] == "segmentation"]
    checks.require(
        "fimix.candidate_k_inventory",
        len(baseline) == 1
        and baseline[0]["k"] == 1
        and [row["k"] for row in segmented] == [2, 3, 4, 5]
        and cell["analysis"].get("locked_algorithm") is None
        and cell["analysis"].get("locked_k") is None
        and cell["analysis"].get("inference_lock") is None,
        observed={"baseline": [row["k"] for row in baseline], "segmented": [row["k"] for row in segmented]},
        expected="read-only K=1 plus K=2..5, never auto-selected",
    )
    validate_raw_authority(cell, checks, "fimix.candidate_k")
    for evidence in cell["evidence"]["fimix"]:
        prefix = f"fimix.candidate_k.k{evidence['k']}"
        validate_fimix_math(evidence["result"], checks, prefix)
        validate_fimix_raw_likelihood(cell, evidence["result"], checks, prefix)


def structural_coefficient_spread(result: dict[str, Any]) -> float:
    by_parameter: dict[tuple[str, str], list[float]] = defaultdict(list)
    for class_row in result["classes"]:
        for equation in class_row["equations"]:
            for coefficient in equation["coefficients"]:
                if coefficient["parameter_id"] != "(intercept)":
                    by_parameter[(equation["equation_id"], coefficient["parameter_id"])].append(
                        coefficient["estimate"]
                    )
    return max(max(values) - min(values) for values in by_parameter.values())


def validate_homogeneous_null(report: dict[str, Any], checks: Checks) -> None:
    cell = report["fimix"]["homogeneous_null"]
    result = fimix_result(cell)
    # A failed/unstable K=2 candidate is conservative under the homogeneous null.
    if result is None:
        candidate = next(
            row
            for row in cell["analysis"]["candidates"]
            if row["method"].get("algorithm") == "fimix_pls_v2" and row["k"] == 2
        )
        conservative = candidate["state"] in {"failed", "unstable", "ineligible"}
        spread = None
    else:
        spread = structural_coefficient_spread(result)
        validate_fimix_raw_likelihood(cell, result, checks, "fimix.homogeneous_null")
        conservative = spread <= 0.35
        validate_fimix_math(result, checks, "fimix.homogeneous_null", require_stable=False)
    checks.require(
        "fimix.homogeneous_null.no_spurious_structural_separation",
        conservative
        and cell["analysis"].get("locked_algorithm") is None
        and cell["analysis"].get("locked_k") is None,
        observed={"structural_coefficient_spread": spread, "locked_k": cell["analysis"].get("locked_k")},
        expected="no auto-selection and K=2 coefficient spread <= 0.35 (or failed/unstable)",
    )


def interaction_count(cell: dict[str, Any]) -> int:
    return len(cell["compiled_plan"]["interactions"])


def validate_pos_multistart_reproducibility(
    result: dict[str, Any], checks: Checks, prefix: str
) -> None:
    observations = int(result["observations"])
    segments = len(result["segments"])
    evidence = result["multistart_evidence"]
    receipts = evidence["completed_starts"]
    diagnostics = {
        int(row["start_index"]): row for row in result["starts"] if row["completed"]
    }
    receipt_indices = [int(row["start_index"]) for row in receipts]
    selected_index = int(result["selected_start_index"])
    selected_signatures = [
        [float(value) for value in segment["fit"]["parameter_signature"]]
        for segment in result["segments"]
    ]
    widths = [len(row) for row in selected_signatures]
    shape_and_digest_ok = (
        evidence["schema_version"] == 1
        and int(evidence["selected_start_index"]) == selected_index
        and int(evidence["observations"]) == observations
        and int(evidence["segments"]) == segments
        and int(evidence["required_reproducing_starts"]) >= 2
        and receipt_indices
        and receipt_indices[0] == selected_index
        and len(receipt_indices) == len(set(receipt_indices)) == len(diagnostics)
        and set(receipt_indices) == set(diagnostics)
        and all(width > 0 for width in widths)
    )
    normalized: list[dict[str, Any]] = []
    for receipt in receipts:
        start_index = int(receipt["start_index"])
        objective = float(receipt["final_objective"])
        assignments = [int(value) for value in receipt["canonical_assignments"]]
        signatures = [
            [float(value) for value in row]
            for row in receipt["canonical_parameter_signatures"]
        ]
        receipt_ok = (
            start_index in diagnostics
            and struct.pack("<d", objective)
            == struct.pack("<d", float(diagnostics[start_index]["final_objective"]))
            and len(assignments) == observations
            and all(0 <= value < segments for value in assignments)
            and len(signatures) == segments
            and all(
                len(row) == widths[index] and all(math.isfinite(value) for value in row)
                for index, row in enumerate(signatures)
            )
            and receipt["partition_sha256"] == multistart_partition_sha256(assignments)
            and receipt["parameter_sha256"]
            == multistart_matrix_sha256(MULTISTART_PARAMETER_DOMAIN, signatures)
            and receipt["fit_statistic_sha256"]
            == multistart_fit_statistic_sha256(objective)
        )
        shape_and_digest_ok &= receipt_ok
        normalized.append(
            {
                "start_index": start_index,
                "fit": objective,
                "assignments": assignments,
                "signatures": signatures,
            }
        )
    selected = normalized[0]
    shape_and_digest_ok &= (
        selected["assignments"] == [int(value) for value in result["assignments"]]
        and selected["signatures"] == selected_signatures
        and struct.pack("<d", selected["fit"]) == struct.pack("<d", float(result["objective"]))
    )
    tolerance = float(evidence["objective_and_parameter_tolerance"])
    tolerance_identity_ok = close(tolerance, 1.0e-10, 1.0e-15)
    replayed = [selected_index]
    for candidate in normalized[1:]:
        if abs(candidate["fit"] - selected["fit"]) > tolerance:
            continue
        mapping, matched, ambiguous = multistart_alignment(
            selected["assignments"], candidate["assignments"], segments
        )
        maximum_parameter_difference = max(
            abs(reference - observed)
            for candidate_segment, reference_segment in enumerate(mapping)
            for reference, observed in zip(
                selected["signatures"][reference_segment],
                candidate["signatures"][candidate_segment],
                strict=True,
            )
        )
        if not ambiguous and matched == observations and maximum_parameter_difference <= tolerance:
            replayed.append(candidate["start_index"])
    reported = [int(value) for value in result["reproducing_start_indices"]]
    replay_ok = (
        replayed == reported
        and len(replayed) >= int(evidence["required_reproducing_starts"])
    )
    checks.require(
        f"{prefix}.independent_multistart_reproduction",
        shape_and_digest_ok and tolerance_identity_ok and replay_ok,
        observed={
            "completed_receipts": len(receipts),
            "replayed_reproducing_starts": replayed,
            "reported_reproducing_starts": reported,
        },
        expected="all completed starts, independently rebuilt digests, exhaustive exact-partition alignment, objective and parameter tolerances",
    )


def validate_pos_result(
    result: dict[str, Any], checks: Checks, prefix: str, expected_method: str, expected_profile: str
) -> None:
    validate_pos_multistart_reproducibility(result, checks, prefix)
    start_diagnostics_ok = [row["start_index"] for row in result["starts"]] == list(range(10))
    for row in result["starts"]:
        history = [float(value) for value in row["objective_history"]]
        monotone = all(right > left for left, right in zip(history, history[1:]))
        candidate_failures = row["candidate_refit_failures"]
        typed_failures = all(
            0 <= int(failure["observation"]) < int(result["observations"])
            and 0 <= int(failure["source_segment"]) < len(result["segments"])
            and 0 <= int(failure["destination_segment"]) < len(result["segments"])
            and failure["source_segment"] != failure["destination_segment"]
            and isinstance(failure["reason"], str)
            and bool(failure["reason"].strip())
            for failure in candidate_failures
        )
        if row["completed"]:
            row_ok = (
                row["failure_reason"] is None
                and not candidate_failures
                and bool(history)
                and monotone
                and row["accepted_moves"] == len(history) - 1
                and close(float(row["final_objective"]), history[-1])
            )
        else:
            row_ok = (
                isinstance(row["failure_reason"], str)
                and bool(row["failure_reason"].strip())
                and row["final_objective"] is None
                and typed_failures
                and (not history or monotone)
                and row["accepted_moves"] == max(0, len(history) - 1)
            )
        start_diagnostics_ok &= row_ok
    full_receipts = []
    for segment in result["segments"]:
        receipt = segment["fit"]["receipt"]
        full_receipts.append(receipt)
    published = expected_method == POS_PUBLISHED
    receipt_ok = all(
        row["method_version"] == expected_method
        and row["full_segment_pls_refit"]
        and row["measurement_scores_reestimated"]
        and row["score_orientation_reapplied"]
        and (
            published
            or (
                row["interaction_stage_one_refit"]
                and row["interaction_operands_restandardized_within_destination"]
                and row["interaction_products_rebuilt_within_destination"]
                and row["joint_structural_equations_refit"]
            )
        )
        for row in full_receipts
    )
    checks.require(
        f"{prefix}.ten_start_complete_sweeps",
        result["method_version"] == expected_method
        and len(result["starts"]) == 10
        and start_diagnostics_ok
        and len(result["reproducing_start_indices"]) >= 2
        and set(result["reproducing_start_indices"]).issubset(
            {row["start_index"] for row in result["starts"] if row["completed"]}
        )
        and result["objective_definition"]
        == "unweighted_sum_of_all_endogenous_r_squared_over_all_segments"
        and close(result["objective"], sum(row["objective_contribution"] for row in result["segments"])),
        observed={
            "starts": len(result["starts"]),
            "reproducing": result["reproducing_start_indices"],
            "objective": result["objective"],
        },
        expected="ten attempted starts with coherent completed sweeps or typed failures, plus two reproducing canonical partitions",
    )
    scoring = result["scoring_contract"]
    scoring_ok = (
        published
        and scoring == "published_p0_full_segment_pls"
        or not published
        and scoring.get("destination_scored_interactions", {}).get("profile") == expected_profile
    )
    checks.require(
        f"{prefix}.full_refit_scoring_receipts",
        scoring_ok and receipt_ok,
        observed={"scoring_contract": scoring, "receipts": full_receipts},
        expected="publication-faithful P0 or complete destination-scored interaction refit",
    )
    reconstructed_segments = []
    audit_rows = set()
    audit_ok = True
    for segment_index, segment in enumerate(result["segments"]):
        reported_r2 = {
            row["outcome_id"]: float(row["r_squared"])
            for row in segment["fit"]["r_squared"]
        }
        reconstructed_r2 = {}
        for audit in segment["fit"].get("outcome_fit_audits", []):
            source_rows = audit["source_row_indices"]
            observed = [float(value) for value in audit["observed_scores"]]
            fitted = [float(value) for value in audit["fitted_scores"]]
            audit_ok &= len(source_rows) == len(observed) == len(fitted) == segment["observations"]
            audit_ok &= len(set(source_rows)) == len(source_rows)
            audit_ok &= all(result["assignments"][row] == segment_index for row in source_rows)
            audit_ok &= all(math.isfinite(value) for value in observed + fitted)
            audit_rows.update((audit["outcome_id"], row) for row in source_rows)
            residual = sum((actual - predicted) ** 2 for actual, predicted in zip(observed, fitted, strict=True))
            observed_mean = sum(observed) / len(observed)
            total = sum((actual - observed_mean) ** 2 for actual in observed)
            audit_ok &= abs(observed_mean) <= 1.0e-8
            audit_ok &= close(float(audit["observed_mean"]), observed_mean, 1.0e-12)
            audit_ok &= close(
                float(audit["centered_total_sum_of_squares"]), total, 1.0e-12
            )
            audit_ok &= math.isfinite(total) and total > 0.0
            reconstructed_r2[audit["outcome_id"]] = (
                max(0.0, min(1.0, 1.0 - residual / total)) if total > 0.0 else math.nan
            )
        audit_ok &= reconstructed_r2.keys() == reported_r2.keys()
        audit_ok &= all(close(reconstructed_r2[key], reported_r2[key]) for key in reported_r2)
        reconstructed_segments.append(sum(reconstructed_r2.values()))
    expected_audit_rows = {
        (outcome["outcome_id"], row)
        for segment in result["segments"]
        for outcome in segment["fit"]["r_squared"]
        for row in range(result["observations"])
    }
    checks.require(
        f"{prefix}.independent_outcome_r2_objective",
        audit_ok
        and audit_rows == expected_audit_rows
        and all(
            close(observed, segment["objective_contribution"])
            for observed, segment in zip(reconstructed_segments, result["segments"], strict=True)
        )
        and close(sum(reconstructed_segments), result["objective"]),
        observed={
            "segment_objectives": reconstructed_segments,
            "objective": result["objective"],
            "audit_rows": len(audit_rows),
        },
        expected="R-squared from centered standardized observed/fitted score rows and their unweighted segment/outcome sum",
    )


def validate_pos_discoveries(report: dict[str, Any], checks: Checks) -> None:
    acceptance = report["pos"]["recovery_acceptance"]
    cells = [
        (report["pos"]["published_p0_discovery"], "pos.published_p0", POS_PUBLISHED, "p0_structural", 0, acceptance["published_p0_ari_minimum"]),
        (report["pos"]["destination_p2_discovery"], "pos.destination_p2", POS_INTERACTIONS, "p2_multi_two_way", 2, acceptance["destination_p2_ari_minimum"]),
        (report["pos"]["destination_p23_discovery"], "pos.destination_p23", POS_INTERACTIONS, "p23_all_current", 4, acceptance["destination_p23_ari_minimum"]),
    ]
    for cell, prefix, method, profile, interactions, ari_minimum in cells:
        validate_raw_authority(cell, checks, prefix)
        result = pos_result(cell)
        checks.require(
            f"{prefix}.point_evidence_present",
            result is not None and interaction_count(cell) == interactions,
            observed={"pos_results": len(cell["evidence"]["pos"]), "interactions": interaction_count(cell)},
            expected={"pos_results": 1, "interactions": interactions},
        )
        if result is not None:
            validate_pos_result(result, checks, prefix, method, profile)
            ari = adjusted_rand_index(cell["true_classes"], result["assignments"])
            checks.require(
                f"{prefix}.partition_recovery_ari",
                ari >= ari_minimum,
                observed=ari,
                expected=f">= {ari_minimum}",
            )
        fimix = fimix_result(cell)
        checks.require(
            f"{prefix}.same_k_fimix_tandem",
            fimix is not None,
            observed=len(cell["evidence"]["fimix"]),
            expected="same-K FIMIX partition available as tenth POS start authority",
        )
    overlap = report["pos"]["overlap_discovery"]
    validate_raw_authority(overlap, checks, "pos.overlap")
    overlap_result = pos_result(overlap)
    checks.require(
        "pos.overlap.point_evidence_present",
        overlap_result is not None,
        observed=len(overlap["evidence"]["pos"]),
        expected="one published P0 POS overlap result",
    )
    if overlap_result is not None:
        validate_pos_result(
            overlap_result, checks, "pos.overlap", POS_PUBLISHED, "p0_structural"
        )
        overlap_ari = adjusted_rand_index(overlap["true_classes"], overlap_result["assignments"])
        checks.require(
            "pos.overlap.partition_recovery_ari",
            overlap_ari >= acceptance["overlap_ari_minimum"],
            observed=overlap_ari,
            expected=f">= {acceptance['overlap_ari_minimum']}",
        )
    null = report["pos"]["homogeneous_null_discovery"]
    validate_raw_authority(null, checks, "pos.homogeneous_null")
    null_result = pos_result(null)
    if null_result is None:
        null_candidate = next(
            row
            for row in null["analysis"]["candidates"]
            if row["method"].get("algorithm") == "pls_pos_published_v2"
        )
        conservative = null_candidate["state"] in {"failed", "unstable", "ineligible"}
        null_ari = None
    else:
        validate_pos_result(
            null_result, checks, "pos.homogeneous_null", POS_PUBLISHED, "p0_structural"
        )
        null_ari = adjusted_rand_index(null["true_classes"], null_result["assignments"])
        conservative = null_ari <= acceptance["homogeneous_null_ari_maximum"]
    checks.require(
        "pos.homogeneous_null.no_truth_recovery_claim",
        conservative and null["analysis"].get("locked_k") is None,
        observed={"ari": null_ari, "locked_k": null["analysis"].get("locked_k")},
        expected=f"failed/unstable or ARI <= {acceptance['homogeneous_null_ari_maximum']}; never auto-lock",
    )

    validate_pos_k3_through_k5_discoveries(report, checks)


def validate_pos_k3_through_k5_discoveries(report: dict[str, Any], checks: Checks) -> None:
    cells = report["pos"].get("published_p0_k3_through_k5_discovery", [])
    expected_ids = {f"pos-published-p0-k{k}-discovery" for k in range(3, 6)}
    by_id = {cell.get("cell_id"): cell for cell in cells}
    matrix_ok = len(cells) == 3 and set(by_id) == expected_ids
    observed = []
    for classes in range(3, 6):
        cell_id = f"pos-published-p0-k{classes}-discovery"
        cell = by_id.get(cell_id)
        if cell is None:
            checks.require(
                f"pos.published_p0.k{classes}.candidate_profile_algorithm_identity",
                False,
                observed=None,
                expected=f"one production P0 published-POS discovery candidate at K={classes}",
            )
            matrix_ok = False
            continue
        validate_raw_authority(cell, checks, f"pos.published_p0.k{classes}")
        phase = cell["config"]["phase"]
        plan = cell["compiled_plan"]
        result = pos_result(cell, classes)
        candidates = [
            row
            for row in cell["analysis"]["candidates"]
            if row["method"].get("algorithm") == "pls_pos_published_v2" and row["k"] == classes
        ]
        segmented_candidates = [
            row
            for row in cell["analysis"]["candidates"]
            if row["method"].get("kind") == "segmentation"
        ]
        truth_counts = Counter(int(value) for value in cell["true_classes"])
        identity_ok = (
            cell["profile"] == "p0_structural"
            and cell["fixture_id"] == f"heterogeneity-p0-strong-k{classes}"
            and phase.get("kind") == "discovery"
            and phase.get("candidate_k") == [classes]
            and phase.get("algorithms") == ["pls_pos_published_v2"]
            and plan.get("profile") == "p0_structural"
            and plan.get("candidate_k") == [classes]
            and plan.get("algorithms") == ["pls_pos_published_v2"]
            and cell["analysis"].get("locked_algorithm") is None
            and cell["analysis"].get("locked_k") is None
            and len(candidates) == 1
            and len(segmented_candidates) == 1
            and candidates[0]["state"] == "completed"
            and len(cell["evidence"]["pos"]) == 1
            and cell["evidence"]["fimix"] == []
            and result is not None
            and result["method_version"] == POS_PUBLISHED
            and len(result["segments"]) == classes
            and len(truth_counts) == classes
            and set(truth_counts) == set(range(classes))
            and max(truth_counts.values()) - min(truth_counts.values()) <= 1
        )
        checks.require(
            f"pos.published_p0.k{classes}.candidate_profile_algorithm_identity",
            identity_ok,
            observed={
                "cell_id": cell.get("cell_id"),
                "profile": cell.get("profile"),
                "phase": phase,
                "plan_algorithms": plan.get("algorithms"),
                "candidate_states": [row["state"] for row in candidates],
                "result_method": result.get("method_version") if result else None,
                "result_segments": len(result["segments"]) if result else None,
                "truth_counts": dict(sorted(truth_counts.items())),
            },
            expected=f"exact production P0/qpls.pls-pos.published.v2 candidate identity at K={classes}",
        )
        matrix_ok &= identity_ok
        if result is not None:
            validate_pos_result(
                result,
                checks,
                f"pos.published_p0.k{classes}",
                POS_PUBLISHED,
                "p0_structural",
            )
        observed.append({"k": classes, "identity_ok": identity_ok})

    k2 = report["pos"]["published_p0_discovery"]
    k2_result = pos_result(k2, 2)
    matrix_ok &= (
        k2["profile"] == "p0_structural"
        and k2_result is not None
        and k2_result["method_version"] == POS_PUBLISHED
        and len(k2_result["segments"]) == 2
    )
    checks.require(
        "pos.published_p0.k2_through_k5_candidate_profile_algorithm_identity",
        matrix_ok,
        observed=observed,
        expected="completed production qpls.pls-pos.published.v2 P0 candidates at each exact K=2,3,4,5 identity",
    )


def validate_scientific_interaction_targets(cell: dict[str, Any], checks: Checks, prefix: str) -> None:
    kinds = Counter(row["parameter"]["target_kind"] for row in cell["analysis"]["parameters"])
    profile = cell["profile"]
    expected = {
        "class_specific_scientific_gamma",
        "class_specific_fixed_simple_slope",
    }
    if profile == "p23_all_current":
        expected |= {
            "class_specific_scientific_delta",
            "class_specific_fixed_three_way_simple_slope",
        }
    checks.require(
        f"{prefix}.scientific_gamma_delta_slope_targets",
        expected.issubset(kinds),
        observed=kinds,
        expected=sorted(expected),
    )


def validate_bootstrap_cell(cell: dict[str, Any], checks: Checks, prefix: str) -> None:
    evidence = cell["evidence"]["bootstrap"]
    analysis_ledger = cell["analysis"].get("bootstrap_ledger")
    checks.require(
        f"{prefix}.bootstrap_evidence_present",
        len(evidence) == 1 and analysis_ledger is not None,
        observed={"evidence": len(evidence), "ledger": analysis_ledger is not None},
        expected="one fixed-K evidence ledger",
    )
    if len(evidence) != 1 or analysis_ledger is None:
        return
    prepared = evidence[0]
    entries = prepared["entries"]
    targets = prepared["targets"]
    usable = [row for row in entries if row["status"] == "usable"]
    target_ids = [row["target_id"] for row in targets]
    target_vectors_complete = all(len(row.get("estimates", [])) == len(entries) for row in targets)
    retained_k: set[int] = set()
    retained_observations: set[int] = set()
    alignment_ok = (
        bool(targets)
        and target_vectors_complete
        and len(set(target_ids)) == len(target_ids)
        and all(target_ids)
    )
    alignment_details = []
    for position, entry in enumerate(entries):
        estimates = (
            [target["estimates"][position] for target in targets]
            if target_vectors_complete
            else []
        )
        if entry["status"] == "usable":
            alignment = entry.get("label_alignment")
            reconstructed, classes, observations = (
                reconstruct_label_alignment(alignment) if alignment is not None else (False, 0, 0)
            )
            retained_k.add(classes)
            retained_observations.add(observations)
            finite_values = [float(value) for value in estimates if value is not None]
            complete_targets = len(finite_values) == len(estimates) and all(
                math.isfinite(value) for value in finite_values
            )
            digest = target_payload_sha256(finite_values) if complete_targets else None
            row_ok = (
                reconstructed
                and alignment["ambiguous"] is False
                and alignment["mutual_majority"] is True
                and complete_targets
                and entry.get("target_payload_sha256") == digest
                and math.isfinite(float(entry.get("fit_statistic", math.nan)))
                and entry.get("failure_reason") is None
            )
        else:
            row_ok = (
                all(value is None for value in estimates)
                and entry.get("fit_statistic") is None
                and entry.get("label_alignment") is None
                and entry.get("target_payload_sha256") is None
                and isinstance(entry.get("failure_reason"), str)
                and bool(entry["failure_reason"].strip())
            )
        alignment_ok &= row_ok
        if not row_ok:
            alignment_details.append({"replicate": position, "status": entry["status"]})
    expected_k = int(cell["analysis"]["locked_k"])
    expected_observations = int(raw_fimix_input(cell)["metric"]["observation_count"])
    alignment_ok &= retained_k == {expected_k} and retained_observations == {expected_observations}
    checks.require(
        f"{prefix}.fixed_k_label_aligned_shared_ledger",
        len(entries) == 500
        and [row["replicate_index"] for row in entries] == list(range(500))
        and len(usable) >= 450
        and alignment_ok
        and prepared["complete_stage_one_and_segmentation_rerun"]
        and prepared["exhaustive_label_alignment_applied"]
        and analysis_ledger["requested"] == 500
        and analysis_ledger["usable"] == len(usable)
        and analysis_ledger["minimum_required"] == 450
        and analysis_ledger["complete"] is True,
        observed={
            "entries": len(entries),
            "usable": len(usable),
            "ledger": analysis_ledger,
            "retained_k": sorted(retained_k),
            "retained_observations": sorted(retained_observations),
            "invalid_evidence": alignment_details[:10],
        },
        expected="500 no-retry draws, >=450 usable, recomputed target digests, and exhaustive overlap-derived unambiguous majority alignment",
    )
    validity_bitmap = [row["status"] == "usable" for row in entries]
    bitmap_ok = target_vectors_complete and all(
        len(target["estimates"]) == 500
        and all((estimate is not None) == validity_bitmap[index] for index, estimate in enumerate(target["estimates"]))
        for target in targets
    )
    public_parameters = {
        row["parameter"]["target_id"]: row["parameter"] for row in cell["analysis"]["parameters"]
    }
    interval_ok = bitmap_ok and set(public_parameters) == {row["target_id"] for row in targets}
    for target in targets:
        draws = [value for value in target["estimates"] if value is not None]
        expected_lower = type7(draws, 0.025)
        expected_upper = type7(draws, 0.975)
        interval = public_parameters.get(target["target_id"], {}).get("interval")
        interval_ok &= (
            interval is not None
            and interval["family"] == "type_7_two_sided_percentile"
            and close(interval["lower"], expected_lower)
            and close(interval["upper"], expected_upper)
        )
    target_draws = {row["target_id"]: row["estimates"] for row in targets}
    for contrast in cell["analysis"]["contrasts"]:
        left_id = f"class_{contrast['left_class_id']}:{contrast['target_id']}"
        right_id = f"class_{contrast['right_class_id']}:{contrast['target_id']}"
        left = target_draws.get(left_id)
        right = target_draws.get(right_id)
        if left is None or right is None:
            interval_ok = False
            continue
        draws = [
            left_value - right_value
            for index, (left_value, right_value) in enumerate(zip(left, right, strict=True))
            if validity_bitmap[index] and left_value is not None and right_value is not None
        ]
        interval = contrast.get("interval")
        interval_ok &= (
            interval is not None
            and interval["family"] == "type_7_two_sided_percentile"
            and close(interval["lower"], type7(draws, 0.025))
            and close(interval["upper"], type7(draws, 0.975))
        )
    checks.require(
        f"{prefix}.shared_validity_bitmap_type7",
        interval_ok,
        observed={"targets": len(targets), "usable": len(usable)},
        expected="one validity bitmap and independently reproduced Type-7 intervals for every target",
    )


def validate_common_metric(cell: dict[str, Any], checks: Checks, prefix: str, should_pass: bool) -> None:
    evidence = cell["evidence"]["common_metric"]
    checks.require(
        f"{prefix}.evidence_present",
        len(evidence) == 1,
        observed=len(evidence),
        expected="one raw pooled common-metric/MICOM receipt",
    )
    if len(evidence) != 1:
        return
    value = evidence[0]
    gate = value["gate_result"]
    gate_input = value["gate_input"]
    expected_status = "passed" if should_pass else "descriptive_only"
    output = cell["analysis"]
    segments = int(gate_input["segments"])
    expected_pairs = {
        (left, right) for left in range(segments) for right in range(left + 1, segments)
    }
    required_constructs = gate_input["required_construct_ids"]
    required_set = set(required_constructs)
    evidence_by_construct = {
        row["construct_id"]: row for row in gate_input.get("evidence", [])
    }
    micom_by_pair = {
        (int(row["pair"]["group_a"]), int(row["pair"]["group_b"])): row
        for row in value["micom_pairs"]
    }
    exact_inventory = (
        2 <= segments <= 5
        and bool(required_constructs)
        and len(required_set) == len(required_constructs)
        and set(gate["required_construct_ids"]) == required_set
        and len(gate["required_construct_ids"]) == len(required_set)
        and set(evidence_by_construct) == required_set
        and len(gate_input.get("evidence", [])) == len(required_set)
        and set(micom_by_pair) == expected_pairs
        and len(value["micom_pairs"]) == len(expected_pairs)
    )
    for construct_id, construct_evidence in evidence_by_construct.items():
        compositional_pairs = {
            (int(row["left_segment"]), int(row["right_segment"]))
            for row in construct_evidence["compositional_invariance"]
        }
        step3_pairs = {
            (int(row["left_segment"]), int(row["right_segment"]))
            for row in construct_evidence["step3_equality"]
        }
        exact_inventory &= (
            compositional_pairs == expected_pairs
            and len(construct_evidence["compositional_invariance"]) == len(expected_pairs)
            and step3_pairs == expected_pairs
            and len(construct_evidence["step3_equality"]) == len(expected_pairs)
        )
    for pair_id, pair in micom_by_pair.items():
        exact_inventory &= {row["construct_id"] for row in pair["constructs"]} == required_set
        exact_inventory &= len(pair["constructs"]) == len(required_set)
        for construct in pair["constructs"]:
            construct_evidence = evidence_by_construct.get(construct["construct_id"], {})
            compositional = next(
                (
                    row
                    for row in construct_evidence.get("compositional_invariance", [])
                    if (int(row["left_segment"]), int(row["right_segment"])) == pair_id
                ),
                None,
            )
            step3 = next(
                (
                    row
                    for row in construct_evidence.get("step3_equality", [])
                    if (int(row["left_segment"]), int(row["right_segment"])) == pair_id
                ),
                None,
            )
            exact_inventory &= compositional is not None and step3 is not None
            if compositional is not None and step3 is not None:
                exact_inventory &= (
                    construct_evidence["configural_identity_passed"]
                    is all(pair["configural_receipt"].values())
                    and compositional["passed"]
                    is (pair["complete"] and construct["compositional_invariance"])
                    and close(
                        float(compositional["permutation_p_value"]),
                        float(construct["compositional_invariance_probability"]),
                    )
                    and step3["mean_equality_passed"]
                    is (pair["complete"] and construct["equal_means"])
                    and step3["variance_equality_passed"]
                    is (pair["complete"] and construct["equal_variances"])
                )
    parameter_suffixes: dict[int, set[str]] = defaultdict(set)
    parameter_inventory_ok = bool(value["common_metric_parameters"])
    expected_metric = f"qpls.pos.pooled-common-metric.v1:{gate_input['pooled_metric_sha256']}"
    for parameter in value["common_metric_parameters"]:
        segment = int(parameter["class_id"])
        prefix_value = f"class_{segment}:"
        target_id = parameter["parameter"]["target_id"]
        suffix = target_id.removeprefix(prefix_value) if target_id.startswith(prefix_value) else ""
        parameter_inventory_ok &= (
            1 <= segment <= segments
            and bool(suffix)
            and suffix not in parameter_suffixes[segment]
            and parameter["metric"] == expected_metric
            and math.isfinite(float(parameter["parameter"]["estimate"]))
        )
        parameter_suffixes[segment].add(suffix)
    parameter_inventory_ok &= set(parameter_suffixes) == set(range(1, segments + 1))
    if parameter_suffixes:
        reference_targets = next(iter(parameter_suffixes.values()))
        parameter_inventory_ok &= bool(reference_targets) and all(
            targets == reference_targets for targets in parameter_suffixes.values()
        )
    coherence = gate["status"] == expected_status
    coherence &= gate["step3_required_for_standardized_path_comparison"] is False
    coherence &= gate_input["applied_identically_to_all_segments"] is True
    coherence &= exact_inventory and parameter_inventory_ok
    independent_step2 = True
    step2_rows = []
    for pair in value["micom_pairs"]:
        for construct in pair["constructs"]:
            null = [
                float(observed)
                for observed in construct["permutation_compositional_correlations"]
            ]
            observed = float(construct["observed_compositional_correlation"])
            lower = type7(null, 0.05)
            probability = (1 + sum(candidate <= observed for candidate in null)) / (len(null) + 1)
            invariant = observed >= lower
            independent_step2 &= len(null) == pair["usable_permutations"]
            independent_step2 &= close(lower, construct["compositional_lower_quantile"])
            independent_step2 &= close(
                probability, construct["compositional_invariance_probability"]
            )
            independent_step2 &= construct["compositional_invariance"] is invariant
            independent_step2 &= construct["partial_measurement_invariance"] is (
                pair["complete"] and invariant
            )
            step2_rows.append(
                {
                    "construct": construct["construct_id"],
                    "lower": lower,
                    "probability": probability,
                    "invariant": invariant,
                }
            )
    independent_step2 &= bool(step2_rows)
    independent_step2 &= all(row["invariant"] for row in step2_rows) is should_pass
    if should_pass:
        coherence &= gate["inferential_gamma_delta_slope_contrasts_allowed"] is True
        coherence &= not gate["blockers"] and output["descriptive_only"] is False
        coherence &= bool(output["contrasts"])
        coherence &= prepared_common_metric_repeated(cell)
    else:
        coherence &= gate["inferential_gamma_delta_slope_contrasts_allowed"] is False
        coherence &= bool(gate["blockers"]) and output["descriptive_only"] is True
        coherence &= output["contrasts"] == []
        coherence &= all(
            row["inferential_interpretation_blocked"] or row["common_metric_comparability_satisfied"]
            for row in output["contrasts"]
        )
    checks.require(
        f"{prefix}.{'pass' if should_pass else 'fail_closed'}",
        coherence,
        observed={
            "gate": gate,
            "descriptive_only": output["descriptive_only"],
            "contrasts": len(output["contrasts"]),
            "required_constructs": sorted(required_set),
            "retained_pairs": sorted(micom_by_pair),
            "exact_inventory": exact_inventory,
            "parameter_inventory": parameter_inventory_ok,
        },
        expected=(
            "configural+pairwise compositional pass enables pooled-metric inference"
            if should_pass
            else "compositional failure suppresses all segment contrasts and remains descriptive"
        ),
    )
    checks.require(
        f"{prefix}.independent_micom_step2",
        independent_step2,
        observed=step2_rows,
        expected=(
            "all required construct/pair Type-7 Step-2 decisions pass"
            if should_pass
            else "at least one independently reconstructed Step-2 decision fails"
        ),
    )


def prepared_common_metric_repeated(cell: dict[str, Any]) -> bool:
    bootstrap = cell["evidence"]["bootstrap"]
    return len(bootstrap) == 1 and bootstrap[0]["pooled_common_metric_refit_repeated"] is True


def validate_bootstrap_matrix(report: dict[str, Any], checks: Checks) -> None:
    cells = report["fixed_k_bootstrap"]
    expected_ids = {
        "fimix-p0-fixed-k-bootstrap",
        "pos-published-p0-fixed-k-bootstrap",
        "fimix-p2-fixed-k-bootstrap",
        "pos-destination-p2-fixed-k-bootstrap",
        "fimix-p23-fixed-k-bootstrap",
        "pos-destination-p23-fixed-k-bootstrap",
        "pos-p2-common-metric-failure-fixed-k-bootstrap",
        "pos-published-p0-k3-fixed-k-bootstrap",
        "pos-published-p0-k4-fixed-k-bootstrap",
        "pos-published-p0-k5-fixed-k-bootstrap",
    }
    actual_ids = {cell["cell_id"] for cell in cells}
    checks.require(
        "heterogeneity.bootstrap.full_profile_matrix",
        len(cells) == len(expected_ids) and actual_ids == expected_ids,
        observed={"count": len(cells), "cell_ids": sorted(actual_ids)},
        expected=sorted(expected_ids),
    )
    expected_k_by_id = {
        cell_id: (
            int(cell_id.split("-k", 1)[1].split("-", 1)[0])
            if cell_id.startswith("pos-published-p0-k")
            else 2
        )
        for cell_id in expected_ids
    }
    for cell in cells:
        prefix = f"heterogeneity.bootstrap.{cell['cell_id']}"
        lock = cell["analysis"]["inference_lock"]
        expected_k = expected_k_by_id.get(cell["cell_id"], -1)
        checks.require(
            f"{prefix}.discovery_lock",
            lock["analyst_lock_confirmed"]
            and lock["selected_k"] == expected_k
            and lock["discovery_result_identity_sha256"]
            == cell["analysis"]["discovery_result_identity_sha256"]
            and lock["discovery_candidate_k"] == [expected_k]
            and cell["analysis"]["locked_k"] == expected_k
            and cell["analysis"]["locked_algorithm"] == lock["selected_algorithm"],
            observed=lock,
            expected=f"identity-matched analyst lock for fixed K={expected_k}",
        )
        validate_bootstrap_cell(cell, checks, prefix)
        if lock["selected_algorithm"] == "fimix_pls_v2":
            result = fimix_result(cell, expected_k)
            if result is not None:
                validate_fimix_math(result, checks, f"{prefix}.fimix")
                validate_fimix_raw_likelihood(cell, result, checks, f"{prefix}.fimix")
            if cell["profile"] != "p0_structural":
                validate_scientific_interaction_targets(cell, checks, prefix)
        else:
            result = pos_result(cell, expected_k)
            expected_method = (
                POS_PUBLISHED
                if lock["selected_algorithm"] == "pls_pos_published_v2"
                else POS_INTERACTIONS
            )
            if result is not None:
                validate_pos_result(result, checks, f"{prefix}.pos", expected_method, cell["profile"])
            if cell["profile"] != "p0_structural" and cell["analysis"]["descriptive_only"] is False:
                validate_scientific_interaction_targets(cell, checks, prefix)
    by_id = {cell["cell_id"]: cell for cell in cells}
    published_matrix = []
    published_matrix_ok = True
    for classes in range(2, 6):
        cell_id = (
            "pos-published-p0-fixed-k-bootstrap"
            if classes == 2
            else f"pos-published-p0-k{classes}-fixed-k-bootstrap"
        )
        cell = by_id.get(cell_id)
        result = pos_result(cell, classes) if cell is not None else None
        lock = cell["analysis"]["inference_lock"] if cell is not None else {}
        expected_algorithms = (
            ["fimix_pls_v2", "pls_pos_published_v2"]
            if classes == 2
            else ["pls_pos_published_v2"]
        )
        row_ok = (
            cell is not None
            and cell["profile"] == "p0_structural"
            and lock.get("selected_algorithm") == "pls_pos_published_v2"
            and lock.get("selected_k") == classes
            and lock.get("discovery_algorithms") == expected_algorithms
            and lock.get("discovery_candidate_k") == [classes]
            and cell["analysis"].get("locked_algorithm") == "pls_pos_published_v2"
            and cell["analysis"].get("locked_k") == classes
            and result is not None
            and result.get("method_version") == POS_PUBLISHED
            and len(result.get("segments", [])) == classes
            and len(cell["evidence"]["pos"]) == 1
            and len(cell["evidence"]["fimix"]) == (1 if classes == 2 else 0)
        )
        published_matrix_ok &= row_ok
        published_matrix.append({"cell_id": cell_id, "k": classes, "passed": row_ok})
    checks.require(
        "heterogeneity.bootstrap.pos_published_p0_k2_through_k5_inventory",
        published_matrix_ok,
        observed=published_matrix,
        expected="exact P0/qpls.pls-pos.published.v2 fixed-K production bootstrap cells for K=2,3,4,5",
    )
    for cell_id in (
        "pos-published-p0-fixed-k-bootstrap",
        "pos-destination-p2-fixed-k-bootstrap",
        "pos-destination-p23-fixed-k-bootstrap",
    ):
        validate_common_metric(by_id[cell_id], checks, f"pos.common_metric.{cell_id}", True)
    validate_common_metric(
        by_id["pos-p2-common-metric-failure-fixed-k-bootstrap"],
        checks,
        "pos.common_metric.failure_fixture",
        False,
    )


def boundary_failed_closed(boundary: dict[str, Any]) -> bool:
    if boundary["status"] == "failed_closed":
        return True
    return any(
        row["k"] == 2 and row["state"] in {"failed", "ineligible"}
        for row in boundary["run"]["analysis"]["candidates"]
        if row["method"]["kind"] == "segmentation"
    )


def validate_boundaries(report: dict[str, Any], checks: Checks) -> None:
    boundaries = report["fimix"]["boundaries"]
    checks.require(
        "fimix.boundary.rank_deficient_fail_closed",
        boundary_failed_closed(boundaries["rank_deficient"]),
        observed=boundaries["rank_deficient"]["status"],
        expected="production rank/singularity blocker",
    )
    checks.require(
        "fimix.boundary.variance_collapse_fail_closed",
        boundary_failed_closed(boundaries["variance_collapse"]),
        observed=boundaries["variance_collapse"]["status"],
        expected="production residual-variance-collapse blocker",
    )
    rare = boundaries["rare_class"]
    counts = [int(value) for value in rare["true_class_counts"]]
    checks.require(
        "fimix.boundary.rare_class_fixture_exercised",
        rare["scenario"] == "rareclass"
        and rare["fixture_id"] == "heterogeneity-rare-class"
        and len(counts) == 2
        and sum(counts) > 0
        and min(counts) / sum(counts) < 0.05
        and rare["status"] in {"completed", "failed_closed"}
        and boundaries["separately_bound_kernel_test_filter"] == "fimix_failure_boundary_",
        observed={
            "status": rare["status"],
            "scenario": rare["scenario"],
            "true_class_counts": counts,
        },
        expected="a production raw rare-class attempt below five percent plus the separately bound kernel-test filter",
    )


def validate_report(report: dict[str, Any], checks: Checks) -> None:
    checks.require(
        "heterogeneity.receipt.identity",
        report.get("schema_version") == 1
        and report.get("suite_id") == SUITE_ID
        and report.get("scale") == "qualification"
        and report.get("qualification_claim") == "raw_sut_facts_for_independent_comparison_only",
        observed={key: report.get(key) for key in ("schema_version", "suite_id", "scale", "qualification_claim")},
        expected="qualification-scale raw-SUT receipt",
    )
    contract = report.get("multistart_reproducibility_contract", {})
    checks.require(
        "heterogeneity.multistart.digest_contract_identity",
        contract.get("schema_version") == 1
        and contract.get("partition_digest_domain")
        == MULTISTART_PARTITION_DOMAIN.decode("utf-8")
        and contract.get("coefficient_digest_domain")
        == MULTISTART_COEFFICIENT_DOMAIN.decode("utf-8")
        and contract.get("posterior_digest_domain")
        == MULTISTART_POSTERIOR_DOMAIN.decode("utf-8")
        and contract.get("parameter_digest_domain")
        == MULTISTART_PARAMETER_DOMAIN.decode("utf-8")
        and contract.get("fit_statistic_digest_domain")
        == MULTISTART_FIT_STATISTIC_DOMAIN.decode("utf-8")
        and contract.get("completed_start_cardinality")
        == "exactly_one_receipt_per_completed_start"
        and contract.get("verification")
        == "independent_exhaustive_alignment_and_tolerance_replay",
        observed=contract,
        expected="the versioned ordered multistart digest and replay contract",
    )
    validate_fimix_recovery(report, checks)
    validate_fimix_simulation_matrix(report, checks)
    validate_candidate_table(report, checks)
    validate_homogeneous_null(report, checks)
    validate_pos_discoveries(report, checks)
    validate_label_alignment_decision_matrix(report, checks)
    validate_bootstrap_matrix(report, checks)
    validate_boundaries(report, checks)
    replay_rows = [
        row
        for row in checks.rows
        if row["check_id"].endswith(".independent_multistart_reproduction")
    ]
    checks.require(
        "heterogeneity.multistart.independent_replay_present",
        len(replay_rows) >= 2 and all(row["status"] == "passed" for row in replay_rows),
        observed={
            "replayed_cells": len(replay_rows),
            "failed_cells": [row["check_id"] for row in replay_rows if row["status"] != "passed"],
        },
        expected="independent completed-start replay for both FIMIX and PLS-POS production cells",
    )
    emitted = {row["check_id"] for row in checks.rows}
    checks.require(
        "heterogeneity.binding.required_check_inventory",
        BOUND_GATE_CHECK_IDS.issubset(emitted),
        observed=sorted(BOUND_GATE_CHECK_IDS - emitted),
        expected="every exact downstream scientific-check identity is emitted",
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    return parser.parse_args()


def main() -> int:
    arguments = parse_args()
    report = json.loads(arguments.input.read_text(encoding="utf-8"))
    checks = Checks()
    try:
        validate_report(report, checks)
    except Exception as error:
        checks.require(
            "heterogeneity.comparator.unhandled_exception",
            False,
            observed=f"{type(error).__name__}: {error}",
            expected="complete independent comparison",
        )
    output = {
        "schema_version": 1,
        "suite_id": "qpls.multimod.heterogeneity.independent-comparison.v2",
        "status": "passed" if checks.passed else "failed",
        "input_sha256": file_sha256(arguments.input),
        "scope_status": {
            "covered_checks": "passed" if checks.passed else "failed",
            "full_heterogeneity_qualification": (
                "pending" if checks.coverage_gaps else ("passed" if checks.passed else "failed")
            ),
            "pending_check_ids": [row["check_id"] for row in checks.coverage_gaps],
        },
        "results": {
            "heterogeneity": {
                "status": "passed" if checks.passed else "failed",
                "full_scope_status": (
                    "pending" if checks.coverage_gaps else ("passed" if checks.passed else "failed")
                ),
                "checks": checks.rows,
                "coverage_gaps": checks.coverage_gaps,
            }
        },
    }
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_text(json.dumps(output, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0 if checks.passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
