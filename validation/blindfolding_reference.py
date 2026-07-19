import csv
import json
import math
import subprocess
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "validation" / "fixtures" / "corporate_reputation.csv"
RECIPE = ROOT / "validation" / "fixtures" / "corporate_reputation.recipe.json"
QUICKPLS = ROOT / "validation" / "results" / "blindfolding_quickpls_reference.json"
OUTPUT = ROOT / "validation" / "results" / "blindfolding_python_reference.json"
COMPARISON = ROOT / "validation" / "results" / "blindfolding_python_comparison.json"
TOLERANCE = 1e-6


def sample_sd(values):
    return float(np.std(values, ddof=1))


def standardize(values):
    values = np.asarray(values, dtype=float)
    centered = values - float(np.mean(values))
    deviation = sample_sd(centered)
    if deviation <= np.finfo(float).eps or not math.isfinite(deviation):
        raise ValueError("zero-variance vector")
    return centered / deviation


def covariance(left, right):
    return float(np.cov(left, right, ddof=1)[0, 1])


def correlation(left, right):
    return covariance(left, right) / (sample_sd(left) * sample_sd(right))


def orient_block_weights(columns, block_indices, weights):
    score = sum(columns[index] * weight for index, weight in zip(block_indices, weights))
    reference = sum(columns[index] for index in block_indices)
    association = covariance(score, reference)
    if association < -1e-15 or (abs(association) <= 1e-15 and float(np.sum(weights)) < 0):
        return -weights
    return weights


def normalize_block_weights(columns, block_indices, weights):
    weights = orient_block_weights(columns, block_indices, np.asarray(weights, dtype=float))
    score = sum(columns[index] * weight for index, weight in zip(block_indices, weights))
    deviation = sample_sd(score)
    if deviation <= np.finfo(float).eps or not math.isfinite(deviation):
        raise ValueError("outer weights produce a zero-variance score")
    return orient_block_weights(columns, block_indices, weights / deviation)


def ols(predictors, outcome):
    if not predictors:
        return np.asarray([], dtype=float)
    x = np.column_stack([predictor - float(np.mean(predictor)) for predictor in predictors])
    rank = np.linalg.matrix_rank(x)
    if rank < x.shape[1]:
        raise ValueError("rank-deficient regression")
    beta, *_ = np.linalg.lstsq(x, outcome, rcond=None)
    return beta


def block_scores(columns, blocks, weights):
    scores = []
    for block, block_weights in zip(blocks, weights):
        score = sum(columns[index] * weight for index, weight in zip(block, block_weights))
        scores.append(standardize(score))
    return scores


def inner_proxies(scores, recipe):
    construct_ids = [construct["id"] for construct in recipe["model"]["constructs"]]
    index = {construct: position for position, construct in enumerate(construct_ids)}
    incoming = [[] for _ in construct_ids]
    outgoing = [[] for _ in construct_ids]
    for path in recipe["model"]["paths"]:
        source = index[path["source"]]
        target = index[path["target"]]
        incoming[target].append(source)
        outgoing[source].append(target)
    proxies = []
    for construct_index, construct_id in enumerate(construct_ids):
        proxy = np.zeros_like(scores[construct_index])
        if recipe["settings"].get("weighting_scheme", "path") == "path" and incoming[construct_index]:
            beta = ols([scores[source] for source in incoming[construct_index]], scores[construct_index])
            for source, coefficient in zip(incoming[construct_index], beta):
                proxy += scores[source] * coefficient
        else:
            for source in incoming[construct_index]:
                proxy += scores[source] * correlation(scores[construct_index], scores[source])
        for target in outgoing[construct_index]:
            proxy += scores[target] * correlation(scores[construct_index], scores[target])
        proxies.append(standardize(proxy))
    return proxies


