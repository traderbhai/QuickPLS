#!/usr/bin/env python3
"""Historical combined MICOM/permutation-MGA v3 diagnostic reference.

This validation-only module does not import QuickPLS product code.  It reuses
the independently implemented NumPy PLS primitives from ``micom_v2_reference``
but owns the v3 canonical-plan, retry, and A/B metamorphic checks.  The small
finite-plan gate is exhaustive; the numerical gate re-estimates both groups
for every usable permutation in both requested directions.

Important: this combined diagnostic retains the legacy replacement-retry policy
(``attempted`` may exceed the requested usable count).  It is therefore not a
QualificationSpec V2 oracle and can never declare promotion readiness.  The
MICOM-only, exactly-once no-retry contract lives in ``micom_v3_oracle.py``.
"""

from __future__ import annotations

import argparse
import csv
import itertools
import json
import math
import os
import subprocess
from pathlib import Path
from typing import Any

import numpy as np

import micom_v2_reference as independent


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
DATA = RESULTS / "micom_v2_reference.csv"
SOURCE_RECIPE = RESULTS / "micom_v2_reference.recipe.json"
OUTPUT = RESULTS / "micom_mga_v3_reference_report.json"
BASE_RECIPE = RESULTS / "micom_mga_v3_reference.recipe.json"
SWAP_RECIPE = RESULTS / "micom_mga_v3_reference_swapped.recipe.json"
BASE_QUICKPLS = RESULTS / "micom_mga_v3_reference.quickpls.json"
SWAP_QUICKPLS = RESULTS / "micom_mga_v3_reference_swapped.quickpls.json"

SEED = independent.SEED
CONFIDENCE_LEVEL = independent.CONFIDENCE_LEVEL
PROMOTION_PERMUTATIONS = 5_000
TOLERANCE = 2e-6


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )


def load_fixture() -> tuple[np.ndarray, np.ndarray]:
    with DATA.open(newline="", encoding="utf-8-sig") as handle:
        rows = list(csv.DictReader(handle))
    raw = np.asarray(
        [[float(row[indicator]) for indicator in independent.INDICATORS] for row in rows],
        dtype=float,
    )
    labels = np.asarray([0 if row["group"] == "A" else 1 for row in rows], dtype=int)
    return raw, labels


def micom_statistics(
    raw: np.ndarray,
    labels: np.ndarray,
    group_a: independent.Fit,
    group_b: independent.Fit,
    pooled: independent.Fit,
) -> dict[str, dict[str, float]]:
    result: dict[str, dict[str, float]] = {}
    for construct_index, (construct, _) in enumerate(independent.CONSTRUCTS):
        score_a = independent.effective_scores(raw, group_a, construct_index)
        score_b = independent.effective_scores(raw, group_b, construct_index)
        pooled_score = pooled.scores[construct_index]
        left = pooled_score[labels == 0]
        right = pooled_score[labels == 1]
        variance_a = independent.sample_variance(left)
        variance_b = independent.sample_variance(right)
        result[construct] = {
            "compositional_correlation": max(
                -1.0, min(1.0, independent.correlation(score_a, score_b))
            ),
            "mean_a": independent.mean(left),
            "mean_b": independent.mean(right),
            "mean_difference": independent.mean(left) - independent.mean(right),
            "variance_a": variance_a,
            "variance_b": variance_b,
            "variance_difference": math.log(variance_a) - math.log(variance_b),
        }
    return result


