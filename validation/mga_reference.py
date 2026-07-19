"""Independent bounded two-group MGA reference for the experimental v0.6 slice."""

import csv
import json
import math
import random
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
DATA = RESULTS / "mga_reference.csv"
RECIPE = RESULTS / "mga_reference.recipe.json"
QUICKPLS = RESULTS / "mga_reference_quickpls.json"
OUTPUT = RESULTS / "mga_reference_report.json"
CLI_EXE = ROOT / "target" / "debug" / "qpls.exe"
TOLERANCE = 1e-6
CONVERGENCE_TOLERANCE = 1e-10
CLI_READY = False


def ensure_cli():
    global CLI_READY
    if CLI_READY:
        return CLI_EXE
    subprocess.run(["cargo", "build", "-p", "qpls-cli"], cwd=ROOT, check=True, stdout=subprocess.DEVNULL)
    CLI_READY = True
    return CLI_EXE


def qpls(args, **kwargs):
    return subprocess.run([str(ensure_cli()), *args], cwd=ROOT, **kwargs)


def write_dataset(seed=20260719, n_per_group=90):
    RESULTS.mkdir(parents=True, exist_ok=True)
    rng = random.Random(seed)
    with DATA.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=["group", "x1", "x2", "z1", "z2", "y1", "y2"])
        writer.writeheader()
        for group, bx, bz in [("A", 0.85, 0.25), ("B", -0.45, 0.75)]:
            for _ in range(n_per_group):
                x = rng.gauss(0.0, 1.0)
                z = rng.gauss(0.0, 1.0)
                y = bx * x + bz * z + rng.gauss(0.0, 0.24)
                writer.writerow({
                    "group": group,
                    "x1": f"{0.92 * x + rng.gauss(0.0, 0.08):.12f}",
                    "x2": f"{0.81 * x + rng.gauss(0.0, 0.10):.12f}",
                    "z1": f"{0.88 * z + rng.gauss(0.0, 0.08):.12f}",
                    "z2": f"{0.77 * z + rng.gauss(0.0, 0.10):.12f}",
                    "y1": f"{0.90 * y + rng.gauss(0.0, 0.08):.12f}",
                    "y2": f"{0.80 * y + rng.gauss(0.0, 0.10):.12f}",
                })


def dataset_fingerprint(path, name):
    project_path = RESULTS / f"{name}.fingerprint.qpls"
    qpls(["import", str(path.relative_to(ROOT)), str(project_path.relative_to(ROOT)), "--name", name], check=True, stdout=subprocess.DEVNULL)
    payload = json.loads(qpls(["inspect", str(project_path.relative_to(ROOT)), "--json"], check=True, capture_output=True, text=True).stdout)
    return payload["datasets"][0]["fingerprint"]


def write_recipe(fingerprint):
    payload = {
        "schema_version": 2,
        "id": "00000000-0000-0000-0000-000000000071",
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": "00000000-0000-0000-0000-000000000072",
            "name": "MGA two-group reference",
            "constructs": [
                {"id": "x", "name": "X", "short_name": "X", "mode": "reflective", "indicators": ["x1", "x2"]},
                {"id": "z", "name": "Z", "short_name": "Z", "mode": "reflective", "indicators": ["z1", "z2"]},
                {"id": "y", "name": "Y", "short_name": "Y", "mode": "reflective", "indicators": ["y1", "y2"]},
            ],
            "paths": [{"source": "x", "target": "y"}, {"source": "z", "target": "y"}],
        },
        "settings": {
            "method": "mga",
            "weighting_scheme": "path",
            "tolerance": CONVERGENCE_TOLERANCE,
            "max_iterations": 3000,
            "bootstrap_samples": 0,
            "seed": 20260719,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
        },
        "metadata": {"fixture": "independent_mga_reference", "mga_group_column": "group"},
    }
    RECIPE.write_text(json.dumps(payload, indent=2), encoding="utf-8")


def load_rows():
    with DATA.open(newline="", encoding="utf-8") as handle:
        return [{key: (value if key == "group" else float(value)) for key, value in row.items()} for row in csv.DictReader(handle)]


def mean(values):
    return sum(values) / len(values)


def sd(values):
    m = mean(values)
    return math.sqrt(sum((value - m) ** 2 for value in values) / (len(values) - 1))


def standardize(values):
    m = mean(values)
    s = sd(values)
    return [(value - m) / s for value in values]


def cov(left, right):
    lm = mean(left)
    rm = mean(right)
    return sum((a - lm) * (b - rm) for a, b in zip(left, right)) / (len(left) - 1)


def corr(left, right):
    return cov(left, right) / (sd(left) * sd(right))


def solve_linear(matrix, rhs):
    a = [row[:] + [value] for row, value in zip(matrix, rhs)]
    n = len(rhs)
    for col in range(n):
        pivot = max(range(col, n), key=lambda row: abs(a[row][col]))
        a[col], a[pivot] = a[pivot], a[col]
        scale = a[col][col]
        if abs(scale) < 1e-12:
            raise RuntimeError("singular independent reference system")
        for j in range(col, n + 1):
            a[col][j] /= scale
        for row in range(n):
            if row == col:
                continue
            factor = a[row][col]
            for j in range(col, n + 1):
                a[row][j] -= factor * a[col][j]
    return [a[row][n] for row in range(n)]


def ols(predictors, outcome):
    xtx = [[sum(a * b for a, b in zip(left, right)) for right in predictors] for left in predictors]
    xty = [sum(a * y for a, y in zip(predictor, outcome)) for predictor in predictors]
    return solve_linear(xtx, xty)