def estimate_pls(columns_by_indicator, recipe):
    indicators = [
        indicator
        for construct in recipe["model"]["constructs"]
        for indicator in construct["indicators"]
    ]
    raw_columns = [np.asarray(columns_by_indicator[indicator], dtype=float) for indicator in indicators]
    prepared = [standardize(column) for column in raw_columns]
    indicator_index = {indicator: position for position, indicator in enumerate(indicators)}
    blocks = [
        [indicator_index[indicator] for indicator in construct["indicators"]]
        for construct in recipe["model"]["constructs"]
    ]
    weights = [
        normalize_block_weights(prepared, block, np.ones(len(block)))
        for block in blocks
    ]
    tolerance = float(recipe["settings"].get("tolerance", 1e-7))
    max_iterations = int(recipe["settings"].get("max_iterations", 3000))
    for _ in range(max_iterations):
        scores = block_scores(prepared, blocks, weights)
        proxies = inner_proxies(scores, recipe)
        updated = []
        for construct_index, (construct, block) in enumerate(zip(recipe["model"]["constructs"], blocks)):
            if construct.get("mode") == "formative":
                candidate = ols([prepared[column] for column in block], proxies[construct_index])
            else:
                candidate = np.asarray(
                    [covariance(prepared[column], proxies[construct_index]) for column in block],
                    dtype=float,
                )
            updated.append(normalize_block_weights(prepared, block, candidate))
        change = max(
            abs(float(old) - float(new))
            for old_block, new_block in zip(weights, updated)
            for old, new in zip(old_block, new_block)
        )
        weights = updated
        if change <= tolerance:
            scores = block_scores(prepared, blocks, weights)
            break
    else:
        raise ValueError("PLS did not converge")
    construct_ids = [construct["id"] for construct in recipe["model"]["constructs"]]
    score_map = {construct_id: scores[index] for index, construct_id in enumerate(construct_ids)}
    path_coefficients = {}
    for construct_id in construct_ids:
        predecessors = [
            path["source"] for path in recipe["model"]["paths"] if path["target"] == construct_id
        ]
        if not predecessors:
            continue
        beta = ols([score_map[source] for source in predecessors], score_map[construct_id])
        for source, coefficient in zip(predecessors, beta):
            path_coefficients[(source, construct_id)] = float(coefficient)
    loading_map = {}
    for construct_index, construct in enumerate(recipe["model"]["constructs"]):
        for indicator in construct["indicators"]:
            loading_map[indicator] = correlation(prepared[indicator_index[indicator]], scores[construct_index])
    return {
        "scores": score_map,
        "paths": path_coefficients,
        "loadings": loading_map,
    }


def load_data():
    with DATA.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    columns = {name: [] for name in rows[0]}
    for row in rows:
        for name, value in row.items():
            columns[name].append(float(value))
    return columns


def load_recipe():
    return json.loads(RECIPE.read_text(encoding="utf-8"))


def run_quickpls():
    subprocess.run(
        [
            "cargo",
            "run",
            "-p",
            "qpls-cli",
            "--",
            "run",
            str(RECIPE.relative_to(ROOT)),
            "--data",
            str(DATA.relative_to(ROOT)),
            "--output",
            str(QUICKPLS.relative_to(ROOT)),
        ],
        cwd=ROOT,
        check=True,
    )
    envelope = json.loads(QUICKPLS.read_text(encoding="utf-8"))
    return envelope["payload"]["assessment"]["blindfolding"]


