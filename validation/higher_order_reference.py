"""Independent repeated-indicator higher-order construct reference.

The fixture defines two lower-order reflective constructs, an empty HOC
placeholder, and an outcome. QuickPLS expands the HOC into the ordered union of
component indicators. This script implements the same repeated-indicator
expansion and PLS path-weighting stages independently in Python, then compares
path coefficients and HOC loadings against the QuickPLS CLI output.
"""

import csv
import json
import math
import random
import subprocess
from pathlib import Path

import numpy as np


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
DATA = RESULTS / "higher_order_reference.csv"
RECIPE = RESULTS / "higher_order_reference.recipe.json"
QUICKPLS = RESULTS / "higher_order_reference_quickpls.json"
OUTPUT = RESULTS / "higher_order_reference_report.json"
TOLERANCE = 1e-6
CLI_EXE = ROOT / "target" / "debug" / "qpls.exe"


def ensure_cli():
    if not CLI_EXE.exists():
        subprocess.run(
            ["cargo", "build", "-p", "qpls-cli"],
            cwd=ROOT,
            check=True,
            stdout=subprocess.DEVNULL,
        )
    return CLI_EXE


def qpls_cli(args, **kwargs):
    return subprocess.run([str(ensure_cli()), *args], cwd=ROOT, check=True, **kwargs)


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
    if np.linalg.matrix_rank(x) < x.shape[1]:
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


def expand_repeated_indicator_hoc(recipe):
    expanded = json.loads(json.dumps(recipe))
    constructs = {construct["id"]: construct for construct in expanded["model"]["constructs"]}
    original = {
        construct["id"]: list(construct.get("indicators", []))
        for construct in recipe["model"]["constructs"]
    }
    for hoc in recipe["model"].get("higher_order_constructs", []):
        if hoc["method"] != "repeated_indicators":
            continue
        indicators = []
        seen = set()
        for component in hoc["components"]:
            for indicator in original[component]:
                if indicator not in seen:
                    seen.add(indicator)
                    indicators.append(indicator)
        constructs[hoc["id"]]["indicators"] = indicators
    return expanded


def collect_indicators(recipe):
    names = []
    seen = set()
    for construct in recipe["model"]["constructs"]:
        for indicator in construct["indicators"]:
            if indicator not in seen:
                seen.add(indicator)
                names.append(indicator)
    return names


def estimate_pls(columns_by_indicator, recipe):
    indicators = collect_indicators(recipe)
    prepared = [standardize(columns_by_indicator[indicator]) for indicator in indicators]
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
    for iteration in range(1, max_iterations + 1):
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
    paths = {}
    for construct_id in construct_ids:
        predecessors = [
            path["source"] for path in recipe["model"]["paths"] if path["target"] == construct_id
        ]
        if not predecessors:
            continue
        beta = ols([score_map[source] for source in predecessors], score_map[construct_id])
        for source, coefficient in zip(predecessors, beta):
            paths[(source, construct_id)] = float(coefficient)

    outer = {}
    for construct_index, construct in enumerate(recipe["model"]["constructs"]):
        for within, indicator in enumerate(construct["indicators"]):
            outer[(construct["id"], indicator)] = {
                "loading": correlation(prepared[indicator_index[indicator]], scores[construct_index]),
                "weight": float(weights[construct_index][within]),
            }
    return {
        "paths": paths,
        "outer": outer,
        "scores": score_map,
        "weights": {
            construct["id"]: {
                indicator: float(weights[construct_index][within])
                for within, indicator in enumerate(construct["indicators"])
            }
            for construct_index, construct in enumerate(recipe["model"]["constructs"])
        },
        "iterations": iteration,
    }


def generated_rows(seed=20260719, n=96):
    rng = random.Random(seed)
    rows = []
    for index in range(n):
        x_latent = rng.gauss(0.0, 1.0)
        z_latent = 0.25 * x_latent + rng.gauss(0.0, 1.0)
        y_latent = 0.52 * x_latent + 0.43 * z_latent + rng.gauss(0.0, 0.25)
        rows.append({
            "x1": x_latent + rng.gauss(0.0, 0.08),
            "x2": 0.92 * x_latent + rng.gauss(0.0, 0.09),
            "z1": z_latent + rng.gauss(0.0, 0.08),
            "z2": 0.88 * z_latent + rng.gauss(0.0, 0.10),
            "y1": y_latent + rng.gauss(0.0, 0.08),
            "y2": 0.95 * y_latent + rng.gauss(0.0, 0.09),
        })
    return rows


def write_dataset(rows):
    RESULTS.mkdir(parents=True, exist_ok=True)
    fields = ["x1", "x2", "z1", "z2", "y1", "y2"]
    with DATA.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        for row in rows:
            writer.writerow({field: f"{row[field]:.12f}" for field in fields})