def numerical_reference(raw: np.ndarray, labels: np.ndarray, samples: int) -> dict[str, Any]:
    pooled = independent.estimate_pls(raw)
    original_a = independent.align_to_pooled(raw, pooled, independent.estimate_pls(raw[labels == 0]))
    original_b = independent.align_to_pooled(raw, pooled, independent.estimate_pls(raw[labels == 1]))
    paths = {key: original_a.paths[key] - original_b.paths[key] for key in original_a.paths}
    values_a = independent.measurement_values(original_a)
    values_b = independent.measurement_values(original_b)
    measurements = {key: values_a[key] - values_b[key] for key in values_a}
    observed_micom = micom_statistics(raw, labels, original_a, original_b, pooled)

    path_distribution = {key: [] for key in paths}
    measurement_distribution = {key: [] for key in measurements}
    correlation_distribution = {key: [] for key in observed_micom}
    mean_distribution = {key: [] for key in observed_micom}
    variance_distribution = {key: [] for key in observed_micom}
    usable = attempted = failed = 0
    maximum_attempts = samples + max(samples // 5, 100)
    while usable < samples and attempted < maximum_attempts:
        replicate = attempted
        attempted += 1
        shuffled = independent.deterministic_labels(labels, SEED ^ 0x9E37, replicate)
        try:
            fit_a = independent.align_to_pooled(
                raw, pooled, independent.estimate_pls(raw[shuffled == 0])
            )
            fit_b = independent.align_to_pooled(
                raw, pooled, independent.estimate_pls(raw[shuffled == 1])
            )
        except (RuntimeError, np.linalg.LinAlgError):
            failed += 1
            continue
        for key in paths:
            path_distribution[key].append(fit_a.paths[key] - fit_b.paths[key])
        perm_a = independent.measurement_values(fit_a)
        perm_b = independent.measurement_values(fit_b)
        for key in measurements:
            measurement_distribution[key].append(perm_a[key] - perm_b[key])
        for construct, values in micom_statistics(raw, shuffled, fit_a, fit_b, pooled).items():
            correlation_distribution[construct].append(values["compositional_correlation"])
            mean_distribution[construct].append(values["mean_difference"])
            variance_distribution[construct].append(values["variance_difference"])
        usable += 1
    if usable != samples:
        raise RuntimeError(
            f"independent v3 reference produced {usable} usable fits after {attempted} attempts"
        )

    def permutation_rows(
        observed: dict[Any, float], distributions: dict[Any, list[float]]
    ) -> dict[Any, dict[str, float]]:
        return {
            key: {
                "original_difference": value,
                "empirical_p_value_two_sided": (
                    sum(abs(item) >= abs(value) for item in distributions[key]) + 1.0
                )
                / (samples + 1.0),
                "percentile_rank": sum(item <= value for item in distributions[key]) / samples,
            }
            for key, value in observed.items()
        }

    alpha = 1.0 - CONFIDENCE_LEVEL
    micom: dict[str, dict[str, Any]] = {}
    for construct, observed in observed_micom.items():
        correlations = correlation_distribution[construct]
        means = mean_distribution[construct]
        variances = variance_distribution[construct]
        correlation_lower = independent.type7_quantile(correlations, alpha)
        mean_lower = independent.type7_quantile(means, alpha / 2.0)
        mean_upper = independent.type7_quantile(means, 1.0 - alpha / 2.0)
        variance_lower = independent.type7_quantile(variances, alpha / 2.0)
        variance_upper = independent.type7_quantile(variances, 1.0 - alpha / 2.0)
        partial = observed["compositional_correlation"] + 1e-12 >= correlation_lower
        equal_means = mean_lower <= observed["mean_difference"] <= mean_upper
        equal_variances = variance_lower <= observed["variance_difference"] <= variance_upper
        micom[construct] = {
            **observed,
            "compositional_p_value": independent.lower_tail_p(
                correlations, observed["compositional_correlation"]
            ),
            "mean_p_value": independent.two_tail_p(means, observed["mean_difference"]),
            "variance_p_value": independent.two_tail_p(
                variances, observed["variance_difference"]
            ),
            "partial_invariance": partial,
            "equal_means": equal_means,
            "equal_variances": equal_variances,
            "full_invariance": partial and equal_means and equal_variances,
        }
    return {
        "paths": permutation_rows(paths, path_distribution),
        "measurements": permutation_rows(measurements, measurement_distribution),
        "micom": micom,
        "usable_permutations": usable,
        "attempted_permutations": attempted,
        "failed_permutations": failed,
    }


def exhaustive_partition_reference() -> dict[str, Any]:
    checked = 0
    for observations in range(5, 10):
        values = np.asarray([((index * 7) % 11) - 4.0 for index in range(observations)])
        for group_a_size in range(2, observations - 1):
            for selected in itertools.combinations(range(observations), group_a_size):
                labels = np.ones(observations, dtype=int)
                labels[list(selected)] = 0
                swapped = 1 - labels
                difference = float(np.mean(values[labels == 0]) - np.mean(values[labels == 1]))
                reversed_difference = float(
                    np.mean(values[swapped == 0]) - np.mean(values[swapped == 1])
                )
                if difference != -reversed_difference:
                    raise AssertionError("exhaustive signed partition symmetry failed")
                checked += 1
    return {"passed": True, "partitions_checked": checked, "observation_range": [5, 9]}


def deterministic_stream_reference() -> dict[str, Any]:
    checked = failures = 0
    for first, second in [(3, 5), (5, 3), (7, 7), (10, 13), (13, 10)]:
        labels = np.asarray([0] * first + [1] * second, dtype=int)
        for seed in [0, 1, SEED, (1 << 64) - 1]:
            for replicate in range(257):
                forward = independent.deterministic_labels(labels, seed, replicate)
                reverse = independent.deterministic_labels(1 - labels, seed, replicate)
                if not np.array_equal(forward, 1 - reverse):
                    raise AssertionError("same-seed complemented stream mismatch")
                # A synthetic order-free fit-failure predicate demonstrates
                # that retry indices and failures remain coupled under swap.
                failure_forward = int(np.flatnonzero(forward == 0).sum()) % 19 == 0
                failure_reverse = int(np.flatnonzero(reverse == 1).sum()) % 19 == 0
                if failure_forward != failure_reverse:
                    raise AssertionError("failed-fit retry coupling mismatch")
                failures += int(failure_forward)
                checked += 1
    return {"passed": True, "streams_checked": checked, "synthetic_failures": failures}


def swap_checks(forward: dict[str, Any], reverse: dict[str, Any]) -> dict[str, Any]:
    signed_residuals: list[float] = []
    p_deltas: list[float] = []
    for collection in ("paths", "measurements"):
        if forward[collection].keys() != reverse[collection].keys():
            return {"passed": False, "reason": f"{collection} identities differ"}
        for key, row in forward[collection].items():
            swapped = reverse[collection][key]
            signed_residuals.append(abs(row["original_difference"] + swapped["original_difference"]))
            p_deltas.append(
                abs(row["empirical_p_value_two_sided"] - swapped["empirical_p_value_two_sided"])
            )
    decisions_match = True
    micom_p_deltas: list[float] = []
    for construct, row in forward["micom"].items():
        swapped = reverse["micom"][construct]
        signed_residuals.extend(
            [
                abs(row["mean_difference"] + swapped["mean_difference"]),
                abs(row["variance_difference"] + swapped["variance_difference"]),
            ]
        )
        for key in ("compositional_p_value", "mean_p_value", "variance_p_value"):
            micom_p_deltas.append(abs(row[key] - swapped[key]))
        decisions_match &= all(
            row[key] == swapped[key]
            for key in ("partial_invariance", "equal_means", "equal_variances", "full_invariance")
        )
    accounting_match = all(
        forward[key] == reverse[key]
        for key in ("usable_permutations", "attempted_permutations", "failed_permutations")
    )
    maximum_signed_residual = max(signed_residuals, default=math.inf)
    maximum_two_tailed_p_delta = max([*p_deltas, *micom_p_deltas], default=math.inf)
    return {
        "passed": maximum_signed_residual == 0.0
        and maximum_two_tailed_p_delta == 0.0
        and accounting_match
        and decisions_match,
        "maximum_signed_residual": maximum_signed_residual,
        "maximum_two_tailed_p_delta": maximum_two_tailed_p_delta,
        "accounting_match": accounting_match,
        "decisions_match": decisions_match,
        "forward_attempted": forward["attempted_permutations"],
        "forward_failed": forward["failed_permutations"],
        "reverse_attempted": reverse["attempted_permutations"],
        "reverse_failed": reverse["failed_permutations"],
    }


def cli_path() -> Path:
    configured = os.environ.get("QUICKPLS_CLI_PATH")
    candidate = Path(configured) if configured else ROOT / "target" / "release" / "qpls.exe"
    if not candidate.is_file():
        raise RuntimeError(f"QuickPLS CLI not found at {candidate}")
    return candidate


def execute_quickpls(samples: int, group_a: str, group_b: str, recipe_path: Path, output: Path) -> dict[str, Any]:
    scratch = RESULTS / "micom_mga_v3_reference.fingerprint.qpls"
    scratch.unlink(missing_ok=True)
    try:
        subprocess.run(
            [str(cli_path()), "import", str(DATA.relative_to(ROOT)), str(scratch.relative_to(ROOT)), "--name", "micom_mga_v3_reference"],
            cwd=ROOT,
            check=True,
            capture_output=True,
        )
        inspected = json.loads(
            subprocess.run(
                [str(cli_path()), "inspect", str(scratch.relative_to(ROOT)), "--json"],
                cwd=ROOT,
                check=True,
                capture_output=True,
                text=True,
            ).stdout
        )
    finally:
        scratch.unlink(missing_ok=True)
    base_recipe = json.loads(SOURCE_RECIPE.read_text(encoding="utf-8"))
    base_recipe["schema_version"] = 3
    base_recipe["dataset_fingerprint"] = inspected["datasets"][0]["fingerprint"]
    base_recipe["model"].setdefault("controls", [])
    base_recipe["model"].setdefault("higher_order_constructs", [])
    base_recipe["model"].setdefault("interactions", [])
    base_recipe["settings"].update(
        {"studentized_inner_samples": 0, "permutation_samples": 0, "workers": 1, "case_weight_column": None}
    )
    base_recipe["method_config"] = {
        "kind": "mga",
        "group_column": "group",
        "group_a": group_a,
        "group_b": group_b,
        "methods": ["micom", "mga_permutation"],
        "permutation_samples": samples,
        "configural_invariance_confirmed": True,
    }
    base_recipe["metadata"] = {
        "status": "validated_micom_v3_and_permutation_mga_v3_swap_coupled_scope"
    }
    write_json(recipe_path, base_recipe)
    output.unlink(missing_ok=True)
    subprocess.run(
        [
            str(cli_path()),
            "run",
            str(recipe_path.relative_to(ROOT)),
            "--data",
            str(DATA.relative_to(ROOT)),
            "--output",
            str(output.relative_to(ROOT)),
            "--allow-experimental",
        ],
        cwd=ROOT,
        check=True,
    )
    return json.loads(output.read_text(encoding="utf-8"))


def product_projection(payload: dict[str, Any]) -> dict[str, Any]:
    estimation = payload["payload"]["estimation"]
    permutation = estimation["mga_permutation"]
    micom = estimation["micom"]
    return {
        "method_version": estimation["method_version"],
        "permutation_method_version": permutation["method_version"],
        "micom_method_version": micom["method_version"],
        "paths": {
            (row["source"], row["target"]): {
                key: row[key]
                for key in ("original_difference", "empirical_p_value_two_sided", "percentile_rank")
            }
            for row in permutation["comparisons"]
        },
        "measurements": {
            (row["parameter"].removeprefix("outer_"), row["construct"], row["indicator"]): {
                key: row[key]
                for key in ("original_difference", "empirical_p_value_two_sided", "percentile_rank")
            }
            for row in permutation["measurement_comparisons"]
        },
        "micom": {row["construct"]: row for row in micom["constructs"]},
        "usable_permutations": permutation["usable_permutations"],
        "attempted_permutations": permutation["attempted_permutations"],
        "failed_permutations": permutation["failed_permutations"],
    }


def product_comparison(reference: dict[str, Any], product: dict[str, Any]) -> dict[str, Any]:
    projected = product_projection(product)
    deltas: list[float] = []
    shapes = reference["paths"].keys() == projected["paths"].keys()
    shapes &= reference["measurements"].keys() == projected["measurements"].keys()
    for collection in ("paths", "measurements"):
        for key, expected in reference[collection].items():
            actual = projected[collection].get(key, {})
            for field, expected_value in expected.items():
                if field in actual:
                    deltas.append(abs(float(expected_value) - float(actual[field])))
                else:
                    shapes = False
    for construct, expected in reference["micom"].items():
        actual = projected["micom"].get(construct, {})
        if not actual:
            shapes = False
            continue
        for field in (
            "compositional_correlation",
            "compositional_p_value",
            "mean_difference",
            "mean_p_value",
            "variance_difference",
            "variance_p_value",
        ):
            deltas.append(abs(float(expected[field]) - float(actual[field])))
    accounting = all(
        reference[key] == projected[key]
        for key in ("usable_permutations", "attempted_permutations", "failed_permutations")
    )
    versions = (
        projected["method_version"] == "pls_mga_two_group_v3"
        and projected["permutation_method_version"] == "pls_mga_permutation_v3"
        and projected["micom_method_version"] == "micom_v3"
    )
    maximum_delta = max(deltas, default=math.inf)
    return {
        "passed": shapes and accounting and versions and maximum_delta <= TOLERANCE,
        "shapes_match": shapes,
        "accounting_matches": accounting,
        "versions_match": versions,
        "maximum_numerical_delta": maximum_delta,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--permutations", type=int, default=99)
    parser.add_argument("--run-quickpls", action="store_true")
    args = parser.parse_args()
    if args.permutations < 19:
        raise SystemExit("--permutations must be at least 19")
    if args.run_quickpls and args.permutations < PROMOTION_PERMUTATIONS:
        raise SystemExit("QuickPLS comparison requires at least 5000 permutations")

    raw, forward_labels = load_fixture()
    reverse_labels = 1 - forward_labels
    forward = numerical_reference(raw, forward_labels, args.permutations)
    reverse = numerical_reference(raw, reverse_labels, args.permutations)
    exhaustive = exhaustive_partition_reference()
    streams = deterministic_stream_reference()
    swap = swap_checks(forward, reverse)
    comparisons: dict[str, Any] | None = None
    if args.run_quickpls:
        forward_product = execute_quickpls(
            args.permutations, "A", "B", BASE_RECIPE, BASE_QUICKPLS
        )
        reverse_product = execute_quickpls(
            args.permutations, "B", "A", SWAP_RECIPE, SWAP_QUICKPLS
        )
        comparisons = {
            "forward": product_comparison(forward, forward_product),
            "reverse": product_comparison(reverse, reverse_product),
            "product_swap": swap_checks(
                product_projection(forward_product), product_projection(reverse_product)
            ),
        }
    reference_passed = exhaustive["passed"] and streams["passed"] and swap["passed"]
    comparison_passed = comparisons is None or all(item["passed"] for item in comparisons.values())
    report = {
        "schema_version": 1,
        "target": "micom_v3_and_pls_mga_permutation_v3_swap_coupled_independent_reference",
        "method_versions": {
            "mga": "pls_mga_two_group_v3",
            "permutation_mga": "pls_mga_permutation_v3",
            "micom": "micom_v3",
        },
        "permutation_samples": args.permutations,
        "seed": SEED,
        "reference_passed": reference_passed,
        "quickpls_comparison": comparisons,
        "promotion_sample_size": args.permutations >= PROMOTION_PERMUTATIONS,
        "work_evidence_only": True,
        "qualification_ready": False,
        "promotion_ready": False,
        "passed": reference_passed and comparison_passed,
        "exhaustive_partition_reference": exhaustive,
        "deterministic_stream_reference": streams,
        "numerical_swap_reference": swap,
        "independence": "NumPy PLS reference plus independently specified canonical permutation coupling; no QuickPLS product module import",
        "retry_policy": "legacy_fill_usable_target",
        "qualification_blockers": [
            "The combined diagnostic replaces failed fits until it reaches the requested usable count.",
            "MICOM and structural-path permutation MGA require separate scientific contracts and qualification evidence.",
            "Use validation/micom_v3_oracle.py for the frozen MICOM-only exactly-once no-retry work contract.",
        ],
    }
    write_json(OUTPUT, report)
    print(json.dumps(report, indent=2, sort_keys=True, allow_nan=False))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
