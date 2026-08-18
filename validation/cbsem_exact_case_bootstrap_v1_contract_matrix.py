#!/usr/bin/env python3
"""Compact independent arithmetic/metamorphic matrix for exact-CFA bootstrap v1.

This is deliberately not a coverage simulation and does not refit the product
estimator.  It independently freezes Type-7 percentile, reverse-pivot analytic
studentized, BCa, indexed-ledger, no-retry, and unsigned-integrity boundaries
that can be checked in well under a second.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from statistics import NormalDist
from typing import Any, Iterable


FEATURE_ID = "qpls3.cbsem.bootstrap"
METHOD_VERSION = "cbsem_exact_case_bootstrap_v1"
CATALOGUE_SNAPSHOT_DATE = "2026-08-12"
NORMAL = NormalDist()
ESTIMATES = (0.72, 0.81, 0.90, 0.95, 1.02, 1.08, 1.11, 1.19, 1.27)
OUTER_SE = (0.115, 0.109, 0.104, 0.101, 0.098, 0.096, 0.099, 0.106, 0.118)
DELETE_ONE = (0.96, 0.99, 1.03, 1.01, 1.05, 1.00, 1.04, 0.98)
POINT = 1.0
POINT_SE = 0.1
EXPECTED: dict[str, Any] | None = {
    "summaries": {
        "bootstrap_standard_error": 0.17840341302166218,
        "percentile_type7": [0.738, 1.254],
        "analytic_studentized_type7": [0.7811000959385993, 1.229644994016753],
        "bca_type7": [0.729399302745221, 1.24098200612232],
    },
    "metamorphic": {
        "estimate_reorder_invariant": True,
        "worker_index_projection_invariant": True,
    },
    "boundaries": {
        "attempted": 4,
        "usable": 2,
        "failed": 2,
        "no_retry_accounting": True,
        "structural_tamper_rejected": True,
        "coordinated_rewrite_not_authenticated": True,
    },
}


def type7(values: Iterable[float], probability: float) -> float:
    ordered = sorted(float(value) for value in values)
    if not ordered or not 0.0 <= probability <= 1.0:
        raise ValueError("Type-7 requires a nonempty finite sample and p in [0,1]")
    if not all(math.isfinite(value) for value in ordered):
        raise ValueError("Type-7 values must be finite")
    position = (len(ordered) - 1) * probability
    lower = math.floor(position)
    upper = math.ceil(position)
    fraction = position - lower
    return ordered[lower] + fraction * (ordered[upper] - ordered[lower])


def sample_se(values: Iterable[float]) -> float:
    rows = tuple(float(value) for value in values)
    mean = math.fsum(rows) / len(rows)
    return math.sqrt(math.fsum((value - mean) ** 2 for value in rows) / (len(rows) - 1))


def percentile_interval(values: Iterable[float]) -> tuple[float, float]:
    rows = tuple(values)
    return type7(rows, 0.025), type7(rows, 0.975)


def studentized_interval(
    point: float,
    point_se: float,
    estimates: Iterable[float],
    outer_standard_errors: Iterable[float],
) -> tuple[float, float]:
    pivots = tuple(
        (estimate - point) / standard_error
        for estimate, standard_error in zip(estimates, outer_standard_errors, strict=True)
    )
    if point_se <= 0.0 or any(not math.isfinite(value) for value in pivots):
        raise ValueError("studentized inputs must be finite with positive standard errors")
    return (
        point - type7(pivots, 0.975) * point_se,
        point - type7(pivots, 0.025) * point_se,
    )


def bca_interval(
    point: float,
    estimates: Iterable[float],
    delete_one: Iterable[float],
) -> tuple[float, float]:
    outer = tuple(float(value) for value in estimates)
    jackknife = tuple(float(value) for value in delete_one)
    less = sum(value < point for value in outer)
    ties = sum(value == point for value in outer)
    midrank_probability = (less + 0.5 * ties) / len(outer)
    if not 0.0 < midrank_probability < 1.0:
        raise ValueError("BCa bias correction is at a probability boundary")
    z0 = NORMAL.inv_cdf(midrank_probability)
    jackknife_mean = math.fsum(jackknife) / len(jackknife)
    influence = tuple(jackknife_mean - value for value in jackknife)
    sum_squares = math.fsum(value * value for value in influence)
    if sum_squares <= 0.0:
        raise ValueError("BCa acceleration is degenerate")
    acceleration = math.fsum(value ** 3 for value in influence) / (6.0 * sum_squares ** 1.5)

    def adjusted(alpha: float) -> float:
        z_alpha = NORMAL.inv_cdf(alpha)
        denominator = 1.0 - acceleration * (z0 + z_alpha)
        if denominator == 0.0:
            raise ValueError("BCa acceleration adjustment is singular")
        return NORMAL.cdf(z0 + (z0 + z_alpha) / denominator)

    lower_probability = adjusted(0.025)
    upper_probability = adjusted(0.975)
    if not 0.0 <= lower_probability < upper_probability <= 1.0:
        raise ValueError("BCa adjusted probabilities are invalid")
    return type7(outer, lower_probability), type7(outer, upper_probability)


def canonical_digest(value: Any) -> str:
    payload = json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def indexed_projection(workers: int) -> tuple[float, ...]:
    if workers < 1:
        raise ValueError("workers must be positive")
    partitions = [list() for _ in range(workers)]
    for index, estimate in enumerate(ESTIMATES):
        partitions[index % workers].append((index, estimate))
    return tuple(value for _, value in sorted(row for partition in partitions for row in partition))


def observed_matrix() -> dict[str, Any]:
    percentile = percentile_interval(ESTIMATES)
    studentized = studentized_interval(POINT, POINT_SE, ESTIMATES, OUTER_SE)
    bca = bca_interval(POINT, ESTIMATES, DELETE_ONE)
    summaries = {
        "bootstrap_standard_error": sample_se(ESTIMATES),
        "percentile_type7": list(percentile),
        "analytic_studentized_type7": list(studentized),
        "bca_type7": list(bca),
    }
    ledger = [
        {"index": 0, "status": "usable"},
        {"index": 1, "status": "failed", "reason": "non_convergence"},
        {"index": 2, "status": "usable"},
        {"index": 3, "status": "failed", "reason": "inadmissible_solution"},
    ]
    attempted = len(ledger)
    usable = sum(row["status"] == "usable" for row in ledger)
    failed = sum(row["status"] == "failed" for row in ledger)
    evidence = {"method": METHOD_VERSION, "attempted": attempted, "usable": usable, "failed": failed, "ledger": ledger}
    digest = canonical_digest(evidence)
    tampered = json.loads(json.dumps(evidence))
    tampered["ledger"][0]["status"] = "failed"
    structural_tamper_rejected = canonical_digest(tampered) != digest
    coordinated = json.loads(json.dumps(tampered))
    coordinated_digest = canonical_digest(coordinated)
    coordinated_rewrite_not_authenticated = coordinated_digest == canonical_digest(coordinated)
    return {
        "summaries": summaries,
        "metamorphic": {
            "estimate_reorder_invariant": summaries == {
                "bootstrap_standard_error": sample_se(reversed(ESTIMATES)),
                "percentile_type7": list(percentile_interval(reversed(ESTIMATES))),
                "analytic_studentized_type7": list(studentized_interval(POINT, POINT_SE, reversed(ESTIMATES), reversed(OUTER_SE))),
                "bca_type7": list(bca_interval(POINT, reversed(ESTIMATES), reversed(DELETE_ONE))),
            },
            "worker_index_projection_invariant": indexed_projection(1) == indexed_projection(3) == indexed_projection(9),
        },
        "boundaries": {
            "attempted": attempted,
            "usable": usable,
            "failed": failed,
            "no_retry_accounting": attempted == usable + failed and [row["index"] for row in ledger] == list(range(attempted)),
            "structural_tamper_rejected": structural_tamper_rejected,
            "coordinated_rewrite_not_authenticated": coordinated_rewrite_not_authenticated,
        },
    }


def report() -> dict[str, Any]:
    observed = observed_matrix()
    if EXPECTED is None:
        raise RuntimeError("freeze EXPECTED from --emit-observed before using this matrix as evidence")
    passed = observed == EXPECTED
    return {
        "passed": passed,
        "feature_id": FEATURE_ID,
        "method_version": METHOD_VERSION,
        "catalogue_snapshot_date": CATALOGUE_SNAPSHOT_DATE,
        "kind": "cbsem_exact_case_bootstrap_v1_compact_contract_matrix",
        "scope": "independent interval arithmetic, indexed accounting, metamorphic equality, and unsigned structural-integrity boundary; no coverage or estimator-refit claim",
        "observed": observed,
        "expected": EXPECTED,
        "source_artifacts": ["validation/cbsem_exact_case_bootstrap_v1_contract_matrix.py"],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--emit-observed", action="store_true")
    args = parser.parse_args()
    document = observed_matrix() if args.emit_observed else report()
    print(json.dumps(document, indent=2, sort_keys=True, allow_nan=False))
    return 0 if args.emit_observed or document["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
