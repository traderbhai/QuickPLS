"""Independent weighted PLS reference for the experimental WPLS slice."""

import csv
import json
import math
import random
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
DATA = RESULTS / "wpls_reference.csv"
BAD_DATA = RESULTS / "wpls_bad_weights.csv"
RECIPE = RESULTS / "wpls_reference.recipe.json"
QUICKPLS = RESULTS / "wpls_reference_quickpls.json"
OUTPUT = RESULTS / "wpls_reference_report.json"
MISSING_WEIGHT_RECIPE = RESULTS / "wpls_missing_weight.recipe.json"
BAD_WEIGHT_RECIPE = RESULTS / "wpls_bad_weight.recipe.json"
BAD_WEIGHT_RESULT = RESULTS / "wpls_bad_weight_quickpls.json"
CLI_EXE = ROOT / "target" / "debug" / "qpls.exe"
TOLERANCE = 1e-10
CLI_READY = False


def ensure_cli():
    global CLI_READY
    if CLI_READY:
        return CLI_EXE
    subprocess.run(
        ["cargo", "build", "-p", "qpls-cli"],
        cwd=ROOT,
        check=True,
        stdout=subprocess.DEVNULL,
    )
    CLI_READY = True
    return CLI_EXE


def qpls(args, **kwargs):
    return subprocess.run([str(ensure_cli()), *args], cwd=ROOT, **kwargs)


def generated_rows(seed=20260719, n=128):
    rng = random.Random(seed)
    rows = []
    for index in range(n):
        segment_shift = 0.8 if index % 5 == 0 else -0.1
        x = rng.gauss(segment_shift, 1.0)
        y = 0.62 * x + rng.gauss(0.0, 0.55)
        weight = 0.65 + (2.25 if index % 7 == 0 else 0.35 if index % 3 == 0 else 1.0)
        rows.append(
            {
                "x1": 0.94 * x + rng.gauss(0.0, 0.14),
                "x2": 0.83 * x + rng.gauss(0.0, 0.18),
                "y1": 0.91 * y + rng.gauss(0.0, 0.16),
                "y2": 0.78 * y + rng.gauss(0.0, 0.20),
                "case_wt": weight,
            }
        )
    return rows


def write_dataset(path, rows, bad_weight=False):
    RESULTS.mkdir(parents=True, exist_ok=True)
    fields = ["x1", "x2", "y1", "y2", "case_wt"]
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        for index, row in enumerate(rows):
            output = dict(row)
            if bad_weight and index == 4:
                output["case_wt"] = -1.0
            writer.writerow({field: f"{output[field]:.12f}" for field in fields})