def blindfolding_reference(columns, recipe):
    observation_count = len(next(iter(columns.values())))
    distance = next(
        d for d in [7, 5, 6, 8, 9, 10, 11, 12] if d < observation_count and observation_count % d != 0
    )
    construct_rows = []
    for construct in recipe["model"]["constructs"]:
        if not any(path["target"] == construct["id"] for path in recipe["model"]["paths"]):
            continue
        if construct.get("mode") == "formative":
            construct_rows.append(
                {
                    "construct": construct["id"],
                    "q_squared": None,
                    "prediction_error_sum_squares": None,
                    "observation_sum_squares": None,
                }
            )
            continue
        press = 0.0
        sso = 0.0
        for round_index in range(distance):
            blinded = {name: list(values) for name, values in columns.items()}
            omitted = []
            for indicator_offset, indicator in enumerate(construct["indicators"]):
                original = columns[indicator]
                omitted_rows = [
                    row
                    for row in range(observation_count)
                    if (indicator_offset * observation_count + row) % distance == round_index
                ]
                retained = [
                    value for row, value in enumerate(original) if row not in set(omitted_rows)
                ]
                retained_mean = sum(retained) / len(retained)
                for row in omitted_rows:
                    omitted.append((indicator, row, original[row]))
                    blinded[indicator][row] = retained_mean
            round_estimate = estimate_pls(blinded, recipe)
            predecessors = [
                path for path in recipe["model"]["paths"] if path["target"] == construct["id"]
            ]
            predicted_scores = []
            for row in range(observation_count):
                predicted_scores.append(
                    sum(
                        round_estimate["paths"].get((path["source"], path["target"]), 0.0)
                        * round_estimate["scores"][path["source"]][row]
                        for path in predecessors
                    )
                )
            for indicator, row, actual in omitted:
                training = np.asarray(blinded[indicator], dtype=float)
                mean = float(np.mean(training))
                sd = sample_sd(training)
                actual_standardized = (actual - mean) / sd
                predicted_standardized = (
                    round_estimate["loadings"][indicator] * predicted_scores[row]
                )
                press += (actual_standardized - predicted_standardized) ** 2
                sso += actual_standardized**2
        construct_rows.append(
            {
                "construct": construct["id"],
                "q_squared": 1.0 - press / sso,
                "prediction_error_sum_squares": press,
                "observation_sum_squares": sso,
            }
        )
    return {
        "settings": {
            "omission_distance": distance,
            "selection": "preferred_7_then_smallest_valid_5_to_12",
            "missing_value_treatment": "indicator_mean_replacement",
        },
        "constructs": construct_rows,
    }


def compare(quickpls, reference):
    comparisons = []
    reference_by_construct = {row["construct"]: row for row in reference["constructs"]}
    for row in quickpls["constructs"]:
        ref = reference_by_construct[row["construct"]]
        for field in ("q_squared", "prediction_error_sum_squares", "observation_sum_squares"):
            actual = row[field]
            expected = ref[field]
            difference = None if actual is None or expected is None else actual - expected
            abs_diff = None if difference is None else abs(difference)
            comparisons.append(
                {
                    "construct": row["construct"],
                    "field": field,
                    "quickpls": actual,
                    "python_reference": expected,
                    "difference": difference,
                    "abs_diff": abs_diff,
                    "passed": bool(abs_diff is not None and abs_diff <= TOLERANCE),
                }
            )
    return comparisons


def main():
    recipe = load_recipe()
    columns = load_data()
    quickpls = run_quickpls()
    reference = blindfolding_reference(columns, recipe)
    OUTPUT.write_text(json.dumps(reference, indent=2) + "\n", encoding="utf-8")
    comparisons = compare(quickpls, reference)
    report = {
        "status": "passed" if all(row["passed"] for row in comparisons) else "failed",
        "tolerance": TOLERANCE,
        "reference": "independent NumPy implementation of the frozen QuickPLS blindfolding v4 contract",
        "artifacts": {
            "data": str(DATA.relative_to(ROOT)),
            "recipe": str(RECIPE.relative_to(ROOT)),
            "quickpls": str(QUICKPLS.relative_to(ROOT)),
            "reference": str(OUTPUT.relative_to(ROOT)),
        },
        "max_abs_diff": max(row["abs_diff"] for row in comparisons if row["abs_diff"] is not None),
        "comparisons": comparisons,
    }
    COMPARISON.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        raise SystemExit(f"blindfolding comparison failed; see {COMPARISON}")


if __name__ == "__main__":
    main()
