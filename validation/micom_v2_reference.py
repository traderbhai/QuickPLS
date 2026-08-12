#!/usr/bin/env python3
"""Independent numerical reference for MICOM v2 and permutation-MGA v2.

This validation-only implementation uses NumPy and does not import QuickPLS code. It
reproduces the documented bounded path-weighting workflow from raw CSV values. The
reference follows Henseler, Ringle, and Sarstedt (2016), DOI
10.1108/IMR-09-2014-0304: group weights are applied to the pooled indicator matrix for
MICOM Step 2, while a single pooled PLS fit supplies the scores used by Step 3.

The default command performs the independent calculation and writes a non-promotable
reference-only report. Add ``--run-quickpls`` (or ``--quickpls-json PATH``) to compare
the independent result with a current QuickPLS CLI result. This script never builds the
CLI; set QUICKPLS_CLI_PATH when target/release/qpls.exe is not the desired binary.
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import os
import random
import subprocess
import warnings
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import numpy as np

try:
    from numba import njit
    from numba.core.errors import NumbaPerformanceWarning

    warnings.simplefilter("ignore", category=NumbaPerformanceWarning)
except ImportError:  # pragma: no cover - the pure NumPy fallback remains authoritative.
    njit = None


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
DATA = RESULTS / "micom_v2_reference.csv"
RECIPE = RESULTS / "micom_v2_reference.recipe.json"
QUICKPLS = RESULTS / "micom_v2_reference_quickpls.json"
OUTPUT = RESULTS / "micom_v2_reference_report.json"

SEED = 20_260_811
CONFIDENCE_LEVEL = 0.95
PROMOTION_PERMUTATIONS = 5_000
N_PER_GROUP = 80
TOLERANCE = 2e-6
CONVERGENCE_TOLERANCE = 1e-10
MAX_ITERATIONS = 3_000
MASK_64 = (1 << 64) - 1
GOLDEN_64 = 0x9E37_79B9_7F4A_7C15
LCG_MULTIPLIER = 6_364_136_223_846_793_005
LCG_INCREMENT = 1_442_695_040_888_963_407

CONSTRUCTS: tuple[tuple[str, tuple[str, ...]], ...] = (
    ("x", ("x1", "x2", "x3")),
    ("z", ("z1", "z2", "z3")),
    ("r", ("r1", "r2", "r3")),
    ("y", ("y1", "y2", "y3")),
)
PATHS: tuple[tuple[str, str], ...] = (("x", "y"), ("z", "y"), ("r", "y"))
INDICATORS = tuple(indicator for _, block in CONSTRUCTS for indicator in block)
CONSTRUCT_INDEX = {construct: index for index, (construct, _) in enumerate(CONSTRUCTS)}
INDICATOR_INDEX = {indicator: index for index, indicator in enumerate(INDICATORS)}
BLOCKS = tuple(
    np.asarray([INDICATOR_INDEX[indicator] for indicator in indicators], dtype=int)
    for _, indicators in CONSTRUCTS
)


@dataclass
class Fit:
    observations: int
    means: np.ndarray
    scales: np.ndarray
    weights: list[np.ndarray]
    scores: list[np.ndarray]
    loadings: list[np.ndarray]
    paths: dict[tuple[str, str], float]


def mean(values: np.ndarray) -> float:
    return float(np.mean(values))


def sample_variance(values: np.ndarray) -> float:
    return float(np.var(values, ddof=1))


def sample_sd(values: np.ndarray) -> float:
    return float(np.std(values, ddof=1))


def standardize(values: np.ndarray) -> np.ndarray:
    deviation = sample_sd(values)
    if not math.isfinite(deviation) or deviation <= np.finfo(float).eps:
        raise RuntimeError("zero-variance vector in independent reference")
    return (values - mean(values)) / deviation


def covariance(left: np.ndarray, right: np.ndarray) -> float:
    return float(np.dot(left - mean(left), right - mean(right)) / (len(left) - 1))


def correlation(left: np.ndarray, right: np.ndarray) -> float:
    return covariance(left, right) / (sample_sd(left) * sample_sd(right))


def normalize_block(columns: np.ndarray, block: np.ndarray, candidate: np.ndarray) -> np.ndarray:
    weights = np.asarray(candidate, dtype=float).copy()
    score = columns[:, block] @ weights
    reference = np.sum(columns[:, block], axis=1)
    association = covariance(score, reference)
    if association < -1e-15 or (abs(association) <= 1e-15 and float(np.sum(weights)) < 0.0):
        weights *= -1.0
        score *= -1.0
    deviation = sample_sd(score)
    if not math.isfinite(deviation) or deviation <= np.finfo(float).eps:
        raise RuntimeError("outer weights produced a zero-variance score")
    weights /= deviation
    score = columns[:, block] @ weights
    association = covariance(score, reference)
    if association < -1e-15 or (abs(association) <= 1e-15 and float(np.sum(weights)) < 0.0):
        weights *= -1.0
    return weights


def block_scores(columns: np.ndarray, weights: list[np.ndarray]) -> list[np.ndarray]:
    return [standardize(columns[:, block] @ weight) for block, weight in zip(BLOCKS, weights)]


def ols(predictors: list[np.ndarray], outcome: np.ndarray) -> np.ndarray:
    matrix = np.column_stack(predictors)
    return np.linalg.solve(matrix.T @ matrix, matrix.T @ outcome)


def inner_proxies(scores: list[np.ndarray]) -> list[np.ndarray]:
    proxies: list[np.ndarray] = []
    for construct_index, (construct, _) in enumerate(CONSTRUCTS):
        incoming = [CONSTRUCT_INDEX[source] for source, target in PATHS if target == construct]
        outgoing = [CONSTRUCT_INDEX[target] for source, target in PATHS if source == construct]
        if incoming:
            coefficients = ols([scores[index] for index in incoming], scores[construct_index])
            proxy = sum(
                (coefficient * scores[index] for coefficient, index in zip(coefficients, incoming)),
                np.zeros_like(scores[construct_index]),
            )
        else:
            proxy = sum(
                (
                    correlation(scores[construct_index], scores[index]) * scores[index]
                    for index in outgoing
                ),
                np.zeros_like(scores[construct_index]),
            )
        proxies.append(standardize(proxy))
    return proxies


def estimate_pls_numpy(raw: np.ndarray) -> Fit:
    means = np.mean(raw, axis=0)
    scales = np.std(raw, axis=0, ddof=1)
    if np.any(~np.isfinite(scales)) or np.any(scales <= np.finfo(float).eps):
        raise RuntimeError("constant indicator in independent reference")
    columns = (raw - means) / scales
    weights = [normalize_block(columns, block, np.ones(len(block))) for block in BLOCKS]
    for _ in range(MAX_ITERATIONS):
        scores = block_scores(columns, weights)
        proxies = inner_proxies(scores)
        updated = [
            normalize_block(
                columns,
                block,
                np.asarray([covariance(columns[:, column], proxies[index]) for column in block]),
            )
            for index, block in enumerate(BLOCKS)
        ]
        change = max(
            float(np.max(np.abs(before - after))) for before, after in zip(weights, updated)
        )
        weights = updated
        if change <= CONVERGENCE_TOLERANCE:
            break
    else:
        raise RuntimeError("independent PLS reference did not converge")
    scores = block_scores(columns, weights)
    loadings = [
        np.asarray([correlation(columns[:, column], scores[index]) for column in block])
        for index, block in enumerate(BLOCKS)
    ]
    paths: dict[tuple[str, str], float] = {}
    for target, _ in CONSTRUCTS:
        incoming = [source for source, candidate_target in PATHS if candidate_target == target]
        if not incoming:
            continue
        coefficients = ols(
            [scores[CONSTRUCT_INDEX[source]] for source in incoming],
            scores[CONSTRUCT_INDEX[target]],
        )
        paths.update({(source, target): float(value) for source, value in zip(incoming, coefficients)})
    return Fit(len(raw), means, scales, weights, scores, loadings, paths)


def _numeric_standardize(values: np.ndarray) -> np.ndarray:
    centered = values - np.mean(values)
    deviation = math.sqrt(float(np.dot(centered, centered)) / (len(values) - 1))
    return centered / deviation


def _numeric_normalize(columns: np.ndarray, start: int, candidate: np.ndarray) -> np.ndarray:
    block = columns[:, start : start + 3]
    weights = candidate.copy()
    score = block @ weights
    reference = block[:, 0] + block[:, 1] + block[:, 2]
    association = float(np.dot(score - np.mean(score), reference - np.mean(reference))) / (
        len(score) - 1
    )
    if association < -1e-15 or (abs(association) <= 1e-15 and float(np.sum(weights)) < 0.0):
        weights *= -1.0
        score *= -1.0
    centered = score - np.mean(score)
    deviation = math.sqrt(float(np.dot(centered, centered)) / (len(score) - 1))
    weights /= deviation
    score = block @ weights
    association = float(np.dot(score - np.mean(score), reference - np.mean(reference))) / (
        len(score) - 1
    )
    if association < -1e-15 or (abs(association) <= 1e-15 and float(np.sum(weights)) < 0.0):
        weights *= -1.0
    return weights


def _estimate_pls_numeric(raw: np.ndarray) -> tuple[np.ndarray, ...]:
    """Fast fixed-fixture implementation; kept separate from comparison code."""

    observations = len(raw)
    means = np.empty(raw.shape[1])
    scales = np.empty(raw.shape[1])
    centered = np.empty_like(raw)
    for column in range(raw.shape[1]):
        means[column] = np.mean(raw[:, column])
        centered[:, column] = raw[:, column] - means[column]
        scales[column] = math.sqrt(
            float(np.dot(centered[:, column], centered[:, column])) / (observations - 1)
        )
    columns = centered / scales
    weights = np.empty((4, 3))
    for construct in range(4):
        weights[construct] = _numeric_normalize(columns, construct * 3, np.ones(3))
    scores = np.empty((4, observations))
    for _ in range(MAX_ITERATIONS):
        for construct in range(4):
            block = columns[:, construct * 3 : construct * 3 + 3]
            scores[construct] = _numeric_standardize(block @ weights[construct])
        proxies = np.empty_like(scores)
        for construct in range(3):
            association = float(np.dot(scores[construct], scores[3])) / (observations - 1)
            proxies[construct] = scores[3] if association >= 0.0 else -scores[3]
        predictors = scores[:3].T
        coefficients = np.linalg.solve(predictors.T @ predictors, predictors.T @ scores[3])
        proxies[3] = _numeric_standardize(predictors @ coefficients)
        updated = np.empty_like(weights)
        for construct in range(4):
            block = columns[:, construct * 3 : construct * 3 + 3]
            candidate = block.T @ proxies[construct] / (observations - 1)
            updated[construct] = _numeric_normalize(columns, construct * 3, candidate)
        change = float(np.max(np.abs(weights - updated)))
        weights = updated
        if change <= CONVERGENCE_TOLERANCE:
            break
    else:
        return means, scales, weights, scores, np.empty((0, 0)), np.empty(0)
    for construct in range(4):
        block = columns[:, construct * 3 : construct * 3 + 3]
        scores[construct] = _numeric_standardize(block @ weights[construct])
    loadings = np.empty((4, 3))
    for construct in range(4):
        block = columns[:, construct * 3 : construct * 3 + 3]
        loadings[construct] = block.T @ scores[construct] / (observations - 1)
    predictors = scores[:3].T
    path_values = np.linalg.solve(predictors.T @ predictors, predictors.T @ scores[3])
    return means, scales, weights, scores, loadings, path_values


if njit is not None:
    _numeric_standardize = njit(cache=False)(_numeric_standardize)
    _numeric_normalize = njit(cache=False)(_numeric_normalize)
    _estimate_pls_numeric = njit(cache=False)(_estimate_pls_numeric)


def estimate_pls(raw: np.ndarray) -> Fit:
    means, scales, weights, scores, loadings, path_values = _estimate_pls_numeric(raw)
    if loadings.size == 0:
        raise RuntimeError("independent PLS reference did not converge")
    paths = {path: float(path_values[index]) for index, path in enumerate(PATHS)}
    return Fit(
        len(raw),
        np.asarray(means),
        np.asarray(scales),
        [np.asarray(row) for row in weights],
        [np.asarray(row) for row in scores],
        [np.asarray(row) for row in loadings],
        paths,
    )


def effective_scores(raw: np.ndarray, fit: Fit, construct_index: int) -> np.ndarray:
    block = BLOCKS[construct_index]
    return raw[:, block] @ (fit.weights[construct_index] / fit.scales[block])


def align_to_pooled(raw_pooled: np.ndarray, pooled: Fit, candidate: Fit) -> Fit:
    signs: dict[str, float] = {}
    for construct_index, (construct, _) in enumerate(CONSTRUCTS):
        signs[construct] = (
            -1.0
            if correlation(
                effective_scores(raw_pooled, pooled, construct_index),
                effective_scores(raw_pooled, candidate, construct_index),
            )
            < 0.0
            else 1.0
        )
    candidate.weights = [
        weights * signs[construct] for weights, (construct, _) in zip(candidate.weights, CONSTRUCTS)
    ]
    candidate.loadings = [
        loadings * signs[construct]
        for loadings, (construct, _) in zip(candidate.loadings, CONSTRUCTS)
    ]
    candidate.scores = [
        scores * signs[construct] for scores, (construct, _) in zip(candidate.scores, CONSTRUCTS)
    ]
    candidate.paths = {
        (source, target): value * signs[source] * signs[target]
        for (source, target), value in candidate.paths.items()
    }
    return candidate


def measurement_values(fit: Fit) -> dict[tuple[str, str, str], float]:
    values: dict[tuple[str, str, str], float] = {}
    for construct_index, (construct, indicators) in enumerate(CONSTRUCTS):
        for within, indicator in enumerate(indicators):
            values[("loading", construct, indicator)] = float(fit.loadings[construct_index][within])
            values[("weight", construct, indicator)] = float(fit.weights[construct_index][within])
    return values


def micom_statistics(
    raw_pooled: np.ndarray,
    labels: np.ndarray,
    group_a: Fit,
    group_b: Fit,
    pooled: Fit,
) -> dict[str, dict[str, float]]:
    statistics: dict[str, dict[str, float]] = {}
    for construct_index, (construct, _) in enumerate(CONSTRUCTS):
        score_a = effective_scores(raw_pooled, group_a, construct_index)
        score_b = effective_scores(raw_pooled, group_b, construct_index)
        pooled_score = pooled.scores[construct_index]
        left = pooled_score[labels == 0]
        right = pooled_score[labels == 1]
        variance_a = sample_variance(left)
        variance_b = sample_variance(right)
        statistics[construct] = {
            "compositional_correlation": max(-1.0, min(1.0, correlation(score_a, score_b))),
            "mean_a": mean(left),
            "mean_b": mean(right),
            "mean_difference": mean(left) - mean(right),
            "variance_a": variance_a,
            "variance_b": variance_b,
            "variance_difference": math.log(variance_a / variance_b),
        }
    return statistics


def deterministic_labels(labels: np.ndarray, seed: int, replicate: int) -> np.ndarray:
    values = labels.tolist()
    state = (seed ^ (((replicate + 1) * GOLDEN_64) & MASK_64)) & MASK_64
    for index in range(len(values) - 1, 0, -1):
        state = (state * LCG_MULTIPLIER + LCG_INCREMENT) & MASK_64
        swap = state % (index + 1)
        values[index], values[swap] = values[swap], values[index]
    return np.asarray(values, dtype=int)


def type7_quantile(values: list[float], probability: float) -> float:
    ordered = sorted(values)
    position = max(0.0, min(1.0, probability)) * (len(ordered) - 1)
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return ordered[lower]
    fraction = position - lower
    return ordered[lower] + fraction * (ordered[upper] - ordered[lower])


def lower_tail_p(values: list[float], observed: float) -> float:
    return (sum(value <= observed for value in values) + 1.0) / (len(values) + 1.0)


def two_tail_p(values: list[float], observed: float) -> float:
    lower = (sum(value <= observed for value in values) + 1.0) / (len(values) + 1.0)
    upper = (sum(value >= observed for value in values) + 1.0) / (len(values) + 1.0)
    return min(1.0, 2.0 * min(lower, upper))


def permutation_reference(raw: np.ndarray, labels: np.ndarray, samples: int) -> dict[str, Any]:
    pooled = estimate_pls(raw)
    original_a = align_to_pooled(raw, pooled, estimate_pls(raw[labels == 0]))
    original_b = align_to_pooled(raw, pooled, estimate_pls(raw[labels == 1]))
    original_paths = {
        key: original_a.paths[key] - original_b.paths[key] for key in original_a.paths
    }
    measurement_a = measurement_values(original_a)
    measurement_b = measurement_values(original_b)
    original_measurements = {
        key: measurement_a[key] - measurement_b[key] for key in measurement_a
    }
    observed_micom = micom_statistics(raw, labels, original_a, original_b, pooled)
    path_distributions = {key: [] for key in original_paths}
    measurement_distributions = {key: [] for key in original_measurements}
    correlation_distributions = {construct: [] for construct, _ in CONSTRUCTS}
    mean_distributions = {construct: [] for construct, _ in CONSTRUCTS}
    variance_distributions = {construct: [] for construct, _ in CONSTRUCTS}
    usable = 0
    attempted = 0
    failed = 0
    maximum_attempts = samples + max(samples // 5, 100)
    while usable < samples and attempted < maximum_attempts:
        replicate = attempted
        attempted += 1
        shuffled = deterministic_labels(labels, SEED ^ 0x9E37, replicate)
        try:
            fit_a = align_to_pooled(raw, pooled, estimate_pls(raw[shuffled == 0]))
            fit_b = align_to_pooled(raw, pooled, estimate_pls(raw[shuffled == 1]))
        except (RuntimeError, np.linalg.LinAlgError):
            failed += 1
            continue
        for key in path_distributions:
            path_distributions[key].append(fit_a.paths[key] - fit_b.paths[key])
        perm_a = measurement_values(fit_a)
        perm_b = measurement_values(fit_b)
        for key in measurement_distributions:
            measurement_distributions[key].append(perm_a[key] - perm_b[key])
        perm_micom = micom_statistics(raw, shuffled, fit_a, fit_b, pooled)
        for construct, values in perm_micom.items():
            correlation_distributions[construct].append(values["compositional_correlation"])
            mean_distributions[construct].append(values["mean_difference"])
            variance_distributions[construct].append(values["variance_difference"])
        usable += 1
    if usable != samples:
        raise RuntimeError(
            f"independent reference produced {usable} usable fits after {attempted} attempts"
        )
    paths = {
        key: {
            "original_difference": observed,
            "empirical_p_value_two_sided": (
                sum(abs(value) >= abs(observed) for value in path_distributions[key]) + 1.0
            )
            / (samples + 1.0),
            "percentile_rank": sum(value <= observed for value in path_distributions[key]) / samples,
        }
        for key, observed in original_paths.items()
    }
    measurements = {
        key: {
            "original_difference": observed,
            "empirical_p_value_two_sided": (
                sum(abs(value) >= abs(observed) for value in measurement_distributions[key]) + 1.0
            )
            / (samples + 1.0),
            "percentile_rank": sum(value <= observed for value in measurement_distributions[key])
            / samples,
        }
        for key, observed in original_measurements.items()
    }
    alpha = 1.0 - CONFIDENCE_LEVEL
    micom: dict[str, dict[str, Any]] = {}
    for construct, observed in observed_micom.items():
        correlation_values = correlation_distributions[construct]
        mean_values = mean_distributions[construct]
        variance_values = variance_distributions[construct]
        correlation_lower = type7_quantile(correlation_values, alpha)
        mean_lower = type7_quantile(mean_values, alpha / 2.0)
        mean_upper = type7_quantile(mean_values, 1.0 - alpha / 2.0)
        variance_lower = type7_quantile(variance_values, alpha / 2.0)
        variance_upper = type7_quantile(variance_values, 1.0 - alpha / 2.0)
        compositional = observed["compositional_correlation"] + 1e-12 >= correlation_lower
        equal_means = mean_lower <= observed["mean_difference"] <= mean_upper
        equal_variances = variance_lower <= observed["variance_difference"] <= variance_upper
        micom[construct] = {
            **observed,
            "compositional_p_value": lower_tail_p(
                correlation_values, observed["compositional_correlation"]
            ),
            "compositional_correlation_lower": correlation_lower,
            "mean_p_value": two_tail_p(mean_values, observed["mean_difference"]),
            "mean_difference_lower": mean_lower,
            "mean_difference_upper": mean_upper,
            "variance_p_value": two_tail_p(
                variance_values, observed["variance_difference"]
            ),
            "variance_difference_lower": variance_lower,
            "variance_difference_upper": variance_upper,
            "equal_means": equal_means,
            "equal_variances": equal_variances,
            "partial_invariance": compositional,
            "full_invariance": compositional and equal_means and equal_variances,
        }
    return {
        "pooled": pooled,
        "groups": {"A": original_a, "B": original_b},
        "paths": paths,
        "measurements": measurements,
        "micom": micom,
        "attempted_permutations": attempted,
        "failed_permutations": failed,
    }


def write_dataset() -> tuple[np.ndarray, np.ndarray]:
    RESULTS.mkdir(parents=True, exist_ok=True)
    rng = random.Random(SEED)
    repeated_r: list[tuple[float, tuple[float, float, float]]] = []
    for _ in range(N_PER_GROUP):
        latent = rng.gauss(0.0, 1.0)
        errors = (rng.gauss(0.0, 0.10), rng.gauss(0.0, 0.12), rng.gauss(0.0, 0.14))
        repeated_r.append((latent, errors))
    rows: list[dict[str, str]] = []
    for group in ("A", "B"):
        for index in range(N_PER_GROUP):
            latent_x = rng.gauss(0.0 if group == "A" else 0.85, 1.0)
            latent_z = rng.gauss(0.0, 1.0 if group == "A" else 1.65)
            latent_r, r_errors = repeated_r[index]
            latent_y = (
                (0.72 if group == "A" else 0.25) * latent_x
                + (0.28 if group == "A" else 0.72) * latent_z
                + 0.18 * latent_r
                + rng.gauss(0.0, 0.24)
            )
            values = {
                "x1": 0.95 * latent_x + rng.gauss(0.0, 0.10),
                "x2": 0.82 * latent_x + rng.gauss(0.0, 0.13),
                "x3": 0.70 * latent_x + rng.gauss(0.0, 0.17),
                "z1": 0.94 * latent_z + rng.gauss(0.0, 0.10),
                "z2": 0.80 * latent_z + rng.gauss(0.0, 0.14),
                "z3": 0.68 * latent_z + rng.gauss(0.0, 0.18),
                "r1": 0.96 * latent_r + r_errors[0],
                "r2": 0.82 * latent_r + r_errors[1],
                "r3": 0.70 * latent_r + r_errors[2],
            }
            if group == "A":
                values.update(
                    {
                        "y1": 0.94 * latent_y + rng.gauss(0.0, 0.11),
                        "y2": 0.80 * latent_y + rng.gauss(0.0, 0.14),
                        "y3": 0.67 * latent_y + rng.gauss(0.0, 0.17),
                    }
                )
            else:
                nuisance = rng.gauss(0.0, 1.0)
                values.update(
                    {
                        "y1": 0.93 * latent_y + rng.gauss(0.0, 0.11),
                        "y2": 0.28 * latent_y + 0.80 * nuisance + rng.gauss(0.0, 0.12),
                        "y3": -0.62 * latent_y + rng.gauss(0.0, 0.17),
                    }
                )
            rows.append({"group": group, **{key: f"{value:.12f}" for key, value in values.items()}})
    with DATA.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=["group", *INDICATORS])
        writer.writeheader()
        writer.writerows(rows)
    raw = np.asarray([[float(row[indicator]) for indicator in INDICATORS] for row in rows])
    labels = np.asarray([0 if row["group"] == "A" else 1 for row in rows], dtype=int)
    return raw, labels


def cli_path() -> Path:
    configured = os.environ.get("QUICKPLS_CLI_PATH")
    candidate = Path(configured) if configured else ROOT / "target" / "release" / "qpls.exe"
    if not candidate.is_file():
        raise RuntimeError(
            f"QuickPLS CLI not found at {candidate}; build it separately or set QUICKPLS_CLI_PATH"
        )
    return candidate


def run_cli(arguments: list[str], **kwargs: Any) -> subprocess.CompletedProcess[str]:
    return subprocess.run([str(cli_path()), *arguments], cwd=ROOT, check=True, text=True, **kwargs)


def relative(path: Path) -> str:
    return str(path.relative_to(ROOT))


def write_recipe(fingerprint: str, samples: int) -> None:
    payload = {
        "schema_version": 2,
        "id": "00000000-0000-0000-0000-000000000281",
        "created_at": "2026-08-11T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": "00000000-0000-0000-0000-000000000282",
            "name": "MICOM v2 independent reference",
            "constructs": [
                {
                    "id": construct,
                    "name": construct.upper(),
                    "short_name": construct.upper(),
                    "mode": "reflective",
                    "indicators": list(indicators),
                }
                for construct, indicators in CONSTRUCTS
            ],
            "paths": [{"source": source, "target": target} for source, target in PATHS],
        },
        "settings": {
            "method": "mga",
            "weighting_scheme": "path",
            "tolerance": CONVERGENCE_TOLERANCE,
            "max_iterations": MAX_ITERATIONS,
            "bootstrap_samples": 0,
            "seed": SEED,
            "confidence_level": CONFIDENCE_LEVEL,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
        },
        "metadata": {
            "fixture": "independent_micom_v2_reference",
            "group_methods": "micom,mga_permutation",
            "group_permutation_samples": str(samples),
            "micom_configural_confirmed": "true",
            "mga_group_column": "group",
            "mga_group_a": "A",
            "mga_group_b": "B",
        },
    }
    RECIPE.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")


def execute_quickpls(samples: int) -> dict[str, Any]:
    project = RESULTS / "micom_v2_reference.fingerprint.qpls"
    project.unlink(missing_ok=True)
    try:
        run_cli(
            ["import", relative(DATA), relative(project), "--name", "micom_v2_reference"],
            stdout=subprocess.DEVNULL,
        )
        inspected = json.loads(
            run_cli(["inspect", relative(project), "--json"], capture_output=True).stdout
        )
        fingerprint = inspected["datasets"][0]["fingerprint"]
    finally:
        project.unlink(missing_ok=True)
    write_recipe(fingerprint, samples)
    run_cli(
        [
            "run",
            relative(RECIPE),
            "--data",
            relative(DATA),
            "--output",
            relative(QUICKPLS),
            "--allow-experimental",
        ],
        stdout=subprocess.DEVNULL,
    )
    return json.loads(QUICKPLS.read_text(encoding="utf-8"))


def max_delta(expected: dict[Any, float], actual: dict[Any, float]) -> float:
    if expected.keys() != actual.keys():
        return math.inf
    return max((abs(expected[key] - actual[key]) for key in expected), default=0.0)


def compare_quickpls(reference: dict[str, Any], run: dict[str, Any], samples: int) -> dict[str, Any]:
    estimation = run.get("payload", {}).get("estimation", {})
    mga = estimation.get("mga") or {}
    permutation = estimation.get("mga_permutation") or {}
    micom = estimation.get("micom") or {}
    quick_groups = {item.get("group"): item for item in mga.get("groups", [])}
    group_deltas: dict[str, float] = {}
    group_shapes = True
    for group_name, expected in reference["groups"].items():
        actual = quick_groups.get(group_name, {})
        expected_paths = {f"{a}->{b}": value for (a, b), value in expected.paths.items()}
        actual_paths = {
            f"{item['source']}->{item['target']}": float(item["coefficient"])
            for item in actual.get("paths", [])
        }
        expected_outer = measurement_values(expected)
        actual_outer: dict[tuple[str, str, str], float] = {}
        for item in actual.get("outer_estimates", []):
            actual_outer[("loading", item["construct"], item["indicator"])] = float(item["loading"])
            actual_outer[("weight", item["construct"], item["indicator"])] = float(item["weight"])
        expected_transforms = {
            f"{indicator}.{value_name}": float(value)
            for indicator, transform_index in INDICATOR_INDEX.items()
            for value_name, value in (
                ("mean", expected.means[transform_index]),
                ("scale", expected.scales[transform_index]),
            )
        }
        actual_transforms = {
            f"{item['indicator']}.{value_name}": float(item[value_name])
            for item in actual.get("transforms", [])
            for value_name in ("mean", "scale")
        }
        group_deltas[group_name] = max(
            max_delta(expected_paths, actual_paths),
            max_delta(expected_outer, actual_outer),
            max_delta(expected_transforms, actual_transforms),
        )
        group_shapes = group_shapes and actual.get("observations") == expected.observations

    expected_paths = {
        f"{a}->{b}": values for (a, b), values in reference["paths"].items()
    }
    actual_paths = {
        f"{item['source']}->{item['target']}": item for item in permutation.get("comparisons", [])
    }
    path_delta = max(
        (
            abs(float(actual_paths[key].get(field)) - float(values[field]))
            for key, values in expected_paths.items()
            for field in (
                "original_difference",
                "empirical_p_value_two_sided",
                "percentile_rank",
            )
            if key in actual_paths and actual_paths[key].get(field) is not None
        ),
        default=math.inf,
    )
    path_shape = expected_paths.keys() == actual_paths.keys()

    expected_measurements = reference["measurements"]
    actual_measurements = {
        (str(item["parameter"]).removeprefix("outer_"), item["construct"], item["indicator"]): item
        for item in permutation.get("measurement_comparisons", [])
    }
    measurement_delta = max(
        (
            abs(float(actual_measurements[key].get(field)) - float(values[field]))
            for key, values in expected_measurements.items()
            for field in (
                "original_difference",
                "empirical_p_value_two_sided",
                "percentile_rank",
            )
            if key in actual_measurements and actual_measurements[key].get(field) is not None
        ),
        default=math.inf,
    )
    measurement_shape = expected_measurements.keys() == actual_measurements.keys()

    actual_micom = {item["construct"]: item for item in micom.get("constructs", [])}
    numeric_micom_fields = (
        "compositional_correlation",
        "compositional_p_value",
        "compositional_correlation_lower",
        "mean_a",
        "mean_b",
        "mean_difference",
        "mean_p_value",
        "mean_difference_lower",
        "mean_difference_upper",
        "variance_a",
        "variance_b",
        "variance_difference",
        "variance_p_value",
        "variance_difference_lower",
        "variance_difference_upper",
    )
    micom_delta = max(
        (
            abs(float(actual_micom[construct].get(field)) - float(values[field]))
            for construct, values in reference["micom"].items()
            for field in numeric_micom_fields
            if construct in actual_micom and actual_micom[construct].get(field) is not None
        ),
        default=math.inf,
    )
    micom_shape = reference["micom"].keys() == actual_micom.keys()
    micom_flags = micom_shape and all(
        actual_micom[construct].get(field) == values[field]
        for construct, values in reference["micom"].items()
        for field in ("equal_means", "equal_variances", "partial_invariance", "full_invariance")
    )
    micom_flags = micom_flags and all(
        actual_micom[construct].get("configural_invariance") is True
        for construct in reference["micom"]
    )
    micom_groups = {
        item.get("group"): item.get("observations") for item in micom.get("groups", [])
    }
    provenance = str(run.get("provenance", {}).get("method_version", ""))
    checks = {
        "method_versions": estimation.get("method_version") == "pls_mga_two_group_v2"
        and mga.get("method_version") == "pls_mga_two_group_v2"
        and permutation.get("method_version") == "pls_mga_permutation_v2"
        and micom.get("method_version") == "micom_v2",
        "provenance_versions": all(
            token in provenance
            for token in ("pls_mga_two_group_v2", "pls_mga_permutation_v2", "micom_v2")
        ),
        "analysis_identity": mga.get("group_column") == "group"
        and permutation.get("group_column") == "group"
        and micom.get("group_column") == "group"
        and micom_groups == {"A": N_PER_GROUP, "B": N_PER_GROUP}
        and abs(float(micom.get("confidence_level", math.nan)) - CONFIDENCE_LEVEL) <= TOLERANCE,
        "group_contract": group_shapes and max(group_deltas.values(), default=math.inf) <= TOLERANCE,
        "path_permutation_agreement": path_shape and path_delta <= TOLERANCE,
        "measurement_permutation_agreement": measurement_shape and measurement_delta <= TOLERANCE,
        "micom_numeric_agreement": micom_shape and micom_delta <= TOLERANCE,
        "micom_decision_agreement": micom_flags,
        "permutation_accounting": permutation.get("permutation_samples") == samples
        and permutation.get("usable_permutations") == samples
        and permutation.get("attempted_permutations") == reference["attempted_permutations"]
        and permutation.get("failed_permutations") == reference["failed_permutations"]
        and micom.get("permutation_samples") == samples
        and micom.get("usable_permutations") == samples
        and micom.get("attempted_permutations") == reference["attempted_permutations"]
        and micom.get("failed_permutations") == reference["failed_permutations"],
    }
    return {
        "passed": all(checks.values()),
        "checks": checks,
        "max_group_delta": max(group_deltas.values()) if group_deltas and all(math.isfinite(value) for value in group_deltas.values()) else None,
        "max_path_permutation_delta": path_delta if math.isfinite(path_delta) else None,
        "max_measurement_permutation_delta": measurement_delta if math.isfinite(measurement_delta) else None,
        "max_micom_delta": micom_delta if math.isfinite(micom_delta) else None,
    }


def compact_reference(reference: dict[str, Any]) -> dict[str, Any]:
    return {
        "attempted_permutations": reference["attempted_permutations"],
        "failed_permutations": reference["failed_permutations"],
        "groups": {
            group: {
                "observations": fit.observations,
                "paths": {f"{source}->{target}": value for (source, target), value in fit.paths.items()},
                "outer": {
                    f"{parameter}.{construct}.{indicator}": value
                    for (parameter, construct, indicator), value in measurement_values(fit).items()
                },
            }
            for group, fit in reference["groups"].items()
        },
        "path_permutation": {
            f"{source}->{target}": values
            for (source, target), values in reference["paths"].items()
        },
        "measurement_permutation": {
            f"{parameter}.{construct}.{indicator}": values
            for (parameter, construct, indicator), values in reference["measurements"].items()
        },
        "micom": reference["micom"],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--permutations", type=int, default=PROMOTION_PERMUTATIONS)
    parser.add_argument("--run-quickpls", action="store_true")
    parser.add_argument("--quickpls-json", type=Path)
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    if args.permutations < 19:
        raise SystemExit("--permutations must be at least 19")
    if args.run_quickpls and args.quickpls_json:
        raise SystemExit("choose --run-quickpls or --quickpls-json, not both")
    raw, labels = write_dataset()
    reference = permutation_reference(raw, labels, args.permutations)
    quickpls_run: dict[str, Any] | None = None
    source: str | None = None
    quickpls_error: str | None = None
    if args.run_quickpls:
        source = relative(QUICKPLS)
        try:
            quickpls_run = execute_quickpls(args.permutations)
        except (RuntimeError, subprocess.CalledProcessError) as error:
            quickpls_error = str(error)
    elif args.quickpls_json:
        source = str(args.quickpls_json)
        try:
            quickpls_run = json.loads(args.quickpls_json.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            quickpls_error = str(error)
    comparison = (
        compare_quickpls(reference, quickpls_run, args.permutations) if quickpls_run else None
    )
    micom_values = list(reference["micom"].values())
    fixture_checks = {
        "full_invariance_control_present": any(item["full_invariance"] for item in micom_values),
        "location_shift_detected": any(not item["equal_means"] for item in micom_values),
        "dispersion_shift_detected": any(not item["equal_variances"] for item in micom_values),
        "composition_shift_detected": any(not item["partial_invariance"] for item in micom_values),
        "path_and_measurement_distributions_present": len(reference["paths"]) == len(PATHS)
        and len(reference["measurements"]) == 2 * len(INDICATORS),
    }
    reference_passed = all(fixture_checks.values())
    promotion_sample_size = args.permutations >= PROMOTION_PERMUTATIONS
    passed = reference_passed and promotion_sample_size and comparison is not None and comparison["passed"]
    report = {
        "schema_version": 1,
        "target": "micom_v2_and_pls_mga_permutation_v2_independent_reference",
        "reference_implementation": "independent_numpy_path_weighting_and_permutation",
        "seed": SEED,
        "confidence_level": CONFIDENCE_LEVEL,
        "permutation_samples": args.permutations,
        "promotion_sample_size": promotion_sample_size,
        "reference_checks": fixture_checks,
        "reference_passed": reference_passed,
        "quickpls_source": source,
        "quickpls_error": quickpls_error,
        "quickpls_comparison": comparison,
        "passed": passed,
        "reference": compact_reference(reference),
        "note": (
            "Passing promotion evidence requires 5000 or more permutations and a current QuickPLS "
            "comparison; reference-only execution is intentionally non-promotable."
        ),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, allow_nan=False) + "\n", encoding="utf-8")
    print(
        json.dumps(
            {
                "reference_passed": reference_passed,
                "quickpls_compared": comparison is not None,
                "quickpls_error": quickpls_error,
                "passed": passed,
                "output": str(args.output),
            },
            indent=2,
        )
    )
    comparison_requested = args.run_quickpls or args.quickpls_json is not None
    return 0 if reference_passed and (
        (not comparison_requested and comparison is None)
        or (comparison is not None and comparison["passed"])
    ) else 1


if __name__ == "__main__":
    raise SystemExit(main())