def dataset_fingerprint():
    project_path = RESULTS / "higher_order_reference.fingerprint.qpls"
    qpls_cli(
        [
            "import",
            str(DATA.relative_to(ROOT)),
            str(project_path.relative_to(ROOT)),
            "--name",
            "higher_order_reference",
        ],
        stdout=subprocess.DEVNULL,
    )
    completed = qpls_cli(
        [
            "inspect",
            str(project_path.relative_to(ROOT)),
            "--json",
        ],
        capture_output=True,
        text=True,
    )
    project_path.unlink(missing_ok=True)
    return json.loads(completed.stdout)["datasets"][0]["fingerprint"]


def write_recipe(fingerprint):
    recipe = {
        "schema_version": 2,
        "id": "00000000-0000-0000-0000-000000000071",
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": "00000000-0000-0000-0000-000000000072",
            "name": "Repeated-indicator HOC independent reference",
            "constructs": [
                {"id": "x", "name": "X", "short_name": "X", "mode": "reflective", "indicators": ["x1", "x2"]},
                {"id": "z", "name": "Z", "short_name": "Z", "mode": "reflective", "indicators": ["z1", "z2"]},
                {"id": "hoc", "name": "HOC", "short_name": "HOC", "mode": "reflective", "indicators": []},
                {"id": "y", "name": "Y", "short_name": "Y", "mode": "reflective", "indicators": ["y1", "y2"]},
            ],
            "paths": [
                {"source": "x", "target": "hoc"},
                {"source": "z", "target": "hoc"},
                {"source": "hoc", "target": "y"},
            ],
            "higher_order_constructs": [
                {
                    "id": "hoc",
                    "components": ["x", "z"],
                    "method": "repeated_indicators",
                    "stage_one_recipe": None,
                }
            ],
        },
        "settings": {
            "method": "pls_pm",
            "weighting_scheme": "path",
            "tolerance": 1e-7,
            "max_iterations": 3000,
            "bootstrap_samples": 0,
            "studentized_inner_samples": 0,
            "permutation_samples": 0,
            "seed": 20260719,
            "workers": 1,
            "confidence_level": 0.95,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
        },
        "metadata": {"fixture": "independent repeated-indicator HOC reference"},
    }
    RECIPE.write_text(json.dumps(recipe, indent=2) + "\n", encoding="utf-8")
    return recipe


def run_quickpls():
    qpls_cli(
        [
            "run",
            str(RECIPE.relative_to(ROOT)),
            "--data",
            str(DATA.relative_to(ROOT)),
            "--output",
            str(QUICKPLS.relative_to(ROOT)),
            "--allow-experimental",
        ],
        stdout=subprocess.DEVNULL,
    )
    return json.loads(QUICKPLS.read_text(encoding="utf-8"))


def compare(reference, quickpls):
    estimation = quickpls["payload"]["estimation"]
    actual_paths = {
        (row["source"], row["target"]): row["coefficient"]
        for row in estimation["paths"]
    }
    actual_outer = {
        (row["construct"], row["indicator"]): {
            "loading": row["loading"],
            "weight": row["weight"],
        }
        for row in estimation["outer_estimates"]
    }
    differences = []
    for pair, expected in reference["paths"].items():
        actual = actual_paths[pair]
        differences.append({
            "kind": "path",
            "id": list(pair),
            "expected": expected,
            "actual": actual,
            "abs_difference": abs(expected - actual),
        })
    for key, expected in reference["outer"].items():
        if key[0] != "hoc":
            continue
        actual = actual_outer[key]
        for metric in ["loading", "weight"]:
            differences.append({
                "kind": f"hoc_{metric}",
                "id": list(key),
                "expected": expected[metric],
                "actual": actual[metric],
                "abs_difference": abs(expected[metric] - actual[metric]),
            })
    max_abs_difference = max(row["abs_difference"] for row in differences)
    warning_present = any(
        "Repeated-indicator higher-order constructs are experimental" in warning
        for warning in estimation["warnings"]
    )
    return {
        "method": "repeated_indicator_hoc_independent_reference_v1",
        "tolerance": TOLERANCE,
        "passed": max_abs_difference <= TOLERANCE and warning_present,
        "max_abs_difference": max_abs_difference,
        "warning_present": warning_present,
        "differences": differences,
    }


def main():
    rows = generated_rows()
    write_dataset(rows)
    recipe = write_recipe(dataset_fingerprint())
    quickpls = run_quickpls()
    columns = {name: [row[name] for row in rows] for name in rows[0]}
    reference = estimate_pls(columns, expand_repeated_indicator_hoc(recipe))
    report = compare(reference, quickpls)
    OUTPUT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if not report["passed"]:
        raise SystemExit(f"higher-order reference failed: {report}")
    print(
        f"higher-order repeated-indicator reference passed; max_abs_difference={report['max_abs_difference']:.3g}"
    )


if __name__ == "__main__":
    main()
