"""Transparent independent micro-oracle for PLS model comparison v1.

This module deliberately imports no QuickPLS code.  It freezes the published
equations for equation-level prediction-oriented BIC, BIC-derived Akaike
weights, paired CVPAT case loss, and the shared-fold SHA-256 plan.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import struct
from dataclasses import asdict, dataclass


FOLD_VERSION = "seeded_sha256_shared_complete_rows_round_robin_v1"


def prediction_oriented_bic(sample_size: int, sse: float, parameter_count: int) -> float:
    if sample_size < 3 or not math.isfinite(sse) or sse <= 0.0 or parameter_count < 1:
        raise ValueError("invalid prediction-oriented BIC input")
    return sample_size * math.log(sse / sample_size) + parameter_count * math.log(sample_size)


def two_candidate_weights(first_bic: float, second_bic: float) -> tuple[float, float, float, float]:
    if not math.isfinite(first_bic) or not math.isfinite(second_bic):
        raise ValueError("BIC values must be finite")
    minimum = min(first_bic, second_bic)
    first_delta = first_bic - minimum
    second_delta = second_bic - minimum
    first_relative = math.exp(-0.5 * first_delta)
    second_relative = math.exp(-0.5 * second_delta)
    denominator = first_relative + second_relative
    return (
        first_delta,
        second_delta,
        first_relative / denominator,
        second_relative / denominator,
    )


@dataclass(frozen=True)
class CvpatMicroResult:
    established_mean_loss: float
    alternative_mean_loss: float
    average_loss_difference: float
    sample_variance: float
    standard_error: float
    t_statistic: float
    degrees_of_freedom: int


def paired_cvpat(established: list[float], alternative: list[float]) -> CvpatMicroResult:
    if len(established) != len(alternative) or len(established) < 3:
        raise ValueError("paired loss vectors must have equal length >= 3")
    if any(not math.isfinite(value) or value < 0.0 for value in established + alternative):
        raise ValueError("losses must be finite and nonnegative")
    differences = [right - left for left, right in zip(established, alternative, strict=True)]
    count = len(differences)
    mean_difference = sum(differences) / count
    variance = sum((value - mean_difference) ** 2 for value in differences) / (count - 1)
    standard_error = math.sqrt(variance / count)
    return CvpatMicroResult(
        established_mean_loss=sum(established) / count,
        alternative_mean_loss=sum(alternative) / count,
        average_loss_difference=mean_difference,
        sample_variance=variance,
        standard_error=standard_error,
        t_statistic=mean_difference / standard_error,
        degrees_of_freedom=count - 1,
    )


def _u64(value: int) -> bytes:
    return struct.pack("<Q", value)


def shared_fold_plan(rows: list[int], folds: int, repeats: int, seed: int) -> dict[str, object]:
    if rows != sorted(set(rows)) or folds < 2 or repeats < 1 or len(rows) < 2 * folds:
        raise ValueError("invalid shared-fold input")
    digest = hashlib.sha256()
    digest.update(FOLD_VERSION.encode("utf-8"))
    digest.update(_u64(seed))
    digest.update(_u64(folds))
    digest.update(_u64(repeats))
    ledger: list[dict[str, int]] = []
    for repeat in range(repeats):
        ranked: list[tuple[bytes, int, int]] = []
        for complete_index, source_row in enumerate(rows):
            rank = hashlib.sha256()
            rank.update(FOLD_VERSION.encode("utf-8"))
            rank.update(_u64(seed))
            rank.update(_u64(repeat))
            rank.update(_u64(source_row))
            ranked.append((rank.digest(), complete_index, source_row))
        ranked.sort(key=lambda value: (value[0], value[1]))
        assignments = [0] * len(rows)
        for position, (_, complete_index, _) in enumerate(ranked):
            assignments[complete_index] = position % folds
        for source_row, fold in zip(rows, assignments, strict=True):
            digest.update(_u64(repeat))
            digest.update(_u64(source_row))
            digest.update(_u64(fold))
            ledger.append({"repeat": repeat, "source_row": source_row, "fold": fold})
    return {"assignment_digest": f"sha256:{digest.hexdigest()}", "ledger": ledger}


def build_report() -> dict[str, object]:
    bic = prediction_oriented_bic(10, 5.0, 3)
    weights = two_candidate_weights(10.0, 12.0)
    cvpat = paired_cvpat([4.0, 1.0, 9.0, 4.0], [1.0, 1.0, 4.0, 9.0])
    folds = shared_fold_plan(list(range(23)), folds=5, repeats=3, seed=47)
    return {
        "oracle": "independent_python_standard_library_v1",
        "bic": bic,
        "weights": {
            "first_delta": weights[0],
            "second_delta": weights[1],
            "first_weight": weights[2],
            "second_weight": weights[3],
        },
        "cvpat": asdict(cvpat),
        "fold_assignment_digest": folds["assignment_digest"],
        "fold_ledger_entries": len(folds["ledger"]),
    }


def check_report(report: dict[str, object]) -> None:
    expected_bic = 10.0 * math.log(0.5) + 3.0 * math.log(10.0)
    assert math.isclose(float(report["bic"]), expected_bic, rel_tol=0.0, abs_tol=1e-14)
    weights = report["weights"]
    assert isinstance(weights, dict)
    assert math.isclose(float(weights["first_weight"]), 0.7310585786300049, abs_tol=1e-15)
    assert math.isclose(float(weights["second_weight"]), 0.2689414213699951, abs_tol=1e-15)
    cvpat = report["cvpat"]
    assert isinstance(cvpat, dict)
    assert math.isclose(float(cvpat["average_loss_difference"]), -0.75, abs_tol=1e-15)
    assert math.isclose(float(cvpat["sample_variance"]), 18.916666666666668, abs_tol=1e-14)
    assert math.isclose(float(cvpat["standard_error"]), 2.174664725116648, abs_tol=1e-14)
    assert math.isclose(float(cvpat["t_statistic"]), -0.34488074935770635, abs_tol=1e-14)
    assert report["fold_ledger_entries"] == 69
    assert report["fold_assignment_digest"] == (
        "sha256:b08f53b2641bc2a2bc8eef4c46c56a5b4f5ad3a413fc195f210ec68212a25c74"
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="fail if the frozen microcase drifts")
    args = parser.parse_args()
    report = build_report()
    if args.check:
        check_report(report)
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
