#!/usr/bin/env python3
"""Compact independent calibration for prospective PLS power v2.

This is deliberately smaller than the historical v1 qualification simulation.
It independently implements the bounded two-construct data generator, Mode-A PLS
estimator, indexed case bootstrap, null-centred two-sided tail count, and plus-one
probability.  The frozen profile targets the scientific defect that blocked v1:
null type-I calibration.  It also includes one modest signal-recovery check.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from datetime import datetime, timezone
from pathlib import Path
from statistics import NormalDist
from typing import Any

import numpy as np


METHOD_VERSION = "pls_sample_size_power_v2"
INFERENCE_METHOD = "pls_pm_case_bootstrap_null_centered_two_sided_plus_one_v2"
STREAM_DOMAIN = "quickpls/validation/pls_sample_size_power_v2_reference"
MASTER_SEED = 20_260_818
LOADINGS = np.asarray([0.80, 0.80, 0.80], dtype=float)
ALPHA = 0.05
BOOTSTRAP_REPLICATES = 99


def _digest(effect: float, sample_size: int, replicate: int, subdomain: str) -> bytes:
    digest = hashlib.sha256()
    for value in (STREAM_DOMAIN, METHOD_VERSION, f"effect={effect:.6f}"):
        digest.update(value.encode("ascii"))
        digest.update(b"\0")
    digest.update(MASTER_SEED.to_bytes(8, "little"))
    digest.update(sample_size.to_bytes(4, "little"))
    digest.update(replicate.to_bytes(4, "little"))
    digest.update(subdomain.encode("ascii"))
    return digest.digest()


def _generate(effect: float, sample_size: int, replicate: int) -> tuple[np.ndarray, np.ndarray]:
    rng = np.random.default_rng(
        int.from_bytes(_digest(effect, sample_size, replicate, "generated_data")[:16], "little")
    )
    predictor = rng.standard_normal(sample_size)
    outcome = effect * predictor + math.sqrt(1.0 - effect * effect) * rng.standard_normal(sample_size)
    indicator_error = np.sqrt(1.0 - LOADINGS * LOADINGS)
    predictor_block = predictor[:, None] * LOADINGS + rng.standard_normal(
        (sample_size, LOADINGS.size)
    ) * indicator_error
    outcome_block = outcome[:, None] * LOADINGS + rng.standard_normal(
        (sample_size, LOADINGS.size)
    ) * indicator_error
    return predictor_block, outcome_block


def _standardize(matrix: np.ndarray) -> np.ndarray:
    deviations = np.std(matrix, axis=0, ddof=1)
    if np.any(~np.isfinite(deviations)) or np.any(deviations <= np.finfo(float).eps):
        raise FloatingPointError("indicator variance is unavailable")
    return (matrix - np.mean(matrix, axis=0)) / deviations


def _standardize_score(score: np.ndarray) -> np.ndarray:
    centered = score - np.mean(score)
    scale = float(np.std(centered, ddof=1))
    if not math.isfinite(scale) or scale <= np.finfo(float).eps:
        raise FloatingPointError("construct score variance is unavailable")
    return centered / scale


def _estimate(predictor_block: np.ndarray, outcome_block: np.ndarray) -> float:
    predictor = _standardize(predictor_block)
    outcome = _standardize(outcome_block)
    predictor_weights = np.ones(predictor.shape[1], dtype=float)
    outcome_weights = np.ones(outcome.shape[1], dtype=float)
    predictor_weights /= np.linalg.norm(predictor_weights)
    outcome_weights /= np.linalg.norm(outcome_weights)
    converged = False
    for _ in range(3_000):
        predictor_score = _standardize_score(predictor @ predictor_weights)
        outcome_score = _standardize_score(outcome @ outcome_weights)
        direction = 1.0 if float(np.dot(predictor_score, outcome_score)) >= 0.0 else -1.0
        next_predictor = predictor.T @ (direction * outcome_score)
        next_outcome = outcome.T @ (direction * predictor_score)
        predictor_norm = float(np.linalg.norm(next_predictor))
        outcome_norm = float(np.linalg.norm(next_outcome))
        if predictor_norm <= np.finfo(float).eps or outcome_norm <= np.finfo(float).eps:
            raise FloatingPointError("Mode-A update is singular")
        next_predictor /= predictor_norm
        next_outcome /= outcome_norm
        if next_predictor[0] < 0.0:
            next_predictor *= -1.0
        if next_outcome[0] < 0.0:
            next_outcome *= -1.0
        change = max(
            float(np.max(np.abs(next_predictor - predictor_weights))),
            float(np.max(np.abs(next_outcome - outcome_weights))),
        )
        predictor_weights, outcome_weights = next_predictor, next_outcome
        if change <= 1e-7:
            converged = True
            break
    if not converged:
        raise FloatingPointError("Mode-A PLS did not converge")
    predictor_score = _standardize_score(predictor @ predictor_weights)
    outcome_score = _standardize_score(outcome @ outcome_weights)
    denominator = float(np.dot(predictor_score, predictor_score))
    if denominator <= np.finfo(float).eps:
        raise FloatingPointError("structural regression is singular")
    estimate = float(np.dot(predictor_score, outcome_score) / denominator)
    if not math.isfinite(estimate):
        raise FloatingPointError("path estimate is non-finite")
    return estimate


def _test(effect: float, sample_size: int, replicate: int) -> tuple[float, int, int]:
    predictor, outcome = _generate(effect, sample_size, replicate)
    original = _estimate(predictor, outcome)
    rng = np.random.default_rng(
        int.from_bytes(_digest(effect, sample_size, replicate, "bootstrap_inference")[:16], "little")
    )
    estimates: list[float] = []
    failed = 0
    for _ in range(BOOTSTRAP_REPLICATES):
        indices = rng.integers(0, sample_size, size=sample_size)
        try:
            estimates.append(_estimate(predictor[indices], outcome[indices]))
        except (FloatingPointError, np.linalg.LinAlgError):
            failed += 1
    required = math.ceil(BOOTSTRAP_REPLICATES * 0.90)
    if len(estimates) < required:
        raise FloatingPointError(f"only {len(estimates)} usable bootstrap estimates; {required} required")
    exceedances = sum(abs(estimate - original) >= abs(original) for estimate in estimates)
    p_value = (exceedances + 1.0) / (len(estimates) + 1.0)
    return p_value, len(estimates), failed


def _wilson(successes: int, trials: int, confidence: float = 0.95) -> tuple[float, float]:
    z = NormalDist().inv_cdf(0.5 + confidence / 2.0)
    proportion = successes / trials
    z_squared = z * z
    denominator = 1.0 + z_squared / trials
    center = (proportion + z_squared / (2.0 * trials)) / denominator
    half = z * math.sqrt(
        proportion * (1.0 - proportion) / trials + z_squared / (4.0 * trials * trials)
    ) / denominator
    return max(0.0, center - half), min(1.0, center + half)


def _scenario(effect: float, sample_size: int, replicates: int) -> dict[str, Any]:
    rejected = 0
    failed_runs = 0
    failed_bootstraps = 0
    usable_bootstraps = 0
    for replicate in range(replicates):
        try:
            p_value, usable, failed = _test(effect, sample_size, replicate)
        except (FloatingPointError, np.linalg.LinAlgError, ValueError):
            failed_runs += 1
            continue
        rejected += int(p_value <= ALPHA)
        usable_bootstraps += usable
        failed_bootstraps += failed
    successful = replicates - failed_runs
    interval = _wilson(rejected, successful) if successful else (0.0, 1.0)
    return {
        "effect": effect,
        "sample_size": sample_size,
        "requested_runs": replicates,
        "successful_runs": successful,
        "failed_runs": failed_runs,
        "rejections": rejected,
        "rejection_rate": rejected / successful if successful else None,
        "wilson_95": list(interval),
        "bootstrap_requested_per_run": BOOTSTRAP_REPLICATES,
        "bootstrap_usable_total": usable_bootstraps,
        "bootstrap_failed_total": failed_bootstraps,
    }


def run(profile: str) -> dict[str, Any]:
    if profile == "signal":
        scenarios = [_scenario(0.45, 60, 100)]
        checks = {"diagnostic_only": True}
        passed = True
    else:
        scenarios = [
            _scenario(0.0, 160, 300),
            _scenario(0.0, 320, 300),
            _scenario(0.45, 60, 100),
        ]
        null = scenarios[:2]
        signal = scenarios[2]
        pooled_rejections = sum(item["rejections"] for item in null)
        pooled_runs = sum(item["successful_runs"] for item in null)
        pooled_interval = _wilson(pooled_rejections, pooled_runs)
        bootstrap_requested = sum(
            item["successful_runs"] * item["bootstrap_requested_per_run"] for item in scenarios
        )
        bootstrap_failed = sum(item["bootstrap_failed_total"] for item in scenarios)
        checks = {
            "all_runs_succeeded": all(item["failed_runs"] == 0 for item in scenarios),
            "bootstrap_failure_rate_at_most_0_001": bootstrap_failed / bootstrap_requested <= 0.001,
            "each_null_rate_at_most_0_08": all(item["rejection_rate"] <= 0.08 for item in null),
            "pooled_null_rate_at_most_0_065": pooled_rejections / pooled_runs <= 0.065,
            "pooled_null_wilson_contains_0_05": pooled_interval[0] <= 0.05 <= pooled_interval[1],
            "signal_rejection_rate_at_least_0_60": signal["rejection_rate"] >= 0.60,
            "pooled_null_rejections": pooled_rejections,
            "pooled_null_runs": pooled_runs,
            "pooled_null_rate": pooled_rejections / pooled_runs,
            "pooled_null_wilson_95": list(pooled_interval),
            "bootstrap_requested_total": bootstrap_requested,
            "bootstrap_failed_total": bootstrap_failed,
            "bootstrap_failure_rate": bootstrap_failed / bootstrap_requested,
        }
        passed = all(value for value in checks.values() if isinstance(value, bool))
    return {
        "schema_version": 1,
        "report_kind": "quickpls_pls_sample_size_power_v2_independent_calibration",
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "passed": passed,
        "method_version": METHOD_VERSION,
        "inference_method": INFERENCE_METHOD,
        "independence": "NumPy generator and estimator; no QuickPLS binary or Rust result is consumed",
        "profile": profile,
        "scenarios": scenarios,
        "checks": checks,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile", choices=("signal", "compact"), default="compact")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    report = run(args.profile)
    rendered = json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    print(rendered, end="")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