def orient(block, columns, weights):
    score = [sum(columns[column][i] * weight for column, weight in zip(block, weights)) for i in range(len(columns[block[0]]))]
    reference = [sum(columns[column][i] for column in block) for i in range(len(columns[block[0]]))]
    if cov(score, reference) < 0:
        return [-value for value in weights]
    return weights


def normalize(block, columns, weights):
    weights = orient(block, columns, list(weights))
    score = [sum(columns[column][i] * weight for column, weight in zip(block, weights)) for i in range(len(columns[block[0]]))]
    score_sd = sd(score)
    return orient(block, columns, [weight / score_sd for weight in weights])


def block_scores(columns, blocks, weights):
    scores = []
    for block, block_weights in zip(blocks, weights):
        raw = [sum(columns[column][i] * weight for column, weight in zip(block, block_weights)) for i in range(len(columns[block[0]]))]
        scores.append(standardize(raw))
    return scores


def estimate_group(rows):
    names = ["x1", "x2", "z1", "z2", "y1", "y2"]
    columns = [standardize([row[name] for row in rows]) for name in names]
    blocks = [[0, 1], [2, 3], [4, 5]]
    weights = [normalize(block, columns, [1.0] * len(block)) for block in blocks]
    for _ in range(3000):
        scores = block_scores(columns, blocks, weights)
        inner = []
        for i in range(3):
            proxy = [0.0] * len(rows)
            if i == 2:
                bx, bz = ols([scores[0], scores[1]], scores[2])
                proxy = [bx * x_score + bz * z_score for x_score, z_score in zip(scores[0], scores[1])]
            else:
                coefficient = corr(scores[i], scores[2])
                proxy = [coefficient * value for value in scores[2]]
            proxy = standardize(proxy)
            inner.append(proxy)
        updated = [normalize(block, columns, [cov(columns[column], inner[i]) for column in block]) for i, block in enumerate(blocks)]
        change = max(abs(old - new) for before, after in zip(weights, updated) for old, new in zip(before, after))
        weights = updated
        if change <= CONVERGENCE_TOLERANCE:
            break
    scores = block_scores(columns, blocks, weights)
    bx, bz = ols([scores[0], scores[1]], scores[2])
    fitted = [bx * x + bz * z for x, z in zip(scores[0], scores[1])]
    y_mean = mean(scores[2])
    residual = sum((actual - fit) ** 2 for actual, fit in zip(scores[2], fitted))
    total = sum((actual - y_mean) ** 2 for actual in scores[2])
    return {"paths": {("x", "y"): bx, ("z", "y"): bz}, "r_squared": {"y": 1.0 - residual / total}}


def run_quickpls():
    qpls(["run", str(RECIPE.relative_to(ROOT)), "--data", str(DATA.relative_to(ROOT)), "--output", str(QUICKPLS.relative_to(ROOT)), "--allow-experimental"], check=True, stdout=subprocess.DEVNULL)
    return json.loads(QUICKPLS.read_text(encoding="utf-8"))


def main():
    write_dataset()
    fingerprint = dataset_fingerprint(DATA, "mga_reference")
    write_recipe(fingerprint)
    rows = load_rows()
    reference = {group: estimate_group([row for row in rows if row["group"] == group]) for group in ["A", "B"]}
    result = run_quickpls()
    estimation = result["payload"]["estimation"]
    mga = estimation["mga"]
    quick_groups = {group["group"]: group for group in mga["groups"]}
    quick_paths = {
        group: {(path["source"], path["target"]): path["coefficient"] for path in payload["paths"]}
        for group, payload in quick_groups.items()
    }
    path_deltas = {
        f"{group}.{source}->{target}": abs(quick_paths[group][(source, target)] - value)
        for group, payload in reference.items()
        for (source, target), value in payload["paths"].items()
    }
    comparisons = {(item["source"], item["target"]): item for item in mga["comparisons"]}
    checks = {
        "method_version": estimation["method_version"] == "pls_mga_two_group_v1" and mga["method_version"] == "pls_mga_two_group_v1",
        "provenance_version": "pls_mga_two_group_v1" in result["provenance"]["method_version"],
        "group_column": mga["group_column"] == "group",
        "group_counts": {group["group"]: group["observations"] for group in mga["groups"]} == {"A": 90, "B": 90},
        "independent_reference_path_agreement": max(path_deltas.values()) < TOLERANCE,
        "path_difference_direction": comparisons[("x", "y")]["difference"] > 1.0 and comparisons[("z", "y")]["difference"] < -0.3,
        "p_values_available": all(item["p_value_two_sided"] is not None for item in mga["comparisons"]),
        "warnings_present": any("MGA" in warning for warning in mga["warnings"]),
    }
    report = {
        "checks": checks,
        "passed": all(checks.values()),
        "max_path_delta": max(path_deltas.values()),
        "path_deltas": path_deltas,
        "quickpls_comparisons": mga["comparisons"],
        "reference": {group: {"paths": {f"{source}->{target}": value for (source, target), value in payload["paths"].items()}, "r_squared": payload["r_squared"]} for group, payload in reference.items()},
        "quickpls": {"groups": mga["groups"], "warnings": mga["warnings"]},
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    if not report["passed"]:
        raise SystemExit(json.dumps(report, indent=2))
    print(json.dumps({"passed": True, "max_path_delta": report["max_path_delta"]}, indent=2))


if __name__ == "__main__":
    main()
