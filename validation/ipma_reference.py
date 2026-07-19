"""Independent IPMA reference for the experimental v0.6 bounded slice."""

import csv
import json
import math
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
DATA = RESULTS / "ipma_reference.csv"
RECIPE = RESULTS / "ipma_reference.recipe.json"
QUICKPLS = RESULTS / "ipma_reference_quickpls.json"
OUTPUT = RESULTS / "ipma_reference_report.json"
CLI_EXE = ROOT / "target" / "debug" / "qpls.exe"
TOLERANCE = 1e-6
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


def write_dataset():
    RESULTS.mkdir(parents=True, exist_ok=True)
    with DATA.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=["x1", "z1", "m1", "y1"])
        writer.writeheader()
        for index in range(1, 81):
            x = index / 10.0
            z = ((index * 7) % 29) / 10.0
            m = 0.62 * x + 0.38 * z + ((index % 5) - 2) * 0.035
            y = 0.34 * x + 0.21 * z + 0.71 * m + ((index % 7) - 3) * 0.025
            writer.writerow({
                "x1": f"{x:.12f}",
                "z1": f"{z:.12f}",
                "m1": f"{m:.12f}",
                "y1": f"{y:.12f}",
            })


def dataset_fingerprint(path, name):
    project_path = RESULTS / f"{name}.fingerprint.qpls"
    qpls(["import", str(path.relative_to(ROOT)), str(project_path.relative_to(ROOT)), "--name", name], check=True, stdout=subprocess.DEVNULL)
    payload = json.loads(qpls(["inspect", str(project_path.relative_to(ROOT)), "--json"], check=True, capture_output=True, text=True).stdout)
    return payload["datasets"][0]["fingerprint"]


def write_recipe(fingerprint):
    payload = {
        "schema_version": 2,
        "id": "00000000-0000-0000-0000-000000000081",
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": "00000000-0000-0000-0000-000000000082",
            "name": "IPMA reference",
            "constructs": [
                {"id": "x", "name": "X", "short_name": "X", "mode": "reflective", "indicators": ["x1"]},
                {"id": "z", "name": "Z", "short_name": "Z", "mode": "reflective", "indicators": ["z1"]},
                {"id": "m", "name": "M", "short_name": "M", "mode": "reflective", "indicators": ["m1"]},
                {"id": "y", "name": "Y", "short_name": "Y", "mode": "reflective", "indicators": ["y1"]},
            ],
            "paths": [
                {"source": "x", "target": "m"},
                {"source": "z", "target": "m"},
                {"source": "x", "target": "y"},
                {"source": "z", "target": "y"},
                {"source": "m", "target": "y"},
            ],
        },
        "settings": {
            "method": "ipma",
            "weighting_scheme": "path",
            "tolerance": 1e-10,
            "max_iterations": 3000,
            "bootstrap_samples": 0,
            "seed": 20260719,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
        },
        "metadata": {"fixture": "independent_ipma_reference", "ipma_targets": "y"},
    }
    RECIPE.write_text(json.dumps(payload, indent=2), encoding="utf-8")


def load_rows():
    with DATA.open(newline="", encoding="utf-8") as handle:
        return [{key: float(value) for key, value in row.items()} for row in csv.DictReader(handle)]


def mean(values):
    return sum(values) / len(values)


def sd(values):
    m = mean(values)
    return math.sqrt(sum((value - m) ** 2 for value in values) / (len(values) - 1))


def standardize(values):
    m = mean(values)
    s = sd(values)
    return [(value - m) / s for value in values]


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


def performance(values):
    lo = min(values)
    hi = max(values)
    if abs(hi - lo) <= 1e-12:
        return 50.0
    return mean([100.0 * (value - lo) / (hi - lo) for value in values])


def independent_reference(rows):
    scores = {name: standardize([row[f"{name}1"] for row in rows]) for name in ["x", "z", "m", "y"]}
    bx_m, bz_m = ols([scores["x"], scores["z"]], scores["m"])
    bx_y, bz_y, bm_y = ols([scores["x"], scores["z"], scores["m"]], scores["y"])
    effects = {
        "x": bx_y + bx_m * bm_y,
        "z": bz_y + bz_m * bm_y,
        "m": bm_y,
        "y": 1.0,
    }
    return {
        "constructs": {
            construct: {
                "importance": effects[construct],
                "performance": performance(scores[construct]),
                "score_mean": mean(scores[construct]),
            }
            for construct in ["x", "z", "m", "y"]
        },
        "indicators": {
            f"{construct}1": {
                "construct": construct,
                "construct_importance": effects[construct],
                "loading": 1.0,
                "performance": performance(scores[construct]),
                "score_mean": mean(scores[construct]),
            }
            for construct in ["x", "z", "m", "y"]
        },
    }


def run_quickpls():
    qpls(["run", str(RECIPE.relative_to(ROOT)), "--data", str(DATA.relative_to(ROOT)), "--output", str(QUICKPLS.relative_to(ROOT)), "--allow-experimental"], check=True, stdout=subprocess.DEVNULL)
    return json.loads(QUICKPLS.read_text(encoding="utf-8"))


def main():
    write_dataset()
    fingerprint = dataset_fingerprint(DATA, "ipma_reference")
    write_recipe(fingerprint)
    reference = independent_reference(load_rows())
    result = run_quickpls()
    estimation = result["payload"]["estimation"]
    ipma = estimation["ipma"]
    quick_constructs = {row["construct"]: row for row in ipma["constructs"] if row["target"] == "y"}
    quick_indicators = {row["indicator"]: row for row in ipma["indicators"] if row["target"] == "y"}
    construct_deltas = {
        f"{construct}.{metric}": abs(quick_constructs[construct][metric] - expected[metric])
        for construct, expected in reference["constructs"].items()
        for metric in ["importance", "performance", "score_mean"]
    }
    indicator_deltas = {
        f"{indicator}.{metric}": abs(quick_indicators[indicator][metric] - expected[metric])
        for indicator, expected in reference["indicators"].items()
        for metric in ["construct_importance", "loading", "performance", "score_mean"]
    }
    checks = {
        "method_version": estimation["method_version"] == "ipma_v1" and ipma["method_version"] == "ipma_v1",
        "provenance_version": "ipma_v1" in result["provenance"]["method_version"],
        "target_selection": ipma["targets"] == ["y"] and set(quick_constructs) == {"x", "z", "m", "y"},
        "construct_reference_agreement": max(construct_deltas.values()) < TOLERANCE,
        "indicator_reference_agreement": max(indicator_deltas.values()) < TOLERANCE,
        "performance_bounds": all(0.0 <= row["performance"] <= 100.0 for row in ipma["constructs"] + ipma["indicators"]),
        "warnings_present": any("IPMA" in warning for warning in ipma["warnings"]),
    }
    report = {
        "checks": checks,
        "passed": all(checks.values()),
        "max_construct_delta": max(construct_deltas.values()),
        "max_indicator_delta": max(indicator_deltas.values()),
        "construct_deltas": construct_deltas,
        "indicator_deltas": indicator_deltas,
        "quickpls": ipma,
        "reference": reference,
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    if not report["passed"]:
        raise SystemExit(json.dumps(report, indent=2))
    print(json.dumps({"passed": True, "max_construct_delta": report["max_construct_delta"], "max_indicator_delta": report["max_indicator_delta"]}, indent=2))


if __name__ == "__main__":
    main()