def dataset_fingerprint(path, name):
    project_path = RESULTS / f"{name}.fingerprint.qpls"
    qpls(
        ["import", str(path.relative_to(ROOT)), str(project_path.relative_to(ROOT)), "--name", name],
        check=True,
        stdout=subprocess.DEVNULL,
    )
    payload = json.loads(
        qpls(
            ["inspect", str(project_path.relative_to(ROOT)), "--json"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout
    )
    return payload["datasets"][0]["fingerprint"]


def recipe_payload(fingerprint):
    return {
        "schema_version": 2,
        "id": "00000000-0000-0000-0000-000000000015",
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": "00000000-0000-0000-0000-000000000016",
            "name": "WPLS reference",
            "constructs": [
                {"id": "x", "name": "X", "short_name": "X", "mode": "reflective", "indicators": ["x1", "x2"]},
                {"id": "y", "name": "Y", "short_name": "Y", "mode": "reflective", "indicators": ["y1", "y2"]},
            ],
            "paths": [{"source": "x", "target": "y"}],
        },
        "settings": {
            "method": "wpls",
            "weighting_scheme": "path",
            "tolerance": 1e-7,
            "max_iterations": 3000,
            "bootstrap_samples": 0,
            "seed": 20260719,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
            "case_weight_column": "case_wt",
        },
        "metadata": {"fixture": "independent_wpls_reference"},
    }


def weighted_mean(values, weights):
    return sum(value * weight for value, weight in zip(values, weights)) / sum(weights)


def weighted_dof(weights):
    total = sum(weights)
    return total - sum(weight * weight for weight in weights) / total


def weighted_cov(left, right, weights):
    left_mean = weighted_mean(left, weights)
    right_mean = weighted_mean(right, weights)
    return sum(weight * (a - left_mean) * (b - right_mean) for a, b, weight in zip(left, right, weights)) / weighted_dof(weights)


def weighted_sd(values, weights):
    return math.sqrt(weighted_cov(values, values, weights))


def weighted_standardize(values, weights):
    mean = weighted_mean(values, weights)
    sd = weighted_sd(values, weights)
    return [(value - mean) / sd for value in values]


def add_scaled(target, source, coefficient):
    for index, value in enumerate(source):
        target[index] += value * coefficient


def orient_block(columns, block, weights, case_weights):
    score = [0.0] * len(case_weights)
    reference = [0.0] * len(case_weights)
    for column, coefficient in zip(block, weights):
        add_scaled(score, columns[column], coefficient)
        add_scaled(reference, columns[column], 1.0)
    association = weighted_cov(score, reference, case_weights)
    if association < -1e-15 or (abs(association) <= 1e-15 and sum(weights) < 0.0):
        return [-value for value in weights]
    return weights


def normalize_weights(columns, block, weights, case_weights):
    weights = orient_block(columns, block, list(weights), case_weights)
    score = [0.0] * len(case_weights)
    for column, coefficient in zip(block, weights):
        add_scaled(score, columns[column], coefficient)
    sd = weighted_sd(score, case_weights)
    normalized = [weight / sd for weight in weights]
    return orient_block(columns, block, normalized, case_weights)


def block_scores(columns, blocks, weights, case_weights):
    scores = []
    for block, block_weight in zip(blocks, weights):
        score = [0.0] * len(case_weights)
        for column, coefficient in zip(block, block_weight):
            add_scaled(score, columns[column], coefficient)
        scores.append(weighted_standardize(score, case_weights))
    return scores


def inner_proxies(scores, case_weights):
    association = weighted_cov(scores[0], scores[1], case_weights)
    sign = 1.0 if association >= 0.0 else -1.0
    return [[sign * value for value in scores[1]], [sign * value for value in scores[0]]]


def ols_weighted(predictors, outcome, case_weights):
    # Single-predictor fixture keeps the independent reference transparent.
    predictor = predictors[0]
    return [weighted_cov(predictor, outcome, case_weights) / weighted_cov(predictor, predictor, case_weights)]


def estimate_reference(rows):
    names = ["x1", "x2", "y1", "y2"]
    case_weights = [row["case_wt"] for row in rows]
    columns = [weighted_standardize([row[name] for row in rows], case_weights) for name in names]
    blocks = [[0, 1], [2, 3]]
    weights = [normalize_weights(columns, block, [1.0] * len(block), case_weights) for block in blocks]
    iterations = 0
    for iteration in range(1, 3001):
        scores = block_scores(columns, blocks, weights, case_weights)
        inner = inner_proxies(scores, case_weights)
        updated = []
        for construct_index, block in enumerate(blocks):
            candidate = [weighted_cov(columns[column], inner[construct_index], case_weights) for column in block]
            updated.append(normalize_weights(columns, block, candidate, case_weights))
        change = max(abs(old - new) for left, right in zip(weights, updated) for old, new in zip(left, right))
        weights = updated
        iterations = iteration
        if change <= 1e-7:
            break
    scores = block_scores(columns, blocks, weights, case_weights)
    path = ols_weighted([scores[0]], scores[1], case_weights)[0]
    fitted = [path * value for value in scores[0]]
    y_mean = weighted_mean(scores[1], case_weights)
    residual = sum(weight * (actual - fit) ** 2 for actual, fit, weight in zip(scores[1], fitted, case_weights))
    total = sum(weight * (actual - y_mean) ** 2 for actual, weight in zip(scores[1], case_weights))
    loadings = {
        ("x", "x1"): weighted_cov(columns[0], scores[0], case_weights) / (weighted_sd(columns[0], case_weights) * weighted_sd(scores[0], case_weights)),
        ("x", "x2"): weighted_cov(columns[1], scores[0], case_weights) / (weighted_sd(columns[1], case_weights) * weighted_sd(scores[0], case_weights)),
        ("y", "y1"): weighted_cov(columns[2], scores[1], case_weights) / (weighted_sd(columns[2], case_weights) * weighted_sd(scores[1], case_weights)),
        ("y", "y2"): weighted_cov(columns[3], scores[1], case_weights) / (weighted_sd(columns[3], case_weights) * weighted_sd(scores[1], case_weights)),
    }
    weight_rows = {
        ("x", "x1"): weights[0][0],
        ("x", "x2"): weights[0][1],
        ("y", "y1"): weights[1][0],
        ("y", "y2"): weights[1][1],
    }
    weight_sum = sum(case_weights)
    return {
        "iterations": iterations,
        "paths": {("x", "y"): path},
        "r_squared": {"y": 1.0 - residual / total},
        "loadings": loadings,
        "weights": weight_rows,
        "weight_sum": weight_sum,
        "effective_sample_size": weight_sum * weight_sum / sum(weight * weight for weight in case_weights),
    }


def run_quickpls():
    QUICKPLS.unlink(missing_ok=True)
    qpls(
        [
            "run",
            str(RECIPE.relative_to(ROOT)),
            "--data",
            str(DATA.relative_to(ROOT)),
            "--output",
            str(QUICKPLS.relative_to(ROOT)),
            "--allow-experimental",
        ],
        check=True,
        stdout=subprocess.DEVNULL,
    )
    return json.loads(QUICKPLS.read_text(encoding="utf-8"))


def check_guards(fingerprint, rows):
    missing = recipe_payload(fingerprint)
    missing["settings"].pop("case_weight_column")
    MISSING_WEIGHT_RECIPE.write_text(json.dumps(missing, indent=2) + "\n", encoding="utf-8")
    validation = qpls(
        ["validate", str(MISSING_WEIGHT_RECIPE.relative_to(ROOT)), "--json"],
        capture_output=True,
        text=True,
    )
    validation_codes = [issue["code"] for issue in json.loads(validation.stdout)]

    write_dataset(BAD_DATA, rows, bad_weight=True)
    bad_fingerprint = dataset_fingerprint(BAD_DATA, "wpls_bad_weights")
    bad_recipe = recipe_payload(bad_fingerprint)
    BAD_WEIGHT_RECIPE.write_text(json.dumps(bad_recipe, indent=2) + "\n", encoding="utf-8")
    BAD_WEIGHT_RESULT.unlink(missing_ok=True)
    bad_run = qpls(
        [
            "run",
            str(BAD_WEIGHT_RECIPE.relative_to(ROOT)),
            "--data",
            str(BAD_DATA.relative_to(ROOT)),
            "--output",
            str(BAD_WEIGHT_RESULT.relative_to(ROOT)),
            "--allow-experimental",
        ],
        capture_output=True,
        text=True,
    )
    return {
        "passed": validation.returncode != 0
        and "wpls.case_weight_required" in validation_codes
        and bad_run.returncode != 0
        and "case weights must be positive and finite" in (bad_run.stderr + bad_run.stdout)
        and not BAD_WEIGHT_RESULT.exists(),
        "missing_weight_validation_codes": validation_codes,
        "bad_weight_stderr": bad_run.stderr,
    }


def main():
    rows = generated_rows()
    write_dataset(DATA, rows)
    fingerprint = dataset_fingerprint(DATA, "wpls_reference")
    RECIPE.write_text(json.dumps(recipe_payload(fingerprint), indent=2) + "\n", encoding="utf-8")
    quickpls = run_quickpls()
    estimation = quickpls["payload"]["estimation"]
    expected = estimate_reference(rows)
    observed_paths = {(row["source"], row["target"]): row["coefficient"] for row in estimation["paths"]}
    observed_loadings = {(row["construct"], row["indicator"]): row["loading"] for row in estimation["outer_estimates"]}
    observed_weights = {(row["construct"], row["indicator"]): row["weight"] for row in estimation["outer_estimates"]}
    deltas = {}
    for key, value in expected["paths"].items():
        deltas[f"path::{key[0]}::{key[1]}"] = abs(observed_paths[key] - value)
    for key, value in expected["r_squared"].items():
        deltas[f"r2::{key}"] = abs(estimation["r_squared"][key] - value)
    for key, value in expected["loadings"].items():
        deltas[f"loading::{key[0]}::{key[1]}"] = abs(observed_loadings[key] - value)
    for key, value in expected["weights"].items():
        deltas[f"weight::{key[0]}::{key[1]}"] = abs(observed_weights[key] - value)
    analysis = estimation["wpls"]
    deltas["weight_sum"] = abs(analysis["weight_sum"] - expected["weight_sum"])
    deltas["effective_sample_size"] = abs(analysis["effective_sample_size"] - expected["effective_sample_size"])
    max_delta = max(deltas.values())
    guards = check_guards(fingerprint, rows)
    checks = {
        "method_version": estimation["method_version"] == "wpls_case_weighted_v1",
        "payload_version": analysis["method_version"] == "wpls_case_weighted_v1",
        "case_weight_column": analysis["case_weight_column"] == "case_wt",
        "max_delta_within_tolerance": max_delta <= TOLERANCE,
        "guards": guards["passed"],
    }
    report = {
        "schema_version": 1,
        "kind": "wpls_reference_v1",
        "passed": all(checks.values()),
        "tolerance": TOLERANCE,
        "max_delta": max_delta,
        "checks": checks,
        "deltas": deltas,
        "guards": guards,
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {OUTPUT} | passed={report['passed']} | max_delta={max_delta:.3g}")
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
