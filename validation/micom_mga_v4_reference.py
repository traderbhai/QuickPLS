#!/usr/bin/env python3
"""Independent fixed-plan oracle for combined MICOM/permutation-MGA v4.

The oracle imports no QuickPLS product module. It reuses the independent NumPy
PLS estimator from ``micom_v2_reference`` and independently implements the v4
canonical case-token, indexed partition, no-retry ledger, MICOM, and MGA rules.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import subprocess
import struct
from collections import Counter
from pathlib import Path
from typing import Any

import numpy as np

import micom_mga_v3_reference as historical


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "micom_mga_v4_reference_report.json"
BASE_RECIPE = RESULTS / "micom_mga_v4_reference.recipe.json"
SWAP_RECIPE = RESULTS / "micom_mga_v4_reference_swapped.recipe.json"
BASE_QUICKPLS = RESULTS / "micom_mga_v4_reference.quickpls.json"
SWAP_QUICKPLS = RESULTS / "micom_mga_v4_reference_swapped.quickpls.json"
SEED = historical.SEED
CONFIDENCE_LEVEL = historical.CONFIDENCE_LEVEL
PROMOTION_PERMUTATIONS = 5_000
TOLERANCE = 2e-6


def canonical_cases(
    raw: np.ndarray, actual_groups: list[str], requested_groups: tuple[str, str]
) -> tuple[np.ndarray, np.ndarray, list[str]]:
    cases: list[tuple[str, int, int]] = []
    for row, group in enumerate(actual_groups):
        digest = hashlib.sha256()
        digest.update(b"quickpls-micom-v3.1-case-token\0")
        digest.update(group.encode())
        for indicator, value in zip(historical.independent.INDICATORS, raw[row]):
            digest.update(indicator.encode())
            digest.update(struct.pack("<d", float(value)))
        cases.append((digest.hexdigest(), row, requested_groups.index(group)))
    cases.sort(key=lambda item: (item[0], item[2], item[1]))
    duplicates: Counter[str] = Counter()
    rows: list[int] = []
    labels: list[int] = []
    tokens: list[str] = []
    for token, row, label in cases:
        rows.append(row)
        labels.append(label)
        tokens.append(f"{token}#{duplicates[token]}")
        duplicates[token] += 1
    return raw[rows], np.asarray(labels, dtype=int), tokens


def partition_labels(
    tokens: list[str], requested_groups: tuple[str, str], group_sizes: tuple[int, int],
    seed: int, replicate: int,
) -> tuple[np.ndarray, str]:
    canonical_group = int(requested_groups[1] < requested_groups[0])
    ranked = []
    for index, token in enumerate(tokens):
        digest = hashlib.sha256()
        digest.update(b"quickpls-micom-v3.1-partition\0")
        digest.update(struct.pack("<Q", seed))
        digest.update(struct.pack("<Q", replicate))
        digest.update(token.encode())
        ranked.append((digest.digest(), token, index))
    ranked.sort(key=lambda item: (item[0], item[1]))
    selected = {item[2] for item in ranked[: group_sizes[canonical_group]]}
    selected_tokens = sorted(tokens[index] for index in selected)
    partition = hashlib.sha256()
    for token in selected_tokens:
        partition.update(token.encode())
        partition.update(b"\n")
    if canonical_group == 0:
        labels = np.asarray([0 if index in selected else 1 for index in range(len(tokens))])
    else:
        labels = np.asarray([1 if index in selected else 0 for index in range(len(tokens))])
    return labels, partition.hexdigest()


def numerical_reference(
    raw: np.ndarray, actual_groups: list[str], requested_groups: tuple[str, str], samples: int
) -> dict[str, Any]:
    ordered, observed_labels, tokens = canonical_cases(raw, actual_groups, requested_groups)
    group_sizes = (
        int(np.sum(observed_labels == 0)),
        int(np.sum(observed_labels == 1)),
    )
    pooled = historical.independent.estimate_pls(ordered)
    original_a = historical.independent.align_to_pooled(
        ordered, pooled, historical.independent.estimate_pls(ordered[observed_labels == 0])
    )
    original_b = historical.independent.align_to_pooled(
        ordered, pooled, historical.independent.estimate_pls(ordered[observed_labels == 1])
    )
    paths = {key: original_a.paths[key] - original_b.paths[key] for key in original_a.paths}
    values_a = historical.independent.measurement_values(original_a)
    values_b = historical.independent.measurement_values(original_b)
    measurements = {key: values_a[key] - values_b[key] for key in values_a}
    observed_micom = historical.micom_statistics(
        ordered, observed_labels, original_a, original_b, pooled
    )
    path_distribution = {key: [] for key in paths}
    measurement_distribution = {key: [] for key in measurements}
    correlation_distribution = {key: [] for key in observed_micom}
    mean_distribution = {key: [] for key in observed_micom}
    variance_distribution = {key: [] for key in observed_micom}
    ledger: list[dict[str, Any]] = []
    plan = hashlib.sha256()
    for replicate in range(samples):
        labels, partition_sha256 = partition_labels(
            tokens, requested_groups, group_sizes, SEED, replicate
        )
        plan.update(struct.pack("<Q", replicate))
        plan.update(partition_sha256.encode())
        entry = {
            "replicate": replicate,
            "partition_sha256": partition_sha256,
            "group_a_rows": int(np.sum(labels == 0)),
            "group_b_rows": int(np.sum(labels == 1)),
            "step2_status": "failed",
            "step2_failure_code": "micom.observed_model_fit_failed",
            "step3_status": "usable",
            "step3_failure_code": None,
        }
        pooled_scores = {
            construct: pooled.scores[index]
            for index, (construct, _) in enumerate(historical.independent.CONSTRUCTS)
        }
        try:
            for construct, scores in pooled_scores.items():
                left, right = scores[labels == 0], scores[labels == 1]
                mean_distribution[construct].append(float(np.mean(left) - np.mean(right)))
                variance_distribution[construct].append(
                    math.log(historical.independent.sample_variance(left))
                    - math.log(historical.independent.sample_variance(right))
                )
        except (ValueError, ZeroDivisionError):
            entry["step3_status"] = "failed"
            entry["step3_failure_code"] = "micom.degenerate_composite_score"
        try:
            fit_a = historical.independent.align_to_pooled(
                ordered, pooled, historical.independent.estimate_pls(ordered[labels == 0])
            )
            fit_b = historical.independent.align_to_pooled(
                ordered, pooled, historical.independent.estimate_pls(ordered[labels == 1])
            )
            for key in paths:
                path_distribution[key].append(fit_a.paths[key] - fit_b.paths[key])
            perm_a = historical.independent.measurement_values(fit_a)
            perm_b = historical.independent.measurement_values(fit_b)
            for key in measurements:
                measurement_distribution[key].append(perm_a[key] - perm_b[key])
            for construct, values in historical.micom_statistics(
                ordered, labels, fit_a, fit_b, pooled
            ).items():
                correlation_distribution[construct].append(values["compositional_correlation"])
            entry["step2_status"] = "usable"
            entry["step2_failure_code"] = None
        except (RuntimeError, np.linalg.LinAlgError, ValueError, ZeroDivisionError):
            pass
        ledger.append(entry)
    step2_usable = sum(row["step2_status"] == "usable" for row in ledger)
    step3_usable = sum(row["step3_status"] == "usable" for row in ledger)
    if min(step2_usable, step3_usable) < 19:
        raise RuntimeError("independent v4 oracle produced fewer than 19 usable partitions")

    def permutation_rows(observed: dict[Any, float], distributions: dict[Any, list[float]]):
        return {
            key: {
                "original_difference": value,
                "empirical_p_value_two_sided":
                    (sum(abs(item) >= abs(value) for item in distributions[key]) + 1.0)
                    / (len(distributions[key]) + 1.0),
                "percentile_rank":
                    sum(item <= value for item in distributions[key]) / len(distributions[key]),
            }
            for key, value in observed.items()
        }

    alpha = 1.0 - CONFIDENCE_LEVEL
    micom: dict[str, dict[str, Any]] = {}
    for construct, observed in observed_micom.items():
        correlations = correlation_distribution[construct]
        means = mean_distribution[construct]
        variances = variance_distribution[construct]
        correlation_lower = historical.independent.type7_quantile(correlations, alpha)
        mean_lower = historical.independent.type7_quantile(means, alpha / 2.0)
        mean_upper = historical.independent.type7_quantile(means, 1.0 - alpha / 2.0)
        variance_lower = historical.independent.type7_quantile(variances, alpha / 2.0)
        variance_upper = historical.independent.type7_quantile(variances, 1.0 - alpha / 2.0)
        partial = observed["compositional_correlation"] + 1e-12 >= correlation_lower
        equal_means = mean_lower <= observed["mean_difference"] <= mean_upper
        equal_variances = variance_lower <= observed["variance_difference"] <= variance_upper
        micom[construct] = {
            **observed,
            "compositional_p_value": historical.independent.lower_tail_p(
                correlations, observed["compositional_correlation"]
            ),
            "mean_p_value": historical.independent.two_tail_p(means, observed["mean_difference"]),
            "variance_p_value": historical.independent.two_tail_p(
                variances, observed["variance_difference"]
            ),
            "partial_invariance": partial,
            "equal_means": equal_means,
            "equal_variances": equal_variances,
            "full_invariance": partial and equal_means and equal_variances,
        }
    effective_usable = min(step2_usable, step3_usable)
    return {
        "paths": permutation_rows(paths, path_distribution),
        "measurements": permutation_rows(measurements, measurement_distribution),
        "micom": micom,
        "usable_permutations": step2_usable,
        "attempted_permutations": samples,
        "failed_permutations": samples - step2_usable,
        "micom_usable_permutations": effective_usable,
        "micom_failed_permutations": samples - effective_usable,
        "step2_usable_permutations": step2_usable,
        "step3_usable_permutations": step3_usable,
        "permutation_plan_sha256": f"sha256:{plan.hexdigest()}",
        "ledger": ledger,
    }


def product_projection(payload: dict[str, Any]) -> dict[str, Any]:
    projected = historical.product_projection(payload)
    estimation = payload["payload"]["estimation"]
    permutation = estimation["mga_permutation"]
    micom = estimation["micom"]
    projected.update({
        "retry_policy": permutation.get("retry_policy"),
        "permutation_plan_sha256": permutation.get("permutation_plan_sha256"),
        "ledger": permutation.get("permutation_ledger", []),
        "micom_usable_permutations": micom.get("usable_permutations"),
        "micom_failed_permutations": micom.get("failed_permutations"),
        "step2_usable_permutations": micom.get("step2_usable_permutations"),
        "step3_usable_permutations": micom.get("step3_usable_permutations"),
        "micom_plan_sha256": micom.get("permutation_plan_sha256"),
        "micom_ledger": micom.get("permutation_ledger", []),
    })
    return projected


def execute_product(
    samples: int, group_a: str, group_b: str, recipe_path: Path, output: Path
) -> dict[str, Any]:
    scratch = RESULTS / "micom_mga_v4_reference.fingerprint.qpls"
    scratch.unlink(missing_ok=True)
    try:
        subprocess.run(
            [str(historical.cli_path()), "import", str(historical.DATA.relative_to(ROOT)),
             str(scratch.relative_to(ROOT)), "--name", "micom_mga_v4_reference"],
            cwd=ROOT, check=True, capture_output=True,
        )
        inspected = json.loads(subprocess.run(
            [str(historical.cli_path()), "inspect", str(scratch.relative_to(ROOT)), "--json"],
            cwd=ROOT, check=True, capture_output=True, text=True,
        ).stdout)
    finally:
        scratch.unlink(missing_ok=True)
    recipe = json.loads(historical.SOURCE_RECIPE.read_text(encoding="utf-8"))
    recipe["schema_version"] = 3
    recipe["dataset_fingerprint"] = inspected["datasets"][0]["fingerprint"]
    recipe["model"].setdefault("controls", [])
    recipe["model"].setdefault("higher_order_constructs", [])
    recipe["model"].setdefault("interactions", [])
    recipe["settings"].update({
        "studentized_inner_samples": 0, "permutation_samples": 0,
        "workers": 1, "case_weight_column": None,
    })
    recipe["method_config"] = {
        "kind": "mga", "group_column": "group", "group_a": group_a,
        "group_b": group_b, "methods": ["micom", "mga_permutation"],
        "permutation_samples": samples, "configural_invariance_confirmed": True,
    }
    recipe["metadata"] = {
        "status": "validated_micom_v4_and_permutation_mga_v4_fixed_plan_scope"
    }
    historical.write_json(recipe_path, recipe)
    output.unlink(missing_ok=True)
    probe = ROOT / "target" / "release" / "examples" / "validation_probe_mga_v4.exe"
    if not probe.is_file():
        raise RuntimeError(f"MGA v4 validation probe not found at {probe}")
    subprocess.run(
        [str(probe), str(recipe_path), str(historical.DATA), str(output)],
        cwd=ROOT, check=True,
    )
    return json.loads(output.read_text(encoding="utf-8"))


def product_comparison(reference: dict[str, Any], product: dict[str, Any]) -> dict[str, Any]:
    projected = product_projection(product)
    deltas: list[float] = []
    shapes = reference["paths"].keys() == projected["paths"].keys()
    shapes &= reference["measurements"].keys() == projected["measurements"].keys()
    for collection in ("paths", "measurements"):
        for key, expected in reference[collection].items():
            actual = projected[collection].get(key, {})
            for field, expected_value in expected.items():
                if field not in actual:
                    shapes = False
                else:
                    deltas.append(abs(float(expected_value) - float(actual[field])))
    for construct, expected in reference["micom"].items():
        actual = projected["micom"].get(construct, {})
        for field in (
            "compositional_correlation", "compositional_p_value", "mean_difference",
            "mean_p_value", "variance_difference", "variance_p_value",
        ):
            if field not in actual:
                shapes = False
            else:
                deltas.append(abs(float(expected[field]) - float(actual[field])))
    accounting = all(reference[key] == projected[key] for key in (
        "usable_permutations", "attempted_permutations", "failed_permutations",
        "micom_usable_permutations", "micom_failed_permutations",
        "step2_usable_permutations", "step3_usable_permutations",
    ))
    def canonical_ledger(rows: list[dict[str, Any]]) -> list[tuple[Any, ...]]:
        return [(
            row.get("replicate"), row.get("partition_sha256"), row.get("group_a_rows"),
            row.get("group_b_rows"), row.get("step2_status"), row.get("step2_failure_code"),
            row.get("step3_status"), row.get("step3_failure_code"),
        ) for row in rows]
    ledger = canonical_ledger(reference["ledger"]) == canonical_ledger(projected["ledger"]) \
        == canonical_ledger(projected["micom_ledger"])
    plan = reference["permutation_plan_sha256"] == projected["permutation_plan_sha256"] \
        == projected["micom_plan_sha256"]
    versions = projected["method_version"] == "pls_mga_two_group_v4" \
        and projected["permutation_method_version"] == "pls_mga_permutation_v4" \
        and projected["micom_method_version"] == "micom_v4"
    maximum_delta = max(deltas, default=math.inf)
    return {
        "passed": shapes and accounting and ledger and plan and versions
        and projected["retry_policy"] == "none" and maximum_delta <= TOLERANCE,
        "shapes_match": shapes,
        "accounting_matches": accounting,
        "ledger_matches": ledger,
        "plan_matches": plan,
        "versions_match": versions,
        "maximum_numerical_delta": maximum_delta,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--permutations", type=int, default=39)
    parser.add_argument("--run-quickpls", action="store_true")
    args = parser.parse_args()
    if args.permutations < 19:
        raise SystemExit("--permutations must be at least 19")
    if args.run_quickpls and args.permutations < PROMOTION_PERMUTATIONS:
        raise SystemExit("QuickPLS comparison requires at least 5000 permutations")
    raw, labels = historical.load_fixture()
    actual_groups = ["A" if label == 0 else "B" for label in labels]
    forward = numerical_reference(raw, actual_groups, ("A", "B"), args.permutations)
    reverse = numerical_reference(raw, actual_groups, ("B", "A"), args.permutations)
    swap = historical.swap_checks(forward, reverse)
    comparisons = None
    if args.run_quickpls:
        forward_product = execute_product(
            args.permutations, "A", "B", BASE_RECIPE, BASE_QUICKPLS
        )
        reverse_product = execute_product(
            args.permutations, "B", "A", SWAP_RECIPE, SWAP_QUICKPLS
        )
        comparisons = {
            "forward": product_comparison(forward, forward_product),
            "reverse": product_comparison(reverse, reverse_product),
            "product_swap": historical.swap_checks(
                product_projection(forward_product), product_projection(reverse_product)
            ),
        }
    comparison_passed = comparisons is None or all(row["passed"] for row in comparisons.values())
    ledger_swap = all(
        left["partition_sha256"] == right["partition_sha256"]
        and left["step2_status"] == right["step2_status"]
        and left["step3_status"] == right["step3_status"]
        for left, right in zip(forward["ledger"], reverse["ledger"])
    )
    report = {
        "schema_version": 1,
        "target": "micom_v4_and_pls_mga_permutation_v4_fixed_plan_independent_reference",
        "method_versions": {
            "mga": "pls_mga_two_group_v4",
            "permutation_mga": "pls_mga_permutation_v4",
            "micom": "micom_v4",
        },
        "permutation_samples": args.permutations,
        "seed": SEED,
        "retry_policy": "none",
        "fixed_plan": True,
        "ledger_swap_coupled": ledger_swap,
        "numerical_swap_reference": swap,
        "quickpls_comparison": comparisons,
        "promotion_sample_size": args.permutations >= PROMOTION_PERMUTATIONS,
        "passed": swap["passed"] and ledger_swap and comparison_passed,
        "independence": "NumPy PLS estimator and independently specified SHA-256 case-token and partition plan; no QuickPLS product module import",
    }
    historical.write_json(OUTPUT, report)
    print(json.dumps(report, indent=2, sort_keys=True, allow_nan=False))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
